//! Shared text-to-speech URL helpers.

use reqwest::Url;

const DEFAULT_YOUDAO_AUDIO_BASE: &str = "https://dict.youdao.com";

/// Build a default playable audio URL for arbitrary text.
///
/// The current implementation uses Youdao's public dictionary voice endpoint,
/// matching the built-in Youdao service behavior. The returned URL is suitable
/// for short source text and translated text playback.
pub fn default_text_audio_url(text: &str, language: Option<&str>) -> Option<String> {
    youdao_text_audio_url(text, language, "us", DEFAULT_YOUDAO_AUDIO_BASE)
}

/// Build a Youdao dictionary voice URL for the provided text.
pub fn youdao_text_audio_url(
    text: &str,
    language: Option<&str>,
    accent: &str,
    web_base_url: &str,
) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let language = tts_language(language.unwrap_or("auto"));
    let accent_type = if accent.eq_ignore_ascii_case("uk") {
        "1"
    } else {
        "2"
    };
    let mut url = Url::parse(&format!("{}/dictvoice", web_base_url.trim_end_matches('/'))).ok()?;
    url.query_pairs_mut()
        .append_pair("audio", text)
        .append_pair("le", &language)
        .append_pair("type", accent_type);
    Some(url.to_string())
}

fn tts_language(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "zh-chs" | "zh-hans" | "zh-cn" | "zh" => "zh".to_string(),
        "zh-cht" | "zh-hant" | "zh-tw" | "zh-hk" => "zh".to_string(),
        "" | "auto" => "en".to_string(),
        value => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_encoded_audio_url() {
        let url = youdao_text_audio_url("你好", Some("zh-Hans"), "us", "https://dict.youdao.com")
            .expect("audio url");

        assert!(url.contains("/dictvoice?"));
        assert!(url.contains("audio=%E4%BD%A0%E5%A5%BD"));
        assert!(url.contains("le=zh"));
        assert!(url.contains("type=2"));
    }
}
