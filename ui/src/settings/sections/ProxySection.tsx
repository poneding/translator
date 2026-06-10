import { useConfigStore } from "../../stores/config";
import { useT } from "../../i18n";

export function ProxySection() {
  const { config, save } = useConfigStore();
  const t = useT();
  if (!config) return null;

  return (
    <div className="space-y-3">
      <label className="flex cursor-pointer items-start gap-2 text-sm">
        <input
          type="checkbox"
          className="checkbox"
          checked={config.general.proxy.enabled}
          onChange={(e) =>
            void save({
              ...config,
              general: {
                ...config.general,
                proxy: { ...config.general.proxy, enabled: e.target.checked },
              },
            })
          }
        />
        <span className="min-w-0 leading-5">
          {t("settings-general-use-proxy")}
        </span>
      </label>
      <div>
        <label className="label">{t("settings-general-proxy-url")}</label>
        <input
          className="input"
          aria-label={t("settings-general-proxy-url")}
          placeholder="http://127.0.0.1:7890"
          value={config.general.proxy.url}
          onChange={(e) =>
            void save({
              ...config,
              general: {
                ...config.general,
                proxy: { ...config.general.proxy, url: e.target.value },
              },
            })
          }
        />
      </div>
    </div>
  );
}
