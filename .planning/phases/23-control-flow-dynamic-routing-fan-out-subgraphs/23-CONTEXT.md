# Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs - Context

**Gathered:** 2026-09-03
**Status:** Ready for planning
**Mode:** `--auto` (all gray areas auto-selected on recommended defaults; audit trail in 23-DISCUSSION-LOG.md)

<domain>
## Phase Boundary

Phase 23 delivers epic `CF` (PRD 02) on top of the Phase 22/22.1 engine, and nothing from later
epics beyond the seams PRD 02 itself declares:

1. **BUG-01 fixed fail-closed, test-first (CF-01).** A registered-`EdgeConditionEvaluator`
   mechanism on both the legacy `CampaignExecutionService` and the `WarEngine`; validation fails
   before any node executes, naming every unregistered `EdgeCondition::Custom(name)`; the two
   warn-and-return-true placeholders (`campaign_service.rs:392-397`, `engine/superstep.rs:1206`)
   are removed with no restoring configuration; runtime evaluator errors fail the run. This is the
   program's single sanctioned behavioral break (`MIGRATION.md` M-B-01), whose worked before/after
   example is owed by this phase.
2. **Directive node-driven routing (CF-02).** `Directive { delta, next: NextStep }` with
   `NextStep::{Edges, Goto, Muster, End, Parley}` in `paladin-core`; `StateNode::run` returns it;
   validated `Goto`, tested End-over-Goto precedence; per-node `DirectiveParser` for Paladin nodes
   (`PlainOutput` default, `StructuredDirective` with `on_parse_error`).
3. **Muster dynamic fan-out (CF-03).** Runtime-N worker tasks from a planner's Directive, executed
   concurrently in one superstep with payload isolation, deterministic `task_key`-ordered
   aggregation, duplicate-key rejection, `EngineLimits.max_muster_tasks`, and mid-muster resume
   that re-runs only unfinished tasks. The muster/defer/order half of program scenario E2E-3 passes
   now with a manually-succeeding mock; the Aegis retry half is Phase 25's.
4. **Subgraph composition (CF-04).** `NodeSpec::Battalion` embeds a child `WarGraph` with
   `StateMap` in/out mapping and private child fields; namespaced checkpoint inheritance with
   resume-mid-child; `restart_on_resume` opt-out; recursive embedding rejected at validation; the
   Phase 22 `from_formation`/`from_phalanx`/`from_campaign` graphs embed unchanged
   (Formation-inside-Campaign integration test).
5. **LLM-evaluated routing, off by default (CF-05).** An `LlmDecision` edge evaluator (choice
   matching, `on_ambiguous`) and Commander `StrategySelection::Semantic`, both falling back
   deterministically on LLM failure with the fallback recorded; existing Commander tests pass
   unmodified.

**Out of this phase** (later phases own them): Parley suspension/resume-with-payload semantics
(Phase 24 — only the `NextStep::Parley` variant is declared here); per-task retry, timeouts and
typed error handlers (Phase 25); native provider JSON mode for `StructuredDirective` (Phase 26,
RT-05); HTTP exposure of threads or subgraph state (Phases 24/27); trace consumers (Phase 28).
Any other behavioral change discovered mid-implementation is an X-03 stop-and-flag event.

</domain>

<decisions>
## Implementation Decisions

PRD 02 is the FR-level source of truth and already locks the type shapes (`Directive`, `NextStep`,
`MusterTask`, `StateMap`), defaults (`max_muster_tasks` 100, `on_parse_error: FailRun`,
`StrategySelection::Heuristic`), FR semantics and the §4 TDD ordering. The decisions below settle
only what PRD 02 left open or what the shipped Phase 22/22.1 tree makes concrete. Do not re-litigate
anything PRD 02, PRD 01, overview §3 (X-01…X-11), or Phase 22/22.1's CONTEXT decisions state.

### BUG-01 fail-closed mechanism & error shapes (CF-01)
- **D-01: Trait, context and registry live in `paladin-battalion`.** `EdgeConditionEvaluator`,
  `EdgeContext` and an `EdgeEvaluatorRegistry` go in a new battalion module shared by
  `campaign_service.rs` and `engine/`, mirroring the `CustomDispatchResolver` house pattern in
  `crates/paladin-battalion/src/engine/dispatch_registry.rs`. `paladin-core`'s `EdgeCondition`
  is untouched (its §9.2 row already says "no variant change"). Rationale: both consumers are in
  battalion; battalion already holds port trait objects (`Arc<dyn PaladinPort>`), so the
  `LlmDecision` evaluator (D-25) can live beside the trait without core learning about LLMs.
  — **Reversibility:** costly — moving a public trait between published crates later touches
  facade re-exports and downstream imports.
- **D-02: The evaluator trait is async.** `#[async_trait] async fn evaluate(&self, output: &str,
  ctx: &EdgeContext<'_>) -> Result<bool, EdgeEvaluatorError>` — a deliberate deviation from PRD
  02's sync sketch, because CF-FR-18's `LlmDecision` must `await` an `LlmPort` call and blocking a
  Tokio worker is a house anti-pattern (`.planning/codebase/ARCHITECTURE.md`). The trait is new
  in v0.10, so X-10 does not apply. `EdgeContext` carries `source`/`target` `NodeId`s and
  `battlefield: Option<&Battlefield>` (`Some` on the engine path, `None` on the legacy path).
  `output` is the source Paladin's output string on the legacy path; on the engine path it is
  the source node's `output_field` value when the source is a Paladin node, else the canonical
  Battlefield JSON the existing engine `Contains`/`Regex` evaluation already renders. The
  evaluator error is typed and converted at each boundary: legacy → `BattalionError`, engine → a
  new structured `EngineError::EdgeEvaluatorFailed { from, to, evaluator, source }` (X-06).
