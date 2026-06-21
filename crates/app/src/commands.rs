//! IPC commands exposed to the React frontend via `invoke()`.

use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, Position,
    Runtime, Size, State, WebviewWindow, Window,
};
use tauri_plugin_updater::UpdaterExt;

use translator_core::config::{WindowConfig, WindowPosition};
use translator_core::{Config, ServiceError, ServiceId, TranslateResult};
use translator_platform::SelectionError;

use crate::{
    state::AppState,
    tray::{rebuild_tray, sync_tray_visibility},
};

const STABLE_UPDATE_ENDPOINT: &str =
    "https://github.com/poneding/translator/releases/latest/download/latest.json";
const BETA_UPDATE_ENDPOINT: &str =
    "https://github.com/poneding/translator/releases/download/beta/latest.json";
const MAIN_WINDOW_DEFAULT_WIDTH: f64 = 680.0;
const MAIN_WINDOW_DEFAULT_HEIGHT: f64 = 560.0;
const MAIN_WINDOW_MAX_WIDTH: f64 = 920.0;
const MAIN_WINDOW_MAX_HEIGHT: f64 = 4096.0;
const WINDOW_EDGE_MARGIN: i32 = 24;

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

/// Return the user's currently selected text, or `null` if none.
#[tauri::command]
pub async fn get_selected_text(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    let monitor = state.selection_monitor().await;
    monitor
        .get_selected_text()
        .await
        .map_err(|e| format!("{}:{}", e.code(), e))
}

/// Return whether the OS-level selection permission is granted.
#[tauri::command]
pub fn check_permission(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    // The permission check is sync on the trait, but we go through the lazy monitor.
    let monitor = futures::executor::block_on(state.selection_monitor());
    Ok(monitor.is_permission_granted())
}

