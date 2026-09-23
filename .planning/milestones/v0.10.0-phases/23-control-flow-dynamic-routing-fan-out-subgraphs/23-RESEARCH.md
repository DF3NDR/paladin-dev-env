# Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs - Research

**Researched:** 2026-09-03
**Domain:** Rust workspace, brownfield — superstep-engine control flow (dynamic routing, dynamic
fan-out/map-reduce, subgraph composition, LLM-evaluated routing) on top of the Phase 22/22.1
`WarEngine`
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

PRD 02 (`.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md`) is the FR-level source of
truth and already locks the type shapes (`Directive`, `NextStep`, `MusterTask`, `StateMap`),
defaults (`max_muster_tasks` 100, `on_parse_error: FailRun`, `StrategySelection::Heuristic`), FR
semantics and the §4 TDD ordering. CONTEXT.md's D-01…D-30 settle only what PRD 02 left open or what
the shipped Phase 22/22.1 tree makes concrete:

- **D-01…D-06 (BUG-01 fail-closed mechanism, CF-01):** `EdgeConditionEvaluator`/`EdgeContext`/
  `EdgeEvaluatorRegistry` live in a new `paladin-battalion` module mirroring
  `dispatch_registry.rs`'s `CustomDispatchResolver` house pattern; the trait is `#[async_trait]`
  (deliberate deviation from PRD 02's sync sketch, since `LlmDecision` must `await`); registry
  placement mirrors `with_dispatch_rule` (`WarEngine::with_edge_evaluator`,
  `CampaignExecutionService::with_evaluator`); legacy path reuses `BattalionError::InvalidGraph`
  (existing variant, not `#[non_exhaustive]` — X-10 forbids adding a variant, PRD names this exact
  variant), engine path gets new `EngineError::UnregisteredEdgeCondition { names: Vec<String> }`
  (sorted) and `EngineError::EdgeEvaluatorFailed { from, to, evaluator, source }`; both placeholder
  sites die in one fix commit with RED tests committed first in a separate commit; M-B-01's worked
  example lands in `MIGRATION.md` §9.1 now.
- **D-07…D-11 (Directive semantics, CF-02):** `Directive`/`NextStep`/`MusterTask` land in
  `paladin-core` (no new core dep — `serde_json::Value` already there) with
  `impl From<StateDelta> for Directive` (`next: Edges`); `StateNode::run` becomes
  `Result<Directive, NodeError>`; Goto targets validated on receipt
  (`EngineError::GotoUnknownNode`), enter next Vanguard bypassing `Frontier::is_ready` subject to
  `max_node_visits`, emitting node's static outgoing edges resolve `NotFiring` that superstep, a
  Goto-only target must be declared via the existing `WarGraph::mark_dynamic_target`; End completes
  the run after merge (peers still merge, End beats Goto same-superstep), and
  `StarvedNodeAtCompletion` is suppressed for End-terminated runs; `NextStep::Parley` this phase is
  a typed `EngineError::ParleyNotSupported { node }`, never silently `Edges`; `DirectiveParser` is
  per-node on `NodeSpec::Paladin` (`directive_parser: DirectiveParser`, default `PlainOutput`),
  `StructuredDirective { on_parse_error: OnParseError }` writes ONLY the envelope's `delta` (no
  implicit `output_field` write), `on_parse_error: FallbackPlain` falls back to `output_field`; the
  parser is hashed into the fingerprint (D-19).
- **D-12…D-17 (Muster execution & mid-muster persistence, CF-03):** `WarGraph::add_worker_template`
  populates a `worker_templates: HashSet<NodeId>` marker set (mirrors `add_deferred_node`); a
  worker template may not be an entry, is exempt from the eligible-set "unreachable" rejection, may
  have static outgoing edges but no static incoming edges. Planner's `Muster(tasks)` in superstep N
  → all tasks execute concurrently in superstep N+1; duplicate `task_key` and `max_muster_tasks`
  breach detected when the Directive is received, before any task starts; worker deltas merge in
  lexicographic `task_key` order regardless of completion order, repeat-tested ≥20 iterations.
  Mid-muster resume via intra-superstep progress Waypoints: same superstep index, `status: Running`,
  additive `#[serde(default)] muster_progress: Option<MusterProgress>` carrying the muster spec and
  completed tasks' UNMERGED deltas keyed by `task_key` (Battlefield stays as it was at superstep
  start); one progress Waypoint per completed task; ENG-FR-11 is clarified (not changed) with a PRD
  01 note. `NodeContext` gains `muster: Option<MusterContext { payload, task_key }>`; worker
  `InputMapping` may reference `{muster.payload}`/`{muster.task_key}`, resolved from muster context
  never the Battlefield; graph validation rejects a schema field named `muster.*`. `max_muster_tasks`
  brings `EngineConfig` (`src/config/engine.rs`, X-09) into the tree — `MIGRATION.md` §9.5's
  "planned, not yet in the tree" claim closes here. A `defer: true` node downstream of a worker
  template runs exactly once after all mustered tasks resolve; E2E-3's muster/defer/order half is a
  new `tests/integration/` test with a manually-succeeding-on-attempt-N mock; the X-05 stress test
  is the PRD's 50-task muster on `flavor = "multi_thread"`.
- **D-18 (Fingerprint coverage, `v2` → `v3`):** new hashed sections — worker-template set; per
  `Battalion` node the child graph's fingerprint + `StateMap` + `restart_on_resume`; per Paladin
  node its `DirectiveParser` kind + `on_parse_error`. Still excluded: prompts, models,
  `InputMapping` templates, `EngineLimits` (incl. `max_muster_tasks`). Golden-hex test re-pinned,
  one difference test per new property.
- **D-19…D-22 (Subgraph identity & checkpoint namespacing, CF-04):**
  `NodeSpec::Battalion { graph: Arc<WarGraph>, state_map: StateMap, restart_on_resume: bool }` on
  the already-`#[non_exhaustive]` enum; `StateMap { inputs: Vec<(FieldName, FieldName)>, outputs:
  Vec<(FieldName, FieldName)> }`; parent `validate` checks mapped fields exist in both schemas,
  validates the child recursively with the SAME dispatch resolver and evaluator registry, rejects
  recursive embedding via `EngineError::RecursiveEmbedding { path }` (path-set walk over child
  fingerprints). `ThreadId::child(parent, node_id)` builds an unambiguous (length-prefixed/escaped,
  never bare-delimiter) child thread id; `Waypoint` gains additive `#[serde(default)] checkpoint_ns:
  Option<String>`; no `WaypointPort` method change — `latest(child_thread)` is the child's own
  latest namespaced Waypoint; `restart_on_resume: true` opts out. Child run inherits the parent
  engine wholesale (port, durability, parallelism, dispatch resolver, evaluator registry, trace
  sink, interceptors, cancellation token); one parent superstep spans the whole child run. Legacy
  `from_formation`/`from_phalanx`/`from_campaign` embed unchanged via `Arc::new(...)`.
- **D-23…D-26 (LLM routing shape, CF-05):** `LlmDecision` is a registered evaluator (D-01
  mechanism) under `EdgeCondition::Custom("<decision name>")`, NOT a new `EdgeCondition` variant
  (that enum is not `#[non_exhaustive]` and §9.2 already records "no variant change").
  `LlmDecisionEvaluator { llm: Arc<dyn LlmPort>, model, prompt_template, choices: Vec<(String,
  NodeId)>, on_ambiguous: OnAmbiguous::{Fail, Default(String)} }` lives in `paladin-battalion`
  beside the trait; template renders from the Battlefield via `InputMapping` on the engine path,
  from source output on the legacy path. One LLM call per (thread, superstep, source node),
  memoized — every outgoing edge of that decision judged against the same answer (else N edges → N
  independent calls → both-or-neither firing, a BUG-01-class corruption); matching is
  exact-after-trim, case-insensitive. Commander `StrategySelection::{Heuristic, Semantic { llm:
  Arc<dyn LlmPort>, model: String }}` (manual `Debug` impl), default `Heuristic`;
  `CommanderBuilder::strategy_selection(sel)` additive method; `Commander::new` signature unchanged,
  **no new public field** on `Commander` (store privately). Any LLM error or unrecognized/ambiguous
  answer falls back to heuristic with the fallback recorded in `strategy_selection_reasoning`. Off
  by default, code-configured (no `APP_*` env for `LlmDecision`/`Semantic` — X-09 adds no config
  struct for them).
- **D-27…D-30 (Program-gate obligations):** §9.2 rows resolved this phase (`EdgeCondition`,
  `Commander`/`CommanderBuilder`, `CampaignExecutionService`); a "deliberate zero" note for the
  new-in-0.10 engine/waypoint types this phase reshapes; `cargo semver-checks`/`msrv`/`make
  security`/`cargo clippy -- -D warnings` clean. New mdBook control-flow page (X-08). PRD 01 gets
  the ENG-FR-11 clarification note; `08-traceability-matrix.md`'s BUG-01 row updated with RED/GREEN
  commits. All CF work is Tier 1 (mocks/in-memory); SQLite/Postgres contract-suite additions for
  `muster_progress`/`checkpoint_ns` run in their existing tiers. Coverage stays ≥82% workspace.

### Claude's Discretion

- Module layout for `Directive`/`NextStep`/`MusterTask` in `paladin-core`; battalion module name
  for the evaluator trait/registry; whether `LlmDecisionEvaluator` gets its own file.
- The `NodeSpec::Paladin` constructor keeping existing literals compiling after D-11; whether
  `NodeSpec::Battalion` gets a builder.
- Exact `EngineError` variant names/messages for D-02, D-04, D-08, D-10, D-13, D-19; duplicate
  evaluator-name registration policy (typed error vs replace — mirror `ReservedDispatchName`).
- `MusterProgress`, `MusterContext`, `FrontierSnapshot` interplay and `checkpoint_ns` exact shapes
  (core value types per ADR-0016); whether `BATTLEFIELD_SCHEMA_VERSION` bumps for the additive
  Waypoint fields (follow the `visit_counts`/`frontier` precedent — precedent is NOT to bump it,
  see Code Examples below).
