//! Shared data models used across the workspace.

use serde::{Deserialize, Serialize};

/// Unique identifier for a translation service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceId {
    /// Youdao (有道) translation
    Youdao,
    /// DeepL
    DeepL,
    /// Google Cloud Translation
    Google,
    /// Microsoft Bing / Azure Translator
    Bing,
    /// OpenAI-compatible chat completions API
    OpenAI,
}

impl ServiceId {
    /// Stable string id (used in config and persisted state).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Youdao => "youdao",
            Self::DeepL => "deepl",
            Self::Google => "google",
            Self::Bing => "bing",
            Self::OpenAI => "openai",
        }
    }

    /// Human-readable display name (English).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Youdao => "Youdao",
            Self::DeepL => "DeepL",
            Self::Google => "Google Translate",
            Self::Bing => "Microsoft Translator",
            Self::OpenAI => "OpenAI Compatible",
        }
    }

    /// All known service ids in canonical order.
    pub fn all() -> &'static [ServiceId] {
        &[
            Self::Youdao,
            Self::DeepL,
            Self::Google,
            Self::Bing,
            Self::OpenAI,
        ]
    }
}

/// A request to translate a piece of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
    /// Source text. Caller is responsible for trimming.
    pub text: String,
    /// Source language code (e.g. `"en"`, `"zh-Hans"`) or `None` for auto-detect.
    pub from: Option<String>,
    /// Target language code (e.g. `"zh-Hans"`, `"en"`). Always required.
    pub to: String,
}

impl TranslateRequest {
    /// Convenience constructor for auto-detect source.
    pub fn auto(text: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from: None,
            to: to.into(),
        }
    }
}

/// The outcome of a single service translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    /// Which service produced this result.
    pub service_id: ServiceId,
    /// Human-readable service name (resolved at call time).
    pub service_name: String,
    /// Source language resolved for this request, or `None` when provider auto-detect is used.
    pub from: Option<String>,
    /// Target language resolved for this request.
    pub to: String,
    /// Translated text.
    pub text: String,
    /// Primary translated-text audio URL, when the service provides one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
    /// Language code detected by the service, if any.
    pub detected_source: Option<String>,
    /// Wall-clock time spent on the request.
    pub elapsed_ms: u64,
    /// Structured dictionary details for word lookup results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dictionary: Option<DictionaryResult>,
    /// Dictionary details for the original source text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_dictionary: Option<DictionaryResult>,
    /// Dictionary details for the translated target text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_dictionary: Option<DictionaryResult>,
    /// Optional extras (e.g. Youdao dictionary entries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
    /// Alternative translations, when a service provides ranked alternatives (e.g. DeepL).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
}

/// Detailed dictionary data aligned with Easydict word-result rendering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DictionaryResult {
    /// Pronunciation rows such as US and UK phonetics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phonetics: Vec<WordPhonetic>,
    /// Meanings grouped by part of speech.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<DictionaryPart>,
    /// Word form transformations such as plural or past tense.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exchanges: Vec<WordExchange>,
    /// Simple word or web translation entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub simple_words: Vec<SimpleDictionaryWord>,
    /// Dictionary tags such as exam categories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Synonym groups by part of speech.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub synonyms: Vec<DictionaryPart>,
    /// Antonym groups by part of speech.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub antonyms: Vec<DictionaryPart>,
    /// Collocation groups by part of speech.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collocation: Vec<DictionaryPart>,
    /// Etymology text, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etymology: Option<String>,
}

impl DictionaryResult {
    /// Returns whether no displayable dictionary data is present.
    pub fn is_empty(&self) -> bool {
        self.phonetics.is_empty()
            && self.parts.is_empty()
            && self.exchanges.is_empty()
            && self.simple_words.is_empty()
            && self.tags.is_empty()
            && self.synonyms.is_empty()
            && self.antonyms.is_empty()
            && self.collocation.is_empty()
            && self.etymology.is_none()
    }

    /// Return the first playable phonetic URL, matching Easydict's source-audio behavior.
    pub fn primary_audio_url(&self) -> Option<String> {
        self.phonetics
            .iter()
            .find_map(|phonetic| phonetic.audio_url.clone())
    }
}

/// A single phonetic row for a word.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordPhonetic {
    /// Display label, such as `US`, `UK`, or `Pinyin`.
    pub label: String,
    /// Phonetic text without surrounding slashes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Playable pronunciation URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<String>,
    /// Accent id used by the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}