/// Open the platform's permission settings UI.
#[tauri::command]
pub async fn open_permission_settings(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let monitor = state.selection_monitor().await;
    monitor
        .open_permission_settings()
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Translation
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TranslateArgs {
    pub text: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub request_id: Option<String>,
}

/// Translate the text via all enabled services, returning one result per service.
#[tauri::command]
pub async fn translate_text(
    args: TranslateArgs,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ServiceOutcomeDto>, String> {
    let mut cfg = Config::load().map_err(|e| e.to_string())?;
    let request_id = args.request_id.unwrap_or_else(new_request_id);
    let req = translator_core::translate_request(
        args.text.trim().to_string(),
        &cfg.general,
        args.from,
        args.to,
    );
    if req.text.is_empty() {
        return Err("text is empty".to_string());
    }

    let pending = pending_outcomes(&cfg);
    let _ = app.emit(
        "translator://translation-started",
        TranslationStartedDto {
            request_id: request_id.clone(),
            outcomes: pending,
        },
    );

    let event_app = app.clone();
    let event_request_id = request_id.clone();
    let outcomes = state
        .translator
        .translate_each(&req, &cfg, |outcome| {
            let dto = ServiceOutcomeDto::from(outcome);
            let _ = event_app.emit(
                "translator://translation-outcome",
                TranslationOutcomeDto {
                    request_id: event_request_id.clone(),
                    outcome: dto,
                },
            );
        })
        .await;
    let dtos: Vec<ServiceOutcomeDto> = outcomes.into_iter().map(ServiceOutcomeDto::from).collect();

    if let Some(first) = dtos.iter().find_map(|outcome| outcome.result.as_ref()) {
        cfg.record_history(
            req.text.clone(),
            first.text.clone(),
            first.service_id.as_str().to_string(),
            first.service_name.clone(),
            req.from.clone().unwrap_or_else(|| "auto".to_string()),
            req.to.clone(),
        );
        cfg.save().map_err(|e| e.to_string())?;

        if cfg.general.auto_copy {
            use tauri_plugin_clipboard_manager::ClipboardExt;
            app.clipboard()
                .write_text(first.text.clone())
                .map_err(|e| e.to_string())?;
        }
    }

    let _ = app.emit(
        "translator://translation-finished",
        TranslationFinishedDto { request_id },
    );

    Ok(dtos)
}

#[derive(Debug, Deserialize)]
pub struct TranslateServiceArgs {
    pub service_id: String,
    pub text: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub request_id: Option<String>,
}

/// Translate text via one service. Used by per-card refresh in the main UI.
#[tauri::command]
pub async fn translate_service(
    args: TranslateServiceArgs,
    state: State<'_, Arc<AppState>>,
) -> Result<ServiceOutcomeDto, String> {
    let cfg = Config::load().map_err(|e| e.to_string())?;
    let service_id = parse_service_id(&args.service_id)?;
    let req = translator_core::translate_request(
        args.text.trim().to_string(),
        &cfg.general,
        args.from,
        args.to,
    );
    if req.text.is_empty() {
        return Err("text is empty".to_string());
    }

    let _request_id = args.request_id.unwrap_or_else(new_request_id);
    let outcome = state
        .translator
        .translate_service(service_id, &req, &cfg)
        .await;
    Ok(ServiceOutcomeDto::from(outcome))
}

#[derive(Debug, Deserialize)]
pub struct TextAudioArgs {
    pub text: String,
    pub language: Option<String>,
}

/// Return a text-to-speech URL for source-editor playback.
#[tauri::command]
pub fn get_text_audio_url(args: TextAudioArgs) -> Result<Option<String>, String> {
    Ok(translator_core::audio::default_text_audio_url(
        &args.text,
        args.language.as_deref(),
    ))
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceOutcomeDto {
    pub service_id: String,
    pub service_name: String,
    pub result: Option<TranslateResult>,
    pub error: Option<ServiceErrorDto>,
}

impl From<&translator_core::translator::TranslateOutcome> for ServiceOutcomeDto {
    fn from(o: &translator_core::translator::TranslateOutcome) -> Self {
        match &o.result {
            Ok(r) => Self {
                service_id: o.service_id.as_str().to_string(),
                service_name: o.service_name.clone(),
                result: Some(r.clone()),
                error: None,
            },
            Err(e) => Self {
                service_id: o.service_id.as_str().to_string(),
                service_name: o.service_name.clone(),
                result: None,
                error: Some(ServiceErrorDto::from(e)),
            },
        }
    }
}

impl From<translator_core::translator::TranslateOutcome> for ServiceOutcomeDto {
    fn from(o: translator_core::translator::TranslateOutcome) -> Self {
        Self::from(&o)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceErrorDto {
    pub code: String,
    pub message: String,
}

impl From<&ServiceError> for ServiceErrorDto {
    fn from(e: &ServiceError) -> Self {
        Self {
            code: e.code().to_string(),
            message: e.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TranslationStartedDto {
    pub request_id: String,
    pub outcomes: Vec<ServiceOutcomeDto>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TranslationOutcomeDto {
    pub request_id: String,
    pub outcome: ServiceOutcomeDto,
}

#[derive(Clone, Debug, Serialize)]
pub struct TranslationFinishedDto {
    pub request_id: String,
}

fn pending_outcomes(cfg: &Config) -> Vec<ServiceOutcomeDto> {
    let mut services: Vec<_> = cfg
        .services
        .values()
        .filter(|service| service.enabled)
        .collect();
    services.sort_by_key(|service| service.priority);
    services
        .into_iter()
        .map(|service| ServiceOutcomeDto {
            service_id: service.id.as_str().to_string(),
            service_name: service.id.display_name().to_string(),
            result: None,
            error: None,
        })
        .collect()
}

fn new_request_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("translation-{millis}")
}

fn parse_service_id(value: &str) -> Result<ServiceId, String> {
    match value {
        "youdao" => Ok(ServiceId::Youdao),
        "deepl" => Ok(ServiceId::DeepL),
        "google" => Ok(ServiceId::Google),
        "bing" => Ok(ServiceId::Bing),
        "openai" => Ok(ServiceId::OpenAI),
        other => Err(format!("unknown service: {other}")),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HotkeyPayload {
    pub text: Option<String>,
    pub error: Option<String>,
    pub source: String,
}

// ---------------------------------------------------------------------------
// Settings view + config
// ---------------------------------------------------------------------------

/// Open the main window and switch it to the settings view.
#[tauri::command]
pub fn open_settings<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    show_main_window(&app, Some("translator://open-settings"))
}

/// Open the main translation window.
#[tauri::command]
pub fn open_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    show_main_window(&app, Some("translator://open-main"))
}

/// Restart the application, preserving the last known main-window position first.
#[tauri::command]
pub fn restart_app<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = remember_main_webview_window_position(&window);
    }
    app.restart();
}

pub(crate) fn show_main_window<R: Runtime>(
    app: &AppHandle<R>,
    event: Option<&str>,
) -> Result<(), String> {
    let cfg = Config::load().unwrap_or_default();
    let win: WebviewWindow<R> = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    prepare_main_window_for_show(app, &win, &cfg)?;
    #[cfg(target_os = "macos")]
    let _ = app.show();
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    if let Some(event) = event {
        let _ = app.emit(event, ());
    }
    Ok(())
}

pub(crate) fn prepare_main_window<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    cfg: &Config,
) -> Result<(), String> {
    prepare_main_window_inner(app, win, cfg, true)
}

fn prepare_main_window_for_show<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    cfg: &Config,
) -> Result<(), String> {
    prepare_main_window_inner(
        app,
        win,
        cfg,
        should_position_main_window_on_show(&cfg.window.display_position),
    )
}

fn prepare_main_window_inner<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    cfg: &Config,
    should_position: bool,
) -> Result<(), String> {
    win.set_always_on_top(cfg.window.always_on_top)
        .map_err(|e| e.to_string())?;
    let _ = win.set_maximizable(false);
    let _ = win.set_max_size(Some(Size::Logical(LogicalSize::new(
        MAIN_WINDOW_MAX_WIDTH,
        MAIN_WINDOW_MAX_HEIGHT,
    ))));
    if should_position {
        position_main_window(app, win, &cfg.window)?;
    }
    Ok(())
}

fn should_position_main_window_on_show(display_position: &str) -> bool {
    display_position != "remember"
}

fn position_main_window<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    window_config: &WindowConfig,
) -> Result<(), String> {
    let cursor = app.cursor_position().ok();
    let remembered_target =
        remembered_position_target(app, &window_config.display_position, window_config)?;
    let (monitor, remembered_position) = if let Some((monitor, position)) = remembered_target {
        (Some(monitor), Some(position))
    } else {
        (target_monitor(app, win, cursor)?, None)
    };
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let size = win.outer_size().unwrap_or_else(|_| {
        PhysicalSize::new(
            MAIN_WINDOW_DEFAULT_WIDTH as u32,
            MAIN_WINDOW_DEFAULT_HEIGHT as u32,
        )
    });
    let position = resolve_main_window_position(
        &window_config.display_position,
        remembered_position,
        cursor,
        MainWindowWorkArea::from_monitor(&monitor),
        size,
    );
    win.set_position(Position::Physical(position))
        .map_err(|e| e.to_string())
}

fn remembered_position_target<R: Runtime>(
    app: &AppHandle<R>,
    display_position: &str,
    window_config: &WindowConfig,
) -> Result<Option<(Monitor, WindowPosition)>, String> {
    if display_position != "remember" {
        return Ok(None);
    }
    let Some(position) = window_config.last_position else {
        return Ok(None);
    };
    let monitor = app
        .monitor_from_point(position.x as f64, position.y as f64)
        .map_err(|e| e.to_string())?;
    Ok(monitor.map(|monitor| (monitor, position)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MainWindowWorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl MainWindowWorkArea {
    fn from_monitor(monitor: &Monitor) -> Self {
        let work = monitor.work_area();
        let left = work.position.x;
        let top = work.position.y;
        Self {
            left,
            top,
            right: left + work.size.width as i32,
            bottom: top + work.size.height as i32,
        }
    }
}

fn resolve_main_window_position(
    display_position: &str,
    remembered_position: Option<WindowPosition>,
    cursor: Option<PhysicalPosition<f64>>,
    work_area: MainWindowWorkArea,
    size: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let width = size.width as i32;
    let height = size.height as i32;
    let work_width = work_area.right - work_area.left;
    let work_height = work_area.bottom - work_area.top;
    let top_right = || {
        (
            work_area.right - width - WINDOW_EDGE_MARGIN,
            work_area.top + WINDOW_EDGE_MARGIN,
        )
    };

    let (raw_x, raw_y) = match display_position {
        "remember" => remembered_position
            .map(|position| (position.x, position.y))
            .unwrap_or_else(top_right),
        "center" => (
            work_area.left + (work_width - width) / 2,
            work_area.top + (work_height - height) / 2,
        ),
        "mouse" => {
            if let Some(cursor) = cursor {
                (cursor.x.round() as i32 + 16, cursor.y.round() as i32 + 16)
            } else {
                top_right()
            }
        }
        _ => top_right(),
    };

    PhysicalPosition::new(
        clamp_window_axis(raw_x, work_area.left, work_area.right, width),
        clamp_window_axis(raw_y, work_area.top, work_area.bottom, height),
    )
}

pub(crate) fn remember_main_window_position<R: Runtime>(window: &Window<R>) -> Result<(), String> {
    let position = window.outer_position().map_err(|e| e.to_string())?;
    remember_main_window_position_inner(window.label(), position)
}

pub(crate) fn remember_main_webview_window_position<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<(), String> {
    let position = window.outer_position().map_err(|e| e.to_string())?;
    remember_main_window_position_inner(window.label(), position)
}

fn remember_main_window_position_inner(
    label: &str,
    position: PhysicalPosition<i32>,
) -> Result<(), String> {
    if label != "main" {
        return Ok(());
    }
    let next = WindowPosition {
        x: position.x,
        y: position.y,
    };
    let mut cfg = Config::load().map_err(|e| e.to_string())?;
    if cfg.window.last_position == Some(next) {
        return Ok(());
    }
    cfg.window.last_position = Some(next);
    cfg.save().map_err(|e| e.to_string())
}

fn target_monitor<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    cursor: Option<PhysicalPosition<f64>>,
) -> Result<Option<Monitor>, String> {
    if let Some(cursor) = cursor
        && let Some(monitor) = app
            .monitor_from_point(cursor.x, cursor.y)
            .map_err(|e| e.to_string())?
    {
        return Ok(Some(monitor));
    }
    if let Some(monitor) = win.current_monitor().map_err(|e| e.to_string())? {
        return Ok(Some(monitor));
    }
    win.primary_monitor().map_err(|e| e.to_string())
}

fn clamp_window_axis(value: i32, min: i32, max: i32, window_size: i32) -> i32 {
    let upper = max - window_size;
    if upper <= min {
        min
    } else {
        value.clamp(min, upper)
    }
}

#[derive(Debug, Deserialize)]
pub struct AlwaysOnTopArgs {
    pub always_on_top: bool,
}

/// Apply the main window always-on-top state immediately.
#[tauri::command]
pub fn set_main_window_always_on_top<R: Runtime>(
    app: AppHandle<R>,
    args: AlwaysOnTopArgs,
) -> Result<(), String> {
    apply_main_window_always_on_top(&app, args.always_on_top)
}

pub(crate) fn apply_main_window_always_on_top<R: Runtime>(
    app: &AppHandle<R>,
    always_on_top: bool,
) -> Result<(), String> {
    let win: WebviewWindow<R> = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.set_always_on_top(always_on_top)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct OpenExternalUrlArgs {
    pub url: String,
}

/// Open a trusted external URL in the user's default browser.
#[tauri::command]
pub fn open_external_url(args: OpenExternalUrlArgs) -> Result<(), String> {
    let url = args.url.trim();
    if !is_allowed_external_url(url) {
        return Err("only http(s) URLs can be opened externally".to_string());
    }
    open_url_with_system_browser(url)
}

fn is_allowed_external_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(char::is_control)
        && !url.chars().any(char::is_whitespace)
}

fn open_url_with_system_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("open external URL: {e}"))
}

/// Return the current config (sanitized: never includes API keys).
#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    Config::load().map_err(|e| e.to_string())
}

/// Save the provided config to disk.
#[tauri::command]
pub fn save_config<R: Runtime>(app: AppHandle<R>, config: Config) -> Result<(), String> {
    let old = Config::load().unwrap_or_default();
    let config = config.normalized();
    if old.app.launch_at_startup != config.app.launch_at_startup {
        sync_autostart(&app, config.app.launch_at_startup)?;
    }
    config.save().map_err(|e| e.to_string())?;
    apply_main_window_always_on_top(&app, config.window.always_on_top)?;
    if config.app.show_menu_bar_icon
        && old.app.show_menu_bar_icon
        && old.general.app_language != config.general.app_language
    {
        rebuild_tray(&app).map_err(|e| e.to_string())?;
    } else {
        sync_tray_visibility(&app, config.app.show_menu_bar_icon).map_err(|e| e.to_string())?;
    }
    let _ = app.emit("translator://config-updated", &config);
    Ok(())
}

/// Clear all persisted translation history.
#[tauri::command]
pub fn clear_history<R: Runtime>(app: AppHandle<R>) -> Result<Config, String> {
    let mut config = Config::load().map_err(|e| e.to_string())?;
    config.history.clear();
    config.save().map_err(|e| e.to_string())?;
    let _ = app.emit("translator://config-updated", &config);
    Ok(config)
}

#[derive(Debug, Serialize)]
pub struct AppInfoDto {
    /// App version from `Cargo.toml`.
    pub version: String,
    /// Git commit the binary was built from; "dev" if not set at build time.
    pub commit: String,
    /// ISO-8601 build date; "dev" if not set at build time.
    pub build_date: String,
    /// URL to the project repository.
    pub repo_url: String,
}

/// Return build-time metadata for the About section (BH-12.1).
///
/// The `commit` and `build_date` fields are populated at build time via
/// `cargo:rustc-env=GIT_COMMIT=…` and `cargo:rustc-env=BUILD_DATE=…` from
/// `.cargo/config.toml` or the CI pipeline. For local dev builds they fall
/// back to `"dev"`.
#[tauri::command]
pub fn get_app_info() -> AppInfoDto {
    AppInfoDto {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("GIT_COMMIT").unwrap_or("dev").to_string(),
        build_date: option_env!("BUILD_DATE").unwrap_or("dev").to_string(),
        repo_url: "https://github.com/poneding/translator".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct UpdateInfoDto {
    pub available: bool,
    pub version: Option<String>,
    pub current_version: String,
    pub channel: String,
    pub date: Option<String>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpdateStatusDto {
    pub status: String,
    pub update: Option<UpdateInfoDto>,
    pub error: Option<String>,
    pub downloaded: Option<u64>,
    pub total: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CheckUpdateArgs {
    pub manual: Option<bool>,
}

/// Check for updates with the official Tauri updater plugin.
#[tauri::command]
pub async fn check_update<R: Runtime>(
    app: AppHandle<R>,
    args: CheckUpdateArgs,
) -> Result<UpdateStatusDto, String> {
    let _manual = args.manual.unwrap_or(false);
    emit_update_status(&app, UpdateStatusDto::checking());
    let cfg = Config::load().map_err(|e| e.to_string())?;
    let status = check_update_inner(&app, &cfg).await;
    emit_update_status(&app, status.clone());
    Ok(status)
}

/// Download and install the currently available update.
#[tauri::command]
pub async fn download_and_install_update<R: Runtime>(
    app: AppHandle<R>,
) -> Result<UpdateStatusDto, String> {
    emit_update_status(&app, UpdateStatusDto::installing(None, None));
    let cfg = Config::load().map_err(|e| e.to_string())?;
    let updater = match updater_for_config(&app, &cfg) {
        Ok(updater) => updater,
        Err(error) => {
            let status = UpdateStatusDto::failed(error);
            emit_update_status(&app, status.clone());
            return Ok(status);
        }
    };
    let update = match updater.check().await {
        Ok(Some(update)) => update,
        Ok(None) => {
            let status = UpdateStatusDto::failed("no update available".to_string());
            emit_update_status(&app, status.clone());
            return Ok(status);
        }
        Err(error) => {
            let status = UpdateStatusDto::failed(error.to_string());
            emit_update_status(&app, status.clone());
            return Ok(status);
        }
    };

    let progress_app = app.clone();
    let mut downloaded = 0_u64;
    let result = update
        .download_and_install(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                emit_update_status(
                    &progress_app,
                    UpdateStatusDto::installing(Some(downloaded), total),
                );
            },
            || {
                emit_update_status(&progress_app, UpdateStatusDto::installing(None, None));
            },
        )
        .await;

    match result {
        Ok(()) => {
            let status = UpdateStatusDto {
                status: "installed".to_string(),
                update: None,
                error: None,
                downloaded: None,
                total: None,
            };
            emit_update_status(&app, status.clone());
            Ok(status)
        }
        Err(error) => {
            let status = UpdateStatusDto::failed(error.to_string());
            emit_update_status(&app, status.clone());
            Ok(status)
        }
    }
}

pub(crate) async fn run_startup_update_check<R: Runtime>(app: AppHandle<R>) {
    let cfg = Config::load().unwrap_or_default();
    if !cfg.updates.check_on_startup {
        return;
    }
    let status = check_update_inner(&app, &cfg).await;
    if status.status == "failed"
        && let Some(error) = &status.error
    {
        tracing::warn!(error = %error, "startup update check failed");
    }
    emit_update_status(&app, status);
}

async fn check_update_inner<R: Runtime>(app: &AppHandle<R>, cfg: &Config) -> UpdateStatusDto {
    let updater = match updater_for_config(app, cfg) {
        Ok(updater) => updater,
        Err(error) => return UpdateStatusDto::failed(error),
    };

    match updater.check().await {
        Ok(Some(update)) => UpdateStatusDto {
            status: "available".to_string(),
            update: Some(UpdateInfoDto {
                available: true,
                version: Some(update.version.clone()),
                current_version: update.current_version.clone(),
                channel: update_channel(&update.version),
                date: update.date.map(|date| date.to_string()),
                body: update.body.clone(),
            }),
            error: None,
            downloaded: None,
            total: None,
        },
        Ok(None) => UpdateStatusDto {
            status: "up-to-date".to_string(),
            update: Some(UpdateInfoDto {
                available: false,
                version: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                channel: if cfg.updates.allow_beta {
                    "beta".to_string()
                } else {
                    "stable".to_string()
                },
                date: None,
                body: None,
            }),
            error: None,
            downloaded: None,
            total: None,
        },
        Err(error) => UpdateStatusDto::failed(error.to_string()),
    }
}

fn updater_for_config<R: Runtime>(
    app: &AppHandle<R>,
    cfg: &Config,
) -> Result<tauri_plugin_updater::Updater, String> {
    let endpoint = if cfg.updates.allow_beta {
        BETA_UPDATE_ENDPOINT
    } else {
        STABLE_UPDATE_ENDPOINT
    };
    let endpoint = url::Url::parse(endpoint).map_err(|e| e.to_string())?;
    let allow_beta = cfg.updates.allow_beta;
    let mut builder = app
        .updater_builder()
        .version_comparator(move |current, remote| {
            remote.version > current && (allow_beta || remote.version.pre.is_empty())
        });

    if cfg.general.proxy.enabled && !cfg.general.proxy.url.trim().is_empty() {
        let proxy = url::Url::parse(cfg.general.proxy.url.trim()).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }

    let builder = builder
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?;
    builder.build().map_err(|e| e.to_string())
}

fn update_channel(version: &str) -> String {
    if version.contains('-') {
        "beta".to_string()
    } else {
        "stable".to_string()
    }
}

fn emit_update_status<R: Runtime>(app: &AppHandle<R>, status: UpdateStatusDto) {
    let _ = app.emit("translator://update-status", status);
}

impl UpdateStatusDto {
    fn checking() -> Self {
        Self {
            status: "checking".to_string(),
            update: None,
            error: None,
            downloaded: None,
            total: None,
        }
    }

    fn installing(downloaded: Option<u64>, total: Option<u64>) -> Self {
        Self {
            status: "installing".to_string(),
            update: None,
            error: None,
            downloaded,
            total,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            status: "failed".to_string(),
            update: None,
            error: Some(error),
            downloaded: None,
            total: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SetApiKeyArgs {
    #[serde(alias = "serviceId")]
    pub service_id: String,
    #[serde(alias = "apiKey")]
    pub api_key: String,
}

/// Persist an API key to the OS Keychain.
#[tauri::command]
pub fn set_api_key(args: SetApiKeyArgs) -> Result<(), String> {
    translator_core::secrets::set_api_key(&args.service_id, &args.api_key)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct DeleteApiKeyArgs {
    #[serde(alias = "serviceId")]
    pub service_id: String,
}

/// Remove an API key from the OS Keychain.
#[tauri::command]
pub fn delete_api_key(args: DeleteApiKeyArgs) -> Result<(), String> {
    translator_core::secrets::delete_api_key(&args.service_id).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct HasApiKeyArgs {
    #[serde(alias = "serviceId")]
    pub service_id: String,
}

/// BH-8.1: cheap boolean probe so the settings row can show a status indicator
/// without trying to read the secret value.
#[tauri::command]
pub fn has_api_key(args: HasApiKeyArgs) -> Result<bool, String> {
    translator_core::secrets::has_api_key(&args.service_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use serde_json::json;

    use super::{
        DeleteApiKeyArgs, HasApiKeyArgs, MainWindowWorkArea, PhysicalPosition, PhysicalSize,
        SetApiKeyArgs, WINDOW_EDGE_MARGIN, macos_designated_requirement_is_cdhash_only,
        resolve_main_window_position, should_position_main_window_on_show,
    };
    use translator_core::config::WindowPosition;

    #[test]
    fn keychain_ipc_args_accept_frontend_shape() {
        let set: SetApiKeyArgs =
            serde_json::from_value(json!({ "serviceId": "openai", "apiKey": "secret" }))
                .expect("set args should deserialize from frontend payload");
        assert_eq!(set.service_id, "openai");
        assert_eq!(set.api_key, "secret");

        let delete: DeleteApiKeyArgs = serde_json::from_value(json!({ "serviceId": "openai" }))
            .expect("delete args should deserialize from frontend payload");
        assert_eq!(delete.service_id, "openai");

        let has: HasApiKeyArgs = serde_json::from_value(json!({ "serviceId": "openai" }))
            .expect("has args should deserialize from frontend payload");
        assert_eq!(has.service_id, "openai");
    }

    #[test]
    fn remembered_window_position_uses_saved_coordinates_when_available() {
        let work_area = MainWindowWorkArea {
            left: 0,
            top: 0,
            right: 1440,
            bottom: 900,
        };

        let position = resolve_main_window_position(
            "remember",
            Some(WindowPosition { x: 240, y: 160 }),
            None,
            work_area,
            PhysicalSize::new(680, 560),
        );

        assert_eq!(position, PhysicalPosition::new(240, 160));
    }

    #[test]
    fn remembered_window_position_falls_back_to_top_right_without_saved_coordinates() {
        let work_area = MainWindowWorkArea {
            left: 0,
            top: 0,
            right: 1440,
            bottom: 900,
        };

        let position = resolve_main_window_position(
            "remember",
            None,
            None,
            work_area,
            PhysicalSize::new(680, 560),
        );

        assert_eq!(
            position,
            PhysicalPosition::new(1440 - 680 - WINDOW_EDGE_MARGIN, WINDOW_EDGE_MARGIN),
        );
    }

    #[test]
    fn remembered_window_position_is_preserved_on_regular_show() {
        assert!(!should_position_main_window_on_show("remember"));
        assert!(should_position_main_window_on_show("right"));
        assert!(should_position_main_window_on_show("center"));
        assert!(should_position_main_window_on_show("mouse"));
    }

    #[test]
    fn macos_dock_reopen_routes_to_main_window_show() {
        let main_source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
        let source = fs::read_to_string(main_source_path).expect("main.rs should be readable");

        assert!(
            source.contains("RunEvent::Reopen")
                && source
                    .contains("commands::show_main_window(app, Some(\"translator://open-main\"))"),
            "macOS Dock reopen should route through the normal main-window show path",
        );
    }

    #[test]
    fn macos_defaults_to_menu_bar_only_without_dock_icon() {
        let main_source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
        let main_source = fs::read_to_string(main_source_path).expect("main.rs should be readable");
        let tauri_config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tauri.conf.json")
            .canonicalize()
            .expect("tauri.conf.json path should resolve");
        let tauri_config =
            fs::read_to_string(tauri_config_path).expect("tauri.conf.json should be readable");
        let info_plist_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("Info.plist")
            .canonicalize()
            .expect("Info.plist path should resolve");
        let info_plist =
            fs::read_to_string(&info_plist_path).expect("Info.plist should be readable");

        assert!(
            main_source.contains("ActivationPolicy::Accessory")
                && main_source.contains("set_dock_visibility(false)"),
            "macOS app startup should switch to accessory activation policy and hide the Dock icon",
        );
        assert!(
            !tauri_config.contains("\"infoPlist\"") && info_plist_path.exists(),
            "macOS bundle config should rely on the local Info.plist override file",
        );
        assert!(
            info_plist.contains("<key>LSUIElement</key>") && info_plist.contains("<true/>"),
            "Info.plist should declare LSUIElement so packaged macOS builds default to menu-bar-only mode",
        );
    }

    #[test]
    fn macos_activation_policy_import_is_cfg_gated() {
        let main_source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
        let main_source = fs::read_to_string(main_source_path).expect("main.rs should be readable");
        let normalized_main_source = main_source.replace("\r\n", "\n");

        assert!(
            !main_source.contains("use tauri::{ActivationPolicy, Manager};"),
            "ActivationPolicy is macOS-only in Tauri and must not be imported on every target",
        );
        assert!(
            normalized_main_source
                .contains("#[cfg(target_os = \"macos\")]\nuse tauri::ActivationPolicy;")
                && main_source.contains("use tauri::Manager;"),
            "main.rs should cfg-gate ActivationPolicy while importing cross-platform Tauri traits normally",
        );
    }

    #[test]
    fn startup_respects_menu_bar_icon_app_setting() {
        let main_source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
        let main_source = fs::read_to_string(main_source_path).expect("main.rs should be readable");

        assert!(
            main_source.contains("cfg.app.show_menu_bar_icon"),
            "startup should create the tray/menu-bar icon based on the app setting",
        );
        assert!(
            !main_source.contains("// Build tray.\n            tray::build_tray(app.handle())?;"),
            "startup should not build the tray/menu-bar icon unconditionally",
        );
    }

    #[test]
    fn save_config_synchronizes_app_shell_settings() {
        let commands_source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands.rs");
        let commands_source =
            fs::read_to_string(commands_source_path).expect("commands.rs should be readable");
        let production_source = commands_source
            .split("#[cfg(test)]")
            .next()
            .expect("commands.rs should contain production source");

        assert!(
            production_source.contains("old.app.launch_at_startup")
                && production_source.contains("config.app.launch_at_startup"),
            "save_config should sync autostart from app settings",
        );
        assert!(
            production_source.contains("sync_tray_visibility(&app, config.app.show_menu_bar_icon)"),
            "save_config should immediately sync tray/menu-bar visibility",
        );
        assert!(
            production_source.contains("old.general.app_language != config.general.app_language")
                && production_source.contains("rebuild_tray(&app)"),
            "save_config should rebuild the tray/menu-bar menu when the app language changes",
        );
    }

    #[test]
    fn macos_cdhash_only_requirement_is_unstable_for_tcc() {
        let ad_hoc_requirement = r#"Executable=/Applications/Translator.app/Contents/MacOS/translator-app
# designated => cdhash H"43048cea4985caba72b8373a0694923c9f21a0b7""#;

        assert!(macos_designated_requirement_is_cdhash_only(
            ad_hoc_requirement
        ));
    }

    #[test]
    fn macos_certificate_requirement_is_not_cdhash_only() {
        let developer_id_requirement = r#"Executable=/Applications/Translator.app/Contents/MacOS/translator-app
# designated => identifier "dev.translator.desktop" and anchor apple generic and certificate leaf[subject.OU] = TEAMID1234"#;

        assert!(!macos_designated_requirement_is_cdhash_only(
            developer_id_requirement
        ));
    }

    #[test]
    fn release_workflow_requires_fixed_macos_code_signing_identity() {
        let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.github/workflows/release.yml")
            .canonicalize()
            .expect("release workflow path should resolve");
        let workflow =
            fs::read_to_string(workflow_path).expect("release workflow should be readable");

        for required in [
            "Install macOS signing certificate",
            "MACOS_CODESIGN_CERTIFICATE",
            "MACOS_CODESIGN_CERTIFICATE_PASSWORD",
            "MACOS_CODESIGN_IDENTITY",
            "APPLE_SIGNING_IDENTITY=$MACOS_CODESIGN_IDENTITY",
            "max-parallel: 1",
            "Clean stale Tauri bundles",
            "retryAttempts: 2",
            "Verify macOS code signature",
            "scripts/macos-sign-app.sh --verify-only",
        ] {
            assert!(
                workflow.contains(required),
                "release workflow must contain {required:?}",
            );
        }
    }

    #[test]
    fn macos_signing_script_rejects_cdhash_only_apps() {
        let script_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/macos-sign-app.sh")
            .canonicalize()
            .expect("macOS signing script path should resolve");
        let script =
            fs::read_to_string(script_path).expect("macOS signing script should be readable");

        assert!(
            script.contains("cdhash H\\\"") || script.contains("cdhash H\""),
            "signing script should inspect designated requirements for cdhash-only signatures",
        );
        assert!(
            script.contains("macOS designated requirement is cdhash-only")
                && script.contains("ad-hoc signing is not stable enough"),
            "signing script must reject signatures that will break TCC grants",
        );
    }

    #[test]
    fn main_window_uses_native_platform_decorations() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(config_path).expect("tauri.conf.json should be readable"),
        )
        .expect("tauri.conf.json should be valid JSON");
        let windows = config["app"]["windows"]
            .as_array()
            .expect("app.windows should be an array");
        let main = windows
            .iter()
            .find(|window| window["label"] == "main")
            .expect("main window config should exist");

        assert_ne!(
            main.get("decorations").and_then(serde_json::Value::as_bool),
            Some(false),
            "main window must keep native platform decorations so macOS gets default rounded corners and traffic lights",
        );
    }

    #[test]
    fn app_does_not_render_fake_macos_traffic_lights() {
        let app_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/App.tsx")
            .canonicalize()
            .expect("App.tsx path should resolve");
        let source = fs::read_to_string(app_source).expect("App.tsx should be readable");

        assert!(
            !source.contains("MacWindowControls"),
            "macOS must use native traffic-light controls instead of CSS-drawn window controls",
        );
    }

    #[test]
    fn macos_traffic_lights_are_vertically_centered_in_titlebar() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(config_path).expect("tauri.conf.json should be readable"),
        )
        .expect("tauri.conf.json should be valid JSON");
        let app_css_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/app.css")
            .canonicalize()
            .expect("app.css path should resolve");
        let app_css = fs::read_to_string(app_css_path).expect("app.css should be readable");
        let main = config["app"]["windows"]
            .as_array()
            .expect("app.windows should be an array")
            .iter()
            .find(|window| window["label"] == "main")
            .expect("main window config should exist");

        assert!(
            app_css.contains("h-[34px]"),
            "titlebar height changed; re-check traffic light centering",
        );
        assert_eq!(
            main["trafficLightPosition"]["x"].as_i64(),
            Some(12),
            "macOS traffic lights should sit close to the left edge without touching it",
        );
        assert_eq!(
            main["trafficLightPosition"]["y"].as_i64(),
            Some(14),
            "macOS traffic lights should be visually centered inside the 34px titlebar",
        );
    }

    #[test]
    fn language_dropdown_flags_are_platform_gated() {
        let combobox_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/components/Combobox.tsx")
            .canonicalize()
            .expect("Combobox.tsx path should resolve");
        let source = fs::read_to_string(combobox_source).expect("Combobox.tsx should be readable");

        assert!(
            source.contains("showOptionFlag") && source.contains("macos"),
            "language dropdown flags should be rendered only when the host platform is macOS",
        );
    }

    #[test]
    fn language_direction_renders_language_names_and_short_codes() {
        let app_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/App.tsx")
            .canonicalize()
            .expect("App.tsx path should resolve");
        let source = fs::read_to_string(app_source).expect("App.tsx should be readable");

        assert!(
            source.contains("source-language-token") && source.contains("selectedDisplay=\"full\""),
            "source-to-target language display should include language names plus short codes",
        );
    }

    #[test]
    fn enabled_service_logos_do_not_use_negative_overlap() {
        let app_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/App.tsx")
            .canonicalize()
            .expect("App.tsx path should resolve");
        let source = fs::read_to_string(app_source).expect("App.tsx should be readable");
        let css_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/app.css")
            .canonicalize()
            .expect("app.css path should resolve");
        let css = fs::read_to_string(css_path).expect("app.css should be readable");

        assert!(
            source.contains("service-logo-frame") && css.contains("-space-x-1"),
            "enabled service logos should overlap as connected frames",
        );
        assert!(
            css.contains(".service-logo-frame")
                && css.contains("ring-2")
                && css.contains("ring-bg")
                && !css.contains(
                    ".service-logo-strip {\n    @apply inline-flex shrink-0 items-center gap-1;"
                ),
            "overlapped service logos should use background rings so borders do not cover adjacent logos",
        );
    }

    #[test]
    fn services_can_be_reordered_with_drag_handle_button() {
        let services_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/ServicesSection.tsx")
            .canonicalize()
            .expect("ServicesSection.tsx path should resolve");
        let source =
            fs::read_to_string(services_source).expect("ServicesSection.tsx should be readable");

        assert!(
            source.contains("<button")
                && source.contains("onPointerDown")
                && source.contains("onPointerEnter")
                && source.contains("onPointerUp")
                && source.contains("settings-services-drag-aria")
                && source.contains("save({ ...config, services: nextServices })"),
            "services should use a draggable handle button that persists priority order",
        );
        assert!(
            !source.contains("ArrowUp")
                && !source.contains("ArrowDown")
                && !source.contains("settings-services-move-up")
                && !source.contains("settings-services-move-down"),
            "service ordering should not add separate up/down move buttons",
        );
    }

    #[test]
    fn default_main_window_width_uses_original_680() {
        let commands_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands.rs");
        let commands = fs::read_to_string(commands_path).expect("commands.rs should be readable");
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(config_path).expect("tauri.conf.json should be readable"),
        )
        .expect("tauri.conf.json should be valid JSON");
        let main = config["app"]["windows"]
            .as_array()
            .expect("app.windows should be an array")
            .iter()
            .find(|window| window["label"] == "main")
            .expect("main window config should exist");

        assert!(
            commands.contains("MAIN_WINDOW_DEFAULT_WIDTH: f64 = 680.0")
                && main["width"].as_u64() == Some(680)
                && main["minWidth"].as_u64() == Some(680),
            "default main window width and minimum width should use the original 680",
        );
    }

    #[test]
    fn scrollbars_use_theme_variables_for_thumb_and_track() {
        let css_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/app.css")
            .canonicalize()
            .expect("app.css path should resolve");
        let source = fs::read_to_string(css_path).expect("app.css should be readable");

        assert!(
            source.contains("--scrollbar-track")
                && source.contains("scrollbar-color: transparent transparent")
                && source.contains(".is-scrolling")
                && source.contains("scrollbar-color: rgb(var(--scrollbar-thumb)) transparent",),
            "scrollbars should be hidden by default and use theme colors while scrolling",
        );
        assert!(
            !source.contains("background-color: rgb(var(--scrollbar-track))")
                && !source.contains(":focus-within::-webkit-scrollbar"),
            "scrollbar tracks should stay transparent and scrollbars should not remain visible just because a control has focus",
        );
    }

    #[test]
    fn results_scrollbar_is_hidden_without_affecting_source_editor() {
        let app_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/App.tsx")
            .canonicalize()
            .expect("App.tsx path should resolve");
        let app = fs::read_to_string(app_source).expect("App.tsx should be readable");
        let css_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/app.css")
            .canonicalize()
            .expect("app.css path should resolve");
        let css = fs::read_to_string(css_path).expect("app.css should be readable");

        assert!(
            app.contains("main-shell") && app.contains("results-scroll"),
            "results should use named layout containers for independent scrolling",
        );
        assert!(
            css.contains(".results-scroll")
                && css.contains("overflow-y-auto")
                && css.contains("scrollbar-width: none")
                && css.contains(".results-scroll::-webkit-scrollbar")
                && css.contains("display: none"),
            "result area should remain scrollable while hiding its scrollbar",
        );
        assert!(
            !css.contains(".main-shell:not(.main-shell-history) .results-scroll")
                && !css.contains("padding-right: calc(1rem + var(--scrollbar-size))")
                && !css.contains("gap-3 overflow-hidden"),
            "result scrolling must not add horizontal compensation or clip the source editor focus ring",
        );
    }

    #[test]
    fn settings_window_position_defaults_to_remember_option() {
        let general_section_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/GeneralSection.tsx")
            .canonicalize()
            .expect("GeneralSection.tsx path should resolve");
        let source = fs::read_to_string(general_section_path)
            .expect("GeneralSection.tsx should be readable");

        assert!(
            source.contains("\"remember\"")
                && source.contains("settings-general-window-position-remember")
                && source.contains(": \"remember\""),
            "settings window position selector should offer remembered position and normalize unknown values to remember",
        );
    }

    #[test]
    fn app_shell_controls_live_in_general_settings() {
        let settings_app_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/SettingsApp.tsx")
            .canonicalize()
            .expect("SettingsApp.tsx path should resolve");
        let settings_app =
            fs::read_to_string(settings_app_path).expect("SettingsApp.tsx should be readable");
        let general_section_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/GeneralSection.tsx")
            .canonicalize()
            .expect("GeneralSection.tsx path should resolve");
        let general_section = fs::read_to_string(general_section_path)
            .expect("GeneralSection.tsx should be readable");

        assert!(
            !settings_app.contains("AppSection")
                && !settings_app.contains("\"settings-nav-app\"")
                && !settings_app.contains("id: \"app\""),
            "settings UI should not expose a separate App settings group",
        );
        for required in [
            "config.app.show_menu_bar_icon",
            "config.app.launch_at_startup",
            "settings-general-show-menu-bar-icon",
            "settings-general-launch-at-startup",
        ] {
            assert!(
                general_section.contains(required),
                "General settings should contain {required:?}",
            );
        }
    }

    #[test]
    fn update_check_action_lives_in_update_heading_and_idle_status_is_hidden() {
        let settings_app_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/SettingsApp.tsx")
            .canonicalize()
            .expect("SettingsApp.tsx path should resolve");
        let settings_app =
            fs::read_to_string(settings_app_path).expect("SettingsApp.tsx should be readable");
        let update_section_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/UpdateSection.tsx")
            .canonicalize()
            .expect("UpdateSection.tsx path should resolve");
        let update_section =
            fs::read_to_string(update_section_path).expect("UpdateSection.tsx should be readable");

        assert!(
            settings_app.contains("titleAction")
                && settings_app.contains("<UpdateCheckButton")
                && settings_app.contains("justify-between"),
            "update check button should render in the update section heading, aligned to the right",
        );
        assert!(
            update_section.contains("status.status !== \"idle\"")
                && !update_section.contains("settings-update-status-idle"),
            "idle update status should not render the default 'not checked' message",
        );
    }

    #[test]
    fn restart_action_is_available_from_tray_and_ipc() {
        let commands_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands.rs");
        let commands = fs::read_to_string(commands_path).expect("commands.rs should be readable");
        let production_commands = commands
            .split("#[cfg(test)]")
            .next()
            .expect("commands.rs should contain production source");
        let main_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
        let main = fs::read_to_string(main_path).expect("main.rs should be readable");
        let tray_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tray.rs");
        let tray = fs::read_to_string(tray_path).expect("tray.rs should be readable");
        let frontend_commands_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/ipc/commands.ts")
            .canonicalize()
            .expect("frontend commands path should resolve");
        let frontend_commands =
            fs::read_to_string(frontend_commands_path).expect("commands.ts should be readable");

        assert!(
            production_commands.contains("pub fn restart_app")
                && production_commands.contains("app.restart();")
                && !production_commands.contains("app.request_restart();")
                && production_commands.contains("remember_main_webview_window_position"),
            "restart command should remember window position and restart the app",
        );
        assert!(
            main.contains("commands::restart_app")
                && frontend_commands.contains("restartApp")
                && frontend_commands.contains("invoke<void>(\"restart_app\")"),
            "restart command should be registered for frontend IPC",
        );
        assert!(
            tray.contains("\"restart\"")
                && tray.contains("tray-restart")
                && tray.contains("&labels.restart")
                && tray.contains("commands::restart_app(app.clone())"),
            "tray/menu-bar menu should expose a localized restart item that uses the shared restart command",
        );
    }

    #[test]
    fn check_update_action_opens_settings_update_section_from_tray() {
        let tray_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tray.rs");
        let tray = fs::read_to_string(tray_path).expect("tray.rs should be readable");
        let app_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/App.tsx")
            .canonicalize()
            .expect("App.tsx path should resolve");
        let app = fs::read_to_string(app_path).expect("App.tsx should be readable");
        let settings_app_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/SettingsApp.tsx")
            .canonicalize()
            .expect("SettingsApp.tsx path should resolve");
        let settings_app =
            fs::read_to_string(settings_app_path).expect("SettingsApp.tsx should be readable");
        let frontend_commands_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/ipc/commands.ts")
            .canonicalize()
            .expect("frontend commands path should resolve");
        let frontend_commands =
            fs::read_to_string(frontend_commands_path).expect("commands.ts should be readable");
        let en_locale_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/locales/en.ftl")
            .canonicalize()
            .expect("en locale path should resolve");
        let en_locale = fs::read_to_string(en_locale_path).expect("en locale should be readable");
        let zh_locale_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/locales/zh-Hans.ftl")
            .canonicalize()
            .expect("zh-Hans locale path should resolve");
        let zh_locale =
            fs::read_to_string(zh_locale_path).expect("zh-Hans locale should be readable");

        assert!(
            tray.contains("\"check_update\"")
                && tray.contains("tray-check-update")
                && tray.contains("translator://check-update"),
            "tray/menu-bar menu should expose a localized check-update item that emits the check-update event",
        );
        assert!(
            frontend_commands.contains("onCheckUpdateRequested")
                && frontend_commands.contains("translator://check-update"),
            "frontend IPC should listen for the check-update menu event",
        );
        assert!(
            app.contains("api.onCheckUpdateRequested")
                && app.contains("setActiveView(\"settings\")")
                && app.contains("checkUpdate: true")
                && app.contains("section: \"update\""),
            "main window should switch to settings and request the update section check",
        );
        assert!(
            settings_app.contains("scrollIntoView")
                && settings_app.contains("void checkForUpdates()")
                && settings_app.contains("scroll-pt-3")
                && settings_app.contains("scroll-mt-3")
                && settings_app.contains("onRequestHandled"),
            "settings view should scroll to the requested section with top breathing room and start update detection once",
        );
        assert!(
            en_locale.contains("tray-check-update = Check for Updates")
                && zh_locale.contains("tray-check-update = 检查更新"),
            "check-update tray action should be localized in English and Simplified Chinese",
        );
        assert!(
            en_locale.contains("tray-open-main = Main")
                && en_locale.contains("tray-open-settings = Settings")
                && en_locale.contains("tray-restart = Restart")
                && zh_locale.contains("tray-open-main = 主界面")
                && zh_locale.contains("tray-open-settings = 设置")
                && zh_locale.contains("tray-restart = 重新启动"),
            "primary tray actions should use short localized labels",
        );
    }

    #[test]
    fn installed_update_status_shows_restart_action() {
        let update_section_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/UpdateSection.tsx")
            .canonicalize()
            .expect("UpdateSection.tsx path should resolve");
        let update_section =
            fs::read_to_string(update_section_path).expect("UpdateSection.tsx should be readable");
        let update_controls_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/useUpdateControls.ts")
            .canonicalize()
            .expect("useUpdateControls.ts path should resolve");
        let update_controls = fs::read_to_string(update_controls_path)
            .expect("useUpdateControls.ts should be readable");
        let en_locale_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/locales/en.ftl")
            .canonicalize()
            .expect("en locale path should resolve");
        let en_locale = fs::read_to_string(en_locale_path).expect("en locale should be readable");
        let zh_locale_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/locales/zh-Hans.ftl")
            .canonicalize()
            .expect("zh-Hans locale path should resolve");
        let zh_locale =
            fs::read_to_string(zh_locale_path).expect("zh-Hans locale should be readable");

        assert!(
            update_section.contains("status.status === \"installed\"")
                && update_section.contains("controls.restart()")
                && update_section.contains("settings-update-restart"),
            "installed update status should render a restart button",
        );
        assert!(
            update_controls.contains("restart: () => Promise<void>")
                && update_controls.contains("api.restartApp()"),
            "update controls should expose a restart action",
        );
        assert!(
            en_locale.contains("settings-update-restart = Restart")
                && zh_locale.contains("settings-update-restart = 重新启动"),
            "restart action should be localized in English and Simplified Chinese",
        );
    }

    #[test]
    fn available_update_body_renders_as_markdown_changelog() {
        let update_section_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/settings/sections/UpdateSection.tsx")
            .canonicalize()
            .expect("UpdateSection.tsx path should resolve");
        let update_section =
            fs::read_to_string(update_section_path).expect("UpdateSection.tsx should be readable");
        let app_css_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/app.css")
            .canonicalize()
            .expect("app.css path should resolve");
        let app_css = fs::read_to_string(app_css_path).expect("app.css should be readable");
        let package_json_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/package.json")
            .canonicalize()
            .expect("package.json path should resolve");
        let package_json =
            fs::read_to_string(package_json_path).expect("package.json should be readable");

        assert!(
            update_section.contains("ReactMarkdown")
                && update_section.contains("remarkGfm")
                && update_section.contains("status.update.body")
                && update_section.contains("update-changelog"),
            "available update body should render through markdown with GitHub-flavored markdown enabled",
        );
        assert!(
            update_section.contains("openExternalUrl(url)"),
            "links inside update markdown should open through the trusted external URL command",
        );
        assert!(
            app_css.contains(".update-changelog")
                && app_css.contains(".update-changelog ul")
                && app_css.contains(".update-changelog table"),
            "markdown changelog should have local styles for common release-note elements",
        );
        assert!(
            package_json.contains("\"react-markdown\"") && package_json.contains("\"remark-gfm\""),
            "markdown rendering dependencies should be declared",
        );
    }

    #[test]
    fn app_and_settings_install_auto_hide_scrollbars() {
        let app_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/App.tsx")
            .canonicalize()
            .expect("App.tsx path should resolve");
        let app = fs::read_to_string(app_source).expect("App.tsx should be readable");
        let settings_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/SettingsApp.tsx")
            .canonicalize()
            .expect("SettingsApp.tsx path should resolve");
        let settings =
            fs::read_to_string(settings_source).expect("SettingsApp.tsx should be readable");
        let hook_source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/hooks/useAutoHideScrollbars.ts")
            .canonicalize()
            .expect("useAutoHideScrollbars.ts path should resolve");
        let hook = fs::read_to_string(hook_source).expect("hook should be readable");

        assert!(
            app.contains("useAutoHideScrollbars();")
                && settings.contains("useAutoHideScrollbars();"),
            "main app and standalone settings app should both install the scrollbar auto-hide listener",
        );
        assert!(
            hook.contains("is-scrolling") && hook.contains("setTimeout"),
            "auto-hide hook should mark scrolling containers only while scrolling",
        );
    }

    #[test]
    fn html_sets_initial_theme_before_react_loads() {
        for name in ["index.html", "settings.html"] {
            let html_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../ui")
                .join(name)
                .canonicalize()
                .expect("html path should resolve");
            let html = fs::read_to_string(html_path).expect("html should be readable");

            assert!(
                html.contains("translator-theme")
                    && html.contains("data-theme")
                    && html.contains("prefers-color-scheme: dark")
                    && html.contains("rgb(20, 22, 28)")
                    && html.contains("color-scheme"),
                "{name} should paint the initial theme in <head> before React and app.css load",
            );
        }
    }

    #[test]
    fn main_window_uses_dark_startup_background_color() {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
        let config: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(config_path).expect("tauri.conf.json should be readable"),
        )
        .expect("tauri.conf.json should be valid JSON");
        let windows = config["app"]["windows"]
            .as_array()
            .expect("app.windows should be an array");
        let main = windows
            .iter()
            .find(|window| window["label"] == "main")
            .expect("main window config should exist");

        assert_eq!(
            main["backgroundColor"].as_str(),
            Some("#14161c"),
            "main window and webview should start on the dark app background instead of white",
        );
    }

    #[test]
    fn theme_hook_preserves_bootstrap_theme_until_config_loads() {
        let hook_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../ui/src/hooks/useTheme.ts")
            .canonicalize()
            .expect("useTheme.ts path should resolve");
        let source = fs::read_to_string(hook_path).expect("useTheme.ts should be readable");

        assert!(
            source.contains("ThemeChoice | null | undefined")
                && source.contains("if (!choice) return;")
                && source.contains("translator-theme")
                && source.contains("localStorage.setItem")
                && source.contains("localStorage.removeItem"),
            "useTheme should leave the inline bootstrap theme alone until config loads, then cache explicit choices",
        );
    }

    #[test]
    fn apps_wait_for_config_before_applying_theme_choice() {
        for name in ["App.tsx", "SettingsApp.tsx"] {
            let app_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../ui/src")
                .join(name)
                .canonicalize()
                .expect("app source path should resolve");
            let source = fs::read_to_string(app_path).expect("app source should be readable");

            assert!(
                source.contains("useTheme(config?.general.theme")
                    && !source.contains("??\n      \"system\""),
                "{name} should not apply the system theme before persisted config has loaded",
            );
        }
    }

    #[test]
    fn tray_uses_shared_icon_asset_with_runtime_menu_bar_scaling() {
        let tray_source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tray.rs");
        let source = fs::read_to_string(tray_source_path).expect("tray.rs should be readable");

        assert!(
            source.contains("include_bytes!(\"../icons/icon.png\")")
                && !source.contains("tray-icon.png")
                && source.contains("extract_template_glyph_mask")
                && source.contains("crop_transparent_padding")
                && source.contains(".icon_as_template(true)"),
            "tray should use the shared app icon asset, extract the white logo glyph for the macOS template image, and crop transparent padding for menu bar sizing",
        );
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateHotkeyArgs {
    pub shortcut: String,
}

/// BH-10.3: re-register the live global shortcut. Parses the new shortcut
/// string in tauri-plugin-global-shortcut format, unregisters the previous
/// one, and registers the new one. On success the new value is persisted
/// to config so it survives restart.
///
/// BH-1.5: if the OS rejects the registration (conflict with another app),
/// the `hotkey_registration_failed` flag is set in config so the settings UI
/// can show a red banner. On next launch the launch code resets the
/// shortcut to the default and clears the flag.
#[tauri::command]
pub fn update_hotkey<R: Runtime>(app: AppHandle<R>, args: UpdateHotkeyArgs) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    use tracing::warn;

    let new = parse_shortcut(&args.shortcut).map_err(|e| e.to_string())?;

    // Unregister everything currently registered, then try to register the new one.
    // If the OS denies the registration, mark the failure in config (BH-1.5)
    // BEFORE returning the error so the UI can surface it immediately.
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if let Err(e) = gs.register(new) {
        warn!("hotkey registration failed: {e}");
        if let Ok(mut cfg) = Config::load() {
            cfg.hotkey_registration_failed = true;
            let _ = cfg.save();
        }
        return Err(format!("hotkey registration failed: {e}"));
    }

    // Registration succeeded. Persist the new shortcut and clear the flag.
    let mut cfg = Config::load().map_err(|e| e.to_string())?;
    cfg.shortcut = args.shortcut.clone();
    cfg.hotkey_registration_failed = false;
    cfg.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Parse a tauri-plugin-global-shortcut string like "Cmd+T" or "Alt+T".
pub(crate) fn parse_shortcut(s: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;
    for part in s.split('+') {
        let p = part.trim();
        if p.is_empty() {
            return Err(format!("empty modifier in shortcut: {s:?}"));
        }
        match p.to_ascii_lowercase().as_str() {
            "cmdorctrl" | "commandorcontrol" | "commandorctrl" | "cmdorcontrol" => {
                mods |= cmd_or_ctrl_modifier();
            }
            "cmd" | "command" | "super" | "meta" | "win" => mods |= Modifiers::SUPER,
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            other => {
                key = Some(
                    map_key_code(other)
                        .ok_or_else(|| format!("unrecognized key in shortcut {s:?}: {other:?}"))?,
                );
            }
        }
    }
    let key = key.ok_or_else(|| format!("shortcut {s:?} has no key"))?;
    Ok(Shortcut::new(Some(mods), key))
}

#[cfg(target_os = "macos")]
fn cmd_or_ctrl_modifier() -> tauri_plugin_global_shortcut::Modifiers {
    tauri_plugin_global_shortcut::Modifiers::SUPER
}

#[cfg(not(target_os = "macos"))]
fn cmd_or_ctrl_modifier() -> tauri_plugin_global_shortcut::Modifiers {
    tauri_plugin_global_shortcut::Modifiers::CONTROL
}

fn map_key_code(name: &str) -> Option<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code;
    // Limited but useful subset. Keys are matched case-insensitively by the
    // caller's `to_ascii_lowercase`. Extend as users request more bindings.
    Some(match name {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "escape" | "esc" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "delete" | "del" => Code::Delete,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Clipboard
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CopyArgs {
    pub text: String,
}

/// Copy text to the system clipboard.
#[tauri::command]
pub fn copy_to_clipboard<R: Runtime>(app: AppHandle<R>, args: CopyArgs) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(args.text)
        .map_err(|e| e.to_string())
}

/// Read text from the system clipboard.
#[tauri::command]
pub fn read_clipboard<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| e.to_string())
}

fn sync_autostart<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let current = manager.is_enabled().map_err(|e| e.to_string())?;
    match (enabled, current) {
        (true, false) => manager.enable().map_err(|e| e.to_string()),
        (false, true) => manager.disable().map_err(|e| e.to_string()),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Hotkey handler (called by the global-shortcut plugin)
// ---------------------------------------------------------------------------

/// Called by the global-shortcut plugin when the registered shortcut is pressed.
///
/// Pipeline:
/// 1. Read the source selection before the main window can take focus.
/// 2. If configured and the selection is empty, read clipboard text once.
/// 3. Show the main window and emit the captured source payload.
pub async fn on_hotkey<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<Arc<crate::state::AppState>>();
    let monitor = state.selection_monitor().await;
    let cfg = Config::load().unwrap_or_default();

    let result = monitor.get_selected_text().await;

    // macOS TCC quirk: after an app update the binary changes, and the
    // Accessibility grant may go stale even though the entry is still
    // checked in System Settings. When we see PermissionDenied, force a
    // re-evaluation via the prompt option and retry once.
    #[cfg(target_os = "macos")]
    let result = if matches!(result, Err(SelectionError::PermissionDenied)) {
        translator_platform::request_accessibility_permission();
        monitor.get_selected_text().await
    } else {
        result
    };

    let mut payload = match result {
        Ok(Some(text)) if !text.trim().is_empty() => HotkeyPayload {
            text: Some(text),
            error: None,
            source: "selection".to_string(),
        },
        Ok(_) => HotkeyPayload {
            text: None,
            error: None,
            source: "none".to_string(),
        },
        Err(SelectionError::Empty) => HotkeyPayload {
            text: None,
            error: None,
            source: "none".to_string(),
        },
        Err(error) => HotkeyPayload {
            text: None,
            error: Some(selection_error_payload(&error)),
            source: "selection".to_string(),
        },
    };

    if payload.text.is_none()
        && payload.error.is_none()
        && cfg.general.auto_translate_clipboard_on_hotkey
    {
        use tauri_plugin_clipboard_manager::ClipboardExt;

        match app.clipboard().read_text() {
            Ok(text) if !text.trim().is_empty() => {
                payload.text = Some(text);
                payload.source = "clipboard".to_string();
            }
            Ok(_) => {}
            Err(error) => {
                payload.error = Some(format!("clipboard:{error}"));
                payload.source = "clipboard".to_string();
            }
        }
    }

    if let Err(error) = show_main_window(app, None) {
        tracing::warn!(error = %error, "main window not registered; hotkey is a no-op");
    }

    let _ = app.emit("translator://hotkey-source", payload);
}

// ---------------------------------------------------------------------------
// Error trait helper
// ---------------------------------------------------------------------------

trait SelectionErrorExt {
    fn code(&self) -> &'static str;
}

impl SelectionErrorExt for SelectionError {
    fn code(&self) -> &'static str {
        match self {
            SelectionError::PermissionDenied => "permission_denied",
            SelectionError::Platform(_) => "platform",
            SelectionError::Timeout => "timeout",
            SelectionError::Empty => "empty",
        }
    }
}

fn selection_error_payload(error: &SelectionError) -> String {
    match error {
        SelectionError::PermissionDenied => permission_denied_payload(),
        SelectionError::Empty => "empty".to_string(),
        _ => format!("{}:{}", error.code(), error),
    }
}

fn permission_denied_payload() -> String {
    #[cfg(target_os = "macos")]
    {
        if macos_current_app_is_cdhash_only_signed() {
            return "macos_unstable_signature".to_string();
        }
    }

    "permission_denied".to_string()
}

#[cfg(target_os = "macos")]
fn macos_current_app_is_cdhash_only_signed() -> bool {
    let Some(path) = macos_current_app_or_exe_path() else {
        return false;
    };
    let output = match Command::new("/usr/bin/codesign")
        .arg("-dr")
        .arg("-")
        .arg(&path)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!(error = %error, "could not inspect macOS code signature");
            return false;
        }
    };

    let mut requirement = String::from_utf8_lossy(&output.stderr).to_string();
    requirement.push_str(&String::from_utf8_lossy(&output.stdout));
    let cdhash_only = macos_designated_requirement_is_cdhash_only(&requirement);
    if cdhash_only {
        tracing::warn!(
            path = %path.display(),
            requirement = %requirement.trim(),
            "macOS accessibility was denied for a cdhash-only signed app; the TCC grant may be stale after an update"
        );
    }
    cdhash_only
}

#[cfg(target_os = "macos")]
fn macos_current_app_or_exe_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    for ancestor in exe.ancestors() {
        if ancestor.extension().and_then(|ext| ext.to_str()) == Some("app") {
            return Some(ancestor.to_path_buf());
        }
    }
    Some(exe)
}

#[cfg(any(target_os = "macos", test))]
fn macos_designated_requirement_is_cdhash_only(requirement: &str) -> bool {
    let normalized = requirement.trim();
    normalized.contains("cdhash H\"") && !normalized.contains("identifier \"")
}
