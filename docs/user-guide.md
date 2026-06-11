# translator user guide

> Cross-platform select-and-translate. Pick text anywhere, press a hotkey, get translations in the main window.

## Install

| Platform | Where to get it |
| --- | --- |
| macOS | `brew install --cask translator` (when published) or download the `.dmg` from [Releases]( https://github.com/poneding/translator/releases) |
| Windows | Download the `.msi` from [Releases]( https://github.com/poneding/translator/releases) |
| Linux | Download the `.AppImage` or `.deb` from [Releases]( https://github.com/poneding/translator/releases) |

## First run

### 1. Grant the "Accessibility" permission (macOS only)

macOS requires every screen-reader-style app to be granted the **Accessibility** permission. translator will prompt you the first time you press the hotkey; if you said "no" or the prompt didn't appear:

1. Open **System Settings → Privacy & Security → Accessibility**
2. Toggle **translator** on
3. Restart the app

Windows and Linux do not require any special permission.

### 2. Configure a translation service

1. Click the translator tray/menubar icon → **Settings**
2. Go to **Services** and enable at least one provider
3. Enter your API key (or appKey/appSecret for Youdao). Keys are stored in the OS Keychain — never in plain text on disk.
4. For **OpenAI Compatible**, set the **Base URL** and **Model** for your provider of choice (OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter, …).

### 3. Press the hotkey

Default: **Cmd+T** on macOS, **Alt+T** elsewhere.

1. Select any text in any app
2. Press the hotkey
3. The main translator window opens, fills the source text, and starts translation
4. Click **Copy** to put a result on the clipboard; press **Esc** to hide the window

If no text is selected, Settings -> General can enable **Use clipboard text
when hotkey opens with no selection**. The app reads the clipboard only when
the hotkey is pressed; it does not monitor the clipboard continuously.

## Main window

- **Pin** in the titlebar keeps the translator window always on top and
  persists across restarts.
- The source editor grows with your text. Its bottom toolbar always contains
  source audio, copy, clear, and translate.
- The row below the editor shows the detected source short code, the target
  language short-code dropdown, and the enabled service count with service
  logos.
- Each result card has its own refresh button on hover/focus, so one service
  can be retried without replacing the others.
- Source audio speaks the source text. Result audio speaks the translated text.

## Supported services

| Service | Free tier | Auth |
| --- | --- | --- |
| **Youdao (有道)** | 1M chars/month | appKey + appSecret |
| **DeepL** | 500K chars/month | Auth key |
| **Google Translate** | $300 credit (90 days) for new GCP accounts | API key |
| **Microsoft Bing** | Free Azure tier | Subscription key + region |
| **OpenAI Compatible** | Pay-as-you-go (or local Ollama) | base_url + api_key + model |

## Changing the hotkey

Settings → **Hotkey** → edit the field. Use the Tauri global-shortcut syntax, e.g. `CmdOrCtrl+Shift+D`, `Alt+T`, `Super+E`.

## Appearance and language

Settings -> **Appearance** controls the theme and app language. App language
can follow the system or use any common translation language listed in the
dropdown. Changes apply without restarting the app.

## Updates

Settings -> **Update** controls automatic update checks, beta eligibility, and
manual checks. Startup checks run asynchronously and do not block the tray,
hotkey, or first window paint. Updates are never installed silently; click
**Download and install** when an available update is shown.

## Privacy

- translator runs entirely on your machine. It does not phone home.
- The only network traffic is between your machine and the translation services you have configured.
- Your text is sent to **every enabled service** in parallel. If this concerns you, enable only the services you trust.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| Main window says "Accessibility permission" | macOS: grant it in System Settings (see above). |
| Hotkey opens an empty source editor | Select text before pressing the hotkey, or enable clipboard fallback in Settings -> General. |
| DeepL returns 403 | Free and Pro keys use different endpoints; the service auto-selects, but if you have a Pro key, set `endpoint: "pro"` in options. |
| Google returns 403 | Make sure Cloud Translation API is enabled for your GCP project. |
| OpenAI returns 401 | Verify the API key and the base URL. |
