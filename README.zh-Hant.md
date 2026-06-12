<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### 跨平台劃詞翻譯工具

任意位置選中文字 → 按下快速鍵 → 即時翻譯

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## 📥 安裝

### 從 Release 下載

**最新版本：v0.2.0**

從 [GitHub Releases](https://github.com/poneding/translator/releases/latest) 下載適合您平台的安裝套件：

- **macOS**: `translator_0.2.0_universal.dmg` 或 `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - 開啟 DMG 檔案，將應用程式拖曳到「應用程式」資料夾
  - 首次啟動時，右鍵點選應用程式並選擇「開啟」以繞過 Gatekeeper
  
- **Windows**: `translator_0.2.0_x64-setup.exe` 或 `translator_0.2.0_x64_en-US.msi`
  - 執行安裝程式並依照設定精靈操作
  - Windows Defender SmartScreen 可能會顯示警告；點選「更多資訊」 → 「仍要執行」
  
- **Linux**: `translator_0.2.0_amd64.deb` 或 `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB): `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage**: `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### 從原始碼建置

請參閱下方的[快速開始](#-快速開始)部分。

---

## ✨ 特色

- 🌍 **全域快速鍵** — 在任意應用程式中選取文字即可翻譯
- 🔌 **5 個翻譯服務** — 有道、DeepL、Google、Bing（Azure）、OpenAI 相容
- 🤖 **自動語言偵測** — 智慧識別來源語言
- 🎯 **主視窗翻譯** — 支援視窗固定、歷史記錄、音訊播放、單服務重試
- 📋 **剪貼簿回退** — 無法取得選取文字時可翻譯剪貼簿內容
- 🔄 **內建更新** — 穩定版/測試版更新通道
- 🎨 **深色模式** — 跟隨系統偏好設定
- 🌏 **12 種介面語言** — 支援即時切換應用程式語言
- 🔐 **安全儲存** — API 金鑰儲存在系統鑰匙圈中
- ⚡ **輕量級** — 約 6 MB 二進位檔案，記憶體佔用 < 50 MB

## 📸 螢幕截圖

<div align="center">

<table>
<tr>
<td width="50%">

### 淺色模式
<img src="docs/screenshots/light.png" alt="淺色模式">

</td>
<td width="50%">

### 深色模式
<img src="docs/screenshots/dark.png" alt="深色模式">

</td>
</tr>
</table>

</div>

## 🚀 快速開始

### 前置需求

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **平台相依性:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2（Win10+ 預裝）
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### 開發模式

```bash
# 安裝 JavaScript 相依性套件
cd ui && npm install && cd ..

# 執行開發伺服器（支援熱重載）
cargo tauri dev
```

### 建置正式版

```bash
cargo tauri build
```

**輸出位置：** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 文件

- 📐 [設計文件](docs/DESIGN.md) — v0.2 架構概覽
- 🏛️ [架構圖](docs/ARCHITECTURE.svg) — 視覺化元件圖
- 🛠️ [開發者指南](docs/dev-guide.md) — 編碼規範、測試、除錯
- 👤 [使用者指南](docs/user-guide.md) — 設定說明、API 金鑰、快速鍵自訂

## 📂 專案結構

```txt
translator/
├── crates/
│   ├── core/         # 純 Rust 業務邏輯 + 5 個翻譯服務
│   ├── platform/     # 跨平台選取監控（macOS/Win/Linux）
│   └── app/          # Tauri 外殼（命令、托盤、IPC）
├── ui/               # React + Vite 前端（主視窗 + 設定）
├── ui/src/locales/   # Fluent 國際化檔案（12 種應用程式語言）
├── docs/             # 設計 + 使用者/開發者指南
└── .github/          # CI + 發佈工作流程
```

## 🤝 貢獻

歡迎貢獻！提交 PR 前請閱讀我們的[開發者指南](docs/dev-guide.md)。

## 📄 授權

GPL-3.0-only。詳見 [LICENSE](LICENSE)。

## ⭐ Star 歷史

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**使用 Rust + Tauri 2 + React 建置，傾情奉獻 ❤️**

</div>
