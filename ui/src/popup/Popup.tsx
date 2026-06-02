import { useEffect, useRef, useState } from "react";
import { useResultsStore } from "../stores/results";
import { useT } from "../i18n";
import { useTheme } from "../hooks/useTheme";
import { useConfigStore } from "../stores/config";
import * as api from "../ipc/commands";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

interface PopupGeometry {
  width: number;
  height: number;
  x: number;
  y: number;
}

// Reasonable default popup size; the backend may resize.
const DEFAULT_GEOMETRY: PopupGeometry = { width: 480, height: 320, x: 200, y: 200 };

// BH-2.3: SPEC hard cap on source text length. Longer selections are
// truncated and a warning is added to the popup.
const MAX_SOURCE_CHARS = 100_000;

export function Popup() {
  const t = useT();
  const { outcomes, loading, error, queryText, setOutcomes, setLoading, setError, setQueryText, reset } =
    useResultsStore();
  const { config } = useConfigStore();

  // BH-11.x: apply the resolved theme (system | light | dark) to <html>.
  // The popup window doesn't have a settings UI, but it should still
  // respect the user's theme choice (BH-11.1) and follow live OS changes
  // when set to "system" (BH-11.2).
  useTheme((config?.general.theme as "system" | "light" | "dark" | undefined) ?? "system");

  // BH-2.3: surface a warning when the user's selection was truncated.
  const [truncation, setTruncation] = useState<{ original: number; kept: number } | null>(null);

  // Bumped on every (re)run so the pipeline can guard against stale resolutions.
  const runIdRef = useRef(0);

  // Hold the latest `runPipeline` in a ref so mount + hotkey listeners can
  // call it without depending on it in their effect dependency arrays.
  const runPipelineRef = useRef<() => Promise<void>>(async () => {});

  // The full translation pipeline. Reusable from both mount and hotkey event.
  const runPipeline = async () => {
    const myRunId = ++runIdRef.current;
    try {
      setLoading(true);
      setError(null);
      setOutcomes([]);
      setQueryText("");
      setTruncation(null);

      // 1. Permission gate.
      const ok = await api.checkPermission();
      if (myRunId !== runIdRef.current) return;
      if (!ok) {
        setError("permission_denied");
        return;
      }

      // 2. Read selected text.
      const text = await api.getSelectedText();
      if (myRunId !== runIdRef.current) return;
      if (!text || !text.trim()) {
        setError("empty");
        return;
      }

      // 3. BH-2.3: truncate before translation so we never ship
      // 100k+ chars of source to a third-party API.
      const originalChars = text.length;
      const sent = originalChars > MAX_SOURCE_CHARS ? text.slice(0, MAX_SOURCE_CHARS) : text;
      if (sent.length !== originalChars) {
        setTruncation({ original: originalChars, kept: sent.length });
      }
      setQueryText(sent);

      // 4. Translate.
      const config = await api.getConfig();
      const to = config.general.target_language;
      const from = config.general.default_from === "auto" ? null : config.general.default_from;
      const results = await api.translateText({ text: sent, from, to });
      if (myRunId !== runIdRef.current) return;
      setOutcomes(results);
    } catch (e) {
      if (myRunId === runIdRef.current) setError(String(e));
    }
  };

  // Keep the ref pointed at the latest `runPipeline` on every render.
  runPipelineRef.current = runPipeline;

  // Run on mount.
  useEffect(() => {
    void runPipelineRef.current();
  }, []);

  // Re-run on every hotkey press (consecutive presses pick up new selections).
  useEffect(() => {
    const unlistenPromise = listen("translator://hotkey-pressed", () => {
      void runPipelineRef.current();
    });
    return () => {
      void unlistenPromise.then((u) => u());
    };
  }, []);

  // Close on Escape.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        void getCurrentWindow().hide();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // BH-5.8: clicking outside the popup hides it after a 500 ms grace period.
  // If focus returns to the popup before the timer fires, the hide is cancelled.
  useEffect(() => {
    const win = getCurrentWindow();
    let hideTimer: ReturnType<typeof setTimeout> | null = null;

    const unlistenPromise = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        // Lost focus: start the grace period.
        hideTimer = setTimeout(() => {
          void win.hide();
        }, 500);
      } else if (hideTimer !== null) {
        // Focus returned within the grace period: cancel the hide.
        clearTimeout(hideTimer);
        hideTimer = null;
      }
    });

    return () => {
      if (hideTimer !== null) {
        clearTimeout(hideTimer);
      }
      void unlistenPromise.then((u) => u());
    };
  }, []);

  return (
    <div className="h-screen w-screen overflow-hidden rounded-xl border border-border bg-bg shadow-2xl">
      <div className="flex h-full flex-col">
        <Header onClose={() => void getCurrentWindow().hide()} onRetry={reset} />
        <div className="flex-1 overflow-y-auto p-3">
          {queryText && (
            <div className="mb-2 rounded-md border border-border bg-bg-subtle p-2 text-xs text-fg-subtle">
              <span className="font-medium text-fg">{t("popup-source", null, "source:")}</span> {queryText}
              {truncation && (
                <div className="mt-1 text-yellow-500">
                  {t(
                    "popup-truncated",
                    { kept: truncation.kept, original: truncation.original },
                    `Selection truncated: ${truncation.kept} of ${truncation.original} characters kept (100 000-char limit).`,
                  )}
                </div>
              )}
            </div>
          )}
          {loading && <div className="py-4 text-center text-fg-subtle">{t("popup-loading")}</div>}
          {error === "permission_denied" && (
            <ErrorView
              message={t("popup-permission-denied")}
              actionLabel={t("popup-open-settings")}
              onAction={async () => {
                await api.openPermissionSettings();
                void invoke("open_settings");
              }}
            />
          )}
          {error === "empty" && (
            <ErrorView message={t("popup-empty")} />
          )}
          {error && error !== "permission_denied" && error !== "empty" && (
            <ErrorView message={error} />
          )}
          {!loading && !error && queryText && outcomes.length === 0 && (
            <div className="py-4 text-center text-xs text-fg-subtle">
              {t("popup-no-services-enabled")}
            </div>
          )}
          {!loading && !error && outcomes.length > 0 && (
            <ResultsList outcomes={outcomes} />
          )}
        </div>
      </div>
    </div>
  );
}

