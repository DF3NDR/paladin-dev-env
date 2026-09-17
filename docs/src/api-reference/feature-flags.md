# Paladin Feature Flags

Paladin uses Cargo feature flags to enable fine-grained control over compiled dependencies and functionality. This allows you to build minimal, focused binaries for specific use cases while reducing compile times and binary sizes.

> See also the [Crate Map & Feature Flags](crate-map.md) reference for per-crate flag tables, the crate dependency graph, and copy-paste consumer profiles.

## Table of Contents

- [Overview](#overview)
- [Available Feature Flags](#available-feature-flags)
- [Default Configuration](#default-configuration)
- [Usage Examples](#usage-examples)
- [Build Comparison](#build-comparison)
- [Feature Dependencies](#feature-dependencies)
- [Best Practices](#best-practices)

## Overview

### Philosophy

Feature flags in Paladin follow these principles:

1. **Core Framework Always Available** - Paladin agents, Battalion orchestration, Garrison memory, Arsenal tools, and Herald formatters are always compiled
2. **Provider Choice** - Choose which of the nine LLM providers to support (OpenAI, Anthropic, DeepSeek, Kimi, Qwen, Grok, Ollama, Gemini, or the generic OpenAI-compatible adapter)
3. **Subsystem Opt-In** - Enable only the subsystems you need (web servers, content processing, notifications)
4. **Infrastructure Selection** - Pick storage/queue adapters (Redis, S3/MinIO, Qdrant)
5. **Testing Flexibility** - Enable integration tests only when needed

### Default vs. Full

| Configuration | Features Enabled | Use Case |
|--------------|------------------|----------|
| **Default** | `llm-openai`, `llm-anthropic`, `llm-deepseek` | Production orchestration with any of the three original providers |
| **Full** | All optional features (`llm-all` plus every subsystem) | Development, testing, full functionality |
| **No Default** | Core framework only | Library usage, custom integrations |

> **This default has not changed.** The `llm-*` flags were rewired (see below) so each one
> now actually gates its adapter instead of being an inert stub, but the *compiled default
> provider set* is deliberately unchanged from before that fix — `openai` + `anthropic` +
> `deepseek`. See the `[Unreleased]` section of `CHANGELOG.md` for the fix this note refers
> to. No consumer action is required.

## Available Feature Flags

### LLM Provider Flags

| Flag | Dependencies | Modules Gated | Description |
|------|--------------|---------------|-------------|
| `llm-openai` | None (uses `reqwest`) | `paladin_llm::openai` | OpenAI GPT models (GPT-3.5, GPT-4, GPT-4-turbo, GPT-4o). Compiled by default. |
| `llm-anthropic` | None (uses `reqwest`) | `paladin_llm::anthropic` | Anthropic Claude models. Compiled by default. |
| `llm-deepseek` | None (uses `reqwest`) | `paladin_llm::deepseek` | DeepSeek models (DeepSeek-V3, DeepSeek-Chat). Compiled by default. |
| `llm-kimi` | None (uses `reqwest`) | `paladin_llm::kimi` | Kimi (Moonshot AI). Not compiled by default. |
| `llm-qwen` | None (uses `reqwest`) | `paladin_llm::qwen` | Qwen (Alibaba DashScope). Not compiled by default. |
| `llm-grok` | None (uses `reqwest`) | `paladin_llm::grok` | Grok (xAI). Not compiled by default. |
| `llm-ollama` | None (uses `reqwest`) | `paladin_llm::ollama` | Ollama (self-hosted, no credential required). Not compiled by default. |
| `llm-gemini` | None (uses `reqwest`) | `paladin_llm::gemini` | Gemini (Google) — bespoke `generateContent` protocol, not OpenAI-compatible. Not compiled by default. |
| `llm-openai-compatible` | None (uses `reqwest`) | `paladin_llm::openai_compatible` | Generic operator-configured adapter for any OpenAI-compatible endpoint not covered above. Not compiled by default. |
| `llm-all` | all nine flags above | All LLM adapters | Every supported LLM provider plus the generic OpenAI-compatible adapter |

> Vendor base URLs and default model IDs for Kimi/Qwen/Grok/Gemini were recorded from vendor
> documentation but have not been verified against a live endpoint in this environment.

### Subsystem Flags

| Flag | Dependencies | Modules Gated | Description |
|------|--------------|---------------|-------------|
| `vision` | None | Vision-related types, prompt builders | Enable vision capabilities for multimodal LLM interactions |
| `content-processing` | `pdf-extract`, `scraper`, `tiktoken-rs`, `rss` | Content extraction, tokenization | PDF parsing, web scraping, RSS feeds, token counting |
| `web-server` | `actix-web`, `axum` | REST API controllers, server setup | HTTP/REST API servers for user management and content delivery |
| `notifications` | `lettre`, `handlebars` | Email adapter, templating | Email notifications with template rendering |

### Storage & Queue Flags

> `paladin-storage`'s SQLite adapters are **always compiled** — the facade depends on
> `paladin-storage` unconditionally with its `sqlite` feature enabled, so there is no
> `storage-sqlite` facade flag to opt into.

| Flag | Dependencies | Modules Gated | Description |
|------|--------------|---------------|-------------|
| `redis-queue` | `redis` | `paladin-storage/redis-queue` | Redis-based async queue adapter |
| `redis-cache` | `redis` | `paladin-storage/redis-cache` | Redis-backed `NodeCachePort` adapter. Shares the `redis-queue` dependency; not part of `default`, `storage`, or `full`. |
| `s3-storage` | `rust-s3` | `paladin-storage/s3` | S3/MinIO file storage adapter |
| `openai-embeddings` | None | Embedding generation utilities | OpenAI embedding model support |
| `qdrant` | `qdrant-client` | Qdrant vector database adapter | Vector database for semantic search |
| `storage-mysql` | `sqlx` (mysql) | `paladin-storage/mysql` | MySQL-based persistent repository |
| `storage-postgres` | `sqlx` (postgres) | `paladin-storage/postgres` | PostgreSQL `WaypointPort` adapter. Not part of `default` or `full`'s implicit set beyond this explicit passthrough. |
| `storage` | `storage-mysql`, `storage-postgres` | Both non-SQLite storage adapters | Convenience flag enabling MySQL and PostgreSQL backends (SQLite is always on) |

### Observability & Admin Flags

| Flag | Dependencies | Modules Gated | Description |
|------|--------------|---------------|-------------|
| `otel` | `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` | OTLP trace export | Exports Paladin traces via OpenTelemetry OTLP. Not part of `default` or `full` — the default build must gain no OTel dependency. |
| `dev-ui` | None | `paladin-web/dev-ui` | Admin-only `GET /v1/dev-ui/threads/{id}` run-inspector HTML page. Not part of `default` or `full` — the default build must gain no dev-ui HTML page. |

### Special Build Flags

| Flag | Description |
|------|-------------|
| `vendored-openssl` | Statically compile OpenSSL from source. Used for cross-compiled release binaries that lack a target-arch system libssl. |

### CLI Flags

| Flag | Dependencies | Modules Gated | Description |
|------|--------------|---------------|-------------|
| `cli` | `clap`, `dialoguer`, `indicatif`, `console`, `serde_yaml` | `application::cli` | Command-line tooling for the `paladin-cli` binary |

Build the `paladin-cli` binary with:
```bash
cargo build --bin paladin-cli --features cli
```

### Testing Flags

| Flag | Dependencies | Modules Gated | Description |
|------|--------------|---------------|-------------|
| `integration-tests` | None | Integration test modules | Enable integration tests (Docker services required) |
| `live-api-tests` | None | Live API test modules | Tests requiring real API keys (OpenAI, Anthropic, DeepSeek) |

### Convenience Flags

| Flag | Enables | Description |
|------|---------|-------------|
| `full` | `llm-all`, `content-processing`, `web-server`, `notifications`, `storage`, `vision`, `redis-queue`, `s3-storage`, `openai-embeddings`, `qdrant`, `cli` | All optional features for development/testing. Deliberately excludes `otel`, `dev-ui` and `redis-cache`, each of which must stay opt-in. |

## Default Configuration

**Current Default:**

```toml
[dependencies]
paladin-ai = "0.10.0"
```

This enables:
- ✅ `llm-openai` - OpenAI LLM provider
- ✅ `llm-anthropic` - Anthropic LLM provider
- ✅ `llm-deepseek` - DeepSeek LLM provider
- ✅ Core framework (always available)

The six providers Phase 17 added (`llm-kimi`, `llm-qwen`, `llm-grok`, `llm-ollama`,
`llm-gemini`, `llm-openai-compatible`) are **not** in the default set — opt in explicitly
per-provider or via `llm-all`. See `CHANGELOG.md`'s `[Unreleased]` entry: the `llm-*` flags
were rewired to actually gate their adapters, but the compiled default provider set is
unchanged from before that fix.

See [migration-guide.md](migration-guide.md) for migration guidance.

## Usage Examples

### Minimal Build (Core Only)

No external LLM providers, storage, or queues:

```toml
[dependencies]
paladin-ai = { version = "0.10.0", default-features = false }
```

**Use case**: Custom LLM integrations, library embedding, edge deployments

### Single Provider Builds

**OpenAI Only** (default):
```toml
[dependencies]
paladin-ai = "0.10.0"
# Or explicitly:
paladin-ai = { version = "0.10.0", features = ["llm-openai"] }
```

**Anthropic Only**:
```toml
[dependencies]
paladin-ai = { version = "0.10.0", default-features = false, features = ["llm-anthropic"] }
```

**DeepSeek Only**:
```toml
[dependencies]
paladin-ai = { version = "0.10.0", default-features = false, features = ["llm-deepseek"] }
```

### Multi-Provider Builds

**All LLM Providers**:
```toml
[dependencies]
paladin-ai = { version = "0.10.0", default-features = false, features = ["llm-all"] }
```

**OpenAI + Anthropic**:
```toml
[dependencies]
paladin-ai = { version = "0.10.0", default-features = false, features = ["llm-openai", "llm-anthropic"] }
```

### Orchestration Platform Build

Agents + web API + Redis queue + S3 storage:

```toml
[dependencies]
paladin-ai = { version = "0.10.0", features = ["web-server", "redis-queue", "s3-storage"] }
```

### Content Processing Build

Content ingestion + processing + all providers:

```toml
[dependencies]
paladin-ai = { version = "0.10.0", features = ["llm-all", "content-processing", "qdrant", "s3-storage"] }
```

### Full Development Build

All features enabled:

```toml
[dependencies]
paladin-ai = { version = "0.10.0", features = ["full"] }
```

Or use the CLI:

```bash
cargo build --features full
cargo test --features full
```

### Production API Server

Web server + notifications + OpenAI + storage:

```toml
[dependencies]
paladin-ai = { version = "0.10.0", features = ["web-server", "notifications", "redis-queue", "s3-storage"] }
```

## Build Comparison

### Binary Size Comparison

| Configuration | Features | Dependencies | Approx. Binary Size* | Compile Time* |
|---------------|----------|--------------|---------------------|---------------|
| Core Only | None | ~50 crates | 8-12 MB | 30-45s |
| Default | `llm-openai` | ~55 crates | 10-14 MB | 40-60s |
| Full | All | ~120 crates | 25-35 MB | 3-5 min |

*Approximate values for release builds on x86_64 Linux. Actual values vary by system.

### Compile Time Optimization

**Fast iteration** (core only):
```bash
cargo build --no-default-features
cargo test --lib --no-default-features
```

**Full testing** (all features):
```bash
cargo test --features full
```

## Feature Dependencies

### Dependency Tree

```
full
├── llm-all
│   ├── llm-openai
│   ├── llm-anthropic
│   ├── llm-deepseek
│   ├── llm-kimi
│   ├── llm-qwen
│   ├── llm-grok
│   ├── llm-ollama
│   ├── llm-gemini
│   └── llm-openai-compatible
├── content-processing
│   ├── pdf-extract
│   ├── scraper
│   ├── tiktoken-rs
│   └── rss
├── web-server
│   ├── actix-web
│   └── axum
├── notifications
│   ├── lettre
│   └── handlebars
├── vision
├── redis-queue
│   └── redis
├── s3-storage
│   └── rust-s3
├── openai-embeddings
└── qdrant
    └── qdrant-client
```

### Conditional Compilation Examples

**In Your Code:**

```rust,ignore
// Always available (core framework)
use paladin::core::platform::container::paladin::Paladin;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;

// Conditionally compiled — the LLM adapters live in the paladin_llm crate;
// the facade kept no shim for the old, now-removed infrastructure-adapter module path.
#[cfg(feature = "llm-openai")]
use paladin_llm::openai::OpenAIAdapter;

#[cfg(feature = "redis-queue")]
use paladin::infrastructure::adapters::queue::redis::RedisQueueAdapter;

#[cfg(feature = "web-server")]
use paladin::infrastructure::web::server::start_web_server;
```

## Best Practices

### 1. Start Minimal, Add as Needed

Begin with default features, add others only when required:

```toml
# Start here
[dependencies]
paladin-ai = "0.10.0"

# Add features as needed
paladin-ai = { version = "0.10.0", features = ["redis-queue"] }
```

### 2. Use `full` for Development Only

Enable all features during development, but specify exact features for production:

```toml
[dependencies]
# Production - explicit features
paladin-ai = { version = "0.10.0", features = ["llm-anthropic", "s3-storage"] }

[dev-dependencies]
# Development - all features
paladin-ai = { version = "0.10.0", features = ["full"] }
```

### 3. Document Feature Requirements

If your application requires specific features, document them:

```rust,ignore
//! # Example Application
//!
//! **Required Features:**
//! ```toml
//! paladin-ai = { version = "0.10.0", features = ["llm-openai", "redis-queue", "s3-storage"] }
//! ```
```

### 4. Test with Multiple Feature Combinations

Use CI to test critical combinations:

```yaml
# .github/workflows/ci.yml
strategy:
  matrix:
    features:
      - "--no-default-features"
      - ""  # default
      - "--features full"
```

See [.github/workflows/](https://github.com/DF3NDR/paladin-dev-env/tree/main/.github/workflows) for Paladin's complete feature matrix testing.

### 5. Feature-Gate Examples

Add feature requirements to example documentation:

```rust,ignore
//! # Redis Queue Example
//!
//! **Required Cargo Features:**
//! ```toml
//! paladin-ai = { version = "0.10.0", features = ["redis-queue"] }
//! ```
//!
//! Run with: `cargo run --example redis_queue --features redis-queue`
```

## Migration Guide

If you're upgrading from a version before the feature flag reorganization, see [migration-guide.md](migration-guide.md) for detailed migration instructions.

## CI/CD Integration

### GitHub Actions

```yaml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features:
          - ""                              # default
          - "--no-default-features"         # core only
          - "--features full"               # all features
          - "--features llm-anthropic"      # specific provider
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Test
        run: cargo test ${{ matrix.features }}
```

### Docker Multi-Stage Builds

```dockerfile
# Builder with only needed features. Base image kept in sync with the real
# builder stage in `Dockerfile` — see that file for the authoritative pin.
FROM rust:1.93-slim-bookworm as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features "llm-openai,redis-queue,s3-storage"

# Runtime image
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/paladin /usr/local/bin/
CMD ["paladin"]
```

## Support

For issues or questions about feature flags:
- **Documentation**: [Configuration Guide](../getting-started/configuration.md)
- **Migration**: [Migration Guide](migration-guide.md)
- **Issues**: [GitHub Issues](https://github.com/DF3NDR/paladin-dev-env/issues)
- **Discussions**: [GitHub Discussions](https://github.com/DF3NDR/paladin-dev-env/discussions)
