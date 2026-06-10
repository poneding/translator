// Fluent-backed i18n for the React UI.

import {
  FluentBundle,
  FluentResource,
  type FluentVariable,
} from "@fluent/bundle";
import { useCallback, useSyncExternalStore } from "react";
import {
  normalizeAppLanguage,
  type AppLanguageCode,
  type CommonLanguageCode,
} from "./languages";
import arSource from "../locales/ar.ftl?raw";
import deSource from "../locales/de.ftl?raw";
import enSource from "../locales/en.ftl?raw";
import esSource from "../locales/es.ftl?raw";
import frSource from "../locales/fr.ftl?raw";
import itSource from "../locales/it.ftl?raw";
import jaSource from "../locales/ja.ftl?raw";
import koSource from "../locales/ko.ftl?raw";
import ptSource from "../locales/pt.ftl?raw";
import ruSource from "../locales/ru.ftl?raw";
import zhHansSource from "../locales/zh-Hans.ftl?raw";
import zhHantSource from "../locales/zh-Hant.ftl?raw";

const SUPPORTED: readonly CommonLanguageCode[] = [
  "en",
  "zh-Hans",
  "zh-Hant",
  "ja",
  "ko",
  "fr",
  "de",
  "es",
  "ru",
  "pt",
  "it",
  "ar",
];

type SupportedLocale = CommonLanguageCode;

const localeSources: Record<SupportedLocale, string> = {
  ar: arSource,
  de: deSource,
  en: enSource,
  es: esSource,
  fr: frSource,
  it: itSource,
  ja: jaSource,
  ko: koSource,
  pt: ptSource,
  ru: ruSource,
  "zh-Hans": zhHansSource,
  "zh-Hant": zhHantSource,
};

function detectLocale(): SupportedLocale {
  // navigator.language is a BCP-47 tag like "en-US" or "zh-Hans-CN".
  const raw = (typeof navigator !== "undefined" && navigator.language) || "en";
  const normalized = normalizeAppLanguage(raw);
  return normalized === "system" ? "en" : normalized;
}

const bundles: Record<SupportedLocale, FluentBundle> = (() => {
  const result = {} as Record<SupportedLocale, FluentBundle>;
  for (const loc of SUPPORTED) {
    const resource = new FluentResource(localeSources[loc]);
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

const listeners = new Set<() => void>();
let configuredLocale: AppLanguageCode = "system";
let currentLocale: SupportedLocale = detectLocale();

function resolveLocale(locale: AppLanguageCode): SupportedLocale {
  return locale === "system" ? detectLocale() : locale;
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function emitChange() {
  for (const listener of listeners) listener();
}

export function getConfiguredLocale(): AppLanguageCode {
  return configuredLocale;
}

export function getLocale(): SupportedLocale {
  return currentLocale;
}

export function setLocale(loc: AppLanguageCode): void {
  const nextConfigured = normalizeAppLanguage(loc);
  const nextResolved = resolveLocale(nextConfigured);
  if (configuredLocale === nextConfigured && currentLocale === nextResolved)
    return;
  configuredLocale = nextConfigured;
  currentLocale = nextResolved;
  emitChange();
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
  for (const other of ["en", "zh-Hans"] as const) {
    if (other === currentLocale) continue;
    const otherMsg = bundles[other].getMessage(key);
    if (otherMsg && otherMsg.value) {
      return bundles[other].formatPattern(otherMsg.value, args ?? undefined);
    }
  }
  return fallback ?? key;
}

export function useT() {
  const locale = useSyncExternalStore(subscribe, getLocale, getLocale);
  return useCallback(
    (
      key: string,
      args?: Record<string, FluentVariable> | null,
      fallback?: string,
    ) => {
      void locale;
      return translate(key, args, fallback);
    },
    [locale],
  );
}
