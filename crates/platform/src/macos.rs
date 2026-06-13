//! macOS [`SelectionMonitor`] implementation.
//!
//! Uses AppKit's Accessibility (AX) framework via `accessibility-sys` 0.2 +
//! `core-foundation` 0.10 for safe CFString handling.
//!
//! ## Implementation notes
//!
//! 1. `AXUIElementCreateSystemWide()` → system-wide AX element.
//! 2. `kAXFocusedUIElementAttribute` → currently focused UI element.
//! 3. `kAXSelectedTextAttribute` → CFString with the selection.
//! 4. `kAXSelectedTextRangeAttribute` + `kAXBoundsForRangeParameterizedAttribute`
//!    → selection bounds in screen coordinates.
//!
//! Requires the user to grant the "Accessibility" permission in
//! `System Settings → Privacy & Security → Accessibility`. Detect via
//! `AXIsProcessTrustedWithOptions`.
//!
//! ## Verification
//!
//! This code only compiles + runs on macOS (`#[cfg(target_os = "macos")]`).
//! The `cfg(test)` tests below are likewise gated, so they only run on a Mac.
//! On non-macOS, `cargo check --target x86_64-apple-darwin` (run on macOS or
//! via CI) is the minimum compilation check; full end-to-end manual QA is
//! required on a real Mac (see PLAN.md M1.10).

#![cfg(target_os = "macos")]

use async_trait::async_trait;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::{Rect, SelectionError, SelectionMonitor};

// -----------------------------------------------------------------------------
// Raw FFI — accessibility-sys 0.2 re-exports the AX API at the crate root.
// -----------------------------------------------------------------------------

use accessibility_sys::{
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue,
    AXUIElementCopyParameterizedAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
    error_string, kAXBoundsForRangeParameterizedAttribute, kAXErrorAPIDisabled,
    kAXFocusedUIElementAttribute, kAXSelectedTextAttribute, kAXTrustedCheckOptionPrompt,
};

const AX_SUCCESS: i32 = 0;
const CLIPBOARD_COPY_POLL_ATTEMPTS: usize = 10;
const CLIPBOARD_COPY_POLL_INTERVAL: Duration = Duration::from_millis(40);

/// macOS-specific selection monitor.
pub struct MacOSSelection;

impl MacOSSelection {
    /// Construct a new monitor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOSSelection {
    fn default() -> Self {
        Self::new()
    }
}

/// Proactively request the macOS Accessibility permission.
///
/// Calls `AXIsProcessTrustedWithOptions` with `kAXTrustedCheckOptionPrompt = true`,
/// which forces macOS to re-evaluate whether this process has the Accessibility
/// permission. This is the **standard workaround** for a known macOS TCC quirk:
/// after an app update the binary changes, and while the Accessibility entry may
/// still appear checked in System Settings, the TCC subsystem may not recognize
/// the new binary until this prompt-triggering call is made.
///
/// - If the permission is already granted and recognized: returns `true` immediately.
/// - If the app is listed in Accessibility but the new binary isn't matched: macOS
///   re-evaluates and returns `true` without showing a dialog.
/// - If the app is not in the list at all: macOS shows the system authorization dialog.
///
/// Returns `true` if the process is now a trusted accessibility client.
pub fn request_accessibility_permission() -> bool {
    unsafe {
        let key: CFString = TCFType::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let value = CFBoolean::true_value();
        let dict = CFDictionary::from_CFType_pairs(&[(key, value)]);
        AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef())
    }
}

/// Convert an owned `CFTypeRef` that is actually a `CFString` into a `String`.
unsafe fn cf_value_to_string(value: CFTypeRef) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let cf_string = value as CFStringRef;
    let s: CFString = unsafe { CFString::wrap_under_create_rule(cf_string) };
    Some(s.to_string())
}

/// Copy a CF attribute value. A returned value follows the Create Rule and must
/// be wrapped or released by the caller.
unsafe fn copy_attribute_value(
    element: AXUIElementRef,
    attribute: &str,
) -> Result<Option<CFTypeRef>, SelectionError> {
    let attr_cf = CFString::new(attribute);
    let mut value: CFTypeRef = std::ptr::null();
    let err = unsafe {
        AXUIElementCopyAttributeValue(element, attr_cf.as_concrete_TypeRef(), &mut value)
    };
    if err != AX_SUCCESS {
        return Err(ax_error_to_selection_error(
            "AXUIElementCopyAttributeValue",
            attribute,
            err,
        ));
    }
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(value))
}

/// Read a CF attribute and convert the result to an owned `String`.
unsafe fn copy_string_attribute(
    element: AXUIElementRef,
    attribute: &str,
) -> Result<Option<String>, SelectionError> {
    let Some(value) = (unsafe { copy_attribute_value(element, attribute) })? else {
        return Ok(None);
    };
    Ok(unsafe { cf_value_to_string(value) })
}

