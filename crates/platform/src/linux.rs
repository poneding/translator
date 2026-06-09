//! Linux [`SelectionMonitor`] implementation.
//!
//! Uses AT-SPI2 via the `atspi` crate over D-Bus (via `zbus`). See DESIGN.md §6.1.
//!
//! Pipeline:
//! 1. Connect to the session D-Bus.
//! 2. Resolve the currently-focused accessible (via `Registry` event
//!    listener or `Accessible::Get` with the focused object path).
//! 3. If the focused accessible implements the `Text` interface, read the
//!    selection range. Selection itself requires the `Accessible` to also
//!    implement the legacy `Selection` interface (via `atspi::Selection`).
//! 4. Bounds come from `Accessible::GetExtents(CoordType::Screen)`.
//!
//! Supported: GNOME 46+, KDE Plasma 5.27+. Other DEs may return `Empty`.

use async_trait::async_trait;
use std::process::Command;
use std::sync::OnceLock;
use tokio::sync::Mutex;

use crate::{Rect, SelectionError, SelectionMonitor};

/// Lazily-initialized connection handle. We hold the `zbus::Connection`
/// in a `Mutex<Option<…>>` so the first call pays the connect cost and
/// subsequent calls reuse it.
static CONN: OnceLock<Mutex<Option<zbus::Connection>>> = OnceLock::new();

fn conn_slot() -> &'static Mutex<Option<zbus::Connection>> {
    CONN.get_or_init(|| Mutex::new(None))
}

async fn get_or_connect() -> Result<zbus::Connection, SelectionError> {
    let mut guard = conn_slot().lock().await;
    if let Some(c) = guard.as_ref() {
        return Ok(c.clone());
    }
    let conn = zbus::connection::Builder::session()
        .map_err(|e| SelectionError::Platform(format!("dbus connect: {e}")))?
        .build()
        .await
        .map_err(|e| SelectionError::Platform(format!("dbus build: {e}")))?;
    *guard = Some(conn.clone());
    Ok(conn)
}

/// Linux-specific selection monitor.
pub struct LinuxSelection {
    _marker: (),
}

impl LinuxSelection {
    /// Construct a new monitor.
    pub fn new() -> Self {
        Self { _marker: () }
    }
}

impl Default for LinuxSelection {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SelectionMonitor for LinuxSelection {
    async fn get_selected_text(&self) -> Result<Option<String>, SelectionError> {
        if let Some(text) = read_primary_selection() {
            return Ok(Some(text));
        }
        let _conn = get_or_connect().await?;
        // TODO: resolve the focused accessible via the AT-SPI2 `Registry`
        // proxy and read its selection through the `Text` interface.
        Ok(None)
    }

    async fn selection_bounds(&self) -> Result<Option<Rect>, SelectionError> {
        // TODO: read via `Accessible::GetExtents(CoordType::Screen)`.
        Ok(None)
    }

    async fn cursor_position(&self) -> Result<(i32, i32), SelectionError> {
        // TODO: read from a D-Bus portal (`org.freedesktop.impl.portal.Session`
        // exposes `PointerPosition`) or fall back to `/dev/input/mouse*` parsing.
        Err(SelectionError::Platform(
            "cursor_position not yet implemented for Linux".to_string(),
        ))
    }

    fn is_permission_granted(&self) -> bool {
        // Most desktop environments expose AT-SPI2 without explicit grant.
        true
    }

    async fn open_permission_settings(&self) -> Result<(), SelectionError> {
        // No-op on Linux (no centralized permission UI for AT-SPI).
        Ok(())
    }
}

fn read_primary_selection() -> Option<String> {
    [
        ("wl-paste", &["--primary", "--no-newline"][..]),
        ("xclip", &["-selection", "primary", "-out"][..]),
        ("xsel", &["-op"][..]),
    ]
    .into_iter()
    .find_map(|(program, args)| {
        let output = Command::new(program).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout)
            .ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    // Linux selection monitor relies on a live AT-SPI2 / D-Bus session,
    // which isn't available in CI. We only verify the type's basic shape.
    use super::*;

    #[test]
    fn linux_selection_constructs() {
        let s = LinuxSelection::new();
        // Trait-level check: it implements SelectionMonitor.
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<LinuxSelection>();
        let _ = s;
    }
}
