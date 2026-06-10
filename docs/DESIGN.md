# Translator 设计文档

> 版本：v0.1 · 日期：2026-06-02 · 状态：历史设计文档
>
> 目标：跨平台（macOS / Windows / Linux）选词翻译工具。选中文字后按快捷键 → 自动检测语种 → 并行调用已配置服务 → 浮窗显示结果。
>
> v0.2.0 已迁移为主窗口优先流程；当前行为以
> [`SPEC-v0.2.0.md`](SPEC-v0.2.0.md) 和
> [`PLAN-v0.2.0.md`](PLAN-v0.2.0.md) 为准。

---

## 1. 项目概述

### 1.1 一句话定义

`translator` 是一个常驻系统托盘的轻量级跨平台选词翻译应用：选中任意文字 → 按下全局快捷键 → 自动识别源语言 → 并行调用 1～5 个翻译服务 → 浮窗分 Tab 展示结果 → 一键复制。

### 1.2 与同类项目的差异

| 项目 | 平台 | 体积 | 速度 | 限制 |
| --- | --- | --- | --- | --- |
| **Easydict** | macOS only | ~25 MB | 慢（SwiftUI + 27 服务） | 仅 macOS |
| **Bob** | macOS only | ~20 MB | 慢 | 仅 macOS，需手动导入服务 |
| **Saladict** | 浏览器扩展 | N/A | — | 仅网页 |
| **DeepL App** | 全平台 | ~120 MB | 中 | 仅自家 |
| **本项目 translator** | **macOS / Windows / Linux** | **~6 MB** | **快（Rust + 小 WebView）** | 5 个服务，无 OCR |

### 1.3 目标用户

- 重度查阅外文资料的开发者 / 研究者 / 翻译者
- 不愿意被特定平台绑定的 macOS + Windows 双修用户
- 愿意自己申请 API key 换取翻译质量/隐私的用户

---

## 2. 范围 (Scope)

### 2.1 v1 包含

| 功能 | 形态 |
| --- | --- |
| 全局选区文本捕获 | macOS / Windows / Linux 三平台 |
| 全局快捷键（默认 ⌘+Shift+D / Ctrl+Shift+D） | Tauri 官方插件 |
| 自动语言识别 | 由各翻译服务的 `auto` 参数处理 |
| 5 个翻译服务（见 §4） | 并行调用，UI 分 Tab |
| 浮窗结果展示 | 跟随光标，5 秒无操作自动隐藏 |
| 系统托盘（macOS 菜单栏 / Win 托盘 / Linux AppIndicator） | Tauri 官方插件 |
| 设置面板（服务 / API key / 目标语言 / 快捷键 / 主题） | 独立窗口 |
| API key 安全存储 | 系统 Keychain（macOS/Windows/Linux Secret Service） |
| 中英双语 UI | `fluent` 国际化 |
| 暗色模式 | 跟随系统 |
| 一键复制结果 | 剪贴板插件 |
| 平台原生打包 | .dmg / .msi / .AppImage / .deb |

### 2.2 v1 明确不做

- OCR / 截图翻译
- 离线词典 / 离线翻译
- 文本替换（直接回写到原窗口）
- TTS 发音
- iOS / Android 客户端
- 第三方服务插件系统
- 历史记录云同步
- 账号系统
- 自动更新（v1 走 GitHub Release 手动分发）

### 2.3 未来可能加（v2+）

- 流式输出（SSE）改善 LLM 翻译的首字延迟
- 自定义服务（用户填 HTTP 端点 + 请求/响应模板）
- 翻译记忆 / 术语表
- 选词高亮触发（除快捷键外）

---

## 3. 技术选型

