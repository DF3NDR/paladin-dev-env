# PRD: Configurable Web Host & Server Binary (Milestone 12, Epic 2)

**Project:** Paladin Framework
**Milestone:** 12 — Web API / HTTP Service Host Topology, Out of the Box
**Epic:** 2 — Configurable Web Host & Server Binary
**Version Target:** v0.6.0 (Unreleased)
**Status:** Ready for Implementation
**Created:** 2026-06-08
**Author:** AI Coding Agent (Claude Code)
**Depends on:** Milestone 12 Epic 1 (agent registry & execution API in `paladin-web`)

---

## 1. Introduction / Overview

Milestone 12 Epic 1 built the agent-execution HTTP surface inside `paladin-web`: an
[`AgentRegistry`](../Epic_1/prd-agent-registry-execution-api.md), the five `/agents/*` routes
(`agent_router`), the `AgentApiState`, and the `AgentProvisioner` *seam* for runtime
registration. But none of it is **runnable** yet. The registry is built from an in-memory list
only in tests; there is no concrete provisioner; and there is no process that loads
configuration, builds agents, and serves the router. A consumer still cannot "run a Paladin HTTP
service" without writing Rust.

**This Epic makes the topology runnable with no Rust required.** It adds a `host` + `agents`
configuration schema, a builder that turns that config into a populated `AgentRegistry`, a
**concrete `AgentProvisioner`** (so `POST /agents` works at runtime), and a shipped
**`paladin-server` binary** that loads config, composes the agent router, and serves it with
graceful shutdown. The outcome: `paladin-server` + a `config.yml` = a running instance that
executes configured agents over HTTP.

### Scope decisions (from PRD clarification)

- **Binary home:** a new `[[bin]]` in the **facade crate `paladin-ai`** at
  `src/bin/paladin-server.rs`, behind `required-features = ["web-server"]` — mirroring the
  existing `src/bin/paladin-cli.rs`. The facade is the composition root and may depend on both
  `paladin-web` and the execution/LLM machinery; `paladin-web` gains nothing new.
- **Router scope:** **agent API only** (`agent_router`). No user-management, auth, or
  content-delivery wiring — the server boots from config alone. (Auth → Epic 5; full-app
  composition is out of scope here.)
- **Agent capabilities:** **LLM + prompt only** — provider/model, system prompt, temperature,
  max-loops, stop words. No garrison (memory) or arsenal (tools) wiring in this Epic.
- **Runtime registration:** the concrete `AgentProvisioner` **is wired**, so `POST /agents`
  builds and registers agents live (not `501`).

---

## 2. Goals

1. A consumer can start a Paladin agent HTTP service with **only** a `config.yml` and the
   `paladin-server` binary — writing no Rust.
2. A `host` section (bind address) and an `agents` list are loaded via the existing `Settings`
   configuration system (`config.yml` + `APP_*` env overrides; API keys from env, never the file).
3. A **registry builder** turns the `agents` config into a populated `AgentRegistry`, each agent
   backed by an LLM provider resolved through the existing provider factory and executed via
   `PaladinExecutionService`.
4. A concrete **`AgentProvisioner`** (in the facade) lets `POST /agents` build and register agents
   at runtime using the same logic as config load.
5. The `paladin-server` binary loads config → builds the registry + provisioner → composes
   `agent_router` → binds and serves with **graceful shutdown** on SIGINT/SIGTERM.
6. Startup **fails fast** with actionable errors on invalid config, and logs the bound address and
   served routes on success.
7. All new code compiles warning-free and passes `cargo fmt`/`clippy -D warnings`/`cargo test`;
   the new logic has unit + a smoke integration test.

---

## 3. User Stories

- **As an operator**, I want to write a `config.yml` listing my agents and run one binary, so I
  get an HTTP service that executes those agents — without compiling anything myself.
- **As an integrator**, I want the server to bind the address from config and log the routes it
  serves, so I know where to send requests.
- **As an operator**, I want the server to refuse to start (with a clear message) if an agent
  names an unknown provider or the bind address is malformed, so misconfiguration fails at boot,
  not mid-request.
- **As an API client**, I want `POST /agents` to register a new agent at runtime against the
  running server, so I can add agents without a restart.
- **As an operator**, I want `Ctrl-C` / `SIGTERM` to drain and shut the server down cleanly, so
  deployments and restarts don't drop work abruptly.

---

## 4. Functional Requirements

### 4.1 Configuration schema

1. The system **must** extend the existing configuration (`src/config/settings.rs` / `config.yml`,
   loaded via `Settings`) with a **`host`** section containing at least `bind_address`
   (e.g. `"0.0.0.0:3000"`), and an **`agents`** section: a list of agent definitions.
2. Each agent definition **must** support: `id` (required, unique), `system_prompt` (required),
   `model` (required), and optional `provider`, `temperature`, `max_loops`, and `stop_words`.
   When `provider` is omitted it **must** default to the configured `llm.default_provider`.
3. API keys and secrets **must not** be read from the `agents`/`host` config; they continue to come
   from environment variables via the existing `llm` provider configuration.
