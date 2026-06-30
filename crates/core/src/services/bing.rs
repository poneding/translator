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
use std::collections::HashMap;
use std::time::Instant;

use crate::error::{ServiceError, ServiceResult};
use crate::model::{
    DictionaryPart, DictionaryResult, ServiceId, SimpleDictionaryWord, TranslateRequest,
    TranslateResult, WordExchange, WordPhonetic, part_abbreviation,
};
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
        let to = req.to.clone();
        let from_lang = req
            .from
            .as_deref()
            .filter(|from| !from.eq_ignore_ascii_case("auto"))
            .unwrap_or("auto-detect");

        // v7 dictionary path: English single-word -> Chinese. Falls back to the
        // translate + lookup path when the dict endpoint fails or yields nothing.
        if is_english_word_to_chinese(&req.text, req.from.as_deref(), &to)
            && let Ok(dict_json) = fetch_bing_dict(client, &base_url, &req.text).await
            && let Some(dict) = parse_bing_dict(&dict_json, &req.text)
        {
            let audio_url = dict.primary_audio_url();
            let text = dict
                .parts
                .first()
                .and_then(|p| p.means.first().cloned())
                .unwrap_or_default();
            return Ok(TranslateResult {
                service_id: ServiceId::Bing,
                service_name: "Microsoft Translator".to_string(),
                from: Some("en".to_string()),
                to,
                text,
                audio_url,
                detected_source: Some("en".to_string()),
                elapsed_ms: started.elapsed().as_millis() as u64,
                dictionary: None,
                source_dictionary: Some(dict),
                target_dictionary: None,
                extra: None,
                alternatives: Vec::new(),
            });
        }

        let web_config = fetch_web_config(client, &base_url).await?;
        let translate_json =
            fetch_bing_translate(client, &base_url, &web_config, &req.text, from_lang, &to).await?;
        let (text, detected, transliteration_phonetic) =
            parse_bing_translate(&translate_json, &req.text)?;

        // Lookup (tlookupv3): use the concrete source language, or the one the
        // translate endpoint detected when the request was auto-detect.
        let lookup_from = req
            .from
            .as_deref()
            .filter(|f| !f.eq_ignore_ascii_case("auto"))
            .map(str::to_string)
            .or(detected.clone())
            .unwrap_or_else(|| from_lang.to_string());
        let from_zh_to_en = lookup_from.to_ascii_lowercase().starts_with("zh")
            && to.to_ascii_lowercase().starts_with("en");
        let lookup_dict =
            fetch_bing_lookup(client, &base_url, &web_config, &req.text, &lookup_from, &to)
                .await
                .ok()
                .and_then(|j| parse_bing_lookup(&j, from_zh_to_en));

        let mut source_dict = lookup_dict.unwrap_or_default();
        if let Some(ph) = transliteration_phonetic {
            source_dict.phonetics.push(ph);
        }
        let source_dictionary = if source_dict.is_empty() {
            None
        } else {
            Some(source_dict)
        };

        Ok(TranslateResult {
            service_id: ServiceId::Bing,
            service_name: "Microsoft Translator".to_string(),
            from: req.from.clone(),
            to,
            text,
            audio_url: None,
            detected_source: detected,
            elapsed_ms: started.elapsed().as_millis() as u64,
            dictionary: None,
            source_dictionary,
            target_dictionary: None,
            extra: None,
            alternatives: Vec::new(),
        })
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
            alternatives: Vec::new(),
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
// Bing dictionary (v7) + lookup (tlookupv3) — mirrors Easydict's BingService.
// =============================================================================

/// The v7 dictionary-words app id + market baked into Easydict's BingConfig.
const BING_DICT_APPID: &str = "371E7B2AF0F9B84EC491D731DF90A55719C7D209";

