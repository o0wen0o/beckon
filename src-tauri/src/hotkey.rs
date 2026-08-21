//! Hotkey parsing and registration.
//!
//! Two kinds live here: the one global hotkey that summons the Launcher, and a
//! Direct Hotkey per Action that declares one. Registration failures are never
//! silent — they come back in [`ApplyReport`] and end up red in Settings or on
//! the tray icon (README).

use std::collections::HashMap;
use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::config::Language;
use crate::i18n;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Launcher,
    Action(String),
}

#[derive(Debug, Default)]
pub struct HotkeyState {
    /// What each currently-registered accelerator does.
    pub bindings: HashMap<Shortcut, Target>,
    /// Action id ⇒ why its Direct Hotkey is not active (conflict or refusal).
    pub action_errors: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct ApplyReport {
    pub launcher_error: Option<String>,
    pub action_errors: HashMap<String, String>,
}

impl ApplyReport {
    pub fn is_clean(&self) -> bool {
        self.launcher_error.is_none() && self.action_errors.is_empty()
    }

    /// One line per problem, for the tray balloon.
    pub fn summary(&self, language: Language) -> String {
        let mut lines = Vec::new();
        if let Some(error) = &self.launcher_error {
            lines.push(format!("{}: {error}", i18n::hotkey_launcher(language)));
        }
        let mut ids: Vec<&String> = self.action_errors.keys().collect();
        ids.sort();
        for id in ids {
            lines.push(format!("{id}: {}", self.action_errors[id]));
        }
        lines.join("\n")
    }
}

/// Parse the README's `"Ctrl+Shift+Space"` form.
///
/// A bare key with no modifier is rejected: registering `T` globally would
/// swallow the letter everywhere, which no user means to ask for.
///
/// `language` is carried in because every refusal here is read by the person
/// recording the hotkey: the wording lives in [`i18n`], the rule lives here.
pub fn parse(accelerator: &str, language: Language) -> Result<Shortcut, String> {
    let accelerator = accelerator.trim();
    if accelerator.is_empty() {
        return Err(i18n::hotkey_missing(language).to_string());
    }
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| i18n::hotkey_invalid(language, accelerator, &e.to_string()))?;
    if shortcut.mods.is_empty() {
        return Err(i18n::hotkey_needs_modifier(language, accelerator));
    }
    Ok(shortcut)
}

/// Try to register `accelerator` right now, then let it go again. This is what
/// makes the Settings recorder able to flag a taken hotkey on the spot (README).
pub fn probe(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let language = app.state::<AppState>().config_snapshot().language;
    let shortcut = parse(accelerator, language)?;

    // Already ours: registering it again would fail for the wrong reason.
    let already_ours = {
        let state = app.state::<AppState>();
        let hotkeys = state.hotkeys.lock().expect("hotkey lock");
        hotkeys.bindings.contains_key(&shortcut)
    };
    if already_ours {
        return Ok(());
    }

    let manager = app.global_shortcut();
    manager
        .register(shortcut)
        .map_err(|e| i18n::hotkey_not_registered(language, accelerator, &e.to_string()))?;
    let _ = manager.unregister(shortcut);
    Ok(())
}

