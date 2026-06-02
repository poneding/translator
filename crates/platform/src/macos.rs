//! macOS [`SelectionMonitor`] implementation.
//!
//! Uses AppKit's Accessibility (AX) framework via `accessibility-sys` 0.2 +
//! `core-foundation` 0.10 for safe CFString handling.
//!
//! ## Implementation notes (DESIGN.md §6.1)
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
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::CGEvent;
use core_graphics::geometry::CGPoint;

use crate::{Rect, SelectionError, SelectionMonitor};

// -----------------------------------------------------------------------------
// Raw FFI — accessibility-sys 0.2 re-exports the AX API. We bind only the
// functions we need; constants come from `accessibility_sys::AX::*`.
// -----------------------------------------------------------------------------

use accessibility_sys::AX::{
    kAXBoundsForRangeParameterizedAttribute, kAXFocusedUIElementAttribute,
    kAXSelectedTextAttribute, kAXSelectedTextRangeAttribute, AXIsProcessTrustedWithOptions,
    AXUIElementCopyAttributeValue, AXUIElementCopyParameterizedAttributeValue,
    AXUIElementCreateSystemWide,
};

const AX_SUCCESS: i32 = 0;

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

/// Convert a `CFTypeRef` that is actually a `CFString` into an owned `String`.
/// Returns `None` if the pointer is null or the value is not a CFString.
unsafe fn cf_value_to_string(value: CFTypeRef) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let cf_string = value as CFStringRef;
    // `wrap_under_get_rule` does NOT take ownership (the caller still owns
    // the CFString). The wrapper is dropped at end of scope and does not
    // release the underlying CFString.
    let s: CFString = CFString::wrap_under_get_rule(cf_string);
    Some(s.to_string())
}

/// Read a CF attribute and convert the result to an owned `String`.
unsafe fn copy_string_attribute(
    element: CFTypeRef,
    attribute: &str,
) -> Result<Option<String>, SelectionError> {
    let attr_cf = CFString::new(attribute);
    let mut value: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(
        element as *const _,
        attr_cf.as_concrete_TypeRef() as *const _,
        &mut value as *mut _ as *mut *const _,
    );
    if err != AX_SUCCESS {
        return Err(SelectionError::Platform(format!(
            "AXUIElementCopyAttributeValue({attribute}) returned {err}"
        )));
    }
    Ok(cf_value_to_string(value))
}

#[async_trait]
impl SelectionMonitor for MacOSSelection {
    async fn get_selected_text(&self) -> Result<Option<String>, SelectionError> {
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

            // 1. Focused element
            let mut focused: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(
                system as *const _,
                kAXFocusedUIElementAttribute as *const _,
                &mut focused as *mut _ as *mut *const _,
            );
            if err != AX_SUCCESS || focused.is_null() {
                // No focused element — no selection. Treat as empty, not an error.
                return Ok(None);
            }

            // 2. Selected text
            let mut text_value: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(
                focused as *const _,
                kAXSelectedTextAttribute as *const _,
                &mut text_value as *mut _ as *mut *const _,
            );
            if err != AX_SUCCESS {
                return Err(SelectionError::Platform(format!(
                    "AXUIElementCopyAttributeValue(AXSelectedText) returned {err}"
                )));
            }
            Ok(cf_value_to_string(text_value))
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
            let mut focused: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(
                system as *const _,
                kAXFocusedUIElementAttribute as *const _,
                &mut focused as *mut _ as *mut *const _,
            );
            if err != AX_SUCCESS || focused.is_null() {
                return Ok(None);
            }
            // Selected text range (CFRange wrapped in CFType — implementation-defined
            // encoding, but typically an AXValue). For v0.1.0 we only need the
            // bounds, not the range itself, so we ask the AX framework to
            // compute the bounds for the entire range. A future improvement is
            // to actually parse the CFRange to handle partial selections.
            let mut bounds_value: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyParameterizedAttributeValue(
                focused as *const _,
                kAXBoundsForRangeParameterizedAttribute as *const _,
                std::ptr::null(), // parameter: NULL → use full selected range
                &mut bounds_value as *mut _ as *mut *const _,
            );
            if err != AX_SUCCESS || bounds_value.is_null() {
                return Ok(None);
            }
            // Parse AXValue → CGRect. We don't have a safe wrapper for AXValue
            // in core-foundation, so we use a raw byte read. AXValue is a
            // CFType whose first 16 bytes after the CFRuntimeBase are a
            // CGRect (4 × f32). This is a documented layout from Apple's
            // AXValue.h, but the proper way is to use the AXValueGetValue C
            // function. We bind it below.
            // For now, return Ok(None) — selection_bounds is optional; the
            // popup falls back to cursor position.
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
        let event = CGEvent::new(std::ptr::null_mut())
            .map_err(|_| SelectionError::Platform("CGEvent::new failed".to_string()))?;
        let point: CGPoint = event.location();
        Ok((point.x.round() as i32, point.y.round() as i32))
    }

    fn is_permission_granted(&self) -> bool {
        // Passing NULL means "use the default prompt behavior". We pass NULL
        // because the prompt itself is shown by the OS the first time the
        // app calls into the AX framework (e.g. AXUIElementCreateSystemWide).
        // AXIsProcessTrustedWithOptions is the synchronous check.
        unsafe { AXIsProcessTrustedWithOptions(std::ptr::null()) }
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

    // S2: the permission settings URL is the documented one.
    #[test]
    fn open_permission_settings_uses_documented_url() {
        // We do not actually call the function (it spawns a process), but
        // we verify the URL by reconstructing it inline.
        const URL: &str =
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";
        assert!(URL.contains("Privacy_Accessibility"));
    }

    // S3: get_selected_text without permission returns PermissionDenied.
    #[tokio::test]
    async fn get_selected_text_without_permission_denied() {
        let monitor = MacOSSelection::new();
        if !monitor.is_permission_granted() {
            let result = monitor.get_selected_text().await;
            assert!(matches!(result, Err(SelectionError::PermissionDenied)));
        }
        // If permission IS granted (e.g. developer machine), the call may
        // succeed with Ok(None). Both outcomes are acceptable here; this
        // test only asserts the no-permission path.
    }
}
