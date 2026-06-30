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
use crate::model::{
    DictionaryPart, DictionaryResult, ServiceId, SimpleDictionaryWord, TranslateRequest,
    TranslateResult, part_abbreviation,
};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

const DEFAULT_CLOUD_BASE: &str = "https://translation.googleapis.com";
const DEFAULT_GTX_BASE: &str = "https://translate.googleapis.com";
/// Default v3 location; per DESIGN.md we always use `global`.
const LOCATION: &str = "global";
/// Default TKK baked into Easydict's `google-translate-sign.js`.
const GOOGLE_DEFAULT_TKK: &str = "444000.1270171236";
/// Base URL for the webapp `translate_a/single` endpoint and TKK refresh.
const DEFAULT_WEBAPP_BASE: &str = "https://translate.google.com";
const WEBAPP_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/77.0.3865.120 Safari/537.36";

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

    /// Resolve the webapp base URL (default = translate.google.com).
    fn resolve_webapp_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("webapp_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_WEBAPP_BASE.to_string())
    }

    /// Try the dictionary-capable webapp path for en<->zh single words, falling
    /// back to GTX when it yields nothing or errors. Mirrors Easydict's routing.
    async fn translate_web_or_gtx(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        if should_use_webapp(&req.text, req.from.as_deref(), &req.to) {
            if let Ok(json) = fetch_webapp(req, cfg, client).await {
                if let Some((text, detected, dict)) =
                    parse_webapp(&json, req.from.as_deref(), &req.to)
                {
                    if !text.is_empty() || dict.is_some() {
                        return Ok(TranslateResult {
                            service_id: ServiceId::Google,
                            service_name: "Google Translate".to_string(),
                            from: req.from.clone(),
                            to: req.to.clone(),
                            text,
                            audio_url: None,
                            detected_source: detected,
                            elapsed_ms: started.elapsed().as_millis() as u64,
                            dictionary: None,
                            source_dictionary: dict,
                            target_dictionary: None,
                            extra: None,
                            alternatives: Vec::new(),
                        });
                    }
                }
            }
        }
        Self::translate_gtx(req, cfg, client).await
    }
}

// =============================================================================
// Google webapp (dictionary-capable) — mirrors Easydict's GoogleService.
// =============================================================================

/// Whether to use the dictionary-capable webapp endpoint: a single short word
/// translated between English and Chinese. Auto-detect source is inferred from
/// the text shape (ASCII -> en, otherwise zh).
fn should_use_webapp(text: &str, from: Option<&str>, to: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return false;
    }
    let len = trimmed.chars().count();
    let ascii = trimmed.chars().all(|c| c.is_ascii());
    let short = if ascii { len <= 20 } else { len <= 7 };
    if !short {
        return false;
    }
    let to_lc = to.to_ascii_lowercase();
    let from_lc = from.map(|f| f.to_ascii_lowercase());
    let from_en = from_lc
        .as_deref()
        .map(|f| f.starts_with("en"))
        .unwrap_or(ascii);
    let from_zh = from_lc
        .as_deref()
        .map(|f| f.starts_with("zh"))
        .unwrap_or(!ascii);
    let to_en = to_lc.starts_with("en");
    let to_zh = to_lc.starts_with("zh");
    (from_en && to_zh) || (from_zh && to_en)
}

/// The `xr` mixing function from `google-translate-sign.js`.
fn google_xr(mut a: u32, b: &str) -> u32 {
    let bytes = b.as_bytes();
    let mut c = 0;
    while c + 2 < bytes.len() {
        let d_char = bytes[c + 2] as char;
        let d: u32 = if d_char >= 'a' {
            (d_char as u32) - 87
        } else {
            d_char.to_digit(10).unwrap_or(0)
        };
        let shifted = if bytes[c + 1] == b'+' {
            a.wrapping_shr(d)
        } else {
            a.wrapping_shl(d)
        };
        a = if bytes[c] == b'+' {
            a.wrapping_add(shifted)
        } else {
            a ^ shifted
        };
        c += 3;
    }
    a
}

