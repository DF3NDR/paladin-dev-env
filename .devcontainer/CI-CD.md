# DevContainer CI/CD Integration Guide

This guide explains how to use the Paladin DevContainer in CI/CD pipelines.

## Overview

The DevContainer setup provides three Docker configurations:

1. **Dockerfile.dev** - Full development environment with all tools
2. **Dockerfile** (production) - Minimal runtime image
3. **docker-compose.yml** - Services orchestration

## GitHub Actions

### Option 1: Use DevContainer in CI

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Build DevContainer
        run: |
          docker build -f .devcontainer/Dockerfile.dev -t paladin-dev .

      - name: Run tests in DevContainer
        run: |
          docker run --rm \
            -v ${{ github.workspace }}:/workspace \
            -w /workspace \
            paladin-dev \
            bash -c "cargo test --all-features"

      - name: Run clippy
        run: |
          docker run --rm \
            -v ${{ github.workspace }}:/workspace \
            -w /workspace \
            paladin-dev \
            bash -c "cargo clippy -- -D warnings"

      - name: Check formatting
        run: |
          docker run --rm \
            -v ${{ github.workspace }}:/workspace \
            -w /workspace \
            paladin-dev \
            bash -c "cargo fmt --check"
```

### Option 2: Use DevContainer Image with Services

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      minio:
        image: quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
        ports:
          - 9000:9000
        env:
          MINIO_ROOT_USER: minioadmin
          MINIO_ROOT_PASSWORD: minioadmin
        options: >-
          --health-cmd "curl -f http://localhost:9000/minio/health/live"
          --health-interval 30s
          --health-timeout 20s
          --health-retries 3

      mysql:
        image: mysql:8.0
        ports:
          - 3306:3306
        env:
          MYSQL_ROOT_PASSWORD: rootpassword
          MYSQL_DATABASE: paladin
          MYSQL_USER: paladin
          MYSQL_PASSWORD: paladinpass
        options: >-
          --health-cmd="mysqladmin ping"
          --health-interval=10s
          --health-timeout=5s
          --health-retries=5

    container:
      image: rust:1.93-slim-bullseye

    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: |
          apt-get update
          apt-get install -y pkg-config libssl-dev g++

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

      - name: Cache target directory
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        env:
          REDIS_URL: redis://redis:6379
          MINIO_ENDPOINT: minio:9000
          DATABASE_URL: mysql://paladin:paladinpass@mysql:3306/paladin
        run: cargo test --all-features
```

### Option 3: Build and Push DevContainer Image

```yaml
name: Build DevContainer

on:
  push:
    branches: [ main ]
    paths:
      - '.devcontainer/**'
      - 'Cargo.toml'
      - 'Cargo.lock'

jobs:
  build-push:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v4

      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push DevContainer image
        uses: docker/build-push-action@v5
        with:
          context: .
          file: .devcontainer/Dockerfile.dev
          push: true
          tags: |
            ghcr.io/${{ github.repository }}/devcontainer:latest
            ghcr.io/${{ github.repository }}/devcontainer:${{ github.sha }}
          cache-from: type=registry,ref=ghcr.io/${{ github.repository }}/devcontainer:latest
          cache-to: type=inline
```

## GitLab CI

```yaml
# .gitlab-ci.yml
image: rust:1.93-slim-bullseye

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo
  RUST_BACKTRACE: "1"

cache:
  paths:
    - .cargo/
    - target/

stages:
  - build
  - test
  - lint

before_script:
  - apt-get update
  - apt-get install -y pkg-config libssl-dev g++
  - rustc --version
  - cargo --version

build:
  stage: build
  script:
    - cargo build --all-features
  artifacts:
    paths:
      - target/debug/paladin
    expire_in: 1 hour

test:
  stage: test
  services:
    - redis:7-alpine
    - quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
    - mysql:8.0
  variables:
    REDIS_URL: "redis://redis:6379"
    MINIO_ENDPOINT: "minio:9000"
    MYSQL_ROOT_PASSWORD: "rootpassword"
    MYSQL_DATABASE: "paladin"
    DATABASE_URL: "mysql://root:rootpassword@mysql:3306/paladin"
  script:
    - cargo test --all-features

lint:
  stage: lint
  script:
    - rustup component add rustfmt clippy
    - cargo fmt --check
    - cargo clippy -- -D warnings
```

## Jenkins

```groovy
// Jenkinsfile
pipeline {
    agent {
        docker {
            image 'rust:1.93-slim-bullseye'
            args '-v $HOME/.cargo:/usr/local/cargo'
        }
    }

    environment {
        CARGO_HOME = '/usr/local/cargo'
        RUST_BACKTRACE = '1'
    }

    stages {
        stage('Setup') {
            steps {
                sh 'apt-get update && apt-get install -y pkg-config libssl-dev g++'
                sh 'rustc --version'
                sh 'cargo --version'
            }
        }

        stage('Build') {
            steps {
                sh 'cargo build --all-features'
            }
        }

        stage('Test') {
            steps {
                sh 'cargo test --all-features'
            }
        }

        stage('Lint') {
            steps {
                sh 'rustup component add rustfmt clippy'
                sh 'cargo fmt --check'
                sh 'cargo clippy -- -D warnings'
            }
        }

        stage('Security Audit') {
            steps {
                sh 'cargo install cargo-audit || true'
                sh 'cargo audit'
            }
        }
    }

    post {
        always {
            cleanWs()
        }
    }
}
```

## CircleCI

