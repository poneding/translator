//! OpenAI-compatible chat completions translation service.
//!
//! Endpoint: `POST {base_url}/chat/completions` (e.g. OpenAI, DeepSeek, Zhipu, Ollama).
//! Auth: `Authorization: Bearer <api_key>`.
//!
//! Presets (see DESIGN.md §4.2.5) auto-fill `baseUrl` + `model` for known vendors.
//!
//! See DESIGN.md §4.2.5.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::{ServiceError, ServiceResult};
use crate::model::{ServiceId, TranslateRequest, TranslateResult};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

/// OpenAI-compatible service implementation.
pub struct OpenAIService;

impl OpenAIService {
    /// Apply a preset if specified; returns the (effective baseUrl, model) pair.
    /// Preset wins over `baseUrl`/`model` defaults but is overridden by explicit
    /// user-supplied `baseUrl`/`model` in options.
    fn resolve_endpoint(cfg: &ServiceConfig) -> ServiceResult<(String, String)> {
        let preset = cfg
            .options
            .get("preset")
            .and_then(|v| v.as_str())
            .unwrap_or("custom");

        let (preset_base, preset_model) = match preset {
            "openai" => ("https://api.openai.com/v1", "gpt-4o-mini"),
            "deepseek" => ("https://api.deepseek.com/v1", "deepseek-chat"),
            "zhipu" => ("https://open.bigmodel.cn/api/paas/v4", "glm-4-flash"),
            "ollama" => ("http://localhost:11434/v1", "llama3.2"),
            "openrouter" => ("https://openrouter.ai/api/v1", "openai/gpt-4o-mini"),
            _ => ("https://api.openai.com/v1", "gpt-4o-mini"),
        };

        let base = cfg
            .options
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| preset_base.to_string());
        let model = cfg
            .options
            .get("model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| preset_model.to_string());

        if base.is_empty() {
            return Err(ServiceError::MissingCredentials(
                "openai.baseUrl".to_string(),
            ));
        }
        if model.is_empty() {
            return Err(ServiceError::MissingCredentials("openai.model".to_string()));
        }
        Ok((base, model))
    }

    /// Build the user message that instructs the model to translate.
    fn build_user_prompt(req: &TranslateRequest) -> String {
        let from = req
            .from
            .as_deref()
            .unwrap_or("the source language (auto-detect)");
        let to = req.to.as_str();
        format!(
            "Translate the following text from {from} to {to}.\n\
             Output ONLY the translation, no commentary, no quotation marks.\n\
             If the source is already in the target language, repeat it unchanged.\n\
             \n\
             Text:\n{text}",
            from = from,
            to = to,
            text = req.text
        )
    }
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResp,
}

#[derive(Deserialize)]
struct ChatMessageResp {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiErrorBody {
    error: Option<ApiErrorDetail>,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    kind: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

#[async_trait]
impl TranslationService for OpenAIService {
    fn id(&self) -> ServiceId {
        ServiceId::OpenAI
    }

    fn display_name(&self) -> &'static str {
        "OpenAI Compatible"
    }

    fn api_key_requirement(&self) -> ApiKeyRequirement {
        ApiKeyRequirement::Required
    }

    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["baseUrl", "model"],
            "properties": {
                "baseUrl": {
                    "type": "string",
                    "title": "Base URL",
                    "default": "https://api.openai.com/v1"
                },
                "model": {
                    "type": "string",
                    "title": "Model",
                    "default": "gpt-4o-mini"
                },
                "preset": {
                    "type": "string",
                    "title": "Preset",
                    "enum": ["custom", "openai", "deepseek", "zhipu", "ollama", "openrouter"],
                    "default": "custom"
                },
                "temperature": {
                    "type": "number",
                    "title": "Temperature",
                    "default": 0.3,
                    "minimum": 0.0,
                    "maximum": 2.0
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
            api_key.ok_or_else(|| ServiceError::MissingCredentials("openai.apiKey".to_string()))?;
        let (base, model) = Self::resolve_endpoint(cfg)?;
        let started = Instant::now();
        let temperature = cfg
            .options
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .unwrap_or(0.3);

        let user_prompt = Self::build_user_prompt(req);
        let body = ChatCompletionRequest {
            model: &model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: "You are a translation engine.",
                },
                ChatMessage {
                    role: "user",
                    content: &user_prompt,
                },
            ],
            temperature,
        };

