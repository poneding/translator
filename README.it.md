<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Strumento di traduzione multipiattaforma

Seleziona il testo ovunque → premi un tasto di scelta rapida → traduci istantaneamente

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## ✨ Caratteristiche

- 🌍 **Tasto di scelta rapida globale** — Traduci il testo selezionato da qualsiasi app istantaneamente
- 🔌 **5 servizi di traduzione** — Youdao (有道), DeepL, Google, Bing (Azure), compatibile OpenAI
- 🤖 **Rilevamento automatico della lingua** — Riconoscimento intelligente della lingua di origine
- 🎯 **Traduzione nella finestra principale** — Blocca, cronologia, riproduzione audio, riprova per servizio
- 📋 **Fallback degli appunti** — Traduci gli appunti quando la selezione non è disponibile
- 🔄 **Aggiornamenti integrati** — Canali di rilascio stabile/beta
- 🎨 **Modalità scura** — Segue le preferenze di sistema
- 🌏 **12 lingue dell'interfaccia** — Cambio lingua dell'app in tempo reale
- 🔐 **Archiviazione sicura** — Chiavi API memorizzate nel Portachiavi del sistema operativo
- ⚡ **Leggero** — Binario ~6 MB, memoria < 50 MB

## 📸 Screenshot

<div align="center">

<table>
<tr>
<td width="50%">

### Modalità chiara
<img src="docs/screenshots/light.png" alt="Modalità chiara">

</td>
<td width="50%">

### Modalità scura
<img src="docs/screenshots/dark.png" alt="Modalità scura">

</td>
</tr>
</table>

</div>

## 📥 Installazione

Scarica l'installer per la tua piattaforma da [GitHub Releases](https://github.com/poneding/translator/releases/latest):

| Piattaforma | Architettura | Download consigliato |
| --- | --- | --- |
| macOS | Intel / Apple Silicon | `.dmg` |
| Windows | x86_64 | `.msi` o `.exe` |
| Linux | x86_64 / arm64 | `.AppImage`, `.deb` o `.rpm` |

Dopo il download, installa come di consueto: su macOS, apri il `.dmg` e trascina l'app in Applicazioni (al primo avvio, clic destro → "Apri" per ignorare Gatekeeper); su Windows, esegui l'installer (se SmartScreen avvisa, clicca su "Ulteriori informazioni" → "Esegui comunque"); su Linux, esegui direttamente il `.AppImage` o installa il `.deb` / `.rpm` con il gestore di pacchetti.

Per compilare dal codice sorgente, consulta la sezione [Avvio rapido](#-avvio-rapido) di seguito.

## 🚀 Avvio rapido

### Prerequisiti

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Dipendenze della piattaforma:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (preinstallato su Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Sviluppo

```bash
# Installa le dipendenze JavaScript
cd ui && npm install && cd ..

# Esegui il server di sviluppo (ricarica automatica abilitata)
cargo tauri dev
```

### Build di rilascio

```bash
cargo tauri build
```

**Posizione di output:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 Documentazione

- 📐 [Documento di progettazione](docs/DESIGN.md) — Panoramica dell'architettura v0.2
- 🏛️ [Diagramma dell'architettura](docs/ARCHITECTURE.svg) — Mappa visiva dei componenti
- 🛠️ [Guida per sviluppatori](docs/dev-guide.md) — Convenzioni di codifica, test, debug
- 👤 [Guida utente](docs/user-guide.md) — Istruzioni di configurazione, chiavi API, personalizzazione dei tasti di scelta rapida

## 📂 Struttura del progetto

```txt
translator/
├── crates/
│   ├── core/         # Logica di business Rust pura + 5 servizi di traduzione
│   ├── platform/     # Monitor di selezione multipiattaforma (macOS/Win/Linux)
│   └── app/          # Shell Tauri (comandi, vassoio, IPC)
├── ui/               # Frontend React + Vite (finestra principale + impostazioni)
├── ui/src/locales/   # File di internazionalizzazione Fluent (12 lingue app)
├── docs/             # Progettazione + guide utente/sviluppatore
└── .github/          # Workflow CI + rilascio
```

## 🤝 Contribuire

I contributi sono benvenuti! Si prega di leggere la nostra [Guida per sviluppatori](docs/dev-guide.md) prima di inviare PR.

## 📄 Licenza

GPL-3.0-only. Vedere [LICENSE](LICENSE) per i dettagli.

## ⭐ Cronologia stelle

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Realizzato con ❤️ usando Rust + Tauri 2 + React**

</div>
