---
phase: 42-mid-run-halt-sse-terminal-status
reviewed: 2026-10-07T14:00:00Z
depth: standard
files_reviewed: 93
files_reviewed_list:
  - .project/current-exports.txt
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/src/engine/graph.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-battalion/src/engine/mod.rs
  - crates/paladin-battalion/src/engine/settlement.rs
  - crates/paladin-battalion/src/engine/superstep.rs
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/src/platform/container/allowance.rs
  - crates/paladin-core/src/platform/container/execution_result.rs
  - crates/paladin-core/src/platform/container/herald.rs
  - crates/paladin-core/src/platform/container/run.rs
  - crates/paladin-core/src/platform/container/run_scope.rs
  - crates/paladin-core/src/platform/container/trace.rs
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-eval/src/assertion.rs
  - crates/paladin-eval/src/runner.rs
  - crates/paladin-eval/tests/assertion_snapshots.rs
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-herald/src/json_herald.rs
  - crates/paladin-herald/src/markdown_herald.rs
  - crates/paladin-herald/src/table_herald.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/input/allowance_admission_port.rs
  - crates/paladin-ports/src/input/run_submission_port.rs
  - crates/paladin-ports/src/output/mod.rs
  - crates/paladin-ports/src/output/run_repository_port.rs
  - crates/paladin-ports/src/output/spend_guard.rs
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-ports/src/output/treasury_notice_port.rs
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/migrations/postgres/013_add_run_halt_reason.sql
  - crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql
  - crates/paladin-storage/migrations/sqlite/013_add_run_halt_reason.sql
  - crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql
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
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/openapi.json
  - crates/paladin-web/src/agent_controller.rs
  - crates/paladin-web/src/error.rs
  - crates/paladin-web/src/run_controller.rs
  - crates/paladin-web/src/schedule_controller.rs
  - crates/paladin-web/tests/openapi_golden_v0_9.rs
  - docs/src/api-reference/platform-api.md
  - docs/src/getting-started/configuration.md
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
findings:
  critical: 0
  warning: 2
  info: 14
  total: 16
status: issues_found
---

# Phase 42: Code Review Report (iteration 2, HEAD d44ba469)

**Reviewed:** 2026-10-07
**Depth:** standard
**Files Reviewed:** 93
**Status:** issues_found

## Summary

This pass verified the five fix commits (22d1aacd CR-1, 3e4d08a6 WR-1, 2724a7bb WR-2, 6bd6744a WR-3,
44209d8e WR-4) against HEAD, re-evaluated the nine Info findings, and read the `9b7ee07a..HEAD` diff
again around the fix commits and the worker, derive, replay, cancel-probe, submission, agent-controller and
storage paths. I also ran the CR-1 and WR-4 engine tests
(`cargo test -p paladin-battalion --lib -- child_battalion spend_halt lost_settlement`): 8 passed.
The root-crate treasurer and stream tests were not re-run (disk budget); those two fixes were verified by
reading the code paths.

**Fix verification**

| Finding | Verdict on HEAD |
|---|---|
| CR-1 (halted nested child recorded as a completed node) | Resolved. `ChildHalted` is carried through `NodeRunOutcome`, recorded like `Interrupted`, pushed onto `aborted_node_ids`, never retried or handed to `on_error`, and the Halted Waypoint vanguard re-lists the Battalion node. Both the terminal and the with-successor shape pass and a resume runs the child's node exactly once. One cosmetic residual: IN-10. |
| WR-1 (child guard blind to own spend) | Resolved as documentation only, as the fix report states. The bound is now stated in ADR-0057, the `SpendGuard` rustdoc and platform-api.md. A `CHANGELOG.md` "Known limitations" line still says "one superstep" (IN-13). |
| WR-2 (replay scoped to the thread) | Partly resolved. The foreign-run skip works (records are stamped with the run id by `TraceDispatcher`). The "stale drained Halted record" skip covers only a run row of `Completed`/`Failed` (WR-5). The seq-collision root cause is still open and is the sharper defect (WR-6). |
| WR-3 (memo set before the write) | Resolved. The tri-state `NoticeClaim` is correct, and the memo is only written for `Won` and `AlreadyRecorded`. The check-then-insert is no longer atomic, but the store's `ON CONFLICT` dedup keeps a concurrent child guard from double-delivering. Two small residuals: IN-14. |
| WR-4 (failed settlement blinds the guard) | Resolved. `settle_boundary` returns `SettleHealth`, the engine calls `note_unsettled_spend` before the next boundary, and the guard fails closed only when a ceiling applies and shows headroom. One log-line defect (IN-10) and documentation drift (IN-11). |

