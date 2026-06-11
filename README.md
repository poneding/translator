# translator

> Cross-platform select-and-translate tool. Pick text anywhere -> press a hotkey -> translate in the main window.

Built with **Rust + Tauri 2 + React**. Supports macOS, Windows, and Linux.

## Features

- Global hotkey to translate selected text from any app
- 5 translation services: **Youdao (有道)**, **DeepL**, **Google**, **Bing (Azure)**, **OpenAI-compatible** (OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter, …)
- Auto language detection
- Main-window translation flow with pin, history, source/result audio, and per-service retry
- Clipboard fallback on hotkey when enabled
- Built-in update checks with stable/beta eligibility
- System tray / menubar for quick access
- Dark mode follows system
- 12 UI locales with live app-language switching
- Secure API key storage in OS Keychain
- ~6 MB binary, < 50 MB memory

## Quick start (development)

### Prerequisites

- Rust 1.81+ (`rustup install stable`)
- Node.js 20+
- Platform deps:
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (preinstalled on Win10+)
  - **Linux**: `libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`

### Install + run

```bash
# install JS deps
cd ui && npm install && cd ..

# run dev (Tauri + Vite hot reload)
cargo tauri dev
```

### Build release

```bash
cargo tauri build
```

Outputs to `target/release/bundle/`:
- macOS: `.dmg` + `.app`
- Windows: `.msi` + `.exe`
- Linux: `.AppImage` + `.deb`

## Documentation

- 📐 **[Design document](docs/DESIGN.md)** — current v0.2 architecture
- 🏛️ **[Architecture diagram](docs/ARCHITECTURE.svg)** — visual overview
- 🛠️ **[Dev guide](docs/dev-guide.md)** — coding conventions, testing, debugging
- 👤 **[User guide](docs/user-guide.md)** — setup, API keys, hotkey customization

## Project layout

```
translator/
├── crates/
│   ├── core/         # Pure-Rust business logic + 5 translation services
│   ├── platform/     # Cross-platform selection monitor (macOS/Win/Linux)
│   └── app/          # Tauri shell (commands, tray, IPC)
├── ui/               # React + Vite frontend (main window + integrated settings)
├── ui/src/locales/   # Fluent i18n files (12 app languages)
├── docs/             # Design + user/dev guides
└── .github/          # CI + release workflows
```

## License

GPL-3.0-only. See [LICENSE](LICENSE).

## Status

v0.2.0 release candidate work is on `dev`.