        let url = format!("{}/chat/completions", base.trim_end_matches('/'));

        let response = client
            .post(&url)
            .bearer_auth(key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

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
            let message = serde_json::from_str::<ApiErrorBody>(&body_text)
                .ok()
                .and_then(|b| b.error)
                .and_then(|e| e.message)
                .unwrap_or(body_text);
            return Err(ServiceError::Api {
                code: mapped.to_string(),
                message,
            });
        }

        let parsed: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("openai json: {e}")))?;

        let choice =
            parsed.choices.into_iter().next().ok_or_else(|| {
                ServiceError::Parse("openai: no choices[] in response".to_string())
            })?;
        let text = choice
            .message
            .content
            .ok_or_else(|| {
                ServiceError::Parse("openai: choices[0].message.content missing".to_string())
            })?
            .trim()
            .to_string();
        if text.is_empty() {
            return Err(ServiceError::Parse(
                "openai: empty translated text".to_string(),
            ));
        }

        let elapsed_ms = started.elapsed().as_millis() as u64;
        Ok(TranslateResult {
            service_id: ServiceId::OpenAI,
            service_name: "OpenAI Compatible".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            text,
            audio_url: None,
            detected_source: None,
            elapsed_ms,
            dictionary: None,
            source_dictionary: None,
            target_dictionary: None,
            extra: None,
            alternatives: Vec::new(),
        })
    }
}