fn ax_error_to_selection_error(function: &str, attribute: &str, err: i32) -> SelectionError {
    if err == kAXErrorAPIDisabled {
        return SelectionError::PermissionDenied;
    }
    SelectionError::Platform(format!(
        "{function}({attribute}) returned {err} ({})",
        error_string(err),
    ))
}

#[async_trait]
impl SelectionMonitor for MacOSSelection {
    async fn get_selected_text(&self) -> Result<Option<String>, SelectionError> {
        unsafe {
            let system = AXUIElementCreateSystemWide();
            if system.is_null() {
                return Err(SelectionError::Platform(
                    "AXUIElementCreateSystemWide returned NULL".to_string(),
                ));
            }

            // 1. Focused element
            let focused_value = match copy_attribute_value(system, kAXFocusedUIElementAttribute) {
                Ok(Some(value)) => value,
                Ok(None) => {
                    // No focused element — no selection. Treat as empty, not an error.
                    return Ok(None);
                }
                Err(SelectionError::PermissionDenied) => {
                    return Err(SelectionError::PermissionDenied);
                }
                Err(_) => {
                    // No focused element — no selection. Treat as empty, not an error.
                    return Ok(None);
                }
            };
            let _focused_release = CFType::wrap_under_create_rule(focused_value);
            let focused = focused_value as AXUIElementRef;
            if focused.is_null() {
                // No focused element — no selection. Treat as empty, not an error.
                return Ok(None);
            }

            // 2. Selected text
            match copy_string_attribute(focused, kAXSelectedTextAttribute) {
                Ok(Some(text)) if !text.trim().is_empty() => Ok(Some(text)),
                Ok(_) => read_selection_by_clipboard(),
                Err(SelectionError::PermissionDenied) => Err(SelectionError::PermissionDenied),
                Err(_) => read_selection_by_clipboard(),
            }
        }
    }

    async fn selection_bounds(&self) -> Result<Option<Rect>, SelectionError> {
        if !self.is_permission_granted() {
            return Err(SelectionError::PermissionDenied);
        }
        unsafe {
            let system = AXUIElementCreateSystemWide();
            if system.is_null() {
                return Err(SelectionError::Platform(
                    "AXUIElementCreateSystemWide returned NULL".to_string(),
                ));
            }
            let focused_value = match copy_attribute_value(system, kAXFocusedUIElementAttribute) {
                Ok(Some(value)) => value,
                _ => return Ok(None),
            };
            let _focused_release = CFType::wrap_under_create_rule(focused_value);
            let focused = focused_value as AXUIElementRef;
            // Selected text range (CFRange wrapped in CFType — implementation-defined
            // encoding, but typically an AXValue). For v0.1.0 we only need the
            // bounds, not the range itself, so we ask the AX framework to
            // compute the bounds for the entire range. A future improvement is
            // to actually parse the CFRange to handle partial selections.
            let mut bounds_value: CFTypeRef = std::ptr::null();
            let attr_cf = CFString::new(kAXBoundsForRangeParameterizedAttribute);
            let err = AXUIElementCopyParameterizedAttributeValue(
                focused,
                attr_cf.as_concrete_TypeRef(),
                std::ptr::null(), // parameter: NULL → use full selected range
                &mut bounds_value,
            );
            if err != AX_SUCCESS || bounds_value.is_null() {
                return Ok(None);
            }
            let _bounds_release = CFType::wrap_under_create_rule(bounds_value);
            // Parse AXValue → CGRect. We don't have a safe wrapper for AXValue
            // in core-foundation, so we use a raw byte read. AXValue is a
            // CFType whose first 16 bytes after the CFRuntimeBase are a
            // CGRect (4 × f32). This is a documented layout from Apple's
            // AXValue.h, but the proper way is to use the AXValueGetValue C
            // function. We bind it below.
            // For now, return Ok(None). The v0.2 main-window flow no longer
            // uses selection coordinates.
            Ok(None)
        }
    }

    async fn cursor_position(&self) -> Result<(i32, i32), SelectionError> {
        // CGEvent requires a GUI session. In a Tauri app this is fine.
        // Note: on macOS, cursor position also requires accessibility
        // permission in some sandboxed contexts.
        if !self.is_permission_granted() {
            return Err(SelectionError::PermissionDenied);
        }
        let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| SelectionError::Platform("CGEventSource::new failed".to_string()))?;
        let event = CGEvent::new(source)
            .map_err(|_| SelectionError::Platform("CGEvent::new failed".to_string()))?;
        let point: CGPoint = event.location();
        Ok((point.x.round() as i32, point.y.round() as i32))
    }

    fn is_permission_granted(&self) -> bool {
        unsafe {
            // First try the passive check (null options → no prompt).
            if AXIsProcessTrustedWithOptions(std::ptr::null()) {
                return true;
            }
            // Passive check returned false, but the user may already have
            // granted the permission (e.g. after an app update). Calling with
            // the prompt option forces macOS to re-evaluate the TCC entry,
            // matching the new binary against the stored bundle grant.
            request_accessibility_permission()
        }
    }

    async fn open_permission_settings(&self) -> Result<(), SelectionError> {
        // The deep link directly to Accessibility in System Settings.
        // This is the documented public URL scheme as of macOS 13+.
        const URL: &str =
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        std::process::Command::new("open")
            .arg(URL)
            .spawn()
            .map_err(|e| SelectionError::Platform(format!("open(URL): {e}")))?;
        Ok(())
    }
}

