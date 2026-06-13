# 简体中文
# 前端通过 @fluent/bundle 加载。

app-name = Translator
app-tagline = 跨平台选词翻译

# Common
common-loading = 加载中…
common-load-config-failed = 加载配置失败：{ $msg }

# Languages
lang-auto = 自动
lang-en = 英语
lang-zh-hans = 简体中文
lang-zh-hant = 繁体中文
lang-ja = 日语
lang-ko = 韩语
lang-fr = 法语
lang-de = 德语
lang-es = 西班牙语
lang-ru = 俄语
lang-pt = 葡萄牙语
lang-it = 意大利语
lang-ar = 阿拉伯语

# Main window
main-subtitle = 快速翻译文本、剪贴板和选中文字
main-open-settings = 设置
main-source-language = 源语言
main-target-language = 目标语言
main-input-placeholder = 在这里输入或粘贴文本...
main-translate = 翻译
main-translating = 翻译中...
main-clipboard-translate = 翻译剪贴板
main-clear = 清空
main-enabled-services = 已启用 { $count } 个服务
main-error-empty = 请输入要翻译的文本。
main-results-empty = 翻译结果会显示在这里。
main-history = 历史记录
main-clear-history = 清空
main-history-empty = 成功的翻译会保存在这里。
main-pin-window = 固定窗口
main-unpin-window = 取消固定窗口
main-refresh = 刷新
main-refresh-all = 重新翻译
main-refresh-service = 刷新此服务
main-error-clipboard = 无法读取剪贴板：{ $msg }

# Main window shared strings
main-window-title = 翻译
main-window-loading = 翻译中…
main-window-empty-selection = 未选中文字
main-permission-denied = Translator 需要获取「辅助功能」权限
main-macos-signature-error = 此 macOS 版本未使用稳定身份签名。请安装已签名版本，然后在「辅助功能」中重新启用 Translator。
common-open-settings = 打开设置
common-copy = 复制
common-copied = 已复制
main-retry = 重试
common-close = 关闭
main-source-label = 原文:
main-detected-language = 检测到：{ $lang }
main-selection-truncated = 已截断选择：保留 { $kept } / { $original } 字符（10 万字符上限）。
main-no-services-enabled = 尚未启用任何服务。请打开设置启用至少一个。
main-play-audio = 播放发音

# Common errors
error-network = 网络错误
error-rate-limited = 请求过于频繁,请稍后再试
error-timeout = 翻译超时
error-missing-credentials = 尚未配置 API key
error-unknown = 未知错误

# Settings - sidebar
settings-back-main = 返回主界面
settings-nav-general = 通用
settings-nav-proxy = 代理
settings-nav-services = 翻译服务
settings-nav-shortcut = 快捷键
settings-nav-appearance = 外观
settings-nav-update = 更新
settings-nav-about = 关于

# Settings - general
settings-general-target-lang = 目标语言
settings-general-target-lang-hint = 从常用目标语言中选择，或在下方输入自定义 BCP-47 值。
settings-general-default-from = 默认源语言
settings-general-default-from-hint = 填 auto 表示由各服务自动识别
settings-general-first-language = 第一偏好语言
settings-general-second-language = 第二偏好语言
settings-general-preferred-languages-hint = 如果原文匹配其中一个偏好语言，会自动翻译到另一个；否则翻译到第一偏好语言。
settings-general-custom-bcp47 = 自定义 BCP-47 值
settings-general-custom-target-aria = 自定义 BCP-47 目标语言
settings-general-first-language-custom-aria = 自定义 BCP-47 第一偏好语言
settings-general-second-language-custom-aria = 自定义 BCP-47 第二偏好语言
settings-general-default-from-aria = 默认源语言，auto 或 BCP-47
settings-general-invalid-bcp47 = 无效的 BCP-47 代码，例如 zh-Hans 或 en-US。
settings-general-invalid-source = 使用 auto 或 BCP-47 代码，例如 en 或 zh-Hans。
settings-general-duplicate-language = 两个偏好语言必须不同。
settings-general-auto-copy = 自动复制第一个成功结果
settings-general-clipboard-hotkey = 快捷键打开且无选中文本时使用剪贴板文本
settings-general-launch-at-startup = 登录时启动
settings-general-window-position = 打开窗口时默认显示位置
settings-general-window-position-remember = 记忆上次位置
settings-general-window-position-right = 屏幕右上角
settings-general-window-position-center = 居中显示
settings-general-window-position-mouse = 跟随鼠标
settings-general-use-proxy = 翻译请求使用 HTTP 代理
settings-general-proxy-url = 代理 URL

