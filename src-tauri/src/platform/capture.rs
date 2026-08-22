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
//!
//! What is deliberately *not* here is the sentence a person reads about a
//! [`Fault`]: this module has no `Language` in reach, and English prose written
//! one layer below the one that does is how a Chinese reader ends up reading
//! half a message (ADR-0015). A Fault carries the *fact*; `trigger` phrases it.

use std::io::Cursor;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::ImageFormat;
use serde::Serialize;

/// The `data:` URL prefix every Capture carries. Named because the encoder
/// writes straight into the buffer behind it, and the tests strip it back off.
const DATA_URL_PREFIX: &str = "data:image/png;base64,";

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

/// Why a Capture that *was* taken cannot be used — the fact, not the sentence.
///
/// Two arms, because the two have different advice attached: shrink the region,
/// against something is wrong with the clipboard. `trigger::describe_fault`
/// turns either into the kind-plus-message pair the Popover reads.
#[derive(Debug)]
pub enum Fault {
    /// Over [`MAX_BYTES`]. Carries the encoded length, because the sentence
    /// names both numbers and only `i18n` knows how to say them.
    TooLarge { bytes: usize },
    /// Bytes no decoder recognised. Carries the decoder's own words, which are
    /// a cause quoted verbatim rather than something Beckon phrases.
    Unreadable(String),
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
    Failed(Fault),
}

impl Outcome {
    /// Normalise the bytes a snip tool left behind, or say why not.
    ///
    /// The tail both platform halves end with, so "an empty clipboard is a
    /// phase, unusable bytes are a Fault" is decided in one place.
    pub fn from_clipboard(bytes: Option<Vec<u8>>) -> Self {
        let Some(bytes) = bytes else {
            return Outcome::Nothing;
        };
        match from_clipboard_bytes(&bytes) {
            Ok(capture) => Outcome::Captured(capture),
            Err(fault) => Outcome::Failed(fault),
        }
    }
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
fn from_clipboard_bytes(bytes: &[u8]) -> Result<Capture, Fault> {
    let image = image::load_from_memory(bytes).map_err(|e| Fault::Unreadable(e.to_string()))?;

    // Pre-sized off what the clipboard held: a snip-sized encode into an empty
    // `Vec` is twenty reallocations, each copying megabytes.
    let mut png = Cursor::new(Vec::with_capacity(bytes.len()));
    image
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|e| Fault::Unreadable(e.to_string()))?;
    let png = png.into_inner();

    if png.len() > MAX_BYTES {
        return Err(Fault::TooLarge { bytes: png.len() });
    }

    // Encoded into the finished `data:` URL rather than into a string of its
    // own: the base64 of a snip is megabytes, and `format!` over it would
    // allocate and copy the whole thing a second time.
    let mut data_url = String::with_capacity(DATA_URL_PREFIX.len() + png.len().div_ceil(3) * 4);
    data_url.push_str(DATA_URL_PREFIX);
    STANDARD.encode_string(&png, &mut data_url);

    Ok(Capture {
        data_url,
        width: image.width(),
        height: image.height(),
        bytes: png.len(),
    })
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
        assert!(capture.data_url.starts_with(DATA_URL_PREFIX));
        assert!(capture.bytes > 0);
    }

    /// The base64 payload has to decode back to a PNG, or the provider gets a
    /// `data:` URL it cannot read and answers with something unhelpful.
    #[test]
    fn the_data_url_decodes_to_png_bytes() {
        let capture = from_clipboard_bytes(&bmp(4, 4)).unwrap();
        let payload = capture.data_url.strip_prefix(DATA_URL_PREFIX).unwrap();
        let decoded = STANDARD.decode(payload).unwrap();
        assert_eq!(decoded.len(), capture.bytes);
        assert_eq!(image::guess_format(&decoded).unwrap(), ImageFormat::Png);
    }

    /// The decoder's own words travel with the Fault, because they are the
    /// cause the Popover quotes verbatim.
    #[test]
    fn unreadable_bytes_carry_the_decoders_own_words() {
        match from_clipboard_bytes(b"not an image at all").unwrap_err() {
            Fault::Unreadable(detail) => assert!(!detail.is_empty()),
            other => panic!("expected Unreadable, got {other:?}"),
        }
    }

    /// The tail both platform halves share: nothing on the clipboard is a
    /// phase, bytes that will not decode are a Fault.
    #[test]
    fn nothing_on_the_clipboard_is_nothing_rather_than_a_fault() {
        assert!(matches!(Outcome::from_clipboard(None), Outcome::Nothing));
        assert!(matches!(
            Outcome::from_clipboard(Some(bmp(1, 1))),
            Outcome::Captured(_)
        ));
        assert!(matches!(
            Outcome::from_clipboard(Some(b"junk".to_vec())),
            Outcome::Failed(Fault::Unreadable(_))
        ));
    }
}
