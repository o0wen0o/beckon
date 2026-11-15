//! Action commands. Identity is the filename, so every one of these
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
    // Probed only when the hotkey is what changed, the way `save_config` probes
    // the Launcher's. An Action whose Direct Hotkey already lost a conflict —
    // two files edited by hand to the same combination — must still be able to
    // save its prompt: the conflict is flagged on its row already, and refusing
    // every save until it is cleared would hold an unrelated edit hostage.
    let hotkey_before = state
        .registry
        .read()
        .expect("registry lock")
        .get(&parsed.id)
        .and_then(|action| action.file.hotkey.clone());
    if let Some(accelerator) = parsed.file.hotkey.as_deref().map(str::trim) {
        let unchanged = hotkey_before.as_deref().map(str::trim) == Some(accelerator);
        if !accelerator.is_empty() && !unchanged {
            hotkey::probe(
                &app,
                accelerator,
                &hotkey::Target::Action(parsed.id.clone()),
            )?;
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
///
/// The name is checked and handed back **as it came**, never tidied. Identity
/// is the filename, so a name that is trimmed on the way in is a different
/// Action on the way out: ` translate.toml` — legal on macOS — used to be saved
/// over `translate.toml`. And the test for "stays in this directory" is that
/// the name is one ordinary path component, not that it lacks the substring
/// `..`: `notes..v2.toml` is a file the loader lists, and it could be neither
/// saved, read nor deleted from Settings.
fn sanitize_file_name(file_name: &str) -> Result<String, String> {
    if file_name.trim().is_empty() {
        return Err("no file name given".to_string());
    }
    let invalid = || format!("\"{file_name}\" is not a valid Action file name");
    // Spelled out rather than left to `Path`: `\` and `:` are ordinary
    // characters in a path on macOS, and a name carrying either must be refused
    // there too — the directory is one the README says copies between platforms.
    if file_name.contains(['/', '\\', ':']) {
        return Err(invalid());
    }
    let mut components = std::path::Path::new(file_name).components();
    if !matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(_)), None)
    ) {
        return Err(invalid());
    }
    let Some(stem) = file_name.strip_suffix(".toml") else {
        return Err("an Action file name must end in .toml".to_string());
    };
    if crate::action::is_reserved_stem(stem) {
        return Err(invalid());
    }
    Ok(file_name.to_string())
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
        assert!(sanitize_file_name("..").is_err());
        assert!(sanitize_file_name("..toml").is_ok());
        assert_eq!(
            sanitize_file_name("translate.toml").unwrap(),
            "translate.toml"
        );
    }

    /// Identity is the filename, so the name that comes in is the name that is
    /// written: trimming one turns a save into a write over a different Action.
    #[test]
    fn a_name_is_never_tidied_into_another_one() {
        assert_eq!(
            sanitize_file_name(" translate.toml").unwrap(),
            " translate.toml"
        );
    }

    /// `..` as a substring is not a parent directory.
    #[test]
    fn dots_inside_a_name_are_only_dots() {
        assert_eq!(
            sanitize_file_name("notes..v2.toml").unwrap(),
            "notes..v2.toml"
        );
    }

    /// A device name is not a file on Windows whatever extension follows it,
    /// and the directory is meant to copy between platforms unchanged.
    #[test]
    fn rejects_windows_device_names() {
        assert!(sanitize_file_name("con.toml").is_err());
        assert!(sanitize_file_name("NUL.toml").is_err());
        assert!(sanitize_file_name("com1.toml").is_err());
        assert!(sanitize_file_name("console.toml").is_ok());
    }
}
