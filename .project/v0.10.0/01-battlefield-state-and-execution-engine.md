# PRD 01 — Battlefield State & Superstep Execution Engine (Epic `ENG`)

**Depends on:** nothing (keystone epic).
**Unlocks:** Docs 02, 03, 04, 06, 07.
**Primary crates:** `paladin-core` (domain types), `paladin-ports` (ports), `paladin-battalion` (engine), `paladin-storage`/`paladin-memory` (Waypoint backends), facade (wiring).

---

## 1. Problem Statement

Today, data flows between Paladins as bare strings: `PaladinPort::execute(&Paladin, &str) -> PaladinResult { output: String, .. }`. Campaign fan-in concatenates parent outputs with a `"\n\n---\n\n"` separator. Consequences:

- Structured data must be smuggled as JSON-in-strings with no schema, no validation, and no merge semantics.
- Parallel branches cannot merge results deterministically; last-writer or concatenation is all we have.
- There is no shared object to snapshot, so persistence (Citadel) can only save coarse whole-entity states, and only when explicitly invoked.
- Campaign rejects any cycle (`toposort` error), so iterative workflows (retry-and-refine, evaluate-optimize loops, negotiation-until-consensus) cannot be expressed at the orchestration level at all — only inside a single Paladin's internal reasoning loop.

This epic introduces: (a) a typed shared state — the **Battlefield** — with per-field **dispatch rules** (reducers), (b) a **superstep execution engine** that supports cycles with bounded iteration, and (c) **automatic Waypoint checkpointing** after every superstep, addressed by `(thread_id, waypoint_id)`.

## 2. Goals / Non-Goals

**Goals**
- G1. Typed state passed to and returned (as deltas) from every node.
- G2. Deterministic merging of concurrent deltas via per-field dispatch rules.
- G3. Cyclic graph execution with mandatory loop bounds and a recursion limit.
- G4. A Waypoint written after every superstep, automatically, with pluggable backends.
- G5. Resume any thread from its latest Waypoint with zero re-execution of completed work.
- G6. Full backward compatibility for string-based execution (X-03).

**Non-Goals**
- Dynamic routing/fan-out (Doc 02), pause/resume semantics (Doc 03), per-node retry/timeout (Doc 04). This epic must expose the seams they need (see §8) but not implement them.

## 3. Domain Design

### 3.1 Battlefield (in `paladin-core`, new module `platform::container::battlefield`)

```rust
/// Typed shared state for one workflow run.
/// Internally a map of named fields; each field has a declared dispatch rule.
pub struct Battlefield {
    schema: BattlefieldSchema,
    values: HashMap<FieldName, serde_json::Value>,
}

pub struct BattlefieldSchema {
    pub fields: Vec<FieldSpec>,
    pub schema_version: String,
}

pub struct FieldSpec {
    pub name: FieldName,               // newtype over String, validated non-empty
    pub dispatch: DispatchRule,
    pub default: Option<serde_json::Value>,
    pub required: bool,                // engine errors at start if required & no default & not in initial state
}

pub enum DispatchRule {
    /// Last write wins. Concurrent writes in the same superstep to a LastWrite
    /// field are a conflict → typed error (see ENG-FR-07).
    LastWrite,
    /// Value must be a JSON array; deltas append. Concurrent appends merge in
    /// deterministic order (see ENG-FR-08).
    Append,
    /// Value must be a JSON object; deltas shallow-merge keys. Same-key concurrent
    /// writes are a conflict.
    MergeObject,
    /// Numeric accumulation (value += delta). Value and delta must be numbers.
    Sum,
    /// Named custom rule, resolved at engine level (see ENG-FR-09).
    Custom(String),
}

/// A node's partial update: field name → new value / append item / merge fragment.
pub struct StateDelta(pub HashMap<FieldName, serde_json::Value>);
```

