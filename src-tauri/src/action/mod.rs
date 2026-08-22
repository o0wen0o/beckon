//! The Action: a preset prompt plus how it is triggered (ADR-0003).
//!
//! An Action's **identity is its filename stem**; `name` is display only. `id`
//! is therefore derived on load and never stored in the file — renaming the
//! display name must not change identity.

pub mod registry;
pub mod watcher;

use serde::{Deserialize, Serialize};

use crate::config::Config;

/// What `prompt.user` defaults to when the file omits it.
pub const DEFAULT_USER_TEMPLATE: &str = "{{input}}";
/// The placeholder replaced by the Selection or the typed input.
pub const INPUT_PLACEHOLDER: &str = "{{input}}";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputSource {
    /// Typed input only; any Selection is ignored. The arm that has to exist:
    /// the grab lands before the Action is known (ADR-0006), so without it an
    /// "ask anything" Action fired while text happens to be selected would send
    /// that text (ADR-0020).
    Prompt,
    /// Selection if there is one, otherwise ask for typed input. The default,
    /// and now the only other arm — `selection` was a third that only ever
    /// produced a hint where this one produces an input box (ADR-0020). The
    /// alias keeps a config that still names it loading, rather than failing the
    /// whole file on an unknown variant.
    #[default]
    #[serde(alias = "selection")]
    Auto,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptSpec {
    pub system: String,
    /// Omitted in the file ⇒ [`DEFAULT_USER_TEMPLATE`].
    pub user: Option<String>,
}

/// The Action's `[model]` table. Unknown keys are tolerated deliberately, and
/// no `deny_unknown_fields` may be added here: it is what lets an Action still
/// naming the withdrawn `temperature` load and drop it on the next write rather
/// than becoming a diagnostic (ADR-0019).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelOverrides {
    /// Which endpoint this Action posts to — a
    /// [`Provider::id`](crate::config::Provider::id). Absent means
    /// `[defaults] provider`, exactly as absent means "inherit" for the two
    /// below (ADR-0021).
    ///
    /// This is what replaced a global switch: two endpoints can be live at once,
    /// so "where does my text go" is a question each Action answers.
    pub provider: Option<String>,
    pub model: Option<String>,
    pub thinking: Option<bool>,
}

/// Effective per-request model parameters: the Action's `[model]` table laid
/// over the [`Provider`](crate::config::Provider) row it resolves to.
///
/// The provider is carried by *id* rather than as the row itself, because the
/// row is re-read at request time — a `base_url` edited while an Exchange is
/// open must reach the next follow-up, and a row deleted under one has to be
/// reportable (`exchange/turn.rs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelParams {
    pub provider: String,
    pub model: String,
    pub thinking: bool,
}

impl ModelOverrides {
    /// Field-by-field override. This is the one merge function in the codebase.
    ///
    /// It takes the whole [`Config`] rather than a defaults struct because the
    /// two values it falls back to now live on whichever row the *first* field
    /// resolves to: overriding `provider` changes what `model` and `thinking`
    /// inherit, and that chain is the design (ADR-0021).
    ///
    /// A provider id naming no row leaves the overrides standing on their own.
    /// Nothing is invented here — `turn.rs` reports the missing row, which is
    /// where the user can be told about it.
    pub fn merge_over(&self, config: &Config) -> ModelParams {
        let provider = self
            .provider
            .clone()
            .unwrap_or_else(|| config.defaults.provider.clone());
        let row = config.api.find(&provider);
        ModelParams {
            provider,
            model: self
                .model
                .clone()
                .or_else(|| row.map(|one| one.model.clone()))
                .unwrap_or_default(),
            thinking: self
                .thinking
                .or_else(|| row.map(|one| one.thinking))
                .unwrap_or(false),
        }
    }
}

/// The on-disk shape of an Action file. Unknown fields are tolerated so a file
/// written by a newer Beckon still loads.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActionFile {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_source: InputSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    pub prompt: PromptSpec,
    pub model: ModelOverrides,
}

/// A loaded Action: the file contents plus the identity derived from its path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Action {
    /// Filename stem — the identity (README).
    pub id: String,
    /// Filename including the `.toml` extension, surfaced in the editor so the
    /// "renaming the display name does not rename the file" rule is visible.
    pub file_name: String,
    #[serde(flatten)]
    pub file: ActionFile,
}

