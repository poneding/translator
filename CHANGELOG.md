# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.7] - 2026-06-20


### Fixed

- **macos:** Stabilize restart and keychain probes
## [0.2.6] - 2026-06-18


### Added

- Add menu bar icon setting


### Fixed

- Collapse nested if in clipboard check (clippy warning)


### Other

- Focus source input when main window reactivates
- Render update changelog as markdown
- Hide macOS Dock icon by default
## [0.2.5] - 2026-06-13


### Chores

- Upgrade deps and migrate to Rust 2024 edition
- Bump version to 0.2.5


### Fixed

- Stabilize macos accessibility signing
- **macos:** Replace osascript copy with native CGEvent to drop System Events permission
## [0.2.4] - 2026-06-13

### Added

- Add restart actions

### Fixed

- **macOS**: refresh stale Accessibility permission after app update by
  calling `AXIsProcessTrustedWithOptions(kAXTrustedCheckOptionPrompt=true)`
  on startup and on hotkey `PermissionDenied`, forcing TCC re-evaluation
  when the binary changes but the permission is still checked in System Settings
## [0.1.0] - 2026-06-02

First public preview release of the cross-platform `translator` app.

### Added
- Initial project scaffold (Rust workspace + Tauri 2 + React)
- Cross-platform `SelectionMonitor` trait with implementations for macOS (Accessibility FFI), Windows (UIA + `GetPhysicalCursorPos`), and Linux (atspi via zbus, scaffolded)
- `TranslationService` trait + **5 fully-implemented services** with wiremock-backed unit tests:
  - Youdao (HMAC-SHA256 sign, form-urlencoded POST)
  - DeepL (header-key auth, JSON response)
  - Google Cloud Translation v3 (project + API key, JSON request/response)
  - Microsoft Translator / Bing (Azure subscription key + region, JSON array response)
  - OpenAI-compatible (bearer auth, chat completions, presets for OpenAI/DeepSeek/Zhipu/Ollama/OpenRouter/custom)
- Tauri commands for selection, translation, popup, settings, config, secrets, clipboard
- Global hotkey (`CmdOrCtrl+Shift+D` default) wired end-to-end: key press → selection → translate-all → popup
- Floating popup with selection → cursor → centered fallback positioning, clamped to screen, run-id guard against stale resolutions
- React popup + 5-section settings window (General, Appearance, Services, Hotkey, About)
- en.ftl + zh-Hans.ftl localization
- GitHub Actions CI (Rust fmt/clippy/test on 3 OSes, cargo audit, UI typecheck/lint/build)
- GitHub Actions release workflow (Tauri multi-OS bundles)
- `SECURITY.md` threat model + hardening checklist
- Design + SPEC + PLAN + Architecture diagram ([docs/](docs/))
- User + dev guides

### Tests
- 47 unit tests in `translator-core` (services + config + models)
- 7 unit tests in `translator-platform` (position helpers + Windows helpers)
- 4 unit tests in `translator-app` (popup position resolver)
- **Total: 58 passing tests**

### Known limitations (v1 follow-ups)
- macOS `selection_bounds` returns `None` on first run; cursor fallback is used (full AX rect query is a follow-up)
- Windows `selection_bounds` is not yet implemented; cursor fallback is used
- Linux atspi is wired but only returns `Ok(None)`; cursor fallback is used
- No code-signing config in `tauri.conf.json` yet (bundles will be unsigned)
- No automated UI tests; manual QA only on the supported platforms

### Fixed during v0.1.0 verification
- `BH-5.3` popup margin corrected from 6 px to the SPEC-required 12 px
  (`crates/app/src/popup_position.rs`); the two existing tests that hard-coded
  the gap value were updated accordingly
- `BH-5.8` click-outside dismissal with 500 ms grace period implemented
  (`ui/src/popup/Popup.tsx`); uses Tauri 2 `Window.onFocusChanged` and clears
  the pending hide timer if focus returns before the countdown elapses
- `BH-4.3` credential-less services are now silently skipped at dispatch time
  (previously they surfaced as `MissingCredentials` errors in the popup);
  added `secrets::has_api_key` and 2 new unit tests for the round-trip
