import { useConfigStore } from "../../stores/config";

export function AppearanceSection() {
  const { config, save } = useConfigStore();
  if (!config) return null;
  const theme = config.general.theme;
  return (
    <div>
      <label className="label">Theme</label>
      <div className="inline-flex rounded-lg border border-border bg-bg-subtle p-0.5 text-sm">
        {(["system", "light", "dark"] as const).map((t) => (
          <button
            key={t}
            className={
              "rounded-md px-3 py-1 " +
              (theme === t ? "bg-bg text-fg shadow" : "text-fg-subtle hover:text-fg")
            }
            onClick={() => void save({ ...config, general: { ...config.general, theme: t } })}
          >
            {t}
          </button>
        ))}
      </div>
    </div>
  );
}
