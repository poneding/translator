# English strings for translator
# Loaded by the React frontend via @fluent/bundle.

app-name = translator
app-tagline = Cross-platform select-and-translate

# Common
common-loading = Loading…
common-load-config-failed = Failed to load config: { $msg }

# Popup
popup-title = Translation
popup-loading = Translating…
popup-empty = No text selected
popup-permission-denied = translator needs the Accessibility permission
popup-open-settings = Open Settings
popup-copy = Copy
popup-copied = Copied
popup-retry = Retry
popup-close = Close
popup-source = source:
popup-detected = detected: { $lang }
popup-truncated = Selection truncated: { $kept } of { $original } characters kept (100 000-char limit).
popup-no-services-enabled = No services enabled. Open Settings to enable at least one.

# Common errors
error-network = Network error
error-rate-limited = Rate limited, try again later
error-timeout = Translation timed out
error-missing-credentials = API key not configured
error-unknown = Unknown error

# Settings - sidebar
settings-nav-general = General
settings-nav-services = Services
settings-nav-shortcut = Hotkey
settings-nav-appearance = Appearance
settings-nav-about = About

# Settings - general
settings-general-target-lang = Target language
settings-general-target-lang-hint = BCP-47 code, e.g. zh-Hans, en, ja
settings-general-default-from = Default source language
settings-general-default-from-hint = Use "auto" to let services detect

# Settings - services
settings-services-title = Translation Services
settings-services-enabled = Enabled
settings-services-priority = Priority (lower = shown first)
settings-services-api-key = API key
settings-services-save = Save
settings-services-remove = Remove
settings-services-key-saved = Saved to OS Keychain
settings-services-key-removed = Removed from OS Keychain

# Settings - Youdao
settings-services-youdao-appkey = App Key
settings-services-youdao-appsecret = App Secret

# Settings - OpenAI compat
settings-services-openai-baseurl = Base URL
settings-services-openai-model = Model
settings-services-openai-presets = Presets: OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter, custom

# Settings - shortcut
settings-shortcut-label = Global hotkey
settings-shortcut-hint = Tauri global-shortcut syntax (e.g. CmdOrCtrl+Shift+D)

# Settings - appearance
settings-appearance-theme = Theme

# Settings - about
settings-about-built-with = Built with Rust, Tauri 2, and React.
