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
mod models_cache;
mod platform;
mod reload;
mod secrets;
mod seeds;
mod state;
mod tray;
mod trigger;
mod update;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::action::registry::Registry;
use crate::action::watcher::{self, Change, Watched};
use crate::hotkey::Target;
use crate::secrets::KeyStatus;
use crate::state::{AppState, Paths};

/// Where `log::` goes: standard error, and nowhere else.
///
/// There was no logger at all, so every failure a comment in this crate says
/// "is logged" — a paste that could not be sent, a watcher that did not start,
/// a copy shortcut the OS refused — was a macro expanding to nothing. Under
/// `tauri dev` they are now on the terminal, which is where the next person
/// to chase one of them will be looking.
///
/// Not a file. Beckon keeps nothing of what passes through it, and a log
/// beside the config is a place a Selection could end up by one careless
/// `format!`. For the same reason the level stops at `Info`: the one `debug!`
/// in the crate quotes a stream frame, which is a piece of an answer.
struct StderrLog;

impl log::Log for StderrLog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            use std::io::Write;
            // A release build on Windows has no console; a write to a handle
            // that is not there must not be a panic, which `eprintln!` makes it.
            let _ = writeln!(
                std::io::stderr(),
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn main() {
    // First, so that what `load_state` has to say about the config is heard.
    if log::set_logger(&StderrLog).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }

    // Everything the windows can ask for is prepared *before* the builder runs.
    // Tauri creates the configured windows during `build()`, and a webview
    // starts loading — and can invoke a command — before `setup` is reached, so
    // state managed inside `setup` would be managed too late.
    let state = load_state();
    // `None` when the file did not load: the config in force is then the
    // defaults, and a default is not something the user asked for. Applying it
    // would switch autostart *on* for someone whose file says `false` beside a
    // typo — and fixing the typo would not switch it back off.
    let autostart_wanted = state
        .config_fault()
        .is_none()
        .then(|| state.config_snapshot().autostart);

    let app = tauri::Builder::default()
        .manage(state)
        // The first plugin on purpose. Beckon is one process: a
        // second launch reaches the running one, is told to open Settings, and
        // exits. Registered ahead of everything because plugin setups run in
        // order and `setup` runs after all of them, so a second instance dies
        // before it can claim a hotkey, add a second tray icon or start a
        // second watcher on the same files.
        //
        // The callback runs in *this* process, on the thread that pumps events
        // (a `WM_COPYDATA` handler on Windows). `show_settings` spawns, so the
        // pump is never blocked and `WebviewWindowBuilder::build` stays off the
        // main thread.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            trigger::show_settings(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::get_config_fault,
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
            commands::insert_answer,
            commands::open_link,
        ])
        .setup(move |app| setup(app, autostart_wanted))
        .on_window_event(|window, event| {
            match event {
                // Hiding, never closing: the windows are created once and reused.
                // The Exchange is never persisted, so it is dropped here,
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
                // and nothing in it is editable (that is Settings' job). The
                // Popover instead stays until Esc, so a
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
        // macOS is the exception, and deliberately so. Tauri fits
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
    let state = AppState::new(paths, loaded.config, registry);
    // Kept, not only logged: `reload::report_faults` puts it on the tray and
    // Settings draws it above every pane.
    *state.config_fault.lock().expect("config fault lock") = loaded.error;
    state
}

fn setup(
    app: &mut tauri::App,
    autostart_wanted: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // The filesystem is authoritative: external edits must reach the UI, so the watcher is not
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

    if let Some(wanted) = autostart_wanted {
        if let Err(err) = reload::sync_autostart(&handle, wanted) {
            log::warn!("could not apply the autostart setting: {err}");
        }
    }

    // The pre-provider credential moves onto the default row before anything
    // asks whether there is one, or an existing install would come back
    // reporting no key and yanking the user to Settings.
    // One snapshot for both questions: a `Config` clone carries the whole
    // provider table, and the row asked about is the same row either time.
    let config = handle.state::<AppState>().config_snapshot();
    let default_provider = config.defaults.provider.clone();
    if secrets::migrate_legacy(&default_provider) {
        log::info!("moved the stored API key onto the \"{default_provider}\" endpoint");
    }

    // First run is "no key readable", never a file check. A read
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

    // One quiet check per launch. Last, and on its own task: it is
    // network-bound, nothing else in `setup` waits on it, and a machine that is
    // offline at login must reach the end of this function anyway.
    update::check_on_startup(&handle);

    Ok(())
}
