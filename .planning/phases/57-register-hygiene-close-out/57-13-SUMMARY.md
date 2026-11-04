---
phase: 57-register-hygiene-close-out
plan: 13
subsystem: run-worker-agent-path
tags: [run-worker, terminal-event, spend-scope, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "57-10 triage (owner option-a) that sent 45-REVIEW WR-04 and WR-06 to fix-in-phase; 57-05 bounded_run_error rebinding inside persist_failure"
provides:
  - "run_agent emits its one terminal RunFinished only after the exit's durable writes and ack, on the completed, failed and both halt exits (45 WR-04)"
  - "run_agent installs a per-attempt SpendScope through ATTEMPT_SPEND; the Err arm's NodeFinished carries the spend billed before the failure (45 WR-06)"
  - "WINDOWS rows 95 (45-REVIEW WR-04) and 96 (45-REVIEW WR-06), both open, for 57-11 to mark fixed on this plan's tests and commits"
  - "MIGRATION 11.1 rows M-B-42 and M-B-43 and two CHANGELOG [Unreleased] Fixed bullets"
affects: [57-11]

tech-stack:
  added: []
  patterns:
    - "A private on_committed: impl FnOnce() + Send callback on persist_failure / persist_agent_halt, called after ack and before the webhook enqueue, never on the nack branch"
    - "Read the 50-29 ATTEMPT_SPEND carrier on the failure arm only; the success arm keeps PaladinResult figures (one source per outcome)"

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md
    - src/application/services/run/worker.rs
    - src/application/services/run/worker_tests.rs
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "PD-13-2: the two halt exits take the same on_committed callback as the failed exit, so no run_agent exit emits before its commit"
  - "PD-13-3: a failed status or outcome write on the success arm logs one warn! (run id and repository error only) and nacks with a 1 s delay, returning Ok(true); a failed ack still propagates the queue error"
  - "PD-13-4: partial spend crosses the error through the existing public ATTEMPT_SPEND carrier; no PaladinError field, no port change, crates/ diff empty"

patterns-established:
  - "Terminal wire events are emitted from the commit point, not before it, on every worker exit"

requirements-completed: [REG-05, REG-02]

duration: 26min
completed: 2026-11-04
---

# Phase 57 Plan 13: Agent terminal-event ordering and failed-run spend Summary

**`run_agent` now publishes its single terminal `RunFinished` only after the status write, outcome write and `ack` have succeeded (completed, failed and halted exits), and a failed agent run reports the usage and cost its loop was billed before failing, read from the existing `ATTEMPT_SPEND` carrier.**

## Performance

- **Duration:** ~26 min (two executors: Task 1 and its checkpoint, then Tasks 2 and 3)
- **Started:** 2026-11-04T18:53Z (plan-start SHA `4c3376e4a8881f982d33b995e3acfb2161b39530`)
- **Completed:** 2026-11-04T19:19Z
- **Tasks:** 3 (1 tracer, 1 tdd auto, 1 auto)
- **Files modified:** 5

## Accomplishments

- 45-REVIEW WR-04: the completed arm no longer emits before its writes. `update_status` and `record_outcome` run in one block; on success `ack`, then `emit_agent_run_finished(Completed)`, then the webhook enqueue. On a repository error one `log::warn!` names `run.run_id` and the repository error only (P-13-3), then the message is nacked with a 1 s delay and `Ok(true)` is returned with no terminal event.
- `persist_failure` and `persist_agent_halt` take a trailing `on_committed: impl FnOnce() + Send`, called after `ack` and before the webhook enqueue, never on the nack branch. The `run_agent` Err arm and both halt call sites pass closures that emit `RunFinished`; `record_engine_failure` passes `|| {}`, so the graph path is unchanged. Every exit still reaches `unbind_after_drain` (T-45-11). Rustdoc on `emit_agent_run_finished`, `persist_failure`, `persist_agent_halt` and `run_agent` states the new order.
- 45-REVIEW WR-06: `run_agent` creates `SpendScope::new()` and runs the traced agent call inside `ATTEMPT_SPEND.scope(spend.clone(), ..)`. The Err arm's `NodeFinished` takes `spend.snapshot()` usage and cost; the Ok arm still reads `result.usage` / `result.cost` (P-13-4). The `TokenUsage::default()` / `cost: None` pair is gone from the Err arm, and the `TokenUsage` import it needed was removed from `worker.rs`.
- No public item changed: `git diff 4c3376e4a -- crates/` is empty, `make api-surface` reports `unchanged` (4331 items), and the `paladin-ai` semver run reports `0 failure(s)`.
- MIGRATION 11.1 gains `M-B-42` and `M-B-43`; `[Unreleased]` `### Fixed` gains two bullets. `## 11.2` is byte-identical, `# 10.` onward in MIGRATION.md and `## [0.11.0]` onward in CHANGELOG.md are byte-identical to the plan-start commit (D-00c).

## Task Commits

1. **Task 1 (tracer): 45 WR-04 end to end** - `2d15a0bdd` (fix)
2. **Task 2: 45 WR-06 spend through the attempt scope** - `25536a97c` (fix)
3. **Task 3: MIGRATION and CHANGELOG records** - `226126c51` (docs)

## WINDOWS rows minted

| Id | Description starts | File:line | Kind / phase | Status |
|----|--------------------|-----------|--------------|--------|
| 95 | `45-REVIEW WR-04 (found live by the Phase 57 sweep, D-14): run_agent emits the terminal RunFinished (Completed at :1793, Failed at :1844) before update_status/record_outcome/ack ...` | `src/application/services/run/worker.rs:1793` | `deviation` / `45` | `open` |
| 96 | `45-REVIEW WR-06 (found live by the Phase 57 sweep, D-14): the run_agent Err arm emits NodeFinished with TokenUsage::default() and cost None, so a failed agent run reports zero usage ...` | `src/application/services/run/worker.rs:1838` | `deviation` / `45` | `open` |

Each was minted before its fix commit (EA-57-11 guard: `windows status` checked first, no duplicate) and stays `open`; 57-11 marks them `fixed` on this plan's tests and commits.

## Red-green

All tests are under `application::services::run::worker_tests::`.

| Test | RED on the pre-fix head | GREEN |
|------|-------------------------|-------|
| `agent_completion_whose_status_write_fails_emits_no_terminal_event_and_nacks` | `a failed status write nacks and returns Ok(true), got Err(Repository(Backend { source: "injected write failure" }))` | passed |
| `agent_completion_whose_outcome_write_fails_emits_no_terminal_event_and_nacks` | `a failed outcome write nacks and returns Ok(true), got Err(Repository(Backend { source: "injected write failure" }))` | passed |
| `agent_completion_whose_ack_fails_emits_no_terminal_event` | terminal count `left: 1 right: 0` | passed |
| `agent_failure_whose_persist_fails_emits_no_terminal_event_and_nacks` | terminal count `left: 1 right: 0` | passed |
| `agent_budget::agent_kind_halt_whose_outcome_write_fails_emits_no_terminal_event` | terminal count `left: 1 right: 0` (the event carried `halt_reason`) | passed |
| `failed_agent_run_reports_the_spend_it_incurred_before_failing` | `the failed NodeFinished carries the usage billed before the failure` / `left: TokenUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0, .. }` / `right: TokenUsage { prompt_tokens: 1000, completion_tokens: 2000, total_tokens: 3000, .. }` | passed |
| `failed_agent_run_with_no_billed_call_reports_zero_spend` | passed (guard, expected) | passed |

Head, WR-04: `cargo test --lib --features web-server _emits_no_terminal_event` failed 5 of 5; after the fix it passes 5 of 5. Head, WR-06: `cargo test --lib --features web-server failed_agent_run_` ran 3 tests (the two new plus the existing `failed_agent_run_node_finished_carries_no_reasoning`) and failed only the spend test; after the fix it passes 3 of 3.

After both fixes: `agent_kind_run_emits_exactly_one_terminal_event` 1 passed; `application::services::run::` 274 passed, 0 failed (272 after Task 1, plus the two Task 2 tests); `non_llm_failure_after_billed_calls_reports_their_spend` 1 passed; `buffered_calls_are_recorded_in_the_attempt_spend_scope` 1 passed.

## Gates

`df -BM --output=avail /` was printed before the builds (6520M, then 6241M). Every cargo run used `CARGO_INCREMENTAL=0`.

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --lib --tests --features web-server -- -D warnings` | exit 0 (`Finished dev profile`) after the clone_on_copy fix noted under Deviations |
| `make clean-code` | exit 0 (doc-check passed, ADR-0033; all 105 D-05 public API entry points carry a plural `# Examples` heading) |
| `make api-surface` | `API surface unchanged` (4331 items) |
| `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh --package paladin-ai` | `paladin-ai: 223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)` |
| `make security` | exit 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| `make check-gates` | exit 0 (12 crates in publish order; version strings equal `0.11.0`; stub-panic scan `0 finding(s)`) |
| `git diff 4c3376e4a -- crates/` | empty (no public item, PD-13-4) |

## Evidence for 57-11

- **45 WR-04:** WINDOWS row **95** (open, kind `deviation`, phase `45`); fix commit `2d15a0bdd` (Task 1); records commit `226126c51` (Task 3). Tests: `application::services::run::worker_tests::agent_completion_whose_status_write_fails_emits_no_terminal_event_and_nacks`, `application::services::run::worker_tests::agent_completion_whose_outcome_write_fails_emits_no_terminal_event_and_nacks`, `application::services::run::worker_tests::agent_completion_whose_ack_fails_emits_no_terminal_event`, `application::services::run::worker_tests::agent_failure_whose_persist_fails_emits_no_terminal_event_and_nacks`, `application::services::run::worker_tests::agent_budget::agent_kind_halt_whose_outcome_write_fails_emits_no_terminal_event`. Command: `cargo test --lib --features web-server _emits_no_terminal_event`. Records: MIGRATION `M-B-42`.
- **45 WR-06:** WINDOWS row **96** (open, kind `deviation`, phase `45`); fix commit `25536a97c` (Task 2); records commit `226126c51` (Task 3). Tests: `application::services::run::worker_tests::failed_agent_run_reports_the_spend_it_incurred_before_failing`, `application::services::run::worker_tests::failed_agent_run_with_no_billed_call_reports_zero_spend`. Command: `cargo test --lib --features web-server failed_agent_run_`. Service-level half: `non_llm_failure_after_billed_calls_reports_their_spend`. Records: MIGRATION `M-B-43`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Clippy `clone_on_copy` in a new Task 2 test helper**
- **Found during:** Task 3 (`cargo clippy --lib --tests --features web-server -- -D warnings`)
- **Issue:** the `failed_run_figures` helper added in Task 2 called `status.clone()` on `RunFinishStatus`, which is `Copy`.
- **Fix:** `*status`. Folded into the unpushed Task 2 commit with `git commit --amend --no-edit` so that commit stays clippy-clean; this is why Task 2's hash is `25536a97c`.
- **Files modified:** `src/application/services/run/worker_tests.rs`
- **Commit:** `25536a97c`

**2. [Rule 3 - Blocking] Borrow lifetime when keeping `ATTEMPT_SPEND.scope` on one line**
- **Found during:** Task 2 GREEN
- **Issue:** rustfmt split `ATTEMPT_SPEND\n.scope(` across lines, which the plan's key-link pattern `ATTEMPT_SPEND\.scope` would not match. Binding the traced future to a local left `&HeartbeatHandle::new()` as a dropped temporary (E0716).
- **Fix:** a named `let heartbeat = HeartbeatHandle::new();` and `let call = ATTEMPT_SPEND.scope(spend.clone(), traced_call).await;`. Behaviour is identical.
- **Files modified:** `src/application/services/run/worker.rs`
- **Commit:** `25536a97c`

Otherwise the plan was executed as written.

## Auth gates

None.

## Known Stubs

None. `SpendsThenFailsPaladinPort`, `FailingWriteRepository` and `AckNackQueue` are test-only doubles by design.

## Threat Flags

None. The change moves an existing emission behind the commit point and adds a spend read; it adds no endpoint, auth path, file access or schema change. The one new log line names the run id and the repository error only (P-13-3).

## Issues Encountered

None beyond the two small compile and lint fixes recorded above.

## Self-Check: PASSED

- FOUND: `src/application/services/run/worker.rs` (6 `on_committed` references; `ATTEMPT_SPEND.scope` present on one line)
- FOUND: `src/application/services/run/worker_tests.rs` (all seven new tests present)
- FOUND: MIGRATION.md `M-B-42` and `M-B-43`; CHANGELOG.md bullets citing `45-REVIEW WR-04` and `45-REVIEW WR-06`
- FOUND: WINDOWS rows 95 and 96, both `open`
- FOUND: commits `2d15a0bdd`, `25536a97c` and `226126c51`
