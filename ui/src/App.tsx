import { getCurrentWindow } from "@tauri-apps/api/window";
import { platform as getPlatform, type Platform } from "@tauri-apps/plugin-os";
import {
  ArrowRight,
  Check,
  Copy,
  History as HistoryIcon,
  Minus,
  Pin,
  PinOff,
  RefreshCw,
  Settings,
  Trash2,
  Volume1,
  Volume2,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Combobox, type ComboboxOption } from "./components/Combobox";
import { useTheme } from "./hooks/useTheme";
import { setLocale, useT } from "./i18n";
import {
  autoLanguageParts,
  languagePartsForCode,
  translationLanguageOptions,
  type TFunction,
} from "./i18n/languages";
import * as api from "./ipc/commands";
import { SettingsView } from "./SettingsApp";
import { ServiceLogo } from "./services/ServiceLogo";
import { serviceMeta } from "./services/serviceMeta";
import { useConfigStore } from "./stores/config";
import type {
  GeneralConfig,
  DictionaryResult,
  HistoryItem,
  ServiceId,
  ServiceOutcomeDto,
  TranslateResult,
  WordPhonetic,
} from "./types/bindings";

type HostPlatform = Platform | "unknown";
type AppView = "main" | "settings";

const SOURCE_MIN_HEIGHT = 108;
const SOURCE_MAX_HEIGHT = 156;

