//! Platform facade (ADR-0001, ADR-0013).
//!
//! Every Win32 call lives under `windows/` and every AppKit/CoreGraphics call
//! under `macos/`. The rest of the app calls the functions re-exported here, so
//! adding a third platform means adding a sibling directory, not chasing
//! `#[cfg]` through business logic.
//!
//! Three things are deliberately *not* per-platform: `cursor`, which Tauri
//! already normalises for us, `capture`, which is what to do with the bytes a
//! snip produced rather than how to produce them, and the geometry below. Those
//! are the parts of this layer that can be unit-tested.

use serde::Serialize;

pub mod capture;
pub mod cursor;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{focus, permission, selection, snip};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::{focus, permission, selection, snip};

#[cfg(not(any(windows, target_os = "macos")))]
mod fallback;
#[cfg(not(any(windows, target_os = "macos")))]
pub use self::fallback::{focus, permission, selection, snip};

/// Whether the OS will let Beckon synthesise the copy keystroke ADR-0002 is
/// built on.
///
/// Windows asks nobody, so it answers `NotRequired` rather than `Granted`: the
/// UI has to be able to say nothing about a permission that does not exist.
/// macOS gates `CGEventPost` behind Accessibility, and refuses it *silently* —
/// which is why this is surfaced instead of inferred from an empty grab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
// Every target constructs a proper subset: Windows and the fallback only ever
// answer `NotRequired`, macOS only ever `Granted` or `Denied`. The variants the
// current target cannot reach are still part of the shape the frontend switches
// on, and `NotRequired`'s only macOS construction is in `mod tests`, which the
// bin target's dead-code pass does not see — so the exemption is unconditional
// rather than one target's leftovers.
#[allow(dead_code)]
pub enum InputPermission {
    NotRequired,
    Granted,
    Denied,
}

/// A monitor's usable area in physical pixels (taskbar, Dock and menu bar
/// excluded).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Gap between the cursor and the Popover's top-left corner.
pub const CURSOR_OFFSET: i32 = 12;

/// Place a window of `width` × `height` next to the cursor, clamped so it stays
/// fully inside `area`. When the window does not fit below/right of the cursor
/// it flips to the other side rather than sliding under the pointer.
pub fn place_near_cursor(cursor: (i32, i32), size: (i32, i32), area: WorkArea) -> (i32, i32) {
    let (cx, cy) = cursor;
    let (width, height) = size;

    let mut x = cx + CURSOR_OFFSET;
    if x + width > area.x + area.width {
        x = cx - CURSOR_OFFSET - width;
    }
    let mut y = cy + CURSOR_OFFSET;
    if y + height > area.y + area.height {
        y = cy - CURSOR_OFFSET - height;
    }

    // A window larger than the work area still has to start on-screen.
    let x = x.clamp(area.x, (area.x + area.width - width).max(area.x));
    let y = y.clamp(area.y, (area.y + area.height - height).max(area.y));
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: WorkArea = WorkArea {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
    };

    /// The Popover's out-of-the-box size (`config::DEFAULT_POPOVER_W` ×
    /// `DEFAULT_POPOVER_H`) — the user can drag it to any other (ADR-0018),
    /// which is exactly why this function takes the size rather than reading it.
    /// It is pure, so a stale number here would never fail a test; it would just
    /// quietly stop describing the app.
    const POPOVER: (i32, i32) = (620, 500);
    /// A Popover dragged short. No phase produces one on its own any more
    /// (ADR-0020), but the user can, and the placement has to follow the height
    /// it is given rather than the one it started at.
    const POPOVER_SHORT: (i32, i32) = (620, 220);

    #[test]
    fn sits_below_right_of_the_cursor_when_it_fits() {
        assert_eq!(place_near_cursor((100, 100), POPOVER, SCREEN), (112, 112));
    }

    #[test]
    fn flips_instead_of_overflowing_the_edges() {
        let (x, y) = place_near_cursor((1900, 1030), POPOVER, SCREEN);
        assert_eq!((x, y), (1900 - 12 - 620, 1030 - 12 - 500));
    }

    /// A short Popover still fits below a cursor where the full one would have
    /// had to flip — the height it is placed with is the height it is, which is
    /// the whole reason this function takes a size.
    #[test]
    fn a_short_popover_fits_where_the_full_one_would_flip() {
        let (_, tall) = place_near_cursor((100, 600), POPOVER, SCREEN);
        let (_, short) = place_near_cursor((100, 600), POPOVER_SHORT, SCREEN);
        assert_eq!(tall, 600 - 12 - 500);
        assert_eq!(short, 612);
    }

    #[test]
    fn clamps_when_it_fits_on_neither_side() {
        let (x, y) = place_near_cursor((5, 5), (2000, 2000), SCREEN);
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn respects_a_secondary_monitor_origin() {
        let right = WorkArea {
            x: 1920,
            y: -200,
            width: 1280,
            height: 1000,
        };
        let (x, y) = place_near_cursor((3100, 700), POPOVER, right);
        assert!(x >= right.x && x + POPOVER.0 <= right.x + right.width);
        assert!(y >= right.y && y + POPOVER.1 <= right.y + right.height);
    }

    #[test]
    fn a_negative_origin_monitor_still_places_below_right() {
        let left = WorkArea {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1040,
        };
        assert_eq!(place_near_cursor((-1800, 100), POPOVER, left), (-1788, 112));
    }

    /// A macOS work area starts below the menu bar, so the origin is not the
    /// screen's — the same case as a secondary monitor, and the reason nothing
    /// here may assume `area.y == 0`.
    #[test]
    fn a_work_area_inset_from_the_screen_top_is_respected() {
        let mac = WorkArea {
            x: 0,
            y: 38,
            width: 1512,
            height: 892,
        };
        assert_eq!(place_near_cursor((10, 10), POPOVER, mac), (22, 38));
    }

    #[test]
    fn serialises_the_permission_with_a_discriminant() {
        let json = serde_json::to_string(&InputPermission::NotRequired).unwrap();
        assert_eq!(json, r#""not-required""#);
    }
}
