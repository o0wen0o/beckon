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

/// The Popover's size out of the box, in logical pixels (ADR-0018). Mirrored in
/// `tauri.conf.json` so the very first paint is not at the wrong size.
pub const DEFAULT_POPOVER_W: f64 = 620.0;
pub const DEFAULT_POPOVER_H: f64 = 500.0;
/// The floors, mirrored in `tauri.conf.json` as `minWidth`/`minHeight` so the
/// window manager refuses a smaller drag rather than us undoing one afterwards.
///
/// The width floor is the composer's row — camera, box, Send — which wraps
/// below it. The height floor is a composer plus one line of answer: with the
/// hint window gone (ADR-0020) the product no longer sizes the Popover itself,
/// so the floor only has to keep a hand-drag readable.
pub const MIN_POPOVER_W: f64 = 380.0;
pub const MIN_POPOVER_H: f64 = 200.0;
/// The ceilings exist only to keep a garbled value out of the file; a 4K panel
/// is the largest thing a window can usefully be dragged to.
pub const MAX_POPOVER_W: f64 = 3840.0;
pub const MAX_POPOVER_H: f64 = 2160.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub launcher_hotkey: String,
    pub autostart: bool,
    /// Declared before the tables: `toml` serializes in field order and a
    /// scalar written after a table would land inside it.
    pub theme: Theme,
    pub language: Language,
    pub api: ApiConfig,
    pub defaults: ModelDefaults,
    pub popover: PopoverSize,
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

/// Which language the three surfaces are written in.
///
/// `En` is the default, so an absent `language` resolves to English like every
/// other missing field. There is no `system` arm the way [`Theme`] has one: the
/// OS locale is a guess about a *reader*, not a setting, and a wrong guess here
/// replaces every word in the product rather than its palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Zh,
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
}

/// The size the Popover is summoned at, in logical pixels (ADR-0018).
///
/// It is config rather than window state because the window is created hidden at
/// startup and re-sized on every trigger (ADR-0007): a size that lived only in
/// the window would be overwritten by the next summon, so remembering a drag at
/// all means writing it to the file ADR-0003 makes authoritative.
///
/// Logical, not physical: the same file has to mean the same window on a 100%
/// monitor and a 150% one, and it is what `set_size` takes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PopoverSize {
    pub width: f64,
    pub height: f64,
}

impl PopoverSize {
    /// Clamped rather than refused: the value arrives from a drag, and a window
    /// dragged to the edge of the plausible is not an error to report. A `NaN`
    /// loses both comparisons, so it falls back to the default instead of
    /// poisoning the placement arithmetic.
    pub fn clamped(self) -> Self {
        Self {
            width: clamp_or_default(self.width, MIN_POPOVER_W, MAX_POPOVER_W, DEFAULT_POPOVER_W),
            height: clamp_or_default(self.height, MIN_POPOVER_H, MAX_POPOVER_H, DEFAULT_POPOVER_H),
        }
    }
}

fn clamp_or_default(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if !value.is_finite() {
        return fallback;
    }
    value.clamp(min, max)
}

impl Default for PopoverSize {
    fn default() -> Self {
        Self {
            width: DEFAULT_POPOVER_W,
            height: DEFAULT_POPOVER_H,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            launcher_hotkey: DEFAULT_LAUNCHER_HOTKEY.to_string(),
            autostart: true,
            theme: Theme::default(),
            language: Language::default(),
            api: ApiConfig::default(),
            defaults: ModelDefaults::default(),
            popover: PopoverSize::default(),
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
            // The one field clamped on the way in rather than reported: it is
            // written by a drag, not typed, so a value out of range is this
            // program's own bug or a hand-edit — and either way the window has
            // to be some size. The file is left as the user left it.
            Ok(mut config) => {
                config.popover = config.popover.clamped();
                Loaded {
                    config,
                    error: None,
                }
            }
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
    fn absent_language_is_english() {
        assert_eq!(Config::default().language, Language::En);
        assert_eq!(toml::from_str::<Config>("").unwrap().language, Language::En);
        // Not the OS locale: Chinese has to be asked for.
        assert_eq!(
            toml::from_str::<Config>("language = \"zh\"\n")
                .unwrap()
                .language,
            Language::Zh
        );
    }

    #[test]
    fn language_survives_a_toml_round_trip() {
        for language in [Language::En, Language::Zh] {
            let config = Config {
                language,
                ..Config::default()
            };
            let text = toml::to_string_pretty(&config).unwrap();
            assert!(!text.contains("[api]\nlanguage"), "{text}");
            assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        }
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
    fn absent_popover_size_is_the_default() {
        let parsed: Config = toml::from_str("").unwrap();
        assert_eq!(parsed.popover.width, DEFAULT_POPOVER_W);
        assert_eq!(parsed.popover.height, DEFAULT_POPOVER_H);
    }

    /// One dimension named is one dimension changed: the container-level
    /// `serde(default)` fills the other from [`PopoverSize::default`], not from
    /// `f64::default`, which would be a zero-width window.
    #[test]
    fn half_a_popover_size_keeps_the_other_default() {
        let parsed: Config = toml::from_str("[popover]\nwidth = 900\n").unwrap();
        assert_eq!(parsed.popover.width, 900.0);
        assert_eq!(parsed.popover.height, DEFAULT_POPOVER_H);
    }

    #[test]
    fn popover_size_survives_a_toml_round_trip() {
        let config = Config {
            popover: PopoverSize {
                width: 900.0,
                height: 700.0,
            },
            ..Config::default()
        };
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }

    #[test]
    fn an_out_of_range_popover_size_is_clamped_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[popover]\nwidth = 12\nheight = 99999\n").unwrap();

        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config.popover.width, MIN_POPOVER_W);
        assert_eq!(loaded.config.popover.height, MAX_POPOVER_H);
        // Clamping is ours, not a rewrite of the user's file (ADR-0003).
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[popover]\nwidth = 12\nheight = 99999\n"
        );
    }

    #[test]
    fn a_nonsense_popover_size_falls_back_to_the_default() {
        let clamped = PopoverSize {
            width: f64::NAN,
            height: f64::INFINITY,
        }
        .clamped();
        assert_eq!(clamped, PopoverSize::default());
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
language = "en"

[api]
base_url = "https://api.deepseek.com"

[defaults]
model = "deepseek-v4-flash"
thinking = false

[popover]
width = 620.0
height = 500.0
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
