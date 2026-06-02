import { useEffect, useState } from "react";
import * as api from "../../ipc/commands";

export function AboutSection() {
  // BH-12.1: show app version, build commit, build date, and the repo link.
  const [info, setInfo] = useState<api.AppInfo | null>(null);
  useEffect(() => {
    void api.getAppInfo().then(setInfo);
  }, []);

  return (
    <div className="space-y-2 text-sm text-fg-subtle">
      <p>
        <strong className="text-fg">translator</strong>{" "}
        v{info?.version ?? "…"} — cross-platform select-and-translate.
      </p>
      <p>
        Built with Rust, Tauri 2, and React. MIT-licensed core, GPL-3.0 for the app.
      </p>
      <p>
        Commit: <code className="text-fg">{info?.commit ?? "…"}</code>
        {" · "}
        Built: <code className="text-fg">{info?.build_date ?? "…"}</code>
      </p>
      <p>
        Source:{" "}
        <a
          href={info?.repo_url ?? "#"}
          target="_blank"
          rel="noreferrer"
          className="text-accent underline"
        >
          {info?.repo_url ?? "…"}
        </a>
      </p>
    </div>
  );
}
