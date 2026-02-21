# ifconfig-rs — Top-level Makefile
# https://github.com/lukaspustina/ifconfig-rs

SHELL       := /bin/bash
.DEFAULT_GOAL := all

# ── Project metadata ─────────────────────────────────────────────
APP         := ifconfig-rs
VERSION     := $(shell cargo metadata --format-version=1 --no-deps 2>/dev/null | \
                 python3 -c "import sys,json; print(json.load(sys.stdin)['packages'][0]['version'])" 2>/dev/null || echo "unknown")
GIT_SHA     := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
DOCKER_TAG  := $(APP):$(VERSION)

# ── Directories ──────────────────────────────────────────────────
FRONTEND_DIR := frontend
DIST_DIR     := $(FRONTEND_DIR)/dist

# ── Flags (override from CLI: make CARGO_FLAGS=--release build) ──
CARGO_FLAGS  ?=
NPM_CI_FLAGS ?=

# ── Phony targets ────────────────────────────────────────────────
.PHONY: all build check clippy fmt fmt-fix clean \
        backend-build backend-check backend-clippy backend-fmt backend-fmt-fix backend-clean backend-test \
        frontend-build frontend-clean frontend-dev \
        dev test unit integration acceptance bench \
        docker-build push-to-prod \
        stats help

# ══════════════════════════════════════════════════════════════════
#  Top-level (backend + frontend)
# ══════════════════════════════════════════════════════════════════

all: check unit integration ## Lint and run unit + integration tests

build: frontend-build backend-build ## Build everything (frontend then backend)

check: backend-check ## Run all checks

clippy: backend-clippy ## Run all linters

fmt: backend-fmt ## Check formatting

fmt-fix: backend-fmt-fix ## Auto-fix formatting

clean: backend-clean frontend-clean ## Remove all build artifacts

# ══════════════════════════════════════════════════════════════════
#  Backend (Rust / Cargo)
# ══════════════════════════════════════════════════════════════════

backend-build: frontend-build ## Compile Rust (builds frontend first)
	cargo build $(CARGO_FLAGS)

backend-check: ## Run cargo check
	cargo check $(CARGO_FLAGS)

backend-clippy: ## Run clippy lints
	cargo clippy $(CARGO_FLAGS) -- -D warnings

backend-fmt: ## Check Rust formatting
	cargo fmt -- --check

backend-fmt-fix: ## Auto-fix Rust formatting
	cargo fmt

backend-clean: ## Remove cargo build artifacts
	cargo clean

backend-test: frontend-build ## Run Rust unit and in-process integration tests
	cargo test --lib --no-fail-fast $(CARGO_FLAGS)
	cargo test --no-fail-fast $(CARGO_FLAGS)

# ══════════════════════════════════════════════════════════════════
#  Frontend (SolidJS / Vite)
# ══════════════════════════════════════════════════════════════════

frontend-build: $(DIST_DIR)/index.html ## Build the SolidJS frontend

$(DIST_DIR)/index.html: $(FRONTEND_DIR)/package.json $(FRONTEND_DIR)/package-lock.json $(shell find $(FRONTEND_DIR)/src -type f 2>/dev/null)
	cd $(FRONTEND_DIR) && npm ci $(NPM_CI_FLAGS) && npm run build

frontend-clean: ## Remove frontend dist
	rm -rf $(DIST_DIR)

frontend-dev: ## Start Vite dev server with API proxy
	cd $(FRONTEND_DIR) && npm run dev

# ══════════════════════════════════════════════════════════════════
#  Development
# ══════════════════════════════════════════════════════════════════

dev: ## Run local dev server on :8080
	cargo run $(CARGO_FLAGS) -- ifconfig.dev.toml

# ══════════════════════════════════════════════════════════════════
#  Tests
# ══════════════════════════════════════════════════════════════════

test: unit integration acceptance ## Run all tests (unit + integration + E2E)

unit: backend-test ## Run unit and in-process integration tests

integration: ## Run Docker-based integration tests
	$(MAKE) -C tests integration

acceptance: ## Run Playwright E2E tests
	$(MAKE) -C tests acceptance

bench: ## Run Criterion benchmarks
	cargo bench

# ══════════════════════════════════════════════════════════════════
#  Docker
# ══════════════════════════════════════════════════════════════════

docker-build: ## Build production Docker image
	docker build . --tag $(DOCKER_TAG) --tag $(APP):latest

# ══════════════════════════════════════════════════════════════════
#  Release
# ══════════════════════════════════════════════════════════════════

push-to-prod: ## Merge master into prod and push (triggers CI/CD)
	git checkout prod
	git merge master
	git push
	git checkout master

# ══════════════════════════════════════════════════════════════════
#  Stats
# ══════════════════════════════════════════════════════════════════

stats: ## Show project statistics
	@echo "─── $(APP) v$(VERSION) ($(GIT_SHA)) ───"
	@echo ""
	@echo "Backend (Rust):"
	@find src -name '*.rs' | xargs wc -l | tail -1 | awk '{printf "  Lines of code:  %s (across ", $$1}' && find src -name '*.rs' | wc -l | awk '{printf "%s files)\n", $$1}'
	@echo "  Crate deps:    $$(cargo metadata --format-version=1 2>/dev/null | python3 -c "import sys,json; pkgs=json.load(sys.stdin)['packages']; print(len([p for p in pkgs if p['name'] != '$(APP)']))")"
	@find src -name '*.rs' -exec grep -cE '#\[test\]|#\[tokio::test\]' {} + 2>/dev/null | awk -F: '{s+=$$2} END {printf "  Unit tests:     %d\n", s}'
	@find tests -name '*.rs' -exec grep -cE '#\[test\]|#\[tokio::test\]' {} + 2>/dev/null | awk -F: '{s+=$$2} END {printf "  Integration:    %d\n", s}'
	@echo ""
	@echo "Frontend (SolidJS):"
	@find $(FRONTEND_DIR)/src -name '*.ts' -o -name '*.tsx' -o -name '*.css' 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{printf "  Lines of code:  %s (across ", $$1}' && find $(FRONTEND_DIR)/src \( -name '*.ts' -o -name '*.tsx' -o -name '*.css' \) 2>/dev/null | wc -l | awk '{printf "%s files)\n", $$1}'
	@echo ""
	@echo "Git:"
	@echo "  Commits:       $$(git rev-list --count HEAD 2>/dev/null || echo '?')"
	@echo "  Contributors:  $$(git shortlog -sn --no-merges HEAD 2>/dev/null | wc -l | tr -d ' ')"
	@echo "  Branch:        $$(git branch --show-current 2>/dev/null || echo '?')"
	@if [ -d "$(DIST_DIR)" ]; then \
		echo ""; \
		echo "Build artifacts:"; \
		echo "  Frontend dist:  $$(du -sh $(DIST_DIR) 2>/dev/null | cut -f1 | tr -d ' ')"; \
	fi
	@if [ -f target/release/$(APP) ]; then \
		echo "  Release binary: $$(du -sh target/release/$(APP) | cut -f1 | tr -d ' ')"; \
	fi

# ══════════════════════════════════════════════════════════════════
#  Help
# ══════════════════════════════════════════════════════════════════

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | \
		awk -F ':.*## ' '{printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}' | sort
