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

/// `{base_url}/v1/{path}`, tolerating a `base_url` that already carries the
/// version segment. Every endpoint goes through here so the tolerance is one
/// rule and not one per route.
fn api_url(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/{path}")
    } else {
        format!("{base}/v1/{path}")
    }
}

/// `POST {base_url}/v1/chat/completions`.
pub fn chat_url(base_url: &str) -> String {
    api_url(base_url, "chat/completions")
}

/// `GET {base_url}/v1/models`, the OpenAI-compatible list endpoint.
pub fn models_url(base_url: &str) -> String {
    api_url(base_url, "models")
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
        let body = response
            .text()
            .await
            .unwrap_or_else(|e| format!("<no response body: {e}>"));
        return Err(status_error(status.as_u16(), &body));
    }

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();
    let mut received_any = false;
    let mut saw_done = false;

    'stream: loop {
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
                    break 'stream;
                }
            }
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

    // One parse per frame: this runs once per streamed token.
    let chunk: Chunk = match serde_json::from_str(&payload) {
        Ok(chunk) => chunk,
        Err(err) => {
            // An unrecognised frame is not worth killing a live answer over.
            log::debug!("ignoring unparseable SSE payload ({err}): {payload}");
            return Ok(Flow::Continue);
        }
    };

    if let Some(error) = chunk.error {
        return Err(LlmError::Interrupted(error.message));
    }

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
    Err(status_error(
        status.as_u16(),
        &response.text().await.unwrap_or_default(),
    ))
}

/// The ids the endpoint says it serves.
///
/// Every failure comes back as an ordinary [`LlmError`], so a rejected key
/// stays distinguishable from an unreachable API (ADR-0005) even though the
/// caller's response to both is the same: fall back to the documented list.
pub async fn list_models(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, LlmError> {
    let response = http
        .get(models_url(base_url))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(status_error(
            status.as_u16(),
            &response.text().await.unwrap_or_default(),
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;
    parse_model_list(&body)
}

/// Split out from the request so the shape can be tested without a network.
fn parse_model_list(body: &str) -> Result<Vec<String>, LlmError> {
    let list: ModelList = serde_json::from_str(body).map_err(|e| LlmError::Http {
        status: 200,
        message: format!("the model list could not be read: {e}"),
    })?;
    Ok(list
        .data
        .into_iter()
        .filter_map(|entry| {
            let id = entry.id.trim().to_string();
            (!id.is_empty()).then_some(id)
        })
        .collect())
}

/// The one place a status code becomes an [`LlmError`]. ADR-0005 needs "key
/// rejected" to stay distinguishable from every other failure, so the 401/403
/// boundary is decided here rather than at each call site.
fn status_error(status: u16, body: &str) -> LlmError {
    let message = trim_error_body(body);
    if status == 401 || status == 403 {
        LlmError::Auth { status, message }
    } else {
        LlmError::Http { status, message }
    }
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
    /// Some gateways report failures as an SSE frame rather than a status code,
    /// so the same frame carries either deltas or an error — never both.
    error: Option<ApiError>,
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

/// `GET /models`: `{"object":"list","data":[{"id":…,"object":"model",…}]}`
/// (<https://api-docs.deepseek.com/api/list-models>). Only `id` is used.
#[derive(Debug, Deserialize)]
struct ModelList {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    #[serde(default)]
    id: String,
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
    fn tolerates_a_base_url_that_already_carries_the_version() {
        for (base, expected) in [
            ("https://api.deepseek.com", "https://api.deepseek.com/v1/x"),
            ("https://api.deepseek.com/", "https://api.deepseek.com/v1/x"),
            ("http://localhost:11434/v1", "http://localhost:11434/v1/x"),
            (
                "  https://example.com/api/  ",
                "https://example.com/api/v1/x",
            ),
        ] {
            assert_eq!(api_url(base, "x"), expected, "base {base}");
        }
    }

    #[test]
    fn names_the_two_endpoints() {
        assert_eq!(
            chat_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            models_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/models"
        );
    }

    #[test]
    fn reads_the_documented_model_list_shape() {
        // Verbatim from https://api-docs.deepseek.com/api/list-models.
        let body = r#"{"object":"list","data":[
            {"id":"deepseek-v4-flash","object":"model","owned_by":"deepseek"},
            {"id":"deepseek-v4-pro","object":"model","owned_by":"deepseek"}]}"#;
        assert_eq!(
            parse_model_list(body).unwrap(),
            vec!["deepseek-v4-flash", "deepseek-v4-pro"]
        );
    }

    #[test]
    fn an_empty_or_odd_model_list_is_not_a_crash() {
        assert!(parse_model_list(r#"{"object":"list","data":[]}"#)
            .unwrap()
            .is_empty());
        assert!(parse_model_list("{}").unwrap().is_empty());
        // Blank ids are dropped rather than offered as an unselectable option.
        assert!(parse_model_list(r#"{"data":[{"id":"  "}]}"#)
            .unwrap()
            .is_empty());
        // A page of HTML is a failure, not an empty list — the caller has to be
        // able to say why it fell back.
        assert!(parse_model_list("<html>nope</html>").is_err());
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
