//! DeepL translation service.
//!
//! Endpoint: `https://api-free.deepl.com/v2/translate` (Free) or
//! `https://api.deepl.com/v2/translate` (Pro).
//! Auth: `Authorization: DeepL-Auth-Key <KEY>` header.
//! Body: `application/x-www-form-urlencoded`.
//!
//! See DESIGN.md §4.2.2.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::Instant;

use crate::error::{ServiceError, ServiceResult};
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

const DEFAULT_FREE_BASE: &str = "https://api-free.deepl.com";
const DEFAULT_PRO_BASE: &str = "https://api.deepl.com";

/// DeepL service implementation.
pub struct DeepLService;

impl DeepLService {
    /// Resolve the base URL: explicit `base_url` option wins, then `endpoint`, then free default.
    fn resolve_base_url(cfg: &ServiceConfig) -> String {
        if let Some(s) = cfg.options.get("base_url").and_then(|v| v.as_str()) {
            return s.trim_end_matches('/').to_string();
        }
        let endpoint = cfg
            .options
            .get("endpoint")
            .and_then(|v| v.as_str())
            .unwrap_or("free");
        match endpoint {
            "pro" => DEFAULT_PRO_BASE.to_string(),
            _ => DEFAULT_FREE_BASE.to_string(),
        }
    }

    /// Map a DeepL error response to a typed `ServiceError`.
    fn map_error(
        status: StatusCode,
        retry_after_header: Option<&str>,
        body: String,
    ) -> ServiceError {
        match status.as_u16() {
            401 | 403 => ServiceError::Api {
                code: "invalid_credentials".to_string(),
                message: body,
            },
            429 => ServiceError::RateLimited {
                retry_after_ms: parse_retry_after_ms(retry_after_header).unwrap_or(5_000),
            },
            456 => ServiceError::Api {
                code: "quota_exceeded".to_string(),
                message: body,
            },
            500..=599 => ServiceError::Api {
                code: "upstream".to_string(),
                message: body,
            },
            _ => ServiceError::Api {
                code: "api".to_string(),
                message: body,
            },
        }
    }
}

/// Parse a Retry-After header. Supports delta-seconds (integer) and HTTP-date.
/// Returns the wait in milliseconds.
fn parse_retry_after_ms(header: Option<&str>) -> Option<u64> {
    let s = header?.trim();
    if let Ok(secs) = s.parse::<u64>() {
        return Some(secs * 1000);
    }
    // HTTP-date form (RFC 7231): not commonly used by DeepL, omitted for now.
    None
}

#[async_trait]
impl TranslationService for DeepLService {
    fn id(&self) -> ServiceId {
        ServiceId::DeepL
    }

