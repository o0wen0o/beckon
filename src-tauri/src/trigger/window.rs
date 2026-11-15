//! Making the three windows appear: sizes, placement, and the one window that
//! is built on demand.
//!
//! Launcher and Popover are created hidden at startup and only shown/hidden:
//! WebView creation costs far too much to pay per keypress. The Exchange is
//! never persisted, so it is destroyed on hide instead, which `hide_popover` does.
//!
//! Settings is the exception: nothing about it is latency-sensitive, and a third
//! live WebView is the most expensive thing in a resident tool. It is built on
//! first use and kept afterwards.

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow};

use crate::config::PopoverSize;
use crate::i18n;
use crate::platform;
use crate::state::AppState;

use super::WINDOW_SETTINGS;

pub(super) fn build_settings_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let language = app.state::<AppState>().config_snapshot().language;
    tauri::WebviewWindowBuilder::new(
        app,
        WINDOW_SETTINGS,
        WebviewUrl::App("settings.html".into()),
    )
    .title(i18n::settings_window_title(language))
    // 240px of that width is the navigation column; the rest is the pane the
    // Action editor lives in, which is the widest thing in the product.
    .inner_size(980.0, 760.0)
    .min_inner_size(780.0, 560.0)
    .center()
    .resizable(true)
    .visible(false)
    .build()
}

/// The title bar is chrome, not markup, so `config-changed` does not reach it —
/// the same gap `tray::rebuild` fills. Only Settings has one to redraw.
pub fn retitle_settings(app: &AppHandle, language: crate::config::Language) {
    if let Some(window) = app.get_webview_window(WINDOW_SETTINGS) {
        let _ = window.set_title(i18n::settings_window_title(language));
    }
}

pub(super) fn reveal(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

/// Size the Popover, then place it cursor-adjacent (README), clamped to the
/// work area.
///
/// The physical size is derived from the logical one rather than read back with
/// `outer_size()`. This runs on the hotkey thread, so `set_size` is dispatched
/// to the event loop and an immediate read can still return the old rect —
/// which would place a window the user has dragged short using the previous
/// rect's bounds.
///
/// Both the fit and the conversion use the monitor *under the cursor*, which
/// is where the window is going, not the one it was last on. `window.scale_factor()`
/// answers for the latter: summoned on a 200% display after a 100% one, a
/// 620×500 window was placed as if it were 620×500 physical and then doubled
/// by the window manager as it crossed, off the right edge and the bottom.
pub(super) fn size_and_place_at_cursor(window: &WebviewWindow, width: f64, height: f64) {
    let app = window.app_handle();
    let screen = platform::cursor::cursor_position(app).and_then(|cursor| {
        platform::cursor::screen_at(app, cursor.0, cursor.1).map(|screen| (cursor, screen))
    });

    // The user's size, or as much of it as this monitor has room for. Config
    // is not touched: the fit is for this showing only.
    let (width, height) = match screen {
        Some((_, screen)) => platform::fit_to_area((width, height), screen.area, screen.scale),
        None => (width, height),
    };
    let size = LogicalSize::new(width, height);
    let _ = window.set_size(size);

    // Recorded before the placement can fail out: this is what tells the resize
    // that follows apart from one the user dragged. The *fitted* size, because
    // that is what was asked for — recording the remembered one would make the
    // echo of a fit look like a drag and write the small monitor's size down.
    *app.state::<AppState>()
        .popover_asked_size
        .lock()
        .expect("popover size lock") = PopoverSize { width, height };
    let Some((cursor, screen)) = screen else {
        return;
    };
    let physical = size.to_physical::<i32>(screen.scale);
    let (x, y) =
        platform::place_near_cursor(cursor, (physical.width, physical.height), screen.area);
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// A size the window reported, kept if the *user* is what produced it.
///
/// The Popover is undecorated, so the grips that drag it are markup and the
/// resize itself is the window manager's; this is the only place the result of
/// one is written down. Every resize reports itself though — including the
/// `set_size` above — so a report matching what we last asked for is dropped
/// rather than saved. Without that, a clamped or rounded echo of our own summon
/// would be persisted as if the user had dragged to it.
///
/// Through the config funnel like every other write:
/// [`crate::reload::write_config`] marks it as our own so the watcher swallows
/// the echo, then re-reads and broadcasts.
pub fn remember_popover_size(app: &AppHandle, width: f64, height: f64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let wanted = PopoverSize { width, height }.clamped();
    {
        let asked = state.popover_asked_size.lock().expect("popover size lock");
        // Rounded to the pixel: the round trip through physical pixels on a
        // fractional scale factor does not come back exact.
        if asked.width.round() == wanted.width.round()
            && asked.height.round() == wanted.height.round()
        {
            return Ok(());
        }
    }

    let mut config = state.config_snapshot();
    if config.popover == wanted {
        return Ok(());
    }
    config.popover = wanted;
    crate::reload::write_config(app, &config)
}

/// The Launcher is centred on the monitor the cursor is on: the README only
/// promises the *Popover* is cursor-adjacent, and a centred Launcher is what
/// every comparable tool does.
pub(super) fn center_on_active_monitor(window: &WebviewWindow) {
    let app = window.app_handle();
    let Some(cursor) = platform::cursor::cursor_position(app) else {
        let _ = window.center();
        return;
    };
    let Some(screen) = platform::cursor::screen_at(app, cursor.0, cursor.1) else {
        let _ = window.center();
        return;
    };
    let area = screen.area;
    let (Ok(size), Ok(current)) = (window.outer_size(), window.scale_factor()) else {
        return;
    };
    // `outer_size` is in the pixels of the monitor the window was last on. On
    // the one it is going to it will be this many, once the window manager has
    // rescaled it to keep its logical size — and centring with the old number
    // lands it off-centre by half the difference.
    let rescale = if current > 0.0 {
        screen.scale / current
    } else {
        1.0
    };
    let width = (f64::from(size.width) * rescale).round() as i32;
    let height = (f64::from(size.height) * rescale).round() as i32;
    let x = area.x + (area.width - width) / 2;
    // Slightly above centre: a list grows downward, so this keeps the eye still.
    let y = area.y + (area.height - height) / 3;
    let _ = window.set_position(PhysicalPosition::new(x.max(area.x), y.max(area.y)));
}
