import { useState, useEffect, useRef } from "react";
import { ChevronDown, GripVertical, Save, Trash2 } from "lucide-react";
import { useConfigStore } from "../../stores/config";
import { useT } from "../../i18n";
import * as api from "../../ipc/commands";
import { ServiceLogo } from "../../services/ServiceLogo";
import { Combobox } from "../../components/Combobox";
import type { ServiceId } from "../../types/bindings";

interface ServiceMeta {
  id: ServiceId;
  name: string;
  descriptionKey: string;
  descriptionFallback: string;
  needsKey:
    | "single"
    | "optional_single"
    | "appkey_secret"
    | "google_cloud"
    | "openai_compat";
}

const SERVICES: ServiceMeta[] = [
  {
    id: "youdao",
    name: "Youdao (有道)",
    descriptionKey: "settings-services-youdao-description",
    descriptionFallback:
      "Built-in web translation; optional OpenAPI credentials.",
    needsKey: "appkey_secret",
  },
  {
    id: "deepl",
    name: "DeepL",
    descriptionKey: "settings-services-deepl-description",
    descriptionFallback: "Built-in web translation; optional official API key.",
    needsKey: "optional_single",
  },
  {
    id: "google",
    name: "Google Translate",
    descriptionKey: "settings-services-google-description",
    descriptionFallback:
      "Built-in web translation; optional Cloud v3 credentials.",
    needsKey: "google_cloud",
  },
  {
    id: "bing",
    name: "Microsoft Translator",
    descriptionKey: "settings-services-bing-description",
    descriptionFallback: "Built-in web translation; optional Azure key.",
    needsKey: "optional_single",
  },
  {
    id: "openai",
    name: "OpenAI Compatible",
    descriptionKey: "settings-services-openai-description",
    descriptionFallback: "Any OpenAI-style chat completion endpoint.",
    needsKey: "openai_compat",
  },
];

export function ServicesSection() {
  const { config, save } = useConfigStore();
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [dragging, setDragging] = useState<ServiceId | null>(null);
  const [draftOrder, setDraftOrder] = useState<ServiceId[] | null>(null);
  const draggingRef = useRef<ServiceId | null>(null);
  const draftOrderRef = useRef<ServiceId[] | null>(null);
  const commitOrderRef = useRef<() => void>(() => {});

  useEffect(() => {
    if (!dragging) return;

    const finishDrag = () => commitOrderRef.current();
    window.addEventListener("pointerup", finishDrag);
    window.addEventListener("pointercancel", finishDrag);
    return () => {
      window.removeEventListener("pointerup", finishDrag);
      window.removeEventListener("pointercancel", finishDrag);
    };
  }, [dragging]);

  if (!config) return null;

  const savedServices = SERVICES.filter((svc) => config.services[svc.id])
    .map((svc, index) => ({ svc, index }))
    .sort((a, b) => {
      const left = config.services[a.svc.id];
      const right = config.services[b.svc.id];
      return left.priority - right.priority || a.index - b.index;
    })
    .map(({ svc }) => svc);

  const serviceById = new Map(SERVICES.map((svc) => [svc.id, svc]));
  const orderedServices = (draftOrder ?? savedServices.map((svc) => svc.id))
    .map((id) => serviceById.get(id))
    .filter((svc): svc is ServiceMeta =>
      Boolean(svc && config.services[svc.id]),
    );

  const persistOrder = (nextOrder: ServiceId[]) => {
    const nextServices = { ...config.services };
    nextOrder.forEach((id, index) => {
      const current = nextServices[id];
      if (current) nextServices[id] = { ...current, priority: index };
    });
    draggingRef.current = null;
    draftOrderRef.current = null;
    setDraftOrder(null);
    setDragging(null);
    void save({ ...config, services: nextServices });
  };

  const moveDraft = (from: ServiceId, to: ServiceId) => {
    const currentOrder =
      draftOrderRef.current ?? orderedServices.map((svc) => svc.id);
    const fromIndex = currentOrder.indexOf(from);
    const toIndex = currentOrder.indexOf(to);
    if (fromIndex === -1 || toIndex === -1 || fromIndex === toIndex) return;

    const nextOrder = [...currentOrder];
    const [moved] = nextOrder.splice(fromIndex, 1);
    nextOrder.splice(toIndex, 0, moved);
    draftOrderRef.current = nextOrder;
    setDraftOrder(nextOrder);
  };

  const beginDragService = (id: ServiceId) => {
    const nextOrder = orderedServices.map((svc) => svc.id);
    draggingRef.current = id;
    draftOrderRef.current = nextOrder;
    setDragging(id);
    setDraftOrder(nextOrder);
  };

  const hoverDragService = (id: ServiceId) => {
    const dragged = draggingRef.current;
    if (dragged) moveDraft(dragged, id);
  };

  const commitOrder = () => {
    const nextOrder = draftOrderRef.current;
    if (!nextOrder) {
      draggingRef.current = null;
      setDragging(null);
      return;
    }
    persistOrder(nextOrder);
  };
  commitOrderRef.current = commitOrder;

  return (
    <div className="space-y-3">
      {orderedServices.map((svc) => {
        const sc = config.services[svc.id];
        if (!sc) return null;
        return (
          <ServiceRow
            key={svc.id}
            meta={svc}
            enabled={sc.enabled}
            onToggle={(enabled) =>
              void save({
                ...config,
                services: { ...config.services, [svc.id]: { ...sc, enabled } },
              })
            }
            onSaveOptions={(options) =>
              void save({
                ...config,
                services: { ...config.services, [svc.id]: { ...sc, options } },
              })
            }
            expanded={expanded[svc.id] ?? false}
            onToggleExpanded={() =>
              setExpanded((current) => ({
                ...current,
                [svc.id]: !(current[svc.id] ?? false),
              }))
            }
            dragging={dragging === svc.id}
            onDragStart={() => beginDragService(svc.id)}
            onDragOverService={() => hoverDragService(svc.id)}
            onDragEnd={commitOrder}
            onDrop={commitOrder}
            options={sc.options as Record<string, unknown>}
          />
        );
      })}
    </div>
  );
}

