//! Whose window was in front before ours, and when to give it back.

use std::thread::sleep;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::platform;
use crate::state::AppState;

use super::{WINDOW_LAUNCHER, WINDOW_POPOVER, WINDOW_SETTINGS};

pub(super) fn remember_foreground(app: &AppHandle) {
    let Some(hwnd) = platform::focus::foreground_window() else {
        return;
    };
    // Do not remember one of our own windows: closing the Popover would then
    // "restore" focus to the Launcher we just hid.
    if is_ours(app, hwnd) {
        return;
    }
    let state = app.state::<AppState>();
    *state.previous_foreground.lock().expect("foreground lock") = Some(hwnd);
}

/// The window an Insert would paste into, if one is remembered.
///
/// Read, not taken: an Insert can still be refused after this, and a refusal
/// leaves the Popover up — which must still be able to hand the foreground back
/// when it does close.
pub(super) fn insert_target(app: &AppHandle) -> Option<isize> {
    *app.state::<AppState>()
        .previous_foreground
        .lock()
        .expect("foreground lock")
}

/// How long the window Beckon was summoned over gets to come back to the front
/// before an Insert gives up on the keystroke.
const FOREGROUND_CAP: Duration = Duration::from_millis(500);
const FOREGROUND_POLL: Duration = Duration::from_millis(10);

/// Wait until `handle` really is in front. Asking for the foreground is a
/// request the OS grants a beat later, and a paste sent in that beat lands in
/// the Popover that is on its way out — or nowhere.
///
/// Polled rather than slept, for the reason the grab polls the clipboard:
/// a fixed sleep is either a race or a stall.
pub(super) fn wait_for_foreground(handle: isize) -> bool {
    poll_until(FOREGROUND_CAP, FOREGROUND_POLL, || {
        platform::focus::foreground_window() == Some(handle)
    })
}

/// `true` as soon as `ready` says so, `false` once `cap` has passed without it.
/// Asked at least once, so a zero cap is "check, do not wait".
fn poll_until(cap: Duration, interval: Duration, ready: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + cap;
    loop {
        if ready() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        sleep(interval);
    }
}

fn is_ours(app: &AppHandle, hwnd: isize) -> bool {
    [WINDOW_LAUNCHER, WINDOW_POPOVER, WINDOW_SETTINGS]
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .any(|window| platform::focus::window_handle(&window) == Some(hwnd))
}

/// Hand focus back once nothing of ours is on screen.
///
/// Settings counts. The Launcher hides itself on the way there, and restoring
/// the previous app in that instant is what pulls the Settings window that was
/// just shown straight back behind whatever Beckon was summoned over.
pub(super) fn restore_foreground_if_idle(app: &AppHandle) {
    let still_showing = [WINDOW_LAUNCHER, WINDOW_POPOVER, WINDOW_SETTINGS]
        .iter()
        .filter_map(|label| app.get_webview_window(label))
        .any(|window| window.is_visible().unwrap_or(false));
    if still_showing {
        return;
    }

    let state = app.state::<AppState>();
    let handle = state
        .previous_foreground
        .lock()
        .expect("foreground lock")
        .take();
    if let Some(handle) = handle {
        platform::focus::restore_foreground(handle);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn a_window_already_in_front_is_not_waited_for() {
        let started = Instant::now();
        assert!(poll_until(
            Duration::from_secs(5),
            Duration::from_secs(5),
            || true
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_window_that_never_comes_back_is_given_up_on() {
        assert!(!poll_until(
            Duration::from_millis(20),
            Duration::from_millis(5),
            || false
        ));
    }

    /// The foreground arrives a beat after it is asked for; the poll is what
    /// turns that beat into a wait rather than a miss.
    #[test]
    fn the_wait_ends_on_the_poll_that_finds_it() {
        let asked = Cell::new(0);
        assert!(poll_until(
            Duration::from_secs(5),
            Duration::from_millis(1),
            || {
                asked.set(asked.get() + 1);
                asked.get() == 3
            }
        ));
        assert_eq!(asked.get(), 3);
    }

    #[test]
    fn a_zero_cap_still_asks_once() {
        assert!(poll_until(Duration::ZERO, Duration::ZERO, || true));
    }

    #[test]
    fn the_foreground_wait_is_polled_not_slept() {
        assert!(FOREGROUND_POLL < FOREGROUND_CAP);
        assert!(FOREGROUND_CAP <= Duration::from_secs(1));
    }
}
