# translator - Implementation Plan (v0.2.0 draft)

> Status: Draft for next-stage development
> Companion spec: [SPEC-v0.2.0.md](SPEC-v0.2.0.md)
> Planning date: 2026-06-09

---

## 1. Strategy

v0.2.0 should be implemented as a main-window migration, not as isolated UI
polish. The highest-risk change is replacing the popup hotkey flow with the
main window while preserving fast translation and selection/clipboard behavior.

Recommended order:

1. Add config/schema and shared language foundations.
2. Move hotkey translation to the main window and remove popup dependencies.
3. Redesign the main source editor and result cards.
4. Fix audio semantics.
5. Add updater support.
6. Finish localization, QA, and docs.

Do not start by deleting popup files. First make the main-window hotkey flow
work end to end, then remove dead popup code.

---

## 2. Milestones

| Milestone | Theme | Estimate | Exit criteria |
| --- | --- | ---: | --- |
| M1 | Config, language registry, i18n foundation | 1-2 days | New config defaults/migrations and shared language registry are tested. |
| M2 | Main-window hotkey flow, pin, titlebar | 2-3 days | Hotkey opens main, selection/clipboard fallback works, Pin and Escape work. |
| M3 | Source editor and result UX redesign | 3-4 days | Toolbar, service row, refreshes, compact cards match spec. |
| M4 | Audio semantics and ghost controls | 1-2 days | Source audio plays source text; result audio plays translated text. |
| M5 | Update settings and updater integration | 3-5 days | Manual and startup update checks work against release metadata. |
| M6 | Localization, cleanup, QA, docs | 2-3 days | CI, release checks, and manual QA pass. |

Total: roughly 12-19 focused engineering days depending on updater signing and localization effort.

---

## 3. M1 - Config, Language Registry, i18n

**Goal**: Create stable shared primitives before UI work spreads duplicated state.

| # | Task | Files | Notes |
| --- | --- | --- | --- |
| 1.1 | Add config fields and defaults. | `crates/core/src/config.rs`, `ui/src/types/bindings.ts` | Add `general.app_language`, `general.auto_translate_clipboard_on_hotkey`, `window.always_on_top`, `updates.check_on_startup`, `updates.allow_beta`. |
| 1.2 | Add migration tests for v0.1.0 configs. | `crates/core/src/config.rs` | Ensure missing fields normalize to defaults without changing services/history. |
| 1.3 | Create shared language registry. | `ui/src/i18n/languages.ts` or `ui/src/languages.ts` | One source for code, label key, fallback label, flag, locale availability, provider code helpers if needed. |
| 1.4 | Replace duplicate language lists. | `ui/src/App.tsx`, `ui/src/settings/sections/GeneralSection.tsx` | Main and settings must use the same registry; include pt/it/ar in main. |
| 1.5 | Upgrade i18n runtime to reactive locale state. | `ui/src/i18n/index.ts`, new hook/provider if needed | `setLocale()` must trigger re-render across the app. |
| 1.6 | Add locale files for all app language options. | `ui/src/locales/*.ftl` | Required locales: en, zh-Hans, zh-Hant, ja, ko, fr, de, es, ru, pt, it, ar. |
| 1.7 | Add missing-key check. | `scripts/`, `ui/package.json`, CI | Script compares all locale files against English keys and fails on missing keys. |

**Verification**

- `cargo test -p translator-core config`
- `npm run typecheck`
- Locale key checker passes.

**Dependency notes**

- M2 titlebar and M3 dropdowns should wait for 1.3.
- M5 update settings can start after 1.1.

---

## 4. M2 - Main Window, Hotkey, Titlebar

**Goal**: Replace popup-first hotkey behavior with main-window-first behavior.

| # | Task | Files | Notes |
| --- | --- | --- | --- |
| 2.1 | Add logo asset/component for titlebar. | `ui/public`, `ui/src/components` or existing icon assets | Use existing app icon rendered at titlebar size. |
| 2.2 | Redesign titlebar structure. | `ui/src/App.tsx`, `ui/src/app.css` | Pin at left, centered logo + app name, no subtitle. Preserve drag regions. |
| 2.3 | Implement Pin toggle. | `ui/src/App.tsx`, `ui/src/ipc/commands.ts`, `crates/app/src/commands.rs` | Use Tauri always-on-top APIs; persist in config. |
| 2.4 | Apply persisted pin on startup/config load. | `crates/app/src/main.rs`, `ui/src/hooks` | Ensure main window starts pinned when configured. |
| 2.5 | Add Escape-to-close for main window. | `ui/src/App.tsx` | Hide, do not quit; avoid closing when menus/dialogs consume Escape. |
| 2.6 | Change hotkey backend to open main window. | `crates/app/src/commands.rs`, `crates/app/src/main.rs` | Selection read happens before focus changes. Emit event to main with source payload. |
| 2.7 | Add clipboard fallback in hotkey flow. | `crates/app/src/commands.rs`, `ui/src/ipc/commands.ts` | Selection wins; clipboard is read only if config enables it and selection is empty. |
| 2.8 | Add frontend event handler for hotkey source payload. | `ui/src/App.tsx`, `ui/src/ipc/commands.ts` | Fill source, show main view, start translation with request id. |
| 2.9 | Decommission popup flow after main flow works. | `crates/app/tauri.conf.json`, `ui/vite.config.ts`, `ui/src/popup*`, `crates/app/src/popup_position.rs` | Remove or leave only until tests are replaced; avoid breaking permission errors. |
| 2.10 | Update tray/menu actions if needed. | `crates/app/src/tray.rs` | Tray "Open Translator" opens main; settings still opens integrated settings view. |

