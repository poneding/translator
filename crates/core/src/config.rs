//! Persistent configuration (JSON file in user config dir).
//!
//! Sensitive credentials (API keys) are stored in the OS Keychain via
//! [`crate::secrets`], NOT in this JSON file.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::language_direction::normalized_preferred_languages;
use crate::model::ServiceId;
use crate::service::ServiceConfig;

const CURRENT_CONFIG_VERSION: u32 = 3;

/// Top-level user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Schema version.
    #[serde(default = "current_config_version")]
    pub version: u32,

    /// General user preferences.
    #[serde(default)]
    pub general: GeneralConfig,

    /// Global hotkey in tauri-plugin-global-shortcut format
    /// (e.g. `"Cmd+T"` on macOS or `"Alt+T"` elsewhere).
    #[serde(default = "default_shortcut")]
    pub shortcut: String,

    /// Per-service config, keyed by `ServiceId::as_str()`.
    #[serde(default = "default_services")]
    pub services: std::collections::BTreeMap<String, ServiceConfig>,

    /// Recent successful translations, newest first.
    #[serde(default)]
    pub history: Vec<HistoryItem>,

    /// Main-window preferences.
    #[serde(default)]
    pub window: WindowConfig,

    /// Update-check preferences.
    #[serde(default)]
    pub updates: UpdateConfig,

    /// BH-1.5: set when the OS denied the last hotkey registration (e.g. due
    /// to a conflict with another app). Reset to `false` on next launch
    /// after the app falls back to the default shortcut. The settings UI
    /// reads this to show a red banner.
    #[serde(default)]
    pub hotkey_registration_failed: bool,
}

/// General preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Target language code (e.g. `"zh-Hans"`, `"en"`).
    ///
    /// Kept for compatibility with older config files. New code should prefer
    /// `preferred_languages` when deriving the translation direction.
    #[serde(default = "default_target_language")]
    pub target_language: String,
    /// Default source language; `"auto"` = let services detect.
    ///
    /// Kept for compatibility with older config files and manual callers.
    #[serde(default = "default_source_language")]
    pub default_from: String,
    /// Ordered language preferences used to derive translation direction.
    ///
    /// When the source language matches one entry, translation targets the
    /// first different entry. Otherwise the first entry is used as target.
    #[serde(default)]
    pub preferred_languages: Vec<String>,
    /// `light` / `dark` / `system`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// UI language: `system` or one of the supported app locale ids.
    #[serde(default = "default_app_language")]
    pub app_language: String,
    /// Copy the first successful translation to the system clipboard.
    #[serde(default)]
    pub auto_copy: bool,
    /// When the global hotkey has no selected text, read the clipboard and
    /// translate it if it contains text.
    #[serde(default)]
    pub auto_translate_clipboard_on_hotkey: bool,
    /// Register the app to launch when the user signs in.
    #[serde(default)]
    pub launch_at_startup: bool,
    /// Optional HTTP proxy used by translation service requests.
    #[serde(default)]
    pub proxy: ProxyConfig,
}

/// Main-window preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Keep the main window above other windows.
    #[serde(default)]
    pub always_on_top: bool,
    /// Where the main window appears when opened.
    #[serde(default = "default_window_display_position")]
    pub display_position: String,
    /// Last remembered outer window position in physical screen coordinates.
    #[serde(default)]
    pub last_position: Option<WindowPosition>,
}

/// Physical screen position for the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPosition {
    /// X coordinate in physical pixels.
    pub x: i32,
    /// Y coordinate in physical pixels.
    pub y: i32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            always_on_top: false,
            display_position: default_window_display_position(),
            last_position: None,
        }
    }
}

/// Update-check preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Check for updates asynchronously after app startup.
    #[serde(default = "default_check_updates_on_startup")]
    pub check_on_startup: bool,
    /// Include prerelease/beta versions in update checks.
    #[serde(default)]
    pub allow_beta: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_on_startup: default_check_updates_on_startup(),
            allow_beta: false,
        }
    }
}

/// HTTP proxy preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether proxy routing is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Proxy URL, for example `http://127.0.0.1:7890`.
    #[serde(default)]
    pub url: String,
}

