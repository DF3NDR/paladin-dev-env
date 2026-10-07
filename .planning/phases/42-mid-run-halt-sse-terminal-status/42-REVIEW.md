---
phase: 42-mid-run-halt-sse-terminal-status
reviewed: 2026-10-07T12:00:00Z
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
  critical: 1
  warning: 4
  info: 9
  total: 14
status: issues_found
---

# Phase 42: Code Review Report

**Reviewed:** 2026-10-07
**Depth:** standard
**Files Reviewed:** 93
**Status:** issues_found

## Summary

Reviewed the Phase 42 diff (`9b7ee07..HEAD`) against ADR-0057 and the phase CONTEXT. Production paths
were read at diff level with surrounding context: the engine boundary check and `HaltCause`
plumbing, `settlement.rs`, the worker (`map_outcome`, `run_once`, `run_agent`,
`derive_dispatch_budget`, `persist_agent_halt`), `Treasurer::evaluate` / `derive` / `guard`,
`PerRunCancelProbe`, `RunEventBusSink` and the three SSE paths (live, degraded, replay), the three run
adapters, the notice adapters and migrations 013/014, the webhook payload and delivery signing branch,
the agent controller, the shared-service `TokenBudget` wiring, and `run_api_wiring`. Tests,
CHANGELOGs, docs and the OpenAPI golden were skimmed for contract drift and test reliability, not
line-audited. No code was executed: the repository-wide build is large and the findings below were
established by reading the code paths end to end, and each states the exact lines to confirm.

The newly written code is generally careful. Checked and found sound: the integer-only ceiling
comparison and `i128` token derivation, the strict `>` cutoff, the single `wire_json` builder shared by
every surface, the `$N` placeholder arithmetic in both `INSERT_RUN*` statements, the 014 arbiter column
lists, the `Local`-then-`shutdown` read order in `PerRunCancelProbe` (a cancelled child implies a
cancelled parent, so the order is race-safe), the operator-notice signing branch accepting
`allowance_halted`, and the absence of key values in any new log line, error body or payload. There are
no `unwrap`/`expect`/`panic!` calls in new non-test code.

One correctness defect falls in the nested-Battalion path that Phase 42 newly routes spend halts
through (CR-1). Four warnings follow, mostly about a guard or replay claim being weaker than the ADR
states.

## Critical Issues

### CR-1: A nested `Battalion` child that halts is recorded as a completed node, so the parent can finish `Completed` or resume past the child's unfinished work

