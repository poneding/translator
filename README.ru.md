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

## 📥 Установка

Загрузите установщик для вашей платформы из [GitHub Releases](https://github.com/poneding/translator/releases/latest):

| Платформа | Архитектура | Рекомендуемый файл |
| --- | --- | --- |
| macOS | Intel / Apple Silicon | `.dmg` |
| Windows | x86_64 | `.msi` или `.exe` |
| Linux | x86_64 / arm64 | `.AppImage`, `.deb` или `.rpm` |

После загрузки установите как обычно: на macOS откройте `.dmg` и перетащите приложение в «Программы» (при первом запуске щелкните правой кнопкой → «Открыть» для обхода Gatekeeper); на Windows запустите установщик (при предупреждении SmartScreen нажмите «Подробнее» → «Выполнить в любом случае»); на Linux запустите `.AppImage` напрямую или установите `.deb` / `.rpm` через менеджер пакетов.

Для сборки из исходников см. раздел [Быстрый старт](#-быстрый-старт) ниже.

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

## 🙏 Благодарности

- **[EasyDict](https://github.com/tisfeng/EasyDict)** — Translator во многом вдохновлён тем, как EasyDict показывает результаты перевода. Несколько словарных функций и функций множественных результатов были перенесены из его Swift-реализации, включая разбор словаря Youdao V4, словарь Bing v7 и запрос `tlookupv3`, endpoint Google WebApp с подписью `tk`, а также компоновку карточек результатов. EasyDict — отличное приложение **только для macOS**; Translator стремится дать сопоставимый опыт на macOS, **Windows и Linux**. Мы искренне благодарим автора EasyDict и его участников.

## 📄 Лицензия

GPL-3.0-only. Подробности см. в [LICENSE](LICENSE).

## ⭐ История звезд

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**Создано с ❤️ на Rust + Tauri 2 + React**

</div>
