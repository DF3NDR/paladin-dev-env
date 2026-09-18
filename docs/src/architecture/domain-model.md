# Domain Model

This document describes all domain entities in `paladin-ai-core` and the
Medieval Military naming convention that provides Paladin's ubiquitous language.

## Medieval Military Naming Convention

Paladin applies Domain-Driven Design's ubiquitous language principle through a
consistent Medieval Military theme. Use these terms in all code, documentation,
and discussions:

| Term | DDD Concept | Rust Type / Location |
|------|-------------|---------------------|
| **Paladin** | Aggregate root — autonomous AI agent | `Paladin` · `crates/paladin-core/src/platform/container/paladin.rs` |
| **Battalion** | Aggregate — coordinated group of Paladins | `Battalion` types · `crates/paladin-core/src/platform/container/battalion/` |
| **Formation** | Sequential execution pattern | `FormationService` · `crates/paladin-battalion/src/formation_service.rs` |
| **Phalanx** | Concurrent execution pattern | `PhalanxService` · `crates/paladin-battalion/src/phalanx_service.rs` |
| **Campaign** | Graph / DAG execution pattern | `CampaignService` · `crates/paladin-battalion/src/campaign_service.rs` |
| **Chain of Command** | Hierarchical delegation pattern | `ChainOfCommandService` · `crates/paladin-battalion/src/chain_of_command_service.rs` |
| **Conclave** | Mixture-of-experts synthesis | `ConclaveExecutionService` · `crates/paladin-battalion/src/conclave_execution_service.rs` |
| **Council** | Multi-agent discussion and consensus | `CouncilService` · `crates/paladin-battalion/src/council_service.rs` |
| **Grove** | Semantic routing | `GroveService` · `crates/paladin-battalion/src/grove_service.rs` |
| **Maneuver** | Flow DSL execution | `maneuver/` · `crates/paladin-battalion/src/maneuver/` |
| **Commander** | Strategy auto-detect router | `Commander` · `crates/paladin-battalion/src/commander.rs` |
| **Garrison** | Short-term conversation memory | `Garrison` domain · `crates/paladin-core/src/platform/container/garrison.rs` |
| **Sanctum** | Long-term vector / semantic memory | `Sanctum` domain · `crates/paladin-core/src/platform/container/sanctum.rs` |
| **Arsenal** | Tool registry | `Arsenal` domain · `crates/paladin-core/src/platform/container/arsenal/` |
| **Armament** | A single registered tool | Part of Arsenal |
| **Citadel** | State persistence and recovery | `Citadel` domain · `crates/paladin-core/src/platform/container/citadel.rs` |
| **Commissary** | Input-side, per-call window-rationing officer | `Commissary` · `crates/paladin-llm/src/services/commissary.rs` |
| **Herald** | Output formatting system | `Herald` · `crates/paladin-core/src/platform/container/herald.rs` |
| **Quest** | A task or mission assigned to a Paladin | Informal / documentation term |

**Plain vs. Medieval-Military vocabulary (ADR-0049).** Not every token-economy concept in this
table gets a Medieval-Military name. Units and measures (`TokenUsage`, `max_tokens`,
`max_context_tokens`, `TokenBudget`) and technical port traits (`TokenCounterPort`, `LlmPort`,
`EmbeddingPort`) keep plain industry names — they describe quantities and technical seams, not
domain roles. Domain roles, places and events — the rows in the table above — get
Medieval-Military names.

## Node<T> Pattern

All domain entities use the `Node<T>` wrapper which adds identifier, timestamps,
and metadata to any data payload:

```rust,ignore
// crates/paladin-core/src/base/node.rs
pub struct Node<T> {
    pub id:         Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub node:       T,         // The domain payload
}
```

Domain types are type aliases:

```rust,ignore
// crates/paladin-core/src/platform/container/paladin.rs
pub type Paladin = Node<PaladinData>;
```

## Core Domain Entities

### Paladin (Aggregate Root)

```rust,ignore
// PaladinData — the domain payload inside Node<PaladinData>
pub struct PaladinData {
    pub name:          String,
    pub system_prompt: String,
    pub model:         String,
    pub temperature:   f32,
    pub max_loops:     u32,
    pub stop_words:    Vec<String>,
    pub status:        PaladinStatus,
}

pub enum PaladinStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Timeout,
}

pub type Paladin = Node<PaladinData>;
```

**Invariants:**
- `system_prompt` must not be empty
- `temperature` must be in `0.0..=2.0`
- `max_loops` must be `>= 1`

### Garrison (Memory Domain)

```rust,ignore
// crates/paladin-core/src/platform/container/garrison.rs
pub struct GarrisonEntry {
    pub id:          Uuid,
    pub role:        ConversationRole,   // System | User | Assistant | Tool
    pub content:     String,
    pub timestamp:   DateTime<Utc>,
    pub metadata:    HashMap<String, Value>,
    pub token_count: Option<u32>,
    pub is_summary:  bool,
}
```

Adapters: `InMemoryGarrison`, `SqliteGarrison` (feature `sqlite`) — see
[Garrison Memory](../user-guides/garrison-memory.md).