impl Action {
    /// Parse one Action file. `file_name` carries the extension; the id is its
    /// stem.
    pub fn parse(file_name: &str, text: &str) -> Result<Self, String> {
        let file: ActionFile = toml::from_str(text).map_err(|e| e.to_string())?;
        Self::from_parts(file_name, file)
    }

    pub fn from_parts(file_name: &str, file: ActionFile) -> Result<Self, String> {
        if file.name.trim().is_empty() {
            return Err("`name` is required and must not be empty".to_string());
        }
        if file.prompt.system.trim().is_empty() {
            return Err("`prompt.system` is required and must not be empty".to_string());
        }
        let id = file_name
            .strip_suffix(".toml")
            .unwrap_or(file_name)
            .to_string();
        if id.is_empty() {
            return Err("file name has no stem".to_string());
        }
        Ok(Self {
            id,
            file_name: file_name.to_string(),
            file,
        })
    }

    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(&self.file).map_err(|e| e.to_string())
    }

    pub fn model_params(&self, config: &Config) -> ModelParams {
        self.file.model.merge_over(config)
    }

    /// The user message for a turn: `prompt.user` with `{{input}}` substituted,
    /// defaulting to the bare placeholder.
    pub fn render_user(&self, input: &str) -> String {
        self.file
            .prompt
            .user
            .as_deref()
            .unwrap_or(DEFAULT_USER_TEMPLATE)
            .replace(INPUT_PLACEHOLDER, input)
    }
}

