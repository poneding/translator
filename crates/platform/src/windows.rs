//! Windows [`SelectionMonitor`] implementation.
//!
//! Uses `IUIAutomation` via the `windows` crate. See docs/DESIGN.md.
//!
//! Pipeline:
//! 1. Initialize COM (apartment-threaded) — done once per process.
//! 2. `CoCreateInstance(CLSID_CUIAutomation)` → `IUIAutomation`
//! 3. `pAutomation->GetFocusedElement(&element)` → `IUIAutomationElement`
//! 4. `element->GetCurrentPattern(UIA_TextPatternId, &pattern)` → `IUIAutomationTextPattern`
//! 5. `pattern->GetSelection(&ranges)` → `IUIAutomationTextRangeArray`
//! 6. `range->GetText(max, &bstr)` → selection string
//! 7. Optional future hook: `range->GetBoundingRectangles(&rects)` → screen coordinates
//!
//! No special permission is required by default on Windows.

use async_trait::async_trait;
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
    IUIAutomationTextRangeArray, UIA_TextPatternId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_KEYUP, VK_C, VK_CONTROL, keybd_event};
use windows::Win32::UI::WindowsAndMessaging::GetPhysicalCursorPos;
use windows::core::{BSTR, Interface};

use crate::{Rect, SelectionError, SelectionMonitor};

// ---------------------------------------------------------------------------
// Pure helpers (testable without COM / FFI)
// ---------------------------------------------------------------------------

/// Placeholder that always returns `None`; the `IUIAutomation` SAFEARRAY
/// parsing path is tracked as a follow-up (see module docs).
#[allow(dead_code)] // kept as the documented future hook for v2
fn rect_from_uia_slice(_rects: &[f64]) -> Option<Rect> {
    None
}

/// Convert a `BSTR` to an owned `String` (UTF-16 LE → UTF-8, lossy).
fn bstr_to_string(bstr: &BSTR) -> String {
    String::from_utf16_lossy(bstr)
}

// ---------------------------------------------------------------------------
// COM plumbing
// ---------------------------------------------------------------------------

/// COM is initialized exactly once per process (apartment-threaded).
static COM_INIT: OnceLock<Result<(), String>> = OnceLock::new();

fn ensure_com_initialized() -> Result<(), SelectionError> {
    let cached = COM_INIT.get_or_init(|| unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // S_OK == 0, S_FALSE == 1 (already initialized). Both are fine.
        if hr.0 == 0 || hr.0 == 1 {
            Ok(())
        } else {
            Err(format!("CoInitializeEx failed: hr=0x{:08x}", hr.0))
        }
    });
    cached.clone().map_err(SelectionError::Platform)
}

fn map_com_error(e: windows::core::Error) -> SelectionError {
    SelectionError::Platform(format!("UIA error: {e}"))
}

/// Borrow the focused element's `IUIAutomationTextPattern`, ready to use.
fn focused_text_pattern() -> Result<IUIAutomationTextPattern, SelectionError> {
    ensure_com_initialized()?;
    let automation: IUIAutomation = unsafe {
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(map_com_error)?
    };
    let element = unsafe { automation.GetFocusedElement().map_err(map_com_error)? };
    let pattern = unsafe {
        element
            .GetCurrentPattern(UIA_TextPatternId)
            .map_err(map_com_error)?
    };
    pattern
        .cast()
        .map_err(|e| SelectionError::Platform(format!("UIA cast: {e}")))
}

/// Read the selected text and the first bounding rectangle from the focused element.
fn read_focused_selection() -> Result<(Option<String>, Option<Rect>), SelectionError> {
    let text_pattern = focused_text_pattern()?;
    let ranges: IUIAutomationTextRangeArray =
        unsafe { text_pattern.GetSelection().map_err(map_com_error)? };
    let length = unsafe { ranges.Length().map_err(map_com_error)? };
    if length == 0 {
        return Ok((None, None));
    }
    let range: IUIAutomationTextRange = unsafe { ranges.GetElement(0).map_err(map_com_error)? };
    // Text
    let bstr: BSTR = unsafe { range.GetText(100_000).map_err(map_com_error)? };
    let text = bstr_to_string(&bstr);
    let text = if text.is_empty() { None } else { Some(text) };
    // Bounds: SAFEARRAY-based bounding-rectangle parsing is deferred. The
    // v0.2 main-window flow no longer uses selection coordinates.
    Ok((text, None))
}

