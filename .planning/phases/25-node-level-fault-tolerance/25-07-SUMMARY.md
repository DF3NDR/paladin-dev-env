---
phase: 25-node-level-fault-tolerance
plan: 07
subsystem: infra
tags: [retry, aegis, waypoint, trace-sink, node-error, muster, superstep-engine, serde]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-01)
    provides: "NodeError/NodeErrorSource/AttemptRecord value types, the Aegis sidecar on WarGraph, and the retry loop wrapping superstep.rs's per-node dispatch closure"
  - phase: 25-node-level-fault-tolerance (plan 25-02)
    provides: "PaladinError::transience() and PaladinError::LlmFailure { transience, status, provider, message }; BattalionError::Node(NodeError)"
  - phase: 25-node-level-fault-tolerance (plan 25-03)
    provides: "WarGraph::validate's Aegis clauses (a set_aegis on a worker template validates as a declared node)"
  - phase: 25-node-level-fault-tolerance (plan 25-06)
    provides: "llm_failure::to_paladin_error, the one LlmError -> LlmFailure conversion this plan's NodeErrorSource conversion sits beside"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "Muster dispatch (one spawned future per task, MusterContext { payload, task_key }), intra-superstep muster-progress Waypoints and the MusterProgress stored contract (D-14)"
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "NextStep::Parley suspension, resume_with + ctx.parley_response(), the grace-deadline abort path and Halted-vanguard re-listing this plan's Interrupted outcome reuses"
provides:
  - "NodeExecutionRecord.attempts: Vec<AttemptRecord> (#[serde(default)], FAILED attempts only, ascending) and NodeExecutionRecord.cache_hit: bool (#[serde(default)], false until plan 25-13)"
  - "TraceEvent::NodeStarted { attempt } and NodeFinished { attempt, cache_hit }, emitted once per attempt (PRD 07 span-per-attempt field names)"
  - "WaypointStatus::Failed { error, failed_node, node_error: Option<NodeError> } -- additive #[serde(default)], no reshape, display line unchanged"
  - "EngineError::NodeFailed(NodeError) rendering the legacy `node execution error: {message}` line; EngineError::node_error() and RunOutcome::node_error() accessors; impl From<EngineError> for BattalionError (NodeFailed -> BattalionError::Node)"
  - "NodeFailure::Paladin(PaladinError) + NodeFailure::node_error(): Function -> NodeErrorSource::Function/Unknown, PaladinPort failure -> NodeErrorSource::Paladin { kind: variant name } or ::Llm { status, provider } with typed transience (D-07)"
  - "paladin_battalion::llm_failure::to_node_error_source(&PaladinError) -> NodeErrorSource, beside to_paladin_error"
  - "NodeRunOutcome::Interrupted: a node cancelled mid-backoff is re-listed on the Halted vanguard (or its Muster round's preserved MusterProgress) so resume re-executes it from attempt 1"
  - "Per-task Muster retry pinned: each (worker, task_key) retries inside its own spawned future with its own attempt counter and AttemptRecord vector; siblings never wait or re-run"
  - "Three-backend contract cases waypoint_with_attempt_history_round_trips and failed_waypoint_with_node_error_round_trips (Some + absent-key)"
  - "Test doubles: FailingPaladinPort, MusterFailThenSucceedWorker (per-task_key fail counts, tokio-clock call log, save-count observer), AttemptObservingNode"
affects: [25-09-timeouts, 25-10-error-handlers, 25-11-error-handlers-continued, 25-12-e2e-replacements, 25-13-node-cache-engine-integration, 25-14-migration-doc, 28-observability-otel]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Additive #[serde(default)] fields on persisted Waypoint types (record fields, an existing enum variant's new field) with a strip-key round-trip test and a three-backend contract case, never a BATTLEFIELD_SCHEMA_VERSION bump (Phase 22.1 D-21..D-25 precedent)"
    - "A structured error variant whose Display is deliberately byte-identical to the stringly line it supersedes (EngineError::NodeFailed renders `node execution error: {message}`), so persisted display lines and message-parsing consumers see no change while the structure travels beside them"
    - "Live error kept in the failure enum until the engine boundary (NodeFailure::Paladin(PaladinError)) so typed transience/status/provider survive into NodeError instead of being erased one line early"
    - "Accessor methods (RunOutcome::node_error(), EngineError::node_error()) expose a new structured payload without adding a field that would break every existing `RunOutcome::Failed { error, waypoint }` pattern (28 exact-pattern sites in-tree)"
    - "Per-attempt observation through test doubles that read a RecordingWaypointStore's save_call_count() at each run and stamp tokio::time::Instant under a paused clock, so 'no Waypoint between attempts' and 'siblings do not wait' are asserted, not assumed"

