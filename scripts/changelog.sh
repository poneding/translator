#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
    cat <<'USAGE'
Usage:
  ./scripts/changelog.sh write [git-cliff args...]
      Regenerate CHANGELOG.md for the current repository history.

  ./scripts/changelog.sh preview [git-cliff args...]
      Print release notes for unreleased commits without the changelog header/footer.

  ./scripts/changelog.sh release vX.Y.Z [git-cliff args...]
      Replace the CHANGELOG.md Unreleased section with generated notes for vX.Y.Z.

The script prefers a local git-cliff binary and falls back to Docker
(`orhunp/git-cliff`). Override the Docker image tag with GIT_CLIFF_VERSION.
USAGE
}

run_git_cliff() {
    if command -v git-cliff >/dev/null 2>&1; then
        git-cliff "$@"
        return
    fi

    if git cliff --version >/dev/null 2>&1; then
        git cliff "$@"
        return
    fi

    if command -v docker >/dev/null 2>&1; then
        docker run --rm \
            -v "$REPO_ROOT:/app" \
            -w /app \
            "orhunp/git-cliff:${GIT_CLIFF_VERSION:-latest}" \
            "$@"
        return
    fi

    echo "error: git-cliff is required" >&2
    echo "       install git-cliff, or install Docker for the fallback runner" >&2
    exit 1
}

replace_unreleased_section() {
    VERSION="$1"
    shift

    NOTES_FILE="$(mktemp)"
    OUTPUT_FILE="$(mktemp)"
    trap 'rm -f "$NOTES_FILE" "$OUTPUT_FILE"' RETURN

    run_git_cliff \
        --config cliff.toml \
        --tag "$VERSION" \
        --unreleased \
        --strip all \
        "$@" >"$NOTES_FILE"

    if [[ ! -s "$NOTES_FILE" ]]; then
        echo "error: git-cliff produced empty release notes for $VERSION" >&2
        exit 1
    fi

    if ! awk -v notes_file="$NOTES_FILE" '
        BEGIN {
            while ((getline line < notes_file) > 0) {
                notes = notes line ORS
            }
            close(notes_file)
            skipping = 0
            replaced = 0
        }
        /^## \[Unreleased\]/ && !replaced {
            print "## [Unreleased]"
            print ""
            printf "%s", notes
            skipping = 1
            replaced = 1
            next
        }
        skipping && /^## \[/ {
            skipping = 0
        }
        !skipping {
            print
        }
        END {
            if (!replaced) {
                exit 3
            }
        }
    ' CHANGELOG.md >"$OUTPUT_FILE"; then
        echo "error: CHANGELOG.md must contain a '## [Unreleased]' heading" >&2
        exit 1
    fi

    mv "$OUTPUT_FILE" CHANGELOG.md
}

MODE="${1:-write}"
if [[ $# -gt 0 ]]; then
    shift
fi

case "$MODE" in
    write)
        run_git_cliff --config cliff.toml --output CHANGELOG.md "$@"
        ;;
    preview)
        run_git_cliff --config cliff.toml --unreleased --strip all "$@"
        ;;
    release)
        if [[ $# -lt 1 ]]; then
            usage >&2
            exit 2
        fi
        VERSION="$1"
        shift
        replace_unreleased_section "$VERSION" "$@"
        ;;
    -h | --help | help)
        usage
        ;;
    *)
        echo "error: unknown mode '$MODE'" >&2
        usage >&2
        exit 2
        ;;
esac
