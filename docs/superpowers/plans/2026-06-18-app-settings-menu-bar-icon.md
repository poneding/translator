# App Settings Menu Bar Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add app-level settings for menu bar icon visibility and login startup, with immediate tray synchronization.

**Architecture:** Store app shell preferences under a new `Config.app` group, migrate legacy startup config during normalization, and keep tray visibility synchronized from startup and config saves. Expose the controls in the existing React General settings section.

**Tech Stack:** Rust, Tauri 2, serde, React, TypeScript, Fluent locales.

---

### Task 1: Config Shape and Migration

**Files:**
- Modify: `crates/core/src/config.rs`
- Modify: `ui/src/types/bindings.ts`

- [x] Add failing Rust tests in `crates/core/src/config.rs`:
  - `default_config_serializes_app_settings`
  - `normalized_migrates_legacy_launch_at_startup_to_app_settings`
- [x] Run `cargo test -p translator-core config -- --nocapture` and confirm the new tests fail.
- [x] Add `AppConfig` with `show_menu_bar_icon` and `launch_at_startup`.
- [x] Add `Config.app` and bump the schema version.
- [x] Make legacy `GeneralConfig.launch_at_startup` skip serialization.
- [x] Migrate legacy startup values from versions older than the new schema.
- [x] Update TypeScript `Config` with `app: AppConfig`.
- [x] Re-run `cargo test -p translator-core config -- --nocapture`.

### Task 2: Tray Synchronization

**Files:**
- Modify: `crates/app/src/tray.rs`
- Modify: `crates/app/src/main.rs`
- Modify: `crates/app/src/commands.rs`

- [x] Add failing static tests proving startup and save paths reference tray visibility sync.
- [x] Run `cargo test -p translator-app commands::tests -- --nocapture` and confirm failures.
- [x] Add idempotent `build_tray` behavior and `sync_tray_visibility`.
- [x] Load config before tray setup and only build the tray when `show_menu_bar_icon` is true.
- [x] Update `save_config` to sync autostart from `config.app.launch_at_startup` and tray visibility from `config.app.show_menu_bar_icon`.
- [x] Re-run `cargo test -p translator-app commands::tests -- --nocapture`.

### Task 3: Settings UI

**Files:**
- Modify: `ui/src/SettingsApp.tsx`
- Modify: `ui/src/settings/sections/GeneralSection.tsx`
- Modify: `ui/src/locales/en.ftl`
- Modify: `ui/src/locales/zh-Hans.ftl`

- [x] Add General checkboxes for menu bar icon and launch at startup.
- [x] Keep app shell preferences backed by `config.app`.
- [x] Do not add a separate App section to the sidebar or main settings content.
- [x] Add English and Simplified Chinese strings.
- [x] Run `npm --prefix ui run build`.

### Task 4: Final Verification

**Files:**
- Verify all touched files.

- [x] Run `cargo test -p translator-core config -- --nocapture`.
- [x] Run `cargo test -p translator-app commands::tests -- --nocapture`.
- [x] Run `cargo test`.
- [x] Run `npm --prefix ui run build`.
- [x] Review `git diff --stat` and `git diff` for unrelated changes.
