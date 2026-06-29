//! Youdao (有道) translation service.
//!
//! Uses the official OpenAPI when `appKey` and `appSecret` are configured.
//! Otherwise it falls back to the web endpoint used by Easydict's built-in
//! Youdao service.
//!
//! See DESIGN.md §4.2.1 for the request/response schema.

use aes::Aes128;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use cbc::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::audio::youdao_text_audio_url;
use crate::error::{ServiceError, ServiceResult};
use crate::model::{
    DictionaryPart, DictionaryResult, ServiceId, SimpleDictionaryWord, TranslateRequest,
    TranslateResult, WordExchange, WordPhonetic, part_abbreviation,
};
use crate::service::{ApiKeyRequirement, ServiceConfig, TranslationService};

const DEFAULT_BASE: &str = "https://openapi.youdao.com";
const DEFAULT_WEB_BASE: &str = "https://dict.youdao.com";
const WEB_REFERER: &str = "https://fanyi.youdao.com";
const WEB_COOKIE: &str = "OUTFOX_SEARCH_USER_ID=1796239350@10.110.96.157;";
const WEB_CLIENT: &str = "fanyideskweb";
const WEB_PRODUCT: &str = "webfanyi";
const WEB_DEFAULT_KEY: &str = "asdjnjfenknafdfsdfsd";

