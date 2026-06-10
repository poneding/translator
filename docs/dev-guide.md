# translator dev guide

> Conventions, layout, and how to add a new translation service.

## Build & run

Prerequisites:
- Rust 1.81+ (`rustup install stable`)
- Node.js 20+
- Platform deps (see [README.md](../README.md))

```bash
# install JS deps
cd ui && npm install && cd ..

# dev (Tauri + Vite hot reload)
cargo tauri dev

# release build
cargo tauri build
```

## Code style

```bash
./scripts/format.sh   # cargo fmt + prettier
./scripts/lint.sh     # clippy + cargo test + tsc + eslint
```

We follow the standard Rust style guide and the React/TS conventions in the source.

## Adding a new translation service

1. Create a new file `crates/core/src/services/<id>.rs` with a `pub struct <Name>Service` that implements `TranslationService`.
2. Add a variant to `ServiceId` in `crates/core/src/model.rs` and to `ServiceId::all()`.
3. Add a default row to `Config::default()` for the new id in `crates/core/src/config.rs`.
4. Register the service in `Translator::new()` in `crates/core/src/translator.rs`.
5. Add an icon (`crates/app/icons/service-icon/<id>.png`) — 64×64 transparent PNG.
6. Add a row in `ServicesSection.tsx` with the matching `ServiceMeta`.
7. Write unit tests using `wiremock` to mock the upstream HTTP responses.
8. Add the service to `docs/DESIGN.md §4.2`.

## Project layout

See [README.md](../README.md) and [DESIGN.md](DESIGN.md).

## Platform glue

The `crates/platform` crate holds the cross-platform `SelectionMonitor` trait and per-platform implementations. To add a new platform:

1. Create a `crates/platform/src/<os>.rs` module
2. Gate it with `#[cfg(target_os = "<os>")]` in `lib.rs`
3. Implement all `SelectionMonitor` methods
4. Add system deps to `.github/workflows/ci.yml` and `Dockerfile` (if you have one)

## Cross-platform testing matrix

| OS | Test on | Service impl. tested |
| --- | --- | --- |
| macOS 13+ | GitHub Actions `macos-latest` (arm64) | Manual + CI |
| Windows 10/11 | GitHub Actions `windows-latest` | Manual + CI |
| Ubuntu 22.04+ | GitHub Actions `ubuntu-latest` + manual GNOME/KDE VM | Manual + CI |

The Rust core has no platform-specific code and is unit-tested on all three. Platform crates compile on all three but their `SelectionMonitor` is only meaningfully testable on the corresponding OS.

## How services are tested

All five translation services use [`wiremock`](https://crates.io/crates/wiremock) for unit tests.
The pattern is:

1. Spin up a `MockServer` on a random port.
2. Inject the mock URL as `base_url` / `baseUrl` in the service's `ServiceConfig.options`.
3. Register `Mock::given(method/header/path)` with canned responses per scenario
   (happy path, auth failure, rate-limit, malformed body, missing fields, …).
4. Assert the typed `ServiceError` variant or the parsed `TranslateResult`.

To run just the service tests:

```bash
cargo test -p translator-core --lib services::
```

## How a release is cut

1. Bump `version` in the root `Cargo.toml` `[workspace.package]`.
2. Run `./scripts/changelog.sh preview` to inspect the git-cliff release
   notes for unreleased commits.
3. Run `./release.sh v0.1.0`; it regenerates `CHANGELOG.md`, commits the
   changelog update, creates the tag, and pushes after confirmation.
4. The `release.yml` workflow builds `.dmg` / `.msi` / `.AppImage` / `.deb`
   artifacts and opens a draft GitHub release with git-cliff notes attached.
5. Manually verify the draft artifacts and notes, then publish.

## Updater development

The app uses the official `tauri-plugin-updater`.

- Generate a Tauri updater key pair with `cargo tauri signer generate`.
- Commit only the public key in `crates/app/tauri.conf.json`
  `plugins.updater.pubkey`; never commit the private key.
- Store the private key in GitHub Actions as `TAURI_SIGNING_PRIVATE_KEY` and
  the password as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Enable `bundle.createUpdaterArtifacts` only after those secrets are present.
- Stable checks use
  `https://github.com/poneding/translator/releases/latest/download/latest.json`.
- Beta checks use the `beta` release/tag manifest URL configured in
  `crates/app/src/commands.rs`; keep that manifest pointed at the newest beta
  or prerelease build.
- For local updater QA, serve a static `latest.json` and signed updater
  artifact, then temporarily point the endpoint constants at the local server.