- How `InputMapping::render` receives the muster context (D-15) and how the `muster.` prefix
  rejection is phrased.
- Memo scope for D-24; the Semantic prompt wording for D-25; `restart_on_resume` child-thread
  policy for D-20.
- Whether `NodeInterceptor::after` should see `NextStep` (recommended: leave the ENG-07 hook
  untouched this phase).
- Plan/wave decomposition, respecting PRD 02 §4's TDD order. Suggested: (1) CF-01 RED tests → fix
  on both paths → M-B-01 example; (2) Directive types, `StateNode` change, Goto/End/Parley arms,
  `DirectiveParser`; (3) Muster (marker, timing, limits, progress Waypoints, `muster.` namespace),
  `EngineConfig`, fingerprint `v3`; (4) subgraphs (variant, `StateMap`, child threads, inheritance,
  recursion check, bridge-embedding test); (5) `LlmDecision` + Commander Semantic; (6) E2E-3 half,
  stress test, mdBook page, MIGRATION.md sweep, CI evidence. (2) is a prerequisite of (3)–(5); (4)
  and (5) are independent of each other.

### Deferred Ideas (OUT OF SCOPE)

- No mdBook page for the WarEngine itself exists — D-28 adds only a short control-flow preamble;
  the full engine page is a Phase 29 (SHIP-01) / docs-pass item.
- Overview §4 describes Directive routing as `Goto`, `End`, `Halt` — PRD 02's `NextStep` has no
  `Halt`. PRD 02 governs; the overview wording is a Phase 29 doc-sweep item.
- Native provider JSON mode for `StructuredDirective` — RT-05 (Phase 26) may swap D-11's text
  extraction for `execute_structured`/`response_format`; the envelope is designed to be reusable.
- `NodeInterceptor` visibility of `NextStep` — Phase 26 middleware may want it; ENG-07 untouched
  here.
- Per-task retry inside a Muster (FT-FR-06, Phase 25), Parley suspension (HITL-01, Phase 24),
  subgraph fork semantics (HITL-FR-12, Phase 24) — seams only, not implemented this phase.
- `22-deferred-items.md` item 1 (`qdrant` `--all-features` rustdoc break) and rmcp 3.x — unchanged.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CF-01 | BUG-01 fixed fail-closed, test-first: registered-evaluator mechanism on both `CampaignExecutionService` and `WarEngine`; validation fails naming every unregistered `Custom(name)` before any node executes; warn-and-return-true removed; runtime evaluator errors fail the run | Exact defect sites confirmed at `campaign_service.rs:392-397` (`evaluate_edge_condition`) and `engine/superstep.rs:1206-1211`; `dispatch_registry.rs`'s `DispatchRegistry` is the concrete pattern to mirror for `EdgeEvaluatorRegistry`; `campaign.validate()` call site at `campaign_service.rs:174` is where the legacy fail-closed pre-check must run; `WarGraph::validate` at `graph.rs:290` is where the engine-side check joins `validate_eligible_set`/`validate_schedulable`. See Architecture Patterns §1, Code Examples §1. |
| CF-02 | `Directive`/`NextStep::{Edges,Goto,End,Muster,Parley}`; `StateNode::run` returns it; validated `Goto`; End-over-Goto precedence; `DirectiveParser` (`PlainOutput` default, `StructuredDirective`) | `StateNode::run`'s exact signature and its one call site in `execute_vanguard_node` (superstep.rs:86-120) confirmed; the Paladin output-write point (`delta.set(output_field, result.output.clone())`, superstep.rs:111) is the exact `DirectiveParser` insertion seam; `Frontier`/`compute_next_vanguard`/`EdgeState` internals fully read (superstep.rs:677-1180) — Goto injection and `NotFiring` resolution mapped to exact functions. See Architecture Patterns §2, Common Pitfalls §1-2. |
| CF-03 | Muster dynamic fan-out: runtime-N workers in one superstep, payload isolation, deterministic `task_key` aggregation, duplicate-key rejection, `max_muster_tasks`, mid-muster resume | `WarGraph::add_deferred_node`/`defer_flags`/`mark_dynamic_target` read as the exact model `add_worker_template` must mirror; `superstep::run`'s per-superstep Waypoint-write call sites (6 distinct `build_waypoint`/`persist_waypoint` pairs) identified as where intra-superstep progress Waypoints slot in; `EngineConfig` pattern confirmed absent, `CitadelConfig`/`WaypointRetentionConfig` read in full as the X-09 shape to mirror. See Architecture Patterns §3, Common Pitfalls §3-5, Code Examples §3. |
| CF-04 | `NodeSpec::Battalion` subgraph with `StateMap`, namespaced checkpoint inheritance, resume-mid-child, recursive-embedding rejection, legacy patterns embeddable | `NodeSpec` confirmed `#[non_exhaustive]` already (rustdoc at `graph.rs:31` pre-announces `Battalion`); `Waypoint`'s `#[serde(default)]` additive-field precedent (`visit_counts`, `frontier`) read in full as the exact pattern `checkpoint_ns` follows; `ThreadId::new` confirmed to reject whitespace and cap at 256 bytes (constrains `ThreadId::child`'s encoding), while `NodeId`/`FieldName` have NO charset restriction (only non-emptiness) — this is the exact hazard CR-01 already exploited once. See Common Pitfalls §6, Code Examples §4. |
| CF-05 | `LlmDecision` edge evaluator (off by default) + Commander `StrategySelection::Semantic`, deterministic fallback, existing Commander tests pass unmodified | `LlmPort` trait location confirmed in `paladin-ports` (already a `paladin-battalion` dependency — no new prod dep needed); `paladin-llm`/`MockLlmAdapter` confirmed NOT currently a dependency (dev or prod) of `paladin-battalion` — a new dev-dependency line is required for D-24/D-25 tests, verified acyclic (`paladin-llm` depends only on `paladin-core`+`paladin-ports`); `CommanderBuilder`'s private-field construction pattern read in full, confirming a new private `Commander` field is safe regardless of `#[non_exhaustive]` status. See Package Legitimacy Audit, Common Pitfalls §7. |
</phase_requirements>

## Summary

