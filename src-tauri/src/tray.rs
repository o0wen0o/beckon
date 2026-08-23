//! Tray icon — the Windows notification area, the macOS menu bar: the only
//! always-visible part of Beckon.
//!
//! Two icon states — normal, and error for "a hotkey did not register", which
//! the README insists must never be silent.
//!
//! The icon is **not** a macOS template image, which is the platform's default
//! for a menu-bar item. A template is rendered from alpha alone, and the two
//! states here are one silhouette that differs only in accent colour — so
//! template mode would erase the error state and leave a black squircle
//! (ADR-0013).

use std::sync::atomic::Ordering;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::i18n;
use crate::state::AppState;
use crate::trigger;
use crate::update::{self, Voice};

pub const TRAY_ID: &str = "beckon";

const ICON_NORMAL: &[u8] = include_bytes!("../icons/tray-normal.png");
const ICON_ERROR: &[u8] = include_bytes!("../icons/tray-error.png");

/// The menu, built from scratch per language and per pending update.
///
/// Rebuilt rather than relabelled: `tauri::menu` hands back items by id, and
/// keeping handles alive in state so their text can be set later is more
/// machinery than a three-item menu is worth (`rebuild` below).
///
/// Both halves of the label — the language and the pending version — are read
/// out of state here rather than handed in, so no caller can pass one that has
/// already moved on.
fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let language = app.state::<AppState>().config_snapshot().language;
    let settings = MenuItem::with_id(
        app,
        "settings",
        i18n::tray_settings(language),
        true,
        None::<&str>,
    )?;
    // One item with two labels (ADR-0022): with nothing pending it asks, and
    // with a version pending it offers that version by name. The handler
    // branches on the same value, so the label and what clicking it does cannot
    // disagree.
    let update = MenuItem::with_id(
        app,
        "update",
        match pending_update(app) {
            Some(version) => i18n::tray_update_to(language, &version),
            None => i18n::tray_check_updates(language).to_string(),
        },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", i18n::tray_quit(language), true, None::<&str>)?;
    Menu::with_items(app, &[&settings, &update, &quit])
}

/// The version a check found, if one did.
fn pending_update(app: &AppHandle) -> Option<String> {
    app.state::<AppState>()
        .pending_update
        .lock()
        .expect("pending update lock")
        .clone()
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu(app)?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON_NORMAL)?)
        .icon_as_template(false)
        .tooltip("Beckon")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "settings" => trigger::show_settings(app),
            "update" => match pending_update(app) {
                Some(_) => update::install(app),
                None => update::check(app, Voice::Aloud),
            },
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left click opens Settings. Neither platform routes a
            // notification's click back to us reliably, so the tray icon is the
            // one dependable target for "the error notification says: open
            // Settings".
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

/// The one place the menu is swapped: in the language the config now names, and
/// around the version a check has found.
///
/// The tray is the one surface no `config-changed` event reaches: it is not a
/// window, so nothing re-renders it. `reload::reload_config` calls this.
pub fn rebuild(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

/// Remember the version an update check found, and redraw the menu around it
/// (ADR-0022). `None` clears it — a version installed by hand, or withdrawn.
///
/// An unchanged value returns instead, because the common case is a no-op: the
/// one quiet check per launch usually finds nothing and stores `None` over
/// `None`, and swapping the native menu for an identical one under the user's
/// cursor is not free.
pub fn set_pending_update(app: &AppHandle, version: Option<String>) {
    {
        let state = app.state::<AppState>();
        let mut pending = state.pending_update.lock().expect("pending update lock");
        if *pending == version {
            return;
        }
        *pending = version;
    }
    rebuild(app);
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
        // Replace, never append: `summary` is already the complete list of
        // what is currently broken. Appending kept the previous summary around
        // after the user had fixed it, so Settings reported repairs as faults.
        *errors = vec![summary.to_string()];
        if state.balloon_shown.swap(true, Ordering::SeqCst) {
            return;
        }
    }

    let language = app.state::<AppState>().config_snapshot().language;
    let _ = app
        .notification()
        .builder()
        .title(i18n::tray_error_title(language))
        .body(i18n::tray_error_body(language, summary))
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