**File:** `crates/paladin-battalion/src/engine/superstep.rs:1519-1531` (child `Halted` arm), `3995-4075` (completion), `2234-2290` (the only place the guard halts the parent)
**Issue:** G11 gives a nested `NodeSpec::Battalion` child run the same per-run guard, so a child can now
halt on spend at its own first boundary (for example because other runs spent the allowance after the
parent's last boundary). The child then returns `RunOutcome::Halted`, and the parent's Battalion arm
maps it to `(None, TokenUsage::default(), None, Ok(StateDelta::new().into()))`, which is a successful
node with an empty delta. The parent only learns about the halt at its NEXT top-of-loop boundary, via
the sticky guard. Two consequences follow from the loop structure:

1. **Terminal Battalion node.** If the Battalion node has no successor, the superstep's
   `next_vanguard` is empty and `run_with_namespace` takes the `next_vanguard.is_empty()` branch at
   `superstep.rs:4045-4075`: it persists a `Completed` Waypoint and returns `RunOutcome::Completed`
   before any further top-of-loop check runs. The run is recorded `completed`, the SSE `done` says
   `completed`, the webhook fires `completed`, no `halt_reason` is set, no resume point exists, and the
   child's outputs are silently missing from the parent's Battlefield. A sub-workflow as the last step
   of a graph is a common shape. The pre-existing test
   `cancellation_is_observed_at_the_child_superstep_boundary` documents this hazard in its comment
   ("the Battalion node has a static successor so the parent's run does NOT short-circuit through
   'vanguard empty -> Completed'"), and the new
   `child_battalion_halt_on_spend_halts_the_parent` copies the workaround (an `after` node) instead of
   testing the unsafe shape.
2. **Non-terminal Battalion node.** The parent's Halted Waypoint is built from the next frontier, so it
   lists `after` as the vanguard and records `sub` as a completed node with an empty delta. The fork
   recipe in `platform-api.md` ("Resuming a halted run") restarts from that Waypoint and never
   re-dispatches `sub`. The "resume-mid-child" logic at `superstep.rs:1299-1345` only helps when `sub`
   is in the resumed vanguard, so the child's own Halted Waypoint is orphaned and its remaining
   supersteps never run. The resumed run then proceeds with the child's mapped outputs absent.

For a token cancel this was a latent flaw (D-21). Phase 42 makes it reachable by a spend halt, which is
a normal operating condition rather than an operator action, and the user sees a successful run.
**Fix:** Do not report a halted child as a successful node. In the `Ok(RunOutcome::Halted { cause, .. })`
arm return a distinct node result (for example `NodeFailure::ChildHalted(cause)`) and handle it where
`aborted_node_ids` is handled at `superstep.rs:3830`: persist a `Halted` Waypoint whose vanguard
re-lists the Battalion node alongside `next_vanguard`, so the resume re-enters the child through the
existing resume-mid-child path, and return `RunOutcome::Halted { cause }` with the CHILD's cause so the
spend reason survives. Add tests for both shapes, with the Battalion node terminal and with a
successor, asserting the parent's outcome is `Halted`, its Halted Waypoint vanguard contains the
Battalion node, and a resume re-enters the child exactly once.

## Warnings

### WR-1: A nested child's boundary guard is blind to its own run's spend, so "at most one superstep" holds only for top-level supersteps

**File:** `crates/paladin-battalion/src/engine/settlement.rs:160-200` (`SpendHook::child`, `settle_boundary`), `crates/paladin-battalion/src/engine/superstep.rs:2252-2262` (guard call), `.planning/decisions/0057-mid-run-halt-contract.md:72-75`
**Issue:** ADR-0057 says that on the engine path a run "can spend at most one superstep beyond a
ceiling". A child hook has `ledger: None`, so its `settle_boundary` returns immediately, and the child's
spend folds into the parent's accumulator, which is written only at the parent's own boundary. While a
child graph runs its N inner supersteps, each inner boundary calls `guard.check`, but the ledger
contains none of this run's in-flight spend. The inner checks can only detect spend by other runs. A
parent superstep that hosts a long child graph can therefore overspend by that entire graph, and the
inner checks give a false impression of coverage. The bound is correct only if "superstep" means a
top-level superstep.
**Fix:** At minimum, state the bound precisely in the ADR, `platform-api.md` and the `SpendGuard`
rustdoc ("one top-level superstep, including any nested Battalion run it contains") and add a
`WINDOWS.md` row. To enforce it, let the child boundary settle its own charge under a derived key such
as `(child_run_id, child_superstep, attempt)`, or pass the unsettled accumulator total to the guard
(`check(&self, thread, in_flight: Option<&Cost>)`) and compare `balance + in_flight >= ceiling`.

### WR-2: Replay is scoped to the thread, not the run, so a prior run's `RunFinished` can end a later run's replay

**File:** `src/application/services/run/events.rs:734-786` (`replay_terminal_override`), `813-840` (`replay_stream` sets `finished` on the first `Emit`), `1038-1046` (`stream` reads by `run.thread_id`), `src/application/services/run/worker.rs:1247` and `1549` (a fresh `TraceDispatcher` per dispatch), `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql` (`PRIMARY KEY (thread_id, seq)`)
**Issue:** `RunEventStreamService::stream` replays `port.read(&run.thread_id, 0, ..)`: every record on the
thread, from every run, with no `run_id` filter. Phase 42's documented recovery path is a NEW run on the
SAME thread (`POST /threads/{id}/fork` from the Halted Waypoint). Two problems follow.

- The new G10 rule turns the first replayed `RunFinished` it meets into the terminal event whenever the
  run row being replayed is terminal. For the second run, the first `RunFinished` on the thread is the
  halted FIRST run's. The override rewrites its `status` and `halt_reason` from the second run's row and
  ends the stream (`state.finished = true`). The subscriber gets a `done` carrying the first run's
  `usage`/`trace_seq` and none of the second run's events. The same truncation happens for a single run
  that was drained and requeued: the persisting sink does not filter the drain's reasonless
  `RunFinished { Halted }` (only the bus sink does, `events.rs:300-320`), so once the row is terminal
  that stale record ends the replay early.
- Each dispatch builds a fresh dispatcher whose `seq` starts at 0, and `run_traces` is keyed
  `(thread_id, seq)` with `ON CONFLICT DO NOTHING`. A second run on a thread therefore collides with
  the first run's rows for every `seq` the first run used, and those appends are silently dropped. The
  replay of the second run then shows the first run's records. This is inherited behaviour, but the new
  replay convergence rule (G10, "trust the row") now grafts the second run's status onto the first run's
  events instead of exposing the problem.

**Fix:** Filter replayed records by `record.run_id == Some(state.run_id)` before treating a
`RunFinished` as terminal, and skip records of other runs. Only treat a `RunFinished` as the end when no
later record of the same run exists (peek the next page), or skip a reasonless `Halted` record when the
row is `Completed` or `Failed`. Fix the seq origin separately, for example by seeding the dispatcher's
`seq` from the thread's current maximum (or by keying `run_traces` on `(thread_id, run_id, seq)`), and
add a replay test with two sequential runs on one thread.

### WR-3: The guard's in-run notice memo is set before the store write, so one transient store error permanently drops that run's mid-run warning

**File:** `src/application/services/treasurer/guard.rs:137-143` (`first_attempt`), `150-180` (`claim_mid_run_warnings`), `182-199` (`claim_spend_halt`); `src/application/services/treasurer/mod.rs` (`claim_record`)
**Issue:** `first_attempt(key)` inserts the claim key into the memo before `claim_notice` runs.
`claim_record` collapses `Err(store error)` into `None`, the same value as `AlreadyRecorded`. If the
first write fails transiently (pool timeout, brief outage), the memo already says "tried", so this run
never retries that ceiling and window. If no other run wins the claim, the operator is never warned. The
doc comment says the memo "only skips writes the store would answer `AlreadyRecorded` to", which is false
for the failure case. For the halt claim the run ends at once, so a lost halt notice is similarly
unrecoverable by that run.
**Fix:** Return a tri-state from `claim_record` (`Won`, `AlreadyRecorded`, `Failed`). Insert the key into
the memo only for `Won` and `AlreadyRecorded`, leaving it unmemoised on `Failed` so the next boundary
retries. Add a test with a notice store that fails once and then succeeds.

### WR-4: A failed or skipped settlement silently blinds the guard

**File:** `crates/paladin-battalion/src/engine/settlement.rs:195-250` (`settle_boundary`)
**Issue:** D-03 fails closed when the ledger cannot be READ. The guard is only as good as the WRITES that
feed `balance`, and `settle_boundary` logs and drops a ledger `Err` and a `CurrencyMismatch` charge
(the pre-Phase-42 "never halts a run" rule). Phase 42 makes the ledger authoritative for halting, so a
run whose settlements keep failing while balance reads still succeed (a write-path permission or
constraint problem, or a price table currency that differs from the policy's) is never halted and spends
unmetered with only `error` log lines as evidence. That is the opposite posture to D-03 for the same
principal.
**Fix:** Have `settle_boundary` return or record a health flag the engine passes into the next
`SpendGuard::check`. When a guard is attached and a superstep's charge could not be written, answer
`Halt(HaltReason::LedgerUnavailable)`. If that is out of scope, record the gap in `WINDOWS.md` as an
accepted exception.

## Info

### IN-1: A spend halt ignores a persisted cancel flag, so a cancel that the debounced probe missed is recorded `halted`

**File:** `src/application/services/run/worker.rs:510-535` (the `HaltCause::Spend` arm), test `map_outcome_spend_halt_ignores_a_cancel_flag` (`worker.rs:2382`)
**Issue:** At the engine, cancel wins over spend. At the worker the Spend arm decides from the cause alone
and ignores `cancel_requested`, which `run_once` has already read. The cross-instance probe is debounced
(`with_cancellation_probing`), so a cancel written by another instance can easily miss the boundary where
the guard halts. The caller who asked to cancel then sees `halted` with an allowance reason. ADR-0057
pins this deliberately (the test is listed there), so this is a product-contract question rather than a
bug.
**Fix:** Either keep it and say so in the `POST .../cancel` docs ("a run that halts on spend in the same
window may report `halted`"), or return the `Cancelled` transition (no `halt_reason`) when
`cancel_requested` is true.

### IN-2: Agent-kind spend halts claim no operator `allowance_halted` notice

**File:** `src/application/services/run/worker.rs:1832-1862` (`persist_agent_halt`), `1793-1830` (`derive_dispatch_budget`); `.planning/phases/42-mid-run-halt-sse-terminal-status/42-CONTEXT.md` D-18
**Issue:** D-18 says "a spend halt notifies the operator once per window". Only the engine guard
(`TreasurerSpendGuard::claim_spend_halt`) calls `claim_halt_notice` / `notify_operator`. The dispatch-time
halt and the derived-budget `AllowanceHalted` halt of an agent-kind run, which are the halts the
derived-budget design produces, record the run `Halted` and send the caller's webhook but never claim a
halt notice or log at `error`. The docs scope the notice to "a run halted at a superstep boundary", so
this is consistent with the documentation but not with D-18's wording.
**Fix:** Either call `claim_halt_notice` / `notify_operator` from `persist_agent_halt` for an
`AllowanceExhausted` reason (the store dedups), or reword D-18 and `platform-api.md` to say the notice is
for engine-path halts only.

### IN-3: The truncation notice calls an allowance halt "the final answer"

**File:** `src/application/services/paladin/middleware/limits.rs:62-63` (`TOKEN_BUDGET_NOTICE`), appended at `196`
**Issue:** The same constant is appended for `StopReason::AllowanceHalted`: "Token budget reached - the
response above is this run's final answer." For a halted run the output is partial and the run is a
resume point, so the text misleads readers of the output.
**Fix:** Append a distinct allowance variant (for example "Allowance reached - this response is partial;
the run was halted.") and keep the current text for the operator budget. Extend the two tests that assert
the exact string.

### IN-4: `HaltReason::herald_line` labels a `Window` refusal that has no window as a lifetime cap

**File:** `crates/paladin-core/src/platform/container/allowance.rs:294` (`_ => "lifetime cap".to_string()`)
**Issue:** The `(AllowanceLimitKind::Window, None)` case, reachable only through a malformed or
deserialised refusal, falls into the wildcard and claims "lifetime cap". It is harmless in practice but
states the wrong fact.
**Fix:** Match `(AllowanceLimitKind::Lifetime, _) => "lifetime cap"` and
`(AllowanceLimitKind::Window, None) => "window"` explicitly.

### IN-5: A derived-budget halt persists `balance == ceiling` as if it were a measured balance

**File:** `src/application/services/treasurer/derive.rs:176-185` (the `Some(max_tokens)` arm), consumers `worker.rs` (`persist_agent_halt`), `events.rs`, `agent_controller.rs`
**Issue:** Per ADR A5 the cutoff reports the ceiling as the balance because the loop cannot know the
post-spend balance. That conservative figure is written to the durable `runs.halt_reason`, the webhook
and the SSE `done`, tagged `allowance_exhausted`. A client reading `GET /runs/{id}` is told the allowance
is exhausted when a resubmission will in fact be admitted with a smaller remainder.
**Fix:** Document in `platform-api.md` that `balance` on an agent-kind halt is a bound, not a reading, or
persist the balance actually read at derivation time plus a `"bound": "conservative"` marker.

### IN-6: One run row with an unreadable `halt_reason` fails `GET /runs`, `get` and the SSE degraded poller

**File:** `crates/paladin-storage/src/run/sqlite.rs:319-327`, `crates/paladin-storage/src/run/postgres.rs:301-310`
**Issue:** Row decode maps any `halt_reason` that does not deserialize into `HaltReason` to
`RunRepositoryError::Serialization`. `HaltReason` is `#[non_exhaustive]` with no `#[serde(other)]`
fallback, so a future variant read by an older binary fails the whole page. The behaviour matches how the
same decode treats `output`, attribution and `schema_version`, and the test pins it, so this is
consistent rather than a regression; the cost is that an informational column can take a listing down.
**Fix:** Decode tolerantly: log the run id (never the payload) at `error` and read `halt_reason` as
`None`. Keep the strict error on the write side.

### IN-7: Migration 014 drops the arbiter index that older binaries' `ON CONFLICT` list targets

**File:** `crates/paladin-storage/migrations/postgres/014_add_treasury_notice_kind.sql:24`, `crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql:35`
**Issue:** The migration drops `idx_treasury_notices_once` and recreates it with `notice_kind` appended.
A pre-014 binary still running during a rolling deploy issues `ON CONFLICT (scope_kind, ..., ceiling_nanos)`,
which no longer matches any unique index, so every notice claim from those instances fails. `claim_record`
logs and swallows the error, so warnings are lost silently for the rollout window. The header comment
calls the change one-way but does not mention this window.
**Fix:** Note the rollout constraint in the migration header and `MIGRATION.md` ("upgrade every replica
before relying on warnings"), or keep the old index until a later migration drops it.

### IN-8: A halted or cancelled run can briefly expose its outcome while still `Running`

**File:** `src/application/services/run/worker.rs:1404-1420`
**Issue:** The G14 reorder writes `record_outcome` before `update_status` for `Halted` and `Cancelled`.
If `update_status` then fails, the worker returns the error and the message is redelivered, but until the
next attempt rewrites the outcome a `Running` row reports `halt_reason` and `final_waypoint_id`. The
window is narrow and self-healing.
**Fix:** Document it beside the reorder, or clear the outcome fields when a retry re-dispatches a
`Running` run that already carries a `halt_reason`.

### IN-9: `run_agent` has grown and now repeats the `RunFinished` literal four times; two functions gained `too_many_arguments` allows

**File:** `src/application/services/run/worker.rs:1512-1790` (`run_agent`), `src/application/services/paladin/paladin_execution_service.rs:1331` and `1530`
**Issue:** `run_agent` is roughly 280 lines and builds a near-identical
`TraceEvent::RunFinished { status, total_supersteps: 0, usage, cost, duration_ms, halt_reason, trace_dropped_total: 0 }`
in four branches (dispatch halt, unpriced, derived halt, completed/failed), which makes the "exactly one
terminal event per run" invariant harder to audit. `execute_bounded` and `execute_internal` suppress
`clippy::too_many_arguments` rather than bundling the per-call state.
**Fix:** Add a small `emit_agent_run_finished(dispatcher, status, halt_reason, duration_ms)` helper and
move the per-call arguments of the two execution-service functions into a context struct instead of
suppressing the lint. Separately, `paladin-eval` has no `Cancelled` arm in `run_status_matches` and no
`RunStatusValue::Cancelled`; the eval CHANGELOG records this as deferred, so no action is needed beyond
tracking it.

---

_Reviewed: 2026-10-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
