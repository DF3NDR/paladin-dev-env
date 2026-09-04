# PRD 02 — Control Flow: Dynamic Routing, Custom Conditions (Bug Fix), Muster Fan-Out, Subgraphs (Epic `CF`)

**Depends on:** PRD 01 (Battlefield, WarEngine, Waypoint).
**Primary crates:** `paladin-core`, `paladin-battalion`, `paladin-ports`.

---

## 1. Problem Statement

Control flow in Paladin today is fixed at graph-build time. Campaign edges carry `EdgeCondition::{Always, Contains, Regex, Custom}` evaluated against a node's string output; `Custom` is a stub that logs a warning and evaluates **true** (BUG-01) — a silent routing corruption. Nodes cannot decide their successor, cannot spawn a runtime-determined number of parallel workers, and Battalions cannot nest (a Formation cannot be a node inside a Campaign).

This epic delivers: (1) the BUG-01 fix, (2) the **Directive** return type letting nodes steer routing, (3) **Muster** dynamic fan-out (map-reduce), (4) **subgraph composition**, and (5) optional LLM-evaluated routing for both edges and Commander strategy selection.

## 2. Functional Requirements

### 2.1 BUG-01 — Custom edge conditions (standalone; may ship before the engine)

- **CF-FR-01 (Registered evaluators).** `CampaignExecutionService` (legacy) and the WarEngine (new) MUST each hold a registry `HashMap<String, Arc<dyn EdgeConditionEvaluator>>` where:

  ```rust
  pub trait EdgeConditionEvaluator: Send + Sync {
      /// Legacy signature: string output. Engine variant receives &Battlefield too.
      fn evaluate(&self, output: &str, ctx: &EdgeContext) -> Result<bool, BattalionError>;
  }
  ```

  `EdgeContext` carries source/target NodeIds and (engine path) a read-only Battlefield reference.
- **CF-FR-02 (Fail closed at validation).** Graph/campaign validation MUST fail with `BattalionError::InvalidGraph { reason }` naming every `Custom(name)` condition whose `name` is not registered — **before any node executes**. The current behavior (warn + return true at runtime) MUST be removed. There is no configuration that restores the old behavior. This is the program's single sanctioned behavioral break (overview §7, `MIGRATION.md` M-B-01): the migration entry MUST include a worked before/after example showing how to register an evaluator for an existing `Custom` condition. Compatibility constraint: `CampaignExecutionService::new(paladin_port)` keeps its signature; the registry is supplied through an additional builder method (e.g. `with_evaluator(name, evaluator)`) — register the change in §9.2 as additive.
- **CF-FR-03 (Runtime evaluator error).** If a registered evaluator returns `Err` at runtime, the run fails with a typed error naming the edge and evaluator; it does not default to either branch.
- **CF-FR-04 (Regression tests).** Tests MUST cover: unregistered custom condition rejected at validation; registered condition true-routes; registered condition false-routes; evaluator error fails the run. The fix MUST be developed test-first (overview §5.5).

### 2.2 Directive — node-driven routing

- **CF-FR-05 (Directive type).** In `paladin-core`:

  ```rust
  pub struct Directive {
      pub delta: StateDelta,
      pub next: NextStep,
  }
  pub enum NextStep {
      /// Follow the graph's static edges (default; preserves PRD-01 semantics).
      Edges,
      /// Jump to these nodes next superstep, ignoring this node's static outgoing edges.
      Goto(Vec<NodeId>),
      /// Muster: dynamic fan-out (CF-FR-09..12).
      Muster(Vec<MusterTask>),
      /// End the whole run successfully after this superstep's merge.
      End,
      /// Pause (Doc 03 defines semantics; enum variant declared here).
      Parley(ParleyRequest),
  }
  ```

  `StateNode::run` return type becomes `Result<Directive, NodeError>` (with `impl From<StateDelta> for Directive` defaulting `next: Edges`, so PRD-01 nodes need a one-line change at most — coordinate: PRD 01 may land `Directive` directly if both epics are scheduled together).
- **CF-FR-06 (Paladin nodes emit Directives).** For `NodeSpec::Paladin`, a configurable `DirectiveParser` maps `PaladinResult` → `Directive`. Built-ins that MUST ship: `PlainOutput` (write to `output_field`, `next: Edges` — default, backward compatible) and `StructuredDirective` (parse a JSON block in the output matching a documented envelope `{"delta": {...}, "next": {"goto": [...]}}`; parse failure → configurable `on_parse_error: FailRun | FallbackPlain`, default `FailRun`).
- **CF-FR-07 (Goto validation).** `Goto` targets MUST exist in the graph; an unknown target fails the run with `EngineError::GotoUnknownNode { from, to }`. `Goto` targets enter the next Vanguard subject to `max_node_visits` (loops via Goto are legal and bounded).
- **CF-FR-08 (End semantics).** `End` completes the run after merging the emitting superstep's deltas; peers in the same superstep still merge. Two nodes emitting conflicting `End` + `Goto` in one superstep: `End` wins; document and test this precedence.

### 2.3 Muster — dynamic fan-out / map-reduce

- **CF-FR-09 (MusterTask).**

  ```rust
  pub struct MusterTask {
      pub worker: NodeId,                 // must reference a node marked as a worker template
      pub payload: serde_json::Value,     // task-scoped input, NOT merged into the Battlefield
      pub task_key: String,               // unique within the muster; used for ordering & idempotency
  }
  ```

  A `NodeSpec` gains `worker_template: bool`. Worker-template nodes are excluded from entry points and from static-edge Vanguard computation; they run only when mustered.