Design constraints:
- `Battlefield`, `BattlefieldSchema`, `StateDelta` all derive `Serialize`/`Deserialize`, `Clone`, `Debug` (X-04). `Battlefield` serialization embeds its schema so a Waypoint is self-describing.
- Typed accessors: `battlefield.get::<T: DeserializeOwned>(&FieldName) -> Result<Option<T>, BattlefieldError>` and `StateDelta::set<T: Serialize>(...)`. Raw-JSON accessors also public.
- `paladin-core` stays dependency-pure: `serde_json` is already a core dependency; nothing else is added.

### 3.2 Errors (new `BattlefieldError` in core, thiserror)

Variants (all with structured fields, per X-06): `UnknownField { field }`, `TypeMismatch { field, expected, got }`, `DispatchConflict { field, superstep, writers: Vec<NodeId> }`, `MissingRequiredField { field }`, `SchemaVersionUnsupported { found, supported }`, `CustomDispatchNotRegistered { name }`.

### 3.3 Waypoint & Thread addressing (in `paladin-core`, module `platform::container::waypoint`)

```rust
pub struct ThreadId(pub String);      // caller-supplied, non-empty, ≤ 256 chars, no whitespace
pub struct WaypointId(pub Uuid);      // engine-generated, v7 (time-ordered) preferred

pub struct Waypoint {
    pub thread_id: ThreadId,
    pub waypoint_id: WaypointId,
    pub parent_waypoint_id: Option<WaypointId>, // None only for the first waypoint of a thread or a fork root
    pub superstep: u64,
    pub battlefield: Battlefield,               // full snapshot (delta-encoding is a backend optimization, not a contract)
    pub vanguard: Vec<NodeId>,                  // nodes ready for the NEXT superstep
    pub completed: Vec<NodeExecutionRecord>,    // what ran in the superstep that produced this waypoint
    pub status: WaypointStatus,
    pub created_at: DateTime<Utc>,
    pub schema_version: String,
}

pub enum WaypointStatus {
    Running,          // more supersteps pending (vanguard non-empty)
    Completed,        // run finished normally
    Failed { error: String, failed_node: NodeId },
    AwaitingInput { parley: ParleyRequest },    // defined fully in Doc 03; type stub lands here
    Halted,           // graceful shutdown (Doc 03)
}

pub struct NodeExecutionRecord {
    pub node_id: NodeId,
    pub paladin_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub token_count: u64,
    pub outcome: NodeOutcomeKind, // Succeeded | Failed | Skipped(reason)
    pub attempt: u32,             // populated meaningfully by Doc 04; 1 until then
}
```

`NodeId` is a newtype over `String`, unique within a graph, human-readable (e.g. `"researcher"`), replacing raw `Uuid` node identity in new APIs (existing Campaign keeps Uuid; the engine maps).

### 3.4 WaypointPort (in `paladin-ports`, new `output::waypoint_port`)

```rust
#[async_trait]
pub trait WaypointPort: Send + Sync {
    async fn save(&self, wp: &Waypoint) -> Result<(), WaypointError>;
    async fn latest(&self, thread: &ThreadId) -> Result<Option<Waypoint>, WaypointError>;
    async fn get(&self, thread: &ThreadId, id: &WaypointId) -> Result<Option<Waypoint>, WaypointError>;
    async fn history(&self, thread: &ThreadId, limit: Option<u32>, before: Option<WaypointId>)
        -> Result<Vec<WaypointSummary>, WaypointError>;   // newest-first, paginated
    async fn list_threads(&self, limit: Option<u32>, before: Option<DateTime<Utc>>)
        -> Result<Vec<ThreadSummary>, WaypointError>;
    async fn delete_thread(&self, thread: &ThreadId) -> Result<u64, WaypointError>; // returns waypoints deleted
}
```

`WaypointError` is a thiserror enum: `Backend { source }`, `Serialization { .. }`, `SchemaVersionUnsupported { .. }`, `NotFound { .. }`.

This port is **separate from `CitadelPort`**. Citadel remains for whole-entity persistence (backward compat). Rationale documented in rustdoc: Waypoints are high-frequency, append-mostly, thread-addressed; Citadel is coarse entity snapshots.

