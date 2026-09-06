---
phase: 25-node-level-fault-tolerance
plan: 12
subsystem: testing
tags: [aegis, retry, muster, timeout, cancellation, waypoint, e2e, stress, tokio-pause, sqlite, mock-paladin-port]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-01)
    provides: "Aegis/RetryPolicy on WarGraph::set_aegis, retry::wait_backoff's cancellation-aware select!, the paused-clock idiom"
  - phase: 25-node-level-fault-tolerance (plan 25-02)
    provides: "PaladinError::LlmFailure { transience, status, provider, message } and PaladinError::transience()"
  - phase: 25-node-level-fault-tolerance (plan 25-07)
    provides: "NodeExecutionRecord.attempts/attempt, NodeRunOutcome::Interrupted re-listed on the Halted vanguard, per-task Muster retry, NodeErrorSource::Llm classification of an LlmFailure"
  - phase: 25-node-level-fault-tolerance (plan 25-09)
    provides: "TimeoutPolicy, TimeoutKind::{Run,Idle,EngineRun}, EngineLimits.run_timeout enforced as EngineError::RunTimeoutExceeded, EngineRun never retried"
  - phase: 23-control-flow-dynamic-routing-fan-out-subgraphs
    provides: "Muster dispatch, MusterContext, intra-superstep progress Waypoints, the E2E-3 muster/defer/order fixture and its fenced PHASE 25 SEAM"
  - phase: 24-pause-resume-history-graceful-shutdown
    provides: "WarEngine::with_cancellation_token, RunOutcome::Halted, resume from a Halted Waypoint"
provides:
  - "FaultyPaladinPort::fail_paladin_until_attempt(name, n): an additive, chainable per-Paladin failure counter producing PaladinError::LlmFailure { transience: Transient, status: Some(503), provider: Some(\"faulty-paladin-port\"), .. }; the global fail_until_attempt semantics and its tests are unchanged, precedence documented on the type"
  - "E2E-3's recovering-worker half through a REAL per-task Aegis retry: one_worker_recovers_by_real_per_task_retry asserts exactly 7 port calls, w3 at attempt 3 with two AttemptRecords carrying the 503 by value, siblings at attempt 1, one aggregator run, 5 results in task_key order, one muster-superstep Waypoint + 5 progress Waypoints and none between attempts; the PHASE 25 SEAM block is deleted"
  - "tests/integration/aegis_retry_stress_test.rs ([[test]] aegis_retry_stress): the X-05 multi-thread Muster + per-task retry stress with exact counts and a timeout guard, kill-during-backoff abort + resume-at-attempt-1 under a paused clock, and the nested run-timeout bounds named by TimeoutKind in both directions over SQLite"
affects: [25-13-node-cache-engine-integration, 25-14-migration-doc-and-guide, 26-agent-runtime, 28-observability-otel]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "One worker template per mustered task when a test must address a single task by value: the engine hands every task its TEMPLATE's Paladin, so a shared template's tasks are indistinguishable to a name-keyed port mock and to NodeExecutionRecord.node_id"
    - "Paused-clock cancellation test outside the battalion crate: spawn the run, spin on tokio::task::yield_now (never sleep) until the port has been called, cancel the token, bound the join with tokio::time::timeout, and read the abort's duration off tokio::time::Instant"
    - "Wall-clock retry fixtures over SQLite shorten initial_interval and disable jitter but never touch retry_on; timing is never asserted, only counts and typed kinds"

key-files:
  created:
    - tests/integration/aegis_retry_stress_test.rs
  modified:
    - tests/helpers/mock_paladin_port.rs
    - tests/integration/e2e_muster_defer_order_test.rs
    - Cargo.toml

