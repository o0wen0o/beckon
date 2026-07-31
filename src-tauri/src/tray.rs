//! Tray icon: the only always-visible part of Beckon.
//!
//! Two icon states — normal, and error for "a hotkey did not register", which
//! the README insists must never be silent.

use std::sync::atomic::Ordering;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;
use crate::trigger;

pub const TRAY_ID: &str = "beckon";

const ICON_NORMAL: &[u8] = include_bytes!("../icons/tray-normal.png");
const ICON_ERROR: &[u8] = include_bytes!("../icons/tray-error.png");

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Beckon", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON_NORMAL)?)
        .tooltip("Beckon")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => trigger::show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left click opens Settings. On Windows a toast's click cannot be
            // routed back to us, so the tray icon is the reliable target for
            // "the error notification says: open Settings".
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                trigger::show_settings(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Switch to the error icon and, the first time only, show the balloon.
pub fn set_error(app: &AppHandle, summary: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(icon) = Image::from_bytes(ICON_ERROR) {
            let _ = tray.set_icon(Some(icon));
        }
        let _ = tray.set_tooltip(Some(format!("Beckon — {summary}")));
    }

    {
        let state = app.state::<AppState>();
        let mut errors = state.startup_errors.lock().expect("startup errors lock");
        if !errors.iter().any(|e| e == summary) {
            errors.push(summary.to_string());
        }
        if state.balloon_shown.swap(true, Ordering::SeqCst) {
            return;
        }
    }

    let _ = app
        .notification()
        .builder()
        .title("Beckon: a hotkey is not active")
        .body(format!(
            "{summary}\n\nClick the Beckon tray icon to open Settings and fix it."
        ))
        .show();
}

pub fn set_normal(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(icon) = Image::from_bytes(ICON_NORMAL) {
            let _ = tray.set_icon(Some(icon));
        }
        let _ = tray.set_tooltip(Some("Beckon"));
    }
    let state = app.state::<AppState>();
    state
        .startup_errors
        .lock()
        .expect("startup errors lock")
        .clear();
}