key-files:
  created: []
  modified:
    - crates/paladin-core/src/platform/container/waypoint.rs
    - crates/paladin-core/src/platform/container/node_error.rs
    - crates/paladin-ports/src/output/trace_sink_port.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-storage/src/waypoint/contract_tests.rs
    - crates/paladin-storage/src/waypoint/in_memory.rs
    - crates/paladin-storage/src/waypoint/sqlite.rs
    - crates/paladin-storage/src/waypoint/postgres.rs

key-decisions:
  - "node_error on WaypointStatus::Failed (and EngineError::NodeFailed) is produced ONLY for a node with a resolved Aegis; a no-Aegis node's failure keeps the byte-identical pre-Phase-25 EngineError::Node path and node_error: None, exactly as the plan's pre_aegis_and_limit_failures_carry_none behaviour specifies and D-09's 'no policy, no change' truth requires"
  - "EngineError::NodeFailed renders `node execution error: {message}` (the source's own message) rather than NodeError's Display, so WaypointStatus::Failed.error is byte-identical to the line the same failure produced before -- pinned by the_display_line_on_a_failed_waypoint_is_unchanged for both Function and Paladin failures"
  - "RunOutcome::Failed exposes the NodeError through a RunOutcome::node_error() accessor (delegating to EngineError::node_error()) instead of a new struct field: 28 exact `RunOutcome::Failed { error, waypoint }` patterns exist in-tree and every one would break; the accessor satisfies 'exposes the same NodeError' with zero churn"
  - "EngineError::Node is NOT retired: it is the engine's generic internal-error variant at ~15 non-failure-path sites (join errors, missing child resources, vanguard lookup). Only the exhausted-failure path's arm was superseded; the plan's 'retain unconstructed' instruction has no applicable variant"
  - "From<EngineError> for BattalionError maps NodeFailed -> BattalionError::Node and every other variant -> BattalionError::CampaignError(Display) through a wildcard arm, because no prior EngineError -> BattalionError conversion existed anywhere and an exhaustive in-crate match would break the moment a sibling wave-3 plan (25-09 adds RunTimeoutExceeded etc.) lands a new variant at merge"
  - "to_node_error_source lives in paladin-battalion's llm_failure.rs (a free fn, since both types are foreign there) beside to_paladin_error as the plan asked; message = PaladinError's Display for both Paladin and Llm variants so the display line stays legacy-identical; the compiler-required wildcard yields kind \"Other\""
  - "Task 3 found the per-task Muster retry structure ALREADY in place (each muster task is its own dispatch entry with its own spawned future, attempt counter and AttemptRecord vector, and aegis_for(worker) resolves the template's own policy); the plan's 'move the retry loop into each task's future' reduced to pinning it with the eight tests and documenting the contract on the loop"
  - "The Postgres tier of both new contract cases is CI-only evidence: locally `store_or_skip` self-skips without Docker (the wrappers report ok in ~0ms); never claimed as locally passed"
  - "The paladin-core crate's package name is paladin-ai-core, so every plan-listed `cargo test -p paladin-core ...` was run as `-p paladin-ai-core`"

patterns-established:
  - "Retry interruption is a first-class NodeRunOutcome (Interrupted) recorded Skipped{shutdown} AND re-listed for resume, mirroring the grace-deadline abort path -- never an ordinary skip"
  - "A failure enum arm carries the live error until the boundary conversion (NodeFailure::Paladin), and the conversion is a method on the enum (NodeFailure::node_error) returning None for arms that are not node-execution failures"

requirements-completed: [FT-01, FT-02]

