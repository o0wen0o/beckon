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
///
/// A provider row leaving the table takes its credential with it: the account is
/// `provider:{id}`, so a row deleted and later re-added under the same id would
/// otherwise silently inherit the old key (ADR-0021). Done here rather than in
/// `reload`, because it is a consequence of *this* edit and not of every re-read.
#[tauri::command]
pub fn save_config(
    app: AppHandle,
    state: State<AppState>,
    mut config: Config,
) -> Result<(), String> {
    // One snapshot for both comparisons below: a `Config` clone drags the whole
    // provider table with it, and neither read needs its own copy.
    let before = state.config_snapshot();
    if config.launcher_hotkey != before.launcher_hotkey {
        hotkey::probe(&app, &config.launcher_hotkey)?;
    }

    // The table's invariants are `fold_legacy`'s, so they hold at the *boundary*
    // and not merely on the way back in: a window is not where "never empty",
    // "ids are distinct" and "`defaults.provider` names a row" get enforced, and
    // the resize path writes config through this funnel too (ADR-0018).
    config.fold_legacy();

    let removed: Vec<String> = before
        .api
        .providers
        .iter()
        .filter(|before| {
            !config
                .api
                .providers
                .iter()
                .any(|after| after.id == before.id)
        })
        .map(|before| before.id.clone())
        .collect();

    reload::write_config(&app, &config)?;

    // After the write, and failures are logged rather than returned: the config
    // is already on disk, so refusing here would report a save that happened.
    for id in removed {
        if let Err(err) = crate::secrets::delete(&id) {
            log::warn!("could not remove the stored key for \"{id}\": {err}");
        }
    }
    Ok(())
}

#[tauri::command]
pub fn reveal_config_dir(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let path = state.paths.root.clone();
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// The filled-in rows "Add from preset" offers (ADR-0021).
///
/// A command rather than a table in the frontend: the value a preset exists to
/// carry is `reasoning`, which is a wire fact, and a wrong one is a 400 on every
/// turn. Rows already in the config are not filtered out here — which of them
/// the pane still offers is the pane's question.
#[tauri::command]
pub fn get_provider_presets() -> Vec<crate::config::Provider> {
    crate::config::presets()
}

#[tauri::command]
pub fn get_startup_errors(state: State<AppState>) -> Vec<String> {
    state
        .startup_errors
        .lock()
        .expect("startup errors lock")
        .clone()
}