/// True for an English single-word query targeting Chinese (the v7 dict path).
/// Easydict gates on `from == english && to == simplifiedChinese && wordCount == 1`;
/// we also accept auto-detect source when the text is unambiguously a single
/// English word, since the app does not pre-detect the source language.
fn is_english_word_to_chinese(text: &str, from: Option<&str>, to: &str) -> bool {
    if !to.to_ascii_lowercase().starts_with("zh") {
        return false;
    }
    let from_is_en = from
        .map(|f| f.to_ascii_lowercase().starts_with("en"))
        .unwrap_or(true);
    if !from_is_en {
        return false;
    }
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !trimmed.contains(char::is_whitespace)
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-' || c == '\'')
        && trimmed.chars().count() <= 20
}

/// Matches Easydict's `isShortWordLength`: ASCII single word <= 20 chars,
/// otherwise <= 7 chars. Gates the transliteration phonetic.
fn is_short_word(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.contains('\n') || trimmed.contains(char::is_whitespace) {
        return false;
    }
    let len = trimmed.chars().count();
    if trimmed.is_ascii() {
        len <= 20
    } else {
        len <= 7
    }
}

/// Parse the v7 dictionary-words response into structured dictionary data.
fn parse_bing_dict(json: &serde_json::Value, word: &str) -> Option<DictionaryResult> {
    let value = json.get("value")?.as_array()?.first()?;
    let meaning_groups = value.get("meaningGroups")?.as_array()?;
    if meaning_groups.is_empty() {
        return None;
    }
    let audio_url = value
        .get("pronunciationAudio")
        .and_then(|a| a.get("contentUrl"))
        .and_then(|u| u.as_str());

    let mut dict = DictionaryResult::default();
    for mg in meaning_groups {
        let parts_of_speech = match mg.get("partsOfSpeech").and_then(|p| p.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => continue,
        };
        let name = parts_of_speech[0]
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let description = parts_of_speech[0]
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let meanings = match mg.get("meanings").and_then(|m| m.as_array()) {
            Some(arr) if !arr.is_empty() => arr,
            _ => continue,
        };
        let rich_definitions = meanings[0]
            .get("richDefinitions")
            .and_then(|r| r.as_array());
        let fragments: Option<&[serde_json::Value]> = rich_definitions
            .and_then(|r| r.first())
            .and_then(|rd| rd.get("fragments"))
            .and_then(|f| f.as_array())
            .map(Vec::as_slice);

        match description {
            "发音" => {
                if let Some(ph) = parse_bing_phonetic(name, word, fragments, audio_url) {
                    dict.phonetics.push(ph);
                }
            }
            "快速释义" => {
                if let Some(part) = parse_bing_part(name, fragments) {
                    dict.parts.push(part);
                }
            }
            "词组" => dict
                .simple_words
                .extend(parse_bing_phrases(rich_definitions)),
            "分类词典" => parse_bing_synonyms_antonyms(&meanings[0], name, &mut dict),
            "搭配" => {
                if let Some(part) = parse_bing_part(name, fragments) {
                    dict.collocation.push(part);
                }
            }
            _ => {}
        }
        if name == "变形" {
            dict.exchanges.extend(parse_bing_exchanges(fragments));
        }
    }
    if dict.is_empty() { None } else { Some(dict) }
}

fn parse_bing_phonetic(
    name: &str,
    word: &str,
    fragments: Option<&[serde_json::Value]>,
    audio_url: Option<&str>,
) -> Option<WordPhonetic> {
    if name != "US" && name != "UK" {
        return None;
    }
    let _ = word;
    let value = fragments
        .and_then(|f| f.first())
        .and_then(|f| f.get("text"))
        .and_then(|t| t.as_str())
        .map(String::from);
    let audio = match (name, audio_url) {
        ("US", Some(u)) => Some(u.to_string()),
        ("UK", Some(u)) => Some(u.replace("tom", "george")),
        _ => None,
    };
    Some(WordPhonetic {
        label: name.to_string(),
        value,
        audio_url: audio,
        accent: Some(name.to_lowercase()),
    })
}