coverage:
  - id: D1
    description: "Failed attempts are recorded ascending on NodeExecutionRecord.attempts (failed only; attempt keeps the succeeding number), a first-time success has an empty list, and the fields are #[serde(default)] with BATTLEFIELD_SCHEMA_VERSION unchanged"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::failed_attempts_are_recorded_in_order"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::a_node_that_succeeds_first_time_records_an_empty_attempts_list"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/waypoint.rs#tests::{failed_attempts_are_recorded_in_order,record_round_trips_with_the_new_fields_and_without_them,battlefield_schema_version_is_unchanged}"
        status: pass
    human_judgment: false
  - id: D2
    description: "TraceEvent::NodeStarted/NodeFinished fire once per attempt carrying attempt; NodeFinished carries cache_hit, false everywhere with no cache configured"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::node_events_are_emitted_once_per_attempt_with_the_attempt_number"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::cache_hit_defaults_to_false_on_every_record_and_event"
        status: pass
    human_judgment: false
  - id: D3
    description: "A Waypoint carrying attempt history, and a Failed Waypoint with node_error Some and with the key absent, round-trip on InMemory and SQLite (Postgres wrapper present, CI-only)"
    requirement: FT-02
    verification:
      - kind: integration
        ref: "crates/paladin-storage/src/waypoint/contract_tests.rs#{waypoint_with_attempt_history_round_trips,failed_waypoint_with_node_error_round_trips} via in_memory::tests and sqlite::tests (cargo test -p paladin-storage --features sqlite,postgres --lib)"
        status: pass
      - kind: integration
        ref: "crates/paladin-storage/src/waypoint/postgres.rs#tests::{waypoint_with_attempt_history_round_trips,failed_waypoint_with_node_error_round_trips} -- Docker-gated postgres-integration CI job only; self-skipped locally"
        status: unknown
    human_judgment: false
  - id: D4
    description: "An Aegis-governed exhausted failure travels as a structured NodeError: WaypointStatus::Failed.node_error, EngineError::NodeFailed, RunOutcome::node_error() and BattalionError::Node all carry the identical value; the display line is unchanged; no-Aegis and engine-limit failures carry None"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::exhausted_retry_writes_a_failed_waypoint_carrying_the_structured_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::the_display_line_on_a_failed_waypoint_is_unchanged"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::pre_aegis_and_limit_failures_carry_none"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::run_outcome_failed_exposes_the_same_node_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::engine_error_node_failed_maps_to_battalion_error_node"
        status: pass
    human_judgment: false
  - id: D5
    description: "A PaladinPort failure becomes NodeErrorSource::Paladin { kind: variant name } and an underlying LlmFailure becomes NodeErrorSource::Llm carrying the typed status/provider and transience -- never Function; NodeError JSON has a stable declaration-ordered field layout"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::a_paladin_node_failure_becomes_node_error_source_paladin"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/llm_failure.rs#tests::{llm_failure_becomes_node_error_source_llm_with_its_typed_fields,non_llm_paladin_failures_become_node_error_source_paladin_named_by_variant}"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/node_error.rs#tests::node_error_json_field_order_is_stable"
        status: pass
    human_judgment: false
  - id: D6
    description: "Retries are per task inside a Muster: a failing task retries three times while every sibling runs once, siblings finish while the failing task is still in backoff (paused clock), and each task has its own attempt counter"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::one_mustered_task_retries_without_re_running_siblings"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::sibling_tasks_do_not_wait_for_a_retrying_task_to_finish"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::each_muster_task_has_its_own_attempt_counter"
        status: pass
    human_judgment: false
  - id: D7
    description: "No Waypoint is written between attempts, muster-progress Waypoints list only completed tasks with the MusterProgress shape unchanged, and a run interrupted mid-retry resumes by re-executing the node from attempt 1"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::no_waypoint_is_written_between_attempts"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::muster_progress_waypoints_record_only_completed_tasks"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::resume_re_executes_an_interrupted_node_from_attempt_one"
        status: pass
      - kind: other
        ref: "git diff 2a266e61 -- crates/paladin-core/src/platform/container/waypoint.rs | grep -c MusterProgress == 0"
        status: pass
    human_judgment: false
  - id: D8
    description: "A NextStep::Parley Directive is a success that is never retried (call count 1, no retry budget consumed) and the post-resume re-run of a parleying node starts at attempt 1"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::a_parley_directive_is_never_retried"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::post_resume_rerun_of_a_parleying_node_starts_at_attempt_one"
        status: pass
    human_judgment: false

# Metrics
duration: 57min
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 07: Attempt History, the Structured Failure Path and Per-Task Muster Retry Summary

