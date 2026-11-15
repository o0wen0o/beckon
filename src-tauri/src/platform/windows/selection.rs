//! Grabbing the Selection by simulating Ctrl+C.
//!
//! The order of operations here is load-bearing; each step exists because of a
//! specific failure:
//!
//! 1. Release the modifiers the user is *physically* holding. The trigger
//!    hotkey is still down, so without this the target app receives
//!    `Ctrl+Alt+C`, not `Ctrl+C`, and nothing is copied.
//! 2. Back up the clipboard, plain text only (losing rich text and images is
//!    accepted).
//! 3. Read the clipboard sequence number, then send Ctrl+C.
//! 4. **Poll** the sequence number rather than sleeping a fixed interval —
//!    a fixed sleep is either a race or a stall.
//! 5. Read the text, restore the backup, and drop the backup immediately:
//!    Beckon must not leave an extra retained copy of the user's clipboard.
//! 6. Timeout or an empty result is `None`, not an error. Elevated windows fail
//!    silently and that is handled by the Action's `input_source`.
//!
//! The other direction lives here too: [`send_paste`] is Ctrl+V
//! through the same `SendInput` and after the same modifier release, and it is
//! the *whole* of the write-back. There is no backup and no restore around it —
//! an Insert is a Copy the user asked for, followed by one keystroke.

use std::thread::sleep;
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN,
};

use super::clipboard;

/// `C`. Spelled out because the named constant is not in every `windows`
/// release, and this value is fixed by the platform.
const VK_C: VIRTUAL_KEY = VIRTUAL_KEY(0x43);
/// `V`, for the same reason.
const VK_V: VIRTUAL_KEY = VIRTUAL_KEY(0x56);
const VK_CONTROL_LEFT: VIRTUAL_KEY = VK_LCONTROL;

/// How long the target app gets to see the modifier key-ups before Ctrl+C
/// arrives. Not the clipboard wait — that one is polled.
const MODIFIER_SETTLE: Duration = Duration::from_millis(15);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const POLL_CAP: Duration = Duration::from_millis(300);

/// The modifiers a hotkey can hold down while we try to send a clean Ctrl+C.
const HELD_MODIFIERS: [VIRTUAL_KEY; 8] = [
    VK_LCONTROL,
    VK_RCONTROL,
    VK_LMENU,
    VK_RMENU,
    VK_LSHIFT,
    VK_RSHIFT,
    VK_LWIN,
    VK_RWIN,
];

/// Grab the Selection. `None` means "there was nothing to grab" — a normal
/// outcome, not a failure.
pub fn grab_selection() -> Option<String> {
    release_held_modifiers();
    sleep(MODIFIER_SETTLE);

    let backup = read_clipboard_text();
    let before = clipboard::sequence_number();

    if let Err(err) = send_ctrl(VK_C) {
        log::warn!("could not synthesise Ctrl+C: {err}");
        return None;
    }

    let grabbed =
        clipboard::poll_until_written(before, POLL_CAP, POLL_INTERVAL, read_clipboard_text);

    // Restore before returning, and let the backup drop at the end of this
    // scope — no long-lived copy of the user's clipboard.
    //
    // Only if the copy shortcut wrote something. What the restore undoes is the
    // pollution the grab caused, and a grab with nothing selected causes none:
    // the clipboard is still exactly what the user left there. "Restoring" over
    // that replaced it with its own plain-text half — and when it held no text
    // at all, a file copied in Explorer or a screenshot, with an empty string.
    // That was every Launcher summon with nothing selected.
    if clipboard::sequence_number() != before {
        restore_clipboard(backup);
    }

    grabbed.filter(|text| !text.trim().is_empty())
}

/// A user-requested clipboard write (the Popover's Copy). Not restored, by
/// design: the user asked for it.
pub fn write_clipboard_text(text: &str) -> Result<(), String> {
    clipboard_win::set_clipboard_string(text).map_err(|e| e.to_string())
}

/// Ctrl+V into whatever has focus. The caller has already put the
/// text on the clipboard and the target window in front; this is the keystroke
/// and nothing else.
///
/// The modifiers are released first for the reason the grab releases them: the
/// Popover's Insert chord is still physically down, and Ctrl+Shift+V is "paste
/// without formatting" in one app and nothing at all in the next.
pub fn send_paste() -> Result<(), String> {
    release_held_modifiers();
    sleep(MODIFIER_SETTLE);
    send_ctrl(VK_V)
}

fn read_clipboard_text() -> Option<String> {
    clipboard_win::get_clipboard_string().ok()
}

fn restore_clipboard(backup: Option<String>) {
    // With no text backup there was nothing of ours to give back; clearing at
    // least avoids leaving the grabbed Selection sitting in the clipboard.
    let text = backup.unwrap_or_default();
    if let Err(err) = clipboard_win::set_clipboard_string(&text) {
        log::warn!("could not restore the clipboard: {err}");
    }
}

fn release_held_modifiers() {
    let mut inputs: Vec<INPUT> = Vec::new();
    for key in HELD_MODIFIERS {
        if is_physically_down(key) {
            inputs.push(key_input(key, KEYEVENTF_KEYUP));
        }
    }
    if inputs.is_empty() {
        return;
    }
    if let Err(err) = send(&inputs) {
        log::warn!("could not release held modifiers: {err}");
    }
}

fn is_physically_down(key: VIRTUAL_KEY) -> bool {
    // Safe: reads keyboard state for one virtual key.
    let state = unsafe { GetAsyncKeyState(key.0 as i32) };
    (state as u16 & 0x8000) != 0
}

/// Ctrl plus one key, down and up, as a single `SendInput` batch so nothing
/// the user types can land between the four events. One function for both
/// directions: the copy and the paste differ by a letter.
fn send_ctrl(key: VIRTUAL_KEY) -> Result<(), String> {
    send(&[
        key_input(VK_CONTROL_LEFT, KEYBD_EVENT_FLAGS(0)),
        key_input(key, KEYBD_EVENT_FLAGS(0)),
        key_input(key, KEYEVENTF_KEYUP),
        key_input(VK_CONTROL_LEFT, KEYEVENTF_KEYUP),
    ])
}

fn key_input(key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), String> {
    // Safe: `inputs` is a live slice and the size argument matches `INPUT`.
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(format!(
            "SendInput accepted {sent} of {} events",
            inputs.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clipboard_wait_is_polled_not_slept() {
        assert!(POLL_INTERVAL < POLL_CAP);
        assert!(POLL_CAP <= Duration::from_millis(500));
    }

    /// The copy and the paste are the same chord with one key changed, and the
    /// two keys are fixed by the platform rather than by a named constant.
    #[test]
    fn the_copy_and_paste_keys_are_c_and_v() {
        assert_eq!(VK_C.0, b'C' as u16);
        assert_eq!(VK_V.0, b'V' as u16);
    }

    #[test]
    fn both_sides_of_every_modifier_are_released() {
        assert_eq!(HELD_MODIFIERS.len(), 8);
        assert!(HELD_MODIFIERS.contains(&VK_LWIN) && HELD_MODIFIERS.contains(&VK_RWIN));
    }
}
