# Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs - Pattern Map

**Mapped:** 2026-09-03
**Files analyzed:** 19 (new/modified, across CF-01…CF-05)
**Analogs found:** 19 / 19 (all have a direct, in-tree analog — this phase is a pure
code-continuation phase with no external-library gap)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-battalion/src/edge_evaluator.rs` (new — trait/registry) | service (registry) | event-driven (named dispatch) | `crates/paladin-battalion/src/engine/dispatch_registry.rs` | exact |
| `crates/paladin-battalion/src/llm_decision.rs` (new, or beside above) | service (adapter over `LlmPort`) | request-response | `crates/paladin-llm/src/mock.rs` (`MockLlmAdapter`) + `dispatch_registry.rs` shape | role-match |
| `crates/paladin-core/src/platform/container/directive.rs` (new, or inline) | model (value type) | transform | `crates/paladin-core/src/platform/container/battlefield.rs` (`StateDelta`) | exact |
| `crates/paladin-battalion/src/campaign_service.rs` (modified: `with_evaluator`, async `evaluate_edge_condition`, fail-closed check) | service | request-response | itself — `with_herald` (additive builder), `evaluate_edge_condition` (existing method to extend) | exact (self-modify) |
| `crates/paladin-battalion/src/commander.rs` (modified: `StrategySelection`, `CommanderBuilder::strategy_selection`) | service | request-response | itself — `CommanderBuilder`'s existing private-field/fluent-method pattern | exact (self-modify) |
| `crates/paladin-battalion/src/engine/graph.rs` (modified: `worker_templates`, `NodeSpec::Battalion`, `validate` param, fingerprint v3) | model / validator | CRUD (graph mutation) + transform (fingerprint) | itself — `add_deferred_node`/`defer_flags`/`mark_dynamic_target`, `validate`, `fingerprint`/`push_field` | exact (self-modify) |
| `crates/paladin-battalion/src/engine/node.rs` (modified: `StateNode::run` return type, `NodeContext.muster`) | model / trait | transform | itself — `StateNode`, `NodeContext` | exact (self-modify) |
| `crates/paladin-battalion/src/engine/superstep.rs` (modified: Goto/End/Parley/Muster arms, progress Waypoints, evaluator-registry consult) | controller (scheduler loop) | event-driven / streaming (superstep loop) | itself — `Frontier`, `compute_next_vanguard`, `evaluate_edge_condition`, `build_waypoint` | exact (self-modify) |
| `crates/paladin-battalion/src/engine/input_mapping.rs` (modified: muster-context param, `muster.` prefix validation) | utility (template render) | transform | itself — `InputMapping::render` | exact (self-modify) |
| `crates/paladin-battalion/src/engine/mod.rs` (modified: `with_edge_evaluator`, new `EngineError` variants, child-run construction) | service (facade / builder) | request-response | itself — `WarEngine` builder methods (`with_dispatch_rule`, `with_trace_sink`), `EngineError` | exact (self-modify) |
| `crates/paladin-core/src/platform/container/waypoint.rs` (modified: `checkpoint_ns`, `muster_progress` additive fields, `ThreadId::child`) | model | CRUD (persistence value type) | itself — `visit_counts`/`frontier` additive-field precedent | exact (self-modify) |
| `src/config/engine.rs` (new) | config | CRUD (load/validate) | `src/config/waypoint_retention.rs` / `src/config/citadel.rs` | exact |
| `tests/integration/e2e_muster_defer_order_test.rs` (new) | test (integration) | event-driven (engine run) | `tests/integration/e2e_crash_resume_test.rs` | exact |
| `tests/integration/` Formation-inside-Campaign subgraph test (new file, name TBD) | test (integration) | event-driven | `tests/integration/e2e_crash_resume_test.rs` | exact |
| `crates/paladin-storage/src/waypoint/contract_tests.rs` (modified: `muster_progress`/`checkpoint_ns` round-trip cases) | test (contract suite) | CRUD (persistence round-trip) | itself — existing `sample_waypoint`/per-clause functions | exact (self-modify) |
| `crates/paladin-battalion/Cargo.toml` (modified: `[dev-dependencies] paladin-llm`) | config (manifest) | — | n/a (manifest edit) | n/a |
| `MIGRATION.md` (modified: §9.1 M-B-01, §9.2 rows, §9.5 `EngineConfig`) | docs | — | itself — existing §9 structure | exact (self-modify) |
| `docs/src/` control-flow mdBook page (new) + `docs/src/SUMMARY.md` (modified) | docs | — | any existing mdBook page under `docs/src/` (Claude's discretion which) | role-match |
| `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` (modified: ENG-FR-11 note) | docs (program artifact) | — | 22.1's own ENG-FR-06a/12a note-edit precedent | exact |
| `.project/v0.10.0/08-traceability-matrix.md` (modified: BUG-01 row) | docs (program artifact) | — | itself — existing BUG-02/03/04 rows | exact |

## Pattern Assignments

### `crates/paladin-battalion/src/edge_evaluator.rs` (service/registry, event-driven)

**Analog:** `crates/paladin-battalion/src/engine/dispatch_registry.rs` (107 lines, read in full)

**Imports pattern:**
```rust
use std::sync::Arc;
use paladin_core::platform::container::battlefield::{CustomDispatchFn, CustomDispatchResolver};
use crate::engine::EngineError;
```

**Registry struct + register/resolver pattern (copy shape verbatim):**
```rust
#[derive(Default)]
pub struct DispatchRegistry {
    inner: CustomDispatchResolver,
}

