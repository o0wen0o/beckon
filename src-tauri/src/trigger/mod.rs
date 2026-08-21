//! The trigger flow: hotkey → grab → resolve `input_source` → show a window.
//!
//! Both paths share one grab, and the grab happens **before any window of ours
//! is shown** (needs ADR-0006): once the Launcher has focus, the foreground
//! window is Beckon, and the copy shortcut sent then would reach the wrong
//! process.
//!
//! This file is the flow and nothing else. [`window`] owns creating, sizing and
//! placing the three windows; [`foreground`] owns remembering and handing back
//! the window that had focus before us.

mod foreground;
mod window;

use tauri::{AppHandle, Emitter, Manager};

use crate::action::InputSource;
use crate::exchange;
use crate::platform;
use crate::state::{AppState, PopoverPhase, PopoverView};

use self::foreground::{remember_foreground, restore_foreground_if_idle};
use self::window::{center_on_active_monitor, reveal, size_and_place_at_cursor, POPOVER_HINT_H};
use self::window::{POPOVER_H, POPOVER_W};

pub const WINDOW_LAUNCHER: &str = "launcher";
pub const WINDOW_POPOVER: &str = "popover";
pub const WINDOW_SETTINGS: &str = "settings";

/// The Popover's view changed: re-read it with `get_popover_view`.
pub const EVENT_POPOVER_VIEW: &str = "popover:view";
/// The Launcher was summoned; payload says whether a Selection came with it.
pub const EVENT_LAUNCHER_OPENED: &str = "launcher:opened";
/// Settings was shown. The window is reused (ADR-0007), so a fresh open is an
/// event, not a mount — this is what clears the last visit's transient state.
pub const EVENT_SETTINGS_OPENED: &str = "settings:opened";

/// Global hotkey: toggle the Launcher, grabbing the Selection on the way up.
pub fn launcher_hotkey(app: &AppHandle) {
    let Some(window) = app.get_webview_window(WINDOW_LAUNCHER) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        hide_launcher(app);
        return;
    }

    remember_foreground(app);
    let selection = platform::selection::grab_selection();
    let selection_chars = selection.as_ref().map(|s| s.chars().count()).unwrap_or(0);
    {
        let state = app.state::<AppState>();
        *state.pending_selection.lock().expect("selection lock") = selection;
    }

    // Emit before revealing: the window is reused, so it is still showing the
    // last summon's query and match list until this lands.
    let _ = app.emit_to(
        WINDOW_LAUNCHER,
        EVENT_LAUNCHER_OPENED,
        serde_json::json!({ "selection_chars": selection_chars }),
    );
    center_on_active_monitor(&window);
    reveal(&window);
}

/// Direct Hotkey: straight to the result, zero interaction.
pub fn action_hotkey(app: &AppHandle, action_id: &str) {
    remember_foreground(app);
    let selection = platform::selection::grab_selection();
    open_action(app, action_id, selection);
}

/// Launcher pick: reuse the Selection grabbed when the Launcher opened — the
/// whole point of grabbing eagerly.
pub fn pick_from_launcher(app: &AppHandle, action_id: &str) {
    let state = app.state::<AppState>();
    let selection = state
        .pending_selection
        .lock()
        .expect("selection lock")
        .take();
    if let Some(window) = app.get_webview_window(WINDOW_LAUNCHER) {
        let _ = window.hide();
    }
    open_action(app, action_id, selection);
}

/// Resolve the Action's `input_source` against the grab and show the Popover.
///
/// Only one Popover exists. A trigger while one is open cancels the in-flight
/// request and replaces the contents — undefined in the docs, decided here.
pub fn open_action(app: &AppHandle, action_id: &str, selection: Option<String>) {
    let state = app.state::<AppState>();
    state.exchanges.discard_all();

    let (action, defaults) = {
        let registry = state.registry.read().expect("registry lock");
        let Some(action) = registry.get(action_id).cloned() else {
            log::warn!("hotkey fired for unknown Action \"{action_id}\"");
            return;
        };
        (action, state.config_snapshot().defaults)
    };

    let params = action.model_params(&defaults);
    let selection = selection.filter(|text| !text.trim().is_empty());

    let (phase, input) = match action.file.input_source {
        // An empty grab is not an error (ADR-0002): hint, and send nothing.
        InputSource::Selection => match selection {
            Some(text) => (PopoverPhase::Running, Some(text)),
            None => (PopoverPhase::EmptySelection, None),
        },
        // The grab is ignored — it was taken before the Action was known.
        InputSource::Prompt => (PopoverPhase::NeedsInput, None),
        InputSource::Auto => match selection {
            Some(text) => (PopoverPhase::Running, Some(text)),
            None => (PopoverPhase::NeedsInput, None),
        },
    };

    let mut view = PopoverView {
        action_id: action.id.clone(),
        action_name: action.file.name.clone(),
        model: params.clone(),
        phase,
        input: input.clone(),
        exchange_id: None,
    };

    if phase == PopoverPhase::Running {
        let input = input.unwrap_or_default();
        let exchange_id = state.exchanges.create(&action.file.prompt.system, params);
        view.exchange_id = Some(exchange_id.clone());
        if let Some(plan) = state
            .exchanges
            .begin_turn(&exchange_id, &action.render_user(&input))
        {
            exchange::spawn_turn(app.clone(), plan);
        }
    }

    *state.popover_view.lock().expect("popover view lock") = Some(view);

    // Emit before revealing. The window is reused and `load()` re-reads the
    // view asynchronously, so revealing first puts the *previous* Exchange's
    // answer on screen under the new Action's name for a few frames.
    let _ = app.emit_to(WINDOW_POPOVER, EVENT_POPOVER_VIEW, ());

    if let Some(window) = app.get_webview_window(WINDOW_POPOVER) {
        let height = if phase == PopoverPhase::EmptySelection {
            POPOVER_HINT_H
        } else {
            POPOVER_H
        };
        size_and_place_at_cursor(&window, POPOVER_W, height);
        reveal(&window);
    }
}