4. Config loading **must** keep working with the existing `APP_*` environment-variable override
   mechanism and the existing `config.yml` / `config.test.yml` precedence.
5. Missing `host`/`agents` sections **must** be handled deterministically: either a sensible
   default (e.g. bind `127.0.0.1:3000`, zero agents) or a clear startup error — see Open Q1.

### 4.2 Registry-from-config builder

6. The system **must** provide a builder (in the facade) that, given the loaded config, produces a
   populated `paladin_web::AgentRegistry`. For each agent definition it **must**:
   - resolve an `Arc<dyn LlmPort>` for the agent's provider via the existing provider factory;
   - build a `Paladin` via `PaladinBuilder` using the agent's prompt/model/temperature/loops/stops;
   - construct a `PaladinExecutionService` (`new(llm, circuit_breaker, None, None)` — no garrison /
     arsenal) and use it as the agent's `Arc<dyn PaladinExecutorPort>`;
   - insert the `(id, Paladin, executor)` triple into the registry.
7. A duplicate `id` in the `agents` list **must** be a startup error (not silently dropped or
   overwritten).
8. A failure to resolve a provider or build an agent **must** surface as a descriptive startup
   error naming the offending agent `id`.

### 4.3 Concrete `AgentProvisioner`

9. The system **must** implement a concrete `AgentProvisioner` (in the facade) whose
   `provision(&AgentSpec)` builds a `(Paladin, Arc<dyn PaladinExecutorPort>)` pair using the **same
   logic** as the config builder (§4.2), mapping build/provider failures to `ProvisionError`.
10. The provisioner **must** be attached to the `AgentApiState` via `with_provisioner`, so
    `POST /agents` returns `201`/`409`/`422`/`400` (not `501`) on the running server.

### 4.4 Server binary

11. The system **must** add a `[[bin]]` named `paladin-server` at `src/bin/paladin-server.rs` in the
    facade crate, gated by `required-features = ["web-server"]`.
12. On startup the binary **must**, in order: load config → build the registry (§4.2) → build the
    `AgentApiState` with the concrete provisioner (§4.3) → compose `agent_router(state)` → bind a
    `TcpListener` to the configured `bind_address` → `axum::serve` it.
13. The binary **must** implement **graceful shutdown**: on `SIGINT` (Ctrl-C) and, on Unix,
    `SIGTERM`, it stops accepting new connections and lets in-flight requests finish before exiting.
14. The binary **must** initialize logging/tracing consistent with the rest of the workspace and
    **must not** print secrets.

### 4.5 Startup validation & diagnostics

15. The binary **must** validate config at boot and **fail fast** (non-zero exit, clear message)
    when: the `bind_address` is unparseable, an agent names an unknown/unavailable provider, a
    required agent field is missing, or an `id` is duplicated.
16. On successful startup the binary **must** log the bound address and a summary of what it serves
    (the agent route paths and the number/ids of agents loaded).

### 4.6 Quality & tests

17. Every new public item **must** have rustdoc.
18. The system **must** include: config-parsing tests (valid config; invalid/missing fields), a
    registry-builder test (builds the expected agents; duplicate-id and unknown-provider errors),
    and a `provision` test for the concrete provisioner.
19. The system **must** include a **smoke integration test** that boots the server on an ephemeral
    port (`127.0.0.1:0`) with a config using a mock/test provider, then asserts `GET /agents`
    responds and `POST /agents/{id}/execute` returns `200`. (Use a hermetic provider — no real
    network/API calls.)

---

## 5. Non-Goals (Out of Scope)

- **User-management / auth / content-delivery routes** — Epic 2 mounts the agent API only.
- **Authentication / authorization** on agent routes — Epic 5.
- **Garrison (memory) and Arsenal (tools/MCP)** wiring for agents — a later enhancement; agents are
  LLM + prompt only here.
- **Streaming / SSE and async jobs** — Epic 3.
- **Health/readiness endpoints, CORS, rate limiting, unified error model** — Epic 4.
- **OpenAPI / Swagger UI** — Epic 6.
- **TLS termination, Dockerfile, k8s manifests** — Epic 7 (the server binds plain HTTP; TLS is
  expected to be terminated by a proxy/ingress).
- **Hot-reload of `config.yml`** — config is read once at startup.
- Changing `paladin-web` (Epic 1 is reused as-is) or the LLM/execution APIs.

---

## 6. Design Considerations

### Config shape (illustrative)

```yaml
host:
  bind_address: "0.0.0.0:3000"

agents:
  - id: "researcher"
    provider: "openai"        # optional; defaults to llm.default_provider
    model: "gpt-4"
    system_prompt: "You research topics thoroughly."
    temperature: 0.7
  - id: "summarizer"
    model: "gpt-4"
    system_prompt: "You write concise summaries."

# Existing `llm:` section (providers, base URLs, defaults) is reused as-is;
# API keys come from OPENAI_API_KEY / DEEPSEEK_API_KEY / ANTHROPIC_API_KEY.
```

