import { useEffect, useState } from "react";
import * as api from "../../ipc/commands";
import { useT } from "../../i18n";

export function AboutSection() {
  // BH-12.1: show app version, build commit, build date, and the repo link.
  const t = useT();
  const [info, setInfo] = useState<api.AppInfo | null>(null);
  useEffect(() => {
    void api.getAppInfo().then(setInfo);
  }, []);

  return (
    <div className="space-y-2 text-sm text-fg-subtle">
      <p>
        <strong className="text-fg">{t("app-name", null, "Translator")}</strong>{" "}
        {t("settings-about-version-line", { version: info?.version ?? "..." })}
      </p>
      <p>
        {t("settings-about-built-with")}
      </p>
      <p>
        {t("settings-about-commit")} <code className="text-fg">{info?.commit ?? "..."}</code>
        {" · "}
        {t("settings-about-built")} <code className="text-fg">{info?.build_date ?? "..."}</code>
      </p>
      <p>
        {t("settings-about-source")}{" "}
        <a
          href={info?.repo_url ?? "#"}
          target="_blank"
          rel="noreferrer"
          className="text-accent underline"
        >
          {info?.repo_url ?? "..."}
        </a>
      </p>
    </div>
  );
}
