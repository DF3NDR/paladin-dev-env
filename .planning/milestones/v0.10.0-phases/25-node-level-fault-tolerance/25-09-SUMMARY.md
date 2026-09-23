---
phase: 25-node-level-fault-tolerance
plan: 09
subsystem: infra
tags: [tokio, watch, timeout, heartbeat, idle-timeout, run-timeout, superstep-engine, aegis, paladin-port]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-01)
    provides: "TimeoutKind/NodeErrorSource::Timeout value types, TimeoutPolicy on Aegis, the retry loop wrapping superstep.rs's per-node dispatch closure"
  - phase: 25-node-level-fault-tolerance (plan 25-03)
    provides: "WarGraph::validate rejecting Some(Duration::ZERO) on either TimeoutPolicy field (TimeoutPolicyInvalid)"
  - phase: 25-node-level-fault-tolerance (plan 25-07)
    provides: "NodeFailure::node_error, AttemptRecord history on NodeExecutionRecord, WaypointStatus::Failed.node_error, EngineError::NodeFailed"
  - phase: 25-node-level-fault-tolerance (plan 25-08)
    provides: "PaladinResult.served_by copy in PaladinExecutionService::execute_internal (kept working through the heartbeat threading)"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "EngineLimits.run_timeout declared-but-unenforced, the RecursionLimitExceeded/NodeVisitLimitExceeded Waypoint path this plan reuses"
provides:
  - "paladin_core::platform::container::heartbeat::HeartbeatHandle (Arc<watch::Sender<u64>>, manual PartialEq always-equal, opaque Debug; beat/beats/subscribe), re-exported at paladin_battalion::engine::heartbeat and paladin_battalion::engine::HeartbeatHandle"
  - "NodeContext.attempt: u32 and NodeContext.heartbeat: HeartbeatHandle, with attempt() and heartbeat() accessors; rebuilt per attempt with a fresh handle"
  - "PaladinPort::execute_observed(&self, paladin, input, heartbeat) DEFAULTED to execute (claims no progress); the engine always dispatches Paladin nodes through it"
  - "PaladinExecutionService::execute_observed / execute_stream_observed (inherent) beating on every LLM completion, every Armament invocation and every streamed chunk"
  - "NodeFailure::Timeout(TimeoutKind) -> NodeError { source: Timeout(kind), transience: Transient }; per-attempt AttemptBounds::resolve + race_attempt (run/idle/engine bounds), idle timer awaiting HeartbeatHandle::subscribe().changed()"
  - "EngineError::RunTimeoutExceeded { elapsed, limit } through the one shared persist_limit_failure helper also used by RecursionLimitExceeded and NodeVisitLimitExceeded"
  - "Per-attempt deadline = min(TimeoutPolicy.run_timeout, remaining EngineLimits.run_timeout), tightest named Run vs EngineRun; EngineRun never retried, ends the run RunTimeoutExceeded with the cut attempt's NodeError on the Failed Waypoint"
  - "run_with_namespace(.., parent_heartbeat: Option<HeartbeatHandle>): a Battalion child beats the parent node's handle once per child superstep"
  - "Test doubles: HeartbeatingNode, ObservedCallRecordingPort, BeatingPaladinPort, TimedFunctionNode (all paused-clock friendly)"
affects: [25-10-error-handlers, 25-11-error-handlers-continued, 25-12-e2e-replacements, 25-13-node-cache-engine-integration, 25-14-migration-doc-and-guide, 26-agent-runtime, 28-observability-otel]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Port value type owned by paladin-core (ADR-0016) and re-exported by the engine so paladin-ports and paladin-battalion share one HeartbeatHandle with no reverse dependency"
    - "tokio::sync::watch as a progress channel: the idle timer is tokio::time::timeout(idle, rx.changed()) in a loop -- awaited, never polled -- so start_paused tests are exact"
    - "Defaulted trait method whose default is a correct claim (no progress), not a stub (X-10.4): existing PaladinPort implementors compile unchanged"
    - "Per-attempt context rebuild (NodeContext { attempt, heartbeat: fresh, ..base }) so a cancelled attempt's stragglers can never reset the next attempt's timer"
    - "tokio::select! biased toward the attempt future, dropping it on expiry so partial work never becomes a Directive (T-25-41)"
    - "One limit-failure helper (persist_limit_failure) for every engine limit so the Waypoint/outcome path is consistent by construction"

