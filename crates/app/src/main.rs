//! translator-app: Tauri shell.
//!
//! Owns the global hotkey, system tray, main window, and IPC bridge to the
//! `core` (translation) and `platform` (selection monitor) crates.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod permissions;
mod state;
mod tray;

use std::sync::Arc;

use tauri::{ActivationPolicy, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::AppState;

fn main() {
    init_tracing();

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if window.label() == "main" {
                match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let _ = commands::remember_main_window_position(window);
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    tauri::WindowEvent::Focused(false) => {
                        let _ = commands::remember_main_window_position(window);
                    }
                    _ => {}
                }
            }
        })
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Released {
                        tracing::info!("hotkey released, triggering translation");
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
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(ActivationPolicy::Accessory);
                app.set_dock_visibility(false);
            }

            // macOS: proactively request accessibility permission so TCC
            // re-evaluates the grant (fixes stale permission after app update).
            #[cfg(target_os = "macos")]
            {
                let trusted = translator_platform::request_accessibility_permission();
                tracing::debug!(trusted, "macOS accessibility permission state");
            }

            // Build app state.
            let state = AppState::new();
            app.manage(Arc::new(state));

            // Build tray.
            tray::build_tray(app.handle())?;

            // BH-1.5: if the previous run failed to register its hotkey, fall
            // back to the default shortcut and clear the banner flag.
            let mut cfg = translator_core::config::Config::load()
                .unwrap_or_else(|_| translator_core::config::Config::default());
            let default_shortcut = translator_core::config::Config::default().shortcut;
            if cfg.hotkey_registration_failed {
                tracing::warn!(
                    previous = %cfg.shortcut,
                    "previous hotkey registration failed; resetting to default"
                );
                cfg.shortcut = default_shortcut.clone();
                cfg.hotkey_registration_failed = false;
                if let Err(e) = cfg.save() {
                    tracing::warn!(error = %e, "could not persist hotkey reset");
                }
            }

            // Register the configured hotkey. If parsing or registration fails,
            // reset to the default and expose the banner flag in settings.
            match commands::parse_shortcut(&cfg.shortcut) {
                Ok(shortcut) => {
                    if let Err(e) = app.global_shortcut().register(shortcut) {
                        tracing::warn!(error = %e, shortcut = %cfg.shortcut, "failed to register global shortcut");
                        cfg.shortcut = default_shortcut.clone();
                        cfg.hotkey_registration_failed = true;
                        let _ = cfg.save();
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, shortcut = %cfg.shortcut, "invalid configured shortcut");
                    cfg.shortcut = default_shortcut.clone();
                    cfg.hotkey_registration_failed = true;
                    let _ = cfg.save();
                    if let Ok(shortcut) = commands::parse_shortcut(&cfg.shortcut) {
                        let _ = app.global_shortcut().register(shortcut);
                    }
                }
            }

            if let Some(win) = app.get_webview_window("main")
                && let Err(e) = commands::prepare_main_window(app.handle(), &win, &cfg)
            {
                tracing::warn!(error = %e, "could not prepare main window");
            }

            if cfg.updates.check_on_startup {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    commands::run_startup_update_check(app_handle).await;
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_selected_text,
            commands::translate_text,
            commands::translate_service,
            commands::get_text_audio_url,
            commands::open_main_window,
            commands::open_settings,
            commands::restart_app,
            commands::set_main_window_always_on_top,
            commands::open_external_url,
            commands::get_config,
            commands::save_config,
            commands::clear_history,
            commands::get_app_info,
            commands::set_api_key,
            commands::delete_api_key,
            commands::has_api_key,
            commands::update_hotkey,
            commands::copy_to_clipboard,
            commands::read_clipboard,
            commands::check_permission,
            commands::open_permission_settings,
            commands::check_update,
            commands::download_and_install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            handle_run_event(app, event);
        });
}

fn handle_run_event<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: tauri::RunEvent) {
    #[cfg(target_os = "macos")]
    {
        if let tauri::RunEvent::Reopen { .. } = event {
            let _ = commands::show_main_window(app, Some("translator://open-main"));
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app, event);
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,translator_app=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