No new Critical issue was found. Two Warnings remain, both about the replay path that Phase 42 makes
first-class by documenting resume-by-fork. IN-1 to IN-9 are carried (IN-7 is downgraded to "not
actionable"; IN-4 and IN-9 are widened). Numbering continues from the previous report, so new items start
at WR-5 and IN-10.

## Warnings

### WR-5: The WR-2 stale-record skip only covers a `Completed`/`Failed` row, so a drained-then-requeued run that ends `Halted` or `Cancelled` still has its replay cut at the stale record

**File:** `src/application/services/run/events.rs:756-760` (the new `Skip` branch in `replay_terminal_override`), `733-739` (its doc comment), test `src/application/services/run/stream_tests.rs` (`replay_skips_a_drained_halted_record_when_the_run_later_completed`)
**Issue:** The persisting sink does not filter the drain's reasonless `RunFinished { Halted }` (only the bus
sink does, `events.rs` `RunEventBusSink`). A run that drains on shutdown, is requeued, and then ends
`Halted` (spend halt) or `Cancelled` has a stale reasonless `Halted` record at seq k in `run_traces`.
Replay of that run now takes the `run.status.is_terminal()` path, and the new skip fires only for
`Completed | Failed`. For a row of `Halted` or `Cancelled` the stale record is emitted as the terminal
`done` (with the row's status and reason grafted on) and `state.finished = true` ends the stream at seq k.
Every record the second dispatch wrote after k is never replayed, so a subscriber sees a spend halt or
cancel arrive before the run's last supersteps. A reasonless `Halted` record cannot be told apart from a
genuine end by its contents alone (a `Token` halt with a persisted cancel flag is a genuine reasonless
`Halted` record with a `Cancelled` row), so the row status is the wrong discriminator.
The new test also uses monotonic seq numbers (1, 2, 3, 4), a layout production cannot produce: every
dispatch builds a fresh `TraceDispatcher` whose seq restarts at 1 (see WR-6), so the test does not model
the real second-dispatch rows.
**Fix:** Decide "end of run" by whether a later record of this run exists, not by row status. In
`replay_stream`, when a `RunFinished` is popped and the row is terminal, treat it as the end only if
`state.pending` holds no later record stamped with this run id and the next `port.read(.., after_seq, ..)`
page has none either (peek the page and push it onto `pending`); otherwise skip it. Keep the existing
`AwaitingInput` and not-terminal rules. Add a test with the row `Halted` (and one with `Cancelled`): a
stale reasonless `Halted` record followed by further same-run records and a final `Halted` record carrying
a reason, asserting the replay reaches the final record.

### WR-6: A second run on a thread (the documented resume-by-fork path) silently loses every trace record whose `seq` the first run used

**File:** `src/application/services/run/worker.rs:1247` and `1549` (a fresh `TraceDispatcher` per dispatch), `crates/paladin-battalion/src/engine/hooks.rs:190-215` (`TraceDispatcher::with_capacity`, seq starts at 1), `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql` and the postgres twin (`PRIMARY KEY (thread_id, seq)`, `ON CONFLICT DO NOTHING` in `run_trace/{sqlite,postgres,in_memory}.rs`)
**Issue:** Carried from the previous WR-2 and left open by the fix pass (recorded under "Known limitations" in
`CHANGELOG.md`). `POST /threads/{id}/fork` starts a new run on the same thread, and a requeued drained run
re-dispatches the same run. Both build a dispatcher whose seq restarts at 1, and `run_traces` is keyed
`(thread_id, seq)`. The second run's records at seq 1..N (N being the first run's last seq) are dropped as
conflicts. The persisted trace of the fork is therefore missing its `RunStarted`, its early supersteps and,
when it is not longer than the original, its own `RunFinished` (the replay then falls back to the
row-synthesized terminal event, which is why the symptom is easy to miss). The replay filter added by the
WR-2 fix hides the foreign rows but cannot restore the dropped ones. This is observability data loss on the
exact path the phase documents as the way to resume a halted run, and every consumer that reads
`run_traces` by thread (replay, any trace export) sees the gap.
**Fix:** Pick one of the two options the fix report proposed, both need a design decision on the public
surface. (a) Add `TraceDispatcher::with_seq_origin(u64)` and seed it in `worker.rs` from the thread's
current maximum seq (needs a `max_seq(&ThreadId)` method on `RunTracePort`, defaulted for existing
implementors). (b) Key `run_traces` on `(thread_id, run_id, seq)` with migration 015 for sqlite and
postgres (append-only: do not edit 006), and change the replay read to filter by run id in SQL. Add a
contract test that appends two runs' records with overlapping seq to one thread and reads both back. If
neither is taken in this phase, record it in `WINDOWS.md` as an accepted exception and state the
consequence in `platform-api.md`'s streaming section ("the replay of a forked run may omit its early
events"), so the limitation is visible where the fork recipe is documented.

