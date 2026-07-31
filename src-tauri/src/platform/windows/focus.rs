//! Foreground window save/restore.
//!
//! The Popover takes focus, so the window that had it must be remembered
//! *before* anything of ours is shown and handed focus back on close (README).

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindow, SetForegroundWindow};

/// The current foreground window as a raw handle value, or `None` if there is
/// none (the desktop, or a window we are not allowed to see).
pub fn foreground_window() -> Option<isize> {
    // Safe: no arguments, returns null when there is no foreground window.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

/// Give focus back. Returns false if the window is gone — the user closed it
/// while the Popover was open, which is not worth reporting.
pub fn restore_foreground(handle: isize) -> bool {
    let hwnd = HWND(handle as *mut std::ffi::c_void);
    // Safe: `IsWindow` tolerates a stale handle, which is exactly why it is
    // checked first.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }
        SetForegroundWindow(hwnd).as_bool()
    }
}
