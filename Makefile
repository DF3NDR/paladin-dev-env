# Makefile for paladin project

# Variables
CARGO := cargo
DOCKER := docker
DOCKER_COMPOSE := docker-compose
PROJECT_NAME := paladin

# Docker compose files
COMPOSE_FILE := docker/docker-compose.yml
COMPOSE_DEV_FILE := docker/docker-compose.dev.yml
COMPOSE_TEST_FILE := docker/docker-compose.test.yml

# Default target
.DEFAULT_GOAL := help

# Colors for output
CYAN := \033[0;36m
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m

##@ Development

.PHONY: help
help: ## Show this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\n$(CYAN)Usage:$(NC)\n  make $(YELLOW)<target>$(NC)\n"} /^[a-zA-Z_0-9-]+:[^#]*##/ { printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(CYAN)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

.PHONY: examples
examples: ## Show common usage examples
	@echo "$(CYAN)Common Usage Examples:$(NC)"
	@echo ""
	@echo "$(YELLOW)Development Workflow:$(NC)"
	@echo "  make setup                    # First time setup"
	@echo "  make dev                      # Start dev environment"
	@echo "  make watch                    # Watch for changes"
	@echo "  make test-integration-minio   # Test MinIO integration"
	@echo ""
	@echo "$(YELLOW)Testing:$(NC)"
	@echo "  make test-all                 # Run all tests"
	@echo "  make test-integration-docker  # Integration tests with Docker"
	@echo "  make ci-test                  # Full CI test suite"
	@echo ""
	@echo "$(YELLOW)Code Quality:$(NC)"
	@echo "  make clean-code               # Format, lint, and check"
	@echo "  make release VERSION=0.4.0    # Cut a release (bump, changelog, tag, push)"
	@echo "  make publish-dry-run          # Dependency-first cargo publish --dry-run"
	@echo "  make audit                    # Security audit"
	@echo "  make doc                      # Generate docs"
	@echo ""
	@echo "$(YELLOW)Services Management:$(NC)"
	@echo "  make services-up              # Start all services"
	@echo "  make redis-cli                # Connect to Redis"
	@echo "  make minio-console            # Open MinIO console"
	@echo "  make health                   # Check service health"

.PHONY: status
status: ## Show project status
	@echo "$(CYAN)Project Status:$(NC)"
	@echo ""
	@echo "$(YELLOW)Build Status:$(NC)"
	@$(CARGO) --version || echo "❌ Cargo not found"
	@rustc --version || echo "❌ Rust not found"
	@echo ""
	@echo "$(YELLOW)Docker Status:$(NC)"
	@$(DOCKER) --version || echo "❌ Docker not found"
	@$(DOCKER_COMPOSE) --version || echo "❌ Docker Compose not found"
	@echo ""
	@echo "$(YELLOW)Services Status:$(NC)"
	@$(MAKE) health
	@echo ""
	@echo "$(YELLOW)Git Status:$(NC)"
	@git status --porcelain || echo "Not a git repository"

# Include custom targets if they exist
-include Makefile.local

# NOTE: `setup` must NOT share the -include line above. As a second include
# operand it is treated as a makefile to load; make then tries to *remake* it,
# finds this rule, and runs the whole recipe (rustup update + cargo install) on
# every single make invocation before the requested goal.
.PHONY: setup
setup: ## Initial project setup
	@echo "$(CYAN)Setting up development environment...$(NC)"
	@rustup update stable
	@rustup component add rustfmt clippy
	@$(CARGO) install cargo-audit cargo-watch cargo-expand
	@$(CARGO) install --locked cargo-release
	@$(CARGO) install --locked cargo-deny cargo-cyclonedx
	@cp .env.example .env 2>/dev/null || echo ".env already exists"
	@echo "$(GREEN)✅ Setup complete!$(NC)"

.PHONY: dev
dev: ## Start development environment with hot reload
	@echo "$(CYAN)Starting development environment...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) -f $(COMPOSE_DEV_FILE) up -d
	@echo "$(GREEN)✅ Development environment started$(NC)"
	@echo "Services available at:"
	@echo "  - Application: http://localhost:8080"
	@echo "  - MinIO Console: http://localhost:9001"
	@echo "  - Redis Commander: http://localhost:8081"

.PHONY: dev-logs
dev-logs: ## Show development environment logs
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) -f $(COMPOSE_DEV_FILE) logs -f

