//! System tray (macOS menu bar / Windows system tray / Linux AppIndicator).
//!
//! Provides a menu with:
//! - Main
//! - Settings
//! - Check for Updates
//! - Restart
//! - Quit
//!
//! Left-click on the tray icon also opens the main translation window.

use fluent::{FluentBundle, FluentResource};
use tauri::{
    AppHandle, Manager, Runtime,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use unic_langid::LanguageIdentifier;

use crate::commands;
use translator_core::Config;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/icon.png");
const TRAY_ID: &str = "main";
#[cfg(target_os = "macos")]
const MACOS_TRAY_ICON_PADDING_PX: u32 = 24;
#[cfg(target_os = "macos")]
const MACOS_TEMPLATE_GLYPH_CHANNEL_FLOOR: u8 = 130;

/// Build the tray icon and menu.
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let labels = tray_menu_labels();
    let open_main = MenuItem::with_id(app, "open_main", &labels.open_main, true, None::<&str>)?;
    let open_settings = MenuItem::with_id(
        app,
        "open_settings",
        &labels.open_settings,
        true,
        None::<&str>,
    )?;
    let check_update = MenuItem::with_id(
        app,
        "check_update",
        &labels.check_update,
        true,
        None::<&str>,
    )?;
    let restart = MenuItem::with_id(app, "restart", &labels.restart, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &labels.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open_main, &open_settings, &check_update, &restart, &quit],
    )?;

    let icon = load_tray_icon().map_err(|e| tauri::Error::AssetNotFound(e.to_string()))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip(&labels.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_main" => {
                let _ = commands::show_main_window(app, Some("translator://open-main"));
            }
            "open_settings" => {
                let _ = commands::show_main_window(app, Some("translator://open-settings"));
            }
            "check_update" => {
                let _ = commands::show_main_window(app, Some("translator://check-update"));
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

pub fn sync_tray_visibility<R: Runtime>(
    app: &AppHandle<R>,
    show_menu_bar_icon: bool,
) -> tauri::Result<()> {
    if show_menu_bar_icon {
        build_tray(app)
    } else {
        let _ = app.remove_tray_by_id(TRAY_ID);
        Ok(())
    }
}

pub fn rebuild_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let _ = app.remove_tray_by_id(TRAY_ID);
    build_tray(app)
}

struct TrayMenuLabels {
    tooltip: String,
    open_main: String,
    open_settings: String,
    check_update: String,
    restart: String,
    quit: String,
}

fn tray_menu_labels() -> TrayMenuLabels {
    let cfg = Config::load().unwrap_or_default();
    let locale = resolve_menu_locale(&cfg.general.app_language);
    let bundle = menu_bundle(locale).or_else(|| menu_bundle("en"));

    TrayMenuLabels {
        tooltip: menu_text(bundle.as_ref(), "app-name", "Translator"),
        open_main: menu_text(bundle.as_ref(), "tray-open-main", "Main"),
        open_settings: menu_text(bundle.as_ref(), "tray-open-settings", "Settings"),
        check_update: menu_text(bundle.as_ref(), "tray-check-update", "Check for Updates"),
        restart: menu_text(bundle.as_ref(), "tray-restart", "Restart"),
        quit: menu_text(bundle.as_ref(), "tray-quit", "Quit"),
    }
}

fn menu_bundle(locale: &'static str) -> Option<FluentBundle<FluentResource>> {
    let langid: LanguageIdentifier = locale.parse().ok()?;
    let resource = match FluentResource::try_new(locale_source(locale).to_string()) {
        Ok(resource) => resource,
        Err((resource, errors)) => {
            tracing::warn!(locale, ?errors, "could not parse tray locale resource");
            resource
        }
    };
    let mut bundle = FluentBundle::new(vec![langid]);
    if let Err(errors) = bundle.add_resource(resource) {
        tracing::warn!(locale, ?errors, "could not add tray locale resource");
        return None;
    }
    Some(bundle)
}

fn menu_text(
    bundle: Option<&FluentBundle<FluentResource>>,
    key: &str,
    fallback: &'static str,
) -> String {
    let Some(bundle) = bundle else {
        return fallback.to_string();
    };
    let Some(message) = bundle.get_message(key) else {
        return fallback.to_string();
    };
    let Some(pattern) = message.value() else {
        return fallback.to_string();
    };
    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, None, &mut errors);
    if !errors.is_empty() {
        tracing::warn!(key, ?errors, "could not format tray locale message");
    }
    value.into_owned()
}

fn resolve_menu_locale(app_language: &str) -> &'static str {
    if app_language.trim().eq_ignore_ascii_case("system") {
        let system_locale = tauri_plugin_os::locale().unwrap_or_else(|| "en".to_string());
        normalize_menu_locale(&system_locale)
    } else {
        normalize_menu_locale(app_language)
    }
}

fn normalize_menu_locale(raw: &str) -> &'static str {
    let normalized = raw.trim().replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh") {
        if normalized.contains("hant")
            || normalized.contains("-tw")
            || normalized.contains("-hk")
            || normalized.contains("-mo")
        {
            return "zh-Hant";
        }
        return "zh-Hans";
    }

    match normalized.split('-').next().unwrap_or("en") {
        "ar" => "ar",
        "de" => "de",
        "en" => "en",
        "es" => "es",
        "fr" => "fr",
        "it" => "it",
        "ja" => "ja",
        "ko" => "ko",
        "pt" => "pt",
        "ru" => "ru",
        _ => "en",
    }
}

fn locale_source(locale: &str) -> &'static str {
    match locale {
        "ar" => include_str!("../../../ui/src/locales/ar.ftl"),
        "de" => include_str!("../../../ui/src/locales/de.ftl"),
        "es" => include_str!("../../../ui/src/locales/es.ftl"),
        "fr" => include_str!("../../../ui/src/locales/fr.ftl"),
        "it" => include_str!("../../../ui/src/locales/it.ftl"),
        "ja" => include_str!("../../../ui/src/locales/ja.ftl"),
        "ko" => include_str!("../../../ui/src/locales/ko.ftl"),
        "pt" => include_str!("../../../ui/src/locales/pt.ftl"),
        "ru" => include_str!("../../../ui/src/locales/ru.ftl"),
        "zh-Hans" => include_str!("../../../ui/src/locales/zh-Hans.ftl"),
        "zh-Hant" => include_str!("../../../ui/src/locales/zh-Hant.ftl"),
        _ => include_str!("../../../ui/src/locales/en.ftl"),
    }
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
