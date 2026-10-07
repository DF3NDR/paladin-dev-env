---
phase: 42-mid-run-halt-sse-terminal-status
reviewed: 2026-10-07T00:00:00Z
depth: standard
files_reviewed: 93
files_reviewed_list:
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/settlement.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-core/src/platform/container/allowance.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-core/src/platform/container/herald.rs
  - crates/paladin-core/src/platform/container/run.rs
  - crates/paladin-core/src/platform/container/run_scope.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-eval/src/assertion.rs
  - crates/paladin-eval/src/runner.rs
  - crates/paladin-eval/tests/assertion_snapshots.rs
  - crates/paladin-herald/src/json_herald.rs
  - crates/paladin-herald/src/markdown_herald.rs
  - crates/paladin-herald/src/table_herald.rs
  - crates/paladin-ports/src/input/allowance_admission_port.rs
  - crates/paladin-ports/src/input/run_submission_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/run_repository_port.rs
  - crates/paladin-ports/src/output/spend_guard.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-ports/src/output/treasury_notice_port.rs
  - crates/paladin-storage/src/run/contract_tests.rs
  - crates/paladin-storage/src/run/in_memory.rs
  - crates/paladin-storage/src/run/postgres.rs
  - crates/paladin-storage/src/run/sqlite.rs
  - crates/paladin-storage/src/run_trace/contract_tests.rs
  - crates/paladin-storage/src/run_trace/in_memory.rs
  - crates/paladin-storage/src/run_trace/postgres.rs
  - crates/paladin-storage/src/run_trace/sqlite.rs
  - crates/paladin-storage/src/treasury/in_memory.rs
  - crates/paladin-storage/src/treasury/mod.rs
  - crates/paladin-storage/src/treasury/notice_contract_tests.rs
  - crates/paladin-storage/src/treasury/postgres.rs
  - crates/paladin-storage/src/treasury/sqlite.rs
  - crates/paladin-storage/src/webhook/contract_tests.rs
  - crates/paladin-storage/src/webhook/in_memory.rs
  - crates/paladin-storage/src/webhook/postgres.rs
  - crates/paladin-storage/src/webhook/sqlite.rs
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/error.rs
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/src/schedule_controller.rs
  - crates/paladin-web/tests/openapi_golden_v0_9.rs
  - examples/graceful_shutdown.rs
  - src/application/services/paladin/middleware/context.rs
  - src/application/services/paladin/middleware/limits.rs
  - src/application/services/paladin/paladin_execution_service.rs
  - src/application/services/run/cancel.rs
  - src/application/services/run/cancel_tests.rs
  - src/application/services/run/events.rs
  - src/application/services/run/http_surface_tests.rs
  - src/application/services/run/mod.rs
  - src/application/services/run/schedule/service.rs
  - src/application/services/run/schedule/tests.rs
  - src/application/services/run/stream_tests.rs
  - src/application/services/run/submission.rs
  - src/application/services/run/webhook/mod.rs
  - src/application/services/run/webhook/service.rs
  - src/application/services/run/webhook/tests.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/application/services/treasurer/derive.rs
  - src/application/services/treasurer/evaluate.rs
  - src/application/services/treasurer/guard.rs
  - src/application/services/treasurer/mod.rs
  - src/application/services/treasurer/tests.rs
  - src/infrastructure/telemetry/herald_sink.rs
  - src/infrastructure/telemetry/otel_sink.rs
  - src/infrastructure/telemetry/persisting_sink.rs
  - src/infrastructure/web/agent_host.rs
  - src/infrastructure/web/facade_provisioner.rs
  - src/infrastructure/web/run_api_wiring.rs
  - tests/cli/eval_run_test.rs
  - tests/integration/otel_transport_test.rs
  - tests/treasurer_vocabulary_guard.rs
  - crates/paladin-storage/migrations/postgres/013_add_run_halt_reason.sql
  - crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql
  - crates/paladin-storage/migrations/sqlite/013_add_run_halt_reason.sql
  - crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql
  - crates/paladin-web/openapi.json
  - docs/src/api-reference/platform-api.md
  - docs/src/getting-started/configuration.md
  - .project/current-exports.txt
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-web/CHANGELOG.md
findings:
  critical: 1
  warning: 5
  info: 7
  total: 13
