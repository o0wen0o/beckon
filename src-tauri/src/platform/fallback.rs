//! Stubs for a platform Beckon does not ship on yet (ADR-0001, ADR-0013).
//!
//! Nothing here works; the point is that the crate still compiles off Windows
//! and macOS, which is what keeps the isolation honest — a Win32 or AppKit call
//! that leaked into business logic would break this file first.

use crate::platform::InputPermission;

pub mod focus {
    pub fn foreground_window() -> Option<isize> {
        None
    }
    pub fn window_handle(_window: &tauri::WebviewWindow) -> Option<isize> {
        None
    }
    pub fn restore_foreground(_handle: isize) -> bool {
        false
    }
}

pub mod selection {
    pub fn grab_selection() -> Option<String> {
        None
    }
    pub fn write_clipboard_text(_text: &str) -> Result<(), String> {
        Err("clipboard access is not implemented on this platform".to_string())
    }
}

pub mod permission {
    use super::InputPermission;

    pub fn input_permission() -> InputPermission {
        InputPermission::NotRequired
    }
}
