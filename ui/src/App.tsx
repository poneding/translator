import { useEffect, useMemo, useRef, useState } from "react";
import {
  Check,
  Copy,
  History as HistoryIcon,
  Maximize2,
  Minus,
  Settings,
  Square,
  Trash2,
  Volume2,
  X,
} from "lucide-react";
import { useConfigStore } from "./stores/config";
import { useT } from "./i18n";
import { useTheme } from "./hooks/useTheme";
import * as api from "./ipc/commands";
import { ServiceLogo } from "./services/ServiceLogo";
import { Combobox } from "./components/Combobox";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform as getPlatform, type Platform } from "@tauri-apps/plugin-os";
import type {
  DictionaryResult,
  HistoryItem,
  ServiceOutcomeDto,
  TranslateResult,
} from "./types/bindings";

type HostPlatform = Platform | "unknown";

const LANGS: Array<{ code: string; labelKey: string; fallback: string }> = [
  { code: "auto", labelKey: "lang-auto", fallback: "Auto" },
  { code: "en", labelKey: "lang-en", fallback: "English" },
  { code: "zh-Hans", labelKey: "lang-zh-hans", fallback: "Simplified Chinese" },
  {
    code: "zh-Hant",
    labelKey: "lang-zh-hant",
    fallback: "Traditional Chinese",
  },
  { code: "ja", labelKey: "lang-ja", fallback: "Japanese" },
  { code: "ko", labelKey: "lang-ko", fallback: "Korean" },
  { code: "fr", labelKey: "lang-fr", fallback: "French" },
  { code: "de", labelKey: "lang-de", fallback: "German" },
  { code: "es", labelKey: "lang-es", fallback: "Spanish" },
  { code: "ru", labelKey: "lang-ru", fallback: "Russian" },
];

