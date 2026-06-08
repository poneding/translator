import { useConfigStore } from "../../stores/config";
import { useT } from "../../i18n";

export function AppearanceSection() {
  const { config, save } = useConfigStore();
  const t = useT();
  if (!config) return null;
  const theme = config.general.theme;
  const themeLabels: Record<"system" | "light" | "dark", string> = {
    system: t("settings-appearance-theme-system"),
    light: t("settings-appearance-theme-light"),
    dark: t("settings-appearance-theme-dark"),
  };
  return (
    <div>
      <label className="label">{t("settings-appearance-theme")}</label>
      <div className="inline-flex rounded-lg border border-border bg-bg-subtle p-0.5 text-sm">
        {(["system", "light", "dark"] as const).map((item) => (
          <button
            key={item}
            aria-label={t("settings-appearance-theme-aria", { theme: themeLabels[item] }, `Theme: ${themeLabels[item]}`)}
            aria-pressed={theme === item}
            className={
              "rounded-md px-3 py-1 " +
              (theme === item ? "bg-bg text-fg shadow" : "text-fg-subtle hover:text-fg")
            }
            onClick={() => void save({ ...config, general: { ...config.general, theme: item } })}
          >
            {themeLabels[item]}
          </button>
        ))}
      </div>
    </div>
  );
}