### Arsenal (Tool Domain)

```rust,ignore
// crates/paladin-core/src/platform/container/arsenal/
pub struct ToolDefinition {
    pub name:        String,
    pub description: String,
    pub parameters:  serde_json::Value,  // JSON Schema
}

pub struct ToolCall {
    pub id:        String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

pub struct ToolResult {
    pub tool_call_id: String,
    pub content:      String,
    pub is_error:     bool,
}
```

These types are real, registered and invocable — Arsenal/MCP tool execution ships and works.
What no shipped `LlmPort` adapter exercises is the LLM-initiated path that would populate them
from a provider response: `generate()` never returns a populated function call today, on any
of OpenAI, Anthropic, DeepSeek, or the bundled mock. See ADR-0042 for the tracked status.

### Citadel (State Persistence)

```rust,ignore
// crates/paladin-core/src/platform/container/citadel.rs
pub struct CitadelEntry {
    pub paladin_name: String,
    pub state:        PaladinState,
    pub saved_at:     DateTime<Utc>,
}
```

### Herald (Output Formatting)

```rust,ignore
// crates/paladin-core/src/platform/container/herald.rs
pub trait Herald: Send + Sync {
    fn format(&self, result: &PaladinResult, paladin: &Paladin) -> Result<String, HeraldError>;
}
```

Implementations: `JsonHerald`, `MarkdownHerald`, `TableHerald` — see
[Herald Output](../user-guides/herald-output.md).

### Battlefield (Superstep Shared State)

`crates/paladin-core/src/platform/container/battlefield.rs` — the typed, schema-declared shared
state passed to and returned (as `StateDelta`) from every node in a `WarGraph` run, merged each
superstep by each field's declared `DispatchRule`. See [WarEngine: Battlefield State & Superstep
Execution](../user-guides/superstep-engine.md).

### Waypoint (Superstep Checkpoint)

`crates/paladin-core/src/platform/container/waypoint.rs` — the checkpoint the superstep engine
persists after every superstep: a full `Battlefield` snapshot addressed by `(ThreadId,
WaypointId)`, enough to resume a run with zero re-execution of completed work. See [WarEngine:
Battlefield State & Superstep Execution](../user-guides/superstep-engine.md).

### Aegis (Fault-Tolerance Policy)

`crates/paladin-core/src/platform/container/aegis.rs` — the per-node fault-tolerance policy
family (retry, timeout, error handlers, model fallback, node caching), attached as a
`NodeId`-keyed sidecar on a `WarGraph` rather than as a field on any node spec. See [Aegis:
Retry, Timeout, Error Handlers, Model Fallback and Node Caching](../user-guides/fault-tolerance.md).

### TraceRecord (Observability Envelope)

`crates/paladin-core/src/platform/container/trace.rs` — the envelope every observability sink
receives: `thread_id`, an optional `run_id`, a per-run monotonic `seq`, `at`, and the
`#[non_exhaustive]` `TraceEvent` itself. See [Observability: Traces, Sinks and
Persistence](../operations/observability.md).

## Base Primitives (`crates/paladin-core/src/base/`)

| Type | Purpose |
|------|---------|
| `Node<T>` | Universal entity wrapper (id + timestamps + payload) |
| `Collection<T>` | Typed collection with pagination |
| `Field` | Dynamic field definition for content metadata |
| `Message` | Inter-agent message passing |
| `Action` | Agent action descriptor |
| `Event` | Domain event for cross-context communication |

## Error Types

Each domain has a dedicated error enum using `thiserror`:

| Error type | Location |
|-----------|---------|
| `PaladinError` | `crates/paladin-core/src/platform/container/paladin_error.rs` |
| `GarrisonError` | `crates/paladin-core/src/platform/container/garrison_error.rs` |
| `LlmError` | `crates/paladin-llm/src/error.rs` |
| `ArsenalError` | `crates/paladin-ports/src/output/arsenal_port.rs` |
| `SanctumError` | `crates/paladin-memory/src/sanctum/` |

See [Design Patterns](design-patterns.md) for the error handling convention.

## Bounded Contexts

| Context | Crates | Aggregate root |
|---------|--------|---------------|
| Agent execution | `paladin-ai-core`, `paladin-ports`, `paladin-llm` | `Paladin` |
| Memory | `paladin-ai-core`, `paladin-ports`, `paladin-memory` | `Garrison` / `Sanctum` |
| Orchestration | `paladin-ai-core`, `paladin-ports`, `paladin-battalion` | `Battalion` |
| Tool integration | `paladin-ai-core`, `paladin-ports` | `Arsenal` |
| State persistence | `paladin-ai-core`, `paladin-ports` | `Citadel` |
| Content ingestion | `paladin-ai-core`, `paladin-ports`, `paladin-content` | `ContentItem` |
| Storage | `paladin-ai-core`, `paladin-ports`, `paladin-storage` | `User` / `ContentList` |

## See Also

- [Architecture Overview](overview.md)
- [Hexagonal Design](hexagonal-design.md)
- [Crate Map](crate-map.md)
- [Design Patterns](design-patterns.md) — `PaladinBuilder` and error conventions