export function App() {
  const { config, load, loading, error, setConfig } = useConfigStore();
  const t = useT();
  const [text, setText] = useState("");
  const [from, setFrom] = useState("auto");
  const [to, setTo] = useState("auto");
  const [outcomes, setOutcomes] = useState<ServiceOutcomeDto[]>([]);
  const [busy, setBusy] = useState(false);
  const [translateError, setTranslateError] = useState<string | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const requestIdRef = useRef<string | null>(null);

  useTheme(
    (config?.general.theme as "system" | "light" | "dark" | undefined) ??
      "system",
  );

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const win = getCurrentWindow();
    const unlistenPromise = win.onCloseRequested((event) => {
      event.preventDefault();
      void win.hide();
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const unlistenPromise = api.onConfigUpdated((nextConfig) => {
      setConfig(nextConfig);
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [setConfig]);

  useEffect(() => {
    const unlistenStarted = api.onTranslationStarted((payload) => {
      if (payload.request_id !== requestIdRef.current) return;
      setOutcomes(payload.outcomes);
      setBusy(payload.outcomes.length > 0);
      setTranslateError(null);
    });
    const unlistenOutcome = api.onTranslationOutcome((payload) => {
      if (payload.request_id !== requestIdRef.current) return;
      setOutcomes((current) => upsertOutcome(current, payload.outcome));
      setTranslateError(null);
    });
    const unlistenFinished = api.onTranslationFinished((payload) => {
      if (payload.request_id !== requestIdRef.current) return;
      setBusy(false);
      void load();
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
  }, [load]);

  const enabledServices = useMemo(() => {
    if (!config) return [];
    return Object.entries(config.services)
      .map(([id, service]) => ({ ...service, id }))
      .filter((service) => service.enabled)
      .sort((a, b) => a.priority - b.priority);
  }, [config]);

  const languageOptions = useMemo(
    () =>
      LANGS.map((lang) => ({
        value: lang.code,
        label: `${t(lang.labelKey, null, lang.fallback)} (${lang.code})`,
      })),
    [t],
  );

  const runTranslate = async (nextText = text) => {
    const source = nextText.trim();
    if (!source) {
      setTranslateError(
        t("main-error-empty", null, "Enter text to translate."),
      );
      setOutcomes([]);
      return;
    }
    const requestId = api.createRequestId();
    requestIdRef.current = requestId;
    setBusy(true);
    setTranslateError(null);
    setOutcomes([]);
    try {
      const result = await api.translateText({
        text: source,
        from: from === "auto" ? null : from,
        to: to === "auto" ? null : to,
        request_id: requestId,
      });
      if (requestIdRef.current === requestId) {
        setOutcomes(result);
        await load();
      }
    } catch (err) {
      if (requestIdRef.current === requestId) {
        setTranslateError(String(err));
        setOutcomes([]);
      }
    } finally {
      if (requestIdRef.current === requestId) setBusy(false);
    }
  };

  const pasteAndTranslate = async () => {
    setBusy(true);
    setTranslateError(null);
    try {
      const clip = await api.readClipboard();
      setText(clip);
      await runTranslate(clip);
    } catch (err) {
      setTranslateError(String(err));
    } finally {
      setBusy(false);
    }
  };

  if (loading && !config) {
    return (
      <div className="p-8 text-fg-subtle">
        {t("common-loading", null, "Loading...")}
      </div>
    );
  }
  if (error) {
    return (
      <div className="p-8 text-red-500">
        {t(
          "common-load-config-failed",
          { msg: error },
          `Failed to load config: ${error}`,
        )}
      </div>
    );
  }
  if (!config) return null;

  return (
    <div className="flex h-full min-h-0 flex-col bg-bg text-fg">
      <AppTitleBar
        historyOpen={historyOpen}
        onToggleHistory={() => setHistoryOpen((open) => !open)}
        onOpenSettings={() => void api.openSettings()}
      />

      <main
        className={
          "grid min-h-0 flex-1 gap-4 p-4 " +
          (historyOpen ? "grid-cols-[minmax(0,1fr)_280px]" : "grid-cols-1")
        }
      >
        <section className="flex min-h-0 flex-col gap-3">
          <div className="flex items-center gap-2">
            <Combobox
              className="flex-1"
              label={t("main-source-language", null, "Source")}
              options={languageOptions}
              value={from}
              onChange={setFrom}
            />
            <Combobox
              className="flex-1"
              label={t("main-target-language", null, "Target")}
              options={languageOptions}
              value={to}
              onChange={setTo}
            />
          </div>

          <textarea
            className="min-h-[180px] resize-none rounded-lg border border-border bg-bg p-3 text-sm text-fg outline-none focus:border-accent focus:ring-2 focus:ring-accent/30"
            value={text}
            placeholder={t(
              "main-input-placeholder",
              null,
              "Type or paste text here...",
            )}
            onChange={(event) => setText(event.target.value)}
          />

          <div className="flex flex-wrap items-center gap-2">
            <button
              className="btn btn-primary"
              disabled={busy}
              onClick={() => void runTranslate()}
            >
              {busy
                ? t("main-translating", null, "Translating...")
                : t("main-translate", null, "Translate")}
            </button>
            <button
              className="btn"
              disabled={busy}
              onClick={() => void pasteAndTranslate()}
            >
              {t("main-clipboard-translate", null, "Clipboard Translate")}
            </button>
            <button
              className="btn"
              disabled={!text}
              onClick={() => setText("")}
            >
              {t("main-clear", null, "Clear")}
            </button>
            <span className="ml-auto inline-flex min-w-0 items-center gap-2 text-xs text-fg-subtle">
              {enabledServices.length > 0
                ? t(
                    "main-enabled-services",
                    { count: enabledServices.length },
                    `${enabledServices.length} services enabled`,
                  )
                : t(
                    "popup-no-services-enabled",
                    null,
                    "No services enabled. Open Settings to enable at least one.",
                  )}
              {enabledServices.length > 0 && (
                <span className="inline-flex shrink-0 items-center -space-x-1">
                  {enabledServices.map((service) => (
                    <ServiceLogo
                      key={service.id}
                      serviceId={service.id}
                      className="h-5 w-5 rounded-full ring-2 ring-bg"
                    />
                  ))}
                </span>
              )}
            </span>
          </div>

          {translateError && (
            <div className="rounded-lg border border-red-500 bg-red-500/10 p-3 text-sm text-red-500">
              {translateError}
            </div>
          )}

          <ResultsPanel outcomes={outcomes} busy={busy} />
        </section>

        {historyOpen && (
          <HistoryPanel
            history={config.history}
            onClear={async () => {
              await api.clearHistory();
              await load();
            }}
            onPick={(item) => {
              setText(item.source_text);
              setFrom(item.from || "auto");
              setTo(item.to);
              setOutcomes([
                {
                  service_id:
                    item.service_id as ServiceOutcomeDto["service_id"],
                  service_name: item.service_name,
                  result: {
                    service_id:
                      item.service_id as ServiceOutcomeDto["service_id"],
                    service_name: item.service_name,
                    text: item.translated_text,
                    detected_source: item.from === "auto" ? null : item.from,
                    elapsed_ms: 0,
                  },
                  error: null,
                },
              ]);
            }}
          />
        )}
      </main>
    </div>
  );
}

function AppTitleBar({
  historyOpen,
  onToggleHistory,
  onOpenSettings,
}: {
  historyOpen: boolean;
  onToggleHistory: () => void;
  onOpenSettings: () => void;
}) {
  const t = useT();
  const [hostPlatform] = useState<HostPlatform>(() => detectHostPlatform());
  const isMac = hostPlatform === "macos";

  return (
    <header
      className={"app-titlebar " + (isMac ? "app-titlebar-mac" : "")}
      data-tauri-drag-region
      onDoubleClick={(event) => {
        const target = event.target as HTMLElement;
        if (target.closest("button")) return;
        void runWindowAction((win) => win.toggleMaximize());
      }}
    >
      {isMac && <MacWindowControls />}
      <div className="titlebar-title" data-tauri-drag-region>
        <h1 data-tauri-drag-region>{t("app-name", null, "Translator")}</h1>
        <p data-tauri-drag-region>
          {t(
            "main-subtitle",
            null,
            "Fast text, clipboard, and selection translation",
          )}
        </p>
      </div>
      <div className="titlebar-actions">
        <button
          className={"icon-btn !h-7 !w-7 " + (historyOpen ? "btn-primary" : "")}
          onClick={onToggleHistory}
          title={t("main-history", null, "History")}
          aria-label={t("main-history", null, "History")}
        >
          <HistoryIcon size={15} aria-hidden="true" />
        </button>
        <button
          className="icon-btn !h-7 !w-7"
          onClick={onOpenSettings}
          title={t("main-open-settings", null, "Settings")}
          aria-label={t("main-open-settings", null, "Settings")}
        >
          <Settings size={15} aria-hidden="true" />
        </button>
      </div>
      {!isMac && <WindowsWindowControls />}
    </header>
  );
}

function MacWindowControls() {
  const t = useT();
  return (
    <div className="mac-window-controls">
      <button
        className="mac-window-control mac-window-close"
        onClick={() => void runWindowAction((win) => win.close())}
        title={t("popup-close", null, "Close")}
        aria-label={t("popup-close", null, "Close")}
      >
        <X size={8} aria-hidden="true" />
      </button>
      <button
        className="mac-window-control mac-window-minimize"
        onClick={() => void runWindowAction((win) => win.minimize())}
        title="Minimize"
        aria-label="Minimize"
      >
        <Minus size={8} aria-hidden="true" />
      </button>
      <button
        className="mac-window-control mac-window-maximize"
        onClick={() => void runWindowAction((win) => win.toggleMaximize())}
        title="Maximize"
        aria-label="Maximize"
      >
        <Maximize2 size={7} aria-hidden="true" />
      </button>
    </div>
  );
}

function WindowsWindowControls() {
  const t = useT();
  return (
    <div className="windows-window-controls">
      <button
        className="window-control"
        onClick={() => void runWindowAction((win) => win.minimize())}
        title="Minimize"
        aria-label="Minimize"
      >
        <Minus size={14} aria-hidden="true" />
      </button>
      <button
        className="window-control"
        onClick={() => void runWindowAction((win) => win.toggleMaximize())}
        title="Maximize"
        aria-label="Maximize"
      >
        <Maximize2 size={13} aria-hidden="true" />
      </button>
      <button
        className="window-control window-control-close"
        onClick={() => void runWindowAction((win) => win.close())}
        title={t("popup-close", null, "Close")}
        aria-label={t("popup-close", null, "Close")}
      >
        <X size={15} aria-hidden="true" />
      </button>
    </div>
  );
}

function detectHostPlatform(): HostPlatform {
  try {
    return getPlatform();
  } catch {
    const platform = navigator.platform.toLowerCase();
    if (platform.includes("mac")) return "macos";
    if (platform.includes("win")) return "windows";
    if (platform.includes("linux")) return "linux";
    return "unknown";
  }
}

async function runWindowAction(
  action: (win: ReturnType<typeof getCurrentWindow>) => Promise<void>,
) {
  try {
    await action(getCurrentWindow());
  } catch {
    // Window controls are inert in a plain browser preview.
  }
}

function upsertOutcome(
  outcomes: ServiceOutcomeDto[],
  outcome: ServiceOutcomeDto,
): ServiceOutcomeDto[] {
  const index = outcomes.findIndex(
    (item) => item.service_id === outcome.service_id,
  );
  if (index === -1) return [...outcomes, outcome];
  const next = [...outcomes];
  next[index] = outcome;
  return next;
}

function ResultsPanel({
  outcomes,
  busy,
}: {
  outcomes: ServiceOutcomeDto[];
  busy: boolean;
}) {
  const t = useT();
  if (outcomes.length === 0) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center rounded-lg border border-dashed border-border text-sm text-fg-subtle">
        {busy
          ? t("common-loading", null, "Loading...")
          : t(
              "main-results-empty",
              null,
              "Translation results will appear here.",
            )}
      </div>
    );
  }

  return (
    <div className="min-h-0 flex-1 space-y-3 overflow-y-auto">
      {outcomes.map((outcome) => (
        <ResultCard key={outcome.service_id} outcome={outcome} />
      ))}
    </div>
  );
}

function ResultCard({ outcome }: { outcome: ServiceOutcomeDto }) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1200);
    return () => window.clearTimeout(timer);
  }, [copied]);

  return (
    <article className="rounded-lg border border-border bg-bg-subtle p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <ServiceLogo serviceId={outcome.service_id} className="h-6 w-6" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-semibold">
              {outcome.service_name}
            </h2>
            {outcome.result?.detected_source && (
              <p className="text-xs text-fg-subtle">
                {t(
                  "popup-detected",
                  { lang: outcome.result.detected_source },
                  `detected: ${outcome.result.detected_source}`,
                )}
              </p>
            )}
          </div>
        </div>
      </div>
      {outcome.result ? (
        <ResultBody
          result={outcome.result}
          copied={copied}
          onCopied={() => setCopied(true)}
        />
      ) : outcome.error ? (
        <p className="text-sm text-red-500">
          {outcome.error.message}
        </p>
      ) : (
        <PendingResult />
      )}
    </article>
  );
}