function ServiceRow({
  meta,
  enabled,
  expanded,
  dragging,
  onToggle,
  onToggleExpanded,
  onSaveOptions,
  onDragEnd,
  onDragOverService,
  onDragStart,
  onDrop,
  options,
}: {
  meta: ServiceMeta;
  enabled: boolean;
  expanded: boolean;
  dragging: boolean;
  onToggle: (v: boolean) => void;
  onToggleExpanded: () => void;
  onSaveOptions: (o: Record<string, unknown>) => void;
  onDragEnd: () => void;
  onDragOverService: () => void;
  onDragStart: () => void;
  onDrop: () => void;
  options: Record<string, unknown>;
}) {
  const t = useT();
  // BH-8.1: status indicator (configured / built-in / missing key / error).
  const [keyState, setKeyState] = useState<
    "unknown" | "configured" | "builtin" | "missing" | "error"
  >("unknown");
  const [credentialVersion, setCredentialVersion] = useState(0);

  useEffect(() => {
    let cancelled = false;
    if (meta.needsKey === "appkey_secret") {
      const ok =
        typeof options.appKey === "string" &&
        options.appKey.trim().length > 0 &&
        typeof options.appSecret === "string" &&
        options.appSecret.trim().length > 0;
      setKeyState(ok ? "configured" : "builtin");
    } else if (meta.needsKey === "google_cloud") {
      const hasProject =
        typeof options.projectId === "string" &&
        options.projectId.trim().length > 0;
      if (!hasProject) {
        setKeyState("builtin");
        return () => {
          cancelled = true;
        };
      }
      api
        .hasApiKey(meta.id)
        .then((hasKey) => {
          if (!cancelled) setKeyState(hasKey ? "configured" : "builtin");
        })
        .catch(() => {
          if (!cancelled) setKeyState("builtin");
        });
    } else if (meta.needsKey === "optional_single") {
      api
        .hasApiKey(meta.id)
        .then((has) => {
          if (!cancelled) setKeyState(has ? "configured" : "builtin");
        })
        .catch(() => {
          if (!cancelled) setKeyState("builtin");
        });
    } else {
      api
        .hasApiKey(meta.id)
        .then((has) => {
          if (!cancelled) setKeyState(has ? "configured" : "missing");
        })
        .catch(() => {
          if (!cancelled) setKeyState("error");
        });
    }
    return () => {
      cancelled = true;
    };
  }, [
    credentialVersion,
    meta.id,
    meta.needsKey,
    options.appKey,
    options.appSecret,
    options.projectId,
  ]);

  const dot =
    keyState === "configured"
      ? "bg-green-500"
      : keyState === "builtin"
        ? "bg-green-500"
        : keyState === "missing"
          ? "bg-red-500"
          : keyState === "error"
            ? "bg-yellow-500"
            : "bg-fg-subtle";
  const dotTitle =
    keyState === "configured"
      ? t("settings-services-status-configured")
      : keyState === "builtin"
        ? t("settings-services-status-builtin")
        : keyState === "missing"
          ? t("settings-services-status-missing")
          : keyState === "error"
            ? t("settings-services-status-keychain-error")
            : t("settings-services-status-checking");

  return (
    <div
      className={
        "rounded-lg border border-border bg-bg p-3 transition " +
        (dragging ? "opacity-60" : "")
      }
      onPointerEnter={() => {
        onDragOverService();
      }}
      onPointerUp={() => {
        onDrop();
      }}
      onPointerCancel={() => {
        onDrop();
      }}
    >
      <div className="flex items-center justify-between">
        <div className="flex min-w-0 items-center gap-2">
          <button
            type="button"
            className="icon-btn !h-7 !w-7 cursor-grab"
            aria-label={t("settings-services-drag-aria", {
              service: meta.name,
            })}
            title={t("settings-services-drag")}
            onPointerDown={(event) => {
              if (event.button !== 0) return;
              event.preventDefault();
              onDragStart();
            }}
            onPointerUp={onDragEnd}
          >
            <GripVertical size={15} aria-hidden="true" />
          </button>
          <div className="relative shrink-0">
            <ServiceLogo serviceId={meta.id} className="h-7 w-7" />
            <span
              aria-label={dotTitle}
              title={dotTitle}
              className={
                "absolute -bottom-0.5 -right-0.5 inline-block h-2.5 w-2.5 rounded-full ring-2 ring-bg " +
                dot
              }
            />
          </div>
          <div className="min-w-0">
            <div className="font-medium">{meta.name}</div>
            <div className="text-xs text-fg-subtle">
              {t(meta.descriptionKey, null, meta.descriptionFallback)}
            </div>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <label className="inline-flex cursor-pointer items-center gap-2 text-sm">
            <input
              type="checkbox"
              aria-label={t(
                "settings-services-enable-aria",
                { service: meta.name },
                `Enable ${meta.name}`,
              )}
              checked={enabled}
              onChange={(e) => onToggle(e.target.checked)}
              className="checkbox !mt-0"
            />
            {t("settings-services-enabled")}
          </label>
          <button
            type="button"
            className="icon-btn !h-7 !w-7"
            aria-expanded={expanded}
            aria-label={t(
              "settings-services-toggle-panel-aria",
              { service: meta.name },
              `Toggle ${meta.name} settings`,
            )}
            title={t("settings-services-toggle-panel")}
            onClick={onToggleExpanded}
          >
            <ChevronDown
              size={15}
              aria-hidden="true"
              className={
                "transition-transform " + (expanded ? "" : "-rotate-90")
              }
            />
          </button>
        </div>
      </div>
      {expanded && (
        <div className="mt-3 space-y-2 border-t border-border pt-3">
          {(meta.needsKey === "single" ||
            meta.needsKey === "optional_single") && (
            <SingleApiKey
              serviceId={meta.id}
              serviceName={meta.name}
              onCredentialChange={() =>
                setCredentialVersion((version) => version + 1)
              }
            />
          )}
          {meta.needsKey === "appkey_secret" && (
            <YoudaoCreds options={options} onSave={onSaveOptions} />
          )}
          {meta.needsKey === "google_cloud" && (
            <GoogleCreds
              serviceId={meta.id}
              options={options}
              onSave={onSaveOptions}
              onCredentialChange={() =>
                setCredentialVersion((version) => version + 1)
              }
            />
          )}
          {meta.needsKey === "openai_compat" && (
            <OpenAICompat
              options={options}
              onSave={onSaveOptions}
              onCredentialChange={() =>
                setCredentialVersion((version) => version + 1)
              }
            />
          )}
        </div>
      )}
    </div>
  );
}

