//! Cursor position and the work area of the monitor under it, in physical
//! pixels, so the Popover can be clamped on-screen at any DPI.
//!
//! Both come from Tauri rather than from each platform's own API, and that is
//! the one part of this layer that is *not* per-platform on purpose.
//! tao already normalises macOS's bottom-left, Y-up screen space into the
//! top-left, Y-down space `set_position` takes; deriving that flip a second
//! time here would be a second place for it to be wrong. Using one path also
//! means every Windows run exercises the code macOS depends on.

use std::sync::mpsc;

use tauri::AppHandle;

use super::WorkArea;

/// The monitor a window is about to be shown on: where it may go, and how many
/// physical pixels a logical one is there.
///
/// The scale belongs with the area because it is the *destination's*. A window
/// knows only the scale of the monitor it was last on, and a size worked out
/// with that one is wrong by the ratio between the two as soon as the window
/// crosses — the window manager keeps the logical size and rescales the rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub area: WorkArea,
    pub scale: f64,
}

/// Where the pointer is, relative to the top-left of the desktop.
pub fn cursor_position(app: &AppHandle) -> Option<(i32, i32)> {
    let position = app.cursor_position().ok()?;
    Some((position.x as i32, position.y as i32))
}

/// The work area — taskbar, Dock and menu bar excluded — of the monitor
/// containing the point. Falls back to the primary monitor, so a coordinate
/// that lands between screens still resolves.
///
/// Run on the main thread, because the trigger flow is not: the hotkey handler
/// spawns a thread so the grab's clipboard poll cannot stall the event pump.
/// `monitor_from_point` and `primary_monitor` are the two `AppHandle` monitor
/// calls tauri-runtime-wry serves inline instead of dispatching to the event
/// loop, and reading a monitor's scale, position or work area reaches
/// `NSScreen` on macOS, which is main-thread-only. `cursor_position` above is
/// dispatched already, so it needs no hop.
///
/// The hop cannot deadlock a caller that is already on the main thread:
/// `run_on_main_thread` runs the closure inline in that case, so the value is
/// in the channel before the receive.
pub fn screen_at(app: &AppHandle, x: i32, y: i32) -> Option<Screen> {
    let (tx, rx) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(monitor_screen(&handle, x, y));
    })
    .ok()?;
    rx.recv().ok()?
}

/// The lookup itself, main thread assumed.
fn monitor_screen(app: &AppHandle, x: i32, y: i32) -> Option<Screen> {
    let monitor = app
        .monitor_from_point(x as f64, y as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;

    let area = monitor.work_area();
    Some(Screen {
        area: WorkArea {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width as i32,
            height: area.size.height as i32,
        },
        scale: monitor.scale_factor(),
    })
}
