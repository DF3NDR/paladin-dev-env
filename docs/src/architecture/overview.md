# Architecture Overview

Paladin is a **Rust workspace** of eleven library crates plus a facade, organised around
**Hexagonal Architecture** (Ports & Adapters) and **Domain-Driven Design**.
Each workspace crate maps to a distinct architectural layer, keeping the core
domain free of all external dependencies.

> For how to *run* agents built on this architecture — embedded, hosted, queue/worker, or
> sidecar — see [Deployment Topologies](../deployment-topologies/overview.md).

## Workspace Crates at a Glance

| Crate | Layer | Purpose |
|-------|-------|---------|
| `paladin-ai-core` | Core | Pure domain entities and base primitives |
| `paladin-ports` | Application boundary | Port trait contracts (interfaces) |
| `paladin-battalion` | Application services | Multi-agent orchestration patterns, WarEngine superstep execution |
| `paladin-llm` | Infrastructure | LLM provider adapters (OpenAI, Anthropic, DeepSeek), Commissary token rationing |
| `paladin-memory` | Infrastructure | Garrison and Sanctum memory adapters |
| `paladin-storage` | Infrastructure | SQL repository adapters (SQLite, MySQL) |
| `paladin-notifications` | Infrastructure | Email, push, system notification adapters |
| `paladin-content` | Infrastructure | Content ingestion and processing adapters |
| `paladin-web` | Infrastructure | HTTP server (actix-web / axum), Platform API REST surface |
| `paladin-herald` | Infrastructure | Concrete Herald output-formatter adapters (JSON, Markdown, Table) |
| `paladin-eval` | Composition tool | Deterministic evaluation harness — scenario files, scripted `LlmPort`, trace assertions |
| `paladin-ai` *(root)* | Umbrella / facade | Re-exports all crates; workspace feature flags |

## Three-Layer Hexagonal Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      External World                              │
│   LLMs · Databases · Redis · MinIO · MCP tools · HTTP clients   │
└──────────────┬──────────────────────────────────┬───────────────┘
               │                                  │
               │  Infrastructure adapters         │
               │  paladin-llm                     │
               │  paladin-memory                  │
               │  paladin-storage                 │
               │  paladin-notifications            │
               │  paladin-content                 │
               │  paladin-web                     │
               │                                  │
┌──────────────▼──────────────────────────────────▼───────────────┐
│               Application Boundary  (paladin-ports)              │
│   LlmPort · GarrisonPort · SanctumPort · ArsenalPort            │
│   CitadelPort · FileStoragePort · NotificationPort · …          │
│                                                                  │
│               Application Services  (paladin-battalion)          │
│   FormationService · PhalanxService · CampaignService           │
│   ChainOfCommandService · Commander · ConclaveService           │
│   CouncilService · GroveService · ManeuverService               │
└──────────────┬──────────────────────────────────┬───────────────┘
               │  depends on (inward only)         │
┌──────────────▼──────────────────────────────────▼───────────────┐
│                   Core Domain  (paladin-ai-core)                  │
│   Paladin · Battalion · Garrison · Arsenal · Citadel             │
│   Herald · Sanctum · Node<T> · PaladinError · …                 │
│   No I/O · No external SDK imports · Pure domain logic           │
└─────────────────────────────────────────────────────────────────┘
```

### Dependency Flow Rule

**Dependencies flow inward only:**

- `paladin-ai-core` imports nothing from the workspace.
- `paladin-ports` imports only `paladin-ai-core`.
- `paladin-battalion` imports `paladin-ai-core` + `paladin-ports`.
- Infrastructure crates (`paladin-llm`, `paladin-memory`, etc.) import
  `paladin-ai-core` + `paladin-ports`. They never import each other.
- The root `paladin-ai` umbrella crate imports everything.

This rule is enforced by Cargo's dependency graph — `paladin-ai-core` cannot
accidentally pull in `reqwest` or `sqlx`.

## Layer 1: Core Domain (`crates/paladin-core`)

**Package name:** `paladin-ai-core`

Pure business logic with zero external dependencies.

```
crates/paladin-core/src/
├── base/                      # Framework primitives
│   ├── node.rs                # Node<T> entity pattern
│   ├── collection.rs
│   ├── field.rs
│   └── message.rs
└── platform/
    ├── container/
    │   ├── paladin.rs             # Paladin aggregate root
    │   ├── paladin_config.rs
    │   ├── paladin_error.rs
    │   ├── garrison.rs            # Garrison memory domain
    │   ├── arsenal/               # Tool system domain
    │   ├── citadel.rs             # State persistence domain
    │   ├── herald.rs              # Output formatting domain
    │   ├── sanctum.rs             # Vector memory domain
    │   └── battalion/             # Battalion domain types
    └── manager/
        ├── scheduler.rs
        └── event_manager.rs
