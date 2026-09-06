---
phase: 25-node-level-fault-tolerance
plan: 11
subsystem: infra
tags: [aegis, error-handler, muster, worker-template, parley, hitl, compensation, saga, superstep-engine, e2e, fault-tolerance]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-10)
    provides: "Route/Absorb/Custom dispatch (convert-then-reuse: a handled failure becomes Succeeded(directive) + handled_failure=true), validate_aegis_handler_wiring, RecordingErrorHandler/PermanentlyFailingNode/RecoveryNode doubles, and the one visit_counts bound routed visits share"
  - phase: 25-node-level-fault-tolerance (plan 25-07)
    provides: "Structured NodeError on a final Aegis-governed failure, WaypointStatus::Failed.node_error, RunOutcome::node_error(), per-task Muster retry and the MusterFailThenSucceedWorker double"
  - phase: 25-node-level-fault-tolerance (plan 25-06)
    provides: "llm_failure::to_paladin_error / to_node_error_source -- the typed AuthenticationError -> Permanent LlmFailure -> NodeErrorSource::Llm chain the E2E compensation chain rides"
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "NextStep::Parley suspension (one AwaitingInput Waypoint, vanguard = parleying nodes), WarEngine::resume_with and ctx.parley_response() on the post-resume re-run (D-07/D-08)"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "Muster dispatch, muster_completed_so_far, the task_key-ordered fold and the routing_failure path an unknown Goto target takes"
provides:
  - "EngineError::HandlerNotAllowedOnWorkerTemplate { offenders, reason } -- Route on a worker template is rejected at validation, every offender listed, message naming the aggregator alternative and WHY (aggregation semantics for a routed task are undefined; routing out of one task is deferred) (D-22)"
  - "EngineError::MusterHandlerMustBeDeltaOnly { node, task_key, returned } -- a Custom handler returning Goto/End/Parley/Muster from inside a mustered task fails the run before any routing side effect, naming the template, the task key and the offending arm (D-22)"
  - "A permitted worker-template handler's delta (Absorb fallback or a delta-only Custom Edges) is that task's aggregation contribution in the same shape as a successful sibling's; the aggregator sees the full task count and the record still reads Failed"
  - "Handler -> Parley composition pinned (D-23): one AwaitingInput Waypoint through the existing HITL-01 path, RunOutcome::AwaitingInput, fresh attempt 1 with ctx.parley_response() set on resume, no retry budget consumed, peers merge normally, rejected inside a Muster"
  - "tests/integration/e2e_compensation_chain_test.rs ([[test]] e2e_compensation_chain): PRD 04 section 3.5's compensation chain and loop bound, plus 'on payment failure, parley a human' across a simulated process drop and the handler-less structured-error mirror -- all over a real on-disk SqliteWaypointStore"
  - "Test doubles: RecordingErrorHandler::parleying, ParleyObservingNode (engine/test_support.rs)"
affects: [25-12-e2e-replacements, 25-14-migration-doc]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Delta-only enforcement sits at the TOP of the Succeeded arm, keyed on `is_muster_task && handled_failure && !Edges`, BEFORE notfiring_nodes/goto_targets/parley_requests/mustered are touched, and reports through the existing routing_failure path -- no second failure path, no partially applied routing"
    - "The template-placement clause is the shallowest handler-wiring class: collected first in node_order and returned before RouteTargetUnknown, so a Route that is both ON a template and pointing AT one reports the placement fault"
    - "A handler-raised Parley needs no code of its own: 25-10's convert-then-reuse already routes it through the node-raised Parley arm; this plan pins the contract (one AwaitingInput site in production, still) rather than adding a path"
    - "E2E fixtures classify by the real typed table (llm_failure::to_paladin_error(&LlmError::AuthenticationError(..))) rather than hand-building an LlmFailure, so the test proves the classification chain, not a fixture's assumption about it"

key-files:
  created:
    - tests/integration/e2e_compensation_chain_test.rs
  modified:
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - Cargo.toml

