# translator - Behavioral Specification (v0.2.0 draft)

> Status: Draft for next-stage development
> Audience: implementer, reviewer, tester
> Scope: v0.2.0 UX and update-system iteration after the published v0.1.0 release.
> Companion plan: [PLAN-v0.2.0.md](PLAN-v0.2.0.md)

---

## 1. Scope

### 1.1 One-sentence definition

`translator` becomes a main-window-first translator: press the global hotkey,
optionally read selected or clipboard text, translate in the main window, and
manage updates, languages, services, and appearance from one integrated UI.

### 1.2 v0.2.0 deliverables

- Custom titlebar shows a pin button, centered app logo + app name, and no subtitle.
- Main title text has the app logo immediately before it.
- The floating quick-translation popup is removed from the product flow; hotkey translation uses the main window.
- Hotkey translation reads selected text first; when configured, it falls back to clipboard text.
- Main window supports pin/always-on-top and Escape-to-close.
- Source editor is redesigned with an always-visible bottom action row.
- Source and target language controls use a shared language registry. The compact
  main direction row shows short codes only; dropdown options show language name
  plus short code.
- Result cards have compact one-line service headers and per-service retry buttons.
- Audio controls distinguish source-text speech from translated-text speech.
- Settings adds an Update section with startup auto-check, beta update eligibility, and manual check.
- Appearance settings include app language selection for all common translation languages.

### 1.3 Explicit non-goals

| Category | Excluded from v0.2.0 |
| --- | --- |
| Translation inputs | OCR, screenshot translation, image translation, text replacement into the source app |
| Update infrastructure | Self-hosted update backend beyond the JSON/release endpoints needed by Tauri updater |
| Audio | Offline TTS engines, voice selection beyond the provider/default accent |
| Localization | Professional copy review for all 12 locales; v0.2.0 requires complete UI strings but not marketing-quality prose |
| Windowing | Multiple simultaneous translator windows |

---

## 2. Personas & Use Cases

### 2.1 Personas

1. Daily hotkey user: wants one predictable main window, not a separate popup.
2. Clipboard-heavy user: often copies text first and wants the hotkey to translate it.
3. Beta tester: wants prerelease updates without manually visiting GitHub Releases.
4. Multilingual user: wants the app UI itself in their preferred language.

### 2.2 Use cases

| ID | Trigger | Outcome |
| --- | --- | --- |
| UC-1 | User presses global hotkey with selected text | Main window opens, source text is filled, translation starts automatically. |
| UC-2 | User presses global hotkey with no selection and clipboard fallback enabled | Main window opens, clipboard text is filled, translation starts automatically. |
| UC-3 | User presses global hotkey with no usable text | Main window opens without translation and shows an empty source state. |
| UC-4 | User clicks Pin | Main window toggles always-on-top and the icon state reflects it. |
| UC-5 | User changes target language in the source editor toolbar | Future translations use the selected target language immediately. |
| UC-6 | User hovers a service result and clicks its refresh button | Only that service re-runs and only that result card updates. |
| UC-8 | User clicks source audio | Source text is spoken in the source/detected language. |
| UC-9 | User clicks result audio | Translated text is spoken in the target language. |
| UC-10 | User opens Settings -> Update and checks manually | App shows checking, no-update, update-available, or error state. |
| UC-11 | User enables beta updates | Manual and startup checks can offer prerelease versions. |
| UC-12 | User changes app language under Appearance | UI language changes without restart and persists. |

---

## 3. User-Observable Behaviors

### 3.1 Titlebar and main window

| ID | Behavior |
| --- | --- |
| BH-1.1 | The main titlebar title is centered and contains the app logo followed by the localized app name. |
| BH-1.2 | The titlebar does not show the old subtitle/tagline. |
| BH-1.3 | A Pin icon button is the leftmost app control in the titlebar. On macOS, native traffic-light controls may remain in their reserved area, but Pin is the first app-owned control. |
| BH-1.4 | Clicking Pin toggles the main window's always-on-top state. The icon has a distinct active state while pinned. |
| BH-1.5 | Pin state persists across app restarts. |
| BH-1.6 | The titlebar remains draggable except on interactive controls. |
| BH-1.7 | Pressing Escape in the main window hides the main window, unless an open menu/dialog consumes Escape first. The app remains running in the tray/menu bar. |

### 3.2 Hotkey translation flow

