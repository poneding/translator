import { useState, useEffect } from "react";
import { useConfigStore } from "../../stores/config";
import * as api from "../../ipc/commands";
import type { ServiceId } from "../../types/bindings";

interface ServiceMeta {
  id: ServiceId;
  name: string;
  description: string;
  needsKey: "single" | "appkey_secret" | "openai_compat";
}

const SERVICES: ServiceMeta[] = [
  { id: "youdao", name: "Youdao (有道)", description: "Chinese dictionary and translation.", needsKey: "appkey_secret" },
  { id: "deepl",  name: "DeepL",           description: "Best quality for European languages.", needsKey: "single" },
  { id: "google", name: "Google Translate", description: "Broadest language coverage (Cloud v3).", needsKey: "single" },
  { id: "bing",   name: "Microsoft Translator", description: "Free Azure tier.", needsKey: "single" },
  { id: "openai", name: "OpenAI Compatible", description: "Any OpenAI-style chat completion endpoint.", needsKey: "openai_compat" },
];

export function ServicesSection() {
  const { config, save } = useConfigStore();
  if (!config) return null;

  return (
    <div className="space-y-3">
      {SERVICES.map((svc) => {
        const sc = config.services[svc.id];
        if (!sc) return null;
        return (
          <ServiceRow
            key={svc.id}
            meta={svc}
            enabled={sc.enabled}
            onToggle={(enabled) =>
              void save({ ...config, services: { ...config.services, [svc.id]: { ...sc, enabled } } })
            }
            onSaveOptions={(options) =>
              void save({ ...config, services: { ...config.services, [svc.id]: { ...sc, options } } })
            }
            onSavePriority={(priority) =>
              void save({ ...config, services: { ...config.services, [svc.id]: { ...sc, priority } } })
            }
            options={sc.options as Record<string, unknown>}
            priority={sc.priority}
          />
        );
      })}
    </div>
  );
}

