//! Helpers for positioning the popup window relative to a selection or cursor.

use crate::Rect;

/// Constrain a popup rectangle so it stays fully inside the given screen bounds.
///
/// `popup_w` / `popup_h` are the popup's desired size. `preferred_x` / `preferred_y`
/// are the top-left position before clamping (typically just under the selection or
/// at the cursor). `screen` is the target screen's bounds.
///
/// Returns a clamped `(x, y)` pair.
pub fn clamp_to_screen(
    preferred_x: i32,
    preferred_y: i32,
    popup_w: i32,
    popup_h: i32,
    screen: Rect,
) -> (i32, i32) {
    let screen_left = screen.x;
    let screen_top = screen.y;
    let screen_right = screen.x + screen.width;
    let screen_bottom = screen.y + screen.height;

    // Try to place the popup so the desired top-left is preserved, but slide it
    // in if it would overflow any edge.
    let mut x = preferred_x;
    let mut y = preferred_y;

    if x + popup_w > screen_right {
        x = screen_right - popup_w;
    }
    if y + popup_h > screen_bottom {
        // Place popup above the selection instead of below.
        y = preferred_y - popup_h;
    }
    if x < screen_left {
        x = screen_left;
    }
    if y < screen_top {
        y = screen_top;
    }

    (x, y)
}

/// Compute the preferred popup position: just below a selection rect, with
/// a small vertical gap.
pub fn below_rect(selection: Rect, gap: i32) -> (i32, i32) {
    (selection.x, selection.y + selection.height + gap)
}

/// Compute the preferred popup position: just above a selection rect, with
/// a small vertical gap.
pub fn above_rect(selection: Rect, gap: i32) -> (i32, i32) {
    (selection.x, selection.y - gap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_popup_inside_screen() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let (x, y) = clamp_to_screen(1800, 1000, 400, 300, screen);
        assert_eq!(x, 1520);
        assert_eq!(y, 1000 - 300);
    }

    #[test]
    fn clamp_handles_top_left_overflow() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let (x, y) = clamp_to_screen(-100, -50, 400, 300, screen);
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn below_rect_offsets_below() {
        let r = Rect {
            x: 100,
            y: 200,
            width: 50,
            height: 20,
        };
        assert_eq!(below_rect(r, 4), (100, 224));
    }
}
