# ifconfig-rs — Top-level Makefile
# https://github.com/lukaspustina/ifconfig-rs

SHELL       := /bin/bash
.DEFAULT_GOAL := all

# ── Project metadata ─────────────────────────────────────────────
APP         := ifconfig-rs
VERSION     := $(shell grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)"/\1/' 2>/dev/null || echo "unknown")
GIT_SHA     := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
DOCKER_TAG  := $(APP):$(VERSION)

# ── Tools ────────────────────────────────────────────────────────
CARGO       := cargo
NPM         := npm

# ── Directories ──────────────────────────────────────────────────
FRONTEND_DIR := frontend
DIST_DIR     := $(FRONTEND_DIR)/dist

# ── Flags (override from CLI: make CARGO_FLAGS=--release build) ──
CARGO_FLAGS  ?=
NPM_CI_FLAGS ?=

# ── Phony targets ────────────────────────────────────────────────
.PHONY: all build check test lint ci clean dev run \
        frontend frontend-install frontend-dev frontend-test \
        test-rust test-frontend \
        fmt fmt-check clippy \
        docker docker-run \
        unit integration acceptance bench \
        update-data release stats help

# ══════════════════════════════════════════════════════════════════
#  Help
# ══════════════════════════════════════════════════════════════════

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | \
		awk -F ':.*## ' '{printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' | sort

# ══════════════════════════════════════════════════════════════════
#  Production
# ══════════════════════════════════════════════════════════════════

all: frontend build ## Build frontend + release binary

build: frontend ## Build release binary (depends on frontend)
	$(CARGO) build --release $(CARGO_FLAGS)

run: build ## Build and run release binary
	./target/release/$(APP)

# ══════════════════════════════════════════════════════════════════
#  Rust
# ══════════════════════════════════════════════════════════════════

check: ## Fast compile check (cargo check)
	$(CARGO) check $(CARGO_FLAGS)

test-rust: ## Run Rust tests
	$(CARGO) test --lib --no-fail-fast $(CARGO_FLAGS)
	$(CARGO) test --no-fail-fast $(CARGO_FLAGS)

clippy: ## Run clippy with -D warnings
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings

fmt: ## Format Rust code
	$(CARGO) fmt

fmt-check: ## Check Rust formatting
	$(CARGO) fmt -- --check

# ══════════════════════════════════════════════════════════════════
#  Frontend
# ══════════════════════════════════════════════════════════════════

frontend-install: ## Install frontend dependencies (npm ci)
	cd $(FRONTEND_DIR) && $(NPM) ci $(NPM_CI_FLAGS)

frontend: frontend-install ## Build frontend (npm ci + build)
	cd $(FRONTEND_DIR) && $(NPM) run build

frontend-dev: ## Start Vite dev server with API proxy
	cd $(FRONTEND_DIR) && $(NPM) run dev

frontend-test: frontend-install ## Run frontend tests (vitest)
	cd $(FRONTEND_DIR) && npx vitest run --passWithNoTests

# ══════════════════════════════════════════════════════════════════
#  Combined
# ══════════════════════════════════════════════════════════════════

test: test-rust test-frontend ## Run all tests (Rust + frontend)

test-frontend: frontend-test ## Alias for frontend-test

lint: clippy fmt-check ## Run all lints (clippy + fmt-check)

ci: lint test frontend ## Full CI pipeline (lint + test + frontend build)

# ══════════════════════════════════════════════════════════════════
#  Development
# ══════════════════════════════════════════════════════════════════

dev: ## Run local dev server on :8080
	$(CARGO) run $(CARGO_FLAGS) -- ifconfig.dev.toml

clean: ## Remove target/, frontend/dist/, node_modules/
	$(CARGO) clean
	rm -rf $(DIST_DIR) $(FRONTEND_DIR)/node_modules

# ══════════════════════════════════════════════════════════════════
#  Docker
# ══════════════════════════════════════════════════════════════════

docker: ## Build production Docker image
	docker build . --tag $(DOCKER_TAG) --tag $(APP):latest

docker-run: ## Run Docker image locally
	docker run --rm -p 8080:8080 $(APP):latest

# ══════════════════════════════════════════════════════════════════
#  Project-specific: Tests
# ══════════════════════════════════════════════════════════════════

unit: test-rust ## Alias: run Rust unit + integration tests

integration: ## Run Docker-based integration tests
	$(MAKE) -C tests integration

acceptance: ## Run Playwright E2E tests
	$(MAKE) -C tests acceptance

bench: ## Run Criterion benchmarks
	$(CARGO) bench

# ══════════════════════════════════════════════════════════════════
#  Project-specific: Data
# ══════════════════════════════════════════════════════════════════

update-data: ## Refresh all enrichment data files (see data/README.md)
	$(MAKE) -C data get_all

# ══════════════════════════════════════════════════════════════════
#  Project-specific: Release
# ══════════════════════════════════════════════════════════════════

release: ## Tag, push, and create GitHub release
	git push
	git push origin "v$(VERSION)"
	gh release create "v$(VERSION)" --title "v$(VERSION)" --generate-notes

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
