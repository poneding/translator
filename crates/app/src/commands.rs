//! IPC commands exposed to the React frontend via `invoke()`.

use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, Position,
    Runtime, Size, State, WebviewWindow,
};
use tauri_plugin_updater::UpdaterExt;

use translator_core::{Config, ServiceError, ServiceId, TranslateResult};
use translator_platform::SelectionError;

use crate::state::AppState;

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

pub(crate) fn show_main_window<R: Runtime>(
    app: &AppHandle<R>,
    event: Option<&str>,
) -> Result<(), String> {
    let cfg = Config::load().unwrap_or_default();
    let win: WebviewWindow<R> = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    prepare_main_window(app, &win, &cfg)?;
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
    win.set_always_on_top(cfg.window.always_on_top)
        .map_err(|e| e.to_string())?;
    let _ = win.set_maximizable(false);
    let _ = win.set_max_size(Some(Size::Logical(LogicalSize::new(
        MAIN_WINDOW_MAX_WIDTH,
        MAIN_WINDOW_MAX_HEIGHT,
    ))));
    position_main_window(app, win, &cfg.window.display_position)
}

fn position_main_window<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    display_position: &str,
) -> Result<(), String> {
    let cursor = app.cursor_position().ok();
    let monitor = target_monitor(app, win, cursor)?;
    let Some(monitor) = monitor else {
        return Ok(());
    };
    let size = win.outer_size().unwrap_or_else(|_| {
        PhysicalSize::new(
            MAIN_WINDOW_DEFAULT_WIDTH as u32,
            MAIN_WINDOW_DEFAULT_HEIGHT as u32,
        )
    });
    let work = monitor.work_area();
    let left = work.position.x;
    let top = work.position.y;
    let right = left + work.size.width as i32;
    let bottom = top + work.size.height as i32;
    let width = size.width as i32;
    let height = size.height as i32;

    let (raw_x, raw_y) = match display_position {
        "center" => (
            left + (work.size.width as i32 - width) / 2,
            top + (work.size.height as i32 - height) / 2,
        ),
        "mouse" => {
            if let Some(cursor) = cursor {
                (cursor.x.round() as i32 + 16, cursor.y.round() as i32 + 16)
            } else {
                (right - width - WINDOW_EDGE_MARGIN, top + WINDOW_EDGE_MARGIN)
            }
        }
        _ => (right - width - WINDOW_EDGE_MARGIN, top + WINDOW_EDGE_MARGIN),
    };

    let x = clamp_window_axis(raw_x, left, right, width);
    let y = clamp_window_axis(raw_y, top, bottom, height);
    win.set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|e| e.to_string())
}

fn target_monitor<R: Runtime>(
    app: &AppHandle<R>,
    win: &WebviewWindow<R>,
    cursor: Option<PhysicalPosition<f64>>,
) -> Result<Option<Monitor>, String> {
    if let Some(cursor) = cursor {
        if let Some(monitor) = app
            .monitor_from_point(cursor.x, cursor.y)
            .map_err(|e| e.to_string())?
        {
            return Ok(Some(monitor));
        }
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
    if old.general.launch_at_startup != config.general.launch_at_startup {
        sync_autostart(&app, config.general.launch_at_startup)?;
    }
    let config = config.normalized();
    config.save().map_err(|e| e.to_string())?;
    apply_main_window_always_on_top(&app, config.window.always_on_top)?;
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
    if status.status == "failed" {
        if let Some(error) = &status.error {
            tracing::warn!(error = %error, "startup update check failed");
        }
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
    use serde_json::json;

    use super::{DeleteApiKeyArgs, HasApiKeyArgs, SetApiKeyArgs};

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

    let mut payload = match monitor.get_selected_text().await {
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
        SelectionError::PermissionDenied => "permission_denied".to_string(),
        SelectionError::Empty => "empty".to_string(),
        _ => format!("{}:{}", error.code(), error),
    }
}
