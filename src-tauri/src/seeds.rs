//! First-run example Actions.
//!
//! Written **only when `actions/` does not exist** (README). Once the user
//! deletes them they stay deleted — regenerating files someone removed on
//! purpose is the kind of "helpful" the README rules out.

use std::path::Path;

use crate::atomic::write_atomic;

pub const TRANSLATE: &str = r#"name = "Translate"
description = "Chinese <-> English"
input_source = "auto"
hotkey = "Ctrl+Alt+T"

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese.
Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
"""
"#;

pub const ASK: &str = r#"name = "Quick ask"
input_source = "prompt"
hotkey = "Ctrl+Alt+A"

[prompt]
system = "Answer concisely. Unless asked, do not enumerate bullet points and do not preamble at length."

[model]
thinking = true
"#;

/// Returns true when the examples were written.
pub fn seed_if_absent(actions_dir: &Path) -> Result<bool, String> {
    if actions_dir.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(actions_dir).map_err(|e| e.to_string())?;
    write_atomic(&actions_dir.join("translate.toml"), TRANSLATE).map_err(|e| e.to_string())?;
    write_atomic(&actions_dir.join("ask.toml"), ASK).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, InputSource};

    #[test]
    fn both_examples_parse_and_cover_both_paths() {
        let translate = Action::parse("translate.toml", TRANSLATE).unwrap();
        assert_eq!(translate.file.input_source, InputSource::Auto);
        assert_eq!(translate.file.hotkey.as_deref(), Some("Ctrl+Alt+T"));

        let ask = Action::parse("ask.toml", ASK).unwrap();
        assert_eq!(ask.file.input_source, InputSource::Prompt);
        assert_eq!(ask.file.hotkey.as_deref(), Some("Ctrl+Alt+A"));
        assert_eq!(ask.file.model.thinking, Some(true));

        // Both examples ship a Direct Hotkey, so they must not collide.
        assert_ne!(translate.file.hotkey, ask.file.hotkey);
    }

    #[test]
    fn seeds_once_and_never_again() {
        let dir = tempfile::tempdir().unwrap();
        let actions = dir.path().join("actions");

        assert!(seed_if_absent(&actions).unwrap());
        std::fs::remove_file(actions.join("ask.toml")).unwrap();

        // The directory still exists, so nothing is regenerated.
        assert!(!seed_if_absent(&actions).unwrap());
        assert!(!actions.join("ask.toml").exists());
    }
}
