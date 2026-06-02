//! First-run / permission UX.
//!
//! On macOS the user must grant the "Accessibility" permission in
//! `System Settings → Privacy & Security → Accessibility`. This module
//! holds the messaging the frontend shows when the permission is missing.

#[allow(dead_code)]
pub const MACOS_PERMISSION_GUIDE: &str =
    "translator needs the Accessibility permission to read your selected text. \
Open System Settings → Privacy & Security → Accessibility, and enable translator.";

#[allow(dead_code)]
pub const WINDOWS_PERMISSION_GUIDE: &str =
    "translator uses Windows UI Automation. No special permission is required.";

#[allow(dead_code)]
pub const LINUX_PERMISSION_GUIDE: &str = "translator uses AT-SPI2 over D-Bus. \
On GNOME 46+ and KDE Plasma 5.27+ this works out of the box. \
If selection is empty, check that your desktop environment exposes AT-SPI.";
