---
phase: 42-mid-run-halt-sse-terminal-status
plan: 02
subsystem: treasurer
tags: [tracer, allowance, mid-run-halt, spend-guard, war-engine, run-worker, treasurer]

requires:
  - phase: 42-mid-run-halt-sse-terminal-status
    provides: ADR-0057 mid-run halt contract (plan 42-01) -- the check-only boundary, the typed HaltCause and the reason-tagged halt_reason this tracer builds
  - phase: 41-admission-time-allowance-enforcement
    provides: Treasurer facade, AllowancePolicy and ceiling order, TreasuryLedgerPort::balance, AllowanceRefusal, treasury_notices and the operator webhook target
  - phase: 27-platform-api
    provides: CancellationProbe at the superstep boundary, RunWorkerPool per-run engine build, map_outcome, the SQLite run repository and GET /v1/runs/{id}
provides:
  - HaltReason (core, reason-tagged serde, one wire_json builder) and AllowanceRefusal::details_json plus Eq
  - SpendGuard output port with SpendDecision and the NeverHalts default (paladin-ports)
  - HaltCause and RunOutcome::Halted.cause; WarEngine::with_spend_guard and the top-of-loop boundary check after the token and the probe
  - Treasurer::evaluate (the one ceiling evaluation admit and the guard share), TreasurerSpendGuard, Treasurer::spend_guard
  - RunWorkerPool::with_treasurer attaching a per-run guard for every run that records a submitter; map_outcome reading the typed cause
  - build_run_api building the notice store and the Treasurer before the pool and sharing one Arc<Treasurer>
  - engine_spend_halt_tracer -- the phase's end-to-end proof through the real router, worker and on-disk SQLite
  - MIGRATION.md 9.2 rows, the CHANGELOG [Unreleased] entry and the refreshed public-API baseline for the above
affects: [42-03, 42-04, 42-05, 42-06, 42-07, 42-08, 42-09, 42-10, 42-11, 42-12]

tech-stack:
  added: []
  patterns:
    - "Policy in the facade, mechanism in the engine: the superstep loop consults a port (SpendGuard beside CancellationProbe) and never imports a ceiling, policy, ledger or currency type"
    - "One function per rule: Treasurer::admit and TreasurerSpendGuard::check call the same Treasurer::evaluate, so admission and the boundary cannot drift"
    - "Typed cause at the source: RunOutcome::Halted carries HaltCause so the worker maps status from the engine's own answer instead of re-querying flags"
    - "Tracer first: config -> Treasurer -> port -> engine -> worker -> SQLite -> HTTP proven in one plan before any expansion plan starts"

key-files:
  created:
    - crates/paladin-ports/src/output/spend_guard.rs
    - src/application/services/treasurer/evaluate.rs
    - src/application/services/treasurer/guard.rs
  modified:
    - crates/paladin-core/src/platform/container/allowance.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-eval/src/runner.rs
    - examples/graceful_shutdown.rs
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/run/worker.rs
    - src/infrastructure/web/run_api_wiring.rs
    - src/application/services/run/http_surface_tests.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .project/current-exports.txt

key-decisions:
  - "The boundary check is a check-only balance read through the shared Treasurer::evaluate; no reserve row is written and settlement.rs and the ledger migrations are untouched (ADR-0057 D-01, D-00b)"
  - "A spend halt maps to Halted with error None whatever the cancel and shutdown flags say; CancelRequested maps to Cancelled; the in-process Token cause keeps today's precedence (cancel flag, then drain requeue, then Halted)"
  - "A probe cancel and a spend halt at the same boundary resolve to CancelRequested (cancel wins); an engine with no guard attached makes no guard call"
  - "TreasurerSpendGuard memoises its first Halt so a run that has already halted never re-reads the ledger for the same decision"
  - "The four 9.2 rows are N/A (types absent at the v0.9.0 baseline) or N (additive facade methods); no allowlist entry was needed and check-migration-allowlist.sh stays set-equal"

patterns-established:
  - "Register-only closing task per plan: MIGRATION.md 9.2, CHANGELOG and the api-surface baseline land in one commit after the code"

requirements-completed: [ALLOW-03]