impl DispatchRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        name: impl Into<String>,
        rule: Arc<CustomDispatchFn>,
    ) -> Result<(), EngineError> {
        let name = name.into();
        if RESERVED_NAMES.contains(&name.as_str()) {
            return Err(EngineError::ReservedDispatchName { name });
        }
        self.inner.insert(name, rule);
        Ok(())
    }

    pub fn resolver(&self) -> &CustomDispatchResolver {
        &self.inner
    }
}
```

**Deviation required by CONTEXT.md D-01/D-02:** `EdgeEvaluatorRegistry` stores
`Arc<dyn EdgeConditionEvaluator>` (a trait object, not a closure like `CustomDispatchFn`), and per
D-01 there is **no `RESERVED_NAMES` collision guard** — `EdgeCondition::Custom` names are arbitrary
strings with no built-in-variant name to collide with. `EdgeConditionEvaluator::evaluate` is
`#[async_trait]` (see Shared Patterns → Async Trait below), unlike `DispatchRegistry`'s sync
closures — `register`/`get`/`registered_names` stay sync.

**Fail-closed validation pattern** (from `graph.rs:317-325`, the model for the new clause in
`WarGraph::validate`):
```rust
for field in &self.schema.fields {
    if let DispatchRule::Custom(name) = &field.dispatch
        && !custom_dispatch.contains_key(name)
    {
        return Err(EngineError::Battlefield(
            BattlefieldError::CustomDispatchNotRegistered { name: name.clone() },
        ));
    }
}
```
Extend to collect **every** offender (not fail-fast), sorted, matching `validate_eligible_set`'s
and `validate_schedulable`'s existing "collect every offender, report once" discipline.

**Test pattern:** `dispatch_registry.rs`'s `#[cfg(test)] mod tests { ... register_and_resolve_a_custom_rule() ... }` — one test per registry behavior, in-module.

---

### `crates/paladin-battalion/src/llm_decision.rs` (service, request-response)

**Analog:** `crates/paladin-llm/src/mock.rs` (`MockLlmAdapter`) for the `Arc<dyn LlmPort>` call
shape and testing, plus `edge_evaluator.rs`'s trait for the evaluator shape.

**Core pattern (per D-23/D-24):**
```rust
pub struct LlmDecisionEvaluator {
    llm: Arc<dyn LlmPort>,
    model: String,
    prompt_template: String,
    choices: Vec<(String, NodeId)>,
    on_ambiguous: OnAmbiguous,
}

pub enum OnAmbiguous {
    Fail,
    Default(String),
}
```
Memoization key `(ThreadId, superstep, NodeId)` resolved once per decision (D-24) — do not call
`LlmPort` per outgoing edge; the memo is consulted by every `Custom("<decision name>")` edge of
that source node in that superstep. Matching is exact-after-trim, case-insensitive.

