//! Turning a resolved [`Provider`] row plus an Action's effective values into
//! one request body. The only place a wire divergence between endpoints lives.
//!
//! ## The dialect is the endpoint's, not the model's (ADR-0021)
//!
//! Every OpenAI-compatible endpoint agrees about `model`, `messages` and
//! `stream`. They disagree about exactly one thing Beckon needs: how you say
//! *do not think*. DeepSeek takes a `thinking` object, Qwen3 behind vLLM /
//! SGLang / DashScope takes `chat_template_kwargs.enable_thinking`, OpenAI's own
//! API takes `reasoning_effort: "none"`, and most endpoints have no such control
//! at all.
//!
//! Which of those an endpoint speaks cannot be derived from the model id —
//! `deepseek-ai/DeepSeek-V4-Flash` served by SiliconFlow speaks plain OpenAI — so
//! [`Reasoning`] is a field on the row, prefilled by its preset. This module
//! reads it and sends nothing it was not told to: an unrecognised field is a
//! `400` on a strict endpoint, not a field politely ignored.
//!
//! A dialect being the endpoint's property does not mean every model behind that
//! endpoint takes the field. Two of the three arms consult the model as well —
//! [`models::find`] for DeepSeek's catalog, [`EFFORT_NONE_FAMILIES`] for
//! OpenAI's — because both hosts serve models that disagree with their own
//! sibling about the switch. The row still decides *which* field could be sent;
//! the model decides whether it is.
//!
//! ## Thinking mode
//!
//! Verified against the DeepSeek API reference (`POST /chat/completions`):
//! `deepseek-v4-flash` / `deepseek-v4-pro` take `{"type": "enabled"}` or
//! `{"type": "disabled"}`. Thinking is **on by default** there, which is why
//! `thinking = false` has to be sent explicitly rather than omitted.
//!
//! One thing is still a hard error, and only one: an Action asking a model that
//! *always* thinks to stop. Omitting the field there would quietly add seconds
//! of latency to every turn — the exact failure the README wants gone. Asking
//! for thinking that cannot be expressed is the opposite trade and is *not* an
//! error: the endpoint's own default stands, Settings says so in amber, and an
//! Action repointed at Ollama keeps working rather than refusing to run.
//!
//! ## Temperature
//!
//! A per-row `Option`, absent meaning send nothing and let the endpoint decide.
//! The 1.3 that used to ride every request was DeepSeek's own guidance
//! (ADR-0019) and now lives on the DeepSeek row, because it was never a fact
//! about anybody else's endpoint (ADR-0021).
//!
//! ## Images
//!
//! A Capture goes on the wire as an OpenAI-style content part and is never
//! gated on the model: Beckon cannot keep a per-model image table true across
//! every endpoint a `base_url` may point at, so the endpoint's own error is the
//! only authority on what it reads (ADR-0016).

use serde_json::{json, Value};

use crate::action::ModelParams;
use crate::config::{Provider, Reasoning};

use super::models::{self, Thinking};
use super::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThinkingWire {
    /// `thinking: {"type": "enabled"|"disabled"}` — DeepSeek's own API.
    Deepseek(bool),
    /// `chat_template_kwargs: {"enable_thinking": bool}` — the vLLM/SGLang
    /// convention for a Qwen3 chat template.
    Qwen(bool),
    /// `reasoning_effort: "none"` — OpenAI's own API. No payload: the field is
    /// only ever sent to suppress, so the variant carries no direction.
    OpenAi,
    /// Send nothing: this endpoint has no switch, or the model has none.
    Omit,
}

/// Build the request body for one turn.
pub fn build_body(
    provider: &Provider,
    params: &ModelParams,
    messages: &[Message],
) -> Result<Value, String> {
    let mut body = json!({
        "model": params.model,
        "messages": messages,
        "stream": true,
    });

    if let Some(temperature) = provider.temperature {
        body["temperature"] = json!(temperature);
    }

    match thinking_wire(provider.reasoning, &params.model, params.thinking)? {
        ThinkingWire::Deepseek(enabled) => {
            let type_ = if enabled { "enabled" } else { "disabled" };
            body["thinking"] = json!({ "type": type_ });
        }
        ThinkingWire::Qwen(enabled) => {
            body["chat_template_kwargs"] = json!({ "enable_thinking": enabled });
        }
        ThinkingWire::OpenAi => {
            body["reasoning_effort"] = json!("none");
        }
        ThinkingWire::Omit => {}
    }

    Ok(body)
}

/// A minimal body for "Test connection": one token, no streaming, no thinking
/// cost. Deliberately does not go through [`build_body`] — a connection test
/// must not be able to fail on a thinking-mapping error, and it must not depend
/// on the row's dialect being right either.
pub fn build_probe_body(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{ "role": "user", "content": "ping" }],
        "stream": false,
        "max_tokens": 1,
    })
}

