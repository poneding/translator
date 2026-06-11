# translator v0.2.0 — Manual QA Checklist

Each release of v0.2.0 must be manually exercised on at least one host per
target platform (macOS, Windows, Linux). The results are recorded as
`qa/<date>-<platform>.md` (e.g. `qa/2026-06-02-windows.md`).

This file is the **generic** checklist. Platform-specific notes go in the
per-platform run file.

## Setup (one-time per host)

- [ ] Install the bundle (`.dmg`/`.msi`/`.AppImage`/`.deb`) and launch it.
- [ ] Confirm the **tray icon** is visible (menu bar on macOS, system tray on
  Windows/Linux).
- [ ] Open **Settings** from the tray menu. Settings opens inside the main
  window; the window starts at the compact width, has a maximum width, and has
  no maximize control.

## Configuration

- [ ] In **Settings → Services**, fill in at least one API key (e.g. Youdao).
  The status indicator turns green when the key is saved; a green toast
  "Saved to OS Keychain" appears for ~2 s. (BH-8.7)
- [ ] In **Settings → General**, set the target language to `en` and verify
  it persists across app restart.
- [ ] In **Settings → General**, toggle clipboard fallback for the hotkey and
  verify it persists across app restart.
- [ ] In **Settings → Hotkey**, change the shortcut to `CmdOrCtrl+Shift+T`
  and confirm the new shortcut opens the main translator immediately.
- [ ] In **Settings → Appearance**, switch theme `System` → `Light` → `Dark`
  → `System` and confirm the UI updates live. (BH-11.1, BH-11.2)
- [ ] In **Settings → Appearance**, change App Language and confirm visible UI
  strings update without restart.
- [ ] In **Settings → Update**, toggle startup checks and beta eligibility,
  then run a manual update check.
- [ ] In **Settings → Services → Youdao**, click **Remove**; status indicator
  turns red. (BH-8.8)

## Core flows

- [ ] **Selection translate**:
  - [ ] Select text in a non-Easydict app (browser, editor, etc.).
  - [ ] Press the global hotkey.
  - [ ] The main window appears within 150 ms excluding network time.
  - [ ] The selected text is filled into the source editor.
  - [ ] Results stream in service priority order.
- [ ] **Clipboard fallback**:
  - [ ] Disable fallback; press hotkey with no selection and verify no
    translation starts.
  - [ ] Enable fallback; copy text, press hotkey with no selection, and verify
    the clipboard text translates.
- [ ] **Pin** toggles always-on-top and persists after restart.
- [ ] **Escape** hides the main window while the tray process stays alive.
- [ ] The source editor grows with content and its bottom toolbar remains
  visible without overlapping text.
- [ ] The language direction row shows source short code -> target short-code
  dropdown below the source editor; dropdown options show language name plus
  short code.
- [ ] Hover/focus a result card and click its refresh button; only that card
  refreshes.
- [ ] Result card headers are one compact line with a small service logo and
  no detected-source line.
- [ ] Source Play speaks source text; result Play speaks translated text.
- [ ] Play icons alternate between `volume-1` and `volume-2` until playback
  finishes; they never become a square.

## Service behaviors

For each enabled service, with a real API key configured:

- [ ] Youdao translation works end-to-end (HMAC-SHA256 sign).
- [ ] DeepL translation works (note: DeepL Pro keys differ in prefix).
- [ ] Google Cloud Translation v3 works (project id + API key).
- [ ] Bing/Azure Translator works (subscription key + region).
- [ ] OpenAI-compatible works (test the OpenAI preset, then DeepSeek, then
  Ollama local).

## Error paths

- [ ] **No selection**: pressing the hotkey with no selected text opens the
  main window without starting network requests when clipboard fallback is
  disabled.
- [ ] **No services enabled**: shows "No services enabled. Open Settings to
  enable one."
- [ ] **Service without API key**: that service is shown as unavailable or
  skipped according to the configured service policy.
- [ ] **Permission denied** (first run on macOS): main window shows
  "translator needs the Accessibility permission" + an "Open Settings"
  button.
- [ ] **Service returns 401/403**: tab shows "API key invalid".
  (SPEC §5)
- [ ] **Service returns 429**: tab shows "Rate limited". (SPEC §5)
- [ ] **Service times out (>8 s)**: tab shows "Translation timed out".
  (SPEC §5, BH-4.2)
- [ ] **All services fail**: result cards show the service errors; each card's
  hover/focus refresh button can retry that service.
- [ ] **Network disconnected** mid-translation: per-service errors are
  shown, the app does not crash. (SPEC §4.3)

## System integration

- [ ] **Tray menu** has "Open Translator", "Open Settings", and "Quit".
- [ ] **Tray left-click** opens the main translator window.
- [ ] On **macOS**, the tray icon is a template (monochrome) image and
  adapts to the menu bar's light/dark appearance. (BH-14.3)
- [ ] On **Windows/Linux**, the tray icon is a color image. (BH-14.4)
- [ ] **Quit** from the tray menu exits the process. Closing the settings
  window does NOT quit.
- [ ] The OS Keychain is used: no API key is written to `config.json`.
  (SPEC §4.2) — verify by inspecting `config.json`.

## Performance

- [ ] Cold start to tray visible: < 1.5 s on Apple M1 / < 2.5 s on Windows
  10 i5. (SPEC §4.1)
- [ ] Hotkey press to main window visible: < 150 ms (excluding network).
  (SPEC §4.1)
- [ ] Settings window open: < 300 ms. (SPEC §4.1)
- [ ] Idle memory: < 50 MB. (SPEC §4.1)
- [ ] Binary size: < 8 MB per platform (uncompressed). (SPEC §4.1)

## Localization

- [ ] Default UI language is English. (BH-13.1)
- [ ] When the OS locale is `zh-Hans` or `zh-CN`, the UI switches to
  Simplified Chinese on the next launch. (BH-13.2)
- [ ] All 12 app language files pass the locale key checker.

## Updates

- [ ] Startup update check is asynchronous and does not delay tray, hotkey, or
  first paint.
- [ ] Manual update check shows checking, up-to-date, available, and failed
  states with inline text.
- [ ] Beta disabled ignores prerelease manifests; beta enabled accepts a newer
  prerelease.
- [ ] Download/install requires an explicit click and reports progress.

## Final checks

- [ ] No crash during a 5-minute run with the hotkey pressed ~20 times.
- [ ] Quit cleanly from the tray menu (process exits, no zombie tasks).

## Record results

Copy this checklist into `qa/<date>-<platform>.md`, mark each box, and add
any observations at the bottom. The release is not "Done" until at least
one file per platform exists.