- **D-03: Registry placement mirrors `with_dispatch_rule`.** `WarEngine::with_edge_evaluator(name,
  Arc<dyn EdgeConditionEvaluator>)` holds the engine registry; `WarGraph::validate` gains the
  registry as a parameter beside `CustomDispatchResolver` so the fail-closed check runs in the same
  pre-execution validate call (pre-release signature, free to change).
  `CampaignExecutionService::with_evaluator(name, evaluator)` is the additive builder method PRD
  CF-FR-02 requires; `CampaignExecutionService::new(paladin_port)` keeps its signature. The
  legacy check runs inside `execute` immediately after `campaign.validate()`
  (`campaign_service.rs:174`), before any node executes.
- **D-04: Error shapes — reuse the variant the PRD names, structure the new one.** The legacy path
  fails with the **existing** `BattalionError::InvalidGraph(String)` (PRD CF-FR-02 and the
  already-published M-B-01 row both name `InvalidGraph`; `BattalionError` is a pre-existing public
  enum without `#[non_exhaustive]`, so adding a variant or changing `InvalidGraph`'s payload would
  be an X-10 break outside the sanctioned list). The message lists every unregistered name,
  sorted, deterministic; X-06's "no new call sites on String variants" is consciously overridden
  here by the explicit FR. The engine path gets a new structured
  `EngineError::UnregisteredEdgeCondition { names: Vec<String> }` (sorted) on the
  `#[non_exhaustive]` `EngineError`. A registered evaluator returning `Err` at runtime fails the
  run with D-02's typed error naming the edge and evaluator — never a default branch (CF-FR-03).
- **D-05: Both placeholder sites die in one fix commit, RED tests committed first.** CF-FR-04's four
  cases (unregistered rejected at validation; registered true-routes; registered false-routes;
  evaluator error fails the run) are written for **both** paths and committed failing in a
  separate commit before the fix commit, so the traceability protocol (step 4) and SHIP-03's
  history check can see failing-then-passing order. Afterwards `grep -rn "defaulting to true"`
  and the engine's `Ok(true)` `Custom` arm must be absent from the tree.
- **D-06: M-B-01's worked example lands now.** `MIGRATION.md` §9.1 gets the before/after diff (a
  v0.9 `EdgeCondition::Custom("is_urgent")` campaign that silently always routed → the same
  campaign with `.with_evaluator("is_urgent", Arc::new(...))`, plus the `Contains`/`Regex`/
  `Always` replacement route), and §9.2's `EdgeCondition` and `CampaignExecutionService` rows are
  resolved (see D-29).

### Directive semantics at the engine boundary (CF-02)
- **D-07: `StateNode::run` changes return type directly.** `Directive`, `NextStep` and
  `MusterTask` land in `paladin-core` (no new core deps — `serde_json::Value` is already a core
  dependency) with `impl From<StateDelta> for Directive` (`next: Edges`). `StateNode::run`
  becomes `Result<Directive, NodeError>`; every in-tree Function node and test-support node adopts
  `.into()`. `StateNode`, `NodeSpec`, `NodeContext`, `EngineLimits`, `EngineError`, `WarGraph`
  and `Waypoint` are all absent at the `v0.9.0` tag, so none of this phase's changes to them needs
  a §9.2 register row (BUG-02/03/04 pre-release classification; record a "deliberate zero" note
  as Plan 22-01 did). — **Reversibility:** costly — every later epic's nodes (Gate, Aegis
  handlers, middleware) implement this trait.
- **D-08: Goto semantics.** (a) Targets are validated when the Directive is received:
  `EngineError::GotoUnknownNode { from, to }` (PRD). (b) A Goto target enters the next Vanguard
  directly, bypassing `Frontier::is_ready`, subject to `max_node_visits` (loops via Goto legal
  and bounded). (c) The emitting node's static outgoing edges are resolved **`NotFiring`** for
  that superstep so downstream joins and dead-propagation behave per ENG-FR-06 and no BUG-03-style
  starvation is introduced by a Goto. (d) A node reachable only via Goto must be declared with the
  **existing** `WarGraph::mark_dynamic_target` (BUG-02's eligible-set rule; its error text already
  says so) — no new mechanism. — **Reversibility:** costly — Phases 24/25 build on this routing
  contract.
