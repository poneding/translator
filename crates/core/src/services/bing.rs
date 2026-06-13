//! Microsoft Bing / Azure Translator service.
//!
//! Uses Bing's web translator endpoint without credentials by default,
//! matching Easydict. If an Azure key is configured, it uses the official
//! Microsoft Translator API instead.
//!
//! See DESIGN.md §4.2.4.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::{ServiceError, ServiceResult};
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

const DEFAULT_BASE: &str = "https://api.cognitive.microsofttranslator.com";
const DEFAULT_WEB_BASE: &str = "https://cn.bing.com";
const API_VERSION: &str = "3.0";
const WEB_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

/// Bing / Azure Translator service implementation.
pub struct BingService;

impl BingService {
    /// Resolve base URL (option override; default = cognitive.microsofttranslator.com).
    fn resolve_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_BASE.to_string())
    }

    /// Resolve Azure region (defaults to `global`).
    fn resolve_region(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("region")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "global".to_string())
    }

    fn resolve_web_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("web_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_WEB_BASE.to_string())
    }

    async fn translate_official(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        key: &str,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let base_url = Self::resolve_base_url(cfg);
        let region = Self::resolve_region(cfg);

        let body = vec![BingBody {
            text: req.text.as_str(),
        }];

        // Build URL with api-version; from/to are query params.
        let mut url = format!(
            "{base_url}/translate?api-version={API_VERSION}&to={}",
            req.to
        );
        if let Some(from) = req.from.as_deref() {
            url.push_str("&from=");
            url.push_str(from);
        }

        let response = client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", key)
            .header("Ocp-Apim-Subscription-Region", &region)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        Self::parse_translate_response(req, response, started).await
    }

    async fn translate_web(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let base_url = Self::resolve_web_base_url(cfg);
        let web_config = fetch_web_config(client, &base_url).await?;

        let mut form: Vec<(&str, String)> = vec![
            ("text", req.text.clone()),
            ("to", req.to.clone()),
            ("token", web_config.token),
            ("key", web_config.key),
            ("tryFetchingGenderDebiasedTranslations", "true".to_string()),
        ];
        let from = req
            .from
            .as_deref()
            .filter(|from| !from.eq_ignore_ascii_case("auto"))
            .unwrap_or("auto-detect");
        form.push(("fromLang", from.to_string()));

        let url = format!(
            "{base_url}/ttranslatev3?isVertical=1&IG={}&IID={}",
            web_config.ig, web_config.iid
        );
        let response = client
            .post(url)
            .header("User-Agent", WEB_USER_AGENT)
            .form(&form)
            .send()
            .await?;

        Self::parse_translate_response(req, response, started).await
    }

    async fn parse_translate_response(
        req: &TranslateRequest,
        response: reqwest::Response,
        started: Instant,
    ) -> ServiceResult<TranslateResult> {
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let mapped = match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => "invalid_credentials",
                StatusCode::TOO_MANY_REQUESTS => "rate_limited",
                StatusCode::PAYMENT_REQUIRED => "quota_exceeded",
                s if s.is_server_error() => "upstream",
                _ => "api",
            };
            // Azure error format: {"error":{"code":401000,"message":"..."}}
            let message = serde_json::from_str::<serde_json::Value>(&body_text)
                .ok()
                .and_then(|v| {
                    v.get("error")?
                        .get("message")?
                        .as_str()
                        .map(|s| s.to_string())
                })
                .unwrap_or(body_text);
            return Err(ServiceError::Api {
                code: mapped.to_string(),
                message,
            });
        }

        let parsed: Vec<BingResponse> = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("bing json: {e}")))?;

        let item = parsed
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::Parse("bing: empty response array".to_string()))?;
        let translation = item
            .translations
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::Parse("bing: no translations[] entry".to_string()))?;

        let detected = item.detected_language.and_then(|d| d.language);

        let elapsed_ms = started.elapsed().as_millis() as u64;
        Ok(TranslateResult {
            service_id: ServiceId::Bing,
            service_name: "Microsoft Translator".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            text: translation.text,
            audio_url: None,
            detected_source: detected,
            elapsed_ms,
            dictionary: None,
            source_dictionary: None,
            target_dictionary: None,
            extra: None,
        })
    }
}

#[derive(Serialize)]
struct BingBody<'a> {
    #[serde(rename = "Text")]
    text: &'a str,
}

#[derive(Deserialize)]
struct BingResponse {
    #[serde(default)]
    translations: Vec<BingTranslation>,
    #[serde(rename = "detectedLanguage", default)]
    detected_language: Option<BingDetected>,
}

#[derive(Deserialize)]
struct BingTranslation {
    #[serde(rename = "text")]
    text: String,
    #[serde(default, rename = "to")]
    #[allow(dead_code)]
    to: Option<String>,
}

#[derive(Deserialize)]
struct BingDetected {
    #[serde(default, rename = "language")]
    language: Option<String>,
    #[serde(default, rename = "score")]
    #[allow(dead_code)]
    score: Option<f64>,
}

#[async_trait]
impl TranslationService for BingService {
    fn id(&self) -> ServiceId {
        ServiceId::Bing
    }

    fn display_name(&self) -> &'static str {
        "Microsoft Translator"
    }

    fn api_key_requirement(&self) -> ApiKeyRequirement {
        ApiKeyRequirement::Optional
    }

    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "region":   { "type": "string", "title": "Azure Region", "default": "global" },
                "base_url": { "type": "string", "title": "Base URL (override)" },
                "web_base_url": { "type": "string", "title": "Bing web base URL (override)" }
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

struct BingWebConfig {
    ig: String,
    iid: String,
    key: String,
    token: String,
}