function SingleApiKey({
  serviceId,
  serviceName,
  onCredentialChange,
}: {
  serviceId: ServiceId;
  serviceName: string;
  onCredentialChange: () => void;
}) {
  const t = useT();
  const [key, setKey] = useState("");
  const [status, setStatus] = useState<"idle" | "saved" | "deleted" | "error">(
    "idle",
  );

  // BH-8.7: green toast auto-dismisses after 2 s.
  useEffect(() => {
    if (status === "idle") return;
    const t = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(t);
  }, [status]);

  return (
    <div>
      <label className="label">{t("settings-services-api-key")}</label>
      <div className="flex gap-2">
        <input
          type="password"
          className="input min-w-0"
          aria-label={t(
            "settings-services-api-key-aria",
            { service: serviceName },
            `${serviceName} API key`,
          )}
          placeholder="••••••"
          value={key}
          onChange={(e) => setKey(e.target.value)}
        />
        <button
          className="icon-btn btn-primary"
          aria-label={t(
            "settings-services-save-key-aria",
            { service: serviceName },
            `Save ${serviceName} API key to OS Keychain`,
          )}
          title={t("settings-services-save")}
          disabled={!key}
          onClick={async () => {
            try {
              await api.setApiKey(serviceId, key);
              setKey("");
              setStatus("saved");
              onCredentialChange();
            } catch {
              setStatus("error");
            }
          }}
        >
          <Save size={15} aria-hidden="true" />
        </button>
        <button
          className="icon-btn"
          aria-label={t(
            "settings-services-remove-key-aria",
            { service: serviceName },
            `Remove ${serviceName} API key from OS Keychain`,
          )}
          title={t("settings-services-remove")}
          onClick={async () => {
            try {
              await api.deleteApiKey(serviceId);
              setStatus("deleted");
              onCredentialChange();
            } catch {
              setStatus("error");
            }
          }}
        >
          <Trash2 size={15} aria-hidden="true" />
        </button>
      </div>
      {status !== "idle" && (
        <p
          className={
            status === "saved"
              ? "mt-1 text-xs text-green-500"
              : status === "error"
                ? "mt-1 text-xs text-red-500"
                : "mt-1 text-xs text-fg-subtle"
          }
        >
          {status === "saved" && t("settings-services-key-saved")}
          {status === "deleted" && t("settings-services-key-removed")}
          {status === "error" && t("settings-services-keychain-update-failed")}
        </p>
      )}
    </div>
  );
}