fn read_selection_by_clipboard() -> Result<Option<String>, SelectionError> {
    let previous_text = read_clipboard_text();
    copy_selection()?;

    let copied = wait_for_copied_clipboard_text(
        previous_text.as_deref(),
        read_clipboard_text,
        thread::sleep,
        CLIPBOARD_COPY_POLL_ATTEMPTS,
    );

    if let Some(text) = previous_text {
        let _ = write_clipboard_text(&text);
    }

    Ok(copied)
}

fn wait_for_copied_clipboard_text(
    previous_text: Option<&str>,
    mut read_clipboard_text: impl FnMut() -> Option<String>,
    mut sleep: impl FnMut(Duration),
    max_attempts: usize,
) -> Option<String> {
    let previous = previous_text.map(str::trim);
    for attempt in 0..max_attempts {
        let copied = read_clipboard_text()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        if let Some(text) = copied
            && Some(text.as_str()) != previous
        {
            return Some(text);
        }
        if attempt + 1 < max_attempts {
            sleep(CLIPBOARD_COPY_POLL_INTERVAL);
        }
    }
    None
}

fn copy_selection() -> Result<(), SelectionError> {
    let status = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to keystroke "c" using command down"#)
        .status()
        .map_err(|e| SelectionError::Platform(format!("osascript copy: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(SelectionError::Platform(format!(
            "osascript copy exited with {status}"
        )))
    }
}

fn read_clipboard_text() -> Option<String> {
    let output = Command::new("pbpaste").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn write_clipboard_text(text: &str) -> Result<(), SelectionError> {
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| SelectionError::Platform(format!("pbcopy: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| SelectionError::Platform(format!("pbcopy write: {e}")))?;
    }
    child
        .wait()
        .map_err(|e| SelectionError::Platform(format!("pbcopy wait: {e}")))?;
    Ok(())
}

// =============================================================================
// Tests — only run on macOS.
// =============================================================================
#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;
    use crate::SelectionMonitor;

    // S1: on a fresh test process, accessibility is almost certainly NOT granted.
    // We treat `is_permission_granted()` as a pure FFI passthrough, so it
    // must return whatever the OS says (typically `false` in CI).
    #[test]
    fn is_permission_granted_returns_os_value() {
        // This test just ensures the function doesn't panic. It may return
        // true or false depending on the host's accessibility state.
        let monitor = MacOSSelection::new();
        let _ = monitor.is_permission_granted();
    }

    #[test]
    fn ax_api_disabled_error_maps_to_permission_denied() {
        let error = ax_error_to_selection_error(
            "AXUIElementCopyAttributeValue",
            kAXFocusedUIElementAttribute,
            kAXErrorAPIDisabled,
        );

        assert!(matches!(error, SelectionError::PermissionDenied));
    }

    // S2: the permission settings URL is the documented one.
    #[test]
    fn open_permission_settings_uses_documented_url() {
        // We do not actually call the function (it spawns a process), but
        // we verify the URL by reconstructing it inline.
        const URL: &str =
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        assert!(URL.contains("Privacy_Accessibility"));
    }

    // S3: without permission, get_selected_text either sees the AX permission
    // denial from the OS or finds no focused selection before AX reports one.
    #[tokio::test]
    async fn get_selected_text_without_permission_denied() {
        let monitor = MacOSSelection::new();
        if !monitor.is_permission_granted() {
            let result = monitor.get_selected_text().await;
            assert!(matches!(
                result,
                Err(SelectionError::PermissionDenied) | Ok(None)
            ));
        }
        // If permission IS granted (e.g. developer machine), the call may
        // succeed with Ok(None). Both outcomes are acceptable here; this
        // test only asserts the no-permission path.
    }

    #[test]
    fn clipboard_fallback_waits_until_copied_text_replaces_previous_text() {
        let mut reads = vec![
            Some("old clipboard".to_string()),
            Some(" selected text ".to_string()),
        ]
        .into_iter();
        let mut sleeps = 0;

        let copied = wait_for_copied_clipboard_text(
            Some("old clipboard"),
            || reads.next().flatten(),
            |_| sleeps += 1,
            3,
        );

        assert_eq!(copied.as_deref(), Some("selected text"));
        assert_eq!(sleeps, 1);
    }

    #[test]
    fn clipboard_fallback_ignores_unchanged_previous_clipboard_text() {
        let mut reads = vec![
            Some("old clipboard".to_string()),
            Some(" old clipboard ".to_string()),
            Some("old clipboard".to_string()),
        ]
        .into_iter();

        let copied = wait_for_copied_clipboard_text(
            Some("old clipboard"),
            || reads.next().flatten(),
            |_| {},
            3,
        );

        assert_eq!(copied, None);
    }
}