/// A part-of-speech group with one or more meanings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryPart {
    /// Abbreviated part of speech such as `n.` or `v.`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// Meanings belonging to this part of speech.
    pub means: Vec<String>,
}

/// A word-form exchange entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordExchange {
    /// Form label from the dictionary service.
    pub name: String,
    /// Alternative forms under the label.
    pub words: Vec<String>,
}

/// A simple dictionary candidate or web translation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleDictionaryWord {
    /// Candidate word or phrase.
    pub word: String,
    /// Optional part of speech.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// Meanings for this candidate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub means: Vec<String>,
}

/// Normalize a part-of-speech string to EasyDict-style abbreviation
/// (`n.`/`v.`/`adj.`/…). Already-abbreviated and Chinese forms pass through
/// the same table. Mirrors `EZTranslatePart.partAbbreviation()` in EasyDict.
pub fn part_abbreviation(part: &str) -> String {
    let normalized = part.trim().trim_end_matches('.').to_ascii_lowercase();
    let mapped = match normalized.as_str() {
        "adjective" | "形容词" | "adj" => "adj.",
        "adverb" | "副词" | "adv" => "adv.",
        "verb" | "动词" | "v" => "v.",
        "noun" | "名词" | "n" => "n.",
        "pronoun" | "代词" | "pron" => "pron.",
        "preposition" | "介词" | "prep" => "prep.",
        "conjunction" | "连词" | "conj" => "conj.",
        "interjection" | "感叹词" | "int" | "interj" => "int.",
        "article" | "冠词" | "art" => "art.",
        "numeral" | "数词" | "num" => "num.",
        "linking verb" | "linkverb" | "linkv" => "linkv.",
        "auxiliary verb" | "auxverb" | "auxv" => "auxv.",
        "modal verb" | "modalverb" | "modalv" => "modalv.",
        "determiner" | "det" => "det.",
        "abbreviation" | "abbr" => "abbr.",
        "infinitive" | "inf" => "inf.",
        "participle" | "part" => "part.",
        "web" => "Web",
        _ => part.trim(),
    };
    mapped.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_result_is_empty_counts_new_fields() {
        assert!(DictionaryResult::default().is_empty());
        let mut d = DictionaryResult::default();
        d.synonyms.push(DictionaryPart {
            part: None,
            means: vec!["fast".into()],
        });
        assert!(!d.is_empty(), "synonyms should count");
        let mut d = DictionaryResult::default();
        d.etymology = Some("from Old English".into());
        assert!(!d.is_empty(), "etymology should count");
    }

    #[test]
    fn translate_result_skips_empty_alternatives_in_json() {
        let base = TranslateResult {
            service_id: ServiceId::Youdao,
            service_name: "Youdao".into(),
            from: None,
            to: "zh-Hans".into(),
            text: "好".into(),
            audio_url: None,
            detected_source: None,
            elapsed_ms: 0,
            dictionary: None,
            source_dictionary: None,
            target_dictionary: None,
            extra: None,
            alternatives: Vec::new(),
        };
        let json = serde_json::to_string(&base).unwrap();
        assert!(
            !json.contains("alternatives"),
            "empty alternatives must be skipped"
        );
        let mut with_alts = base.clone();
        with_alts.alternatives = vec!["good".into(), "fine".into()];
        let json2 = serde_json::to_string(&with_alts).unwrap();
        assert!(json2.contains("\"alternatives\""));
    }

    #[test]
    fn part_abbreviation_maps_full_easydict_table() {
        assert_eq!(part_abbreviation("noun"), "n.");
        assert_eq!(part_abbreviation("形容词"), "adj.");
        assert_eq!(part_abbreviation("adj."), "adj.");
        assert_eq!(part_abbreviation("linking verb"), "linkv.");
        assert_eq!(part_abbreviation("auxv"), "auxv.");
        assert_eq!(part_abbreviation("modal verb"), "modalv.");
        assert_eq!(part_abbreviation("determiner"), "det.");
        assert_eq!(part_abbreviation("abbreviation"), "abbr.");
        assert_eq!(part_abbreviation("infinitive"), "inf.");
        assert_eq!(part_abbreviation("participle"), "part.");
        assert_eq!(part_abbreviation("Web"), "Web");
    }
}