// =============================================================================
// Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::TranslationService;
    use crate::error::ServiceError;
    use crate::model::{ServiceId, TranslateRequest};
    use crate::service::ServiceConfig;

    use super::OpenAIService;

    const TEST_KEY: &str = "sk-test-key";

    fn cfg_for(mock: &MockServer) -> ServiceConfig {
        ServiceConfig {
            id: ServiceId::OpenAI,
            enabled: true,
            priority: 0,
            options: json!({
                "baseUrl": mock.uri(),
                "model": "gpt-4o-mini"
            }),
        }
    }

    fn ok_response(content: &str) -> serde_json::Value {
        json!({
            "id": "chatcmpl-xxx",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": content },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
        })
    }

    // ---- S1: happy path ----
    #[tokio::test]
    async fn translate_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", format!("Bearer {TEST_KEY}")))
            .and(body_string_contains("\"model\":\"gpt-4o-mini\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("你好")))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: Some("en".to_string()),
            to: "zh-CN".to_string(),
        };
        let res = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "你好");
        assert_eq!(res.service_id, ServiceId::OpenAI);
    }

    // ---- S2: trimmed whitespace from response ----
    #[tokio::test]
    async fn translate_trims_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ok_response("  Bonjour le monde  \n")),
            )
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hello".to_string(),
            from: None,
            to: "fr".to_string(),
        };
        let res = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "Bonjour le monde");
    }

    // ---- S3: missing API key ----
    #[tokio::test]
    async fn translate_missing_api_key() {
        let cfg = ServiceConfig {
            id: ServiceId::OpenAI,
            enabled: true,
            priority: 0,
            options: json!({ "baseUrl": "http://x", "model": "gpt-4o-mini" }),
        };
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::MissingCredentials(ref s) if s.contains("apiKey")));
    }

    // ---- S4: empty baseUrl falls back to preset default (presets always non-empty) ----
    #[tokio::test]
    async fn translate_empty_base_url_falls_back_to_preset() {
        let server = MockServer::start().await;
        // We point the openai preset at our mock by overriding baseUrl. But here
        // we pass baseUrl="" and a valid model. Preset "openai" default = "https://api.openai.com/v1".
        // That URL won't resolve in a test, so we expect a network-ish error, NOT MissingCredentials.
        // This documents: empty baseUrl = use preset default, not an error.
        let cfg = ServiceConfig {
            id: ServiceId::OpenAI,
            enabled: true,
            priority: 0,
            options: json!({ "baseUrl": "", "model": "gpt-4o-mini", "preset": "openai" }),
        };
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        // We don't need the mock to actually receive anything; we just want to confirm
        // the MissingCredentials branch is NOT taken when baseUrl is empty.
        let _ = server; // silence unused
        let result = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await;
        if let Err(ServiceError::MissingCredentials(s)) = result {
            panic!("empty baseUrl should fall back to preset, got MissingCredentials({s})")
        }
        // Any other outcome (network error, parse, etc.) is acceptable here.
    }

    // ---- S6: preset=openai uses correct baseUrl when option absent ----
    #[test]
    fn resolve_endpoint_preset_openai() {
        let cfg = ServiceConfig {
            id: ServiceId::OpenAI,
            enabled: true,
            priority: 0,
            options: json!({ "preset": "openai" }),
        };
        let (base, model) = OpenAIService::resolve_endpoint(&cfg).unwrap();
        assert_eq!(base, "https://api.openai.com/v1");
        assert_eq!(model, "gpt-4o-mini");
    }

    // ---- S7: preset=deepseek uses correct baseUrl + model ----
    #[test]
    fn resolve_endpoint_preset_deepseek() {
        let cfg = ServiceConfig {
            id: ServiceId::OpenAI,
            enabled: true,
            priority: 0,
            options: json!({ "preset": "deepseek" }),
        };
        let (base, model) = OpenAIService::resolve_endpoint(&cfg).unwrap();
        assert_eq!(base, "https://api.deepseek.com/v1");
        assert_eq!(model, "deepseek-chat");
    }

    // ---- S8: explicit model overrides preset model ----
    #[test]
    fn resolve_endpoint_explicit_model_wins() {
        let cfg = ServiceConfig {
            id: ServiceId::OpenAI,
            enabled: true,
            priority: 0,
            options: json!({ "preset": "deepseek", "model": "deepseek-reasoner" }),
        };
        let (_base, model) = OpenAIService::resolve_endpoint(&cfg).unwrap();
        assert_eq!(model, "deepseek-reasoner");
    }

    // ---- S9: 401 -> invalid_credentials ----
    #[tokio::test]
    async fn translate_401_invalid_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": { "message": "Incorrect API key provided", "type": "invalid_request_error", "code": "invalid_api_key" }
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, message } => {
                assert_eq!(code, "invalid_credentials");
                assert!(message.contains("Incorrect API key"));
            }
            other => panic!("expected invalid_credentials, got: {other:?}"),
        }
    }

    // ---- S10: 429 -> rate_limited ----
    #[tokio::test]
    async fn translate_429_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_string("rate limit"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "rate_limited"),
            other => panic!("expected rate_limited, got: {other:?}"),
        }
    }

    // ---- S11: 5xx -> upstream ----
    #[tokio::test]
    async fn translate_500_upstream() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        match err {
            ServiceError::Api { code, .. } => assert_eq!(code, "upstream"),
            other => panic!("expected upstream, got: {other:?}"),
        }
    }

    // ---- S12: empty choices[] -> Parse ----
    #[tokio::test]
    async fn translate_empty_choices() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "choices": [] })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }

    // ---- S13: content=null -> Parse ----
    #[tokio::test]
    async fn translate_null_content() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{ "message": { "role": "assistant", "content": null } }]
            })))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }

    // ---- S14: empty content string -> Parse ----
    #[tokio::test]
    async fn translate_empty_content_string() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_response("   ")))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }

    // ---- S15: malformed JSON -> Parse ----
    #[tokio::test]
    async fn translate_malformed_json() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "Hi".to_string(),
            from: None,
            to: "zh-CN".to_string(),
        };
        let err = OpenAIService
            .translate(&req, &cfg, Some(TEST_KEY), &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }
}
