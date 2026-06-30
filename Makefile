# Translator — top-level developer tasks.
#
# Conventions mirror .github/workflows/ci.yml and scripts/{lint,format}.sh so
# `make ci` matches what runs in CI. Run `make help` to list targets.

CARGO      := cargo
NPM_UI     := npm --prefix ui
TAURI      := cargo tauri

# A minimal ui/dist/index.html stub lets `cargo build`/`cargo test` link the
# Tauri app crate without a full frontend build (same trick CI uses).
UI_DIST_INDEX := ui/dist/index.html

.DEFAULT_GOAL := help
.PHONY: help install fmt fmt-check lint check test build-ui build bundle \
        build-release run dev audit ci clean ensure-ui-dist

help: ## Show available targets
	@grep -E '^[a-zA-Z_][a-zA-Z0-9_ -]*:.*?## ' $(MAKEFILE_LIST) \
	  | awk 'BEGIN {FS = ":.*?## "} {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}'

install: ## Install frontend dependencies (npm install in ui/)
	$(NPM_UI) install

fmt: ## Format Rust + frontend code in place
	$(CARGO) fmt --all
	$(NPM_UI) run format

fmt-check: ## Verify formatting without writing
	$(CARGO) fmt --all -- --check

lint: ## Run clippy (-D warnings) + eslint
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	$(NPM_UI) run lint

check: ## Type-check without producing binaries (cargo check + tsc)
	$(CARGO) check --workspace
	$(NPM_UI) run typecheck

ensure-ui-dist: ## Ensure ui/dist/index.html exists (stub if missing)
	@mkdir -p $(dir $(UI_DIST_INDEX))
	@test -f $(UI_DIST_INDEX) \
	  || printf '<!doctype html><html><body></body></html>' > $(UI_DIST_INDEX)

test: ensure-ui-dist ## Run Rust tests + locale parity check
	$(CARGO) test --workspace
	$(NPM_UI) run locales:check

build-ui: ## Build the frontend (tsc + vite build)
	$(NPM_UI) run build

build: ensure-ui-dist ## Build all workspace crates in debug profile
	$(CARGO) build --workspace

bundle build-release: ## Build the release Tauri bundle (runs npm build first)
	$(TAURI) build

run dev: ## Run the app in dev mode (Tauri, with vite dev server)
	$(TAURI) dev

audit: ## Run cargo audit (requires: cargo install cargo-audit)
	$(CARGO) audit

ci: fmt-check lint check test build-ui ## Run the full local CI equivalent

clean: ## Remove cargo + frontend build artifacts
	$(CARGO) clean
	rm -rf ui/dist
