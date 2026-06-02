//! IPC commands exposed to the React frontend via `invoke()`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Runtime, State, WebviewWindow};

use translator_core::{Config, ServiceError, TranslateRequest, TranslateResult};
use translator_platform::{Rect, SelectionError};

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
    pub to: String,
}

/// Translate the text via all enabled services, returning one result per service.
#[tauri::command]
pub async fn translate_text(
    args: TranslateArgs,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ServiceOutcomeDto>, String> {
    let cfg = Config::load().map_err(|e| e.to_string())?;
    let req = TranslateRequest {
        text: args.text,
        from: args.from,
        to: args.to,
    };
    let outcomes = state.translator.translate_all(&req, &cfg).await;
    Ok(outcomes.into_iter().map(ServiceOutcomeDto::from).collect())
}

#[derive(Debug, Serialize)]
pub struct ServiceOutcomeDto {
    pub service_id: String,
    pub service_name: String,
    pub result: Option<TranslateResult>,
    pub error: Option<ServiceErrorDto>,
}

impl From<translator_core::translator::TranslateOutcome> for ServiceOutcomeDto {
    fn from(o: translator_core::translator::TranslateOutcome) -> Self {
        match o.result {
            Ok(r) => Self {
                service_id: o.service_id.as_str().to_string(),
                service_name: o.service_name,
                result: Some(r),
                error: None,
            },
            Err(e) => Self {
                service_id: o.service_id.as_str().to_string(),
                service_name: o.service_name,
                result: None,
                error: Some(ServiceErrorDto::from(&e)),
            },
        }
    }
}

#[derive(Debug, Serialize)]
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

/// Show the floating popup at the given position.
#[tauri::command]
pub fn show_popup<R: Runtime>(app: AppHandle<R>, args: ShowPopupArgs) -> Result<(), String> {
    let win: WebviewWindow<R> = app
        .get_webview_window("popup")
        .ok_or_else(|| "popup window not found".to_string())?;
    win.set_size(tauri::LogicalSize::new(
        args.width as f64,
        args.height as f64,
    ))
    .map_err(|e| e.to_string())?;
    win.set_position(PhysicalPosition::new(args.x, args.y))
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

/// Return the current config (sanitized: never includes API keys).
#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    Config::load().map_err(|e| e.to_string())
}

/// Save the provided config to disk.
#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
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

/// Parse a tauri-plugin-global-shortcut string like "CmdOrCtrl+Shift+D".
fn parse_shortcut(s: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    let mut mods = Modifiers::empty();
    let mut key: Option<Code> = None;
    for part in s.split('+') {
        let p = part.trim();
        if p.is_empty() {
            return Err(format!("empty modifier in shortcut: {s:?}"));
        }
        match p.to_ascii_lowercase().as_str() {
            "cmdorctrl" | "cmd" | "ctrl" | "super" => mods |= Modifiers::SUPER,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "meta" | "win" => mods |= Modifiers::META,
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

// ---------------------------------------------------------------------------
// Hotkey handler (called by the global-shortcut plugin)
// ---------------------------------------------------------------------------

/// Called by the global-shortcut plugin when the registered shortcut is pressed.
///
/// Pipeline:
/// 1. Show the popup window at a sensible position (selection → cursor → centered).
/// 2. Emit a `translator://hotkey-pressed` event so the React popup re-runs
///    its translation pipeline (the popup listens for this on mount and
///    re-listens thereafter, so back-to-back hotkey presses always pick up
///    the latest selection).
pub async fn on_hotkey<R: Runtime>(app: &AppHandle<R>) {
    use tauri::{LogicalSize, PhysicalPosition};
    use translator_platform::Rect;

    // Lazily build / fetch the platform selection monitor.
    let state = app.state::<Arc<crate::state::AppState>>();
    let monitor = state.selection_monitor().await;

    // Pick a sensible screen to clamp to. For v1 we use the primary monitor
    // (full virtual screen). Tauri's window APIs let us query the primary
    // monitor size at runtime; for now we use a conservative "huge" screen so
    // clamping is a no-op on standard desktops.
    let screen = Rect {
        x: 0,
        y: 0,
        width: i32::MAX,
        height: i32::MAX,
    };

    let (x, y) = crate::popup_position::compute_popup_position(monitor.as_ref(), screen).await;

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

    let _ = app.emit("translator://hotkey-pressed", ());
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

// `Rect` re-export so the frontend can use it via the generated types.
#[allow(unused_imports)]
use Rect as _Rect;