- **D-09: End semantics and the truthful-outcome check.** `End` completes the run after the
  emitting superstep's merge (peers still merge; End beats Goto in the same superstep — PRD, tested).
  The run-end `StarvedNodeAtCompletion` check (22.1 D-04) is **suppressed** for End-terminated
  runs: End is explicit, node-authored termination, not a scheduler lie. Document this in the
  check's rustdoc and cover it with a test where End fires while another node still holds an
  unconsumed fired edge. Which node ended the run should be observable from the Waypoint's
  `completed` records (Claude's discretion on the exact `NodeOutcomeKind`/field).
- **D-10: `NextStep::Parley` returned this phase is a typed failure.** The engine returns
  `EngineError::ParleyNotSupported { node }` (fail loudly); it is never silently treated as
  `Edges` and no `AwaitingInput` Waypoint is written here. Phase 24 replaces this arm with real
  suspension.
- **D-11: `DirectiveParser` is per-node, `StructuredDirective` is delta-only.** `NodeSpec::Paladin`
  gains a `directive_parser: DirectiveParser` field defaulting to `PlainOutput` (a constructor
  keeps in-tree literals compiling — Claude's discretion on its shape).
  `DirectiveParser::{PlainOutput, StructuredDirective { on_parse_error: OnParseError }}`,
  `OnParseError::{FailRun, FallbackPlain}` default `FailRun` (PRD). JSON extraction rule: the
  trimmed whole output if it parses as a JSON object, else the first ```json fenced block, else
  `on_parse_error`. Envelope: `{"delta": {"<field>": <json>, …}, "next": "edges" | {"goto": […]}
  | "end" | {"muster": […]}}`; `delta` is applied through the schema's dispatch rules (an unknown
  field is a schema error → run fails). `StructuredDirective` writes **only** the envelope's
  `delta` — no implicit `output_field` write — and `output_field` is the target `FallbackPlain`
  falls back to. The parser is scheduling-relevant and is hashed into the fingerprint (D-19).

### Muster execution & mid-muster persistence (CF-03)
- **D-12: Worker templates are a graph-level marker set.** `WarGraph::add_worker_template(id,
  spec)` populates `worker_templates: HashSet<NodeId>`, mirroring `add_deferred_node`/
  `defer_flags` (PRD's "`NodeSpec` gains `worker_template: bool`" is satisfied in substance; the
  house pattern keeps every `NodeSpec` variant unchanged and the marker hashable like
  `defer_flags`). Validation: a worker template may not be an entry, is exempt from the
  eligible-set "unreachable" rejection (like a dynamic target), may have static **outgoing**
  edges (e.g. to an aggregator) but **no static incoming edges** — it runs only when mustered.
- **D-13: Timing, limits and ordering.** The planner returns `Muster(tasks)` in superstep N; all
  tasks execute concurrently in superstep N+1 under the engine parallelism limit (PRD's "same
  superstep" = the tasks share one superstep). Duplicate `task_key` and a `max_muster_tasks`
  breach are detected when the Directive is received, **before any task starts**, with typed
  errors naming the mustering node and the offending key or requested count vs limit. Worker
  deltas merge in lexicographic (`String` byte order) `task_key` order regardless of completion
  order, repeat-tested ≥ 20 iterations with the Phase 22 D-11 seeded-shuffle harness.
- **D-14: Mid-muster resume via intra-superstep progress Waypoints.** As tasks complete, the engine
  persists a Waypoint at the **same superstep index** with `status: Running` and an additive
  `#[serde(default)] muster_progress: Option<MusterProgress>` carrying the muster spec (mustering
  node, ordered tasks with payloads) and the completed tasks' **unmerged** deltas keyed by
  `task_key` — the Battlefield stays as it was at superstep start, preserving snapshot isolation.
  Resume from such a Waypoint re-enters the muster, runs only unfinished tasks, and merges all
  deltas in `task_key` order, so the final Battlefield equals the uninterrupted run's. Cadence: one
  progress Waypoint per completed task (bounded by `max_muster_tasks`), honoring the configured
  `WaypointDurability`. PRD 01's ENG-FR-11 is **clarified**, not changed: exactly one
  *superstep-complete* Waypoint per superstep; a Muster may additionally write progress Waypoints
  inside its superstep (E2E-1 has no muster and is unaffected). Record the clarification in PRD 01
  as an ENG-FR-11 note with a cross-reference in `08-traceability-matrix.md`, the same `.project/`
  edit precedent 22.1 used for ENG-FR-06a/12a. The three-backend contract suite gains a
  round-trip case for the new field (22.1 D-23 pattern). Rejected: superstep-end-only
  checkpointing (violates CF-FR-12 and test-plan item 5 "2/5 done → exactly 3 re-run"); one task
  per superstep (violates CF-FR-10's concurrency). — **Reversibility:** one-way after v0.10.0
  ships (the Waypoint payload is a stored contract); cheap now because no Waypoint exists outside
  this branch.
- **D-15: Payload isolation and the `muster.` namespace.** `NodeContext` gains
  `muster: Option<MusterContext { payload, task_key }>` with `muster_payload()`/`task_key()`
  accessors; the payload never enters the Battlefield. Worker Paladin `InputMapping` templates may
  reference `{muster.payload}` and `{muster.task_key}`: `InputMapping::render` receives the
  optional muster context and resolves the `muster.` prefix from it, never from the Battlefield;
  graph validation rejects a schema field whose name starts with `muster.` so the namespace is
  unambiguous. Exact plumbing is Claude's discretion; the two constraints are locked.
- **D-16: `max_muster_tasks` brings `EngineConfig` into the tree (X-09).** `EngineLimits` gains
  `max_muster_tasks: u32` (default 100, PRD). X-09 requires a config struct for a new tunable, and
  `MIGRATION.md` §9.5 still says `EngineConfig` at `src/config/engine.rs` is "planned, not yet in
  the tree" — it never landed in Phase 22. This phase creates it (`Default`, `validate()`,
  `EnvOverridable`, `APP_ENGINE_*`, mirroring `src/config/citadel.rs` and
  `waypoint_retention.rs`) carrying the four documented fields plus `max_muster_tasks`
  (`APP_ENGINE_MAX_MUSTER_TASKS`) and a conversion into `EngineLimits`/`WaypointDurability`;
  §9.5 is updated to "landed". Framing: X-09 compliance for CF-FR-13 requires the struct; covering
  the Phase 22 fields is incidental and cheap.
- **D-17: Deferred aggregation and the E2E-3 half.** A `defer: true` node downstream of a worker
  template runs exactly once, only after every mustered task has resolved (CF-FR-12); the worker
  template's static outgoing edge fires once per task. `tests/integration/` gains the E2E-3
  scenario with the muster/defer/order half asserted now (planner musters 5, deferred aggregator
  runs once, list-dispatch field holds exactly 5 results in deterministic order) using a
  manually-succeeding-on-attempt-N mock, with a clearly marked seam where Phase 25 adds the Aegis
  retry half. The X-05 stress test is the PRD's 50-task muster on `flavor = "multi_thread"` with
  exact counts and a timeout guard.

### Fingerprint coverage for the new graph properties
- **D-18: `GRAPH_FINGERPRINT_VERSION` bumps `v2` → `v3`.** New hashed sections, each sorted and
  length-prefixed under the 22.1 CR-01 encoding: the worker-template set; per `Battalion` node
  the child graph's fingerprint, its `StateMap` and `restart_on_resume`; per Paladin node its
  `DirectiveParser` kind and `on_parse_error`. Still excluded per ENG-FR-14: prompts, models,
  `InputMapping` templates, and every `EngineLimits` field including `max_muster_tasks`. The
  golden-hex test is re-pinned and one difference test per new property is added (22.1 D-17
  pattern). Rejected: keeping `v2` by emitting new sections only when non-empty (fragile, and the
  version tag exists precisely so a layout change is a detectable bump). Pre-release, so free
  (22.1 D-15 reasoning). — **Reversibility:** one-way after v0.10.0 ships — every stored
  fingerprint mismatches on a later layout change.

### Subgraph identity & checkpoint namespacing (CF-04)
- **D-19: `NodeSpec::Battalion { graph: Arc<WarGraph>, state_map: StateMap, restart_on_resume:
  bool }`** on the already-`#[non_exhaustive]` enum (its rustdoc at `graph.rs:31` announces this
  variant). `StateMap { inputs: Vec<(FieldName, FieldName)>, outputs: Vec<(FieldName,
  FieldName)> }` (parent, child) / (child, parent). Parent `validate` checks every mapped parent
  field exists in the parent schema and every mapped child field in the child schema, validates
  the child graph recursively with the **same** dispatch resolver and evaluator registry, and
  rejects recursive embedding with `EngineError::RecursiveEmbedding { path }` via a path-set walk
  over child fingerprints (immutable `Arc<WarGraph>` cannot literally self-contain; the check is
  defensive against structurally identical graphs and bounds nesting depth by construction).
  Unmapped child fields stay private; mapped outputs return as the node's delta under parent
  dispatch rules.
- **D-20: Child checkpoints run under a derived, injective child `ThreadId`.** `ThreadId::child(
  parent, node_id)` (name at Claude's discretion) builds the child thread id from the parent
  thread id and the Battalion node id with an **unambiguous** encoding — length-prefixed or
  escaped segments, never a bare delimiter join (22.1 CR-01's lesson: `NodeId` and `ThreadId`
  accept any non-empty string). The Waypoint gains an additive `#[serde(default)] checkpoint_ns:
  Option<String>` recording the namespace path (`"parent_node_id/"`, nested paths concatenate).
  **No `WaypointPort` method change**: `latest(child_thread)` is the child's own latest namespaced
  Waypoint, so resume of a parent mid-child resumes the child there with zero re-execution;
  `restart_on_resume: true` opts out (fresh child run — whether the old child thread is abandoned
  or overwritten is Claude's discretion; document the trade-off in rustdoc). Retention treats
  child threads as ordinary threads (their latest/AwaitingInput Waypoints are protected
  independently); no change to `WaypointRetentionService`. The contract suite gains a round-trip
  case for `checkpoint_ns`. — **Reversibility:** one-way after v0.10.0 ships (stored thread ids
  and Waypoint payload are contracts).
- **D-21: The child run inherits the parent engine wholesale.** `PaladinPort`, `WaypointPort`,
  durability, parallelism, dispatch resolver, evaluator registry, trace sink, interceptors and
  cancellation token all flow to the child; the child uses its own graph's `EngineLimits`; one
  parent superstep spans the whole child run (CF-FR-16). A child failure surfaces as the Battalion
  node's structured `NodeError` naming the child node and thread; cancellation is observed by the
  child at its superstep boundary (child persists `Halted`), after which the parent halts at its
  own boundary.
- **D-22: Legacy patterns embed unchanged.** `from_formation`/`from_phalanx`/`from_campaign`
  return `WarGraph`s that embed via `Arc::new(...)`; the CF-FR-17 integration test embeds a
  Formation subgraph as a node of a branching parent graph and includes the acceptance-4
  kill-after-child-superstep-1 → resume → child work not repeated assertion.

### LLM routing shape (CF-05)
- **D-23: `LlmDecision` is a registered evaluator, not an `EdgeCondition` variant.** `EdgeCondition`
  is a pre-existing public enum without `#[non_exhaustive]` and §9.2 records "no variant change";
  adding a variant would be an unsanctioned X-10 break. Edges use `EdgeCondition::Custom("<decision
  name>")` and the decision is registered under that name (D-03). `LlmDecisionEvaluator { llm:
  Arc<dyn LlmPort>, model, prompt_template, choices: Vec<(String, NodeId)>, on_ambiguous:
  OnAmbiguous::{Fail, Default(String)} }` lives in `paladin-battalion` beside the trait (D-01);
  core stays pure. The template renders from the Battlefield via `InputMapping` on the engine path
  and from the source output on the legacy path. — **Reversibility:** costly — public API.
- **D-24: One LLM call per decision per superstep, memoized.** The evaluator resolves its verdict
  once per (thread, superstep, source node) and every outgoing edge of that decision is judged
  against the same answer: an edge fires iff the chosen choice's mapped target equals
  `EdgeContext.target`. Without this, N edges → N independent calls → both-or-neither firing, a
  routing corruption of exactly the BUG-01 kind. Matching is exact-after-trim, case-insensitive;
  no match → `on_ambiguous` (PRD). Memo scope and eviction are Claude's discretion (e.g. cleared at
  each superstep boundary).
- **D-25: Commander `StrategySelection`.** `StrategySelection::{Heuristic, Semantic { llm: Arc<dyn
  LlmPort>, model: String }}` with a manual `Debug` impl, default `Heuristic` (today's
  `analyze_and_select` keyword heuristics, unchanged); `CommanderBuilder::strategy_selection(sel)`
  is the additive method; `Commander::new` keeps its signature and `Commander` gains **no new
  public field** (store privately — the §9.2 row says to verify this). Semantic mode prompts with
  the strategy catalog + input and parses the strategy name exact-after-trim, case-insensitive.
  **Any LLM error or an unrecognized/ambiguous answer** falls back to the heuristic with
  `strategy_selection_reasoning` recording the fallback and its cause. The 52 inline Commander
  tests and `tests/integration/commander_*` pass unmodified; new tests use `MockLlmAdapter`
  (`with_response`, `with_error`).
- **D-26: Off by default, code-configured.** No evaluators registered, `PlainOutput`, `Heuristic`
  — a v0.9 configuration boots identically. `LlmDecision`/`Semantic` hold trait objects and are
  configured in code, not via `APP_*` env; `on_ambiguous`/`on_parse_error` are per-evaluator/
  per-node enums, not runtime-global tunables, so X-09 adds no config struct for them. §9.5
  records this explicitly alongside D-16's `EngineConfig`.

### Program-gate obligations & docs
- **D-27: §9.2 rows resolved this phase.** `EdgeCondition` — Mitigation: none required (no
  signature change), Deliberate-breaking: N (the semantic change is M-B-01). `Commander`/
  `CommanderBuilder` — additive method + private field, N. `CampaignExecutionService` — additive
  builder method, N. Plus a "deliberate zero" note for the new-in-0.10 engine/waypoint types this
  phase reshapes (D-07). `cargo semver-checks` (vs 0.9.0) and the `msrv` (1.88) job must be green
  on the phase's final commit; `.project/current-exports.txt` regenerated if the surface it
  tracks moves; `make security` and `cargo clippy -- -D warnings` clean.
- **D-28: Docs (X-08).** A new mdBook page for control flow (Directives/Goto/End, Muster,
  subgraphs, `LlmDecision` and Commander Semantic, each with a minimal example) with a short
  WarGraph preamble, wired into `docs/src/SUMMARY.md`; doc-tests on every new public API; `cargo
  doc` with no new broken intra-doc links. The tree has **no** mdBook page for the WarEngine at all
  today — a Phase 22 X-08 residual recorded under Deferred Ideas, not absorbed here beyond the
  preamble.
- **D-29: `.project/` registrations.** PRD 01 gets the ENG-FR-11 clarification note (D-14);
  `08-traceability-matrix.md`'s BUG-01 row is updated with the fix's RED and GREEN commits when
  they land (protocol step 4); nothing else in `.project/` is edited.
- **D-30: Test tiers.** Everything CF adds is Tier 1 (`MockLlmAdapter`, `RecordingPaladinPort`,
  `InMemoryWaypointStore`, `CountingFunctionNode`); the SQLite/Postgres contract-suite additions
  for `muster_progress` and `checkpoint_ns` run in their existing tiers (Postgres Tier 2 via
  `make test-integration-docker`). Coverage stays ≥ 82% workspace (ADR-0006).

### Claude's Discretion
- Module layout: where `Directive`/`NextStep`/`MusterTask` sit in `paladin-core`
  (`platform::container::directive` vs inside `battlefield`), the battalion module name for the
  evaluator trait/registry, and whether `LlmDecisionEvaluator` gets its own file.
- The `NodeSpec::Paladin` constructor that keeps existing literals compiling after D-11; whether
  `NodeSpec::Battalion` gets a builder.
- Exact `EngineError` variant names/messages for D-02, D-04, D-08, D-10, D-13, D-19; duplicate
  evaluator-name registration policy (typed error vs replace — mirror `ReservedDispatchName`'s
  stance).
- `MusterProgress`, `MusterContext`, `FrontierSnapshot` interplay and `checkpoint_ns` exact
  shapes (core value types per ADR-0016); whether `BATTLEFIELD_SCHEMA_VERSION` bumps for the
  additive Waypoint fields (follow the `visit_counts`/`frontier` precedent).
- How `InputMapping::render` receives the muster context (D-15) and how the `muster.` prefix
  rejection is phrased.
- Memo scope for D-24; the Semantic prompt wording for D-25; `restart_on_resume` child-thread
  policy for D-20.
- Whether `NodeInterceptor::after` should see `NextStep` (recommended: leave the ENG-07 hook
  untouched this phase; Phase 26 middleware may extend it).
- Plan/wave decomposition, respecting PRD 02 §4's TDD order. Suggested: (1) CF-01 RED tests →
  fix on both paths → M-B-01 example; (2) Directive types, `StateNode` change, Goto/End/Parley
  arms, `DirectiveParser`; (3) Muster (marker, timing, limits, progress Waypoints, `muster.`
  namespace), `EngineConfig`, fingerprint `v3`; (4) subgraphs (variant, `StateMap`, child
  threads, inheritance, recursion check, bridge-embedding test); (5) `LlmDecision` + Commander
  Semantic; (6) E2E-3 half, stress test, mdBook page, MIGRATION.md sweep, CI evidence. (2) is a
  prerequisite of (3)–(5); (4) and (5) are independent of each other.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase source of truth (behavior)
- `.project/v0.10.0/02-control-flow-routing-fanout-subgraphs.md` — **The FR-level source of
  truth for this phase.** CF-FR-01…19, type sketches (§2), acceptance criteria (§3), TDD
  test-plan ordering (§4), out-of-scope (§5). Every plan task traces to an FR here.
- `.project/v0.10.0/00-program-overview.md` — §3 cross-cutting rules X-01…X-11 (X-03 stop-and-
  flag and the single sanctioned break; X-06 structured errors; X-09 config structs; X-10 public-
  type discipline), §4 ubiquitous language (Directive, Muster, Parley, Vanguard), §6 **E2E-3**
  (the muster half this phase must pass), §7 **BUG-01** (mandatory, ships with Doc 02) and the
  BUG-02/03/04 pre-release classification this phase reuses, §9.1 **M-B-01** (worked example
  owed here), §9.2 the three CF rows.
- `.project/v0.10.0/01-battlefield-state-and-execution-engine.md` — ENG-FR-02a (eligible-set /
  `mark_dynamic_target`, the Goto rule D-08(d) reuses), ENG-FR-06/06a (join, defer and starvation
  semantics Muster and Goto must preserve), **ENG-FR-11** (one Waypoint per superstep — the note
  D-14 adds), ENG-FR-12/12a (resume, `FrontierSnapshot`), ENG-FR-14 (fingerprint contents and
  exclusions — D-18 extends), ENG-FR-19 (the bridges CF-FR-17 embeds), §8 seams.
- `.project/v0.10.0/08-traceability-matrix.md` — G-07/G-08/G-09/G-20 and BUG-01 rows; protocol
  steps 4 (test-first, grep-absence) and 7 (behavioral-change audit: §9.1 contains exactly
  M-B-01…03).
- `.planning/REQUIREMENTS.md` — CF-01…05 capability clusters with FR ranges; the X-10/X-11
  versioning gate as part of every requirement's definition of done.
- `.planning/ROADMAP.md` — Phase 23 goal, dependency on Phase 22, the five success criteria;
  Phases 24/25 depend on this phase (FT-04's E2E-3 on CF-03).

### Program deliverable this phase appends to
- `MIGRATION.md` — §9.1 M-B-01 (worked example TBD, owner CF-01); §9.2 rows for `EdgeCondition`,
  `Commander`/`CommanderBuilder`, `CampaignExecutionService` (all "TBD — Phase 23"); §9.5's
  "`EngineConfig` … not yet in the tree" claim D-16 closes; §9.8's "register custom evaluators"
  step this phase makes real.

### Prior-phase decisions that constrain this phase
- `.planning/phases/22-battlefield-state-superstep-engine/22-CONTEXT.md` — D-04 (fingerprint
  canonicalization, one-way after release), D-09 (contract-suite style), D-10 (Postgres Tier 2),
  D-11 (seeded-shuffle determinism harness), D-12 (engine defaults, snapshot isolation).
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-CONTEXT.md` — D-01…D-04
  (starvation release, `UnschedulableCycle`, `StarvedNodeAtCompletion` — D-08/D-09 interact with
  these), D-15…D-19 (fingerprint coverage rule "everything that changes scheduling or merge
  semantics is hashed"; golden test; serde-canonical bytes), D-20 (program-gate obligations),
  D-21…D-25 (`FrontierSnapshot` on the Waypoint; additive `#[serde(default)]` fields; no SQL
  migration; contract-suite round-trip cases — the pattern D-14/D-20 follow).
- `.planning/phases/22.1-engine-readiness-defect-and-msrv-follow-up/22.1-REVIEW.md` and
  `22.1-REVIEW-FIX.md` — CR-01 (delimiter-collision lesson behind D-18/D-20's encoding rule; the
  `v2` length-prefixed format now in `graph.rs`), WR-01 (no `.expect()` in spawned tasks).
- `.planning/phases/22-battlefield-state-superstep-engine/22-deferred-items.md` — §2's finding
  named Phase 23's Muster as a consumer of the readiness computation that 22.1 then fixed.

### Standing decisions and governance
- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — 82% workspace line-coverage floor.
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` (ADR-0015) — `paladin-core`
  stays pure: `Directive`/`MusterTask` add no core dependency; `LlmDecision` cannot live in core.
- `.planning/decisions/0016-port-value-type-ownership.md` (ADR-0016) — core owns value types
  (`MusterProgress`, `checkpoint_ns`, `Directive`); battalion owns behavior.
- `.github/instructions/security.instructions.md` — the `LlmDecision` evaluator sends
  Battlefield-rendered content to a model; the prompt template is the author-controlled boundary
  for what leaves the process (document it; no API key may reach logs or errors).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/paladin-battalion/src/engine/dispatch_registry.rs` — `CustomDispatchResolver`: the
  registry pattern D-01/D-03 mirror for `EdgeEvaluatorRegistry` (`with_dispatch_rule`,
  `ReservedDispatchName`).
- `crates/paladin-battalion/src/engine/graph.rs` — `NodeSpec` (`#[non_exhaustive]`, rustdoc
  already promises the `Battalion` variant), `EdgeSpec { from, to, condition: Option<EdgeCondition> }`,
  `EngineLimits` (D-16 adds `max_muster_tasks`), `WarGraph { defer_flags, dynamic_targets, … }`
  with `add_deferred_node`/`mark_dynamic_target` (the model for `add_worker_template`),
  `validate(&self, custom_dispatch)` at line 290 (gains the evaluator registry), `fingerprint()`
  at line 564 (`v2` length-prefixed encoding; D-18 bumps to `v3`).
- `crates/paladin-battalion/src/engine/superstep.rs` — `evaluate_edge_condition` at line 1189
  (the engine BUG-01 site, line 1206), `NodeDispatch` for Paladin nodes (lines 55-115: where
  `PaladinResult.output` is written to `output_field` — the `DirectiveParser` insertion point),
  `Frontier`/`EdgeState`/`compute_next_vanguard` (Goto injection and `NotFiring` resolution,
  starvation-release tiers), the seeded-shuffle determinism tests.
- `crates/paladin-battalion/src/engine/node.rs` — `NodeContext { node_id, thread_id, superstep }`
  (D-15 adds `muster`), `StateNode::run` (D-07 changes its return type).
- `crates/paladin-battalion/src/engine/input_mapping.rs` — `InputMapping::render(&Battlefield)`
  (D-15 extends with the muster context; `LlmDecision` reuses it for prompt templates).
- `crates/paladin-battalion/src/engine/bridges.rs` — `from_formation`/`from_phalanx`/
  `from_campaign` → `WarGraph` (CF-FR-17 embeds them as-is), `dedicated_output_field`.
- `crates/paladin-battalion/src/engine/mod.rs` — `WarEngine` builder methods (`with_dispatch_rule`,
  `with_trace_sink`, `with_interceptors`, `with_cancellation_token` — D-03 adds
  `with_edge_evaluator`; D-21 forwards all of them to the child), `RunOutcome`, `EngineError`
  (`#[non_exhaustive]`; new variants per D-02/D-04/D-08/D-10/D-13/D-19), `ResumeOptions`,
  `resume_with_options`.
- `crates/paladin-battalion/src/engine/test_support.rs` — `CountingFunctionNode`,
  `RecordingPaladinPort` (`set_output` scripts Paladin outputs — the Directive-envelope test
  vehicle), `RecordingWaypointStore` (`fail_next_save`, `saved_waypoints` — mid-muster resume
  tests), `RecordingTraceSink`, `shuffle_seeded`.
- `crates/paladin-battalion/src/campaign_service.rs` — `CampaignExecutionService::new(paladin_port)`
  (unchanged), `with_herald` (the additive-builder precedent for `with_evaluator`), `execute` →
  `campaign.validate()` at line 174 (fail-closed check goes right after), the legacy BUG-01 site
  at lines 392-397 inside `evaluate_edge_condition` (line 378).
- `crates/paladin-battalion/src/commander.rs` — `Commander` (public fields + private
  `paladin_port`), `CommanderBuilder` (private fields, `new(paladin_port)`, fluent methods),
  `analyze_and_select` at line 1023 (the Heuristic default), `strategy_selection_reasoning` on the
  result; 52 inline tests plus `tests/integration/commander_integration_tests.rs` and
  `commander_error_paths_test.rs` that must pass unmodified.
- `crates/paladin-core/src/platform/container/waypoint.rs` — `Waypoint` (additive
  `#[serde(default)]` precedent: `visit_counts`, `frontier`), `FrontierSnapshot`,
  `NodeExecutionRecord { …, attempt }`, `ParleyRequest` stub (line 295), `ThreadId(String)`,
  `NodeId(String)` (no charset restriction — D-20's injectivity requirement).
- `crates/paladin-core/src/platform/container/battalion/campaign.rs:34` — `EdgeCondition`
  (`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`; **not** `#[non_exhaustive]`) — untouched.
- `crates/paladin-core/src/platform/container/battalion/mod.rs:732` — `BattalionError` (not
  `#[non_exhaustive]`; `InvalidGraph(String)` at line 755 is the variant D-04 reuses).
- `crates/paladin-llm/src/mock.rs` — `MockLlmAdapter` (`with_response`, `with_responses`,
  `with_error`, `with_error_then_response`, `call_count`) and `MultiStepMockLlmPort` for D-24/D-25
  tests; `paladin-battalion` already depends on `paladin-ports` (so `Arc<dyn LlmPort>` is
  available) and on `serde_json`/`regex`.
- `crates/paladin-storage/src/waypoint/contract_tests.rs` — the shared three-backend suite that
  gains the `muster_progress`/`checkpoint_ns` round-trip cases.
- `src/config/citadel.rs`, `src/config/waypoint_retention.rs` — the X-09 shape `EngineConfig`
  (D-16) mirrors.
- `tests/integration/e2e_crash_resume_test.rs` — E2E-1, the golden that must not move; the
  template for the E2E-3 integration test.

### Established Patterns
- Test-first defect fixes with failing-then-passing order visible in history (BUG-01…04;
  traceability protocol step 4) — D-05.
- Additive `#[serde(default)]` Waypoint fields with no SQL migration and a contract-suite
  round-trip case (`visit_counts`, `frontier`) — D-14/D-20.
- Fingerprint discipline: hash everything scheduling- or merge-relevant, sorted, length-prefixed,
  pinned by a golden test, version tag bumped on layout change — D-18.
- Pre-release classification for engine/waypoint types absent at `v0.9.0` (no §9.1/§9.2 rows,
  "deliberate zero" note) — D-07/D-27.
- Typed `EngineError` variants, `thiserror`, `#[non_exhaustive]`; no new stringly-typed variants
  except where an FR names an existing one (D-04) — X-06.
- Additive builder methods on legacy services (`with_herald`) instead of constructor changes —
  D-03/D-25.
- Three-tier tests; seeded-shuffle determinism harness (≥ 20 iterations); X-05 multi-thread
  stress with exact counts and a timeout guard.
- Ubiquitous language: Directive, Muster, Vanguard, Parley, Battlefield, Waypoint, Thread,
  WarGraph, WarEngine, Commander — in code, docs and comments.

### Integration Points
- `crates/paladin-core/src/platform/container/` — new `Directive`/`NextStep`/`MusterTask` types;
  `waypoint.rs` gains `muster_progress` and `checkpoint_ns`; `ThreadId::child`.
- `crates/paladin-battalion/src/` — new evaluator trait/registry module (+ `LlmDecisionEvaluator`);
  `campaign_service.rs` (`with_evaluator`, fail-closed check, placeholder removal);
  `commander.rs` (`StrategySelection`, builder method); `engine/graph.rs` (`Battalion` variant,
  `worker_templates`, `validate` signature, fingerprint `v3`); `engine/superstep.rs` (Goto/End/
  Parley/Muster arms, progress Waypoints, `NotFiring` on Goto, End vs starvation check, Custom arm
  removal); `engine/node.rs` (`Directive` return, `MusterContext`); `engine/input_mapping.rs`
  (`muster.` namespace); `engine/mod.rs` (`with_edge_evaluator`, child-run construction, new
  errors).
- `src/config/engine.rs` (new, D-16) + `src/config/mod.rs` registration; `MIGRATION.md`
  §9.1/§9.2/§9.5.
- `tests/integration/` — E2E-3 (muster half), Formation-inside-Campaign subgraph test with
  resume-mid-child; `crates/paladin-storage/src/waypoint/contract_tests.rs` additions.
- `docs/src/SUMMARY.md` + the new control-flow page; `CHANGELOG.md` `[Unreleased]`.
- `.project/v0.10.0/01-…engine.md` (ENG-FR-11 note) and `08-traceability-matrix.md` (BUG-01 row
  commit refs).
- **Constraint confirmed in tree:** the legacy Campaign still rejects cycles
  (`campaign_service.rs`, `campaign.rs:246`); `from_campaign` bridges are the only way a
  Campaign-shaped graph reaches Goto/Muster — legacy services stay byte-identical except BUG-01.

</code_context>

<specifics>
## Specific Ideas

- The BUG-01 fix must be visibly *two* removals: the legacy `warn!("… defaulting to true")` and
  the engine's `Ok(true)` arm that copied it. A grep for "defaulting to true" after the fix must
  return nothing in `crates/`.
- M-B-01's worked example should read like a migration, not a spec: the exact v0.9 line that
  silently always-routed, the exact v0.10 line that registers the evaluator, and the validation
  error text a user will see if they skip it.
- `LlmDecision`'s single-call-per-decision rule (D-24) deserves its own test: two outgoing edges,
  one mock answer, assert exactly one `LlmPort` call and exactly one edge fired.
- The End-vs-`StarvedNodeAtCompletion` interaction (D-09) and the Goto-`NotFiring` rule (D-08c)
  are the two places where this phase touches 22.1's truthful-outcome machinery; each gets a
  named test so a regression is loud, in the spirit of BUG-02/03/04.
- The mid-muster progress Waypoint (D-14) stores *unmerged* deltas — never a partially merged
  Battlefield. A test should assert the Battlefield on a progress Waypoint equals the superstep's
  start snapshot.
- Ubiquitous language holds: Directive, Muster, worker template, Vanguard, Parley, Battlefield,
  Waypoint, Thread, WarGraph, WarEngine, Commander.

</specifics>

<deferred>
## Deferred Ideas

- **No mdBook page for the WarEngine exists** (`docs/src/SUMMARY.md` has no engine/Battlefield/
  Waypoint entry) despite X-08 — a Phase 22 residual. D-28 adds only a short preamble on the
  control-flow page; the full engine page belongs to a docs pass or SHIP-01 (Phase 29).
- **Overview §4 describes Directive routing as `Goto`, `End`, `Halt`**; PRD 02's `NextStep` has
  no `Halt`. PRD 02 governs; the overview wording is a Phase 29 doc-sweep item unless the
  overview is edited for another reason.
- **Native provider JSON mode for `StructuredDirective`** — RT-05 (Phase 26) may swap D-11's
  text extraction for `execute_structured`/`response_format`; the envelope is designed to be
  reusable there.
- **`NodeInterceptor` visibility of `NextStep`** — Phase 26 middleware may want to observe or veto
  routing; the ENG-07 hook is left untouched here.
- **Per-task retry inside a Muster** (FT-FR-06, Phase 25), **Parley suspension** (HITL-01, Phase
  24), **subgraph fork semantics** (HITL-FR-12, Phase 24) — seams only.
- **22-deferred-items.md item 1** (`qdrant` `--all-features` rustdoc break) and **rmcp 3.x** —
  unchanged, not this phase's.

</deferred>

---

*Phase: 23-control-flow-dynamic-routing-fan-out-subgraphs*
*Context gathered: 2026-09-03*
