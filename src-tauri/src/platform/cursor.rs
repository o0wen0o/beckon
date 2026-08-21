//! Cursor position and the work area of the monitor under it, in physical
//! pixels, so the Popover can be clamped on-screen at any DPI.
//!
//! Both come from Tauri rather than from each platform's own API, and that is
//! the one part of this layer that is *not* per-platform on purpose (ADR-0013).
//! tao already normalises macOS's bottom-left, Y-up screen space into the
//! top-left, Y-down space `set_position` takes; deriving that flip a second
//! time here would be a second place for it to be wrong. Using one path also
//! means every Windows run exercises the code macOS depends on.

use tauri::AppHandle;

use super::WorkArea;

/// Where the pointer is, relative to the top-left of the desktop.
pub fn cursor_position(app: &AppHandle) -> Option<(i32, i32)> {
    let position = app.cursor_position().ok()?;
    Some((position.x as i32, position.y as i32))
}

/// The work area — taskbar, Dock and menu bar excluded — of the monitor
/// containing the point. Falls back to the primary monitor, so a coordinate
/// that lands between screens still resolves.
pub fn work_area_at(app: &AppHandle, x: i32, y: i32) -> Option<WorkArea> {
    let monitor = app
        .monitor_from_point(x as f64, y as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;

    let area = monitor.work_area();
    Some(WorkArea {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width as i32,
        height: area.size.height as i32,
    })
}