type Aes128CbcDec = cbc::Decryptor<Aes128>;

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

    /// Resolve the Youdao web base URL used by the no-credential fallback.
    fn resolve_web_base_url(cfg: &ServiceConfig) -> String {
        cfg.options
            .get("web_base_url")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_WEB_BASE.to_string())
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
        hex_lower(digest.as_slice())
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
                };
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

    /// Compute the web endpoint MD5 signature.
    fn web_sign(timestamp: &str, key: &str) -> String {
        let raw =
            format!("client={WEB_CLIENT}&mysticTime={timestamp}&product={WEB_PRODUCT}&key={key}");
        format!("{:x}", md5::compute(raw))
    }

    /// Convert app language ids into Youdao web language ids.
    fn youdao_language(code: &str) -> String {
        match code.to_ascii_lowercase().as_str() {
            "zh-hans" | "zh-cn" => "zh-CHS".to_string(),
            "zh-hant" | "zh-tw" | "zh-hk" => "zh-CHT".to_string(),
            "" => "auto".to_string(),
            other => other.to_string(),
        }
    }

    async fn translate_official(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        app_key: String,
        app_secret: String,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let base_url = Self::resolve_base_url(cfg);

        let salt = Uuid::new_v4().simple().to_string();
        let curtime = now_unix();
        let sign = Self::sign_v3(&app_key, &req.text, &salt, curtime, &app_secret);
        let from = req
            .from
            .as_deref()
            .map(Self::youdao_language)
            .unwrap_or_else(|| "auto".to_string());
        let to = Self::youdao_language(&req.to);

        let form: Vec<(&str, String)> = vec![
            ("q", req.text.clone()),
            ("from", from),
            ("to", to),
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
        let text = parsed
            .translation
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        if text.is_empty() {
            return Err(ServiceError::Parse(
                "youdao: no translation in response".to_string(),
            ));
        }
        let web_base_url = Self::resolve_web_base_url(cfg);
        let source_dictionary = dictionary_from_official(&parsed, &web_base_url);
        let target_dictionary =
            Self::target_dictionary_for_translation(&text, &req.to, cfg, client).await;
        let detected_source = parsed.l.as_deref().and_then(detected_source_from_kind);
        let audio_url = youdao_text_audio_url(&text, Some(&req.to), "us", &web_base_url);

        Ok(TranslateResult {
            service_id: ServiceId::Youdao,
            service_name: "Youdao".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            text,
            audio_url,
            detected_source,
            elapsed_ms: started.elapsed().as_millis() as u64,
            dictionary: source_dictionary.clone(),
            source_dictionary,
            target_dictionary,
            extra: None,
            alternatives: Vec::new(),
        })
    }

    async fn translate_web(
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> ServiceResult<TranslateResult> {
        let started = Instant::now();
        let base_url = Self::resolve_web_base_url(cfg);
        let key = Self::fetch_web_key(&base_url, client).await?;
        let timestamp = now_ms().to_string();
        let sign = Self::web_sign(&timestamp, &key.data.secret_key);
        let from = req
            .from
            .as_deref()
            .map(Self::youdao_language)
            .unwrap_or_else(|| "auto".to_string());
        let to = Self::youdao_language(&req.to);
        let form = [
            ("client", WEB_CLIENT),
            ("product", WEB_PRODUCT),
            ("appVersion", "1.0.0"),
            ("vendor", "web"),
            ("pointParam", "client,mysticTime,product"),
            ("keyfrom", "fanyi.web"),
            ("i", req.text.as_str()),
            ("from", from.as_str()),
            ("to", to.as_str()),
            ("dictResult", "true"),
            ("keyid", "webfanyi"),
            ("sign", sign.as_str()),
            ("mysticTime", timestamp.as_str()),
        ];

        let response = client
            .post(format!("{base_url}/webtranslate"))
            .header("Referer", WEB_REFERER)
            .header("Cookie", WEB_COOKIE)
            .form(&form)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
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

        let decrypted = decrypt_web_payload(&body, &key.data.aes_key, &key.data.aes_iv)?;
        let parsed: YoudaoWebResponse = serde_json::from_str(&decrypted)
            .map_err(|e| ServiceError::Parse(format!("youdao web json: {e}")))?;
        if parsed.code != 0 {
            return Err(ServiceError::Api {
                code: "api".to_string(),
                message: format!("youdao web code={}", parsed.code),
            });
        }
        let text = parsed
            .translate_result
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|item| item.tgt.as_str())
                    .collect::<String>()
            })
            .collect::<String>()
            .trim()
            .to_string();
        if text.is_empty() {
            return Err(ServiceError::Parse(
                "youdao web: empty translated text".to_string(),
            ));
        }

        let detected_source = parsed.kind.as_deref().and_then(detected_source_from_kind);
        let source_dictionary = parsed
            .dict_result
            .as_ref()
            .and_then(|dict| dictionary_from_web_dict(dict, &base_url));
        let target_dictionary =
            Self::target_dictionary_for_translation(&text, &req.to, cfg, client).await;
        let audio_url = youdao_text_audio_url(&text, Some(&req.to), "us", &base_url);

        Ok(TranslateResult {
            service_id: ServiceId::Youdao,
            service_name: "Youdao".to_string(),
            from: req.from.clone(),
            to: req.to.clone(),
            text,
            audio_url,
            detected_source,
            elapsed_ms: started.elapsed().as_millis() as u64,
            dictionary: source_dictionary.clone(),
            source_dictionary,
            target_dictionary,
            extra: None,
            alternatives: Vec::new(),
        })
    }

    async fn target_dictionary_for_translation(
        text: &str,
        target_language: &str,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> Option<DictionaryResult> {
        if !should_lookup_target_dictionary(text, target_language) {
            return None;
        }
        match Self::lookup_web_dictionary(text, "en", "zh-CHS", cfg, client).await {
            Ok(dictionary) => dictionary,
            Err(error) => {
                tracing::warn!(error = %error, text = %text, "youdao target dictionary lookup failed");
                None
            }
        }
    }

    async fn lookup_web_dictionary(
        text: &str,
        from: &str,
        to: &str,
        cfg: &ServiceConfig,
        client: &Client,
    ) -> ServiceResult<Option<DictionaryResult>> {
        let base_url = Self::resolve_web_base_url(cfg);
        let key = Self::fetch_web_key(&base_url, client).await?;
        let timestamp = now_ms().to_string();
        let sign = Self::web_sign(&timestamp, &key.data.secret_key);
        let form = [
            ("client", WEB_CLIENT),
            ("product", WEB_PRODUCT),
            ("appVersion", "1.0.0"),
            ("vendor", "web"),
            ("pointParam", "client,mysticTime,product"),
            ("keyfrom", "fanyi.web"),
            ("i", text),
            ("from", from),
            ("to", to),
            ("dictResult", "true"),
            ("keyid", "webfanyi"),
            ("sign", sign.as_str()),
            ("mysticTime", timestamp.as_str()),
        ];

        let response = client
            .post(format!("{base_url}/webtranslate"))
            .header("Referer", WEB_REFERER)
            .header("Cookie", WEB_COOKIE)
            .form(&form)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
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

        let decrypted = decrypt_web_payload(&body, &key.data.aes_key, &key.data.aes_iv)?;
        let parsed: YoudaoWebResponse = serde_json::from_str(&decrypted)
            .map_err(|e| ServiceError::Parse(format!("youdao target dict json: {e}")))?;
        if parsed.code != 0 {
            return Err(ServiceError::Api {
                code: "api".to_string(),
                message: format!("youdao target dict code={}", parsed.code),
            });
        }
        Ok(parsed
            .dict_result
            .as_ref()
            .and_then(|dict| dictionary_from_web_dict(dict, &base_url)))
    }

    async fn fetch_web_key(base_url: &str, client: &Client) -> ServiceResult<YoudaoWebKey> {
        let timestamp = now_ms().to_string();
        let sign = Self::web_sign(&timestamp, WEB_DEFAULT_KEY);
        let query = [
            ("client", WEB_CLIENT),
            ("product", WEB_PRODUCT),
            ("appVersion", "1.0.0"),
            ("vendor", "web"),
            ("pointParam", "client,mysticTime,product"),
            ("keyfrom", "fanyi.web"),
            ("keyid", "webfanyi-key-getter"),
            ("sign", sign.as_str()),
            ("mysticTime", timestamp.as_str()),
        ];

        let response = client
            .get(format!("{base_url}/webtranslate/key"))
            .header("Referer", WEB_REFERER)
            .header("Cookie", WEB_COOKIE)
            .query(&query)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(ServiceError::Api {
                code: "upstream".to_string(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let key: YoudaoWebKey = response
            .json()
            .await
            .map_err(|e| ServiceError::Parse(format!("youdao web key json: {e}")))?;
        if key.code != 0 {
            return Err(ServiceError::Api {
                code: "api".to_string(),
                message: key.msg,
            });
        }
        Ok(key)
    }
}

fn read_youdao_creds(cfg: &ServiceConfig) -> Option<(String, String)> {
    let app_key = cfg
        .options
        .get("appKey")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let app_secret = cfg
        .options
        .get("appSecret")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    Some((app_key, app_secret))
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decrypt_web_payload(encrypted_text: &str, key: &str, iv: &str) -> ServiceResult<String> {
    let mut encoded = encrypted_text.trim().replace('-', "+").replace('_', "/");
    while !encoded.len().is_multiple_of(4) {
        encoded.push('=');
    }
    let encrypted = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| ServiceError::Parse(format!("youdao web base64: {e}")))?;
    let key_hash = md5::compute(key.as_bytes());
    let iv_hash = md5::compute(iv.as_bytes());
    let decrypted = Aes128CbcDec::new_from_slices(&key_hash.0, &iv_hash.0)
        .map_err(|e| ServiceError::Parse(format!("youdao web aes init: {e}")))?
        .decrypt_padded_vec::<Pkcs7>(&encrypted)
        .map_err(|e| ServiceError::Parse(format!("youdao web aes decrypt: {e}")))?;
    String::from_utf8(decrypted).map_err(|e| ServiceError::Parse(format!("youdao web utf8: {e}")))
}

fn dictionary_from_official(
    response: &YoudaoResponse,
    web_base_url: &str,
) -> Option<DictionaryResult> {
    let mut dictionary = DictionaryResult::default();

    if let Some(basic) = &response.basic {
        if let Some(phonetic) = basic.us_phonetic.as_deref().or(basic.phonetic.as_deref()) {
            dictionary.phonetics.push(WordPhonetic {
                label: "US".to_string(),
                value: Some(phonetic.to_string()),
                audio_url: response
                    .speak_url
                    .clone()
                    .or_else(|| source_audio_url(&response.query, Some("en"), web_base_url)),
                accent: Some("us".to_string()),
            });
        }

        if let Some(phonetic) = basic.uk_phonetic.as_deref() {
            dictionary.phonetics.push(WordPhonetic {
                label: "UK".to_string(),
                value: Some(phonetic.to_string()),
                audio_url: source_audio_url_with_accent(
                    &response.query,
                    Some("en"),
                    "uk",
                    web_base_url,
                ),
                accent: Some("uk".to_string()),
            });
        }

        for explain in &basic.explains {
            if let Some(part) = parse_part_explain(explain) {
                dictionary.parts.push(part);
            }
        }

        dictionary
            .tags
            .extend(basic.exam_type.iter().filter_map(clean_optional));
    }

    append_official_web_entries(&mut dictionary, &response.web);
    non_empty_dictionary(dictionary)
}

fn dictionary_from_web_dict(
    dict: &YoudaoWebDictResult,
    web_base_url: &str,
) -> Option<DictionaryResult> {
    let mut dictionary = DictionaryResult::default();

    if let Some(ec) = &dict.ec {
        append_ec_dictionary(
            &mut dictionary,
            &ec.word,
            ec.exam_type.clone(),
            web_base_url,
        );
    }

    if let Some(ce) = &dict.ce {
        append_ce_dictionary(&mut dictionary, &ce.word, web_base_url);
    }

    if let Some(web_trans) = &dict.web_trans {
        append_web_translation_entries(&mut dictionary, &web_trans.web_translation);
    }

    non_empty_dictionary(dictionary)
}

fn append_ec_dictionary(
    dictionary: &mut DictionaryResult,
    word: &Option<YoudaoEcWord>,
    tags: Vec<String>,
    web_base_url: &str,
) {
    let Some(word) = word else {
        return;
    };

    if let Some(usphone) = clean_optional(&word.usphone) {
        dictionary.phonetics.push(WordPhonetic {
            label: "US".to_string(),
            value: Some(usphone),
            audio_url: word
                .usspeech
                .as_deref()
                .and_then(|speech| dict_voice_url_from_speech(speech, web_base_url)),
            accent: Some("us".to_string()),
        });
    }

    if let Some(ukphone) = clean_optional(&word.ukphone) {
        dictionary.phonetics.push(WordPhonetic {
            label: "UK".to_string(),
            value: Some(ukphone),
            audio_url: word
                .ukspeech
                .as_deref()
                .and_then(|speech| dict_voice_url_from_speech(speech, web_base_url)),
            accent: Some("uk".to_string()),
        });
    }

    for item in &word.trs {
        let Some(mean) = clean_optional(&item.tran) else {
            continue;
        };
        dictionary.parts.push(DictionaryPart {
            part: clean_optional(&item.pos).as_deref().map(part_abbreviation),
            means: vec![mean],
        });
    }

    for item in &word.wfs {
        let Some(wf) = &item.wf else {
            continue;
        };
        let Some(name) = clean_optional(&wf.name) else {
            continue;
        };
        let words = wf
            .value
            .as_deref()
            .unwrap_or_default()
            .split('或')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !words.is_empty() {
            dictionary.exchanges.push(WordExchange { name, words });
        }
    }

    dictionary
        .tags
        .extend(tags.into_iter().filter(|tag| !tag.trim().is_empty()));
}

fn append_ce_dictionary(
    dictionary: &mut DictionaryResult,
    word: &Option<YoudaoCeWord>,
    web_base_url: &str,
) {
    let Some(word) = word else {
        return;
    };

    if let Some(phone) = clean_optional(&word.phone) {
        dictionary.phonetics.push(WordPhonetic {
            label: "Pinyin".to_string(),
            value: Some(phone),
            audio_url: word
                .return_phrase
                .as_deref()
                .and_then(|text| source_audio_url(text, Some("zh-CHS"), web_base_url)),
            accent: None,
        });
    }

    for item in &word.trs {
        let Some(entry) = clean_optional(&item.text) else {
            continue;
        };
        let mut means = Vec::new();
        if let Some(mean) = clean_optional(&item.tran) {
            means.push(mean);
        }
        dictionary.simple_words.push(SimpleDictionaryWord {
            word: entry,
            part: None,
            means,
        });
    }
}

fn append_official_web_entries(dictionary: &mut DictionaryResult, entries: &[YoudaoOfficialWeb]) {
    for entry in entries {
        let Some(word) = clean_optional(&entry.key) else {
            continue;
        };
        let means = entry
            .values
            .iter()
            .filter_map(clean_optional)
            .collect::<Vec<_>>();
        if !means.is_empty() {
            dictionary.simple_words.push(SimpleDictionaryWord {
                word,
                part: Some("Web".to_string()),
                means,
            });
        }
    }
}

fn append_web_translation_entries(
    dictionary: &mut DictionaryResult,
    entries: &[YoudaoWebTranslation],
) {
    for entry in entries {
        let Some(word) = clean_optional(&entry.key) else {
            continue;
        };
        let means = entry
            .trans
            .iter()
            .filter_map(|item| clean_optional(&item.value))
            .collect::<Vec<_>>();
        if !means.is_empty() {
            dictionary.simple_words.push(SimpleDictionaryWord {
                word,
                part: Some("Web".to_string()),
                means,
            });
        }
    }
}

fn parse_part_explain(explain: &str) -> Option<DictionaryPart> {
    let explain = explain.trim();
    if explain.is_empty() {
        return None;
    }

    if let Some((part, mean)) = explain.split_once('.') {
        let part = part.trim();
        let mean = mean.trim();
        if !part.is_empty() && part.chars().count() <= 8 && !mean.is_empty() {
            return Some(DictionaryPart {
                part: Some(part_abbreviation(&format!("{part}."))),
                means: vec![mean.to_string()],
            });
        }
    }

    Some(DictionaryPart {
        part: None,
        means: vec![explain.to_string()],
    })
}

fn non_empty_dictionary(dictionary: DictionaryResult) -> Option<DictionaryResult> {
    if dictionary.is_empty() {
        None
    } else {
        Some(dictionary)
    }
}

fn clean_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn should_lookup_target_dictionary(text: &str, target_language: &str) -> bool {
    let target = target_language
        .trim()
        .replace('_', "-")
        .to_ascii_lowercase();
    if target != "en" && !target.starts_with("en-") {
        return false;
    }
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 80 || text.contains('\n') {
        return false;
    }
    let has_letter = text.chars().any(|ch| ch.is_ascii_alphabetic());
    let lookup_like = text.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || ch.is_ascii_whitespace()
            || matches!(ch, '-' | '\'' | '/' | '.' | '&')
    });
    has_letter && lookup_like
}

fn detected_source_from_kind(kind: &str) -> Option<String> {
    kind.split('2')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "auto")
        .map(str::to_string)
}

