//! Translation service trait + per-service config schema.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::ServiceResult;
use crate::model::{ServiceId, TranslateRequest, TranslateResult};

/// How a service expects to authenticate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyRequirement {
    /// No auth required.
    None,
    /// Auth optional (service works either way, but some features need it).
    Optional,
    /// Auth required for any use.
    Required,
}

/// Per-service user configuration (persisted to disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Which service this config is for.
    pub id: ServiceId,
    /// Whether this service is enabled and should be called.
    pub enabled: bool,
    /// Lower = shown first; ties broken by elapsed_ms.
    pub priority: u8,
    /// Per-service extra fields (e.g. `base_url`, `model` for OpenAI-compatible).
    #[serde(default)]
    pub options: serde_json::Value,
}

impl ServiceConfig {
    /// Default config (enabled, priority 100, no options).
    pub fn default_for(id: ServiceId) -> Self {
        Self {
            id,
            enabled: true,
            priority: 100,
            options: serde_json::Value::Null,
        }
    }
}

/// The contract every translation service implements.
#[async_trait]
pub trait TranslationService: Send + Sync {
    /// Stable service id.
    fn id(&self) -> ServiceId;

    /// Human-readable name (English).
    fn display_name(&self) -> &'static str;

    /// How this service expects credentials.
    fn api_key_requirement(&self) -> ApiKeyRequirement;

    /// JSON Schema describing per-service options (for dynamic form generation).
    /// Return an empty object `{}` if there are no extra options.
    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    /// Translate text and return the result.
    async fn translate(
        &self,
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        api_key: Option<&str>,
        client: &reqwest::Client,
    ) -> ServiceResult<TranslateResult>;
}
