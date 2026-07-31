//! Re-reading disk into state and telling the windows about it.
//!
//! ADR-0003 makes the filesystem authoritative, so every path that changes
//! config or Actions — the watcher, a Settings edit, startup — funnels through
//! here. The windows never patch their own copy; they re-render from the
//! snapshot this module broadcasts.

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::action::registry::{Registry, RegistrySnapshot};
use crate::config;
use crate::hotkey;
use crate::state::AppState;
use crate::tray;

pub const EVENT_ACTIONS_CHANGED: &str = "actions-changed";
pub const EVENT_CONFIG_CHANGED: &str = "config-changed";

pub fn reload_config(app: &AppHandle) {
    let state = app.state::<AppState>();
    let loaded = config::load_or_create(&state.paths.config_file);
    if let Some(error) = &loaded.error {
        log::warn!("{error}");
    }
    *state.config.write().expect("config lock") = loaded.config;

    // The Launcher hotkey may have changed.
    apply_hotkeys(app);
    emit_config(app);
}

pub fn reload_actions(app: &AppHandle) {
    let state = app.state::<AppState>();
    let registry = Registry::load(&state.paths.actions_dir);
    *state.registry.write().expect("registry lock") = registry;

    // Direct Hotkeys are declared in the files, so they are re-derived too.
    apply_hotkeys(app);
    emit_actions(app);
}

/// Re-register every hotkey and reflect the outcome on the tray.
pub fn apply_hotkeys(app: &AppHandle) {
    let report = hotkey::apply(app);
    if report.is_clean() {
        tray::set_normal(app);
    } else {
        tray::set_error(app, &report.summary());
    }
}

/// Autostart is config-derived state, so it is applied from beside the config
/// funnel rather than separately by each caller that changes the setting.
pub fn sync_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    // An unreadable current state is no reason to refuse; just apply the wanted
    // one. Skipping the redundant write keeps startup off the registry.
    if manager.is_enabled().is_ok_and(|current| current == enabled) {
        return Ok(());
    }
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

pub fn actions_snapshot(app: &AppHandle) -> RegistrySnapshot {
    let state = app.state::<AppState>();
    let hotkey_errors = {
        let hotkeys = state.hotkeys.lock().expect("hotkey lock");
        hotkeys.action_errors.clone()
    };
    let registry = state.registry.read().expect("registry lock");
    registry.snapshot(hotkey_errors)
}

/// One event carrying the whole registry snapshot (ADR-0003 consequence: the
/// frontend keeps no authoritative state, so partial updates would be a lie).
pub fn emit_actions(app: &AppHandle) {
    let _ = app.emit(EVENT_ACTIONS_CHANGED, actions_snapshot(app));
}

pub fn emit_config(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _ = app.emit(EVENT_CONFIG_CHANGED, state.config_snapshot());
}
