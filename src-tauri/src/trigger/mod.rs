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
pub mod window;

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::action::InputSource;
use crate::exchange;
use crate::llm::Content;
use crate::platform;
use crate::platform::capture::{Capture, CaptureError};
use crate::state::{AppState, PopoverPhase, PopoverView};

use self::foreground::{remember_foreground, restore_foreground_if_idle};
use self::window::{center_on_active_monitor, reveal, size_and_place_at_cursor, POPOVER_HINT_H};
use self::window::{POPOVER_H, POPOVER_W};

pub const WINDOW_LAUNCHER: &str = "launcher";
pub const WINDOW_POPOVER: &str = "popover";
pub const WINDOW_SETTINGS: &str = "settings";

/// The Popover's view changed: re-read it with `get_popover_view`.
pub const EVENT_POPOVER_VIEW: &str = "popover:view";
/// The attached Capture changed, and *only* that (ADR-0016). Its own event
/// rather than `popover:view`, because re-reading the view is how a new trigger
/// is handled: it resets the conversation and remounts the composer, which is
/// exactly wrong for a screenshot attached in the middle of typing.
pub const EVENT_POPOVER_CAPTURE: &str = "popover:capture";
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
        capture: None,
        capture_cancelled: false,
        capture_error: None,
    };

    if phase == PopoverPhase::Running {
        let input = input.unwrap_or_default();
        let exchange_id = state.exchanges.create(&action.file.prompt.system, params);
        view.exchange_id = Some(exchange_id.clone());
        if let Some(plan) = state
            .exchanges
            .begin_turn(&exchange_id, action.render_user(&input))
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
///
/// An attached Capture is taken here: a Capture belongs to the turn it was
/// attached to, so sending consumes it (ADR-0016).
pub fn submit_input(app: &AppHandle, text: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    // Only the identity is read from the view here. Cloning the whole thing
    // would copy the attached Capture's base64 — megabytes — to reach one
    // `String`, and the Capture itself is moved out below.
    let action_id = state
        .popover_view
        .lock()
        .expect("popover view lock")
        .as_ref()
        .map(|view| view.action_id.clone())
        .ok_or_else(|| "there is no Action to send".to_string())?;

    let (action, defaults) = {
        let registry = state.registry.read().expect("registry lock");
        let action = registry
            .get(&action_id)
            .cloned()
            .ok_or_else(|| format!("the Action \"{action_id}\" no longer exists"))?;
        (action, state.config_snapshot().defaults)
    };

    let params = action.model_params(&defaults);
    let exchange_id = state
        .exchanges
        .create(&action.file.prompt.system, params.clone());
    let content = {
        let mut slot = state.popover_view.lock().expect("popover view lock");
        // Consumed by the turn: it is in the history now, and leaving it
        // attached would send it a second time with the next follow-up.
        let capture = slot.as_mut().and_then(PopoverView::take_capture);
        user_content(&action.render_user(text), capture)
    };
    let plan = state
        .exchanges
        .begin_turn(&exchange_id, content)
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
///
/// A Capture attached after the first answer belongs to this turn, exactly as
/// one attached before the first did.
pub fn follow_up(app: &AppHandle, exchange_id: &str, text: &str) -> Result<(), String> {
    let capture = app
        .state::<AppState>()
        .popover_view
        .lock()
        .expect("popover view lock")
        .as_mut()
        // No view is not a state a follow-up can reach; nothing attached is the
        // answer that cannot be wrong if it ever does.
        .and_then(PopoverView::take_capture);
    // Straight into `retry`, which is exactly "begin this content and spawn the
    // turn" — the only difference between the two is where the content is from.
    retry(app, exchange_id, user_content(text, capture))
}

/// Resend a turn verbatim. A retry repeats the message that failed — the same
/// words and the same Capture — so it does not go through the attach path.
pub fn retry(app: &AppHandle, exchange_id: &str, content: Content) -> Result<(), String> {
    let state = app.state::<AppState>();
    let plan = state
        .exchanges
        .begin_turn(exchange_id, content)
        .ok_or_else(|| "this Exchange is gone; trigger the Action again".to_string())?;
    exchange::spawn_turn(app.clone(), plan);
    Ok(())
}

/// One turn worth of content: the rendered text, plus the attached Capture when
/// there is one. The only place the two are joined, so "the image goes with the
/// words typed beside it" is a single rule (ADR-0016).
///
/// The Capture arrives by value because every caller has just taken it out of
/// the view: its base64 is megabytes, and it is moved onto the wire rather than
/// copied there.
fn user_content(text: &str, capture: Option<Capture>) -> Content {
    match capture {
        Some(capture) => Content::with_image(text, capture.data_url),
        None => Content::from(text),
    }
}

/// The Popover screenshot button: hide our windows, run the OS snip tool, then
/// come back with whatever it produced attached.
///
/// Hiding is deliberately not [`hide_popover`], which discards the Exchange
/// (ADR-0004) — a conversation has to survive a screenshot taken in the middle
/// of it. So the *window* hides and nothing hands the foreground back: the snip
/// tool takes the screen for itself, and we are coming straight back.
///
/// Runs on its own thread, because the snip blocks for as long as the user takes
/// to drag a rectangle.
pub fn start_capture(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        // The button and its shortcut can both land before the window has even
        // hidden; the second one is dropped rather than opening a second tool.
        if state
            .capturing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let mut slot = state.popover_view.lock().expect("popover view lock");
        match slot.as_mut() {
            // A second attempt clears what the first one said, so the window
            // cannot come back saying "nothing was captured" over a fresh
            // Capture. Only the notices: the Capture already attached stays
            // until this snip lands.
            Some(view) => view.clear_capture_notices(),
            // Nothing to capture *for*: no Action is loaded. The flag has to go
            // back down on the way out, or the button is dead for good.
            None => {
                state.capturing.store(false, Ordering::SeqCst);
                return;
            }
        }
    }

    let app = app.clone();
    std::thread::spawn(move || {
        let window = app.get_webview_window(WINDOW_POPOVER);
        if let Some(window) = &window {
            // Otherwise the Popover is in the shot, sitting over the very thing
            // being captured.
            let _ = window.hide();
        }

        let outcome = platform::snip::grab_capture();

        {
            let state = app.state::<AppState>();
            // Read after the snip, not before it: the user had up to 45 seconds
            // in the snip tool, and the sentence belongs in the language the
            // config holds now (ADR-0015).
            let language = state.config_snapshot().language;
            let mut slot = state.popover_view.lock().expect("popover view lock");
            if let Some(view) = slot.as_mut() {
                view.apply_capture(outcome, language);
            }
        }

        app.state::<AppState>()
            .capturing
            .store(false, Ordering::SeqCst);

        // Emit before revealing, as every other view change does: the window is
        // reused (ADR-0007) and would paint its pre-snip self for a few frames.
        emit_capture(&app);
        if let Some(window) = &window {
            reveal(window);
        }
    });
}

/// The Popover remove-the-screenshot button.
pub fn discard_capture(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut slot = state.popover_view.lock().expect("popover view lock");
        let Some(view) = slot.as_mut() else { return };
        view.clear_capture();
    }
    emit_capture(app);
}

