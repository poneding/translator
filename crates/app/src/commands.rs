//! IPC commands exposed to the React frontend via `invoke()`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State, WebviewWindow};

use translator_core::{Config, ServiceError, TranslateResult};
use translator_platform::{position, Rect, SelectionError};

use crate::state::AppState;

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

// ---------------------------------------------------------------------------
// Popup window
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ShowPopupArgs {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct HotkeyPayload {
    pub text: Option<String>,
    pub error: Option<String>,
}

/// Show the floating popup at the given position.
#[tauri::command]
pub fn show_popup<R: Runtime>(app: AppHandle<R>, args: ShowPopupArgs) -> Result<(), String> {
    let win: WebviewWindow<R> = app
        .get_webview_window("popup")
        .ok_or_else(|| "popup window not found".to_string())?;
    let screen = monitor_rects(&app)
        .into_iter()
        .next()
        .unwrap_or_else(fallback_screen);
    let (x, y) = position::clamp_to_screen(
        args.x,
        args.y,
        args.width as i32,
        args.height as i32,
        screen,
    );
    win.set_size(tauri::LogicalSize::new(
        args.width as f64,
        args.height as f64,
    ))
    .map_err(|e| e.to_string())?;
    win.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Hide the floating popup.
#[tauri::command]
pub fn hide_popup<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("popup") {
        let _ = win.hide();
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings window + config
// ---------------------------------------------------------------------------

/// Open the settings window.
#[tauri::command]
pub fn open_settings<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let win: WebviewWindow<R> = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window not found".to_string())?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Open the main translation window.
#[tauri::command]
pub fn open_main_window<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let win: WebviewWindow<R> = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
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
        repo_url: "https://github.com/your-org/translator".to_string(),
    }
}

#[derive(Debug, Deserialize)]
pub struct SetApiKeyArgs {
    pub service_id: String,
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
    pub service_id: String,
}

/// Remove an API key from the OS Keychain.
#[tauri::command]
pub fn delete_api_key(args: DeleteApiKeyArgs) -> Result<(), String> {
    translator_core::secrets::delete_api_key(&args.service_id).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct HasApiKeyArgs {
    pub service_id: String,
}

/// BH-8.1: cheap boolean probe so the settings row can show a status indicator
/// without trying to read the secret value.
#[tauri::command]
pub fn has_api_key(args: HasApiKeyArgs) -> Result<bool, String> {
    translator_core::secrets::has_api_key(&args.service_id).map_err(|e| e.to_string())
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
/// 1. Read the source selection before the popup can take focus.
/// 2. Show the popup window at a sensible position (selection → cursor → centered).
/// 3. Emit a `translator://hotkey-pressed` event with the captured text so the
///    React popup can translate without re-reading focus-dependent selection.
pub async fn on_hotkey<R: Runtime>(app: &AppHandle<R>) {
    use tauri::{LogicalSize, PhysicalPosition};

    // Lazily build / fetch the platform selection monitor.
    let state = app.state::<Arc<crate::state::AppState>>();
    let monitor = state.selection_monitor().await;

    let payload = match monitor.get_selected_text().await {
        Ok(Some(text)) if !text.trim().is_empty() => HotkeyPayload {
            text: Some(text),
            error: None,
        },
        Ok(_) => HotkeyPayload {
            text: None,
            error: Some("empty".to_string()),
        },
        Err(error) => HotkeyPayload {
            text: None,
            error: Some(selection_error_payload(&error)),
        },
    };

    let screens = monitor_rects(app);
    let (x, y) = crate::popup_position::compute_popup_position(monitor.as_ref(), &screens).await;

    if let Some(win) = app.get_webview_window("popup") {
        let _ = win.set_size(LogicalSize::new(
            crate::popup_position::DEFAULT_POPUP_W as f64,
            crate::popup_position::DEFAULT_POPUP_H as f64,
        ));
        let _ = win.set_position(PhysicalPosition::new(x, y));
        let _ = win.show();
        let _ = win.set_focus();
    } else {
        tracing::warn!("popup window not registered; hotkey is a no-op");
    }

    let _ = app.emit("translator://hotkey-pressed", payload);
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

fn monitor_rects<R: Runtime>(app: &AppHandle<R>) -> Vec<Rect> {
    app.available_monitors()
        .ok()
        .map(|monitors| {
            monitors
                .into_iter()
                .map(|monitor| {
                    let pos = monitor.position();
                    let size = monitor.size();
                    Rect {
                        x: pos.x,
                        y: pos.y,
                        width: size.width as i32,
                        height: size.height as i32,
                    }
                })
                .filter(|rect| rect.width > 0 && rect.height > 0)
                .collect()
        })
        .filter(|rects: &Vec<Rect>| !rects.is_empty())
        .unwrap_or_else(|| vec![fallback_screen()])
}

fn fallback_screen() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    }
}

// `Rect` re-export so the frontend can use it via the generated types.
#[allow(unused_imports)]
use Rect as _Rect;