/// Read selection by asking the frontmost application to copy it.
///
/// UI Automation returns empty ranges in several desktop apps even when text is
/// visibly selected. Simulated Ctrl+C is the same fallback Easydict uses on
/// macOS: it is less precise, so it only runs after UIA fails or returns empty.
fn read_selection_by_clipboard() -> Result<Option<String>, SelectionError> {
    let previous_text = clipboard_win::get_clipboard_string().ok();
    let previous_seq = clipboard_win::seq_num().map(|num| num.get());

    send_ctrl_c();
    let copied = wait_for_clipboard_text(previous_seq);

    if let Some(text) = previous_text {
        let _ = clipboard_win::set_clipboard_string(&text);
    }

    copied.map(|text| {
        text.and_then(|text| {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    })
}

fn send_ctrl_c() {
    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, Default::default(), 0);
        keybd_event(VK_C.0 as u8, 0, Default::default(), 0);
        keybd_event(VK_C.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL.0 as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

fn wait_for_clipboard_text(previous_seq: Option<u32>) -> Result<Option<String>, SelectionError> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(350) {
        thread::sleep(Duration::from_millis(25));

        if let Some(previous_seq) = previous_seq
            && clipboard_win::seq_num().map(|num| num.get()) == Some(previous_seq)
        {
            continue;
        }

        return match clipboard_win::get_clipboard_string() {
            Ok(text) => Ok(Some(text)),
            Err(error) => Err(SelectionError::Platform(format!("clipboard copy: {error}"))),
        };
    }

    Ok(None)
}

// ---------------------------------------------------------------------------
// SelectionMonitor impl
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// SelectionMonitor impl
// ---------------------------------------------------------------------------

/// Windows-specific selection monitor.
pub struct WindowsSelection {
    _marker: (),
}

impl WindowsSelection {
    /// Construct a new monitor.
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

impl Default for WindowsSelection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SelectionMonitor for WindowsSelection {
    async fn get_selected_text(&self) -> Result<Option<String>, SelectionError> {
        let res = tokio::task::spawn_blocking(|| match read_focused_selection() {
            Ok((Some(text), bounds)) => Ok((Some(text), bounds)),
            Ok((None, bounds)) => Ok((read_selection_by_clipboard()?, bounds)),
            Err(error) => match read_selection_by_clipboard() {
                Ok(text) => Ok((text, None)),
                Err(_) => Err(error),
            },
        })
        .await
        .map_err(|e| SelectionError::Platform(format!("join: {e}")))??;
        Ok(res.0)
    }

    async fn selection_bounds(&self) -> Result<Option<Rect>, SelectionError> {
        let res = tokio::task::spawn_blocking(read_focused_selection)
            .await
            .map_err(|e| SelectionError::Platform(format!("join: {e}")))??;
        Ok(res.1)
    }

    async fn cursor_position(&self) -> Result<(i32, i32), SelectionError> {
        tokio::task::spawn_blocking(|| -> Result<(i32, i32), SelectionError> {
            let mut point = POINT { x: 0, y: 0 };
            unsafe {
                GetPhysicalCursorPos(&mut point)
                    .map_err(|e| SelectionError::Platform(format!("GetPhysicalCursorPos: {e}")))?;
            }
            Ok((point.x, point.y))
        })
        .await
        .map_err(|e| SelectionError::Platform(format!("join: {e}")))?
    }

    fn is_permission_granted(&self) -> bool {
        // No special permission needed on Windows for UIA.
        true
    }

    async fn open_permission_settings(&self) -> Result<(), SelectionError> {
        // No-op on Windows.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests — cover the pure helpers (COM/UIA itself is only testable on a
// real Windows desktop session).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_uia_slice_returns_none_for_v1() {
        // Documented no-op; cursor fallback handles positioning.
        assert!(rect_from_uia_slice(&[10.0, 20.0, 100.0, 30.0]).is_none());
        assert!(rect_from_uia_slice(&[]).is_none());
        assert!(rect_from_uia_slice(&[1.0, 2.0, 3.0]).is_none());
    }

    #[test]
    fn bstr_ascii_round_trip() {
        let b = BSTR::from("hello");
        assert_eq!(bstr_to_string(&b), "hello");
    }

    #[test]
    fn bstr_unicode_round_trip() {
        let b = BSTR::from("你好");
        assert_eq!(bstr_to_string(&b), "你好");
    }

    #[test]
    fn bstr_empty() {
        let b = BSTR::from("");
        assert_eq!(bstr_to_string(&b), "");
    }
}
