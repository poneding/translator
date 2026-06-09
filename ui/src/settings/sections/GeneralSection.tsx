import { useState } from "react";
import { useConfigStore } from "../../stores/config";
import { useT } from "../../i18n";
import { Combobox } from "../../components/Combobox";

const LANGUAGE_OPTIONS: Array<{ code: string; labelKey: string; fallback: string }> = [
  { code: "en",      labelKey: "lang-en", fallback: "English" },
  { code: "zh-Hans", labelKey: "lang-zh-hans", fallback: "Simplified Chinese" },
  { code: "zh-Hant", labelKey: "lang-zh-hant", fallback: "Traditional Chinese" },
  { code: "ja",      labelKey: "lang-ja", fallback: "Japanese" },
  { code: "ko",      labelKey: "lang-ko", fallback: "Korean" },
  { code: "fr",      labelKey: "lang-fr", fallback: "French" },
  { code: "de",      labelKey: "lang-de", fallback: "German" },
  { code: "es",      labelKey: "lang-es", fallback: "Spanish" },
  { code: "ru",      labelKey: "lang-ru", fallback: "Russian" },
  { code: "pt",      labelKey: "lang-pt", fallback: "Portuguese" },
  { code: "it",      labelKey: "lang-it", fallback: "Italian" },
  { code: "ar",      labelKey: "lang-ar", fallback: "Arabic" },
];

// BH-9.3: BCP-47 primary-language-subtag format. Accepts any of the
// SPEC options above plus the 2-3 letter ISO codes and (optional) script
// subtags. Strict-but-permissive: primary tag is mandatory, script is
// optional and limited to a-z letters.
const BCP47_RE = /^[A-Za-z]{2,3}(?:-[A-Za-z]{4})?(?:-(?:[A-Za-z]{2}|[0-9]{3}))*$/;

export function GeneralSection() {
  const { config, save } = useConfigStore();
  const t = useT();
  const [languageErr, setLanguageErr] = useState<string | null>(null);
  if (!config) return null;

  const preferredLanguages = normalizedPreference(config.general.preferred_languages);
  const firstLanguage = preferredLanguages[0] ?? "zh-Hans";
  const secondLanguage = preferredLanguages[1] ?? "en";

  const saveLanguage = (index: 0 | 1, value: string) => {
    const language = value.trim();
    if (!BCP47_RE.test(language)) {
      setLanguageErr(t("settings-general-invalid-bcp47"));
      return;
    }

    const next = [...preferredLanguages];
    next[index] = language;
    const other = next[index === 0 ? 1 : 0];
    if (other && languageKey(other) === languageKey(language)) {
      setLanguageErr(t("settings-general-duplicate-language"));
      return;
    }

    const deduped = normalizedPreference(next);
    if (deduped.length < 2) {
      setLanguageErr(t("settings-general-duplicate-language"));
      return;
    }

    setLanguageErr(null);
    void save({
      ...config,
      general: {
        ...config.general,
        target_language: deduped[0],
        default_from: "auto",
        preferred_languages: deduped,
      },
    });
  };

  return (
    <div className="space-y-4">
      <div className="space-y-3">
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-3">
          <LanguagePreferenceField
            customAriaLabel={t("settings-general-first-language-custom-aria")}
            label={t("settings-general-first-language")}
            onChange={(value) => saveLanguage(0, value)}
            t={t}
            value={firstLanguage}
          />
          <LanguagePreferenceField
            customAriaLabel={t("settings-general-second-language-custom-aria")}
            label={t("settings-general-second-language")}
            onChange={(value) => saveLanguage(1, value)}
            t={t}
            value={secondLanguage}
          />
        </div>
        <p className="mt-1 text-xs text-fg-subtle">
          {t("settings-general-preferred-languages-hint")}
        </p>
        {languageErr && <p className="mt-1 text-xs text-red-500">{languageErr}</p>}
      </div>
      <div className="space-y-2 border-t border-border pt-4">
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="mt-0.5 h-4 w-4 shrink-0 accent-[rgb(var(--accent))]"
            checked={config.general.auto_copy}
            onChange={(e) =>
              void save({
                ...config,
                general: { ...config.general, auto_copy: e.target.checked },
              })
            }
          />
          <span className="min-w-0 leading-5">
            {t("settings-general-auto-copy")}
          </span>
        </label>
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="mt-0.5 h-4 w-4 shrink-0 accent-[rgb(var(--accent))]"
            checked={config.general.launch_at_startup}
            onChange={(e) =>
              void save({
                ...config,
                general: { ...config.general, launch_at_startup: e.target.checked },
              })
            }
          />
          <span className="min-w-0 leading-5">
            {t("settings-general-launch-at-startup")}
          </span>
        </label>
      </div>
    </div>
  );
}

function LanguagePreferenceField({
  customAriaLabel,
  label,
  onChange,
  t,
  value,
}: {
  customAriaLabel: string;
  label: string;
  onChange: (value: string) => void;
  t: ReturnType<typeof useT>;
  value: string;
}) {
  const selectValue = LANGUAGE_OPTIONS.some((language) => language.code === value) ? value : "__custom__";
  const options = [
    ...LANGUAGE_OPTIONS.map((language) => ({
      value: language.code,
      label: `${t(language.labelKey, null, language.fallback)} (${language.code})`,
    })),
    {
      value: "__custom__",
      label: t("settings-general-custom-bcp47"),
    },
  ];

  return (
    <div className="min-w-0">
      <Combobox
        label={label}
        options={options}
        value={selectValue}
        onChange={(nextValue) => {
          if (nextValue !== "__custom__") onChange(nextValue);
        }}
      />
      {selectValue === "__custom__" && (
        <input
          key={value}
          className="input mt-2"
          aria-label={customAriaLabel}
          defaultValue={value}
          placeholder="e.g. en-US, sr-Latn"
          onBlur={(event) => onChange(event.target.value)}
        />
      )}
    </div>
  );
}

function normalizedPreference(languages: string[] | undefined): string[] {
  const result: string[] = [];
  const source = languages?.length ? languages : ["zh-Hans", "en"];
  for (const language of source) {
    const value = language.trim();
    if (!value || value.toLowerCase() === "auto") continue;
    const key = languageKey(value);
    if (result.some((existing) => languageKey(existing) === key)) continue;
    result.push(value);
  }
  if (result.length === 1) result.push(languageKey(result[0]) === "en" ? "zh-Hans" : "en");
  return result;
}

function languageKey(language: string): string {
  const normalized = language.trim().replaceAll("_", "-").toLowerCase();
  if (normalized.startsWith("zh-hans") || normalized.startsWith("zh-cn")) return "zh-hans";
  if (
    normalized.startsWith("zh-hant") ||
    normalized.startsWith("zh-tw") ||
    normalized.startsWith("zh-hk") ||
    normalized.startsWith("zh-mo")
  ) {
    return "zh-hant";
  }
  return normalized.split("-")[0] ?? normalized;
}
