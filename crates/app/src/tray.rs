//! System tray (macOS menu bar / Windows system tray / Linux AppIndicator).
//!
//! Provides a menu with:
//! - Open Settings
//! - Quit
//!
//! Left-click on the tray icon also opens the main translation window.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Runtime,
};

use crate::commands;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");

/// Build the tray icon and menu.
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, "open_main", "Open Translator", true, None::<&str>)?;
    let open_settings =
        MenuItem::with_id(app, "open_settings", "Open Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_main, &open_settings, &quit])?;

    let icon = Image::from_bytes(TRAY_ICON_BYTES)
        .map_err(|e| tauri::Error::AssetNotFound(e.to_string()))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Translator")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_main" => {
                let _ = commands::show_main_window(app, Some("translator://open-main"));
            }
            "open_settings" => {
                let _ = commands::show_main_window(app, Some("translator://open-settings"));
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let _ = commands::show_main_window(app, Some("translator://open-main"));
            }
        })
        .build(app)?;

    Ok(())
}
