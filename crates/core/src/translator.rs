//! High-level translator that fans out a request to multiple enabled services
//! in parallel and aggregates the results.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use futures::future::join_all;
use tokio::time::timeout;

use crate::config::Config;
use crate::error::ServiceError;
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::secrets;
use crate::service::TranslationService;

/// Coordinator that owns the enabled service implementations and a shared HTTP client.
pub struct Translator {
    services: Vec<Arc<dyn TranslationService>>,
    client: reqwest::Client,
}

impl Translator {
    /// Construct a translator with all known services.
    pub fn new() -> Self {
        let services: Vec<Arc<dyn TranslationService>> = vec![
            Arc::new(super::services::youdao::YoudaoService),
            Arc::new(super::services::deepl::DeepLService),
            Arc::new(super::services::google::GoogleService),
            Arc::new(super::services::bing::BingService),
            Arc::new(super::services::openai::OpenAIService),
        ];
        let client = reqwest::Client::builder()
            .user_agent(concat!("translator/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("build reqwest client");
        Self { services, client }
    }

    /// Get all registered services.
    pub fn services(&self) -> &[Arc<dyn TranslationService>] {
        &self.services
    }

    /// Translate the given request using all enabled services in the config, in parallel.
    ///
    /// Failures of individual services are returned alongside successful results
    /// (so one provider's outage doesn't kill the rest).
    pub async fn translate_all(
        &self,
        req: &TranslateRequest,
        cfg: &Config,
    ) -> Vec<TranslateOutcome> {
        let mut enabled: Vec<_> = cfg
            .services
            .values()
            .filter(|sc| sc.enabled)
            .cloned()
            .collect();
        // BH-4.5: results are returned in priority order (lower number first).
        // `join_all` preserves spawn order, so sorting the input is enough.
        enabled.sort_by_key(|sc| sc.priority);

        // BH-4.3: silently skip services that have no usable credentials
        // (either the service doesn't require a key, or the key is set in the
        // OS keychain). We do this synchronously before spawning any futures so
        // the popup never shows a tab for a service we know won't run.
        let skippable: Vec<(ServiceId, &'static str)> = enabled
            .iter()
            .filter_map(|sc| {
                let svc = self.find_service(sc.id)?;
                use crate::service::ApiKeyRequirement;
                match svc.api_key_requirement() {
                    ApiKeyRequirement::None | ApiKeyRequirement::Optional => None,
                    ApiKeyRequirement::Required => match secrets::has_api_key(sc.id.as_str()) {
                        Ok(true) => None,
                        Ok(false) => Some((sc.id, "no key configured")),
                        Err(_) => Some((sc.id, "keyring error")),
                    },
                }
            })
            .collect();
        let skip_ids: std::collections::HashSet<ServiceId> =
            skippable.iter().map(|(id, _)| *id).collect();
        enabled.retain(|sc| !skip_ids.contains(&sc.id));

        let futures = enabled.into_iter().map(|sc| {
            let client = self.client.clone();
            let req = req.clone();
            async move {
                let svc = match self.find_service(sc.id) {
                    Some(s) => s,
                    None => {
                        return TranslateOutcome {
                            service_id: sc.id,
                            service_name: sc.id.display_name().to_string(),
                            result: Err(ServiceError::MissingCredentials(format!(
                                "service impl not found for {}",
                                sc.id.as_str()
                            ))),
                        };
                    }
                };
                let api_key = match secrets::get_api_key(sc.id.as_str()) {
                    Ok(k) => k,
                    Err(e) => {
                        return TranslateOutcome {
                            service_id: sc.id,
                            service_name: svc.display_name().to_string(),
                            result: Err(ServiceError::Api {
                                code: "keyring".to_string(),
                                message: e.to_string(),
                            }),
                        };
                    }
                };
                let started = Instant::now();
                let fut = svc.translate(&req, &sc, api_key.as_deref(), &client);
                let outcome = match timeout(Duration::from_secs(8), fut).await {
                    Err(_) => Err(ServiceError::Timeout {
                        elapsed_ms: started.elapsed().as_millis() as u64,
                    }),
                    Ok(res) => res,
                };
                TranslateOutcome {
                    service_id: sc.id,
                    service_name: svc.display_name().to_string(),
                    result: outcome,
                }
            }
        });

        join_all(futures).await
    }

    fn find_service(&self, id: ServiceId) -> Option<Arc<dyn TranslationService>> {
        self.services.iter().find(|s| s.id() == id).cloned()
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of translating via a single service.
pub struct TranslateOutcome {
    /// Service id.
    pub service_id: ServiceId,
    /// Service display name.
    pub service_name: String,
    /// Successful result, or the per-service error.
    pub result: Result<TranslateResult, ServiceError>,
}
