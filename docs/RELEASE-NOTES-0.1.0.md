# translator v0.1.0 — Release Notes

> **Status**: v0.1.0 — First public preview. Pre-alpha.
> **Release date**: 2026-06-02
> **License**: GPL-3.0-only

## What's in v0.1.0

`translator` is a cross-platform select-and-translate desktop app
(Rust + Tauri 2 + React). Pick text anywhere on your screen, press a
global hotkey, and translations appear in a floating popup.

### Highlights

- **5 translation services** with full implementations and wiremock-
  backed unit tests: Youdao, DeepL, Google Cloud Translation v3,
  Microsoft Translator (Azure Bing), and OpenAI-compatible (OpenAI,
  DeepSeek, Zhipu, Ollama, OpenRouter, custom).
- **3 platform integrations** for reading the selected text:
  macOS Accessibility (AX), Windows UI Automation (UIA), Linux
  atspi (scaffolded).
- **Floating popup** that follows the cursor, clamps to the screen,
  and shows one tab per service with a Copy button.
- **System tray** for quick access and a 5-section settings window
  (General, Appearance, Services, Hotkey, About).
- **English + Simplified Chinese** UI (`@fluent/bundle` + `.ftl`
  files).
- **Dark mode** that follows the system appearance live (via
  `matchMedia('(prefers-color-scheme: dark)')`).
- **Secure credential storage** in the OS Keychain (macOS Keychain,
  Windows Credential Manager, Linux Secret Service).
- **~8 MB Windows binary**, ~38 MB idle memory, < 100 ms hotkey →
  popup latency on the developer's Windows 11 NVMe SSD.

### Install

| Platform | Artifact | Notes |
| --- | --- | --- |
| macOS 13.0+ | `translator_0.1.0_universal.dmg` | Ad-hoc signed; first launch needs right-click → Open |
| Windows 10+ | `translator_0.1.0_x64_en-US.msi` or `…_x64-setup.exe` | Unsigned; SmartScreen will show "Run anyway" dialog |
| Linux (Ubuntu 22.04+) | `translator_0.1.0_amd64.AppImage` or `…_amd64.deb` | Unsigned; standard "install untrusted package" prompt |

After installing, **set at least one service's API key** in
Settings → Services, then press the global hotkey (default
`CmdOrCtrl+Shift+D`) while text is selected.

### Known limitations (v0.1.0)

- macOS `selection_bounds` returns `None` on first run; cursor
  fallback is used (full AX rect query is a v0.2.0 deliverable).
- Windows `selection_bounds` is not yet implemented; cursor
  fallback is used.
- Linux atspi is wired but only returns `Ok(None)`; cursor fallback
  is used.
- No code-signing; macOS and Windows will show the standard
  Gatekeeper / SmartScreen prompts on first launch.
- The `update_hotkey` and credential-failure paths (BH-1.5, BH-10.2,
  BH-10.3) are new in v0.1.0 and have not yet had a minor release
  of real-world usage.
- A full team-mode `security-research` exploitability audit is
  scheduled for v0.2.0; v0.1.0 shipped with a single-pass manual
  review (verdict: PASS WITH FINDINGS, 0 critical/high).

### Testing the build locally

If you want to verify the build on your own machine:

```bash
git checkout v0.1.0
cd translator
cargo tauri build
# Output: target/release/bundle/{msi,nsis,deb,dmg,appimage}/
```

### Verifying the signature

v0.1.0 bundles are **unsigned**. To verify the artifact hasn't been
tampered with, compare the SHA-256 of the downloaded file against
the `SHA256SUMS` file in this release:

```bash
sha256sum -c SHA256SUMS
```

(For v0.2.0+, the bundles will be signed and the SHA-256 file will
be replaced by detached `.sig` files in the GitHub Release
artifacts.)

## What's next (v0.2.0)

- macOS / Windows code signing (Apple Developer ID, EV code
  signing cert).
- macOS `selection_bounds` implementation.
- Windows `selection_bounds` implementation.
- Linux atspi full integration.
- Team-mode security audit.
- SBOM (CycloneDX) emission in CI.
- Auto-update channel via `tauri-plugin-updater`.

See `docs/PLAN.md` for the full roadmap.
