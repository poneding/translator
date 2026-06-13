//! System tray (macOS menu bar / Windows system tray / Linux AppIndicator).
//!
//! Provides a menu with:
//! - Open Translator
//! - Open Settings
//! - Restart Translator
//! - Quit
//!
//! Left-click on the tray icon also opens the main translation window.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

use crate::commands;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");
#[cfg(target_os = "macos")]
const MACOS_TRAY_ICON_PADDING_PX: u32 = 24;
#[cfg(target_os = "macos")]
const MACOS_TEMPLATE_GLYPH_CHANNEL_FLOOR: u8 = 130;

/// Build the tray icon and menu.
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, "open_main", "Open Translator", true, None::<&str>)?;
    let open_settings =
        MenuItem::with_id(app, "open_settings", "Open Settings", true, None::<&str>)?;
    let restart = MenuItem::with_id(app, "restart", "Restart Translator", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_main, &open_settings, &restart, &quit])?;

    let icon = load_tray_icon().map_err(|e| tauri::Error::AssetNotFound(e.to_string()))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
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
            "restart" => {
                let _ = commands::restart_app(app.clone());
            }
            "quit" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = commands::remember_main_webview_window_position(&window);
                }
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

fn load_tray_icon() -> tauri::Result<Image<'static>> {
    let icon = Image::from_bytes(TRAY_ICON_BYTES)?;
    Ok(adjust_tray_icon_for_platform(icon))
}

#[cfg(target_os = "macos")]
fn adjust_tray_icon_for_platform(icon: Image<'_>) -> Image<'static> {
    let template_icon = extract_template_glyph_mask(&icon).unwrap_or_else(|| icon.to_owned());
    crop_transparent_padding(&template_icon, MACOS_TRAY_ICON_PADDING_PX)
}

#[cfg(not(target_os = "macos"))]
fn adjust_tray_icon_for_platform(icon: Image<'_>) -> Image<'static> {
    icon.to_owned()
}

#[cfg(target_os = "macos")]
fn extract_template_glyph_mask(icon: &Image<'_>) -> Option<Image<'static>> {
    let width = icon.width();
    let height = icon.height();
    let rgba = icon.rgba();
    let mut template = vec![0; rgba.len()];
    let mut found = false;

    for (source, target) in rgba.chunks_exact(4).zip(template.chunks_exact_mut(4)) {
        let alpha = source[3];
        if alpha == 0 {
            continue;
        }

        let lightest_common_channel = source[0].min(source[1]).min(source[2]);
        let glyph_weight =
            lightest_common_channel.saturating_sub(MACOS_TEMPLATE_GLYPH_CHANNEL_FLOOR);
        if glyph_weight == 0 {
            continue;
        }

        let glyph_alpha = ((alpha as u16 * glyph_weight as u16)
            / (u8::MAX - MACOS_TEMPLATE_GLYPH_CHANNEL_FLOOR) as u16)
            .min(u8::MAX as u16) as u8;
        if glyph_alpha == 0 {
            continue;
        }

        target[3] = glyph_alpha;
        found = true;
    }

    found.then(|| Image::new_owned(template, width, height))
}

#[cfg(target_os = "macos")]
fn crop_transparent_padding(icon: &Image<'_>, padding: u32) -> Image<'static> {
    let width = icon.width();
    let height = icon.height();
    let rgba = icon.rgba();

    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            let alpha = rgba[((y * width + x) * 4 + 3) as usize];
            if alpha == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if !found {
        return icon.clone().to_owned();
    }

    let content_width = max_x - min_x + 1;
    let content_height = max_y - min_y + 1;
    let side = content_width.max(content_height) + padding * 2;
    let offset_x = (side - content_width) / 2;
    let offset_y = (side - content_height) / 2;
    let mut cropped = vec![0; (side * side * 4) as usize];

    for y in 0..content_height {
        for x in 0..content_width {
            let source = (((min_y + y) * width + (min_x + x)) * 4) as usize;
            let target = (((offset_y + y) * side + (offset_x + x)) * 4) as usize;
            cropped[target..target + 4].copy_from_slice(&rgba[source..source + 4]);
        }
    }

    Image::new_owned(cropped, side, side)
}
