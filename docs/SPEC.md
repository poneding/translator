# translator — Behavioral Specification (v0.1.0)

> **Status**: Draft for review
> **Audience**: Implementer, reviewer, tester
> **Scope**: This document is the behavioral contract for `v0.1.0`. Anything not listed under "User-observable behaviors" (§3) is either out of scope or implementation detail.
> **Companion docs**: [DESIGN.md](DESIGN.md) (architecture & rationale), [PLAN.md](PLAN.md) (milestones & tasks).

---

## 1. Scope

### 1.1 One-sentence definition

`translator` is a lightweight cross-platform select-and-translate app: select text anywhere → press a global hotkey → see translations from N enabled services in a floating popup → copy a result to the clipboard.

### 1.2 v0.1.0 deliverables

- Runs on **macOS 13+**, **Windows 10+**, **Ubuntu 22.04+** (and other distros that ship `libwebkit2gtk-4.1`).
- 5 translation services implemented: **Youdao**, **DeepL**, **Google Cloud Translation v3**, **Microsoft Bing (Azure) Translator**, **OpenAI-compatible** (OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter, custom).
- 3 platform selection monitors: macOS (`AXUIElement`), Windows (`UI Automation`), Linux (AT-SPI over D-Bus).
- System tray / menubar with: Open Settings, Quit.
- Settings window with 5 sections: General, Services, Hotkey, Appearance, About.
- Floating popup with: per-service tabs, copy button, retry, close-on-blur (500 ms delay), close-on-Escape.
- English + Simplified Chinese UI.
- API keys stored in OS Keychain (never on disk in plain text).
- Non-sensitive config in `~/.config/translator/config.json` (XDG-aware).
- Release artifacts: `.dmg` (macOS), `.msi` (Windows), `.AppImage` + `.deb` (Linux).

### 1.3 Out of scope (explicit non-goals)

| Category | Excluded |
| --- | --- |
| Input methods | OCR / screenshot translation, text replacement (write-back to source app), TTS, image translation |
| Local data | Offline dictionaries, offline translation, translation memory, glossary |
| Sync | Cloud sync, account system, history sync, team sharing |
| Platforms | iOS, Android, web, browser extension, command-line mode |
| Distribution | Auto-update, code signing infrastructure for macOS (we sign with ad-hoc / dev cert only), Sparkle-equivalent update framework |
| Plugin system | Third-party service plugins at runtime, custom service UI builder |

---

## 2. Personas & Use Cases

### 2.1 Personas

1. **Polyglot developer** — reads English + Chinese + Japanese docs daily; wants fast, parallel lookups; will pay for quality (uses DeepL + GPT).
2. **Privacy-conscious user** — runs on Linux, refuses Google/Microsoft; uses self-hosted Ollama or a single trusted API.
3. **Student** — needs free tier, configures Youdao + DeepL free keys; expects one-click copy.

### 2.2 Use cases (in scope)

| # | Actor | Trigger | Outcome |
| --- | --- | --- | --- |
| UC-1 | User | Selects text + presses hotkey | Popup appears with translations |
| UC-2 | User | Selects text, presses hotkey, no permission | Popup shows permission error + open-settings action |
| UC-3 | User | Clicks "Copy" on a result | Translation is on the system clipboard |
| UC-4 | User | Clicks tray icon → "Open Settings" | Settings window appears |
| UC-5 | User | Saves a new API key in Settings | Key is stored in OS Keychain; service becomes usable |
| UC-6 | User | Disables a service in Settings | Service stops being called and stops appearing in popup |
| UC-7 | User | Changes the global hotkey | New shortcut activates on next press |
| UC-8 | User | Presses Escape with popup focused | Popup closes |
| UC-9 | User | Clicks outside the popup | Popup closes after 500 ms |
| UC-10 | User | Switches system appearance to dark | UI follows |

---

## 3. User-observable Behaviors

Each behavior is a testable contract. The format is **Given / When / Then** with concrete expected values.

### 3.1 Global hotkey

| ID | Behavior |
| --- | --- |
| BH-1.1 | **Default hotkey** is `CmdOrCtrl+Shift+D`. |
| BH-1.2 | The hotkey works when **any app is focused**, including apps running as admin (on Windows requires the app itself to be admin; documented limitation). |
| BH-1.3 | Pressing the hotkey while the popup is visible **hides** the popup and **does not** re-trigger translation. |
| BH-1.4 | The hotkey can be re-bound via Settings; the new binding is persisted to `config.json` and applied without app restart. |
| BH-1.5 | If the OS denies the hotkey registration (conflict), a warning is logged and the default is reset on next launch; the settings UI shows a red banner. |