fn source_audio_url(text: &str, language: Option<&str>, web_base_url: &str) -> Option<String> {
    source_audio_url_with_accent(text, language, "us", web_base_url)
}

fn source_audio_url_with_accent(
    text: &str,
    language: Option<&str>,
    accent: &str,
    web_base_url: &str,
) -> Option<String> {
    youdao_text_audio_url(text, language, accent, web_base_url)
}

fn dict_voice_url_from_speech(speech: &str, web_base_url: &str) -> Option<String> {
    let speech = speech.trim();
    if speech.is_empty() {
        return None;
    }
    let mut url = Url::parse(&format!("{}/dictvoice", web_base_url.trim_end_matches('/'))).ok()?;
    for (index, pair) in speech.split('&').enumerate() {
        let Some((key, value)) = pair.split_once('=') else {
            if index == 0 {
                url.query_pairs_mut().append_pair("audio", pair);
            }
            continue;
        };
        if index == 0 && key != "audio" {
            url.query_pairs_mut().append_pair("audio", key);
            url.query_pairs_mut().append_pair(value, "");
        } else {
            url.query_pairs_mut().append_pair(key, value);
        }
    }
    Some(url.to_string())
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
        ApiKeyRequirement::None
    }

    fn options_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "appKey":    { "type": "string", "title": "App Key" },
                "appSecret": { "type": "string", "title": "App Secret", "format": "password" },
                "base_url":  { "type": "string", "title": "OpenAPI Base URL (override)" },
                "web_base_url":  { "type": "string", "title": "Web Base URL (override)" }
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
        if let Some((app_key, app_secret)) = read_youdao_creds(cfg) {
            Self::translate_official(req, cfg, app_key, app_secret, client).await
        } else {
            Self::translate_web(req, cfg, client).await
        }
    }
}