| 层 | 选型 | 备选 / 理由 |
| --- | --- | --- |
| 核心语言 | **Rust 1.81+** | 类型安全 + 跨平台一致 + 单 binary |
| GUI 框架 | **Tauri 2.x** | 体积小、跨平台一致、生态成熟（2024-10 稳定） |
| 前端 | **React 18 + TypeScript + Vite** | 生态最广、HMR 快、ts-rs 自动生成类型 |
| 样式 | **Tailwind CSS 3** + shadcn/ui 风格手写组件 | 不引入完整 shadcn 减少体积 |
| 状态 | **Zustand** | 比 Redux 轻，比 useState 全局好 |
| 异步运行时 | `tokio` 1.41 | 行业标准 |
| HTTP | `reqwest` 0.12 (rustls) | 行业标准 |
| 序列化 | `serde` + `serde_json` | — |
| 错误 | `thiserror` + `anyhow` | — |
| 日志 | `tracing` | 结构化 |
| Keychain | `keyring` 3.6 | 跨平台 |
| i18n | `fluent` 0.17 | Mozilla 出品，message + 复数 + 性别 |
| 平台胶水 | `objc2` (macOS) / `windows` (Win) / `atspi` (Linux) | 见 §6 |
| 类型生成 | `ts-rs` | Rust struct → TypeScript interface |
| Lint | `cargo clippy` + `eslint` + `prettier` | — |
| 格式化 | `cargo fmt` + `prettier` | — |
| 测试 | `cargo test` + `vitest` | — |
| CI | GitHub Actions | 三平台矩阵 + 自动 release |

**不引入**：egui、Iced、Slint（无 WebView 复用）、Druid（已弃用）、GTK-rs（macOS 体验差）、tao 直接用（Tauri 已经包了）。

---

## 4. 翻译服务（v1 共 5 个）

### 4.1 总览

| 编号 | 服务 ID | 显示名 | 鉴权 | 计费 | 质量 | 速度 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `youdao` | 有道翻译 | appKey + appSecret | 免费 1M 字符/月 | 中 | 快 |
| 2 | `deepl` | DeepL | `DeepL-Auth-Key` | 免费 500K 字符/月 | **最高** | 快 |
| 3 | `google` | Google 翻译 | API key (Cloud v3) | 送 $300 额度 | 高 | 中 |
| 4 | `bing` | 微软翻译 | Azure subscription key | 免费层 | 中 | 快 |
| 5 | `openai` | OpenAI 兼容 | base_url + api_key + model | 用户自定 | **可调高** | 慢（流式） |

> 顺序按"开箱即用难度"递增：OpenAI 兼容最灵活但需自配；DeepL 免费额度大且质量好；Google/Bing/有道各有官方/非官方入口。

### 4.2 各服务细节

#### 4.2.1 有道翻译 (Youdao)

- 端点：`https://openapi.youdao.com/api`
- 鉴权：HMAC-SHA256 签名，`sign = sha256(appKey + truncate(q) + salt + curtime + appSecret)`
- 必填：`q`（待翻译文本，≤ 5K 字符）、`from`（auto / `en` / `zh-CHS` 等）、`to`、`appKey`、`salt`、`curtime`、`sign`、`signType=v3`
- 响应字段：`translation[0]`、`basic.explains`、`web[]`（网络释义）、`dict.json`（词典条目）
- 限流：1 QPS
- 文档：<https://ai.youdao.com/console/doc>

#### 4.2.2 DeepL

- 端点 Free：`https://api-free.deepl.com/v2/translate`
- 端点 Pro：`https://api.deepl.com/v2/translate`
- 鉴权：`Authorization: DeepL-Auth-Key <KEY>` header
- 必填：`text`（数组）、`target_lang`（必填）、`source_lang`（可选）
- 可选：`formality`（`default` / `more` / `less` / `prefer_more` / `prefer_less`）
- 响应：`translations[0].text`、`detected_source_language`、`billed_characters`
- 限流：Free 20 请求/秒
- 文档：<https://developers.deepl.com/docs>

#### 4.2.3 Google Translate (Cloud v3)

- 端点：`POST https://translation.googleapis.com/v3/projects/{projectId}/locations/global:translateText`
- 鉴权：`Authorization: Bearer <access_token>` (OAuth) 或 API key (`?key=...`)
- 请求体：`{ "sourceLanguageCode": "en", "targetLanguageCode": "zh-CN", "contents": ["..."], "mimeType": "text/plain" }`
- 响应：`translations[0].translatedText`、`detectedLanguageCode`
- 备用端点（无需 key，不保证可用）：`https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=zh-CN&dt=t&q=...`
- 文档：<https://cloud.google.com/translate/docs/reference/rest>

#### 4.2.4 Microsoft Bing (Azure)

