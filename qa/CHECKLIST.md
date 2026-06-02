# translator v0.1.0 — Manual QA Checklist

Each release of v0.1.0 must be manually exercised on at least one host per
target platform (macOS, Windows, Linux). The results are recorded as
`qa/<date>-<platform>.md` (e.g. `qa/2026-06-02-windows.md`).

This file is the **generic** checklist. Platform-specific notes go in the
per-platform run file.

## Setup (one-time per host)

- [ ] Install the bundle (`.dmg`/`.msi`/`.AppImage`/`.deb`) and launch it.
- [ ] Confirm the **tray icon** is visible (menu bar on macOS, system tray on
  Windows/Linux).
- [ ] Open the **Settings** window from the tray menu. The window is
  resizable, min 720×480, default 880×640. (BH-7.3)

## Configuration

- [ ] In **Settings → Services**, fill in at least one API key (e.g. Youdao).
  The status indicator turns green when the key is saved; a green toast
  "Saved to OS Keychain" appears for ~2 s. (BH-8.7)
- [ ] In **Settings → General**, set the target language to `en` and verify
  it persists across app restart.
- [ ] In **Settings → Hotkey**, change the shortcut to `CmdOrCtrl+Shift+T`
  and confirm the new shortcut fires the popup immediately. (BH-10.3)
- [ ] In **Settings → Appearance**, switch theme `System` → `Light` → `Dark`
  → `System` and confirm the UI updates live. (BH-11.1, BH-11.2)
- [ ] In **Settings → Services → Youdao**, click **Remove**; status indicator
  turns red. (BH-8.8)

## Core flows

- [ ] **Selection translate** (BH-3.1, BH-3.2, BH-5.3):
  - [ ] Select text in a non-Easydict app (browser, editor, etc.).
  - [ ] Press the global hotkey.
  - [ ] The floating popup appears within 150 ms near the selection.
  - [ ] The popup is always-on-top, borderless, transparent, no shadow on
    macOS menu bar. (BH-5.1)
  - [ ] The popup shows the source text in a muted box at the top. (BH-5.7)
  - [ ] The popup shows a "Translating…" loader, then streams results
    tab-by-tab as they arrive. (BH-4.1, BH-5.6)
  - [ ] Tabs are ordered by **priority** (lowest number first). (BH-4.5)
- [ ] **Retry** button in the popup header re-runs all enabled services.
  (BH-6.2)
- [ ] **Copy** button on a result tab copies the translated text; label
  flashes to "Copied" for ~1.5 s. (BH-6.1)
- [ ] **Escape** closes the popup. (BH-6.4)
- [ ] Clicking outside the popup closes it after ~500 ms (focus-grace
  period). (BH-5.8)
- [ ] The popup does **not** steal focus from the source app. (BH-6.5)
- [ ] On a multi-monitor setup, the popup appears on the monitor where the
  selection was made. (BH-5.5)

## Service behaviors

For each enabled service, with a real API key configured:

- [ ] Youdao translation works end-to-end (HMAC-SHA256 sign).
- [ ] DeepL translation works (note: DeepL Pro keys differ in prefix).
- [ ] Google Cloud Translation v3 works (project id + API key).
- [ ] Bing/Azure Translator works (subscription key + region).
- [ ] OpenAI-compatible works (test the OpenAI preset, then DeepSeek, then
  Ollama local).

## Error paths

- [ ] **No selection**: pressing the hotkey with no selected text shows
  "No text is selected". (SPEC §5)
- [ ] **No services enabled**: shows "No services enabled. Open Settings to
  enable one."
- [ ] **Service without API key**: that service is silently skipped — no
  tab appears in the popup. (BH-4.3)
- [ ] **Permission denied** (first run on macOS): popup shows
  "translator needs the Accessibility permission" + an "Open Settings"
  button. (SPEC §5, BH-3.3)
- [ ] **Service returns 401/403**: tab shows "API key invalid".
  (SPEC §5)
- [ ] **Service returns 429**: tab shows "Rate limited". (SPEC §5)
- [ ] **Service times out (>8 s)**: tab shows "Translation timed out".
  (SPEC §5, BH-4.2)
- [ ] **All services fail**: popup shows the list of errors with a Retry
  button. (SPEC §5)
- [ ] **Network disconnected** mid-translation: per-service errors are
  shown, the app does not crash. (SPEC §4.3)

## System integration

- [ ] **Tray menu** has "Open Settings" and "Quit" items only. (BH-14.2)
- [ ] **Tray left-click** opens the settings window. (BH-14.1)
- [ ] On **macOS**, the tray icon is a template (monochrome) image and
  adapts to the menu bar's light/dark appearance. (BH-14.3)
- [ ] On **Windows/Linux**, the tray icon is a color image. (BH-14.4)
- [ ] **Quit** from the tray menu exits the process. Closing the settings
  or popup windows does NOT quit. (BH-14.5)
- [ ] The OS Keychain is used: no API key is written to `config.json`.
  (SPEC §4.2) — verify by inspecting `config.json`.

## Performance

- [ ] Cold start to tray visible: < 1.5 s on Apple M1 / < 2.5 s on Windows
  10 i5. (SPEC §4.1)
- [ ] Hotkey press to popup visible: < 150 ms (excluding network).
  (SPEC §4.1)
- [ ] Settings window open: < 300 ms. (SPEC §4.1)
- [ ] Idle memory: < 50 MB. (SPEC §4.1)
- [ ] Binary size: < 8 MB per platform (uncompressed). (SPEC §4.1)

## Localization

- [ ] Default UI language is English. (BH-13.1)
- [ ] When the OS locale is `zh-Hans` or `zh-CN`, the UI switches to
  Simplified Chinese on the next launch. (BH-13.2)

## Final checks

- [ ] No crash during a 5-minute run with the hotkey pressed ~20 times.
- [ ] Quit cleanly from the tray menu (process exits, no zombie tasks).

## Record results

Copy this checklist into `qa/<date>-<platform>.md`, mark each box, and add
any observations at the bottom. The release is not "Done" until at least
one file per platform exists.
