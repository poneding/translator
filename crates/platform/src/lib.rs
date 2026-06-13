//! translator-platform: cross-platform OS integration.
//!
//! Currently provides [`SelectionMonitor`]: read the user's currently
//! selected text and handle platform permission helpers.
//!
//! The factory [`create`] returns a platform-appropriate implementation.

#![warn(missing_docs)]

use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

/// A rectangle in screen coordinates (top-left origin, y down).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Rect {
    /// Left edge, in screen pixels.
    pub x: i32,
    /// Top edge, in screen pixels.
    pub y: i32,
    /// Width, in screen pixels.
    pub width: i32,
    /// Height, in screen pixels.
    pub height: i32,
}

/// Errors returned by [`SelectionMonitor`].
#[derive(Debug, Error)]
pub enum SelectionError {
    /// The user has not granted the platform-required permission
    /// (e.g. macOS "Accessibility").
    #[error("permission denied; please grant required OS permission")]
    PermissionDenied,

    /// The platform-specific implementation raised an error.
    #[error("platform error: {0}")]
    Platform(String),

    /// The operation exceeded its timeout.
    #[error("selection monitor timed out")]
    Timeout,

    /// No element is focused, or the focused element has no selection.
    #[error("no text is currently selected")]
    Empty,
}

/// Read the user's currently selected text and related context.
#[async_trait]
pub trait SelectionMonitor: Send + Sync {
    /// Read the selected text of the currently focused element, if any.
    async fn get_selected_text(&self) -> Result<Option<String>, SelectionError>;

    /// Read the on-screen bounds of the current selection, if available.
    /// Implementations should return `Ok(None)` if the platform cannot
    /// provide this — callers will fall back to the cursor position.
    async fn selection_bounds(&self) -> Result<Option<Rect>, SelectionError>;

    /// Read the current mouse cursor position in screen coordinates.
    async fn cursor_position(&self) -> Result<(i32, i32), SelectionError>;

    /// Whether the platform-required permission is currently granted.
    /// This is a quick non-blocking check.
    fn is_permission_granted(&self) -> bool;

    /// Open the platform's permission settings UI for this app, if supported.
    /// On Linux this is a no-op (typically no special permission needed).
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    async fn open_permission_settings(&self) -> Result<(), SelectionError>;
}

/// Proactively request the platform selection permission, if needed.
///
/// On macOS this calls `AXIsProcessTrustedWithOptions` with the prompt
/// option, which forces the TCC subsystem to re-evaluate the Accessibility
/// permission. This is the standard workaround for a known macOS quirk:
/// after an app update the binary changes, and the TCC grant may go stale
/// even though the entry is still checked in System Settings.
///
/// On Windows and Linux this is a no-op (no special permission to request).
///
/// Returns `true` if the permission is now granted.
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::request_accessibility_permission()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Construct a platform-appropriate [`SelectionMonitor`] implementation.
pub fn create() -> Box<dyn SelectionMonitor> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOSSelection::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsSelection::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxSelection::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        compile_error!("translator-platform requires macOS, Windows, or Linux");
    }
}
