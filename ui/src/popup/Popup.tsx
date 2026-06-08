import { useEffect, useRef, useState } from "react";
import { Check, Copy, RotateCw, Square, Volume2, X } from "lucide-react";
import { useResultsStore } from "../stores/results";
import { useT } from "../i18n";
import { useTheme } from "../hooks/useTheme";
import { useConfigStore } from "../stores/config";
import * as api from "../ipc/commands";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { ServiceLogo } from "../services/ServiceLogo";
import type {
  DictionaryResult,
  ServiceOutcomeDto,
  TranslateResult,
} from "../types/bindings";

interface PopupGeometry {
  width: number;
  height: number;
  x: number;
  y: number;
}

interface HotkeyPayload {
  text: string | null;
  error: string | null;
}

// Reasonable default popup size; the backend may resize.
const DEFAULT_GEOMETRY: PopupGeometry = {
  width: 480,
  height: 320,
  x: 200,
  y: 200,
};

// BH-2.3: SPEC hard cap on source text length. Longer selections are
// truncated and a warning is added to the popup.
const MAX_SOURCE_CHARS = 100_000;

export function Popup() {
  const t = useT();
  const {
    outcomes,
    loading,
    error,
    queryText,
    setOutcomes,
    setPendingOutcomes,
    mergeOutcome,
    setLoading,
    setError,
    setQueryText,
    finishLoading,
    reset,
  } = useResultsStore();
  const { config } = useConfigStore();

  // BH-11.x: apply the resolved theme (system | light | dark) to <html>.
  // The popup window doesn't have a settings UI, but it should still
  // respect the user's theme choice (BH-11.1) and follow live OS changes
  // when set to "system" (BH-11.2).
  useTheme(
    (config?.general.theme as "system" | "light" | "dark" | undefined) ??
      "system",
  );

  // BH-2.3: surface a warning when the user's selection was truncated.
  const [truncation, setTruncation] = useState<{
    original: number;
    kept: number;
  } | null>(null);

  // Bumped on every (re)run so the pipeline can guard against stale resolutions.
  const runIdRef = useRef(0);
  const requestIdRef = useRef<string | null>(null);

  // Hold the latest `runPipeline` in a ref so hotkey listeners can
  // call it without depending on it in their effect dependency arrays.
  const runPipelineRef = useRef<(payload?: HotkeyPayload) => Promise<void>>(
    async () => {},
  );

  // The full translation pipeline. Reusable from both mount and hotkey event.
  const runPipeline = async (payload?: HotkeyPayload) => {
    const myRunId = ++runIdRef.current;
    requestIdRef.current = null;
    try {
      reset();
      setTruncation(null);
      setLoading(true);

      if (payload?.error) {
        setError(payload.error);
        return;
      }

      // 1. Use text captured before the popup took focus. The fallback keeps
      // direct/manual invocations working, but hotkey events should provide it.
      const text = payload?.text ?? (await api.getSelectedText());
      if (myRunId !== runIdRef.current) return;
      if (!text || !text.trim()) {
        setError("empty");
        return;
      }

      // 2. BH-2.3: truncate before translation so we never ship
      // 100k+ chars of source to a third-party API.
      const originalChars = text.length;
      const sent =
        originalChars > MAX_SOURCE_CHARS
          ? text.slice(0, MAX_SOURCE_CHARS)
          : text;
      if (sent.length !== originalChars) {
        setTruncation({ original: originalChars, kept: sent.length });
      }
      setQueryText(sent);

      // 3. Translate. The backend resolves source/target from preferences.
      const requestId = api.createRequestId();
      requestIdRef.current = requestId;
      const results = await api.translateText({
        text: sent,
        request_id: requestId,
      });
      if (myRunId !== runIdRef.current) return;
      setOutcomes(results);
    } catch (e) {
      if (myRunId === runIdRef.current) setError(String(e));
    }
  };

  // Keep the ref pointed at the latest `runPipeline` on every render.
  runPipelineRef.current = runPipeline;

  useEffect(() => {
    const unlistenStarted = api.onTranslationStarted((payload) => {
      if (payload.request_id !== requestIdRef.current) return;
      setPendingOutcomes(payload.outcomes);
    });
    const unlistenOutcome = api.onTranslationOutcome((payload) => {
      if (payload.request_id !== requestIdRef.current) return;
      mergeOutcome(payload.outcome);
    });
    const unlistenFinished = api.onTranslationFinished((payload) => {
      if (payload.request_id !== requestIdRef.current) return;
      finishLoading();
    });

    return () => {
      void Promise.all([
        unlistenStarted,
        unlistenOutcome,
        unlistenFinished,
      ]).then((unlisteners) => {
        for (const unlisten of unlisteners) unlisten();
      });
    };
  }, [finishLoading, mergeOutcome, setPendingOutcomes]);

  // Re-run on every hotkey press (consecutive presses pick up new selections).
  useEffect(() => {
    const unlistenPromise = listen<HotkeyPayload>(
      "translator://hotkey-pressed",
      ({ payload }) => {
        void runPipelineRef.current(payload);
      },
    );
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

    const cancelPendingHide = () => {
      if (hideTimer !== null) {
        clearTimeout(hideTimer);
        hideTimer = null;
      }
    };

    const unlistenPromise = win.onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        // Lost focus: restart the grace period from the latest event.
        cancelPendingHide();
        hideTimer = setTimeout(() => {
          hideTimer = null;
          void win.hide();
        }, 500);
      } else {
        cancelPendingHide();
      }
    });

    return () => {
      cancelPendingHide();
      void unlistenPromise.then((u) => u());
    };
  }, []);

  return (
    <div className="h-screen w-screen overflow-hidden rounded-lg border border-border bg-bg shadow-2xl">
      <div className="flex h-full flex-col">
        <Header
          onClose={() => void getCurrentWindow().hide()}
          onRetry={() =>
            void runPipelineRef.current(
              queryText ? { text: queryText, error: null } : undefined,
            )
          }
        />
        <div className="flex-1 overflow-y-auto p-3">
          {queryText && (
            <div className="mb-2 rounded-md border border-border bg-bg-subtle p-2 text-xs text-fg-subtle">
              <span className="font-medium text-fg">
                {t("popup-source", null, "source:")}
              </span>{" "}
              {queryText}
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
          {loading && outcomes.length === 0 && (
            <div className="py-4 text-center text-fg-subtle">
              {t("popup-loading")}
            </div>
          )}
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
          {error === "empty" && <ErrorView message={t("popup-empty")} />}
          {error && error !== "permission_denied" && error !== "empty" && (
            <ErrorView message={error} />
          )}
          {!loading && !error && queryText && outcomes.length === 0 && (
            <div className="py-4 text-center text-xs text-fg-subtle">
              {t("popup-no-services-enabled")}
            </div>
          )}
          {!error && outcomes.length > 0 && (
            <ResultsList outcomes={outcomes} />
          )}
        </div>
      </div>
    </div>
  );
}

function Header({
  onClose,
  onRetry,
}: {
  onClose: () => void;
  onRetry: () => void;
}) {
  const t = useT();
  return (
    <div className="flex items-center justify-between border-b border-border bg-bg-subtle px-3 py-1.5 text-xs">
      <span className="font-semibold text-fg">
        {t("app-name", null, "Translator")}
      </span>
      <div className="flex gap-1">
        <button
          className="icon-btn !h-7 !w-7"
          onClick={onRetry}
          title={t("popup-retry")}
          aria-label={t("popup-retry")}
        >
          <RotateCw size={14} aria-hidden="true" />
        </button>
        <button
          className="icon-btn !h-7 !w-7"
          onClick={onClose}
          title={t("popup-close")}
          aria-label={t("popup-close")}
        >
          <X size={14} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}

function ResultsList({ outcomes }: { outcomes: ServiceOutcomeDto[] }) {
  return (
    <div className="space-y-3">
      {outcomes.map((o) => (
        <ResultRow key={o.service_id} outcome={o} />
      ))}
    </div>
  );
}

function ResultRow({ outcome: o }: { outcome: ServiceOutcomeDto }) {
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
        <span className="inline-flex min-w-0 items-center gap-1.5 font-medium text-fg">
          <ServiceLogo serviceId={o.service_id} className="h-4 w-4" />
          <span className="min-w-0 truncate">{o.service_name}</span>
        </span>
        {o.result?.detected_source && (
          <span className="text-fg-subtle">
            {t(
              "popup-detected",
              { lang: o.result.detected_source },
              `detected: ${o.result.detected_source}`,
            )}
          </span>
        )}
      </div>
      {o.result ? (
        <ResultBody
          result={o.result}
          copied={copied}
          onCopied={() => setCopied(true)}
        />
      ) : o.error ? (
        <p className="text-xs text-red-500">{o.error.message}</p>
      ) : (
        <PendingResult />
      )}
    </div>
  );
}

function PendingResult() {
  const t = useT();
  return (
    <div className="flex items-center gap-2 text-xs text-fg-subtle">
      <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
      {t("popup-loading")}
    </div>
  );
}

function ResultBody({
  result,
  copied,
  onCopied,
}: {
  result: TranslateResult;
  copied: boolean;
  onCopied: () => void;
}) {
  const t = useT();
  return (
    <div className="space-y-2">
      <div className="flex items-start justify-between gap-2">
        <p className="min-w-0 whitespace-pre-wrap text-sm text-fg">
          {result.text}
        </p>
        <div className="flex shrink-0 items-center gap-1">
          <AudioButton url={result.audio_url ?? null} />
          <button
            className="icon-btn !h-7 !w-7"
            onClick={async () => {
              await api.copyToClipboard(result.text);
              onCopied();
            }}
            title={copied ? t("popup-copied", null, "Copied") : t("popup-copy")}
            aria-label={
              copied ? t("popup-copied", null, "Copied") : t("popup-copy")
            }
          >
            {copied ? (
              <Check size={14} aria-hidden="true" />
            ) : (
              <Copy size={14} aria-hidden="true" />
            )}
          </button>
        </div>
      </div>
      <DictionaryDetails dictionary={result.dictionary} />
    </div>
  );
}

function AudioButton({ url }: { url?: string | null }) {
  const t = useT();
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [playing, setPlaying] = useState(false);

  useEffect(() => {
    return () => {
      audioRef.current?.pause();
    };
  }, []);

  if (!url) return null;

  return (
    <button
      className="icon-btn !h-6 !w-6"
      title={t("popup-play-audio", null, "Play audio")}
      aria-label={t("popup-play-audio", null, "Play audio")}
      onClick={() => {
        if (playing) {
          audioRef.current?.pause();
          audioRef.current = null;
          setPlaying(false);
          return;
        }

        const audio = new Audio(url);
        audioRef.current = audio;
        audio.onended = () => setPlaying(false);
        audio.onerror = () => setPlaying(false);
        setPlaying(true);
        void audio.play().catch(() => setPlaying(false));
      }}
    >
      {playing ? (
        <Square size={10} aria-hidden="true" />
      ) : (
        <Volume2 size={12} aria-hidden="true" />
      )}
    </button>
  );
}

function DictionaryDetails({
  dictionary,
}: {
  dictionary?: DictionaryResult | null;
}) {
  const phonetics = dictionary?.phonetics ?? [];
  const parts = dictionary?.parts ?? [];
  const simpleWords = dictionary?.simple_words ?? [];
  const exchanges = dictionary?.exchanges ?? [];
  const tags = dictionary?.tags ?? [];

  if (
    phonetics.length === 0 &&
    parts.length === 0 &&
    simpleWords.length === 0 &&
    exchanges.length === 0 &&
    tags.length === 0
  ) {
    return null;
  }

  return (
    <div className="space-y-2 border-t border-border pt-2 text-xs">
      {phonetics.length > 0 && (
        <div className="flex flex-wrap gap-x-3 gap-y-1">
          {phonetics.map((phonetic, index) => (
            <span
              key={`${phonetic.label}-${index}`}
              className="inline-flex items-center gap-1"
            >
              <span className="font-medium text-fg-subtle">
                {phonetic.label}
              </span>
              {phonetic.value && (
                <span className="text-fg">/ {phonetic.value} /</span>
              )}
              <AudioButton url={phonetic.audio_url ?? null} />
            </span>
          ))}
        </div>
      )}

      {tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {tags.map((tag) => (
            <span
              key={tag}
              className="rounded border border-border px-1.5 py-0.5 text-fg-subtle"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {parts.length > 0 && (
        <div className="space-y-1">
          {parts.map((part, index) => (
            <div key={`${part.part ?? "part"}-${index}`} className="flex gap-2">
              {part.part && (
                <span className="w-9 shrink-0 font-medium text-fg-subtle">
                  {part.part}
                </span>
              )}
              <span className="min-w-0 text-fg">{part.means.join("; ")}</span>
            </div>
          ))}
        </div>
      )}

      {simpleWords.length > 0 && (
        <div className="space-y-1">
          {simpleWords.map((word, index) => (
            <div key={`${word.word}-${index}`} className="flex gap-2">
              <span className="min-w-[4rem] shrink-0 font-medium text-fg">
                {word.word}
              </span>
              <span className="min-w-0 text-fg-subtle">
                {[word.part, ...(word.means ?? [])].filter(Boolean).join("  ")}
              </span>
            </div>
          ))}
        </div>
      )}

      {exchanges.length > 0 && (
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-fg-subtle">
          {exchanges.map((exchange) => (
            <span key={exchange.name}>
              {exchange.name}: {exchange.words.join(", ")}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function ErrorView({
  message,
  actionLabel,
  onAction,
}: {
  message: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  return (
    <div className="space-y-2 py-4 text-center text-sm text-fg-subtle">
      <p>{message}</p>
      {actionLabel && onAction && (
        <button className="btn" onClick={onAction} aria-label={actionLabel}>
          {actionLabel}
        </button>
      )}
    </div>
  );
}

// Re-export so the unused import in the file is kept.
export { DEFAULT_GEOMETRY };
