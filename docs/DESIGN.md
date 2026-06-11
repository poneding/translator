# Translator Design

> Version: v0.2.0 · Status: current architecture
>
> The behavioral source of truth is
> [SPEC-v0.2.0.md](SPEC-v0.2.0.md). Historical v0.1 behavior is documented in
> the versioned v0.1 spec and plan.

## Product Shape

`translator` is a tray-resident, main-window-first translation app.

1. The global hotkey reads selected text before the app takes focus.
2. If selection is empty and clipboard fallback is enabled, the clipboard is
   read once for that hotkey press.
3. The main window opens, fills the source editor, and dispatches translation
   requests to enabled services.
4. The user can pin the window, change target language, retry one service,
   copy output, play source or translated audio, and open integrated settings.

The app does not continuously monitor clipboard content and does not create a
secondary quick-translation window in the v0.2 product flow.

## Runtime Architecture

| Layer | Responsibility |
| --- | --- |
| `crates/app` | Tauri shell, global hotkey, tray, window positioning, IPC, updater commands |
| `crates/core` | Config normalization, language direction, translation service registry, audio URL helpers |
| `crates/platform` | Cross-platform selected-text acquisition and permission helpers |
| `ui/src` | React main translator, integrated settings, i18n, service metadata |
| `.github/workflows` | CI and tag-driven release builds with updater artifacts |

## Windows

The Tauri app defines a single product window labeled `main`.

- Custom titlebar with pin on the left, centered logo + app name, history and
  settings actions on the right, and platform close/minimize controls.
- Default size is compact; max width is bounded and maximize is disabled.
- `window.display_position` decides where the window appears when opened:
  `right` means the screen's top-right work area, `center` centers the window,
  and `mouse` opens near the cursor.
- Closing or pressing Escape hides the window. The process remains available in
  the tray until Quit is selected.

## Hotkey Flow

```text
global shortcut released
  -> commands::on_hotkey()
  -> platform SelectionMonitor reads selected text
  -> optional one-shot clipboard fallback
  -> prepare/show/focus main window using saved display position and pin state
  -> emit translator://hotkey-source
  -> React fills source and starts translation when text exists
```

Selection is read before the main window is shown so the source app keeps focus
long enough for platform text acquisition.

## Translation Dispatch

The frontend calls:

- `translate_text` for all enabled services.
- `translate_service` for a single card refresh.
- `get_text_audio_url` for source-editor speech when provider dictionary audio
  is unavailable.

`crates/core::language_direction` resolves automatic source hints and target
language based on `general.preferred_languages`. If the source is detected as
English, the default counterpart is Simplified Chinese; otherwise the default
counterpart is English unless user preferences say otherwise.

Service outcomes are emitted incrementally with request ids so stale all-service
or per-service responses cannot overwrite newer UI state.

## Source Editor And Results

- The source editor reserves two rows and grows to a four-row maximum.
- Enter translates; Shift+Enter inserts a newline.
- The source editor toolbar contains source audio, copy, clear, and translate.
- The row below the editor contains source short code -> target short-code
  dropdown on the left and enabled service count + service logos on the right.
- Result card headers are one compact line: small service logo + service name.
- Result cards do not show detected source language.
- A per-service refresh button appears on hover or keyboard focus.

## Audio Semantics

Source audio and result audio are intentionally different:

- Source editor audio speaks the source text using the detected/source language
  when available.
- Result audio speaks the translated text using the target language.
- Dictionary phonetic audio remains source-word pronunciation and is displayed
  near source phonetics/dictionary details.

The UI uses `Volume1`/`Volume2` animation while audio is playing. Starting a new
audio item stops the previous one.

## Configuration

Config is stored as JSON under the platform config directory and normalized on
read. API keys are stored in the OS Keychain.

Current config groups:

- `general`: preferred languages, theme, app language, clipboard fallback,
  auto-copy, launch at startup, proxy.
- `window`: always-on-top and default open position.
- `updates`: startup check and beta eligibility.
- `services`: enabled state, priority, and non-secret provider options.
- `history`: recent successful translations.

New v0.2 fields are defaulted when older configs are loaded without dropping
history or service configuration.

## Updates

The app uses the official Tauri updater plugin.

- Startup checks run asynchronously after app setup and are quiet on failure.
- Manual checks report inline status in Settings -> Update.
- Stable mode uses the GitHub latest release manifest.
- Beta mode uses the configured beta manifest endpoint and allows prerelease
  versions newer than the current app.
- Download/install requires explicit user action.
- Release builds create updater artifacts and sign them with the private key
  stored in GitHub Actions secrets.

## Localization

The UI uses Fluent files in `ui/src/locales`.

Required locales:

`en`, `zh-Hans`, `zh-Hant`, `ja`, `ko`, `fr`, `de`, `es`, `ru`, `pt`, `it`,
`ar`.

`npm run locales:check` compares every locale against English and must pass
before release.

## Release Gates

Local release gates:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
cd ui && npm run typecheck && npm run lint && npm run build
actionlint .github/workflows/*.yml
```

The tag-driven release workflow must build macOS universal, Linux x64, Linux
ARM64, Windows x64, and Windows ARM64 artifacts, plus signed updater assets.