## Info

### IN-1: A spend halt ignores a persisted cancel flag, so a cancel that the debounced probe missed is recorded `halted`

**File:** `src/application/services/run/worker.rs:520-535` (the `HaltCause::Spend` arm of `map_outcome`), test `map_outcome_spend_halt_ignores_a_cancel_flag` (`worker.rs:2344`)
**Issue:** Still holds on HEAD. At the engine, cancel wins over spend. At the worker the Spend arm decides
from the cause alone and ignores `cancel_requested`, which `run_once` has already read
(`worker.rs:1384`). The cross-instance probe is debounced, so a cancel written by another instance can miss
the boundary where the guard halts, and the caller who asked to cancel then sees `halted` with an
allowance reason. ADR-0057 pins this deliberately, so this is a product-contract question.
**Fix:** Either keep it and state it in the `POST .../cancel` docs ("a run that halts on spend in the same
window may report `halted`"), or return the `Cancelled` transition (no `halt_reason`) from the Spend arm
when `cancel_requested` is true and update the test. Not actionable without a product decision; the doc
sentence is the safe default.

### IN-2: Agent-kind spend halts claim no operator `allowance_halted` notice

**File:** `src/application/services/run/worker.rs:1832-1862` (`persist_agent_halt`), `1793-1830` (`derive_dispatch_budget`); `.planning/phases/42-mid-run-halt-sse-terminal-status/42-CONTEXT.md` D-18
**Issue:** Still holds. Only `TreasurerSpendGuard::claim_spend_halt` calls `claim_halt_notice` /
`notify_operator`. The dispatch-time halt and the derived-budget `AllowanceHalted` halt of an agent-kind run
record the run `Halted` and send the caller's webhook but never claim a halt notice. The docs scope the
notice to "a run halted at a superstep boundary", so docs and code agree but D-18's wording ("a spend halt
notifies the operator once per window") does not.
**Fix:** Either call `claim_halt_notice` / `notify_operator` from `persist_agent_halt` for an
`AllowanceExhausted` reason (the store dedups), or reword D-18 and `platform-api.md` to say the notice is
for engine-path halts only. Prefer the rewording unless operators are expected to be told about agent-kind
halts.

### IN-3: The truncation notice calls an allowance halt "the final answer"

**File:** `src/application/services/paladin/middleware/limits.rs:62-63` (`TOKEN_BUDGET_NOTICE`), appended at `196`, asserted at `562`, `718`, `815`
**Issue:** Still holds. The same constant is appended when the stop reason is `AllowanceHalted`: "Token
budget reached — the response above is this run's final answer." For a halted run the output is partial and
the run is a resume point, so the text misleads readers of the output.
**Fix:** Add `ALLOWANCE_HALT_NOTICE` ("\n\n[budget] Allowance reached — this response is partial; the run
was halted.") and push it when `allowance_figures` is `Some`; keep `TOKEN_BUDGET_NOTICE` for the operator
budget. Update the three assertions.

### IN-4: `herald_line` labels a window refusal that has no window as a lifetime cap (two sites)

**File:** `crates/paladin-core/src/platform/container/allowance.rs:294` (`HaltReason::herald_line`) and `505` (`AllowanceWarning::herald_line`)
**Issue:** Widened from the previous report: both renderers use a `_ => "lifetime cap"` wildcard. The
`(AllowanceLimitKind::Window, None)` case (a malformed or deserialised refusal or warning) prints "lifetime
cap", which states the wrong fact. `AllowanceLimitKind` is not matched exhaustively either, so a future
kind would also read as a lifetime cap.
**Fix:** In both places match `(AllowanceLimitKind::Window, Some(end)) => "window resets ..."`,
`(AllowanceLimitKind::Lifetime, _) => "lifetime cap"` and
`(AllowanceLimitKind::Window, None) => "window"`, with a final arm only if the enum is `#[non_exhaustive]`.

### IN-5: A derived-budget halt persists `balance == ceiling` as if it were a measured balance

**File:** `src/application/services/treasurer/derive.rs:179-185` (the `Some(max_tokens)` arm), consumers `worker.rs` (`persist_agent_halt`), `events.rs`, `agent_controller.rs`
**Issue:** Still holds (ADR A5). The conservative figure is written to `runs.halt_reason`, the webhook and
the SSE `done` tagged `allowance_exhausted`, so a client may be told the allowance is exhausted when a
resubmission would be admitted with a smaller remainder.
**Fix:** Document in `platform-api.md` that `balance` on an agent-kind halt is an upper bound, not a
reading. Persisting a real balance would need a second ledger read at halt time and is not worth it in this
phase.

### IN-6: One run row with an unreadable `halt_reason` fails `GET /runs`, `get` and the SSE degraded poller

**File:** `crates/paladin-storage/src/run/sqlite.rs:319-327`, `crates/paladin-storage/src/run/postgres.rs:301-310`
**Issue:** Still holds. A `halt_reason` that does not deserialize into `HaltReason` (a future variant read by
an older binary; `HaltReason` is `#[non_exhaustive]` with no fallback) becomes
`RunRepositoryError::Serialization` and takes down the whole page. It is consistent with how `output` and
`schema_version` decode, so it is a robustness gap rather than a regression.
**Fix:** Decode `halt_reason` tolerantly: on a deserialisation error log the run id (never the payload) at
`error` and read `None`. Keep the strict error on the write side and update the pinning test.

### IN-7: Migration 014 drops the arbiter index that older binaries' `ON CONFLICT` list targets (not actionable)

**File:** `crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql:24`, `crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql:35`
**Issue:** Downgraded on re-evaluation. The rollout caveat the previous report asked for already exists in
`MIGRATION.md` (the 014 entry: "a replica still running a pre-014 binary ... migrate and upgrade the
replicas together so no warn notice is lost"). The remaining suggestion, to add the note to the migration
header, must NOT be done: sqlx records a checksum per applied migration, so editing a comment in
`014_*.sql` after it has been applied anywhere breaks startup for those databases (and the Postgres test
`include_str!`s the file).
**Fix:** None. Do not edit the migration files.

### IN-8: A halted or cancelled run can briefly expose its outcome while still `Running`

**File:** `src/application/services/run/worker.rs:1404-1420`
**Issue:** Still holds. The G14 reorder writes `record_outcome` before `update_status`. If `update_status`
fails, the message is redelivered, but until the next attempt rewrites the outcome a `Running` row reports
`halt_reason` and `final_waypoint_id`. Narrow and self-healing.
**Fix:** Add one comment beside the reorder stating the window and why it is accepted. No code change.

### IN-9: `run_agent` has grown and now repeats the `RunFinished` literal five times; two functions carry `too_many_arguments` allows

**File:** `src/application/services/run/worker.rs:1512-1790` (`run_agent`; `RunFinished` literals at `1584`, `1598`, `1677`, `1692`, `1748`), `src/application/services/paladin/paladin_execution_service.rs:1331` and `1530`
**Issue:** Updated count (five, was four). `run_agent` is roughly 280 lines and builds a near-identical
`TraceEvent::RunFinished { status, total_supersteps: 0, usage, cost, duration_ms, halt_reason, trace_dropped_total: 0 }`
in each branch, which makes the "exactly one terminal event per run" invariant harder to audit.
`execute_bounded` and `execute_internal` suppress `clippy::too_many_arguments` instead of bundling
per-call state. Separately, `paladin-eval`'s `run_status_matches` (`assertion.rs:420-431`) has no
`Cancelled` arm and `RunStatusValue` has no `Cancelled`; the eval CHANGELOG records this as deferred.
**Fix:** Add `emit_agent_run_finished(&dispatcher, status, halt_reason, duration_ms)` and use it at all five
sites; bundle the per-call arguments of the two execution-service functions into a context struct. The eval
gap needs no action beyond tracking.

### IN-10: The WR-4 fail-closed log line contains a run of literal spaces and bypasses `fail_closed_message`

**File:** `src/application/services/treasurer/guard.rs:270-274`
**Issue:** New in the WR-4 fix. The message literal was meant to be a `\`-continued string, but the line
breaks were flattened into the string, so the logged text is
`"... tenant={}<30 spaces>error=a superstep's spend could not be written ... so<29 spaces>this run's balance can no longer be trusted"`.
This is the one `error` line an operator sees when a run is halted for a lost charge (the `ledger_unavailable`
halt sends no operator notice), and it is not greppable as written. It also bypasses
`fail_closed_message` (`guard.rs:304-324`), the helper that exists so every fail-closed line shares one
shape and has a test asserting it names no key value (T-42-18). The new line has no `scope=` field and no
such test.
A related cosmetic residual of CR-1 sits in `superstep.rs:1645`, `3526`: see IN-12.
**Fix:** Build the line through the helper:
```rust
log::error!(
    "{}",
    fail_closed_message(
        &self.run_id,
        self.treasurer
            .policy
            .ceilings_for(&self.subject)
            .iter()
            .map(|c| c.scope_kind.as_str()),
        &self.subject.tenant_id,
        &"a superstep's spend could not be written to the ledger, so this run's balance can no longer be trusted",
    )
);
```
and extend the existing "no key value in the message" test to cover it.

### IN-11: WR-4 widened the meaning of `ledger_unavailable` but the type docs and platform docs still say "could not be read"

**File:** `crates/paladin-core/src/platform/container/allowance.rs:220-222` (`HaltReason::LedgerUnavailable` doc), `docs/src/api-reference/platform-api.md:155`, `180`, `519`, `767`, `crates/paladin-ports/src/output/spend_guard.rs` (`HaltReason::LedgerUnavailable` mentions in `SpendDecision`)
**Issue:** After 44209d8e a run also halts with `{"reason":"ledger_unavailable"}` when one of its own superstep
charges could not be written (a ledger write error or a currency mismatch), even though every balance read
succeeded. The variant doc says "a ceiling could not be evaluated (a failed balance or store-clock read)", and
platform-api.md describes the halt as "an unreadable ledger" and says the operator gets no notice because
"nothing was measured". A client that sees this reason after a lost write is told the wrong cause.
**Fix:** Reword the variant doc and the four platform-api.md passages to "the ledger could not be read, or a
superstep's spend could not be written to it" and note that the run's own balance can no longer be trusted.
Do not add a new variant (the wire object is frozen at `{"reason":"ledger_unavailable"}`).

### IN-12: A nested child's spend or cancel halt is recorded as `Skipped { reason: "shutdown" }`

**File:** `crates/paladin-battalion/src/engine/superstep.rs:1645-1647` (`node_outcome_kind`), `3524-3528` (the `ChildHalted` bookkeeping record)
**Issue:** New with the CR-1 fix. `ChildHalted` reuses `Interrupted`'s `NodeOutcomeKind::Skipped { reason:
"shutdown" }`, so a Battalion node whose child halted on a spend ceiling shows `shutdown` in the persisted
Waypoint `completed` records and in the `NodeFinished` trace, which contradicts the run's own `halt_reason`.
Functionally harmless: resume keys off the vanguard, not the reason string.
**Fix:** Use a distinct reason for the child case (`"child_halted"`) at the two sites, and pin it in the CR-1
tests. Check the trace and eval snapshots for any assertion on the string before changing it; if one exists
treat this as not actionable.

### IN-13: WR-1 is documented but not enforced, and one "Known limitations" line still says "one superstep"

**File:** `CHANGELOG.md:596-600` (the "Phase 42 allowance halts are bounded" entry), `.planning/` `WINDOWS.md` row 65
**Issue:** The fix report states WR-1 was resolved documentation-only, which is accepted. Two leftovers: the
Known-limitations paragraph still says "overshoot is at most one superstep's spend per run" (the other four
places were updated to "one top-level superstep, including any nested Battalion run"), and `WINDOWS.md`
row 65's description was not amended.
**Fix:** Change the CHANGELOG sentence to the new wording. For `WINDOWS.md` (machine-managed), a human
should amend row 65 through the `gsd-tools` window flow. Enforcement itself (a derived child settlement key
or an `in_flight` argument on `SpendGuard::check`) is a design change and is not actionable in this phase.

### IN-14: WR-3 retry has no ceiling on repeated store failures, and one doc comment line is 130 columns

**File:** `src/application/services/treasurer/guard.rs:112-117` (the `claimed` field doc, line 115 is 130 columns), `150-205` (`claim_mid_run_warnings`), `src/application/services/treasurer/mod.rs` (`claim_record` error arm)
**Issue:** Two small residuals of the WR-3 fix. (1) With the memo no longer set before the write, a notice
store that stays down is written to and logged at `error` once per ceiling crossing per superstep boundary
for the rest of the run, with no backoff or cap. The write is bounded by the superstep count, so this is log
noise and extra store load, not a correctness bug. (2) The fix appended text to an existing rustdoc line
and left it 130 columns wide, against the 100-column convention.
**Fix:** Rewrap the doc comment at 100 columns. Optionally memoise a claim after three consecutive `Failed`
answers for the same key (count in the memo map) so a dead store stops being retried. The halt-notice
residual the fix report mentions (a halt claim lost to a store error is not retried because the run ends at
once) is inherent and not actionable.

---

_Reviewed: 2026-10-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
