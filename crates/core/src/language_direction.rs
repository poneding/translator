//! Language direction policy for translation requests.
//!
//! The UI and persisted config can express language preferences in a flexible
//! way. Translation services still receive a plain `TranslateRequest` with an
//! optional source language and a concrete target language.

use crate::config::GeneralConfig;
use crate::model::TranslateRequest;

/// Fully resolved translation direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationDirection {
    /// Source language sent to providers, or `None` for provider auto-detect.
    pub from: Option<String>,
    /// Concrete target language sent to providers.
    pub to: String,
}

/// Build a service-ready translation request from user preferences.
pub fn translate_request(
    text: impl Into<String>,
    general: &GeneralConfig,
    from: Option<String>,
    to: Option<String>,
) -> TranslateRequest {
    let text = text.into();
    let source = sanitize_source(from).or_else(|| detect_language_hint(&text));
    let direction = resolve_direction(general, source, to);
    TranslateRequest {
        text,
        from: direction.from,
        to: direction.to,
    }
}

/// Resolve source and target languages from config plus optional overrides.
///
/// A manual target override wins. Otherwise the ordered preference list is
/// used: when the source matches a preferred language, target the first
/// different preferred language; when the source is auto or outside the list,
/// target the first preferred language.
pub fn resolve_direction(
    general: &GeneralConfig,
    from_override: Option<String>,
    to_override: Option<String>,
) -> TranslationDirection {
    let preferred = normalized_preferred_languages(
        &general.preferred_languages,
        &general.target_language,
        &general.default_from,
    );
    let from = sanitize_source(from_override);

    if let Some(to) = sanitize_target(to_override) {
        return TranslationDirection { from, to };
    }

    let to = match from.as_deref() {
        Some(source) => preferred
            .iter()
            .find(|language| language_key(language) != language_key(source))
            .cloned()
            .unwrap_or_else(|| default_counterpart(source).to_string()),
        None => preferred
            .first()
            .cloned()
            .unwrap_or_else(|| default_counterpart("").to_string()),
    };

    TranslationDirection { from, to }
}

pub(crate) fn normalized_preferred_languages(
    preferred_languages: &[String],
    target_language: &str,
    default_from: &str,
) -> Vec<String> {
    let mut languages = Vec::new();

    for language in preferred_languages {
        add_language(&mut languages, language);
    }

    if languages.is_empty() {
        add_language(&mut languages, target_language);
        add_language(&mut languages, default_from);
    }

    if languages.is_empty() {
        add_language(&mut languages, "zh-Hans");
        add_language(&mut languages, "en");
    }

    if languages.len() == 1 {
        let counterpart = default_counterpart(&languages[0]);
        add_language(&mut languages, counterpart);
    }

    languages
}

pub(crate) fn language_key(language: &str) -> String {
    let normalized = language.trim().replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-hans") || normalized.starts_with("zh-cn") {
        return "zh-hans".to_string();
    }
    if normalized.starts_with("zh-hant")
        || normalized.starts_with("zh-tw")
        || normalized.starts_with("zh-hk")
        || normalized.starts_with("zh-mo")
    {
        return "zh-hant".to_string();
    }
    normalized.split('-').next().unwrap_or_default().to_string()
}

fn sanitize_source(language: Option<String>) -> Option<String> {
    sanitize_language(language).filter(|value| !value.eq_ignore_ascii_case("auto"))
}

fn sanitize_target(language: Option<String>) -> Option<String> {
    sanitize_language(language).filter(|value| !value.eq_ignore_ascii_case("auto"))
}