/// Re-register everything from the current config and registry. Called at
/// startup and after every reload — registration is derived state, never
/// incrementally patched.
pub fn apply(app: &AppHandle) -> ApplyReport {
    let state = app.state::<AppState>();
    let config = state.config_snapshot();
    let language = config.language;
    let launcher_accelerator = config.launcher_hotkey;
    let plan = state
        .registry
        .read()
        .expect("registry lock")
        .hotkey_plan(language);

    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let mut report = ApplyReport {
        launcher_error: None,
        // Conflicts are decided before the OS is asked at all.
        action_errors: plan.conflicts,
    };
    let mut bindings: HashMap<Shortcut, Target> = HashMap::new();

    match parse(&launcher_accelerator, language) {
        Ok(shortcut) => match manager.register(shortcut) {
            Ok(()) => {
                bindings.insert(shortcut, Target::Launcher);
            }
            Err(err) => {
                report.launcher_error = Some(i18n::hotkey_not_registered(
                    language,
                    &launcher_accelerator,
                    &err.to_string(),
                ));
            }
        },
        Err(err) => report.launcher_error = Some(err),
    }

    for (accelerator, action_id) in plan.assignments {
        let shortcut = match parse(&accelerator, language) {
            Ok(shortcut) => shortcut,
            Err(err) => {
                report.action_errors.insert(action_id, err);
                continue;
            }
        };
        if let Some(existing) = bindings.get(&shortcut) {
            // Only the loser's message needs a display name, so it is looked up
            // here rather than mapped for every Action on every apply.
            let owner = match existing {
                Target::Launcher => i18n::hotkey_owner_launcher(language).to_string(),
                Target::Action(id) => {
                    let registry = state.registry.read().expect("registry lock");
                    let name = registry
                        .get(id)
                        .map_or(id.as_str(), |a| a.file.name.as_str());
                    format!("\"{name}\"")
                }
            };
            report.action_errors.insert(
                action_id,
                i18n::hotkey_taken(language, &accelerator, &owner),
            );
            continue;
        }
        match manager.register(shortcut) {
            Ok(()) => {
                bindings.insert(shortcut, Target::Action(action_id));
            }
            Err(err) => {
                report.action_errors.insert(
                    action_id,
                    i18n::hotkey_not_registered(language, &accelerator, &err.to_string()),
                );
            }
        }
    }

    {
        let mut hotkeys = state.hotkeys.lock().expect("hotkey lock");
        hotkeys.bindings = bindings;
        hotkeys.action_errors = report.action_errors.clone();
    }

    report
}

/// What a pressed accelerator means, or `None` if it is not ours any more.
pub fn target_for(app: &AppHandle, shortcut: &Shortcut) -> Option<Target> {
    let state = app.state::<AppState>();
    let hotkeys = state.hotkeys.lock().expect("hotkey lock");
    hotkeys.bindings.get(shortcut).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The language only decides how a refusal is *worded*; every test below is
    /// about whether the accelerator parses at all.
    fn parse(accelerator: &str) -> Result<Shortcut, String> {
        super::parse(accelerator, Language::En)
    }

    #[test]
    fn parses_the_readme_default() {
        let shortcut = parse("Ctrl+Shift+Space").unwrap();
        assert_eq!(shortcut, parse("  ctrl+shift+space  ").unwrap());
    }

    #[test]
    fn parses_a_direct_hotkey() {
        assert!(parse("Ctrl+Alt+T").is_ok());
        assert!(parse("Ctrl+Shift+F12").is_ok());
    }

    /// Every spelling of the Command/Super modifier parses on both platforms,
    /// which is what makes a `config.toml` portable between them (ADR-0013).
    #[test]
    fn every_spelling_of_the_command_modifier_parses() {
        let cmd = parse("Cmd+Shift+Space").unwrap();
        assert_eq!(cmd, parse("Command+Shift+Space").unwrap());
        assert_eq!(cmd, parse("Super+Shift+Space").unwrap());
    }

    /// Both platform defaults have to survive this function, or first run is
    /// the tray's error icon.
    #[test]
    fn the_shipped_default_registers_as_a_shortcut() {
        assert!(parse(crate::config::DEFAULT_LAUNCHER_HOTKEY).is_ok());
    }

    #[test]
    fn rejects_nonsense_and_modifier_less_keys() {
        assert!(parse("").is_err());
        assert!(parse("Ctrl+").is_err());
        assert!(parse("Banana+T").is_err());
        assert!(parse("T").is_err());
        assert!(parse("Space").is_err());
    }

    #[test]
    fn report_summarises_every_problem() {
        let mut report = ApplyReport {
            launcher_error: Some("taken".into()),
            action_errors: HashMap::new(),
        };
        report
            .action_errors
            .insert("translate".into(), "taken".into());
        assert!(!report.is_clean());
        assert_eq!(
            report.summary(Language::En),
            "Launcher hotkey: taken\ntranslate: taken"
        );
        // The same report, read by someone who set `language = "zh"`.
        assert_eq!(
            report.summary(Language::Zh),
            "启动器热键: taken\ntranslate: taken"
        );
    }
}
