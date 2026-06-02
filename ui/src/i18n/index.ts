// BH-13.x: minimal Fluent-backed i18n for the React UI.
//
// - Loads en.ftl + zh-Hans.ftl at module init via Vite's ?raw imports.
// - Selects the bundle based on navigator.language:
//   - starts with "zh-Hans" or "zh-CN" → zh-Hans
//   - anything else → en (default per SPEC §3.13 BH-13.1)
// - No runtime switcher in v0.1.0 (SPEC §3.13 BH-13.3).
// - All user-facing strings live in locales/{en,zh-Hans}.ftl; the Rust
//   backend has no user-facing strings (SPEC §3.13 BH-13.4).
//
// The API is intentionally tiny: `useT()` returns a `t(key, args?)` function
// scoped to the current locale. Components that want a hardcoded fallback
// (so the UI is still readable if a key is missing) call `t(key, args, fallback)`.

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import enSource from "../locales/en.ftl?raw";
import zhHansSource from "../locales/zh-Hans.ftl?raw";

const SUPPORTED = ["en", "zh-Hans"] as const;
type SupportedLocale = (typeof SUPPORTED)[number];

function detectLocale(): SupportedLocale {
  // navigator.language is a BCP-47 tag like "en-US" or "zh-Hans-CN".
  // SPEC §3.13 BH-13.2: switch to Simplified Chinese when the OS locale
  // starts with "zh-Hans" or "zh-CN".
  const raw = (typeof navigator !== "undefined" && navigator.language) || "en";
  const lower = raw.toLowerCase();
  if (lower.startsWith("zh-hans") || lower.startsWith("zh-cn")) return "zh-Hans";
  return "en";
}

const bundles: Record<SupportedLocale, FluentBundle> = (() => {
  const result = {} as Record<SupportedLocale, FluentBundle>;
  for (const loc of SUPPORTED) {
    const source = loc === "en" ? enSource : zhHansSource;
    const resource = new FluentResource(source);
    const bundle = new FluentBundle(loc);
    const errors = bundle.addResource(resource);
    if (errors.length > 0) {
      // Don't throw — fall back to passing the key through as the display
      // text so missing translations are still visible during dev.
      console.error(`[i18n] ${loc} parse errors:`, errors);
    }
    result[loc] = bundle;
  }
  return result;
})();

let currentLocale: SupportedLocale = detectLocale();

export function getLocale(): SupportedLocale {
  return currentLocale;
}

export function setLocale(loc: SupportedLocale): void {
  if (SUPPORTED.includes(loc)) currentLocale = loc;
}

export function translate(
  key: string,
  args?: Record<string, FluentVariable> | null,
  fallback?: string,
): string {
  const bundle = bundles[currentLocale];
  const msg = bundle.getMessage(key);
  if (msg && msg.value) {
    return bundle.formatPattern(msg.value, args ?? undefined);
  }
  // Fall back to the other locale before showing a raw key.
  const other = currentLocale === "en" ? "zh-Hans" : "en";
  const otherMsg = bundles[other].getMessage(key);
  if (otherMsg && otherMsg.value) {
    return bundles[other].formatPattern(otherMsg.value, args ?? undefined);
  }
  return fallback ?? key;
}

// Lightweight React hook. Re-renders the caller when the locale changes
// (locale is module-singleton in v0.1.0, so this is effectively a no-op
// today, but it future-proofs the API for the v0.2 runtime switcher).
export function useT() {
  return (
    key: string,
    args?: Record<string, FluentVariable> | null,
    fallback?: string,
  ) => translate(key, args, fallback);
}
