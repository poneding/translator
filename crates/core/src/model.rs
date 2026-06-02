//! Shared data models used across the workspace.

use serde::{Deserialize, Serialize};

/// Unique identifier for a translation service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceId {
    /// Youdao (有道) translation
    Youdao,
    /// DeepL
    DeepL,
    /// Google Cloud Translation
    Google,
    /// Microsoft Bing / Azure Translator
    Bing,
    /// OpenAI-compatible chat completions API
    OpenAI,
}

impl ServiceId {
    /// Stable string id (used in config and persisted state).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Youdao => "youdao",
            Self::DeepL => "deepl",
            Self::Google => "google",
            Self::Bing => "bing",
            Self::OpenAI => "openai",
        }
    }

    /// Human-readable display name (English).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Youdao => "Youdao",
            Self::DeepL => "DeepL",
            Self::Google => "Google Translate",
            Self::Bing => "Microsoft Translator",
            Self::OpenAI => "OpenAI Compatible",
        }
    }

    /// All known service ids in canonical order.
    pub fn all() -> &'static [ServiceId] {
        &[
            Self::Youdao,
            Self::DeepL,
            Self::Google,
            Self::Bing,
            Self::OpenAI,
        ]
    }
}

/// A request to translate a piece of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
    /// Source text. Caller is responsible for trimming.
    pub text: String,
    /// Source language code (e.g. `"en"`, `"zh-Hans"`) or `None` for auto-detect.
    pub from: Option<String>,
    /// Target language code (e.g. `"zh-Hans"`, `"en"`). Always required.
    pub to: String,
}

impl TranslateRequest {
    /// Convenience constructor for auto-detect source.
    pub fn auto(text: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from: None,
            to: to.into(),
        }
    }
}

/// The outcome of a single service translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    /// Which service produced this result.
    pub service_id: ServiceId,
    /// Human-readable service name (resolved at call time).
    pub service_name: String,
    /// Translated text.
    pub text: String,
    /// Language code detected by the service, if any.
    pub detected_source: Option<String>,
    /// Wall-clock time spent on the request.
    pub elapsed_ms: u64,
    /// Optional extras (e.g. Youdao dictionary entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}