**Testing pattern:** `crates/paladin-llm/src/mock.rs`'s `MockLlmAdapter::with_response`/
`with_error` — new `[dev-dependencies]` line on `paladin-battalion/Cargo.toml` required first
(confirmed absent today).

**Security boundary note (rustdoc obligation, not code):** document per
`.github/instructions/security.instructions.md` that the rendered prompt is an egress boundary —
mirror the phrasing style of `crates/paladin-llm/src/redaction.rs`'s own rustdoc on redact-before-
truncate.

---

### `crates/paladin-core/src/platform/container/directive.rs` (model, transform)

**Analog:** `crates/paladin-core/src/platform/container/battlefield.rs` (`StateDelta`) for the
core value-type shape and the `impl From<StateDelta> for Directive` conversion (ADR-0016: core
owns value types).

**Core pattern:**
```rust
pub struct Directive {
    pub delta: StateDelta,
    pub next: NextStep,
}

pub enum NextStep {
    Edges,
    Goto(Vec<NodeId>),
    Muster(Vec<MusterTask>),
    End,
    Parley,
}

impl From<StateDelta> for Directive {
    fn from(delta: StateDelta) -> Self {
        Directive { delta, next: NextStep::Edges }
    }
}
```
No new `paladin-core` dependency — `serde_json::Value` is already present (ADR-0015 allowlist).

---

### `crates/paladin-battalion/src/campaign_service.rs` (self-modify, service, request-response)

**Analog:** itself — `with_herald` is the additive-builder precedent named directly by D-03; the
existing `evaluate_edge_condition` (BUG-01 site) is the method to convert to `async`.

**Additive builder pattern to copy (`with_herald`):**
```rust
// campaign_service.rs:100
pub fn with_herald(mut self, herald: Arc<dyn Herald>) -> Self {
    self.herald = Some(herald);
    self
}
```
`with_evaluator(name, evaluator)` follows this exact shape — `CampaignExecutionService::new`
keeps its signature (D-03), the field is added privately like `herald: Option<Arc<dyn Herald>>`.

**BUG-01 site to fix (`campaign_service.rs:377-399`, confirmed current):**
```rust
fn evaluate_edge_condition(
    &self,
    condition: &EdgeCondition,
    output: &str,
) -> Result<bool, BattalionError> {
    match condition {
        EdgeCondition::Always => Ok(true),
        EdgeCondition::Contains(substring) => Ok(output.contains(substring)),
        EdgeCondition::Regex(pattern) => { /* ... */ }
        EdgeCondition::Custom(_) => {
            warn!("Custom edge condition not yet implemented, defaulting to true");
            Ok(true)  // <-- BUG-01, line 396: DELETE, no replacement default
        }
    }
}
```
Call site at `campaign_service.rs:308` (`self.evaluate_edge_condition(&edge_data.condition,
&result.output)?` inside the already-`async fn execute_internal`) — converting to `.await?` after
making the method `async fn` is mechanical (no `Send`/`Sync` obstacle; `paladin_port:
Arc<dyn PaladinPort>` already crosses an await point here).