    fn display_name(&self) -> &'static str {
        "DeepL"
    }

    fn api_key_requirement(&self) -> ApiKeyRequirement {
        ApiKeyRequirement::Required
    }

    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "endpoint": {
                    "type": "string",
                    "title": "Endpoint",
                    "description": "DeepL endpoint tier.",
                    "enum": ["free", "pro"],
                    "default": "free"
                },
                "base_url": {
                    "type": "string",
                    "title": "Base URL (override)",
                    "description": "Override the base URL. Useful for self-hosted proxies or testing. Leave empty for default."
                }
            }
        })
    }

    async fn translate(
        &self,
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        api_key: Option<&str>,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let key =
            api_key.ok_or_else(|| ServiceError::MissingCredentials("deepl.apiKey".to_string()))?;
        let started = Instant::now();

        let base_url = Self::resolve_base_url(cfg);

        let mut form: Vec<(&str, &str)> = vec![
            ("text", req.text.as_str()),
            ("target_lang", req.to.as_str()),
        ];
        if let Some(from) = req.from.as_deref() {
            form.push(("source_lang", from));
        }

        let response = client
            .post(format!("{base_url}/v2/translate"))
            .header("Authorization", format!("DeepL-Auth-Key {key}"))
            .form(&form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let body = response.text().await.unwrap_or_default();
            return Err(Self::map_error(status, retry_after.as_deref(), body));
        }

        let parsed: DeepLResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("deepl response: {e}")))?;

        let first = parsed
            .translations
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::Parse("deepl returned no translations".to_string()))?;

        let elapsed_ms = started.elapsed().as_millis() as u64;

        Ok(TranslateResult {
            service_id: ServiceId::DeepL,
            service_name: "DeepL".to_string(),
            text: first.text,
            detected_source: first.detected_source_language,
            elapsed_ms,
            extra: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct DeepLResponse {
    translations: Vec<DeepLTranslation>,
}

#[derive(Debug, Deserialize)]
struct DeepLTranslation {
    text: String,
    #[serde(default)]
    detected_source_language: Option<String>,
}

// =============================================================================
// Tests — TDD: written before the impl. See PLAN.md M1.1 + M1.2.
// =============================================================================
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use reqwest::Client;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::error::ServiceError;
    use crate::model::{ServiceId, TranslateRequest};
    use crate::service::ServiceConfig;
    use crate::TranslationService;

    use super::DeepLService;

    /// Build a ServiceConfig pointing at the given mock server.
    /// `base_url` override lets the test redirect requests away from the real DeepL host.
    fn cfg_for(mock: &MockServer) -> ServiceConfig {
        ServiceConfig {
            id: ServiceId::DeepL,
            enabled: true,
            priority: 0,
            options: json!({ "base_url": mock.uri() }),
        }
    }

    /// Standard DeepL 200 response body.
    fn ok_body(text: &str, detected: Option<&str>) -> serde_json::Value {
        json!({
            "translations": [{
                "text": text,
                "detected_source_language": detected,
            }]
        })
    }

    // ---- S1: happy path ----
    #[tokio::test]
    async fn translate_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/translate"))
            .and(header("Authorization", "DeepL-Auth-Key test-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ok_body("Hallo, Welt!", Some("EN"))),
            )
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hello, world!", "DE");
        let result = DeepLService
            .translate(&req, &cfg, Some("test-key"), &Client::new())
            .await
            .expect("translate should succeed");

        assert_eq!(result.text, "Hallo, Welt!");
        assert_eq!(result.detected_source.as_deref(), Some("EN"));
        assert_eq!(result.service_id, ServiceId::DeepL);
    }

    // ---- S2: explicit source_lang is forwarded ----
    #[tokio::test]
    async fn translate_forwards_source_lang_when_provided() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/translate"))
            .and(body_string_contains("source_lang=EN"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("Hallo", None)))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hello".into(),
            from: Some("EN".into()),
            to: "DE".into(),
        };
        let _ = DeepLService
            .translate(&req, &cfg, Some("k"), &Client::new())
            .await
            .unwrap();
    }

    // ---- S3: missing api key ----
    #[tokio::test]
    async fn translate_missing_api_key() {
        let server = MockServer::start().await;
        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "DE");
        let err = DeepLService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .expect_err("should fail without key");
        assert!(
            matches!(err, ServiceError::MissingCredentials(_)),
            "got: {err:?}"
        );
    }

    // ---- S4: 401 -> invalid_credentials ----
    #[tokio::test]
    async fn translate_401_invalid_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/translate"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "message": "Authorization failed",
                "detail": "The DeepL Auth Key is invalid."
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "DE");
        let err = DeepLService
            .translate(&req, &cfg, Some("bad"), &Client::new())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "invalid_credentials"),
            other => panic!("expected Api invalid_credentials, got: {other:?}"),
        }
    }

    // ---- S5: 429 -> RateLimited ----
    #[tokio::test]
    async fn translate_429_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/translate"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "5")
                    .set_body_json(json!({ "message": "Too many requests" })),
            )
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "DE");
        let err = DeepLService
            .translate(&req, &cfg, Some("k"), &Client::new())
            .await
            .unwrap_err();
        match err {
            ServiceError::RateLimited { retry_after_ms } => {
                assert!(
                    (4000..=6000).contains(&retry_after_ms),
                    "expected ~5000ms, got {retry_after_ms}"
                );
            }
            other => panic!("expected RateLimited, got: {other:?}"),
        }
    }

    // ---- S6: 5xx -> upstream ----
    #[tokio::test]
    async fn translate_5xx_upstream() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/translate"))
            .respond_with(
                ResponseTemplate::new(503)
                    .set_body_json(json!({ "message": "Service Unavailable" })),
            )
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "DE");
        let err = DeepLService
            .translate(&req, &cfg, Some("k"), &Client::new())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "upstream"),
            other => panic!("expected Api upstream, got: {other:?}"),
        }
    }

    // ---- S7: malformed response -> Parse ----
    #[tokio::test]
    async fn translate_malformed_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/translate"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "DE");
        let err = DeepLService
            .translate(&req, &cfg, Some("k"), &Client::new())
            .await
            .unwrap_err();
        assert!(
            matches!(err, ServiceError::Parse(_) | ServiceError::Network(_)),
            "got: {err:?}"
        );
    }
}