key-files:
  created:
    - crates/paladin-core/src/platform/container/heartbeat.rs
    - crates/paladin-battalion/src/engine/heartbeat.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-core/src/platform/container/aegis.rs
    - crates/paladin-core/src/platform/container/node_error.rs
    - crates/paladin-ports/src/output/paladin_port.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/node.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - src/core/platform/mod.rs
    - src/config/engine.rs

key-decisions:
  - "HeartbeatHandle is a watch::Sender<u64> beat COUNTER (RESEARCH.md's recommended option), not an atomic timestamp: the idle timer awaits changed() instead of polling, is clock-agnostic, and beats() doubles as the test observation surface"
  - "HeartbeatHandle lives in paladin-core (ADR-0016 port value types) because PaladinPort in paladin-ports must name it and paladin-ports cannot depend on paladin-battalion; engine/heartbeat.rs is a pure re-export, never a duplicate"
  - "PaladinExecutionService does NOT implement PaladinPort (it implements PaladinExecutorPort + StreamingExecutorPort), so its D-19 progress reporting lands as inherent execute_observed / execute_stream_observed methods rather than a trait override; the unobserved execute/execute_stream paths are byte-identical (heartbeat: None)"
  - "A timeout's NodeError is structured even on a node with NO Aegis (D-09's no-policy-no-change rule has this one exception) because the ENGINE budget can cut a policy-less node and the fired kind must still be a typed TimeoutKind on the Waypoint"
  - "Timeout(EngineRun) is never retried: the budget is exhausted so a fresh attempt would be cut at once; the run ends RunTimeoutExceeded instead. Run/Idle ARE retried under the predicate like any transient failure"
  - "On an exact per-attempt deadline tie the node's own Run bound is named (the policy the author declared wins the label); ties are measure-zero in practice and never occur in the tests"
  - "The run budget is measured per run_with_namespace call: a resume restarts it; a Battalion child measures its OWN budget against its OWN EngineLimits (child_uses_its_own_engine_limits)"
  - "The engine-level boundary check sits right after the recursion-limit check and before the node-visit check; the mid-superstep EngineRun surfacing is checked ahead of node_failure so the budget, not an incidental sibling failure, names the outcome"

requirements-completed: [FT-03]

coverage:
  - id: D1
    description: "A port beating every 100 ms under idle_timeout 250 ms / run_timeout 10 s completes with one attempt and no timer fires"
    requirement: FT-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_port_beating_every_100ms_survives_a_250ms_idle_timeout"
        status: pass
    human_judgment: false
  - id: D2
    description: "The same policy over a port that stalls 300 ms fails Timeout(Idle), Transient, asserted by typed kind"
    requirement: FT-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_port_that_stalls_300ms_fails_with_timeout_idle"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::the_fired_bound_is_read_from_the_typed_kind"
        status: pass
    human_judgment: false
  - id: D3
    description: "A node that keeps beating is still cut by run_timeout and the failure names Run, not Idle; a timed-out attempt is retried as Transient and its partial delta never merges"
    requirement: FT-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_slow_but_progressing_node_fails_on_run_timeout_not_idle"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_timed_out_attempt_is_retried_as_transient"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::a_timed_out_attempts_partial_work_is_discarded"
        status: pass
    human_judgment: false
  - id: D4
    description: "EngineLimits.run_timeout is enforced (RunTimeoutExceeded, same Waypoint path as the other limits), nested with the attempt bound, tightest named, EngineRun recorded on the cut attempt; None means no bound; excluded from the fingerprint; bridges carry no legacy timeout"
    requirement: FT-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::{engine_run_timeout_ends_the_run_with_a_typed_error,an_attempt_cut_by_the_engine_bound_records_timeout_enginerun,the_tightest_bound_fires,no_engine_run_timeout_means_no_run_level_bound,run_timeout_is_not_hashed_into_the_fingerprint,bridges_carry_no_legacy_battalion_timeout}"
        status: pass
      - kind: unit
        ref: "src/config/engine.rs#config::engine::tests::config_seconds_convert_to_duration_exactly"
        status: pass
    human_judgment: false
  - id: D5
    description: "NodeContext keeps Debug+Clone+PartialEq with a HeartbeatHandle; heartbeat() without idle_timeout is a no-op; ctx.attempt is 2 on the second attempt; execute_observed defaults to execute with no beat; the engine always calls execute_observed; the service beats on LLM completion, stream chunk and Armament call"
    requirement: FT-03
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/node.rs#engine::node::tests::node_context_keeps_its_derives_with_a_heartbeat_handle"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#engine::superstep::tests::{heartbeat_is_a_no_op_without_an_idle_timeout,node_context_exposes_the_current_attempt,the_engine_always_calls_execute_observed}"
        status: pass
      - kind: unit
        ref: "crates/paladin-ports/src/output/paladin_port.rs#output::paladin_port::tests::execute_observed_defaults_to_execute"
        status: pass
      - kind: unit
        ref: "src/application/services/paladin/paladin_execution_service.rs#tests::paladin_execution_service_beats_on_llm_completion_stream_chunk_and_armament"
        status: pass
    human_judgment: false

