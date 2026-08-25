//! The response shapes, and the pure functions that turn them into something
//! the rest of the app can use.
//!
//! Split out from `client` so every frame and every list body can be tested
//! without a network: `client` is then only the request.

use serde::Deserialize;

use super::error::LlmError;
use super::sse::SseEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    /// Answer text.
    Content(String),
    /// Thinking text, when the model is in thinking mode.
    Reasoning(String),
}

/// Whether the stream should keep reading after a frame.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Flow {
    Continue,
    Done,
}

pub(super) fn handle_event(
    event: SseEvent,
    on_event: &mut impl FnMut(StreamEvent),
) -> Result<Flow, LlmError> {
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
        // Whichever field this host names its thinking with; never both, and no
        // host sends both.
        if let Some(text) = delta
            .reasoning_content
            .or(delta.reasoning)
            .filter(|t| !t.is_empty())
        {
            on_event(StreamEvent::Reasoning(text));
        }
        if let Some(text) = delta.content.filter(|t| !t.is_empty()) {
            on_event(StreamEvent::Content(text));
        }
    }

    Ok(Flow::Continue)
}

/// Split out from the request so the shape can be tested without a network.
pub(super) fn parse_model_list(body: &str) -> Result<Vec<String>, LlmError> {
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
    /// OpenRouter streams the same thing under a shorter name, and reads
    /// `reasoning_content` on the way *in* without echoing it on the way out —
    /// so a row pointed there would stream thinking that rendered as nothing.
    ///
    /// An alias rather than a second event: which field a host chose is not a
    /// distinction the Popover has any use for. `reasoning_details` is
    /// deliberately not read — it is a structured array carrying signatures and
    /// redacted blocks for replay, not display text, and the plain field is
    /// present alongside it.
    reasoning: Option<String>,
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
pub(super) struct ApiErrorFrame {
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ApiError {
    #[serde(default)]
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// OpenRouter's spelling of the same thing. Without this arm a row pointed
    /// there streams thinking that renders as nothing at all.
    #[test]
    fn reads_thinking_under_either_field_name() {
        let (events, result) = drain(r#"{"choices":[{"delta":{"reasoning":"hm"}}]}"#);
        assert!(result.is_ok());
        assert_eq!(events, vec![StreamEvent::Reasoning("hm".into())]);
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
}
