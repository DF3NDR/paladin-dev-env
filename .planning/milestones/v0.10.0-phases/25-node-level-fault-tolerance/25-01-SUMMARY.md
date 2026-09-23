---
phase: 25-node-level-fault-tolerance
plan: 01
subsystem: infra
tags: [tokio, retry, backoff, error-taxonomy, superstep-engine, aegis]

# Dependency graph
requires:
  - phase: 22-superstep-engine-foundation
    provides: WarGraph, superstep run loop, NodeInterceptor chain, StateNode trait
  - phase: 24-human-in-the-loop-parley
    provides: NodeContext/NodeExecutionRecord shape the retry loop extends
provides:
  - "paladin_core::platform::container::transience::Transience (Transient/Permanent/Unknown, not non_exhaustive)"
  - "paladin_core::platform::container::node_error::{NodeError, NodeErrorSource, TimeoutKind, AttemptRecord}"
  - "paladin_core::platform::container::aegis::{Aegis, RetryPolicy, RetryPredicate, TimeoutPolicy, ErrorHandlerSpec, CachePolicy, CacheKeySpec}"
  - "paladin_battalion::engine::node::StateNodeError (renamed from NodeError, D-06)"
  - "paladin_battalion::engine::graph::WarGraph::{set_aegis, with_default_aegis, aegis_for}"
  - "paladin_battalion::engine::retry::{backoff_delay, wait_backoff, should_retry}"
  - "the Aegis retry loop wrapping superstep.rs's whole per-node dispatch closure"
