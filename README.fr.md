<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Outil de traduction multiplateforme

Sélectionnez du texte n'importe où → appuyez sur un raccourci → traduisez instantanément

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

### Télécharger depuis les Releases

**Dernière version : v0.2.0**

Téléchargez l'installateur pour votre plateforme depuis [GitHub Releases](https://github.com/poneding/translator/releases/latest) :

- **macOS** : `translator_0.2.0_universal.dmg` ou `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - Ouvrez le fichier DMG et faites glisser l'application vers le dossier Applications
  - Au premier lancement, cliquez avec le bouton droit sur l'application et sélectionnez « Ouvrir » pour contourner Gatekeeper
  
- **Windows** : `translator_0.2.0_x64-setup.exe` ou `translator_0.2.0_x64_en-US.msi`
  - Exécutez l'installateur et suivez l'assistant de configuration
  - Windows Defender SmartScreen peut afficher un avertissement ; cliquez sur « Plus d'infos » → « Exécuter quand même »
  
- **Linux** : `translator_0.2.0_amd64.deb` ou `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB) : `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage** : `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### Compiler depuis les sources

Consultez la section [Démarrage rapide](#-démarrage-rapide) ci-dessous.

---

## ✨ Fonctionnalités

- 🌍 **Raccourci global** — Traduisez le texte sélectionné depuis n'importe quelle application instantanément
- 🔌 **5 services de traduction** — Youdao (有道), DeepL, Google, Bing (Azure), compatible OpenAI
- 🤖 **Détection automatique de la langue** — Reconnaissance intelligente de la langue source
- 🎯 **Traduction dans la fenêtre principale** — Épinglage, historique, lecture audio, réessai par service
- 📋 **Solution de repli du presse-papiers** — Traduisez le presse-papiers lorsque la sélection n'est pas disponible
- 🔄 **Mises à jour intégrées** — Canaux de version stable/bêta
- 🎨 **Mode sombre** — Suit les préférences système
- 🌏 **12 langues d'interface** — Changement de langue de l'application en temps réel
- 🔐 **Stockage sécurisé** — Clés API stockées dans le trousseau de l'OS
- ⚡ **Léger** — Binaire ~6 Mo, mémoire < 50 Mo

## 📸 Captures d'écran

<div align="center">

<table>
<tr>
<td width="50%">

### Mode clair
<img src="docs/screenshots/light.png" alt="Mode clair">

</td>
<td width="50%">

### Mode sombre
<img src="docs/screenshots/dark.png" alt="Mode sombre">

</td>
</tr>
</table>

</div>

## 🚀 Démarrage rapide

### Prérequis

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Dépendances de plateforme :**
  - **macOS** : `xcode-select --install`
  - **Windows** : Microsoft C++ Build Tools + WebView2 (préinstallé sur Win10+)
  - **Linux** : 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Développement

```bash
# Installer les dépendances JavaScript
cd ui && npm install && cd ..

# Exécuter le serveur de développement (rechargement à chaud activé)
cargo tauri dev
```

### Build de release

```bash
cargo tauri build
```

**Emplacement de sortie :** `target/release/bundle/`

- **macOS** : `.dmg` + `.app`
- **Windows** : `.msi` + `.exe`
- **Linux** : `.AppImage` + `.deb`

## 📚 Documentation

- 📐 [Document de conception](docs/DESIGN.md) — Aperçu de l'architecture v0.2
- 🏛️ [Diagramme d'architecture](docs/ARCHITECTURE.svg) — Carte visuelle des composants
- 🛠️ [Guide du développeur](docs/dev-guide.md) — Conventions de codage, tests, débogage
- 👤 [Guide de l'utilisateur](docs/user-guide.md) — Instructions de configuration, clés API, personnalisation des raccourcis

## 📂 Structure du projet

```txt
translator/
├── crates/
│   ├── core/         # Logique métier Rust pure + 5 services de traduction
│   ├── platform/     # Moniteur de sélection multiplateforme (macOS/Win/Linux)
│   └── app/          # Shell Tauri (commandes, barre d'état, IPC)
├── ui/               # Frontend React + Vite (fenêtre principale + paramètres)
├── ui/src/locales/   # Fichiers d'internationalisation Fluent (12 langues d'application)
├── docs/             # Guides de conception + utilisateur/développeur
└── .github/          # Workflows CI + release
```

## 🤝 Contribuer

Les contributions sont les bienvenues ! Veuillez lire notre [Guide du développeur](docs/dev-guide.md) avant de soumettre des PR.

## 📄 Licence

GPL-3.0-only. Voir [LICENSE](LICENSE) pour plus de détails.

## ⭐ Historique des étoiles

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Construit avec ❤️ en utilisant Rust + Tauri 2 + React**

</div>