- 端点：`POST https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&from=auto&to=zh-Hans`
- 鉴权：`Ocp-Apim-Subscription-Key: <KEY>` header（区域：`Ocp-Apim-Subscription-Region: <region>`）
- 请求体：`[{ "Text": "..." }]`
- 响应：`[0].translations[0].text`、`detectedLanguage.language`
- 文档：<https://learn.microsoft.com/azure/ai-services/translator/reference/v3-0-translate>

#### 4.2.5 OpenAI 兼容 (LLM)

- 端点：用户配置 `base_url`（默认 `https://api.openai.com/v1`）
- 路径：`POST {base_url}/chat/completions`
- 鉴权：`Authorization: Bearer <api_key>` header
- 请求体：
  ```json
  {
    "model": "gpt-4o-mini",
    "messages": [
      { "role": "system", "content": "You are a professional translator. Translate from {from} to {to}. Output ONLY the translation, no explanations." },
      { "role": "user", "content": "{text}" }
    ],
    "temperature": 0.3
  }
  ```
- 响应：`choices[0].message.content`
- **预设厂商**（UI 下拉选择自动填 base_url + 默认 model）：

  | 预设 | base_url | 默认 model |
  | --- | --- | --- |
  | OpenAI | `https://api.openai.com/v1` | `gpt-4o-mini` |
  | DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` |
  | Zhipu (智谱) | `https://open.bigmodel.cn/api/paas/v4` | `glm-4-flash` |
  | Ollama (本地) | `http://localhost:11434/v1` | `qwen2.5:7b` |
  | OpenRouter | `https://openrouter.ai/api/v1` | `openai/gpt-4o-mini` |
  | 自定义 | 用户填 | 用户填 |

### 4.3 服务 trait 设计

```rust
// crates/core/src/service.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceId {
    Youdao,
    DeepL,
    Google,
    Bing,
    OpenAI,
}

impl ServiceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Youdao => "youdao",
            Self::DeepL => "deepl",
            Self::Google => "google",
            Self::Bing => "bing",
            Self::OpenAI => "openai",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyRequirement {
    None,
    Optional,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub id: ServiceId,
    pub enabled: bool,
    pub priority: u8,           // 排序，越小越前
    pub credentials: serde_json::Value, // 各服务自定义：DeepL 仅 authKey；Youdao 需 appKey+appSecret；OpenAI 需 base_url+key+model
}

#[derive(Debug, Clone)]
pub struct TranslateRequest {
    pub text: String,
    pub from: Option<String>,   // None = auto
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    pub service_id: ServiceId,
    pub service_name: String,
    pub text: String,
    pub detected_source: Option<String>,
    pub elapsed_ms: u64,
    pub extra: Option<serde_json::Value>, // 词典条目等扩展信息（Youdao）
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("missing credentials: {0}")]
    MissingCredentials(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("api error [{code}]: {message}")]
    Api { code: String, message: String },
    #[error("invalid response: {0}")]
    Parse(String),
    #[error("rate limited, retry after {0}ms")]
    RateLimited(u64),
    #[error("timeout after {0}ms")]
    Timeout(u64),
    #[error("cancelled")]
    Cancelled,
}

#[async_trait]
pub trait TranslationService: Send + Sync {
    fn id(&self) -> ServiceId;
    fn display_name(&self) -> &'static str;
    fn api_key_requirement(&self) -> ApiKeyRequirement;
    fn config_schema(&self) -> serde_json::Value; // 供前端动态生成表单

    async fn translate(
        &self,
        req: &TranslateRequest,
        cfg: &ServiceConfig,
        client: &reqwest::Client,
    ) -> Result<TranslateResult, ServiceError>;
}

pub type DynService = Arc<dyn TranslationService>;

/// 注册表：按 id 拿到 service impl
pub fn default_services() -> Vec<DynService> {
    vec![
        Arc::new(youdao::YoudaoService),
        Arc::new(deepl::DeepLService),
        Arc::new(google::GoogleService),
        Arc::new(bing::BingService),
        Arc::new(openai::OpenAIService),
    ]
}
```

---

## 5. 项目结构