key-decisions:
  - "A handler-raised Parley keeps NodeOutcomeKind::Failed on the node's record (the follow-up 25-10 handed over), not Parleyed: the node DID fail and the handler asked -- consistent with D-21 for every other handler Directive; the suspension itself is fully visible on WaypointStatus::AwaitingInput/vanguard, and nothing in resume_with reads the record kind (grep: the only production Parleyed reader is the raise site itself)"
  - "MusterHandlerMustBeDeltaOnly carries a third field `returned: String` (the arm name) beyond the plan's { node, task_key }: with up to max_muster_tasks concurrent tasks, naming WHICH control-flow arm the handler returned is what makes the failure actionable without re-running"
  - "The delta-only guard keys on handled_failure only (D-22 is a rule about handlers); a SUCCESSFUL mustered task returning Goto/End/Parley/Muster was found to have no guard anywhere and is left untouched -- an engine-semantics question for the orchestrator, not a Rule 1-3 fix (see attention item 2)"
  - "Task 2 needed no production change: every D-23 test passed at RED because 25-10's convert-then-reuse already routes a handler's Parley through the one existing suspension arm; the commit is a pin (test + rustdoc), and no feat(...) GREEN exists for it by construction"
  - "The plan's acceptance grep `WaypointStatus::AwaitingInput -le 3` was already unsatisfiable before this plan (5 hits: 1 production + 4 test); the intent -- exactly ONE production construction site -- holds and is what the SUMMARY reports (11 hits now: 1 production + 10 test)"
  - "E2E `book` asserts `source.Llm.kind == \"LlmFailure\"` and `status` null: LlmError::AuthenticationError is a message-only variant, so llm_failure::typed_origin fabricates no HTTP status for it; the authentication classification travels by value as transience: Permanent, which is what CONTEXT.md's 'AuthenticationError-classified' shape actually means"

requirements-completed: [FT-04]

coverage:
  - id: D1
    description: "On a worker template only Absorb and Custom are allowed; Route is a validation error listing every offender and naming the aggregator alternative"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::route_on_a_worker_template_is_rejected_at_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::absorb_on_a_worker_template_validates"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::custom_on_a_worker_template_validates"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/graph.rs#tests::every_worker_template_route_offender_is_listed"
        status: pass
    human_judgment: false
  - id: D2
    description: "A Custom handler inside a Muster must be delta-only: Goto/End/Parley/Muster are MusterHandlerMustBeDeltaOnly naming node and task_key; an Edges delta or an Absorb fallback is the task's aggregation contribution and the task count is unchanged"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_worker_handler_returning_edges_contributes_its_delta_to_the_aggregation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_worker_handler_returning_goto_fails_the_run_with_a_typed_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_worker_handler_returning_end_or_parley_or_muster_is_the_same_typed_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::an_absorbed_worker_task_still_appears_in_the_aggregation"
        status: pass
    human_judgment: false
  - id: D3
    description: "A Custom handler may Parley through the existing HITL-01 path; the post-resume re-run is a fresh attempt 1 with ctx.parley_response() set; no retry budget consumed; peers merge normally; rejected inside a Muster"
    requirement: FT-04
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_raised_parley_suspends_the_run"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::the_post_resume_rerun_is_a_fresh_attempt_one"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_raised_parley_does_not_consume_the_retry_budget"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::the_suspending_supersteps_peers_merge_normally"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_handler_raised_parley_inside_a_muster_is_rejected"
        status: pass
    human_judgment: false
  - id: D4
    description: "PRD 04 section 3.5 end to end over a real backend: the compensation chain Completes with booking_error's parsed fields and book's record Failed; a permanent failure is not retried; the a/b loop ends NodeVisitLimitExceeded under a timeout guard; a handler parleys a human across a process drop; the handler-less mirror carries node_error: Some"
    requirement: FT-04
    verification:
      - kind: integration
        ref: "tests/integration/e2e_compensation_chain_test.rs#compensation_chain_routes_a_permanent_failure_to_a_recovery_node"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_compensation_chain_test.rs#a_permanent_failure_is_not_retried_before_routing"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_compensation_chain_test.rs#compensation_loop_terminates_at_the_visit_limit"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_compensation_chain_test.rs#on_payment_failure_parley_a_human"
        status: pass
      - kind: integration
        ref: "tests/integration/e2e_compensation_chain_test.rs#the_failed_waypoint_of_a_handler_less_run_carries_the_structured_error"
        status: pass
    human_judgment: false

patterns-established:
  - "Reject-before-side-effect: a Directive that is illegal in its dispatch context is turned into routing_failure at the top of the Succeeded arm, so no Frontier/goto/parley state is ever partially applied for it"
  - "Pin, don't re-implement: when a composition already works through an existing arm, the plan's tests are committed as pins with a rustdoc statement of the contract, and the SUMMARY says so instead of manufacturing a GREEN commit"