status: issues_found
---

# Phase 42: Code Review Report

**Reviewed:** 2026-10-07
**Depth:** standard
**Files Reviewed:** 93
**Status:** issues_found

## Summary

Reviewed the Phase 42 diff (`8df25467..HEAD`) against ADR-0057. Production paths received full
diff-level reading: the engine superstep boundary and `HaltCause` plumbing, worker `map_outcome` /
`run_agent` / `derive_dispatch_budget`, `Treasurer::evaluate` / `derive` / `guard`, the three run
adapters plus the treasury notice adapters and migrations 013/014, `RunEventBusSink` / `replay_stream`
/ `terminal_payload`, the HTTP controllers, wiring and the shared-service `TokenBudget` mode. Tests,
CHANGELOGs, docs and the OpenAPI golden were skimmed for contract drift and test reliability, not
line-audited. The 7 `spend_guard` engine unit tests were run and pass. A broader root-crate run could
not be completed because another cargo process held the build lock, so no other runtime verification
was performed.

Most of the contract is implemented as written. These parts checked out:
- Priority order: probe cancel, then token, then spend.
- Sticky memoised halt.
- `i128` derivation with dearest-axis pricing and tightest-wins.
- Single `wire_json` builder.
- Outcome written before status for `Halted` and `Cancelled`.
- Placeholder arithmetic in the three `runs` INSERTs and the 014 arbiter column lists.
- No credential material in any new log, error or payload.

One finding falls short of the contract's stated bound (CR-01), and four weaken the guard or its
surfaces in specific edge cases (WR-01 through WR-05).

## Critical Issues

### CR-01: A nested `Battalion` child's boundary guard is blind to its own run's spend, so the "one superstep" overshoot bound does not hold