fn parse_bing_part(name: &str, fragments: Option<&[serde_json::Value]>) -> Option<DictionaryPart> {
    let means: Vec<String> = fragments
        .unwrap_or(&[])
        .iter()
        .filter_map(|f| f.get("text").and_then(|t| t.as_str()).map(String::from))
        .collect();
    if means.is_empty() {
        return None;
    }
    Some(DictionaryPart {
        part: Some(part_abbreviation(name)),
        means,
    })
}

fn parse_bing_phrases(
    rich_definitions: Option<&Vec<serde_json::Value>>,
) -> Vec<SimpleDictionaryWord> {
    let mut out = Vec::new();
    let empty: Vec<serde_json::Value> = Vec::new();
    for rd in rich_definitions.unwrap_or(&empty) {
        let examples = match rd.get("examples").and_then(|e| e.as_array()) {
            Some(e) if e.len() == 2 => e,
            _ => continue,
        };
        let word = examples[0].as_str().unwrap_or("").to_string();
        let means: Vec<String> = examples[1]
            .as_str()
            .unwrap_or("")
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        out.push(SimpleDictionaryWord {
            word,
            part: None,
            means,
        });
    }
    out
}

fn parse_bing_synonyms_antonyms(
    meaning: &serde_json::Value,
    name: &str,
    dict: &mut DictionaryResult,
) {
    let collect = |key: &str| -> Vec<String> {
        meaning
            .get(key)
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.get("name").and_then(|n| n.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let syns = collect("synonyms");
    let ants = collect("antonyms");
    if !syns.is_empty() {
        dict.synonyms.push(DictionaryPart {
            part: Some(part_abbreviation(name)),
            means: syns,
        });
    }
    if !ants.is_empty() {
        dict.antonyms.push(DictionaryPart {
            part: Some(part_abbreviation(name)),
            means: ants,
        });
    }
}

fn parse_bing_exchanges(fragments: Option<&[serde_json::Value]>) -> Vec<WordExchange> {
    let mut out = Vec::new();
    for f in fragments.unwrap_or(&[]) {
        let Some(text) = f.get("text").and_then(|t| t.as_str()) else {
            continue;
        };
        if let Some((n, w)) = text.split_once('：') {
            out.push(WordExchange {
                name: n.to_string(),
                words: vec![w.to_string()],
            });
        }
    }
    out
}

/// Parse the tlookupv3 lookup response. `from_zh_to_en` switches between
/// simple_words (zh→en) and parts (everything else), matching Easydict.
fn parse_bing_lookup(json: &serde_json::Value, from_zh_to_en: bool) -> Option<DictionaryResult> {
    let first = json.as_array()?.first()?;
    let translations = first.get("translations")?.as_array()?;
    if translations.is_empty() {
        return None;
    }
    let mut groups: HashMap<String, Vec<&serde_json::Value>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for tr in translations {
        let Some(pos) = tr.get("posTag").and_then(|p| p.as_str()) else {
            continue;
        };
        let pos = pos.to_string();
        if !groups.contains_key(&pos) {
            order.push(pos.clone());
        }
        groups.entry(pos).or_default().push(tr);
    }
    let mut dict = DictionaryResult::default();
    for pos in &order {
        let trs = &groups[pos];
        if from_zh_to_en {
            for tr in trs {
                let word = tr
                    .get("displayTarget")
                    .and_then(|d| d.as_str())
                    .unwrap_or("")
                    .to_string();
                let means: Vec<String> = tr
                    .get("backTranslations")
                    .and_then(|b| b.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| {
                                x.get("displayText")
                                    .and_then(|d| d.as_str())
                                    .map(String::from)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                dict.simple_words.push(SimpleDictionaryWord {
                    word,
                    part: Some(part_abbreviation(&pos.to_lowercase())),
                    means,
                });
            }
        } else {
            let means: Vec<String> = trs
                .iter()
                .filter_map(|tr| {
                    tr.get("displayTarget")
                        .and_then(|d| d.as_str())
                        .map(String::from)
                })
                .collect();
            if !means.is_empty() {
                dict.parts.push(DictionaryPart {
                    part: Some(part_abbreviation(&pos.to_lowercase())),
                    means,
                });
            }
        }
    }
    if dict.parts.is_empty() && dict.simple_words.is_empty() {
        None
    } else {
        Some(dict)
    }
}

/// Parse the ttranslatev3 response array. Returns (text, detected_language,
/// transliteration phonetic) — the phonetic is only set for short words.
fn parse_bing_translate(
    json: &serde_json::Value,
    text: &str,
) -> ServiceResult<(String, Option<String>, Option<WordPhonetic>)> {
    let arr = json
        .as_array()
        .ok_or_else(|| ServiceError::Parse("bing: expected response array".to_string()))?;
    let first = arr
        .first()
        .ok_or_else(|| ServiceError::Parse("bing: empty response array".to_string()))?;
    let translations = first
        .get("translations")
        .and_then(|t| t.as_array())
        .ok_or_else(|| ServiceError::Parse("bing: no translations[] entry".to_string()))?;
    let translation = translations
        .first()
        .ok_or_else(|| ServiceError::Parse("bing: no translations[] entry".to_string()))?;
    let translated_text = translation
        .get("text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| ServiceError::Parse("bing: translation has no text".to_string()))?
        .to_string();
    let detected = first
        .get("detectedLanguage")
        .and_then(|d| d.get("language"))
        .and_then(|l| l.as_str())
        .map(String::from);
    let phonetic = (arr.len() >= 2)
        .then(|| arr.get(1))
        .flatten()
        .and_then(|d| d.get("inputTransliteration"))
        .and_then(|t| t.as_str())
        .filter(|_| is_short_word(text))
        .map(|value| {
            let from_en = detected
                .as_deref()
                .map(|d| d.starts_with("en"))
                .unwrap_or(true);
            WordPhonetic {
                label: if from_en {
                    "US".to_string()
                } else {
                    "Pinyin".to_string()
                },
                value: Some(value.to_string()),
                audio_url: None,
                accent: None,
            }
        });
    Ok((translated_text, detected, phonetic))
}

async fn fetch_bing_dict(
    client: &Client,
    base_url: &str,
    text: &str,
) -> ServiceResult<serde_json::Value> {
    let url = format!(
        "{base_url}/api/v7/dictionarywords/search?appid={BING_DICT_APPID}&mkt=zh-cn&pname=bingdict"
    );
    let resp = client
        .get(&url)
        .header("User-Agent", WEB_USER_AGENT)
        .query(&[("q", text)])
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ServiceError::Api {
            code: "upstream".to_string(),
            message: resp.text().await.unwrap_or_default(),
        });
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| ServiceError::Parse(format!("bing dict json: {e}")))
}

async fn fetch_bing_translate(
    client: &Client,
    base_url: &str,
    web_config: &BingWebConfig,
    text: &str,
    from_lang: &str,
    to: &str,
) -> ServiceResult<serde_json::Value> {
    let form: Vec<(&str, String)> = vec![
        ("text", text.to_string()),
        ("to", to.to_string()),
        ("token", web_config.token.clone()),
        ("key", web_config.key.clone()),
        ("tryFetchingGenderDebiasedTranslations", "true".to_string()),
        ("fromLang", from_lang.to_string()),
    ];
    let url = format!(
        "{base_url}/ttranslatev3?isVertical=1&IG={}&IID={}",
        web_config.ig, web_config.iid
    );
    let resp = client
        .post(url)
        .header("User-Agent", WEB_USER_AGENT)
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ServiceError::Api {
            code: "upstream".to_string(),
            message: resp.text().await.unwrap_or_default(),
        });
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| ServiceError::Parse(format!("bing json: {e}")))
}

async fn fetch_bing_lookup(
    client: &Client,
    base_url: &str,
    web_config: &BingWebConfig,
    text: &str,
    from: &str,
    to: &str,
) -> ServiceResult<serde_json::Value> {
    let form: Vec<(&str, String)> = vec![
        ("text", text.to_string()),
        ("from", from.to_string()),
        ("to", to.to_string()),
        ("token", web_config.token.clone()),
        ("key", web_config.key.clone()),
    ];
    let url = format!(
        "{base_url}/tlookupv3?isVertical=1&IG={}&IID={}",
        web_config.ig, web_config.iid
    );
    let resp = client
        .post(url)
        .header("User-Agent", WEB_USER_AGENT)
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ServiceError::Api {
            code: "upstream".to_string(),
            message: resp.text().await.unwrap_or_default(),
        });
    }
    resp.json::<serde_json::Value>()
        .await
        .map_err(|e| ServiceError::Parse(format!("bing lookup json: {e}")))
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

    // ---- D1: v7 dict parsing covers every meaning-group kind ----
    #[test]
    fn parse_dict_v7_full() {
        let json = json!({
            "value": [{
                "pronunciationAudio": {"contentUrl": "https://cn.bing.com/audio/good.mp3?tom=1"},
                "meaningGroups": [
                    {"partsOfSpeech": [{"name": "US", "description": "发音"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "ɡʊd"}]}]}]},
                    {"partsOfSpeech": [{"name": "UK", "description": "发音"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "ɡʊd"}]}]}]},
                    {"partsOfSpeech": [{"name": "adj.", "description": "快速释义"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "好的"}, {"text": "优良的"}]}]}]},
                    {"partsOfSpeech": [{"name": "n.", "description": "快速释义"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "好处"}]}]}]},
                    {"partsOfSpeech": [{"name": "变形", "description": "其他"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "复数：goods"}, {"text": "比较级：better"}]}]}]},
                    {"partsOfSpeech": [{"name": "adj.", "description": "分类词典"}], "meanings": [{"synonyms": [{"name": "fine"}, {"name": "nice"}], "antonyms": [{"name": "bad"}]}]},
                    {"partsOfSpeech": [{"name": "v.", "description": "搭配"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "do good"}]}]}]},
                    {"partsOfSpeech": [{"name": "n.", "description": "词组"}], "meanings": [{"richDefinitions": [{"examples": ["good news", "好消息;好消"]}]}]}
                ]
            }]
        });
        let dict = super::parse_bing_dict(&json, "good").expect("parsed dict");
        assert_eq!(dict.phonetics.len(), 2);
        assert_eq!(dict.phonetics[0].label, "US");
        assert_eq!(dict.phonetics[0].value.as_deref(), Some("ɡʊd"));
        assert_eq!(
            dict.phonetics[0].audio_url.as_deref(),
            Some("https://cn.bing.com/audio/good.mp3?tom=1")
        );
        assert_eq!(dict.phonetics[1].label, "UK");
        assert_eq!(
            dict.phonetics[1].audio_url.as_deref(),
            Some("https://cn.bing.com/audio/good.mp3?george=1")
        );
        assert_eq!(dict.parts.len(), 2);
        assert_eq!(dict.parts[0].part.as_deref(), Some("adj."));
        assert_eq!(dict.parts[0].means, vec!["好的", "优良的"]);
        assert_eq!(dict.parts[1].part.as_deref(), Some("n."));
        assert_eq!(dict.parts[1].means, vec!["好处"]);
        assert_eq!(dict.exchanges.len(), 2);
        assert_eq!(dict.exchanges[0].name, "复数");
        assert_eq!(dict.exchanges[0].words, vec!["goods"]);
        assert_eq!(dict.exchanges[1].name, "比较级");
        assert_eq!(dict.exchanges[1].words, vec!["better"]);
        assert_eq!(dict.synonyms.len(), 1);
        assert_eq!(dict.synonyms[0].means, vec!["fine", "nice"]);
        assert_eq!(dict.antonyms.len(), 1);
        assert_eq!(dict.antonyms[0].means, vec!["bad"]);
        assert_eq!(dict.collocation.len(), 1);
        assert_eq!(dict.collocation[0].part.as_deref(), Some("v."));
        assert_eq!(dict.collocation[0].means, vec!["do good"]);
        assert_eq!(dict.simple_words.len(), 1);
        assert_eq!(dict.simple_words[0].word, "good news");
        assert_eq!(dict.simple_words[0].means, vec!["好消息", "好消"]);
    }

    // ---- D2: lookup (en->zh) groups translations by posTag into parts ----
    #[test]
    fn parse_lookup_en_to_zh_parts() {
        let json = json!([{
            "normalizedSource": "good",
            "translations": [
                {"displayTarget": "好的", "posTag": "ADJ"},
                {"displayTarget": "优良的", "posTag": "ADJ"},
                {"displayTarget": "好处", "posTag": "NOUN"}
            ]
        }]);
        let dict = super::parse_bing_lookup(&json, false).expect("parsed lookup");
        assert_eq!(dict.parts.len(), 2);
        let adj = dict
            .parts
            .iter()
            .find(|p| p.part.as_deref() == Some("adj."))
            .expect("adj part");
        assert_eq!(adj.means, vec!["好的", "优良的"]);
        let noun = dict
            .parts
            .iter()
            .find(|p| p.part.as_deref() == Some("n."))
            .expect("noun part");
        assert_eq!(noun.means, vec!["好处"]);
    }

    // ---- D3: lookup (zh->en) builds simple_words with backTranslations ----
    #[test]
    fn parse_lookup_zh_to_en_simple_words() {
        let json = json!([{
            "normalizedSource": "好",
            "translations": [{
                "displayTarget": "good",
                "posTag": "ADJ",
                "backTranslations": [{"displayText": "好"}, {"displayText": "良好"}]
            }]
        }]);
        let dict = super::parse_bing_lookup(&json, true).expect("parsed lookup");
        assert_eq!(dict.simple_words.len(), 1);
        assert_eq!(dict.simple_words[0].word, "good");
        assert_eq!(dict.simple_words[0].part.as_deref(), Some("adj."));
        assert_eq!(dict.simple_words[0].means, vec!["好", "良好"]);
    }

    // ---- D4: en single-word -> zh takes the v7 dict path end-to-end ----
    #[tokio::test]
    async fn translate_dict_path_for_english_word_to_chinese() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v7/dictionarywords/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{
                    "pronunciationAudio": {"contentUrl": "https://cn.bing.com/a.mp3"},
                    "meaningGroups": [
                        {"partsOfSpeech": [{"name": "US", "description": "发音"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "ɡʊd"}]}]}]},
                        {"partsOfSpeech": [{"name": "adj.", "description": "快速释义"}], "meanings": [{"richDefinitions": [{"fragments": [{"text": "好的"}]}]}]}
                    ]
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = cfg_for(&server);
        let req = TranslateRequest {
            text: "good".to_string(),
            from: Some("en".to_string()),
            to: "zh-Hans".to_string(),
        };
        let res = BingService
            .translate(&req, &cfg, None, &crate::http::test_client())
            .await
            .unwrap();
        assert_eq!(res.text, "好的");
        assert_eq!(res.detected_source.as_deref(), Some("en"));
        assert_eq!(res.audio_url.as_deref(), Some("https://cn.bing.com/a.mp3"));
        let dict = res.source_dictionary.expect("source dictionary");
        assert_eq!(dict.phonetics.len(), 1);
        assert_eq!(dict.parts.len(), 1);
        assert_eq!(dict.parts[0].means, vec!["好的"]);
    }
}