# Metrics
duration: ~30 min
completed: 2026-09-06
status: complete
---

# Phase 25 Plan 11: Worker-template handler restrictions, handler Parley and the compensation-chain E2E Summary

FT-04's composition story is closed: inside a Muster a handler may only contribute a delta -- `Route`
on a worker template is a typed validation error naming the aggregator alternative, and a `Custom`
handler returning `Goto`/`End`/`Parley`/`Muster` from a mustered task is
`EngineError::MusterHandlerMustBeDeltaOnly { node, task_key, returned }` before any routing side
effect, while a permitted delta is that task's aggregation contribution in the same shape as a
successful sibling's (D-22). A handler may ask a human: its `NextStep::Parley` rides the one
existing HITL-01 suspension path and the post-resume re-run is a fresh attempt 1 with
`ctx.parley_response()` set, having spent no retry budget (D-23). PRD 04 section 3.5's compensation
chain and loop bound now pass end to end over a real on-disk `SqliteWaypointStore`, alongside "on
payment failure, parley a human" across a simulated process drop.

## What was built

### Task 1 -- worker-template handler restrictions (`mod.rs`, `graph.rs`, `superstep.rs`)

- Two `EngineError` variants: `HandlerNotAllowedOnWorkerTemplate { offenders, reason }` (the
  message says only `Absorb` and a delta-only `Custom` are allowed, that aggregation semantics for a
  routed task are undefined today and routing out of one task is deferred, and to handle it at the
  aggregator) and `MusterHandlerMustBeDeltaOnly { node, task_key, returned }` (the display names the
  template, the task key, the offending `NextStep` arm and the alternative).
