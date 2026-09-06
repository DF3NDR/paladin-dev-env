---
phase: 25-node-level-fault-tolerance
plan: 10
subsystem: infra
tags: [aegis, error-handler, route, absorb, compensation, saga, superstep-engine, validation, fault-tolerance]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-01)
    provides: "Aegis/ErrorHandlerSpec value types, the WarGraph aegis sidecar (set_aegis/with_default_aegis/aegis_for) and the retry loop that yields a FINAL failure only after exhaustion or non-retryability"
  - phase: 25-node-level-fault-tolerance (plan 25-03)
    provides: "ErrorHandler trait + ErrorHandlerRegistry, EngineRegistries bundle, WarGraph::validate's Aegis clauses and the fail-closed UnregisteredErrorHandler check this plan's dispatch relies on"
  - phase: 25-node-level-fault-tolerance (plan 25-07)
    provides: "NodeTaskOutput.node_error (structured NodeError on a final Aegis-governed failure), EngineError::NodeFailed, WaypointStatus::Failed.node_error and RunOutcome::node_error() -- the no-handler path this plan pins"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "The Goto machinery (goto_targets unioned into next_vanguard, notfiring_nodes) a Route target rides, and validate_eligible_set's documented Route-target insertion point (22-15)"
provides:
  - "EngineError::{RouteTargetUnknown, RouteTargetIsWorkerTemplate, RouteErrorFieldUndeclared, RouteErrorFieldDispatchInvalid, AbsorbDeltaSchemaInvalid} -- each listing every offender"
  - "WarGraph::validate_aegis_handler_wiring: Route target declared and not a worker template; error_field declared with a non-Sum dispatch; Absorb fallback_delta writes only declared fields (empty delta legal)"
  - "validate_eligible_set seeds every eligible node's Route { to } target into its fixed-point worklist (D-21: no mark_dynamic_target needed); unschedulable_unfed_nodes exempts Route targets like dynamic targets"
  - "superstep::dispatch_error_handler: Route -> serialized NodeError written into error_field + NextStep::Goto([to]); Absorb -> fallback_delta + NextStep::Edges; Custom -> registry handler awaited over the pre-superstep Battlefield snapshot; the handler's Directive is honoured like a node's own with the record reading outcome: Failed"
  - "No-handler path unchanged from 25-07: Failed Waypoint with node_error: Some + RunOutcome::Failed(NodeFailed) -- never a string"
  - "superstep::run/run_with_namespace/ChildEngineResources carry &EngineRegistries (was &EdgeEvaluatorRegistry) so a Battalion child inherits all three registries wholesale"
  - "Test doubles: PermanentlyFailingNode, RecoveryNode, RecordingErrorHandler (engine/test_support.rs)"
affects: [25-11-error-handlers-continued, 25-12-e2e-replacements, 25-14-migration-doc]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Handler dispatch lives in the sequential bookkeeping loop (dispatch order, first-wins preserved), BEFORE the merge, converting a final NodeRunOutcome::Failed into NodeRunOutcome::Succeeded(directive) + handled_failure=true so the existing Succeeded arm (Edges/Goto/Muster/End/Parley) honours the handler's Directive with zero duplicated routing code; only the record's outcome kind is overridden to Failed"
    - "Route reuses NextStep::Goto verbatim: goto_targets -> next_vanguard placement, notfiring_nodes suppression of static successors, and the one visit_counts/max_node_visits bound -- no second placement path and no second counter (T-25-47)"
    - "Timeout(EngineRun) is excluded from handler dispatch: the engine budget is gone, so the run ends RunTimeoutExceeded exactly as plan 25-09 built (D-20)"
    - "Validation clause aggregates every offender per class in node_order, returns the shallowest class first (target unknown -> worker template -> error_field undeclared -> Sum dispatch -> Absorb fields), matching the CF-01 discipline of its siblings"
    - "Eligibility is a true fixed point: the Route target is pushed when its OWNING node pops from the worklist (not pre-seeded from every node), so a target's own outgoing edges re-expand -- pinned by a_route_target_reached_late_has_its_own_edges_expanded"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs

key-decisions:
  - "superstep::run's `evaluators: &EdgeEvaluatorRegistry` parameter became `registries: &EngineRegistries` (pub(crate) signature; 4 engine call sites, ~30 test call sites) rather than adding a fifth registry parameter -- the error-handler registry has to reach the dispatch loop, and a Battalion child now inherits the whole bundle (D-13) instead of only the edge evaluators"
  - "A handler-compensated failure records NodeOutcomeKind::Failed regardless of the Directive it returns (Edges/Goto/End/Parley/Muster): the node DID fail; end_requested/goto_targets/parley_requests still carry the routing, and only tests read Ended/Parleyed record kinds"
  - "The intermediate Custom arm between Task 2 and Task 3 was an explicit re-fail with the original error (run ends NodeFailed, never an Absorb-shaped fallthrough) instead of the plan's 'compile error': ErrorHandlerSpec is #[non_exhaustive] from paladin-core, so a match in this crate must carry a wildcard and cannot be made to fail compilation on a missing arm"
  - "The plan's acceptance grep `grep -c max_node_visits superstep.rs -le 3` was unsatisfiable before this plan (9 pre-existing hits: 3 production + 6 in earlier tests); the intent holds -- production references are exactly the 3 pre-existing ones (visit-bound check, the NodeVisitLimitExceeded constructor, one comment), none added; the 4 new hits are the plan-mandated test name and its doc comment"
  - "Route-target eligibility also exempts the target from unschedulable_unfed_nodes' unfed-cycle survivors (the rustdoc had reserved that exclusion for Phase 25); worker templates need no such exemption because validate_worker_templates already forbids any incoming edge on them"
  - "The paladin-core crate's package name is paladin-ai-core; only paladin-battalion was modified so every test command ran as `-p paladin-battalion`"

requirements-completed: [FT-04]

coverage:
  - id: D1
    description: "Route { to, error_field } is validated (target declared, not a worker template; error_field declared, non-Sum) and Absorb { fallback_delta } against the schema (empty legal), every clause listing all offenders"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::route_error_field_must_be_declared_in_the_schema"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::route_error_field_must_not_use_sum_dispatch"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::route_target_must_be_a_declared_node"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::route_target_must_not_be_a_worker_template"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::absorb_fallback_delta_is_validated_against_the_schema"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::absorb_with_an_empty_fallback_delta_validates"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::every_handler_validation_error_lists_all_offenders"
        status: pass
    human_judgment: false
  - id: D2
    description: "A Route target is auto-eligible through validate_eligible_set's fixed-point insertion point with no mark_dynamic_target call"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::a_route_target_reachable_only_by_routing_is_not_stranded"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::a_route_target_reached_late_has_its_own_edges_expanded"
        status: pass
    human_judgment: false
  - id: D3
    description: "Route writes the structured NodeError JSON into error_field (compared as parsed fields) and places `to` in the next Vanguard replacing the failed node's static successors; the run Completes; a terminal target completes normally"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::route_writes_the_structured_error_and_places_the_target"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::route_replaces_the_failed_nodes_static_successors"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_route_target_with_no_outgoing_edges_completes_the_run_normally"
        status: pass
    human_judgment: false
  - id: D4
    description: "Absorb records outcome: Failed, merges the fallback delta (or nothing for an empty one) and fires static edges as on success"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::absorb_merges_its_delta_and_fires_static_edges"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::absorb_with_an_empty_fallback_delta_merges_nothing_and_continues"
        status: pass
    human_judgment: false
  - id: D5
    description: "No handler: Failed Waypoint with node_error: Some and RunOutcome::Failed carrying the same NodeError"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::no_handler_fails_the_run_with_the_structured_error"
        status: pass
    human_judgment: false
  - id: D6
    description: "A handler runs only after retries exhaust (three attempts, one invocation) or immediately on a non-retryable error (one attempt)"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_does_not_run_while_retries_remain"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_non_retryable_error_reaches_the_handler_immediately"
        status: pass
    human_judgment: false
  - id: D7
    description: "A registered Custom handler receives (&NodeError, &Battlefield pre-failure snapshot) and its Directive is honoured -- Edges merges + fires static edges, Goto places its target, End completes -- while Err fails the run with the handler's own error"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_receives_the_structured_error_and_the_battlefield"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_edges_contributes_its_delta"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_goto_places_its_target"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_end_completes_the_run"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_custom_handler_returning_err_fails_the_run_with_that_error"
        status: pass
    human_judgment: false
  - id: D8
    description: "Handler-routed visits count against max_node_visits through the one existing counter; an A->B->A compensation cycle terminates with NodeVisitLimitExceeded under a timeout guard"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_routed_visit_counts_against_max_node_visits"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_compensation_cycle_terminates_with_the_visit_limit"
        status: pass
    human_judgment: false

patterns-established:
  - "Convert-then-reuse: a compensated failure becomes NodeRunOutcome::Succeeded(handler directive) plus a `handled_failure` flag so every NextStep arm is honoured by the code that already honours it; the only handler-specific line in the Succeeded arm is the record's outcome-kind override"
  - "A validation clause on the aegis sidecar reads the RESOLVED aegis_for value (what will run), aggregates offenders in node_order, and returns the shallowest fault class first"