async fn fetch_web_config(client: &Client, base_url: &str) -> ServiceResult<BingWebConfig> {
    let response = client
        .get(format!("{base_url}/translator"))
        .header("User-Agent", WEB_USER_AGENT)
        .send()
        .await?;
    let status = response.status();
    let html = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(ServiceError::Api {
            code: "bing_web_config".to_string(),
            message: html,
        });
    }

    let ig = capture_between(&html, "IG:\"", "\"")
        .ok_or_else(|| ServiceError::Parse("bing web IG missing".to_string()))?;
    let iid = capture_between(&html, "data-iid=\"", "\"")
        .ok_or_else(|| ServiceError::Parse("bing web IID missing".to_string()))?;
    let params = capture_between(&html, "params_AbusePreventionHelper = [", "]")
        .or_else(|| capture_between(&html, "params_AbusePreventionHelper=[", "]"))
        .ok_or_else(|| ServiceError::Parse("bing web token params missing".to_string()))?;
    let mut parts = params.split(',').map(|part| part.trim().trim_matches('"'));
    let key = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| ServiceError::Parse("bing web key missing".to_string()))?
        .to_string();
    let token = parts
        .next()
        .filter(|part| !part.is_empty())
        .ok_or_else(|| ServiceError::Parse("bing web token missing".to_string()))?
        .to_string();

    Ok(BingWebConfig {
        ig,
        iid,
        key,
        token,
    })
}

fn capture_between(text: &str, prefix: &str, suffix: &str) -> Option<String> {
    let start = text.find(prefix)? + prefix.len();
    let tail = &text[start..];
    let end = tail.find(suffix)?;
    Some(tail[..end].to_string())
}

// =============================================================================
// Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::TranslationService;
    use crate::error::ServiceError;
    use crate::model::{ServiceId, TranslateRequest};
    use crate::service::ServiceConfig;

    use super::BingService;

    const TEST_KEY: &str = "bing-test-key";

    fn cfg_for(mock: &MockServer) -> ServiceConfig {
        ServiceConfig {
            id: ServiceId::Bing,
            enabled: true,
            priority: 0,
            options: json!({
                "region": "eastus",
                "base_url": mock.uri(),
                "web_base_url": mock.uri(),
            }),
        }
    }

    fn ok_response(text: &str, detected: Option<&str>) -> serde_json::Value {
        json!([{
            "translations": [{ "text": text, "to": "zh-Hans" }],
            "detectedLanguage": detected.map(|l| json!({ "language": l, "score": 0.95 }))
        }])
    }

    // ---- S1: happy path with from + to, detected language echoed ----
    #[tokio::test]
    async fn translate_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/translate"))
            .and(query_param("api-version", "3.0"))
            .and(query_param("from", "en"))
            .and(query_param("to", "zh-Hans"))
            .and(header("Ocp-Apim-Subscription-Key", TEST_KEY))
            .and(header("Ocp-Apim-Subscription-Region", "eastus"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("你好", Some("en"))))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: Some("en".to_string()),
            to: "zh-Hans".to_string(),
        };
        let res = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "你好");
        assert_eq!(res.detected_source.as_deref(), Some("en"));
        assert_eq!(res.service_id, ServiceId::Bing);
    }

    // ---- S2: happy path auto-detect (no `from`) ----
    #[tokio::test]
    async fn translate_auto_detect_omits_from_param() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/translate"))
            .and(query_param("api-version", "3.0"))
            // MUST NOT contain "from"
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ok_response("Bonjour", Some("fr"))),
            )
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Bonjour".to_string(),
            from: None,
            to: "en".to_string(),
        };
        let res = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "Bonjour");
        assert_eq!(res.detected_source.as_deref(), Some("fr"));
    }

    // ---- S3: missing API key uses Bing web fallback ----
    #[tokio::test]
    async fn translate_missing_api_key_uses_web_fallback() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/translator"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"IG:"abc123", data-iid="translator.5029" params_AbusePreventionHelper = [1693880687457,"token-value",3600000];"#,
            ))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/ttranslatev3"))
            .and(query_param("IG", "abc123"))
            .and(query_param("IID", "translator.5029"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("你好", Some("en"))))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let result = BingService
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .expect("web fallback should work without key");

        assert_eq!(result.text, "你好");
        assert_eq!(result.detected_source.as_deref(), Some("en"));
    }

    // ---- S4: 401 -> invalid_credentials ----
    #[tokio::test]
    async fn translate_401_invalid_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": { "code": 401000, "message": "Access denied due to invalid subscription key." }
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let err = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, message } => {
                assert_eq!(code, "invalid_credentials");
                assert!(message.contains("invalid subscription key"));
            }
            other => panic!("expected invalid_credentials, got: {other:?}"),
        }
    }

    // ---- S5: 429 -> rate_limited ----
    #[tokio::test]
    async fn translate_429_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limit"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let err = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "rate_limited"),
            other => panic!("expected rate_limited, got: {other:?}"),
        }
    }

    // ---- S6: 5xx -> upstream ----
    #[tokio::test]
    async fn translate_503_upstream() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503).set_body_string("service unavailable"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let err = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "upstream"),
            other => panic!("expected upstream, got: {other:?}"),
        }
    }

    // ---- S7: empty array -> Parse ----
    #[tokio::test]
    async fn translate_empty_array() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let err = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }

    // ---- S8: missing translations field -> Parse ----
    #[tokio::test]
    async fn translate_missing_translations() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{}])))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let err = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }

    // ---- S9: malformed JSON -> Parse ----
    #[tokio::test]
    async fn translate_malformed_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-Hans".to_string(),
        };
        let err = BingService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }
}