coverage:
  - id: D1
    description: "An exhausted allowance halts an in-flight graph run at its next superstep boundary end to end (config -> Treasurer -> SpendGuard -> WarEngine -> RunWorkerPool -> SQLite run store -> GET /v1/runs/{id} answers halted with error null, Halted Waypoint with vanguard [n1], nodes n1 and n2 never ran, no key value in the body)"
    requirement: "ALLOW-03"
    verification:
      - kind: integration
        ref: "src/application/services/run/http_surface_tests.rs#engine_spend_halt_tracer"
        status: pass
    human_judgment: false
  - id: D2
    description: "The engine consults an attached SpendGuard exactly once per boundary after the token and the probe, halts through the existing Halted-Waypoint path with HaltCause::Spend, lets a cancel win over a spend halt at the same boundary, and makes no guard call when none is attached"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib spend_guard (4 tests: halt on boundary 2, halt on boundary 1, never-halts, cancel wins)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-battalion --lib cancellation_probe (4 tests unchanged)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Treasurer::admit and TreasurerSpendGuard::check share one ceiling evaluation (same order, one truncated store-clock read, balance >= ceiling), with every pre-existing Treasurer test unchanged"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib --features web-server application::services::treasurer (43 tests)"
        status: pass
    human_judgment: false
  - id: D4
    description: "map_outcome maps HaltCause::Spend to Halted with error None regardless of flags, CancelRequested to Cancelled and Token to today's precedence; with_treasurer attaches a guard only for a run that records a submitter"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --lib --features web-server application::services::run::worker (56 tests)"
        status: pass
    human_judgment: false
  - id: D5
    description: "HaltReason serializes under a reason tag and wire_json produces the Phase 41 429 details object plus reason; AllowanceRefusal::details_json round-trips; doc tests for every new public item pass"
    requirement: "ALLOW-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib allowance (31) and --doc allowance (16); cargo test -p paladin-ports --doc spend_guard (3); cargo test -p paladin-battalion --doc with_spend_guard (1); cargo test -p paladin-ai --doc spend_guard (1) and --doc with_treasurer (1)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The tracer's surface is registered: four MIGRATION.md 9.2 rows, the CHANGELOG [Unreleased] entry, the refreshed public-API baseline; the set-equality and api-surface gates pass"
    requirement: "ALLOW-03"
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh (exit 0); PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface (exit 0 after api-surface-update)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The tracer is proven red before green: the pool built without with_treasurer completes the same run (status completed) instead of halting"
    requirement: "ALLOW-03"
    verification: []
    human_judgment: true
    rationale: "The red path lives in run_spend_halt_tracer_once(attach_treasurer = false); it was run by the executor at the Task 2 tracer gate and reported to the operator (status completed, no halt), but no committed test invokes it, so the verifier should confirm the red run from the gate record rather than from a test name"

duration: ~58min
completed: 2026-10-06
status: complete
---

# Phase 42 Plan 02: Mid-run halt tracer -- SpendGuard, HaltCause, shared Treasurer evaluation and the end-to-end halt Summary

**An allowance exhausted while a graph run is in flight now halts that run at its next superstep boundary through config, the Treasurer's shared ceiling evaluation, the new `SpendGuard` port, the `WarEngine` top-of-loop check, the run worker and the real SQLite run store, with `GET /v1/runs/{id}` answering `halted` and no error, the Halted Waypoint kept as the restart point, and the whole new surface registered.**

## Checkpoint decision

**Task 2 tracer gate: `verified`.** The operator accepted the end-to-end proof (status `halted`, Waypoint `Halted` with vanguard `[n1]`, node counters `[1, 0, 0]`, the red run recorded) and approved Task 3 and the closeout. Selected by the operator on 2026-10-06 at the plan 42-02 tracer gate; Task 3 ran in the same session after a context continuation.

## Performance

- **Duration:** ~58 min (16:59Z plan 42-01 closeout to 17:57Z Task 3 commit; Tasks 1-2 by the executor, Task 3 and the closeout by the continuation)
- **Started:** 2026-10-06T16:59Z
- **Completed:** 2026-10-06T17:57Z (code); summary and state updates follow
- **Tasks:** 3 (Task 1 auto/TDD, Task 2 tracer with an operator gate, Task 3 auto)
- **Files modified:** 19 (3 created), 1,685 insertions, 138 deletions

