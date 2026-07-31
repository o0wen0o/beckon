//! The trigger flow: hotkey → grab → resolve `input_source` → show a window.
//!
//! Both paths share one grab, and the grab happens **before any window of ours
//! is shown** (needs ADR-0006): once the Launcher has focus, the foreground
//! window is Beckon, and a Ctrl+C sent then would copy from the wrong process.
//!
//! The two hot-path windows — Launcher and Popover — are created hidden at
//! startup and only shown/hidden here (ADR-0007): WebView creation costs far
//! too much to pay per keypress. ADR-0004 is satisfied by destroying the
//! *Exchange* on hide, which is what [`hide_popover`] does.
//!
//! Settings is the exception: nothing about it is latency-sensitive, and a third
//! live WebView is the most expensive thing in a resident tool. It is built on
//! first use and kept afterwards.

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow};

use crate::action::InputSource;
use crate::exchange;
use crate::platform;
use crate::state::{AppState, PopoverPhase, PopoverView};

pub const WINDOW_LAUNCHER: &str = "launcher";
pub const WINDOW_POPOVER: &str = "popover";
pub const WINDOW_SETTINGS: &str = "settings";

/// The Popover's view changed: re-read it with `get_popover_view`.
pub const EVENT_POPOVER_VIEW: &str = "popover:view";
/// The Launcher was summoned; payload says whether a Selection came with it.
pub const EVENT_LAUNCHER_OPENED: &str = "launcher:opened";

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

    center_on_active_monitor(&window);
    reveal(&window);
    let _ = app.emit_to(
        WINDOW_LAUNCHER,
        EVENT_LAUNCHER_OPENED,
        serde_json::json!({ "selection_chars": selection_chars }),
    );
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

    if let Some(window) = app.get_webview_window(WINDOW_POPOVER) {
        place_at_cursor(&window);
        reveal(&window);
    }
    let _ = app.emit_to(WINDOW_POPOVER, EVENT_POPOVER_VIEW, ());
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
    restore_foreground_if_idle(app);
}

pub fn hide_launcher(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        // The cached Selection dies with the Launcher; nothing keeps a copy.
        *state.pending_selection.lock().expect("selection lock") = None;
    }
    if let Some(window) = app.get_webview_window(WINDOW_LAUNCHER) {
        let _ = window.hide();
    }
    restore_foreground_if_idle(app);
}

/// Show Settings, building the window the first time it is asked for.
pub fn show_settings(app: &AppHandle) {
    let window = match app.get_webview_window(WINDOW_SETTINGS) {
        Some(window) => window,
        None => match build_settings_window(app) {
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
}

fn build_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    tauri::WebviewWindowBuilder::new(
        app,
        WINDOW_SETTINGS,
        WebviewUrl::App("settings.html".into()),
    )
    .title("Beckon Settings")
    .inner_size(940.0, 720.0)
    .min_inner_size(720.0, 520.0)
    .center()
    .resizable(true)
    .visible(false)
    .build()
}

fn reveal(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

fn remember_foreground(app: &AppHandle) {
    let Some(hwnd) = platform::focus::foreground_window() else {
        return;
    };
    // Do not remember one of our own windows: closing the Popover would then
    // "restore" focus to the Launcher we just hid.
    if is_ours(app, hwnd) {
        return;
    }
    let state = app.state::<AppState>();
    *state.previous_foreground.lock().expect("foreground lock") = Some(hwnd);
}

fn is_ours(app: &AppHandle, hwnd: isize) -> bool {
    [WINDOW_LAUNCHER, WINDOW_POPOVER, WINDOW_SETTINGS]
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .any(|window| platform::focus::window_handle(&window) == Some(hwnd))
}

/// Hand focus back once nothing of ours is on screen.
fn restore_foreground_if_idle(app: &AppHandle) {
    let still_showing = [WINDOW_LAUNCHER, WINDOW_POPOVER]
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .any(|window| window.is_visible().unwrap_or(false));
    if still_showing {
        return;
    }

    let state = app.state::<AppState>();
    let handle = state
        .previous_foreground
        .lock()
        .expect("foreground lock")
        .take();
    if let Some(handle) = handle {
        platform::focus::restore_foreground(handle);
    }
}

/// The Popover is cursor-adjacent (README), clamped to the work area.
fn place_at_cursor(window: &WebviewWindow) {
    let Some(cursor) = platform::cursor::cursor_position() else {
        return;
    };
    let Some(area) = platform::cursor::work_area_at(cursor.0, cursor.1) else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let (x, y) = platform::place_near_cursor(cursor, (size.width as i32, size.height as i32), area);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The Launcher is centred on the monitor the cursor is on: the README only
/// promises the *Popover* is cursor-adjacent, and a centred Launcher is what
/// every comparable tool does.
fn center_on_active_monitor(window: &WebviewWindow) {
    let Some(cursor) = platform::cursor::cursor_position() else {
        let _ = window.center();
        return;
    };
    let Some(area) = platform::cursor::work_area_at(cursor.0, cursor.1) else {
        let _ = window.center();
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let x = area.x + (area.width - size.width as i32) / 2;
    // Slightly above centre: a list grows downward, so this keeps the eye still.
    let y = area.y + (area.height - size.height as i32) / 3;
    let _ = window.set_position(PhysicalPosition::new(x.max(area.x), y.max(area.y)));
}