.PHONY: watch
watch: ## Watch for file changes and run tests
	@echo "$(CYAN)Starting file watcher...$(NC)"
	@$(CARGO) watch -x check -x test -x 'clippy --all-targets'

##@ Testing

.PHONY: test
test: ## Run unit tests
	@echo "$(CYAN)Running unit tests...$(NC)"
	@$(CARGO) test --workspace --lib --bins

.PHONY: test-doc
test-doc: ## Run documentation tests
	@echo "$(CYAN)Running documentation tests...$(NC)"
	@$(CARGO) test --workspace --doc

.PHONY: test-integration
test-integration: ## Run integration tests (local mode)
	@echo "$(CYAN)Running integration tests in local mode...$(NC)"
	@./scripts/run_integration_tests.sh -m local

.PHONY: test-integration-docker
test-integration-docker: ## Run integration tests with docker-compose (includes the Ollama and Postgres Tier 2 suites, 17-07/D-15, 22-06/D-10)
	@echo "$(CYAN)Running integration tests with docker-compose...$(NC)"
	@./scripts/run_integration_tests.sh -m docker -v
	@echo "$(CYAN)Starting ollama-test for the Ollama Docker-gated Tier 2 suite (17-07)...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) up -d ollama-test ollama-test-init
	@OLLAMA_TEST_URL=http://localhost:11435/v1 $(CARGO) test --test ollama_docker --features integration-tests,llm-ollama -- --nocapture
	@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) down -v --remove-orphans || true
	@echo "$(CYAN)Starting postgres-test for the PostgresWaypointStore Docker-gated Tier 2 suite (22-06)...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) up -d postgres-test
	@WAYPOINT_POSTGRES_TEST_URL=postgres://paladin:paladin@localhost:5433/paladin_waypoint_test \
		$(CARGO) test -p paladin-storage --features postgres --lib waypoint::postgres -- --nocapture
	@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) down -v --remove-orphans || true

.PHONY: test-integration-redis
test-integration-redis: ## Run Redis integration tests only
	@echo "$(CYAN)Running Redis integration tests...$(NC)"
	@./scripts/run_integration_tests.sh -t "redis" -m local

.PHONY: test-integration-minio
test-integration-minio: ## Run MinIO integration tests only
	@echo "$(CYAN)Running MinIO integration tests...$(NC)"
	@./scripts/run_integration_tests.sh -t "file_storage" -m local

.PHONY: test-all
test-all: test test-doc test-integration ## Run all tests
	@echo "$(GREEN)✅ All tests completed!$(NC)"

.PHONY: check-doc-examples
check-doc-examples: ## Compile doc examples (paladin-doc-examples crate) + syntax-scan inline rust blocks
	@echo "$(CYAN)Checking doc code examples...$(NC)"
	@./scripts/check-doc-examples.sh

.PHONY: check-doc-config
check-doc-config: ## Validate fenced YAML config snippets in docs/src parse correctly
	@echo "$(CYAN)Checking doc config snippets...$(NC)"
	@./scripts/check-doc-config.sh

.PHONY: check-changelogs
check-changelogs: ## Verify every publishable crate carries a CHANGELOG.md
	@./scripts/check-changelogs.sh

.PHONY: check-crate-names
check-crate-names: ## Guard against crates.io package-name collisions (allow-list)
	@./scripts/check-crate-names.sh

.PHONY: check-advisory-register
check-advisory-register: ## Verify SECURITY-EXCEPTIONS.md agrees with deny.toml/.cargo/audit.toml/Cargo.lock
	@./scripts/check-advisory-register.sh

.PHONY: check-workflow-suppressions
check-workflow-suppressions: ## Verify no workflow file passes an advisory-ignore flag to cargo audit or cargo deny
	@./scripts/check-workflow-suppressions.sh