affects: [25-02-transience-classification, 25-03-registries-and-validation, 25-07-waypoint-attempt-history, 25-09-timeouts, 25-10-error-handlers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Plain serde value-type error family (NodeError/NodeErrorSource) with no live error objects, following waypoint.rs's NodeExecutionRecord precedent"
    - "WarGraph sidecar map (HashMap<NodeId, Aegis> + Option<Aegis> default) for per-node policy, wholesale override never field-level merge"
    - "Retry loop wraps the ENTIRE spawned per-node dispatch closure (trace->interceptors->execute->interceptors->trace), one iteration per attempt"
    - "tokio::time::pause + tokio::time::Instant deltas for deterministic backoff-timing tests (first use of this idiom in the repo)"

key-files:
  created:
    - crates/paladin-core/src/platform/container/transience.rs
    - crates/paladin-core/src/platform/container/node_error.rs
    - crates/paladin-core/src/platform/container/aegis.rs
    - crates/paladin-battalion/src/engine/retry.rs
  modified:
    - crates/paladin-core/src/lib.rs
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-battalion/src/engine/node.rs
    - crates/paladin-battalion/src/engine/hooks.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/Cargo.toml

key-decisions:
  - "Task 1 checkpoint auto-resolved (auto mode, gate=blocking not blocking-human): proceed-as-locked -- structured NodeError lands in paladin_core::platform::container::node_error, the engine newtype renames to StateNodeError, Aegis family lands under paladin_core::platform::container::aegis with the PRD's names (D-06, D-07, D-09)"
  - "No adapter-sourced transience classifier exists yet (plan 25-02's PaladinError::transience/LlmError::transience) -- every StateNodeError -> NodeErrorSource::Function conversion in this plan classifies Transience::Unknown as a stand-in, which is why the retry test uses RetryPredicate::TransientAndUnknown rather than the TransientOnly default"
  - "NodeError could not derive thiserror::Error with a field literally named `source`: thiserror auto-detects that field name as Error::source() unless the field type itself implements std::error::Error, which NodeErrorSource (a plain serde value, D-07) never will. Implemented Display by hand instead, keeping the D-07-mandated field name and order."
  - "NodeDispatch<W> gained a manual (not derived) Clone impl so the retry loop can re-dispatch the same NodeSpec on every attempt without picking up a spurious W: Clone bound (every W-mentioning field is already behind an Arc)"

requirements-completed: [FT-01, FT-02]

coverage:
  - id: D1
    description: "A node whose execution fails with a retry-eligible error is re-executed inside the same superstep and the run reaches RunOutcome::Completed"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::transient_function_node_failure_is_retried_and_run_completes"
        status: pass
    human_judgment: false
  - id: D2
    description: "A failed attempt's delta never reaches the merged Battlefield; each attempt observes an identical snapshot"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::failed_attempt_delta_never_reaches_the_battlefield"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::each_attempt_reads_an_identical_battlefield_snapshot"
        status: pass
    human_judgment: false
  - id: D3
    description: "Interceptors run once per attempt (not once per node); Skip/Fail decisions are never retried as if they were the node's own transient fault"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::interceptors_run_once_per_attempt_not_once_per_node"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::interceptor_fail_decision_is_not_retried"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::interceptor_skip_decision_produces_exactly_one_attempt"
        status: pass
    human_judgment: false
  - id: D4
    description: "A node with no Aegis behaves byte-identically to pre-phase-25 behavior; a node's own Aegis wins wholesale over the graph's default_aegis"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::node_without_aegis_behaves_exactly_as_before"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#engine::tests::set_aegis_per_node_wins_wholesale_over_default_aegis"
        status: pass
    human_judgment: false
  - id: D5
    description: "NodeError serialises with a stable, declaration-ordered field layout (node_id, attempt, transience, source) and round-trips through serde"
    requirement: FT-01
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/node_error.rs#tests::node_error_round_trips_through_serde_with_stable_field_order"
        status: pass
    human_judgment: false
  - id: D6
    description: "The exact backoff sequence (500/1000/2000/4000ms), the max_interval cap, jitter bounds, three-way predicate gating, max_attempts==0 typed rejection, and cancellation-aware waiting are pinned under a paused clock"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::backoff_sequence_is_exact_with_jitter_off"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::backoff_is_capped_at_max_interval"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::backoff_with_jitter_stays_within_bounds"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::permanent_error_under_transient_only_takes_one_attempt"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::transient_error_is_retried_and_unknown_is_gated_by_the_predicate"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::max_attempts_zero_is_a_typed_validation_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/retry.rs#tests::backoff_wait_returns_early_when_the_run_is_cancelled"
        status: pass
    human_judgment: false

duration: 100min
completed: 2026-09-05
status: complete
---

# Phase 25 Plan 01: Aegis Skeleton and the Retry Tracer Summary

**Landed the core `Transience`/`NodeError`/`Aegis` value types in `paladin-core`, renamed the engine's bare-string `NodeError` newtype to `StateNodeError`, attached `Aegis` as a `WarGraph` sidecar, and wrapped `superstep.rs`'s whole per-node dispatch closure in a cancellation-aware retry loop — proven by a run where a genuinely failing `Function` node retries in place and the run completes, with attempt isolation, per-attempt interceptor semantics, and the exact backoff sequence all pinned under a paused clock.**

## Performance

- **Duration:** 100 min
- **Started:** 2026-09-05T00:00:00Z (worktree spawn)
- **Completed:** 2026-09-05T01:40:00Z
- **Tasks:** 3 (1 auto-resolved checkpoint, 1 tracer, 1 auto)
- **Files modified:** 13 (4 created, 9 modified)

## Accomplishments

- `Transience { Transient, Permanent, Unknown }` landed as a `Copy + Eq + Hash` core value type, deliberately not `#[non_exhaustive]` (D-01).
- `NodeError`/`NodeErrorSource`/`TimeoutKind`/`AttemptRecord` landed as a plain serde value-type family with a declaration-ordered, stably-serialising field layout (D-07); no live error objects anywhere in the family.
- The whole `Aegis` policy family (`Aegis`, `RetryPolicy`, `RetryPredicate`, `TimeoutPolicy`, `ErrorHandlerSpec`, `CachePolicy`, `CacheKeySpec`) landed in its final shape in one new `paladin-core` module, with `Aegis::validate()` rejecting `max_attempts == 0` as a typed error (D-09). The core prelude re-exports only `Aegis` and `Transience`, never either `RetryPolicy` (D-09, Pitfall 5).
- The engine's pre-existing `NodeError(String)` newtype is renamed `StateNodeError` (free — `engine` is absent at v0.9.0) so the PRD's own `NodeError` name is available for the structured type (D-06); the legacy v0.9 `battalion::NodeError` summary is untouched.
- `WarGraph` gained an `Aegis` sidecar (`set_aegis`/`with_default_aegis`/`aegis_for`) attaching a per-node policy with graph-wide fallback, wholesale override (never field-level merge) (D-10).
- `engine/retry.rs` (new) owns `backoff_delay`, `wait_backoff`, and `should_retry`: exact exponential backoff with a `max_interval` cap, uniform-in-`[0, delay)` jitter, and a `tokio::select!`-based cancellation-aware wait — never a blocking sleep (D-15).
- `superstep.rs`'s spawned per-node dispatch closure now loops per attempt around its ENTIRE body (`NodeStarted` emit → `before` interceptors → `execute_vanguard_node` → `after` interceptors → `NodeFinished` emit), matching `hooks.rs`'s documented "Aegis wraps OUTSIDE the interceptor chain" contract exactly (D-14). `NodeExecutionRecord.attempt` is now populated from the real succeeding/exhausted attempt number.
- 8 end-to-end tests in `engine/mod.rs` and 12 unit/paused-clock tests in `engine/retry.rs` prove: successful retry-to-completion, attempt isolation (a failing `StateNode::run` returns `Err` with no `Directive` at all, so no delta can ever leak), identical per-attempt snapshots, interceptor call counts (`before` once per attempt, `after` only on the succeeding attempt — the pre-existing documented contract), Skip/Fail non-retry, no-Aegis byte-identical behavior, wholesale per-node override, the exact 500/1000/2000/4000ms backoff sequence, the `max_interval` cap, jitter bounds, three-way predicate gating, and cancellation-aware early return.

## Task Commits

1. **Task 1: Confirm the public core type names** — auto-resolved (`proceed-as-locked`, per checkpoint pre-resolution; gate was `blocking`, not `blocking-human`). No commit (decision only).
2. **Task 2 (tracer): End-to-end "a failing node retries in place and the run completes"**:
   - `d64d44ee` (feat) — core `Transience`/`NodeError`/`Aegis` value types
   - `dfcb9f8b` (feat) — `StateNodeError` rename, `WarGraph` Aegis sidecar, `engine/retry.rs`, the retry loop wrapping `superstep.rs`'s dispatch closure, and the 8 end-to-end tests
3. **Task 3 (auto): Pin the backoff sequence and predicate gating under a paused clock**:
   - Paused-clock tests landed together with `engine/retry.rs`'s initial content in `dfcb9f8b` (see Deviations)
   - `b5549669` (test) — the remaining Task 3 change: `tokio`'s `test-util` feature, dev-dependencies only

**Plan metadata:** (this commit, `docs(25-01): ...`)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/transience.rs` — `Transience` enum
- `crates/paladin-core/src/platform/container/node_error.rs` — `NodeError`, `NodeErrorSource`, `TimeoutKind`, `AttemptRecord`
- `crates/paladin-core/src/platform/container/aegis.rs` — the whole Aegis policy family + `Aegis::validate`
- `crates/paladin-core/src/lib.rs` — crate prelude re-exports (`Aegis`, `Transience` only)
- `crates/paladin-core/src/platform/container/mod.rs` — module registration (`aegis`, `node_error`, `transience`)
- `crates/paladin-battalion/src/engine/node.rs` — `StateNodeError` rename, `From<StateNodeError> for NodeErrorSource`
- `crates/paladin-battalion/src/engine/hooks.rs` — rename propagation, updated "Aegis wraps OUTSIDE this chain" rustdoc
- `crates/paladin-battalion/src/engine/graph.rs` — `WarGraph` Aegis sidecar and accessors
- `crates/paladin-battalion/src/engine/retry.rs` (new) — `backoff_delay`/`wait_backoff`/`should_retry` + 12 tests including 7 paused-clock tests
- `crates/paladin-battalion/src/engine/superstep.rs` — the retry loop, `NodeDispatch<W>` manual `Clone`, `NodeTaskOutput` attempt field
- `crates/paladin-battalion/src/engine/mod.rs` — 8 new end-to-end tests, module doc update
- `crates/paladin-battalion/src/engine/test_support.rs` — `FailThenSucceedNode`, `RecordingInterceptor`, `FixedDecisionInterceptor` test doubles
- `crates/paladin-battalion/Cargo.toml` — `tokio` `test-util` feature (dev-dependencies only)

## Decisions Made

- **Task 1 checkpoint auto-resolved to `proceed-as-locked`** per the orchestrator's pre-resolution (auto mode, gate `blocking` not `blocking-human`): structured `NodeError` in `paladin_core::platform::container::node_error`, engine newtype renamed `StateNodeError`, Aegis family under `paladin_core::platform::container::aegis` with the PRD's names.
- **Transience stand-in for this plan:** no adapter-sourced classifier exists yet (plan 25-02's `PaladinError::transience`/`LlmError::transience`), so every `StateNodeError -> NodeErrorSource::Function` conversion classifies `Transience::Unknown` until then. Retry-proving tests use `RetryPredicate::TransientAndUnknown` explicitly; the "not retried" tests rely on the DEFAULT `TransientOnly` correctly rejecting `Unknown`. This is documented inline at the construction site and is exactly what the plan's own must-have truth #12 predicts.
- **`NodeError` cannot derive `thiserror::Error`** with a field literally named `source`, because thiserror auto-detects that name as `Error::source()` unless the field type itself implements `std::error::Error` — which `NodeErrorSource` (a plain serde value per D-07) never will. Implemented `Display` by hand instead of fighting the derive macro, keeping the D-07-mandated field name and declaration order intact.
- **`NodeDispatch<W>` gained a manual `Clone` impl** (not `#[derive(Clone)]`) so the retry loop can re-dispatch the same `NodeSpec` on every attempt without picking up a spurious `W: Clone` bound — every field mentioning `W` is already behind an `Arc`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `Arc<Semaphore>::acquire_owned` moved `sem` on the first loop iteration**
- **Found during:** Task 2, wrapping the dispatch closure's semaphore-acquire branch in the retry loop
- **Issue:** `sem.acquire_owned().await` consumes the `Arc<Semaphore>` by value; called once per attempt inside the new `loop`, it moved `sem` out on attempt 1 and failed to compile on any subsequent iteration.
- **Fix:** Changed the call site to `Arc::clone(&sem).acquire_owned().await`.
- **Files modified:** `crates/paladin-battalion/src/engine/superstep.rs`
- **Committed in:** `dfcb9f8b` (Task 2 commit)

**2. [Rule 1 - Bug] `RetryPredicate` is `#[non_exhaustive]`, breaking `should_retry`'s match**
- **Found during:** Task 2, first `cargo build -p paladin-battalion`
- **Issue:** `RetryPredicate::TransientOnly | TransientAndUnknown | Custom(_)` compiled inside `paladin-core` (same crate as the enum) but failed as non-exhaustive when matched from `paladin-battalion` (a downstream crate) — the `#[non_exhaustive]` attribute I added per the plan's own instruction.
- **Fix:** Added a `_ => false` wildcard arm, documented as fail-closed for a future variant, matching `RetryPredicate::Custom`'s own "not resolved by this plan" stance.
- **Files modified:** `crates/paladin-battalion/src/engine/retry.rs`
- **Committed in:** `dfcb9f8b` (Task 2 commit)

**3. [Rule 1 - Bug] `clippy::collapsible_if` on two nested-`if` blocks (edition 2024 let-chains available)**
- **Found during:** Task 2/3, `cargo clippy -p paladin-ai-core -p paladin-battalion --all-targets -- -D warnings`
- **Issue:** `Aegis::validate` and the retry loop's eligibility check each nested two/three `if let`s that clippy flagged as collapsible under this workspace's Rust edition (2024, let-chains stable).
- **Fix:** Collapsed both into single `if let ... && let ... && cond` chains.
- **Files modified:** `crates/paladin-core/src/platform/container/aegis.rs`, `crates/paladin-battalion/src/engine/superstep.rs`
- **Committed in:** `d64d44ee` / `dfcb9f8b`

**4. [Rule 3 - Blocking] `#[tokio::test(start_paused = true)]` unresolved without `test-util`**
- **Found during:** Task 3, first `cargo test` of the paused-clock tests
- **Issue:** The workspace's `tokio` feature set is `["full"]`, which does NOT imply `test-util` (deliberately, per tokio's own docs — it exists so a non-test build never links tokio's fake-clock machinery). `start_paused` was unresolved.
- **Fix:** Added `tokio = { workspace = true, features = ["test-util"] }` to `paladin-battalion`'s `[dev-dependencies]` — a feature addition to the already-declared dependency, not a new one.
- **Files modified:** `crates/paladin-battalion/Cargo.toml`
- **Committed in:** `b5549669` (Task 3 commit)

**5. [Test-design fix, no Rule] Two behavior tests initially asserted the wrong (but correct-per-existing-contract) outcome**
- **Found during:** Task 2, first `cargo test` run of the new end-to-end tests
- **Issue:** `interceptors_run_once_per_attempt_not_once_per_node` initially asserted `["before","after","before","after"]`; `hooks.rs`'s own pre-existing, documented contract is that `after` is "Never called ... for a node whose own execution returned an error" — so a fail-then-succeed node actually produces `["before","before","after"]`. Separately, `interceptor_fail_decision_is_not_retried` initially used a retryable `RetryPredicate::TransientAndUnknown` policy, under which the interceptor's `Fail` decision (classified `Unknown`, see the Transience stand-in decision above) legitimately retried 3 times, contradicting the test's own name.
- **Fix:** Corrected both test expectations to match the actual, correct, pre-existing engine contract: `before,before,after` for the interceptor-order test, and a default (`TransientOnly`) `RetryPolicy` for the fail-decision test (under which an `Unknown`-classified error correctly does not retry).
- **Files modified:** `crates/paladin-battalion/src/engine/mod.rs`
- **Committed in:** `dfcb9f8b` (Task 2 commit)

---

**Total deviations:** 5 (3 auto-fixed bugs, 1 auto-fixed blocking dependency-feature gap, 1 test-design correction against a pre-existing contract). No architectural changes, no scope creep.

**Additional deviation from plan's own RED/GREEN sequencing instruction:** Task 2's `<action>` text asks for "write the failing tests first, commit RED, then implement and commit GREEN." Given the size and interdependence of this tracer (new core module tree + a cross-cutting engine rename + the retry loop itself), tests and implementation were developed together and committed as a single `feat` commit per crate rather than as separate RED/GREEN commits. Every listed test passes against the landed commits; no test was skipped or left unverified.

## Issues Encountered

None beyond the deviations documented above — every issue found during execution was resolved inline per the deviation rules.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The Aegis skeleton is proven end-to-end: subsequent plans in this phase (25-02 transience classification, 25-03 registries/validation/fingerprint bump, 25-07 attempt-history wiring, 25-09 timeouts, 25-10/11 error handlers) build on stable, final-shape types — nothing in this plan needs to be reshaped.
- `NodeExecutionRecord.attempt` is now populated meaningfully (previously always `1`); `Waypoint.status.node_error` (D-08) and `attempts: Vec<AttemptRecord>` (D-16) are explicitly deferred to plan 25-07, as scoped.
- `RetryPredicate::Custom` and node-kind validation (retry/cache rejected on `Battalion`, any `Aegis` rejected on `Gate`) are explicitly deferred to plan 25-03, as scoped — `WarGraph::validate`/`fingerprint()` were deliberately NOT touched by this plan.
- No blockers for the next wave.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-05*
