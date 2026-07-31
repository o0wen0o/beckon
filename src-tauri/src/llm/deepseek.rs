//! Provider quirks, kept to one module.
//!
//! ## Thinking mode
//!
//! Verified against the DeepSeek API reference (`POST /chat/completions`):
//! `deepseek-v4-flash` / `deepseek-v4-pro` take a `thinking` object,
//! `{"type": "enabled"}` or `{"type": "disabled"}`, alongside an optional
//! `reasoning_effort`. Thinking is **on by default**, which is why
//! `thinking = false` has to be sent explicitly rather than omitted.
//!
//! Anything we cannot map is a hard error. Silently omitting the field on a
//! DeepSeek model would leave thinking on and quietly add seconds of latency to
//! every translation — the exact failure the README wants gone.

use serde_json::{json, Value};

use crate::action::ModelParams;

use super::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThinkingWire {
    /// Send `thinking: {"type": ...}`.
    Object(bool),
    /// Send nothing: the model has no thinking mode and none is requested.
    Omit,
}

/// Build the request body for one turn.
pub fn build_body(params: &ModelParams, messages: &[Message]) -> Result<Value, String> {
    let mut body = json!({
        "model": params.model,
        "messages": messages,
        "stream": true,
        "temperature": params.temperature,
    });

    if let ThinkingWire::Object(enabled) = thinking_wire(&params.model, params.thinking)? {
        let type_ = if enabled { "enabled" } else { "disabled" };
        body["thinking"] = json!({ "type": type_ });
    }

    Ok(body)
}

/// A minimal body for "Test connection": one token, no streaming, no thinking
/// cost. Deliberately does not go through [`build_body`] — a connection test
/// must not be able to fail on a thinking-mapping error.
pub fn build_probe_body(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{ "role": "user", "content": "ping" }],
        "stream": false,
        "max_tokens": 1,
    })
}

fn thinking_wire(model: &str, thinking: bool) -> Result<ThinkingWire, String> {
    let model_lower = model.to_ascii_lowercase();

    // The v4 family: documented `thinking` object, both directions.
    if model_lower.starts_with("deepseek-v4") {
        return Ok(ThinkingWire::Object(thinking));
    }

    // Older DeepSeek naming picked the mode through the model id, so a
    // `thinking` flag that disagrees with the model cannot be honoured.
    if model_lower == "deepseek-reasoner" {
        return if thinking {
            Ok(ThinkingWire::Omit)
        } else {
            Err(format!(
                "{model} always thinks; set thinking = true for it, or choose deepseek-v4-flash \
                 to turn thinking off"
            ))
        };
    }
    if model_lower == "deepseek-chat" {
        return if thinking {
            Err(format!(
                "{model} has no thinking mode; set thinking = false for it, or choose \
                 deepseek-v4-pro"
            ))
        } else {
            Ok(ThinkingWire::Omit)
        };
    }

    // Any other `deepseek-*`: we do not know its wire format, and guessing
    // wrong is invisible. Refuse instead.
    if model_lower.starts_with("deepseek") {
        return Err(format!(
            "Beckon does not know how to set thinking = {thinking} for {model}. Use a \
             deepseek-v4-* model, or remove the thinking setting for this Action."
        ));
    }

    // Non-DeepSeek model behind a custom base_url: no thinking concept, so
    // `false` is simply its behaviour. `true` cannot be honoured.
    if thinking {
        return Err(format!(
            "thinking = true is not supported for {model}; only deepseek-v4-* models accept it"
        ));
    }
    Ok(ThinkingWire::Omit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(model: &str, thinking: bool) -> ModelParams {
        ModelParams {
            model: model.to_string(),
            thinking,
            temperature: 1.3,
        }
    }

    #[test]
    fn v4_disabled_sends_the_documented_object() {
        let body = build_body(&params("deepseek-v4-flash", false), &[Message::user("hi")]).unwrap();
        assert_eq!(body["thinking"], json!({ "type": "disabled" }));
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["model"], json!("deepseek-v4-flash"));
        assert_eq!(body["temperature"], json!(1.3));
    }

    #[test]
    fn v4_enabled_sends_the_documented_object() {
        let body = build_body(&params("deepseek-v4-pro", true), &[Message::user("hi")]).unwrap();
        assert_eq!(body["thinking"], json!({ "type": "enabled" }));
    }

    #[test]
    fn messages_serialize_in_openai_shape() {
        let body = build_body(
            &params("deepseek-v4-flash", false),
            &[Message::system("s"), Message::user("u")],
        )
        .unwrap();
        assert_eq!(
            body["messages"],
            json!([
                { "role": "system", "content": "s" },
                { "role": "user", "content": "u" }
            ])
        );
    }

    #[test]
    fn legacy_model_ids_are_mapped_by_name() {
        assert_eq!(
            thinking_wire("deepseek-chat", false).unwrap(),
            ThinkingWire::Omit
        );
        assert_eq!(
            thinking_wire("deepseek-reasoner", true).unwrap(),
            ThinkingWire::Omit
        );
        assert!(thinking_wire("deepseek-chat", true).is_err());
        assert!(thinking_wire("deepseek-reasoner", false).is_err());
    }

    #[test]
    fn unknown_deepseek_models_fail_loudly_rather_than_omitting() {
        let err = thinking_wire("deepseek-v9-quantum", false).unwrap_err();
        assert!(err.contains("deepseek-v9-quantum"));
        assert!(thinking_wire("deepseek-v9-quantum", true).is_err());
    }

    #[test]
    fn non_deepseek_models_omit_when_thinking_is_off() {
        assert_eq!(
            thinking_wire("gpt-4o-mini", false).unwrap(),
            ThinkingWire::Omit
        );
        assert!(thinking_wire("gpt-4o-mini", true).is_err());
    }

    #[test]
    fn probe_body_is_cheap_and_never_streams() {
        let body = build_probe_body("deepseek-v4-flash");
        assert_eq!(body["stream"], json!(false));
        assert_eq!(body["max_tokens"], json!(1));
        assert!(body.get("thinking").is_none());
    }
}