### 3.2 Selection reading

| ID | Behavior |
| --- | --- |
| BH-2.1 | The app reads the **currently focused** app's selected text. Selecting text in app A, then switching to app B without re-selecting, yields no text. |
| BH-2.2 | Empty or whitespace-only selections produce an "empty" error in the popup (not a crash, not a network call). |
| BH-2.3 | Text up to **100 000 characters** is supported; longer selections are truncated and a `warning` is added to the popup. |
| BH-2.4 | On macOS, the first time the hotkey is pressed the app checks `AXIsProcessTrustedWithOptions(prompt=true)`. If denied, BH-3 applies. |
| BH-2.5 | On Linux, if `dbus-daemon` is not running, the monitor returns `SelectionError::Platform` and the popup shows a clear error. |

### 3.3 Permission flow

| ID | Behavior |
| --- | --- |
| BH-3.1 | When permission is **not granted**, the popup shows the message "translator needs the Accessibility permission" (localized) and a single "Open Settings" button. |
| BH-3.2 | Clicking "Open Settings" calls the OS-specific permission settings URL (macOS: `x-apple.systempreferences:...Privacy_Accessibility`; Windows: `ms-settings:privacy-accessibility`; Linux: no-op). |
| BH-3.3 | The popup does **not** auto-retry; user must press the hotkey again after granting permission. |
| BH-3.4 | The permission state is checked **on every hotkey press** (not cached), so a user who grants permission mid-session sees the popup work immediately. |

### 3.4 Translation dispatch

| ID | Behavior |
| --- | --- |
| BH-4.1 | Translations are dispatched to all **enabled** services **in parallel** (`futures::join_all`). |
| BH-4.2 | Each service has an **8-second per-service timeout**. On timeout, that service's result is `ServiceError::Timeout`; other services' results are unaffected. |
| BH-4.3 | Services with **no configured credentials** are skipped silently (no error in popup, just no tab for that service). |
| BH-4.4 | Services whose API returns a **4xx/5xx error** are reported as `ServiceError::Api { code, message }`; the popup shows the error in that service's tab. |
| BH-4.5 | The popup displays results **in priority order** (lower priority number = first tab). |
| BH-4.6 | At least one result tab is always rendered, even if all services failed (showing the error for each). |
| BH-4.7 | The detected source language (if the service returns one) is shown in a small `detected: <lang>` line above the result. |

### 3.5 Popup display

| ID | Behavior |
| --- | --- |
| BH-5.1 | Popup window is **always-on-top**, **non-resizable**, **borderless**, **transparent background**, **no shadow on macOS menubar**; visible on all virtual desktops. |
| BH-5.2 | Default size: **480 × 320 px**. Max content: 2000 × 1500 px (popup scrolls inside). |
| BH-5.3 | Position: prefer the **selection bounds** (with 12 px margin below). If unavailable, fall back to the **cursor position** with 12 px margin below. If both unavailable, center on the focused monitor. |
| BH-5.4 | The popup is **clamped to the visible screen** — if the calculated position would push the popup off-screen, it is moved to stay on-screen. |
| BH-5.5 | On multi-monitor setups, the popup appears on the **monitor where the selection is**. |
| BH-5.6 | Initial render shows a **"Translating…"** loading state; results stream in as they arrive. |
| BH-5.7 | The source text is shown at the top in a muted box. |

### 3.6 Result interactions

| ID | Behavior |
| --- | --- |
| BH-6.1 | Each result tab has a **Copy** button; clicking puts the result text on the system clipboard and briefly changes the button label to "Copied" (1.5 s). |
| BH-6.2 | A **Retry** button (in the popup header) re-runs all enabled services with the same source text. |
| BH-6.3 | A **Close** button (×) hides the popup. |
| BH-6.4 | Pressing **Escape** hides the popup. |
| BH-5.8 | Clicking outside the popup hides it after a **500 ms** grace period. The grace period is reset if focus returns to the popup during the countdown. |
| BH-6.5 | The popup **does not** steal focus from the source application (focus = false on creation), so the user can keep typing. |

### 3.7 Settings window