### 3.5 Engine (in `paladin-battalion`, new module `engine`)

```rust
pub struct WarGraph {                              // the executable graph
    nodes: HashMap<NodeId, NodeSpec>,
    edges: Vec<EdgeSpec>,                          // static edges; Doc 02 adds dynamic routing
    schema: BattlefieldSchema,
    entry: Vec<NodeId>,
    limits: EngineLimits,
}

pub struct EngineLimits {
    pub max_supersteps: u64,        // REQUIRED, default 50; run fails with typed RecursionLimitExceeded when hit
    pub max_node_visits: u32,       // per-node visit cap within one run, default 25
    pub run_timeout: Option<Duration>,
}

pub enum NodeSpec {
    Paladin { paladin: Paladin, input_template: InputMapping, output_field: FieldName },
    Function(Arc<dyn StateNode>),   // pure state→delta node, for deterministic steps
    Battalion(/* Doc 02: subgraph */),
}

#[async_trait]
pub trait StateNode: Send + Sync {
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<StateDelta, NodeError>;
}

pub struct WarEngine<W: WaypointPort> { /* paladin_port, waypoint_port, dispatch registry, hooks */ }

impl<W: WaypointPort> WarEngine<W> {
    pub async fn start(&self, graph: &WarGraph, thread: ThreadId, initial: StateDelta)
        -> Result<RunOutcome, EngineError>;
    pub async fn resume(&self, graph: &WarGraph, thread: ThreadId)
        -> Result<RunOutcome, EngineError>;      // from latest waypoint
}

pub enum RunOutcome {
    Completed { final_state: Battlefield, waypoint: WaypointId },
    AwaitingInput { parley: ParleyRequest, waypoint: WaypointId }, // fully wired in Doc 03
    Halted { waypoint: WaypointId },
    Failed { error: EngineError, waypoint: Option<WaypointId> },
}
```

`InputMapping` renders a Paladin's string input from the Battlefield: a template string with `{field}` placeholders resolved from state (values JSON-stringified unless the field is a JSON string, which is inserted raw). This is the bridge that lets today's string-in/string-out Paladins participate in typed workflows unchanged (X-03). `output_field` is where the Paladin's `PaladinResult.output` is written as a delta (dispatch rule of that field applies).

## 4. Functional Requirements

Execution semantics:

- **ENG-FR-01 (Superstep loop).** The engine MUST execute in supersteps: (1) take the current Vanguard; (2) execute all its nodes concurrently (bounded by a configurable parallelism limit, default = number of vanguard nodes); (3) collect each node's `StateDelta`; (4) merge deltas into the Battlefield via dispatch rules; (5) compute the next Vanguard from edges whose conditions pass and whose target's dependencies are satisfied; (6) persist a Waypoint; (7) repeat until Vanguard is empty (Completed) or a limit/failure/parley intervenes.
- **ENG-FR-02 (Cycles allowed).** The engine MUST accept graphs containing cycles, including self-loops. `WarGraph::validate()` MUST NOT reject cycles; instead it MUST require `EngineLimits.max_supersteps ≥ 1` and MUST reject a graph where `max_supersteps` or `max_node_visits` is zero.
- **ENG-FR-02a (Reachability validation — BUG-02 fix, test-first).** Allowing cycles removed toposort's implicit connectivity guarantee, creating the *silent stranded node* defect: a non-entry node whose only incoming edges originate from itself (or from a component not reachable from entry) can never become ready, yet the run reports `RunOutcome::Completed`. `WarGraph::validate()` MUST therefore compute the **eligible set** = (nodes reachable from `entry` via static edges) ∪ (nodes with `worker_template: true`, reachable via Muster) ∪ (nodes referenced as `Route { to }` targets in any Aegis `on_error` of an eligible node, applied to a fixed point) ∪ (nodes explicitly marked `dynamic_target: true` on `NodeSpec`, the declared escape hatch for Goto-only targets). Every declared node outside the eligible set fails validation with `EngineError::UnreachableNode { node, reason }` (or an `InvalidGraph` listing all such nodes at once — prefer listing all), **before any node executes**. Notes: (a) a self-loop on an *entry* or otherwise-reachable node remains legal — the check rejects strandedness, not self-loops; (b) `dynamic_target: true` shifts responsibility to CF-FR-07's runtime Goto validation and MUST be rustdoc'd as such; (c) `Route`/handler targets and worker templates need no annotation — their declarations already prove intent; (d) the fix MUST be developed test-first against a currently-passing stranded-node fixture (red), and every existing test or fixture that works around strandedness (including the E2E-1 fixture's looping-node arrangement) MUST be revisited: remove the workaround where it exists only to dodge the bug, and add one regression test asserting a graph with a stranded self-loop-only node is rejected at validation while the equivalent graph with the node made reachable (or marked `dynamic_target`) passes. Completion semantics are unchanged — `Completed` still means "Vanguard empty" — because validation now guarantees every declared node was at least eligible.
- **ENG-FR-03 (Bounded iteration).** When the superstep count reaches `max_supersteps`, the engine MUST stop with `EngineError::RecursionLimitExceeded { limit, thread_id }` and persist a `Failed` Waypoint. Same for `max_node_visits` per node (`NodeVisitLimitExceeded { node, limit }`).
- **ENG-FR-04 (Deterministic frontier).** Given identical node outputs, the sequence of Vanguards MUST be deterministic. Node execution order within a superstep is concurrent, but merge order is deterministic (ENG-FR-08), and next-Vanguard computation MUST iterate nodes/edges in stable (insertion) order.
- **ENG-FR-05 (Isolation within a superstep).** All nodes in one superstep read the SAME pre-superstep Battlefield snapshot. A node MUST NOT observe deltas produced by peers in the same superstep.
- **ENG-FR-06 (Join semantics).** A node with multiple incoming edges MUST NOT execute until every incoming edge from a node that is *reachable in this run* has resolved (fired or provably not-firing). "Provably not-firing" = the source node completed and the edge condition evaluated false, or the source is unreachable given conditions already resolved. This makes the current Campaign behavior (dependencies-satisfied check) precise and adds the not-firing case, so a false branch does not deadlock a downstream join. A `defer: bool` flag on `NodeSpec` MUST additionally delay the node until the Vanguard contains no other executable nodes (aggregate-after-all-branches semantics).
- **ENG-FR-06a (Starvation release — BUG-03 fix, test-first).** ENG-FR-06's join rule has a bootstrap gap: a cycle's own back-edge into one of its members is `Pending` until that member has executed once, so ENG-FR-06's "every incoming edge resolved" requirement can never be satisfied for that member's *first* execution — the cycle can never bootstrap, and the run reports `RunOutcome::Completed` over a node that never ran. This is true both for a node that is its own back-edge source (a self-loop combined with an upstream edge) and for the general shape `entry -> a -> b -> a`, where `a`'s incoming edges are the upstream `entry -> a` and the cyclic `b -> a`. The engine MUST therefore guarantee: every declared node in the eligible set that receives at least one fired incoming edge MUST execute at least once before the run may report `Completed`. This is implemented as a fallback: when no node is executable by ENG-FR-06's normal join rule, and no `defer`-marked node is executable either, the engine releases every node that already holds at least one fresh fired incoming edge and whose every other unresolved incoming edge is `Pending` from a live source that has never executed — the starvation case, as opposed to a node still legitimately waiting on a live source's next firing (releasing that node WOULD violate ENG-FR-06's join semantics, and MUST NOT happen). ENG-FR-06's acyclic join semantics are otherwise unchanged; this release is a fallback tier, never a substitute for the normal rule. Release order MUST be `node_order`-deterministic (ENG-FR-04), identical in discipline to the `defer` release. The fix MUST be developed test-first against the two BUG-03 reproductions (the self-loop-plus-upstream-edge shape and the general `entry -> a -> b -> a` shape). `Completed` still means "Vanguard empty" — now truthfully, for cycle-bootstrap shapes as well as the acyclic case ENG-FR-06 already covered. Any graph shape the release still cannot schedule MUST fail at `validate()` with a typed error naming the offending nodes, and a run about to report `Completed` while a non-dead eligible node holds an unconsumed fired incoming edge MUST fail with a typed error instead of completing silently.

