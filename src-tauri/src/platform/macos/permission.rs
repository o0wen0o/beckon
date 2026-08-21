//! Accessibility trust, the macOS counterpart to "nothing to ask" on Windows.
//!
//! `CGEventPost` is how ADR-0002's grab is performed, and macOS refuses it
//! *silently* for an untrusted process: no error, no event, an empty clipboard
//! and an Action that looks broken. So the state is read directly rather than
//! inferred from a failed grab, and Settings says what to do about it.
//!
//! `AXIsProcessTrusted` is plain C, so it is linked here rather than reached
//! through a binding crate.

use crate::platform::InputPermission;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// The pane that grants it. A constant, not an argument: this is one dead end
/// being unblocked, not a general "open what the webview asks for".
pub fn settings_url() -> Option<&'static str> {
    Some("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
}

pub fn input_permission() -> InputPermission {
    // Safe: no arguments, no ownership, reads a process-wide TCC decision.
    if unsafe { AXIsProcessTrusted() } {
        InputPermission::Granted
    } else {
        InputPermission::Denied
    }
}