function YoudaoCreds({
  options,
  onSave,
}: {
  options: Record<string, unknown>;
  onSave: (o: Record<string, unknown>) => void;
}) {
  const t = useT();
  const appKey = String(options.appKey ?? "");
  const appSecret = String(options.appSecret ?? "");
  const baseUrl = String(options.base_url ?? "");
  const [saved, setSaved] = useState(false);

  // BH-8.7: green toast auto-dismisses after 2 s for the Youdao cred row.
  useEffect(() => {
    if (!saved) return;
    const t = setTimeout(() => setSaved(false), 2000);
    return () => clearTimeout(t);
  }, [saved]);

  const saveAndToast = (next: Record<string, unknown>) => {
    onSave(next);
    setSaved(true);
  };

  return (
    <div className="space-y-2">
      <div className="grid grid-cols-2 gap-2">
        <div>
          <label className="label">
            {t("settings-services-youdao-appkey")}
          </label>
          <input
            className="input"
            aria-label={t("settings-services-youdao-appkey")}
            value={appKey}
            onChange={(e) =>
              saveAndToast({ ...options, appKey: e.target.value })
            }
          />
        </div>
        <div>
          <label className="label">
            {t("settings-services-youdao-appsecret")}
          </label>
          <input
            type="password"
            className="input"
            aria-label={t("settings-services-youdao-appsecret")}
            value={appSecret}
            onChange={(e) =>
              saveAndToast({ ...options, appSecret: e.target.value })
            }
          />
        </div>
      </div>
      <div>
        <label className="label">{t("settings-services-base-url")}</label>
        <input
          className="input"
          aria-label={t(
            "settings-services-base-url-aria",
            { service: "Youdao" },
            "Youdao Base URL",
          )}
          placeholder="https://openapi.youdao.com"
          value={baseUrl}
          onChange={(e) =>
            saveAndToast({ ...options, base_url: e.target.value })
          }
        />
      </div>
      {saved && (
        <p className="mt-1 text-xs text-green-500">
          {t("settings-services-saved")}
        </p>
      )}
    </div>
  );
}