fn sanitize_language(language: Option<String>) -> Option<String> {
    language
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn detect_language_hint(text: &str) -> Option<String> {
    let mut latin = 0;
    let mut cjk = 0;
    let mut hiragana_katakana = 0;
    let mut hangul = 0;
    let mut cyrillic = 0;
    let mut arabic = 0;

    for ch in text.chars() {
        let code = ch as u32;
        match code {
            0x0041..=0x005A | 0x0061..=0x007A | 0x00C0..=0x024F => latin += 1,
            0x3040..=0x30FF => hiragana_katakana += 1,
            0x3400..=0x9FFF => cjk += 1,
            0xAC00..=0xD7AF => hangul += 1,
            0x0400..=0x052F => cyrillic += 1,
            0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF => arabic += 1,
            _ => {}
        }
    }

    let candidates = [
        ("ja", hiragana_katakana),
        ("ko", hangul),
        ("zh-Hans", cjk),
        ("ru", cyrillic),
        ("ar", arabic),
        ("en", latin),
    ];
    let (language, count) = candidates.into_iter().max_by_key(|(_, count)| *count)?;
    (count > 0).then(|| language.to_string())
}

fn add_language(languages: &mut Vec<String>, language: &str) {
    let Some(trimmed) = sanitize_target(Some(language.to_string())) else {
        return;
    };
    let key = language_key(&trimmed);
    if languages
        .iter()
        .any(|existing| language_key(existing) == key)
    {
        return;
    }
    languages.push(trimmed);
}

fn default_counterpart(language: &str) -> &'static str {
    if language_key(language) == "en" {
        "zh-Hans"
    } else {
        "en"
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn general(preferred_languages: Vec<&str>) -> GeneralConfig {
        GeneralConfig {
            preferred_languages: preferred_languages.into_iter().map(String::from).collect(),
            ..GeneralConfig::default()
        }
    }

    #[test]
    fn auto_source_targets_first_preferred_language() {
        let direction = resolve_direction(&general(vec!["zh-Hans", "en"]), None, None);

        assert_eq!(direction.from, None);
        assert_eq!(direction.to, "zh-Hans");
    }

    #[test]
    fn source_matching_first_preferred_language_targets_second() {
        let direction = resolve_direction(
            &general(vec!["zh-Hans", "en"]),
            Some("zh-CN".to_string()),
            None,
        );

        assert_eq!(direction.from.as_deref(), Some("zh-CN"));
        assert_eq!(direction.to, "en");
    }

    #[test]
    fn source_matching_later_preferred_language_targets_first() {
        let direction = resolve_direction(
            &general(vec!["zh-Hans", "en", "ja"]),
            Some("en-US".to_string()),
            None,
        );

        assert_eq!(direction.from.as_deref(), Some("en-US"));
        assert_eq!(direction.to, "zh-Hans");
    }

    #[test]
    fn source_outside_preferences_targets_first_preferred_language() {
        let direction = resolve_direction(
            &general(vec!["zh-Hans", "en"]),
            Some("ja".to_string()),
            None,
        );

        assert_eq!(direction.from.as_deref(), Some("ja"));
        assert_eq!(direction.to, "zh-Hans");
    }

    #[test]
    fn manual_target_override_wins() {
        let direction = resolve_direction(
            &general(vec!["zh-Hans", "en"]),
            Some("en".to_string()),
            Some("fr".to_string()),
        );

        assert_eq!(direction.from.as_deref(), Some("en"));
        assert_eq!(direction.to, "fr");
    }

    #[test]
    fn legacy_target_and_source_seed_preferences_when_list_is_empty() {
        let general = GeneralConfig {
            preferred_languages: Vec::new(),
            target_language: "fr".to_string(),
            default_from: "en".to_string(),
            ..GeneralConfig::default()
        };

        let direction = resolve_direction(&general, None, None);

        assert_eq!(direction.from, None);
        assert_eq!(direction.to, "fr");
    }

    #[test]
    fn single_preferred_language_gets_default_counterpart() {
        let direction = resolve_direction(&general(vec!["en"]), Some("en".to_string()), None);

        assert_eq!(direction.to, "zh-Hans");
    }

    #[test]
    fn translate_request_uses_detected_source_hint() {
        let request = translate_request("你好", &general(vec!["zh-Hans", "en"]), None, None);

        assert_eq!(request.from.as_deref(), Some("zh-Hans"));
        assert_eq!(request.to, "en");
    }

    #[test]
    fn translate_request_keeps_english_target_when_source_is_english() {
        let request = translate_request("Hello", &general(vec!["en", "zh-Hans"]), None, None);

        assert_eq!(request.from.as_deref(), Some("en"));
        assert_eq!(request.to, "zh-Hans");
    }
}
