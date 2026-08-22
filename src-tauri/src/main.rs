// A tray app must not flash a console window on start.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod action;
mod atomic;
mod commands;
mod config;
mod exchange;
mod failure;
mod hotkey;
mod i18n;
mod llm;
mod platform;
mod reload;
mod secrets;
mod seeds;
mod state;
mod tray;
mod trigger;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::action::registry::Registry;
use crate::action::watcher::{self, Change, Watched};
use crate::hotkey::Target;
use crate::secrets::KeyStatus;
use crate::state::{AppState, Paths};

fn main() {
    // Everything the windows can ask for is prepared *before* the builder runs.
    // Tauri creates the configured windows during `build()`, and a webview
    // starts loading — and can invoke a command — before `setup` is reached, so
    // state managed inside `setup` would be managed too late.
    let state = load_state();
    let autostart_wanted = state.config_snapshot().autostart;

    let app = tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let Some(target) = hotkey::target_for(app, shortcut) else {
                        return;
                    };
                    // The grab sleeps for up to ~300ms polling the clipboard, so
                    // it must not run on the thread that pumps events.
                    let app = app.clone();
                    std::thread::spawn(move || match target {
                        Target::Launcher => trigger::launcher_hotkey(&app),
                        Target::Action(id) => trigger::action_hotkey(&app, &id),
                    });
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::reveal_config_dir,
            commands::get_provider_presets,
            commands::get_startup_errors,
            commands::get_actions,
            commands::save_action,
            commands::create_action,
            commands::delete_action,
            commands::read_action_raw,
            commands::write_action_raw,
            commands::get_key_statuses,
            commands::get_key_status,
            commands::open_key_page,
            commands::set_api_key,
            commands::delete_api_key,
            commands::test_connection,
            commands::get_models,
            commands::probe_hotkey,
            commands::get_input_permission,
            commands::open_input_permission_settings,
            commands::get_popover_view,
            commands::pick_action,
            commands::submit_input,
            commands::follow_up,
            commands::cancel_exchange,
            commands::retry_exchange,
            commands::start_capture,
            commands::discard_capture,
            commands::set_popover_size,
            commands::hide_popover,
            commands::hide_launcher,
            commands::show_settings,
            commands::copy_to_clipboard,
        ])
        .setup(move |app| setup(app, autostart_wanted))
        .on_window_event(|window, event| {
            match event {
                // Hiding, never closing: the windows are created once and reused
                // (ADR-0007). ADR-0004 is honoured by dropping the Exchange,
                // which `hide_popover` does.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    match window.label() {
                        trigger::WINDOW_POPOVER => trigger::hide_popover(window.app_handle()),
                        trigger::WINDOW_LAUNCHER => trigger::hide_launcher(window.app_handle()),
                        _ => {
                            let _ = window.hide();
                        }
                    }
                }
                // A Launcher that outlives its focus is a bug: it is a picker,
                // and nothing in it is editable (ADR-0003 puts that in
                // Settings). The Popover instead stays until Esc, so a
                // follow-up survives a glance at another app.
                WindowEvent::Focused(false) if window.label() == trigger::WINDOW_LAUNCHER => {
                    trigger::hide_launcher(window.app_handle());
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to start Beckon");

    app.run(|_app, event| {
        // No window is ever "the last window": Beckon lives in the tray and
        // quits only from the tray menu.
        //
        // macOS is the exception, and deliberately so (ADR-0013). Tauri fits
        // every macOS app with a default menu — which is the only reason Cmd+C
        // and Cmd+V work in Settings' text fields at all — and that menu owns
        // Cmd+Q. Every source of an exit request there is a person asking to
        // quit: Cmd+Q, the Dock, or logging out. There is no window-count exit
        // to guard against, because every window refuses to close.
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() && !cfg!(target_os = "macos") {
                api.prevent_exit();
            }
        }
    });
}

/// Everything a window can query, prepared before the first window exists:
/// seed the examples, read config and Actions, and build the state.
fn load_state() -> AppState {
    let paths = Paths::resolve().expect("could not locate the application-data directory");
    if let Err(err) = std::fs::create_dir_all(&paths.root) {
        // Without a config directory there is nothing to configure, but the
        // Settings window can still explain itself — so this is not fatal.
        log::warn!("could not create {}: {err}", paths.root.display());
    }

    // First run only, and never again after deletion (README).
    match seeds::seed_if_absent(&paths.actions_dir) {
        Ok(true) => log::info!("wrote the example Actions"),
        Ok(false) => {}
        Err(err) => log::warn!("could not write the example Actions: {err}"),
    }

    let loaded = config::load_or_create(&paths.config_file);
    if let Some(error) = &loaded.error {
        log::warn!("{error}");
    }
    let registry = Registry::load(&paths.actions_dir, loaded.config.language);
    AppState::new(paths, loaded.config, registry)
}

fn setup(app: &mut tauri::App, autostart_wanted: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Beckon lives in the menu bar, so it is an accessory: no Dock tile, no
    // Cmd+Tab entry, and — the part that matters — the Launcher can take focus
    // without a Dock icon bouncing into view first. This is the macOS half of
    // what `windows_subsystem = "windows"` does at the top of this file.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    let handle = app.handle().clone();
    let (paths, self_writes) = {
        let state = handle.state::<AppState>();
        (state.paths.clone(), state.self_writes.clone())
    };

    tray::build(&handle)?;

    // ADR-0003: external edits must reach the UI, so the watcher is not
    // optional — but a watcher that fails to start must not stop the app.
    let watcher_handle = handle.clone();
    match watcher::spawn(
        Watched {
            root: paths.root.clone(),
            config_file: paths.config_file.clone(),
            actions_dir: paths.actions_dir.clone(),
        },
        self_writes,
        move |change| match change {
            Change::Config => reload::reload_config(&watcher_handle),
            Change::Actions => reload::reload_actions(&watcher_handle),
        },
    ) {
        Ok(guard) => {
            let state = handle.state::<AppState>();
            *state.watcher.lock().expect("watcher lock") = Some(guard);
        }
        Err(err) => log::warn!("the config watcher could not start: {err}"),
    }

    // Startup registration failures are never silent: this switches the tray to
    // its error state and fires the one-time balloon (README).
    reload::apply_hotkeys(&handle);

    if let Err(err) = reload::sync_autostart(&handle, autostart_wanted) {
        log::warn!("could not apply the autostart setting: {err}");
    }

    // The pre-provider credential moves onto the default row before anything
    // asks whether there is one, or an existing install would come back
    // reporting no key and yanking the user to Settings (ADR-0021).
    // One snapshot for both questions: a `Config` clone carries the whole
    // provider table, and the row asked about is the same row either time.
    let config = handle.state::<AppState>().config_snapshot();
    let default_provider = config.defaults.provider.clone();
    if secrets::migrate_legacy(&default_provider) {
        log::info!("moved the stored API key onto the \"{default_provider}\" endpoint");
    }

    // First run is "no key readable", never a file check (ADR-0005). A read
    // error also lands here — the user has to be guided through it. Asked of the
    // *default* row only: an endpoint no Action has been pointed at yet is not
    // what makes this a first run, and a local one wants no key at all.
    let needs_key = match config.provider(None) {
        Some(row) if row.is_local() => false,
        Some(row) => !matches!(secrets::status(&row.id), KeyStatus::Present { .. }),
        None => true,
    };
    if needs_key {
        trigger::show_settings(&handle);
    }

    Ok(())
}
