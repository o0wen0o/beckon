//! What a Capture is, and the part of getting one that is not platform code
//! (ADR-0016).
//!
//! The platform halves under `windows/` and `macos/` do two things only: run the
//! OS snip tool, and hand back whatever bytes the clipboard ended up holding.
//! Everything after that is here, so the normalisation is written once and can
//! be unit-tested on either platform:
//!
//! - the clipboard's own format is never the wire format — Windows gives a BMP,
//!   macOS a TIFF — and the provider takes PNG, JPEG, GIF or WebP
//!   (<https://api-docs.deepseek.com/guides/vision/>);
//! - the request carries the image as a `data:` URL, because a Capture never
//!   reaches disk and so has no URL of its own (ADR-0004);
//! - a snip of a 6K display is a real image, so there is a size ceiling and it
//!   is checked before a request is built rather than after one is refused.

use std::io::Cursor;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::ImageFormat;
use serde::Serialize;

/// The ceiling on an encoded Capture.
///
/// The provider allows 32 MiB per image inside a 48 MiB request body, and base64
/// inflates by 4/3 — so 8 MiB encoded is comfortably inside both while still
/// admitting a full-screen snip of a 6K display. It is a Beckon limit, not the
/// provider's, and it exists so the refusal is a sentence in the reader's
/// language instead of a 413.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// A grabbed screenshot, normalised to PNG.
///
/// Serialised straight to the Popover: the window needs the same bytes to draw
/// the thumbnail that the request needs to send, so there is one copy and no
/// second, smaller preview.
#[derive(Debug, Clone, Serialize)]
pub struct Capture {
    /// `data:image/png;base64,…` — both the `<img src>` and the wire value.
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    /// Encoded PNG length, before base64. What the size ceiling is about, and
    /// what the Popover shows beside the thumbnail.
    pub bytes: usize,
}

/// Why a Capture that *was* taken cannot be used.
///
/// Shaped like `commands::Failure` on purpose — kind plus message — because it
/// reaches the Popover through the same path and is read by the same
/// `describeFailure`: the cause is named in the reader's language and the detail
/// is quoted verbatim.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureError {
    pub kind: String,
    pub message: String,
}

impl CaptureError {
    fn new(kind: &str, message: String) -> Self {
        Self {
            kind: kind.to_string(),
            message,
        }
    }
}

/// What one run of the snip tool produced.
///
/// Three arms, not `Option`: "the user pressed Esc" and "that screenshot is
/// 40 MB" are different things to say, and collapsing them is how the second one
/// would arrive as the first (which is a lie the user cannot act on).
#[derive(Debug)]
pub enum Outcome {
    Captured(Capture),
    /// Cancelled, or a tool that never answered. Not an error.
    Nothing,
    Failed(CaptureError),
}

/// Normalise whatever the clipboard held into a PNG Capture.
///
/// The format is guessed from the bytes rather than passed in: the Windows
/// clipboard offers a real PNG under a registered format *and* a BMP under
/// `CF_BITMAP`, macOS offers PNG or TIFF, and which one a given snip tool wrote
/// is not something the caller should have to claim.
///
/// A PNG in is still decoded and re-encoded. It costs a few tens of
/// milliseconds on a snip-sized image, and it buys the dimensions, one code
/// path, and a guarantee that what we send is a PNG we produced.
pub fn from_clipboard_bytes(bytes: &[u8]) -> Result<Capture, CaptureError> {
    let image = image::load_from_memory(bytes)
        .map_err(|e| CaptureError::new("capture-unreadable", e.to_string()))?;

    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|e| CaptureError::new("capture-unreadable", e.to_string()))?;
    let png = png.into_inner();

    if png.len() > MAX_BYTES {
        return Err(CaptureError::new(
            "capture-too-large",
            too_large_message(png.len()),
        ));
    }

    Ok(Capture {
        data_url: format!("data:image/png;base64,{}", STANDARD.encode(&png)),
        width: image.width(),
        height: image.height(),
        bytes: png.len(),
    })
}

/// Said in whole mebibytes: the numbers are megabyte-scale, and a byte count
/// is not something a reader can act on.
fn too_large_message(bytes: usize) -> String {
    let mib = |value: usize| (value as f64) / (1024.0 * 1024.0);
    format!(
        "the screenshot is {:.1} MB, over Beckon's {:.0} MB limit; capture a smaller region",
        mib(bytes),
        mib(MAX_BYTES)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×3 BMP, which is what the Windows clipboard hands over — the point
    /// being that what comes out is a PNG with the right dimensions either way.
    fn bmp(width: u32, height: u32) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        image::RgbImage::new(width, height)
            .write_to(&mut out, ImageFormat::Bmp)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn a_clipboard_bitmap_becomes_a_png_data_url() {
        let capture = from_clipboard_bytes(&bmp(2, 3)).unwrap();
        assert_eq!((capture.width, capture.height), (2, 3));
        assert!(capture.data_url.starts_with("data:image/png;base64,"));
        assert!(capture.bytes > 0);
    }

    /// The base64 payload has to decode back to a PNG, or the provider gets a
    /// `data:` URL it cannot read and answers with something unhelpful.
    #[test]
    fn the_data_url_decodes_to_png_bytes() {
        let capture = from_clipboard_bytes(&bmp(4, 4)).unwrap();
        let payload = capture
            .data_url
            .strip_prefix("data:image/png;base64,")
            .unwrap();
        let decoded = STANDARD.decode(payload).unwrap();
        assert_eq!(decoded.len(), capture.bytes);
        assert_eq!(image::guess_format(&decoded).unwrap(), ImageFormat::Png);
    }

    /// Two kinds, because the two have different advice attached: shrink the
    /// region, against something is wrong with the clipboard.
    #[test]
    fn an_unusable_clipboard_says_which_kind_of_unusable() {
        let err = from_clipboard_bytes(b"not an image at all").unwrap_err();
        assert_eq!(err.kind, "capture-unreadable");
        assert!(!err.message.is_empty());
    }

    /// The ceiling is stated in the message, so a user who hits it knows what
    /// to aim under.
    #[test]
    fn the_size_ceiling_names_both_numbers() {
        let message = too_large_message(9 * 1024 * 1024);
        assert!(message.contains("9.0 MB"), "{message}");
        assert!(message.contains("8 MB"), "{message}");
    }

    /// The kinds are read by the frontend catalogs, so they are part of the
    /// contract rather than log text.
    #[test]
    fn the_two_kinds_are_the_ones_the_catalogs_carry() {
        assert_eq!(
            CaptureError::new("capture-too-large", String::new()).kind,
            "capture-too-large"
        );
        assert_eq!(
            CaptureError::new("capture-unreadable", String::new()).kind,
            "capture-unreadable"
        );
    }
}
