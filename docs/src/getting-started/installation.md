# Installation

This guide covers adding Paladin to an existing Rust project or setting up the
Paladin workspace for development.

## Prerequisites

### Required

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| **Rust** | 1.88.0 | Latest stable (1.95+) |
| **Cargo** | Included with Rust | - |
| **Edition** | 2024 | 2024 |
| **LLM API Key** | At least one | - |

> **Why Rust >= 1.88?** Paladin uses edition 2024 features. Verify your toolchain:
> ```bash
> rustc --version   # should print >= 1.88.0
> ```
> Update with `rustup update stable`.

### Optional (for Docker-based services)

- **Docker + Docker Compose** v2 -- required for the built-in Redis, MinIO, and
  MySQL services (see [Docker Guide](../deployment/docker.md))

## Installing Rust

```bash
# Install rustup and the stable toolchain
curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build tools -- Linux (Ubuntu/Debian)
sudo apt-get install -y build-essential pkg-config libssl-dev

# Build tools -- macOS (via Homebrew)
brew install openssl pkg-config
```

Windows users should use [rustup-init.exe](https://rustup.rs/) or WSL 2.

## Adding Paladin to a Rust Project

### Cargo.toml -- choose your crates

Paladin v0.10.0 is published as a workspace of focused crates. Add only what you need:

```toml
[dependencies]
# Core framework -- always required
paladin-ai-core   = "0.10.0"
paladin-ports     = "0.10.0"

# LLM providers (pick one or more)
paladin-llm       = { version = "0.10.0", features = ["llm-openai"] }

# Multi-agent orchestration (optional)
paladin-battalion = "0.10.0"

# Memory / Garrison (optional)
paladin-memory    = "0.10.0"

# Storage adapters (optional)
paladin-storage   = "0.10.0"

# Async runtime (required)
tokio = { version = "1", features = ["full"] }
```

### Umbrella crate

The `paladin-ai` umbrella crate (v0.10.0) re-exports everything and accepts workspace feature flags:

```toml
[dependencies]
paladin-ai = { version = "0.10.0", features = ["redis-queue", "s3-storage"] }
tokio      = { version = "1", features = ["full"] }
```

### Feature Flag Profiles

A minimal profile for the common getting-started case -- the three default LLM providers plus
the adapters most guides exercise:

| Flag | Default | Description |
|------|---------|-------------|
| `llm-openai` | yes | OpenAI GPT adapter |
| `llm-anthropic` | yes | Anthropic Claude adapter |
| `llm-deepseek` | yes | DeepSeek adapter |
| `redis-queue` | no | Redis async task queue |
| `s3-storage` | no | MinIO / AWS S3 file storage |
| `openai-embeddings` | no | OpenAI embedding API |
| `qdrant` | no | Qdrant vector database for Sanctum |

#### Full feature inventory

Regenerated from the facade `Cargo.toml` `[features]` block -- every shipped flag, the crate it
forwards into, and what it gates. `default = ["llm-openai", "llm-anthropic", "llm-deepseek"]`.

| Flag | Crate | Gates |
|------|-------|-------|
| `llm-openai` | paladin-llm | OpenAI adapter (default) |
| `llm-anthropic` | paladin-llm | Anthropic adapter (default) |
| `llm-deepseek` | paladin-llm | DeepSeek adapter (default) |
| `llm-kimi` | paladin-llm | Kimi adapter |
| `llm-qwen` | paladin-llm | Qwen adapter |
| `llm-grok` | paladin-llm | Grok adapter |
| `llm-ollama` | paladin-llm | Ollama adapter |
| `llm-gemini` | paladin-llm | Gemini adapter |
| `llm-openai-compatible` | paladin-llm | Generic OpenAI-compatible adapter |
| `llm-all` | paladin-llm | Aggregate: all nine LLM provider adapters above |
| `vision` | paladin-llm | Vision / multimodal support (forwards into `paladin-llm/vision`, requires `llm-openai`) |
| `content-processing` | paladin-content, paladin-memory | Content ingestion: PDF, HTTP, RSS, news, summarization, LLM bridge |
| `web-server` | paladin-web | HTTP/REST API surface (Axum) |
| `notifications` | paladin-notifications | Email, push, and system notification adapters |
| `storage-mysql` | paladin-storage | MySQL repository adapters |
| `storage-postgres` | paladin-storage | PostgreSQL `WaypointPort` adapter |
| `storage` | paladin-storage | Aggregate: `storage-mysql` + `storage-postgres` |
| `redis-queue` | paladin-storage | Redis async task queue |
| `redis-cache` | paladin-storage | Redis-backed `NodeCachePort` adapter |
| `s3-storage` | paladin-storage | MinIO / AWS S3 file storage |
| `openai-embeddings` | paladin-llm | OpenAI embedding API |
| `qdrant` | paladin-memory | Qdrant vector database for Sanctum |
| `otel` | facade (`paladin-ai`) | OTLP trace export |
| `dev-ui` | paladin-web | Admin-only `dev-ui` run inspector page |
| `cli` | facade, binary only | Builds the `paladin-cli` binary and its dependencies (clap, dialoguer, indicatif, ...) |
| `integration-tests` | facade | Gate for integration test suites that require backing services |
| `live-api-tests` | facade | Gate for tests that require real provider API keys |
| `full` | facade | Aggregate: `llm-all` + `content-processing` + `web-server` + `notifications` + `storage` + `vision` + `redis-queue` + `s3-storage` + `openai-embeddings` + `qdrant` + `cli` -- deliberately excludes `otel`, `dev-ui`, and `redis-cache` |

The `paladin-cli` binary carries `required-features = ["cli"]` in `Cargo.toml`, so it is **not**
built by a default `cargo build` -- pass `--features cli` (or `--bin paladin-cli --features cli`)
to build it.

### Verification

```bash
cargo check
```

No errors means all selected features resolved correctly.

## Cloning the Source for Development

```bash
# 1. Clone
git clone https://github.com/DF3NDR/paladin-dev-env.git
cd paladin-dev-env

# 2. Build the workspace
cargo build

# 3. Run unit tests
cargo test --workspace --lib

# 4. (Optional) Start backing services
make services-up   # Redis, MinIO, MySQL via Docker Compose
```

See [Development Setup](../contributing/development-setup.md) for the full contributor workflow.

## Environment Variables for LLM Keys

Paladin reads API keys exclusively from environment variables -- **never put keys in config files**.

```bash
# Set at least one provider key before running
export OPENAI_API_KEY="sk-..."       # OpenAI
export DEEPSEEK_API_KEY="sk-..."     # DeepSeek
export ANTHROPIC_API_KEY="sk-..."    # Anthropic
```

Copy `.env.example` to `.env` for local development (`.env` is git-ignored).

## Next Steps

- **[Quickstart](quickstart.md)** -- write your first Paladin agent in minutes
- **[Configuration](configuration.md)** -- full `config.yml` schema reference
- **[User Guides](../user-guides/paladin-agents.md)** -- in-depth agent patterns
