<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Cross-platform select-and-translate tool

Pick text anywhere → press a hotkey → translate instantly

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## 📥 Installation

### Download from Releases

**Latest version: v0.2.0**

Download the installer for your platform from [GitHub Releases](https://github.com/poneding/translator/releases/latest):

- **macOS**: `translator_0.2.0_universal.dmg` or `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - Open the DMG file and drag the app to Applications folder
  - On first launch, right-click the app and select "Open" to bypass Gatekeeper
  
- **Windows**: `translator_0.2.0_x64-setup.exe` or `translator_0.2.0_x64_en-US.msi`
  - Run the installer and follow the setup wizard
  - Windows Defender SmartScreen may show a warning; click "More info" → "Run anyway"
  
- **Linux**: `translator_0.2.0_amd64.deb` or `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB): `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage**: `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### Build from Source

See [Quick Start](#-quick-start) section below.

---

## ✨ Features

- 🌍 **Global hotkey** — Translate selected text from any app instantly
- 🔌 **5 translation services** — Youdao (有道), DeepL, Google, Bing (Azure), OpenAI-compatible
- 🤖 **Auto language detection** — Smart source language recognition
- 🎯 **Main-window translation** — Pin, history, audio playback, per-service retry
- 📋 **Clipboard fallback** — Translate clipboard when selection unavailable
- 🔄 **Built-in updates** — Stable/beta release channels
- 🎨 **Dark mode** — Follows system preferences
- 🌏 **12 UI languages** — Live app-language switching
- 🔐 **Secure storage** — API keys stored in OS Keychain
- ⚡ **Lightweight** — ~6 MB binary, <50 MB memory

## 📸 Screenshots

<div align="center">

<table>
<tr>
<td width="50%">

### Light Mode
<img src="docs/screenshots/light.png" alt="Light Mode">

</td>
<td width="50%">

### Dark Mode
<img src="docs/screenshots/dark.png" alt="Dark Mode">

</td>
</tr>
</table>

</div>

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Platform dependencies:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (preinstalled on Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Development

```bash
# Install JavaScript dependencies
cd ui && npm install && cd ..

# Run dev server (hot reload enabled)
cargo tauri dev
```

### Build Release

```bash
cargo tauri build
```

**Output location:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 Documentation

- 📐 [Design Document](docs/DESIGN.md) — Architecture overview for v0.2
- 🏛️ [Architecture Diagram](docs/ARCHITECTURE.svg) — Visual component map
- 🛠️ [Developer Guide](docs/dev-guide.md) — Coding conventions, testing, debugging
- 👤 [User Guide](docs/user-guide.md) — Setup instructions, API keys, hotkey customization

## 📂 Project Structure

```txt
translator/
├── crates/
│   ├── core/         # Pure-Rust business logic + 5 translation services
│   ├── platform/     # Cross-platform selection monitor (macOS/Win/Linux)
│   └── app/          # Tauri shell (commands, tray, IPC)
├── ui/               # React + Vite frontend (main window + settings)
├── ui/src/locales/   # Fluent i18n files (12 app languages)
├── docs/             # Design + user/dev guides
└── .github/          # CI + release workflows
```

## 🤝 Contributing

Contributions are welcome! Please read our [Developer Guide](docs/dev-guide.md) before submitting PRs.

## 📄 License

GPL-3.0-only. See [LICENSE](LICENSE) for details.

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Built with ❤️ using Rust + Tauri 2 + React**

</div>