/// Compute Google's `tk` token for `text` under `tkk` (e.g. "444000.1270171236").
/// Verbatim port of `sign()` in `google-translate-sign.js`, using u32 arithmetic.
fn google_tk(text: &str, tkk: &str) -> String {
    let parts: Vec<&str> = tkk.split('.').collect();
    let b: u32 = parts
        .first()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let second: u32 = parts
        .get(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let mut a: u32 = b;
    for &byte in text.as_bytes() {
        a = a.wrapping_add(byte as u32);
        a = google_xr(a, "+-a^+6");
    }
    a = google_xr(a, "+-3^+b+-f");
    a ^= second;
    a %= 1_000_000;
    format!("{}.{}", a, a ^ b)
}

fn capture_tkk(html: &str) -> Option<String> {
    let start = html.find("tkk:'")? + "tkk:'".len();
    let tail = &html[start..];
    let end = tail.find('\'')?;
    Some(tail[..end].to_string())
}

/// Fetch a fresh TKK from the webapp homepage HTML; falls back to the default.
async fn fetch_tkk(client: &Client, base_url: &str) -> String {
    let html = match client
        .get(base_url)
        .header("User-Agent", WEBAPP_USER_AGENT)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default(),
        _ => String::new(),
    };
    capture_tkk(&html).unwrap_or_else(|| GOOGLE_DEFAULT_TKK.to_string())
}

/// Build and send the webapp `translate_a/single` request, returning the raw
/// JSON array.
async fn fetch_webapp(
    req: &TranslateRequest,
    cfg: &ServiceConfig,
    client: &Client,
) -> ServiceResult<serde_json::Value> {
    let base_url = GoogleService::resolve_webapp_base_url(cfg);
    let tkk = fetch_tkk(client, &base_url).await;
    let tk = google_tk(&req.text, &tkk);
    let source = req
        .from
        .as_deref()
        .map(GoogleService::gtx_language)
        .unwrap_or_else(|| "auto".to_string());
    let target = GoogleService::gtx_language(&req.to);
    let query: Vec<(&str, String)> = vec![
        ("client", "webapp".to_string()),
        ("sl", source),
        ("tl", target),
        ("hl", "en".to_string()),
        ("otf", "2".to_string()),
        ("ssel", "3".to_string()),
        ("tsel", "0".to_string()),
        ("kc", "6".to_string()),
        ("dt", "at".to_string()),
        ("dt", "bd".to_string()),
        ("dt", "ex".to_string()),
        ("dt", "ld".to_string()),
        ("dt", "md".to_string()),
        ("dt", "qca".to_string()),
        ("dt", "rw".to_string()),
        ("dt", "rm".to_string()),
        ("dt", "ss".to_string()),
        ("dt", "t".to_string()),
        ("tk", tk),
        ("q", req.text.clone()),
    ];
    let resp = client
        .get(format!("{base_url}/translate_a/single"))
        .header("User-Agent", WEBAPP_USER_AGENT)
        .query(&query)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(map_google_error(
            status,
            resp.text().await.unwrap_or_default(),
        ));
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| ServiceError::Parse(format!("google webapp json: {e}")))
}