**File:** `crates/paladin-battalion/src/engine/superstep.rs:2234-2270` (guard call), `crates/paladin-battalion/src/engine/settlement.rs:167-200` (child hook), `crates/paladin-battalion/src/engine/superstep.rs:3509-3513` (only the top-level hook settles)
**Issue:** ADR-0057 D-01 and the module docs promise a ceiling is overshot by at most one superstep's
spend. A `NodeSpec::Battalion` child run inherits the same guard (G11), and its own boundaries do call
`guard.check`. But the child carries a `SpendHook::child` whose `settle_boundary` returns at once
(`self.ledger` is `None`), and its spend folds into the parent's accumulator, which is only written at
the parent's own boundary. The ledger therefore contains none of the child run's own spend (and none of
the parent's in-flight superstep) while the child's N inner boundaries are evaluated. The child guard
can only detect spend by other runs. A parent superstep containing a child graph of up to its own
`max_supersteps` LLM-calling supersteps can overspend by the whole child graph, not one superstep. The
test `child_battalion_halt_on_spend_halts_the_parent` uses a stub guard that halts on call count, so it
proves propagation of a halt but not that a real spend is ever seen from inside a child. WINDOWS rows
65 to 67 do not cover this.
**Fix:** Either make the guard see in-flight spend, or state the weaker bound.
- Preferred: let the child's boundary settle its own accumulated charge. For example, give the child
  hook the ledger and a derived settlement key such as `(child_run_id, child_superstep, attempt)`, so
  the ledger is current at each child boundary and the parent does not double-charge.
- Alternative: pass the unsettled accumulator total to the guard, for example
  `SpendGuard::check(&self, thread, in_flight: Option<&Cost>)`, and have `Treasurer::evaluate` compare
  `balance + in_flight >= ceiling`.
- Minimum: add a WINDOWS.md row, correct the ADR, `platform-api.md` and rustdoc wording ("one
  top-level superstep, including any nested Battalion run it contains"), and add a test with a real
  `TreasurerSpendGuard` over a spending child graph.

## Warnings

### WR-01: A failed or skipped settlement silently blinds the guard (no fail-closed on the write side)

**File:** `crates/paladin-battalion/src/engine/settlement.rs:195-250` (`settle_boundary`), with `src/application/services/treasurer/guard.rs:check`
**Issue:** D-03 fails closed when the ledger cannot be read. The guard is only as good as the writes
that feed `balance`, and a settlement write failure (`Err(e)`) or a `CurrencyMismatch` charge is logged
and dropped (pre-Phase-42 D-08 behaviour, deliberately "never halts a run"). Phase 42 makes the ledger
authoritative for halting, so a run whose settlements keep failing while `balance` reads succeed (for
example a write-path permission or constraint problem, or a price table currency that differs from the
policy) is never halted. It spends unmetered with only `error` log lines as evidence. This is the
opposite of the fail-closed posture D-03 sets for the same principal.
**Fix:** Surface the failure to the guard path. For example, have `SpendHook::settle_boundary` return
or record a `SettlementHealth` flag that the engine passes into `SpendGuard::check`. When a guard is
attached and settlement failed, or the superstep had a charge that could not be written, answer
`Halt(HaltReason::LedgerUnavailable)`. At minimum, document the gap in WINDOWS.md as an accepted
exception.

### WR-02: `map_outcome` records a caller-cancelled run as `Halted` with an allowance reason when the cancel flag is set

**File:** `src/application/services/run/worker.rs` (the `HaltCause::Spend(reason)` arm of `map_outcome`, roughly lines 519-535; test `map_outcome_spend_halt_ignores_a_cancel_flag`)
**Issue:** ADR-0057 says cancel wins over spend and that `map_outcome` keeps re-querying
`is_cancel_requested` as defence in depth so it "never regresses to reading only the cause". The Spend
arm ignores `cancel_requested` entirely. A cancel written by another instance is only seen by the
debounced DB probe, so it can easily miss the boundary at which the guard halted. In that window the
row ends `Halted` with a `halt_reason`, the webhook says `halted`, and the SSE `done` says `halted`.
The caller who asked for a cancel receives a spend halt, and the status contradicts the documented rule
"a caller cancel is recorded `cancelled`". The test pins this behaviour, so it looks deliberate.
**Fix:** In the Spend arm, when `cancel_requested` is true, return the `Cancelled` transition (no
`halt_reason`) and keep `shutting_down` ignored. Update `map_outcome_spend_halt_ignores_a_cancel_flag`
accordingly. The `RunFinished` the engine already emitted would still say `Halted` plus reason, so also
confirm the live-vs-row agreement the invariant test claims, or document this narrow window.

### WR-03: A replay can end at a stale drain `RunFinished` even when the run continued afterwards

**File:** `src/application/services/run/events.rs:707-760` (`replay_terminal_override`), `813-840` (`replay_stream` sets `state.finished = true` on the first `Emit`)
**Issue:** A drain persists `RunFinished { Halted, halt_reason: None }` into `run_traces` (only the bus
sink filters it, not the persisting sink), then the run is requeued and a later attempt appends more
records and finishes. Once the run row is terminal, the replay hits that stale record first.
`replay_terminal_override` sees a terminal row and returns `Emit`. The stream then emits `done` with
the row's status (for example `completed`) at the drain point and stops, so the subscriber never
receives the later attempt's node events, and `done` is ordered before events that happened earlier
than it in reality. The Skip rule added in this phase fixes only the non-terminal-row case.
**Fix:** Only treat a replayed `RunFinished` as the terminal event when no later record exists. For
example, peek the next page (`port.read(thread, record.seq, 1)`) before setting `finished`. Or skip a
reason-less `Halted` record whose status disagrees with a row that is `Completed` or `Failed`, since
the pre-phase caller-cancel convergence (G10) only needs the `Cancelled` case.

### WR-04: One run row with an unreadable `halt_reason` makes `GET /runs` (list) and the run's own `get` fail

**File:** `crates/paladin-storage/src/run/sqlite.rs:319-327`, `crates/paladin-storage/src/run/postgres.rs:301-310` (row decode), test `a_corrupt_stored_halt_reason_is_a_serialization_error_not_a_panic`
**Issue:** Row decoding turns any `halt_reason` that does not deserialize into `HaltReason` into
`RunRepositoryError::Serialization`. `HaltReason` is `#[non_exhaustive]`, so a later phase adding a
variant makes older readers fail on those rows. During a rolling deploy or a rollback the same happens.
The decode is shared by `get`, `get_active_for_thread` and `list`, so a single such row fails every
`GET /v1/runs` page it appears in, the run inspector, the worker's `get` for that run and the SSE
degraded poller. The test asserts that a list containing one bad row errors out wholesale. A purely
informational column should not be able to take the listing down.
**Fix:** Decode tolerantly. On a `HaltReason` parse failure, `log::error!` with the run id (never the
payload) and set `halt_reason = None`, or use a custom deserializer with an `Unknown` fallback. Keep
the strict error only where a write is concerned. Update the corrupt-row test to assert the run still
loads.

### WR-05: The guard's in-run notice memo is set before the store result, so one transient store error permanently drops that run's mid-run warning

**File:** `src/application/services/treasurer/guard.rs` (`claim_mid_run_warnings`, `first_attempt` ahead of `claim_notice`), `src/application/services/treasurer/mod.rs` (`claim_record` maps `Err` to `None`)
**Issue:** `first_attempt(key)` inserts the claim key into the in-run `HashSet` before the notice-store
write. `claim_record` collapses `Err(store error)` into `None`, the same value as `AlreadyRecorded`. If
the first write fails transiently (a pool timeout, a brief outage), the memo already says "tried", so
this run never retries for that ceiling and window. If no other run in the window wins the claim, the
operator is never warned. The doc comment says the memo "only skips writes the store would answer
`AlreadyRecorded` to", which is false for the failure case. The same pattern in `claim_spend_halt` is
harmless only because the run ends.
**Fix:** Have `claim_record` return a tri-state (`Won`, `AlreadyRecorded`, `Failed`). In the guard,
insert the key into the memo only for `Won` and `AlreadyRecorded`, and leave it unmemoised on `Failed`
so the next boundary retries. Add a test with a failing notice store that succeeds on the second
boundary.

## Info

### IN-01: A derived-budget halt persists `balance == ceiling` as if it were the measured balance

**File:** `src/application/services/treasurer/derive.rs` (`derive_from`, the `Some(max_tokens)` arm), consumers in `worker.rs` (`persist_agent_halt`), `events.rs`, `agent_controller.rs`
**Issue:** Per ADR A5 the halt figures report `balance` equal to the ceiling because the loop cannot
know the post-spend balance. The dearest-axis token cap is pessimistic, so the real ledger balance is
usually below the ceiling. The object is serialised as `reason: "allowance_exhausted"` onto the durable
`runs.halt_reason`, the webhook, the SSE `done` and the informational stream `done`. A client or
operator reading `GET /runs/{id}` is told the allowance is exhausted when a fork resume will in fact be
admitted. The ADR accepts this as a conservative bound.
**Fix:** Optional, to avoid a wrong durable fact. Persist the figures actually read at derivation time
(the real balance at the start of the run) plus a boolean such as `"bound": "conservative"`, or
document prominently in `platform-api.md` that an agent-kind or agent-route `halt_reason.balance` is a
bound, not a reading. The current docs only say so for the derivation.

### IN-02: The truncation notice says "final answer" for an allowance halt

**File:** `src/application/services/paladin/middleware/limits.rs:62-63` (`TOKEN_BUDGET_NOTICE`, appended at 196)
**Issue:** The same constant is appended for `StopReason::AllowanceHalted`: "Token budget reached — the
response above is this run's final answer." For a halted run the output is partial and the run is a
resume point, so the text misleads end users reading the output.
**Fix:** Append a variant for the allowance case (for example "Allowance reached — this response is
partial; the run was halted."). Keep `TOKEN_BUDGET_NOTICE` for the operator budget, and extend the tests
that assert the exact text.

### IN-03: `HaltReason::herald_line` mislabels a `Window` refusal that has no window as a lifetime cap

**File:** `crates/paladin-core/src/platform/container/allowance.rs` (`herald_line`, the `match (refusal.limit_kind, refusal.window)` fallthrough `_ => "lifetime cap"`)
**Issue:** `(Window, None)`, which is only reachable through a malformed or deserialised refusal, renders
as "lifetime cap". It is the wrong claim, though harmless in practice.
**Fix:** Match `(AllowanceLimitKind::Lifetime, _) => "lifetime cap"` and
`(AllowanceLimitKind::Window, None) => "window"`, or render the limit kind string.

### IN-04: `run_agent` and `map_outcome` have grown, with four near-identical `RunFinished` literals and added `too_many_arguments` allows

**File:** `src/application/services/run/worker.rs:1520-1790` (`run_agent`), `src/application/services/paladin/paladin_execution_service.rs:1328,1527` (`#[allow(clippy::too_many_arguments)]`)
**Issue:** `run_agent` is now roughly 270 lines. It builds the same `TraceEvent::RunFinished { ... }`
literal in four branches (halt-at-dispatch, unpriced, derived-halt, completed/failed). Two internal
functions in the execution service gained a clippy suppression because the derived budget became yet
another positional argument. This is maintainability debt that makes the "one terminal event per run"
invariant harder to audit.
**Fix:** Add a small `emit_run_finished(dispatcher, status, halt_reason, duration_ms)` helper for the
agent path, and bundle the per-call arguments of the two execution-service functions into a context
struct instead of suppressing the lint.

### IN-05: `paladin-eval` cannot assert or match a `Cancelled` finish

**File:** `crates/paladin-eval/src/assertion.rs:420-430` (`run_status_matches`), `crates/paladin-eval/src/scenario.rs` (`RunStatusValue`)
**Issue:** `RunFinishStatus::Cancelled` is new and `#[non_exhaustive]`, but `run_status_matches` has no
arm for it and `RunStatusValue` has no `Cancelled` value. Eval attaches no cancellation probe today, so
it is unreachable now. Any scenario that later cancels would report a mismatch no assertion can
express.
**Fix:** Add `RunStatusValue::Cancelled` and the matching arm now, or record the gap in the eval
CHANGELOG.

### IN-06: The ALLOW-05 vocabulary guard is a whole-word, case-sensitive match

**File:** `tests/treasurer_vocabulary_guard.rs:70-100` (`contains_whole_word`, `is_downstream_path`)
**Issue:** A downstream fixture that names a type `TreasurerLedger` or uses a lower-case `treasurer`
passes the scan, because the guard requires non-identifier characters on both sides of the exact token
`Treasurer`. The stated intent is that the framework officer word never leaks into downstream code.
**Fix:** Use a case-insensitive word-prefix match (`Treasurer*`) for the downstream paths, or document
that only the bare officer token is policed.

### IN-07: A run row can expose a stale outcome if `update_status` fails after the reordered `record_outcome`

**File:** `src/application/services/run/worker.rs:1404-1420` (the halting transition: `record_outcome` then `update_status`)
**Issue:** The G14 reorder writes `final_waypoint_id`, `halt_reason` and `output` before the status
flips. If the following `update_status` fails (a transient backend error), the worker returns an error
and the message is redelivered, but until the new attempt rewrites the outcome the still-`Running` row
reports a halted outcome (`halt_reason` set, `final_waypoint_id` set). This is a narrow, self-healing
window.
**Fix:** Optional. Document it beside the reorder, or have the retry path clear the outcome fields when
it re-dispatches a `Running` run that already carries a `halt_reason`.

---

_Reviewed: 2026-10-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
