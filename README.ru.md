<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### Кроссплатформенный инструмент перевода выделенного текста

Выделите текст где угодно → нажмите горячую клавишу → мгновенный перевод

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## 📥 Установка

### Загрузка из Releases

**Последняя версия: v0.2.0**

Загрузите установщик для вашей платформы из [GitHub Releases](https://github.com/poneding/translator/releases/latest):

- **macOS**: `translator_0.2.0_universal.dmg` или `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - Откройте DMG-файл и перетащите приложение в папку «Программы»
  - При первом запуске щелкните правой кнопкой мыши на приложении и выберите «Открыть», чтобы обойти Gatekeeper
  
- **Windows**: `translator_0.2.0_x64-setup.exe` или `translator_0.2.0_x64_en-US.msi`
  - Запустите установщик и следуйте мастеру установки
  - Windows Defender SmartScreen может показать предупреждение; нажмите «Подробнее» → «Выполнить в любом случае»
  
- **Linux**: `translator_0.2.0_amd64.deb` или `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB): `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage**: `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### Сборка из исходников

См. раздел [Быстрый старт](#-быстрый-старт) ниже.

---

## ✨ Особенности

- 🌍 **Глобальная горячая клавиша** — Мгновенный перевод выделенного текста из любого приложения
- 🔌 **5 служб перевода** — Youdao (有道), DeepL, Google, Bing (Azure), совместимость с OpenAI
- 🤖 **Автоопределение языка** — Интеллектуальное распознавание исходного языка
- 🎯 **Перевод в главном окне** — Закрепление, история, воспроизведение аудио, повтор по службам
- 📋 **Резервный буфер обмена** — Перевод из буфера обмена, когда выделение недоступно
- 🔄 **Встроенные обновления** — Каналы стабильных/бета-релизов
- 🎨 **Темный режим** — Следует системным настройкам
- 🌏 **12 языков интерфейса** — Переключение языка приложения в реальном времени
- 🔐 **Безопасное хранилище** — API-ключи хранятся в связке ключей ОС
- ⚡ **Легкий** — Бинарник ~6 МБ, память < 50 МБ

## 📸 Скриншоты

<div align="center">

<table>
<tr>
<td width="50%">

### Светлая тема
<img src="docs/screenshots/light.png" alt="Светлая тема">

</td>
<td width="50%">

### Темная тема
<img src="docs/screenshots/dark.png" alt="Темная тема">

</td>
</tr>
</table>

</div>

## 🚀 Быстрый старт

### Предварительные требования

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **Зависимости платформы:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (предустановлен на Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### Разработка

```bash
# Установить зависимости JavaScript
cd ui && npm install && cd ..

# Запустить dev-сервер (с горячей перезагрузкой)
cargo tauri dev
```

### Сборка релиза

```bash
cargo tauri build
```

**Расположение вывода:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 Документация

- 📐 [Проектный документ](docs/DESIGN.md) — Обзор архитектуры v0.2
- 🏛️ [Архитектурная диаграмма](docs/ARCHITECTURE.svg) — Визуальная карта компонентов
- 🛠️ [Руководство разработчика](docs/dev-guide.md) — Соглашения о коде, тестирование, отладка
- 👤 [Руководство пользователя](docs/user-guide.md) — Инструкции по настройке, API-ключи, настройка горячих клавиш

## 📂 Структура проекта

```txt
translator/
├── crates/
│   ├── core/         # Чистая бизнес-логика Rust + 5 служб перевода
│   ├── platform/     # Кроссплатформенный монитор выделения (macOS/Win/Linux)
│   └── app/          # Оболочка Tauri (команды, трей, IPC)
├── ui/               # Frontend React + Vite (главное окно + настройки)
├── ui/src/locales/   # Файлы интернационализации Fluent (12 языков приложения)
├── docs/             # Проектная документация + руководства пользователя/разработчика
└── .github/          # CI + рабочие процессы релизов
```

## 🤝 Вклад

Приветствуются вклады! Пожалуйста, прочитайте наше [Руководство разработчика](docs/dev-guide.md) перед отправкой PR.

## 📄 Лицензия

GPL-3.0-only. Подробности см. в [LICENSE](LICENSE).

## ⭐ История звезд

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Создано с ❤️ на Rust + Tauri 2 + React**

</div>
