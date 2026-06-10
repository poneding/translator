//! High-level translator that fans out a request to multiple enabled services
//! in parallel and aggregates the results.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::time::timeout;

use crate::config::Config;
use crate::error::ServiceError;
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

/// Coordinator that owns the enabled service implementations and a shared HTTP client.
pub struct Translator {
    services: Vec<Arc<dyn TranslationService>>,
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
        Self { services }
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
        self.translate_each(req, cfg, |_| {}).await
    }

    /// Translate with all enabled services and call `on_outcome` as each
    /// provider finishes. The callback fires in completion order; the returned
    /// vector is sorted back into service priority order for compatibility with
    /// callers that consume the aggregate result.
    pub async fn translate_each<F>(
        &self,
        req: &TranslateRequest,
        cfg: &Config,
        mut on_outcome: F,
    ) -> Vec<TranslateOutcome>
    where
        F: FnMut(&TranslateOutcome),
    {
        let mut enabled: Vec<_> = cfg
            .services
            .values()
            .filter(|sc| sc.enabled)
            .cloned()
            .collect();
        // BH-4.5: aggregate results are returned in priority order (lower
        // number first). Streaming callbacks still fire in completion order.
        enabled.sort_by_key(|sc| sc.priority);

        let client = match client_for_config(cfg) {
            Ok(client) => client,
            Err(error) => {
                return enabled
                    .into_iter()
                    .map(|sc| {
                        let outcome = TranslateOutcome {
                            service_id: sc.id,
                            service_name: sc.id.display_name().to_string(),
                            result: Err(ServiceError::Api {
                                code: "proxy".to_string(),
                                message: error.to_string(),
                            }),
                        };
                        on_outcome(&outcome);
                        outcome
                    })
                    .collect();
            }
        };

        let mut jobs = Vec::new();
        for sc in enabled {
            let Some(service) = self.find_service(sc.id) else {
                continue;
            };

            let api_key = match api_key_for_service(service.as_ref(), &sc) {
                Ok(ApiKeyDecision::Ready(api_key)) => api_key,
                Err(error) => {
                    jobs.push(TranslationJob {
                        service,
                        config: sc,
                        api_key: None,
                        client: client.clone(),
                        preflight_error: Some(error),
                    });
                    continue;
                }
            };

            jobs.push(TranslationJob {
                service,
                config: sc,
                api_key,
                client: client.clone(),
                preflight_error: None,
            });
        }

        let req = req.clone();
        let mut futures: FuturesUnordered<_> = jobs
            .into_iter()
            .map(|job| {
                let req = req.clone();
                async move { job.translate(&req).await }
            })
            .collect();

        let mut outcomes = Vec::new();
        while let Some(outcome) = futures.next().await {
            on_outcome(&outcome);
            outcomes.push(outcome);
        }

        outcomes.sort_by_key(|outcome| service_priority(cfg, outcome.service_id));
        outcomes
    }

    /// Translate with one enabled service.
    pub async fn translate_service(
        &self,
        service_id: ServiceId,
        req: &TranslateRequest,
        cfg: &Config,
    ) -> TranslateOutcome {
        let Some(sc) = cfg
            .services
            .get(service_id.as_str())
            .filter(|service| service.enabled)
            .cloned()
        else {
            return TranslateOutcome {
                service_id,
                service_name: service_id.display_name().to_string(),
                result: Err(ServiceError::Api {
                    code: "disabled".to_string(),
                    message: format!("{} is disabled", service_id.as_str()),
                }),
            };
        };

        let Some(service) = self.find_service(service_id) else {
            return TranslateOutcome {
                service_id,
                service_name: service_id.display_name().to_string(),
                result: Err(ServiceError::Api {
                    code: "unknown_service".to_string(),
                    message: format!("unknown service: {}", service_id.as_str()),
                }),
            };
        };

        let client = match client_for_config(cfg) {
            Ok(client) => client,
            Err(error) => {
                return TranslateOutcome {
                    service_id,
                    service_name: service.display_name().to_string(),
                    result: Err(ServiceError::Api {
                        code: "proxy".to_string(),
                        message: error.to_string(),
                    }),
                };
            }
        };

        let (api_key, preflight_error) = match api_key_for_service(service.as_ref(), &sc) {
            Ok(ApiKeyDecision::Ready(api_key)) => (api_key, None),
            Err(error) => (None, Some(error)),
        };

        TranslationJob {
            service,
            config: sc,
            api_key,
            client,
            preflight_error,
        }
        .translate(req)
        .await
    }

    fn find_service(&self, id: ServiceId) -> Option<Arc<dyn TranslationService>> {
        self.services.iter().find(|s| s.id() == id).cloned()
    }
}