/// Start a turn for a Popover that was waiting for typed input.
pub fn submit_input(app: &AppHandle, text: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    let view = state
        .popover_view
        .lock()
        .expect("popover view lock")
        .clone()
        .ok_or_else(|| "there is no Action to send".to_string())?;

    let (action, defaults) = {
        let registry = state.registry.read().expect("registry lock");
        let action = registry
            .get(&view.action_id)
            .cloned()
            .ok_or_else(|| format!("the Action \"{}\" no longer exists", view.action_id))?;
        (action, state.config_snapshot().defaults)
    };

    let params = action.model_params(&defaults);
    let exchange_id = state
        .exchanges
        .create(&action.file.prompt.system, params.clone());
    let plan = state
        .exchanges
        .begin_turn(&exchange_id, &action.render_user(text))
        .ok_or_else(|| "the Exchange disappeared".to_string())?;

    {
        let mut slot = state.popover_view.lock().expect("popover view lock");
        if let Some(view) = slot.as_mut() {
            view.phase = PopoverPhase::Running;
            view.input = Some(text.to_string());
            view.exchange_id = Some(exchange_id.clone());
            view.model = params;
        }
    }

    exchange::spawn_turn(app.clone(), plan);
    Ok(exchange_id)
}

/// A follow-up turn inside the same Exchange (ADR-0004: full history resent).
pub fn follow_up(app: &AppHandle, exchange_id: &str, text: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let plan = state
        .exchanges
        .begin_turn(exchange_id, text)
        .ok_or_else(|| "this Exchange is gone; trigger the Action again".to_string())?;
    exchange::spawn_turn(app.clone(), plan);
    Ok(())
}

/// Esc or focus loss: cancel, drop the Exchange, hand focus back.
pub fn hide_popover(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        state.exchanges.discard_all();
        *state.popover_view.lock().expect("popover view lock") = None;
    }
    if let Some(window) = app.get_webview_window(WINDOW_POPOVER) {
        let _ = window.hide();
    }
    // The view is gone, so tell the window: it keeps rendering the Exchange it
    // last saw otherwise, and a hidden window that still holds an answer shows
    // it again for a few frames on the next trigger.
    let _ = app.emit_to(WINDOW_POPOVER, EVENT_POPOVER_VIEW, ());
    restore_foreground_if_idle(app);
}

pub fn hide_launcher(app: &AppHandle) {
    hide_launcher_window(app);
    restore_foreground_if_idle(app);
}

/// Hide the Launcher without deciding who gets the foreground next.
///
/// `show_settings` hides it on the way to a window that is also ours, where
/// handing the foreground back is exactly wrong.
fn hide_launcher_window(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        // The cached Selection dies with the Launcher; nothing keeps a copy.
        *state.pending_selection.lock().expect("selection lock") = None;
    }
    if let Some(window) = app.get_webview_window(WINDOW_LAUNCHER) {
        let _ = window.hide();
    }
}

/// Show Settings, building the window the first time it is asked for.
///
/// The whole flow runs on a spawned thread because of the first open:
/// `WebviewWindowBuilder::build` deadlocks on Windows when it is reached from
/// the main thread, and *every* caller is on it — a synchronous command from
/// the Launcher or the Popover, a tray menu event, a tray click. Everything
/// else here is dispatched to the event loop regardless, so moving it costs
/// nothing.
pub fn show_settings(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let window = match app.get_webview_window(WINDOW_SETTINGS) {
            Some(window) => window,
            None => match window::build_settings_window(&app) {
                Ok(window) => window,
                Err(err) => {
                    log::error!("could not create the Settings window: {err}");
                    return;
                }
            },
        };
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        // The Launcher is a picker on the way to somewhere; going to Settings
        // ends it. `hide_launcher` would restore the app Beckon was summoned
        // over and pull Settings straight behind it, so this is the one hide
        // that leaves the foreground alone.
        hide_launcher_window(&app);
        // Harmless on the very first open, when the webview is still loading and
        // nothing is listening: the window does its own load on mount.
        let _ = app.emit_to(WINDOW_SETTINGS, EVENT_SETTINGS_OPENED, ());
    });
}
