//! Google Translate service.
//!
//! Uses Cloud Translation v3 when both `projectId` and an API key are
//! configured, otherwise falls back to the public GTX endpoint.
//!
//! See DESIGN.md §4.2.3.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::Instant;

use crate::error::{ServiceError, ServiceResult};
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

const DEFAULT_CLOUD_BASE: &str = "https://translation.googleapis.com";
const DEFAULT_GTX_BASE: &str = "https://translate.googleapis.com";
/// Default v3 location; per DESIGN.md we always use `global`.
const LOCATION: &str = "global";

/// Google Translate service implementation.
pub struct GoogleService;

impl GoogleService {
    /// Resolve base URL (option override; default = translation.googleapis.com).
    fn resolve_cloud_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_CLOUD_BASE.to_string())
    }

    /// Resolve the public GTX base URL.
    fn resolve_gtx_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("gtx_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_GTX_BASE.to_string())
    }

    /// Resolve GCP project ID from `cfg.options`.
    fn resolve_project(cfg: &ServiceConfig) -> Option<String> {
        cfg.options
            .get("projectId")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    /// Convert app language ids into Google web language ids.
    fn gtx_language(code: &str) -> String {
        match code.to_ascii_lowercase().as_str() {
            "zh-hans" | "zh-cn" => "zh-CN".to_string(),
            "zh-hant" | "zh-tw" | "zh-hk" => "zh-TW".to_string(),
            "" => "auto".to_string(),
            other => other.to_string(),
        }
    }

    async fn translate_cloud(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        project: String,
        key: &str,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let base_url = Self::resolve_cloud_base_url(cfg);

        let body = TranslateTextRequest {
            source_language_code: req.from.as_deref(),
            target_language_code: req.to.as_str(),
            contents: vec![req.text.as_str()],
            mime_type: "text/plain",
        };

        let url = format!(
            "{base_url}/v3/projects/{project}/locations/{LOCATION}:translateText?key={key}"
        );

        let response = client.post(&url).json(&body).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_google_error(
                status,
                response.text().await.unwrap_or_default(),
            ));
        }

        let parsed: TranslateTextResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("google v3 json: {e}")))?;

        let item =
            parsed.translations.into_iter().next().ok_or_else(|| {
                ServiceError::Parse("google v3: no translations[] entry".to_string())
            })?;

        Ok(TranslateResult {
            service_id: ServiceId::Google,
            service_name: "Google Translate".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            text: item.translated_text,
            audio_url: None,
            detected_source: item.detected_language_code,
            elapsed_ms: started.elapsed().as_millis() as u64,
            dictionary: None,
            source_dictionary: None,
            target_dictionary: None,
            extra: None,
            alternatives: Vec::new(),
        })
    }

    async fn translate_gtx(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let base_url = Self::resolve_gtx_base_url(cfg);
        let source = req
            .from
            .as_deref()
            .map(Self::gtx_language)
            .unwrap_or_else(|| "auto".to_string());
        let target = Self::gtx_language(&req.to);
        let query = [
            ("q", req.text.as_str()),
            ("sl", source.as_str()),
            ("tl", target.as_str()),
            ("dt", "t"),
            ("dj", "1"),
            ("ie", "UTF-8"),
            ("client", "gtx"),
        ];

        let response = client
            .get(format!("{base_url}/translate_a/single"))
            .query(&query)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(map_google_error(
                status,
                response.text().await.unwrap_or_default(),
            ));
        }

        let parsed: GtxResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("google gtx json: {e}")))?;
        let text = parsed
            .sentences
            .into_iter()
            .filter_map(|sentence| sentence.trans)
            .collect::<String>()
            .trim()
            .to_string();
        if text.is_empty() {
            return Err(ServiceError::Parse(
                "google gtx: empty translated text".to_string(),
            ));
        }

        Ok(TranslateResult {
            service_id: ServiceId::Google,
            service_name: "Google Translate".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            text,
            audio_url: None,
            detected_source: parsed.src,
            elapsed_ms: started.elapsed().as_millis() as u64,
            dictionary: None,
            source_dictionary: None,
            target_dictionary: None,
            extra: None,
            alternatives: Vec::new(),
        })
    }
}

#[derive(serde::Serialize)]
struct TranslateTextRequest<'a> {
    #[serde(rename = "sourceLanguageCode")]
    source_language_code: Option<&'a str>,
    #[serde(rename = "targetLanguageCode")]
    target_language_code: &'a str,
    contents: Vec<&'a str>,
    #[serde(rename = "mimeType")]
    mime_type: &'static str,
}

#[derive(Deserialize)]
struct TranslateTextResponse {
    #[serde(default)]
    translations: Vec<TranslationItem>,
}

#[derive(Deserialize)]
struct TranslationItem {
    #[serde(rename = "translatedText")]
    translated_text: String,
    #[serde(rename = "detectedLanguageCode", default)]
    detected_language_code: Option<String>,
}

#[derive(Deserialize)]
struct GtxResponse {
    #[serde(default)]
    sentences: Vec<GtxSentence>,
    #[serde(default)]
    src: Option<String>,
}

#[derive(Deserialize)]
struct GtxSentence {
    #[serde(default)]
    trans: Option<String>,
}

#[derive(Deserialize)]
struct GoogleErrorBody {
    error: Option<GoogleError>,
}

#[derive(Deserialize)]
struct GoogleError {
    #[allow(dead_code)]
    code: Option<u16>,
    message: Option<String>,
    status: Option<String>,
}

