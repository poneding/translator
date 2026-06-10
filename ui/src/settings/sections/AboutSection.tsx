import { useEffect, useState } from "react";
import { useT } from "../../i18n";
import * as api from "../../ipc/commands";

export function AboutSection() {
  // BH-12.1: show app version, build commit, build date, and the repo link.
  const t = useT();
  const [info, setInfo] = useState<api.AppInfo | null>(null);
  const repoUrl = info?.repo_url.trim() ?? "";
  useEffect(() => {
    void api.getAppInfo().then(setInfo);
  }, []);

  return (
    <div className="space-y-2 text-sm text-fg-subtle">
      <p>
        <strong className="text-fg">{t("app-name", null, "Translator")}</strong>{" "}
        {t("settings-about-version-line", { version: info?.version ?? "..." })}
      </p>
      <p>{t("settings-about-built-with")}</p>
      <p>
        {t("settings-about-commit")}{" "}
        <code className="text-fg">{info?.commit ?? "..."}</code>
        {" · "}
        {t("settings-about-built")}{" "}
        <code className="text-fg">{info?.build_date ?? "..."}</code>
      </p>
      <p>
        {t("settings-about-source")}{" "}
        <a
          href={repoUrl || "#"}
          target="_blank"
          rel="noreferrer"
          className="text-accent underline"
          onClick={(event) => {
            if (!repoUrl) return;
            event.preventDefault();
            void api.openExternalUrl(repoUrl).catch(() => {
              window.open(repoUrl, "_blank", "noopener,noreferrer");
            });
          }}
        >
          {repoUrl || "..."}
        </a>
      </p>
    </div>
  );
}