### Startup flow

```text
load Settings (config.yml + APP_* env)
      │
      ▼
build AgentRegistry  ── for each agent: provider factory → LlmPort
      │                                  PaladinBuilder    → Paladin
      │                                  PaladinExecutionService (no garrison/arsenal)
      ▼
AgentApiState::new(registry).with_provisioner(FacadeProvisioner)
      │
      ▼
agent_router(state) → TcpListener::bind(bind_address) → axum::serve(..).with_graceful_shutdown(..)
      │
      ▼
log "serving N agents on <addr>: GET/POST /agents, …"
```

### Run command (target experience)

```bash
OPENAI_API_KEY=sk-... cargo run --bin paladin-server --features web-server
# or, with an explicit config:
APP_CONFIG=./config.yml paladin-server
```

---

## 7. Technical Considerations

- **Crate / layer:** all new code in the **facade crate `paladin-ai`** (the composition root):
  the binary at `src/bin/paladin-server.rs`, and the registry builder + concrete `AgentProvisioner`
  in a facade module (e.g. `src/infrastructure/web/` or `src/application/services/web/`). This is
  the only place allowed to depend on both `paladin-web` and `PaladinExecutionService`.
- **Reused building blocks (verified):**
  - Config: `src/config/settings.rs` `Settings` (`Settings::new()` multi-source load,
    `load_from_file`), with an existing `llm: Option<LlmConfig>` and a host-style section to extend.
  - LLM: the provider factory in `paladin-llm` (`create(provider_name) -> Result<Arc<dyn LlmPort>,
    _>`, `get_default_provider`, `list_available_providers`) — keys from env.
  - Execution: `PaladinExecutionService::new(llm_port, circuit_breaker, garrison: Option, arsenal:
    Option)` — pass `None, None`; `CircuitBreaker` from `infrastructure::resilience`.
  - Agent build: `PaladinBuilder` (`.name().system_prompt().model()... .build().await`).
  - Web: `paladin_web::{AgentRegistry, AgentApiState, AgentProvisioner, AgentSpec, ProvisionError,
    agent_router}` (Epic 1).
- **Feature gating:** the binary requires `web-server`; provider availability follows the existing
  `llm-openai` / `llm-anthropic` / `llm-deepseek` feature flags. Requesting a provider whose feature
  is disabled is a startup error (§4.15).
- **Graceful shutdown:** `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal())`,
  where `shutdown_signal()` awaits `tokio::signal::ctrl_c()` and (Unix) a `SIGTERM` stream.
- **Shared circuit breaker:** a single `Arc<CircuitBreaker>` may be shared across agents (as the
  doc-example does) unless per-agent isolation is later required.
- **No new `paladin-web` dependency on the facade** — the dependency direction is preserved
  (facade → paladin-web, never the reverse).
- **Docs example:** `crates/doc-examples/src/http_service_host.rs` already demonstrates the
  registry + serve shape; the binary generalizes it to be config-driven.

---

## 8. Success Metrics

1. With a sample `config.yml` and a provider key set, `cargo run --bin paladin-server --features
   web-server` boots, logs the bound address + agent ids, and serves `GET /agents`.
2. `POST /agents/{id}/execute` against the running server returns the agent's output as JSON.
3. `POST /agents` against the running server registers a new agent at runtime (`201`), which is
   then visible via `GET /agents`.
4. Invalid config (bad bind address, unknown provider, duplicate id, missing field) makes the
   binary exit non-zero with a clear, specific message — verified by tests where feasible.
5. `Ctrl-C` / `SIGTERM` shuts the server down without dropping in-flight requests.
6. `cargo test` (incl. the smoke test), `cargo fmt --check`, `cargo clippy -- -D warnings`, and
   `make deny` are green; the facade's API-surface check still passes (binary/bin-only additions
   don't change the library surface).

---

## 9. Open Questions

1. **Empty/missing config:** if `host`/`agents` are absent, should the server start with a default
   bind and zero agents (useful for then `POST /agents`-ing them in), or refuse to start? (Default
   assumption: start with `127.0.0.1:3000` and zero agents; log a warning.)
2. **`bind_address` key & precedence:** confirm the exact config key/section name and how it relates
   to any existing `host` field already in `Settings` (avoid duplicating an existing server
   section).
3. **Per-agent provider/model defaulting:** confirm the resolution order — agent `provider` →
   `llm.default_provider`; agent `model` → provider's `default_model` when omitted.
4. **Hermetic smoke-test provider:** which mock/test `LlmPort` to use for the boot test
   (`MockLlmAdapter` is exported by the facade) and whether it's selectable via config (e.g.
   `provider: "mock"`) or injected in the test harness only.
5. **Config-load strictness:** reject unknown fields in the `agents`/`host` sections (serde
   `deny_unknown_fields`) or ignore them for forward-compatibility? (Default assumption: lenient.)

---

*Next step: run `/generate-tasks` against this PRD to produce
`tasks-configurable-web-host-server-binary.md` in this `Epic_2/` folder.*
