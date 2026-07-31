//! The IPC surface. Thin on purpose: every command validates, delegates, and
//! lets the reload path broadcast the result.

use std::fs;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::action::registry::RegistrySnapshot;
use crate::action::{slug, Action, ActionFile};
use crate::atomic::write_atomic;
use crate::config::Config;
use crate::llm::client::{self, LlmError};
use crate::secrets::{self, KeyStatus};
use crate::state::{AppState, PopoverView};
use crate::{hotkey, platform, reload, trigger};

/// An error the UI has to react to differently depending on cause, rather than
/// just print.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub kind: String,
    pub message: String,
}

impl Failure {
    fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
        }
    }
}

impl From<LlmError> for Failure {
    fn from(error: LlmError) -> Self {
        Failure::new(error.kind(), error.to_string())
    }
}

// ---------------------------------------------------------------- config

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.config_snapshot()
}

/// Persist config. A Launcher hotkey that cannot be registered is **refused**,
/// not saved (README).
#[tauri::command]
pub fn save_config(app: AppHandle, state: State<AppState>, config: Config) -> Result<(), String> {
    let previous = state.config_snapshot();
    if config.launcher_hotkey != previous.launcher_hotkey {
        hotkey::probe(&app, &config.launcher_hotkey)?;
    }

    let path = state.paths.config_file.clone();
    state.self_writes.mark(&path);
    crate::config::save(&path, &config)?;
    *state.config.write().expect("config lock") = config.clone();

    if config.autostart != previous.autostart {
        sync_autostart(&app, config.autostart)?;
    }

    reload::apply_hotkeys(&app);
    reload::emit_config(&app);
    Ok(())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<(), String> {
    let mut config = state.config_snapshot();
    config.autostart = enabled;
    save_config(app, state, config)
}

fn sync_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn reveal_config_dir(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let path = state.paths.root.clone();
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_startup_errors(state: State<AppState>) -> Vec<String> {
    state
        .startup_errors
        .lock()
        .expect("startup errors lock")
        .clone()
}

// ---------------------------------------------------------------- actions

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
/// repaired in Settings instead of only being flagged red.
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

// ---------------------------------------------------------------- secrets

#[tauri::command]
pub fn get_key_status() -> KeyStatus {
    secrets::status()
}

#[tauri::command]
pub fn set_api_key(key: String) -> Result<KeyStatus, String> {
    secrets::write(&key)?;
    Ok(secrets::status())
}

#[tauri::command]
pub fn delete_api_key() -> Result<KeyStatus, String> {
    secrets::delete()?;
    Ok(secrets::status())
}

/// "Test connection": one minimal request, reporting a rejected key separately
/// from an unreachable API (ADR-0005).
#[tauri::command]
pub async fn test_connection(app: AppHandle) -> Result<(), Failure> {
    let (base_url, model, http) = {
        let state = app.state::<AppState>();
        let config = state.config_snapshot();
        (
            config.api.base_url,
            config.defaults.model,
            state.http.clone(),
        )
    };

    let key = match secrets::read() {
        Ok(Some(key)) => key,
        Ok(None) => {
            return Err(Failure::new(
                "no-credential",
                "No API key is stored yet. Enter one above, then test again.",
            ))
        }
        Err(message) => {
            return Err(Failure::new(
                "read-error",
                format!("The Credential Manager could not be read: {message}"),
            ))
        }
    };

    client::test_connection(&http, &base_url, &key, &model)
        .await
        .map_err(Failure::from)
}

// ---------------------------------------------------------------- hotkeys

/// Register `accelerator` immediately to prove it is free, then release it.
#[tauri::command]
pub fn probe_hotkey(app: AppHandle, accelerator: String) -> Result<(), String> {
    hotkey::probe(&app, &accelerator)
}

// ---------------------------------------------------------------- windows

#[tauri::command]
pub fn get_popover_view(state: State<AppState>) -> Option<PopoverView> {
    state
        .popover_view
        .lock()
        .expect("popover view lock")
        .clone()
}

#[tauri::command]
pub fn pick_action(app: AppHandle, action_id: String) {
    trigger::pick_from_launcher(&app, &action_id);
}

#[tauri::command]
pub fn submit_input(app: AppHandle, text: String) -> Result<String, String> {
    trigger::submit_input(&app, &text)
}

#[tauri::command]
pub fn follow_up(app: AppHandle, exchange_id: String, text: String) -> Result<(), String> {
    trigger::follow_up(&app, &exchange_id, &text)
}

/// Esc during a request (README): cancel, keep the window open.
#[tauri::command]
pub fn cancel_exchange(state: State<AppState>, exchange_id: String) {
    state.exchanges.cancel(&exchange_id);
}

/// Retry after an error: same input, new turn.
#[tauri::command]
pub fn retry_exchange(app: AppHandle, exchange_id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let plan = {
        let exchange = state
            .exchanges
            .get(&exchange_id)
            .ok_or_else(|| "this Exchange is gone; trigger the Action again".to_string())?;
        // The last user message is what failed; resend it untouched.
        let last_user = exchange
            .messages
            .iter()
            .rev()
            .find(|m| m.role == crate::llm::Role::User)
            .map(|m| m.content.clone())
            .ok_or_else(|| "there is nothing to retry".to_string())?;
        state.exchanges.begin_turn(&exchange_id, &last_user)
    }
    .ok_or_else(|| "this Exchange is gone; trigger the Action again".to_string())?;

    crate::exchange::spawn_turn(app.clone(), plan);
    Ok(())
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    trigger::hide_popover(&app);
}

#[tauri::command]
pub fn hide_launcher(app: AppHandle) {
    trigger::hide_launcher(&app);
}

#[tauri::command]
pub fn show_settings(app: AppHandle) {
    trigger::show_settings(&app);
}

/// The Popover's Copy button. A user-requested write, so it is *not* restored
/// (ADR-0002) — this is the only way a result leaves Beckon.
#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    platform::selection::write_clipboard_text(&text)
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
