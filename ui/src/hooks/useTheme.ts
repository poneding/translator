// BH-11.x: single source of truth for the app theme.
//
// - Reads config.general.theme ("system" | "light" | "dark").
// - When the user picks "system", we follow the OS appearance and update
//   live when the OS appearance changes (BH-11.2).
// - When the user picks "light" or "dark", we lock to that value and ignore
//   the OS.
// - We mutate document.documentElement.dataset.theme. CSS in app.css reacts
//   to data-theme="light" / data-theme="dark"; absence of the attribute
//   means "follow the system" (the prefers-color-scheme media query takes
//   over).
//
// OS theme detection uses the WebView's native `matchMedia` API — the same
// media query the CSS already consumes. This avoids needing a custom Rust
// command or relying on a plugin export that isn't in the current
// @tauri-apps/plugin-os v2.0.0 JS surface.

import { useEffect } from "react";

export type ResolvedTheme = "light" | "dark";
export type ThemeChoice = ResolvedTheme | "system";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function apply(choice: ThemeChoice, os: ResolvedTheme | null): void {
  if (typeof document === "undefined") return;
  const el = document.documentElement;
  if (choice === "system") {
    // No data-theme → CSS media query decides the palette.
    delete el.dataset.theme;
    if (os) el.dataset.osTheme = os;
    else delete el.dataset.osTheme;
  } else {
    el.dataset.theme = choice;
  }
}

export function useTheme(choice: ThemeChoice): void {
  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      // No media-query support (very old WebView) — fall back to the choice.
      apply(choice, null);
      return;
    }

    const mql = window.matchMedia(DARK_QUERY);

    // 1. Paint the initial resolved theme immediately so there's no flash.
    const os0: ResolvedTheme | null = mql.matches ? "dark" : "light";
    apply(choice, os0);

    // 2. BH-11.2: while the choice is "system", listen for the WebView's
    //    live OS theme change. The event fires on macOS / Windows / Linux
    //    when the user toggles dark mode in system settings.
    const onChange = (e: MediaQueryListEvent) => {
      if (choice === "system") apply("system", e.matches ? "dark" : "light");
    };
    mql.addEventListener("change", onChange);

    // 3. Apply the choice itself (handles "system" → "light"/"dark" etc.)
    apply(choice, mql.matches ? "dark" : "light");

    return () => {
      mql.removeEventListener("change", onChange);
    };
  }, [choice]);
}