.PHONY: check-workflow-triggers
check-workflow-triggers: ## Verify every workflow's trigger surface matches the recorded policy table
	@./scripts/check-workflow-triggers.sh

.PHONY: check-codeql-dismissals
check-codeql-dismissals: ## Verify CODEQL-DISMISSALS.md is schema-complete, non-drifted, non-stale and self-consistent
	@./scripts/check-codeql-dismissals.sh

.PHONY: check-release-consistency
# Deliberately NOT part of check-gates: every sibling guard above is a
# no-argument offline check runnable against the current tree as-is: this
# one requires a release tag (RELEASE_TAG) to check against, so it cannot be
# folded into the no-argument composite without a default/guessed tag,
# which is exactly the silent-wrong-tag failure mode this gate exists to
# prevent (see the guard's own MISSING_TAG check).
check-release-consistency: ## Verify a release tag's version matches every publishable manifest (RELEASE_TAG=vX.Y.Z required)
	@if [ -z "$(RELEASE_TAG)" ]; then \
		echo "$(RED)❌ RELEASE_TAG is required. Usage: make check-release-consistency RELEASE_TAG=v0.8.1-rc.2$(NC)"; \
		exit 1; \
	fi
	@./scripts/check-release-consistency.sh --tag "$(RELEASE_TAG)"

.PHONY: check-gates
check-gates: check-changelogs check-crate-names check-advisory-register check-workflow-suppressions check-workflow-triggers check-codeql-dismissals ## Run all offline release-gate guards

.PHONY: test-shell-guards
# Loops over every tests/scripts/*_test.sh rather than a hardcoded list, so
# a new guard's regression test (this phase adds one; later plans add more)
# cannot be silently left out of the run by being forgotten here. A glob
# that matches zero files is itself a named failure, never a silent pass --
# the same discovery-safety convention the guard scripts themselves follow.
test-shell-guards: ## Run regression tests for the offline gate guard scripts (not part of check-gates)
	@bash -c ' \
		shopt -s nullglob; \
		files=(tests/scripts/*_test.sh); \
		if [ "$${#files[@]}" -eq 0 ]; then \
			echo -e "$(RED)❌ no tests/scripts/*_test.sh files found -- a broken glob or an empty test directory is a named failure, never a silently-empty pass.$(NC)"; \
			exit 1; \
		fi; \
		for f in "$${files[@]}"; do \
			echo -e "$(CYAN)Running $$f...$(NC)"; \
			"$$f" || exit 1; \
		done \
	'

.PHONY: test-ci
test-ci: ## Run tests in CI mode
	@echo "$(CYAN)Running tests in CI mode...$(NC)"
	@./scripts/run_integration_tests.sh -m ci

.PHONY: test-cli
test-cli: ## Run CLI snapshot tests (86 snapshots, requires --features cli)
	@echo "$(CYAN)Running CLI snapshot tests...$(NC)"
	@$(CARGO) test -p paladin-ai --features cli --test cli

.PHONY: bench-check
bench-check: ## Compile-check benchmarks without running them
	@echo "$(CYAN)Checking benchmark compilation...$(NC)"
	@$(CARGO) bench --workspace --no-run

##@ Per-Crate Testing

.PHONY: test-core
test-core: ## Run tests for paladin-core
	@echo "$(CYAN)Running tests for paladin-core...$(NC)"
	@$(CARGO) test -p paladin-core

.PHONY: test-ports
test-ports: ## Run tests for paladin-ports
	@echo "$(CYAN)Running tests for paladin-ports...$(NC)"
	@$(CARGO) test -p paladin-ports

.PHONY: test-battalion
test-battalion: ## Run tests for paladin-battalion
	@echo "$(CYAN)Running tests for paladin-battalion...$(NC)"
	@$(CARGO) test -p paladin-battalion

.PHONY: test-llm
test-llm: ## Run tests for paladin-llm
	@echo "$(CYAN)Running tests for paladin-llm...$(NC)"
	@$(CARGO) test -p paladin-llm

.PHONY: test-memory
test-memory: ## Run tests for paladin-memory
	@echo "$(CYAN)Running tests for paladin-memory...$(NC)"
	@$(CARGO) test -p paladin-memory

