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
//!
//! ## Images
//!
//! A Capture goes on the wire as an OpenAI-style content part and is never
//! gated on the model: Beckon cannot keep a per-model image table true across
//! every provider a custom `base_url` may point at, so the endpoint's own error
//! is the only authority on what it reads (ADR-0016).
//!
//! Per-model behaviour is not written down here: it lives in
//! [`super::models::CATALOG`], which is also what the Settings dropdown is built
//! from. Keeping one table means the set of models we offer and the set we know
//! how to send cannot drift apart.

use serde_json::{json, Value};

use crate::action::ModelParams;

use super::models::{self, Thinking};
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
    // A catalogued model: the one table decides. Legacy DeepSeek naming picked
    // the mode through the model id, so for those a `thinking` flag that
    // disagrees with the model cannot be honoured.
    if let Some(entry) = models::find(model) {
        let suggestion = models::switchable_suggestion();
        return match entry.thinking {
            Thinking::Switchable => Ok(ThinkingWire::Object(thinking)),
            Thinking::AlwaysOn if thinking => Ok(ThinkingWire::Omit),
            Thinking::AlwaysOn => Err(format!(
                "{model} always thinks; set thinking = true for it, or choose {suggestion} to \
                 turn thinking off"
            )),
            Thinking::Never if thinking => Err(format!(
                "{model} has no thinking mode; set thinking = false for it, or choose {suggestion}"
            )),
            Thinking::Never => Ok(ThinkingWire::Omit),
        };
    }

    let model_lower = model.to_ascii_lowercase();

    // Uncatalogued, but in the documented v4 family: the `thinking` object is
    // the family's wire format, so a v4 id newer than this build is still safe
    // to map.
    if model_lower.starts_with("deepseek-v4") {
        return Ok(ThinkingWire::Object(thinking));
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
    use crate::llm::Content;

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

    /// The dropdown offers exactly what this loop walks; if the two lists were
    /// kept separately, this is the test that would stop existing.
    #[test]
    fn every_catalogued_model_maps_without_guessing() {
        for entry in models::CATALOG {
            match entry.thinking {
                Thinking::Switchable => {
                    assert_eq!(
                        thinking_wire(entry.id, true).unwrap(),
                        ThinkingWire::Object(true),
                        "{}",
                        entry.id
                    );
                    assert_eq!(
                        thinking_wire(entry.id, false).unwrap(),
                        ThinkingWire::Object(false),
                        "{}",
                        entry.id
                    );
                }
                Thinking::AlwaysOn => {
                    assert!(thinking_wire(entry.id, true).is_ok(), "{}", entry.id);
                    assert!(thinking_wire(entry.id, false).is_err(), "{}", entry.id);
                }
                Thinking::Never => {
                    assert!(thinking_wire(entry.id, false).is_ok(), "{}", entry.id);
                    assert!(thinking_wire(entry.id, true).is_err(), "{}", entry.id);
                }
            }
        }
    }

    #[test]
    fn model_ids_are_matched_case_insensitively() {
        assert_eq!(
            thinking_wire("DeepSeek-V4-Flash", false).unwrap(),
            ThinkingWire::Object(false)
        );
    }

    #[test]
    fn a_v4_id_newer_than_the_catalog_still_maps() {
        assert_eq!(
            thinking_wire("deepseek-v4-turbo", true).unwrap(),
            ThinkingWire::Object(true)
        );
    }

    #[test]
    fn errors_suggest_a_model_the_dropdown_offers() {
        let err = thinking_wire("deepseek-chat", true).unwrap_err();
        assert!(err.contains(models::switchable_suggestion()), "{err}");
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
    fn an_image_sends_the_documented_parts_array() {
        let content = Content::with_images("read this", ["data:image/png;base64,AA"]);
        let body = build_body(
            &params("deepseek-v4-flash-vision-exp", false),
            &[Message::system("s"), Message::user(content)],
        )
        .unwrap();
        assert_eq!(
            body["messages"][1]["content"],
            json!([
                { "type": "text", "text": "read this" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,AA" } }
            ])
        );
        // Thinking is undocumented for it, so nothing is sent.
        assert!(body.get("thinking").is_none());
    }

    /// No model is gated on images, catalogued or not: whether the endpoint
    /// reads one is the endpoint's answer to give (ADR-0016).
    #[test]
    fn any_model_may_be_sent_an_image() {
        for model in ["deepseek-v4-pro", "llava:13b"] {
            let content = Content::with_images("hi", ["data:image/png;base64,AA"]);
            assert!(build_body(&params(model, false), &[Message::user(content)]).is_ok());
        }
    }

    #[test]
    fn probe_body_is_cheap_and_never_streams() {
        let body = build_probe_body("deepseek-v4-flash");
        assert_eq!(body["stream"], json!(false));
        assert_eq!(body["max_tokens"], json!(1));
        assert!(body.get("thinking").is_none());
    }
}
