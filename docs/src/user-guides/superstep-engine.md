# WarEngine: Battlefield State & Superstep Execution

**Since:** v0.10.0 (Phase 22)
**Crates:** `paladin-battalion` (`WarEngine`, `WarGraph`), `paladin-core` (`Battlefield`,
`Waypoint`), `paladin-storage` (the `WaypointPort` backends)

> Every code example targets the current **v0.10.0** workspace. The substantive examples are
> real, compiled code pulled from the `paladin-doc-examples` crate via mdBook `{{#include}}`, so
> they are checked against the live API; a few illustrative fragments are marked `rust,ignore`.
> The API forms are verified against `crates/paladin-battalion/` and `crates/paladin-core/`.

The **WarEngine** is the superstep engine every Paladin Battalion pattern in this book ultimately
runs on: it executes a `WarGraph` of nodes over a typed `Battlefield`, in bounded **supersteps**
that checkpoint automatically after each one and resume with zero re-execution after a crash.
Unlike the legacy Campaign graph, a `WarGraph` permits cycles — including self-loops — so
iterative workflows (retry-and-refine, evaluate-optimize loops) are expressible directly. This
page is the substrate the [Control Flow](control-flow.md), [Parley & Chronicle](parley-and-chronicle.md),
[Aegis](fault-tolerance.md) and [Agent Runtime](agent-runtime.md) guides build on — those guides
cover routing, pause/resume, fault tolerance and middleware respectively; this page does not
re-explain any of them.

## Table of Contents