function GoogleCreds({
  serviceId,
  options,
  onSave,
  onCredentialChange,
}: {
  serviceId: ServiceId;
  options: Record<string, unknown>;
  onSave: (o: Record<string, unknown>) => void;
  onCredentialChange: () => void;
}) {
  const t = useT();
  const [key, setKey] = useState("");
  const [status, setStatus] = useState<"idle" | "saved" | "deleted" | "error">(
    "idle",
  );
  const projectId = String(options.projectId ?? "");
  const baseUrl = String(options.base_url ?? "");

  useEffect(() => {
    if (status === "idle") return;
    const t = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(t);
  }, [status]);

  return (
    <div className="space-y-2">
      <div>
        <label className="label">
          {t("settings-services-google-project-id")}
        </label>
        <input
          className="input"
          aria-label={t("settings-services-google-project-id")}
          value={projectId}
          onChange={(e) => onSave({ ...options, projectId: e.target.value })}
        />
      </div>
      <div>
        <label className="label">{t("settings-services-base-url")}</label>
        <input
          className="input"
          aria-label={t(
            "settings-services-base-url-aria",
            { service: "Google" },
            "Google Base URL",
          )}
          placeholder="https://translation.googleapis.com"
          value={baseUrl}
          onChange={(e) => onSave({ ...options, base_url: e.target.value })}
        />
      </div>
      <div>
        <label className="label">{t("settings-services-api-key")}</label>
        <div className="flex gap-2">
          <input
            type="password"
            className="input min-w-0"
            aria-label={t(
              "settings-services-api-key-aria",
              { service: "Google" },
              "Google API key",
            )}
            placeholder="••••••"
            value={key}
            onChange={(e) => setKey(e.target.value)}
          />
          <button
            className="icon-btn btn-primary"
            aria-label={t(
              "settings-services-save-key-aria",
              { service: "Google" },
              "Save Google API key to OS Keychain",
            )}
            title={t("settings-services-save")}
            disabled={!key}
            onClick={async () => {
              try {
                await api.setApiKey(serviceId, key);
                setKey("");
                setStatus("saved");
                onCredentialChange();
              } catch {
                setStatus("error");
              }
            }}
          >
            <Save size={15} aria-hidden="true" />
          </button>
          <button
            className="icon-btn"
            aria-label={t(
              "settings-services-remove-key-aria",
              { service: "Google" },
              "Remove Google API key from OS Keychain",
            )}
            title={t("settings-services-remove")}
            onClick={async () => {
              try {
                await api.deleteApiKey(serviceId);
                setStatus("deleted");
                onCredentialChange();
              } catch {
                setStatus("error");
              }
            }}
          >
            <Trash2 size={15} aria-hidden="true" />
          </button>
        </div>
        {status !== "idle" && (
          <p
            className={
              status === "saved"
                ? "mt-1 text-xs text-green-500"
                : status === "error"
                  ? "mt-1 text-xs text-red-500"
                  : "mt-1 text-xs text-fg-subtle"
            }
          >
            {status === "saved" && t("settings-services-key-saved")}
            {status === "deleted" && t("settings-services-key-removed")}
            {status === "error" &&
              t("settings-services-keychain-update-failed")}
          </p>
        )}
      </div>
    </div>
  );
}

// BH-8.5: real Presets dropdown. Selecting a preset auto-fills baseUrl+model
// and (for non-custom presets) greys them out to indicate they are
// preset-locked. "Custom" leaves the fields editable.
interface OpenAIPreset {
  id: string;
  name: string;
  nameKey: string;
  baseUrl: string;
  model: string;
}

const OPENAI_PRESETS: OpenAIPreset[] = [
  {
    id: "openai",
    name: "OpenAI",
    nameKey: "settings-services-openai-preset-openai",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    nameKey: "settings-services-openai-preset-deepseek",
    baseUrl: "https://api.deepseek.com/v1",
    model: "deepseek-chat",
  },
  {
    id: "zhipu",
    name: "Zhipu",
    nameKey: "settings-services-openai-preset-zhipu",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    model: "glm-4-flash",
  },
  {
    id: "ollama",
    name: "Ollama",
    nameKey: "settings-services-openai-preset-ollama",
    baseUrl: "http://localhost:11434/v1",
    model: "llama3.1",
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    nameKey: "settings-services-openai-preset-openrouter",
    baseUrl: "https://openrouter.ai/api/v1",
    model: "openai/gpt-4o-mini",
  },
  {
    id: "custom",
    name: "Custom",
    nameKey: "settings-services-openai-preset-custom",
    baseUrl: "",
    model: "",
  },
];

