<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### クロスプラットフォーム選択翻訳ツール

テキストを選択 → ホットキーを押す → 即座に翻訳

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## ✨ 機能

- 🌍 **グローバルホットキー** — 任意のアプリで選択したテキストを即座に翻訳
- 🔌 **5つの翻訳サービス** — Youdao（有道）、DeepL、Google、Bing（Azure）、OpenAI互換
- 🤖 **自動言語検出** — スマートなソース言語認識
- 🎯 **メインウィンドウ翻訳** — ピン留め、履歴、音声再生、サービス別再試行に対応
- 📋 **クリップボードフォールバック** — 選択が利用できない場合はクリップボードを翻訳
- 🔄 **組み込みアップデート** — 安定版/ベータ版リリースチャンネル
- 🎨 **ダークモード** — システム設定に従う
- 🌏 **12の UI言語** — リアルタイムアプリ言語切り替え
- 🔐 **安全なストレージ** — APIキーはOSキーチェーンに保存
- ⚡ **軽量** — 約6MBバイナリ、メモリ使用量50MB未満

## 📸 スクリーンショット

<div align="center">

<table>
<tr>
<td width="50%">

### ライトモード
<img src="docs/screenshots/light.png" alt="ライトモード">

</td>
<td width="50%">

### ダークモード
<img src="docs/screenshots/dark.png" alt="ダークモード">

</td>
</tr>
</table>

</div>

## 📥 インストール

[GitHub Releases](https://github.com/poneding/translator/releases/latest) からお使いのプラットフォーム用インストーラーをダウンロード：

| プラットフォーム | アーキテクチャ | 推奨ダウンロード |
| --- | --- | --- |
| macOS | Intel / Apple Silicon | `.dmg` |
| Windows | x86_64 | `.msi` または `.exe` |
| Linux | x86_64 / arm64 | `.AppImage`、`.deb` または `.rpm` |

ダウンロード後、通常通りインストール：macOS は `.dmg` を開いてアプリを Applications フォルダーにドラッグ（初回起動時は右クリック → 「開く」で Gatekeeper をバイパス）、Windows はインストーラーを実行（SmartScreen が警告する場合は「詳細情報」→「実行」）、Linux は `.AppImage` を直接実行するかパッケージマネージャーで `.deb` / `.rpm` をインストール。

ソースからビルドする場合は、下記の[クイックスタート](#-クイックスタート)セクションを参照してください。

## 🚀 クイックスタート

### 前提条件

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **プラットフォーム依存関係:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2（Win10+にプリインストール）
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### 開発

```bash
# JavaScript依存関係をインストール
cd ui && npm install && cd ..

# 開発サーバーを実行（ホットリロード有効）
cargo tauri dev
```

### リリースビルド

```bash
cargo tauri build
```

**出力場所:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 ドキュメント

- 📐 [設計ドキュメント](docs/DESIGN.md) — v0.2アーキテクチャ概要
- 🏛️ [アーキテクチャ図](docs/ARCHITECTURE.svg) — ビジュアルコンポーネントマップ
- 🛠️ [開発者ガイド](docs/dev-guide.md) — コーディング規約、テスト、デバッグ
- 👤 [ユーザーガイド](docs/user-guide.md) — セットアップ手順、APIキー、ホットキーカスタマイズ

## 📂 プロジェクト構造

```txt
translator/
├── crates/
│   ├── core/         # 純粋なRustビジネスロジック + 5つの翻訳サービス
│   ├── platform/     # クロスプラットフォーム選択モニター（macOS/Win/Linux）
│   └── app/          # Tauriシェル（コマンド、トレイ、IPC）
├── ui/               # React + Viteフロントエンド（メインウィンドウ + 設定）
├── ui/src/locales/   # Fluent国際化ファイル（12のアプリ言語）
├── docs/             # 設計 + ユーザー/開発者ガイド
└── .github/          # CI + リリースワークフロー
```

## 🤝 コントリビューション

コントリビューションを歓迎します！PRを提出する前に[開発者ガイド](docs/dev-guide.md)をお読みください。

## 📄 ライセンス

GPL-3.0-only。詳細は[LICENSE](LICENSE)をご覧ください。

## ⭐ スター履歴

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Rust + Tauri 2 + React で構築、❤️ を込めて**

</div>