.PHONY: test-storage
test-storage: ## Run tests for paladin-storage
	@echo "$(CYAN)Running tests for paladin-storage...$(NC)"
	@$(CARGO) test -p paladin-storage

.PHONY: test-notifications
test-notifications: ## Run tests for paladin-notifications
	@echo "$(CYAN)Running tests for paladin-notifications...$(NC)"
	@$(CARGO) test -p paladin-notifications

.PHONY: test-content
test-content: ## Run tests for paladin-content
	@echo "$(CYAN)Running tests for paladin-content...$(NC)"
	@$(CARGO) test -p paladin-content

.PHONY: test-web
test-web: ## Run tests for paladin-web
	@echo "$(CYAN)Running tests for paladin-web...$(NC)"
	@$(CARGO) test -p paladin-web

.PHONY: test-facade
test-facade: ## Run tests for paladin facade crate
	@echo "$(CYAN)Running tests for paladin (facade)...$(NC)"
	@$(CARGO) test -p paladin

##@ Coverage

.PHONY: coverage
coverage: ## Measure workspace coverage (mirrors CI's `coverage` job — requires make services-up)
	@echo "$(CYAN)Measuring coverage...$(NC)"
	@# Delegates to scripts/coverage.sh — shared with CI's `coverage` job so the
	@# feature list cannot drift. The script auto-detects service endpoints, so this
	@# works both on the host (localhost:6380/9010) and inside the devcontainer
	@# (redis:6379 / minio:9000), which the old hardcoded preflight could not.
	@bash scripts/coverage.sh

.PHONY: coverage-html
coverage-html: ## Generate an HTML coverage report at target/coverage
	@echo "$(CYAN)Generating HTML coverage report...$(NC)"
	@$(CARGO) llvm-cov --workspace --features integration-tests --html --output-dir target/coverage
	@echo "Report at target/coverage/html/index.html"

##@ Code Quality

.PHONY: fmt
fmt: ## Format code
	@echo "$(CYAN)Formatting code...$(NC)"
	@$(CARGO) fmt --all

.PHONY: lint
lint: ## Run linter
	@echo "$(CYAN)Running linter...$(NC)"
	@$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: lint-shell
lint-shell: ## Lint shell scripts with shellcheck (matches the pre-commit gate)
	@echo "$(CYAN)Running shellcheck...$(NC)"
	@command -v shellcheck >/dev/null 2>&1 || { \
		echo "$(RED)shellcheck not found. Install with 'sudo apt-get install shellcheck' (preinstalled in the devcontainer).$(NC)"; \
		exit 1; \
	}
	@# .claude/ is the vendored GSD toolchain (upstream-owned; excluded by the
	@# pre-commit gate for the same reason) — its *.snippet.sh files are inlined
	@# fragments with intentionally no shebang, which trips SC2148.
	@git ls-files -z '*.sh' ':!:.claude/**' | xargs -0 shellcheck --severity=warning
	@echo "$(GREEN)✅ shellcheck clean$(NC)"

.PHONY: check
check: ## Check code without building
	@echo "$(CYAN)Checking code...$(NC)"
	@$(CARGO) check --workspace --all-targets

.PHONY: audit
audit: ## Run security audit (vulnerability advisories)
	@echo "$(CYAN)Running security audit...$(NC)"
	@# Self-heal the advisory-db clone before auditing.
	@# cargo-audit updates its clone with fetch+reset, which never deletes UNTRACKED
	@# files. When RustSec renames or moves an advisory upstream (e.g. #3128 moved
	@# RUSTSEC-2026-0244 from gettext-sys to gettext-rs), the old path survives locally
	@# and collides with its own replacement, so the DB fails to load with
	@# "duplicate advisory ID" and the whole security gate dies. Dropping untracked
	@# files restores the clone to exactly what upstream tracks. Never fatal: a missing
	@# clone (first run) or absent git just falls through to cargo audit's own fetch.
	@git -C "$${CARGO_HOME:-$$HOME/.cargo}/advisory-db" clean -qfd 2>/dev/null || true
	@# Exceptions are sourced from .cargo/audit.toml (single source of truth).
	@$(CARGO) audit