# Metrics
duration: ~50 min
completed: 2026-09-06
status: complete
---

# Phase 25 Plan 09: Timeouts, Heartbeat and the Run-Level Bound Summary

**A watch-based `HeartbeatHandle` reaching Paladin nodes through a defaulted `PaladinPort::execute_observed`, per-attempt `run_timeout`/`idle_timeout` raced against the attempt and named by a typed `TimeoutKind`, and `EngineLimits.run_timeout` finally enforced as `RunTimeoutExceeded` through the same limit-failure path the recursion and visit limits take.**

## Objective

Distinguish a stalled node from a slow one. A stream that emits a chunk every 100 ms is healthy; one that stalls 300 ms is not; no wall-clock cap can tell them apart. This plan lands the progress channel (`HeartbeatHandle`), the two per-attempt bounds that read it, and the run-level budget that has been declared since Phase 23 but never acted on — all three bounds named by value, never by message text.

## What was built

### Task 1 — HeartbeatHandle, NodeContext.attempt/heartbeat(), defaulted execute_observed (`918bf235` RED → `995fd9a8` GREEN)

- `crates/paladin-core/src/platform/container/heartbeat.rs`: `HeartbeatHandle` is an `Arc<tokio::sync::watch::Sender<u64>>` beat counter with `new`/`beat`/`beats`/`subscribe`, a manual `PartialEq` under which any two handles compare equal, `Eq`, and a `Debug` that prints `HeartbeatHandle(..)`. The module rustdoc records why `watch` beat the atomic-timestamp alternative (the idle timer awaits `changed()` instead of polling; clock-agnostic; `beats()` is the observation surface). It lives in core because `paladin-ports` must name it and cannot depend on `paladin-battalion` (ADR-0016). `crates/paladin-battalion/src/engine/heartbeat.rs` is a pure re-export; `engine::HeartbeatHandle` is also re-exported.
- `NodeContext` gains `pub attempt: u32` and `pub heartbeat: HeartbeatHandle` with `attempt()` and `heartbeat()`; the derive set `Debug, Clone, PartialEq` is unchanged and guarded by `node_context_keeps_its_derives_with_a_heartbeat_handle`. The superstep loop now builds an attempt-invariant `base_ctx` once per dispatch and rebuilds `ctx` per attempt with the current attempt number and a FRESH handle.
- `PaladinPort::execute_observed(&self, paladin, input, heartbeat)` is a defaulted method whose body is `self.execute(paladin, input).await` — a correct claim of no progress. The rustdoc states the degrade-to-wall-clock consequence and carries a compiling example. The engine's Paladin dispatch calls it exclusively (`the_engine_always_calls_execute_observed`).
- `PaladinExecutionService` gains inherent `execute_observed` (same timeout wrapper as `execute`, via a shared `execute_bounded`) and `execute_stream_observed` (via a shared `execute_stream_inner`); `execute_internal` takes `heartbeat: Option<&HeartbeatHandle>` and beats after every LLM response and after every Armament invocation; the stream forwarder beats before every chunk send. The 25-08 `served_by` copy is untouched and still runs.
- `run_with_namespace` gains `parent_heartbeat: Option<HeartbeatHandle>`; a `NodeSpec::Battalion` dispatch passes the parent's handle and the child beats it at the top of every child superstep. The two `WarEngine` call sites (`resume_with`, `fork`) and the `run` wrapper pass `None`.

### Task 2 — Per-attempt run_timeout and idle_timeout named by TimeoutKind (`49836dc7` RED → `976cb3c4` GREEN)