Phase 23 is a pure code-continuation phase inside an already-live, fully-typed superstep engine
(`crates/paladin-battalion/src/engine/`). Nothing in this phase pulls in a new external crate for
production code — `async-trait`, `serde_json`, `blake3`, `thiserror`, `uuid`, `tokio` are all
already workspace dependencies, and the one new *internal* dependency edge (`paladin-llm` as a
`paladin-battalion` dev-dependency, for `MockLlmAdapter` in CF-05 tests) is acyclic and safe. The
phase's difficulty is entirely in getting five interlocking scheduler changes right against a
`Frontier`/`compute_next_vanguard` state machine that already carries two hard-won defect fixes
(BUG-03 starvation release, BUG-04 frontier persistence) and one hard-won encoding fix
(CR-01's length-prefixed fingerprint). Every one of CONTEXT.md's D-01…D-30 decisions is directly
grounded against the live tree, and this research did not find any decision that is infeasible
against the code as read — the CONTEXT.md decisions are implementation-ready.

The five capability clusters land in a strict dependency order the code confirms: CF-01 (BUG-01) is
genuinely standalone — its two defect sites (`campaign_service.rs:392-397`,
`engine/superstep.rs:1206-1211`) do not touch scheduling. CF-02 (Directive) is the prerequisite for
everything else: `StateNode::run`'s return-type change ripples through the one call site in
`execute_vanguard_node`, and Goto/End must be threaded through `Frontier`/`compute_next_vanguard`
before Muster (CF-03, itself a Directive variant) or the `Battalion` node's child-run completion
(CF-04, which returns a delta the parent dispatch rules treat like any other node's output) can be
built on top. CF-04 (subgraphs) and CF-05 (LLM routing) are mutually independent once CF-02/CF-03
land.

**Primary recommendation:** Sequence plans exactly as CONTEXT.md's discretion note suggests — CF-01
standalone first, then CF-02 as the sole prerequisite wave, then CF-03/CF-04/CF-05 in parallel
(CF-03 has the heaviest engine-internals surface; CF-04 and CF-05 barely touch `superstep.rs` at
all) — and budget CF-03 (muster + progress Waypoints + `EngineConfig` + fingerprint v3) as the
largest single wave in the phase by a wide margin, since it is the only cluster that adds new
per-superstep persistence semantics to the hot resume path.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| BUG-01 fail-closed evaluator registry (CF-01) | API / Backend (`paladin-battalion`, application layer) | Database / Storage (none — validation-time only) | Evaluator registration and validation are pure orchestration-layer concerns; no new persistence. |
| Directive / NextStep routing (CF-02) | API / Backend (`paladin-battalion::engine::superstep`) | Database / Storage (`Waypoint`'s `vanguard`/`completed` fields already carry routing outcomes) | Routing decisions are made and consumed entirely inside the superstep loop; persistence is a side effect via the existing Waypoint write, not a new store. |
| Muster dynamic fan-out (CF-03) | API / Backend (`paladin-battalion::engine::superstep`) | Database / Storage (new `muster_progress` field on `Waypoint`, three-backend JSON payload — no new table/migration) | Worker dispatch and aggregation are engine-internal; the ONLY new storage surface is an additive field inside the existing `waypoints` row's JSON payload. |
| Subgraph composition (CF-04) | API / Backend (`paladin-battalion::engine::graph`/`mod.rs`) | Database / Storage (`checkpoint_ns` on `Waypoint`; child threads are ordinary rows under the SAME `WaypointPort`) | A child run reuses the parent's engine and port wholesale; no new backend, no new port method. |
| LLM-evaluated routing (CF-05) | API / Backend (`paladin-battalion`, holds `Arc<dyn LlmPort>`) | — | `paladin-core` stays LLM-ignorant (ADR-0015); the evaluator is an application-layer adapter over the existing `LlmPort` port, not a new port. |

## Standard Stack

### Core

No new external crates. Every type this phase introduces is built from already-present workspace
dependencies:

| Library | Version | Purpose | Why Standard (in this tree) |
|---------|---------|---------|------------------------------|
| `async-trait` | 0.1.88 (workspace-pinned) | `#[async_trait]` on `EdgeConditionEvaluator` (D-02) | Already used identically for `PaladinPort`, `WaypointPort`, `TraceSink`, `StateNode`. |
| `serde_json` | workspace-pinned | `Directive.delta: StateDelta` reuses `serde_json::Value`; `MusterTask.payload` | Already a `paladin-core` dependency (ADR-0015 allowlist); no new core dependency for `Directive`/`NextStep`/`MusterTask`. |
| `thiserror` | workspace-pinned | New `EngineError` variants (`UnregisteredEdgeCondition`, `EdgeEvaluatorFailed`, `GotoUnknownNode`, `ParleyNotSupported`, `RecursiveEmbedding`, etc.) | `EngineError` is already `#[error(...)]`/`#[non_exhaustive]` via `thiserror`; new variants are additive to an enum already marked `#[non_exhaustive]` — zero X-10 register burden (confirmed, see Code Examples §1). |
| `blake3` | workspace-pinned | Fingerprint `v3` bump (D-18) | Already the fingerprint hash function (`GraphFingerprint::from_canonical_bytes`); no algorithm change, only new hashed sections. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `paladin-llm` (workspace crate) | 0.9.0 (path dep) | `MockLlmAdapter` for CF-05 tests (D-24/D-25) | **New dev-dependency line needed** on `paladin-battalion`'s `Cargo.toml` — confirmed absent today (`paladin-battalion`'s only current deps are `paladin-core`+`paladin-ports`). Verified acyclic: `paladin-llm` depends only on `paladin-core`+`paladin-ports`, never on `paladin-battalion`. |
| `paladin-ports::output::llm_port::LlmPort` | in-tree | `Arc<dyn LlmPort>` for `LlmDecisionEvaluator` and `StrategySelection::Semantic` | Already reachable from `paladin-battalion` (existing `paladin-ports` prod dependency) — no new prod dependency for CF-05's production code, only its tests. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled `EdgeEvaluatorRegistry` (`HashMap<String, Arc<dyn EdgeConditionEvaluator>>`) | A generic plugin/registry crate (e.g. `inventory`, `linkme`) | Rejected by CONTEXT.md D-01 implicitly — the house pattern (`DispatchRegistry`) already solves this exact problem with zero extra dependencies and is the established idiom in this crate; introducing a compile-time registration crate would be inconsistent with `with_dispatch_rule`'s runtime-builder style. |
| `blake3` v3 fingerprint bump | Switching hash algorithms (e.g. to `sha2`) | Not considered — no reason to change algorithms; only the canonical-byte layout changes (new sections appended), which the existing `push_field` length-prefixed helper already handles safely. |

**Installation:** None — no `Cargo.toml` change to any published-crate `[dependencies]` table.
Only `[dev-dependencies] paladin-llm = { version = "0.9.0", path = "../paladin-llm" }` is added to
`crates/paladin-battalion/Cargo.toml`.

**Version verification:** All dependencies used by this phase are already resolved in the
workspace lockfile at their current pinned versions (`async-trait 0.1.88`, `blake3` per workspace
pin, `thiserror` per workspace pin). No `cargo view`/registry check is needed — this is an
internal-crate-only change; `cargo build -p paladin-battalion` after adding the dev-dependency line
is the correct verification step, not a registry lookup.

## Package Legitimacy Audit

**No external packages are installed by this phase.** The only dependency-graph change is a new
intra-workspace edge (`paladin-battalion` dev-dependency on the already-published, already-in-tree
`paladin-llm` crate at its existing pinned version `0.9.0`). This is not a registry package
addition and the Package Legitimacy Gate does not apply — there is no `npm view`/`pip
index`/`cargo search` verification to perform against a workspace-local path dependency.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `paladin-llm` (dev-dep addition) | workspace-local path dependency | pre-existing in this workspace (not a new crate) | N/A (not fetched from crates.io in this configuration) | this repository, `crates/paladin-llm/` | N/A — not an external package | Approved (verified acyclic against `paladin-battalion`) |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────────────────┐
                    │  WarGraph::validate(registry, evaluator_registry)        │
                    │  (graph.rs) — BEFORE any node executes                   │
                    │                                                          │
  Custom("name")    │  1. limits non-zero                                     │
  edge condition ───┼─▶ 2. edges/entry reference known nodes                  │
  in schema         │  3. Custom dispatch names registered (ENG-FR-09)        │
                    │  4. eligible-set reachability (BUG-02)                  │
                    │  5. unschedulable-cycle guard (BUG-03)                  │
                    │  6. NEW: every EdgeCondition::Custom(name) has a        │
                    │     registered EdgeConditionEvaluator (CF-01)           │
                    │  7. NEW: worker templates well-formed (CF-03)           │
                    │  8. NEW: no NodeSpec::Battalion recursion (CF-04)       │
                    └───────────────────────┬─────────────────────────────────┘
                                             │ Ok(())
                                             ▼
        ┌───────────────────────────────────────────────────────────────────┐
        │ superstep::run  (superstep.rs) — one iteration of the loop:       │
        │                                                                    │
        │  Vanguard ──▶ execute_vanguard_node per node (concurrent) ──▶      │
        │   ┌─ NodeSpec::Function → StateNode::run(state, ctx)               │
        │   │    -> Result<Directive, NodeError>            (CF-02)         │
        │   ├─ NodeSpec::Paladin  → PaladinPort::execute + DirectiveParser   │
        │   │    PlainOutput: delta.set(output_field, output), next: Edges  │
        │   │    StructuredDirective: parse envelope -> Directive  (CF-02)  │
        │   └─ NodeSpec::Battalion → child WarEngine::start/resume,         │
        │        StateMap maps parent fields -> child schema in,            │
        │        child final_state -> StateMap outputs -> parent delta (CF-04)│
        │                                                                    │
        │  deltas.sort_by(NodeId) ──▶ battlefield.merge(...)                │
        │                                                                    │
        │  frontier.record_execution per ran node:                          │
        │    Directive::next drives THIS node's edge resolution:            │
        │      Edges -> evaluate_edge_condition per static outgoing edge     │
        │               (NEW: Custom(name) -> registry.evaluate(...).await) │
        │      Goto(targets) -> static outgoing edges -> NotFiring;          │
        │               targets validated + injected into next Vanguard      │
        │      Muster(tasks) -> spawn worker superstep N+1 (CF-03)          │
        │      End -> run completes after this superstep's merge            │
        │      Parley -> EngineError::ParleyNotSupported (this phase)       │
        │                                                                    │
        │  compute_next_vanguard(graph, frontier)                           │
        │    tier 1: normal-ready  tier 2: starved_release (BUG-03)          │
        │    tier 3: defer-ready   tier 4: starved_deferred_release          │
        │    + Goto-injected targets folded in ahead of tier 1 (CF-02)       │
        │                                                                    │
        │  build_waypoint(...) -> persist_waypoint (ONE per superstep,       │
        │    PLUS zero-or-more muster_progress Waypoints mid-superstep       │
        │    at the SAME superstep index, CF-03/D-14)                        │
        └───────────────────────────────────────────────────────────────────┘
                                             │
                                             ▼
                              WaypointPort::save (InMemory | SQLite | Postgres)
                              — whole Waypoint as JSON; frontier/checkpoint_ns/
                                muster_progress all additive #[serde(default)]
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
├── waypoint.rs              # Waypoint gains checkpoint_ns, muster_progress fields (additive)
├── directive.rs             # NEW (or inline in waypoint.rs/battlefield.rs, Claude's discretion):
│                             #   Directive, NextStep, MusterTask, MusterProgress, MusterContext
└── battlefield.rs           # unchanged — StateDelta already exists; Directive wraps it

crates/paladin-battalion/src/
├── edge_evaluator.rs         # NEW (or engine/edge_evaluator.rs): EdgeConditionEvaluator trait,
│                             #   EdgeContext, EdgeEvaluatorRegistry — mirrors dispatch_registry.rs
├── llm_decision.rs           # NEW (or beside edge_evaluator.rs): LlmDecisionEvaluator
├── campaign_service.rs       # + with_evaluator(name, evaluator); fail-closed pre-check after
│                             #   campaign.validate() at line 174; evaluate_edge_condition (line
│                             #   378) becomes async, consults the registry for Custom
├── commander.rs              # + StrategySelection enum; CommanderBuilder::strategy_selection;
│                             #   Commander private field; analyze_and_select gains a Semantic path
└── engine/
    ├── graph.rs               # + worker_templates: HashSet<NodeId>, add_worker_template,
    │                          #   NodeSpec::Battalion variant, validate() gains evaluator registry
    │                          #   param + worker-template well-formedness + recursion check,
    │                          #   fingerprint() v3 sections
    ├── node.rs                 # StateNode::run -> Result<Directive, NodeError>;
    │                          # NodeContext gains muster: Option<MusterContext>
    ├── superstep.rs            # Frontier/compute_next_vanguard gain Goto injection + NotFiring
    │                          #   resolution; muster dispatch + progress Waypoints;
    │                          #   evaluate_edge_condition's Custom(_) arm calls the registry
    ├── input_mapping.rs        # render() gains an optional muster-context parameter;
    │                          #   muster. prefix validation
    ├── mod.rs                  # WarEngine::with_edge_evaluator; new EngineError variants;
    │                          #   child-run construction inside superstep.rs's Battalion dispatch
    └── edge_evaluator_registry.rs  # (if not top-level) EdgeEvaluatorRegistry wiring into WarEngine

src/config/
└── engine.rs                 # NEW: EngineConfig (Default, validate(), EnvOverridable) — mirrors
                              #   citadel.rs/waypoint_retention.rs exactly; adds max_muster_tasks

tests/integration/
├── e2e_crash_resume_test.rs  # unmodified — E2E-1 golden, must stay green
└── e2e_muster_defer_order_test.rs  # NEW — E2E-3's muster/defer/order half (CF-03/D-17)
```

### Pattern 1: Fail-closed evaluator registry, mirroring `DispatchRegistry` (CF-01)

**What:** A `HashMap<String, Arc<dyn EdgeConditionEvaluator>>`-backed registry with a `register`
method and a `resolver()`/borrow accessor, validated against at `WarGraph::validate` time and
consulted at runtime — the exact shape `crates/paladin-battalion/src/engine/dispatch_registry.rs`
already establishes for `DispatchRule::Custom`.

**When to use:** Any "named extension point resolved against a registry, fail-closed at
validation" requirement in this codebase — this is now a two-for-two house pattern
(`DispatchRegistry` for `DispatchRule::Custom`, and now `EdgeEvaluatorRegistry` for
`EdgeCondition::Custom`).

**Example (adapted from the live `DispatchRegistry`, `dispatch_registry.rs:32-67`):**
```rust
// Source: crates/paladin-battalion/src/engine/dispatch_registry.rs (existing pattern)
#[derive(Default)]
pub struct EdgeEvaluatorRegistry {
    inner: HashMap<String, Arc<dyn EdgeConditionEvaluator>>,
}

impl EdgeEvaluatorRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, name: impl Into<String>, evaluator: Arc<dyn EdgeConditionEvaluator>) {
        self.inner.insert(name.into(), evaluator);
        // Unlike DispatchRegistry, there is no RESERVED_NAMES collision to
        // guard against here -- EdgeCondition::Custom names are arbitrary
        // strings with no built-in-variant name to collide with (unlike
        // DispatchRule::Custom colliding with "LastWrite" etc).
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn EdgeConditionEvaluator>> {
        self.inner.get(name)
    }

    /// Every registered name, sorted -- for validate()'s "list every
    /// unregistered name" message and for the eventual difference check.
    pub fn registered_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.inner.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }
}
```

**Fail-closed validation, mirroring the existing `Custom` dispatch check at `graph.rs:317-325`:**
```rust
// Source: crates/paladin-battalion/src/engine/graph.rs:317-325 (existing pattern to mirror)
for field in &self.schema.fields {
    if let DispatchRule::Custom(name) = &field.dispatch
        && !custom_dispatch.contains_key(name)
    {
        return Err(EngineError::Battlefield(
            BattlefieldError::CustomDispatchNotRegistered { name: name.clone() },
        ));
    }
}
// NEW, mirroring the above exactly, but collecting ALL offenders (not
// erroring on the first) per CF-FR-02's "naming every unregistered Custom
// condition" requirement:
let mut unregistered: Vec<&str> = self.edges.iter()
    .filter_map(|e| match &e.condition {
        Some(EdgeCondition::Custom(name)) if evaluator_registry.get(name).is_none() => Some(name.as_str()),
        _ => None,
    })
    .collect();