**Fail-closed pre-check insertion point:** `execute` → `campaign.validate()` at line 174 — the new
check runs immediately after, before any node executes (mirrors `WarGraph::validate`'s "checked
after structural clauses" discipline).

**Error handling pattern:** reuse `BattalionError::InvalidGraph(String)`
(`crates/paladin-core/src/platform/container/battalion/mod.rs:755`) — do not add a variant
(`BattalionError` is not `#[non_exhaustive]`, X-10).

---

### `crates/paladin-battalion/src/commander.rs` (self-modify, service, request-response)

**Analog:** itself — `CommanderBuilder`'s existing private-field + fluent-method construction
(read in full lines 1230-1420).

**Core pattern:**
```rust
pub enum StrategySelection {
    Heuristic,
    Semantic { llm: Arc<dyn LlmPort>, model: String },
}
// manual Debug impl — Arc<dyn LlmPort> is not Debug
```
`CommanderBuilder::strategy_selection(sel)` is additive, mirroring `with_herald`'s shape above;
`Commander::new` signature unchanged; the new field is **private** on `Commander` (§9.2 already
records this requirement — verify by grep for `pub` on any new field before commit).

**Fallback pattern (D-25):** any LLM error or unrecognized/ambiguous answer falls back to the
existing `analyze_and_select` heuristic (line 1023) and records the fallback + cause in
`strategy_selection_reasoning` — the existing result field, not a new one.

**Testing pattern:** `crates/paladin-llm/src/mock.rs`'s `MockLlmAdapter::with_response`/
`with_error`; the 52 existing inline tests plus `tests/integration/commander_integration_tests.rs`
and `commander_error_paths_test.rs` must pass unmodified — run these explicitly per plan task.

---

### `crates/paladin-battalion/src/engine/graph.rs` (self-modify, model/validator)

**Analog:** itself — `add_deferred_node`/`defer_flags`/`mark_dynamic_target` for
`add_worker_template`/`worker_templates`; `validate`'s three-clause structure; `fingerprint`'s
`push_field` encoding.

**`validate`'s exact insertion pattern (confirmed current, `graph.rs:290-329`):**
```rust
pub fn validate(&self, custom_dispatch: &CustomDispatchResolver) -> Result<(), EngineError> {
    if self.limits.max_supersteps == 0 { /* ... */ }
    if self.limits.max_node_visits == 0 { /* ... */ }
    for edge in &self.edges { /* unknown node checks */ }
    for entry in &self.entry { /* unknown entry checks */ }
    for field in &self.schema.fields { /* Custom dispatch registered? */ }

    self.validate_eligible_set()?;
    self.validate_schedulable()
}
```
CF-01 adds an `evaluator_registry: &EdgeEvaluatorRegistry` parameter + one new clause; CF-03 adds
worker-template well-formedness; CF-04 adds recursion rejection — each extends this **same**
function, never a parallel validate entry point (State of the Art table).

**Eligible-set insertion point (do not re-derive):** `validate_eligible_set`
(`graph.rs:334-409`) has a documented, unfilled seam for worker-template eligibility — seed
`worker_templates` into the **same** fixpoint worklist (`graph.rs:368-374`).

**Fingerprint v3 pattern:** `push_field` (`graph.rs:629-639`, length-prefixed, collision-free
since CR-01) — every new v3 section (worker-template set, per-`Battalion`-node child fingerprint +
`StateMap` + `restart_on_resume`, per-Paladin-node `DirectiveParser` kind + `on_parse_error`) must
call `push_field`, sorted, never a delimiter join. `GRAPH_FINGERPRINT_VERSION` bump `v2` → `v3`
at the top of `fingerprint()` (`graph.rs:564`).

**Exclusion test to extend, not duplicate (Pitfall 5):**
`fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits` (`graph.rs:1165-1180`) — add
`max_muster_tasks` variation to this **same** test.

---

### `crates/paladin-battalion/src/engine/node.rs` (self-modify, model/trait)

**Analog:** itself (38 lines, read in full) — `StateNode`, `NodeContext`.

**Change:** `StateNode::run` return type `Result<StateDelta, NodeError>` →
`Result<Directive, NodeError>`. `NodeContext` gains `muster: Option<MusterContext { payload,
task_key }>` alongside the existing `node_id`/`thread_id`/`superstep` fields — additive struct
field, same shape discipline as the existing fields.

**Every in-tree `StateNode` implementor needs `.into()`** — confirmed call sites: `graph.rs`'s
test `NoopNode`, `test_support.rs`'s `CountingFunctionNode`, and Function nodes in
`tests/integration/e2e_crash_resume_test.rs`. `impl From<StateDelta> for Directive` (defined on
`Directive` itself, see above) makes `Ok(delta.into())` compile everywhere `Ok(delta)` compiled
before.

---

### `crates/paladin-battalion/src/engine/superstep.rs` (self-modify, controller, event-driven)

**Analog:** itself (3029 lines) — `Frontier`, `compute_next_vanguard`, `evaluate_edge_condition`
(BUG-01 site), `build_waypoint`, the seeded-shuffle determinism harness.

**BUG-01 site to fix (confirmed current, ~line 1189-1213):**
```rust
EdgeCondition::Custom(_) => {
    // Engine-level custom edge predicates (mirroring
    // DispatchRule::Custom's registry) are a later plan's
    // expansion; default to firing, matching
    // campaign_service.rs's own placeholder for the same variant.
    Ok(true)  // DELETE — replace with registry.get(name) lookup + await evaluate(...)
}
```
Replacement calls `EdgeEvaluatorRegistry::get(name)` (looked up during `WarGraph::validate`, so
by the time this runs the name is guaranteed registered) and `.await`s
`EdgeConditionEvaluator::evaluate`, converting a runtime `Err` to
`EngineError::EdgeEvaluatorFailed { from, to, evaluator, source }` (D-02/D-04) — never a default
branch (CF-FR-03).

**`execute_vanguard_node`'s call site (superstep.rs:92-96, confirmed current):**
```rust
NodeDispatch::Function(node) => {
    let result = node.run(snapshot, ctx).await;
    (None, 0, result)
}
```
Widens to carry `(StateDelta, NextStep)` instead of bare `StateDelta` — Pitfall 1 flags that
`NodeInterceptor::after` (`hooks.rs`) still takes `&mut StateDelta` only; destructure `Directive`
immediately after `run`/`DirectiveParser::parse` returns and run `after` against `directive.delta`
only, unchanged (D-09 discretion note, confirmed correct against `hooks.rs`'s live signature).

**Goto/`NotFiring` insertion point:** `Frontier::record_execution` (`superstep.rs:824-847`) — a
Goto-emitting node's static outgoing edges get `EdgeState::NotFiring(superstep)` directly, skipping
`evaluate_edge_condition`; `goto_targets: Vec<NodeId>` collected in the same per-node loop
(`superstep.rs:479-522`) as `deltas`/`completed_records`, unioned into `next_vanguard` after
`compute_next_vanguard` returns (Pattern 3 in RESEARCH.md; verify against A2's flagged edge case:
a node both Goto-target this superstep AND tier-1-ready via a static edge).

**End/`StarvedNodeAtCompletion` interaction:** track `end_requested: bool` per superstep
(alongside `deltas`) and gate the suppression on that flag specifically, not on
`next_vanguard.is_empty()` generally (Pitfall 2) — the existing check lives at
`superstep.rs:598-636` and the entry-vanguard-empty path at `superstep.rs:199-237`.

**Muster dispatch pattern:** reuse the existing snapshot/spawn/semaphore machinery
(`snapshot = Arc::new(battlefield.clone())` at `superstep.rs:367`, parallelism semaphore at
`superstep.rs:369`) — each `MusterTask` becomes a synthetic vanguard entry for superstep N+1
dispatched through `execute_vanguard_node`, not a bespoke parallel loop (Pitfall 3).

**Progress-Waypoint pattern:** extend the existing `build_waypoint`/persist call sites — a
progress Waypoint's `battlefield` field is `battlefield.clone()` of the superstep-**start**
snapshot (never incrementally merged), with completed-task deltas in the new
`muster_progress: Option<MusterProgress>` field keyed by `task_key` (Pitfall 4); merge into the
real Battlefield only once, exactly like the existing end-of-superstep merge
(`superstep.rs:551-578`).

**Determinism-test pattern:** the existing seeded-shuffle harness (Phase 22 D-11,
`test_support.rs`'s `shuffle_seeded`) — extend for ≥20-iteration `task_key`-order repeat tests
(CF-FR-09/10/11).

---

### `crates/paladin-battalion/src/engine/input_mapping.rs` (self-modify, utility, transform)

**Analog:** itself (299 lines, read in full) — `InputMapping::render(&Battlefield)`.

**Change:** `render` gains an optional muster-context parameter; a `{muster.payload}`/
`{muster.task_key}` placeholder resolves from `MusterContext`, never the Battlefield; schema
validation rejects any field name starting with `muster.` so the namespace stays unambiguous
(D-15). `LlmDecisionEvaluator`'s prompt template reuses this same `render` function for its own
placeholder resolution.

---

### `crates/paladin-battalion/src/engine/mod.rs` (self-modify, service/facade)

**Analog:** itself (2642 lines) — `WarEngine`'s existing builder methods
(`with_dispatch_rule`, `with_trace_sink`, `with_interceptors`, `with_cancellation_token`) and the
already-`#[non_exhaustive]` `EngineError` enum (`mod.rs:137-139`).

**Builder-method pattern to copy (shape, not literal — mirrors `with_herald`/`with_dispatch_rule`):**
```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    // existing variants...
}
```
Since `EngineError` is already `#[non_exhaustive]`, every new variant this phase adds
(`UnregisteredEdgeCondition { names: Vec<String> }`, `EdgeEvaluatorFailed { from, to, evaluator,
source }`, `GotoUnknownNode { from, to }`, `ParleyNotSupported { node }`,
`RecursiveEmbedding { path }`, muster-limit variants) is a zero-X-10-burden addition — unlike
`BattalionError::InvalidGraph`, reused unchanged on the legacy path.

`WarEngine::with_edge_evaluator(name, Arc<dyn EdgeConditionEvaluator>)` follows the exact
`with_dispatch_rule` shape; D-21 requires it (and every other builder-configured resource — port,
durability, parallelism, dispatch resolver, trace sink, interceptors, cancellation token) to
forward wholesale into the child engine constructed for a `NodeSpec::Battalion` node's run.

---

### `crates/paladin-core/src/platform/container/waypoint.rs` (self-modify, model)

**Analog:** itself — `visit_counts`/`frontier`, the additive-`#[serde(default)]`-field precedent
proven twice already (Phase 22, Phase 22.1 BUG-04).

**Field declaration pattern (confirmed current, `waypoint.rs:363-374`):**
```rust
pub visit_counts: BTreeMap<NodeId, u32>,
/// `#[serde(default)]`, matching `visit_counts`' precedent: a `Waypoint`
/// ...
pub frontier: FrontierSnapshot,
```
`checkpoint_ns: Option<String>` (CF-04) and `muster_progress: Option<MusterProgress>` (CF-03)
follow this exact shape — `#[serde(default)]`, a `Default` impl for the field's type, **no**
`BATTLEFIELD_SCHEMA_VERSION` bump (neither prior addition bumped it — confirmed by direct read;
Assumption A3).

**Round-trip test pattern to copy (confirmed current test at `waypoint.rs:664-711`):**
```rust
#[test]
fn waypoint_payload_without_frontier_field_deserializes_with_an_empty_snapshot() {
    // build a Waypoint, serialize to serde_json::Value, remove the
    // "frontier" key entirely, assert absence, deserialize back, assert
    // restored.frontier == FrontierSnapshot::default()
}
```
Write the equivalent test for `checkpoint_ns` and `muster_progress` each.

**`ThreadId::child` encoding hazard:** `NodeId`/`FieldName` have **no charset restriction** (only
non-emptiness); `ThreadId::new` rejects whitespace and caps at 256 bytes. Do not `format!("{parent}/{node_id}")`
— use `push_field`'s length-prefixed encoding (`graph.rs:629-639`) or percent-escaping, per CR-01's
precedent (Common Pitfalls, Known Threat Patterns table).

---

### `src/config/engine.rs` (new, config, CRUD)

**Analog:** `src/config/waypoint_retention.rs` (read in full, 60 lines shown) — mirrors
`src/config/citadel.rs` field-for-field per its own rustdoc.

**Full shape to copy (imports, struct, manual `Default`, `validate`):**
```rust
use crate::config::env_utils::{EnvOverridable, read_env};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaypointRetentionConfig {
    pub enabled: bool,
    pub max_age_days: Option<u32>,
    pub max_waypoints_per_thread: Option<u32>,
}

#[allow(clippy::derivable_impls)]
impl Default for WaypointRetentionConfig {
    fn default() -> Self {
        Self { enabled: false, max_age_days: None, max_waypoints_per_thread: None }
    }
}

impl WaypointRetentionConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_waypoints_per_thread == Some(0) {
            return Err("max_waypoints_per_thread must be greater than 0 when set".to_string());
        }
        if self.max_age_days == Some(0) {
            return Err("max_age_days must be greater than 0 when set".to_string());
        }
        Ok(())
    }
}
```
`EngineConfig` carries the four fields MIGRATION.md §9.5 already documents
(`max_supersteps`, `max_node_visits`, `run_timeout_secs`, `waypoint_durability`) plus the new
`max_muster_tasks: u32` (default 100, `APP_ENGINE_MAX_MUSTER_TASKS`, D-16) — manual `Default`
(not derive), `validate()` returning `Result<(), String>`, `EnvOverridable::apply_env_overrides`
using the `if let Ok(v) = std::env::var(...) && let Ok(parsed) = v.parse() { ... }` idiom seen in
both `citadel.rs` and `waypoint_retention.rs`, plus a conversion into `EngineLimits`/
`WaypointDurability`.

**Error handling pattern:** `validate()` returns `Result<(), String>` (not a `thiserror` enum) —
matches both existing config structs exactly; do not introduce a new error type here.

---

### `tests/integration/e2e_muster_defer_order_test.rs` (new, test/integration)

**Analog:** `tests/integration/e2e_crash_resume_test.rs` (read in full, 60 lines shown here;
1000+ lines total) — E2E-1's structure.

**Imports pattern (copy the shared-helper wiring):**
```rust
use std::sync::Arc;
use std::time::Duration;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, InputMapping, NodeContext, NodeError, NodeSpec, RunOutcome, StateNode,
    WarEngine, WarGraph,
};
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId, Waypoint, WaypointStatus};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

#[allow(dead_code, unused_imports)]
#[path = "../helpers/mod.rs"]
mod helpers;
use helpers::FaultyPaladinPort;
```
Doc comment at the top should name the program scenario (E2E-3, per
`.project/v0.10.0/00-program-overview.md` §6) exactly as `e2e_crash_resume_test.rs` names E2E-1,
including the "why simulated crash via re-seeding rather than aborting a live task" rationale if
this test also exercises mid-muster resume (D-14's own crash-resume half is E2E-3's remaining
seam — this test covers the muster/defer/order half per D-17, using a manually-succeeding-on-
attempt-N mock, not a real crash simulation unless the plan also folds in mid-muster resume here).

**Formation-inside-Campaign subgraph test:** same analog and import shape; asserts
kill-after-child-superstep-1 → resume → child work not repeated (D-22/CF-FR-17 acceptance
criterion 4).

---

### `crates/paladin-storage/src/waypoint/contract_tests.rs` (self-modify, test/contract-suite)

**Analog:** itself (read in full through the helper functions shown) — `sample_waypoint`/
`sample_waypoint_at` fixture builders, per-clause named test functions (not a macro).

**Fixture pattern:**
```rust
pub fn sample_waypoint(thread: &ThreadId, superstep: u64) -> Waypoint {
    sample_waypoint_at(thread, superstep, Utc::now())
}
```
Add one new per-clause async function (e.g. `muster_progress_round_trips`,
`checkpoint_ns_round_trips`) following the existing "named per-clause, not a declarative macro, so
a failure names the violated contract clause" discipline (D-09 pattern) — every backend
(`InMemoryWaypointStore`, `SqliteWaypointStore`, `PostgresWaypointStore`) invokes the new function
unchanged from its own `#[tokio::test]`.

---

## Shared Patterns

### Async Trait (house idiom)
**Source:** `crates/paladin-battalion/src/engine/node.rs` (`StateNode`) and the general
`#[async_trait]` convention already used for `PaladinPort`, `WaypointPort`, `TraceSink`.
**Apply to:** `EdgeConditionEvaluator` (D-02 — deliberate deviation from PRD 02's sync sketch,
required because `LlmDecisionEvaluator` must `.await` an `LlmPort` call).
```rust
use async_trait::async_trait;

#[async_trait]
pub trait EdgeConditionEvaluator: Send + Sync {
    async fn evaluate(
        &self,
        output: &str,
        ctx: &EdgeContext<'_>,
    ) -> Result<bool, EdgeEvaluatorError>;
}

pub struct EdgeContext<'a> {
    pub source: &'a NodeId,
    pub target: &'a NodeId,
    pub battlefield: Option<&'a Battlefield>,
}
```

### Named registry, fail-closed at validation
**Source:** `crates/paladin-battalion/src/engine/dispatch_registry.rs` (full file)
**Apply to:** `EdgeEvaluatorRegistry` (CF-01), and by extension the discipline every new
`WarGraph::validate` clause (worker-template well-formedness CF-03, recursion rejection CF-04)
should follow — collect every offender, sorted, checked after the more-specific structural
clauses, never fail-fast on the first offender.

### Additive `#[serde(default)]` Waypoint fields, no schema-version bump
**Source:** `crates/paladin-core/src/platform/container/waypoint.rs` — `visit_counts` (Phase 22),
`frontier` (Phase 22.1 BUG-04)
**Apply to:** `checkpoint_ns` (CF-04), `muster_progress` (CF-03) — new `pub` field,
`#[serde(default)]`, a `Default` impl for the field's type, a "field absent from JSON still
deserializes" round-trip test, and **no** `BATTLEFIELD_SCHEMA_VERSION` bump.

### Additive builder method on a legacy/existing service
**Source:** `crates/paladin-battalion/src/campaign_service.rs:100` (`with_herald`)
**Apply to:** `CampaignExecutionService::with_evaluator` (CF-01), `WarEngine::with_edge_evaluator`
(CF-01, mirrors `with_dispatch_rule` specifically), `CommanderBuilder::strategy_selection` (CF-05)
— constructor signature never changes; the new capability is always an additive fluent method
storing into a new field.

### Config struct: `Default` (manual) + `validate() -> Result<(), String>` + `EnvOverridable`
**Source:** `src/config/waypoint_retention.rs` (full shape shown above), mirrors
`src/config/citadel.rs` field-for-field
**Apply to:** `src/config/engine.rs` (`EngineConfig`, CF-03/D-16 — the X-09 config-struct
obligation for `max_muster_tasks`, plus the four pre-existing engine tunables MIGRATION.md §9.5
already promises).

### Fingerprint discipline: sorted, length-prefixed, version-tagged
**Source:** `crates/paladin-battalion/src/engine/graph.rs` — `push_field` (`graph.rs:629-639`,
the CR-01 fix), `GRAPH_FINGERPRINT_VERSION`
**Apply to:** every CF-01…CF-05 change that is scheduling- or merge-relevant and therefore must
be hashed into fingerprint `v3` (D-18) — worker-template set, per-`Battalion`-node child
fingerprint/`StateMap`/`restart_on_resume`, per-Paladin-node `DirectiveParser` kind/
`on_parse_error`. Never a delimiter join for a new section.

### Typed `EngineError`, zero-burden additive variants
**Source:** `crates/paladin-battalion/src/engine/mod.rs:137-139` (`#[non_exhaustive] enum
EngineError`)
**Apply to:** every new engine-path error this phase adds (`UnregisteredEdgeCondition`,
`EdgeEvaluatorFailed`, `GotoUnknownNode`, `ParleyNotSupported`, `RecursiveEmbedding`, muster-limit
variants) — no register/X-10 burden since the enum is already `#[non_exhaustive]`. Contrast:
`BattalionError::InvalidGraph` on the legacy path is **not** `#[non_exhaustive]` — reuse the
existing variant, never add one (D-04).

### Named-per-clause contract-suite functions (not a macro)
**Source:** `crates/paladin-storage/src/waypoint/contract_tests.rs`
**Apply to:** new `muster_progress`/`checkpoint_ns` round-trip clauses — one plain async function
per clause, invoked unchanged by every backend's own test module.

## No Analog Found

None. Every file this phase creates or modifies has a direct, in-tree analog — CONTEXT.md's own
"Established Patterns" section and RESEARCH.md's "Don't Hand-Roll" table both confirm this phase's
entire pattern surface is intra-codebase (Phase 22/22.1 already solved every structural shape
this phase needs: named fail-closed registry, additive Waypoint field, config-struct scaffolding,
length-prefixed fingerprint hashing, additive builder method).

## Metadata

**Analog search scope:** `crates/paladin-battalion/src/`, `crates/paladin-core/src/platform/
container/`, `crates/paladin-storage/src/waypoint/`, `crates/paladin-llm/src/`, `src/config/`,
`tests/integration/`
**Files scanned/read directly:** `dispatch_registry.rs` (full), `campaign_service.rs` (full, via
RESEARCH.md's prior read), `commander.rs` (targeted), `graph.rs` (targeted + RESEARCH.md's full
read), `superstep.rs` (targeted BUG-01 site + RESEARCH.md's full read), `node.rs` (full, via
RESEARCH.md), `waypoint.rs` (targeted `visit_counts`/`frontier` fields), `waypoint_retention.rs`
(full), `contract_tests.rs` (targeted head), `e2e_crash_resume_test.rs` (targeted head + doc
comment)
**Pattern extraction date:** 2026-09-03
</content>
