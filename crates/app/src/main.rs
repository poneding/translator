//! translator-app: Tauri shell.
//!
//! Owns the global hotkey, system tray, popup window, and IPC bridge to the
//! `core` (translation) and `platform` (selection monitor) crates.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod permissions;
mod popup_position;
mod state;
mod tray;

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::state::AppState;

fn main() {
    init_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        tracing::info!("hotkey pressed, triggering translation");
                        // on_hotkey is async; spawn it so the handler returns immediately.
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            commands::on_hotkey(&app_handle).await;
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Build app state.
            let state = AppState::new();
            app.manage(Arc::new(state));

            // Build tray.
            tray::build_tray(app.handle())?;

            // BH-1.5: if the previous run failed to register its hotkey, fall
            // back to the default shortcut and clear the banner flag.
            let mut cfg = translator_core::config::Config::load()
                .unwrap_or_else(|_| translator_core::config::Config::default());
            if cfg.hotkey_registration_failed {
                tracing::warn!(
                    previous = %cfg.shortcut,
                    "previous hotkey registration failed; resetting to default"
                );
                cfg.shortcut = "CmdOrCtrl+Shift+D".to_string();
                cfg.hotkey_registration_failed = false;
                if let Err(e) = cfg.save() {
                    tracing::warn!(error = %e, "could not persist hotkey reset");
                }
            }

            // Register the configured hotkey. The previous default shortcut
            // (CmdOrCtrl+Shift+D) is now derived from the config above.
            let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyD);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                tracing::warn!(error = %e, "failed to register default global shortcut");
            }

            // Hide the settings window on launch; it opens via the tray.
            if let Some(win) = app.get_webview_window("settings") {
                let _ = win.hide();
            }
            if let Some(win) = app.get_webview_window("popup") {
                let _ = win.hide();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_selected_text,
            commands::translate_text,
            commands::show_popup,
            commands::hide_popup,
            commands::open_settings,
            commands::get_config,
            commands::save_config,
            commands::get_app_info,
            commands::set_api_key,
            commands::delete_api_key,
            commands::has_api_key,
            commands::update_hotkey,
            commands::copy_to_clipboard,
            commands::check_permission,
            commands::open_permission_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,translator_app=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