- **CF-FR-10 (Worker execution).** All tasks of one Muster execute in the same superstep, concurrently, bounded by the engine parallelism limit. Each worker's `NodeContext` exposes `muster_payload()` and `task_key()`. For Paladin worker templates, `InputMapping` may reference `{muster.payload}` and `{muster.task_key}`.
- **CF-FR-11 (Deterministic aggregation).** Worker deltas merge under the normal dispatch rules but ordered by `task_key` (lexicographic), NOT by completion time. Two runs with identical worker outputs MUST produce identical Battlefields regardless of completion order (repeat test ≥ 20 iterations).
- **CF-FR-12 (Muster + join/defer).** A downstream node with `defer: true` after a Muster MUST run exactly once, only after every mustered task has resolved (including tasks recovered by Doc 04 retries). Waypoints written mid-muster MUST record per-task completion so resume (ENG-FR-12) re-runs only unfinished tasks. Duplicate `task_key` within one Muster → typed error before any task starts.
- **CF-FR-13 (Limits).** `EngineLimits.max_muster_tasks: u32` (default 100). Exceeding it fails the run with a typed error naming the mustering node and requested count.

### 2.4 Subgraph composition

- **CF-FR-14 (Battalion as node).** `NodeSpec::Battalion { graph: Arc<WarGraph>, state_map: StateMap }` embeds a child WarGraph as a node. `StateMap` declares `inputs: Vec<(parent_field, child_field)>` and `outputs: Vec<(child_field, parent_field)>`; child runs with its own schema seeded from mapped inputs; on completion, mapped outputs return as the node's delta (parent dispatch rules apply). Unmapped child fields stay private to the child.
- **CF-FR-15 (Checkpoint inheritance).** A child graph uses the parent's `WaypointPort` and thread, with a namespaced thread segment (`thread_id` + `checkpoint_ns` path such as `"parent_node_id/"`, recorded on the Waypoint). Resume of a parent mid-child MUST resume the child at its own latest namespaced Waypoint, not restart it. A child MAY be compiled with `restart_on_resume: true` to opt out (document the trade-off).
- **CF-FR-16 (Nested limits & cycles).** Child runs count against their own `EngineLimits`; a parent superstep containing a child counts as one parent superstep regardless of child superstep count. Recursive embedding (graph containing itself directly or transitively) MUST be rejected at validation via fingerprint cycle check.
- **CF-FR-17 (Legacy patterns as subgraphs).** The `from_formation` / `from_phalanx` / `from_campaign` constructors (ENG-FR-19) MUST produce graphs embeddable via CF-FR-14, giving "Formation inside a Campaign" for free. One integration test MUST demonstrate a Formation subgraph as a node of a branching parent graph.

### 2.5 LLM-evaluated routing (optional-on, deterministic-off by default)

- **CF-FR-18 (LlmEdgeCondition).** A new condition variant/evaluator `LlmDecision { prompt_template, choices: Vec<String> }`: renders the template from the Battlefield, calls a configured (cheap) model via `LlmPort`, and matches the response against `choices` (exact-after-trim, case-insensitive). No match → configurable `on_ambiguous: Fail | Default(choice)`. The evaluator is an engine-registered evaluator (CF-FR-01 mechanism) living in the application layer (it holds an `Arc<dyn LlmPort>`; core stays pure).
- **CF-FR-19 (Commander semantic mode).** `Commander` gains `StrategySelection::Semantic { llm: Arc<dyn LlmPort>, model: String }` alongside the existing keyword heuristics (`StrategySelection::Heuristic`, current default — unchanged). Semantic mode prompts the model with the strategy catalog + input and MUST fall back to Heuristic (with the fallback recorded in `strategy_selection_reasoning`) on any LLM error. Existing Commander tests must pass unmodified.

## 3. Acceptance Criteria

1. BUG-01: all CF-FR-04 tests green; grep confirms the warn-and-return-true branch is gone.
2. E2E-3 (overview §6: planner musters 5 workers, one recovers via retry, deferred aggregator runs once, ordered results) passes — retry half owned by Doc 04; the muster/defer/order half must pass with a manually-succeeding-on-attempt-N mock even before Doc 04 lands.
3. A Goto-based refine loop (writer → reviewer → Goto(writer) until reviewer output contains APPROVED, capped by max_node_visits) passes.
4. Formation-inside-Campaign subgraph test passes, including resume-mid-child (kill after child superstep 1, resume, assert child work not repeated).
5. Determinism repeat tests for Muster ordering green.
6. Coverage per X-02.
7. **Versioning gate (X-10/X-11):** any pre-existing public type touched by this epic is recorded in `MIGRATION.md` §9.2 with its mitigation; `cargo semver-checks` and the MSRV job pass; new dependencies listed in §9.3; new migrations in §9.4; new config/env in §9.5.

## 4. Test Plan (TDD ordering)

1. CF-FR-04 regression tests (fail first against current code).
2. Directive unit tests with Function nodes: Goto, End, precedence, unknown-target error.
3. DirectiveParser tests: PlainOutput passthrough; StructuredDirective happy path, malformed JSON under both `on_parse_error` modes.
4. Muster unit tests: payload isolation, task_key ordering, duplicate key rejection, limit breach, defer-join.
5. Muster resume tests (kill mid-muster with 2/5 tasks done; resume; exactly 3 execute).
6. Subgraph tests: state mapping in/out, private fields invisible to parent, namespaced waypoints, recursion rejection, restart_on_resume.
7. LLM routing tests with MockLlmAdapter: choice match, ambiguity modes, Commander semantic fallback.
8. Multi-thread stress: 50-task muster, exact counts, timeout guard.

## 5. Out of Scope

Parley runtime semantics (Doc 03 — only the enum variant is declared here); per-task retry policy internals (Doc 04); HTTP exposure of subgraph state (Doc 06).