# Settings - services
settings-services-title = 翻译服务
settings-services-enabled = 启用
settings-services-priority = 优先级(越小越靠前)
settings-services-priority-aria = { $service } 优先级
settings-services-api-key = API key
settings-services-api-key-aria = { $service } API key
settings-services-save = 保存
settings-services-remove = 删除
settings-services-key-saved = 已保存
settings-services-key-removed = 已删除
settings-services-keychain-update-failed = 更新系统钥匙串失败
settings-services-saved = 已保存
settings-services-enable-aria = 启用 { $service }
settings-services-drag = 拖拽排序
settings-services-drag-aria = 拖拽排序 { $service }
settings-services-toggle-panel = 展开或收起设置
settings-services-toggle-panel-aria = 展开或收起 { $service } 设置
settings-services-status-configured = 已配置
settings-services-status-builtin = 内置可用，无需配置 key
settings-services-status-missing = 缺少凭据
settings-services-status-keychain-error = 钥匙串错误
settings-services-status-checking = 检查中...
settings-services-save-key-aria = 将 { $service } API key 保存到系统钥匙串
settings-services-remove-key-aria = 从系统钥匙串删除 { $service } API key
settings-services-base-url = Base URL
settings-services-base-url-aria = { $service } Base URL
settings-services-youdao-description = 内置网页翻译可用，可选配置 OpenAPI 凭据。
settings-services-deepl-description = 内置网页翻译可用，可选配置官方 API key。
settings-services-google-description = 内置网页翻译可用，可选配置 Cloud v3 凭据。
settings-services-bing-description = 内置网页翻译可用，可选配置 Azure key。
settings-services-openai-description = 任意 OpenAI 风格的聊天补全端点。

# Settings - Youdao
settings-services-youdao-appkey = App Key
settings-services-youdao-appsecret = App Secret

# Settings - Google
settings-services-google-project-id = GCP Project ID

# Settings - OpenAI compat
settings-services-openai-baseurl = Base URL
settings-services-openai-model = 模型
settings-services-openai-model-aria = OpenAI 兼容模型名称
settings-services-openai-presets = 预设:OpenAI、DeepSeek、Zhipu、Ollama、OpenRouter、自定义
settings-services-openai-presets-label = 预设
settings-services-openai-preset-openai = OpenAI
settings-services-openai-preset-deepseek = DeepSeek
settings-services-openai-preset-zhipu = Zhipu
settings-services-openai-preset-ollama = Ollama
settings-services-openai-preset-openrouter = OpenRouter
settings-services-openai-preset-custom = 自定义

# Settings - shortcut
settings-shortcut-label = 全局快捷键
settings-shortcut-hint = 点击录制后，直接按下想使用的快捷键组合。
settings-shortcut-aria = 全局快捷键，Tauri global-shortcut 语法
settings-shortcut-invalid = 语法无效。请使用“<修饰键>+<按键>”，例如“CmdOrCtrl+Shift+D”或“Alt+T”。
settings-shortcut-registration-denied = 系统拒绝注册快捷键，可能是快捷键冲突。下次启动时会恢复默认快捷键。
settings-shortcut-record = 录制
settings-shortcut-recording = 请按键...

# Settings - appearance
settings-appearance-theme = 主题
settings-appearance-theme-system = 跟随系统
settings-appearance-theme-light = 浅色
settings-appearance-theme-dark = 深色
settings-appearance-theme-aria = 主题：{ $theme }
settings-appearance-language = 应用语言
settings-appearance-language-system = 跟随系统

# Settings - update
settings-update-check-startup = 启动时检查更新
settings-update-allow-beta = 允许 Beta 版本
settings-update-check = 检查更新
settings-update-install = 下载并安装
settings-update-restart = 重新启动
settings-update-status-idle = 尚未检查更新。
settings-update-status-checking = 正在检查更新...
settings-update-status-up-to-date = 当前已是最新版本。
settings-update-status-available = 发现可用更新。
settings-update-status-installing = 正在下载并安装更新...
settings-update-status-installing-progress = 正在下载更新：{ $downloaded } KB / { $total } KB
settings-update-status-installed = 更新已安装。请重启应用完成更新。
settings-update-status-failed = 更新检查失败：{ $msg }
settings-update-version = 版本 { $version }（{ $channel }）
settings-update-date = 发布时间：{ $date }

# Settings - about
settings-about-built-with = 由 Rust、Tauri 2 和 React 构建。
settings-about-version-line = v{ $version } - 跨平台选词翻译。
settings-about-commit = 提交：
settings-about-built = 构建：
settings-about-source = 源码：