```

**Constraints:**
- No imports from `paladin-ports` or any infrastructure crate
- No I/O operations
- No HTTP clients, database drivers, or LLM SDKs

## Layer 2: Application Boundary (`crates/paladin-ports` + `crates/paladin-battalion`)

### Port Contracts (`paladin-ports`)

Defines abstract `trait` interfaces for every external integration point:

```
crates/paladin-ports/src/
├── output/
│   ├── llm_port.rs              # LLM provider abstraction
│   ├── garrison_port.rs         # Memory CRUD operations
│   ├── sanctum_port.rs          # Vector memory search
│   ├── arsenal_port.rs          # Tool invocation
│   ├── citadel_port.rs          # State persistence
│   ├── file_storage_port.rs     # File upload/download
│   ├── notification_port.rs     # Alert delivery
│   ├── queue_port.rs            # Async task queue
│   └── …
└── input/
    ├── content_delivery_port.rs
    └── …
```

### Orchestration Services (`paladin-battalion`)

```
crates/paladin-battalion/src/
├── formation_service.rs         # Sequential pipeline (N→N+1)
├── phalanx_service.rs           # Concurrent (parallel) execution
├── campaign_service.rs          # DAG / graph-based execution
├── chain_of_command_service.rs  # Hierarchical delegation
├── conclave_execution_service.rs # Mixture-of-experts synthesis
├── council_service.rs           # Multi-agent discussion
├── grove_service.rs             # Semantic routing
├── maneuver/                    # Flow DSL (parser + runtime)
└── commander.rs                 # Auto-detect strategy router
```

## Layer 3: Infrastructure Adapters

| Crate | Key adapters |
|-------|-------------|
| `paladin-llm` | `OpenAIAdapter`, `AnthropicAdapter`, `DeepSeekAdapter`, `MockLlmAdapter` |
| `paladin-memory` | `InMemoryGarrison`, `SqliteGarrison`, `InMemorySanctum`, `QdrantSanctumAdapter` |
| `paladin-storage` | `SqliteContentRepository`, `MySqlContentRepository`, `SqliteUserRepository` |
| `paladin-notifications` | `EmailNotificationAdapter`, `PushNotificationAdapter`, `SystemNotificationAdapter` |
| `paladin-content` | HTTP/file fetcher, RSS ingestion, document parsing, LLM analysis pipeline |
| `paladin-web` | actix-web/axum HTTP server, RBAC middleware, user REST API |

Each adapter implements the corresponding port trait from `paladin-ports`.

## System Components

### Paladin (Agent)

```
Create via PaladinBuilder
        │
        ▼
   ┌─────────┐
   │  Idle   │ ← waiting for input
   └────┬────┘
        │  execute()
        ▼
   ┌─────────┐
   │ Running │ ← LLM reasoning loop (1..max_loops)
   └────┬────┘
        ├── tool call? → Arsenal.invoke() → inject result → continue
        ├── stop word? → StopWordDetected
        └── max loops? → MaxLoops
