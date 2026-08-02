//! Action commands. Identity is the filename (ADR-0003), so every one of these
//! takes a `file_name` — and every `file_name` arrives over IPC, hence
//! [`sanitize_file_name`].

use std::fs;

use tauri::{AppHandle, State};

use crate::action::registry::RegistrySnapshot;
use crate::action::{slug, Action, ActionFile};
use crate::atomic::write_atomic;
use crate::state::AppState;
use crate::{hotkey, reload};

#[tauri::command]
pub fn get_actions(app: AppHandle) -> RegistrySnapshot {
    reload::actions_snapshot(&app)
}

/// Save an edited Action. Identity is the filename, so `file_name` is what
/// decides *which* Action this is — renaming `name` never moves the file.
#[tauri::command]
pub fn save_action(
    app: AppHandle,
    state: State<AppState>,
    file_name: String,
    action: ActionFile,
) -> Result<(), String> {
    let file_name = sanitize_file_name(&file_name)?;
    let parsed = Action::from_parts(&file_name, action)?;
    if let Some(accelerator) = parsed.file.hotkey.as_deref().map(str::trim) {
        if !accelerator.is_empty() {
            hotkey::probe(&app, accelerator)?;
        }
    }

    let path = state.paths.actions_dir.join(&file_name);
    state.self_writes.mark(&path);
    write_atomic(&path, &parsed.to_toml()?).map_err(|e| e.to_string())?;

    reload::reload_actions(&app);
    Ok(())
}

/// Create a new Action file from a display name: slug it, de-duplicate with a
/// numeric suffix. Returns the file name, which is the new identity.
#[tauri::command]
pub fn create_action(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<String, String> {
    let display = if name.trim().is_empty() {
        "New Action".to_string()
    } else {
        name.trim().to_string()
    };
    let dir = state.paths.actions_dir.clone();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let stem = slug(&display);
    let mut file_name = format!("{stem}.toml");
    let mut suffix = 2;
    while dir.join(&file_name).exists() {
        file_name = format!("{stem}-{suffix}.toml");
        suffix += 1;
    }

    let action = Action::from_parts(
        &file_name,
        ActionFile {
            name: display,
            input_source: crate::action::InputSource::Auto,
            prompt: crate::action::PromptSpec {
                system: "You are a helpful assistant.".to_string(),
                user: None,
            },
            ..Default::default()
        },
    )?;

    let path = dir.join(&file_name);
    state.self_writes.mark(&path);
    write_atomic(&path, &action.to_toml()?).map_err(|e| e.to_string())?;

    reload::reload_actions(&app);
    Ok(file_name)
}

#[tauri::command]
pub fn delete_action(
    app: AppHandle,
    state: State<AppState>,
    file_name: String,
) -> Result<(), String> {
    let file_name = sanitize_file_name(&file_name)?;
    let path = state.paths.actions_dir.join(&file_name);
    state.self_writes.mark(&path);
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.to_string()),
    }
    reload::reload_actions(&app);
    Ok(())
}

/// The raw text of an Action file, so a file that fails to parse can still be
/// repaired in the Launcher instead of only being flagged red.
#[tauri::command]
pub fn read_action_raw(state: State<AppState>, file_name: String) -> Result<String, String> {
    let file_name = sanitize_file_name(&file_name)?;
    fs::read_to_string(state.paths.actions_dir.join(&file_name)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_action_raw(
    app: AppHandle,
    state: State<AppState>,
    file_name: String,
    text: String,
) -> Result<(), String> {
    let file_name = sanitize_file_name(&file_name)?;
    // Parse first: writing something known-broken helps nobody.
    Action::parse(&file_name, &text)?;

    let path = state.paths.actions_dir.join(&file_name);
    state.self_writes.mark(&path);
    write_atomic(&path, &text).map_err(|e| e.to_string())?;
    reload::reload_actions(&app);
    Ok(())
}

/// Keep names inside the Actions directory — a `file_name` arrives over IPC.
fn sanitize_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("no file name given".to_string());
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains(':')
    {
        return Err(format!("\"{trimmed}\" is not a valid Action file name"));
    }
    if !trimmed.ends_with(".toml") {
        return Err("an Action file name must end in .toml".to_string());
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_dressed_up_as_file_names() {
        assert!(sanitize_file_name("../config.toml").is_err());
        assert!(sanitize_file_name("sub/dir.toml").is_err());
        assert!(sanitize_file_name("C:\\evil.toml").is_err());
        assert!(sanitize_file_name("notes.txt").is_err());
        assert!(sanitize_file_name("  ").is_err());
        assert_eq!(
            sanitize_file_name(" translate.toml ").unwrap(),
            "translate.toml"
        );
    }
}