- `NodeFailure::Timeout(TimeoutKind)` converts to `NodeError { source: Timeout(kind), transience: Transient }`.
- `AttemptBounds::resolve(policy, engine_deadline)` computes the per-attempt deadline as `min(now + run_timeout, engine_deadline)` and names it `Run` or `EngineRun`, plus the idle window. `race_attempt` is a `tokio::select!` biased toward the attempt future against `deadline_or_pending` and `idle_or_pending`; on expiry the attempt future is dropped (partial work discarded) and the failure carries the typed kind.
- `idle_or_pending` subscribes to the handle and loops `tokio::time::timeout(idle, rx.changed())`: a beat restarts the window, a dropped sender goes pending, an elapsed window fires `Idle`. A node without `idle_timeout` never subscribes, so `heartbeat()` is a cheap no-op there.
- The retry gate skips `Timeout(EngineRun)`; the final `node_error` is structured for any timeout even on a no-Aegis node; the surfacing match routes `Timeout` with a `NodeError` to `EngineError::NodeFailed`.
- All seven tests use `#[tokio::test(start_paused = true)]`; no `std::thread::sleep` was added; the whole battalion lib suite runs in 3 s.

### Task 3 — EngineLimits.run_timeout enforced, nested, named (`d5301335` RED → `482a63e1` GREEN)

- `EngineError::RunTimeoutExceeded { elapsed: Duration, limit: Duration }`.
- `run_with_namespace` records `run_started_at = tokio::time::Instant::now()` and `engine_deadline = run_timeout.map(|l| run_started_at + l)`; the deadline is threaded into every attempt's `AttemptBounds`.
- One `persist_limit_failure` helper now serves `RecursionLimitExceeded`, `NodeVisitLimitExceeded`, the new boundary-time `RunTimeoutExceeded` check (placed right after the recursion check) and the mid-superstep `EngineRun` cut — consistency is structural. The CR-01 `failed_node` fallback is a shared `boundary_failed_node` closure.
- The bookkeeping loop tracks the first (dispatch-order) `Timeout(EngineRun)` failure and, ahead of `node_failure`, ends the run `RunTimeoutExceeded` with this superstep's records and the cut attempt's `NodeError` on `WaypointStatus::Failed.node_error`.
- `src/config/engine.rs`: the `run_timeout_secs` rustdoc now states the landed semantics (the words "plumbing-only" no longer appear in the file; `config_seconds_convert_to_duration_exactly` pins `Some(90) -> Duration::from_secs(90)` and that wording). `EngineLimits.run_timeout`'s rustdoc documents the bridge vacuity: `from_formation`/`from_phalanx`/`from_campaign` all use `EngineLimits::default()`, and `bridges_carry_no_legacy_battalion_timeout` is both a behavioural check and an `include_str!` tripwire on `bridges.rs`. The legacy Formation/Phalanx/Campaign services are untouched (verified by `git diff --name-only` against the base commit).

## Verification (all exit 0)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets --all-features` | exit 0 |
| `cargo test -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-ai --lib` | 552 / 487 / 653 / 121 passed, 0 failed |
| `cargo test --doc -p paladin-ai-core -p paladin-ports -p paladin-battalion -p paladin-ai` | 115 / 73 / 48 / 119 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |

No integration binary (`tests/` or crate `tests/`) was modified by this plan; they compile under `--all-targets` above. Plan acceptance greps: `execute_observed` default delegates within 6 lines (1), no `paladin-battalion` in `paladin-ports/Cargo.toml` (0), `pub attempt: u32` (1), `pub fn heartbeat(` (1), `TimeoutKind::Run` (5), `TimeoutKind::Idle` (3), `start_paused = true` (8), `std::thread::sleep` (0), `RunTimeoutExceeded` in mod.rs (1), `TimeoutKind::EngineRun` (5), `plumbing-only` in config (0), legacy services diff (0 files).

## Deviations from Plan

### Auto-fixed / adapted

**1. [Rule 3 - Blocking] `PaladinExecutionService` does not implement `PaladinPort`**
- **Found during:** Task 1
- **Issue:** The plan says the service "implements `execute_observed`", but the service implements `PaladinExecutorPort` and `StreamingExecutorPort`, not `PaladinPort`; every in-tree `PaladinPort` over it is a test double or an unreachable placeholder.
- **Fix:** Landed the progress reporting as inherent `execute_observed` and `execute_stream_observed` methods on the service (same execution path, same timeout wrapper, identical results for identical inputs), so a `PaladinPort` adapter over the service can delegate to them. No trait beyond `PaladinPort` gained a method.
- **Files:** `src/application/services/paladin/paladin_execution_service.rs`
- **Commit:** `995fd9a8`

