<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Herramienta de traducción multiplataforma

Selecciona texto en cualquier lugar → pulsa una tecla → traduce al instante

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## 📥 Instalación

### Descargar desde Releases

**Última versión: v0.2.0**

Descargue el instalador para su plataforma desde [GitHub Releases](https://github.com/poneding/translator/releases/latest):

- **macOS**: `translator_0.2.0_universal.dmg` o `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - Abra el archivo DMG y arrastre la aplicación a la carpeta Aplicaciones
  - En el primer inicio, haga clic derecho en la aplicación y seleccione "Abrir" para omitir Gatekeeper
  
- **Windows**: `translator_0.2.0_x64-setup.exe` o `translator_0.2.0_x64_en-US.msi`
  - Ejecute el instalador y siga el asistente de configuración
  - Windows Defender SmartScreen puede mostrar una advertencia; haga clic en "Más información" → "Ejecutar de todas formas"
  
- **Linux**: `translator_0.2.0_amd64.deb` o `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB): `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage**: `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### Compilar desde código fuente

Consulte la sección [Inicio rápido](#-inicio-rápido) a continuación.

---

## ✨ Características

- 🌍 **Atajo global** — Traduce texto seleccionado desde cualquier aplicación al instante
- 🔌 **5 servicios de traducción** — Youdao (有道), DeepL, Google, Bing (Azure), compatible con OpenAI
- 🤖 **Detección automática de idioma** — Reconocimiento inteligente del idioma fuente
- 🎯 **Traducción en ventana principal** — Fijar, historial, reproducción de audio, reintentar por servicio
- 📋 **Respaldo del portapapeles** — Traduce el portapapeles cuando la selección no está disponible
- 🔄 **Actualizaciones integradas** — Canales de lanzamiento estable/beta
- 🎨 **Modo oscuro** — Sigue las preferencias del sistema
- 🌏 **12 idiomas de interfaz** — Cambio de idioma de la aplicación en tiempo real
- 🔐 **Almacenamiento seguro** — Claves API almacenadas en el llavero del SO
- ⚡ **Ligero** — Binario de ~6 MB, memoria < 50 MB

## 📸 Capturas de pantalla

<div align="center">

<table>
<tr>
<td width="50%">

### Modo claro
<img src="docs/screenshots/light.png" alt="Modo claro">

</td>
<td width="50%">

### Modo oscuro
<img src="docs/screenshots/dark.png" alt="Modo oscuro">

</td>
</tr>
</table>

</div>

## 🚀 Inicio rápido

### Requisitos previos

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Dependencias de plataforma:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (preinstalado en Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Desarrollo

```bash
# Instalar dependencias de JavaScript
cd ui && npm install && cd ..

# Ejecutar servidor de desarrollo (recarga en caliente habilitada)
cargo tauri dev
```

### Compilar versión de lanzamiento

```bash
cargo tauri build
```

**Ubicación de salida:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 Documentación

- 📐 [Documento de diseño](docs/DESIGN.md) — Descripción general de la arquitectura v0.2
- 🏛️ [Diagrama de arquitectura](docs/ARCHITECTURE.svg) — Mapa visual de componentes
- 🛠️ [Guía del desarrollador](docs/dev-guide.md) — Convenciones de codificación, pruebas, depuración
- 👤 [Guía del usuario](docs/user-guide.md) — Instrucciones de configuración, claves API, personalización de atajos

## 📂 Estructura del proyecto

```txt
translator/
├── crates/
│   ├── core/         # Lógica de negocio Rust pura + 5 servicios de traducción
│   ├── platform/     # Monitor de selección multiplataforma (macOS/Win/Linux)
│   └── app/          # Shell Tauri (comandos, bandeja, IPC)
├── ui/               # Frontend React + Vite (ventana principal + configuración)
├── ui/src/locales/   # Archivos de i18n Fluent (12 idiomas de aplicación)
├── docs/             # Diseño + guías de usuario/desarrollador
└── .github/          # Flujos de trabajo CI + lanzamiento
```

## 🤝 Contribuir

¡Las contribuciones son bienvenidas! Por favor lea nuestra [Guía del desarrollador](docs/dev-guide.md) antes de enviar PRs.

## 📄 Licencia

GPL-3.0-only. Consulte [LICENSE](LICENSE) para más detalles.

## ⭐ Historial de estrellas

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Construido con ❤️ usando Rust + Tauri 2 + React**

</div>