```
translator/
├── Cargo.toml                      # workspace
├── README.md
├── LICENSE                         # GPL-3.0
├── .gitignore
├── .rustfmt.toml
├── .clippy.toml
│
├── crates/
│   ├── core/                       # 业务核心（无 UI、无平台依赖）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── service.rs          # trait + ServiceId + 注册表
│   │       ├── model.rs            # TranslateRequest/Result/Language
│   │       ├── error.rs            # ServiceError
│   │       ├── config.rs           # 加载/保存 ~/.config/translator/config.json
│   │       ├── secrets.rs          # keyring 包装
│   │       ├── detect.rs           # 简单启发式语言识别（备选）
│   │       └── services/
│   │           ├── mod.rs
│   │           ├── youdao.rs
│   │           ├── deepl.rs
│   │           ├── google.rs
│   │           ├── bing.rs
│   │           └── openai.rs
│   │
│   ├── platform/                   # 平台抽象 + 实现
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # pub trait SelectionMonitor
│   │       ├── position.rs         # 选区坐标 → 浮窗定位
│   │       ├── macos.rs            # objc2 + AXUI + CGEventTap
│   │       ├── windows.rs          # windows-rs + UIA
│   │       └── linux.rs            # atspi + zbus
│   │
│   └── app/                        # Tauri 集成层
│       ├── Cargo.toml
│       ├── build.rs
│       ├── tauri.conf.json
│       ├── capabilities/
│       │   └── default.json        # Tauri 2 capabilities (permissions)
│       ├── icons/
│       │   ├── 32x32.png
│       │   ├── 128x128.png
│       │   ├── 128x128@2x.png
│       │   └── icon.ico / icon.icns  # 由 tauri icon 生成
│       └── src/
│           ├── main.rs             # Tauri Builder 入口
│           ├── commands.rs         # #[tauri::command] 桥接 core+platform
│           ├── tray.rs             # 菜单栏/托盘
│           ├── state.rs            # AppState
│           └── permissions.rs      # 权限引导
│
├── ui/                             # React + Vite 前端
│   ├── package.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── postcss.config.js
│   ├── tsconfig.json
│   ├── index.html                  # settings 窗口入口
│   ├── popup.html                  # popup 浮窗入口
│   ├── public/
│   └── src/
│       ├── main.tsx                # settings 入口
│       ├── popup.tsx               # popup 入口
│       ├── popup/
│       │   ├── Popup.tsx
│       │   ├── ResultTab.tsx
│       │   ├── LoadingDots.tsx
│       │   └── popup.css
│       ├── settings/
│       │   ├── Settings.tsx
│       │   ├── sections/
│       │   │   ├── GeneralSection.tsx
│       │   │   ├── ServicesSection.tsx
│       │   │   ├── ShortcutSection.tsx
│       │   │   ├── AppearanceSection.tsx
│       │   │   └── AboutSection.tsx
│       │   └── settings.css
│       ├── components/
│       │   ├── Button.tsx
│       │   ├── Input.tsx
│       │   ├── Select.tsx
│       │   ├── Switch.tsx
│       │   └── KeyCombo.tsx
│       ├── stores/
│       │   ├── config.ts           # zustand: 配置
│       │   └── results.ts          # zustand: 当前结果
│       ├── i18n/
│       │   ├── index.ts
│       │   └── bindings/           # ts-rs 自动生成
│       └── types/                  # ts-rs 生成的 TS 类型
│
├── locales/                        # fluent 资源
│   ├── en.ftl
│   └── zh-Hans.ftl
│
├── docs/
│   ├── DESIGN.md                   # 本文件
│   ├── ARCHITECTURE.svg            # 由 fireworks-tech-graph 生成
│   ├── user-guide.md
│   └── dev-guide.md
│
├── scripts/
│   ├── format.sh
│   ├── lint.sh
│   └── generate-types.sh           # 调用 ts-rs
│
└── .github/
    └── workflows/
        ├── ci.yml                  # 三平台 cargo check + clippy + npm test
        └── release.yml             # tag → 打包 + release
```

---

## 6. 跨平台集成

### 6.1 选区文本捕获