```yaml
# .circleci/config.yml
version: 2.1

orbs:
  rust: circleci/rust@1.6.0

jobs:
  build-and-test:
    docker:
      - image: rust:1.93-slim-bullseye
      - image: redis:7-alpine
      - image: quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
        command: server /data
        environment:
          MINIO_ROOT_USER: minioadmin
          MINIO_ROOT_PASSWORD: minioadmin
      - image: mysql:8.0
        environment:
          MYSQL_ROOT_PASSWORD: rootpassword
          MYSQL_DATABASE: paladin

    environment:
      RUST_BACKTRACE: "1"
      REDIS_URL: redis://localhost:6379
      MINIO_ENDPOINT: localhost:9000
      DATABASE_URL: mysql://root:rootpassword@localhost:3306/paladin

    steps:
      - checkout

      - run:
          name: Install dependencies
          command: |
            apt-get update
            apt-get install -y pkg-config libssl-dev g++

      - restore_cache:
          keys:
            - cargo-cache-{{ arch }}-{{ checksum "Cargo.lock" }}
            - cargo-cache-{{ arch }}-

      - run:
          name: Build
          command: cargo build --all-features

      - run:
          name: Test
          command: cargo test --all-features

      - run:
          name: Lint
          command: |
            rustup component add rustfmt clippy
            cargo fmt --check
            cargo clippy -- -D warnings

      - save_cache:
          key: cargo-cache-{{ arch }}-{{ checksum "Cargo.lock" }}
          paths:
            - ~/.cargo
            - ./target

workflows:
  version: 2
  build-test:
    jobs:
      - build-and-test
```

## Using Pre-built DevContainer Image

If you've pushed the DevContainer image to a registry:

```yaml
# GitHub Actions
jobs:
  test:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/your-org/paladin/devcontainer:latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --all-features
```

## Optimizations for CI

### 1. Use Caching Effectively

```yaml
# Cache Cargo registry and build artifacts
- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

### 2. Parallel Testing

```bash
# Use nextest for faster parallel testing
cargo install cargo-nextest
cargo nextest run --all-features
```

### 3. Incremental Compilation

```yaml
env:
  CARGO_INCREMENTAL: 1
  CARGO_NET_RETRY: 10
  RUST_BACKTRACE: short
```

### 4. Build Only What's Needed

```bash
# Skip documentation generation
cargo build --no-default-features --features="required-features"

# Check instead of build for faster feedback
cargo check --all-features
```

## Security Scanning

### Snyk — evaluated and rejected (2026-08-18)

Do not add `snyk/actions/rust`. Snyk has no useful coverage for this workspace and
was removed; see `.github/instructions/security.instructions.md` for the evidence.
In short: `snyk test` (SCA) has no Cargo support and exits `SNYK-CLI-0008`, and
Snyk Code (SAST) ingests `.rs` files but found 0 of 4 deliberately planted
vulnerabilities that it caught in the equivalent JavaScript. A green Snyk run here
means nothing was analysed. Use the Rust-native tools below.

### Using cargo-audit

```yaml
- name: Security Audit
  run: |
    cargo install cargo-audit
    cargo audit
```

### Using cargo-deny

```yaml
- name: Check Dependencies
  run: |
    cargo install cargo-deny
    cargo deny check
```

## Deployment Pipeline

Complete example with build, test, security, and deploy:

```yaml
name: Complete Pipeline

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  check:
    runs-on: ubuntu-latest
    container: rust:1.93-slim-bullseye
    steps:
      - uses: actions/checkout@v4
      - name: Install deps
        run: apt-get update && apt-get install -y pkg-config libssl-dev g++
      - name: Cargo fmt
        run: cargo fmt --check
      - name: Cargo clippy
        run: cargo clippy -- -D warnings

  test:
    needs: check
    runs-on: ubuntu-latest
    container: rust:1.93-slim-bullseye
    services:
      redis:
        image: redis:7-alpine
      minio:
        image: quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
      mysql:
        image: mysql:8.0
        env:
          MYSQL_ROOT_PASSWORD: rootpassword
    steps:
      - uses: actions/checkout@v4
      - name: Install deps
        run: apt-get update && apt-get install -y pkg-config libssl-dev g++
      - name: Run tests
        run: cargo test --all-features

  security:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Security audit
        run: |
          cargo install cargo-audit
          cargo audit

  build-image:
    needs: security
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build production image
        run: docker build -t paladin:latest .
      - name: Push to registry
        run: |
          echo "${{ secrets.REGISTRY_PASSWORD }}" | docker login -u "${{ secrets.REGISTRY_USERNAME }}" --password-stdin
          docker push paladin:latest
```

## Tips for CI/CD

1. **Use sparse registry** for faster dependency fetching:
   ```toml
   # .cargo/config.toml
   [registries.crates-io]
   protocol = "sparse"
   ```

2. **Minimize image layers** in production Dockerfile

3. **Use multi-stage builds** to reduce final image size

4. **Cache dependencies** aggressively

5. **Run tests in parallel** with nextest

6. **Use check before build** for faster feedback

7. **Separate lint, test, and build jobs** for better parallelization

## Troubleshooting CI

### Out of Memory

```yaml
# Reduce parallel jobs
env:
  CARGO_BUILD_JOBS: 2
```

### Slow Builds

```yaml
# Use pre-built DevContainer image
container:
  image: ghcr.io/your-org/paladin-devcontainer:latest
```

### Service Connection Issues

```yaml
# Wait for services to be ready
- name: Wait for services
  run: |
    until nc -z redis 6379; do sleep 1; done
    until nc -z minio 9000; do sleep 1; done
```

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Docker in CI/CD](https://docs.docker.com/ci-cd/)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