**Every failed attempt now lands as an ordered `AttemptRecord` on the execution record with per-attempt `NodeStarted`/`NodeFinished` events; an Aegis-governed node's exhausted failure travels as one structured `NodeError` through `WaypointStatus::Failed.node_error`, `EngineError::NodeFailed`, `RunOutcome::node_error()` and `BattalionError::Node` without changing the persisted display line; Paladin failures classify by their typed `PaladinError::transience()` into `NodeErrorSource::Paladin`/`Llm`; and per-task Muster retry, the no-Waypoint-between-attempts rule and the Parley-is-a-success rule are pinned by fourteen engine tests -- one of which exposed and fixed a latent bug where a node cancelled mid-backoff was silently dropped instead of re-listed for resume.**

## Performance

- **Duration:** ~57 min
- **Started:** 2026-09-05T21:45:00Z (approx., worktree spawn; first commit 22:14:35Z)
- **Completed:** 2026-09-05T22:41:35Z
- **Tasks:** 3 (all `auto`, each RED then GREEN)
- **Files modified:** 11 (0 created)

## Accomplishments

- `NodeExecutionRecord` gained `attempts: Vec<AttemptRecord>` (failed attempts only, ascending by construction and `debug_assert`ed at the record site) and `cache_hit: bool`, both `#[serde(default)]`; `attempt`'s placeholder rustdoc was replaced with the real contract (the succeeding attempt for a success, the exhausted attempt for a failure, `1` on a resume). `BATTLEFIELD_SCHEMA_VERSION` is unchanged (`1.0.0`, pinned by `battlefield_schema_version_is_unchanged`), following the `visit_counts`/`frontier`/`fork_of` additive precedent.
- `TraceEvent::NodeStarted`/`NodeFinished` gained `attempt: u32` and `NodeFinished` gained `cache_hit: bool`, using PRD 07 §2's names exactly (no other PRD 07 field added); they were already emitted inside the attempt loop, so the four-event `1,1,2,2` order is pinned rather than re-plumbed.
- `WaypointStatus::Failed` gained `#[serde(default)] node_error: Option<NodeError>` beside the untouched `error`/`failed_node` -- no reshape; an absent-key (pre-D-08) payload is proven to deserialise and round-trip on every backend.
- `EngineError::NodeFailed(NodeError)` supersedes the generic `EngineError::Node` arm on the exhausted-failure path for Aegis-governed nodes and renders the identical `node execution error: {message}` line; `RunOutcome::node_error()` exposes the same value; `impl From<EngineError> for BattalionError` maps it to `BattalionError::Node`.
- Plan 25-01's temporary "everything is `NodeErrorSource::Function`" mapping is replaced: `NodeFailure::Paladin(PaladinError)` keeps the live error to the boundary, and `NodeFailure::node_error()` classifies a Paladin failure by `PaladinError::transience()` into `Paladin { kind: variant name }` or, for `LlmFailure`, `Llm { status, provider }` via the new `llm_failure::to_node_error_source` beside `to_paladin_error`. `PaladinPort` failures are now retry-eligible with their real transience.
- Per-task Muster retry (D-17, FT-FR-06) is proven end to end: the failing task ran three times while every sibling ran once, siblings completed while the failing task was still in its 500 ms/1000 ms backoffs on a paused clock, and each task's `attempt` counter and `AttemptRecord` history is its own.
- FT-FR-07 is asserted rather than assumed: every attempt of a node observes the same `save_call_count()`, progress Waypoints list only completed tasks (the retrying task first appears in the third), and a run interrupted mid-retry resumes by re-executing the node from attempt 1.
- `NextStep::Parley` leaves the retry loop as a success on the first execution and the post-resume re-run is a fresh attempt 1 (D-17, Phase 24 D-07/D-08 path unchanged).

## Task Commits

Each task was committed RED then GREEN:

1. **Task 1: AttemptRecord history and cache_hit on the record; attempt on the trace events** -- `f052d332` (test, RED: `failed_attempts_are_recorded_in_order` fails with an empty history) -> `098a86b6` (feat, GREEN)
2. **Task 2: The structured failure path -- Failed Waypoint, EngineError::NodeFailed, BattalionError::Node** -- `4bda070b` (test, RED: four engine tests fail with `node_error: None`) -> `875e20c8` (feat, GREEN)
3. **Task 3: Per-task retry inside a Muster, and the Waypoint and Parley interactions** -- `7ceffd49` (test, RED: `resume_re_executes_an_interrupted_node_from_attempt_one` fails) -> `bd20ada5` (fix, GREEN -- the Rule 1 bug below) -> `93dc8b30` (style: clippy `-D warnings` fixes across Tasks 2/3)