```rust
// crates/platform/src/lib.rs

use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Rect {
    pub x: i32, pub y: i32, pub width: i32, pub height: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum SelectionError {
    #[error("accessibility permission not granted")]
    PermissionDenied,
    #[error("platform error: {0}")]
    Platform(String),
    #[error("timeout")]
    Timeout,
    #[error("no selection")]
    Empty,
}

#[async_trait]
pub trait SelectionMonitor: Send + Sync {
    /// 读取当前焦点元素（或鼠标位置元素）的选中文本
    async fn get_selected_text(&self) -> Result<Option<String>, SelectionError>;

    /// 读取选区的屏幕坐标，用于浮窗定位
    /// 实现有困难时返回 Ok(None)，调用方退回到鼠标位置
    async fn selection_bounds(&self) -> Result<Option<Rect>, SelectionError>;

    /// 鼠标当前屏幕坐标（浮窗定位的兜底方案）
    async fn cursor_position(&self) -> Result<(i32, i32), SelectionError>;
}

pub fn create() -> Box<dyn SelectionMonitor> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOSSelection::new());

    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsSelection::new());

    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxSelection::new());
}
```

#### macOS (`crates/platform/src/macos.rs`)

- 框架：AppKit + ApplicationServices（`objc2` + `objc2-app-kit` + `accessibility-sys`）
- 流程：
  1. `AXUIElementCreateSystemWide()` → `kAXSystemWideUIElement`
  2. `AXUIElementCopyAttributeValue(system, kAXFocusedUIElementAttribute, &focused)`
  3. `AXUIElementCopyAttributeValue(focused, kAXSelectedTextAttribute, &cfstring)`
  4. `CFStringGetCString(...)` 转 Rust `String`
- 选区坐标：`kAXBoundsForRangeParameterizedAttribute` + `kAXSelectedTextRangeAttribute`
- 权限：首次需要用户授权"系统设置 → 隐私与安全 → 辅助功能"
- 权限检查：`AXIsProcessTrustedWithOptions`
- 失败回退：检测到未授权时返回 `SelectionError::PermissionDenied`，前端显示引导 UI

#### Windows (`crates/platform/src/windows.rs`)

- 框架：`windows` crate (Microsoft 官方)
- 流程：
  1. `CoCreateInstance` CLSID_CUIAutomation → `IUIAutomation`
  2. `pAutomation->GetFocusedElement(&element)`
  3. `IUIAutomationTextPattern::GetSelection(&ranges)` → `IUIAutomationTextRangeArray`
  4. `range->GetText(maxLength, &bstr)`
- 选区坐标：`IUIAutomationTextRange::GetBoundingRectangles`
- 权限：默认开启，无需用户操作

#### Linux (`crates/platform/src/linux.rs`)

- 框架：`atspi` + `zbus`（D-Bus）
- 流程：
  1. `atspi::AccessibilityProxy::accessible_at_point(cursor_x, cursor_y)` → `Accessible`
  2. `proxy.get_text(&selection_i32, 0, &end_i32, &mut text)`（通过 AT-SPI2 的 Selection/Text 接口）
- 坐标：`Accessible::get_extents(CoordType::Screen)`
- 限制：GNOME 46+ / KDE Plasma 5.27+ 完整支持；其他 DE/合成器可能返回空
- Wayland：需要 xdg-desktop portal 配合（v1 标"Linux 仅 GNOME/KDE"）

### 6.2 全局快捷键

使用 Tauri 官方插件 `tauri-plugin-global-shortcut`：

```rust
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

let shortcut = Shortcut::new(
    Some(Modifiers::SUPER | Modifiers::SHIFT),
    Code::KeyD,
);

tauri::Builder::default()
    .plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    trigger_translation_flow();
                }
            })
            .build()
    )
    .setup(|app| {
        app.global_shortcut().register(shortcut)?;
        Ok(())
    });
```

- macOS：监听 `NSEvent` 系统级（无需辅助功能权限，但首次会弹"快捷键"提示）
- Windows：注册 `RegisterHotKey`
- Linux：通过 D-Bus（X11 用 XGrabKey，Wayland 用 `GlobalShortcuts` portal）

### 6.3 浮窗窗口

Tauri 窗口配置：

```json
{
  "label": "popup",
  "width": 480,
  "height": 320,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "visible": false,
  "focus": false,
  "url": "popup.html"
}
```

显示逻辑：
1. 后端在 `commands::show_popup(text, x, y)` 中：
   - 调用 `selection_monitor.get_selected_text()` 拿文本
   - 计算浮窗位置（优先选区坐标，否则鼠标位置，否则屏幕中心）
   - 调整 x/y 防止超出屏幕
   - `window.set_position(LogicalPosition::new(x, y))?; window.show()?;`
