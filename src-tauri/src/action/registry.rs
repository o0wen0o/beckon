//! Loading the Actions directory.
//!
//! ADR-0003: a file that fails to parse is skipped and reported, never fatal.
//! `load` therefore returns *both* the good Actions and the per-file errors.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use super::Action;
use crate::config::Language;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionError {
    pub file_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Registry {
    /// Sorted by file name, which also decides hotkey conflict precedence.
    pub actions: Vec<Action>,
    pub errors: Vec<ActionError>,
}

/// What the frontend receives. `hotkey_errors` is keyed by Action id and filled
/// in after registration, so a conflict or an OS refusal can be flagged red
/// next to the Action that lost.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RegistrySnapshot {
    pub actions: Vec<Action>,
    pub errors: Vec<ActionError>,
    pub hotkey_errors: HashMap<String, String>,
}

/// The result of resolving `hotkey` fields across the whole registry.
#[derive(Debug, Clone, Default)]
pub struct HotkeyPlan {
    /// `(accelerator, action id)` in filename order — what to register.
    pub assignments: Vec<(String, String)>,
    /// Action id ⇒ why its hotkey was not taken up.
    pub conflicts: HashMap<String, String>,
}

impl Registry {
    /// A missing directory is an empty registry, not an error: first run has no
    /// `actions/` yet, and seeding it is a separate decision (README).
    ///
    /// `language` is spent only on the errors: a file that cannot be read is
    /// reported rather than dropped (ADR-0003), and that report is read by a
    /// person in Settings.
    pub fn load(dir: &Path, language: Language) -> Self {
        let mut registry = Registry::default();

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return registry,
            Err(err) => {
                registry.errors.push(ActionError {
                    file_name: String::new(),
                    message: crate::i18n::actions_dir_unreadable(language, &err.to_string()),
                });
                return registry;
            }
        };

        // Editors' and our own temp files are skipped by the same rule the
        // watcher uses, so the two can never disagree.
        let mut files: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_file() && !super::watcher::is_ignored(&entry.path()))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();

        for file_name in files {
            let path = dir.join(&file_name);
            match fs::read_to_string(&path) {
                Ok(text) => match Action::parse(&file_name, &text) {
                    Ok(action) => registry.actions.push(action),
                    Err(message) => registry.errors.push(ActionError { file_name, message }),
                },
                Err(err) => registry.errors.push(ActionError {
                    file_name,
                    message: crate::i18n::action_file_unreadable(language, &err.to_string()),
                }),
            }
        }

        registry
    }

    pub fn get(&self, id: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.id == id)
    }

    pub fn snapshot(&self, hotkey_errors: HashMap<String, String>) -> RegistrySnapshot {
        RegistrySnapshot {
            actions: self.actions.clone(),
            errors: self.errors.clone(),
            hotkey_errors,
        }
    }

    /// Resolve Direct Hotkeys. Two Actions claiming the same accelerator: the
    /// first by filename wins, the loser is flagged — same "skip and flag"
    /// treatment ADR-0003 gives unparseable files.
    pub fn hotkey_plan(&self, language: Language) -> HotkeyPlan {
        let mut plan = HotkeyPlan::default();
        let mut claimed: HashMap<String, String> = HashMap::new();

        for action in &self.actions {
            let Some(hotkey) = action.file.hotkey.as_deref() else {
                continue;
            };
            let hotkey = hotkey.trim();
            if hotkey.is_empty() {
                continue;
            }
            let key = normalize_accelerator(hotkey);
            match claimed.get(&key) {
                Some(winner) => {
                    plan.conflicts.insert(
                        action.id.clone(),
                        crate::i18n::hotkey_claimed(language, hotkey, winner),
                    );
                }
                None => {
                    claimed.insert(key, action.id.clone());
                    plan.assignments
                        .push((hotkey.to_string(), action.id.clone()));
                }
            }
        }

        plan
    }
}

/// Case- and order-insensitive key for accelerator comparison, so
/// `Alt+Ctrl+T` and `ctrl+alt+t` are recognised as the same claim.
fn normalize_accelerator(accelerator: &str) -> String {
    let mut parts: Vec<String> = accelerator
        .split('+')
        .map(|p| p.trim().to_ascii_lowercase())
        .filter(|p| !p.is_empty())
        .collect();
    parts.sort();
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) {
        fs::write(dir.join(name), text).unwrap();
    }

    fn valid(name: &str) -> String {
        format!("name = \"{name}\"\n\n[prompt]\nsystem = \"s\"\n")
    }

    #[test]
    fn missing_directory_is_empty_not_an_error() {
        let registry = Registry::load(Path::new("does-not-exist-anywhere"), Language::En);
        assert!(registry.actions.is_empty());
        assert!(registry.errors.is_empty());
    }

    #[test]
    fn one_corrupt_file_does_not_stop_the_others() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.toml", &valid("A"));
        write(dir.path(), "b.toml", "name = \"B\"\n[prompt\n");
        write(dir.path(), "c.toml", &valid("C"));

        let registry = Registry::load(dir.path(), Language::En);
        let ids: Vec<&str> = registry.actions.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "c"]);
        assert_eq!(registry.errors.len(), 1);
        assert_eq!(registry.errors[0].file_name, "b.toml");
    }

    #[test]
    fn ignores_non_toml_and_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.toml", &valid("A"));
        write(dir.path(), "notes.txt", "hello");
        write(dir.path(), ".a.toml.beckon-tmp", "garbage");
        write(dir.path(), ".#a.toml", "garbage");

        let registry = Registry::load(dir.path(), Language::En);
        assert_eq!(registry.actions.len(), 1);
        assert!(registry.errors.is_empty());
    }

    #[test]
    fn hotkey_conflict_first_filename_wins() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "aaa.toml",
            "name = \"A\"\nhotkey = \"Ctrl+Alt+T\"\n[prompt]\nsystem = \"s\"\n",
        );
        write(
            dir.path(),
            "zzz.toml",
            "name = \"Z\"\nhotkey = \"alt+ctrl+t\"\n[prompt]\nsystem = \"s\"\n",
        );

        let plan = Registry::load(dir.path(), Language::En).hotkey_plan(Language::En);
        assert_eq!(
            plan.assignments,
            vec![("Ctrl+Alt+T".to_string(), "aaa".to_string())]
        );
        assert!(plan.conflicts.contains_key("zzz"));
    }

    #[test]
    fn actions_without_hotkeys_are_launcher_only() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.toml", &valid("A"));
        let plan = Registry::load(dir.path(), Language::En).hotkey_plan(Language::En);
        assert!(plan.assignments.is_empty());
        assert!(plan.conflicts.is_empty());
    }
}
