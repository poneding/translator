# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.2.x   | Yes       |
| 0.1.x   | Security fixes only until 0.2.0 is published |
| < 0.1   | No        |

## Reporting A Vulnerability

Please do not open a public GitHub issue for security problems. Email the
repository maintainer with a description, affected version, reproduction steps,
and any relevant logs. We aim to acknowledge critical reports within 2 business
days and ship a fix within 30 days.

## Threat Model

`translator` is a local-first desktop app. Its security boundary is:

1. **Tauri IPC**: frontend code invokes a fixed command surface in
   `crates/app/src/commands.rs`. Commands validate inputs and return
   `Result<T, String>` rather than panicking on user input.
2. **OS Keychain**: API keys are stored through `crates/core/src/secrets.rs`.
   Secrets are not written to `config.json`.
3. **Outbound HTTPS**: selected text is sent only to the translation services
   the user enables. The app has no telemetry backend.
4. **Updater**: update checks use the official Tauri updater plugin and signed
   updater artifacts. The public key is committed in `tauri.conf.json`; the
   private key is stored only in GitHub Actions secrets.

Out of scope: malicious local users with access to the same desktop session,
kernel compromise, compromised upstream translation providers, and professional
copy review for every localized string.

## Hardening Checklist

- [x] No plaintext API keys on disk; OS Keychain only.
- [x] CSP configured in `crates/app/tauri.conf.json`.
- [x] No remote scripts in the frontend bundle.
- [x] Update artifacts are signed by Tauri updater signing keys.
- [x] Updater private key is not committed.
- [x] Startup update failures are logged and do not block launch.
- [ ] Platform code signing for macOS/Windows installers.
- [ ] SBOM emission in release artifacts.
- [ ] `cargo audit`/advisory gate in CI.

## Audit Notes

`unsafe` is limited to platform FFI layers:

- `crates/platform/src/macos.rs` wraps macOS Accessibility APIs.
- `crates/platform/src/windows.rs` wraps Windows UI Automation and keyboard
  fallback APIs.

There is no `unsafe` in `crates/app` or `crates/core`. Frontend secrets are
write-only from the UI perspective: the app can set/delete/probe key presence,
but cannot read a stored API key back into React state.

## Release Security Checks

Before publishing a release:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
cd ui && npm run typecheck && npm run lint && npm run build
actionlint .github/workflows/*.yml
```

For updater-enabled releases, also verify:

- `bundle.createUpdaterArtifacts` is enabled.
- `TAURI_SIGNING_PRIVATE_KEY_B64` decodes to the full private key file.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` matches the key.
- The GitHub Release includes updater manifests, updater archives, and
  signatures for every published platform.
