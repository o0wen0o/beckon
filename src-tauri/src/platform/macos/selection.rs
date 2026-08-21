//! Grabbing the Selection by simulating Cmd+C (ADR-0002, ADR-0013).
//!
//! Step for step this is the Windows grab with three substitutions —
//! `CGEventPost` for `SendInput`, `NSPasteboard` for the Win32 clipboard, and
//! the pasteboard's `changeCount` for `GetClipboardSequenceNumber`, which is
//! the same counter under a different name. The order is load-bearing for the
//! same reasons:
//!
//! 1. Force the synthetic event's modifier flags to Command *exactly*. The
//!    trigger hotkey is still physically down, so an event that inherited the
//!    live flags would arrive as Cmd+Shift+C or worse. The physically-held
//!    non-Command modifiers are released as well, because a target app can read
//!    the hardware state rather than the event's.
//! 2. Back up the pasteboard, plain text only (ADR-0002 accepts losing rich
//!    text and images).
//! 3. Read `changeCount`, then post Cmd+C.
//! 4. **Poll** `changeCount` rather than sleeping a fixed interval.
//! 5. Read the text, restore the backup, and drop the backup immediately.
//! 6. Timeout or an empty result is `None`, not an error. Without Accessibility
//!    trust macOS drops the event silently and this is the path that runs —
//!    which is why `permission::input_permission` exists to say so out loud.

use std::thread::sleep;
use std::time::{Duration, Instant};

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;

/// `kVK_ANSI_C`, fixed by the platform.
const KEY_C: CGKeyCode = 8;
/// `kVK_Shift`, `kVK_RightShift`, `kVK_Control`, `kVK_RightControl`,
/// `kVK_Option`, `kVK_RightOption`. Command is absent on purpose: it is the one
/// modifier the synthetic event asserts.
const HELD_MODIFIERS: [(CGEventFlags, [CGKeyCode; 2]); 3] = [
    (CGEventFlags::CGEventFlagShift, [56, 60]),
    (CGEventFlags::CGEventFlagControl, [59, 62]),
    (CGEventFlags::CGEventFlagAlternate, [58, 61]),
];

/// How long the target app gets to see the modifier key-ups before Cmd+C
/// arrives. Not the pasteboard wait — that one is polled.
const MODIFIER_SETTLE: Duration = Duration::from_millis(15);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const POLL_CAP: Duration = Duration::from_millis(300);

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    /// `CGEventSourceFlagsState`. Not in `core-graphics` 0.24, and the only way
    /// to ask which modifiers the user is physically holding.
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
}

/// Grab the Selection. `None` means "there was nothing to grab" — a normal
/// outcome, not a failure.
pub fn grab_selection() -> Option<String> {
    release_held_modifiers();
    sleep(MODIFIER_SETTLE);

    let backup = read_clipboard_text();
    let before = change_count();

    if let Err(err) = send_command_c() {
        log::warn!("could not synthesise Cmd+C: {err}");
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
    let pasteboard = NSPasteboard::generalPasteboard();
    let _ = pasteboard.clearContents();
    let written =
        unsafe { pasteboard.setString_forType(&NSString::from_str(text), NSPasteboardTypeString) };
    if written {
        Ok(())
    } else {
        Err("the pasteboard refused the text".to_string())
    }
}

/// The pasteboard's own change counter — the exact analogue of Win32's
/// clipboard sequence number, and the reason neither platform has to sleep.
fn change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}

fn read_clipboard_text() -> Option<String> {
    let pasteboard = NSPasteboard::generalPasteboard();
    let text = unsafe { pasteboard.stringForType(NSPasteboardTypeString) }?;
    Some(text.to_string())
}

fn restore_clipboard(backup: Option<String>) {
    // With no text backup there was nothing of ours to give back; clearing at
    // least avoids leaving the grabbed Selection sitting in the pasteboard.
    let Some(text) = backup else {
        let _ = NSPasteboard::generalPasteboard().clearContents();
        return;
    };
    // Through the same write the Popover's Copy uses, so the grab's restore
    // cannot diverge from it on how a pasteboard write is performed.
    if let Err(err) = write_clipboard_text(&text) {
        log::warn!("could not restore the pasteboard: {err}");
    }
}

/// Poll the change counter until it moves, then read. Returns `None` on
/// timeout: an app that never answers must not hang the trigger.
fn poll_for_new_clipboard_text(before: isize) -> Option<String> {
    let deadline = Instant::now() + POLL_CAP;
    while Instant::now() < deadline {
        if change_count() != before {
            // The counter moves when the owner *declares* the new types, which
            // can happen a beat before the string is readable; a failed read is
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
    // `CombinedSessionState` is the live hardware state, which is what "the
    // user is still holding the hotkey" means.
    let live = CGEventFlags::from_bits_truncate(unsafe {
        CGEventSourceFlagsState(CGEventSourceStateID::CombinedSessionState as i32)
    });

    for (flag, keys) in HELD_MODIFIERS {
        if !live.contains(flag) {
            continue;
        }
        // The flags say which modifier is down, never which side, so both are
        // released; a key-up for a key that was never down is ignored.
        for key in keys {
            if let Err(err) = post(key, false, CGEventFlags::CGEventFlagNull) {
                log::warn!("could not release a held modifier: {err}");
            }
        }
    }
}

fn send_command_c() -> Result<(), String> {
    post(KEY_C, true, CGEventFlags::CGEventFlagCommand)?;
    post(KEY_C, false, CGEventFlags::CGEventFlagCommand)?;
    Ok(())
}

/// One key event with its flags set *explicitly*. Setting them is what stops
/// the live modifier state being merged into the event.
fn post(key: CGKeyCode, down: bool, flags: CGEventFlags) -> Result<(), String> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|()| "could not create a CGEventSource".to_string())?;
    let event = CGEvent::new_keyboard_event(source, key, down)
        .map_err(|()| format!("could not create a key event for {key}"))?;
    event.set_flags(flags);
    // The HID tap is the bottom of the stack, so the event reaches whatever has
    // focus rather than only the session's own taps.
    event.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clipboard_wait_is_polled_not_slept() {
        assert!(POLL_INTERVAL < POLL_CAP);
        assert!(POLL_CAP <= Duration::from_millis(500));
    }

    /// Command is the one modifier the grab asserts, so releasing it here would
    /// be undoing the event's own flags.
    #[test]
    fn command_is_not_among_the_released_modifiers() {
        assert_eq!(HELD_MODIFIERS.len(), 3);
        assert!(HELD_MODIFIERS
            .iter()
            .all(|(flag, _)| *flag != CGEventFlags::CGEventFlagCommand));
    }

    #[test]
    fn both_sides_of_every_released_modifier_are_covered() {
        for (_, keys) in HELD_MODIFIERS {
            assert_ne!(keys[0], keys[1]);
        }
    }
}
