//! Making the three windows appear: sizes, placement, and the one window that
//! is built on demand.
//!
//! Launcher and Popover are created hidden at startup and only shown/hidden
//! (ADR-0007): WebView creation costs far too much to pay per keypress. ADR-0004
//! is satisfied by destroying the *Exchange* on hide, which `hide_popover` does.
//!
//! Settings is the exception: nothing about it is latency-sensitive, and a third
//! live WebView is the most expensive thing in a resident tool. It is built on
//! first use and kept afterwards.

use tauri::{AppHandle, LogicalSize, PhysicalPosition, WebviewUrl, WebviewWindow};

use crate::platform;

use super::WINDOW_SETTINGS;

/// The Popover's normal size. Mirrored in `tauri.conf.json` so the very first
/// paint is not at the wrong size.
pub(super) const POPOVER_W: f64 = 620.0;
pub(super) const POPOVER_H: f64 = 500.0;
/// `empty-selection` issues no request and offers no input, so it can never
/// grow — which is what makes a smaller window safe here and nowhere else.
/// A two-line hint does not need 500px of empty Popover.
pub(super) const POPOVER_HINT_H: f64 = 220.0;

pub(super) fn build_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    tauri::WebviewWindowBuilder::new(
        app,
        WINDOW_SETTINGS,
        WebviewUrl::App("settings.html".into()),
    )
    .title("Beckon Settings")
    // 240px of that width is the navigation column; the rest is the pane the
    // Action editor lives in, which is the widest thing in the product.
    .inner_size(980.0, 760.0)
    .min_inner_size(780.0, 560.0)
    .center()
    .resizable(true)
    .visible(false)
    .build()
}

pub(super) fn reveal(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

/// Size the Popover, then place it cursor-adjacent (README), clamped to the
/// work area.
///
/// The physical size is derived from the logical one rather than read back with
/// `outer_size()`. This runs on the hotkey thread, so `set_size` is dispatched
/// to the event loop and an immediate read can still return the old rect —
/// which would place a hint-sized window using the full-sized bounds.
pub(super) fn size_and_place_at_cursor(window: &WebviewWindow, width: f64, height: f64) {
    let size = LogicalSize::new(width, height);
    let _ = window.set_size(size);

    let Some(cursor) = platform::cursor::cursor_position() else {
        return;
    };
    let Some(area) = platform::cursor::work_area_at(cursor.0, cursor.1) else {
        return;
    };
    let Ok(scale) = window.scale_factor() else {
        return;
    };
    let physical = size.to_physical::<i32>(scale);
    let (x, y) = platform::place_near_cursor(cursor, (physical.width, physical.height), area);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The Launcher is centred on the monitor the cursor is on: the README only
/// promises the *Popover* is cursor-adjacent, and a centred Launcher is what
/// every comparable tool does.
pub(super) fn center_on_active_monitor(window: &WebviewWindow) {
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