## Accomplishments

- **The halt mechanism, policy-free in the engine.** `paladin-ports` gained `SpendGuard` (`check(&ThreadId) -> SpendDecision { Continue, Halt(HaltReason) }`) with the `NeverHalts` default beside the cancellation probe; the superstep loop consults an attached guard once per boundary, after the `CancellationToken` and the `CancellationProbe`, and on `Halt` writes the same `WaypointStatus::Halted` Waypoint the cancel path writes and returns `RunOutcome::Halted { cause: HaltCause::Spend(reason), .. }` without dispatching any node of that superstep. `superstep.rs` imports no `AllowancePolicy`, `BalanceQuery`, `TreasuryLedgerPort` or `Ceiling` (the acceptance grep prints nothing).
- **The typed cause.** `HaltCause { CancelRequested, Token, Spend(HaltReason) }` is `#[non_exhaustive]`; the compiler-forced pattern updates in `paladin-eval`, `examples/graceful_shutdown.rs`, the graph test helper and the worker landed in the same commit. `HaltReason { AllowanceExhausted(AllowanceRefusal), LedgerUnavailable }` is serde-tagged `reason`, and `wire_json()` is the one caller-facing builder (the Phase 41 `429` details object plus `reason`), with `AllowanceRefusal::details_json()` factored out for it.
- **One evaluation for admission and the boundary.** `Treasurer::evaluate` (new `evaluate.rs`) is the single ceiling evaluation; `admit` now calls it (its own `for ceiling in &ceilings` loop is gone) and `TreasurerSpendGuard` (new `guard.rs`) answers `check` from it, memoising its first `Halt`. Every pre-existing `treasurer::tests` case passes unchanged (43).
- **Worker and wiring.** `RunWorkerPool::with_treasurer` attaches `treasurer.spend_guard(attribution, run_id)` to the per-run engine beside `with_cancellation_probe` and `with_treasury_ledger`, only when the run row records a submitter; `map_outcome` reads the cause (`Spend` -> `Halted` with `error: None` whatever the flags, `CancelRequested` -> `Cancelled`, `Token` -> today's precedence). `build_run_api` now builds the notice store and the `Treasurer` before the pool and hands one `Arc<Treasurer>` to the pool and, coerced, to the submission service.
- **The tracer.** `engine_spend_halt_tracer` deserializes `treasurer.allowance.api_keys.svc-h: { period: "1d", amount: "1.00" }`, builds the real router over on-disk SQLite, submits a three-node chain whose first node settles exactly `1.00 USD` under `(acme, svc-h)`, runs `run_once`, and asserts `GET /v1/runs/{id}` is `halted` with `error: null`, the latest Waypoint is `Halted` with vanguard `[n1]`, the counters read `[1, 0, 0]`, and the body carries no key value. It re-runs once if the UTC day window rolls over mid-scenario.
- **The surface, registered.** Four `MIGRATION.md` 9.2 rows (battalion `RunOutcome`/`HaltCause`/`with_spend_guard`, ports `SpendGuard`, core `HaltReason`/`AllowanceRefusal`, facade `with_treasurer`/`spend_guard`/`TreasurerSpendGuard`), one CHANGELOG `[Unreleased]` bullet, and the public-API baseline refreshed under the pinned nightly (16 added lines, all `with_treasurer`, `spend_guard` and `TreasurerSpendGuard`).

## Task Commits

Each task was committed atomically:

1. **Task 1: HaltReason, SpendGuard port, engine boundary check with a typed cause** - `5c4c756` (feat)
2. **Task 2: shared Treasurer evaluation, TreasurerSpendGuard, worker attachment and map_outcome, reordered build_run_api, engine_spend_halt_tracer** - `95672fd` (feat); deferred-items log `086601a` (docs)
3. **Task 3: MIGRATION.md 9.2 rows, CHANGELOG entry, refreshed public-API baseline** - `c933eae` (docs)

## Files Created/Modified

- `crates/paladin-ports/src/output/spend_guard.rs` - the `SpendGuard` port, `SpendDecision`, `NeverHalts`, doc tests
- `crates/paladin-ports/src/output/mod.rs` - `pub mod spend_guard`
- `crates/paladin-core/src/platform/container/allowance.rs` - `HaltReason` (`as_str`, `wire_json`), `AllowanceRefusal::details_json`, `Eq`
- `crates/paladin-battalion/src/engine/mod.rs` - `HaltCause`, `RunOutcome::Halted.cause`, `with_spend_guard`, the guard threaded to the loop, `spend_guard_tests`
- `crates/paladin-battalion/src/engine/superstep.rs` - the top-of-loop guard check, `ChildEngineResources.spend_guard`, pattern sites
- `crates/paladin-battalion/src/engine/graph.rs`, `crates/paladin-eval/src/runner.rs`, `examples/graceful_shutdown.rs` - `..`/`cause` pattern updates for the new field
- `src/application/services/treasurer/evaluate.rs` - `Evaluation`, `CeilingReading`, `Treasurer::evaluate`
- `src/application/services/treasurer/guard.rs` - `TreasurerSpendGuard` (`impl SpendGuard`, memoised first halt), `Treasurer::spend_guard`
- `src/application/services/treasurer/mod.rs`, `tests.rs` - `admit` over `evaluate`; re-exports
- `src/application/services/run/worker.rs` - `with_treasurer`, per-run guard attachment, cause-aware `map_outcome` and its tests
- `src/infrastructure/web/run_api_wiring.rs` - Treasurer and notice store built before the pool; one shared `Arc<Treasurer>`
- `src/application/services/run/http_surface_tests.rs` - `engine_spend_halt_tracer` and `run_spend_halt_tracer_once`
- `MIGRATION.md`, `CHANGELOG.md`, `.project/current-exports.txt` - the registers (Task 3)
- `.planning/phases/42-mid-run-halt-sse-terminal-status/deferred-items.md` - two out-of-scope discoveries (see Issues Encountered)

## Verification

Re-run at the closeout on the Task 3 tree (`c933eae`):

- Task 1 verify: `cargo test -p paladin-ai-core --lib allowance` 31 passed; `--doc allowance` 16 passed; `cargo test -p paladin-ports --doc spend_guard` 3 passed; `cargo test -p paladin-battalion --doc with_spend_guard` 1 passed; `--lib spend_guard` 4 passed; `--lib cancellation_probe` 4 passed; `cargo check --workspace --all-targets --all-features` exit 0.
- Task 2 verify: `cargo test -p paladin-ai --lib --features web-server application::services::treasurer` 43 passed; `application::services::run::worker` 56 passed; `engine_spend_halt_tracer` 1 passed; `cargo test -p paladin-ai --doc spend_guard` 1 passed; `--doc with_treasurer` 1 passed; `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0; `cargo fmt --all --check` exit 0. `run_api_wiring`: 24 of 26 pass; the remaining 2 are the pre-existing sandbox-only network cases logged in `deferred-items.md` (see Issues Encountered).
- Task 3 verify: `./scripts/check-migration-allowlist.sh` exit 0 (set-equal); the 9.2 section contains `HaltCause` and matches `RunOutcome|SpendGuard|HaltReason|with_treasurer` on 6 lines; `[Unreleased]` contains `ALLOW-03` and `SpendGuard`; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` exit 0 after `make api-surface-update`; the baseline contains `with_treasurer` (4) and `TreasurerSpendGuard` (12).
- Acceptance greps: every file-content criterion of Tasks 1-3 holds (port items 3/3, `pub mod spend_guard`, `HaltReason`/`tag = "reason"`/`wire_json`/`details_json` 4/4, `Eq` on `AllowanceRefusal`, `HaltCause`/`with_spend_guard`/`spend_guard_tests` 3/3, no policy types in `superstep.rs`, `async fn evaluate`, `impl SpendGuard for TreasurerSpendGuard` + `OnceLock`, `self.evaluate(` in `admit` and no `for ceiling in &ceilings`, `with_treasurer`/`with_spend_guard(`/`HaltCause::Spend` in the worker, `Treasurer::new(` at line 781 before `RunWorkerPool::new(` at line 794, `git diff --stat b7f934e..HEAD -- settlement.rs migrations` empty).
- Manual credential-handling review: no new log line, error, trace event or response interpolates a key value; the tracer asserts the run body never contains `halt-key`, and a tree grep for that literal outside the test prints nothing.

## Decisions Made

- Followed the plan as written for the mechanism, the policy sharing and the wiring order.
- 9.2 rows: `RunOutcome`/`HaltCause`/`with_spend_guard`, `SpendGuard`, `HaltReason`/`AllowanceRefusal` are `N/A` (absent at the v0.9.0 baseline) and the facade row is `N` (additive), so no allowlist entry was added and the set-equality check stays green.
- The CHANGELOG bullet names the full new public surface per crate so later Phase 42 plans can extend one entry rather than add parallel ones.

## Deviations from Plan

- **Task 3 acceptance "the baseline contains `with_spend_guard`" cannot hold literally.** `.project/current-exports.txt` is the `paladin` facade crate's `cargo-public-api` listing; it never enumerates `paladin-battalion`'s inherent methods (the committed pre-plan baseline likewise contains no `with_cancellation_probe` and no `AllowanceRefusal`, 0 hits each, although both are public). The refreshed baseline contains the facade-level items the plan added (`with_treasurer`, `spend_guard`, `TreasurerSpendGuard`); `WarEngine::with_spend_guard` is registered on the 9.2 row and in the CHANGELOG instead. No baseline line was hand-edited.
- **The red run is a gate record, not a committed test.** `run_spend_halt_tracer_once(attach_treasurer = false)` is the red path and the executor ran it at the Task 2 gate (status `completed`, no halt, reported to the operator before the green run); no committed test invokes it. Coverage entry D7 routes that fact to the human verifier rather than claiming an automated proof.

## Issues Encountered

- `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` do not reach a terminal status in this sandbox ("the run never reached a terminal status" after the 10 s poll): they submit an agent-kind run whose model call reaches a real provider with a hermetic fake key, and the sandbox has no direct network. Logged in `deferred-items.md` with the reproduction on the pre-plan tree; CI (network available) is the authority.
- The Tasks 1-2 commit trailers carry the executor dispatch's model name (`Claude Sonnet 5.5`); the Task 3 commit carries this session's (`Claude Fable 5.1`). Not amended (the branch is shared and pushed).

## Authentication Gates

None.

## Known Stubs

None. `TreasurerSpendGuard` is production code; `NeverHalts` is the documented explicit no-op default, not a stub.

## Threat Flags

- Prohibition "no role bypasses the boundary check": the guard is built from `RunAttribution` (tenant id and key name) and the run's id only; no role reaches `Treasurer::spend_guard` or `evaluate` (T-42 safety, mitigated).
- Prohibition "no API key value in any log, error, trace, reason or payload": `HaltReason::wire_json` carries the refusal figures only; the tracer's final assertion and the tree grep hold (T-42 privacy, mitigated).
- Accepted: the same-instant over-admission race stays open by ADR-0057 D-01 (check-only boundary); `deferred-items.md` notes the stale sentence in `treasurer/tests.rs`'s module doc for plan 42-12 to reword.

## Next Phase Readiness

Ready for 42-03 (migration `013` `runs.halt_reason`, the reason written before the status flips, `GET /runs` `halt_reason` + `final_waypoint_id` from `HaltReason::wire_json`, the caller `halted` webhook key). 42-03 reads `HaltCause::Spend(HaltReason)` off `RunOutcome::Halted` in `map_outcome` and `wire_json()` for the wire object; nothing in this plan needs reworking for it.

`requirements-completed` copies the plan's `requirements` field verbatim (`ALLOW-03`) as the summary contract requires; ALLOW-03 is not marked complete in `REQUIREMENTS.md` by this plan, because 42-03..42-09 still build its persisted reason, its ledger-unavailable and resume clauses and its agent-loop half.

## Self-Check: PASSED

- FOUND: `crates/paladin-ports/src/output/spend_guard.rs`, `src/application/services/treasurer/evaluate.rs`, `src/application/services/treasurer/guard.rs`
- FOUND: commits `5c4c756`, `95672fd`, `086601a`, `c933eae` in `git log`
- FOUND: `async fn engine_spend_halt_tracer` in `src/application/services/run/http_surface_tests.rs`; `pub fn with_spend_guard` in `crates/paladin-battalion/src/engine/mod.rs`; `pub trait SpendGuard` in the port file
- Plan-level verification re-run on the final tree: all commands above exit 0 apart from the two documented sandbox-only `run_api_wiring` cases
