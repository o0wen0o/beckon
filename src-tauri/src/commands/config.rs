//! Config commands: read the snapshot, write it back through the one reload
//! funnel, and open the directory it lives in.

use std::fs;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::config::Config;
use crate::state::AppState;
use crate::{hotkey, reload};

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.config_snapshot()
}

/// Persist config. A Launcher hotkey that cannot be registered is **refused**,
/// not saved (README).
///
/// Validate and delegate, like every command: the probe is the refusal, and
/// `reload::write_config` is the one funnel — mark, write, re-read, broadcast.
#[tauri::command]
pub fn save_config(app: AppHandle, state: State<AppState>, config: Config) -> Result<(), String> {
    if config.launcher_hotkey != state.config_snapshot().launcher_hotkey {
        hotkey::probe(&app, &config.launcher_hotkey)?;
    }
    reload::write_config(&app, &config)
}

#[tauri::command]
pub fn reveal_config_dir(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let path = state.paths.root.clone();
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Open the page a DeepSeek key comes from.
///
/// The URL is a constant rather than an argument: a first run with no key and no
/// way to get one is a dead end, but "open whatever the webview asks for" is a
/// far larger surface than that gap is worth.
#[tauri::command]
pub fn open_api_key_page(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url("https://platform.deepseek.com/api_keys", None::<&str>)
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
