#!/usr/bin/env bash
# release.sh — cut a Translator release.
#
# This script is for the project maintainer. It automates the local
# steps of M6 (generate changelog + cut + push a git tag) and then
# lets GitHub Actions do the heavy lifting (cross-platform builds +
# draft release creation).
#
# Prerequisites:
#   - You have a GitHub remote configured (e.g. `git remote add origin
#     git@github.com:<your-org>/translator.git`).
#   - You have permission to push tags to the remote.
#   - You have git-cliff installed, or Docker installed so
#     scripts/changelog.sh can use the orhunp/git-cliff image.
#   - The `release.yml` workflow has the secrets it needs (default
#     `GITHUB_TOKEN` is sufficient for the first run; signing needs
#     additional secrets per docs/RELEASE.md).
#
# Usage:
#   ./release.sh              # cut + push v0.1.0
#   ./release.sh v0.1.0       # same
#   ./release.sh v0.1.1 "fix X"  # custom tag + extra commit
#
# After this script completes, go to:
#   https://github.com/<your-org>/translator/releases
#   → review the draft release created by release.yml
#   → smoke-test on a clean machine for each platform (M6.3)
#   → publish

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

VERSION="${1:-v0.1.0}"
EXTRA_COMMIT_MSG="${2:-}"

if ! command -v git >/dev/null 2>&1; then
    echo "error: git is required" >&2
    exit 1
fi

# 1. Sanity checks
echo "==> Sanity checks"
if ! git rev-parse --git-dir >/dev/null 2>&1; then
    echo "error: not a git repository (expected $REPO_ROOT/.git)" >&2
    exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
    echo "error: working tree is not clean" >&2
    echo "       commit or stash your changes first:" >&2
    git status --short >&2
    exit 1
fi

REMOTE_URL="$(git remote get-url origin 2>/dev/null || true)"
if [[ -z "$REMOTE_URL" ]]; then
    echo "error: no 'origin' remote configured" >&2
    echo "       run: git remote add origin git@github.com:<your-org>/translator.git" >&2
    exit 1
fi
echo "    origin = $REMOTE_URL"
echo "    current branch = $(git rev-parse --abbrev-ref HEAD)"

# 2. Optional extra commit
if [[ -n "$EXTRA_COMMIT_MSG" ]]; then
    echo "==> Creating extra commit"
    git commit --allow-empty -m "$EXTRA_COMMIT_MSG"
fi

# 3. Generate and commit the changelog
echo "==> Generating changelog"
./scripts/changelog.sh release "$VERSION"

if [[ -n "$(git status --porcelain CHANGELOG.md)" ]]; then
    echo "==> Committing changelog"
    git add CHANGELOG.md
    git commit -m "chore(release): update changelog for $VERSION"
else
    echo "    CHANGELOG.md is already up to date"
fi

# 4. Run the machine gates one last time
echo "==> Running gates"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --quiet
(cd ui && npm run typecheck && npm run lint)
echo "    all gates green"

# 5. Tag
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    echo "==> Tag $VERSION already exists locally"
    echo "    delete with 'git tag -d $VERSION' to re-create"
else
    echo "==> Creating tag $VERSION"
    git tag -a "$VERSION" -m "$VERSION - released by release.sh on $(date -u +%Y-%m-%d)"
fi

# 6. Push
echo "==> Pushing branch and tag to $REMOTE_URL"
read -r -p "    Push? [y/N] " CONFIRM
if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    echo "    aborted; tag is local-only. Push manually with:"
    echo "        git push origin HEAD"
    echo "        git push origin $VERSION"
    exit 0
fi

git push origin HEAD
git push origin "$VERSION"

echo ""
echo "==> Done. Next steps:"
echo "    1. Watch the release.yml workflow at:"
echo "       $REMOTE_URL/actions/workflows/release.yml"
echo "    2. When the workflow finishes, a draft release will be at:"
echo "       $REMOTE_URL/releases/tag/$VERSION"
echo "    3. Smoke-test on a clean machine for each platform (M6.3)"
echo "    4. Review the generated release notes"
echo "    5. Publish"
