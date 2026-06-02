//! Persistent configuration (JSON file in user config dir).
//!
//! Sensitive credentials (API keys) are stored in the OS Keychain via
//! [`crate::secrets`], NOT in this JSON file.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::ServiceId;
use crate::service::ServiceConfig;

/// Top-level user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Schema version, currently `1`.
    pub version: u32,

    /// General user preferences.
    pub general: GeneralConfig,

    /// Global hotkey in tauri-plugin-global-shortcut format
    /// (e.g. `"CmdOrCtrl+Shift+D"`).
    pub shortcut: String,

    /// Per-service config, keyed by `ServiceId::as_str()`.
    pub services: std::collections::BTreeMap<String, ServiceConfig>,

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
    pub target_language: String,
    /// Default source language; `"auto"` = let services detect.
    pub default_from: String,
    /// `light` / `dark` / `system`.
    pub theme: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            target_language: "zh-Hans".to_string(),
            default_from: "auto".to_string(),
            theme: "system".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut services = std::collections::BTreeMap::new();
        for (i, id) in ServiceId::all().iter().enumerate() {
            let cfg = ServiceConfig {
                id: *id,
                enabled: matches!(id, ServiceId::DeepL | ServiceId::Youdao | ServiceId::OpenAI),
                priority: i as u8,
                options: serde_json::Value::Null,
            };
            services.insert(id.as_str().to_string(), cfg);
        }
        Self {
            version: 1,
            general: GeneralConfig::default(),
            shortcut: "CmdOrCtrl+Shift+D".to_string(),
            services,
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
        let cfg: Self = serde_json::from_str(&text)
            .with_context(|| format!("parse config: {}", path.display()))?;
        Ok(cfg)
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
}
