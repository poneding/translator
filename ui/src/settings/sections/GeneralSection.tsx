import { useState } from "react";
import { useConfigStore } from "../../stores/config";
import { useT } from "../../i18n";
import { Combobox } from "../../components/Combobox";
import { COMMON_LANGUAGES, commonLanguageOptions } from "../../i18n/languages";

// BH-9.3: BCP-47 primary-language-subtag format. Accepts any of the
// SPEC options above plus the 2-3 letter ISO codes and (optional) script
// subtags. Strict-but-permissive: primary tag is mandatory, script is
// optional and limited to a-z letters.
const BCP47_RE =
  /^[A-Za-z]{2,3}(?:-[A-Za-z]{4})?(?:-(?:[A-Za-z]{2}|[0-9]{3}))*$/;

type WindowDisplayPosition = "remember" | "right" | "center" | "mouse";

export function GeneralSection() {
  const { config, save } = useConfigStore();
  const t = useT();
  const [languageErr, setLanguageErr] = useState<string | null>(null);
  if (!config) return null;

  const preferredLanguages = normalizedPreference(
    config.general.preferred_languages,
  );
  const firstLanguage = preferredLanguages[0] ?? "zh-Hans";
  const secondLanguage = preferredLanguages[1] ?? "en";
  const windowDisplayPosition = normalizeWindowDisplayPosition(
    config.window.display_position,
  );
  const windowDisplayPositionOptions = [
    {
      value: "remember",
      label: t(
        "settings-general-window-position-remember",
        null,
        "Remember last position",
      ),
    },
    {
      value: "right",
      label: t("settings-general-window-position-right", null, "Top right"),
    },
    {
      value: "center",
      label: t("settings-general-window-position-center", null, "Center"),
    },
    {
      value: "mouse",
      label: t("settings-general-window-position-mouse", null, "Follow mouse"),
    },
  ];

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
        {languageErr && (
          <p className="mt-1 text-xs text-red-500">{languageErr}</p>
        )}
      </div>
      <div className="border-t border-border pt-4">
        <Combobox
          label={t(
            "settings-general-window-position",
            null,
            "Default open position",
          )}
          options={windowDisplayPositionOptions}
          value={windowDisplayPosition}
          onChange={(value) => {
            const displayPosition = normalizeWindowDisplayPosition(value);
            void save({
              ...config,
              window: {
                ...config.window,
                display_position: displayPosition,
              },
            });
          }}
        />
      </div>
      <div className="space-y-2 border-t border-border pt-4">
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="checkbox"
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
            className="checkbox"
            checked={config.general.auto_translate_clipboard_on_hotkey}
            onChange={(e) =>
              void save({
                ...config,
                general: {
                  ...config.general,
                  auto_translate_clipboard_on_hotkey: e.target.checked,
                },
              })
            }
          />
          <span className="min-w-0 leading-5">
            {t("settings-general-clipboard-hotkey")}
          </span>
        </label>
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="checkbox"
            checked={config.app.show_menu_bar_icon}
            onChange={(e) =>
              void save({
                ...config,
                app: {
                  ...config.app,
                  show_menu_bar_icon: e.target.checked,
                },
              })
            }
          />
          <span className="min-w-0 leading-5">
            {t("settings-general-show-menu-bar-icon")}
          </span>
        </label>
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="checkbox"
            checked={config.app.launch_at_startup}
            onChange={(e) =>
              void save({
                ...config,
                app: {
                  ...config.app,
                  launch_at_startup: e.target.checked,
                },
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
  const selectValue = COMMON_LANGUAGES.some(
    (language) => language.code === value,
  )
    ? value
    : "__custom__";
  const options = [
    ...commonLanguageOptions(t),
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
  if (result.length === 1)
    result.push(languageKey(result[0]) === "en" ? "zh-Hans" : "en");
  return result;
}

function languageKey(language: string): string {
  const normalized = language.trim().replaceAll("_", "-").toLowerCase();
  if (normalized.startsWith("zh-hans") || normalized.startsWith("zh-cn"))
    return "zh-hans";
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

function normalizeWindowDisplayPosition(
  value: string | undefined,
): WindowDisplayPosition {
  return value === "right" || value === "center" || value === "mouse"
    ? value
    : "remember";
}
