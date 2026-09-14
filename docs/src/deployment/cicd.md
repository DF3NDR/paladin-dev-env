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
│   ├── docs.yml                  # MDBook build + GitHub Pages deploy
│   ├── feature-flags.yml         # Feature-flag matrix tests
│   ├── pre-commit.yml            # Pre-commit checks
│   └── release.yml               # Release automation
└── dependabot.yml                # Dependency updates
```

> **docs.yml** builds MDBook, runs `./scripts/check-doc-examples.sh` (validates all fenced Rust code blocks), and deploys to GitHub Pages on merge to `main`.

## CI Pipeline

### ci.yml

```yaml
name: CI

on:
  push:
    branches: [ '**' ]
  pull_request:
    branches: [ main, 'release/**' ]
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  lint:
    name: Code Quality
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Check
        run: cargo check --all-features

  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust ${{ matrix.rust }}
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}

      - name: Run tests
        run: cargo test --all-features

      - name: Run doc tests
        run: cargo test --doc --all-features

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true
```

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

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

permissions:
  contents: write
  packages: write

jobs:
  build-release:
    name: Build Release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools (Linux ARM64)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Package (Unix)
        if: matrix.os != 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          tar czf paladin-${{ github.ref_name }}-${{ matrix.target }}.tar.gz paladin
          mv paladin-${{ github.ref_name }}-${{ matrix.target }}.tar.gz ${{ github.workspace }}/

      - name: Package (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          cd target/${{ matrix.target }}/release
          7z a paladin-${{ github.ref_name }}-${{ matrix.target }}.zip paladin.exe
          move paladin-${{ github.ref_name }}-${{ matrix.target }}.zip ${{ github.workspace }}/

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: release-${{ matrix.target }}
          path: |
            paladin-*.tar.gz
            paladin-*.zip

  create-release:
    name: Create Release
    needs: build-release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download artifacts
        uses: actions/download-artifact@v3

      - name: Generate changelog
        id: changelog
        run: |
          # Extract changelog for this version
          VERSION="${{ github.ref_name }}"
          awk "/^## \[$VERSION\]/,/^## \[/" CHANGELOG.md | head -n -1 > release_notes.md

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            release-*/paladin-*.tar.gz
            release-*/paladin-*.zip
          body_path: release_notes.md
          draft: false
          prerelease: ${{ contains(github.ref_name, '-') }}
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

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
