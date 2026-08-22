//! Running the macOS screenshot tool and reading what it left on the pasteboard
//! (ADR-0016).
//!
//! `screencapture -i -c` is the same interactive selection Cmd+Shift+4 starts,
//! writing to the pasteboard instead of the desktop. Unlike the Windows verb it
//! is a child process, so this side has two things Windows does not:
//!
//! - it **blocks** until the user is done, so there is no polling and no cap —
//!   the wait ends when the tool exits;
//! - it reports a cancel. `screencapture` exits non-zero when the user presses
//!   Esc, and even where it does not, the pasteboard's `changeCount` has not
//!   moved. Both are checked, because only one of them is documented.
//!
//! ADR-0002's clipboard restore deliberately does not apply: nothing was
//! synthesised here, so the pasteboard write is the user's own action.

use std::process::Command;

use objc2_app_kit::{NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeTIFF};

use crate::platform::capture::{self, Outcome};

/// The tool's absolute path. Not looked up on `PATH`: this runs in a GUI process
/// whose environment is the launch agent's, not a shell's.
const SCREENCAPTURE: &str = "/usr/sbin/screencapture";

/// Run the screenshot tool and hand back what it produced. [`Outcome::Nothing`]
/// is Esc, or a tool that could not run — a phase, not an error.
pub fn grab_capture() -> Outcome {
    let before = change_count();

    // `-i` interactive selection, `-c` to the pasteboard. No `-x`: the shutter
    // sound is the OS telling the user the shot was taken.
    let status = match Command::new(SCREENCAPTURE).args(["-i", "-c"]).status() {
        Ok(status) => status,
        Err(err) => {
            log::warn!("could not run screencapture: {err}");
            return Outcome::Nothing;
        }
    };

    // Either signal is enough to call it cancelled: the exit code is not
    // documented, and an unchanged pasteboard cannot be a successful capture.
    if !status.success() || change_count() == before {
        return Outcome::Nothing;
    }

    let Some(bytes) = read_clipboard_image() else {
        return Outcome::Nothing;
    };
    match capture::from_clipboard_bytes(&bytes) {
        Ok(capture) => Outcome::Captured(capture),
        Err(error) => Outcome::Failed(error),
    }
}

/// The same counter the Selection grab polls, read once here rather than in a
/// loop: the child process is the thing that says when to look.
fn change_count() -> isize {
    NSPasteboard::generalPasteboard().changeCount()
}

/// PNG first, TIFF second. `screencapture -c` has written TIFF historically and
/// PNG on recent systems; asking for both is cheaper than depending on which.
fn read_clipboard_image() -> Option<Vec<u8>> {
    let pasteboard = NSPasteboard::generalPasteboard();
    // The pasteboard *type* constants are extern statics, hence the unsafe —
    // the calls themselves are safe (the same shape as `selection.rs`).
    let data = unsafe {
        pasteboard
            .dataForType(NSPasteboardTypePNG)
            .or_else(|| pasteboard.dataForType(NSPasteboardTypeTIFF))
    }?;
    Some(data.to_vec())
}