/// The three fields of the view a Capture can change, as one payload.
///
/// Borrowed from the view rather than cloned into a `serde_json::Value`: the
/// base64 is megabytes and Tauri serialises the payload anyway, so a `Value` in
/// between would be one whole copy of the image for nothing. `Clone` is what
/// `emit_to` asks for, and cloning this copies two references rather than the
/// megabytes behind them.
#[derive(Serialize, Clone, Default)]
struct CapturePayload<'a> {
    capture: Option<&'a Capture>,
    cancelled: bool,
    error: Option<&'a CaptureError>,
}

/// Tell the window what the three fields now are, rather than telling it to go
/// and look: looking means `get_popover_view`, which is the new-trigger path.
///
/// The guard is held across the emit, which is allowed — this is a plain `std`
/// lock and `emit_to` is not a suspension point.
fn emit_capture(app: &AppHandle) {
    let state = app.state::<AppState>();
    let slot = state.popover_view.lock().expect("popover view lock");
    let payload = match slot.as_ref() {
        Some(view) => CapturePayload {
            capture: view.capture.as_ref(),
            cancelled: view.capture_cancelled,
            error: view.capture_error.as_ref(),
        },
        None => CapturePayload::default(),
    };
    let _ = app.emit_to(WINDOW_POPOVER, EVENT_POPOVER_CAPTURE, payload);
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