/// Parse the webapp response array. Returns (text, detected_language, dict).
fn parse_webapp(
    json: &serde_json::Value,
    from: Option<&str>,
    to: &str,
) -> Option<(String, Option<String>, Option<DictionaryResult>)> {
    let arr = json.as_array()?;
    // [2] = detected source language.
    let detected = arr.get(2).and_then(|v| v.as_str()).map(String::from);

    // [0] = translation segments; each sub-array's first element is the text.
    let mut text_parts = Vec::new();
    if let Some(segments) = arr.get(0).and_then(|v| v.as_array()) {
        for seg in segments {
            if let Some(t) = seg
                .as_array()
                .and_then(|s| s.first())
                .and_then(|f| f.as_str())
            {
                let t = t.trim();
                if !t.is_empty() {
                    text_parts.push(t.to_string());
                }
            }
        }
    }
    let text = text_parts.join("\n");

    // [1] = dictionary block. en->zh yields parts; zh->en yields simple_words.
    let from_eff: Option<&str> = from.or(detected.as_deref());
    let from_en = from_eff
        .map(|f| f.to_ascii_lowercase().starts_with("en"))
        .unwrap_or(false);
    let from_zh = from_eff
        .map(|f| f.to_ascii_lowercase().starts_with("zh"))
        .unwrap_or(false);
    let to_en = to.to_ascii_lowercase().starts_with("en");
    let to_zh = to.to_ascii_lowercase().starts_with("zh");

    let mut dict = DictionaryResult::default();
    if let Some(dict_block) = arr.get(1).and_then(|v| v.as_array()) {
        if from_en && to_zh {
            for obj in dict_block {
                let Some(o) = obj.as_array() else { continue };
                let Some(part) = o.first().and_then(|p| p.as_str()) else {
                    continue;
                };
                let means: Vec<String> = o
                    .get(1)
                    .and_then(|m| m.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                if !means.is_empty() {
                    dict.parts.push(DictionaryPart {
                        part: Some(part_abbreviation(part)),
                        means,
                    });
                }
            }
        } else if from_zh && to_en {
            for obj in dict_block {
                let Some(o) = obj.as_array() else { continue };
                let Some(part) = o.first().and_then(|p| p.as_str()) else {
                    continue;
                };
                let Some(part_words) = o.get(2).and_then(|p| p.as_array()) else {
                    continue;
                };
                for word_obj in part_words {
                    let Some(w) = word_obj.as_array() else {
                        continue;
                    };
                    let Some(word) = w.first().and_then(|x| x.as_str()) else {
                        continue;
                    };
                    let means: Vec<String> = w
                        .get(1)
                        .and_then(|m| m.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    dict.simple_words.push(SimpleDictionaryWord {
                        word: word.to_string(),
                        part: Some(part_abbreviation(part)),
                        means,
                    });
                }
            }
        }
    }
    let dict = if dict.is_empty() { None } else { Some(dict) };

    // Only consider the webapp result usable if it produced text or dict.
    if text.is_empty() && dict.is_none() {
        return None;
    }
    Some((text, detected, dict))
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
            _ => Self::translate_web_or_gtx(req, cfg, client).await,
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
            options: json!({ "projectId": TEST_PROJECT, "gtx_base_url": server.uri(), "webapp_base_url": server.uri() }),
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
            options: json!({ "gtx_base_url": server.uri(), "webapp_base_url": server.uri() }),
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

    // ---- W1: tk token matches the reference google-translate-sign.js ----
    #[test]
    fn google_tk_matches_reference_js() {
        let tkk = "444000.1270171236";
        assert_eq!(super::google_tk("good", tkk), "567648.945920");
        assert_eq!(super::google_tk("hello world", tkk), "389.444389");
        assert_eq!(super::google_tk("你好", tkk), "113934.490350");
        assert_eq!(super::google_tk("apple", tkk), "413840.38640");
        assert_eq!(super::google_tk("Test123", tkk), "633386.1010762");
        assert_eq!(super::google_tk("a", tkk), "160105.309001");
        assert_eq!(super::google_tk("", tkk), "19547.428603");
    }

    // ---- W2: en single-word -> zh uses the webapp dict path ----
    #[tokio::test]
    async fn translate_webapp_dict_for_english_word_to_chinese() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("tkk:'444000.1270171236',"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .and(query_param("client", "webapp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                [["好的", ["good", "ɡʊd", null, "ɡʊd"]]],
                [["adj.", ["好的", "优良的"]], ["n.", ["好处"]]],
                "en"
            ])))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = ServiceConfig {
            id: ServiceId::Google,
            enabled: true,
            priority: 0,
            options: json!({ "webapp_base_url": server.uri() }),
        };
        let req = TranslateRequest {
            text: "good".to_string(),
            from: Some("en".to_string()),
            to: "zh-CN".to_string(),
        };
        let res = GoogleService
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "好的");
        assert_eq!(res.detected_source.as_deref(), Some("en"));
        let dict = res.source_dictionary.expect("source dictionary");
        assert_eq!(dict.parts.len(), 2);
        assert_eq!(dict.parts[0].part.as_deref(), Some("adj."));
        assert_eq!(dict.parts[0].means, vec!["好的", "优良的"]);
        assert_eq!(dict.parts[1].part.as_deref(), Some("n."));
        assert_eq!(dict.parts[1].means, vec!["好处"]);
    }
}