2. 前端 `Popup.tsx` 通过 Tauri IPC 拿 `TranslateResult[]`，分 Tab 渲染
3. 监听窗口 `blur` 事件：500ms 后 `window.hide()`（避免翻译点击交互时误关）
4. 用户点浮窗外区域：立即 `window.hide()`

### 6.4 系统托盘

```rust
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};

TrayIconBuilder::with_id("main")
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            open_settings_window();
        }
    })
    .build(app)?;
```

菜单项：
- 打开设置
- 立即翻译（手动触发选区读取）
- ---
- 开机启动（macOS `LaunchAtLogin` / Windows 注册表 / Linux `.desktop` autostart）
- ---
- 退出

---

## 7. 数据流

### 7.1 端到端流程

```
[用户选中文字]
       │
       ▼
[用户按下 ⌘+Shift+D]
       │
       ▼
[Tauri 拦截快捷键] ──→ commands::on_hotkey()
       │
       ▼
[platform::get_selected_text()] ──→ String
       │
       ▼
[core::translate_parallel(text, from=auto, to, services)]
       │   │
       │   ├──→ youdao.translate()   ─→ TranslateResult
       │   ├──→ deepl.translate()    ─→ TranslateResult
       │   ├──→ google.translate()   ─→ TranslateResult
       │   └──→ bing.translate()     ─→ TranslateResult
       │         (5 个服务并行，3s 超时，fail-fast 不影响其他)
       │
       ▼
[Vec<TranslateResult>]
       │
       ▼
[commands::show_popup(results, x, y)]
       │
       ▼
[popup 窗口显示]  ←── 用户操作: copy / close / switch service tab
       │
       ▼
[5s 无操作 / blur → window.hide()]
```

### 7.2 状态管理

| 状态 | 存储位置 | 同步方式 |
| --- | --- | --- |
| 配置（服务列表、API key、目标语言、快捷键） | `~/.config/translator/config.json` + 系统 Keychain | Tauri `app_data_dir()` |
| 当前浮窗结果 | 内存 | Tauri command 返回值 |
| 服务自定义设置（OpenAI 的 model/base_url） | 同上 config.json | — |
| 历史记录 | **v1 不持久化** | — |

---

## 8. 持久化与安全

### 8.1 配置文件

位置（跨平台）：

| 平台 | 路径 |
| --- | --- |
| macOS | `~/Library/Application Support/translator/config.json` |
| Windows | `%APPDATA%\translator\config.json` |
| Linux | `~/.config/translator/config.json` |

通过 `dirs::config_dir()` 取得。

格式：
```json
{
  "version": 1,
  "general": {
    "target_language": "zh-Hans",
    "default_from": "auto",
    "theme": "system",
    "show_popup_on": "hotkey"
  },
  "shortcut": "CmdOrCtrl+Shift+D",
  "services": {
    "youdao": { "enabled": true, "priority": 1 },
    "deepl":  { "enabled": true, "priority": 2 },
    "google": { "enabled": false, "priority": 3 },
    "bing":   { "enabled": false, "priority": 4 },
    "openai": { "enabled": true, "priority": 0, "base_url": "https://api.deepseek.com/v1", "model": "deepseek-chat" }
  }
}
```

### 8.2 凭据存储

API key **绝不入 JSON**。用 `keyring` crate 写入系统 Keychain：

| 平台 | 后端 |
| --- | --- |
| macOS | Keychain |
| Windows | Credential Manager |
| Linux | Secret Service (gnome-keyring / kwallet) |

服务名：`dev.translator.app`，账号名：`service:{service_id}`，密码：API key。

```rust
let entry = keyring::Entry::new("dev.translator.app", &format!("service:{}", service_id))?;
entry.set_password(api_key)?;
let key = entry.get_password()?;
```

### 8.3 网络

- 所有 HTTP 走 HTTPS，禁用明文。
- 复用单个 `reqwest::Client`（连接池）。
- 单次请求超时：5s；并行请求整体超时：8s。
- 用户隐私：本应用不上传任何数据到自有服务器；只与用户配置的翻译服务通信。

---

## 9. 国际化

使用 `fluent` 0.17 + 静态 `.ftl` 文件。

### 9.1 资源目录

- `locales/en.ftl` (英文)
- `locales/zh-Hans.ftl` (简体中文)

### 9.2 命名规范

`<scope>.<category>.<subcategory>.<element>`，全小写 + 点分 + 下划线。