1. [Building a Graph: Battlefield State and Superstep Merge Semantics](#building-a-graph-battlefield-state-and-superstep-merge-semantics)
2. [Waypoint Checkpointing and Addressing](#waypoint-checkpointing-and-addressing)
3. [The Three WaypointPort Backends](#the-three-waypointport-backends)
4. [EngineConfig, EngineLimits and Bounded Iteration](#engineconfig-enginelimits-and-bounded-iteration)
5. [WaypointRetentionService](#waypointretentionservice)
6. [The Graph Fingerprint](#the-graph-fingerprint)
7. [Where to Go Next](#where-to-go-next)

## Building a Graph: Battlefield State and Superstep Merge Semantics

A **Battlefield** is the typed shared state a `WarGraph`'s nodes read and write. Its shape is
declared once, as a `BattlefieldSchema` of `FieldSpec`s — each field names a `DispatchRule`
(`LastWrite`, `Append`, `MergeObject`, `Sum`, or `Custom`) that decides how two nodes' concurrent
writes to the same field within one superstep are merged. A node's contribution is a `StateDelta`:
a set of field values, never a direct mutation — the engine merges every delta produced in a
superstep into the Battlefield in one step, through each field's own dispatch rule, so the merge
order is deterministic regardless of how many nodes ran concurrently.

The graph below is deliberately small and cyclic: one node self-loops over a `(count, status)`
Battlefield a few times before falling out of the loop — the same shape that makes cyclic
execution useful for retry-and-refine style workflows.

```rust,ignore
{{#include ../../../crates/doc-examples/src/superstep_engine.rs:build_graph}}
```

## Waypoint Checkpointing and Addressing

Exactly one **Waypoint** is persisted automatically after every superstep — a full snapshot of
the Battlefield as of that point, never an incremental diff. A Waypoint is addressed by the pair
`(ThreadId, WaypointId)`: `ThreadId` identifies the run, `WaypointId` identifies one checkpoint
within it, and each Waypoint also carries `parent_waypoint_id` lineage back to the start of the
thread. A Waypoint's `vanguard` field — a `Vec<NodeId>`, not a struct of its own — lists the nodes
ready to execute in the next superstep; an empty vanguard after a superstep is what `RunOutcome::
Completed` means.

```rust,ignore
{{#include ../../../crates/doc-examples/src/superstep_engine.rs:run_engine}}
```

```rust,ignore
{{#include ../../../crates/doc-examples/src/superstep_engine.rs:inspect_waypoints}}
```

## The Three WaypointPort Backends

Every Waypoint write goes through the `WaypointPort` trait (`paladin-ports`), and three backends
implement it, all passing the same shared contract test suite:

| Backend | Path |
|---|---|
| In-memory | `crates/paladin-storage/src/waypoint/in_memory.rs` (`InMemoryWaypointStore`) |
| SQLite | `crates/paladin-storage/src/waypoint/sqlite.rs` |
| Postgres | `crates/paladin-storage/src/waypoint/postgres.rs` |

`InMemoryWaypointStore` needs no `Cargo.toml` feature flag beyond what `crates/doc-examples`
already declares — `paladin-storage`'s `waypoint` module is not feature-gated, unlike its
`sqlite`/`mysql`/`postgres` submodules.

## EngineConfig, EngineLimits and Bounded Iteration

Because a `WarGraph` permits cycles, every run needs bounds so it always terminates.
`EngineLimits` (`paladin-battalion`) is what a `WarGraph` is constructed with; the app-facing
`EngineConfig` (`src/config/engine.rs`) is what a deployment actually configures, and converts
into `EngineLimits` via `impl From<EngineConfig> for EngineLimits`:

```rust,ignore
{{#include ../../../crates/doc-examples/src/superstep_engine.rs:configure_limits}}
```

| `EngineConfig` field | Bounds | `APP_ENGINE_*` override |
|---|---|---|
| `max_supersteps` | superstep count before `EngineError::RecursionLimitExceeded` | `APP_ENGINE_MAX_SUPERSTEPS` |
| `max_node_visits` | per-node execution count before `EngineError::NodeVisitLimitExceeded` | `APP_ENGINE_MAX_NODE_VISITS` |
| `run_timeout_secs` | whole-run wall-clock budget before `EngineError::RunTimeoutExceeded` | `APP_ENGINE_RUN_TIMEOUT_SECS` |
| `waypoint_durability` | `Strict` (a save failure fails the run) or `BestEffort` (logged, run continues) | `APP_ENGINE_WAYPOINT_DURABILITY` |
| `max_muster_tasks` | tasks one `NextStep::Muster` directive may request before `EngineError::MusterTaskLimitExceeded` | `APP_ENGINE_MAX_MUSTER_TASKS` |

A graph that never falls out of its own loop hits `EngineError::RecursionLimitExceeded` once
`max_supersteps` is exhausted — the same limit `EngineLimits::default()` sets to `50` and this
page's own example graph would hit if its `LoopUntil` node never wrote `status = "done"`.

## WaypointRetentionService

Waypoint history grows without bound unless something prunes it. `WaypointRetentionService`
(`src/application/services/waypoint_retention.rs`) is the application-layer policy: it defines
the single project-wide rule for what may never be deleted — a thread's latest Waypoint, plus
every Waypoint whose status is `AwaitingInput` — and drives the storage-layer `prune()` free
function (`crates/paladin-storage/src/waypoint/retention.rs`) with that rule and a configured
age/count bound. The storage layer itself carries no opinion about what "protected" means; it is
handed the answer as a plain function argument.

## The Graph Fingerprint

A `WarGraph`'s content fingerprint — `GRAPH_FINGERPRINT_VERSION` (currently `"v6"`) plus a
`blake3` hash over node ids, edge specs and schema field names — is compared on `resume`, so
resuming a thread against a structurally different graph fails fast with `GraphMismatch` rather
than silently replaying stale routing. The fingerprint is deliberately **not** computed over
prompts or models (those may be hot-swapped without changing run semantics), and it excludes
every `EngineLimits` field — raising `max_supersteps` to let a resumed run continue is a
legitimate operator action, not a graph change. The version tag has bumped twice since Phase 22:
`v1` → `v2` fixed a delimiter-collision encoding bug, and the current `v6` covers the Aegis and
structured-output-schema sections the fingerprint's canonical byte stream now includes.

## Where to Go Next

This page covers the engine's own state, checkpointing and bounds — not what runs on top of it:

- **Routing** a node's output to the next node, including dynamic jumps and Muster fan-out — see
  [Control Flow: Dynamic Routing & Subgraphs](control-flow.md).
- **Pausing and resuming** a run, including human-in-the-loop Parleys and graceful shutdown — see
  [Parley & Chronicle](parley-and-chronicle.md).
- **Per-node fault tolerance** — retry, timeout, error handlers, model fallback and caching — see
  [Aegis: Retry, Timeout, Error Handlers, Model Fallback and Node Caching](fault-tolerance.md).
- **Middleware, context management, Vault memory and structured output** — see
  [Agent Runtime](agent-runtime.md).