/// The OpenAI model families whose `reasoning_effort` accepts `"none"`.
///
/// **Checked 2026-08-24.** Prefixes, matched case-insensitively: a family's dated
/// snapshots take the same parameter set as the alias that names them.
///
/// This is per-*model* knowledge inside a per-endpoint dialect, which ADR-0021
/// otherwise keeps out — and it is here for the same reason
/// [`Reasoning::Deepseek`] consults [`models::find`]: one host serves models that
/// disagree about the field. `api.openai.com` also serves `gpt-4o`, which rejects
/// `reasoning_effort` outright, and `o3`, whose floor is `low`; `"none"` is a
/// `400` on every turn for either, and `get_models` offers both from the
/// endpoint's own list. `gpt-5.6` is the family that added the real floor, which
/// is what made this arm expressible at all.
///
/// **Unmatched is [`ThinkingWire::Omit`], never an error**, and the asymmetry is
/// deliberate: a model missing from this list either does not reason, in which
/// case nothing was lost, or reasons at its own default, which costs a slower
/// turn and not a failed one. The list falling behind OpenAI is therefore cheap,
/// while an allowlist guessed wide costs the turn — so widen it only by adding a
/// family the docs state, never by loosening the rule.
const EFFORT_NONE_FAMILIES: &[&str] = &["gpt-5.6"];

/// Whether OpenAI documents `reasoning_effort: "none"` for this model.
fn accepts_effort_none(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    EFFORT_NONE_FAMILIES
        .iter()
        .any(|family| model.starts_with(family))
}

