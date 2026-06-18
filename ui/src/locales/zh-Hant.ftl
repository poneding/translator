# English strings for translator
# Loaded by the React frontend via @fluent/bundle.

app-name = Translator
app-tagline = Cross-platform select-and-translate

# Common
common-loading = Loading…
common-load-config-failed = Failed to load config: { $msg }

# Languages
lang-auto = Auto
lang-en = English
lang-zh-hans = Simplified Chinese
lang-zh-hant = Traditional Chinese
lang-ja = Japanese
lang-ko = Korean
lang-fr = French
lang-de = German
lang-es = Spanish
lang-ru = Russian
lang-pt = Portuguese
lang-it = Italian
lang-ar = Arabic

# Main window
main-subtitle = Fast text, clipboard, and selection translation
main-open-settings = Settings
main-source-language = Source
main-target-language = Target
main-input-placeholder = Type or paste text here...
main-translate = Translate
main-translating = Translating...
main-clipboard-translate = Clipboard Translate
main-clear = Clear
main-enabled-services = { $count } services enabled
main-error-empty = Enter text to translate.
main-results-empty = Translation results will appear here.
main-history = History
main-clear-history = Clear
main-history-empty = Successful translations are saved here.
main-pin-window = Pin window
main-unpin-window = Unpin window
main-refresh = Refresh
main-refresh-all = Refresh translations
main-refresh-service = Refresh this service
main-error-clipboard = Could not read clipboard: { $msg }

# Main window shared strings
main-window-title = Translation
main-window-loading = Translating…
main-window-empty-selection = No text selected
main-permission-denied = Translator needs the Accessibility permission
main-macos-signature-error = 此 macOS 版本未使用穩定身分簽名。請安裝已簽名版本，然後在「輔助使用」中重新啟用 Translator。
common-open-settings = Open Settings
common-copy = Copy
common-copied = Copied
main-retry = Retry
common-close = Close
main-source-label = source:
main-detected-language = detected: { $lang }
main-selection-truncated = Selection truncated: { $kept } of { $original } characters kept (100 000-char limit).
main-no-services-enabled = No services enabled. Open Settings to enable at least one.
main-play-audio = Play audio

# Common errors
error-network = Network error
error-rate-limited = Rate limited, try again later
error-timeout = Translation timed out
error-missing-credentials = API key not configured
error-unknown = Unknown error

# Settings - sidebar
settings-back-main = Back to main
settings-nav-general = General
settings-nav-proxy = Proxy
settings-nav-services = Services
settings-nav-shortcut = Hotkey
settings-nav-appearance = Appearance
settings-nav-update = Update
settings-nav-about = About

# Settings - general
settings-general-target-lang = Target language
settings-general-target-lang-hint = Pick from the common target languages, or enter a custom BCP-47 value below.
settings-general-default-from = Default source language
settings-general-default-from-hint = Use "auto" to let services detect
settings-general-first-language = First preferred language
settings-general-second-language = Second preferred language
settings-general-preferred-languages-hint = Translations automatically target the other preferred language when the source matches one of them. Otherwise they target the first preferred language.
settings-general-custom-bcp47 = Custom BCP-47 value
settings-general-custom-target-aria = Custom BCP-47 target language
settings-general-first-language-custom-aria = Custom BCP-47 first preferred language
settings-general-second-language-custom-aria = Custom BCP-47 second preferred language
settings-general-default-from-aria = Default source language, auto or BCP-47
settings-general-invalid-bcp47 = Invalid BCP-47 code, for example zh-Hans or en-US.
settings-general-invalid-source = Use "auto" or a BCP-47 code, for example en or zh-Hans.
settings-general-duplicate-language = The two preferred languages must be different.
settings-general-auto-copy = Auto-copy first successful result
settings-general-clipboard-hotkey = Use clipboard text when hotkey opens with no selection
settings-general-launch-at-startup = Launch at startup
settings-general-show-menu-bar-icon = Show menu bar icon
settings-general-window-position = 開啟視窗時預設顯示位置
settings-general-window-position-remember = 記住上次位置
settings-general-window-position-right = 螢幕右上角
settings-general-window-position-center = 置中顯示
settings-general-window-position-mouse = 跟隨滑鼠
settings-general-use-proxy = Use HTTP proxy for translation requests
settings-general-proxy-url = Proxy URL

