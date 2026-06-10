import type { FluentVariable } from "@fluent/bundle";

export type TFunction = (
  key: string,
  args?: Record<string, FluentVariable> | null,
  fallback?: string,
) => string;

export type CommonLanguageCode =
  | "en"
  | "zh-Hans"
  | "zh-Hant"
  | "ja"
  | "ko"
  | "fr"
  | "de"
  | "es"
  | "ru"
  | "pt"
  | "it"
  | "ar";

export type TranslationLanguageCode = "auto" | CommonLanguageCode;
export type AppLanguageCode = "system" | CommonLanguageCode;

export interface LanguageMeta {
  code: CommonLanguageCode;
  labelKey: string;
  fallback: string;
  flag: string;
  shortCode: string;
}

export const COMMON_LANGUAGES: readonly LanguageMeta[] = [
  {
    code: "en",
    labelKey: "lang-en",
    fallback: "English",
    flag: "🇬🇧",
    shortCode: "EN",
  },
  {
    code: "zh-Hans",
    labelKey: "lang-zh-hans",
    fallback: "Simplified Chinese",
    flag: "🇨🇳",
    shortCode: "CN",
  },
  {
    code: "zh-Hant",
    labelKey: "lang-zh-hant",
    fallback: "Traditional Chinese",
    flag: "🇭🇰",
    shortCode: "TW",
  },
  {
    code: "ja",
    labelKey: "lang-ja",
    fallback: "Japanese",
    flag: "🇯🇵",
    shortCode: "JA",
  },
  {
    code: "ko",
    labelKey: "lang-ko",
    fallback: "Korean",
    flag: "🇰🇷",
    shortCode: "KO",
  },
  {
    code: "fr",
    labelKey: "lang-fr",
    fallback: "French",
    flag: "🇫🇷",
    shortCode: "FR",
  },
  {
    code: "de",
    labelKey: "lang-de",
    fallback: "German",
    flag: "🇩🇪",
    shortCode: "DE",
  },
  {
    code: "es",
    labelKey: "lang-es",
    fallback: "Spanish",
    flag: "🇪🇸",
    shortCode: "ES",
  },
  {
    code: "ru",
    labelKey: "lang-ru",
    fallback: "Russian",
    flag: "🇷🇺",
    shortCode: "RU",
  },
  {
    code: "pt",
    labelKey: "lang-pt",
    fallback: "Portuguese",
    flag: "🇵🇹",
    shortCode: "PT",
  },
  {
    code: "it",
    labelKey: "lang-it",
    fallback: "Italian",
    flag: "🇮🇹",
    shortCode: "IT",
  },
  {
    code: "ar",
    labelKey: "lang-ar",
    fallback: "Arabic",
    flag: "🇸🇦",
    shortCode: "AR",
  },
];

export const SUPPORTED_APP_LOCALES = COMMON_LANGUAGES.map(
  (language) => language.code,
) as readonly CommonLanguageCode[];

export function languageLabel(
  language: LanguageMeta,
  t: TFunction,
  includeCode = true,
): string {
  const name = languageName(language, t);
  return includeCode ? `${language.flag} ${name} ${language.shortCode}` : name;
}

export interface LanguageDisplayParts {
  flag: string;
  leading: string;
  label: string;
}

export function languageDisplayParts(
  language: LanguageMeta,
  t: TFunction,
): LanguageDisplayParts {
  return {
    flag: language.flag,
    leading: language.shortCode,
    label: languageName(language, t),
  };
}

export function languageName(language: LanguageMeta, t: TFunction): string {
  return t(language.labelKey, null, language.fallback);
}

export function autoLanguageLabel(t: TFunction): string {
  const parts = autoLanguageParts(t);
  return `${parts.flag} ${parts.label} ${parts.leading}`;
}

export function autoLanguageParts(t: TFunction): LanguageDisplayParts {
  return {
    flag: "🌐",
    leading: "AUTO",
    label: t("lang-auto", null, "Auto"),
  };
}

export function commonLanguageOptions(t: TFunction) {
  return COMMON_LANGUAGES.map((language) => {
    const parts = languageDisplayParts(language, t);
    return {
      value: language.code,
      label: parts.label,
      flag: parts.flag,
      leading: parts.leading,
    };
  });
}

export function translationLanguageOptions(
  t: TFunction,
  { includeAuto = false }: { includeAuto?: boolean } = {},
) {
  const options = commonLanguageOptions(t);
  if (!includeAuto) return options;
  return [
    {
      value: "auto",
      ...autoLanguageParts(t),
    },
    ...options,
  ];
}

export function appLanguageOptions(t: TFunction) {
  return [
    {
      value: "system",
      flag: "🌐",
      leading: "SYS",
      label: t("settings-appearance-language-system", null, "System"),
    },
    ...commonLanguageOptions(t),
  ];
}

export function isCommonLanguageCode(
  value: string,
): value is CommonLanguageCode {
  return COMMON_LANGUAGES.some((language) => language.code === value);
}

export function commonLanguageMetaForCode(
  value: string | null | undefined,
): LanguageMeta | null {
  const normalized = normalizeCommonLanguage(value);
  return (
    COMMON_LANGUAGES.find((language) => language.code === normalized) ?? null
  );
}

export function languageLabelForCode(
  value: string | null | undefined,
  t: TFunction,
): string | null {
  const parts = languagePartsForCode(value, t);
  return parts ? `${parts.flag} ${parts.label} ${parts.leading}` : null;
}

export function languagePartsForCode(
  value: string | null | undefined,
  t: TFunction,
): LanguageDisplayParts | null {
  const language = commonLanguageMetaForCode(value);
  return language ? languageDisplayParts(language, t) : null;
}

export function normalizeCommonLanguage(
  value: string | null | undefined,
): CommonLanguageCode | null {
  if (!value) return null;
  const lower = value.trim().replaceAll("_", "-").toLowerCase();
  if (!lower || lower === "auto") return null;
  if (
    lower === "zh" ||
    lower === "zh-hans" ||
    lower === "zh-cn" ||
    lower === "zh-chs" ||
    lower.startsWith("zh-hans-") ||
    lower.startsWith("zh-cn-")
  ) {
    return "zh-Hans";
  }
  if (
    lower === "zh-hant" ||
    lower === "zh-tw" ||
    lower === "zh-hk" ||
    lower === "zh-mo" ||
    lower === "zh-cht" ||
    lower.startsWith("zh-hant-") ||
    lower.startsWith("zh-tw-") ||
    lower.startsWith("zh-hk-") ||
    lower.startsWith("zh-mo-")
  ) {
    return "zh-Hant";
  }
  if (isCommonLanguageCode(lower)) return lower;
  const primary = lower.split("-")[0] ?? lower;
  return isCommonLanguageCode(primary) ? primary : null;
}

export function normalizeAppLanguage(
  value: string | undefined,
): AppLanguageCode {
  if (!value || value === "system") return "system";
  return normalizeCommonLanguage(value) ?? "system";
}
