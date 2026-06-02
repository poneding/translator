//! Youdao (有道) translation service.
//!
//! Endpoint: `https://openapi.youdao.com/api`
//! Auth: HMAC-SHA256 signature with `appKey + truncate(q) + salt + curtime + appSecret`.
//! Body: `application/x-www-form-urlencoded`.
//!
//! See DESIGN.md §4.2.1 for the request/response schema.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::{ServiceError, ServiceResult};
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

const DEFAULT_BASE: &str = "https://openapi.youdao.com";

/// Youdao service implementation.
pub struct YoudaoService;

impl YoudaoService {
    /// Resolve the base URL (option override; default = openapi.youdao.com).
    fn resolve_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_BASE.to_string())
    }

    /// Compute `truncate(q)` per Youdao spec:
    /// - if `q.len() <= 20`: return `q` as-is
    /// - else: return `q[0..10] + q.len().to_string() + q[q.len()-10..]`
    fn truncate(q: &str) -> String {
        let len = q.chars().count();
        if len <= 20 {
            return q.to_string();
        }
        let chars: Vec<char> = q.chars().collect();
        let head: String = chars.iter().take(10).collect();
        let tail: String = chars.iter().skip(len - 10).collect();
        format!("{head}{len}{tail}")
    }

    /// Compute the v3 sign: `sha256(appKey + truncate(q) + salt + curtime + appSecret)`.
    fn sign_v3(app_key: &str, q: &str, salt: &str, curtime: i64, app_secret: &str) -> String {
        let truncated = Self::truncate(q);
        let raw = format!("{app_key}{truncated}{salt}{curtime}{app_secret}");
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let digest = hasher.finalize();
        format!("{:x}", digest)
    }

    /// Map a Youdao error code to a typed ServiceError.
    /// Common codes: "0" = success, "101" = missing appKey, "102" = missing appSecret,
    /// "108" = appKey/appSecret mismatch, "202" = missing q, "302" = translation fail,
    /// "401" = account balance exhausted, "411" = access frequency limited.
    fn map_error_code(code: &str) -> ServiceError {
        let mapped = match code {
            "0" => {
                return ServiceError::Api {
                    code: "success".to_string(),
                    message: code.to_string(),
                }
            }
            "101" | "102" | "108" => "invalid_credentials",
            "202" => "bad_request",
            "401" => "quota_exceeded",
            "411" => "rate_limited",
            "302" | "303" => "upstream",
            _ => "api",
        };
        ServiceError::Api {
            code: mapped.to_string(),
            message: format!("youdao errorCode={code}"),
        }
    }
}

fn read_youdao_creds(cfg: &ServiceConfig) -> ServiceResult<(String, String)> {
    let app_key = cfg
        .options
        .get("appKey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServiceError::MissingCredentials("youdao.appKey".to_string()))?
        .to_string();
    let app_secret = cfg
        .options
        .get("appSecret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ServiceError::MissingCredentials("youdao.appSecret".to_string()))?
        .to_string();
    Ok((app_key, app_secret))
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[async_trait]
impl TranslationService for YoudaoService {
    fn id(&self) -> ServiceId {
        ServiceId::Youdao
    }

    fn display_name(&self) -> &'static str {
        "Youdao"
    }

    fn api_key_requirement(&self) -> ApiKeyRequirement {
        ApiKeyRequirement::Required
    }

    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["appKey", "appSecret"],
            "properties": {
                "appKey":    { "type": "string", "title": "App Key" },
                "appSecret": { "type": "string", "title": "App Secret", "format": "password" },
                "base_url":  { "type": "string", "title": "Base URL (override)" }
            }
        })
    }

    async fn translate(
        &self,
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        _api_key: Option<&str>,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let (app_key, app_secret) = read_youdao_creds(cfg)?;
        let started = Instant::now();
        let base_url = Self::resolve_base_url(cfg);

        let salt = Uuid::new_v4().simple().to_string();
        let curtime = now_unix();
        let sign = Self::sign_v3(&app_key, &req.text, &salt, curtime, &app_secret);

        // Youdao language codes: EN, ZH_CHS, JA, KO, FR, DE, ES, RU, etc.
        // We pass through whatever the user supplied; Youdao will reject unknown.
        let from = req.from.as_deref().unwrap_or("auto");
        let to = req.to.as_str();

        let form: Vec<(&str, String)> = vec![
            ("q", req.text.clone()),
            ("from", from.to_string()),
            ("to", to.to_string()),
            ("appKey", app_key),
            ("salt", salt),
            ("curtime", curtime.to_string()),
            ("sign", sign),
            ("signType", "v3".to_string()),
        ];

        let response = client
            .post(format!("{base_url}/api"))
            .form(&form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status {
                StatusCode::TOO_MANY_REQUESTS => ServiceError::RateLimited {
                    retry_after_ms: 5_000,
                },
                _ => ServiceError::Api {
                    code: "upstream".to_string(),
                    message: body,
                },
            });
        }

        let parsed: YoudaoResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("youdao json: {e}")))?;

        if parsed.error_code != "0" {
            return Err(Self::map_error_code(&parsed.error_code));
        }
        let text =
            parsed.translation.into_iter().next().ok_or_else(|| {
                ServiceError::Parse("youdao: no translation in response".to_string())
            })?;

        let elapsed_ms = started.elapsed().as_millis() as u64;

        Ok(TranslateResult {
            service_id: ServiceId::Youdao,
            service_name: "Youdao".to_string(),
            text,
            detected_source: None, // Youdao doesn't echo the detected source in /api
            elapsed_ms,
            extra: None,
        })
    }
}

