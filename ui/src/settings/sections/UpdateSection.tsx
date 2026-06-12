import { Download, RefreshCw } from "lucide-react";
import { useMemo } from "react";
import { useT } from "../../i18n";
import { useConfigStore } from "../../stores/config";
import type { UpdateStatusDto } from "../../types/bindings";
import type { UpdateControls } from "./useUpdateControls";

export function UpdateCheckButton({ controls }: { controls: UpdateControls }) {
  const t = useT();
  return (
    <button
      className="btn"
      disabled={controls.checking || controls.installing}
      onClick={() => void controls.check()}
    >
      <RefreshCw
        size={15}
        aria-hidden="true"
        className={controls.checking ? "animate-spin" : ""}
      />
      {t("settings-update-check")}
    </button>
  );
}

export function UpdateSection({ controls }: { controls: UpdateControls }) {
  const { config, save } = useConfigStore();
  const t = useT();
  const { available, installing, status } = controls;
  const statusText = useMemo(() => updateStatusText(status, t), [status, t]);

  if (!config) return null;

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="checkbox"
            checked={config.updates.check_on_startup}
            onChange={(event) =>
              void save({
                ...config,
                updates: {
                  ...config.updates,
                  check_on_startup: event.target.checked,
                },
              })
            }
          />
          <span className="min-w-0 leading-5">
            {t("settings-update-check-startup")}
          </span>
        </label>
        <label className="flex cursor-pointer items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="checkbox"
            checked={config.updates.allow_beta}
            onChange={(event) =>
              void save({
                ...config,
                updates: {
                  ...config.updates,
                  allow_beta: event.target.checked,
                },
              })
            }
          />
          <span className="min-w-0 leading-5">
            {t("settings-update-allow-beta")}
          </span>
        </label>
      </div>

      {available && (
        <div className="flex flex-wrap items-center gap-2 border-t border-border pt-4">
          <button
            className="btn btn-primary"
            disabled={installing}
            onClick={() => void controls.install()}
          >
            <Download size={15} aria-hidden="true" />
            {t("settings-update-install")}
          </button>
        </div>
      )}

      {status.status !== "idle" && (
        <div className="rounded-md border border-border bg-bg px-3 py-2 text-sm">
          <p className={status.status === "failed" ? "text-red-500" : "text-fg"}>
            {statusText}
          </p>
          {status.update?.available && (
            <div className="mt-2 space-y-1 text-xs text-fg-subtle">
              <p>
                {t("settings-update-version", {
                  version: status.update.version ?? "",
                  channel: status.update.channel,
                })}
              </p>
              {status.update.date && (
                <p>{t("settings-update-date", { date: status.update.date })}</p>
              )}
              {status.update.body && (
                <p className="line-clamp-3 whitespace-pre-wrap">
                  {status.update.body}
                </p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function updateStatusText(
  status: UpdateStatusDto,
  t: ReturnType<typeof useT>,
): string {
  switch (status.status) {
    case "checking":
      return t("settings-update-status-checking");
    case "up-to-date":
      return t("settings-update-status-up-to-date");
    case "available":
      return t("settings-update-status-available");
    case "installing": {
      if (status.downloaded !== null && status.total) {
        return t("settings-update-status-installing-progress", {
          downloaded: Math.round(status.downloaded / 1024),
          total: Math.round(status.total / 1024),
        });
      }
      return t("settings-update-status-installing");
    }
    case "installed":
      return t("settings-update-status-installed");
    case "failed":
      return t("settings-update-status-failed", {
        msg: status.error ?? "unknown",
      });
    default:
      return "";
  }
}