**2. [Rule 3 - Blocking] Root-crate facade re-export**
- **Found during:** Task 1
- **Issue:** `crate::core::platform::container` in the root crate is an explicit re-export facade; `heartbeat` was not reachable until listed.
- **Fix:** Added `pub use paladin_core::platform::container::heartbeat;` to `src/core/platform/mod.rs`.
- **Commit:** `995fd9a8`

**3. [Rule 2 - Correctness] Rustdoc on `TimeoutPolicy` / `TimeoutKind::Run` said "across every attempt"**
- **Found during:** Task 3
- **Issue:** Plan 25-01's placeholder docs described `run_timeout` as spanning every attempt, contradicting D-20's per-attempt semantics and the plan's truth that the degrade-to-wall-clock rule is stated on `TimeoutPolicy`'s rustdoc.
- **Fix:** Rewrote both rustdocs (doc-only; `aegis.rs`/`node_error.rs` were not in `files_modified`).
- **Commit:** `482a63e1`

**4. [Rule 3 - Blocking] `hooks.rs` test literal** — the one other `NodeContext { .. }` literal in the tree gained `attempt: 1, heartbeat: HeartbeatHandle::new()`. Commit `995fd9a8`.

**5. [Rule 2 - Correctness] Structured `NodeError` for a timeout on a no-Aegis node** — D-09's "no policy, no change" rule is deliberately relaxed for `NodeFailure::Timeout` only, because the engine budget can cut a node that declared no policy and the fired kind must still be a typed value on the Waypoint. Commit `976cb3c4`.

### Not done (by design)

- `REQUIREMENTS.md`, `STATE.md`, `ROADMAP.md` were not touched: worktree mode, the orchestrator owns shared-file writes after the wave merges (FT-03 is listed under `requirements-completed` above for that step).
- `MIGRATION.md` §9.2 (`PaladinPort` defaulted-method row `N`) and §9.5 (`run_timeout_secs` bullet) are plan 25-14's, per the plan text.

## Threat register outcome

| Threat | Disposition | Pinned by |
|---|---|---|
| T-25-39 node beats forever to evade idle | mitigated | `a_slow_but_progressing_node_fails_on_run_timeout_not_idle`, `the_tightest_bound_fires` |
| T-25-40 stalled stream holds a superstep open | mitigated | `a_port_that_stalls_300ms_fails_with_timeout_idle`; non-beating port degrades to a wall clock (rustdoc on `TimeoutPolicy` and `execute_observed`) |
| T-25-41 timed-out partial delta reaches the Battlefield | mitigated | `a_timed_out_attempts_partial_work_is_discarded` (attempt future dropped in `race_attempt`) |
| T-25-42 misreading which bound fired | mitigated | `the_fired_bound_is_read_from_the_typed_kind`, every test asserts `TimeoutKind` by value |
| T-25-43 required method on a published trait | mitigated | `execute_observed` default body delegates to `execute`; acceptance grep passes |
| T-25-44 hanging `EdgeConditionEvaluator` | accepted (R-23-01 stays open, D-34) | per-attempt bounds wrap node execution, not edge evaluation |

## Known Stubs

None. Every new method has a real body; no placeholder values flow anywhere.

## Threat Flags

None: no new network endpoint, auth path, file access or schema change. `HeartbeatHandle` is process-local.

## Self-Check: PASSED

- `crates/paladin-core/src/platform/container/heartbeat.rs` — FOUND
- `crates/paladin-battalion/src/engine/heartbeat.rs` — FOUND
- Commits on `worktree-agent-ad2bb1b0000025738` above base `7b3ee1ad`: `918bf235`, `995fd9a8`, `49836dc7`, `976cb3c4`, `d5301335`, `482a63e1` — all FOUND in `git log`.

## TDD Gate Compliance

Each task has a `test(...)` RED commit followed by a `feat(...)` GREEN commit (Task 1 and Task 3 RED commits intentionally do not compile on the not-yet-existing symbol, per the house convention established in 25-08; Task 2 RED compiles and fails 5 of 7 with the two no-change guards passing by design).