示例：
```ftl
popup.title = Translation
popup.loading = Translating...
popup.copy = Copy
popup.retry = Retry
popup.error.no-selection = No text selected
popup.error.permission = Please grant accessibility permission in System Settings

settings.title = Settings
settings.services.title = Translation Services
settings.services.youdao.name = Youdao
settings.services.youdao.description = Chinese-friendly dictionary and translation
```

### 9.3 接入位置

- 全部 SwiftUI/AppKit 文本（设置面板、浮窗、托盘菜单）
- 错误提示、状态消息

---

## 10. 前端（React）

### 10.1 入口策略

Tauri 2 多窗口：

- `popup.html` → 浮窗（无边框、透明、最上层、不可聚焦）
- `index.html` → 设置窗口（标准窗口）

打包在 `dist/popup.html` 和 `dist/index.html` 两个 entry。Vite 多页面配置。

### 10.2 关键组件

#### `<Popup />` (`ui/src/popup/Popup.tsx`)

```tsx
interface Props {}
export function Popup() {
  const { results, loading, error } = useResultsStore();

  if (loading) return <LoadingDots />;
  if (error) return <ErrorView error={error} />;

  return (
    <div className="popup">
      <Tabs>
        {results.map(r => (
          <Tab key={r.service_id} name={r.service_name}>
            <p>{r.text}</p>
            {r.detected_source && <span>detected: {r.detected_source}</span>}
            <CopyButton text={r.text} />
          </Tab>
        ))}
      </Tabs>
    </div>
  );
}
```

#### `<Settings />`

- 左侧导航（General / Services / Shortcut / Appearance / About）
- 右侧表单（受控组件 + zustand）
- 保存按钮 → 调 `commands::save_config()` → 通知后端 reload

### 10.3 状态

```ts
// stores/config.ts
interface ConfigState {
  config: Config;
  load: () => Promise<void>;
  save: (config: Config) => Promise<void>;
}

// stores/results.ts
interface ResultsState {
  results: TranslateResult[];
  loading: boolean;
  error: string | null;
  setResults: (r: TranslateResult[]) => void;
  setLoading: (l: boolean) => void;
  setError: (e: string | null) => void;
  reset: () => void;
}
```

### 10.4 与后端通信

仅通过 Tauri 提供的 `invoke`：

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 主动调用
const text = await invoke<string | null>("get_selected_text");
await invoke<TranslateResult[]>("translate_text", {
  text, from: "auto", to: "zh-Hans", services: ["youdao", "deepl", "openai"]
});