function Header({ onClose, onRetry }: { onClose: () => void; onRetry: () => void }) {
  const t = useT();
  return (
    <div className="flex items-center justify-between border-b border-border bg-bg-subtle px-3 py-1.5 text-xs">
      <span className="font-semibold text-fg">{t("app-name", null, "translator")}</span>
      <div className="flex gap-1">
        <button
          className="btn !px-2 !py-0.5 text-xs"
          onClick={onRetry}
          title={t("popup-retry")}
          aria-label={t("popup-retry")}
        >
          ↻
        </button>
        <button
          className="btn !px-2 !py-0.5 text-xs"
          onClick={onClose}
          title={t("popup-close")}
          aria-label={t("popup-close")}
        >
          ×
        </button>
      </div>
    </div>
  );
}

function ResultsList({ outcomes }: { outcomes: import("../types/bindings").ServiceOutcomeDto[] }) {
  return (
    <div className="space-y-3">
      {outcomes.map((o) => (
        <ResultRow key={o.service_id} outcome={o} />
      ))}
    </div>
  );
}

function ResultRow({ outcome: o }: { outcome: import("../types/bindings").ServiceOutcomeDto }) {
  const t = useT();
  // BH-6.1: Copy button briefly flashes "Copied" after a successful clipboard write.
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const tmr = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(tmr);
  }, [copied]);

  return (
    <div className="rounded-md border border-border p-2">
      <div className="mb-1 flex items-center justify-between text-xs">
        <span className="font-medium text-fg">{o.service_name}</span>
        {o.result?.detected_source && (
          <span className="text-fg-subtle">
            {t("popup-detected", { lang: o.result.detected_source }, `detected: ${o.result.detected_source}`)}
          </span>
        )}
      </div>
      {o.result ? (
        <div className="flex items-start justify-between gap-2">
          <p className="whitespace-pre-wrap text-sm text-fg">{o.result.text}</p>
          <button
            className="btn !px-2 !py-0.5 text-xs"
            onClick={async () => {
              await api.copyToClipboard(o.result!.text);
              setCopied(true);
            }}
          >
            {copied ? t("popup-copied", null, "Copied") : t("popup-copy")}
          </button>
        </div>
      ) : o.error ? (
        <p className="text-xs text-red-500">{o.error.message}</p>
      ) : null}
    </div>
  );
}

function ErrorView({ message, actionLabel, onAction }: { message: string; actionLabel?: string; onAction?: () => void }) {
  return (
    <div className="space-y-2 py-4 text-center text-sm text-fg-subtle">
      <p>{message}</p>
      {actionLabel && onAction && (
        <button className="btn" onClick={onAction}>{actionLabel}</button>
      )}
    </div>
  );
}

// Re-export so the unused import in the file is kept.
export { DEFAULT_GEOMETRY };
