//! Frontmost-application save/restore.
//!
//! The Popover takes focus, so whoever had it must be remembered *before*
//! anything of ours is shown and handed focus back on close (README).
//!
//! The unit differs from Windows and that is the whole of the difference:
//! Windows restores a *window* by `HWND`, macOS restores an *application* by
//! pid. `foreground_window` therefore returns a pid here, and `window_handle`
//! answers with our own — which is exactly what `is_ours` needs, since on macOS
//! "one of our windows is in front" and "we are the active app" are the same
//! statement.

use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWorkspace};

/// The frontmost application's pid, or `None` when nothing is active (the
/// Finder desktop, or an app that reports no pid).
pub fn foreground_window() -> Option<isize> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    // Apple documents -1 for an application without a pid.
    (pid > 0).then_some(pid as isize)
}

/// Our own pid, for every window: see the module note.
pub fn window_handle(_window: &tauri::WebviewWindow) -> Option<isize> {
    Some(std::process::id() as isize)
}

/// Give focus back. Returns false if the app has quit in the meantime, which is
/// not worth reporting.
pub fn restore_foreground(handle: isize) -> bool {
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(handle as i32)
    else {
        return false;
    };
    // All windows, not just the key one: the user is going back to the app they
    // were reading, and a single restored window is the wrong half of it.
    app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows)
}