/// Turn a display name into a filename stem: lowercase, non-alphanumerics
/// collapsed to `-`. Non-ASCII names can slug to nothing, hence the fallback.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "action".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSLATE: &str = r#"
name = "Translate"
description = "Chinese <-> English"
input_source = "prompt"
hotkey = "Ctrl+Alt+T"

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese.
Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
"""

[model]
thinking = true
"#;

    #[test]
    fn parses_readme_example() {
        let action = Action::parse("translate.toml", TRANSLATE).unwrap();
        assert_eq!(action.id, "translate");
        assert_eq!(action.file.name, "Translate");
        assert_eq!(action.file.input_source, InputSource::Prompt);
        assert_eq!(action.file.hotkey.as_deref(), Some("Ctrl+Alt+T"));
        assert_eq!(action.file.model.thinking, Some(true));
        assert_eq!(action.file.model.model, None);
    }

    #[test]
    fn multi_line_system_prompt_round_trips() {
        let action = Action::parse("translate.toml", TRANSLATE).unwrap();
        assert!(action.file.prompt.system.contains("translation engine"));
        assert!(action.file.prompt.system.contains('\n'));

        let text = action.to_toml().unwrap();
        let again = Action::parse("translate.toml", &text).unwrap();
        assert_eq!(again.file.prompt.system, action.file.prompt.system);
        assert_eq!(again.file, action.file);
    }

    #[test]
    fn id_comes_from_the_filename_not_the_name_field() {
        let action = Action::parse("my-file.toml", TRANSLATE).unwrap();
        assert_eq!(action.id, "my-file");
        assert_eq!(action.file.name, "Translate");
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        let text = "name = \"X\"\nfuture = true\n\n[prompt]\nsystem = \"s\"\n";
        assert!(Action::parse("x.toml", text).is_ok());
    }

    /// The key ADR-0021 added, and the absence that means "inherit".
    #[test]
    fn a_provider_override_round_trips() {
        let text = "name = \"X\"\n[prompt]\nsystem = \"s\"\n[model]\nprovider = \"ollama\"\n";
        let action = Action::parse("x.toml", text).unwrap();
        assert_eq!(action.file.model.provider.as_deref(), Some("ollama"));
        assert!(action.to_toml().unwrap().contains("provider = \"ollama\""));

        let bare = Action::parse("x.toml", "name = \"X\"\n[prompt]\nsystem = \"s\"\n").unwrap();
        assert_eq!(bare.file.model.provider, None);
        // Absent is how inheriting is spelled, so nothing is written for it.
        assert!(!bare.to_toml().unwrap().contains("provider"));
    }

    #[test]
    fn input_source_defaults_to_auto() {
        let action = Action::parse("x.toml", "name = \"X\"\n[prompt]\nsystem = \"s\"\n").unwrap();
        assert_eq!(action.file.input_source, InputSource::Auto);
    }

    /// The arm ADR-0020 removed still has to load, or trimming the enum would
    /// break every Action file written before it.
    #[test]
    fn the_retired_selection_arm_still_loads_as_auto() {
        let text = "name = \"X\"\ninput_source = \"selection\"\n[prompt]\nsystem = \"s\"\n";
        let action = Action::parse("x.toml", text).unwrap();
        assert_eq!(action.file.input_source, InputSource::Auto);
        // ...and is written back under the surviving name, not preserved.
        assert!(action
            .to_toml()
            .unwrap()
            .contains("input_source = \"auto\""));
    }

    /// Likewise the `[model]` key ADR-0019 removed: an unknown key is tolerated,
    /// so the file loads and the stale value is dropped on the next write.
    #[test]
    fn a_stale_temperature_key_is_ignored() {
        let text = "name = \"X\"\n[prompt]\nsystem = \"s\"\n[model]\ntemperature = 0.2\n";
        let action = Action::parse("x.toml", text).unwrap();
        assert!(!action.to_toml().unwrap().contains("temperature"));
    }

    #[test]
    fn empty_name_or_system_is_a_parse_error() {
        assert!(Action::parse("x.toml", "[prompt]\nsystem = \"s\"\n").is_err());
        assert!(Action::parse("x.toml", "name = \"X\"\n").is_err());
    }

    #[test]
    fn user_template_defaults_to_the_placeholder() {
        let action = Action::parse("x.toml", "name = \"X\"\n[prompt]\nsystem = \"s\"\n").unwrap();
        assert_eq!(action.render_user("hello"), "hello");
    }

    #[test]
    fn user_template_substitutes_every_occurrence() {
        let text = "name = \"X\"\n[prompt]\nsystem = \"s\"\nuser = \"a {{input}} b {{input}}\"\n";
        let action = Action::parse("x.toml", text).unwrap();
        assert_eq!(action.render_user("Z"), "a Z b Z");
    }

    /// Two rows and a default, so "inherit" has somewhere to come from and
    /// overriding the provider has somewhere to go.
    fn config() -> Config {
        let mut config = Config::default();
        config.api.providers.push(crate::config::Provider {
            id: "ollama".into(),
            label: "Ollama".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: "qwen3:8b".into(),
            thinking: true,
            ..Default::default()
        });
        config
    }

    #[test]
    fn action_model_overrides_win_over_the_provider_row() {
        let merged = ModelOverrides {
            provider: None,
            model: None,
            thinking: Some(true),
        }
        .merge_over(&config());
        assert_eq!(merged.provider, "deepseek");
        assert_eq!(merged.model, crate::config::DEFAULT_MODEL);
        assert!(merged.thinking);

        // Empty overrides are exactly the default row.
        let untouched = ModelOverrides::default().merge_over(&config());
        assert_eq!(untouched.provider, "deepseek");
        assert_eq!(untouched.model, crate::config::DEFAULT_MODEL);
        assert!(!untouched.thinking);
    }

    /// The chain that is the whole design: overriding `provider` moves what the
    /// two rows below it inherit (ADR-0021).
    #[test]
    fn overriding_the_provider_moves_what_model_and_thinking_inherit() {
        let merged = ModelOverrides {
            provider: Some("ollama".into()),
            model: None,
            thinking: None,
        }
        .merge_over(&config());
        assert_eq!(merged.provider, "ollama");
        assert_eq!(merged.model, "qwen3:8b");
        assert!(merged.thinking);
    }

    /// A model pinned before the provider changed is *kept*, never rewritten to
    /// something the new endpoint serves — the rule everywhere in this codebase.
    /// It goes on the wire and the endpoint's own error is the authority.
    #[test]
    fn a_pinned_model_survives_a_provider_override() {
        let merged = ModelOverrides {
            provider: Some("ollama".into()),
            model: Some("deepseek-v4-pro".into()),
            thinking: None,
        }
        .merge_over(&config());
        assert_eq!(merged.provider, "ollama");
        assert_eq!(merged.model, "deepseek-v4-pro");
    }

    /// A provider id naming no row invents nothing: the id travels as written so
    /// `turn.rs` can name it, and the overrides stand alone.
    #[test]
    fn an_unknown_provider_is_carried_rather_than_redirected() {
        let merged = ModelOverrides {
            provider: Some("deleted".into()),
            model: None,
            thinking: None,
        }
        .merge_over(&config());
        assert_eq!(merged.provider, "deleted");
        assert_eq!(merged.model, "");
    }

    #[test]
    fn slugs_display_names() {
        assert_eq!(slug("Quick ask"), "quick-ask");
        assert_eq!(slug("Translate  ZH <-> EN"), "translate-zh-en");
        assert_eq!(slug("---"), "action");
        assert_eq!(slug("翻译"), "action");
    }
}
