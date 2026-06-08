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

/// Place a popup near a selection rectangle, flipping above the selection when
/// there is not enough room below it.
pub fn fit_near_rect(
    selection: Rect,
    gap: i32,
    popup_w: i32,
    popup_h: i32,
    screen: Rect,
) -> (i32, i32) {
    let x = fit_axis(selection.x, popup_w, screen.x, screen.x + screen.width);
    let below = selection.y + selection.height + gap;
    let above = selection.y - gap - popup_h;
    let y = fit_vertical(below, above, popup_h, screen);
    (x, y)
}

/// Place a popup near a cursor/caret point, flipping above the point when the
/// lower edge would leave the screen.
pub fn fit_near_point(
    x: i32,
    y: i32,
    gap: i32,
    popup_w: i32,
    popup_h: i32,
    screen: Rect,
) -> (i32, i32) {
    let fitted_x = fit_axis(x, popup_w, screen.x, screen.x + screen.width);
    let below = y + gap;
    let above = y - gap - popup_h;
    let fitted_y = fit_vertical(below, above, popup_h, screen);
    (fitted_x, fitted_y)
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

fn fit_axis(value: i32, popup_len: i32, start: i32, end: i32) -> i32 {
    let max = end - popup_len;
    value.clamp(start, max.max(start))
}

fn fit_vertical(below: i32, above: i32, popup_h: i32, screen: Rect) -> i32 {
    let screen_top = screen.y;
    let screen_bottom = screen.y + screen.height;
    if below + popup_h <= screen_bottom {
        return below.max(screen_top);
    }
    if above >= screen_top {
        return above;
    }
    fit_axis(below, popup_h, screen_top, screen_bottom)
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

    #[test]
    fn fit_near_rect_flips_above_bottom_edge() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let selection = Rect {
            x: 1500,
            y: 1000,
            width: 100,
            height: 20,
        };

        let (x, y) = fit_near_rect(selection, 12, 480, 320, screen);

        assert_eq!(x, 1440);
        assert_eq!(y, 668);
    }

    #[test]
    fn fit_near_point_flips_above_bottom_edge() {
        let screen = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        let (x, y) = fit_near_point(1800, 1040, 12, 480, 320, screen);

        assert_eq!(x, 1440);
        assert_eq!(y, 708);
    }
}
