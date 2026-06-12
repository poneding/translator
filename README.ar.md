<div align="center">

# Translator

<img src="ui/public/app-icon.png" alt="Translator Logo" width="120" height="120">

### أداة ترجمة متعددة المنصات

حدد النص في أي مكان → اضغط على مفتاح الاختصار → ترجمة فورية

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8.svg)](https://tauri.app)
[![React](https://img.shields.io/badge/React-18+-61DAFB.svg)](https://reactjs.org)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/poneding/translator)
<a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>

[English](README.md) | [简体中文](README.zh-Hans.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [Español](README.es.md) | [Русский](README.ru.md) | [Português](README.pt.md) | [Italiano](README.it.md) | [العربية](README.ar.md)

</div>

---

## 📥 التثبيت

### التحميل من Releases

**أحدث إصدار: v0.2.0**

قم بتنزيل المثبت لمنصتك من [GitHub Releases](https://github.com/poneding/translator/releases/latest):

- **macOS**: `translator_0.2.0_universal.dmg` أو `translator_0.2.0_aarch64.dmg` / `translator_0.2.0_x64.dmg`
  - افتح ملف DMG واسحب التطبيق إلى مجلد التطبيقات
  - عند التشغيل الأول، انقر بزر الماوس الأيمن على التطبيق واختر "فتح" لتجاوز Gatekeeper
  
- **Windows**: `translator_0.2.0_x64-setup.exe` أو `translator_0.2.0_x64_en-US.msi`
  - قم بتشغيل المثبت واتبع معالج الإعداد
  - قد يظهر Windows Defender SmartScreen تحذيرًا؛ انقر على "مزيد من المعلومات" → "تشغيل على أي حال"
  
- **Linux**: `translator_0.2.0_amd64.deb` أو `translator_0.2.0_amd64.AppImage`
  - **Debian/Ubuntu** (DEB): `sudo dpkg -i translator_0.2.0_amd64.deb`
  - **AppImage**: `chmod +x translator_0.2.0_amd64.AppImage && ./translator_0.2.0_amd64.AppImage`

### البناء من المصدر

راجع قسم [البدء السريع](#-البدء-السريع) أدناه.

---

## ✨ الميزات

- 🌍 **مفتاح اختصار عام** — ترجمة النص المحدد من أي تطبيق على الفور
- 🔌 **5 خدمات ترجمة** — Youdao (有道)، DeepL، Google، Bing (Azure)، متوافق مع OpenAI
- 🤖 **الكشف التلقائي عن اللغة** — التعرف الذكي على اللغة المصدر
- 🎯 **الترجمة في النافذة الرئيسية** — تثبيت، سجل، تشغيل صوتي، إعادة المحاولة لكل خدمة
- 📋 **احتياطي الحافظة** — ترجمة الحافظة عندما يكون التحديد غير متاح
- 🔄 **التحديثات المدمجة** — قنوات إصدار مستقرة/تجريبية
- 🎨 **الوضع الداكن** — يتبع تفضيلات النظام
- 🌏 **12 لغة واجهة** — تبديل لغة التطبيق في الوقت الفعلي
- 🔐 **التخزين الآمن** — مفاتيح API مخزنة في سلسلة مفاتيح نظام التشغيل
- ⚡ **خفيف الوزن** — ملف ثنائي ~6 ميجابايت، ذاكرة < 50 ميجابايت

## 📸 لقطات الشاشة

<div align="center">

<table>
<tr>
<td width="50%">

### الوضع الفاتح
<img src="docs/screenshots/light.png" alt="الوضع الفاتح">

</td>
<td width="50%">

### الوضع الداكن
<img src="docs/screenshots/dark.png" alt="الوضع الداكن">

</td>
</tr>
</table>

</div>

## 🚀 البدء السريع

### المتطلبات الأساسية

- **Rust** 1.81+ (`rustup install stable`)
- **Node.js** 20+
- **تبعيات المنصة:**
  - **macOS**: `xcode-select --install`
  - **Windows**: Microsoft C++ Build Tools + WebView2 (مثبت مسبقًا على Win10+)
  - **Linux**: 
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev \
                     libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

### التطوير

```bash
# تثبيت تبعيات JavaScript
cd ui && npm install && cd ..

# تشغيل خادم التطوير (إعادة التحميل الساخن مفعلة)
cargo tauri dev
```

### بناء الإصدار

```bash
cargo tauri build
```

**موقع الإخراج:** `target/release/bundle/`

- **macOS**: `.dmg` + `.app`
- **Windows**: `.msi` + `.exe`
- **Linux**: `.AppImage` + `.deb`

## 📚 الوثائق

- 📐 [وثيقة التصميم](docs/DESIGN.md) — نظرة عامة على البنية v0.2
- 🏛️ [مخطط البنية](docs/ARCHITECTURE.svg) — خريطة مرئية للمكونات
- 🛠️ [دليل المطور](docs/dev-guide.md) — اتفاقيات الترميز، الاختبار، التصحيح
- 👤 [دليل المستخدم](docs/user-guide.md) — تعليمات الإعداد، مفاتيح API، تخصيص مفاتيح الاختصار

## 📂 هيكل المشروع

```txt
translator/
├── crates/
│   ├── core/         # منطق الأعمال Rust النقي + 5 خدمات ترجمة
│   ├── platform/     # مراقب التحديد متعدد المنصات (macOS/Win/Linux)
│   └── app/          # غلاف Tauri (الأوامر، الدرج، IPC)
├── ui/               # واجهة React + Vite (النافذة الرئيسية + الإعدادات)
├── ui/src/locales/   # ملفات التدويل Fluent (12 لغة تطبيق)
├── docs/             # التصميم + أدلة المستخدم/المطور
└── .github/          # سير عمل CI + الإصدار
```

## 🤝 المساهمة

المساهمات مرحب بها! يرجى قراءة [دليل المطور](docs/dev-guide.md) قبل تقديم PRs.

## 📄 الترخيص

GPL-3.0-only. راجع [LICENSE](LICENSE) للتفاصيل.

## ⭐ تاريخ النجوم

[![Star History Chart](https://api.star-history.com/svg?repos=poneding/translator&type=Date)](https://star-history.com/#poneding/translator&Date)

---

<div align="center">

**مبني بـ ❤️ باستخدام Rust + Tauri 2 + React**

</div>