fn thinking_wire(
    reasoning: Reasoning,
    model: &str,
    thinking: bool,
) -> Result<ThinkingWire, String> {
    match reasoning {
        // Nothing to suppress, so nothing to send — whatever the model is. This
        // covers every plain OpenAI-compatible endpoint, the reasoning models
        // you pick deliberately included.
        Reasoning::None => Ok(ThinkingWire::Omit),
        Reasoning::Qwen => Ok(ThinkingWire::Qwen(thinking)),
        // One direction only, and only for a family that documents the floor.
        // `thinking = true` sends nothing and lets the endpoint's own default
        // effort stand, which is the same shape as an always-on model asked to
        // think; an undocumented family omits for the reason
        // [`EFFORT_NONE_FAMILIES`] gives.
        Reasoning::OpenAi => Ok(if !thinking && accepts_effort_none(model) {
            ThinkingWire::OpenAi
        } else {
            ThinkingWire::Omit
        }),
        Reasoning::Deepseek => match models::find(model) {
            // A catalogued DeepSeek model: the one table decides, because the
            // legacy ids picked the mode through the id itself.
            Some(entry) => match entry.thinking {
                Thinking::Switchable => Ok(ThinkingWire::Deepseek(thinking)),
                Thinking::AlwaysOn if thinking => Ok(ThinkingWire::Omit),
                // The one refusal in this module. Silently leaving thinking on
                // is invisible latency on every turn.
                Thinking::AlwaysOn => Err(format!(
                    "{model} always thinks; set thinking = true for it, or choose {} to turn \
                     thinking off",
                    models::switchable_suggestion()
                )),
                // Documented as having no thinking mode — the vision model's
                // `thinking` object is a 400. Asking for it anyway is a
                // warning in Settings, not a refused turn.
                Thinking::Never => Ok(ThinkingWire::Omit),
            },
            // Uncatalogued, on an endpoint whose row says it speaks DeepSeek's
            // dialect: the dialect is the endpoint's property, so a v4 id newer
            // than this build — or a DeepSeek-weighted model behind a proxy the
            // user pointed here — still maps.
            None => Ok(ThinkingWire::Deepseek(thinking)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Content;

    fn provider(reasoning: Reasoning, temperature: Option<f64>) -> Provider {
        Provider {
            id: "p".into(),
            base_url: "https://example.com/v1".into(),
            reasoning,
            temperature,
            ..Provider::default()
        }
    }

    fn deepseek() -> Provider {
        Provider::deepseek()
    }

    fn params(model: &str, thinking: bool) -> ModelParams {
        ModelParams {
            provider: "p".into(),
            model: model.to_string(),
            thinking,
        }
    }

    #[test]
    fn a_deepseek_row_sends_the_documented_object_and_its_temperature() {
        let body = build_body(
            &deepseek(),
            &params("deepseek-v4-flash", false),
            &[Message::user("hi")],
        )
        .unwrap();
        assert_eq!(body["thinking"], json!({ "type": "disabled" }));
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["model"], json!("deepseek-v4-flash"));
        assert_eq!(body["temperature"], json!(1.3));

        let on = build_body(
            &deepseek(),
            &params("deepseek-v4-pro", true),
            &[Message::user("hi")],
        )
        .unwrap();
        assert_eq!(on["thinking"], json!({ "type": "enabled" }));
    }

    /// The Qwen3 convention, and the reason [`Reasoning`] has a second named
    /// arm at all: `false` has to be expressible for a family that reasons by
    /// default, and this is not the shape DeepSeek takes.
    #[test]
    fn a_qwen_row_sends_the_chat_template_flag() {
        for thinking in [true, false] {
            let body = build_body(
                &provider(Reasoning::Qwen, None),
                &params("Qwen3-8B", thinking),
                &[Message::user("hi")],
            )
            .unwrap();
            assert_eq!(
                body["chat_template_kwargs"],
                json!({ "enable_thinking": thinking })
            );
            assert!(body.get("thinking").is_none());
        }
    }

    /// The only one-directional arm. OpenAI's `reasoning_effort` has a real
    /// `none`, so `false` is expressible — but there is no value that means
    /// "reason as you normally would", and inventing a level the user never
    /// chose would be worse than silence.
    #[test]
    fn an_openai_row_sends_the_effort_floor_only_to_suppress() {
        let off = build_body(
            &provider(Reasoning::OpenAi, None),
            &params("gpt-5.6-terra", false),
            &[Message::user("hi")],
        )
        .unwrap();
        assert_eq!(off["reasoning_effort"], json!("none"));

        let on = build_body(
            &provider(Reasoning::OpenAi, None),
            &params("gpt-5.6-terra", true),
            &[Message::user("hi")],
        )
        .unwrap();
        assert!(on.get("reasoning_effort").is_none());
    }

    /// The dialect is the row's, but the field is the model's: `api.openai.com`
    /// serves models that reject `reasoning_effort` outright and models whose
    /// floor is `low`, and the dropdown offers whatever `/v1/models` returns. A
    /// row that sent the floor for all of them would `400` on every turn the
    /// moment the user picked one.
    #[test]
    fn an_openai_row_sends_the_floor_only_for_a_family_that_documents_it() {
        for model in ["gpt-4o", "o3", "gpt-4.1-mini", "text-embedding-3-large"] {
            assert_eq!(
                thinking_wire(Reasoning::OpenAi, model, false).unwrap(),
                ThinkingWire::Omit,
                "{model}"
            );
        }
        // A dated snapshot of a family that does document it, and the id's case
        // is the API's to normalise.
        for model in ["gpt-5.6-terra", "GPT-5.6-Terra", "gpt-5.6-terra-2026-07-11"] {
            assert_eq!(
                thinking_wire(Reasoning::OpenAi, model, false).unwrap(),
                ThinkingWire::OpenAi,
                "{model}"
            );
        }
    }

    /// Every family on the allowlist is a prefix rather than a whole id, and the
    /// preset row's starting model is one of them — a preset whose own model
    /// silently lost suppression is the failure this arm was added to fix.
    #[test]
    fn the_openai_preset_model_can_suppress_its_own_reasoning() {
        let openai = crate::config::presets()
            .into_iter()
            .find(|row| row.reasoning == Reasoning::OpenAi)
            .expect("a preset row speaks the OpenAI dialect");
        assert!(
            accepts_effort_none(&openai.model),
            "{} is not on EFFORT_NONE_FAMILIES",
            openai.model
        );
    }

    /// The default arm, and the one most endpoints get: nothing about thinking
    /// on the wire in either direction. An unknown field is a 400 on a strict
    /// endpoint, so silence is the only safe answer for a host that said nothing.
    #[test]
    fn a_plain_row_sends_nothing_about_thinking_either_way() {
        for thinking in [true, false] {
            let body = build_body(
                &provider(Reasoning::None, None),
                &params("some-model", thinking),
                &[Message::user("hi")],
            )
            .unwrap();
            assert!(body.get("thinking").is_none());
            assert!(body.get("chat_template_kwargs").is_none());
            assert!(body.get("reasoning_effort").is_none());
        }
    }

    /// Omitted `temperature` means the endpoint's own default. The 1.3 that used
    /// to be on every request was a fact about DeepSeek, never about a local
    /// llama.cpp (ADR-0019 → ADR-0021).
    #[test]
    fn a_row_without_a_temperature_sends_none() {
        let body = build_body(
            &provider(Reasoning::None, None),
            &params("m", false),
            &[Message::user("hi")],
        )
        .unwrap();
        assert!(body.get("temperature").is_none());
    }

    /// The dialect travels with the row, so the same model id sends different
    /// bodies to different endpoints — which is the property no rule over model
    /// ids could have produced.
    #[test]
    fn the_same_model_sends_a_different_body_per_endpoint() {
        let id = "deepseek-ai/DeepSeek-V4-Flash";
        let native = build_body(&deepseek(), &params(id, false), &[Message::user("hi")]).unwrap();
        let proxied = build_body(
            &provider(Reasoning::None, None),
            &params(id, false),
            &[Message::user("hi")],
        )
        .unwrap();
        assert_eq!(native["thinking"], json!({ "type": "disabled" }));
        assert!(proxied.get("thinking").is_none());
    }

    #[test]
    fn messages_serialize_in_openai_shape() {
        let body = build_body(
            &deepseek(),
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

    /// The one hard error left: a model that always thinks, asked to stop.
    /// Omitting the field would leave thinking on invisibly.
    #[test]
    fn asking_a_model_that_always_thinks_to_stop_is_refused() {
        let err = thinking_wire(Reasoning::Deepseek, "deepseek-reasoner", false).unwrap_err();
        assert!(err.contains("deepseek-reasoner"));
        assert!(err.contains(models::switchable_suggestion()), "{err}");
        // The other direction is fine — it is already what the model does.
        assert_eq!(
            thinking_wire(Reasoning::Deepseek, "deepseek-reasoner", true).unwrap(),
            ThinkingWire::Omit
        );
    }

    /// The opposite trade, and why it is not an error: thinking that cannot be
    /// expressed costs the user nothing but the feature. Refusing would break
    /// every Action repointed at an endpoint with no switch, which is the move
    /// this whole layer exists to make easy.
    #[test]
    fn thinking_that_cannot_be_expressed_is_omitted_rather_than_refused() {
        // Documented as having no thinking mode.
        assert_eq!(
            thinking_wire(Reasoning::Deepseek, "deepseek-v4-flash-vision-exp", true).unwrap(),
            ThinkingWire::Omit
        );
        // An endpoint with no switch at all, whatever the model.
        assert_eq!(
            thinking_wire(Reasoning::None, "deepseek-v4-pro", true).unwrap(),
            ThinkingWire::Omit
        );
    }

    /// The dropdown offers exactly what this walks; if the two lists were kept
    /// separately, this is the test that would stop existing.
    #[test]
    fn every_catalogued_model_maps_without_guessing() {
        for entry in models::CATALOG {
            match entry.thinking {
                Thinking::Switchable => {
                    for thinking in [true, false] {
                        assert_eq!(
                            thinking_wire(Reasoning::Deepseek, entry.id, thinking).unwrap(),
                            ThinkingWire::Deepseek(thinking),
                            "{}",
                            entry.id
                        );
                    }
                }
                Thinking::AlwaysOn => {
                    assert!(thinking_wire(Reasoning::Deepseek, entry.id, true).is_ok());
                    assert!(thinking_wire(Reasoning::Deepseek, entry.id, false).is_err());
                }
                Thinking::Never => {
                    for thinking in [true, false] {
                        assert_eq!(
                            thinking_wire(Reasoning::Deepseek, entry.id, thinking).unwrap(),
                            ThinkingWire::Omit,
                            "{}",
                            entry.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn model_ids_are_matched_case_insensitively() {
        assert_eq!(
            thinking_wire(Reasoning::Deepseek, "DeepSeek-V4-Flash", false).unwrap(),
            ThinkingWire::Deepseek(false)
        );
    }

    /// A v4 id newer than the catalog still maps, because the row said what the
    /// endpoint speaks. This used to be a special case over the model id; it is
    /// now the ordinary path.
    #[test]
    fn an_uncatalogued_model_follows_the_rows_dialect() {
        assert_eq!(
            thinking_wire(Reasoning::Deepseek, "deepseek-v9-quantum", true).unwrap(),
            ThinkingWire::Deepseek(true)
        );
    }

    #[test]
    fn an_image_sends_the_documented_parts_array() {
        let content = Content::with_images("read this", ["data:image/png;base64,AA"]);
        let body = build_body(
            &deepseek(),
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

    /// No model is gated on images, catalogued or not, on any endpoint: whether
    /// it reads one is the endpoint's answer to give (ADR-0016).
    #[test]
    fn any_model_on_any_endpoint_may_be_sent_an_image() {
        for row in [deepseek(), provider(Reasoning::None, None)] {
            for model in ["deepseek-v4-pro", "llava:13b"] {
                let content = Content::with_images("hi", ["data:image/png;base64,AA"]);
                assert!(build_body(&row, &params(model, false), &[Message::user(content)]).is_ok());
            }
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
