//! Platform facade (ADR-0001).
//!
//! Everything Win32 lives under `windows/`. The rest of the app calls the
//! functions re-exported here, so porting means adding a sibling directory, not
//! chasing `#[cfg]` through business logic.
//!
//! The geometry below is deliberately platform-free: it is the one part of this
//! layer that can be unit-tested.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use self::windows::{cursor, focus, selection};

#[cfg(not(windows))]
pub mod cursor {
    //! Non-Windows stub: the app is Windows-only for now (ADR-0001), but the
    //! code must still compile elsewhere so the isolation stays honest.
    use super::WorkArea;
    pub fn cursor_position() -> Option<(i32, i32)> {
        None
    }
    pub fn work_area_at(_x: i32, _y: i32) -> Option<WorkArea> {
        None
    }
}

#[cfg(not(windows))]
pub mod focus {
    pub fn foreground_window() -> Option<isize> {
        None
    }
    pub fn window_handle(_window: &tauri::WebviewWindow) -> Option<isize> {
        None
    }
    pub fn restore_foreground(_hwnd: isize) -> bool {
        false
    }
}

#[cfg(not(windows))]
pub mod selection {
    pub fn grab_selection() -> Option<String> {
        None
    }
    pub fn write_clipboard_text(_text: &str) -> Result<(), String> {
        Err("clipboard access is only implemented on Windows".to_string())
    }
}

/// A monitor's usable area in physical pixels (taskbar excluded).
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

    /// The Popover's real size (`trigger::POPOVER_W` × `POPOVER_H`). This
    /// function is pure, so a stale number here would never fail a test — it
    /// would just quietly stop describing the app.
    const POPOVER: (i32, i32) = (620, 500);
    /// The `empty-selection` Popover, which is deliberately shorter.
    const POPOVER_HINT: (i32, i32) = (620, 220);

    #[test]
    fn sits_below_right_of_the_cursor_when_it_fits() {
        assert_eq!(place_near_cursor((100, 100), POPOVER, SCREEN), (112, 112));
    }

    #[test]
    fn flips_instead_of_overflowing_the_edges() {
        let (x, y) = place_near_cursor((1900, 1030), POPOVER, SCREEN);
        assert_eq!((x, y), (1900 - 12 - 620, 1030 - 12 - 500));
    }

    /// The shorter hint window still fits below a cursor where the full one
    /// would have had to flip.
    #[test]
    fn the_hint_sized_popover_fits_where_the_full_one_would_flip() {
        let (_, tall) = place_near_cursor((100, 600), POPOVER, SCREEN);
        let (_, short) = place_near_cursor((100, 600), POPOVER_HINT, SCREEN);
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
}
