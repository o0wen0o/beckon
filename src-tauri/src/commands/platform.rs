//! What the OS will and will not let Beckon do, and the way to the switch.
//!
//! Only macOS has anything to answer here (ADR-0013); Windows reports
//! `not-required` and Settings says nothing at all. Both commands exist on both
//! platforms so the frontend has one question to ask rather than a platform
//! test — and both are delegates, so the `#[cfg]` stays under `platform/`.

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::platform::{self, InputPermission};

#[tauri::command]
pub fn get_input_permission() -> InputPermission {
    platform::permission::input_permission()
}

/// Open the pane that grants it, where there is one.
#[tauri::command]
pub fn open_input_permission_settings(app: AppHandle) -> Result<(), String> {
    let url = platform::permission::settings_url()
        .ok_or_else(|| "there is no input permission to grant on this platform".to_string())?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}