function PendingResult() {
  const t = useT();
  return (
    <div className="flex items-center gap-2 text-sm text-fg-subtle">
      <span className="h-2 w-2 rounded-full bg-accent animate-pulse" />
      {t("common-loading", null, "Loading...")}
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
    <div className="space-y-3">
      <div className="flex items-start justify-between gap-3">
        <p className="min-w-0 whitespace-pre-wrap text-sm leading-6">
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
            title={
              copied
                ? t("popup-copied", null, "Copied")
                : t("popup-copy", null, "Copy")
            }
            aria-label={
              copied
                ? t("popup-copied", null, "Copied")
                : t("popup-copy", null, "Copy")
            }
          >
            {copied ? (
              <Check size={15} aria-hidden="true" />
            ) : (
              <Copy size={15} aria-hidden="true" />
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
      className="icon-btn !h-7 !w-7"
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
        <Volume2 size={13} aria-hidden="true" />
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
                <span className="w-10 shrink-0 font-medium text-fg-subtle">
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
              <span className="min-w-[5rem] shrink-0 font-medium text-fg">
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

function HistoryPanel({
  history,
  onClear,
  onPick,
}: {
  history: HistoryItem[];
  onClear: () => Promise<void>;
  onPick: (item: HistoryItem) => void;
}) {
  const t = useT();
  return (
    <aside className="flex min-h-0 flex-col rounded-lg border border-border bg-bg-subtle">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <h2 className="text-sm font-semibold">
          {t("main-history", null, "History")}
        </h2>
        <button
          className="icon-btn !h-7 !w-7"
          disabled={history.length === 0}
          onClick={() => void onClear()}
          title={t("main-clear-history", null, "Clear")}
          aria-label={t("main-clear-history", null, "Clear")}
        >
          <Trash2 size={14} aria-hidden="true" />
        </button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {history.length === 0 ? (
          <p className="p-3 text-xs text-fg-subtle">
            {t(
              "main-history-empty",
              null,
              "Successful translations are saved here.",
            )}
          </p>
        ) : (
          <div className="space-y-2">
            {history.map((item) => (
              <button
                key={item.id}
                className="block w-full rounded-md border border-border bg-bg p-2 text-left hover:border-accent"
                onClick={() => onPick(item)}
              >
                <p className="line-clamp-2 text-xs text-fg">
                  {item.source_text}
                </p>
                <p className="mt-1 line-clamp-2 text-xs text-fg-subtle">
                  {item.translated_text}
                </p>
                <p className="mt-1 text-[11px] text-fg-subtle">
                  {item.service_name}
                </p>
              </button>
            ))}
          </div>
        )}
      </div>
    </aside>
  );
}