| ID | Behavior |
| --- | --- |
| BH-2.1 | Pressing the global hotkey always targets the main window, not a popup. |
| BH-2.2 | If non-empty selected text is available, selected text wins over clipboard text. |
| BH-2.3 | If selected text is unavailable/empty and `auto_translate_clipboard_on_hotkey` is enabled, the app reads clipboard text. |
| BH-2.4 | Clipboard fallback is checked only when the hotkey is pressed; the app does not continuously monitor the clipboard. |
| BH-2.5 | If source text is found from selection or clipboard, the main window shows, source text is filled, previous results are replaced with pending rows, and translation starts automatically. |
| BH-2.6 | If no usable text is found, the main window shows without starting network requests. |
| BH-2.7 | The removed popup window is not created, shown, focused, or referenced by hotkey flow. |
| BH-2.8 | Permission errors from selection reading are shown in the main window with the existing localized "Open Settings" action. |

### 3.3 Source editor

| ID | Behavior |
| --- | --- |
| BH-3.1 | The source editor height grows with text content from a minimum height to a bounded maximum height; after the maximum, text scrolls inside the editor. |
| BH-3.2 | The source editor has an always-visible bottom toolbar row inside the editor container. Text never overlaps or hides behind the toolbar. |
| BH-3.3 | Source toolbar left side contains ghost buttons: Play source audio, Copy source text. |
| BH-3.4 | The language direction row below the source editor shows `source short code => target short code`. Source language is derived from detection or Auto Detect and is not editable. Target language is a dropdown. |
| BH-3.5 | Source toolbar right side contains Clear and Translate controls. Clear is ghost-style; Translate is the primary command. |
| BH-3.6 | Play, Copy, Clear, Translate are disabled when their required source text is empty. |
| BH-3.7 | Manual edits to source text clear stale errors but do not automatically translate until Translate, global hotkey, or service refresh is invoked. |

### 3.4 Language registry and dropdowns

| ID | Behavior |
| --- | --- |
| BH-4.1 | App uses one shared language registry for translation language controls and app-language controls. |
| BH-4.2 | Common translation languages are: English, Simplified Chinese, Traditional Chinese, Japanese, Korean, French, German, Spanish, Russian, Portuguese, Italian, Arabic. |
| BH-4.3 | Main source/target direction controls show short codes in the compact row. Dropdown options show language display name and short code. |
| BH-4.4 | Flag markers may be used in settings/app-language controls as visual markers only; the canonical persisted value is still a BCP-47-style language code. |
| BH-4.5 | The compact main direction row does not show flags. |
| BH-4.6 | Unknown/custom BCP-47 values remain supported in General settings but are not shown in the compact main toolbar unless currently selected. |

### 3.5 Main status row and translation dispatch

| ID | Behavior |
| --- | --- |
| BH-5.1 | Below the source editor, the left side shows source short code -> target short-code dropdown. |
| BH-5.2 | Below the source editor, the right side shows "N services enabled" and the enabled service logo group in priority order. |
| BH-5.3 | Translate uses the current source text and target language and replaces all current outcomes with pending rows. |
| BH-5.4 | Empty source text prevents translation and shows empty-source validation. |
| BH-5.5 | Services remain dispatched in configured priority order and independently succeed/fail. |
| BH-5.6 | A service with missing credentials is skipped according to the existing service policy. |

### 3.6 Result cards and per-service refresh

| ID | Behavior |
| --- | --- |
| BH-6.1 | Result cards no longer display a detected source language line. |
| BH-6.2 | Each service header is one line: small service logo plus localized service name, for example `[Youdao icon] Youdao Translate`. |
| BH-6.3 | Service logo in result headers is compact, 16px or smaller unless the design system requires a nearby equivalent. |
| BH-6.4 | Each service card has a refresh button that is hidden until the card is hovered or keyboard-focused. |
| BH-6.5 | Clicking a service refresh button re-runs only that service with the current source text and target language. Other service cards remain unchanged. |
| BH-6.6 | While a single service is refreshing, that card shows pending state and disables its own refresh button. |
| BH-6.7 | Per-service refresh results are guarded by request ids so stale responses cannot overwrite newer results. |

### 3.7 Audio and copy controls