```

> The "tool call?" branch is taken only when the `LlmPort` implementation in use returns a
> populated function call from `generate()` — no shipped adapter (OpenAI, Anthropic,
> DeepSeek, or the bundled mock) ever does. See the [Tool Integration
> Guide](../user-guides/tool-integration.md#overview) and ADR-0042 for the tracked reachability
> status.

### Battalion (Orchestration)

Eight patterns routed by the Commander auto-detector:

| Pattern | Crate module | When to use |
|---------|-------------|-------------|
| Formation | `formation_service` | Strict sequential pipeline |
| Phalanx | `phalanx_service` | Independent parallel tasks |
| Campaign | `campaign_service` | DAG dependencies |
| Chain of Command | `chain_of_command_service` | Hierarchical delegation |
| Conclave | `conclave_execution_service` | Expert synthesis |
| Council | `council_service` | Multi-agent discussion |
| Grove | `grove_service` | Semantic routing |
| Maneuver | `maneuver/` | Flow DSL expressions |

### Garrison (Short-term Memory)

Conversation history stored in `paladin-memory`:
- `InMemoryGarrison` — always available, zero deps
- `SqliteGarrison` — persistent (feature `sqlite`)

Configured via `garrison:` section in `config.yml`.

### Sanctum (Long-term Vector Memory)

Semantic memory in `paladin-memory`:
- `InMemorySanctum` — in-process (testing / dev)
- `QdrantSanctumAdapter` — production (feature `qdrant`)

### Arsenal (Tool System)

MCP-compatible tool registry. Connects to external tools via:
- STDIO process servers (command-line tools)
- SSE HTTP servers (web services)

Configured via `arsenal.mcp_servers` in `config.yml`.

### Sentinel (Vision System)

Multimodal capability layer, extending Paladin's reasoning loop to analyze images and process
documents alongside text — enabled via `PaladinBuilder::enable_vision`. Sentinel follows the same
hexagonal shape as the rest of the framework: a vision port abstraction in the application boundary,
implemented by provider adapters in `paladin-llm` (OpenAI, Anthropic, DeepSeek vision-capable
models), with encryption-at-rest and automatic memory cleanup for sensitive visual data at the
infrastructure edge.

See [Sentinel](../appendix/sentinel.md) for the full reference — content types, supported providers,
document processing, CLI usage, YAML configuration, security, and Battalion integration.

### WarEngine (Superstep Execution)

A cyclic graph execution engine in `paladin-battalion`: a `WarGraph` of nodes runs in
synchronized supersteps over a typed, schema-declared `Battlefield` shared state, checkpointing
a `Waypoint` after each superstep so a run resumes with zero re-execution of completed work. See
[WarEngine: Battlefield State & Superstep Execution](../user-guides/superstep-engine.md).

### Aegis (Fault Tolerance)

Per-node retry, timeout, error-handler, model-fallback and caching policy, attached as a
`NodeId`-keyed sidecar on a `WarGraph` rather than baked into any node spec. See [Aegis: Retry,
Timeout, Error Handlers, Model Fallback and Node Caching](../user-guides/fault-tolerance.md).

### Commissary (Token-Budget Rationing)

The input-side, per-call window-rationing officer in `paladin-llm`: a pre-flight
`verify_fits` guard plus a bounded `dispense` allocator over caller-prioritised material. See
[Commissary](commissary.md).

### Observability (TraceRecord)

Every superstep engine and facade-middleware event is enqueued as a `TraceRecord` envelope
around a twelve-variant `TraceEvent`, delivered to configured sinks. See
[Observability: Traces, Sinks and Persistence](../operations/observability.md).

### Platform API

An HTTP route surface (`paladin-web`) for Runs, Threads, Assistants, Schedules and Webhooks. See
[Platform API](../api-reference/platform-api.md).

## Technology Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (edition 2024, MSRV 1.88) |
| Async runtime | Tokio |
| HTTP client | reqwest |
| Serialization | serde / serde_json / serde_yaml |
| LLM providers | OpenAI, Anthropic, DeepSeek APIs |
| Vector DB | Qdrant (optional) |
| Relational DB | SQLite, MySQL (optional) |
| Cache / Queue | Redis (optional) |
| Object storage | MinIO / S3 (optional) |
| Web framework | actix-web / axum |
| Error handling | thiserror / anyhow |
| Build / test | Cargo, nextest, cargo-tarpaulin |
| Docs | mdBook, mdbook-mermaid, mdbook-linkcheck |

## See Also

- [Hexagonal Design](hexagonal-design.md) — port and adapter patterns in detail
- [Domain Model](domain-model.md) — all domain entities and the Medieval Military naming convention
- [Crate Map](crate-map.md) — workspace crate dependency graph
- [Design Patterns](design-patterns.md) — `PaladinBuilder`, error types, port traits
