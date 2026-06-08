//! Popup positioning: place the floating popup near the selection or cursor,
//! clamped to the active screen bounds.

#[cfg(test)]
use translator_platform::SelectionError;
use translator_platform::{position, Rect, SelectionMonitor};

/// Default popup size when the frontend doesn't override it.
pub const DEFAULT_POPUP_W: i32 = 480;
pub const DEFAULT_POPUP_H: i32 = 320;
/// Vertical gap between the selection/cursor and the popup, in pixels.
/// BH-5.3: SPEC requires 12 px margin below the selection (or cursor).
pub const POPUP_GAP: i32 = 12;

/// Compute the top-left screen position for the popup window.
///
/// Resolution order:
/// 1. Selection bounds (preferred — popup appears just below the highlighted text).
/// 2. Cursor position (fallback — popup appears just below the caret).
/// 3. Centered on `screen` (last resort — neither is available).
///
/// The result is always clamped to be fully visible inside the chosen screen.
pub async fn compute_popup_position(
    monitor: &dyn SelectionMonitor,
    screens: &[Rect],
) -> (i32, i32) {
    // Try selection bounds first.
    if let Ok(Some(sel)) = monitor.selection_bounds().await {
        let screen = screen_for_rect(sel, screens);
        return position::fit_near_rect(sel, POPUP_GAP, DEFAULT_POPUP_W, DEFAULT_POPUP_H, screen);
    }
    // Fall back to cursor position.
    if let Ok((cx, cy)) = monitor.cursor_position().await {
        let screen = screen_for_point(cx, cy, screens);
        return position::fit_near_point(
            cx,
            cy,
            POPUP_GAP,
            DEFAULT_POPUP_W,
            DEFAULT_POPUP_H,
            screen,
        );
    }
    // Last resort: centered.
    let screen = screens.first().copied().unwrap_or_else(fallback_screen);
    let x = screen.x + (screen.width - DEFAULT_POPUP_W) / 2;
    let y = screen.y + (screen.height - DEFAULT_POPUP_H) / 2;
    (x.max(screen.x), y.max(screen.y))
}

fn screen_for_rect(rect: Rect, screens: &[Rect]) -> Rect {
    let center_x = rect.x + rect.width / 2;
    let center_y = rect.y + rect.height / 2;
    screen_for_point(center_x, center_y, screens)
}

fn screen_for_point(x: i32, y: i32, screens: &[Rect]) -> Rect {
    screens
        .iter()
        .copied()
        .find(|screen| {
            x >= screen.x
                && x < screen.x + screen.width
                && y >= screen.y
                && y < screen.y + screen.height
        })
        .or_else(|| screens.first().copied())
        .unwrap_or_else(fallback_screen)
}

fn fallback_screen() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    /// Mock monitor that returns the configured selection bounds / cursor.
    struct MockMonitor {
        bounds: Option<Rect>,
        cursor: Result<(i32, i32), String>,
    }
    #[async_trait]
    impl SelectionMonitor for MockMonitor {
        async fn get_selected_text(&self) -> Result<Option<String>, SelectionError> {
            Ok(None)
        }
        async fn selection_bounds(&self) -> Result<Option<Rect>, SelectionError> {
            Ok(self.bounds)
        }
        async fn cursor_position(&self) -> Result<(i32, i32), SelectionError> {
            match &self.cursor {
                Ok(c) => Ok(*c),
                Err(e) => Err(SelectionError::Platform(e.clone())),
            }
        }
        fn is_permission_granted(&self) -> bool {
            true
        }
        async fn open_permission_settings(&self) -> Result<(), SelectionError> {
            Ok(())
        }
    }

    fn full_hd_screens() -> Vec<Rect> {
        vec![Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }]
    }

    #[tokio::test]
    async fn uses_selection_bounds_when_available() {
        let monitor: Arc<dyn SelectionMonitor> = Arc::new(MockMonitor {
            bounds: Some(Rect {
                x: 100,
                y: 200,
                width: 80,
                height: 20,
            }),
            cursor: Ok((0, 0)),
        });
        // (x, y) should be (100, 200 + 20 + 12) = (100, 232), then clamped to 1920x1080.
        let (x, y) = compute_popup_position(monitor.as_ref(), &full_hd_screens()).await;
        assert_eq!(x, 100);
        assert_eq!(y, 232);
    }

    #[tokio::test]
    async fn falls_back_to_cursor_when_no_selection() {
        let monitor: Arc<dyn SelectionMonitor> = Arc::new(MockMonitor {
            bounds: None,
            cursor: Ok((500, 400)),
        });
        let (x, y) = compute_popup_position(monitor.as_ref(), &full_hd_screens()).await;
        assert_eq!(x, 500);
        assert_eq!(y, 400 + POPUP_GAP);
    }

    #[tokio::test]
    async fn centers_on_screen_when_nothing_available() {
        let monitor: Arc<dyn SelectionMonitor> = Arc::new(MockMonitor {
            bounds: None,
            cursor: Err("not impl".to_string()),
        });
        let (x, y) = compute_popup_position(monitor.as_ref(), &full_hd_screens()).await;
        // Centered: (1920-480)/2 = 720, (1080-320)/2 = 380
        assert_eq!(x, 720);
        assert_eq!(y, 380);
    }

    #[tokio::test]
    async fn clamps_to_screen_bottom_edge() {
        // Selection near the bottom of the screen -> popup should jump above the cursor
        // because `clamp_to_screen` flips to above on bottom overflow.
        let monitor: Arc<dyn SelectionMonitor> = Arc::new(MockMonitor {
            bounds: Some(Rect {
                x: 1500,
                y: 1000,
                width: 100,
                height: 20,
            }),
            cursor: Ok((0, 0)),
        });
        let (x, y) = compute_popup_position(monitor.as_ref(), &full_hd_screens()).await;
        // 1500 + 480 = 1980 > 1920 -> x clamped to 1440
        assert_eq!(x, 1440);
        // Not enough room below: flip above selection with the same 12 px gap.
        assert_eq!(y, 668);
    }

    #[tokio::test]
    async fn chooses_screen_containing_selection_center() {
        let monitor: Arc<dyn SelectionMonitor> = Arc::new(MockMonitor {
            bounds: Some(Rect {
                x: 2100,
                y: 980,
                width: 100,
                height: 20,
            }),
            cursor: Ok((0, 0)),
        });
        let screens = vec![
            Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            Rect {
                x: 1920,
                y: 0,
                width: 1280,
                height: 1024,
            },
        ];

        let (x, y) = compute_popup_position(monitor.as_ref(), &screens).await;

        assert_eq!(x, 2100);
        assert_eq!(y, 648);
    }
}
