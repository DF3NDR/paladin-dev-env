# Phase 23: Control Flow — Dynamic Routing, Fan-Out & Subgraphs - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-03
**Phase:** 23-control-flow-dynamic-routing-fan-out-subgraphs
**Mode:** `--auto` — every question below was answered by selecting the recommended option without
user prompting. `[--auto] Selected all gray areas.`
**Areas discussed:** BUG-01 fail-closed mechanism & error shapes, Directive semantics at the engine
boundary, Muster execution & mid-muster persistence, Subgraph identity & checkpoint namespacing,
LLM routing shape, Program-gate obligations & docs

---

## BUG-01 fail-closed mechanism & error shapes

| Option | Description | Selected |
|--------|-------------|----------|
| Trait/registry in `paladin-battalion` | Mirrors `engine/dispatch_registry.rs`; both consumers live there; `EdgeCondition` in core untouched | ✓ |
| Trait in `paladin-core` beside `EdgeCondition` | Pure, but no house precedent and the LLM evaluator cannot follow it there | |
| Trait in `paladin-ports` | Not an infrastructure port | |

`[auto] Q: "Sync or async evaluator trait?" → Selected: "async_trait" (recommended — CF-FR-18 must await an LlmPort call; blocking a Tokio worker is a house anti-pattern; new trait so X-10 does not apply)`

`[auto] Q: "Legacy error shape?" → Selected: "reuse existing BattalionError::InvalidGraph(String)" (recommended — PRD CF-FR-02 and the published M-B-01 row name it; a new variant on the non-non_exhaustive BattalionError would be an unsanctioned X-10 break). Engine path gets structured EngineError::UnregisteredEdgeCondition { names }.`

`[auto] Q: "Registry placement?" → Selected: "WarEngine::with_edge_evaluator + WarGraph::validate parameter; CampaignExecutionService::with_evaluator additive" (recommended — mirrors with_dispatch_rule / with_herald)`

`[auto] Q: "Commit ordering?" → Selected: "RED tests for both paths in a separate commit before the fix" (recommended — traceability protocol step 4, SHIP-03)`

**Notes:** X-06 tension (new call site on a `String` variant) recorded and resolved in favor of the explicit FR.

---

## Directive semantics at the engine boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Change `StateNode::run` return type directly | Pre-release trait; `From<StateDelta>`; no register row | ✓ |
| Add `run_directive` with a default impl | Keeps `run` but doubles the trait surface for a type absent at v0.9.0 | |

`[auto] Q: "Goto vs the emitting node's static outgoing edges?" → Selected: "resolve NotFiring; Goto-only targets use the existing mark_dynamic_target rule" (recommended — preserves ENG-FR-06 join semantics, avoids BUG-03-style starvation, no new mechanism)`

`[auto] Q: "End vs the StarvedNodeAtCompletion check?" → Selected: "suppress for End-terminated runs, documented and tested" (recommended — End is node-authored termination, not a scheduler lie)`

`[auto] Q: "NextStep::Parley returned this phase?" → Selected: "typed EngineError::ParleyNotSupported" (recommended — fail loudly; alternatives 'treat as Edges' and 'persist AwaitingInput stub' rejected)`

`[auto] Q: "StructuredDirective extraction and output_field?" → Selected: "whole-output JSON else first json fence; delta-only write; output_field used by FallbackPlain" (recommended)`

`[auto] Q: "DirectiveParser placement?" → Selected: "per-node field on NodeSpec::Paladin with PlainOutput default" (PRD: 'for NodeSpec::Paladin, a configurable DirectiveParser')`

---

## Muster execution & mid-muster persistence

| Option | Description | Selected |
|--------|-------------|----------|
| Graph-level `worker_templates` set via `add_worker_template` | Mirrors `defer_flags`/`dynamic_targets`; hashable; `NodeSpec` variants unchanged | ✓ |
| `worker_template: bool` on every `NodeSpec` variant | Literal PRD wording; awkward on tuple variant; no house precedent | |

`[auto] Q: "Mid-muster resume persistence?" → Selected: "intra-superstep progress Waypoints carrying unmerged per-task deltas; ENG-FR-11 clarified in PRD 01" (recommended — the only option satisfying CF-FR-12 and test-plan item 5 while keeping CF-FR-10 concurrency; alternatives 'superstep-end only' and 'one task per superstep' rejected)`

`[auto] Q: "When do tasks run?" → Selected: "all tasks concurrently in the superstep after the planner; duplicate key / limit checked before any task starts" (recommended)`

`[auto] Q: "X-09 for max_muster_tasks?" → Selected: "create src/config/engine.rs EngineConfig now, including the four Phase 22 fields MIGRATION.md §9.5 already documents as planned" (recommended — closes a stale 'not yet in the tree' claim at low cost; alternative 'defer to Phase 29' rejected)`