key-decisions:
  - "The per-Paladin counter sits SECOND in FaultyPaladinPort's precedence (global fail_until_attempt -> fail_paladin_until_attempt -> fail_always -> fail_paladin), and every call advances both the global counter and the executed Paladin's own counter regardless of which mode decides the call -- so the global counter's semantics are literally untouched and the composition is predictable from the rustdoc"
  - "E2E-3's recovering-worker fixture registers five worker templates w1..w5 (one task each, keys a..e) instead of the single shared `worker` template: the engine dispatches a mustered task with its template's Paladin, so `w3` is only addressable -- by the name-keyed counter and by the record's node_id -- when it is its own template; the muster/defer/order tests keep the original single-template fixture"
  - "The recovering-worker RetryPolicy keeps max_attempts: 3 and the default TransientOnly predicate but shortens initial_interval to 20 ms: the fixture drives a real on-disk SQLite backend, so it cannot run under tokio::time::pause and the default 500 ms/1 s waits would add seconds to count-only assertions"
  - "The paused-clock kill/resume tests use paladin_storage's public InMemoryWaypointStore rather than paladin-battalion's RecordingWaypointStore, which is pub(crate); 'no Waypoint between attempts' is asserted through the port's own history() (exactly 1 then exactly 2 Waypoints)"
  - "Both timeout-direction tests attach a 3-attempt retry policy so the same file proves D-20's asymmetry end to end: Timeout(EngineRun) is never retried (1 call, RunTimeoutExceeded), Timeout(Run) is retried to exhaustion (3 calls, NodeFailed with two Run-cut AttemptRecords)"

patterns-established:
  - "Negative control beside a green E2E: the identical fixture with the policy removed must FAIL (without_a_retry_policy_the_same_transient_failure_fails_the_run), so a green run is attributable to the mechanism under test rather than the fixture (T-25-59)"

requirements-completed: [FT-02, FT-03, FT-04]

coverage:
  - id: D1
    description: "FaultyPaladinPort::fail_paladin_until_attempt fails one named Paladin for its own first N calls with a Transient LlmFailure { status: Some(503) }, independently per name, composing with -- and not changing -- the global fail_until_attempt counter"
    requirement: FT-02
    verification:
      - kind: unit
        ref: "tests/helpers/mock_paladin_port.rs#tests::{fail_paladin_until_attempt_is_scoped_to_one_paladin,fail_paladin_until_attempt_returns_a_transient_llm_failure,the_global_fail_until_attempt_semantics_are_unchanged,the_two_mechanisms_compose,per_paladin_counters_are_independent} (run via cargo test --test e2e_muster_defer_order)"
        status: pass
      - kind: unit
        ref: "tests/helpers/mock_paladin_port.rs#tests::faulty_paladin_port_fail_until_attempt_then_succeeds -- pre-existing, unedited (git diff shows 0 removed lines naming it)"
        status: pass
    human_judgment: false
  - id: D2
    description: "E2E-3's recovering worker recovers through a real per-task Aegis retry under the default TransientOnly predicate: exactly 7 port calls, w3 attempt 3 with two AttemptRecords, siblings attempt 1, aggregator once, 5 results in task_key order, one muster-superstep Waypoint plus 5 progress Waypoints and none between attempts; the scripted seam is gone"
    requirement: FT-02
    verification:
      - kind: e2e
        ref: "tests/integration/e2e_muster_defer_order_test.rs#{one_worker_recovers_by_real_per_task_retry,the_default_predicate_is_used,without_a_retry_policy_the_same_transient_failure_fails_the_run} (cargo test --test e2e_muster_defer_order)"
        status: pass
      - kind: other
        ref: "grep -c 'PHASE 25 SEAM' tests/integration/e2e_muster_defer_order_test.rs == 0; no one_worker_recovers_by_manual_attempt_scripting, no fail_until_attempt(2), no TransientAndUnknown"
        status: pass
    human_judgment: false
  - id: D3
    description: "X-05: Muster combined with per-task retry holds under real multi-thread concurrency with exact per-task and total counts, exact attempt histories, task_key-ordered aggregation and no cross-task retry-state leakage, under a timeout guard"
    requirement: FT-02
    verification:
      - kind: integration
        ref: "tests/integration/aegis_retry_stress_test.rs#{muster_with_per_task_retry_under_concurrency_has_exact_counts,concurrent_tasks_do_not_share_retry_state} (cargo test --test aegis_retry_stress; 6 consecutive runs green)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A run killed while a node sleeps in a 60 s backoff aborts in under 1 s of virtual time, records the node Skipped { shutdown } re-listed on the Halted vanguard, and resuming from that Waypoint re-executes it at attempt 1 with no Waypoint written between the attempts"
    requirement: FT-02
    verification:
      - kind: integration
        ref: "tests/integration/aegis_retry_stress_test.rs#{a_run_killed_during_backoff_aborts_immediately,resuming_after_a_kill_during_backoff_restarts_at_attempt_one}"
        status: pass
    human_judgment: false
  - id: D5
    description: "EngineLimits.run_timeout tighter than both per-attempt bounds names Timeout(EngineRun) by value, is never retried and ends the run EngineError::RunTimeoutExceeded against a real SQLite Waypoint backend; the mirror names Timeout(Run), is retried to exhaustion and ends NodeFailed"
    requirement: FT-03
    verification:
      - kind: integration
        ref: "tests/integration/aegis_retry_stress_test.rs#{an_engine_run_timeout_tighter_than_both_bounds_names_enginerun,a_node_run_timeout_tighter_than_the_engine_budget_names_run}"
        status: pass
    human_judgment: false
  - id: D6
    description: "Nothing in this plan's tests depends on Docker, Redis or any live service, and every test binary that includes the edited shared helper still passes"
    requirement: FT-04
    verification:
      - kind: other
        ref: "grep -qiE 'redis|docker|127.0.0.1:6379' tests/integration/aegis_retry_stress_test.rs -> no match; 13 root [[test]] binaries run green (table below)"
        status: pass
    human_judgment: false