| ID | Behavior |
| --- | --- |
| BH-7.1 | Source editor Play speaks the source text using the source/detected language when known, otherwise Auto Detect/default provider behavior. |
| BH-7.2 | Result Play speaks the translated text using the target language. It must not play the original source text. |
| BH-7.3 | Dictionary phonetic audio remains source-word pronunciation and is shown only inside dictionary details, not as the main result audio. |
| BH-7.4 | Audio buttons use ghost button styling. |
| BH-7.5 | Copy buttons use ghost button styling. |
| BH-7.6 | While audio is playing, the icon alternates between `Volume1` and `Volume2` until playback ends or errors. It never changes to a square/stop icon. |
| BH-7.7 | Clicking the same audio button during playback stops that audio and returns the icon to idle state. |
| BH-7.8 | Starting another audio playback stops any currently playing app audio. |

### 3.8 Settings: Update

| ID | Behavior |
| --- | --- |
| BH-8.1 | Settings sidebar has an Update section with an appropriate icon. |
| BH-8.2 | Update settings include `Check for updates on startup`. New configs default to enabled. |
| BH-8.3 | Startup update checks run asynchronously after app initialization and never block window creation, hotkey registration, tray setup, or config loading. |
| BH-8.4 | Update settings include `Allow beta versions`. Default is disabled. |
| BH-8.5 | When beta is disabled, prerelease versions are ignored. When beta is enabled, eligible updates include stable and prerelease versions newer than the current app version. |
| BH-8.6 | Update section includes a manual `Check for updates` action. |
| BH-8.7 | Manual and startup checks report one of: idle, checking, up to date, update available, download/install progress, installed/restart needed, failed. |
| BH-8.8 | An available update shows version, channel, release date when available, and a concise release-note summary or link. |
| BH-8.9 | Download/install requires an explicit user action. The app does not silently install updates. |
| BH-8.10 | On Windows, if the updater must exit the app before installation, the UI clearly says so before the user confirms. |
| BH-8.11 | Update failures do not show modal alerts on startup; they are logged and shown in Settings -> Update. Manual check failures show inline error text. |

### 3.9 Settings: Appearance and app language

| ID | Behavior |
| --- | --- |
| BH-9.1 | Appearance settings include an App Language dropdown. |
| BH-9.2 | App Language options are System plus all common translation languages listed in BH-4.2. |
| BH-9.3 | Changing App Language updates visible UI strings without app restart. |
| BH-9.4 | App Language persists in config. `system` follows OS locale using the existing locale-detection rules. |
| BH-9.5 | Release builds include Fluent locale resources for every App Language option. Missing keys fail CI. |

### 3.10 Configuration and migration

| ID | Behavior |
| --- | --- |
| BH-10.1 | Existing v0.1.0 config files migrate without data loss. |
| BH-10.2 | New config fields have defaults: `window.always_on_top=false`, `general.app_language=system`, `general.auto_translate_clipboard_on_hotkey=false`, `updates.check_on_startup=true`, `updates.allow_beta=false`. |
| BH-10.3 | Removed popup settings, if any, are ignored on read and not written back after save. |
| BH-10.4 | Language and update settings are included in config update broadcasts so open UI reacts immediately. |

### 3.11 Performance and accessibility

| ID | Behavior |
| --- | --- |
| BH-11.1 | Hotkey to main-window visible target: under 150 ms excluding network translation. |
| BH-11.2 | Startup update check begins no earlier than first config load and does not add more than 50 ms to first window paint. |
| BH-11.3 | All icon buttons have localized accessible names and tooltips. |
| BH-11.4 | Hover-only service refresh is also reachable by keyboard focus. |
| BH-11.5 | Text in toolbar controls never overlaps at the minimum supported window size. |

---

## 4. Acceptance Criteria

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --all-features` pass.
- `npm run typecheck`, `npm run lint`, and `npm run build` pass.
- `actionlint .github/workflows/*.yml` passes.
- Manual QA confirms hotkey selection, clipboard fallback, pin, Escape close,
  language switching, update check, per-service refresh, source audio, result
  audio, and ghost copy/play controls.
- Release workflow produces updater artifacts required by the configured updater endpoint.

---

## 5. Implementation Notes

- Use Tauri updater rather than a custom updater. The official updater requires update artifacts, a public key, endpoints, and `updater:default` permissions.
- Runtime updater endpoints should support channel selection so `allow_beta` can switch between stable-only and beta-eligible checks.
- Current `TranslateResult.audio_url` behavior is source-oriented for Youdao. v0.2.0 must redefine or extend the model so the main result audio URL points to translated text.
- Removing the popup should include deleting or disabling stale `popup.html`, popup Tauri window config, popup IPC, and popup positioning code only after main-window hotkey flow is complete.
