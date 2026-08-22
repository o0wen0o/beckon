//! Running the Windows snip tool and reading what it left on the clipboard
//! (ADR-0016).
//!
//! Step for step this is the Selection grab with two differences, and both are
//! consequences of the snip being *interactive*:
//!
//! 1. Nothing is synthesised. `ms-screenclip:` is the shell verb Win+Shift+S
//!    fires, so the tool that opens is the one the user already knows, and the
//!    image lands on the clipboard because that is what it does — not because
//!    we sent a keystroke (which is why ADR-0002's clipboard restore does not
//!    apply here: the write is the user's own).
//! 2. The poll cap is [`SNIP_CAP`], not 300ms. A person is dragging a
//!    rectangle; the wait is theirs, not the network's.
//!
//! The polling itself is [`clipboard::poll_until_written`], the same code the
//! Selection grab runs — only the two constants and the reader differ, which is
//! exactly what ADR-0016 means by "the way the Selection grab does".
//!
//! `ms-screenclip:` reports nothing back — not whether it opened, not whether
//! the user pressed Esc. A cancelled snip is therefore indistinguishable from a
//! slow one until the cap runs out, which is the whole reason the cap is
//! generous and the Popover comes back with a plain "nothing was captured".

use std::process::Command;
use std::time::Duration;

use super::clipboard;
use crate::platform::capture::Outcome;

/// How long the user gets to drag a rectangle before we give up waiting.
const SNIP_CAP: Duration = Duration::from_secs(45);
/// Slower than the Selection's 5ms: nothing here is on the hot path, and the
/// wait is measured in seconds.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Run the snip tool and hand back what it produced.
///
/// [`Outcome::Nothing`] covers a cancelled snip, a timeout, and a shell that
/// refused the verb: all three left nothing on the clipboard, which is a phase
/// rather than an error.
pub fn grab_capture() -> Outcome {
    let before = clipboard::sequence_number();

    if let Err(err) = launch_snip() {
        log::warn!("could not open the screen snip tool: {err}");
        return Outcome::Nothing;
    }

    Outcome::from_clipboard(clipboard::poll_until_written(
        before,
        SNIP_CAP,
        POLL_INTERVAL,
        read_clipboard_image,
    ))
}

/// Through the shell, not through `SnippingTool.exe`: the executable's name and
/// arguments have changed twice across Windows versions, and the protocol
/// handler is the documented way in. `explorer.exe` is what resolves it, which
/// is also why this returns as soon as the tool is *launched*.
fn launch_snip() -> Result<(), String> {
    Command::new("explorer.exe")
        .arg("ms-screenclip:")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// The registered `PNG` format is preferred over `CF_BITMAP`: the snip tools
/// offer both, and the PNG is the tool's own encoding rather than a DIB we would
/// re-encode from. Neither is guaranteed, hence the fall-through.
fn read_clipboard_image() -> Option<Vec<u8>> {
    read_clipboard_png().or_else(read_clipboard_bitmap)
}

fn read_clipboard_png() -> Option<Vec<u8>> {
    let format = clipboard_win::raw::register_format("PNG")?;
    clipboard_win::get_clipboard(clipboard_win::formats::RawData(format.get())).ok()
}

fn read_clipboard_bitmap() -> Option<Vec<u8>> {
    clipboard_win::get_clipboard(clipboard_win::formats::Bitmap).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Selection's cap is 300ms because an app either answers a keystroke or
    /// does not. This one is a person with a mouse, and the two now share a
    /// poll loop, so only the constants keep them apart.
    #[test]
    fn the_snip_wait_is_a_human_wait() {
        assert!(SNIP_CAP >= Duration::from_secs(30));
        assert!(POLL_INTERVAL < Duration::from_secs(1));
    }
}