# Metrics
duration: 21min
completed: 2026-09-06
status: complete
---

# Phase 25 Plan 12: E2E-3 Seam Replacement, X-05 Stress, Kill-During-Backoff and Run-Timeout E2E Summary

**The Phase 23 scripted stand-in for a recovering mustered worker is deleted and replaced by a genuinely retrying `w3` (7 port calls, two `AttemptRecord`s, default `TransientOnly` predicate) driven by a new additive per-Paladin counter on the shared `FaultyPaladinPort`; a new `aegis_retry_stress` binary proves Muster + per-task retry under multi-thread concurrency with exact counts, a mid-backoff kill that aborts at once and resumes at attempt 1, and the nested run-timeout bounds naming `EngineRun` vs `Run` by value over SQLite.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-09-06T01:35:55Z
- **Completed:** 2026-09-06T01:57:09Z
- **Tasks:** 3
- **Files modified:** 4 (1 created)

## Accomplishments

- `FaultyPaladinPort::fail_paladin_until_attempt(name, n)` (D-31): a chainable per-Paladin threshold backed by a per-name call counter beside the untouched global counter, producing `PaladinError::LlmFailure { transience: Transient, status: Some(503), provider: Some("faulty-paladin-port"), .. }` so the default retry predicate retries it by value. Precedence (global counter, per-Paladin counter, `fail_always`, `fail_paladin`) is stated on the type and on the builder; five new in-file tests pin scoping, transience, unchanged global semantics, composition and independence, and the seven pre-existing tests pass unedited.
- E2E-3's recovering-worker half is now real: the fenced `PHASE 25 SEAM` block, its `fail_until_attempt(2)` warm-up calls and their assertions are gone; `one_worker_recovers_by_real_per_task_retry` musters `w1`..`w5` (one Paladin worker template per task, each with `RetryPolicy { max_attempts: 3, .. }`, `retry_on` at its default) and asserts exactly 7 port calls, `w3` at `attempt: 3` with `AttemptRecord`s 1 and 2 whose `NodeErrorSource::Llm { status: Some(503) }` is Transient, every sibling at `attempt: 1` with an empty history, one aggregator run, all 5 results in `task_key` order, exactly one superstep-complete Waypoint for the muster superstep plus Phase 23's five progress Waypoints, 8 Waypoints on the thread and no `Failed` status. A negative control proves the same fixture without the policy fails the run with exactly 5 calls.
- `tests/integration/aegis_retry_stress_test.rs` (registered as `[[test]] aegis_retry_stress`): the X-05 stress (24 tasks x 3 rotated failure plans, plus a 16-task two-failing-tasks case) on `#[tokio::test(flavor = "multi_thread")]` under a 30 s guard with exact per-task/total call counts, attempt numbers, `AttemptRecord` numbering, `task_key`-ordered aggregation and one progress Waypoint per completed task; the kill-during-backoff pair under `start_paused` (abort in < 1 s virtual against a 60 s backoff, `Skipped { reason: "shutdown" }` re-listed on the Halted vanguard, exactly one Waypoint; resume completes at `attempt: 1` with an empty history and a two-Waypoint thread); and the timeout pair over a real `SqliteWaypointStore` (`Timeout(EngineRun)` never retried, `RunTimeoutExceeded { limit: 200ms }`; `Timeout(Run)` retried to three attempts, `NodeFailed`, the persisted `NodeError` equal to the outcome's).

## Task Commits

1. **Task 1: An additive per-Paladin failure counter on the shared mock port** -- `9d22da30` (test, RED: five tests fail to compile on the missing builder) -> `0d6a865b` (feat, GREEN)
2. **Task 2: Replace the Phase 23 stand-in with a real per-task retry in E2E-3** -- `89d8eb09` (test)
3. **Task 3: The X-05 stress, kill-during-backoff and engine-run-timeout integration tests** -- `c5a32c4c` (test) -> `35fb97a9` (style: two `needless_borrows_for_generic_args` fixes under `clippy -D warnings`)

**Plan metadata:** the `docs(25-12)` commit carrying this SUMMARY.

## Files Created/Modified

- `tests/helpers/mock_paladin_port.rs` -- `fail_paladin_until_attempt`, `PROVIDER`, the per-name threshold map and call-counter map, the precedence block and rustdoc, five new tests
- `tests/integration/e2e_muster_defer_order_test.rs` -- seam deleted; `PlannerNode` now carries its task list (`single_template()` / `one_template_per_task()`); `build_graph_with_a_template_per_task(Option<RetryPolicy>)`, `per_task_retry_policy()`, `expected_per_template_worker_outputs()`; `one_worker_recovers_by_real_per_task_retry`, `the_default_predicate_is_used`, `without_a_retry_policy_the_same_transient_failure_fails_the_run`; module rustdoc rewritten
- `tests/integration/aegis_retry_stress_test.rs` -- new: `wide_muster_graph`, `run_stress_round`, `flaky_graph`, `kill_during_backoff`, `slow_graph` and the six tests
- `Cargo.toml` -- `[[test]] name = "aegis_retry_stress"`

## Verification (all exit 0)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets --all-features` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 (after `35fb97a9`) |
| `cargo fmt --all -- --check` | 0 |
| `cargo test -p paladin-ai --lib` | 0 (552 passed) |
| `cargo test --doc -p paladin-ai` | 0 (115 passed, 17 ignored) |
| `cargo test --test e2e_muster_defer_order` | 0 (37 passed) |
| `cargo test --test aegis_retry_stress` | 0 (37 passed; the 6 plan tests green on 6 further consecutive runs) |
| `cargo test --test e2e_compensation_chain` | 0 (5 passed) |
| `cargo test --test unit` | 0 (431 passed, 11 ignored) |
| `cargo test --test subgraph_formation_in_campaign` | 0 (38 passed) |
| `cargo test --test e2e_crash_resume` | 0 (32 passed) |
| `cargo test --test e2e_approval_gate` | 0 (35 passed) |
| `cargo test --test golden_bridge_equivalence` | 0 (36 passed) |
| `cargo test --test multi_parley_suspension` | 0 (36 passed) |
| `cargo test --test parley_resume_stress` | 0 (34 passed) |
| `cargo test --test lib` | 0 (700 passed, 14 ignored) |
| `cargo test --test war_engine_tracer` | 0 (3 passed) |
| `cargo test --test waypoint_retention_fault_injection` | 0 (3 passed) |

Integration binaries touched or including the edited helper: `e2e_muster_defer_order` (edited), `aegis_retry_stress` (added), and -- via `#[path = "../helpers/mod.rs"]` -- `e2e_crash_resume`, `e2e_approval_gate`, `golden_bridge_equivalence`, `multi_parley_suspension`, `parley_resume_stress`, `subgraph_formation_in_campaign`, `lib`; `e2e_compensation_chain`, `unit`, `war_engine_tracer` and `waypoint_retention_fault_injection` were run as regression checks. The `cli` binary (which also includes the helper) requires `--features cli`; it was compiled by the `--all-features` check and clippy runs above but not executed. No crate under `crates/` was modified, so no per-crate `--lib`/`--doc` run beyond the root package was required.

Acceptance greps: `pub fn fail_paladin_until_attempt(`, `pub fn fail_until_attempt(`, `pub fn fail_paladin(` all present; `LlmFailure` (9) and `Some(503)` (5) present in the mock; `git diff HEAD~1 -- tests/helpers/mock_paladin_port.rs | grep -c '^-.*faulty_paladin_port_fail_until_attempt_then_succeeds'` = 0 at the GREEN commit; `PHASE 25 SEAM` count 0; no `END PHASE 25 SEAM`, `one_worker_recovers_by_manual_attempt_scripting`, `fail_until_attempt(2)` or `TransientAndUnknown` in the E2E file; `fail_paladin_until_attempt(` present there; `call_count()` asserted `7`; `attempts.len()` asserted `2`; `flavor = "multi_thread"` and a `Duration::from_secs` guard present in the stress file; `TimeoutKind::EngineRun` asserted by value; `record.attempt, 1` asserted; no `redis|docker|127.0.0.1:6379` match.

## Decisions Made

See `key-decisions` in the frontmatter. In short: the per-Paladin counter is second in precedence and advances on every call; E2E-3's recovering-worker fixture uses one template per task so `w3` is addressable; the recovering-worker policy shortens only the backoff interval; the paused-clock tests use the public `InMemoryWaypointStore`; both timeout-direction tests carry a retry policy to prove the EngineRun/Run asymmetry.

## Deviations from Plan

### Adapted (documented, not silent)

**1. [Rule 3 - Blocking] One worker template per task in the recovering-worker fixture**
- **Found during:** Task 2 (designing the replacement before writing it)
- **Issue:** The plan attaches the retry policy to "the worker template" (singular) and names the recovering task `w3`, but the engine passes every mustered task its TEMPLATE's own `Paladin` (`superstep.rs`, `execute_observed(&paladin, ..)`), so with one shared template every task is named `worker` -- `fail_paladin_until_attempt("worker", 2)` would fail whichever two dispatches ran first, not one task twice, and "w3's record" has no `node_id` to find it by.
- **Fix:** `build_graph_with_a_template_per_task` registers `w1`..`w5` as five Paladin worker templates, each carrying the per-task `Aegis`, mustered one task each against keys `a`..`e`; the three muster/defer/order tests keep the original single-template `build_graph()`. The post-block regression assertion keeps its shape and message ("all 5 results, in task_key order") against `expected_per_template_worker_outputs()` (`"FaultyPaladinPort: w<i> processed <key>"`), since the worker name is part of the mock's output.
- **Files modified:** `tests/integration/e2e_muster_defer_order_test.rs`
- **Verification:** `one_worker_recovers_by_real_per_task_retry` passes with every count exact; `the_default_predicate_is_used` checks all five templates' policies
- **Committed in:** `89d8eb09`

**2. [Rule 3 - Blocking] `initial_interval: 20 ms` on the recovering-worker policy**
- **Found during:** Task 2
- **Issue:** The plan writes `RetryPolicy { max_attempts: 3, ..Default::default() }`; the default 500 ms first backoff (with jitter, up to ~3 s over two retries) runs on the wall clock because the fixture drives a real `SqliteWaypointStore` -- `tokio::time::pause` is unsafe over sqlx's own timers.
- **Fix:** `per_task_retry_policy()` keeps `max_attempts: 3` and the default `retry_on`/`jitter`/`backoff_factor`/`max_interval`, and sets only `initial_interval: 20 ms`. The must-have truth (`RetryPolicy { max_attempts: 3, .. }` under the default predicate) holds as written.
- **Committed in:** `89d8eb09`

**3. [Rule 3 - Blocking] `InMemoryWaypointStore` instead of `RecordingWaypointStore`**
- **Found during:** Task 3
- **Issue:** The plan says to resume "from the `RecordingWaypointStore`", which is `pub(crate)` in `paladin-battalion` and unreachable from a root `tests/` binary.
- **Fix:** The paused-clock pair uses `paladin_storage::waypoint::in_memory::InMemoryWaypointStore` (public, no I/O, safe under `start_paused`) and asserts Waypoint counts through the port's own `history()`.
- **Committed in:** `c5a32c4c`

**4. [Scope, small] A negative control added to the E2E-3 file**
- `without_a_retry_policy_the_same_transient_failure_fails_the_run` (same fixture, `None` policy -> `RunOutcome::Failed`, exactly 5 calls, `w3` once). Not in the task list; added because T-25-59's mitigation is precisely "a green E2E-3 that proves nothing", and this is the cheapest direct proof the green run depends on the retry. `build_graph_with_a_template_per_task` took an `Option<RetryPolicy>` to support it.
- **Committed in:** `89d8eb09`

**5. [Wording] Module rustdoc sentence reworded** -- "Nothing here touches Docker, Redis or any live service" tripped the plan's case-insensitive `redis|docker` negative grep; reworded to "needs a live service or a container runtime". Committed in `c5a32c4c`.

**6. [Rule 1 - Lint] Two `needless_borrows_for_generic_args`** in the stress file under `clippy -D warnings` (`ThreadId::new(&format!(..))`). Committed in `35fb97a9`.

---

**Total deviations:** 3 blocking adaptations, 1 small scope addition, 1 wording fix, 1 lint fix. **Impact on plan:** every must-have truth and prohibition holds as written (no widened predicate, no pre-driven port, no relaxed count, global counter unchanged); the adaptations change fixture shape, not what is proven.

## TDD Gate Compliance

- Task 1 followed RED (`9d22da30`, five tests fail to compile on the missing builder) -> GREEN (`0d6a865b`).
- Tasks 2 and 3 have no separate RED commit: the engine behaviour they assert (per-task Muster retry, Interrupted re-listing, EngineRun/Run naming) already exists from plans 25-07 and 25-09, so a test written first passes at once. Their RED evidence is (a) Task 1's own RED, where the E2E binary could not build without the per-Paladin counter, and (b) the committed negative control, which fails the identical fixture without the policy. Each was committed as a single `test(25-12)` commit.

## Issues Encountered

- The root crate's `paladin::core` module does not re-export `transience`; the mock imports `paladin_core::platform::container::transience::Transience` directly (the E2E files' convention).
- `WORKER_NAMES.iter().zip(..)` yields `&&str`; one deref fixed at first compile.