Dispatch / merge:

- **ENG-FR-07 (LastWrite conflict).** Two deltas in the same superstep writing the same `LastWrite` field MUST fail the run with `DispatchConflict` naming the field, superstep, and writer NodeIds. This is a hard error, not a warning.
- **ENG-FR-08 (Deterministic Append/merge order).** Concurrent `Append` deltas MUST merge ordered by (source NodeId lexicographic, then delta emission index). Two runs with identical node outputs MUST produce byte-identical serialized Battlefields. A property/repeat test MUST assert this over ≥ 20 randomized-scheduling iterations.
- **ENG-FR-09 (Custom dispatch).** Custom dispatch rules are registered on the engine as `Arc<dyn Fn(&Value, &Value) -> Result<Value, BattlefieldError> + Send + Sync>` under a string name. Graph validation MUST fail (`CustomDispatchNotRegistered`) if the schema references an unregistered name. Registration lives in the engine (application layer), never in `paladin-core`.
- **ENG-FR-10 (Schema enforcement).** A delta touching an undeclared field → `UnknownField` hard error. A run starting without all `required` fields resolvable (initial delta ∪ defaults) → `MissingRequiredField` before any node executes.

Checkpointing:

- **ENG-FR-11 (Automatic Waypoint per superstep).** The engine MUST persist exactly one Waypoint after every superstep merge, before computing whether the run continues. Waypoint write failure fails the run with `EngineError::WaypointWrite { source }` — durable-by-default; a documented `WaypointDurability::BestEffort` engine option may downgrade write failure to a logged warning, default is `Strict`.
- **ENG-FR-12 (Resume).** `resume(graph, thread)` MUST load the latest Waypoint, verify graph compatibility (see ENG-FR-14), restore Battlefield + Vanguard + per-node visit counts, and continue. Completed nodes MUST NOT re-execute (E2E-1). Resuming a thread whose latest Waypoint is `Completed` returns `RunOutcome::Completed` immediately without executing anything. Resuming an unknown thread → `EngineError::ThreadNotFound`.
- **ENG-FR-12a (Resume restores frontier state — BUG-04 fix, test-first).** ENG-FR-12's "continue" has a gap: for every graph shape, a run resumed from any Waypoint MUST be superstep-for-superstep equivalent to the uninterrupted run — the same nodes scheduled in the same supersteps, the same final Battlefield, zero re-execution — which is ENG-FR-12's own promise stated for all shapes, not only E2E-1's fixture. Restoring only Battlefield, Vanguard and visit counts is not enough: the scheduler's own per-edge resolution state was discarded, so a pre-crash fired edge into a join node that was not yet ready is never seen again after resume. The engine MUST therefore also persist and restore a frontier snapshot: every Waypoint carries a serializable snapshot of per-edge resolution — keyed by edge identity (`from`, `to`, canonical condition), never by edge index — plus per-node last-executed superstep, additive and `#[serde(default)]` so a Waypoint written without it still loads with an empty snapshot and resumes exactly as it does today. Under `ResumeOptions::allow_graph_change`, the restored snapshot MUST degrade rather than mis-assign: a snapshot edge whose identity the new graph no longer declares is dropped, and a new-graph edge with no snapshot entry starts `Pending`. No SQL migration is required, because all three `WaypointPort` backends (ENG-FR-15/16/17) persist the whole Waypoint as JSON. The fix MUST be developed test-first against the divergence reproduction — a join shape whose only fired incoming edge fired before the crash point. Cross-references ENG-FR-12 (the guarantee this amends) and ENG-FR-14 (the fingerprint check this frontier restoration is independent of — a matching fingerprint says nothing about whether the Frontier itself was restored).
- **ENG-FR-13 (Thread lineage).** Every Waypoint records `parent_waypoint_id`, forming a chain (a tree once Doc 03 forking lands). `WaypointPort::history` returns newest-first with pagination.
- **ENG-FR-14 (Graph fingerprint).** `WarGraph` exposes a stable content fingerprint (hash over node ids, edge specs, schema — NOT over prompts/models, which may be hot-swapped). The fingerprint is stored in each Waypoint; `resume` with a mismatched fingerprint → `EngineError::GraphMismatch { expected, got }` unless the caller passes an explicit `allow_graph_change: true` option (in which case unknown vanguard NodeIds fail with `UnknownField`-style precision: `EngineError::VanguardNodeMissing { node }`).

