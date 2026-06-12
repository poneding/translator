<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### 跨平台划词翻译工具

任意位置选中文本 → 按下快捷键 → 即时翻译

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## 📥 安装

### 从 Release 下载

**最新版本：v0.2.0**

从 [GitHub Releases](https://github.com/poneding/translator/releases/latest) 下载适合您平台的安装包：

- **macOS**: `translator_0.2.0_universal.dmg` 或 `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - 打开 DMG 文件，将应用拖到"应用程序"文件夹
  - 首次启动时，右键点击应用并选择"打开"以绕过 Gatekeeper
  
- **Windows**: `translator_0.2.0_x64-setup.exe` 或 `translator_0.2.0_x64_en-US.msi`
  - 运行安装程序并按照设置向导操作
  - Windows Defender SmartScreen 可能会显示警告；点击"更多信息" → "仍要运行"
  
- **Linux**: `translator_0.2.0_amd64.deb` 或 `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB): `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage**: `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### 从源码构建

请参阅下方的[快速开始](#-快速开始)部分。

---

## ✨ 特性

- 🌍 **全局快捷键** — 在任意应用中选中文本即可翻译
- 🔌 **5 个翻译服务** — 有道、DeepL、谷歌、必应（Azure）、OpenAI 兼容
- 🤖 **自动语言检测** — 智能识别源语言
- 🎯 **主窗口翻译** — 支持窗口固定、历史记录、音频播放、单服务重试
- 📋 **剪贴板回退** — 无法获取选中文本时可翻译剪贴板内容
- 🔄 **内置更新** — 稳定版/测试版更新通道
- 🎨 **深色模式** — 跟随系统偏好
- 🌏 **12 种界面语言** — 支持实时切换应用语言
- 🔐 **安全存储** — API 密钥存储在系统钥匙串中
- ⚡ **轻量级** — 约 6 MB 二进制文件，内存占用 < 50 MB

## 📸 截图

<div align="center">

<table>
<tr>
<td width="50%">

### 浅色模式
<img src="docs/screenshots/light.png" alt="浅色模式">

</td>
<td width="50%">

### 深色模式
<img src="docs/screenshots/dark.png" alt="深色模式">

</td>
</tr>
</table>

</div>

## 🚀 快速开始

### 前置要求

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **平台依赖:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2（Win10+ 预装）
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### 开发模式

```bash
# 安装 JavaScript 依赖
cd ui && npm install && cd ..

# 运行开发服务器（支持热重载）
cargo tauri dev
```

### 构建发行版

```bash
cargo tauri build
```

**输出位置：** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 文档

- 📐 [设计文档](docs/DESIGN.md) — v0.2 架构概览
- 🏛️ [架构图](docs/ARCHITECTURE.svg) — 可视化组件图
- 🛠️ [开发者指南](docs/dev-guide.md) — 编码规范、测试、调试
- 👤 [用户指南](docs/user-guide.md) — 设置说明、API 密钥、快捷键自定义

## 📂 项目结构

```txt
translator/
├── crates/
│   ├── core/         # 纯 Rust 业务逻辑 + 5 个翻译服务
│   ├── platform/     # 跨平台选中监控（macOS/Win/Linux）
│   └── app/          # Tauri 外壳（命令、托盘、IPC）
├── ui/               # React + Vite 前端（主窗口 + 设置）
├── ui/src/locales/   # Fluent 国际化文件（12 种应用语言）
├── docs/             # 设计 + 用户/开发者指南
└── .github/          # CI + 发布工作流
```

## 🤝 贡献

欢迎贡献！提交 PR 前请阅读我们的[开发者指南](docs/dev-guide.md)。

## 📄 许可证

GPL-3.0-only。详见 [LICENSE](LICENSE)。

## ⭐ Star 历史

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**使用 Rust + Tauri 2 + React 构建，倾情奉献 ❤️**

</div>