**Verification**

- Manual: selected text + hotkey opens main and translates.
- Manual: no selection + clipboard fallback enabled opens main and translates clipboard.
- Manual: no selection + fallback disabled opens main without network requests.
- Manual: Pin survives restart.
- Manual: Escape hides main and app remains in tray.

**Risk**

- Selection reading can fail if the main window steals focus too early. Keep source acquisition in backend before `show()`/`set_focus()`.

---

## 5. M3 - Source Editor and Result UX

**Goal**: Implement the new main translation surface.

| # | Task | Files | Notes |
| --- | --- | --- | --- |
| 3.1 | Build auto-resizing source editor shell. | `ui/src/App.tsx`, optional `ui/src/components/SourceEditor.tsx`, `ui/src/app.css` | Min/max height, internal scroll after max, bottom padding for toolbar. |
| 3.2 | Add always-visible source toolbar. | same | Left: Play/Copy ghost. Center: Auto Detect => target dropdown. Right: Clear ghost + Translate primary. |
| 3.3 | Convert target language dropdown to flag/name rendering. | `Combobox` or new `LanguageSelect` | Must fit min width and be keyboard accessible. |
| 3.4 | Remove standalone top source/target controls. | `ui/src/App.tsx` | Source is fixed auto-detect in toolbar. |
| 3.5 | Add below-editor service status row. | `ui/src/App.tsx`, `ui/src/services/ServiceLogo.tsx` | Left service count + icon group; right all-services refresh ghost. |
| 3.6 | Implement all-services refresh. | `ui/src/App.tsx` | Reuse current source/target and reset all outcomes to pending. |
| 3.7 | Add single-service translation command. | `crates/app/src/commands.rs`, `crates/core/src/translator.rs`, `ui/src/ipc/commands.ts` | `translate_service({ service_id, text, from, to, request_id })` should return one outcome. |
| 3.8 | Implement per-service hover/focus refresh. | `ui/src/App.tsx`, `app.css` | Hidden until hover/focus; keyboard reachable. |
| 3.9 | Add per-service stale response protection. | `ui/src/App.tsx` | Track request id per service, not only global request id. |
| 3.10 | Compact result card header. | `ui/src/App.tsx` | One line: small logo + service name; remove detected source display. |
| 3.11 | Keep dictionary details but visually subordinate. | `ui/src/App.tsx` | Dictionary phonetics remain source pronunciation controls. |

**Verification**

- `npm run typecheck`
- Manual responsive check at minimum window size.
- Manual keyboard pass: Tab reaches source toolbar, target dropdown, refresh buttons, result actions.
- Service refresh updates only the selected card.

---

## 6. M4 - Audio and Ghost Controls

**Goal**: Fix the source/target audio semantics and polish play/copy buttons.

| # | Task | Files | Notes |
| --- | --- | --- | --- |
| 4.1 | Split audio semantics in data model. | `crates/core/src/model.rs`, services, `ui/src/types/bindings.ts` | Make result-level audio mean translated text audio. Keep dictionary phonetic audio as source-word audio. |
| 4.2 | Add source TTS generation path. | `crates/core/src/services/youdao.rs` or new `audio.rs`, `crates/app/src/commands.rs` | Source editor Play requests source text audio from source/detected language. |
| 4.3 | Generate translated TTS URL for result text. | services or shared audio helper | Use target language, not original query. |
| 4.4 | Update Youdao tests around audio URLs. | `crates/core/src/services/youdao.rs` | Add regression tests: result audio contains translated text; source/dictionary audio contains source text. |
| 4.5 | Replace Square playing state. | `ui/src/App.tsx`, remove `Square` import | Alternate `Volume1`/`Volume2` while playing. |
| 4.6 | Make Play and Copy ghost buttons. | `ui/src/App.tsx`, `ui/src/app.css` | Apply to source toolbar and result cards. |
| 4.7 | Centralize audio playback. | `ui/src/hooks/useAudioPlayer.ts` or component | Only one audio plays at a time; clicking current button stops it. |

**Verification**

- Unit tests for audio URL generation.
- Manual: source Play speaks original text.
- Manual: result Play speaks translated text.
- Manual: icon alternates Volume1/Volume2 until playback ends.

---

## 7. M5 - Update Settings and Updater Integration

**Goal**: Add safe, asynchronous update checks and manual update UI.

| # | Task | Files | Notes |
| --- | --- | --- | --- |
| 5.1 | Add Tauri updater dependency and plugin setup. | `Cargo.toml`, `crates/app/src/main.rs`, `crates/app/capabilities/default.json` | Add `tauri-plugin-updater`, configure `updater:default`. |
| 5.2 | Configure updater public key and endpoints. | `crates/app/tauri.conf.json` | Use official Tauri updater signing. Add stable/beta endpoint strategy. |
| 5.3 | Update release workflow for updater artifacts. | `.github/workflows/release.yml` | Ensure release build creates update artifacts/signatures and uploads them. |
| 5.4 | Add backend/IPC update commands. | `crates/app/src/commands.rs`, `ui/src/ipc/commands.ts` | `check_update`, `download_and_install_update`, maybe `get_update_state`. |
| 5.5 | Add Update settings section. | `ui/src/SettingsApp.tsx`, `ui/src/settings/sections/UpdateSection.tsx`, locales | Include startup check toggle, allow beta toggle, manual check, status, install/restart action. |
| 5.6 | Run startup check asynchronously. | `crates/app/src/main.rs` or frontend bootstrap | Must not block first paint/hotkey/tray. Persist last status. |
| 5.7 | Implement beta eligibility. | backend updater command | Stable mode ignores prerelease; beta mode considers prerelease and stable versions newer than current. |
| 5.8 | Add failure states and logging. | backend + UI | Startup failure is quiet; manual failure is inline. |
| 5.9 | Test against a local/static updater manifest. | `qa/`, scripts | Avoid burning real release tags during development. |

**Updater implementation guidance**

- Use official Tauri updater APIs, not a custom downloader.
- Generate and store updater signing keys securely; do not commit private keys.
- Release artifacts must include updater signatures expected by Tauri updater.
- Keep beta channel selection runtime-configurable so the UI toggle can switch behavior.

**Verification**

- `actionlint .github/workflows/release.yml`
- Manual local update manifest: no update, stable update, beta ignored, beta accepted, network failure.
- Real release dry run before publishing v0.2.0.

**Risk**

- Updater signing and endpoint format are release-blocking. This milestone should start before final UI polish if schedule is tight.

---

## 8. M6 - Localization, Cleanup, QA, Docs

**Goal**: Make the release shippable and remove stale v0.1 popup assumptions.

| # | Task | Files | Notes |
| --- | --- | --- | --- |
| 6.1 | Complete all locale strings. | `ui/src/locales/*.ftl` | All 12 app languages must pass missing-key check. |
| 6.2 | Update user guide. | `docs/user-guide.md` | Main-window hotkey flow, clipboard fallback, pin, update settings, language settings. |
| 6.3 | Update dev/release docs. | `docs/dev-guide.md`, `docs/RELEASE.md` | Updater keys, artifacts, beta releases. |
| 6.4 | Update QA checklist. | `qa/` | Add manual cases from SPEC-v0.2.0 acceptance criteria. |
| 6.5 | Remove dead popup docs and code references. | `docs/DESIGN.md`, source comments | Keep historical v0.1 docs intact where versioned. |
| 6.6 | Run full local gates. | repo root and `ui/` | See verification below. |
| 6.7 | Cross-platform smoke test. | manual | Windows, macOS, Ubuntu. |

**Full verification**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
cd ui && npm run typecheck && npm run lint && npm run build
actionlint .github/workflows/*.yml
```

Manual QA matrix:

- Hotkey selected text -> main translation.
- Hotkey clipboard fallback disabled/enabled.
- Permission error shown in main window.
- Pin toggles always-on-top and persists.
- Escape hides main window.
- Source editor autoresizes and toolbar remains visible.
- Target language dropdown with flags works.
- App Language changes UI without restart.
- All-services refresh.
- Per-service refresh.
- Source/result/dictionary audio semantics.
- Update manual check: no update, update available, failure.
- Startup update check is asynchronous and non-blocking.

---

## 9. Open Decisions

| ID | Decision | Recommended default |
| --- | --- | --- |
| D-1 | Should `check_on_startup` default on? | Yes, because it is asynchronous and non-blocking. |
| D-2 | Should Pin persist? | Yes, users expect always-on-top preference to survive restart. |
| D-3 | Should clipboard fallback auto-translate by default? | No, default off for privacy and surprise minimization. |
| D-4 | Which flag should represent English and Arabic? | en=US, ar=SA as visual defaults; flags are display markers only. |
| D-5 | Should result audio exist for services that do not provide TTS? | Yes via shared TTS helper when possible; otherwise hide the Play button. |
| D-6 | Should updater install automatically after download? | No, require explicit user confirmation. |

---

## 10. Branching Recommendation

- Create a feature branch from `dev`: `feat/v0.2-main-window-flow`.
- Land M1-M2 together only after hotkey-to-main flow is verified.
- M3-M4 can be separate PRs if needed.
- M5 should be its own PR because updater signing/release workflow changes are high-risk.
- Keep `main` reserved for release-ready merges.