fn service_priority(cfg: &Config, id: ServiceId) -> u8 {
    cfg.services
        .get(id.as_str())
        .map(|service| service.priority)
        .unwrap_or(u8::MAX)
}

struct TranslationJob {
    service: Arc<dyn TranslationService>,
    config: ServiceConfig,
    api_key: Option<String>,
    client: reqwest::Client,
    preflight_error: Option<ServiceError>,
}

impl TranslationJob {
    async fn translate(self, req: &TranslateRequest) -> TranslateOutcome {
        if let Some(error) = self.preflight_error {
            return TranslateOutcome {
                service_id: self.config.id,
                service_name: self.service.display_name().to_string(),
                result: Err(error),
            };
        }

        let started = Instant::now();
        let fut = self
            .service
            .translate(req, &self.config, self.api_key.as_deref(), &self.client);
        let result = match timeout(Duration::from_secs(8), fut).await {
            Err(_) => Err(ServiceError::Timeout {
                elapsed_ms: started.elapsed().as_millis() as u64,
            }),
            Ok(result) => result,
        };

        TranslateOutcome {
            service_id: self.config.id,
            service_name: self.service.display_name().to_string(),
            result,
        }
    }
}

enum ApiKeyDecision {
    Ready(Option<String>),
}

fn api_key_for_service(
    service: &dyn TranslationService,
    cfg: &ServiceConfig,
) -> std::result::Result<ApiKeyDecision, ServiceError> {
    match service.api_key_requirement() {
        ApiKeyRequirement::None => Ok(ApiKeyDecision::Ready(None)),
        ApiKeyRequirement::Optional => match crate::secrets::get_api_key(cfg.id.as_str()) {
            Ok(Some(key)) if !key.trim().is_empty() => Ok(ApiKeyDecision::Ready(Some(key))),
            Ok(_) => Ok(ApiKeyDecision::Ready(None)),
            Err(_) => Ok(ApiKeyDecision::Ready(None)),
        },
        ApiKeyRequirement::Required => match crate::secrets::get_api_key(cfg.id.as_str()) {
            Ok(Some(key)) if !key.trim().is_empty() => Ok(ApiKeyDecision::Ready(Some(key))),
            Ok(_) => Err(ServiceError::MissingCredentials(format!(
                "{}.apiKey",
                cfg.id.as_str()
            ))),
            Err(error) => Err(ServiceError::Api {
                code: "keyring".to_string(),
                message: error.to_string(),
            }),
        },
    }
}

fn client_for_config(cfg: &Config) -> reqwest::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("translator/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(5));

    if cfg.general.proxy.enabled && !cfg.general.proxy.url.trim().is_empty() {
        builder = builder.proxy(reqwest::Proxy::all(cfg.general.proxy.url.trim())?);
    }

    builder.build()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn disabled_services_return_no_outcomes() {
        let translator = Translator::new();
        let request = TranslateRequest::auto("hello", "zh-Hans");
        let mut config = Config::default();
        for service in config.services.values_mut() {
            service.enabled = false;
        }

        let outcomes = translator.translate_all(&request, &config).await;

        assert!(outcomes.is_empty());
    }

    #[tokio::test]
    async fn translate_each_emits_in_completion_order() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/jsonrpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "texts": [{ "text": "Hallo langsam" }],
                            "lang": "EN"
                        }
                    }))
                    .set_delay(Duration::from_millis(150)),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sentences": [{ "trans": "Hallo schnell", "orig": "Hello" }],
                "src": "en"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let mut config = Config::default();
        for service in config.services.values_mut() {
            service.enabled = false;
        }
        config.services.insert(
            ServiceId::DeepL.as_str().to_string(),
            ServiceConfig {
                id: ServiceId::DeepL,
                enabled: true,
                priority: 0,
                options: json!({ "web_base_url": format!("{}/jsonrpc", server.uri()) }),
            },
        );
        config.services.insert(
            ServiceId::Google.as_str().to_string(),
            ServiceConfig {
                id: ServiceId::Google,
                enabled: true,
                priority: 1,
                options: json!({ "gtx_base_url": server.uri() }),
            },
        );

        let translator = Translator::new();
        let request = TranslateRequest::auto("Hello", "DE");
        let mut emitted = Vec::new();

        let outcomes = translator
            .translate_each(&request, &config, |outcome| {
                emitted.push(outcome.service_id);
            })
            .await;

        assert_eq!(emitted, vec![ServiceId::Google, ServiceId::DeepL]);
        assert_eq!(
            outcomes
                .iter()
                .map(|outcome| outcome.service_id)
                .collect::<Vec<_>>(),
            vec![ServiceId::DeepL, ServiceId::Google]
        );
    }
}
