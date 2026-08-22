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

/// The one write path for `config.toml` (ADR-0003).
///
/// Marked in `SelfWrites` before the write so the watcher swallows its own
/// echo, then the one config-derived thing that is not markup is brought into
/// line, then [`reload_config`] re-reads what was written and broadcasts it.
/// Nothing downstream reads the `Config` handed in — the file is authoritative
/// the moment it lands.
///
/// Every writer goes through here rather than repeating the sequence: a step
/// added to it has to reach the resize path (ADR-0018) as well as Settings.
pub fn write_config(app: &AppHandle, config: &config::Config) -> Result<(), String> {
    let state = app.state::<AppState>();
    let previous = state.config_snapshot();
    let path = state.paths.config_file.clone();
    state.self_writes.mark(&path);
    config::save(&path, config)?;

    if config.autostart != previous.autostart {
        sync_autostart(app, config.autostart)?;
    }

    reload_config(app);
    Ok(())
}

pub fn reload_config(app: &AppHandle) {
    let state = app.state::<AppState>();
    let loaded = config::load_or_create(&state.paths.config_file);
    if let Some(error) = &loaded.error {
        log::warn!("{error}");
    }
    let language = loaded.config.language;
    let language_changed = state.config.read().expect("config lock").language != language;
    *state.config.write().expect("config lock") = loaded.config;

    // Neither the tray nor a title bar is markup, so no `config-changed` event
    // redraws either of them.
    if language_changed {
        tray::retranslate(app, language);
        crate::trigger::window::retitle_settings(app, language);
    }

    // The Launcher hotkey may have changed.
    apply_hotkeys(app);
    emit_config(app);

    // Every per-Action diagnostic — a file that will not parse, a Direct Hotkey
    // that lost its conflict — was phrased in the *previous* language. They are
    // derived state (ADR-0003), so re-deriving them is the whole fix.
    if language_changed {
        reload_actions(app);
    }
}

pub fn reload_actions(app: &AppHandle) {
    let state = app.state::<AppState>();
    let language = state.config.read().expect("config lock").language;
    let registry = Registry::load(&state.paths.actions_dir, language);
    *state.registry.write().expect("registry lock") = registry;

    // Direct Hotkeys are declared in the files, so they are re-derived too.
    apply_hotkeys(app);
    emit_actions(app);
}

/// Re-register every hotkey and reflect the outcome on the tray.
pub fn apply_hotkeys(app: &AppHandle) {
    let state = app.state::<AppState>();
    let language = state.config.read().expect("config lock").language;
    let report = hotkey::apply(app);
    if report.is_clean() {
        tray::set_normal(app);
    } else {
        tray::set_error(app, &report.summary(language));
    }
}

/// Autostart is config-derived state, so it is applied from inside the config
/// funnel rather than separately by each caller that changes the setting. Still
/// `pub` for startup, which applies the stored setting without writing one.
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
