# 简体中文
# 前端通过 @fluent/bundle 加载。

app-name = translator
app-tagline = 跨平台选词翻译

# Common
common-loading = 加载中…
common-load-config-failed = 加载配置失败：{ $msg }

# Popup
popup-title = 翻译
popup-loading = 翻译中…
popup-empty = 未选中文字
popup-permission-denied = translator 需要获取「辅助功能」权限
popup-open-settings = 打开设置
popup-copy = 复制
popup-copied = 已复制
popup-retry = 重试
popup-close = 关闭
popup-source = 原文:
popup-detected = 检测到：{ $lang }
popup-truncated = 已截断选择：保留 { $kept } / { $original } 字符（10 万字符上限）。
popup-no-services-enabled = 尚未启用任何服务。请打开设置启用至少一个。

# Common errors
error-network = 网络错误
error-rate-limited = 请求过于频繁,请稍后再试
error-timeout = 翻译超时
error-missing-credentials = 尚未配置 API key
error-unknown = 未知错误

# Settings - sidebar
settings-nav-general = 通用
settings-nav-services = 翻译服务
settings-nav-shortcut = 快捷键
settings-nav-appearance = 外观
settings-nav-about = 关于

# Settings - general
settings-general-target-lang = 目标语言
settings-general-target-lang-hint = BCP-47 语言代码,例如 zh-Hans、en、ja
settings-general-default-from = 默认源语言
settings-general-default-from-hint = 填 auto 表示由各服务自动识别

# Settings - services
settings-services-title = 翻译服务
settings-services-enabled = 启用
settings-services-priority = 优先级(越小越靠前)
settings-services-api-key = API key
settings-services-save = 保存
settings-services-remove = 删除
settings-services-key-saved = 已保存到系统钥匙串
settings-services-key-removed = 已从系统钥匙串删除

# Settings - Youdao
settings-services-youdao-appkey = App Key
settings-services-youdao-appsecret = App Secret

# Settings - OpenAI compat
settings-services-openai-baseurl = Base URL
settings-services-openai-model = 模型
settings-services-openai-presets = 预设:OpenAI、DeepSeek、Zhipu、Ollama、OpenRouter、自定义

# Settings - shortcut
settings-shortcut-label = 全局快捷键
settings-shortcut-hint = Tauri global-shortcut 语法,例如 CmdOrCtrl+Shift+D

# Settings - appearance
settings-appearance-theme = 主题

# Settings - about
settings-about-built-with = 由 Rust、Tauri 2 和 React 构建。