# Settings - services
settings-services-title = Translation Services
settings-services-enabled = Enabled
settings-services-priority = Priority (lower = shown first)
settings-services-priority-aria = { $service } priority
settings-services-api-key = API key
settings-services-api-key-aria = { $service } API key
settings-services-save = Save
settings-services-remove = Remove
settings-services-key-saved = Saved
settings-services-key-removed = Removed
settings-services-keychain-update-failed = Failed to update OS Keychain
settings-services-saved = Saved
settings-services-enable-aria = Enable { $service }
settings-services-drag = Reorder
settings-services-drag-aria = Reorder { $service }
settings-services-toggle-panel = Toggle settings
settings-services-toggle-panel-aria = Toggle { $service } settings
settings-services-status-configured = Configured
settings-services-status-builtin = Built-in, no key required
settings-services-status-missing = Missing credential
settings-services-status-keychain-error = Keychain error
settings-services-status-checking = Checking...
settings-services-save-key-aria = Save { $service } API key to OS Keychain
settings-services-remove-key-aria = Remove { $service } API key from OS Keychain
settings-services-base-url = Base URL
settings-services-base-url-aria = { $service } Base URL
settings-services-youdao-description = Built-in web translation; optional OpenAPI credentials.
settings-services-deepl-description = Built-in web translation; optional official API key.
settings-services-google-description = Built-in web translation; optional Cloud v3 credentials.
settings-services-bing-description = Built-in web translation; optional Azure key.
settings-services-openai-description = Any OpenAI-style chat completion endpoint.

# Settings - Youdao
settings-services-youdao-appkey = App Key
settings-services-youdao-appsecret = App Secret

# Settings - Google
settings-services-google-project-id = GCP Project ID

# Settings - OpenAI compat
settings-services-openai-baseurl = Base URL
settings-services-openai-model = Model
settings-services-openai-model-aria = OpenAI-compatible model name
settings-services-openai-presets = Presets: OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter, custom
settings-services-openai-presets-label = Presets
settings-services-openai-preset-openai = OpenAI
settings-services-openai-preset-deepseek = DeepSeek
settings-services-openai-preset-zhipu = Zhipu
settings-services-openai-preset-ollama = Ollama
settings-services-openai-preset-openrouter = OpenRouter
settings-services-openai-preset-custom = Custom

# Settings - shortcut
settings-shortcut-label = Global hotkey
settings-shortcut-hint = Click record, then press the shortcut you want to use.
settings-shortcut-aria = Global hotkey, Tauri global-shortcut syntax
settings-shortcut-invalid = Invalid syntax. Use "<modifiers>+<key>", for example "CmdOrCtrl+Shift+D" or "Alt+T".
settings-shortcut-registration-denied = The OS denied hotkey registration, likely due to a conflict. The default shortcut will be restored on next launch.
settings-shortcut-record = Record
settings-shortcut-recording = Press keys...

# Settings - appearance
settings-appearance-theme = Theme
settings-appearance-theme-system = System
settings-appearance-theme-light = Light
settings-appearance-theme-dark = Dark
settings-appearance-theme-aria = Theme: { $theme }
settings-appearance-language = App language
settings-appearance-language-system = System

# Settings - update
settings-update-check-startup = Check for updates on startup
settings-update-allow-beta = Allow beta versions
settings-update-check = Check for updates
settings-update-install = Download and install
settings-update-restart = 重新啟動
settings-update-status-idle = No update check has run yet.
settings-update-status-checking = Checking for updates...
settings-update-status-up-to-date = You are up to date.
settings-update-status-available = An update is available.
settings-update-status-installing = Downloading and installing update...
settings-update-status-installing-progress = Downloading update: { $downloaded } KB / { $total } KB
settings-update-status-installed = Update installed. Restart the app to finish.
settings-update-status-failed = Update check failed: { $msg }
settings-update-version = Version { $version } ({ $channel })
settings-update-date = Released { $date }

# Settings - about
settings-about-built-with = Built with Rust, Tauri 2, and React.
settings-about-version-line = v{ $version } - cross-platform select-and-translate.
settings-about-commit = Commit:
settings-about-built = Built:
settings-about-source = Source:
