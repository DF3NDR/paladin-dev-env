# CI/CD Guide

Complete guide for setting up continuous integration and deployment pipelines for Paladin using GitHub Actions.

## Table of Contents

- [Overview](#overview)
- [GitHub Actions Workflows](#github-actions-workflows)
- [CI Pipeline](#ci-pipeline)
- [Docker Build Pipeline](#docker-build-pipeline)
- [Release Pipeline](#release-pipeline)
- [Integration Testing](#integration-testing)
- [Security Scanning](#security-scanning)
- [Deployment Automation](#deployment-automation)
- [Best Practices](#best-practices)

## Overview

Paladin uses GitHub Actions for CI/CD with the following pipelines:
- **CI**: Build, test, lint on every PR
- **Docker**: Build and publish multi-arch images
- **Release**: Automated releases with semantic versioning
- **Integration**: Integration tests with Docker services
- **Security**: Dependency scanning and vulnerability checks

## GitHub Actions Workflows

### Workflow Structure

```
.github/
├── workflows/
│   ├── benchmarks.yml            # Performance benchmark tracking
│   ├── ci.yml                    # Main CI pipeline (lint, test, integration, audit)
│   ├── codeql.yml                # Rust SAST scan — advisory only, does not gate a merge
│   ├── docs.yml                  # MDBook build + GitHub Pages deploy
│   ├── feature-flags.yml         # Feature-flag matrix tests
│   ├── pre-commit.yml            # Pre-commit checks
│   └── release.yml               # Release automation
└── dependabot.yml                # Dependency updates
```

> **docs.yml** builds MDBook, runs `./scripts/check-doc-examples.sh` (validates all fenced Rust code blocks), and deploys to GitHub Pages on merge to `main`.

## CI Pipeline

### ci.yml

`ci.yml` runs on every push (`branches: ['**']`, D-03) and on pull requests targeting `main` or
`release/**`. It has grown well beyond the three-job (`lint`/`test`/`coverage`) sample previously
shown here — the table below names every job the live file declares. The **Required or advisory**
column is taken directly from `.github/rulesets/protect-main-branch.json`'s
`required_status_checks` array: a check whose display name is not listed there can fail without
blocking a merge into `main`.

| Job (`ci.yml`) | Display name | What it gates | Required or advisory |
|---|---|---|---|
| `lint` | Code Quality | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo doc` warnings | Required |
| `actionlint` | Workflow Lint | Lints every `.github/workflows/*.yml` file with `actionlint` | Required |
| `security-audit` | Security Audit | `cargo audit` against the RustSec advisory database, exceptions from `.cargo/audit.toml` | Required |
| `cargo-deny` | License & Dependency Policy | `cargo deny check` plus the repository's own policy scripts (changelogs, crate names, advisory register, workflow-suppression and workflow-trigger guards, CodeQL dismissal register, shell-guard regression tests) | Required |
| `osv-scanner` | OSV Scanner | Google OSV database scan of `Cargo.lock`; SARIF uploaded for PR annotation | Required |
| `api-surface` | API Surface Tracking | `cargo public-api` diff against `.project/current-exports.txt`, plus deprecation-warning checks | Required |
| `msrv` | MSRV (Rust 1.88) | `cargo check --workspace --all-features --all-targets` at the pinned MSRV | Advisory |
| `semver` | Semver Checks (vs v0.9.0) | `cargo-semver-checks` for every publishable crate against the published v0.9.0 baseline | Advisory |
| `test` | Unit Tests (stable / beta) | `cargo test --workspace --lib --bins` and `cargo test --workspace --doc`, matrixed over stable and beta | Required |
| `examples` | Example Muster (Feature Matrix) | Builds all 47 `examples/*.rs` targets across a 4-invocation feature matrix | Required |
| `crate-isolation` | Crate Isolation (`<crate>`) | Each of the 10 matrixed workspace crates builds and tests independently, with and without default features | Required |
| `integration-tests` | Integration Tests | Redis + MinIO `--ignored` suites, plus the broad `--features integration-tests` workspace sweep | Required |
| `docker-integration` | Docker Integration Tests | Runs the Docker Compose test stack's `integration-tests` service | Required |
| `ollama-integration` | Ollama Integration Tests (live server) | Live Ollama server suite (`ollama_docker`) | Advisory |
| `postgres-integration` | Postgres Storage Contract Suites (live server) | Every `*::postgres` contract suite against a live Postgres container | Advisory |
| `redis-cache-integration` | Redis Node Cache Contract Suite (live server) | `node_cache::redis` against a live Redis container | Advisory |
| `redis-queue` | Redis Run Queue Contract Suite (live server) | `run_queue::redis` against a live Redis container | Advisory |
| `sdk-clients` | Generated SDK Clients (Python + TypeScript) smoke | Generates and smoke-tests the OpenAPI Python and TypeScript clients against a live `paladin-server` | Advisory |
| `e2e-platform-api` | E2E Platform API (PRD 06 acceptance-1 lifecycle + SHIP-02 boot proof) | The assistant → run → SSE → `AwaitingInput` → webhook → resume → history → fork lifecycle, plus the `v0_9_config_boot` backward-compat proof | Advisory |
| `coverage` | Coverage | Workspace line-coverage measurement and floor gate (see excerpt below) | Required |
| `cli-tests` | CLI Snapshot Tests | `cargo test -p paladin-ai --features cli --test cli` | Required |
| `bench-check` | Benchmark Compile Check | `cargo bench --workspace --no-run` (compiles every `[[bench]]` target; runs none) | Required |
| `docker` | Docker Build | Multi-arch image build and the 500 MB size budget; wall-clock is reported, not enforced | Advisory |
| `kubernetes-smoke` | Kubernetes Smoke Test | Deploys to a `kind` cluster and checks pod readiness | Advisory |
| `e2e-tests` | End-to-End Tests | Full Docker Compose stack end-to-end test; push-to-`main` only | Required |
| `benchmark-regression-signal` | Benchmark Regression Signal (Non-Blocking) | Criterion regression check on PRs/dispatch; `continue-on-error: true` | Advisory |
| `publish-dry-run` | Publish Dry Run | `cargo publish --workspace --dry-run`; push-to-`main` only | Advisory |

The `coverage` floor is not inlined in the workflow — the job delegates to the same script
`make coverage` runs locally:

```yaml
# excerpt: .github/workflows/ci.yml — job: coverage
      - name: Measure coverage
        env:
          USE_EXTERNAL_TEST_SERVICES: "true"
          TEST_REDIS_HOST: localhost
          TEST_REDIS_PORT: 6380
          TEST_MINIO_ENDPOINT: localhost:9010
          TEST_MINIO_ACCESS_KEY: testuser
          TEST_MINIO_SECRET_KEY: testpass123
        run: bash scripts/coverage.sh
```

```bash
# excerpt: scripts/coverage.sh
exec cargo llvm-cov --workspace --features integration-tests,llm-all \
    --lcov --output-path lcov.info --fail-under-lines "$FLOOR" -- --test-threads=1
```

`$FLOOR` defaults to `82` — the ADR-0006 coverage floor. See the
[Testing Guide](../contributing/testing-guide.md) for why the `llm-all` feature is load-bearing
for that measurement.

### codeql.yml — Rust SAST (advisory only)

`codeql.yml` runs Rust static analysis on every push, pull request and schedule (Wednesdays
07:00 UTC), reporting findings into the code-scanning UI. It is **not pinned in any ruleset and
does not gate a merge** — CodeQL was evaluated and disqualified as a required-check-grade Rust
SAST at CodeQL `2.26.3` (2026-08-25); the manual credential-handling review documented in
[`.github/instructions/security.instructions.md`](https://github.com/Am0rfu5/paladin/blob/main/.github/instructions/security.instructions.md)
stays the primary control for that class of code.

## Docker Build Pipeline

> **Corrected 2026-08-24 (Phase 16 / DOCS-01).** This section previously documented a
> `docker-publish.yml` workflow with a full YAML sample. **No such workflow exists** in
> `.github/workflows/` and none ever did in this repository — the sample was fabricated.
> Docker image building and publishing is part of the release pipeline, described below and
> in [Release Pipeline](#release-pipeline).

Container images are built and published by the **`build-docker`** job in
[`.github/workflows/release.yml`](https://github.com/Am0rfu5/paladin/blob/main/.github/workflows/release.yml)
(`release.yml:157`), not by a standalone workflow.

| Aspect | Actual configuration | Source |
|---|---|---|
| Registry | `ghcr.io` | `release.yml:21` (`REGISTRY`) |
| Multi-architecture | QEMU + Buildx | `docker/setup-qemu-action@v3`, `docker/setup-buildx-action@v3` |
| Authentication | `docker/login-action@v3` | `release.yml:175` |
| Tagging | `docker/metadata-action@v5` | `release.yml:183` |
| Published tags | `<version>` and `latest` | `release.yml:146-147` |

Pull a published image with:

```bash
docker pull ghcr.io/<owner>/<image>:<version>
docker pull ghcr.io/<owner>/<image>:latest
```

The Dockerfiles themselves are described in [Docker Deployment](docker.md).

## Release Pipeline

### release.yml

`release.yml` triggers on a `v*.*.*` tag push or manual `workflow_dispatch` (with an optional
`dry_run` input). An earlier version of this page described a single combined build-and-package
job under a name that does not exist in the live file. The real jobs, in dependency order, are:

| Job (`release.yml`) | What it does |
|---|---|
| `verify-tag-source` | The tag-source guard: resolves the release commit and fails the whole run closed unless that commit is an ancestor of `origin/main` — enforces the "main is the source of truth" invariant before anything else runs |
| `test` | `cargo test --workspace`; gates crates.io publishing only — Docker images and release binaries are **not** gated on it (a release with a failing test suite can still push an image and attach binaries, just not publish to crates.io) |
| `create-release` | Extracts the matching `## [X.Y.Z]` section from `CHANGELOG.md` and creates (or reuses) the GitHub release |
| `build-docker` | Builds and pushes the multi-arch (`linux/amd64`, `linux/arm64`) image to `ghcr.io` |
| `build-binaries` | Cross-compiles and uploads release binaries for 4 platform targets (Linux amd64/arm64, macOS amd64/arm64) |
| `check-release-consistency` | Pre-publish gate: fails closed if the tag disagrees with any publishable crate's manifest version, or with the tagged commit's own recorded CI conclusion |
| `sbom` | Generates a CycloneDX SBOM and uploads it to the release |
| `finalize-release-body` | Aggregates the Docker image digest, aggregated binary checksums, and SBOM asset name into the release body |
| `publish-crates` | Publishes to crates.io in dependency order via crates.io Trusted Publishing (short-lived OIDC token), after `check-release-consistency` and `test` both pass |

`verify-tag-source`'s guard is the reason a release tag must be cut from a merged PR into `main`
rather than from a feature branch directly — see
[Branch Protection](../appendix/branch-protection.md).

## Integration Testing

### ci.yml — `integration-tests` job

Integration testing runs as the `integration-tests` job inside `ci.yml`, absorbed from the
former standalone `integration-tests` workflow file (deleted in commit `2cf9919`). It shares
`ci.yml`'s trigger shown above rather than defining its own `on:` block.

```yaml
jobs:
  integration-tests:
    name: Integration Tests
    runs-on: ubuntu-latest

    services:
      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 6379:6379

      minio:
        image: quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
        env:
          MINIO_ROOT_USER: minioadmin
          MINIO_ROOT_PASSWORD: minioadmin
        options: >-
          --health-cmd "curl -f http://localhost:9000/minio/health/live"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 9000:9000

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Wait for services
        run: |
          timeout 60 bash -c 'until curl -f http://localhost:9000/minio/health/live; do sleep 2; done'
          timeout 60 bash -c 'until redis-cli -h localhost ping; do sleep 2; done'

      - name: Run integration tests
        run: cargo test --features integration-tests --test '*_integration_test'
        env:
          REDIS_URL: redis://localhost:6379
          MINIO_ENDPOINT: localhost:9000
          MINIO_ACCESS_KEY: minioadmin
          MINIO_SECRET_KEY: minioadmin
          RUST_LOG: debug

      - name: Integration test coverage
        run: |
          cargo install cargo-llvm-cov
          cargo llvm-cov --features integration-tests --test '*_integration_test' --lcov --output-path integration-lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: integration-lcov.info
          flags: integration
```

## Security Scanning

> **Corrected 2026-08-24 (Phase 16 / DOCS-01).** This section previously documented a
> `security.yml` workflow containing a **Snyk** job (`snyk/actions/rust@master` with a
> `SNYK_TOKEN` secret). **No such workflow exists**, and the Snyk step in particular
> contradicts a recorded project decision: Snyk was evaluated and **removed on 2026-08-18**
> because it has no meaningful Rust coverage — a "clean" Snyk result on this workspace means
> *nothing was analysed*, which is worse than no scan because it reads as assurance. See
> [`.github/instructions/security.instructions.md`](https://github.com/Am0rfu5/paladin/blob/main/.github/instructions/security.instructions.md).
> **Do not reintroduce a Snyk step.** The real security jobs are listed below.

Security scanning runs as three jobs inside
[`.github/workflows/ci.yml`](https://github.com/Am0rfu5/paladin/blob/main/.github/workflows/ci.yml):

| Job | Name | What it checks | Location |
|---|---|---|---|
| `security-audit` | Security Audit | `cargo audit` against the RustSec advisory database, with exceptions declared in `.cargo/audit.toml` | `ci.yml:83` |
| `cargo-deny` | License & Dependency Policy | Licences, bans, sources and advisories via `cargo-deny`, plus the repository's own policy scripts (changelogs, crate names, advisory register, workflow suppressions and triggers) | `ci.yml:103` |
| `osv-scanner` | OSV Scanner | Open Source Vulnerabilities database scan | `ci.yml:155` |

Run the dependency checks locally with the same tools CI uses:

```bash
make audit      # cargo-audit (RustSec advisory DB)
make deny       # cargo-deny (licenses, bans, sources, advisories)
make security   # both of the above
make sbom       # cargo-cyclonedx dependency inventory
```

**Known gap, stated plainly:** there is no static taint analysis (SAST) for first-party Rust in
this pipeline. `cargo-audit` and `cargo-deny` scan *dependencies*; `clippy` is a lint. Evaluating
a Rust-capable SAST is open work. Until then, credential-handling code is reviewed by hand per the
manual checklist in `security.instructions.md`.

## Deployment Automation

### Deploy to Kubernetes

```yaml
name: Deploy

on:
  push:
    tags:
      - 'v*.*.*'
  workflow_dispatch:
    inputs:
      environment:
        description: 'Environment to deploy to'
        required: true
        type: choice
        options:
          - staging
          - production

jobs:
  deploy:
    name: Deploy to ${{ github.event.inputs.environment || 'production' }}
    runs-on: ubuntu-latest
    environment:
      name: ${{ github.event.inputs.environment || 'production' }}
      url: https://paladin.${{ github.event.inputs.environment || 'prod' }}.example.com

    steps:
      - uses: actions/checkout@v4

      - name: Configure kubectl
        uses: azure/k8s-set-context@v3
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG }}

      - name: Deploy with Helm
        run: |
          helm upgrade --install paladin ./paladin-chart \
            --namespace paladin \
            --create-namespace \
            --set image.tag=${{ github.ref_name }} \
            --set secrets.openaiApiKey=${{ secrets.OPENAI_API_KEY }} \
            --values values-${{ github.event.inputs.environment || 'production' }}.yaml \
            --wait

      - name: Verify deployment
        run: |
          kubectl rollout status deployment/paladin -n paladin
          kubectl get pods -n paladin
```

## Best Practices

### 1. Branch Protection

Configure branch protection rules in GitHub:

```yaml
# Required status checks
- CI / check
- CI / test (ubuntu-latest, stable)
- CI / test (macos-latest, stable)
- CI / coverage
- Integration Tests

# Required reviews: 1
# Dismiss stale reviews: true
# Require linear history: true
```

### 2. Secrets Management

Store secrets in GitHub repository settings:

```bash
# Required secrets
GITHUB_TOKEN          # Auto-provided
OPENAI_API_KEY        # For integration tests
KUBE_CONFIG           # For K8s deployment
```

### 3. Caching Strategy

```yaml
# Cache Cargo dependencies
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

### 4. Concurrency Control

```yaml
# Cancel in-progress runs for same PR
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

### 5. Conditional Workflows

```yaml
# Skip CI for docs-only changes
on:
  push:
    paths-ignore:
      - '**.md'
      - 'docs/**'
```

### 6. Matrix Testing

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, beta, nightly]
  fail-fast: false  # Continue other jobs on failure
```

### 7. Artifact Retention

```yaml
- uses: actions/upload-artifact@v3
  with:
    name: test-results
    path: target/test-results/
    retention-days: 30
```

### 8. Notifications

```yaml
- name: Slack Notification
  if: failure()
  uses: 8398a7/action-slack@v3
  with:
    status: ${{ job.status }}
    webhook_url: ${{ secrets.SLACK_WEBHOOK }}
```

## Next Steps

- **[Production Best Practices](production.md)** - Production checklist
- **[Monitoring](../operations/monitoring.md)** - Observability setup
- **[Docker Deployment](docker.md)** - Docker deployment guide