**Plan metadata:** this commit (`docs(25-07): ...`)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/waypoint.rs` -- `NodeExecutionRecord.attempts`/`cache_hit`, `WaypointStatus::Failed.node_error`, three record/schema tests
- `crates/paladin-core/src/platform/container/node_error.rs` -- `node_error_json_field_order_is_stable`
- `crates/paladin-ports/src/output/trace_sink_port.rs` -- `attempt`/`cache_hit` on the node events
- `crates/paladin-battalion/src/engine/superstep.rs` -- `NodeTaskOutput` struct (attempt history + `node_error`), `NodeFailure::Paladin` + `NodeFailure::node_error()`, `NodeRunOutcome::Interrupted`, the exhausted path's `NodeFailed` mapping, `node_error: None` at the six non-node-failure `Failed` sites
- `crates/paladin-battalion/src/engine/mod.rs` -- `EngineError::NodeFailed`, `node_failed_message`, `EngineError::node_error()`, `RunOutcome::node_error()`, `From<EngineError> for BattalionError`, 18 new tests
- `crates/paladin-battalion/src/engine/test_support.rs` -- `FailingPaladinPort`, `MusterFailThenSucceedWorker`/`WorkerCall`, `AttemptObservingNode`
- `crates/paladin-battalion/src/llm_failure.rs` -- `to_node_error_source`, `paladin_error_kind`, two tests
- `crates/paladin-storage/src/waypoint/contract_tests.rs` -- two new contract cases wired into `run_all`
- `crates/paladin-storage/src/waypoint/{in_memory,sqlite,postgres}.rs` -- per-backend wrappers for both cases

## Decisions Made

See `key-decisions` in the frontmatter. In brief: `node_error` is `Some` only for Aegis-governed failures (the plan's own `pre_aegis_and_limit_failures_carry_none` behaviour); `NodeFailed`'s Display is the legacy line by design; `RunOutcome::node_error()` is an accessor, not a breaking field; `EngineError::Node` stays as the generic internal-error variant; the `BattalionError` conversion uses a merge-safe wildcard; the per-task Muster retry structure already existed and was pinned rather than rebuilt; the Postgres contract tier is CI-only evidence; the core crate's package name is `paladin-ai-core`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A node cancelled mid-backoff was dropped from the resume path**
- **Found during:** Task 3 (`resume_re_executes_an_interrupted_node_from_attempt_one`, RED run)
- **Issue:** Plan 25-01's backoff-cancellation branch broke out of the retry loop with `NodeRunOutcome::Skipped("shutdown")` "exactly like the grace-race abort path" -- but the abort path also pushes the node onto `aborted_node_ids` (which drives the Halted Waypoint's re-listed vanguard) and this branch did not. The interrupted node was recorded as an ordinary skip, its edges stayed `Pending`, the run went on to report `Completed` with an empty Battlefield, and a resume never re-ran it.
- **Fix:** Added `NodeRunOutcome::Interrupted`; the bookkeeping loop records it `Skipped { reason: "shutdown" }` (unchanged record shape) AND re-lists it -- `aborted_node_ids.push` for a vanguard node, `muster_task_aborted = true` (preserving the round's `MusterProgress`) for a Muster task -- so the run halts and `resume` re-executes the node from attempt 1.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
- **Verification:** the RED test now passes; `cargo test --workspace --lib` (all 636 battalion lib tests) green
- **Committed in:** `bd20ada5`

**2. [Rule 1 - Bug] `clippy -D warnings`: collapsible let-chain and a split rustdoc**
- **Found during:** post-Task-3 verification (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
- **Issue:** the GREEN retry block nested `if retry::should_retry(..)` inside the let-chain (`clippy::collapsible_if`), and my section-insertion anchor in `contract_tests.rs` landed inside `run_all`'s multi-line rustdoc, leaving two orphaned `///` lines (`clippy::empty_line_after_doc_comments`).
- **Fix:** folded `should_retry` into the let-chain; restored `run_all`'s doc comment as one block above the function.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`, `crates/paladin-storage/src/waypoint/contract_tests.rs`
- **Verification:** clippy exit 0, `cargo fmt --all -- --check` exit 0
- **Committed in:** `93dc8b30`

---

**Total deviations:** 2 auto-fixed (2 bugs, one of them latent in plan 25-01's code and exposed by this plan's own Test 6). No architectural changes, no scope creep.

