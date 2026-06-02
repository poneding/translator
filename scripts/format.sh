#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all
(cd ui && npm run format)
echo "Formatted."