- `validate_aegis_handler_wiring` gains the shallowest class: any node registered via
  `add_worker_template` whose RESOLVED `on_error` is `Route` is collected in `node_order` and reported
  first -- ahead of `RouteTargetUnknown`/`RouteTargetIsWorkerTemplate` -- so a Route both on and
  pointing at a template reports the placement fault. `Absorb` and `Custom` validate unchanged
  (`Custom`'s delta-only property is a runtime fact). `add_worker_template`'s rustdoc states the rule.
- `superstep.rs`: at the top of the `Succeeded` arm, `is_muster_task && handled_failure &&
  !matches!(next, NextStep::Edges)` pushes the task's `Failed` record and sets `routing_failure`
  (the same path an unknown `Goto` target takes) and `continue`s -- so `notfiring_nodes`,
  `goto_targets`, `parley_requests` and `mustered` are never touched for it. A permitted `Edges` delta
  falls through into `muster_completed_so_far` exactly like a successful sibling's; the aggregation's
  ordering and `task_key` sort are untouched. A small `next_step_arm_name` helper feeds `returned`.

### Task 2 -- a Custom handler may Parley (`superstep.rs`, `test_support.rs`)

- No production change was needed: 25-10's convert-then-reuse already routes a handler's `Parley`
  through the node-raised `Parley` arm, so all five D-23 tests passed at RED (see Deviations).
  `dispatch_error_handler`'s rustdoc now states the D-23/D-22 contract explicitly.
- Doubles: `RecordingErrorHandler::parleying(request)` and `ParleyObservingNode` (fails until
  `ctx.parley_response()` is `Some`, or always; records `(attempt, response value)` per run).
- The tests drive a real `WarEngine` (`with_error_handler`) so `resume_with` is exercised: one
  `AwaitingInput` Waypoint whose `vanguard` is exactly the failed node; `observed_attempts ==
  [1, 2, 3]` before the ask and `[1, 2, 3, 1, 2, 3]` across a second exhaustion after resume (two
  handler invocations, `record.attempt == 3`, two `AttemptRecord`s); peers' deltas merged and their
  records `Succeeded`; inside a Muster the same handler is `MusterHandlerMustBeDeltaOnly { returned:
  "Parley" }` with no `AwaitingInput` Waypoint written.

### Task 3 -- the E2E binary (`tests/integration/e2e_compensation_chain_test.rs`, `Cargo.toml`)

Registered as `[[test]] name = "e2e_compensation_chain"`. Run it with
`cargo test --test e2e_compensation_chain` -- it is under `tests/integration/`, so the default
`make test` (`--lib --bins`) does not cover it.

- `book` is a `NodeSpec::paladin` whose port fails with
  `llm_failure::to_paladin_error(&LlmError::AuthenticationError(..))` -- the real typed
  classification (`Transience::Permanent`), never a hand-built error. Its `Aegis` carries a
  3-attempt `TransientOnly` retry policy and `Route { to: cancel, error_field: booking_error }`;
  `cancel` is declared but only reachable by routing. Asserted: `RunOutcome::Completed`;
  `booking_error`'s parsed `node_id == "book"`, `attempt == 1`, `transience ==
  to_value(Transience::Permanent)`, `source.Llm.kind == "LlmFailure"`, `status` null, message
  containing the provider's text, and a round-trip into `NodeError`; `book`'s record `Failed` at
  attempt 1 with an empty attempt history; `cancel`'s `Succeeded`; the latest Waypoint `Completed`;
  the port called exactly once.
- The loop: `a`/`b` always-failing Function nodes routing to each other, `max_node_visits = 3` --
  `NodeVisitLimitExceeded { limit: 3 }`, at most 5 runs and 6 Waypoints, under a 10 s
  `tokio::time::timeout` guard (the listener.rs convention); every other test carries a 30 s guard.
- `on_payment_failure_parley_a_human`: engine A (handler registered) suspends with the handler's
  request re-stamped to `payment`, whose payload carries the redacted error summary; A is dropped;
  engine B over the same file delivers `approval(true)` via `resume_with` -> `Completed`,
  `payment_status == "approved"`, `observed_attempts == [1, 1]`, handler B never invoked.
- The mirror with `on_error: None` (Aegis retained so the failure is Aegis-governed): `RunOutcome::
  Failed(NodeFailed(err))` and the `Failed` Waypoint's `node_error == Some(err)`, one Waypoint.

## Verification (all exit 0)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo test -p paladin-battalion --lib` | 690 passed, 0 failed |
| `cargo test --doc -p paladin-battalion` | 48 passed, 52 ignored |
| `cargo test --doc -p paladin-ai` (root package: `Cargo.toml` + `tests/` touched) | 115 passed, 17 ignored |
| `cargo test --test e2e_compensation_chain` | 5 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

Integration `[[test]]` binaries added or touched: `e2e_compensation_chain` (new). No crate-level
`tests/` binary was touched. Disk stayed at ~152 GB free throughout; no `--release`, no coverage.

## Deviations from Plan

### Interpretations (no code deviation)

**1. Task 2 had nothing to implement.** The plan asked to make the handler's `Parley` take the
existing HITL-01 path "rather than a second one", extracting if unreachable. It was already
reachable: 25-10 converts a handled failure into `Succeeded(directive)` before the `NextStep` match,
so the node-raised `Parley` arm handles it. All five tests passed at RED against the unchanged
engine; they are committed as pins (18f33495) with the contract written into
`dispatch_error_handler`'s rustdoc. There is deliberately no `feat(...)` commit for Task 2.

**2. `grep -c 'WaypointStatus::AwaitingInput' superstep.rs -le 3`.** Unsatisfiable before this plan
(5 hits: 1 production + 4 in Phase 24 tests). The intent -- exactly one production construction
site, no second suspension path -- holds: the count is now 11, of which 1 is production (the same
site, line 3041) and 10 are test assertions. Verified by
`grep -n ... | awk -F: '$1 < 3900'` (the tests module starts at 3841).

**3. Pre-passing RED tests (pins).** In Task 1, `absorb_on_a_worker_template_validates`,
`custom_on_a_worker_template_validates`, `a_worker_handler_returning_edges_contributes_its_delta_to_the_aggregation`
and `an_absorbed_worker_task_still_appears_in_the_aggregation` passed at RED -- 25-10 noted a handled
`Edges` delta already lands in `muster_completed_so_far`; they pin that. The four that failed at
RED (`route_on_a_worker_template_is_rejected_at_validation`,
`every_worker_template_route_offender_is_listed`, the `Goto` and the `End/Parley/Muster` typed-error
tests) are what 001ba939 made pass. In Task 3 all five E2E tests passed on first green compile after
two fixture corrections (below); they are acceptance pins over the real backend, as E2E tests are.

**4. `MusterHandlerMustBeDeltaOnly` has a third field.** `returned: String` (the arm name) was added
to the plan's `{ node, task_key }` so the error is actionable in a wide fan-out; tests match with
`..`-free full patterns, so the shape is pinned.

**5. Task 2 tests live in `superstep.rs`, driving `WarEngine`.** The plan's file list names
`superstep.rs`; `resume_with` only exists on `WarEngine`, so the tests construct one over the
`RecordingWaypointStore` double (already a `WaypointPort`) rather than moving to `mod.rs`.

### Test-fixture corrections (not engine deviations)

- The Task 2 tests first submitted `"approved"` as an `Approval` response value; `resume_with`
  correctly rejected it (`ResponseShapeInvalid`: an Approval must be bool-shaped). Fixed to `true`.
- The E2E chain test first asserted `source.Llm.status == 401`; `LlmError::AuthenticationError` is
  message-only, so `typed_origin` yields no status. Fixed to assert `kind == "LlmFailure"` and a
  null `status`, with the classification asserted by value (`transience: Permanent`).

### Auto-fixed Issues

None.

## Known Stubs

None. Every new node/handler/port double is test-only and fully wired; no placeholder values flow
anywhere.

## Threat Flags

None beyond the plan's register. T-25-51 (a mustered task escaping its aggregation) is pinned by
the four delta-only tests; T-25-52 by `compensation_loop_terminates_at_the_visit_limit` under its
guard; T-25-53 by the E2E handler placing only the redacted `NodeErrorSource` display string in the
parley payload; T-25-54 by the single production `AwaitingInput` site; T-25-55 by
`a_handler_raised_parley_does_not_consume_the_retry_budget`. No package was installed
(`Cargo.toml` gained only a `[[test]]` entry).

## Things worth the orchestrator's attention

1. **25-10's two follow-ups are resolved here.** A handler-raised Parley records `Failed` (decision
   above, deliberately unchanged); `Route` ON a worker template is now
   `HandlerNotAllowedOnWorkerTemplate`.
