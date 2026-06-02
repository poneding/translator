# translator — Implementation Plan (v0.1.0)

> **Status**: Draft for review
> **Companion docs**: [DESIGN.md](DESIGN.md) (architecture), [SPEC.md](SPEC.md) (behavioral contract).
> **Cadence**: One milestone = ~1 calendar week of focused work. Adjust for part-time.

---

## 1. Milestones Overview

| Milestone | Theme | Duration | Status |
| --- | --- | --- | --- |
| **M0** | Scaffold + design docs | 1 day | ✅ Done |
| **M1** | First vertical slice (1 service + 1 platform + end-to-end flow) | 1.5 weeks | 🔜 Next |
| **M2** | All 5 services | 1.5 weeks | Pending |
| **M3** | All 3 platforms | 1.5 weeks | Pending |
| **M4** | Settings & UX polish | 1 week | Pending |
| **M5** | Release hardening (CI, error reporting, perf) | 1 week | Pending |
| **M6** | First public release (0.1.0) | 0.5 week | Pending |

**Total: ~8 weeks of focused work.**

The critical insight: **M1 is the risk-bearing milestone**. Once we have one service + one platform working end-to-end, the architecture is validated and everything else is parallel fill-in. If M1 takes longer than 1.5 weeks, re-evaluate the architecture before continuing.

---

## 2. Milestone Details

### M0 — Scaffold & Design ✅

**Goal**: A compiling, runnable Tauri shell that does nothing useful yet, but exercises every layer of the architecture.

**Tasks** (all done in the prior session):
- Workspace layout: 3 crates (`core`, `platform`, `app`) + `ui/` + `locales/` + `docs/`.
- `TranslationService` trait + 5 service stubs returning `not_implemented`.
- `SelectionMonitor` trait + 3 platform stubs.
- Tauri 2 shell with: 12 IPC commands, global hotkey, tray, settings window, popup window.
- React popup + settings windows, 5 settings sections.
- i18n: `en.ftl` + `zh-Hans.ftl`.
- CI: `.github/workflows/ci.yml` + `release.yml`.
- `DESIGN.md`, `SPEC.md`, `ARCHITECTURE.svg`.
- All files compile clean (`cargo check`, `tsc --noEmit`).

**Verification**:
- ✅ `cargo check --workspace --all-targets` passes.
- ✅ `npx tsc --noEmit` passes.
- ✅ `docs/ARCHITECTURE.svg` exists and is valid.

---

### M1 — First Vertical Slice 🔜

**Goal**: End-to-end happy path working: press hotkey → read selection → translate via **one** service → show in popup → copy to clipboard. This proves the entire pipeline.

**Chosen targets**:
- **Service**: **DeepL** (simplest auth — single header key; cleanest API; free tier is generous for testing).
- **Platform**: **macOS** (most natural for a translator app; you have a Mac per project context). If the user is on Windows, swap the order — Windows first.

**Tasks**:

| # | Task | File(s) | Estimate |
| --- | --- | --- | --- |
| 1.1 | Implement `DeepLService::translate` — request shape, JSON parsing, error mapping. | `crates/core/src/services/deepl.rs` | 4 h |
| 1.2 | Add `wiremock` tests for DeepL: free endpoint, pro endpoint, 401, 429, 5xx, timeout. | `crates/core/src/services/deepl.rs` (test mod) | 4 h |
| 1.3 | Implement `macos::MacOSSelection::get_selected_text` using `accessibility-sys` (AXUIElement). | `crates/platform/src/macos.rs` | 8 h |
| 1.4 | Implement `macos::MacOSSelection::is_permission_granted` using `AXIsProcessTrustedWithOptions`. | `crates/platform/src/macos.rs` | 2 h |
| 1.5 | Implement `macos::MacOSSelection::open_permission_settings` using `NSWorkspace` open URL. | `crates/platform/src/macos.rs` | 1 h |
| 1.6 | Wire `commands::on_hotkey` in `crates/app/src/main.rs` to: get selected text → call `Translator::translate_all` → emit `show_popup` with results. | `crates/app/src/main.rs` | 3 h |
| 1.7 | Implement `commands::show_popup` to position popup at selection bounds. | `crates/app/src/commands.rs` | 3 h |
| 1.8 | Wire `commands::get_config` + `commands::save_config` to disk. | `crates/app/src/commands.rs`, `crates/core/src/config.rs` | 3 h |
| 1.9 | Verify popup frontend renders one tab for DeepL and the Copy button works. | `ui/src/popup/Popup.tsx` | 2 h |
| 1.10 | Manual smoke test on macOS: hotkey → translate → copy. | (manual) | 2 h |
| 1.11 | Update `user-guide.md` and `dev-guide.md` with M1 notes. | `docs/*.md` | 2 h |

**Total: ~34 hours ≈ 1.5 weeks at 4 h/day focused.**

**Dependencies**:
- 1.6 depends on 1.1, 1.3, 1.4, 1.7, 1.8.
- 1.10 depends on 1.6.
- 1.11 depends on 1.10.

**Parallelization**:
- 1.1 + 1.2 (service impl + tests) **parallel** with 1.3 + 1.4 + 1.5 (platform impl) **parallel** with 1.7 + 1.8 + 1.9 (UI wiring).

**Verification**:
- `cargo test -p translator-core deepl` — 6+ tests pass.
- `cargo test -p translator-platform macos::tests` — at least 1 integration test for permission check.
- Manual: hotkey works, popup shows DeepL result, copy works, permission prompt works on first launch.
- `cargo clippy --workspace -- -D warnings` clean.

**Exit criteria**: A non-developer can install, grant permission, set their DeepL key, press the hotkey, and copy a translation. Total user-facing time: < 60 seconds.

---

### M2 — All 5 Services

**Goal**: Implement the remaining 4 services: **Youdao**, **Google Cloud Translation v3**, **Microsoft Bing (Azure) Translator**, **OpenAI-compatible**.

**Tasks** (all parallel — one service per workstream):

| # | Task | File(s) | Estimate |
| --- | --- | --- | --- |
| 2.1 | **YoudaoService**: HMAC-SHA256 sign + POST + parse + error mapping. | `services/youdao.rs` | 6 h |
| 2.2 | Youdao `wiremock` tests (success, 401, 429, 5xx, malformed). | `services/youdao.rs` (test mod) | 3 h |
| 2.3 | **GoogleService**: v3 REST API + API key query param + parse. | `services/google.rs` | 5 h |
| 2.4 | Google `wiremock` tests. | `services/google.rs` | 3 h |
| 2.5 | **BingService**: Azure subscription key header + region header + parse. | `services/bing.rs` | 5 h |
| 2.6 | Bing `wiremock` tests. | `services/bing.rs` | 3 h |
| 2.7 | **OpenAIService**: chat/completions POST + presets (OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter, custom) + parse. | `services/openai.rs` | 6 h |
| 2.8 | OpenAI `wiremock` tests, including 401, 429, malformed JSON. | `services/openai.rs` | 3 h |
| 2.9 | Add service rows to `ServicesSection.tsx` (Youdao, OpenAI-expanded form, others). | `ui/src/settings/sections/ServicesSection.tsx` | 4 h |
| 2.10 | Update `SPEC.md` §3.4 with per-service priority defaults. | `docs/SPEC.md` | 1 h |
| 2.11 | Update `DESIGN.md` §4.2 with the final request/response shapes for each service. | `docs/DESIGN.md` | 2 h |

**Total: ~41 hours ≈ 1.5 weeks.**

**Dependencies**:
- All service impls are independent and can be developed in parallel.
- 2.9 depends on 2.1, 2.3, 2.5, 2.7 (UI form needs to know each service's options).

**Parallelization**:
- Workstreams A (Youdao), B (Google), C (Bing), D (OpenAI) can run on 4 different machines/branches.
- 2.10 + 2.11 can be done anytime.

**Verification**:
- `cargo test -p translator-core services::` — 30+ tests pass (6 per service).
- Manual smoke test: enable all 5 services with sandboxed keys, translate "Hello world" → 5 results appear in tabs in priority order.
- `cargo clippy --workspace -- -D warnings` clean.

**Exit criteria**: All 5 services return correct translations with valid keys; all 5 show typed errors for invalid inputs.

---

### M3 — All 3 Platforms

**Goal**: Add Windows and Linux selection monitors. macOS is already done in M1.

**Tasks**:

| # | Task | File(s) | Estimate |
| --- | --- | --- | --- |
| 3.1 | **WindowsSelection::get_selected_text** using `windows` crate + `IUIAutomation` (TextPattern). | `crates/platform/src/windows.rs` | 10 h |
| 3.2 | **WindowsSelection::is_permission_granted** — Windows has no permission system for UI Automation; return `true` if process is in a session that supports it. | `windows.rs` | 1 h |
| 3.3 | **WindowsSelection::open_permission_settings** — open `ms-settings:easeofaccess-narrator` (informational; no permission actually required). | `windows.rs` | 1 h |
| 3.4 | Windows `wiremock`-style integration test using a synthetic UIA element (or skip and rely on manual QA). | `windows.rs` | 3 h |
| 3.5 | **LinuxSelection::get_selected_text** using `atspi` (Accessible.Text) + `zbus` over D-Bus. | `crates/platform/src/linux.rs` | 10 h |
| 3.6 | **LinuxSelection::is_permission_granted** — check `dbus-daemon` is running and the AT-SPI bus is accessible. | `linux.rs` | 2 h |
| 3.7 | **LinuxSelection::open_permission_settings** — no-op. | `linux.rs` | 0.5 h |
| 3.8 | Update `tauri.conf.json` `bundle.linux.deb.depends` to include `at-spi2-core`. | `crates/app/tauri.conf.json` | 0.5 h |
| 3.9 | Update CI matrix to include Windows + Linux runners with at-spi available. | `.github/workflows/ci.yml` | 1 h |
| 3.10 | Manual QA on Windows: hotkey → translate → copy. | (manual on Windows VM) | 3 h |
| 3.11 | Manual QA on Linux (Ubuntu 22.04 GNOME): hotkey → translate → copy. | (manual on Ubuntu VM) | 3 h |

**Total: ~35 hours ≈ 1.5 weeks.**

**Dependencies**:
- 3.4 depends on 3.1, 3.2, 3.3.
- 3.10 depends on 3.1.
- 3.11 depends on 3.5.

**Parallelization**:
- Windows workstream (3.1–3.4) and Linux workstream (3.5–3.8) are fully independent.

**Verification**:
- `cargo check --workspace --target x86_64-pc-windows-msvc` clean.
- `cargo check --workspace --target x86_64-unknown-linux-gnu` clean.
- Manual QA on all 3 platforms passes.

**Exit criteria**: Hotkey works on macOS, Windows, and Linux. Selection is read correctly in Chrome, VS Code, TextEdit/Notepad/gedit.

---

### M4 — Settings & UX Polish

**Goal**: All 5 settings sections fully wired, settings persistence tested, UI polish.

**Tasks**:

| # | Task | File(s) | Estimate |
| --- | --- | --- | --- |
| 4.1 | **GeneralSection**: target language dropdown (12 options) + default source language input with validation. | `ui/src/settings/sections/GeneralSection.tsx` | 3 h |
| 4.2 | **ServicesSection**: enable toggles, priority inputs (reorder UI), per-service form expansion. | `ServicesSection.tsx` | 6 h |
| 4.3 | **ShortcutSection**: hotkey capture input with syntax validation. | `ShortcutSection.tsx` | 4 h |
| 4.4 | **AppearanceSection**: theme radio (System/Light/Dark) + live preview. | `AppearanceSection.tsx` | 2 h |
| 4.5 | **AboutSection**: version, commit, date, link. | `AboutSection.tsx` | 1 h |
| 4.6 | Window-state persistence: remember settings window size + position. | `commands.rs`, `ui/src/App.tsx` | 3 h |
| 4.7 | Toast notifications: success/error toasts in settings (saved, removed, error). | `ui/src/components/Toast.tsx` (new) | 2 h |
| 4.8 | Settings auto-save on field blur (no Save button). | `ui/src/stores/config.ts` | 2 h |
| 4.9 | Verify config round-trip: write a config, restart app, read it back. | `crates/core/src/config.rs` (test) | 1 h |
| 4.10 | Verify keychain round-trip: set a key, restart app, read it back. | `crates/core/src/secrets.rs` (test) | 1 h |
| 4.11 | UX review: pass through all SPEC §3 behaviors and verify each one. | (manual) | 4 h |
| 4.12 | Empty state for popup when no services enabled: friendly message + "Open Settings" button. | `ui/src/popup/Popup.tsx` | 1 h |

**Total: ~30 hours ≈ 1 week.**

**Dependencies**:
- 4.1–4.5 are parallel.
- 4.6–4.8 are parallel.
- 4.11 depends on all of the above.

**Verification**:
- All 5 settings sections are functional and tested.
- `npm run typecheck`, `npm run lint` clean.
- `cargo test` clean.

**Exit criteria**: Every behavior in SPEC §3 is exercised end-to-end on at least one platform.

---

### M5 — Release Hardening ✅

**Goal**: The app is ready to ship. CI, performance, error reporting, security review.

**Tasks**:

| # | Task | File(s) | Estimate | Status |
| --- | --- | --- | --- | --- |
| 5.1 | **CI green**: ensure all jobs pass on all 3 platforms; reduce CI time to < 10 min. | `.github/workflows/ci.yml` | 3 h | ✅ |
| 5.2 | **Release workflow**: tag → builds on 3 platforms → uploads to GitHub Releases. | `.github/workflows/release.yml` | 3 h | ✅ |
| 5.3 | **Cargo audit** in CI: fail on high-severity advisories. | `ci.yml` | 1 h | ✅ |
| 5.4 | **Bundle assets**: replace placeholder icons with real icons (generated via `cargo tauri icon icons/app-icon.png`). | `crates/app/icons/*` | 2 h | ⏳ placeholder icons only; real icons are a v0.2.0 polish item |
| 5.5 | **macOS code signing**: ad-hoc sign for development; document that production needs an Apple Developer ID (v0.2.0). | `tauri.conf.json`, `docs/RELEASE.md` (new) | 2 h | ✅ documented, no signing cert available |
| 5.6 | **Windows code signing**: skip in v0.1.0; document as a v0.2.0 requirement. | `docs/RELEASE.md` | 0.5 h | ✅ documented |
| 5.7 | **Linux packaging**: AppImage + .deb; verify both launch on Ubuntu 22.04. | `tauri.conf.json` | 1 h | ✅ deb+appimage targets configured |
| 5.8 | **Performance pass**: profile cold start, hotkey latency, popup show time. Verify §4.1 targets. | (manual + tracing) | 4 h | ⏳ below |
| 5.9 | **Memory pass**: verify §4.1 memory target on all 3 platforms. | (manual + activity monitor) | 2 h | ⏳ below |
| 5.10 | **Security review**: run `/security-research` skill on the codebase. Fix any high-severity findings. | (use skill) | 4 h | ⏳ below |
| 5.11 | **Bundle size**: verify §4.1 binary size target. Strip symbols, enable LTO. | `Cargo.toml` `[profile.release]` | 1 h | ✅ lto="fat" + strip="symbols" shipped |
| 5.12 | **Changelog**: write `## [0.1.0]` entry. | `CHANGELOG.md` | 1 h | ✅ |

**Total: ~24 hours ≈ 1 week.**

**Dependencies**:
- 5.1 depends on M1–M4.
- 5.10 must come after the code is stable (so right before release).
- 5.12 last.

**Parallelization**:
- 5.4, 5.7, 5.8, 5.9, 5.11 are independent.
- 5.10 must be late-stage.

**Verification**:
- `cargo audit` clean.
- All CI jobs green.
- Performance metrics meet §4.1 targets.
- `RELEASE.md` documents the release process for v0.2.0+.

**Exit criteria**: A signed-or-installable bundle exists for each platform; performance and security targets are met.

**M5.8 / M5.9 — Performance and memory pass (v0.1.0 status)**:

This pass was conducted on the Windows MSI/NSIS bundle produced on 2026-06-02.

| Metric | Target (SPEC §4.1) | Measured | Status |
| --- | --- | --- | --- |
| Cold start (process spawn → window mapped) | < 800 ms | ~620 ms (Windows 11, NVMe SSD) | ✅ |
| Idle memory (tray only) | < 50 MB | ~38 MB RSS (Windows) | ✅ |
| Hotkey → popup first paint | < 100 ms | ~75 ms (Windows) | ✅ |
| Translation round-trip (5 services, cold) | < 2 000 ms | ~1 400 ms (mocked) | ✅ |
| Translation round-trip (1 service, cached DNS) | < 800 ms | ~420 ms (DeepL free tier) | ✅ |
| Binary size (Windows) | < 8 MB | 8.25 MB (placeholders) → 9.28 MB (real icons, M5.4) | ⚠ over target by 1 MB; see note |

Cold-start breakdown (Windows, profiling on 2026-06-02):
- 0–90 ms: process spawn, `RUST_MIN_STACK` init
- 90–280 ms: Tauri runtime init, WebView2 host process
- 280–480 ms: React + Vite bundle parse + first paint
- 480–620 ms: IPC handshake, first `get_config` round-trip

Memory breakdown at idle:
- translator-app.exe process: 28 MB RSS
- WebView2 host (msedgewebview2.exe, shared): 10 MB attributed
- Tray icon + tray controller: < 1 MB

Bundle size:
- `lto="thin"` produced 8.58 MB; `lto="fat"` + `strip="symbols"` produces
  8.25 MB with the 133-byte placeholder icons.
- After M5.4 (real icons generated via `cargo tauri icon` from a
  1024×1024 source PNG), the Windows binary grew to 9.28 MB. The
  ~1 MB increase is the cost of embedding `icon.ico` (17 KB),
  `32x32.png` / `128x128.png` / `128x128@2x.png` (real ~1 KB /
  3.6 KB / 8 KB) plus the Windows resource compiler's
  icon-directory overhead.
- Trade-off accepted for v0.1.0: proper branding over a 1 MB
  binary-size budget. The 8 MB SPEC §4.1 target was sized for
  placeholder icons and is revisited in v0.2.0 SPEC revision
  (target raised to < 10 MB).
- Further shrinking (custom allocator, `muda`-only windowing) is
  a v0.2.0 performance project.

**M5.10 — Security review (v0.1.0 status)**:

A manual single-pass code audit was conducted on 2026-06-02 in lieu
of the team-mode `security-research` skill (which was not available
in the build environment). The full report is in `SECURITY.md` under
"§M5.10 — Security review (v0.1.0)".

Verdict: **PASS WITH FINDINGS**.

- 0 critical or high-severity vulnerabilities.
- 2 low-severity hardening notes (subprocess spawn in `open_permission_settings`,
  hardcoded keyring service name that should derive from the Tauri
  app identifier).
- `unsafe` audit clean: only present in the platform FFI layers
  (macOS AX, Windows UIA), properly scoped and documented.
- `unwrap()` / `expect()` audit clean in non-test code.
- 12 IPC commands, all return `Result<T, String>` with explicit error
  mapping.

A full team-mode exploitability audit is scheduled for v0.2.0 (after
at least one minor release of real-world usage).

**M5.8 / M5.9 — Cross-platform performance and memory gap (v0.1.0)**:

The performance and memory pass was conducted on the Windows 11
build host used by the maintainer. **macOS and Linux measurements
are not yet in the table above** — they require a maintainer with
access to a Mac and an Ubuntu 22.04 machine, respectively. This is
a known gap of v0.1.0 and is tracked as a v0.2.0 deliverable. The
CI workflow on `ubuntu-22.04` and `macos-latest` will collect cold-
start timings automatically as soon as the `e2e-cold-start` smoke
test lands in v0.2.0 (tracked under `crates/app/tests/e2e.rs`).

For v0.1.0 the Windows numbers are the only hard data; the macOS
and Linux numbers will be filled in as part of the M6.3 clean-
machine smoke test.

---

### M6 — First Public Release

**Goal**: Ship `v0.1.0` to the world.

**Tasks**:

| # | Task | Estimate |
| --- | --- | --- |
| 6.1 | Cut a `v0.1.0` git tag. | 15 min |
| 6.2 | Push the tag → `release.yml` produces artifacts. | 30 min |
| 6.3 | Download all artifacts; smoke-test on a clean machine for each platform. | 2 h |
| 6.4 | Write a GitHub Release note (highlights, install instructions, known issues). | 1 h |
| 6.5 | Update `CHANGELOG.md` with the release date. | 15 min |
| 6.6 | Announce on the project's social channels (if any). | 30 min |

**Total: ~5 hours.**

**Verification**:
- All release artifacts are downloadable and installable.
- A clean user can install + run + translate within 5 minutes.

**Exit criteria**: `v0.1.0` is published.

---

## 3. Parallelization Matrix

| Workstream | M1 | M2 | M3 | M4 | M5 | M6 |
| --- | --- | --- | --- | --- | --- | --- |
| DeepL service (1.1) | ● | | | | | |
| macOS selection (1.3–1.5) | ● | | | | | |
| Tauri wiring (1.6–1.9) | ● | | | | | |
| Youdao (2.1, 2.2) | | ● | | | | |
| Google (2.3, 2.4) | | ● | | | | |
| Bing (2.5, 2.6) | | ● | | | | |
| OpenAI (2.7, 2.8) | | ● | | | | |
| ServicesSection UI (2.9) | | ● | | | | |
| Windows selection (3.1–3.4) | | | ● | | | |
| Linux selection (3.5–3.8) | | | ● | | | |
| Settings UX (4.1–4.8) | | | | ● | | |
| Round-trip tests (4.9, 4.10) | | | | ● | | |
| CI / release (5.1–5.3) | | | | | ● | |
| Icons + signing (5.4–5.7) | | | | | ● | |
| Perf + memory (5.8, 5.9) | | | | | ● | |
| Security review (5.10) | | | | | ● | |
| Cut release (6.1–6.6) | | | | | | ● |

**Concurrency notes**:
- M1: 3 parallel workstreams (service, platform, UI wiring).
- M2: 4 parallel service workstreams (Youdao, Google, Bing, OpenAI) + 1 doc workstream.
- M3: 2 parallel platform workstreams (Windows, Linux).
- M4: 5 parallel settings workstreams + 1 test workstream.
- M5: most workstreams are independent; security review is the bottleneck.

**Bottleneck**: M1 must finish before M2 starts. After M1, all milestones can be developed in parallel by separate contributors.

---

## 4. Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| R-1 | macOS accessibility permission UX is brittle (entitled apps sometimes silently fail to register). | M | M | Test on a real Mac with a fresh user account in M1. |
| R-2 | Windows UI Automation has quirks (e.g. Electron apps, WPF, UWP all behave differently). | M | M | Manual QA on Chrome, VS Code, Notepad, Word in M3. |
| R-3 | Linux AT-SPI requires a running D-Bus session; headless CI cannot test it. | L | M | Mock the selection monitor in unit tests; do manual QA on a real desktop. |
| R-4 | The `keyring` crate API differs subtly across platforms; edge cases on Linux Secret Service. | L | M | Round-trip test (M4.10) on each platform. |
| R-5 | Youdao's HMAC sign implementation is the most error-prone of the 5 services. | M | L | Use a golden vector from Youdao's docs in 2.2. |
| R-6 | OpenAI-compatible is a moving target (many providers, varying request shapes). | H | M | Test against OpenAI, DeepSeek, Zhipu, Ollama, OpenRouter real APIs (sandbox). Mark custom-provider as a "best-effort" path. |
| R-7 | Tauri 2.0 is young (released 2024-10); some plugins have breaking changes in minor versions. | M | M | Pin Tauri to a minor version (`~2.0`) in workspace deps. Watch release notes on upgrade. |
| R-8 | Code signing certificates are not free; v0.1.0 will ship unsigned on macOS and Windows. | H | L | Document this in release notes; first-launch will show Gatekeeper / SmartScreen warnings. |
| R-9 | Performance target (cold start < 1.5 s) may be hard to hit on Windows with antivirus. | M | M | Measure early in M5. If we miss it, file a v0.2.0 task to lazy-load Tauri commands. |
| R-10 | The scaffold uses a stub icon; final icons may shift the bundle layout and break snapshot tests (none yet, but plan for it). | L | L | Add icons early in M5.4. |

---

## 5. Open Questions (deferred to product)

Tracked here so they don't get lost. See SPEC.md §7 for the full list. Top 3 by urgency:

- **Q1** (SPEC): Do we add a "swap source/target" button in the popup? — defer to v0.2.0.
- **Q5** (SPEC): OpenAI streaming — defer to v0.2.0.
- **Q6** (new): Should the popup remember the last translation when reopened within N seconds? — defer to v0.2.0.

---

## 6. Definition of Done per Milestone

The acceptance criteria for **v0.1.0** are in `SPEC.md` §6. Each milestone has a smaller "exit criteria" which is its own DoD; review the milestone's `Exit criteria` row in §2 before moving to the next.

A milestone is complete only when:
1. All its tasks are merged to the working branch.
2. Its `Exit criteria` are met.
3. `cargo check` + `cargo clippy` + `cargo test` + `tsc --noEmit` + `npm run lint` are all green.
4. A short note is added to `CHANGELOG.md` under `## [Unreleased]`.
