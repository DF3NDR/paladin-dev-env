---
phase: 50-provider-parity-config-driven-model-swap
plan: 28
subsystem: execution-service
tags: [rust, paladin, treasury-ledger, billed-usage, settlement, agent-loop, streaming, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-26 BilledUsage / LlmError::billed_usage; 50-27 every adapter bills its failures and PricingLlmAdapter prices them, buffered and streamed"
provides:
  - "Private CallFailure { error: PaladinError, bills: Vec<BilledUsage> } returned by execute_with_retry_and_temperature; the closure records e.billed_usage() beside to_paladin_error"
  - "Buffered agent loop folds a failed call's bills into the run usage and CostTally and settles each priced bill through settle_model_call (ordinal loop_num, bill model else paladin model), then returns the unchanged error"
  - "Streamed path settles a billed, priced terminal error once through settle_agent_loop_call (ordinal 1, model_used) before sending the converted error"
  - "Tests billed_empty_completion_is_settled (tracer), billed_failure_after_a_priced_call_settles_both_ordinals, unbilled_or_unpriced_failure_settles_nothing, billed_stream_failure_is_settled"
affects: [50-29, 50-30, 50-31]

tech-stack:
  added: []
  patterns:
    - "Carry a bill out of PaladinError-only machinery: a &mut slot captured by the circuit-breaker future, filled beside the error conversion"

key-files:
  created: []
  modified:
    - src/application/services/paladin/paladin_execution_service.rs

key-decisions:
  - "The bill rides beside the error inside the execution service (CallFailure), not on PaladinError: no public type changes (api-surface unchanged)"
  - "bills accumulates across attempts of one call; a billed error is Permanent, so today it holds at most one bill"
  - "Stream settles under model_used (the only model identity a stream carries), matching the priced terminal-chunk settle"
  - "execute_structured_call unwraps CallFailure to its error (A3-50-28); plan 50-29 reports its bill"

patterns-established:
  - "A failed call's bill settles through the same settle_agent_loop_call path and key shape as a success"

requirements-completed: [PARITY-05]

duration: 40min
completed: 2026-10-18
---

# Phase 50 Plan 28: Billed failures settle to the treasury ledger, buffered and streamed (G-50-5a) Summary

**Some calls end as `EmptyCompletion` or `Refusal` even though the vendor billed them and `PricingLlmAdapter`
priced them. In a run through `PaladinExecutionService`, such a call now settles to `TreasuryLedgerPort` once,
through the same `settle_agent_loop_call` path and `(run_id, ordinal, 1)` key as a success. The caller still gets
the unchanged `to_paladin_error` output.**

- Tracer: a scripted port fails with a billed `EmptyCompletion` priced at 2_403_000 nanos USD on model `m`, under
  `with_treasury_ledger(InMemoryTreasuryLedger, EveryCall)`.
  - `execute` returns `Err(LlmFailure { transience: Permanent, .. })`, and its text matches `to_paladin_error`.
  - The ledger holds exactly one settlement: 2_403_000 nanos, breakdown key `m`, under the execution's run.

## Performance

- **Duration:** about 40 min (2026-10-18 20:33Z to 21:13Z).
- **Tasks:** 2 of 2, two commits.
  - Task 1 was the tracer. Its gate passed in autonomous mode: after the commit, the tracer `<verify>` was re-run
    on the clean committed tree and passed. The other two Task 1 verify commands had passed on that identical tree
    just before the commit. "Tracer verified end-to-end - expanding."
- **Files modified:** 1

## Accomplishments

- **Red first, both tasks.**
  - Task 1: `billed_empty_completion_is_settled` failed with `exactly one settled row`, left 0 / right 1. Its
    error-shape assertions already passed. `billed_failure_after_a_priced_call_settles_both_ordinals` failed with
    `one settle per call ordinal`, left 1 / right 2. The negative control `unbilled_or_unpriced_failure_settles_nothing`
    passed, as it should.
  - Task 2: `billed_stream_failure_is_settled` failed with `exactly one settled row: []`. The delta and the error
    were already delivered.
- **`CallFailure`:** a private `#[derive(Debug)]` struct, beside `AgentLoopLedgerTarget`, with rustdoc (A1-50-28).
- **`execute_with_retry_and_temperature`** now returns `Result<LlmResponse, CallFailure>`.
  - Each attempt captures `&mut Option<BilledUsage>` in the circuit-breaker future. The future sets it from
    `e.billed_usage().cloned()` beside `to_paladin_error(&e)`, and the attempt's bill is appended to `bills`.
  - All four error returns carry `bills`: circuit open, `Permanent`, policy-declined, and max retries.
  - The `# Errors` rustdoc describes the new return.
- **Buffered loop:** the `?` became an explicit match. On `Err(failure)`, each bill is handled in turn:
  - `usage += bill.usage`, and `cost_tally.record_call(bill.cost)`.
  - When `ledger_target` is set and the bill is priced, it goes to `settle_model_call(target, loop_num, bill model
    or paladin model, cost)`.
  - One `debug!` line follows, with the execution id, the loop and the run total tokens. Then `failure.error` is
    returned unchanged.
- **`execute_structured_call`** and the three helper test call sites now map `CallFailure` to its `.error`.
- **Stream:** in the spawned task's `Err` arm, a bill with a cost is settled first. It goes through
  `settle_agent_loop_call(ledger, scope, run_id, 1, &model_used, cost)` when `stream_settlement` is set. Then the
  converted error is sent, and the task returns as before. The `execute_stream_inner` rustdoc was updated.

## Task Commits

1. **Task 1 (tracer): billed failures settle in the buffered agent loop**: `d3472e73` (feat)
2. **Task 2: billed stream failures settle once**: `890aa3b3` (feat)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clippy `type_complexity` on the Task 1 test stub**
- **Found during:** the Task 2 gate run (`cargo clippy --workspace --all-targets --all-features -- -D warnings`).
- **Issue:** `ScriptedOutcomeLlmPort`'s field and `new` parameter types tripped `clippy::type_complexity`, which
  made `make clean-code` exit 2.
- **Fix:** a `type ScriptedOutcome<S>` alias, test-only. It is committed in `890aa3b3`, so commit `d3472e73` alone
  does not pass clippy. Both commits are local and unpushed.
- **Commit:** `890aa3b3`

### Other deviations

- **Extra tracer-slice tests.** The plan's tracer had three behaviours: one call, two loops, and the no-bill / no-cost
  controls. They are split into three named tests rather than one.
- **Folding is not observable yet.** On the buffered path, the folded `usage` and `cost_tally` are local to the loop
  and dropped with the returned error. Only the settlement and the `debug!` line are observable today. Plan 50-29
  reports the folded spend to the engine.

## Verification

All exit 0 unless stated:

- **Tests:**
  - `cargo test --lib billed_stream_failure_is_settled`: passed.
  - `cargo test --lib billed_`: 4 passed.
  - `cargo test --lib paladin_execution_service`: 91 passed.
  - `cargo test --test provider_swap_matrix --features llm-all`: 51 passed.
  - `cargo test --workspace --lib --no-fail-fast` (default features): 13 binaries, 5102 passed, 2 failed, so exit
    101. The two failures are the ledgered baseline (WINDOWS row 73): `build_run_api_persists_no_run_traces_by_default`
    and `build_run_api_persists_run_traces_when_trace_persist_is_set`.
- **Lint and format:**
  - `cargo fmt --all --check`: clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean after the fix above. The first run
    failed on the test-stub type.
  - `make clean-code`: exit 0 after the fix.
- **Gates:**
  - `make check-gates`: OK.
  - `make api-surface`: unchanged (4298 items). No public item changed.
  - `make check-consumer-surface`: passed.
- **`make security`: exit 2, baseline.** It was run during 50-27, and this plan changes no dependency or
  `Cargo.lock`. It reports exactly RUSTSEC-2026-0367/0368/0369 (`zip 3.0.0`) plus the allowed warnings.
- **Acceptance:** `grep -c 'struct CallFailure'` gives 1. The tracer asserts exactly one ledger row, with the billed
  nanos and the breakdown key.
- **`unwrap(` / `expect(` / `panic!(` in the diff:** all 14 additions sit at line 7865 or later, inside the
  `#[cfg(test)]` modules `streamed_cost_tests` (starting at 7680) and `agent_loop_cost_tests` (starting at 8009).
  Non-test code adds none.
- **Manual credential-handling review (settle and error paths):**
  - The new settle calls go through the existing `settle_agent_loop_call`. Its logs carry the run id, ordinal, nanos
    and currency only (the 39-05 rule).
  - The one new `debug!` line carries the execution id, the loop number and a token count. It never carries the
    prompt, response content, key id or a credential.
  - `BilledUsage` holds a model id, counts and a `Cost` only.
  - The error the caller sees is unchanged.
  - No HTTP client, header or redirect policy changed.
- **Not run:** Docker-bound integration tests and live vendor calls. Neither is needed: the change is internal to
  the execution service and is exercised by in-memory ledger tests.
- **D-02:** a bill's `TokenUsage` keeps `total == prompt + completion`. It is never recomputed here, and its
  `AddAssign` into the run usage is the same as for a success.

## Known Stubs

None.

## Threat Flags

None. Against the register:
- T-50-28-01 (mitigated): buffered and streamed settles, asserted on the ledger.
- T-50-28-02 (mitigated): one settle per ordinal under the existing key, and a billed failure is `Permanent`, so it
  is never retried. LEDGR-04 mode rules are unchanged: the stream still settles only in `EveryCall`, and the
  buffered path goes through `ledger_target`.
- T-50-28-03 (mitigated): the 39-05 log rule is kept.

## Flagged assumptions

- A1-50-28 held: `bills` is a `Vec`, holding at most one bill today.
- A2-50-28 held: LEDGR-04 is unchanged. `PlatformRunsOnly` with no run id yields no `ledger_target`, so an engine
  node's failure does not settle here.
- A3-50-28 held: the structured call only unwraps to its error.

## Self-Check: PASSED

- `src/application/services/paladin/paladin_execution_service.rs` exists, and contains `struct CallFailure` and
  `billed_empty_completion_is_settled`.
- Commits `d3472e73` and `890aa3b3` are on `claude/laughing-dirac-e0h2ax`.