2. **Pre-existing gap, out of scope, not fixed:** a *successful* mustered task (no handler involved)
   returning `Goto`/`End`/`Parley`/`Muster` has no guard anywhere in production
   (`grep 'is_muster_task &&'` finds only this plan's `handled_failure`-keyed check). D-22 is a rule
   about handlers, so this plan did not widen it -- whether a worker task may legitimately `End` or
   `Goto` is an engine-semantics decision (Rule 4 territory), suggested for `deferred-items.md`.
3. **25-14 (docs):** the user-facing rule to document is "on a worker template: `Absorb`, or a
   `Custom` that returns `NextStep::Edges`; anything else is rejected -- handle it at the
   aggregator". The `MusterHandlerMustBeDeltaOnly` display already says this verbatim.
4. **25-12 (E2E replacements):** `e2e_compensation_chain` reproduces CONTEXT.md's exact chain shape
   over SQLite; if 25-12 needs the same `book`/`cancel` graph it can lift `compensation_graph` and
   `AuthFailingPaladinPort` from this file rather than re-deriving them.

## Commits

| Commit | Message |
|---|---|
| e08ab560 | test(25-11): add failing worker-template handler restriction tests |
| 001ba939 | feat(25-11): reject Route on a worker template and enforce delta-only muster handlers |
| 18f33495 | test(25-11): pin handler-raised Parley suspension through the existing HITL-01 path |
| 737ed63f | test(25-11): add the compensation-chain and loop-bound E2E tests |

## TDD Gate Compliance

Task 1: RED `test(...)` (e08ab560, four genuinely failing tests) then GREEN `feat(...)` (001ba939).
Task 2: RED passed against the unchanged engine, so only a `test(...)` pin commit exists (18f33495)
-- there was no behavior to add, and a synthetic GREEN would have been an empty commit. Task 3: the
E2E acceptance tests are pins over the real backend (737ed63f); the engine behavior they prove was
built in 25-07/25-10 and Task 1/2 above. No `refactor(...)` step was needed.

## Self-Check: PASSED

- Files: `tests/integration/e2e_compensation_chain_test.rs`, `Cargo.toml`,
  `crates/paladin-battalion/src/engine/{mod,graph,superstep,test_support}.rs` and this SUMMARY all
  exist on disk (`ls -1`).
- Commits: e08ab560, 001ba939, 18f33495, 737ed63f all present in `git log` on
  `worktree-agent-aaf620c2661b53354` (4 of 4 found).
- No file deletions across the plan's range (`git diff --diff-filter=D 2e891d81..HEAD` empty); no
  untracked files (`git status --short` empty).
