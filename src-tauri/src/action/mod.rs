//! The Action: a preset prompt plus how it is triggered (ADR-0003).
//!
//! An Action's **identity is its filename stem**; `name` is display only. `id`
//! is therefore derived on load and never stored in the file — renaming the
//! display name must not change identity.

pub mod registry;
pub mod watcher;

use serde::{Deserialize, Serialize};

use crate::config::ModelDefaults;

/// What `prompt.user` defaults to when the file omits it.
pub const DEFAULT_USER_TEMPLATE: &str = "{{input}}";
/// The placeholder replaced by the Selection or the typed input.
pub const INPUT_PLACEHOLDER: &str = "{{input}}";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputSource {
    /// Selection only. An empty grab shows a hint and sends nothing.
    Selection,
    /// Typed input only; any Selection is ignored.
    Prompt,
    /// Selection if there is one, otherwise ask for typed input. The default
    /// because it degrades to an input box instead of a hint.
    #[default]
    Auto,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptSpec {
    pub system: String,
    /// Omitted in the file ⇒ [`DEFAULT_USER_TEMPLATE`].
    pub user: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelOverrides {
    pub model: Option<String>,
    pub thinking: Option<bool>,
    pub temperature: Option<f64>,
}

/// Effective per-request model parameters: the Action's `[model]` table laid
/// over `config.defaults`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelParams {
    pub model: String,
    pub thinking: bool,
    pub temperature: f64,
}

impl ModelOverrides {
    /// Field-by-field override. This is the one merge function in the codebase.
    pub fn merge_over(&self, defaults: &ModelDefaults) -> ModelParams {
        ModelParams {
            model: self.model.clone().unwrap_or_else(|| defaults.model.clone()),
            thinking: self.thinking.unwrap_or(defaults.thinking),
            temperature: self.temperature.unwrap_or(defaults.temperature),
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

    pub fn model_params(&self, defaults: &ModelDefaults) -> ModelParams {
        self.file.model.merge_over(defaults)
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
input_source = "selection"
hotkey = "Ctrl+Alt+T"

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese.
Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
"""

[model]
temperature = 1.3
"#;

    #[test]
    fn parses_readme_example() {
        let action = Action::parse("translate.toml", TRANSLATE).unwrap();
        assert_eq!(action.id, "translate");
        assert_eq!(action.file.name, "Translate");
        assert_eq!(action.file.input_source, InputSource::Selection);
        assert_eq!(action.file.hotkey.as_deref(), Some("Ctrl+Alt+T"));
        assert_eq!(action.file.model.temperature, Some(1.3));
        assert_eq!(action.file.model.thinking, None);
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

    #[test]
    fn input_source_defaults_to_auto() {
        let action = Action::parse("x.toml", "name = \"X\"\n[prompt]\nsystem = \"s\"\n").unwrap();
        assert_eq!(action.file.input_source, InputSource::Auto);
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

    #[test]
    fn action_model_overrides_win_over_defaults() {
        let defaults = ModelDefaults {
            model: "deepseek-v4-flash".into(),
            thinking: false,
            temperature: 1.3,
        };
        let overrides = ModelOverrides {
            model: None,
            thinking: Some(true),
            temperature: Some(0.2),
        };
        let merged = overrides.merge_over(&defaults);
        assert_eq!(merged.model, "deepseek-v4-flash");
        assert!(merged.thinking);
        assert_eq!(merged.temperature, 0.2);

        // Empty overrides are exactly the defaults.
        let untouched = ModelOverrides::default().merge_over(&defaults);
        assert_eq!(untouched.model, defaults.model);
        assert_eq!(untouched.thinking, defaults.thinking);
        assert_eq!(untouched.temperature, defaults.temperature);
    }

    #[test]
    fn slugs_display_names() {
        assert_eq!(slug("Quick ask"), "quick-ask");
        assert_eq!(slug("Translate  ZH <-> EN"), "translate-zh-en");
        assert_eq!(slug("---"), "action");
        assert_eq!(slug("翻译"), "action");
    }
}
