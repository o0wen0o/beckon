//! The one piece of clipboard mechanics both Win32 grabs are built on.
//!
//! Two callers want the same thing and want it with different timings: the
//! Selection grab polls for 300ms after synthesising Ctrl+C (ADR-0002), the
//! Capture polls for 45 seconds while a person drags a rectangle (ADR-0016).
//! What is *identical* is the subtlety — the sequence number moves when the
//! owner opens the clipboard, which can be a beat before the data is readable —
//! and that is the part worth having exactly one copy of.

use std::thread::sleep;
use std::time::{Duration, Instant};

use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;

/// The clipboard's generation counter. Read before the write is provoked, then
/// polled: the alternative is a fixed sleep, which is either a race or a stall.
pub fn sequence_number() -> u32 {
    // Safe: no arguments, no handles, returns 0 if we lack clipboard access.
    unsafe { GetClipboardSequenceNumber() }
}

/// Poll the sequence number until it moves, then read. `None` on timeout, which
/// is a normal outcome for both callers: an elevated window never answers
/// Ctrl+C, and a snip the user cancelled never writes anything.
pub fn poll_until_written<T>(
    before: u32,
    cap: Duration,
    interval: Duration,
    read: impl Fn() -> Option<T>,
) -> Option<T> {
    let deadline = Instant::now() + cap;
    while Instant::now() < deadline {
        if sequence_number() != before {
            // The number changes when the owner *opens* the clipboard, which
            // can happen a beat before the data is readable; a failed read is
            // retried on the next tick.
            if let Some(value) = read() {
                return Some(value);
            }
        }
        sleep(interval);
    }
    None
}
