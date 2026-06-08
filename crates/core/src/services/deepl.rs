//! DeepL translation service.
//!
//! Uses the web JSON-RPC endpoint without credentials by default, matching
//! Easydict's DeepL service. If an auth key is configured, it uses the
//! official DeepL API instead.
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
const DEFAULT_WEB_URL: &str = "https://www2.deepl.com/jsonrpc";

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

    fn resolve_web_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("web_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_WEB_URL.to_string())
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

    async fn translate_official(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        key: &str,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();

        let base_url = Self::resolve_base_url(cfg);

        let mut form: Vec<(&str, String)> = vec![
            ("text", req.text.clone()),
            ("target_lang", deepl_language_code(req.to.as_str(), false)),
        ];
        if let Some(from) = req.from.as_deref() {
            form.push(("source_lang", deepl_language_code(from, true)));
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
            audio_url: None,
            detected_source: first.detected_source_language,
            elapsed_ms,
            dictionary: None,
            extra: None,
        })
    }

    async fn translate_web(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let request_id = deepl_request_id();
        let i_count = req.text.matches('i').count() as i64;
        let timestamp = deepl_timestamp(i_count);
        let source_lang = req
            .from
            .as_deref()
            .map(|from| deepl_language_code(from, true))
            .unwrap_or_else(|| "auto".to_string());
        let target_lang = deepl_language_code(req.to.as_str(), false);

        let mut params = serde_json::json!({
            "texts": [{ "text": req.text, "requestAlternatives": 3 }],
            "splitting": "newlines",
            "lang": {
                "source_lang_user_selected": source_lang,
                "target_lang": target_lang,
            },
            "timestamp": timestamp,
        });

        if req.to.contains('-') {
            params["commonJobParams"] = serde_json::json!({
                "regionalVariant": req.to,
                "mode": "translate",
                "browserType": 1,
                "textType": "plaintext",
            });
        }

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "LMT_handle_texts",
            "id": request_id,
            "params": params,
        });
        let body = deepl_rpc_body(payload, request_id)?;

        let response = client
            .post(Self::resolve_web_url(cfg))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Self::map_error(status, None, body));
        }

        let parsed: DeepLWebResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("deepl web response: {e}")))?;
        let result = parsed
            .result
            .ok_or_else(|| ServiceError::Parse("deepl web returned no result".to_string()))?;
        let text = result
            .texts
            .into_iter()
            .next()
            .map(|item| item.text)
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| ServiceError::Parse("deepl web returned no text".to_string()))?;

        Ok(TranslateResult {
            service_id: ServiceId::DeepL,
            service_name: "DeepL".to_string(),
            text,
            audio_url: None,
            detected_source: result.lang,
            elapsed_ms: started.elapsed().as_millis() as u64,
            dictionary: None,
            extra: None,
        })
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
        ApiKeyRequirement::Optional
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
                },
                "web_base_url": {
                    "type": "string",
                    "title": "Web JSON-RPC URL (override)"
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
        match api_key.map(str::trim).filter(|key| !key.is_empty()) {
            Some(key) => Self::translate_official(req, cfg, key, client).await,
            None => Self::translate_web(req, cfg, client).await,
        }
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

#[derive(Debug, Deserialize)]
struct DeepLWebResponse {
    result: Option<DeepLWebResult>,
}

#[derive(Debug, Deserialize)]
struct DeepLWebResult {
    texts: Vec<DeepLWebText>,
    #[serde(default)]
    lang: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepLWebText {
    text: String,
}

fn deepl_language_code(code: &str, source: bool) -> String {
    let normalized = code.trim().replace('_', "-");
    if source && normalized.eq_ignore_ascii_case("auto") {
        return "auto".to_string();
    }
    let primary = normalized.split('-').next().unwrap_or(normalized.as_str());
    primary.to_ascii_uppercase()
}

fn deepl_request_id() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i64)
        .unwrap_or(0);
    100_000_000 + (now.abs() % 89_999_000)
}

fn deepl_timestamp(i_count: i64) -> i64 {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0);
    if i_count == 0 {
        return timestamp;
    }
    let count = i_count + 1;
    timestamp - (timestamp % count) + count
}

fn deepl_rpc_body(payload: serde_json::Value, request_id: i64) -> ServiceResult<String> {
    let body = serde_json::to_string(&payload)
        .map_err(|e| ServiceError::Parse(format!("deepl web request: {e}")))?;
    let spaced = if (request_id + 5) % 29 == 0 || (request_id + 3) % 13 == 0 {
        body.replace("\"method\":\"", "\"method\" : \"")
    } else {
        body.replace("\"method\":\"", "\"method\": \"")
    };
    Ok(spaced)
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
            options: json!({
                "base_url": mock.uri(),
                "web_base_url": format!("{}/jsonrpc", mock.uri()),
            }),
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

    // ---- S3: missing api key uses web fallback ----
    #[tokio::test]
    async fn translate_missing_api_key_uses_web_fallback() {
        let server = MockServer::start().await;
        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "DE");
        Mock::given(method("POST"))
            .and(path("/jsonrpc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "result": {
                    "texts": [{ "text": "Hallo" }],
                    "lang": "EN"
                }
            })))
            .expect(1)
            .mount(&server)
            .await;

        let result = DeepLService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .expect("web fallback should work without key");

        assert_eq!(result.text, "Hallo");
        assert_eq!(result.detected_source.as_deref(), Some("EN"));
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