fn map_google_error(status: StatusCode, body_text: String) -> ServiceError {
    let parsed: Result<GoogleErrorBody, _> = serde_json::from_str(&body_text);
    let mapped = match (status, parsed.as_ref().ok().and_then(|b| b.error.as_ref())) {
        (StatusCode::UNAUTHORIZED, _) => "invalid_credentials",
        (StatusCode::FORBIDDEN, _) => "invalid_credentials",
        (StatusCode::TOO_MANY_REQUESTS, _) => "rate_limited",
        (StatusCode::PAYMENT_REQUIRED, _) => "quota_exceeded",
        (_, Some(err)) => match err.status.as_deref() {
            Some("INVALID_ARGUMENT") => "bad_request",
            Some("PERMISSION_DENIED") => "invalid_credentials",
            Some("RESOURCE_EXHAUSTED") => "quota_exceeded",
            Some("UNAVAILABLE") => "upstream",
            _ => "api",
        },
        (_, None) if status.is_server_error() => "upstream",
        (_, None) => "api",
    };
    let message = parsed
        .ok()
        .and_then(|b| b.error)
        .and_then(|e| e.message)
        .unwrap_or(body_text);
    ServiceError::Api {
        code: mapped.to_string(),
        message,
    }
}

#[async_trait]
impl TranslationService for GoogleService {
    fn id(&self) -> ServiceId {
        ServiceId::Google
    }

    fn display_name(&self) -> &'static str {
        "Google Translate"
    }

    fn api_key_requirement(&self) -> ApiKeyRequirement {
        ApiKeyRequirement::Optional
    }

    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "string", "title": "GCP Project ID" },
                "base_url":  { "type": "string", "title": "Cloud Base URL (override)" },
                "gtx_base_url":  { "type": "string", "title": "GTX Base URL (override)" }
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
        match (Self::resolve_project(cfg), api_key) {
            (Some(project), Some(key)) if !key.trim().is_empty() => {
                Self::translate_cloud(req, cfg, project, key, client).await
            }
            _ => Self::translate_gtx(req, cfg, client).await,
        }
    }
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

    use super::GoogleService;

    const TEST_KEY: &str = "google-test-key";
    const TEST_PROJECT: &str = "my-gcp-project";

    fn cfg_for(mock: &MockServer) -> ServiceConfig {
        ServiceConfig {
            id: ServiceId::Google,
            enabled: true,
            priority: 0,
            options: json!({ "projectId": TEST_PROJECT, "base_url": mock.uri() }),
        }
    }

    fn ok_response(text: &str, detected: Option<&str>) -> serde_json::Value {
        json!({
            "translations": [{
                "translatedText": text,
                "detectedLanguageCode": detected,
            }]
        })
    }

    // ---- S1: happy path ----
    #[tokio::test]
    async fn translate_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v3/projects/{TEST_PROJECT}/locations/global:translateText"
            )))
            .and(query_param("key", TEST_KEY))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("你好", Some("en"))))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let res = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "你好");
        assert_eq!(res.detected_source.as_deref(), Some("en"));
        assert_eq!(res.service_id, ServiceId::Google);
    }

    fn gtx_response(text: &str, detected: &str) -> serde_json::Value {
        json!({
            "sentences": [{ "trans": text, "orig": "Hello" }],
            "src": detected
        })
    }

    // ---- S2: missing API key -> public GTX fallback ----
    #[tokio::test]
    async fn translate_missing_api_key_uses_gtx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .and(query_param("client", "gtx"))
            .and(query_param("sl", "auto"))
            .and(query_param("tl", "zh-CN"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gtx_response("你好", "en")))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = ServiceConfig {
            id: ServiceId::Google,
            enabled: true,
            priority: 0,
            options: json!({ "projectId": TEST_PROJECT, "gtx_base_url": server.uri() }),
        };
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let res = GoogleService
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "你好");
        assert_eq!(res.detected_source.as_deref(), Some("en"));
    }

    // ---- S3: missing projectId -> public GTX fallback ----
    #[tokio::test]
    async fn translate_missing_project_id_uses_gtx() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .and(query_param("client", "gtx"))
            .and(query_param("sl", "en"))
            .and(query_param("tl", "zh-CN"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gtx_response("你好", "en")))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = ServiceConfig {
            id: ServiceId::Google,
            enabled: true,
            priority: 0,
            options: json!({ "gtx_base_url": server.uri() }),
        };
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: Some("en".to_string()),
            to: "zh-CN".to_string(),
        };
        let res = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "你好");
        assert_eq!(res.detected_source.as_deref(), Some("en"));
    }

    // ---- S4: 401 -> invalid_credentials ----
    #[tokio::test]
    async fn translate_401_invalid_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": { "code": 401, "message": "API key not valid", "status": "UNAUTHENTICATED" }
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "invalid_credentials"),
            other => panic!("expected invalid_credentials, got: {other:?}"),
        }
    }

    // ---- S5: 403 -> invalid_credentials (per Google conventions) ----
    #[tokio::test]
    async fn translate_403_invalid_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "error": { "code": 403, "message": "permission denied", "status": "PERMISSION_DENIED" }
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "invalid_credentials"),
            other => panic!("expected invalid_credentials, got: {other:?}"),
        }
    }

    // ---- S6: 429 -> rate_limited ----
    #[tokio::test]
    async fn translate_429_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": { "code": 429, "message": "rate limit", "status": "RESOURCE_EXHAUSTED" }
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "rate_limited"),
            other => panic!("expected rate_limited, got: {other:?}"),
        }
    }

    // ---- S7: 5xx -> upstream ----
    #[tokio::test]
    async fn translate_500_upstream() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "upstream"),
            other => panic!("expected upstream, got: {other:?}"),
        }
    }

    // ---- S8: empty translations[] -> Parse ----
    #[tokio::test]
    async fn translate_empty_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "translations": [] })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = GoogleService
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
            .respond_with(ResponseTemplate::new(200).set_body_string("{not-json"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = GoogleService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }
}
