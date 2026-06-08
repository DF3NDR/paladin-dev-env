# Paladin

[![CI](https://github.com/DF3NDR/paladin-dev-env/actions/workflows/ci.yml/badge.svg)](https://github.com/DF3NDR/paladin-dev-env/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/paladin-ai.svg)](https://crates.io/crates/paladin-ai)
[![docs.rs](https://img.shields.io/docsrs/paladin-ai)](https://docs.rs/paladin-ai)
[![docs: mdBook](https://img.shields.io/badge/docs-mdBook-blue.svg)](https://df3ndr.github.io/paladin-dev-env/)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

**Paladin is a Rust framework for building and orchestrating teams of AI agents.**

Paladin lets you define autonomous LLM agents (*Paladins*) and coordinate them with composable
orchestration patterns (*Battalions*) — sequential pipelines, parallel fan-out, DAG workflows,
hierarchical delegation, and an automatic strategy router. It is built on a clean Hexagonal
(ports-and-adapters) architecture, so providers, memory, storage, and tools are all swappable
behind traits. Multi-provider LLM support (OpenAI, Anthropic, DeepSeek), pluggable memory
(*Garrison* / *Sanctum*), tool execution (*Arsenal*), and a content-processing pipeline are
included.

## Quick Example

```rust
use std::sync::Arc;
use std::time::Duration;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::prelude::*; // PaladinBuilder, LlmPort, Paladin, ...

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // An offline mock LLM so this runs without an API key.
    // For real use: `Arc::new(OpenAIAdapter::from_env()?)`.
    let llm: Arc<dyn LlmPort> =
        Arc::new(MockLlmAdapter::new().with_response("Hello from Paladin!"));

    // Build an agent with the fluent builder.
    let agent = PaladinBuilder::new(llm.clone())
        .name("Greeter")
        .system_prompt("You are a friendly assistant.")
        .build()
        .await?;

    // Execute it and print the result.
    let breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let service = PaladinExecutionService::new(llm, breaker, None, None);
    let result = service
        .execute(&agent, "Say hello in one sentence.")
        .await?;

    println!("{}", result.output);
    Ok(())
}
```

> This snippet is compile-verified in CI (it lives in `crates/doc-examples/src/readme.rs`). See the
> [Quickstart](https://df3ndr.github.io/paladin-dev-env/getting-started/quickstart.html) for the
> end-to-end version with a real LLM provider.

## Key Features

- **Autonomous agents** — configurable Paladins with system prompts, models, stop-words, loop limits, and a fluent `PaladinBuilder`.
- **Battalion orchestration** — Formation (sequential), Phalanx (parallel), Campaign (DAG), Chain of Command (hierarchical), plus Conclave / Council / Grove and the **Commander** auto-router.
- **Multi-provider LLM** — OpenAI, Anthropic, and DeepSeek behind one `LlmPort` trait, with a mock adapter for tests.
- **Pluggable memory** — in-memory and SQLite *Garrison* (history) and a Qdrant-backed *Sanctum* (vector search).
- **Tools & content** — tool execution (*Arsenal*) and a content-processing pipeline (PDF, HTTP, news ingestion → AI analysis → delivery).
- **Hexagonal architecture** — dependencies flow inward only; every adapter is swappable behind a port trait.
- **Production-minded** — circuit breakers, retries, structured telemetry, job scheduling, and an event/trigger system.

## Crate Ecosystem

Most applications depend on the **`paladin-ai`** umbrella crate (library name `paladin`), which
re-exports the common types and gates each infrastructure crate behind a feature flag.

| Crate (package) | Purpose | Key feature flags |
|---|---|---|
| `paladin-ai` (umbrella, lib `paladin`) | Entry point; re-exports + feature gating | `llm-openai` *(default)*, `llm-all`, `redis-queue`, `s3-storage`, `qdrant`, `content-processing`, `web-server`, `notifications`, `storage`, `cli`, `full` |
| `paladin-ai-core` | Pure domain types (`Node<T>`, `Paladin`, `Battalion`, …) | — |
| `paladin-ports` | Port trait contracts (hexagonal interfaces) | — |
| `paladin-battalion` | Multi-agent orchestration runtime | — |
| `paladin-llm` | LLM provider adapters | `openai`, `anthropic`, `deepseek`, `mock`, `vision` |
| `paladin-memory` | Garrison (history) + Sanctum (vector) adapters | `sqlite`, `qdrant` |
| `paladin-storage` | SQL repository adapters | `sqlite`, `mysql` |
| `paladin-content` | Content ingestion & processing pipeline | `news-api`, `llm` |
| `paladin-notifications` | Notification delivery adapters | `email`, `push`, `system` |
| `paladin-web` | HTTP server layer (actix-web / axum) | — |

See the [Crate Map & Feature-Flag reference](https://df3ndr.github.io/paladin-dev-env/api-reference/crate-map.html)
for the full table, dependency graph, and copy-paste `Cargo.toml` profiles.

## Documentation

- **Guide (mdBook):** <https://df3ndr.github.io/paladin-dev-env/> — installation, orchestration, content processing, the agent↔orchestrator bridge, architecture, deployment, and operations.
- **API docs (docs.rs):** <https://docs.rs/paladin-ai>

## Getting Started

**Prerequisites:** Rust ≥ 1.85 (edition 2024). Docker is needed only for the optional services
(Redis, MinIO, MySQL, Qdrant) used by some adapters and integration tests.

Add Paladin to your project:

```toml
[dependencies]
paladin-ai = "0.5"                      # default features: ["llm-openai"]
tokio = { version = "1", features = ["full"] }
```

Then follow the [Quickstart](https://df3ndr.github.io/paladin-dev-env/getting-started/quickstart.html)
to build and run your first agent, or the
[Orchestration guide](https://df3ndr.github.io/paladin-dev-env/user-guides/orchestration.html) to
coordinate several.

### Running agents behind an HTTP API

To serve configured agents over HTTP (the *HTTP service-host* topology), copy
[`config.example.yml`](config.example.yml) to `config.yml`, edit its `agents:` section, and run
the `paladin-server` binary:

```bash
# API keys come from the environment, not the config file.
OPENAI_API_KEY=sk-... cargo run --bin paladin-server --features web-server
# or point at a specific config: PALADIN_CONFIG=./config.yml paladin-server
```

It exposes `GET/POST /agents`, `GET/DELETE /agents/{id}`, and `POST /agents/{id}/execute`, and
shuts down gracefully on Ctrl-C / SIGTERM. (Authentication arrives in a later milestone epic; put
it behind a trusted proxy for now.)

## Project Status

Current version: **0.5.1**. Stability guarantees and the public-API policy are documented in the
[Stable API reference](https://df3ndr.github.io/paladin-dev-env/api-reference/stable-api.html); see
[`CHANGELOG.md`](CHANGELOG.md) for release history.

## Contributing

Contributions are welcome. Please read the
[Contributing guides](https://df3ndr.github.io/paladin-dev-env/contributing/development-setup.html)
— development setup, testing, and architecture decisions — before opening a pull request. The
sources live in [`docs/src/contributing/`](docs/src/contributing/).

## License

Licensed under the [MIT License](LICENSE).