export function App() {
  const { config, load, loading, error, save, setConfig } = useConfigStore();
  const t = useT();
  const [text, setText] = useState("");
  const [to, setTo] = useState("zh-Hans");
  const [targetManual, setTargetManual] = useState(false);
  const [outcomes, setOutcomes] = useState<ServiceOutcomeDto[]>([]);
  const [busy, setBusy] = useState(false);
  const [translateError, setTranslateError] = useState<string | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [activeView, setActiveView] = useState<AppView>("main");
  const [refreshingServices, setRefreshingServices] = useState<Set<ServiceId>>(
    () => new Set(),
  );
  const requestIdRef = useRef<string | null>(null);
  const serviceRequestIdsRef = useRef<Partial<Record<ServiceId, string>>>({});

  useTheme(
    (config?.general.theme as "system" | "light" | "dark" | undefined) ??
      "system",
  );

  useEffect(() => {
    setLocale(config?.general.app_language ?? "system");
  }, [config?.general.app_language]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!config || targetManual) return;
    setTo(
      targetLanguageFromOutcomes(outcomes) ??
        resolveAutoTargetLanguage(config.general, text),
    );
  }, [config, outcomes, targetManual, text]);

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
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      const target = event.target as HTMLElement | null;
      if (target?.closest('[role="dialog"], [role="listbox"]')) return;
      event.preventDefault();
      void getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
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
    const unlistenSettingsPromise = api.onOpenSettingsRequested(() => {
      setHistoryOpen(false);
      setActiveView("settings");
    });
    const unlistenMainPromise = api.onOpenMainRequested(() => {
      setHistoryOpen(false);
      setActiveView("main");
    });
    return () => {
      void Promise.all([unlistenSettingsPromise, unlistenMainPromise]).then(
        (unlisteners) => {
          for (const unlisten of unlisteners) unlisten();
        },
      );
    };
  }, []);

  const enabledServices = useMemo(() => {
    if (!config) return [];
    return Object.values(config.services)
      .filter((service) => service.enabled)
      .sort((a, b) => a.priority - b.priority);
  }, [config]);

  const targetLanguageOptions = useMemo(
    () => withCurrentOption(withoutOptionFlags(translationLanguageOptions(t)), to),
    [t, to],
  );

  const detectedSource = useMemo(
    () =>
      outcomes.find((outcome) => outcome.result?.detected_source)?.result
        ?.detected_source ?? null,
    [outcomes],
  );
  const sourceDictionary = useMemo(() => {
    for (const outcome of outcomes) {
      const dictionary =
        outcome.result?.source_dictionary ?? outcome.result?.dictionary;
      if ((dictionary?.phonetics?.length ?? 0) > 0) return dictionary;
    }
    return null;
  }, [outcomes]);

  const runTranslate = useCallback(
    async (nextText = text) => {
      if (!config) return;
      const source = nextText.trim();
      if (!source) {
        setTranslateError(
          t("main-error-empty", null, "Enter text to translate."),
        );
        setOutcomes([]);
        return;
      }

      const requestId = api.createRequestId();
      const targetOverride = targetManual && to !== "auto" ? to : null;
      requestIdRef.current = requestId;
      serviceRequestIdsRef.current = {};
      setBusy(true);
      setTranslateError(null);
      setOutcomes([]);
      if (!targetManual) {
        setTo(resolveAutoTargetLanguage(config.general, source));
      }

      try {
        const result = await api.translateText({
          text: source,
          from: null,
          to: targetOverride,
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
    },
    [config, load, t, targetManual, text, to],
  );

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

  useEffect(() => {
    const unlistenPromise = api.onHotkeySource((payload) => {
      setHistoryOpen(false);
      setActiveView("main");

      if (payload.error) {
        requestIdRef.current = null;
        setBusy(false);
        setOutcomes([]);
        setTranslateError(hotkeyErrorMessage(payload.error, t));
        if (payload.text) setText(payload.text);
        return;
      }

      const source = payload.text?.trim() ? payload.text : "";
      setText(source);
      setTranslateError(null);
      if (source) {
        void runTranslate(source);
      } else {
        requestIdRef.current = null;
        setBusy(false);
        setOutcomes([]);
      }
    });
    return () => {
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [runTranslate, t]);

  const refreshService = useCallback(
    async (serviceId: ServiceId) => {
      const source = text.trim();
      if (!source) {
        setTranslateError(
          t("main-error-empty", null, "Enter text to translate."),
        );
        return;
      }

      const requestId = api.createRequestId();
      serviceRequestIdsRef.current[serviceId] = requestId;
      setTranslateError(null);
      setRefreshingServices((current) => {
        const next = new Set(current);
        next.add(serviceId);
        return next;
      });
      setOutcomes((current) =>
        upsertOutcome(current, {
          service_id: serviceId,
          service_name:
            current.find((item) => item.service_id === serviceId)
              ?.service_name ?? serviceMeta(serviceId).name,
          result: null,
          error: null,
        }),
      );

      try {
        const outcome = await api.translateService({
          service_id: serviceId,
          text: source,
          from: null,
          to: targetManual && to !== "auto" ? to : null,
          request_id: requestId,
        });
        if (serviceRequestIdsRef.current[serviceId] === requestId) {
          setOutcomes((current) => upsertOutcome(current, outcome));
        }
      } catch (err) {
        if (serviceRequestIdsRef.current[serviceId] === requestId) {
          setOutcomes((current) =>
            upsertOutcome(current, {
              service_id: serviceId,
              service_name:
                current.find((item) => item.service_id === serviceId)
                  ?.service_name ?? serviceMeta(serviceId).name,
              result: null,
              error: {
                code: "frontend",
                message: String(err),
              },
            }),
          );
        }
      } finally {
        if (serviceRequestIdsRef.current[serviceId] === requestId) {
          delete serviceRequestIdsRef.current[serviceId];
          setRefreshingServices((current) => {
            const next = new Set(current);
            next.delete(serviceId);
            return next;
          });
        }
      }
    },
    [t, targetManual, text, to],
  );

  const togglePin = useCallback(async () => {
    if (!config) return;
    const nextPinned = !config.window.always_on_top;
    const nextConfig = {
      ...config,
      window: {
        ...config.window,
        always_on_top: nextPinned,
      },
    };
    await api.setMainWindowAlwaysOnTop(nextPinned);
    await save(nextConfig);
  }, [config, save]);

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
        isPinned={config.window.always_on_top}
        showAppActions={activeView === "main"}
        onToggleHistory={() => setHistoryOpen((open) => !open)}
        onTogglePin={() => void togglePin()}
        onOpenSettings={() => {
          setHistoryOpen(false);
          setActiveView("settings");
        }}
      />

      {activeView === "settings" ? (
        <div className="min-h-0 flex-1">
          <SettingsView onBack={() => setActiveView("main")} />
        </div>
      ) : (
        <main
          className={
            "grid min-h-0 flex-1 gap-4 p-4 " +
            (historyOpen ? "grid-cols-[minmax(0,1fr)_280px]" : "grid-cols-1")
          }
        >
          <section className="flex min-h-0 flex-col gap-3">
            <SourceEditor
              busy={busy}
              detectedSource={detectedSource}
              sourceDictionary={sourceDictionary}
              text={text}
              onChange={(value) => {
                setText(value);
                if (translateError) setTranslateError(null);
              }}
              onClear={() => {
                setText("");
                setOutcomes([]);
                setTranslateError(null);
                setTargetManual(false);
                setTo(resolveAutoTargetLanguage(config.general, ""));
                requestIdRef.current = null;
              }}
              onTranslate={() => void runTranslate()}
            />

            <ServiceStatusRow
              detectedSource={detectedSource}
              enabledServices={enabledServices}
              targetLanguage={to}
              targetLanguageOptions={targetLanguageOptions}
              onTargetChange={(value) => {
                setTargetManual(true);
                setTo(value);
              }}
            />

            {translateError && (
              <TranslationErrorPanel message={translateError} />
            )}

            <ResultsPanel
              busy={busy}
              outcomes={outcomes}
              refreshingServices={refreshingServices}
              onRefreshService={(serviceId) => void refreshService(serviceId)}
            />
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
                setTargetManual(true);
                setTo(item.to);
                setTranslateError(null);
                setOutcomes([
                  {
                    service_id: item.service_id as ServiceId,
                    service_name: item.service_name,
                    result: {
                      service_id: item.service_id as ServiceId,
                      service_name: item.service_name,
                      from: item.from === "auto" ? null : item.from,
                      to: item.to,
                      text: item.translated_text,
                      audio_url: null,
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
      )}
    </div>
  );
}

function AppTitleBar({
  historyOpen,
  isPinned,
  showAppActions,
  onToggleHistory,
  onTogglePin,
  onOpenSettings,
}: {
  historyOpen: boolean;
  isPinned: boolean;
  showAppActions: boolean;
  onToggleHistory: () => void;
  onTogglePin: () => void;
  onOpenSettings: () => void;
}) {
  const t = useT();
  const [hostPlatform] = useState<HostPlatform>(() => detectHostPlatform());
  const isMac = hostPlatform === "macos";
  const pinLabel = isPinned
    ? t("main-unpin-window", null, "Unpin window")
    : t("main-pin-window", null, "Pin window");

  return (
    <header
      className={"app-titlebar " + (isMac ? "app-titlebar-mac" : "")}
      data-tauri-drag-region
    >
      {isMac && <MacWindowControls />}
      <div className="titlebar-leading">
        <button
          className={
            "titlebar-icon-btn " + (isPinned ? "titlebar-icon-active" : "")
          }
          aria-label={pinLabel}
          aria-pressed={isPinned}
          title={pinLabel}
          onClick={onTogglePin}
        >
          {isPinned ? (
            <PinOff size={15} aria-hidden="true" />
          ) : (
            <Pin size={15} aria-hidden="true" />
          )}
        </button>
      </div>
      <div className="titlebar-title" data-tauri-drag-region>
        <img src="/app-icon.png" alt="" data-tauri-drag-region />
        <h1 data-tauri-drag-region>{t("app-name", null, "Translator")}</h1>
      </div>
      <div className="titlebar-spacer" data-tauri-drag-region />
      {showAppActions ? (
        <div className="titlebar-actions">
          <button
            className={
              "titlebar-icon-btn " + (historyOpen ? "titlebar-icon-active" : "")
            }
            onClick={onToggleHistory}
            title={t("main-history", null, "History")}
            aria-label={t("main-history", null, "History")}
          >
            <HistoryIcon size={15} aria-hidden="true" />
          </button>
          <button
            className="titlebar-icon-btn"
            onClick={onOpenSettings}
            title={t("main-open-settings", null, "Settings")}
            aria-label={t("main-open-settings", null, "Settings")}
          >
            <Settings size={15} aria-hidden="true" />
          </button>
        </div>
      ) : (
        <div className="titlebar-actions" />
      )}
      {!isMac && <WindowsWindowControls />}
    </header>
  );
}

function SourceEditor({
  busy,
  detectedSource,
  sourceDictionary,
  text,
  onChange,
  onClear,
  onTranslate,
}: {
  busy: boolean;
  detectedSource: string | null;
  sourceDictionary?: DictionaryResult | null;
  text: string;
  onChange: (value: string) => void;
  onClear: () => void;
  onTranslate: () => void;
}) {
  const t = useT();
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const [copied, setCopied] = useState(false);
  const hasText = text.trim().length > 0;

  useLayoutEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "0px";
    const nextHeight = Math.min(
      SOURCE_MAX_HEIGHT,
      Math.max(SOURCE_MIN_HEIGHT, textarea.scrollHeight),
    );
    textarea.style.height = `${nextHeight}px`;
    textarea.style.overflowY =
      textarea.scrollHeight > SOURCE_MAX_HEIGHT ? "auto" : "hidden";
  }, [text]);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1200);
    return () => window.clearTimeout(timer);
  }, [copied]);

  return (
    <div className="space-y-1.5">
      <div className="source-editor">
        <textarea
          ref={textareaRef}
          className="source-editor-textarea"
          rows={2}
          value={text}
          placeholder={t(
            "main-input-placeholder",
            null,
            "Type or paste text here...",
          )}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={(event) => {
            if (
              event.key !== "Enter" ||
              event.shiftKey ||
              event.nativeEvent.isComposing
            ) {
              return;
            }
            event.preventDefault();
            if (hasText && !busy) onTranslate();
          }}
        />
        <div className="source-toolbar">
          <div className="source-toolbar-left">
            <SourceAudioControls
              detectedSource={detectedSource}
              disabled={!hasText}
              phonetics={sourceDictionary?.phonetics ?? []}
              text={text}
            />
            <button
              className="icon-btn btn-ghost !h-7 !w-7"
              disabled={!hasText}
              onClick={async () => {
                await api.copyToClipboard(text);
                setCopied(true);
              }}
              title={
                copied
                  ? t("common-copied", null, "Copied")
                  : t("common-copy", null, "Copy")
              }
              aria-label={
                copied
                  ? t("common-copied", null, "Copied")
                  : t("common-copy", null, "Copy")
              }
            >
              {copied ? (
                <Check size={15} aria-hidden="true" />
              ) : (
                <Copy size={15} aria-hidden="true" />
              )}
            </button>
          </div>

          <div className="source-toolbar-right">
            <button
              className="btn btn-secondary !h-7 px-2.5 text-xs"
              disabled={!hasText}
              onClick={onClear}
              title={t("main-clear", null, "Clear")}
              aria-label={t("main-clear", null, "Clear")}
            >
              {t("main-clear", null, "Clear")}
            </button>
            <button
              className="btn btn-primary !h-7 px-2.5 text-xs"
              disabled={busy || !hasText}
              onClick={onTranslate}
            >
              {busy
                ? t("main-translating", null, "Translating...")
                : t("main-translate", null, "Translate")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function SourceAudioControls({
  detectedSource,
  disabled,
  phonetics,
  text,
}: {
  detectedSource: string | null;
  disabled: boolean;
  phonetics: WordPhonetic[];
  text: string;
}) {
  const playablePhonetics = phonetics.filter((phonetic) => phonetic.audio_url);

  if (playablePhonetics.length === 0) {
    return (
      <AudioButton
        audioKey={`source:${detectedSource ?? "auto"}:${text}`}
        className="icon-btn btn-ghost !h-7 !w-7"
        disabled={disabled}
        resolveUrl={() =>
          api.getTextAudioUrl({
            text,
            language: detectedSource,
          })
        }
      />
    );
  }

  return (
    <div className="source-audio-controls">
      {playablePhonetics.map((phonetic, index) => (
        <span
          key={`${phonetic.label}-${phonetic.value ?? index}`}
          className="source-audio-item"
        >
          <span className="font-medium text-fg-subtle">
            {phoneticDisplayLabel(phonetic.label)}
          </span>
          {phonetic.value && (
            <span className="max-w-24 truncate text-fg">
              /{phonetic.value}/
            </span>
          )}
          <AudioButton
            audioKey={`source-phonetic:${phonetic.label}:${phonetic.audio_url ?? index}`}
            className="icon-btn btn-ghost !h-6 !w-6"
            disabled={disabled}
            url={phonetic.audio_url ?? null}
          />
        </span>
      ))}
    </div>
  );
}

function ServiceStatusRow({
  detectedSource,
  enabledServices,
  targetLanguage,
  targetLanguageOptions,
  onTargetChange,
}: {
  detectedSource: string | null;
  enabledServices: { id: ServiceId; enabled: boolean; priority: number }[];
  targetLanguage: string;
  targetLanguageOptions: ComboboxOption[];
  onTargetChange: (value: string) => void;
}) {
  const t = useT();
  return (
    <div className="service-status-row">
      <LanguageDirectionControl
        detectedSource={detectedSource}
        targetLanguage={targetLanguage}
        targetLanguageOptions={targetLanguageOptions}
        onTargetChange={onTargetChange}
      />
      <span className="service-status-services">
        {enabledServices.length > 0
          ? t(
              "main-enabled-services",
              { count: enabledServices.length },
              `${enabledServices.length} services enabled`,
            )
          : t(
              "main-no-services-enabled",
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
  );
}

function LanguageDirectionControl({
  detectedSource,
  targetLanguage,
  targetLanguageOptions,
  onTargetChange,
}: {
  detectedSource: string | null;
  targetLanguage: string;
  targetLanguageOptions: ComboboxOption[];
  onTargetChange: (value: string) => void;
}) {
  const t = useT();
  const sourceLanguageParts =
    languagePartsForCode(detectedSource, t) ??
    (detectedSource
      ? {
          flag: "",
          leading: detectedSource.toUpperCase(),
          label: detectedSource,
        }
      : autoLanguageParts(t));

  return (
    <div className="source-language-flow">
      <span className="language-short-code">{sourceLanguageParts.leading}</span>
      <ArrowRight size={14} aria-hidden="true" className="shrink-0" />
      <Combobox
        ariaLabel={t("main-target-language", null, "Target")}
        className="source-target-select"
        options={targetLanguageOptions}
        selectedDisplay="leading"
        value={targetLanguage}
        onChange={onTargetChange}
      />
    </div>
  );
}

function TranslationErrorPanel({ message }: { message: string }) {
  const t = useT();
  const isPermission = message === "permission_denied";
  return (
    <div className="rounded-lg border border-red-500 bg-red-500/10 p-3 text-sm text-red-500">
      <div className="flex flex-wrap items-center gap-2">
        <span>
          {isPermission
            ? t(
                "main-permission-denied",
                null,
                "Translator needs the Accessibility permission",
              )
            : message}
        </span>
        {isPermission && (
          <button
            className="btn btn-ghost !h-7 border-red-500/30 text-red-500 hover:bg-red-500/10"
            onClick={() => void api.openPermissionSettings()}
          >
            {t("common-open-settings", null, "Open Settings")}
          </button>
        )}
      </div>
    </div>
  );
}

function ResultsPanel({
  outcomes,
  busy,
  refreshingServices,
  onRefreshService,
}: {
  outcomes: ServiceOutcomeDto[];
  busy: boolean;
  refreshingServices: Set<ServiceId>;
  onRefreshService: (serviceId: ServiceId) => void;
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
        <ResultCard
          key={outcome.service_id}
          outcome={outcome}
          refreshing={refreshingServices.has(outcome.service_id)}
          onRefresh={() => onRefreshService(outcome.service_id)}
        />
      ))}
    </div>
  );
}

function ResultCard({
  outcome,
  refreshing,
  onRefresh,
}: {
  outcome: ServiceOutcomeDto;
  refreshing: boolean;
  onRefresh: () => void;
}) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1200);
    return () => window.clearTimeout(timer);
  }, [copied]);

  return (
    <article className="result-card">
      <div className="mb-2 flex min-h-7 items-center justify-between gap-2">
        <h2 className="inline-flex min-w-0 items-center gap-2 text-sm font-semibold">
          <ServiceLogo serviceId={outcome.service_id} className="h-4 w-4" />
          <span className="min-w-0 truncate">{outcome.service_name}</span>
        </h2>
        <button
          className="result-card-refresh icon-btn btn-ghost !h-7 !w-7"
          disabled={refreshing}
          onClick={onRefresh}
          title={t("main-refresh-service", null, "Refresh this service")}
          aria-label={t("main-refresh-service", null, "Refresh this service")}
        >
          <RefreshCw
            size={14}
            aria-hidden="true"
            className={refreshing ? "animate-spin" : ""}
          />
        </button>
      </div>
      {outcome.result ? (
        <ResultBody
          result={outcome.result}
          copied={copied}
          onCopied={() => setCopied(true)}
        />
      ) : outcome.error ? (
        <p className="text-sm text-red-500">{outcome.error.message}</p>
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
      <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
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
  const targetDictionary = result.target_dictionary ?? null;
  return (
    <div className="space-y-3">
      <div className="flex items-start justify-between gap-3">
        <p className="min-w-0 whitespace-pre-wrap text-sm leading-6">
          {result.text}
        </p>
        <div className="flex shrink-0 items-center gap-1">
          <AudioButton
            audioKey={`result:${result.service_id}:${result.audio_url ?? ""}`}
            className="icon-btn btn-ghost !h-7 !w-7"
            url={result.audio_url ?? null}
          />
          <button
            className="icon-btn btn-ghost !h-7 !w-7"
            onClick={async () => {
              await api.copyToClipboard(result.text);
              onCopied();
            }}
            title={
              copied
                ? t("common-copied", null, "Copied")
                : t("common-copy", null, "Copy")
            }
            aria-label={
              copied
                ? t("common-copied", null, "Copied")
                : t("common-copy", null, "Copy")
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
      <DictionaryDetails dictionary={targetDictionary} />
    </div>
  );
}

function AudioButton({
  audioKey,
  className,
  disabled = false,
  resolveUrl,
  url,
}: {
  audioKey: string;
  className: string;
  disabled?: boolean;
  resolveUrl?: () => Promise<string | null>;
  url?: string | null;
}) {
  const t = useT();
  const [resolving, setResolving] = useState(false);
  const { playing, phase } = useAudioButtonState(audioKey);

  if (!url && !resolveUrl) return null;

  return (
    <button
      className={className}
      disabled={disabled || resolving}
      title={t("main-play-audio", null, "Play audio")}
      aria-label={t("main-play-audio", null, "Play audio")}
      onClick={async () => {
        if (playing) {
          stopActiveAudio();
          return;
        }

        setResolving(true);
        try {
          const nextUrl = url ?? (await resolveUrl?.()) ?? null;
          if (nextUrl) playAudioUrl(audioKey, nextUrl);
        } finally {
          setResolving(false);
        }
      }}
    >
      {playing && phase === 0 ? (
        <Volume1 size={13} aria-hidden="true" />
      ) : (
        <Volume2 size={13} aria-hidden="true" />
      )}
    </button>
  );
}

function DictionaryDetails({
  dictionary,
  showPhonetics = true,
}: {
  dictionary?: DictionaryResult | null;
  showPhonetics?: boolean;
}) {
  const phonetics = showPhonetics ? (dictionary?.phonetics ?? []) : [];
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
                {phoneticDisplayLabel(phonetic.label)}
              </span>
              {phonetic.value && (
                <span className="text-fg">/ {phonetic.value} /</span>
              )}
              <AudioButton
                audioKey={`phonetic:${phonetic.label}:${phonetic.audio_url ?? index}`}
                className="icon-btn btn-ghost !h-6 !w-6"
                url={phonetic.audio_url ?? null}
              />
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

function phoneticDisplayLabel(label: string): string {
  const normalized = label.trim().toLowerCase();
  if (normalized === "us") return "美";
  if (normalized === "uk") return "英";
  if (normalized === "pinyin") return "拼音";
  return label;
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
          className="icon-btn btn-ghost !h-7 !w-7"
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

function MacWindowControls() {
  const t = useT();
  return (
    <div className="mac-window-controls">
      <button
        className="mac-window-control mac-window-close"
        onClick={() => void runWindowAction((win) => win.close())}
        title={t("common-close", null, "Close")}
        aria-label={t("common-close", null, "Close")}
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
      <span
        className="mac-window-control mac-window-spacer"
        aria-hidden="true"
      />
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
        className="window-control window-control-close"
        onClick={() => void runWindowAction((win) => win.close())}
        title={t("common-close", null, "Close")}
        aria-label={t("common-close", null, "Close")}
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

function withCurrentOption(
  options: ComboboxOption[],
  value: string,
): ComboboxOption[] {
  if (options.some((option) => option.value === value)) return options;
  return [...options, { value, label: value }];
}

function withoutOptionFlags(options: ComboboxOption[]): ComboboxOption[] {
  return options.map(({ flag: _flag, ...option }) => option);
}

function targetLanguageFromOutcomes(
  outcomes: ServiceOutcomeDto[],
): string | null {
  for (const outcome of outcomes) {
    const target = outcome.result?.to?.trim();
    if (target && target.toLowerCase() !== "auto") return target;
  }
  return null;
}

function resolveAutoTargetLanguage(general: GeneralConfig, text: string) {
  const preferred = normalizedPreferredLanguages(general);
  const source = detectLanguageHint(text);

  if (source) {
    return (
      preferred.find(
        (language) => languageKey(language) !== languageKey(source),
      ) ?? defaultCounterpart(source)
    );
  }

  return preferred[0] ?? defaultCounterpart("");
}

function normalizedPreferredLanguages(general: GeneralConfig): string[] {
  const languages: string[] = [];
  for (const language of general.preferred_languages ?? []) {
    addPreferredLanguage(languages, language);
  }

  if (languages.length === 0) {
    addPreferredLanguage(languages, general.target_language);
    addPreferredLanguage(languages, general.default_from);
  }

  if (languages.length === 0) {
    addPreferredLanguage(languages, "zh-Hans");
    addPreferredLanguage(languages, "en");
  }

  if (languages.length === 1) {
    addPreferredLanguage(languages, defaultCounterpart(languages[0]));
  }

  return languages;
}

function addPreferredLanguage(languages: string[], language: string) {
  const value = language.trim();
  if (!value || value.toLowerCase() === "auto") return;
  const key = languageKey(value);
  if (languages.some((existing) => languageKey(existing) === key)) return;
  languages.push(value);
}

function languageKey(language: string): string {
  const normalized = language.trim().replaceAll("_", "-").toLowerCase();
  if (normalized.startsWith("zh-hans") || normalized.startsWith("zh-cn")) {
    return "zh-hans";
  }
  if (
    normalized.startsWith("zh-hant") ||
    normalized.startsWith("zh-tw") ||
    normalized.startsWith("zh-hk") ||
    normalized.startsWith("zh-mo")
  ) {
    return "zh-hant";
  }
  return normalized.split("-")[0] ?? normalized;
}

function defaultCounterpart(language: string): string {
  return languageKey(language) === "en" ? "zh-Hans" : "en";
}

function detectLanguageHint(text: string): string | null {
  const counts = {
    ar: 0,
    cjk: 0,
    en: 0,
    ja: 0,
    ko: 0,
    ru: 0,
  };

  for (const char of text) {
    const code = char.codePointAt(0) ?? 0;
    if (
      (code >= 0x0041 && code <= 0x005a) ||
      (code >= 0x0061 && code <= 0x007a) ||
      (code >= 0x00c0 && code <= 0x024f)
    ) {
      counts.en += 1;
    } else if (code >= 0x3040 && code <= 0x30ff) {
      counts.ja += 1;
    } else if (code >= 0x3400 && code <= 0x9fff) {
      counts.cjk += 1;
    } else if (code >= 0xac00 && code <= 0xd7af) {
      counts.ko += 1;
    } else if (code >= 0x0400 && code <= 0x052f) {
      counts.ru += 1;
    } else if (
      (code >= 0x0600 && code <= 0x06ff) ||
      (code >= 0x0750 && code <= 0x077f) ||
      (code >= 0x08a0 && code <= 0x08ff)
    ) {
      counts.ar += 1;
    }
  }

  const candidates: Array<[string, number]> = [
    ["ja", counts.ja],
    ["ko", counts.ko],
    ["zh-Hans", counts.cjk],
    ["ru", counts.ru],
    ["ar", counts.ar],
    ["en", counts.en],
  ];
  const best = candidates.reduce((current, next) =>
    next[1] > current[1] ? next : current,
  );
  return best[1] > 0 ? best[0] : null;
}

function hotkeyErrorMessage(error: string, t: TFunction): string {
  if (error === "permission_denied") return "permission_denied";
  if (error.startsWith("clipboard:")) {
    return t(
      "main-error-clipboard",
      { msg: error.slice("clipboard:".length) },
      `Could not read clipboard: ${error.slice("clipboard:".length)}`,
    );
  }
  return error;
}

let activeAudio: HTMLAudioElement | null = null;
let activeAudioKey: string | null = null;
let activeAudioPhase = 0;
let activeAudioTimer: number | null = null;
const audioListeners = new Set<() => void>();

function emitAudioChange() {
  for (const listener of audioListeners) listener();
}

function stopActiveAudio() {
  if (activeAudio) {
    activeAudio.pause();
    activeAudio = null;
  }
  if (activeAudioTimer !== null) {
    window.clearInterval(activeAudioTimer);
    activeAudioTimer = null;
  }
  activeAudioKey = null;
  activeAudioPhase = 0;
  emitAudioChange();
}

function playAudioUrl(key: string, url: string) {
  stopActiveAudio();
  const audio = new Audio(url);
  activeAudio = audio;
  activeAudioKey = key;
  activeAudioPhase = 0;
  activeAudioTimer = window.setInterval(() => {
    if (activeAudio !== audio) return;
    activeAudioPhase = activeAudioPhase === 0 ? 1 : 0;
    emitAudioChange();
  }, 420);
  audio.onended = () => {
    if (activeAudio === audio) stopActiveAudio();
  };
  audio.onerror = () => {
    if (activeAudio === audio) stopActiveAudio();
  };
  emitAudioChange();
  void audio.play().catch(() => {
    if (activeAudio === audio) stopActiveAudio();
  });
}

function useAudioButtonState(key: string) {
  const [, forceRender] = useState(0);
  useEffect(() => {
    const listener = () => forceRender((value) => value + 1);
    audioListeners.add(listener);
    return () => {
      audioListeners.delete(listener);
    };
  }, []);
  return {
    playing: activeAudioKey === key,
    phase: activeAudioPhase,
  };
}
