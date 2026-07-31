//! The HTTP side: one streaming call and one connection probe.
//!
//! **No timeout, deliberately** (README): a dead network must surface as an
//! immediate error in the Popover rather than a spinner that never resolves,
//! and a long thinking pause must not be mistaken for a hang.

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::sse::{SseEvent, SseParser};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    /// The key was rejected. Distinct from every "cannot read the key" case in
    /// [`crate::secrets`] (ADR-0005).
    #[error("the API key was rejected ({status}): {message}")]
    Auth { status: u16, message: String },
    #[error("the API returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("could not reach the API: {0}")]
    Network(String),
    /// The stream died after partial output. The Popover keeps that text and
    /// marks it interrupted (README).
    #[error("the stream was interrupted: {0}")]
    Interrupted(String),
    #[error("cancelled")]
    Cancelled,
}

impl LlmError {
    /// Stable discriminant for the frontend state machine.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Auth { .. } => "auth",
            Self::Http { .. } => "http",
            Self::Network(_) => "network",
            Self::Interrupted(_) => "interrupted",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// Answer text.
    Content(String),
    /// Thinking text, when the model is in thinking mode.
    Reasoning(String),
}

/// `POST {base_url}/v1/chat/completions`, tolerating a `base_url` that already
/// carries the version segment.
pub fn chat_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

pub fn build_http_client() -> reqwest::Client {
    // No `.timeout(..)` on purpose — see the module docs.
    reqwest::Client::builder()
        .user_agent(concat!("beckon/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP client")
}

/// Stream one completion. `on_event` is called on the calling task, so it may
/// emit Tauri events directly.
pub async fn stream_chat(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    body: &Value,
    cancel: &CancellationToken,
    mut on_event: impl FnMut(StreamEvent),
) -> Result<(), LlmError> {
    let request = http
        .post(chat_url(base_url))
        .bearer_auth(api_key)
        .header("accept", "text/event-stream")
        .json(body);

    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(LlmError::Cancelled),
        result = request.send() => result.map_err(|e| LlmError::Network(e.to_string()))?,
    };

    let status = response.status();
    if !status.is_success() {
        let message = response
            .text()
            .await
            .unwrap_or_else(|e| format!("<no response body: {e}>"));
        let message = trim_error_body(&message);
        return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
            LlmError::Auth {
                status: status.as_u16(),
                message,
            }
        } else {
            LlmError::Http {
                status: status.as_u16(),
                message,
            }
        });
    }

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();
    let mut received_any = false;
    let mut saw_done = false;

    loop {
        let chunk = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(LlmError::Cancelled),
            next = stream.next() => next,
        };

        let Some(chunk) = chunk else { break };
        let chunk = chunk.map_err(|e| {
            // A drop after partial output is an interruption, not a plain
            // network failure: the partial text is worth keeping.
            if received_any {
                LlmError::Interrupted(e.to_string())
            } else {
                LlmError::Network(e.to_string())
            }
        })?;

        for event in parser.push(&chunk) {
            match handle_event(event, &mut on_event)? {
                Flow::Continue => received_any = true,
                Flow::Done => {
                    saw_done = true;
                    break;
                }
            }
        }
        if saw_done {
            break;
        }
    }

    if !saw_done {
        for event in parser.finish() {
            if let Flow::Done = handle_event(event, &mut on_event)? {
                saw_done = true;
            }
        }
    }

    // The server closed without `[DONE]`: partial output, mark interrupted.
    if !saw_done && received_any {
        return Err(LlmError::Interrupted(
            "the connection closed before the response finished".to_string(),
        ));
    }

    Ok(())
}

enum Flow {
    Continue,
    Done,
}

fn handle_event(event: SseEvent, on_event: &mut impl FnMut(StreamEvent)) -> Result<Flow, LlmError> {
    let payload = match event {
        SseEvent::Done => return Ok(Flow::Done),
        SseEvent::Data(payload) => payload,
    };

    // Some gateways report failures as an SSE frame rather than a status code.
    if let Ok(error) = serde_json::from_str::<ApiErrorFrame>(&payload) {
        if let Some(error) = error.error {
            return Err(LlmError::Interrupted(error.message));
        }
    }

    let chunk: Chunk = match serde_json::from_str(&payload) {
        Ok(chunk) => chunk,
        Err(err) => {
            // An unrecognised frame is not worth killing a live answer over.
            log::debug!("ignoring unparseable SSE payload ({err}): {payload}");
            return Ok(Flow::Continue);
        }
    };

    for choice in chunk.choices {
        let Some(delta) = choice.delta else { continue };
        if let Some(text) = delta.reasoning_content.filter(|t| !t.is_empty()) {
            on_event(StreamEvent::Reasoning(text));
        }
        if let Some(text) = delta.content.filter(|t| !t.is_empty()) {
            on_event(StreamEvent::Content(text));
        }
    }

    Ok(Flow::Continue)
}

