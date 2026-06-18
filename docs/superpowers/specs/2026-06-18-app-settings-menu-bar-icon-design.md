# App Settings Menu Bar Icon Design

## Context

Translator is a Tauri desktop app that currently creates a menu bar, tray, or
AppIndicator icon unconditionally during startup. macOS also hides the Dock icon
by using accessory activation policy and Dock visibility APIs. User preferences
are persisted in `translator_core::config::Config` and edited from the React
settings UI.

## Requirements

- Add a setting for whether the app shows the menu bar, system tray, or
  AppIndicator icon.
- The new setting defaults to enabled for existing and new users.
- Changing the setting from Settings applies immediately without restarting.
- On macOS, hiding the menu bar icon must not show the Dock icon. Users can
  still open the app through the global hotkey or by launching it again.
- Keep the menu bar icon and login-startup controls in General settings.
- Existing configs with `general.launch_at_startup` must migrate without losing
  the user's preference.

## Design

Add an `AppConfig` group to the top-level Rust config:

- `app.show_menu_bar_icon: bool`, default `true`.
- `app.launch_at_startup: bool`, default `false`.

Keep `GeneralConfig.launch_at_startup` as a legacy deserialization-only field so
older config files can still load. During normalization, configs older than the
new schema version copy the legacy value into `app.launch_at_startup`. Saved
configs no longer write the legacy field.

Refactor `crates/app/src/tray.rs` so tray creation is idempotent and expose a
`sync_tray_visibility(app, visible)` helper. The helper creates the existing
`main` tray when `visible` is true and removes it with Tauri's
`remove_tray_by_id("main")` when false.

Load config before tray setup in `crates/app/src/main.rs`; create the tray only
when `cfg.app.show_menu_bar_icon` is true. Keep the macOS Dock behavior
unchanged.

Update `save_config` to compare and synchronize `app.launch_at_startup`, save
the normalized config, apply window always-on-top, then call
`sync_tray_visibility` with `config.app.show_menu_bar_icon`.

Add "Show menu bar icon" to General settings next to the existing app behavior
checkboxes. Keep "Launch at startup" in General settings, but wire it to
`config.app.launch_at_startup`. Update the TypeScript config types and locale
strings.

## Tests

- Core config tests cover defaults, serialization shape, and v3 legacy
  migration.
- App static tests verify startup no longer builds the tray unconditionally and
  `save_config` synchronizes tray visibility.
- Frontend verification uses TypeScript/build checks because the UI has no
  dedicated component test suite.