| ID | Behavior |
| --- | --- |
| BH-7.1 | The settings window opens from the tray menu ("Open Settings") or by clicking the tray icon (left-click). |
| BH-7.2 | The settings window has **5 sections** in a left sidebar: General, Services, Hotkey, Appearance, About. |
| BH-7.3 | The window is **resizable** (min 720 × 480 px, default 880 × 640 px) and remembers its size across sessions. |
| BH-7.4 | Changes in the settings window are **auto-saved on blur** (per field) and **not** applied to a running translation until the next hotkey press. |
| BH-7.5 | The settings window can be **closed** without quitting the app; the app keeps running in the tray. |

### 3.8 Service management (Settings → Services)

| ID | Behavior |
| --- | --- |
| BH-8.1 | Each service is shown as a row with: name, enabled toggle, priority number, status indicator (configured / missing key / error). |
| BH-8.2 | Toggling **enabled** persists immediately to `config.json`. |
| BH-8.3 | Changing **priority** (integer ≥ 0) persists immediately; the new order applies to the next translation. |
| BH-8.4 | For **Youdao**: the row expands to show `App Key` and `App Secret` fields (both required, stored in Keychain). |
| BH-8.5 | For **OpenAI Compatible**: the row expands to show `Base URL`, `Model`, `API Key`, and a `Presets` dropdown. Selecting a preset auto-fills `Base URL` and `Model`. |
| BH-8.6 | For all other services: the row expands to show a single `API Key` field. |
| BH-8.7 | Saving a key shows a **green toast** "Saved to OS Keychain" for 2 s. |
| BH-8.8 | A **Remove** button on each row deletes the keychain entry; status indicator turns red. |

### 3.9 Language settings (Settings → General)