.PHONY: deny
deny: ## Run cargo-deny (licenses, bans, advisories, sources)
	@echo "$(CYAN)Running cargo-deny check...$(NC)"
	@$(CARGO) deny check

.PHONY: openapi
openapi: ## Regenerate the committed OpenAPI baseline (crates/paladin-web/openapi.json)
	@echo "$(CYAN)Regenerating OpenAPI baseline...$(NC)"
	@UPDATE_OPENAPI=1 $(CARGO) test -p paladin-web --lib openapi_matches_committed_baseline -- --quiet
	@echo "Wrote crates/paladin-web/openapi.json"

.PHONY: keys
keys: ## Show which LLM API credentials are available (never prints values)
	@# paladin-env.sh is bash; make's default shell is dash, so invoke bash.
	@# Status query: never fail the build just because no keys are present yet.
	@bash -c '. .devcontainer/paladin-env.sh; paladin-keys' || true

.PHONY: security
security: audit deny ## Run all dependency security & license checks

.PHONY: sbom
sbom: ## Generate a CycloneDX SBOM (paladin.cdx.json)
	@echo "$(CYAN)Generating CycloneDX SBOM...$(NC)"
	@command -v cargo-cyclonedx >/dev/null 2>&1 || { \
		echo "$(RED)cargo-cyclonedx not found. Install with 'cargo install --locked cargo-cyclonedx'.$(NC)"; \
		exit 1; \
	}
	@$(CARGO) cyclonedx --all --format json
	@echo "$(GREEN)✅ SBOM generated (see bom.json / <crate>.cdx.json)$(NC)"

.PHONY: doc
doc: ## Generate documentation
	@echo "$(CYAN)Generating documentation...$(NC)"
	@$(CARGO) doc --workspace --no-deps --open

.PHONY: clean-code
clean-code: fmt lint lint-shell check ## Format, lint (Rust + shell), and check code

.PHONY: hooks
hooks: ## Install git pre-commit and pre-push hooks
	@echo "$(CYAN)Installing pre-commit hooks...$(NC)"
	@command -v pre-commit >/dev/null 2>&1 || { \
		echo "$(RED)pre-commit not found. Install it with 'pipx install pre-commit' (see CONTRIBUTING.md).$(NC)"; \
		exit 1; \
	}
	@pre-commit install
	@pre-commit install --hook-type pre-push
	@echo "$(GREEN)✅ Git hooks installed (pre-commit + pre-push)$(NC)"

##@ Build

.PHONY: build
build: ## Build the project
	@echo "$(CYAN)Building project...$(NC)"
	@$(CARGO) build --workspace

.PHONY: build-release
build-release: ## Build release version
	@echo "$(CYAN)Building release version...$(NC)"
	@$(CARGO) build --release --workspace

.PHONY: build-docker
build-docker: ## Build Docker image
	@echo "$(CYAN)Building Docker image...$(NC)"
	@$(DOCKER) build -f docker/Dockerfile -t $(PROJECT_NAME):latest .

.PHONY: docker-build-server
docker-build-server: ## Build the paladin-server HTTP API image (Dockerfile.server)
	@echo "$(CYAN)Building paladin-server image...$(NC)"
	@$(DOCKER) build -f Dockerfile.server -t paladin-server:latest .

##@ Docker Services

.PHONY: services-up
services-up: ## Start all services
	@echo "$(CYAN)Starting all services...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) up -d
	@echo "$(GREEN)✅ Services started$(NC)"

.PHONY: services-down
services-down: ## Stop all services
	@echo "$(CYAN)Stopping all services...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) down
	@echo "$(GREEN)✅ Services stopped$(NC)"

.PHONY: services-restart
services-restart: services-down services-up ## Restart all services

.PHONY: services-logs
services-logs: ## Show service logs
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) logs -f

.PHONY: services-ps
services-ps: ## Show running services
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) ps

.PHONY: redis-cli
redis-cli: ## Connect to Redis CLI
	@echo "$(CYAN)Connecting to Redis...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) exec redis redis-cli

.PHONY: minio-console
minio-console: ## Open MinIO console
	@echo "$(CYAN)Opening MinIO console...$(NC)"
	@echo "MinIO Console: http://localhost:9001"
	@echo "Credentials: minioadmin/minioadmin"

##@ Database & Storage

.PHONY: db-reset
db-reset: ## Reset database
	@echo "$(CYAN)Resetting database...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) exec paladin-app rm -f database.db || true
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) restart paladin-app

.PHONY: storage-reset
storage-reset: ## Reset MinIO storage
	@echo "$(CYAN)Resetting MinIO storage...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) exec minio rm -rf /data/* || true
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) restart minio minio-init

.PHONY: data-reset
data-reset: db-reset storage-reset ## Reset all data

##@ Utilities

.PHONY: clean
clean: ## Clean build artifacts
	@echo "$(CYAN)Cleaning build artifacts...$(NC)"
	@$(CARGO) clean
	@$(DOCKER) system prune -f

.PHONY: clean-docker
clean-docker: ## Clean Docker containers and volumes
	@echo "$(CYAN)Cleaning Docker resources...$(NC)"
	@$(DOCKER_COMPOSE) -f $(COMPOSE_FILE) down -v --remove-orphans
	@$(DOCKER_COMPOSE) -f $(COMPOSE_TEST_FILE) down -v --remove-orphans || true
	@$(DOCKER) system prune -f

.PHONY: deps-update
deps-update: ## Update dependencies
	@echo "$(CYAN)Updating dependencies...$(NC)"
	@$(CARGO) update

.PHONY: deps-tree
deps-tree: ## Show dependency tree
	@$(CARGO) tree

.PHONY: size
size: ## Show binary size
	@echo "$(CYAN)Binary sizes:$(NC)"
	@ls -lh target/release/$(PROJECT_NAME) 2>/dev/null || echo "No release binary found. Run 'make build-release' first."

##@ CI/CD

.PHONY: ci-setup
ci-setup: ## Setup CI environment
	@echo "$(CYAN)Setting up CI environment...$(NC)"
	@rustup component add rustfmt clippy

.PHONY: ci-test
ci-test: ## Run CI test suite
	@echo "$(CYAN)Running CI test suite...$(NC)"
	@$(MAKE) clean-code
	@$(MAKE) test
	@$(MAKE) test-cli
	@$(MAKE) test-doc
	@$(MAKE) audit
	@$(MAKE) test-ci

.PHONY: ci-full
ci-full: ci-test coverage ## Run the full CI gate locally (tests, then coverage)

.PHONY: release-check
release-check: ## Check if ready for release
	@echo "$(CYAN)Checking release readiness...$(NC)"
	@$(MAKE) clean-code
	@$(MAKE) test
	@$(CARGO) test --workspace --doc
	@$(MAKE) audit
	@$(MAKE) build-release
	@echo "$(GREEN)✅ Release check passed!$(NC)"

.PHONY: publish-dry-run
publish-dry-run: release-check ## Run dependency-first `cargo publish --dry-run` for all crates
	@echo "$(CYAN)Running dependency-first publish dry-runs...$(NC)"
	@$(CARGO) publish --dry-run -p paladin-core || true
	@$(CARGO) publish --dry-run -p paladin-ports || true
	@$(CARGO) publish --dry-run -p paladin-battalion || true
	@$(CARGO) publish --dry-run -p paladin-llm || true
	@$(CARGO) publish --dry-run -p paladin-memory || true
	@$(CARGO) publish --dry-run -p paladin-web || true
	@$(CARGO) publish --dry-run -p paladin-notifications || true
	@$(CARGO) publish --dry-run -p paladin-content || true
	@$(CARGO) publish --dry-run -p paladin-storage || true
	@$(CARGO) publish --dry-run -p paladin || true
	@echo "$(YELLOW)Dry-run publish command sequence completed. See docs/RELEASE_CHECKLIST.md for interpretation and publish-order gating.$(NC)"

.PHONY: finalize-crate-changelogs
finalize-crate-changelogs: ## Stamp a dated section into every publishable package's changelog (VERSION=x.y.z required)
	@if [ -z "$(VERSION)" ]; then \
		echo "$(RED)❌ VERSION is required. Usage: make finalize-crate-changelogs VERSION=0.4.0$(NC)"; \
		exit 1; \
	fi
	@./scripts/finalize-crate-changelogs.sh --version "$(VERSION)"

.PHONY: release
release: ## Cut a release: bump version (lockstep), finalize changelog, commit, tag, push. Usage: make release VERSION=0.4.0
	@if [ -z "$(VERSION)" ]; then \
		echo "$(RED)❌ VERSION is required. Usage: make release VERSION=0.4.0$(NC)"; \
		exit 1; \
	fi
	@echo "$(VERSION)" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$$' || { \
		echo "$(RED)❌ VERSION '$(VERSION)' is not a valid semver string (e.g. 0.4.0 or 0.4.0-rc.1).$(NC)"; \
		exit 1; \
	}
	@command -v cargo-release >/dev/null 2>&1 || { \
		echo "$(RED)❌ cargo-release not found. Install with 'cargo install --locked cargo-release'.$(NC)"; \
		exit 1; \
	}
	@# Release-branch protection (Milestone 10 Epic 5): tags may only be cut from
	@# an up-to-date `main`. The CI guard in release.yml is authoritative; this is
	@# fast local feedback. RELEASE_ALLOW_ANY_BRANCH=1 bypasses only the branch-name
	@# check (for rare hotfix branches) — see docs/src/appendix/branch-protection.md.
	@if [ "$(RELEASE_ALLOW_ANY_BRANCH)" = "1" ]; then \
		echo "$(YELLOW)⚠  RELEASE_ALLOW_ANY_BRANCH=1 — skipping main-branch check (CI still enforces main-only).$(NC)"; \
	else \
		CURRENT_BRANCH=$$(git rev-parse --abbrev-ref HEAD); \
		if [ "$$CURRENT_BRANCH" != "main" ]; then \
			echo "$(RED)❌ Releases must be cut from 'main' (current branch: $$CURRENT_BRANCH).$(NC)"; \
			echo "$(RED)   Merge your changes via PR, then 'git checkout main && git pull --ff-only'.$(NC)"; \
			echo "$(RED)   Override (hotfix only): RELEASE_ALLOW_ANY_BRANCH=1. See docs/src/appendix/branch-protection.md.$(NC)"; \
			exit 1; \
		fi; \
	fi
	@echo "$(CYAN)Verifying local branch is up to date with origin/main...$(NC)"
	@git fetch --quiet origin main || { echo "$(RED)❌ Failed to fetch origin/main.$(NC)"; exit 1; }
	@if [ -n "$$(git rev-list HEAD..origin/main)" ]; then \
		echo "$(RED)❌ HEAD is behind origin/main. Run 'git pull --ff-only origin main' before releasing.$(NC)"; \
		exit 1; \
	fi
	@echo "$(CYAN)Cutting release v$(VERSION)...$(NC)"
	@$(MAKE) release-check
	@echo "$(CYAN)Bumping all crates to $(VERSION) (lockstep)...$(NC)"
	@$(CARGO) release version "$(VERSION)" --execute --no-confirm --workspace
	@# The committed OpenAPI baseline (crates/paladin-web/openapi.json) embeds the
	@# workspace version, so every bump invalidates it and the pre-push drift guard
	@# (openapi_matches_committed_baseline) rejects the release commit. Regenerate
	@# it here so `git add -u` below picks it up with the bump. Found live in the
	@# v0.8.1-rc.3 rehearsal (Phase 20, 20-07 finding 1).
	@echo "$(CYAN)Regenerating OpenAPI baseline for $(VERSION)...$(NC)"
	@UPDATE_OPENAPI=1 $(CARGO) test -p paladin-web openapi_matches_committed_baseline --quiet
	@echo "$(CYAN)Finalizing changelogs for all publishable packages (root + crates)...$(NC)"
	@./scripts/finalize-crate-changelogs.sh --version "$(VERSION)"
	@echo "$(CYAN)Verifying release consistency for v$(VERSION) before tagging...$(NC)"
	@# Runs the manifest/changelog agreement clauses directly against the guard
	@# script rather than via a recursive $(MAKE) call: GNU Make always executes
	@# a recipe line that references $(MAKE), even under `make -n`, which would
	@# break dry-run testing of this target (`make -n release VERSION=...`) by
	@# actually invoking the gate against a fake version. This is the last step
	@# before tag/push, so a tag is never pushed for a tree the gate would
	@# reject. The release.yml `check-release-consistency` CI job is
	@# authoritative for the CI-conclusion clause (D-10), which only a live
	@# GitHub Actions run can evaluate -- a local pass here is not the full gate.
	@./scripts/check-release-consistency.sh --tag "v$(VERSION)"
	@echo "$(CYAN)Committing, tagging, and pushing...$(NC)"
	@git add -u
	@git commit -m "chore(release): version $(VERSION)"
	@git tag -a "v$(VERSION)" -m "Release $(VERSION)"
	@git push origin HEAD
	@git push origin "v$(VERSION)"
	@echo "$(GREEN)✅ Release v$(VERSION) tagged and pushed. CI (release.yml) will publish to crates.io.$(NC)"