## Known Stubs

None -- no placeholder values, skipped tests or unrun `<verify>` steps. Every test in this plan runs and passes locally without Docker or Redis.

## Threat Flags

None -- no new network endpoint, auth path, file access pattern or schema change; the only new persistent artifacts are per-test temporary SQLite files under `std::env::temp_dir()`, matching the existing E2E convention.

## For the orchestrator's attention (not added to deferred-items.md -- a sibling agent runs concurrently)

- `NodeExecutionRecord` for a mustered task carries no `task_key`: records of a SHARED worker template are distinguishable only by position or by their `attempt`/`attempts` values. An additive `#[serde(default)] task_key: Option<String>` (or a `MusterContext` echo) would let future E2E tests address one task of a shared template without giving every task its own template. Out of scope here (core `waypoint.rs` is not in this plan's files); candidate for a later Muster-observability pass.
- `RecordingWaypointStore` and the other engine doubles in `paladin-battalion/src/engine/test_support.rs` are `pub(crate)`; root E2E tests count Waypoints through `history()` instead of `save_call_count()`. Fine today; if a future E2E needs save-call observation, a `test-support` feature exposing the module would be the smallest change.

## Next Phase Readiness

- Plan 25-13 (node cache engine integration) and 25-14 (migration doc + guide) can cite `cargo test --test e2e_muster_defer_order` and `cargo test --test aegis_retry_stress` as the acceptance evidence for FT-FR-06/07/10 and X-05.
- `FaultyPaladinPort::fail_paladin_until_attempt` is available to every root test binary that includes `tests/helpers/`.

## Self-Check: PASSED

- Files: `tests/helpers/mock_paladin_port.rs`, `tests/integration/e2e_muster_defer_order_test.rs`, `tests/integration/aegis_retry_stress_test.rs`, `Cargo.toml` all present in the worktree.
- Commits `9d22da30`, `0d6a865b`, `89d8eb09`, `c5a32c4c`, `35fb97a9` all present on `worktree-agent-aafdf11fc75e34ca2` above base `a14ac5ff`.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-06*