/// One successful translation stored for later recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    /// Stable id derived from the insertion timestamp.
    pub id: String,
    /// Source text sent to providers.
    pub source_text: String,
    /// First successful translated text.
    pub translated_text: String,
    /// Service id that produced `translated_text`.
    pub service_id: String,
    /// Service display name at translation time.
    pub service_name: String,
    /// Source language from the request, or `auto`.
    pub from: String,
    /// Target language from the request.
    pub to: String,
    /// Unix timestamp in milliseconds.
    pub created_at_ms: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            target_language: default_target_language(),
            default_from: default_source_language(),
            preferred_languages: default_preferred_languages(),
            theme: default_theme(),
            app_language: default_app_language(),
            auto_copy: false,
            auto_translate_clipboard_on_hotkey: false,
            launch_at_startup: false,
            proxy: ProxyConfig::default(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            general: GeneralConfig::default(),
            shortcut: default_shortcut(),
            services: default_services(),
            history: Vec::new(),
            window: WindowConfig::default(),
            updates: UpdateConfig::default(),
            hotkey_registration_failed: false,
        }
    }
}

impl Config {
    /// Path to the config file for the current platform.
    pub fn config_path() -> Result<PathBuf> {
        let base = dirs::config_dir().context("no config dir on this platform")?;
        Ok(base.join("translator").join("config.json"))
    }

    /// Load config from disk, falling back to defaults on missing/corrupt file.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read config: {}", path.display()))?;
        let cfg: Self = serde_json::from_str(&text).unwrap_or_default();
        Ok(cfg.normalized())
    }

    /// Save config to disk. Creates parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create config dir: {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serialize config")?;
        std::fs::write(&path, text).with_context(|| format!("write config: {}", path.display()))?;
        Ok(())
    }

    /// Add a successful translation to the bounded history list.
    pub fn record_history(
        &mut self,
        source_text: String,
        translated_text: String,
        service_id: String,
        service_name: String,
        from: String,
        to: String,
    ) {
        let created_at_ms = now_ms();
        self.history.insert(
            0,
            HistoryItem {
                id: format!("history-{created_at_ms}"),
                source_text,
                translated_text,
                service_id,
                service_name,
                from,
                to,
                created_at_ms,
            },
        );
        self.history.truncate(50);
    }

    /// Ensure loaded configs from older versions have every required field.
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_CONFIG_VERSION;
        self.general.preferred_languages = normalized_preferred_languages(
            &self.general.preferred_languages,
            &self.general.target_language,
            &self.general.default_from,
        );
        if let Some(first) = self.general.preferred_languages.first() {
            self.general.target_language = first.clone();
        }
        if self
            .shortcut
            .trim()
            .eq_ignore_ascii_case("CmdOrCtrl+Shift+D")
        {
            self.shortcut = default_shortcut();
        }
        self.general.theme = normalized_choice(
            &self.general.theme,
            &["system", "light", "dark"],
            &default_theme(),
        );
        self.general.app_language = normalized_app_language(&self.general.app_language);
        self.window.display_position =
            normalized_window_display_position(&self.window.display_position);

        let defaults = default_services();
        for (key, default_cfg) in defaults {
            self.services.entry(key).or_insert(default_cfg);
        }
        for cfg in self.services.values_mut() {
            if !cfg.options.is_object() {
                cfg.options = serde_json::json!({});
            }
        }
        self.history.truncate(50);
        self
    }
}

fn current_config_version() -> u32 {
    CURRENT_CONFIG_VERSION
}

fn default_shortcut() -> String {
    platform_default_shortcut().to_string()
}

#[cfg(target_os = "macos")]
fn platform_default_shortcut() -> &'static str {
    "Cmd+T"
}

#[cfg(not(target_os = "macos"))]
fn platform_default_shortcut() -> &'static str {
    "Alt+T"
}

fn default_target_language() -> String {
    "zh-Hans".to_string()
}

fn default_source_language() -> String {
    "auto".to_string()
}