#[derive(Debug, Deserialize)]
struct YoudaoResponse {
    #[serde(rename = "errorCode")]
    error_code: String,
    #[serde(default)]
    translation: Vec<String>,
    #[serde(default, rename = "basic")]
    _basic: Option<serde_json::Value>,
    #[serde(default, rename = "web")]
    _web: Vec<serde_json::Value>,
}

// =============================================================================
// Tests — TDD: written before the impl (PLAN.md M2.1 + M2.2).
// =============================================================================
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use reqwest::Client;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::error::ServiceError;
    use crate::model::{ServiceId, TranslateRequest};
    use crate::service::ServiceConfig;
    use crate::TranslationService;

    use super::YoudaoService;

    /// Test fixture creds.
    const TEST_KEY: &str = "test-app-key";
    const TEST_SECRET: &str = "test-app-secret";

    fn cfg_for(mock: &MockServer) -> ServiceConfig {
        ServiceConfig {
            id: ServiceId::Youdao,
            enabled: true,
            priority: 0,
            options: json!({
                "appKey": TEST_KEY,
                "appSecret": TEST_SECRET,
                "base_url": mock.uri(),
            }),
        }
    }

    fn ok_body(translation: &str) -> serde_json::Value {
        json!({
            "errorCode": "0",
            "translation": [translation],
            "basic": null,
            "web": [],
            "query": "Hello",
            "l": "en2zh-CHS"
        })
    }

    // ---- Pure helper: truncate() ----
    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(YoudaoService::truncate("Hi"), "Hi");
        assert_eq!(YoudaoService::truncate("Hello, world!"), "Hello, world!");
    }

    #[test]
    fn truncate_long_string_uses_head_and_tail() {
        // 21+ chars triggers truncation
        let long = "a".repeat(25);
        let result = YoudaoService::truncate(&long);
        assert_eq!(result, format!("{}25{}", "a".repeat(10), "a".repeat(10)));
    }

    // ---- S1: happy path ----
    #[tokio::test]
    async fn translate_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .and(body_string_contains("sign="))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("你好")))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hello", "zh-CHS");
        let result = YoudaoService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .expect("translate should succeed");
        assert_eq!(result.text, "你好");
        assert_eq!(result.service_id, ServiceId::Youdao);
    }

    // ---- S2: sign computation is stable ----
    #[test]
    fn sign_v3_is_deterministic() {
        let s1 = YoudaoService::sign_v3("key", "q", "salt", 1700000000, "secret");
        let s2 = YoudaoService::sign_v3("key", "q", "salt", 1700000000, "secret");
        assert_eq!(s1, s2);
        // 64 hex chars
        assert_eq!(s1.len(), 64);
        assert!(s1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ---- S3: missing appKey ----
    #[tokio::test]
    async fn translate_missing_appkey() {
        let cfg = ServiceConfig {
            id: ServiceId::Youdao,
            enabled: true,
            priority: 0,
            options: json!({ "appSecret": "x", "base_url": "http://x" }),
        };
        let req = TranslateRequest::auto("Hi", "zh-CHS");
        let err = YoudaoService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .expect_err("should fail without appKey");
        assert!(matches!(err, ServiceError::MissingCredentials(ref s) if s.contains("appKey")));
    }

    // ---- S4: errorCode 101 -> invalid_credentials ----
    #[tokio::test]
    async fn translate_errorcode_101_invalid_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errorCode": "101",
                "translation": [],
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "zh-CHS");
        let err = YoudaoService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "invalid_credentials"),
            other => panic!("expected Api invalid_credentials, got: {other:?}"),
        }
    }

    // ---- S5: errorCode 401 -> quota_exceeded ----
    #[tokio::test]
    async fn translate_errorcode_401_quota() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errorCode": "401",
                "translation": [],
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "zh-CHS");
        let err = YoudaoService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "quota_exceeded"),
            other => panic!("expected Api quota_exceeded, got: {other:?}"),
        }
    }

    // ---- S6: empty translation array -> Parse ----
    #[tokio::test]
    async fn translate_empty_translation_array() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errorCode": "0",
                "translation": [],
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest::auto("Hi", "zh-CHS");
        let err = YoudaoService
            .translate(&req, &cfg, None, &Client::new())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }
}
