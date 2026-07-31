//! Grabbing the Selection by simulating Ctrl+C (ADR-0002).
//!
//! The order of operations here is load-bearing; each step exists because of a
//! specific failure:
//!
//! 1. Release the modifiers the user is *physically* holding. The trigger
//!    hotkey is still down, so without this the target app receives
//!    `Ctrl+Alt+C`, not `Ctrl+C`, and nothing is copied.
//! 2. Back up the clipboard, plain text only (ADR-0002 accepts losing rich text
//!    and images).
//! 3. Read the clipboard sequence number, then send Ctrl+C.
//! 4. **Poll** the sequence number rather than sleeping a fixed interval —
//!    a fixed sleep is either a race or a stall.
//! 5. Read the text, restore the backup, and drop the backup immediately:
//!    Beckon must not leave an extra retained copy of the user's clipboard.
//! 6. Timeout or an empty result is `None`, not an error. Elevated windows fail
//!    silently and that is handled by the Action's `input_source`.

use std::thread::sleep;
use std::time::{Duration, Instant};

use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU,
    VK_RSHIFT, VK_RWIN,
};

/// `C`. Spelled out because the named constant is not in every `windows`
/// release, and this value is fixed by the platform.
const VK_C: VIRTUAL_KEY = VIRTUAL_KEY(0x43);
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
    let before = sequence_number();

    if let Err(err) = send_ctrl_c() {
        log::warn!("could not synthesise Ctrl+C: {err}");
        return None;
    }

    let grabbed = poll_for_new_clipboard_text(before);

    // Restore before returning, whatever happened, and let the backup drop at
    // the end of this scope — no long-lived copy of the user's clipboard.
    restore_clipboard(backup);

    grabbed.filter(|text| !text.trim().is_empty())
}

/// A user-requested clipboard write (the Popover's Copy). Not restored, by
/// design: the user asked for it (ADR-0002).
pub fn write_clipboard_text(text: &str) -> Result<(), String> {
    clipboard_win::set_clipboard_string(text).map_err(|e| e.to_string())
}

fn sequence_number() -> u32 {
    // Safe: no arguments, no handles, returns 0 if we lack clipboard access.
    unsafe { GetClipboardSequenceNumber() }
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

/// Poll the sequence number until it moves, then read. Returns `None` on
/// timeout: elevated windows never respond and must not hang the trigger.
fn poll_for_new_clipboard_text(before: u32) -> Option<String> {
    let deadline = Instant::now() + POLL_CAP;
    while Instant::now() < deadline {
        if sequence_number() != before {
            // The number changes when the owner *opens* the clipboard, which
            // can happen a beat before the data is readable; a failed read is
            // retried on the next tick.
            if let Some(text) = read_clipboard_text() {
                return Some(text);
            }
        }
        sleep(POLL_INTERVAL);
    }
    None
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

fn send_ctrl_c() -> Result<(), String> {
    send(&[
        key_input(VK_CONTROL_LEFT, KEYBD_EVENT_FLAGS(0)),
        key_input(VK_C, KEYBD_EVENT_FLAGS(0)),
        key_input(VK_C, KEYEVENTF_KEYUP),
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

    #[test]
    fn both_sides_of_every_modifier_are_released() {
        assert_eq!(HELD_MODIFIERS.len(), 8);
        assert!(HELD_MODIFIERS.contains(&VK_LWIN) && HELD_MODIFIERS.contains(&VK_RWIN));
    }
}