Waypoint backends (in `paladin-memory` or `paladin-storage`, feature-gated per X-07):

- **ENG-FR-15 (InMemory backend).** `InMemoryWaypointStore` for tests/dev: HashMap behind `tokio::sync::RwLock`, full port contract.
- **ENG-FR-16 (SQLite backend).** `SqliteWaypointStore` using the existing `sqlx`/SQLite stack: schema `waypoints(thread_id TEXT, waypoint_id TEXT PRIMARY KEY, parent_id TEXT NULL, superstep INTEGER, status TEXT, payload BLOB/JSON, created_at TEXT)` + index on `(thread_id, created_at DESC)`. Migrations added under `migrations/`.
- **ENG-FR-17 (Postgres backend).** `PostgresWaypointStore` (new `postgres` feature; add `sqlx` postgres feature to `paladin-storage`). Same logical schema; JSONB payload; must pass the identical contract test suite as ENG-FR-15/16 (a shared `waypoint_port_contract_tests!` macro or generic test fn MUST exist so all backends run the same suite).
- **ENG-FR-18 (Retention).** A `WaypointRetentionConfig { max_age_days: Option<u32>, max_waypoints_per_thread: Option<u32> }` plus a cleanup routine callable from the existing job-scheduling system. Pruning MUST never delete a thread's single latest Waypoint or any Waypoint with status `AwaitingInput`. Pruning MUST be monotone and idempotent: under any crash or backend failure mid-prune, the keep-set (the latest Waypoint plus every `AwaitingInput` Waypoint, at minimum) is intact, and re-running the routine after an interruption converges to exactly the keep-set and removes nothing further. Retention is best-effort reclamation — leaving an extra surviving Waypoint behind is acceptable; losing one named in the keep-set is not.

Backward compatibility:

- **ENG-FR-19 (String bridge).** A convenience constructor `WarGraph::from_formation(paladins: Vec<Paladin>)` (and `from_phalanx`, `from_campaign`) MUST build an equivalent typed graph using a default schema (`input: LastWrite`, `output: LastWrite`, `history: Append`) and `InputMapping` templates reproducing today's data flow, including Campaign's fan-in concatenation with `"\n\n---\n\n"`. Golden tests MUST assert output-equivalence with the legacy services for a 3-node Formation, a 3-node Phalanx, and the existing branching Campaign fixtures.
- **ENG-FR-20 (Legacy services untouched).** `FormationExecutionService`, `PhalanxExecutionService`, `CampaignExecutionService`, `Commander`, and their public signatures continue to work with zero behavioral change (except BUG-01, fixed in Doc 02). New engine is additive.

Engine hooks (seams for later docs — implement the hook, not the consumers):