function detectPreset(baseUrl: string, model: string): string {
  const match = OPENAI_PRESETS.find(
    (p) => p.id !== "custom" && p.baseUrl === baseUrl && p.model === model,
  );
  return match ? match.id : "custom";
}

function OpenAICompat({
  options,
  onSave,
  onCredentialChange,
}: {
  options: Record<string, unknown>;
  onSave: (o: Record<string, unknown>) => void;
  onCredentialChange: () => void;
}) {
  const t = useT();
  const [apiKey, setApiKey] = useState("");
  const [status, setStatus] = useState<"idle" | "saved" | "deleted" | "error">(
    "idle",
  );
  const baseUrl = String(options.baseUrl ?? "");
  const model = String(options.model ?? "");
  const preset = detectPreset(baseUrl, model);
  const locked = preset !== "custom";
  const presetOptions = OPENAI_PRESETS.map((item) => ({
    value: item.id,
    label: t(item.nameKey, null, item.name),
  }));

  // BH-8.7: green toast auto-dismisses after 2 s.
  useEffect(() => {
    if (status === "idle") return;
    const t = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(t);
  }, [status]);

  return (
    <div className="space-y-2">
      <div>
        <Combobox
          label={t("settings-services-openai-presets-label")}
          options={presetOptions}
          value={preset}
          onChange={(nextValue) => {
            const next = OPENAI_PRESETS.find((p) => p.id === nextValue);
            if (!next) return;
            onSave({ ...options, baseUrl: next.baseUrl, model: next.model });
          }}
        />
      </div>
      <div className="grid grid-cols-2 gap-2">
        <div>
          <label className="label">{t("settings-services-base-url")}</label>
          <input
            className="input"
            aria-label={t(
              "settings-services-base-url-aria",
              { service: "OpenAI Compatible" },
              "OpenAI-compatible base URL",
            )}
            value={baseUrl}
            disabled={locked}
            onChange={(e) => onSave({ ...options, baseUrl: e.target.value })}
          />
        </div>
        <div>
          <label className="label">{t("settings-services-openai-model")}</label>
          <input
            className="input"
            aria-label={t("settings-services-openai-model-aria")}
            value={model}
            disabled={locked}
            onChange={(e) => onSave({ ...options, model: e.target.value })}
          />
        </div>
      </div>
      <div>
        <label className="label">{t("settings-services-api-key")}</label>
        <div className="flex gap-2">
          <input
            type="password"
            className="input min-w-0"
            aria-label={t(
              "settings-services-api-key-aria",
              { service: "OpenAI Compatible" },
              "OpenAI-compatible API key",
            )}
            placeholder="••••••"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
          <button
            className="icon-btn btn-primary"
            aria-label={t(
              "settings-services-save-key-aria",
              { service: "OpenAI Compatible" },
              "Save OpenAI-compatible API key to OS Keychain",
            )}
            title={t("settings-services-save")}
            disabled={!apiKey}
            onClick={async () => {
              try {
                await api.setApiKey("openai", apiKey);
                setApiKey("");
                setStatus("saved");
                onCredentialChange();
              } catch {
                setStatus("error");
              }
            }}
          >
            <Save size={15} aria-hidden="true" />
          </button>
          <button
            className="icon-btn"
            aria-label={t(
              "settings-services-remove-key-aria",
              { service: "OpenAI Compatible" },
              "Remove OpenAI-compatible API key from OS Keychain",
            )}
            title={t("settings-services-remove")}
            onClick={async () => {
              try {
                await api.deleteApiKey("openai");
                setStatus("deleted");
                onCredentialChange();
              } catch {
                setStatus("error");
              }
            }}
          >
            <Trash2 size={15} aria-hidden="true" />
          </button>
        </div>
        {status !== "idle" && (
          <p
            className={
              status === "saved"
                ? "mt-1 text-xs text-green-500"
                : status === "error"
                  ? "mt-1 text-xs text-red-500"
                  : "mt-1 text-xs text-fg-subtle"
            }
          >
            {status === "saved" && t("settings-services-key-saved")}
            {status === "deleted" && t("settings-services-key-removed")}
            {status === "error" &&
              t("settings-services-keychain-update-failed")}
          </p>
        )}
      </div>
    </div>
  );
}