| ID | Behavior |
| --- | --- |
| BH-9.1 | **Target language** defaults to `en`; options include `en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `fr`, `de`, `es`, `ru`, `pt`, `it`, `ar`. |
| BH-9.2 | **Default source language** defaults to `auto`; can be overridden to a specific BCP-47 code. |
| BH-9.3 | Both fields are BCP-47 strings; validation rejects malformed codes with an inline error. |

### 3.10 Hotkey customization (Settings → Hotkey)

| ID | Behavior |
| --- | --- |
| BH-10.1 | The hotkey is displayed in a single text input. The user types the new shortcut (e.g. `CmdOrCtrl+Shift+D`); syntax follows the [Tauri global-shortcut format](https://docs.rs/tauri-plugin-global-shortcut/latest/tauri_plugin_global_shortcut/). |
| BH-10.2 | Invalid syntax is rejected with an inline error. |
| BH-10.3 | On a successful change, the old shortcut is unregistered and the new one is registered immediately. |

### 3.11 Appearance (Settings → Appearance)

| ID | Behavior |
| --- | --- |
| BH-11.1 | **Theme** is selectable from `System` (default), `Light`, `Dark`. |
| BH-11.2 | When set to `System`, the app follows the OS appearance and updates live when the OS appearance changes. |

### 3.12 About (Settings → About)

| ID | Behavior |
| --- | --- |
| BH-12.1 | Shows the app version, build commit, build date, and a link to the GitHub repo. |

### 3.13 Localization

| ID | Behavior |
| --- | --- |
| BH-13.1 | The UI is rendered in **English** by default. |
| BH-13.2 | The UI switches to **Simplified Chinese** when the OS locale starts with `zh-Hans` or `zh-CN`. |
| BH-13.3 | No runtime language switcher in v0.1.0 (locale detection only). |
| BH-13.4 | All user-facing strings live in `locales/{en,zh-Hans}.ftl`; the Rust backend has no user-facing strings. |

### 3.14 System integration

| ID | Behavior |
| --- | --- |
| BH-14.1 | The tray icon shows on first launch; left-click opens the settings window. |
| BH-14.2 | The tray menu has two items: **Open Settings** and **Quit**. |
| BH-14.3 | On macOS, the tray icon is rendered as a **template image** (monochrome) and respects the menu bar's light/dark appearance. |
| BH-14.4 | On Windows and Linux, the tray icon is a **color** image. |
| BH-14.5 | Quitting via the tray menu exits the process. Closing the settings or popup windows does **not** quit. |
| BH-14.6 | The app starts at login: **off** by default in v0.1.0 (toggle is a v0.2.0 feature). |

---

## 4. Non-Functional Requirements

### 4.1 Performance

| Metric | Target | Measurement |
| --- | --- | --- |
| Cold start to tray visible | < 1.5 s on Apple M1, < 2.5 s on Windows 10 i5 | stopwatch on first launch |
| Hotkey press to popup visible | < 150 ms (excluding network) | logging timestamps |
| Service response time (UI latency) | < 8 s, with progressive display | first tab populates as it arrives |
| Settings window open | < 300 ms | tauri::Window::show event timing |
| Memory (idle) | < 50 MB resident | activity monitor / task manager |
| Binary size | < 8 MB per platform (uncompressed) | `du -h target/release/bundle/...` |

### 4.2 Privacy

- The app makes **no outbound network calls** other than to the configured translation services.
- Telemetry: **none**.
- Crash reports: **none** in v0.1.0.
- API keys are **never** written to `config.json` or any other file; they live only in OS Keychain.
- Source text is sent **only** to services the user has explicitly enabled.

### 4.3 Reliability

- The app must **not crash** on:
  - Loss of network (popup shows per-service error).
  - Malformed API response (logged at `warn`, popup shows generic error).
  - Missing or corrupted `config.json` (re-created with defaults).
  - OS Keychain being unavailable (graceful degradation: services that need a key are disabled with a clear UI message).
- The app must **not** consume a CPU when idle (no polling loops).

### 4.4 Accessibility

- All interactive UI elements have a `tabindex` and a `role`.
- The popup is keyboard-navigable (Tab / Shift-Tab between service tabs and buttons; Enter to activate; Escape to close).
- Color contrast: text vs background ≥ 4.5:1 in both light and dark themes.
- Localized strings include accessible labels for screen readers.

### 4.5 Cross-platform parity

- All behaviors in §3 must work on **all three target platforms** by v0.1.0.
- Platform-specific differences (e.g. permission model) are limited to §3.3 and documented in [user-guide.md](user-guide.md).

---

## 5. Error Handling Contract

| Source | User-facing behavior |
| --- | --- |
| Permission denied | Popup shows "translator needs the Accessibility permission" + "Open Settings" button |
| Empty selection | Popup shows "No text is selected" |
| No services enabled | Popup shows "No services enabled. Open Settings to enable one." |
| One service fails | That service's tab shows the error; other tabs work |
| All services fail | Popup shows the list of errors with a "Retry" button |
| Network timeout (> 8 s) | Tab shows "Translation timed out" |
| Service returns 401/403 | Tab shows "API key invalid" |
| Service returns 429 | Tab shows "Rate limited" |
| Unknown error | Tab shows the error message verbatim + a "Copy" button to copy the error |

All errors are logged with `tracing` at `warn` or `error` level with the service id and a request id.

---

## 6. Acceptance Criteria (Definition of Done for v0.1.0)

A build of `v0.1.0` is releasable when **all** of the following hold:

1. ✅ `cargo check --workspace --all-targets` passes with zero warnings.
2. ✅ `cargo test --workspace` passes (unit tests for the 5 service clients with `wiremock`, plus the platform utilities).
3. ✅ `cargo clippy --workspace --all-targets -- -D warnings` passes.
4. ✅ `npm run typecheck` (i.e. `tsc --noEmit`) passes with zero errors.
5. ✅ `npm run lint` (eslint) passes.
6. ✅ `cargo tauri build` produces signed installable bundles on **all three target platforms** (signing may be ad-hoc).
7. ✅ All behaviors in §3 are covered by at least one **manual QA** check on each platform, recorded in a `qa/<date>-<platform>.md` file.
8. ✅ All services have at least one **integration test** that hits a sandboxed endpoint (or a recorded response) and asserts the request shape + response parsing.
9. ✅ `cargo audit` reports no high-severity advisories.
10. ✅ The user guide (`docs/user-guide.md`) is up to date.
11. ✅ The dev guide (`docs/dev-guide.md`) is up to date.
12. ✅ `CHANGELOG.md` has a `## [0.1.0]` entry.

---

## 7. Open Questions for Product Review

These are explicitly deferred to the reviewer; they do not block v0.1.0 work but should be answered before v0.2.0.

- **Q1**: Do we add a "swap source/target" button in the popup, or is the target language only configurable in Settings? (Current spec: Settings only.)
- **Q2**: Do we show a "translating from N to M" subtitle in the popup, or is that too noisy? (Current spec: no.)
- **Q3**: Should we cache the last 100 translations locally (in `localStorage` of the popup window) to allow viewing history? (Current spec: no.)
- **Q4**: For Youdao, do we support the `dict` endpoint (word lookups with definitions) or only the `translate` endpoint? (Current spec: `translate` only.)
- **Q5**: For the OpenAI-compatible service, do we support **streaming** responses (SSE) in v0.1.0 or fall back to blocking? (Current spec: blocking; streaming is a v0.2.0 candidate.)

These are tracked in `PLAN.md` §10.
