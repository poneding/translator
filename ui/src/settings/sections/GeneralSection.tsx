import { useState } from "react";
import { useConfigStore } from "../../stores/config";

// BH-9.1: SPEC mandates these 12 target-language options.
const TARGET_LANGS: Array<{ code: string; label: string }> = [
  { code: "en",      label: "English" },
  { code: "zh-Hans", label: "Simplified Chinese" },
  { code: "zh-Hant", label: "Traditional Chinese" },
  { code: "ja",      label: "Japanese" },
  { code: "ko",      label: "Korean" },
  { code: "fr",      label: "French" },
  { code: "de",      label: "German" },
  { code: "es",      label: "Spanish" },
  { code: "ru",      label: "Russian" },
  { code: "pt",      label: "Portuguese" },
  { code: "it",      label: "Italian" },
  { code: "ar",      label: "Arabic" },
];

// BH-9.3: BCP-47 primary-language-subtag format. Accepts any of the
// SPEC options above plus the 2-3 letter ISO codes and (optional) script
// subtags. Strict-but-permissive: primary tag is mandatory, script is
// optional and limited to a-z letters.
const BCP47_RE = /^[A-Za-z]{2,3}(?:-[A-Za-z]{4})?(?:-(?:[A-Za-z]{2}|[0-9]{3}))*$/;

export function GeneralSection() {
  const { config, save } = useConfigStore();
  const [targetErr, setTargetErr] = useState<string | null>(null);
  const [fromErr, setFromErr] = useState<string | null>(null);
  if (!config) return null;

  return (
    <div className="space-y-4">
      <div>
        <label className="label">Target language</label>
        <select
          className="input"
          value={
            TARGET_LANGS.some((l) => l.code === config.general.target_language)
              ? config.general.target_language
              : "__custom__"
          }
          onChange={(e) => {
            const v = e.target.value;
            if (v === "__custom__") return;
            setTargetErr(null);
            void save({ ...config, general: { ...config.general, target_language: v } });
          }}
        >
          {TARGET_LANGS.map((l) => (
            <option key={l.code} value={l.code}>
              {l.label} ({l.code})
            </option>
          ))}
        </select>
        <p className="mt-1 text-xs text-fg-subtle">
          BH-9.1: pick from the 12 SPEC-mandated options. Edit the config file
          directly for a custom value (BH-9.3 BCP-47 validation also applies).
        </p>
        <div className="mt-2">
          <label className="text-xs text-fg-subtle">Custom BCP-47 value</label>
          <input
            className={"input " + (targetErr ? "border-red-500" : "")}
            placeholder="e.g. zh-Hans, en-US, sr-Latn"
            defaultValue={config.general.target_language}
            onBlur={(e) => {
              const v = e.target.value.trim();
              if (!BCP47_RE.test(v)) {
                setTargetErr("Invalid BCP-47 code (e.g. zh-Hans, en-US).");
                return;
              }
              setTargetErr(null);
              void save({ ...config, general: { ...config.general, target_language: v } });
            }}
          />
          {targetErr && <p className="mt-1 text-xs text-red-500">{targetErr}</p>}
        </div>
      </div>
      <div>
        <label className="label">Default source</label>
        <input
          className={"input " + (fromErr ? "border-red-500" : "")}
          value={config.general.default_from}
          onChange={(e) => {
            const v = e.target.value.trim();
            if (v !== "auto" && !BCP47_RE.test(v)) {
              setFromErr('Use "auto" or a BCP-47 code (e.g. en, zh-Hans).');
              return;
            }
            setFromErr(null);
            void save({ ...config, general: { ...config.general, default_from: v } });
          }}
        />
        <p className="mt-1 text-xs text-fg-subtle">
          Use <code>auto</code> to let each service detect the source language.
        </p>
        {fromErr && <p className="mt-1 text-xs text-red-500">{fromErr}</p>}
      </div>
    </div>
  );
}
