//! What the OS will and will not let Beckon do, and the way to the switch.
//!
//! Only macOS has anything to answer here (ADR-0013); Windows reports
//! `not-required` and Settings says nothing at all. The command exists on both
//! so the frontend has one question to ask rather than a platform test.

use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri_plugin_opener::OpenerExt;

use crate::platform::{self, InputPermission};

#[tauri::command]
pub fn get_input_permission() -> InputPermission {
    platform::permission::input_permission()
}

/// Open the pane that grants it.
///
/// Like `open_api_key_page`, the destination is a constant rather than an
/// argument: this is one dead end being unblocked, not a general "open what the
/// webview asks for".
#[tauri::command]
pub fn open_input_permission_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.opener()
            .open_url(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
                None::<&str>,
            )
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("there is no input permission to grant on this platform".to_string())
    }
}