unregistered.sort_unstable();
unregistered.dedup();
if !unregistered.is_empty() {
    return Err(EngineError::UnregisteredEdgeCondition { names: unregistered.iter().map(|s| s.to_string()).collect() });
}
```

### Pattern 2: `Directive`-returning `StateNode`, one call site to change (CF-02)

**What:** `StateNode::run`'s return type changes from `Result<StateDelta, NodeError>` to
`Result<Directive, NodeError>`. The ENTIRE blast radius inside the engine is exactly one call site:
`execute_vanguard_node` (`superstep.rs:86-120`), which currently does:

```rust
// Source: crates/paladin-battalion/src/engine/superstep.rs:92-96 (current, pre-CF-02)
NodeDispatch::Function(node) => {
    let result = node.run(snapshot, ctx).await;
    (None, 0, result)
}
```

**After CF-02**, this becomes (illustrative, not the final signature — `execute_vanguard_node`'s
return type must also widen to carry `NextStep`):
```rust
NodeDispatch::Function(node) => {
    let result = node.run(snapshot, ctx).await; // now Result<Directive, NodeError>
    match result {
        Ok(directive) => (None, 0, Ok((directive.delta, directive.next))),
        Err(e) => (None, 0, Err(e)),
    }
}
NodeDispatch::Paladin { paladin, input_template, output_field, directive_parser } => {
    // ... existing render/execute unchanged ...
    let directive = directive_parser.parse(&result.output, &output_field)?; // CF-FR-06
    (paladin_id, token_count, Ok((directive.delta, directive.next)))
}
```

**When to use:** This is the single required change for CF-02; every other Directive-related
mechanism (Goto injection, End precedence, Muster dispatch) is downstream of what `NextStep` this
call site's result carries into `frontier.record_execution`/`compute_next_vanguard`.

**Every in-tree `StateNode` implementor needs a one-line `.into()` change** (D-07's promise) —
confirmed call sites: `graph.rs`'s test `NoopNode` (returns `StateDelta::new()`), `test_support.rs`'s
`CountingFunctionNode`, and any Function node in `tests/integration/e2e_crash_resume_test.rs`
(`loop_gate`-style nodes). `impl From<StateDelta> for Directive` makes every one of these compile
with `Ok(delta.into())` in place of `Ok(delta)`.

### Pattern 3: Goto injection and `NotFiring` resolution inside `Frontier` (CF-02/CF-03)

**What:** The existing `Frontier::record_execution` (`superstep.rs:824-847`) evaluates every
STATIC outgoing edge of a just-ran node against the post-merge Battlefield. A `Goto` directive must:
(a) mark every one of the emitting node's static outgoing edges `NotFiring` for that superstep
(D-08c) — i.e. `record_execution`'s loop must skip the normal `evaluate_edge_condition` call for a
Goto-emitting node and instead set every one of its outgoing edges to `EdgeState::NotFiring(superstep)`
directly; (b) validate every Goto target against `graph.node(target).is_some()` at Directive-receipt
time (`EngineError::GotoUnknownNode`); (c) inject the validated targets directly into
`compute_next_vanguard`'s result, ahead of or alongside tier 1 (they must NOT go through
`Frontier::is_ready`, per D-08b — a Goto target's admission is unconditional, not the normal
"has a fresh fired incoming edge" test).

**Concrete integration point:** `compute_next_vanguard` (`superstep.rs:1049-1082`) currently returns
purely from `Frontier` state (four tiers). A `goto_targets: Vec<NodeId>` collected during this
superstep's node execution must be threaded into `run`'s loop body (it is NOT frontier-derived — it
is a runtime value from THIS superstep's Directives, analogous to how `vanguard`/`next_vanguard`
already flow as local variables in `run`, not as `Frontier` fields). The cleanest seam: collect
`goto_targets` in the same loop at `superstep.rs:479-522` where `deltas`/`completed_records` are
already being accumulated per node, then union them into `next_vanguard` after
`compute_next_vanguard(graph, &frontier)` returns, de-duplicating against nodes already present.

**A `NodeSpec::Battalion` node's Directive** does not itself carry `Goto`/`Muster`/`End`/`Parley` —
per CF-FR-14, a Battalion node's mapped outputs "return as the node's delta under parent dispatch
rules," meaning a `Battalion` node's own contribution to the superstep is always effectively
`NextStep::Edges` (its child run's own internal routing is invisible to the parent's Frontier). This
is a simplifying property worth stating explicitly in the plan: subgraph composition does NOT need
its own `NextStep` variant or Frontier-level special case beyond the existing `NodeSpec::Paladin`/
`NodeSpec::Function` treatment.

### Pattern 4: Additive `#[serde(default)]` Waypoint fields — proven three times already

**What:** `visit_counts` (Phase 22) and `frontier` (Phase 22.1 BUG-04) both establish the exact
pattern `checkpoint_ns` and `muster_progress` must follow: a new `pub` field on `Waypoint`,
`#[serde(default)]`, with a `Default` impl for its type, and NO `BATTLEFIELD_SCHEMA_VERSION` bump
(`Waypoint::current_schema_version()` just stamps `BATTLEFIELD_SCHEMA_VERSION`, which stayed
`"1.0.0"` through both prior additions per the read source). Confirmed by the exact test present
in `waypoint.rs` today:

```rust
// Source: crates/paladin-core/src/platform/container/waypoint.rs:664-711 (existing, CF-04/CF-03 follow this)
#[test]
fn waypoint_payload_without_frontier_field_deserializes_with_an_empty_snapshot() {
    // ... builds a Waypoint, serializes to serde_json::Value, removes the
    // "frontier" key entirely, asserts the key is genuinely absent, then
    // deserializes back and asserts `restored.frontier == FrontierSnapshot::default()`
}
```

**When to use:** This is the mandatory pattern for `checkpoint_ns: Option<String>` (CF-04) and
`muster_progress: Option<MusterProgress>` (CF-03) — write the equivalent "field absent from JSON
still deserializes" round-trip test for each, and do NOT bump `BATTLEFIELD_SCHEMA_VERSION` (the
two prior additive fields didn't; CONTEXT.md leaves this "Claude's discretion" but the strong
precedent, observed directly in the tree, is not to bump it).

### Pattern 5: Config struct mirroring `CitadelConfig`/`WaypointRetentionConfig` exactly (CF-03)