- **ENG-FR-21 (Trace hook).** The engine accepts an optional `Arc<dyn TraceSink>` (port in `paladin-ports`) receiving typed events: `RunStarted`, `SuperstepStarted`, `NodeStarted`, `NodeFinished`, `DeltaMerged { field_changes }`, `WaypointSaved`, `RunFinished`. Fire-and-forget; a slow/failing sink MUST NOT stall or fail the run (bounded channel + drop-oldest, drops counted).
- **ENG-FR-22 (Node middleware hook).** Node execution passes through an ordered chain `Vec<Arc<dyn NodeInterceptor>>` with `before(&NodeContext, &Battlefield) -> InterceptDecision` and `after(&NodeContext, &mut StateDelta)`. `InterceptDecision::{Proceed, Skip(reason), Fail(NodeError)}`. Default chain empty. (Doc 05 populates it; Doc 04's Aegis wraps outside this chain.)
- **ENG-FR-23 (Cancellation token).** Engine accepts a `CancellationToken`; on cancellation it finishes the in-flight superstep, persists a `Halted` Waypoint, and returns `RunOutcome::Halted`. (Doc 03 exposes this as graceful shutdown; the mechanism lands here.)

## 5. Non-Functional Requirements

- **ENG-NFR-01.** Waypoint save for a Battlefield ≤ 1 MiB must add < 10 ms p50 overhead per superstep on the SQLite backend (benchmark in `benches/`).
- **ENG-NFR-02.** Engine memory: one Battlefield clone per superstep maximum, plus one per concurrently executing node view (Arc-shared read snapshot preferred; measure, don't guess — add a bench).
- **ENG-NFR-03.** All engine state is `Send`; the engine future is spawnable on a multi-threaded runtime.

## 6. Acceptance Criteria

1. E2E-1 (crash-resume, overview §6) passes.
2. A self-loop node ("refine until output field contains APPROVED, max 5 visits") executes 1–5 times based on mock output and terminates with the correct typed error when the cap is hit without approval.
2a. BUG-02 (ENG-FR-02a): a graph containing a non-entry node whose only incoming edges are its own self-loop is rejected at validation naming the node; the same graph with the node reachable from entry (or marked `dynamic_target: true`) validates and runs; worker-template and Route-target nodes validate without annotation. Fix demonstrably test-first; no remaining fixture works around strandedness.
3. Dispatch determinism repeat test (ENG-FR-08) green over 20 iterations.
4. All three Waypoint backends pass the shared contract suite; SQLite + Postgres via the existing docker-compose integration target.
5. Golden equivalence tests (ENG-FR-19) green.
6. `DispatchConflict` surfaced with correct field/superstep/writers in a two-writer Phalanx-style graph.
7. Coverage on new modules ≥ 85%; workspace ≥ 82%.
8. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 7. Test Plan (TDD ordering)

1. `Battlefield`/`StateDelta` unit tests (typed get/set, schema enforcement, each DispatchRule incl. conflicts) — pure `paladin-core`, no async.
2. `WaypointPort` contract suite against `InMemoryWaypointStore`.
3. Engine unit tests with `Function` nodes only (no LLM): linear, branch, join, defer, cycle, limits, determinism.
4. Engine + MockPaladinPort integration: `InputMapping`, `output_field` write, token accounting into `NodeExecutionRecord`.
5. Resume tests (drop engine mid-run at every superstep index of a 5-superstep run — parameterized).
6. SQLite/Postgres backend contract + migration tests.
7. Golden legacy-equivalence tests.
8. Multi-thread stress test: 8-node all-parallel superstep, 100 iterations, exact-count call assertions + timeout guard (X-05).
9. Benchmarks (ENG-NFR-01/02).

## 8. Explicit Seams Handed to Later Docs

| Seam | Consumer |
|---|---|
| `NodeOutcome`/Directive extension point on node return type | Doc 02 (routing, Muster) |
| `WaypointStatus::AwaitingInput` + `ParleyRequest` stub | Doc 03 |
| `WaypointPort::get/history` | Doc 03 (time travel), Doc 06 (threads API) |
| `CancellationToken` → `Halted` | Doc 03 (graceful shutdown), Doc 06 |
| Aegis wrapper around node execution | Doc 04 |
| `NodeInterceptor` chain | Doc 05 |
| `TraceSink` | Doc 07 |

## 9. Out of Scope

Dynamic `Goto`, Muster fan-out, subgraph nodes (Doc 02); parley/resume-with-payload, forking (Doc 03); retry/timeout/error-handler (Doc 04); HTTP exposure (Doc 06).
