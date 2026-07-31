//! Cursor position and the monitor's work area, both in physical pixels, so the
//! Popover can be clamped on-screen at any DPI.

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::platform::WorkArea;

pub fn cursor_position() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    // Safe: writes into our own POINT.
    unsafe { GetCursorPos(&mut point).ok()? };
    Some((point.x, point.y))
}

/// The work area (taskbar excluded) of the monitor containing the point. Falls
/// back to the nearest monitor, so an off-screen coordinate still resolves.
pub fn work_area_at(x: i32, y: i32) -> Option<WorkArea> {
    let point = POINT { x, y };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    // Safe: `MonitorFromPoint` with DEFAULTTONEAREST always returns a monitor,
    // and `cbSize` is set as the API requires.
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
    }

    let rect = info.rcWork;
    Some(WorkArea {
        x: rect.left,
        y: rect.top,
        width: rect.right - rect.left,
        height: rect.bottom - rect.top,
    })
}