**What:** `EngineConfig` (`src/config/engine.rs`, currently absent — confirmed by `grep` and by
`MIGRATION.md` §9.5's own "not yet in the tree" note) must be a field-for-field mirror of the
existing pattern: `Default` (manual impl, not derive, per `WaypointRetentionConfig`'s own comment
explaining why), `validate()` returning `Result<(), String>`, and `EnvOverridable::apply_env_overrides`
reading `APP_ENGINE_*` vars with the `if let Ok(v) = std::env::var(...) && let Ok(parsed) =
v.parse() { ... }` idiom seen in both existing configs.

**MIGRATION.md §9.5 already specifies the four pre-existing fields verbatim** (`max_supersteps`,
`max_node_visits`, `run_timeout_secs`, `waypoint_durability`) — this phase adds exactly one more:
`max_muster_tasks: u32` (default 100, `APP_ENGINE_MAX_MUSTER_TASKS`), plus the conversion into
`EngineLimits`/`WaypointDurability` MIGRATION.md's existing bullet promises.

### Anti-Patterns to Avoid

- **Do not re-derive "unreachable" logic for worker templates.** `WarGraph::validate_eligible_set`
  (`graph.rs:334-409`) already has a documented, unfilled insertion point for exactly this: its own
  rustdoc says "Two future sources of eligibility plug into this SAME worklist and nowhere else:
  nodes marked as worker templates... and nodes named as Route { to } targets... Neither concept
  exists in this tree yet." CF-03 must seed `worker_templates` into the SAME eligible-set worklist
  the existing fixpoint already walks (`graph.rs:368-374`), not write a parallel reachability check.
- **Do not let a Goto target bypass `max_node_visits`.** D-08b is explicit: "subject to
  `max_node_visits` (loops via Goto legal and bounded)." The existing visit-count enforcement
  (`superstep.rs:320-358`) runs generically over `vanguard` before dispatch — as long as Goto
  targets are folded into `next_vanguard`/`vanguard` through the normal path (not a side-channel),
  this falls out for free. A bug here would look like an infinite Goto loop with no
  `NodeVisitLimitExceeded`.
- **Do not compute the muster-duplicate-key / `max_muster_tasks` checks after dispatching tasks.**
  D-13 requires both detected "before any task starts" with typed errors — this must happen at
  Directive-receipt time, in the SAME place Goto-target validation happens (both are "validate this
  runtime-produced NextStep before acting on it" concerns), not inside the worker-dispatch loop
  itself where a partial failure after some tasks already launched would be unrecoverable cleanly.
- **Do not evaluate `LlmDecision` once per outgoing edge.** D-24 names this explicitly as a
  BUG-01-class corruption risk (N edges → N independent calls → both-or-neither firing). The memo
  key is `(thread, superstep, source_node)`, resolved ONCE, then every outgoing `Custom(decision_name)`
  edge of that source node in that superstep consults the memo rather than re-invoking
  `EdgeConditionEvaluator::evaluate`.
- **Do not build `ThreadId::child` with a bare delimiter join.** CR-01 (22.1-REVIEW.md) already
  proved this exact class of defect once, in the fingerprint encoding, for exactly this reason:
  `NodeId` has NO charset restriction (`waypoint.rs:130-140`, only `impl NodeId::new` — no
  validation beyond wrapping the string). A `format!("{parent}/{node_id}")`-style join is exactly
  as exploitable as the fingerprint's old `"-"`/`"|"`/`":"` delimiters were — a node named
  `"a/b"` under parent thread `"t"` collides with node `"b"` under parent thread `"t/a"`. Use a
  length-prefixed or percent-escaped encoding (mirroring `push_field` in `graph.rs:636-639`), and
  note `ThreadId::new` ALSO rejects whitespace and caps at 256 bytes — the child id must still pass
  `ThreadId::new`'s validation, so whatever encoding is chosen must not introduce whitespace.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Named-registry-with-fail-closed-validation | A custom trait-object lookup mechanism, a plugin-discovery macro, or a config-driven string-to-behavior map | The `DispatchRegistry` pattern, copied verbatim in shape for `EdgeEvaluatorRegistry` | This exact problem (named extension point, must fail validation before any node executes if a name is unregistered) is already solved once in this codebase, tested, and it is the pattern the CONTEXT.md decisions were written against. |
| Deterministic collection hashing for the fingerprint | A new hash-input builder, JSON canonicalization crate, or ad hoc string-join | `push_field`'s length-prefixed byte-writer (`graph.rs:629-639`), extended with new `push_field` calls for the CF additions | Already proven collision-free (CR-01 fixed exactly the delimiter-join alternative this would otherwise reinvent); reusing the same helper for the `v3` additions is strictly safer than any new approach. |
| Cross-supersede-boundary edge-resolution persistence | A custom "what fired before" tracking structure for Goto/Muster | `FrontierSnapshot`/`Frontier::snapshot`/`Frontier::from_snapshot` (`waypoint.rs:229-250`, `superstep.rs:961-1009`) | BUG-04 already solved "persist and restore per-edge resolution state across a crash" for the general case; Goto's `NotFiring` marking and Muster's task completion tracking are both variations on data this machinery already carries, not a new persistence concept. |
| Config struct scaffolding (`Default`+`validate()`+`EnvOverridable`) | A bespoke `EngineConfig` shape, a builder pattern, or a `serde`-only struct with no validation | Copy `WaypointRetentionConfig`'s exact shape (`src/config/waypoint_retention.rs`, itself explicitly documented as mirroring `CitadelConfig` "field-for-field") | X-09 requires this shape for every new tunable; the codebase already has two worked examples with tests to copy from, including the exact env-var-reading idiom. |

**Key insight:** This phase's entire "don't hand-roll" surface is intra-codebase, not
external-library. Every mechanism CF-01 through CF-05 needs (named registry, deterministic hash
extension, cross-crash state persistence, config scaffolding) has already been built once in this
exact tree for a structurally identical problem in Phase 22/22.1. The research risk in this phase
is not "which library to pick" — it is "did you find the existing pattern before re-inventing it,"
and every pattern has been located above with file:line anchors.

## Common Pitfalls

### Pitfall 1: `execute_vanguard_node`'s return type change ripples further than `StateNode::run`

**What goes wrong:** `execute_vanguard_node` (`superstep.rs:86-120`) currently returns `(Option<Uuid>,
u64, Result<StateDelta, NodeError>)`. Its caller inside the `tokio::spawn`ed closure
(`superstep.rs:419-457`) pattern-matches `Ok(mut delta) => { ... for interceptor in
&node_interceptors { interceptor.after(&ctx, &mut delta).await; } ... }` — `NodeInterceptor::after`
(ENG-FR-22, `hooks.rs`) takes `&mut StateDelta`, NOT `&mut Directive`. CONTEXT.md's discretion note
already flags this ("Whether `NodeInterceptor::after` should see `NextStep`... recommended: leave
the ENG-07 hook untouched this phase") — confirmed correct by reading `hooks.rs`'s signature: the
minimal-blast-radius choice is to run `after` against `directive.delta` only, leaving `directive.next`
untouched by any interceptor this phase.

**Why it happens:** `Directive` bundles two previously-separate concerns (`delta` and routing) into
one return value; every downstream consumer that only cared about `delta` needs an explicit
decision about whether it also now sees `next`.

**How to avoid:** Destructure `Directive` into `(delta, next)` immediately after `StateNode::run`/
`DirectiveParser::parse` returns, run the EXISTING `after` hook chain against `&mut delta` only
(unchanged from today), and carry `next` as a separate value through the rest of the per-node
result tuple and up into `run`'s per-superstep bookkeeping.

**Warning signs:** A compile error inside the `tokio::spawn` closure's `Ok(mut delta) =>` arm is
the first symptom; if instead the code is changed to make `after` take `&mut Directive`, that is an
ENG-FR-22 (`NodeInterceptor`) trait signature change requiring the "new trait method" X-10 rule —
avoid it.

### Pitfall 2: End's interaction with `StarvedNodeAtCompletion` needs exact wiring, not a blanket skip

**What goes wrong:** D-09 requires `StarvedNodeAtCompletion` (the 22.1 BUG-03 truthful-outcome
check, `superstep.rs:598-636` and the entry-vanguard-empty path at `superstep.rs:199-237`) to be
*suppressed* specifically for End-terminated runs, not disabled generally. If a naive
implementation short-circuits the whole check whenever ANY node emitted `End` this superstep, it
would also suppress the check for a run that happens to have End AND a genuinely-broken
scheduler-invariant violation on an unrelated node in the same superstep — exactly the silent-lie
regression BUG-02/BUG-03/BUG-04 exist to prevent.

**Why it happens:** The check currently runs unconditionally whenever `next_vanguard.is_empty()`
(the natural place End's "run completes after this superstep's merge" semantics would also make
`next_vanguard` empty, since nothing is scheduled next). The two conditions ("End fired this
superstep" and "Vanguard is empty") are correlated but not identical.

**How to avoid:** Gate the suppression on the SPECIFIC fact "at least one node in THIS superstep
emitted `NextStep::End`," not on the general emptiness of `next_vanguard` — track an
`end_requested: bool` (or the emitting `NodeId`, for the observability CONTEXT.md's D-09 also asks
for on the Waypoint's `completed` records) alongside `deltas`/`completed_records` in the same
per-superstep accumulation loop, and only skip the `starved_at_completion` call when it is `true`.
CONTEXT.md explicitly asks for a named regression test: "a test where End fires while another node
still holds an unconsumed fired edge" — write this as the proof the suppression is scoped correctly
and not a blanket bypass.

### Pitfall 3: Muster's "same superstep" execution must not violate ENG-FR-05 snapshot isolation

**What goes wrong:** CF-FR-10 says all of one Muster's tasks execute in the SAME superstep (N+1
relative to the planner's superstep N), concurrently, bounded by the engine parallelism limit — the
same superstep concurrency and snapshot-isolation rules (ENG-FR-05, "all nodes in one superstep
read the SAME pre-superstep Battlefield snapshot") that already govern the normal Vanguard. A worker
task must NOT observe another worker task's payload or partial output within the same muster.

**Why it happens:** The existing `snapshot = Arc::new(battlefield.clone())` (`superstep.rs:367`) is
built once per superstep and shared read-only across all spawned node tasks — this already gives
Muster workers the correct Battlefield isolation for free, AS LONG AS worker dispatch reuses the
SAME snapshot/spawn mechanism as ordinary Vanguard nodes, rather than a bespoke "run these N tasks"
loop that might construct its own (potentially stale or double-cloned) snapshot.

**How to avoid:** Model each `MusterTask` as producing its own synthetic vanguard entry for
superstep N+1 (dispatched through the SAME `execute_vanguard_node`/`tokio::spawn`/`Semaphore`
machinery, with `NodeContext.muster = Some(MusterContext { payload, task_key })` set per task) —
not a parallel execution path. This also gets the existing parallelism-limit semaphore
(`superstep.rs:369`, `limit = parallelism.unwrap_or(vanguard.len()).max(1)`) applied to worker tasks
automatically, satisfying "bounded by the engine parallelism limit" without new code.

### Pitfall 4: Mid-muster progress Waypoints must NOT merge into the Battlefield

**What goes wrong:** D-14 is explicit and CONTEXT.md's Specific Ideas section calls this out with
its own dedicated test requirement: "The mid-muster progress Waypoint (D-14) stores *unmerged*
deltas — never a partially merged Battlefield." The natural implementation temptation is to call
`battlefield.merge(...)` incrementally as each worker completes (since that's the existing
merge-and-checkpoint idiom for a normal superstep) — this would violate snapshot isolation for the
STILL-RUNNING sibling workers in the same muster (ENG-FR-05) and would make a mid-muster resume
non-idempotent (re-running would merge the same delta twice).

**How to avoid:** A progress Waypoint's `battlefield` field must be `battlefield.clone()` of the
superstep-START snapshot (unchanged), with the growing set of completed-task deltas carried
SEPARATELY in the new `muster_progress: Option<MusterProgress>` field, keyed by `task_key`, and only
merged into the real Battlefield ONCE — after every task in the muster resolves, exactly like the
existing end-of-superstep merge (`superstep.rs:551-578`) already does for ordinary Vanguard deltas.
Write the exact test CONTEXT.md specifies: assert the Battlefield on a progress Waypoint equals the
superstep's start snapshot.

### Pitfall 5: `EngineLimits.max_muster_tasks` must stay OUT of the fingerprint

**What goes wrong:** `WarGraph::fingerprint()`'s own rustdoc (`graph.rs:507-527`, confirmed current)
is explicit that `EngineLimits` is deliberately excluded ("raising a limit... to let a resumed run
continue is a legitimate operator action"). Since `max_muster_tasks` is a NEW field being added to
`EngineLimits` (D-16), it would be easy to accidentally include it in the `v3` fingerprint additions
alongside the genuinely new sections (worker-template set, child-graph fingerprints, DirectiveParser
kinds) — but `max_muster_tasks` is a LIMIT, not a scheduling-relevant graph shape, and must follow
the same exclusion as `max_supersteps`/`max_node_visits`/`run_timeout` already do.

**How to avoid:** The existing exclusion test (`fingerprint_is_unchanged_by_prompt_model_input_mapping_and_limits`,
`graph.rs:1165-1180`) already asserts this for the pre-existing three `EngineLimits` fields with a
`limits: EngineLimits { max_supersteps: 999, max_node_visits: 999, run_timeout: None }` variant —
extend this SAME test (not a new one) to also vary `max_muster_tasks`, proving the v3 fingerprint
stays unchanged.

### Pitfall 6: `checkpoint_ns` must stay independent of `WaypointPort::latest`'s per-thread contract

**What goes wrong:** D-20 explicitly states "No `WaypointPort` method change: `latest(child_thread)`
is the child's own latest namespaced Waypoint." Confirmed against the live `WaypointPort` trait
(read via PRD 01 §3.4, unchanged in the tree): `latest(&self, thread: &ThreadId)` already returns
"the latest waypoint for this thread" with no namespace parameter. This is only safe if
`ThreadId::child(parent, node_id)` produces a GENUINELY DISTINCT `ThreadId` per child (not the
parent's `ThreadId` with a side-channel `checkpoint_ns` tag on the SAME thread's Waypoints) — since
`latest` has no way to filter by namespace within one thread's Waypoint history.

**How to avoid:** `checkpoint_ns` (the additive `Waypoint` field, D-20) is a RECORD of the namespace
path for observability/debugging (e.g. reconstructing a nested-subgraph execution's structure from
a flat Waypoint history), NOT the mechanism that achieves isolation — isolation comes entirely from
`ThreadId::child` producing a distinct `ThreadId` that the child's own `superstep::run` call
addresses via its own `thread` parameter, resulting in an entirely separate row/history under the
SAME `WaypointPort` instance. Do not build any lookup path that tries to derive a child's Waypoints
from the parent thread's history filtered by `checkpoint_ns` — there is no such lookup in the
locked design.

### Pitfall 7: `LlmDecisionEvaluator`'s prompt template is a security boundary, not just a feature

**What goes wrong:** The evaluator renders its `prompt_template` from the Battlefield (engine path)
or the source node's output (legacy path) and sends the result to a live LLM via `Arc<dyn LlmPort>`.
`.github/instructions/security.instructions.md` (imported by `CLAUDE.md`) calls out exactly this
class of risk for a different subsystem ("Response bodies are redacted before truncation... No log
statement interpolates an API key"). For this phase, the analogous risk is: whatever the
`prompt_template`'s `InputMapping`-style placeholders resolve to is exactly what leaves the process
boundary to a third-party LLM API — if a workflow author's Battlefield schema happens to carry
secret-like data (an API key stashed in a `notes` field, say) and the `LlmDecision` template
references that field, it is sent to the model.

**How to avoid:** This is not a code defect to fix — it's a documentation/rustdoc obligation
(CONTEXT.md's canonical_refs section already names this exact boundary: "the prompt template is the
author-controlled boundary for what leaves the process (document it; no API key may reach logs or
errors)"). Ensure `LlmDecisionEvaluator`'s rustdoc states explicitly that the template is rendered
from live Battlefield state and is therefore an egress boundary the workflow author controls, and
that neither the evaluator's own error paths nor its trace/log output ever interpolate the
`Arc<dyn LlmPort>`'s response body or the rendered prompt in a way that could leak beyond what the
author already intended to send.

## Code Examples

### Async trait for the evaluator, matching the house `#[async_trait]` idiom

```rust
// Source: adapted from paladin-core::platform::container::waypoint's
// WaypointPort pattern (paladin-ports) and the existing StateNode trait
// (crates/paladin-battalion/src/engine/node.rs:33-38) -- both already use
// exactly this shape in this codebase.
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
    /// `Some` on the engine path, `None` on the legacy path (D-02).
    pub battlefield: Option<&'a Battlefield>,
}
```

### `EngineError`'s existing `#[non_exhaustive]` status makes new variants free

```rust
// Source: crates/paladin-battalion/src/engine/mod.rs:137-139 (confirmed current)
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    // ... existing variants ...
}
```

Since `EngineError` is ALREADY `#[non_exhaustive]` (unlike `BattalionError`, which is not), every
new variant this phase adds (`UnregisteredEdgeCondition`, `EdgeEvaluatorFailed`, `GotoUnknownNode`,
`ParleyNotSupported`, `UnschedulableCycle`'s siblings for muster limits, `RecursiveEmbedding`, etc.)
requires ZERO X-10 register burden beyond documenting it — no external crate can exhaustively match
`EngineError` today, so adding arms cannot break a downstream compile. This is a materially
different, lighter-weight situation than `BattalionError::InvalidGraph` (CF-01's legacy path),
which IS a pre-existing, non-`#[non_exhaustive]` public enum, confirmed at
`crates/paladin-core/src/platform/container/battalion/mod.rs:732-755` — this is exactly why D-04
reuses the existing `InvalidGraph(String)` variant rather than adding a new one.

### `WarGraph::validate`'s existing three-clause structure is the exact insertion pattern

```rust
// Source: crates/paladin-battalion/src/engine/graph.rs:290-329 (confirmed current)
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

**CF-01 adds a new parameter** (`evaluator_registry: &EdgeEvaluatorRegistry`) and a new clause
(fail-closed on unregistered `Custom` edge conditions) — following the SAME "collect every
offender, report them all at once" discipline `validate_eligible_set`/`validate_schedulable` already
establish (both return `Vec<NodeId>` of every offender, never fail-fast on the first). **CF-03/CF-04
each add one more clause** (worker-template well-formedness; recursion rejection), extending this
same function rather than introducing a parallel validation entry point. This confirms CONTEXT.md
D-03's framing is directly buildable: `WarGraph::validate` gains the evaluator registry as a
parameter "beside `CustomDispatchResolver`."

### Legacy path's `evaluate_edge_condition` needs to become `async`

```rust
// Source: crates/paladin-battalion/src/campaign_service.rs:377-399 (confirmed current, BUG-01 site)
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
            Ok(true)  // <-- BUG-01, line 396
        }
    }
}
```

This is called synchronously at `campaign_service.rs:308` inside `execute_internal`'s per-edge loop
(`self.evaluate_edge_condition(&edge_data.condition, &result.output)?`). Since `execute_internal`
is already an `async fn` (it awaits `self.paladin_port.execute(...)` two lines earlier at line 287),
converting this call to `self.evaluate_edge_condition(&edge_data.condition, &result.output).await?`
after making the method `async fn` is a purely mechanical change with no `Send`/`Sync` obstacle —
`CampaignExecutionService` already stores `paladin_port: Arc<dyn PaladinPort>` and would gain
`evaluators: EdgeEvaluatorRegistry` (or `Arc<EdgeEvaluatorRegistry>`) alongside it, following the
existing `herald: Option<Arc<dyn Herald>>` field precedent.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| Fingerprint `v1`: delimiter-joined canonical bytes (`\|`, `-`, `:`) | Fingerprint `v2`: length-prefixed `push_field` encoding, collision-free | Phase 22.1 CR-01 | `v3` (this phase) MUST use the SAME `push_field` helper for every new hashed section (worker templates, child-graph fingerprints, DirectiveParser kind/on_parse_error) — do not reintroduce a delimiter join for the new sections. |
| `Frontier` rebuilt from scratch on every `resume` | `Frontier` restored from a persisted `FrontierSnapshot` keyed by edge identity | Phase 22.1 BUG-04 / ENG-FR-12a | Goto's `NotFiring` marking and Muster's per-task progress both build directly on this restoration machinery — there is no separate "resume state" concept to invent. |
| `WarGraph::validate` rejected only structural graph errors | `WarGraph::validate` also runs `validate_eligible_set`/`validate_schedulable` (BUG-02/BUG-03 fixes), checked LAST, collecting every offender in one error | Phase 22 (BUG-02) / Phase 22.1 (BUG-03) | CF-01's evaluator-registry check and CF-03/CF-04's new clauses should follow this SAME "checked after the more-specific structural clauses, collect every offender" discipline, not fail-fast on the first offender or run before the existing clauses. |

**Deprecated/outdated:** Nothing in this phase deprecates an existing public API — every change is
additive (new enum variants on already-`#[non_exhaustive]` types, new builder methods, new
`#[serde(default)]` fields) except the ONE sanctioned behavioral break (BUG-01/M-B-01), which is a
semantics change to an existing variant's runtime behavior, not a signature change.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `execute_vanguard_node`'s exact refactored signature (illustrative code in Pattern 2) is one reasonable shape, not the only one — the actual widened return type (e.g. whether `NextStep` travels inside a new tuple element vs. a new struct) is Claude's discretion per CONTEXT.md and was not locked by this research. | Architecture Patterns §2 | Low — the illustrative code is explicitly marked as such; the actual implementation has latitude and CONTEXT.md does not lock this shape. |
| A2 | The recommended `goto_targets: Vec<NodeId>` threading mechanism (collected in the per-node result loop, unioned into `next_vanguard` after `compute_next_vanguard` returns) is ASSUMED to be workable without violating `compute_next_vanguard`'s existing four-tier "each tier engages only when every earlier tier is empty" discipline — this was reasoned from reading the function, not verified by writing and running code. | Architecture Patterns §3 | Medium — if Goto targets must interact with tier ordering (e.g. a Goto target that would ALSO be tier-1-ready via a static edge) in a way not obvious from static reading, the actual implementation may need a fifth tier or an earlier merge point than proposed here. Flagged for the planner to verify with a concrete test (a node that is both a Goto target this superstep AND has a static incoming edge that fired). |
| A3 | `BATTLEFIELD_SCHEMA_VERSION` should NOT bump for `checkpoint_ns`/`muster_progress` (based on the observed precedent that `visit_counts` and `frontier` did not bump it) — this is presented as a strong precedent-based recommendation, not a verified constraint; CONTEXT.md itself leaves it "Claude's discretion... following the visit_counts/frontier precedent." | Architecture Patterns §4 | Low — CONTEXT.md already frames this as discretionary; the research only strengthens the precedent-following recommendation with direct evidence from the two prior additions. |

**If this table is empty:** N/A — see entries above. All three are LOW-to-MEDIUM risk framing
choices flagged explicitly as such, not load-bearing factual claims about the codebase (every
factual claim about file contents, line numbers, and existing signatures in this document was read
directly from the tree this session).

## Open Questions

1. **Exact `Waypoint`/`Directive`/`NextStep` module boundary for `Directive`'s own routing carried
   through `run`'s internal bookkeeping.**
   - What we know: `Directive`/`NextStep`/`MusterTask` land in `paladin-core` (CONTEXT.md D-07,
     locked); the engine consumes them inside `superstep.rs`.
   - What's unclear: whether `run`'s internal per-superstep accumulation (today: `deltas`,
     `completed_records`) should grow a THIRD parallel `Vec` for routing directives, or whether a
     single richer per-node result struct replaces the current `(NodeId, StateDelta)`-style tuples
     throughout. This is an implementation-detail question that does not affect the public contract.
   - Recommendation: leave to the planner/implementer; not a decision that needs to be pre-settled
     in research, since it does not touch any locked public type.

2. **Whether the `Frontier`'s four-tier `compute_next_vanguard` needs a FIFTH tier for Goto, or
   whether Goto union happens entirely outside `compute_next_vanguard`.**
   - What we know: `compute_next_vanguard` today only consumes `Frontier` state; Goto targets are a
     runtime value from THIS superstep's Directives, not derivable from `Frontier` alone.
   - What's unclear: the cleanest integration point — inside `compute_next_vanguard` (requiring it
     to take a new parameter) vs. entirely in `run`'s loop body after `compute_next_vanguard`
     returns (as sketched in Pattern 3).
   - Recommendation: the `run`-loop-body approach is likely cleaner (keeps `compute_next_vanguard`
     pure over `Frontier` state, matching its current signature `fn(graph: &WarGraph, frontier:
     &Frontier) -> Vec<NodeId>`), but this should be verified against the interaction in A2 above
     before being locked into a plan task.

## Environment Availability

Skipped — this phase has no external service/tool/runtime dependencies beyond the already-present
Rust toolchain (MSRV 1.88, confirmed current via Phase 22.1), `cargo`, and the workspace's existing
crates. No new database, no new external API, no new CLI tool.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in), `#[tokio::test]` / `#[tokio::test(flavor = "multi_thread")]` for async and stress tests |
| Config file | none — no separate test-framework config; workspace `Cargo.toml` + per-crate `[dev-dependencies]` |
| Quick run command | `cargo test -p paladin-battalion --lib` (engine unit tests only, seconds) |
| Full suite command | `cargo test` (workspace) plus `cargo test --test e2e_crash_resume_test` / the new E2E-3 integration test target; `make test-integration-docker` for the Postgres contract-suite additions |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|--------------|
| CF-FR-01/02/04 | Unregistered `Custom` rejected at validation (both paths); registered true/false-routes; runtime evaluator error fails run | unit | `cargo test -p paladin-battalion --lib campaign_service::tests`, `cargo test -p paladin-battalion --lib engine::graph::tests`, `engine::superstep::tests` | ❌ Wave 1 — RED tests committed before the fix per D-05 |
| CF-FR-03 | Evaluator `Err` at runtime names edge + evaluator, does not default a branch | unit | `cargo test -p paladin-battalion --lib` (new test in `superstep.rs` and `campaign_service.rs`) | ❌ Wave 1 |
| CF-FR-05/07/08 | `Directive`/`NextStep`; Goto validated + `max_node_visits`-bounded; End-over-Goto precedence | unit | `cargo test -p paladin-battalion --lib engine::superstep::tests` (Function-node fixtures, no LLM) | ❌ Wave 2 |
| CF-FR-06 | `DirectiveParser`: `PlainOutput` passthrough, `StructuredDirective` happy path + both `on_parse_error` modes | unit | `cargo test -p paladin-battalion --lib` (new `directive_parser` test module, uses `RecordingPaladinPort::set_output`) | ❌ Wave 2 |
| CF-FR-09/10/11 | Payload isolation, `task_key` ordering (≥20 iterations), duplicate-key rejection, defer-join | unit + determinism repeat | `cargo test -p paladin-battalion --lib engine::superstep::tests` (extend the existing Phase 22 D-11 seeded-shuffle harness) | ❌ Wave 3 |
| CF-FR-12/13 | Mid-muster resume (2/5 done → exactly 3 re-run); `max_muster_tasks` breach | unit + integration | `cargo test -p paladin-battalion --lib`, `cargo test --test e2e_muster_defer_order_test` (new) | ❌ Wave 3 |
| CF-FR-14/15/16 | `StateMap` in/out mapping, private child fields, namespaced Waypoints, resume-mid-child, recursion rejection, `restart_on_resume` | unit + integration | `cargo test -p paladin-battalion --lib engine::graph::tests`, new integration test (Formation-inside-Campaign, kill-after-child-superstep-1 → resume) | ❌ Wave 4 |
| CF-FR-17 | Formation-inside-Campaign integration test, kill-after-child-superstep-1 → resume → child work not repeated | integration | `cargo test --test <new bridge-embedding test>` | ❌ Wave 4 |
| CF-FR-18/19 | `LlmDecision` choice match + ambiguity modes; Commander semantic fallback; existing Commander tests unmodified | unit | `cargo test -p paladin-battalion --lib commander::tests`, new evaluator tests using `MockLlmAdapter` | ❌ Wave 5 |
| PRD 02 Test Plan item 8 | 50-task muster stress test, exact counts, timeout guard | stress | `cargo test -p paladin-battalion --lib -- --ignored` or a dedicated `#[tokio::test(flavor = "multi_thread")]` in `engine::superstep::tests`, following `listener.rs`'s X-05 house pattern | ❌ Wave 6 |
| Acceptance criterion 2 (E2E-3 muster/defer/order half) | Planner musters 5, deferred aggregator runs once, ordered results, manually-succeeding mock | integration | `cargo test --test e2e_muster_defer_order_test` (new, modeled on `e2e_crash_resume_test.rs`'s structure) | ❌ Wave 3/6 |

### Sampling Rate

- **Per task commit:** `cargo test -p paladin-battalion --lib` (fast — no Postgres/Docker needed for
  any CF-01…CF-05 unit-level work; all Tier 1 per D-30).
- **Per wave merge:** `cargo test` (workspace) + `cargo fmt --check` + `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` (the pre-commit hook's own command, per the GSD run
  mechanics memory note — matches, no drift needed) + the new E2E integration test target(s).
- **Phase gate:** Full suite green, PLUS `make test-integration-docker` for the SQLite/Postgres
  contract-suite additions (`muster_progress`/`checkpoint_ns` round-trip cases, D-14/D-20/D-30),
  PLUS `cargo semver-checks` (vs 0.9.0) and the `msrv` (1.88) CI job green on the phase's final
  commit, before `/gsd-verify-work`.

### Wave 0 Gaps

- No dedicated new test file/framework install needed — `cargo test`, `tokio::test`, the existing
  `test_support.rs` fixtures (`CountingFunctionNode`, `RecordingPaladinPort`,
  `RecordingWaypointStore`), and `paladin-llm`'s `MockLlmAdapter` (once the new dev-dependency line
  is added per Package Legitimacy Audit) already cover every test type this phase needs.
- One infrastructure gap: `crates/paladin-battalion/Cargo.toml` needs `paladin-llm` added to
  `[dev-dependencies]` before any CF-05 test referencing `MockLlmAdapter` can compile — this is a
  one-line manifest edit, not a framework install, but it IS a prerequisite the first CF-05 plan
  task must do before writing its RED tests.
- `tests/integration/e2e_muster_defer_order_test.rs` and the Formation-inside-Campaign subgraph
  test file do not exist yet — both are new files (`Wave 0` in the sense of "created fresh," not "a
  gap in existing infrastructure"; the pattern to follow, `e2e_crash_resume_test.rs`, already
  exists and was read in full this session).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | No | This phase adds no HTTP surface, no auth mechanism; HTTP exposure of subgraph/thread state is explicitly out of scope (Doc 06/Phase 27). |
| V3 Session Management | No | No session concept touched. |
| V4 Access Control | No | No new access-control boundary; the evaluator registry and `EngineConfig` are code-configured (D-26), not exposed to untrusted callers this phase. |
| V5 Input Validation | Yes | `StructuredDirective`'s JSON envelope parsing (CF-FR-06) is untrusted-shaped input (an LLM's own output, parsed as a routing directive) — the existing `on_parse_error: FailRun \| FallbackPlain` typed-error contract, applied via `serde_json`'s existing typed deserialization (no hand-rolled parser), is the standard control. Goto target validation (`EngineError::GotoUnknownNode`) is likewise input validation over a runtime-produced value before it is acted on. |
| V6 Cryptography | No | No new cryptographic operation; `blake3` fingerprinting is unchanged in algorithm (only new hashed sections), not a security boundary against an adversary (it is a resume-integrity check against accidental graph drift, not a tamper-resistance mechanism). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Prompt/data exfiltration via `LlmDecisionEvaluator`'s Battlefield-rendered template (CF-05) | Information Disclosure | Document the egress boundary in rustdoc (Common Pitfalls §7); this is a workflow-author responsibility, not a code-level filter — mirrors the existing `.github/instructions/security.instructions.md` guidance for other LLM-adjacent code in this repo (redact before truncation, never log a credential). No new mitigation code is warranted; the existing house convention (never interpolate a response body or API key into logs/errors) already covers this evaluator's error paths if followed. |
| Delimiter-collision in a new identity-encoding scheme (`ThreadId::child`, CF-04) | Tampering (a crafted `NodeId` collides two distinct child threads) | Length-prefixed or percent-escaped encoding (mirroring `push_field`, `graph.rs:629-639`), never a bare delimiter join — CR-01 (22.1-REVIEW.md) is a DIRECT precedent of exactly this defect class already found and fixed once in this codebase's fingerprint encoding; the same review-level scrutiny should be applied to `ThreadId::child`'s encoding at implementation and code-review time. |
| Unbounded muster fan-out (resource exhaustion via a malicious/buggy planner Directive) | Denial of Service | `EngineLimits.max_muster_tasks` (default 100, D-16), enforced BEFORE any task starts (D-13) — already a locked decision; this is the standard control and is already specified. |
| Recursive subgraph embedding (unbounded nesting / stack exhaustion at validation or execution time) | Denial of Service | `EngineError::RecursiveEmbedding` validation-time rejection via a path-set walk over child fingerprints (D-19, already locked) — validated BEFORE any node executes, consistent with the house "fail loud at validation, never at runtime" discipline this whole phase's BUG-01 fix also embodies. |

## Sources

### Primary (HIGH confidence — read directly from the live tree this session)
- `crates/paladin-battalion/src/engine/graph.rs` (1732 lines, read in full through line 1293 plus
  targeted grep for the remainder) — `WarGraph`, `NodeSpec`, `EdgeSpec`, `EngineLimits`, `validate`,
  `validate_eligible_set`, `unschedulable_unfed_nodes`/`validate_schedulable`, `fingerprint`,
  `push_field`, and the full golden-fingerprint/BUG-02 test suite.
- `crates/paladin-battalion/src/engine/superstep.rs` (3029 lines; read lines 1-1180 in full, plus
  targeted grep for the remainder) — `NodeDispatch`, `execute_vanguard_node`, the full `run` loop
  (empty-vanguard path, cancellation, limits, per-node dispatch/interceptor chain, merge, frontier
  update, run-end truthful-outcome check), `EdgeState`, `Frontier` (all methods), `NodeEdgeSummary`,
  `compute_next_vanguard`, `starved_nodes`/`starved_release`/`starved_deferred_release`/
  `starved_at_completion`, `evaluate_edge_condition` (BUG-01 site), `build_waypoint`.
- `crates/paladin-battalion/src/engine/mod.rs` (2642 lines; read lines 1-640 in full plus targeted
  grep) — `WaypointDurability`, `RunOutcome`, `EngineError` (full enum, confirmed `#[non_exhaustive]`),
  `ResumeOptions`, `WarEngine` (full struct + all builder methods + `start`/`resume`/
  `resume_with_options`).
- `crates/paladin-battalion/src/engine/node.rs` (38 lines, read in full) — `NodeError`,
  `NodeContext`, `StateNode`.
- `crates/paladin-battalion/src/engine/dispatch_registry.rs` (107 lines, read in full) —
  `DispatchRegistry`, the exact house pattern for `EdgeEvaluatorRegistry`.
- `crates/paladin-battalion/src/engine/input_mapping.rs` (299 lines, read in full) —
  `InputMapping::render`, `FieldName` resolution, error contract.
- `crates/paladin-battalion/src/campaign_service.rs` (618 lines, read in full) — the legacy BUG-01
  site, `execute`/`execute_internal`, `evaluate_edge_condition`, `with_herald` (the additive-builder
  precedent).
- `crates/paladin-battalion/src/commander.rs` (targeted grep + read lines 1230-1420) — `Commander`,
  `CommanderBuilder`, `analyze_and_select` (Heuristic path), builder pattern for the private-field
  addition precedent.
- `crates/paladin-core/src/platform/container/waypoint.rs` (785 lines; read lines 1-460 in full) —
  `ThreadId` (charset/length validation), `NodeId` (no charset restriction), `GraphFingerprint`,
  `FrontierEdgeState`/`FrontierSnapshot`, `NodeOutcomeKind`, `NodeExecutionRecord`, `ParleyRequest`,
  `WaypointStatus`, `Waypoint` (full struct incl. `visit_counts`/`frontier` additive-field
  precedent), `Waypoint::new_root`/`new_child`.
- `crates/paladin-core/src/platform/container/battlefield.rs` (targeted read lines 1-65 plus grep) —
  `FieldName` (no charset restriction beyond non-empty), `StateDelta`, `CustomDispatchFn`.
- `crates/paladin-core/src/platform/container/battalion/campaign.rs` / `mod.rs` (targeted grep) —
  `EdgeCondition` (not `#[non_exhaustive]`), `BattalionError` (not `#[non_exhaustive]`,
  `InvalidGraph(String)` variant).
- `crates/paladin-battalion/src/engine/bridges.rs` (targeted grep) — `from_formation`/`from_phalanx`/
  `from_campaign`, `CAMPAIGN_FAN_IN_SEPARATOR`, `dedicated_output_field`.
- `crates/paladin-battalion/src/engine/test_support.rs` (targeted grep) — `CountingFunctionNode`,
  `RecordingPaladinPort`, `RecordingWaypointStore`.
- `crates/paladin-llm/src/mock.rs` and `crates/paladin-ports/src/output/llm_port.rs` (targeted grep)
  — `MockLlmAdapter`, `LlmPort` trait location and signature.
- `crates/paladin-battalion/Cargo.toml`, `crates/paladin-llm/Cargo.toml` (read in full) — confirmed
  `paladin-llm` is not currently a `paladin-battalion` dependency; confirmed acyclic.
- `src/config/citadel.rs` (read lines 1-90), `src/config/waypoint_retention.rs` (read lines 1-60) —
  the exact X-09 config-struct shape `EngineConfig` must mirror.
- `tests/integration/e2e_crash_resume_test.rs` (read lines 1-60) — E2E-1's structure, the pattern
  E2E-3's new integration test follows.
- `.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` (PRD 02, read in full) — the
  FR-level source of truth, CF-FR-01…19, acceptance criteria, TDD test-plan ordering.
- `.project/v0.10.0/00-program-overview.md` (read in full) — X-01…X-11, ubiquitous language,
  E2E-1/2/3, BUG-01…04 register, `MIGRATION.md` §9 required structure.
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` (PRD 01, read in full) —
  ENG-FR-01…23, the engine contract this phase extends.
- `MIGRATION.md` (read in full) — §9.1 M-B-01…04, §9.2 register (confirming CF-01/CF-05 rows already
  pre-populated as "TBD — Phase 23"), §9.3 MSRV 1.88 chain, §9.5 `EngineConfig` "not yet in the
  tree" claim.
- `.planning/phases/23-control-flow-dynamic-routing-fan-out-subgraphs/23-CONTEXT.md` (read in full)
  — D-01…D-30, canonical refs, code context, specifics, deferred ideas.
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-CONTEXT.md` (read in full)
  — D-01…D-25, the BUG-03/BUG-04/CR-01 fix decisions this phase builds directly on top of.
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-REVIEW.md` (read in full)
  — CR-01's exact collision mechanism and fix, WR-01 (`.expect()` in spawned task, now fixed).
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md` (partial — pages 1-571 of 862; sufficient for
  phase context) — CF-01…05 requirement text, project decision history.

### Secondary (MEDIUM confidence)
- None — no web/external documentation was consulted for this phase; the phase is entirely
  code-grounded per the phase description's explicit instruction ("this is primarily code-grounded,
  not web-grounded").

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new external dependencies; every library used is already resolved in
  the workspace lockfile and read directly from `Cargo.toml` files.
- Architecture: HIGH — every insertion point (BUG-01's two sites, `StateNode::run`'s one call site,
  `Frontier`'s Goto/Muster integration points, the `Waypoint` additive-field pattern, the
  `EngineConfig` shape) was confirmed by reading the actual source, not inferred from CONTEXT.md
  alone.
- Pitfalls: HIGH — all seven pitfalls are grounded in specific, cited line numbers and either an
  existing test that must be extended (Pitfall 5) or an existing defect class already fixed once in
  this exact codebase (Pitfalls 6/CR-01 precedent).

**Research date:** 2026-09-03
**Valid until:** 30 days (stable, internal-codebase-grounded research; not time-sensitive to
external library churn since no external library is being newly adopted).

---

*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Research completed: 2026-09-03*
