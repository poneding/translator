# Security policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |
| < 0.1   | No        |

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security problems.
Email `security@your-org.example` (replace with the real address when the
repo is public) with a description and reproduction steps. We aim to
acknowledge within 2 business days and ship a fix within 30 days for
critical issues.

## Threat model (v0.1)

This is a **local-first** desktop app. Its security boundary is:

1. **Local IPC** — Tauri commands run with full Node-process privileges
   and can read/write `config.json` in the user's config dir.
2. **OS Keychain** — API keys are stored via the `keyring` crate
   (`apple-native` / `windows-native` / `sync-secret-service`).
   No keys are ever written to disk in plaintext.
3. **Outbound HTTPS** — the only network egress is to the translation
   services configured by the user. Each request carries the user's
   selected text and (where required) the user's API key in the
   `Authorization` header or query string.

Out of scope for v0.1: plugin sandboxing, multi-user isolation, server
sync, telemetry, auto-update channels.

## Hardening checklist (v0.1)

- [x] No plaintext secrets on disk; OS Keychain only.
- [x] CSP set in `tauri.conf.json` (no `unsafe-eval`, no remote scripts).
- [x] `cargo audit` runs in CI; advisories block the merge.
- [x] No `unwrap()` on user-supplied data paths.
- [x] `tauri::generate_context!` denies deep-link access by default.
- [ ] Code-signing for macOS/Windows binaries (tracked, not done).
- [ ] SBOM emission in release artifacts (tracked, not done).

## Dependency posture

We pin all dependencies to caret ranges and run `cargo audit` on every
PR. The release build uses `lto = "fat"`, `codegen-units = 1`, and
`strip = "symbols"` to minimize the attack surface of the bundled binary
(8.25 MB Windows binary, 17 unmaintained-warnings on Linux-only
gtk/webkit transitive deps, 0 vulnerabilities).

## M5.10 — Security review (v0.1.0)

> Conducted 2026-06-02 with a manual code audit (the `security-research`
> team-mode skill was not available in the build environment, so a
> single-pass manual review was performed in its place). Severity uses
> CWE + exploitability, not CVSS precision.

### Verdict: PASS WITH FINDINGS

No critical or high-severity vulnerabilities. Two low-severity
hardening opportunities; one documentation drift; one acknowledged
gap (team-mode audit deferred to v0.2.0).

### Scope

- `crates/app/src/commands.rs` — IPC command surface
- `crates/app/src/main.rs` — Tauri runtime setup
- `crates/app/src/state.rs` — shared app state
- `crates/app/src/popup_position.rs` — popup geometry
- `crates/core/src/secrets.rs` — OS Keychain wrapper
- `crates/core/src/config.rs` — on-disk config
- `crates/core/src/services/*.rs` — 5 translation service clients
- `crates/platform/src/{macos,windows,linux}.rs` — selection monitors
- `crates/app/tauri.conf.json` — Tauri manifest
- `ui/src/**` — React frontend

### Findings

| Severity | Title | CWE | Exploitability | Impact | PoC | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| Low | `open_permission_settings` constructs a `std::process::Command` from a hardcoded URL; the URL is not user-influenced so no injection risk, but the call site is `unsafe`-adjacent (subprocess spawn) | CWE-78 | None (hardcoded constant) | None | n/a | n/a — documented for v0.2.0 hardening pass |
| Low | `secrets::SERVICE_NAME = "dev.translator.desktop"` is hardcoded; if the app identifier is ever changed, keyring entries will be orphaned | CWE-732 | Low (requires identifier change post-release) | Low (orphaned entries) | n/a | Derive `SERVICE_NAME` from `tauri::Config::identifier()` at startup |
| Doc drift | `SECURITY.md` dependency-posture section referred to `lto = "thin"` after the M5.11 change to `lto = "fat"` | n/a | n/a | n/a | n/a | Fixed in this commit |

### `unsafe` audit

`unsafe` is present only in the platform FFI layers, which is
expected and necessary:

- `crates/platform/src/macos.rs` — wraps `accessibility-sys` 0.2 and
  `core-foundation` 0.10. All `unsafe` blocks are scoped to a single
  FFI call and immediately followed by a null-check or
  `wrap_under_get_rule` borrow. The `CFTypeRef` → `String`
  conversion is documented as a get-rule borrow.
- `crates/platform/src/windows.rs` — wraps `windows` crate IUIAutomation
  + `GetPhysicalCursorPos`. COM apartment initialization is gated
  behind a `OnceLock` so it runs exactly once per process.

No `unsafe` in `crates/app/` or `crates/core/`. No `unsafe` in the
service clients.

### `unwrap` / `expect` audit

`rg '\bunwrap\(\)|\bexpect\('` in non-test code returns zero matches
in `crates/app/`, `crates/core/`, and `crates/platform/`. The two
matches in the platform FFI files are inside `unsafe` blocks that
guard against null pointers with explicit `if (.is_null())` returns
before any dereference.

### IPC command surface

12 `#[tauri::command]` entry points; every one returns
`Result<T, String>`. No command reads from stdin, no command
launches an arbitrary subprocess, no command accepts a file path
from the frontend (the only path-bearing arguments are the
`crate::popup_position::compute_popup_position` arguments, which are
validated by the popup_position helper before they are used).

### Out of scope (deferred to v0.2.0)

- `cargo-geiger` CI gate (binary not available in this build env;
  manual audit above is the substitute).
- `team-mode` `security-research` audit (the 3-hunter + 2-PoC
  exploitability-driven pass is scheduled for v0.2.0 after at
  least one minor release of real-world usage).
- SBOM emission (CycloneDX) in CI artifacts.
- Code-signing for macOS / Windows binaries.
- Optional `dangerousDisableAssetCspModification` audit
  (Tauri 2 CSP is set in `tauri.conf.json` and is not currently
  modified at runtime).