/// "Test connection" (Phase 2 / ADR-0005): the smallest request that proves the
/// key and `base_url` work, reporting auth failure separately from a network
/// failure.
pub async fn test_connection(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), LlmError> {
    let response = http
        .post(chat_url(base_url))
        .bearer_auth(api_key)
        .json(&super::deepseek::build_probe_body(model))
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let message = trim_error_body(&response.text().await.unwrap_or_default());
    Err(if status.as_u16() == 401 || status.as_u16() == 403 {
        LlmError::Auth {
            status: status.as_u16(),
            message,
        }
    } else {
        LlmError::Http {
            status: status.as_u16(),
            message,
        }
    })
}

/// Error bodies are sometimes an HTML page. Prefer the JSON `error.message`,
/// and cap whatever is left so the Popover stays readable.
fn trim_error_body(body: &str) -> String {
    if let Ok(frame) = serde_json::from_str::<ApiErrorFrame>(body) {
        if let Some(error) = frame.error {
            return error.message;
        }
    }
    let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 300 {
        flat.chars().take(300).collect::<String>() + "…"
    } else {
        flat
    }
}

#[derive(Debug, Deserialize)]
struct Chunk {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
    /// DeepSeek streams thinking text in its own field.
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorFrame {
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(default)]
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_completions_url() {
        assert_eq!(
            chat_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            chat_url("https://api.deepseek.com/"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            chat_url("http://localhost:11434/v1"),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_url("  https://example.com/api/  "),
            "https://example.com/api/v1/chat/completions"
        );
    }

    fn drain(payload: &str) -> (Vec<StreamEvent>, Result<Flow, LlmError>) {
        let mut events = Vec::new();
        let result = handle_event(SseEvent::Data(payload.to_string()), &mut |e| events.push(e));
        (events, result)
    }

    #[test]
    fn extracts_content_and_reasoning_deltas() {
        let (events, result) =
            drain(r#"{"choices":[{"delta":{"reasoning_content":"hm","content":"Hi"}}]}"#);
        assert!(result.is_ok());
        assert_eq!(
            events,
            vec![
                StreamEvent::Reasoning("hm".into()),
                StreamEvent::Content("Hi".into())
            ]
        );
    }

    #[test]
    fn ignores_role_only_and_empty_deltas() {
        let (events, result) = drain(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#);
        assert!(result.is_ok());
        assert!(events.is_empty());

        let (events, _) = drain(r#"{"choices":[{"delta":{"content":""}}]}"#);
        assert!(events.is_empty());
    }

    #[test]
    fn ignores_unparseable_frames_instead_of_failing() {
        let (events, result) = drain("not json at all");
        assert!(matches!(result, Ok(Flow::Continue)));
        assert!(events.is_empty());
    }

    #[test]
    fn an_error_frame_mid_stream_interrupts() {
        let (_, result) = drain(r#"{"error":{"message":"rate limited"}}"#);
        assert_eq!(
            result.err(),
            Some(LlmError::Interrupted("rate limited".into()))
        );
    }

    #[test]
    fn error_bodies_prefer_the_json_message() {
        assert_eq!(
            trim_error_body(r#"{"error":{"message":"Invalid API key"}}"#),
            "Invalid API key"
        );
        let html = format!("<html>{}</html>", "x".repeat(500));
        assert!(trim_error_body(&html).chars().count() <= 301);
    }

    #[test]
    fn error_kinds_are_stable() {
        assert_eq!(
            LlmError::Auth {
                status: 401,
                message: String::new()
            }
            .kind(),
            "auth"
        );
        assert_eq!(LlmError::Network("x".into()).kind(), "network");
        assert_eq!(LlmError::Interrupted("x".into()).kind(), "interrupted");
    }
}
