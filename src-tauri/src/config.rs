//! `config.toml` load/save/merge.
//!
//! Every field has a default: a missing file means "write the defaults", a
//! missing field means "use the default", never an error.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;

/// The Launcher's out-of-the-box hotkey.
///
/// Space with two modifiers on both platforms; only the first modifier differs,
/// because the platform's own launcher does — Spotlight is Cmd+Space, so Cmd is
/// the key a Mac user reaches for and Ctrl is the Windows equivalent. macOS also
/// spends Ctrl+Space and Ctrl+Option+Space on input-source switching.
///
/// Neither default is a Ctrl+Alt chord: that is AltGr on every ISO keyboard, so
/// it doubles as a character-composing combination, and it is the modifier state
/// `selection::grab_selection` has to release before its copy can land at all.
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_LAUNCHER_HOTKEY: &str = "Ctrl+Shift+Space";
#[cfg(target_os = "macos")]
pub const DEFAULT_LAUNCHER_HOTKEY: &str = "Cmd+Shift+Space";

pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
pub const DEFAULT_MODEL: &str = "deepseek-v4-flash";
pub const DEFAULT_TEMPERATURE: f64 = 1.3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub launcher_hotkey: String,
    pub autostart: bool,
    /// Declared before the tables: `toml` serializes in field order and a
    /// scalar written after a table would land inside it.
    pub theme: Theme,
    pub api: ApiConfig,
    pub defaults: ModelDefaults,
}

/// Which palette the three surfaces paint in.
///
/// `Light` is the default, so an absent `theme` resolves to light like every
/// other missing field. `System` reads the OS appearance — but only once it
/// has been *chosen*: the OS preference never applies on its own, which is why
/// this is a three-valued setting rather than a bool plus a media query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub base_url: String,
}

/// Global model defaults. An Action's `[model]` table overrides these
/// field-by-field; see [`ModelOverrides::merge_over`](crate::action::ModelOverrides).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelDefaults {
    pub model: String,
    /// DeepSeek has thinking mode *on* by default, which is pure latency for
    /// translation-shaped Actions. Hence the default of `false` here.
    pub thinking: bool,
    pub temperature: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            launcher_hotkey: DEFAULT_LAUNCHER_HOTKEY.to_string(),
            autostart: true,
            theme: Theme::default(),
            api: ApiConfig::default(),
            defaults: ModelDefaults::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }
}

impl Default for ModelDefaults {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
            thinking: false,
            temperature: DEFAULT_TEMPERATURE,
        }
    }
}

/// The result of loading `config.toml`.
///
/// A corrupt config is *reported*, not overwritten — silently replacing a file
/// the user was editing is the data loss ADR-0003 warns about.
pub struct Loaded {
    pub config: Config,
    pub error: Option<String>,
}

pub fn load_or_create(path: &Path) -> Loaded {
    match fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<Config>(&text) {
            Ok(config) => Loaded {
                config,
                error: None,
            },
            Err(err) => Loaded {
                config: Config::default(),
                error: Some(format!("config.toml could not be parsed: {err}")),
            },
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            let error = save(path, &config)
                .err()
                .map(|e| format!("config.toml could not be written: {e}"));
            Loaded { config, error }
        }
        Err(err) => Loaded {
            config: Config::default(),
            error: Some(format!("config.toml could not be read: {err}")),
        },
    }
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    write_atomic(path, &text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_writes_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config, Config::default());
        assert!(path.exists(), "defaults must be persisted");

        // Round-trips through what we just wrote.
        assert_eq!(load_or_create(&path).config, Config::default());
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let parsed: Config = toml::from_str("autostart = false\n").unwrap();
        assert!(!parsed.autostart);
        assert_eq!(parsed.launcher_hotkey, DEFAULT_LAUNCHER_HOTKEY);
        assert_eq!(parsed.api.base_url, DEFAULT_BASE_URL);
        assert_eq!(parsed.defaults.model, DEFAULT_MODEL);
    }

    #[test]
    fn absent_theme_is_light() {
        assert_eq!(Config::default().theme, Theme::Light);
        assert_eq!(
            toml::from_str::<Config>("autostart = false\n")
                .unwrap()
                .theme,
            Theme::Light
        );
        // Not "whatever the OS is set to": `system` has to be asked for.
        assert_eq!(toml::from_str::<Config>("").unwrap().theme, Theme::Light);
    }

    #[test]
    fn every_theme_round_trips_through_toml() {
        for theme in [Theme::Light, Theme::Dark, Theme::System] {
            let config = Config {
                theme,
                ..Config::default()
            };
            let text = toml::to_string_pretty(&config).unwrap();
            assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        }
    }

    #[test]
    fn theme_is_written_as_a_lowercase_string() {
        let text = toml::to_string_pretty(&Config::default()).unwrap();
        assert!(text.contains("theme = \"light\""), "{text}");
    }

    #[test]
    fn theme_survives_a_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            theme: Theme::Dark,
            ..Config::default()
        };

        save(&path, &config).unwrap();
        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config.theme, Theme::Dark);
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        let parsed: Config =
            toml::from_str("launcher_hotkey = \"Ctrl+Alt+K\"\nfuture_option = 3\n").unwrap();
        assert_eq!(parsed.launcher_hotkey, "Ctrl+Alt+K");
    }

    /// The hotkey is interpolated rather than spelled out: it is the one
    /// default that differs per platform, and the rest of the example is what
    /// this test is actually about.
    #[test]
    fn readme_example_parses() {
        let text = format!(
            r#"
launcher_hotkey = "{DEFAULT_LAUNCHER_HOTKEY}"
autostart = true
theme = "light"

[api]
base_url = "https://api.deepseek.com"

[defaults]
model = "deepseek-v4-flash"
thinking = false
temperature = 1.3
"#
        );
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed, Config::default());
    }

    #[test]
    fn corrupt_config_reports_and_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "launcher_hotkey = ").unwrap();

        let loaded = load_or_create(&path);
        assert!(loaded.error.is_some());
        assert_eq!(loaded.config, Config::default());
        assert_eq!(fs::read_to_string(&path).unwrap(), "launcher_hotkey = ");
    }
}
