import { Monitor, Moon, Sun } from "lucide-react";
import { Combobox } from "../../components/Combobox";
import { useConfigStore } from "../../stores/config";
import { setLocale, useT } from "../../i18n";
import { appLanguageOptions, normalizeAppLanguage } from "../../i18n/languages";
import type { AppLanguageCode } from "../../types/bindings";

type Theme = "system" | "light" | "dark";

export function AppearanceSection() {
  const { config, save } = useConfigStore();
  const t = useT();
  if (!config) return null;
  const theme = config.general.theme as Theme;
  const appLanguage = normalizeAppLanguage(config.general.app_language);
  const languageOptions = appLanguageOptions(t);
  const themeLabels: Record<Theme, string> = {
    system: t("settings-appearance-theme-system"),
    light: t("settings-appearance-theme-light"),
    dark: t("settings-appearance-theme-dark"),
  };
  const themeOptions = [
    {
      value: "system",
      label: themeLabels.system,
      Icon: Monitor,
    },
    {
      value: "light",
      label: themeLabels.light,
      Icon: Sun,
    },
    {
      value: "dark",
      label: themeLabels.dark,
      Icon: Moon,
    },
  ];

  return (
    <div className="grid gap-3 sm:grid-cols-2">
      <Combobox
        ariaLabel={t(
          "settings-appearance-theme-aria",
          { theme: themeLabels[theme] },
          `Theme: ${themeLabels[theme]}`,
        )}
        label={t("settings-appearance-theme")}
        options={themeOptions}
        value={theme}
        onChange={(value) => {
          const next = value as Theme;
          void save({
            ...config,
            general: { ...config.general, theme: next },
          });
        }}
      />
      <div className="min-w-0">
        <Combobox
          label={t("settings-appearance-language", null, "App language")}
          options={languageOptions}
          value={appLanguage}
          onChange={(value) => {
            const next = normalizeAppLanguage(value);
            setLocale(next);
            void save({
              ...config,
              general: {
                ...config.general,
                app_language: next as AppLanguageCode,
              },
            });
          }}
        />
      </div>
    </div>
  );
}
