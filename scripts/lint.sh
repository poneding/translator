#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd ui && npm run typecheck)
(cd ui && npm run lint)
echo "All checks passed."