#[derive(Debug, Deserialize)]
struct YoudaoResponse {
    #[serde(rename = "errorCode")]
    error_code: String,
    #[serde(default)]
    translation: Vec<String>,
    #[serde(default)]
    query: String,
    #[serde(default)]
    l: Option<String>,
    #[serde(default, rename = "speakUrl")]
    speak_url: Option<String>,
    #[serde(default)]
    basic: Option<YoudaoOfficialBasic>,
    #[serde(default)]
    web: Vec<YoudaoOfficialWeb>,
}

#[derive(Debug, Deserialize)]
struct YoudaoOfficialBasic {
    #[serde(default)]
    phonetic: Option<String>,
    #[serde(default, rename = "us-phonetic")]
    us_phonetic: Option<String>,
    #[serde(default, rename = "uk-phonetic")]
    uk_phonetic: Option<String>,
    #[serde(default)]
    explains: Vec<String>,
    #[serde(default, rename = "exam_type")]
    exam_type: Vec<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct YoudaoOfficialWeb {
    #[serde(default)]
    key: Option<String>,
    #[serde(default, rename = "value")]
    values: Vec<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebKey {
    data: YoudaoWebKeyData,
    code: i32,
    msg: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YoudaoWebKeyData {
    secret_key: String,
    aes_key: String,
    aes_iv: String,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebResponse {
    code: i32,
    #[serde(rename = "translateResult", default)]
    translate_result: Vec<Vec<YoudaoWebItem>>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(rename = "dictResult", default)]
    dict_result: Option<YoudaoWebDictResult>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebItem {
    tgt: String,
    #[allow(dead_code)]
    src: String,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebDictResult {
    #[serde(default)]
    ec: Option<YoudaoWebEc>,
    #[serde(default)]
    ce: Option<YoudaoWebCe>,
    #[serde(default, rename = "web_trans")]
    web_trans: Option<YoudaoWebTrans>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebEc {
    #[serde(default)]
    word: Option<YoudaoEcWord>,
    #[serde(default, rename = "exam_type")]
    exam_type: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct YoudaoEcWord {
    #[serde(default)]
    usphone: Option<String>,
    #[serde(default)]
    ukphone: Option<String>,
    #[serde(default)]
    usspeech: Option<String>,
    #[serde(default)]
    ukspeech: Option<String>,
    #[serde(default)]
    trs: Vec<YoudaoEcTran>,
    #[serde(default)]
    wfs: Vec<YoudaoWfItem>,
}

#[derive(Debug, Deserialize)]
struct YoudaoEcTran {
    #[serde(default)]
    pos: Option<String>,
    #[serde(default)]
    tran: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWfItem {
    #[serde(default)]
    wf: Option<YoudaoWf>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWf {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebCe {
    #[serde(default)]
    word: Option<YoudaoCeWord>,
}

#[derive(Debug, Deserialize)]
struct YoudaoCeWord {
    #[serde(default)]
    phone: Option<String>,
    #[serde(default, rename = "return-phrase")]
    return_phrase: Option<String>,
    #[serde(default)]
    trs: Vec<YoudaoCeTran>,
}

#[derive(Debug, Deserialize)]
struct YoudaoCeTran {
    #[serde(default, rename = "#text")]
    text: Option<String>,
    #[serde(default, rename = "#tran")]
    tran: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebTrans {
    #[serde(default, rename = "web-translation")]
    web_translation: Vec<YoudaoWebTranslation>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebTranslation {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    trans: Vec<YoudaoWebTranslationItem>,
}

#[derive(Debug, Deserialize)]
struct YoudaoWebTranslationItem {
    #[serde(default)]
    value: Option<String>,
}

// =============================================================================
// Tests — TDD: written before the impl (PLAN.md M2.1 + M2.2).
// =============================================================================
#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose};
    use cbc::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::TranslationService;
    use crate::error::ServiceError;
    use crate::model::{ServiceId, TranslateRequest};
    use crate::service::ServiceConfig;

    use super::YoudaoService;

    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

    /// Test fixture creds.
    const TEST_KEY: &str = "test-app-key";
    const TEST_SECRET: &str = "test-app-secret";
    const WEB_SECRET: &str = "web-secret";
    const WEB_AES_KEY: &str = "web-aes-key";
    const WEB_AES_IV: &str = "web-aes-iv";

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
            "basic": {
                "us-phonetic": "həˈloʊ",
                "uk-phonetic": "həˈləʊ",
                "explains": ["int. 你好", "n. 招呼"],
                "exam_type": ["CET4"]
            },
            "web": [{
                "key": "Hello",
                "value": ["你好", "哈啰"]
            }],
            "query": "Hello",
            "l": "en2zh-CHS",
            "speakUrl": "https://dict.youdao.com/dictvoice?audio=Hello&type=2"
        })
    }

    fn web_key_body() -> serde_json::Value {
        json!({
            "code": 0,
            "msg": "OK",
            "data": {
                "secretKey": WEB_SECRET,
                "aesKey": WEB_AES_KEY,
                "aesIv": WEB_AES_IV,
            }
        })
    }

    fn encrypted_web_body(translation: &str) -> String {
        let plain = json!({
            "code": 0,
            "type": "en2zh-CHS",
            "dictResult": {
                "ec": {
                    "exam_type": ["CET4"],
                    "word": {
                        "usphone": "həˈloʊ",
                        "ukphone": "həˈləʊ",
                        "usspeech": "Hello&type=2",
                        "ukspeech": "Hello&type=1",
                        "trs": [
                            { "pos": "int", "tran": "你好" },
                            { "pos": "n", "tran": "招呼" }
                        ],
                        "wfs": [{ "wf": { "name": "复数", "value": "hellos" } }]
                    }
                },
                "web_trans": {
                    "web-translation": [{
                        "key": "Hello",
                        "trans": [{ "value": "你好" }, { "value": "哈啰" }]
                    }]
                }
            },
            "translateResult": [[{
                "src": "Hello",
                "tgt": translation,
                "srcPronounce": null,
                "tgtPronounce": null,
            }]]
        })
        .to_string();
        let key_hash = md5::compute(WEB_AES_KEY.as_bytes());
        let iv_hash = md5::compute(WEB_AES_IV.as_bytes());
        let encrypted = Aes128CbcEnc::new_from_slices(&key_hash.0, &iv_hash.0)
            .unwrap()
            .encrypt_padded_vec::<Pkcs7>(plain.as_bytes());
        general_purpose::STANDARD.encode(encrypted)
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
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .expect("translate should succeed");
        assert_eq!(result.text, "你好");
        assert_eq!(result.service_id, ServiceId::Youdao);
        let result_audio = result.audio_url.as_deref().expect("result audio");
        assert!(result_audio.contains("/dictvoice?"));
        assert!(result_audio.contains("audio=%E4%BD%A0%E5%A5%BD"));
        assert!(result_audio.contains("le=zh"));
        assert!(result.source_dictionary.is_some());
        assert!(result.target_dictionary.is_none());
        let dictionary = result.dictionary.expect("dictionary should be parsed");
        assert_eq!(dictionary.phonetics.len(), 2);
        assert!(
            dictionary.phonetics[0]
                .audio_url
                .as_deref()
                .unwrap()
                .contains("audio=Hello")
        );
        assert_eq!(dictionary.parts[0].part.as_deref(), Some("int."));
        assert_eq!(dictionary.parts[0].means, vec!["你好"]);
        assert_eq!(dictionary.simple_words[0].word, "Hello");
        assert_eq!(dictionary.simple_words[0].means, vec!["你好", "哈啰"]);
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

    // ---- S3: missing appKey -> Youdao web fallback ----
    #[tokio::test]
    async fn translate_missing_appkey_uses_web_fallback() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/webtranslate/key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(web_key_body()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/webtranslate"))
            .and(body_string_contains("keyid=webfanyi"))
            .respond_with(ResponseTemplate::new(200).set_body_string(encrypted_web_body("你好")))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = ServiceConfig {
            id: ServiceId::Youdao,
            enabled: true,
            priority: 0,
            options: json!({ "appSecret": "x", "web_base_url": server.uri() }),
        };
        let req = TranslateRequest::auto("Hello", "zh-CHS");
        let result = YoudaoService
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .expect("fallback translate should succeed");
        assert_eq!(result.text, "你好");
        assert_eq!(result.detected_source.as_deref(), Some("en"));
        let result_audio = result.audio_url.as_deref().expect("result audio");
        assert!(result_audio.contains("/dictvoice?"));
        assert!(result_audio.contains("audio=%E4%BD%A0%E5%A5%BD"));
        assert!(result_audio.contains("le=zh"));
        assert!(result.source_dictionary.is_some());
        assert!(result.target_dictionary.is_none());
        let dictionary = result.dictionary.expect("dictionary should be parsed");
        assert_eq!(dictionary.phonetics.len(), 2);
        assert!(
            dictionary.phonetics[0]
                .audio_url
                .as_deref()
                .unwrap()
                .contains("audio=Hello")
        );
        assert_eq!(dictionary.parts[0].part.as_deref(), Some("int."));
        assert_eq!(dictionary.parts[0].means, vec!["你好"]);
        assert_eq!(dictionary.simple_words[0].word, "Hello");
        assert_eq!(dictionary.simple_words[0].means, vec!["你好", "哈啰"]);
        assert_eq!(dictionary.exchanges[0].words, vec!["hellos"]);
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
            .translate(&req, &cfg, None, &crate::http::test_client())
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
            .translate(&req, &cfg, None, &crate::http::test_client())
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
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::Parse(_)));
    }

    #[test]
    fn target_dictionary_lookup_only_for_short_english_targets() {
        assert!(super::should_lookup_target_dictionary("log", "en"));
        assert!(super::should_lookup_target_dictionary("event log", "en-US"));
        assert!(!super::should_lookup_target_dictionary("日志", "en"));
        assert!(!super::should_lookup_target_dictionary("log", "zh-Hans"));
        assert!(!super::should_lookup_target_dictionary(
            &"a".repeat(81),
            "en"
        ));
        assert!(!super::should_lookup_target_dictionary("one\ntwo", "en"));
    }
}
