import { Download, RefreshCw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useT } from "../../i18n";
import * as api from "../../ipc/commands";
import { useConfigStore } from "../../stores/config";
import type { UpdateStatusDto } from "../../types/bindings";

const IDLE_STATUS: UpdateStatusDto = {
  status: "idle",
  update: null,
  error: null,
  downloaded: null,
  total: null,
};

export function UpdateSection() {
  const { config, save } = useConfigStore();
  const t = useT();
  const [status, setStatus] = useState<UpdateStatusDto>(IDLE_STATUS);

  useEffect(() => {
    const unlistenPromise = api.onUpdateStatus((payload) => setStatus(payload));
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const statusText = useMemo(() => updateStatusText(status, t), [status, t]);
  const checking = status.status === "checking";
  const installing = status.status === "installing";
  const available = status.status === "available" && status.update?.available;

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

      <div className="flex flex-wrap items-center gap-2 border-t border-border pt-4">
        <button
          className="btn"
          disabled={checking || installing}
          onClick={async () => {
            setStatus({ ...IDLE_STATUS, status: "checking" });
            const next = await api.checkUpdate(true);
            setStatus(next);
          }}
        >
          <RefreshCw
            size={15}
            aria-hidden="true"
            className={checking ? "animate-spin" : ""}
          />
          {t("settings-update-check")}
        </button>
        {available && (
          <button
            className="btn btn-primary"
            disabled={installing}
            onClick={async () => {
              setStatus({ ...IDLE_STATUS, status: "installing" });
              const next = await api.downloadAndInstallUpdate();
              setStatus(next);
            }}
          >
            <Download size={15} aria-hidden="true" />
            {t("settings-update-install")}
          </button>
        )}
      </div>

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
      return t("settings-update-status-idle");
  }
}