##@ Monitoring & Debug

.PHONY: health
health: ## Check service health
	@echo "$(CYAN)Checking service health...$(NC)"
	@echo "Redis:"
	@curl -f http://localhost:6379 2>/dev/null && echo "✅ Redis OK" || echo "❌ Redis DOWN"
	@echo "MinIO:"
	@curl -f http://localhost:9000/minio/health/live 2>/dev/null && echo "✅ MinIO OK" || echo "❌ MinIO DOWN"
	@echo "Application:"
	@curl -f http://localhost:8080/health 2>/dev/null && echo "✅ App OK" || echo "❌ App DOWN or no health endpoint"

.PHONY: bench
bench: ## Run benchmarks
	@echo "$(CYAN)Running benchmarks...$(NC)"
	@$(CARGO) bench --workspace

.PHONY: profile
profile: ## Profile the application
	@echo "$(CYAN)Profiling application...$(NC)"
	@$(CARGO) build --release
	@echo "Run: perf record target/release/$(PROJECT_NAME)"
	@echo "Then: perf report"

##@ DevContainer

.PHONY: devcontainer-build
devcontainer-build: ## Build the DevContainer image
	@echo "$(CYAN)Building DevContainer image...$(NC)"
	@$(DOCKER) build -f .devcontainer/Dockerfile.dev -t $(PROJECT_NAME)-devcontainer:latest .
	@echo "$(GREEN)✅ DevContainer image built$(NC)"

.PHONY: devcontainer-validate
devcontainer-validate: ## Validate DevContainer setup
	@echo "$(CYAN)Validating DevContainer...$(NC)"
	@.devcontainer/validate.sh

.PHONY: devcontainer-network
devcontainer-network: ## Create DevContainer network
	@.devcontainer/setup-network.sh

.PHONY: devcontainer-services
devcontainer-services: ## Start DevContainer services
	@echo "$(CYAN)Starting DevContainer services...$(NC)"
	@$(DOCKER_COMPOSE) -f .devcontainer/docker-compose.yml up -d redis minio mysql
	@echo "$(GREEN)✅ Services started$(NC)"

.PHONY: devcontainer-services-down
devcontainer-services-down: ## Stop DevContainer services
	@echo "$(CYAN)Stopping DevContainer services...$(NC)"
	@$(DOCKER_COMPOSE) -f .devcontainer/docker-compose.yml down
	@echo "$(GREEN)✅ Services stopped$(NC)"

.PHONY: devcontainer-push
devcontainer-push: ## Push DevContainer image to registry
	@echo "$(CYAN)Pushing DevContainer image...$(NC)"
	@$(DOCKER) tag $(PROJECT_NAME)-devcontainer:latest ghcr.io/df3ndr/$(PROJECT_NAME)-devcontainer:latest
	@$(DOCKER) push ghcr.io/df3ndr/$(PROJECT_NAME)-devcontainer:latest
	@echo "$(GREEN)✅ Image pushed$(NC)"

##@ Help

.PHONY:
