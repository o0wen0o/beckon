//! Re-reading disk into state and telling the windows about it.
//!
//! The filesystem is authoritative, so every path that changes
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

/// The one write path for `config.toml`.
///
/// Marked in `SelfWrites` before the write so the watcher swallows its own
/// echo, then the one config-derived thing that is not markup is brought into
/// line, then [`reload_config`] re-reads what was written and broadcasts it.
/// Nothing downstream reads the `Config` handed in — the file is authoritative
/// the moment it lands.
///
/// Every writer goes through here rather than repeating the sequence: a step
/// added to it has to reach the resize path as well as Settings.
pub fn write_config(app: &AppHandle, config: &config::Config) -> Result<(), String> {
    let state = app.state::<AppState>();
    let previous = state.config_snapshot();
    let path = state.paths.config_file.clone();
    // Before the write, because the write is what would destroy it: a file that
    // does not parse loaded as defaults (`config::load_or_create`), so `config`
    // here is those defaults plus one change, about to replace everything the
    // user typed. The copy is not a `.toml`, so the watcher never sees it.
    if let Some(kept) = config::set_aside_if_unparsable(&path) {
        log::warn!(
            "config.toml did not parse; kept the old file as {}",
            kept.display()
        );
    }
    state.self_writes.mark(&path);
    config::save(&path, config)?;

    // Applied, then reported *after* the reload. The file is already written
    // and already marked as ours, so returning here would leave `AppState` on
    // the old config with no watcher echo coming to correct it — disk and
    // memory disagreeing until some unrelated reload.
    let autostart = if config.autostart != previous.autostart {
        sync_autostart(app, config.autostart)
    } else {
        Ok(())
    };

    reload_config(app);
    autostart
}

pub fn reload_config(app: &AppHandle) {
    let state = app.state::<AppState>();
    let loaded = config::load_or_create(&state.paths.config_file);
    if let Some(error) = &loaded.error {
        log::warn!("{error}");
    }
    // Stored before anything is re-derived from the config, so `apply_hotkeys`
    // below reports this load and not the one before it.
    let loads = loaded.error.is_none();
    let was_broken = std::mem::replace(
        &mut *state.config_fault.lock().expect("config fault lock"),
        loaded.error,
    )
    .is_some();
    let language = loaded.config.language;
    let autostart = loaded.config.autostart;
    let (language_changed, autostart_changed) = {
        let current = state.config.read().expect("config lock");
        (current.language != language, current.autostart != autostart)
    };
    *state.config.write().expect("config lock") = loaded.config;

    // Autostart is derived from the file whoever wrote it, so a hand-edit has
    // to reach the OS the way a Settings toggle does. Only from a file that
    // loaded — the defaults a broken one falls back to are not a request — and
    // always on the load that *repairs* one, because the value in force until
    // now was a default the comparison above cannot tell from the user's own.
    // After `write_config` this is a no-op: it has already applied the same
    // value, and `sync_autostart` asks before it writes.
    if loads && (autostart_changed || was_broken) {
        if let Err(err) = sync_autostart(app, autostart) {
            log::warn!("could not apply the autostart setting: {err}");
        }
    }

    // Neither the tray nor a title bar is markup, so no `config-changed` event
    // redraws either of them.
    if language_changed {
        tray::rebuild(app);
        crate::trigger::window::retitle_settings(app, language);
    }

    // The Launcher hotkey may have changed.
    apply_hotkeys(app);
    emit_config(app);

    // Every per-Action diagnostic — a file that will not parse, a Direct Hotkey
    // that lost its conflict — was phrased in the *previous* language. They are
    // derived state, so re-deriving them is the whole fix.
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
///
/// The tray has one error state and two things that can put it there: a hotkey
/// that did not register, and a `config.toml` that did not load. Both are
/// decided here because this is the one function every reload ends in — the
/// config fault is read rather than passed, so an Action reload that happens
/// while the file is still broken cannot paint the icon normal over it.
pub fn apply_hotkeys(app: &AppHandle) {
    let state = app.state::<AppState>();
    let language = state.config.read().expect("config lock").language;
    let report = hotkey::apply(app);
    let hotkeys = (!report.is_clean()).then(|| report.summary(language));
    let config = state.config_fault();
    if hotkeys.is_none() && config.is_none() {
        tray::set_normal(app);
    } else {
        tray::set_error(app, hotkeys.as_deref(), config.as_deref());
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

/// One event carrying the whole registry snapshot (the frontend keeps no
/// authoritative state, so partial updates would be a lie).
pub fn emit_actions(app: &AppHandle) {
    let _ = app.emit(EVENT_ACTIONS_CHANGED, actions_snapshot(app));
}

pub fn emit_config(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _ = app.emit(EVENT_CONFIG_CHANGED, state.config_snapshot());
}