fn default_preferred_languages() -> Vec<String> {
    vec![default_target_language(), "en".to_string()]
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_app_language() -> String {
    "system".to_string()
}

fn default_window_display_position() -> String {
    "remember".to_string()
}

fn default_check_updates_on_startup() -> bool {
    true
}

fn normalized_choice(value: &str, allowed: &[&str], fallback: &str) -> String {
    let trimmed = value.trim();
    allowed
        .iter()
        .find(|item| item.eq_ignore_ascii_case(trimmed))
        .copied()
        .unwrap_or(fallback)
        .to_string()
}

fn normalized_app_language(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("system") {
        return default_app_language();
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "en" => "en".to_string(),
        "zh-hans" | "zh-cn" => "zh-Hans".to_string(),
        "zh-hant" | "zh-tw" | "zh-hk" | "zh-mo" => "zh-Hant".to_string(),
        "ja" | "ko" | "fr" | "de" | "es" | "ru" | "pt" | "it" | "ar" => {
            trimmed.to_ascii_lowercase()
        }
        _ => default_app_language(),
    }
}

fn normalized_window_display_position(value: &str) -> String {
    normalized_choice(
        value,
        &["remember", "right", "center", "mouse"],
        &default_window_display_position(),
    )
}

fn default_services() -> std::collections::BTreeMap<String, ServiceConfig> {
    let mut services = std::collections::BTreeMap::new();
    for (priority, id) in ServiceId::all().iter().enumerate() {
        let cfg = ServiceConfig {
            id: *id,
            enabled: matches!(id, ServiceId::Youdao | ServiceId::Google),
            priority: priority as u8,
            options: serde_json::json!({}),
        };
        services.insert(id.as_str().to_string(), cfg);
    }
    services
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_migrates_legacy_default_shortcut() {
        let config = Config {
            shortcut: " CmdOrCtrl+Shift+D ".to_string(),
            ..Default::default()
        };

        let normalized = config.normalized();

        assert_eq!(normalized.shortcut, default_shortcut());
    }

    #[test]
    fn normalized_keeps_custom_shortcut() {
        let config = Config {
            shortcut: "Ctrl+Alt+K".to_string(),
            ..Default::default()
        };

        let normalized = config.normalized();

        assert_eq!(normalized.shortcut, "Ctrl+Alt+K");
    }

    #[test]
    fn normalized_migrates_v1_config_defaults_without_losing_data() {
        let json = serde_json::json!({
            "version": 1,
            "general": {
                "target_language": "ja",
                "default_from": "auto",
                "preferred_languages": ["ja", "en"],
                "theme": "dark",
                "auto_copy": true,
                "launch_at_startup": true,
                "proxy": {
                    "enabled": true,
                    "url": "http://127.0.0.1:7890"
                }
            },
            "shortcut": "Ctrl+Alt+K",
            "services": {
                "youdao": {
                    "id": "youdao",
                    "enabled": true,
                    "priority": 0,
                    "options": {}
                }
            },
            "history": [{
                "id": "history-1",
                "source_text": "hello",
                "translated_text": "你好",
                "service_id": "youdao",
                "service_name": "Youdao",
                "from": "en",
                "to": "zh-Hans",
                "created_at_ms": 1
            }]
        });

        let config: Config = serde_json::from_value(json).expect("legacy config");
        let normalized = config.normalized();

        assert_eq!(normalized.version, CURRENT_CONFIG_VERSION);
        assert_eq!(normalized.general.theme, "dark");
        assert_eq!(normalized.general.app_language, "system");
        assert!(normalized.general.auto_copy);
        assert!(!normalized.general.auto_translate_clipboard_on_hotkey);
        assert!(normalized.general.launch_at_startup);
        assert!(normalized.general.proxy.enabled);
        assert_eq!(normalized.shortcut, "Ctrl+Alt+K");
        assert_eq!(normalized.history.len(), 1);
        assert!(!normalized.window.always_on_top);
        assert_eq!(normalized.window.display_position, "remember");
        assert!(normalized.updates.check_on_startup);
        assert!(!normalized.updates.allow_beta);
        for service_id in ServiceId::all() {
            assert!(normalized.services.contains_key(service_id.as_str()));
        }
    }

    #[test]
    fn normalized_sanitizes_theme_and_app_language() {
        let config = Config {
            general: GeneralConfig {
                theme: "purple".to_string(),
                app_language: "zh-cn".to_string(),
                ..GeneralConfig::default()
            },
            window: WindowConfig {
                display_position: "floating".to_string(),
                ..WindowConfig::default()
            },
            ..Config::default()
        };

        let normalized = config.normalized();

        assert_eq!(normalized.general.theme, "system");
        assert_eq!(normalized.general.app_language, "zh-Hans");
        assert_eq!(normalized.window.display_position, "remember");
    }

    #[test]
    fn window_defaults_to_remember_last_position() {
        let window = WindowConfig::default();

        assert_eq!(window.display_position, "remember");
    }
}