# Metrics
duration: ~28 min
completed: 2026-09-06
status: complete
---

# Phase 25 Plan 10: Route, Absorb and Custom error handlers Summary

Saga-style compensation is now expressible: a node's final failure (retries exhausted, or refused
by the predicate) is dispatched through its `Aegis.on_error` -- `Route` writes the structured
`NodeError` JSON into a declared, non-`Sum` field and places the recovery node in the next Vanguard
through the existing Goto machinery (replacing the failed node's static successors); `Absorb` merges
a schema-validated fallback delta and fires static edges as on success; a registered `Custom` handler
is awaited over the pre-superstep Battlefield and its `Directive` honoured like a node's own; with no
handler the run fails carrying the structured error. Every handler fault is a typed validation error
listing all offenders before any node runs, a Route target is auto-eligible with no
`mark_dynamic_target`, and a compensation cycle terminates through the one `max_node_visits` bound.

## What was built

### Task 1 -- validation clauses and Route-target eligibility (`graph.rs`, `mod.rs`)

- Five `EngineError` variants (`RouteTargetUnknown`, `RouteTargetIsWorkerTemplate`,
  `RouteErrorFieldUndeclared`, `RouteErrorFieldDispatchInvalid`, `AbsorbDeltaSchemaInvalid`), each
  `{ offenders: Vec<String>, reason: String }` mirroring `AegisUnsupportedForNodeKind`. The `Sum`
  message says why (a serialized error object cannot be summed); the worker-template message names the
  alternative (handle it at the aggregator, or `Absorb` on the template).
- `WarGraph::validate_aegis_handler_wiring`, called between the node-kind matrix and the policy-value
  clauses. Collects every offender per class over each node's resolved `aegis_for`, sorted
  deterministically (an Absorb delta's `HashMap` keys are sorted before reporting).
- `validate_eligible_set` pushes the popped node's `Route { to }` target onto the worklist -- the
  fixed-point insertion point 22-15 documented -- so a target reached late still has its own edges
  expanded. `unschedulable_unfed_nodes` exempts Route targets exactly like dynamic targets.

### Task 2 -- Route / Absorb / no-handler dispatch (`superstep.rs`, `test_support.rs`)

- `dispatch_error_handler(spec, err, state, registries) -> Result<Directive, NodeError>`.
- In the sequential bookkeeping loop, before the merge: a `Failed` outcome carrying a structured
  `node_error` whose node resolves an `on_error` (and is not an `EngineRun` budget cut) is handed to
  the handler; `Ok(directive)` becomes `Succeeded(directive)` with `handled_failure = true`, so the
  existing `Succeeded` arm does the routing and the record's outcome is overridden to `Failed`.
  `Err(handler_error)` replaces `node_error` and falls through to 25-07's `NodeFailed` path.
- Doubles: `PermanentlyFailingNode` (run counter), `RecoveryNode` (ran flag + observed snapshots),
  `RecordingErrorHandler` (invocation count, `(NodeError, Battlefield)` log, closure reply).

### Task 3 -- Custom dispatch and the loop bound (`superstep.rs`)

- `ErrorHandlerSpec::Custom(name)` resolves in `registries.error_handlers` and awaits
  `handle(err, state)`; a registry miss (unreachable after 25-03's validation) re-fails with the
  original error rather than panicking.
- Routed visits need no new code: a Route/Goto target enters `next_vanguard` and is counted by the
  top-of-superstep visit check. `a_compensation_cycle_terminates_with_the_visit_limit` proves the
  A->B->A loop ends `NodeVisitLimitExceeded { limit: 3 }` within five supersteps under a 10 s
  `tokio::time::timeout` guard.

## Verification (all exit 0)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo test -p paladin-battalion --lib` | 677 passed, 0 failed |
| `cargo test --doc -p paladin-battalion` | 48 passed, 52 ignored |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

No integration test binary (root `tests/` or crate `tests/`) was touched; `superstep::run` is
`pub(crate)`, so the signature change is invisible outside `paladin-battalion`.

## Deviations from Plan

### Interpretations (no code deviation)

**1. `Custom` arm between Task 2 and Task 3.** The plan asked for a compile error on the arm's
absence. `ErrorHandlerSpec` is `#[non_exhaustive]` in `paladin-core`, so a match in this crate must
carry a wildcard and cannot fail compilation on a missing arm. The committed intermediate state
(bd89b6f8) instead re-failed explicitly with the original error -- the run ends `NodeFailed`, never an
`Absorb`-shaped fallthrough -- and Task 3's RED tests genuinely failed against it.

**2. `grep -c max_node_visits ... -le 3`.** Unsatisfiable before this plan (9 pre-existing hits).
Production references remain exactly the 3 pre-existing ones; the 4 new hits are the plan-mandated
test name `a_handler_routed_visit_counts_against_max_node_visits`, its doc, a section comment and the
`EngineLimits { max_node_visits: 3 }` fixture. No second counter or exemption was introduced.

**3. Task 2 tests 7/8 use `Route` as the observed handler.** The plan's "recording handler with an
invocation counter" for the FT-FR-05 tests would need `Custom` dispatch (Task 3). Task 2 asserts the
same facts through the routed recovery node's run count plus the port's call count and the written
`attempt`; Task 3's `RecordingErrorHandler` tests then assert the handler-level invocation count
directly.

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Error-handler registry did not reach the dispatch loop**
- **Found during:** Task 2
- **Issue:** `superstep::run`/`run_with_namespace`/`ChildEngineResources` carried only
  `&EdgeEvaluatorRegistry`; `Custom` dispatch needs `ErrorHandlerRegistry`.
- **Fix:** the parameter became `registries: &EngineRegistries` (a Battalion child now inherits all
  three registries wholesale, D-13). Mechanical, no behavior change; 662 tests green after it.
- **Files modified:** `superstep.rs`, `mod.rs` (4 call sites), `graph.rs` (1 test call site)
- **Commit:** 00b06047

### Pre-passing RED tests (pins, not new behavior)

- `no_handler_fails_the_run_with_the_structured_error` passed at RED: plan 25-07 built that path;
  the test pins it as FT-FR-14 requires.
- `a_handler_routed_visit_counts_against_max_node_visits` and
  `a_compensation_cycle_terminates_with_the_visit_limit` passed at Task 3 RED: Task 2's Goto reuse
  already routed every handler-placed target through the one visit counter -- exactly the
  "no second counter" outcome the plan demands, now pinned.

## Known Stubs

None. The `Custom` registry-miss arm and the `serde_json::to_value` error arm are defensive re-fails
on invariants validation already enforces, not placeholders.

## Threat Flags

None beyond the plan's register. `error_field` carries the already-redacted, bounded `NodeError`
(T-25-45); the `Sum` rejection (T-25-46), visit bound (T-25-47), exhaustion-only entry (T-25-48),
target-kind rejection (T-25-49) and pre-superstep snapshot (T-25-50) are each pinned by the tests
named in `coverage`. No package was installed.

## Notes for later plans

- **25-11 (D-22, worker templates):** `validate_aegis_handler_wiring` already rejects a `Route`
  whose *target* is a worker template; a `Route` *on* a worker template and a `Custom` returning a
  non-`Edges` `NextStep` on one still need `HandlerNotAllowedOnWorkerTemplate` /
  `MusterHandlerMustBeDeltaOnly`. At runtime a muster task's handled `Edges` delta already lands in
  `muster_completed_so_far` because the conversion happens before the `is_muster_task` branch.
- **25-11 (D-23, Parley from a handler):** the `Succeeded` arm's `Parley` branch is reached
  unchanged; the record reads `Failed` rather than `Parleyed` for a handler-raised parley.
- **25-12 (E2E-3):** the CONTEXT.md compensation-chain shape is reproduced verbatim by
  `route_writes_the_structured_error_and_places_the_target` (`node_id: "book"`, `attempt: 1`,
  `transience: "Permanent"`, `source.Paladin.message`).

## Commits

| Commit | Message |
|---|---|
| 2151c370 | test(25-10): add failing Route/Absorb validation tests |
| acfbbf8c | feat(25-10): Route/Absorb validation clauses and Route-target eligibility seeding |
| 00b06047 | refactor(25-10): thread the whole EngineRegistries bundle into superstep::run |
| beb7056b | test(25-10): add failing Route/Absorb/no-handler dispatch tests |
| bd89b6f8 | feat(25-10): dispatch Route and Absorb handlers after retries exhaust |
| 54d321ab | test(25-10): add failing Custom handler dispatch and compensation-cycle bound tests |
| e63e43e3 | feat(25-10): dispatch registered Custom error handlers |

## TDD Gate Compliance

Each task has a `test(...)` RED commit followed by a `feat(...)` GREEN commit (see table). No
`refactor` step was needed after GREEN; the one `refactor(...)` commit precedes Task 2's RED and is a
signature change with no behavior change.

## Self-Check: PASSED
