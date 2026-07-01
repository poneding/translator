<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Plattformübergreifendes Auswahl-Übersetzungstool

Text überall auswählen → Hotkey drücken → sofort übersetzen

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## ✨ Funktionen

- 🌍 **Globaler Hotkey** — Übersetzen Sie ausgewählten Text aus jeder App sofort
- 🔌 **5 Übersetzungsdienste** — Youdao (有道), DeepL, Google, Bing (Azure), OpenAI-kompatibel
- 🤖 **Automatische Spracherkennung** — Intelligente Quellspracherkennung
- 🎯 **Hauptfenster-Übersetzung** — Anheften, Verlauf, Audio-Wiedergabe, Wiederholung pro Dienst
- 📋 **Zwischenablage-Fallback** — Übersetzen Sie die Zwischenablage, wenn keine Auswahl verfügbar ist
- 🔄 **Integrierte Updates** — Stabile/Beta-Release-Kanäle
- 🎨 **Dunkler Modus** — Folgt Systemeinstellungen
- 🌏 **12 UI-Sprachen** — Live-Sprachwechsel der App
- 🔐 **Sichere Speicherung** — API-Schlüssel im OS-Schlüsselbund gespeichert
- ⚡ **Leichtgewichtig** — ~6 MB Binärdatei, < 50 MB Speicher

## 📸 Screenshots

<div align="center">

<table>
<tr>
<td width="50%">

### Heller Modus
<img src="docs/screenshots/light.png" alt="Heller Modus">

</td>
<td width="50%">

### Dunkler Modus
<img src="docs/screenshots/dark.png" alt="Dunkler Modus">

</td>
</tr>
</table>

</div>

## 📥 Installation

Laden Sie den Installer für Ihre Plattform von [GitHub Releases](https://github.com/poneding/translator/releases/latest) herunter:

| Plattform | Architektur | Empfohlener Download |
| --- | --- | --- |
| macOS | Intel / Apple Silicon | `.dmg` |
| Windows | x86_64 | `.msi` oder `.exe` |
| Linux | x86_64 / arm64 | `.AppImage`, `.deb` oder `.rpm` |

Nach dem Download wie gewohnt installieren: Unter macOS die `.dmg` öffnen und die App in Programme ziehen (beim ersten Start Rechtsklick → „Öffnen" zum Umgehen von Gatekeeper); unter Windows den Installer ausführen (bei SmartScreen-Warnung „Weitere Informationen" → „Trotzdem ausführen"); unter Linux die `.AppImage` direkt ausführen oder `.deb` / `.rpm` mit dem Paketmanager installieren.

Für die Erstellung aus Quellen siehe [Schnellstart](#-schnellstart) unten.

## 🚀 Schnellstart

### Voraussetzungen

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Plattform-Abhängigkeiten:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (vorinstalliert auf Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Entwicklung

```bash
# JavaScript-Abhängigkeiten installieren
cd ui && npm install && cd ..

# Dev-Server ausführen (Hot Reload aktiviert)
cargo tauri dev
```

### Release Build

```bash
cargo tauri build
```

**Ausgabeort:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 Dokumentation

- 📐 [Design-Dokument](docs/DESIGN.md) — Architektur-Übersicht v0.2
- 🏛️ [Architektur-Diagramm](docs/ARCHITECTURE.svg) — Visuelle Komponentenkarte
- 🛠️ [Entwicklerhandbuch](docs/dev-guide.md) — Coding-Konventionen, Tests, Debugging
- 👤 [Benutzerhandbuch](docs/user-guide.md) — Setup-Anleitung, API-Schlüssel, Hotkey-Anpassung

## 📂 Projektstruktur

```txt
translator/
├── crates/
│   ├── core/         # Reine Rust-Geschäftslogik + 5 Übersetzungsdienste
│   ├── platform/     # Plattformübergreifender Auswahl-Monitor (macOS/Win/Linux)
│   └── app/          # Tauri-Shell (Befehle, Tray, IPC)
├── ui/               # React + Vite Frontend (Hauptfenster + Einstellungen)
├── ui/src/locales/   # Fluent-i18n-Dateien (12 App-Sprachen)
├── docs/             # Design + Benutzer-/Entwickler-Handbücher
└── .github/          # CI + Release-Workflows
```

## 🤝 Beitragen

Beiträge sind willkommen! Bitte lesen Sie unser [Entwicklerhandbuch](docs/dev-guide.md), bevor Sie PRs einreichen.

## 🙏 Danksagung

- **[EasyDict](https://github.com/tisfeng/EasyDict)** — Translator ist stark von der Übersetzungsergebnis-Erfahrung von EasyDict inspiriert. Mehrere Wörterbuch- und Mehrfachergebnisfunktionen wurden aus dessen Swift-Implementierung übernommen, darunter das Parsen des Youdao-V4-Wörterbuchs, das Bing-v7-Wörterbuch mit `tlookupv3`-Abfrage, der Google-WebApp-Endpunkt mit `tk`-Signatur sowie das Layout der Ergebniskarten. EasyDict ist eine hervorragende **nur für macOS** verfügbare App; Translator möchte ein vergleichbares Erlebnis auf macOS, **Windows und Linux** bieten. Unser aufrichtiger Dank gilt dem Autor von EasyDict und allen Mitwirkenden.

## 📄 Lizenz

GPL-3.0-only. Details siehe [LICENSE](LICENSE).

## ⭐ Star-Verlauf

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Erstellt mit ❤️ unter Verwendung von Rust + Tauri 2 + React**

</div>