// 监听事件
listen<TranslateResult[]>("translation_complete", (e) => {
  useResultsStore.getState().setResults(e.payload);
});
```

类型由 `ts-rs` 从 Rust struct 自动生成到 `ui/src/types/bindings.ts`。

---

## 11. 路线图

| 阶段 | 周 | 内容 | 退出标准 |
| --- | --- | --- | --- |
| **M0 Spike** | 1 | Tauri 2 demo (macOS)：⌘+Shift+D 拦截 + objc2 读选区 + 浮窗 + mock 翻译 | 本地能跑通端到端 |
| **M1 Core** | 3 | 5 个 service 全部实现 + TranslationService trait + config + secrets + 单元测试 + CLI demo | `cargo test` 全绿 + CLI 端到端 |
| **M2 Platform** | 2 | 三平台 SelectionMonitor 实现 + 错误/权限 UX | macOS + Win 端到端跑通；Linux GNOME 能读选区 |
| **M3 Tauri 集成** | 2 | Tauri 集成 + 浮窗 + 托盘 + 快捷键 + IPC | macOS 端到端：选词 → 浮窗 → 翻译 |
| **M4 前端 + 设置** | 1.5 | React popup + settings（动态表单） | 设置可保存/读取 |
| **M5 打包** | 0.5 | GitHub Actions 三平台 + 公证 + release | tag → 三平台 artifact |
| **合计** | **~10 周** | **v1.0 公测** | |

里程碑后休整 1 周 bug fix。

### 团队配置（建议）

- 1 名 Rust 主程（M0–M3）
- 1 名 Rust/平台工程师（M2 + M1 服务）
- 1 名前端工程师（M4）
- 0.1 名设计师（2 天画 UI）

---

## 12. 风险与缓解

| 风险 | 严重 | 缓解 |
| --- | --- | --- |
| macOS 辅助功能权限被拒 | 中 | onboarding 时明确引导，链接到系统设置；UI 显眼"未授权"提示 |
| Linux Wayland 选区拿不到 | 中 | v1 文档明确"仅 GNOME/KDE 完整支持"；提供 X11 fallback |
| 某些服务突发限流 | 中 | 并行调用 + per-service 重试 1 次 + UI 显示错误而非崩溃 |
| OpenAI-compat 用户填错 base_url | 低 | 启动时做"ping 一下 /models 端点"验证，UI 标红无效 |
| Tauri 浮窗在 macOS 不够"原生" | 低 | 接受；M3 验收时与 NSPanel 视觉对比，若差距过大改用 `tao` 原生窗口 |
| 5 个服务中部分厂商更改 API | 中 | 在 `services/*.rs` 内隔离变化；core 测试覆盖 mock HTTP 响应 |
| Keychain 在 Linux Secret Service 未运行 | 低 | 启动时检测；若未运行则 fallback 到加密文件 + 警告 |
| 公证 / 签名失败阻塞 release | 中 | 早期就跑通 CI 打包；macOS 申请 Developer ID；Windows 用 signtool |

---

## 13. 开放问题（待 Spike 后决定）

1. **浮窗位置策略**：始终跟随光标 vs. 始终在选区正下方？→ Spike 阶段对比。
2. **多服务结果排序**：并行按速度返回还是按用户 priority？→ v1 用 priority + elapsed 取最快。
3. **OpenAI streaming 是否进 v1**？→ v1 仅非流式；v2 再加（依赖 SSE 客户端）。
4. **是否支持"右键选词"作为快捷键的补充**？→ v1 仅全局热键；v2 再考虑。
5. **是否支持"替换原文"**？→ v1 不做；v2 加（需要更多平台胶水）。

---

## 14. 附录

### A. 依赖清单（最终 Cargo.lock 预期）

- `tokio = "1.41"`
- `reqwest = { version = "0.12", features = ["json", "rustls-tls"] }`
- `serde = "1.0"` / `serde_json = "1.0"`
- `async-trait = "0.1"`
- `thiserror = "1.0"` / `anyhow = "1.0"`
- `tracing = "0.1"` / `tracing-subscriber = "0.3"`
- `keyring = "3.6"`
- `fluent = "0.17"` / `unic-langid = "0.9"`
- `dirs = "5.0"`
- `anyhow = "1.0"`
- `objc2 = "0.5"` / `objc2-app-kit = "0.2"` / `objc2-foundation = "0.2"` (macOS only)
- `windows = { version = "0.58", features = ["UI_Automation"] }` (Windows only)
- `atspi = "0.22"` / `zbus = "4.0"` (Linux only)
- `tauri = { version = "2", features = ["tray-icon"] }`
- `tauri-plugin-global-shortcut = "2"`
- `tauri-plugin-tray-icon = "2"`
- `tauri-plugin-store = "2"`
- `tauri-plugin-clipboard-manager = "2"`
- `tauri-plugin-fs = "2"`
- `tauri-plugin-os = "2"`
- `ts-rs = "10"` (dev)
- `wiremock = "0.6"` (dev)

### B. 关键参考链接

- Tauri 2 文档：<https://v2.tauri.app/>
- Fluent i18n：<https://projectfluent.org/>
- Youdao OpenAPI：<https://ai.youdao.com/console/doc>
- DeepL API：<https://developers.deepl.com/docs>
- Google Cloud Translation v3：<https://cloud.google.com/translate/docs/reference/rest>
- Azure Translator：<https://learn.microsoft.com/azure/ai-services/translator/>
- macOS Accessibility：<https://developer.apple.com/documentation/applicationservices/axuielement_h>
- Windows UIA：<https://learn.microsoft.com/windows/win32/winauto/entry-uiauto-win32>
- AT-SPI2：<https://docs.gtk.org/atspi2/>

### C. 与 Easydict 项目的差异

- Easydict 是 25 万行的成熟 macOS 翻译 App，本项目是从 0 开始的全新跨平台版本
- 共享灵感（选词翻译、浮窗结果、多服务并行），不共享代码
- 本项目不重命名、不混用任何 Easydict 资源文件 / 服务端点 / API
