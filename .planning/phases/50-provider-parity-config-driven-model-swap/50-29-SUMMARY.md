---
phase: 50-provider-parity-config-driven-model-swap
plan: 29
subsystem: battalion-engine
tags: [rust, paladin, battalion, treasury-ledger, spend-scope, task-local, settlement, timeouts, structured-output, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-28 private CallFailure (a failed call's bills) folded and settled in the agent loop; 50-26/50-27 BilledUsage on billed EmptyCompletion / Refusal, priced by PricingLlmAdapter"
provides:
  - "paladin_core::platform::container::cost::IncurredSpend (record, usage, cost, calls, is_empty): the spend an operation incurred so far, CostTally rule for the cost"
  - "paladin_ports::output::spend_scope::{SpendScope, ATTEMPT_SPEND, record_incurred_spend, current_attempt_spend}: a per-attempt shared accumulator reached through a tokio task-local (the RUN_TRACE_EMITTER precedent)"
  - "race_attempt installs a fresh SpendScope per attempt outside the attempt future; a failed attempt reporting the neutral pair, and a deadline- or idle-cut attempt, report the scope's recorded spend; the structured arm reports the scope's total on Ok when non-empty"
  - "PaladinExecutionService records every call at its fold: buffered successes, CallFailure bills, every structured call (Ok and billed Err), and the streamed terminal chunk / billed terminal error inside the spawned task (scope captured before the spawn and re-installed)"
  - "Tests failed_paladin_attempt_records_its_incurred_spend (tracer), each_failed_retry_attempt_is_charged_once, unrecorded_failures_stay_neutral_and_successes_are_not_double_counted, timed_out_attempt_records_its_incurred_spend, structured_repairs_count_every_call, a_silent_structured_executor_keeps_raw_figures, buffered_calls_are_recorded_in_the_attempt_spend_scope, non_llm_failure_after_billed_calls_reports_their_spend, billed_failure_is_recorded_in_the_attempt_spend_scope, structured_exhaustion_reports_every_call, streamed_calls_are_recorded_in_the_attempt_spend_scope"
affects: [50-30, 50-31]

tech-stack:
  added: []
  patterns:
    - "Report spend through a task-local accumulator installed OUTSIDE the future that may be dropped, so a deadline cannot lose what the future recorded"
    - "Capture a task-local before tokio::spawn and re-install it inside the task (stream_trace_emitter pattern, now also for ATTEMPT_SPEND)"

key-files:
  created:
    - crates/paladin-ports/src/output/spend_scope.rs
  modified:
    - crates/paladin-core/src/platform/container/cost.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - src/application/services/paladin/paladin_execution_service.rs
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md

key-decisions:
  - "The carrier is an attempt spend scope (task-local), not a field on PaladinError: no PaladinError shape change, and it survives a dropped attempt future"
  - "A successful Paladin attempt keeps PaladinResult.cost (A1-50-29); only failed and timed-out attempts and the structured arm read the scope, so each outcome has one source"
  - "with_incurred_spend substitutes only an Err carrying the neutral pair; a DirectiveParse failure that already carries the result's own usage and cost passes through unchanged"
  - "The structured arm on Ok reports the scope's total when non-empty (every repair call was billed), else raw's figures"

patterns-established:
  - "Every fold of a call's usage and cost in the execution service is paired with record_incurred_spend"

requirements-completed: [PARITY-05]

duration: 58min implementation (2026-10-18 21:14Z to 22:12Z) plus closeout verification after a container restart
completed: 2026-10-18
---

# Phase 50 Plan 29: Failed, timed-out and structured Battalion attempts count every billed call (G-50-5a remainder (b)) Summary

**A Battalion engine attempt now reports the spend of every call its Paladin made and was billed for, however the
attempt ends: a non-LLM `PaladinError`, a billed or unbilled `LlmFailure`, `StructuredOutputInvalid` after repair
calls, or a deadline or idle bound that drops the attempt future. `NodeFinished` carries that usage and cost, the
superstep `SpendHook` records it once per attempt, and the boundary settle charges it (LEDGR-03/04). No public type
changed shape.**

- Tracer: a test port records usage U and cost C through `record_incurred_spend`, then fails with a non-LLM
  `PaladinError`. Run as an engine node with a spend hook and an in-memory ledger, the failed attempt's
  `NodeFinished` reports U and C and the boundary settles C once; with retries each failed attempt is charged once.

## Execution history

- A previous executor committed all three tasks and the changelog entries (`2ef8259f`, `01a86a12`, `62b36c51`,
  `e5846a1c`, pushed at `e5846a1c`), then was killed by a container restart before the final gate run and the
  closeout. This closeout read those commits, re-ran the plan's full `<verification>` and the CLAUDE.md gates on the
  committed tree (clean working tree, no edits before or during the runs), and found nothing to fix. No new code
  commit was needed.
- The red-first record and the tracer gate run belong to the killed session; its commit messages name the tests each
  task added. The tracer `<verify>` was re-run here on the committed tree and passed.

## Accomplishments

- **`IncurredSpend`** (`paladin-core` `cost.rs`): private usage sum, `CostTally` and call count; `record` (saturating
  usage sum, any unpriced call makes the cost unknown), `usage`, `cost`, `calls`, `is_empty`, doc-tested.
- **`spend_scope`** (`paladin-ports`, new module): `SpendScope` over `Arc<Mutex<IncurredSpend>>` (`new`, `record`,
  `snapshot`, poison recovered with `PoisonError::into_inner`), the `ATTEMPT_SPEND` task-local, `record_incurred_spend`
  (`false` with no scope) and `current_attempt_spend` (`None` with no scope). Module rustdoc names the
  `RUN_TRACE_EMITTER` precedent and shows how a direct `PaladinPort` caller installs its own scope (A4-50-29).
- **Engine (`superstep.rs`):** `race_attempt` creates a fresh scope per attempt and runs the attempt inside
  `ATTEMPT_SPEND.scope(..)`, holding a clone outside the future.
  - `with_incurred_spend`: an `Err` carrying the neutral pair reports the scope's snapshot instead.
  - Deadline and idle branches report `recorded_spend(&spend_scope)` (the future is dropped, the accumulator is not).
  - Structured arm: `Ok` reports `current_attempt_spend()` when non-empty, else `structured.raw`'s figures.
  - D-10 comments and the `race_attempt` / `NodeDispatchResult` rustdoc rewritten.
- **Execution service:** `record_incurred_spend` beside every fold: buffered success, each `CallFailure` bill,
  `execute_structured_call` (`Ok` response and each `Err` bill, closing A3-50-28), and in the streamed spawned task the
  terminal chunk's usage and cost and a billed terminal error (scope captured with `ATTEMPT_SPEND.try_with(Clone::clone)`
  before the spawn and re-installed; a missing scope stays missing).
- **Test doubles:** `SpendRecordingPaladinPort` / `AfterRecording` (including `Stall`), `SpendRecordingStructuredExecutor`
  / `StructuredEnding` in `engine/test_support.rs`; a failing `ExecutionMiddleware` in the root crate's tests.
- **Changelogs:** root, `paladin-core`, `paladin-ports` and `paladin-battalion` `Added` entries (`e5846a1c`).

## Task Commits

1. **Task 1 (tracer): failed engine attempts count the spend they incurred**: `2ef8259f` (feat)
2. **Task 2: timed-out and structured attempts count every billed call**: `01a86a12` (feat)
3. **Task 3: the execution service records every billed call for its attempt**: `62b36c51` (feat)
4. **Changelog entries for IncurredSpend and the attempt spend scope**: `e5846a1c` (docs)

## Deviations from Plan

### Auto-fixed Issues

None. The closeout gate run found nothing to fix.

### Other deviations

- **Extra tests beyond the plan's named ones:** `each_failed_retry_attempt_is_charged_once`,
  `unrecorded_failures_stay_neutral_and_successes_are_not_double_counted`, `a_silent_structured_executor_keeps_raw_figures`
  and `streamed_calls_are_recorded_in_the_attempt_spend_scope` split the plan's controls and the streamed path into
  their own tests.
- **Semver run shape:** the plan's Task 3 verify names CI's shape (`--default-features --baseline-version 0.9.0`); the
  closeout ran the D-27 shape against the published 0.10.1 (`--release-type minor`) for `paladin-ports`,
  `paladin-ai-core` and `paladin-battalion` instead, as the orchestrator directed. See Verification.
- **Streamed terminal chunk recorded only when it carries usage:** a usage-less terminal chunk records nothing (D-17:
  nothing billable is known), so its cost cannot reach the scope either. Pricing needs usage, so such a chunk never
  carries a cost today.

## Verification

All on the committed tree at `e5846a1c`, `CARGO_INCREMENTAL=0`. Exit 0 unless stated:

- **Tests:**
  - `cargo test -p paladin-ai-core`: 714 unit, 11 integration, 142 doc (38 ignored) passed.
  - `cargo test -p paladin-ports`: 265 unit, 207 doc (94 ignored) passed.
  - `cargo test -p paladin-battalion --lib`: 883 passed, including `failed_paladin_attempt_records_its_incurred_spend`,
    `timed_out_attempt_records_its_incurred_spend` and `structured_repairs_count_every_call`.
  - `cargo test --lib paladin_execution_service`: 96 passed, including
    `non_llm_failure_after_billed_calls_reports_their_spend`, `billed_failure_is_recorded_in_the_attempt_spend_scope`,
    `structured_exhaustion_reports_every_call`, `buffered_calls_are_recorded_in_the_attempt_spend_scope` and
    `streamed_calls_are_recorded_in_the_attempt_spend_scope`.
  - `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
- **Lint and format:** `cargo fmt --check` clean; `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  clean.
- **Gates:** `make check-gates` OK; `make check-consumer-surface` passed (1 test); `make api-surface` unchanged
  (4298 items, so no `api-surface-update`); `make check-api-examples` passed (105 entry points).
- **`make clean-code`:** exit 0 (fmt left the tree unchanged, clippy, shellcheck, `cargo check`, both rustdoc
  passes, 642 workspace doctests, the `# Examples` gate).
- **`cargo semver-checks` 0.50.0 vs the published 0.10.1, `--release-type minor`** (crate-wide allows active):
  - `paladin-ports`: 190 checks, all pass, "no semver update required".
  - `paladin-ai-core`: exit 1, 2 lints, both pre-existing and already recorded in MIGRATION.md 9.2:
    `enum_no_repr_variant_discriminant_changed` on `RunFinishStatus::AwaitingInput` (Phase 42 `Cancelled`) and
    `enum_struct_variant_field_added` on `TraceEvent::NodeFinished.cost`, `RunFinished.cost` (Phase 38) and
    `RunFinished.halt_reason` (Phase 42). `IncurredSpend` fired nothing; this plan does not touch `trace.rs`.
  - `paladin-battalion`: exit 1, 1 lint, pre-existing and recorded: `enum_struct_variant_field_added` on
    `RunOutcome::Halted.cause` (Phase 42).
  - So this plan adds no lint and registers nothing under D-27. CI's own `semver` job diffs against 0.9.0, where
    these new-in-0.10 types do not exist.
- **`make security`: exit 2, the known baseline.** `cargo audit` reports exactly RUSTSEC-2026-0367/0368/0369
  (`zip 3.0.0`) plus 9 allowed warnings, so make stops before `cargo deny`. This plan changes no dependency and no
  `Cargo.lock`.
- **`unwrap(` / `expect(` / `panic!(` added outside `#[cfg(test)]`:** none. Every addition is in `spend_scope.rs`'s
  test module, the engine's test modules or `test_support.rs` (test-only), or the root crate's
  `streamed_cost_tests` / `agent_loop_cost_tests`. The `current_attempt_spend` doc example uses `?`, not `expect`
  (`e5846a1c`).
- **Manual credential-handling review (scope, engine and service paths):**
  - `SpendScope` and `IncurredSpend` hold a `TokenUsage` sum, a `CostTally` and a call count only: no prompt text,
    response content, model id, key id or credential (T-50-29-04).
  - The plan adds no log line anywhere (checked over the whole diff).
  - The lock is recovered on poison, never unwrapped (T-50-29-03).
  - No HTTP client, header, redirect policy or response-body handling changed.
- **Not run:** `cargo test --workspace` beyond the named suites, Docker-bound integration tests and live vendor calls.
  The change is internal to the engine, the ports crate and the execution service and is exercised by in-memory
  ledger tests.

## Known Stubs

None.

## Threat Flags

None. Against the register:
- T-50-29-01 (mitigated): the scope outlives a dropped future; the failed, timed-out and structured arms read it; the
  tracer and timeout tests assert the settle.
- T-50-29-02 (mitigated): successes keep one source; a fresh scope per attempt; `each_failed_retry_attempt_is_charged_once`.
- T-50-29-03 (mitigated): `PoisonError::into_inner`, tested by `a_poisoned_lock_is_recovered_not_propagated`.
- T-50-29-04 (mitigated): usage and cost only.

## Flagged assumptions

- A1-50-29 held: success accounting is unchanged (`unrecorded_failures_stay_neutral_and_successes_are_not_double_counted`).
- A2-50-29 held: one fresh scope per attempt inside the node's own spawned task.
- A3-50-29 held: a child engine scopes its own attempts; the parent's attempt scope is unchanged by them.
- A4-50-29 held: callers outside the engine see no change; the module rustdoc shows the opt-in.

## Self-Check: PASSED

- `crates/paladin-ports/src/output/spend_scope.rs` exists and contains `pub static ATTEMPT_SPEND`.
- `crates/paladin-core/src/platform/container/cost.rs` contains `pub struct IncurredSpend`.
- `superstep.rs` contains `ATTEMPT_SPEND`; the execution service contains `record_incurred_spend`.
- Commits `2ef8259f`, `01a86a12`, `62b36c51` and `e5846a1c` are on `claude/laughing-dirac-e0h2ax`.