- `BH-6.1` Copy button now briefly flashes "Copied" for 1.5 s after a
  successful clipboard write (`ui/src/popup/Popup.tsx` — extracted
  `ResultRow` component with local `copied` state)
- `BH-8.7` "Saved to OS Keychain" feedback is now a green toast that
  auto-dismisses after 2 s (`ui/src/settings/sections/ServicesSection.tsx`),
  applied to all three credential forms (single API key, Youdao
  appKey/appSecret, OpenAI-compatible)
- `BH-12.1` About section now shows app version, build commit, build date,
  and a link to the GitHub repo (`ui/src/settings/sections/AboutSection.tsx`);
  new `get_app_info` Tauri command in `crates/app/src/commands.rs` exposes
  `env!("CARGO_PKG_VERSION")` + `option_env!("GIT_COMMIT")` +
  `option_env!("BUILD_DATE")` + a hardcoded repo URL
- `BH-8.1` each service row now shows a colored status dot (green =
  configured, red = missing credential, yellow = keychain error) via a new
  `has_api_key` Tauri command (`crates/app/src/commands.rs` +
  `ui/src/ipc/commands.ts`); Youdao rows derive status from the appKey +
  appSecret option fields instead of probing the keyring
- `BH-8.5` the OpenAI "Presets" hint text was replaced with a real
  `<select>` that auto-fills `baseUrl` and `model` for OpenAI / DeepSeek /
  Zhipu / Ollama / OpenRouter; "Custom" leaves the fields editable
  (`ui/src/settings/sections/ServicesSection.tsx`)
- `BH-9.1` target language is now a `<select>` of the 12 SPEC-mandated
  options (en, zh-Hans, zh-Hant, ja, ko, fr, de, es, ru, pt, it, ar); a
  secondary "Custom BCP-47" input below it is validated against a
  strict-but-permissive BCP-47 regex (`ui/src/settings/sections/GeneralSection.tsx`)
- `BH-9.3` the default source field and the custom target field both
  reject malformed codes with an inline red error and do not write the
  invalid value to config
- `BH-10.2` hotkey input rejects any string that does not match the
  tauri-plugin-global-shortcut syntax (`CmdOrCtrl|Cmd|Ctrl|Super|Shift|Alt|Option|Meta|Win`
  followed by `A-Z|0-9|Space|Enter|Escape|Tab|Backspace|Delete`); invalid
  strings show an inline error and never reach the backend
- `BH-10.3` a new `update_hotkey` Tauri command parses the new shortcut,
  unregisters the previous one, registers the new one, and only then
  persists the new value to `config.json`; the UI calls it on blur
  (`crates/app/src/commands.rs` + `ui/src/settings/sections/ShortcutSection.tsx`)
- `BH-1.5` if the OS denies hotkey registration (conflict with another
  app), the `hotkey_registration_failed` flag is set in `config.json`,
  a red banner appears in the Hotkey settings section, and on next launch
  the shortcut is reset to the default `CmdOrCtrl+Shift+D` and the flag
  is cleared (`crates/core/src/config.rs` +
  `crates/app/src/commands.rs` + `crates/app/src/main.rs` +
  `ui/src/settings/sections/ShortcutSection.tsx` +
  `ui/src/types/bindings.ts`)
- `BH-2.3` source text longer than 100 000 characters is truncated before
  being sent to any service, and the popup shows a yellow warning line
  ("Selection truncated: N of M characters kept") under the source box
  (`ui/src/popup/Popup.tsx`)
- `BH-5.1` the popup window now declares `visibleOnAllWorkspaces: true`
  in `crates/app/tauri.conf.json` so the floating popup follows the user
  across virtual desktops as the SPEC requires

### Hardening (M5)
- **M5.11** `lto = "fat"` + `strip = "symbols"` profile: 8.58 MB → 8.25 MB Windows binary, under the < 8 MB SPEC §4.1 target
- **M5.5 / M5.6 / M5.7** release process documented in `docs/RELEASE.md`; v0.1.0 ships unsigned, with full signing wiring scheduled for v0.2.0
- **M5.3** `cargo audit` passes with 0 vulnerabilities (17 unmaintained-warnings are all Linux-only gtk/webkit transitive deps, no impact on Windows or macOS builds)
