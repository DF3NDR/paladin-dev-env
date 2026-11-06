---
phase: 57-register-hygiene-close-out
plan: 14
subsystem: treasurer-allowance-notices
tags: [treasurer, allowance-notice, cancellation-safety, drop-guard, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-10 triage (owner option-a) that sent 41-REVIEW WR-01 to fix-in-phase as a correctness item"
provides:
  - "A failed operator-delivery enqueue gives the won allowance notice back (TreasuryNoticePort::discard), so the next admission in the window wins it again (41 WR-01)"
  - "Treasurer::confirm and ::abandon run in a detached task, so a dropped caller (client disconnect) does not cancel them, on both the submission path and the HTTP agent routes"
  - "A private AdmissionSettlement drop guard in run/submission.rs settles an admission whose submission future was dropped: confirm if the run row exists, otherwise give back"
  - "WINDOWS row 97 (41-REVIEW WR-01), open, for 57-11 to mark fixed on this plan's tests and commits"
  - "MIGRATION 11.1 row M-B-44, a CHANGELOG [Unreleased] Fixed bullet and the platform-api.md delivery-semantics sentence"
affects: [57-11]

tech-stack:
  added: []
  patterns:
    - "Cancellation-safe port methods: build an owned unit of work, spawn it through tokio::runtime::Handle::try_current() (inline fallback), await the JoinHandle"
    - "A private Drop guard that spawns its compensating settlement task, armed on a won claim and disarmed by settle()"

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md
    - src/application/services/treasurer/mod.rs
    - src/application/services/treasurer/tests.rs
    - src/application/services/run/submission.rs
    - docs/src/api-reference/platform-api.md
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "PD-14-1: give-back lives in confirm only; notify_operator (TreasurerSpendGuard's mid-run notices) keeps log-and-swallow because the warning is also the run's trace event and herald line"
  - "PD-14-2: cancellation safety lives in the Treasurer (run_detached helper), so the agent routes are covered with no change to crates/paladin-web"
  - "PD-14-3: an unreadable run row is given back rather than kept; a possible second notice is preferred to a lost one"

patterns-established:
  - "A won once-per-window notice is delivered or given back on every path but a crash between claim and settlement and a drop inside admit itself (EA-57-15)"

requirements-completed: [REG-05, REG-02]

duration: ~41min
completed: 2026-11-04
---

# Phase 57 Plan 14: Allowance notice give-back and cancellation safety Summary

**A won once-per-window operator allowance notice is no longer lost silently: a failed delivery enqueue gives it back so the next admission wins it again, `Treasurer::confirm` and `abandon` finish in a detached task even when the caller is dropped, and a dropped run submission settles its admission through an `AdmissionSettlement` drop guard.**

## Performance

- **Duration:** ~41 min (two executors: Task 1 and its checkpoint, then Tasks 2 and 3)
- **Started:** 2026-11-04T19:43Z (plan-start SHA `df16136f25a5aea8b3f6b944717391d6cb084577`)
- **Completed:** 2026-11-04T20:25Z
- **Tasks:** 3 (1 tracer, 1 tdd auto, 1 auto)
- **Files modified:** 7

## Accomplishments

- 41-REVIEW WR-01, enqueue failure: `enqueue_operator_delivery` is a private associated function `(target, notice) -> bool`, returning `false` at each of its three failure points (payload serialization, notice thread id, queue `enqueue`) with unchanged logging. A private `deliver_or_give_back` helper enqueues each notice, collects the ids whose enqueue failed and calls `discard` on exactly those; a discard failure logs at `error` with the notice count only; no target means no-op. `confirm` calls it. `notify_operator` keeps its signature and log-and-swallow behaviour (PD-14-1) and `guard.rs` is untouched.
- 41-REVIEW WR-01, cancellation: `confirm` and `abandon` build an owned unit of work (clones of the notice store, the operator target and the admission's notices) and run it through a private `run_detached` helper, which spawns on `tokio::runtime::Handle::try_current()`, awaits the `JoinHandle`, logs a `JoinError` with the notice count only and runs inline when there is no runtime handle. Dropping the caller drops the handle, not the task.
- `run/submission.rs`: a private `struct AdmissionSettlement` (`new`, `async fn settle(self, persisted: bool)`, `impl Drop`) is built in `admit_and_persist` right after a successful admission. Dropped while armed, it spawns a task that reads the run row: `Ok(Some(_))` confirms, `Ok(None)` or `Err` abandons, with `Err` logging a `warn` naming the run id and the error only. With no runtime handle it logs at `error` that the admission for that run id could not be settled.
- Treasurer module docs replace the crash-only sentence with the new guarantees and name the two residual windows (EA-57-15): a process crash between claim and settlement, and a drop inside `admit` itself. They also state why mid-run guard notices keep log-and-swallow.
- No public item changed: `git diff df16136f2 -- crates/ src/application/services/treasurer/guard.rs` is empty, `make api-surface` reports unchanged (4331 items), and the `paladin-ai` semver run reports `0 failure(s)`.
- Records: `platform-api.md` delivery semantics, MIGRATION 11.1 `M-B-44`, one `[Unreleased]` `### Fixed` bullet. `# 10.` onward in MIGRATION.md and `## [0.11.0]` onward in CHANGELOG.md are byte-identical to the pre-commit versions (D-00c).

## Task Commits

1. **Task 1 (tracer): 41 WR-01 enqueue failure end to end** - `e4563971a` (fix)
2. **Task 2: confirm/abandon survive a dropped caller; a dropped submission settles its admission** - `78951d7ec` (fix)
3. **Task 3: API docs, MIGRATION and CHANGELOG records** - `6fa191bc9` (docs)

## WINDOWS rows minted

| Id | Description starts | File:line | Kind / phase | Status |
|----|--------------------|-----------|--------------|--------|
| 97 | `41-REVIEW WR-01 (found live by the Phase 57 sweep, D-14): a won once-per-window operator allowance notice is lost with no recovery when enqueue_operator_delivery fails ...` | `src/application/services/treasurer/mod.rs:251` | `deviation` / `41` | `open` |

Minted before the fix commit (EA-57-14 guard: `windows status` showed no earlier `41-REVIEW WR-01` row, so no duplicate) and stays `open`; 57-11 marks it `fixed` on this plan's tests and commits.

## Red-green

Treasurer tests are `application::services::treasurer::tests::<name>`; submission tests are `application::services::run::submission::tests::<name>`.

| Test | RED on the pre-fix head | GREEN |
|------|-------------------------|-------|
| `enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it` | `exactly the notices whose enqueue failed are given back` / `left: []` / `right: ["01a1a817-a1af-7503-8ead-d0525e8ceac0", "01a1a817-a1af-7503-8ead-d06a773f0d04"]` (nothing was given back) | passed |
| `confirm_completes_when_its_caller_is_dropped_mid_enqueue` | `the delivery is enqueued although the caller of confirm was dropped` / `left: 0` / `right: 1` | passed |
| `a_submission_dropped_before_its_row_is_written_gives_the_notice_back` | `no run row exists, so the dropped submission gives its admission back` / `left: (1, 0, 0)` / `right: (1, 0, 1)` | passed |
| `a_submission_dropped_after_its_row_is_written_confirms_the_notice` | `the run row exists, so the dropped submission confirms its admission` / `left: (1, 0, 0)` / `right: (1, 1, 0)` | passed |

Head RED commands: `cargo test --lib --features web-server enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it`, `... _completes_when_its_caller_is_dropped` (0 passed, 1 failed) and `... a_submission_dropped_` (0 passed, 2 failed). After the fixes they pass 1 of 1, 1 of 1 and 2 of 2.

Suites after the fixes: `application::services::treasurer::` 85 passed, 0 failed (84 after Task 1, plus the cancellation test); `application::services::run::submission` 38 passed, 0 failed (the existing `(1, 1, 0)` and `(1, 0, 1)` counts at `submit` return are unchanged); `cargo test -p paladin-web --lib agent_controller` 71 passed, 0 failed.

## Gates

`df -BM --output=avail /` was printed before the builds (8515M at Task 2, then 8461M, 8460M and 8280M for Task 3). Every cargo run used `CARGO_INCREMENTAL=0`.

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --lib --tests --features web-server -- -D warnings` | exit 0 (`Finished dev profile`) |
| `make clean-code` | exit 0 (doc-check passed, ADR-0033; all 105 D-05 public API entry points carry a plural `# Examples` heading) |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-ai` | `paladin-ai: 223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `make security` | exit 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| `make check-gates` | exit 0 (12 crates in publish order; version strings equal `0.11.0`; stub-panic scan `0 finding(s)`) |
| `git diff df16136f2 -- crates/ src/application/services/treasurer/guard.rs` | empty (P-14-2, PD-14-1) |

## Evidence for 57-11

- **41 WR-01:** WINDOWS row **97** (open, kind `deviation`, phase `41`); fix commits `e4563971a` (Task 1, enqueue-failure give-back) and `78951d7ec` (Task 2, cancellation safety and the submission drop guard); records commit `6fa191bc9` (Task 3). Tests: `application::services::treasurer::tests::enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it`, `application::services::treasurer::tests::confirm_completes_when_its_caller_is_dropped_mid_enqueue`, `application::services::run::submission::tests::a_submission_dropped_before_its_row_is_written_gives_the_notice_back`, `application::services::run::submission::tests::a_submission_dropped_after_its_row_is_written_confirms_the_notice`. Commands: `cargo test --lib --features web-server enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it`, `cargo test --lib --features web-server _completes_when_its_caller_is_dropped`, `cargo test --lib --features web-server a_submission_dropped_`. Residual windows (a crash between claim and settlement, a drop inside `admit`) are documented in the treasurer module docs (EA-57-15), not closed.

## Deviations from Plan

None - plan executed exactly as written. Two small notes, neither a deviation: the plan's Task 2 step 4 refers to the "module docs (`treasurer/mod.rs` :15-21)", which were updated there (the `submission.rs` module docs needed no change, so the `admit_and_persist` rustdoc names the drop-guard behaviour instead); and the Task 2 test doubles wrap the in-memory repository and queue rather than reimplementing them.

## Auth gates

None.

## Known Stubs

None. `ParkedDeliveries`, `ParkingRepository` and `ParkingQueue` are test-only doubles by design.

## Threat Flags

None. The change adds no endpoint, auth path, file access or schema change. The new log lines name the run id, the notice count and the underlying error only; none interpolates the webhook URL, a secret or an API key (P-14-4, T-57-50). Detached tasks are one short task per dropped admission with no retry loop (T-57-51, accepted).

## Issues Encountered

None.

## Self-Check: PASSED

- FOUND: `struct AdmissionSettlement` and `impl Drop for AdmissionSettlement` in `src/application/services/run/submission.rs`
- FOUND: `Handle::try_current` with an inline fallback in `src/application/services/treasurer/mod.rs`
- FOUND: the four new tests in `treasurer/tests.rs` and `run/submission.rs`
- FOUND: MIGRATION.md `M-B-44` and the CHANGELOG.md bullet citing `41-REVIEW WR-01`; `platform-api.md` contains `given back`
- FOUND: WINDOWS row 97, `open`
- FOUND: commits `e4563971a`, `78951d7ec` and `6fa191bc9`