**Plan-instruction interpretations (not deviations, recorded for the verifier):**
- "Retire any stringly node-failure arm on that path, retaining the variant unconstructed": `EngineError::Node` is the engine's generic internal-error variant at many non-failure-path sites, so only its exhausted-failure-path *arm* was superseded; the variant stays constructed elsewhere and its rustdoc on `NodeFailed` records what superseded what.
- "Extend `RunOutcome::Failed` to expose the same `NodeError`": done via `RunOutcome::node_error()` rather than a new field (28 exact-pattern match sites would break; see key-decisions).
- Task 3's "move the retry loop into each Muster task's own spawned future": the structure was already there (each Muster task is its own `dispatch_entries` entry with its own spawned future and loop-local counters); the task's value is the eight pinning tests, the Rule 1 fix, and the FT-FR-07 contract documented on the loop.

## Issues Encountered

- The plan's `cargo test -p paladin-core ...` commands do not resolve: the crate's package name is `paladin-ai-core`. All such commands were run with the real package name.
- `RunOutcome::Failed` cannot grow a field without touching 28 exact-pattern sites; solved with accessors (above).

## Known Stubs

- `cache_hit: false` at every `NodeExecutionRecord` construction site and every `NodeFinished` emit in `superstep.rs`. This is not a placeholder: no node cache is wired into the engine yet, so `false` is the correct value. Plan 25-13 (node-cache engine integration) is the only plan that sets it `true`, as the plan text specifies. Not recorded in `.planning/WINDOWS.md` (a shared ledger; appending from a parallel worktree risks a merge conflict, and this is a plan-mandated default rather than a defect) -- the orchestrator may add an entry if it prefers to track it there.

## Verification (this worktree, before returning)

| Command | Exit |
|---|---|
| `cargo check --workspace --all-targets --all-features` | 0 |
| `cargo test --workspace --lib` (paladin-ai-core 480, paladin-ports 43/119, paladin-battalion 636, paladin-storage 96, facade 562, others) | 0 |
| `cargo test -p paladin-storage --features sqlite,postgres --lib` (159; Postgres tier self-skipped) | 0 |
| `cargo test --doc -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-storage` | 0 |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| Integration binaries touched | none (no file under `tests/` or a crate's `tests/` was modified) |

Acceptance greps: `serde(default)` precedes `pub attempts: Vec<AttemptRecord>` (pass); `attempt: u32` x2 and `cache_hit: bool` x1 in `trace_sink_port.rs`; no `EdgeEvaluated|SupersetStarted|token_usage` in `trace_sink_port.rs` (pass); `BATTLEFIELD_SCHEMA_VERSION` diff count 0; `Failed {` declares `failed_node` within 4 lines (pass); `NodeFailed(NodeError)` and `impl From<EngineError> for BattalionError` present in `engine/mod.rs`; `MusterProgress` diff count in `waypoint.rs` across the plan 0. No `unwrap()`/`expect()`/`panic!` added to library code (every new hit is inside a `#[cfg(test)]` function).

## User Setup Required

None -- no external service configuration required.

## Next Phase Readiness

- Plan 25-09 (timeouts) can populate `NodeErrorSource::Timeout(kind)` through `NodeFailure::node_error` / the same `NodeFailed` path; `node_failed_message` already renders message-less sources through their `Display`.
- Plans 25-10/25-11 (error handlers) receive a fully classified `NodeError` (Paladin/Llm/Function with real transience) at the boundary, and `RunOutcome::node_error()` / `BattalionError::Node` are the surfaces to route on.
- Plan 25-13 flips `cache_hit` to `true` at the served-from-cache site; nothing else changes shape.
- Plan 25-12's E2E-3 replacement can assert `attempts` ordering, the per-attempt trace events and the structured `Failed` payload directly.
- Phase 28 (OTel) renames nothing on the node events.
- Shared artifacts (`STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md`, `WINDOWS.md`) were deliberately not modified from this worktree; the orchestrator owns those writes after the wave merges.

## Self-Check: PASSED

- FOUND: `.planning/phases/25-node-level-fault-tolerance/25-07-SUMMARY.md`
- FOUND commits: `f052d332`, `098a86b6`, `4bda070b`, `875e20c8`, `7ceffd49`, `bd20ada5`, `93dc8b30`, `8bbb89f9`
- `git status --short` clean after the SUMMARY commit (no untracked or generated files left behind)

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-05*
