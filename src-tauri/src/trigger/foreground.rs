//! Whose window was in front before ours, and when to give it back.

use tauri::{AppHandle, Manager};

use crate::platform;
use crate::state::AppState;

use super::{WINDOW_LAUNCHER, WINDOW_POPOVER, WINDOW_SETTINGS};

pub(super) fn remember_foreground(app: &AppHandle) {
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
pub(super) fn restore_foreground_if_idle(app: &AppHandle) {
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