function ServiceRow({
  meta,
  enabled,
  onToggle,
  onSaveOptions,
  onSavePriority,
  options,
  priority,
}: {
  meta: ServiceMeta;
  enabled: boolean;
  onToggle: (v: boolean) => void;
  onSaveOptions: (o: Record<string, unknown>) => void;
  onSavePriority: (p: number) => void;
  options: Record<string, unknown>;
  priority: number;
}) {
  // BH-8.1: status indicator (configured / missing key / error).
  // For Youdao we require both appKey and appSecret options to be set.
  // For OpenAI we just probe the keychain; options are still editable.
  const [keyState, setKeyState] = useState<"unknown" | "configured" | "missing" | "error">("unknown");

  useEffect(() => {
    let cancelled = false;
    if (meta.needsKey === "appkey_secret") {
      const ok =
        typeof options.appKey === "string" &&
        options.appKey.length > 0 &&
        typeof options.appSecret === "string" &&
        options.appSecret.length > 0;
      setKeyState(ok ? "configured" : "missing");
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
  }, [meta.id, meta.needsKey, options.appKey, options.appSecret]);

  const dot =
    keyState === "configured" ? "bg-green-500" :
    keyState === "missing"    ? "bg-red-500"   :
    keyState === "error"      ? "bg-yellow-500" :
                                "bg-fg-subtle";
  const dotTitle =
    keyState === "configured" ? "Configured" :
    keyState === "missing"    ? "Missing credential" :
    keyState === "error"      ? "Keychain error" :
                                "Checking…";

  return (
    <div className="rounded-lg border border-border bg-bg p-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span
            aria-label={dotTitle}
            title={dotTitle}
            className={"inline-block h-2.5 w-2.5 rounded-full " + dot}
          />
          <div>
            <div className="font-medium">{meta.name}</div>
            <div className="text-xs text-fg-subtle">{meta.description}</div>
          </div>
        </div>
        <label className="inline-flex cursor-pointer items-center gap-2 text-sm">
          <input
            type="checkbox"
            aria-label={`Enable ${meta.name}`}
            checked={enabled}
            onChange={(e) => onToggle(e.target.checked)}
            className="h-4 w-4 accent-[rgb(var(--accent))]"
          />
          Enabled
        </label>
      </div>
      {enabled && (
        <div className="mt-3 space-y-2 border-t border-border pt-3">
          {meta.needsKey === "single" && <SingleApiKey serviceId={meta.id} />}
          {meta.needsKey === "appkey_secret" && <YoudaoCreds options={options} onSave={onSaveOptions} />}
          {meta.needsKey === "openai_compat" && <OpenAICompat options={options} onSave={onSaveOptions} />}
          <div>
            <label className="label">Priority (lower = shown first)</label>
            <input
              type="number"
              className="input w-24"
              aria-label={`${meta.name} priority (lower = shown first)`}
              value={priority}
              min={0}
              max={255}
              onChange={(e) => onSavePriority(Number(e.target.value))}
            />
          </div>
        </div>
      )}
    </div>
  );
}

function SingleApiKey({ serviceId }: { serviceId: ServiceId }) {
  const [key, setKey] = useState("");
  const [status, setStatus] = useState<"idle" | "saved" | "deleted" | "error">("idle");

  // BH-8.7: green toast auto-dismisses after 2 s.
  useEffect(() => {
    if (status === "idle") return;
    const t = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(t);
  }, [status]);

  return (
    <div>
      <label className="label">API key</label>
      <div className="flex gap-2">
        <input
          type="password"
          className="input"
          aria-label={`${serviceId} API key`}
          placeholder="••••••"
          value={key}
          onChange={(e) => setKey(e.target.value)}
        />
        <button
          className="btn btn-primary"
          aria-label={`Save ${serviceId} API key to OS Keychain`}
          disabled={!key}
          onClick={async () => {
            try {
              await api.setApiKey(serviceId, key);
              setKey("");
              setStatus("saved");
            } catch {
              setStatus("error");
            }
          }}
        >
          Save
        </button>
        <button
          className="btn"
          aria-label={`Remove ${serviceId} API key from OS Keychain`}
          onClick={async () => {
            try {
              await api.deleteApiKey(serviceId);
              setStatus("deleted");
            } catch {
              setStatus("error");
            }
          }}
        >
          Remove
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
          {status === "saved" && "Saved to OS Keychain."}
          {status === "deleted" && "Removed from OS Keychain."}
          {status === "error" && "Failed to update Keychain."}
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
  const appKey = String(options.appKey ?? "");
  const appSecret = String(options.appSecret ?? "");
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
          <label className="label">App Key</label>
          <input
            className="input"
            aria-label="Youdao App Key"
            value={appKey}
            onChange={(e) => saveAndToast({ ...options, appKey: e.target.value })}
          />
        </div>
        <div>
          <label className="label">App Secret</label>
          <input
            type="password"
            className="input"
            aria-label="Youdao App Secret"
            value={appSecret}
            onChange={(e) => saveAndToast({ ...options, appSecret: e.target.value })}
          />
        </div>
      </div>
      {saved && <p className="mt-1 text-xs text-green-500">Saved to OS Keychain.</p>}
    </div>
  );
}

// BH-8.5: real Presets dropdown. Selecting a preset auto-fills baseUrl+model
// and (for non-custom presets) greys them out to indicate they are
// preset-locked. "Custom" leaves the fields editable.
interface OpenAIPreset {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
}

const OPENAI_PRESETS: OpenAIPreset[] = [
  { id: "openai",     name: "OpenAI",     baseUrl: "https://api.openai.com/v1",        model: "gpt-4o-mini" },
  { id: "deepseek",   name: "DeepSeek",   baseUrl: "https://api.deepseek.com/v1",      model: "deepseek-chat" },
  { id: "zhipu",      name: "Zhipu",      baseUrl: "https://open.bigmodel.cn/api/paas/v4", model: "glm-4-flash" },
  { id: "ollama",     name: "Ollama",     baseUrl: "http://localhost:11434/v1",        model: "llama3.1" },
  { id: "openrouter", name: "OpenRouter", baseUrl: "https://openrouter.ai/api/v1",     model: "openai/gpt-4o-mini" },
  { id: "custom",     name: "Custom",     baseUrl: "",                                  model: "" },
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
}: {
  options: Record<string, unknown>;
  onSave: (o: Record<string, unknown>) => void;
}) {
  const [apiKey, setApiKey] = useState("");
  const [saved, setSaved] = useState(false);
  const baseUrl = String(options.baseUrl ?? "");
  const model = String(options.model ?? "");
  const preset = detectPreset(baseUrl, model);
  const locked = preset !== "custom";

  // BH-8.7: green toast auto-dismisses after 2 s.
  useEffect(() => {
    if (!saved) return;
    const t = setTimeout(() => setSaved(false), 2000);
    return () => clearTimeout(t);
  }, [saved]);

  return (
    <div className="space-y-2">
      <div>
        <label className="label">Presets</label>
        <select
          className="input"
          aria-label="OpenAI-compatible preset"
          value={preset}
          onChange={(e) => {
            const next = OPENAI_PRESETS.find((p) => p.id === e.target.value);
            if (!next) return;
            onSave({ ...options, baseUrl: next.baseUrl, model: next.model });
          }}
        >
          {OPENAI_PRESETS.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <div>
          <label className="label">Base URL</label>
          <input
            className="input"
            aria-label="OpenAI-compatible base URL"
            value={baseUrl}
            disabled={locked}
            onChange={(e) => onSave({ ...options, baseUrl: e.target.value })}
          />
        </div>
        <div>
          <label className="label">Model</label>
          <input
            className="input"
            aria-label="OpenAI-compatible model name"
            value={model}
            disabled={locked}
            onChange={(e) => onSave({ ...options, model: e.target.value })}
          />
        </div>
      </div>
      <div>
        <label className="label">API key</label>
        <div className="flex gap-2">
          <input
            type="password"
            className="input"
            aria-label="OpenAI-compatible API key"
            placeholder="••••••"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
          <button
            className="btn btn-primary"
            aria-label="Save OpenAI-compatible API key to OS Keychain"
            disabled={!apiKey}
            onClick={async () => {
              try {
                await api.setApiKey("openai", apiKey);
                setApiKey("");
                setSaved(true);
              } catch {
                /* noop */
              }
            }}
          >
            Save
          </button>
        </div>
        {saved && <p className="mt-1 text-xs text-green-500">Saved to OS Keychain.</p>}
      </div>
    </div>
  );
}