`[auto] Q: "Fingerprint version?" → Selected: "bump v2 → v3 with worker-template set, child fingerprints + StateMap + restart_on_resume, DirectiveParser kind" (recommended — 22.1 D-15/D-17 discipline; 'append only when non-empty to keep v2' rejected as fragile)`

`[auto] Q: "muster.payload / muster.task_key resolution?" → Selected: "reserved muster. namespace resolved from NodeContext, schema fields with that prefix rejected at validation" (recommended)`

---

## Subgraph identity & checkpoint namespacing

| Option | Description | Selected |
|--------|-------------|----------|
| Derived injective child `ThreadId` + additive `checkpoint_ns` on Waypoint | No `WaypointPort` method change; `latest(child)` is the child's own namespaced latest | ✓ |
| Same `ThreadId` + namespace-filter methods on `WaypointPort` | Touches three backends and the contract suite; complicates `latest`/retention | |

`[auto] Q: "Child engine configuration?" → Selected: "inherit every parent engine setting (ports, durability, parallelism, resolver, registry, sink, interceptors, cancellation)" (recommended; 'user-supplied child engine' rejected)`

`[auto] Q: "Recursion check?" → Selected: "path-set walk over child fingerprints at parent validate, typed RecursiveEmbedding error" (PRD CF-FR-16 'fingerprint cycle check')`

`[auto] Q: "Retention for child threads?" → Selected: "ordinary threads; protected set applies per child thread; no service change" (recommended)`

**Notes:** Encoding of the child thread id must be injective — 22.1 CR-01's delimiter-collision finding is the cited reason.

---

## LLM routing shape

| Option | Description | Selected |
|--------|-------------|----------|
| `LlmDecision` as a registered evaluator under `Custom(name)` | `EdgeCondition` unchanged; §9.2 row honored | ✓ |
| New `EdgeCondition::LlmDecision` variant | Unsanctioned X-10 break on a non-non_exhaustive core enum | |

`[auto] Q: "LLM calls per decision?" → Selected: "one memoized call per (thread, superstep, source); edge fires iff chosen target == EdgeContext.target" (recommended — per-edge calls could fire both or neither edge, a BUG-01-class corruption)`

`[auto] Q: "Commander Semantic fallback scope?" → Selected: "any LLM error OR unrecognized/ambiguous answer falls back to Heuristic with reasoning recorded" (recommended; 'error only' rejected)`

`[auto] Q: "Commander API shape?" → Selected: "StrategySelection enum with manual Debug; CommanderBuilder::strategy_selection additive; Commander::new unchanged; no new public field" (§9.2 row)`

`[auto] Q: "Config surface for LLM routing?" → Selected: "code-configured, off by default; no APP_* vars; §9.5 records it" (recommended)`

---

## Program-gate obligations & docs

| Option | Description | Selected |
|--------|-------------|----------|
| New control-flow mdBook page with a short WarGraph preamble | X-08 for this epic; missing ENG page flagged as Phase 22 residual | ✓ |
| Also write the full engine page | Absorbs Phase 22 debt into this phase | |

`[auto] Q: "§9.2 rows?" → Selected: "EdgeCondition: no mitigation needed / N; Commander: additive + private field / N; CampaignExecutionService: additive / N; deliberate-zero note for new-in-0.10 types" (recommended)`

`[auto] Q: ".project/ edits?" → Selected: "ENG-FR-11 clarification note in PRD 01 + BUG-01 commit refs in the traceability matrix; nothing else" (recommended)`

---

## Claude's Discretion

Module layout for the new core/battalion types; `NodeSpec::Paladin` constructor shape; exact
`EngineError` names/messages; duplicate evaluator-name policy; `MusterProgress`/`MusterContext`/
`checkpoint_ns` exact shapes and `BATTLEFIELD_SCHEMA_VERSION` handling; `InputMapping` muster
plumbing; memo scope for `LlmDecision`; Semantic prompt wording; `restart_on_resume` thread
policy; whether `NodeInterceptor::after` sees `NextStep` (recommended: no, this phase); plan/wave
decomposition respecting PRD 02 §4.

## Deferred Ideas

- No mdBook page for the WarEngine exists — Phase 22 X-08 residual.
- Overview §4 lists `Halt` among Directive routing; PRD 02 has no `Halt` — Phase 29 doc sweep.
- Native JSON mode for `StructuredDirective` — RT-05, Phase 26.
- `NodeInterceptor` visibility of `NextStep` — Phase 26.
- Per-task retry (Phase 25), Parley suspension and subgraph fork semantics (Phase 24) — seams only.

## Cross-referenced todos

`todo.match-phase 23` returned no matches (1 pending todo, score below threshold) — nothing folded.
