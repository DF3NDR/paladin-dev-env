# ADR-0057: Mid-run halt contract: check-only boundary, typed halt cause, fork-as-resume, derived agent budget

## Status

Accepted

**Date:** 2026-10-06

**Phase:** 42 (Mid-Run Halt & SSE Terminal Status), plan 42-01. The design was confirmed at the plan 42-01
Task 1 design gate on 2026-10-06 with the operator's selection `option-b`: every one of the sixteen proposed
items stands as written, and item 12 is extended so a true streamed `execute/stream` call's terminal `done`
also carries an informational `halt_reason` when its terminal usage crossed the derived figure (see Decision,
"Checkpoint outcome" and group (e)). The selection is recorded verbatim in `42-01-SUMMARY.md` under
"Checkpoint decision".

## Context

Phase 42 delivers ALLOW-03 (a run already admitted keeps drawing from the ledger and stops cleanly, with its
last checkpoint kept, when the next draw would overspend; a halted run resumes once the allowance is replenished
or the window resets), ALLOW-05 (the Treasurer derives a per-run `TokenBudget` from the remaining allowance and
it composes with `TokenBudget`, `ModelCallLimit`, `ToolCallLimit` and the Commissary, replacing none of them,
with `Treasurer` kept a framework-only word) and PLAT-09 (the SSE `done` event matches the persisted status:
`cancelled` for a caller cancel, `halted` with the reason for a spend halt, on the live, degraded and replay
paths). The decisions below are the Phase 42 planning decisions D-01..D-21 (42-CONTEXT.md), with the alternatives
rejected in 42-DISCUSSION-LOG.md and the resolutions of the seventeen gaps G1..G17 that 42-RESEARCH.md found
between CONTEXT's wording and the code as it stands.

Three earlier ADRs are binding and none is re-opened (D-00a, D-00b, D-00c):

- **ADR-0052** fixes where a run is halted mid-flight: the `WarEngine` superstep boundary, where the existing
  cancellation path already writes a `WaypointStatus::Halted` Waypoint and returns `RunOutcome::Halted`, and the
  agent-loop `TokenBudget` `after_model` cutoff. Its rejected alternatives (wiring `build_chain` into the engine
  path; refusing inside the pricing decorator) stay rejected. This ADR builds those two attachment points; it does
  not move them.
- **ADR-0053** fixes the ledger as append-only and derive-on-read with the `007` schema, the `reserve`, `settle`
  and `release` row kinds and the settlement key `(run_id, superstep, attempt)`. Nothing here writes a ledger row
  of any kind or changes that schema (D-00b).
- **ADR-0056** fixes admission: tumbling UTC windows from `store_now()`, every-limit composition in the order
  key-window, key-lifetime, tenant-window, tenant-lifetime, check-only admission, no role bypass, fail closed when
  an allowance applies, store-deduped once-per-window notices and the `429 allowance_exhausted` contract. One line
  of its Downstream Consumers (Phase 42 closing the over-admission race by reserving at the superstep boundary) is
  superseded by decision group (a) below; ADR-0056 carries a dated note saying so and no other line of it changes.

ADR-0050 reserved `Treasurer` as the single officer word for cross-run spend governance, and Phase 27's run state
machine stays as written: `Halted` and `Cancelled` are both absorbing terminal statuses, no edge leaves `Halted`
(D-00d).

The seventeen RESEARCH collisions, in one paragraph: a same-instance caller cancel fires only the in-process child
token, which the engine cannot tell apart from a shutdown drain (G1); `execute/stream` is one provider call that
never invokes `after_model`, so a derived budget cannot halt it (G2); `build_run_api` spawns the worker pool before
the Treasurer exists (G3); `RunFinishStatus` is a `Copy` unit enum persisted in `run_traces`, so it cannot take a
payload variant without rewriting stored rows (G4); `treasury_notices` has a `CHECK` and a unique index that a halt
rung would collide with, and the operator delivery branch is keyed on one event (G5); the discriminator CONTEXT
names `kind` already means the limit kind in the `429` details (G6); callers cannot find the fork point from
`GET /runs/{id}` (G7); admission does not know the model, and a figure derived at `POST /runs` time is stale by
dispatch (G8); the frozen v0.9 OpenAPI golden gate covers `ExecuteResponse` (G9); `replay_stream` is a third `done`
path (G10); a spend halt is not sticky the way a cancel is (G11); an operator `TokenBudget` on the shared boot-time
service would re-create the hazard ADR-0052 rejected (G12); the halted Waypoint's superstep number is skipped on
resume (G13); the engine's `done` can precede the row's status write (G14); unpriced engine nodes are unmetered
(G15); the D-09 arithmetic has a fifth price axis, a zero-price case and a strict-greater comparison (G16); and
`StopReason` derives `Eq` while `AllowanceRefusal` does not (G17).

## Decision

**Checkpoint outcome (42-01 Task 1, resolved option-b).** The consolidated design was approved as proposed, with
no item redirected, and item 12 was amended: a true streamed `execute/stream` call's terminal `done` carries an
informational `halt_reason` object when the terminal chunk's usage crossed the derived figure. The amendment is
informational only: the call has already finished, so there is no behavioural halt, and a stream whose terminal
usage did not cross the figure stays byte-identical to today's `{ "done": true, "usage" }`. Consequently D-12's
stream clause holds for BOTH the buffered fallback and the true stream (group (e)).

**Overshoot, stated exactly (RESEARCH Pitfall 14, G16).** Every guarantee in this ADR is bounded; none is absolute.
The boundary check runs after a superstep, so on the engine path a run can spend at most one TOP-LEVEL superstep
beyond a ceiling. A nested `NodeSpec::Battalion` child run is part of the top-level superstep that hosts it: its
spend folds into the parent's accumulator, which is settled only at the parent's own boundary (CF-FR-16), so the
child's inner boundary checks see other runs' spend but none of this run's in-flight spend, and a parent superstep
that hosts a long child graph can overspend by that whole graph (Phase 42 review WR-1). The derived budget is compared after a response, so on the agent loop a run can spend at most one model
response beyond the figure. A true streamed call is one provider call, so its overshoot is that one whole call.
Two runs admitted in the same instant can both start, and each is halted at its own first boundary after
exhaustion. Neither this ADR nor the docs and rustdoc it governs may promise that an allowance can never be
overspent.

### (a) A check-only boundary at every boundary, failing closed, with policy behind a port (D-01, D-02, D-03, D-04; G3, G11; RESEARCH Pitfall 9)

**D-01. The boundary check reads a balance and writes nothing.** At every superstep boundary, for each applicable
ceiling in the ADR-0056 order, the Treasurer reads `balance` and halts when `balance >= ceiling` before the next
superstep starts, the same evaluation admission performs. No `reserve` row is written and the settlement the engine
already performs per superstep is untouched (D-00b). The over-admission race ADR-0056 D-05 accepted is therefore
**accepted here as well**, bounded to one superstep's spend per run, and ADR-0056's Downstream Consumers line about
reserving at the boundary is superseded. A per-superstep hold can be added beside this check later without
changing it (reversible; see Considered Options).

**D-02. Every boundary, no caching of `Continue`.** One `balance` read per applicable ceiling (up to four) at every
boundary, with no `min_interval` debounce and no local short-circuit: other runs sharing the scope are exactly the
case that matters.

**G11. A spend halt is made sticky; a `Continue` is not cached.** A child battalion run inherits the guard the way
it inherits the cancellation probe, and a parent that re-reads after a window roll could pass what the child
halted on. The per-run guard instance therefore memoises its FIRST `Halt` in an `Arc<OnceLock<HaltReason>>`
shared with the child-run clones and returns it on every later call. This does not conflict with D-02, which
forbids caching `Continue` answers only.

**D-03. Fail closed.** A failed read (`Backend`, `CurrencyMismatch`, a store-clock failure) for a principal with at
least one configured ceiling halts the run with the distinct reason `ledger_unavailable`, keeps its checkpoint, and
is logged at `error` with the scope kind, tenant id and backend error text (never a key value). A principal with no
configured ceiling never reads the ledger and is unaffected, and `requested_by: None` gets no guard at all, so a
deployment without allowances keeps running through a ledger outage. The halted run is resumable (D-07) once the
ledger is back.

**D-04. Policy stays in the Treasurer; the engine consults a port.** `paladin-battalion` learns no allowance. A new
output port `SpendGuard { async fn check(&self, thread: &ThreadId) -> SpendDecision }`, with `#[non_exhaustive]
SpendDecision { Continue, Halt(HaltReason) }`, sits beside `cancellation_probe.rs` and is consulted once per
boundary beside the probe. Unlike the probe it is fallible by design: a failed read is a halt decision, not a
swallowed `false`. The facade's `Treasurer` implements it from `AllowancePolicy::ceilings_for` plus `balance` plus
`store_now`, through one shared `Treasurer::evaluate` that admission, the guard and the budget derivation all call
(one function per rule; admission's short-circuit on the first exhausted ceiling is preserved). The worker attaches
the guard per run where `with_cancellation_probe` and `with_treasury_ledger` are attached today.

**G3. Wiring order.** `build_run_api` is reordered so the notice store and the `Treasurer` are built before the
worker pool; it holds `Arc<Treasurer>` and coerces it to `Arc<dyn AllowanceAdmissionPort>` for the submission
service and the agent state, and the pool takes `RunWorkerPool::with_treasurer(Arc<Treasurer>)`.

**Pitfall 9. One notice write per ceiling per window per run.** The guard's balance reads are required every
boundary, but the store write that claims a notice is memoised per ceiling and window so it happens once per run,
never once per boundary. The store remains the dedup truth; the memo only skips redundant writes.

### (b) A typed halt cause and a correct terminal status (D-05; G1, G4, G10, G14)

**D-05. The engine outcome carries a typed cause.** `RunOutcome::Halted` becomes `Halted { waypoint, cause }` with
`#[non_exhaustive] HaltCause { CancelRequested, Token, Spend(HaltReason) }`. The worker's `map_outcome`, the
engine's own `RunFinished`, the SSE `done`, the run row, the caller webhook and the herald all read that one value
instead of re-deriving the reason from side flags. Boundary priority is probe cancel, then token, then the spend
guard, so a cancel wins over a spend halt at the same boundary.

**G4. `RunFinishStatus` stays a `Copy` unit enum.** It gains a unit variant `Cancelled` and becomes
`#[non_exhaustive]`; the reason rides a new `TraceEvent::RunFinished.halt_reason: Option<HaltReason>` with
`#[serde(default, skip_serializing_if = "Option::is_none")]`, the precedent the `cost` field set. A payload variant
`Halted { reason }` was rejected because it drops `Copy`, rewrites the JSON of every stored halted `run_traces`
row and breaks every `*status ==` comparison. Stored rows read back with `None`, and the hand-written
`TraceRecord` serde impl must round-trip the nested object (the discriminator is never flattened into the
record). Cost rating: costly, because `RunFinishStatus` is persisted and read by `paladin-eval`.

**G1. A same-instance caller cancel reaches the engine as `CancelRequested`.** `RunSubmissionService::cancel`
writes the durable flag and then cancels the per-run child token, which is a child of the worker's shutdown token,
so a bare token halt cannot tell a caller cancel from a drain. The worker attaches a `PerRunCancelProbe` to the
per-run engine answering `db.is_cancelled(t) || (local_child_token.is_cancelled() && !shutdown_token.is_cancelled())`.
`map_outcome` keeps re-querying `is_cancel_requested` and `shutting_down` for the residual `Token` cause, as
defence in depth: it never regresses to reading only the cause. A cancel landing on another instance reaches the
engine through the debounced database probe only, so the row is correct and only that boundary's live `done` can
lag.

**D-15. A drain is not terminal.** A halt whose cause is the worker's own shutdown token produces no `done`: the run
stays `Running` and is requeued as today, and a connected stream falls through to the degraded polling path on
reconnect. `RunEventBusSink` takes the coordinator's shutdown token and drops a `RunFinished { Halted }` that
carries no `halt_reason` while that token is cancelled. Today's `done: halted` for a run that is still `Running` is
removed in the same code region.

**G10. Replay agrees with the row.** `replay_stream` is a third `done` path. When a replayed `RunFinished` maps, it
emits the run row's `terminal_payload` if the run is terminal, so pre-phase traces for caller-cancelled runs (which
stored `halted`) also converge on the truth.

**G14. The reason is written before the status flips.** For `Halted` and `Cancelled` transitions the worker calls
`record_outcome` before `update_status` (no adapter guards `record_outcome` on status, so this is legal), so a
degraded poller never reads `Halted` with a missing `halt_reason`. End-to-end tests that compare `done` with
`GET /runs/{id}` poll until the status is terminal.

**Invariant.** `every_halt_cause_maps_to_one_status_on_every_leg` (plan 42-06): for each `HaltCause` and each
`HaltReason` kind, the worker's `map_outcome` status, the live `done` status and the degraded `terminal_payload`
status are the same string, so a future cause that only one leg learns goes red.

### (c) The persisted reason and its one wire builder (D-06, D-14, D-16; G6, G7)

**D-06. A typed `halt_reason` column on the run row (one-way).** Migration `013_add_run_halt_reason.sql` on both
backends adds only `runs.halt_reason` (`TEXT NULL` on SQLite, `JSONB NULL` on PostgreSQL, matching `runs.output`).
`Run` and `RunOutcomeRecord` gain `halt_reason: Option<HaltReason>` (`#[serde(default)]`, additive). The column
stores the structured serde form (integer nanos plus currency), never a display string (D-00h). `error` stays
`None`: a halt is a resume point, not a failure. All three run adapters round-trip it under the shared contract
suite, and a legacy row reads back with no reason.

**G6. The discriminator is `reason`, not `kind`.** `HaltReason` is
`#[serde(tag = "reason", rename_all = "snake_case")] #[non_exhaustive]` with variants
`AllowanceExhausted(AllowanceRefusal)` and `LedgerUnavailable`, deriving `Eq` (so `AllowanceRefusal` gains `Eq`,
additive; G17). The Phase 41 `429` `details` object already uses `kind` for the limit kind, so naming the
discriminator `kind` would shadow it and the object would stop equalling the `429` body.

**D-14, D-16. One wire object.** `HaltReason::wire_json()` is the only builder of the caller-facing object:
`{"reason":"allowance_exhausted","scope","kind","balance","ceiling","window_start","window_end"}` (the Phase 41
`429` details keys plus `reason`, built from one shared `AllowanceRefusal::details_json()` that
`ApiError::allowance_exhausted` is refactored to call) or `{"reason":"ledger_unavailable"}` with no figures.
`GET /runs/{id}`, `GET /runs`, the SSE `done` on all three paths, the caller webhook key and the agent response all
call it, so the live and degraded modes agree byte-for-byte. A ledger-unavailable halt is a `done`, never an
`error`, on both paths.

**G7. The fork point is on the run.** `RunResponse` gains additive `final_waypoint_id` and `halt_reason`, so the
resume recipe (group (d)) works from `GET /runs/{id}` alone; `GET /threads/{id}/history` stays documented as the
alternative.

**Migration split.** RESEARCH sketched the run column and the notice rung in one `013`. They are split:
`013` is single-purpose (the run column) and the notice change lands as `014_add_treasury_notice_kind.sql` with its
own plan (group (g)), so each one-way schema door is opened by exactly one plan.

### (d) Resume is a fork; agent-kind runs have no checkpoint (D-07, D-08; G13)

**D-07. Resume is a new run forked from the halted Waypoint.** `Halted` stays terminal: no `Halted -> Queued` edge,
no new route, no re-enqueue of the same run id. The caller resumes through the existing
`POST /threads/{id}/fork` with `from_waypoint_id` set to the halted run's `final_waypoint_id`.
`RunSubmissionService::fork` already runs `ensure_thread_visible`, `authorize_invocation` and `admit_and_persist`,
so admission is re-checked: `429 allowance_exhausted` with `Retry-After` while the allowance is still exhausted,
and the new run records `fork_from`. The engine's `resume`-from-Waypoint path is what makes the fork continue
rather than re-execute. A `ledger_unavailable` halt resumes the same way once the ledger is back. The recipe is
documented in `platform-api.md`.

**G13. The skipped superstep number is not fixed.** The top-of-loop `Halted` Waypoint is written with
`superstep = superstep_number` (the superstep about to run) while `resume` and `fork` continue at
`latest.superstep + 1`, so one superstep number is skipped on resume. This is existing cancel-halt behaviour and
fixing it would move existing contract tests, so this phase leaves it. Tests assert the vanguard, the Battlefield
and that nodes of completed supersteps are not re-dispatched, never absolute superstep numbers across a halt and
fork.

**D-08. Agent-kind runs have no checkpoint.** A worker-dispatched `Runnable::Agent` run writes no Waypoint, so
there is nothing to fork from: a halted agent-kind run is resumed by a fresh `POST /runs` that re-executes from the
start, and the HTTP agent routes have no run row at all. ALLOW-03's "continuing from its last checkpoint" is met on
the engine path and recorded as not applicable by construction on the agent loop (a WINDOWS.md row, plan 42-09).

### (e) The derived agent budget (D-09, D-10, D-11, D-12; G2, G8, G12, G16, G17; A5)

**D-09 with G16. The arithmetic.** `max_tokens = floor(min(ceiling - balance) * 1_000_000 / dearest)` computed in
`i128`, where the minimum runs over the applicable ceilings and `dearest` is the largest of ALL FIVE `PriceRow`
axes (`prompt`, `completion`, `cache_write`, `cache_read`, `reasoning`) with the same defaults `cost_of_call` uses,
saturating to `u32` (`u32::try_from(..).unwrap_or(u32::MAX)`). A free model (dearest 0) gets no Treasurer budget,
and a derived figure of zero is a refusal (a `429` carrying the binding ceiling's figures), not a zero-token run.
Because `TokenBudget` compares `cumulative_tokens > max_tokens` (strict) after a response, the guarantee is bounded
by one model response, never absolute. Sub-counts are contained in `prompt` and `completion`, so total tokens at the
dearest price is a valid upper bound for the run.

**G8. Where the figure is derived.** Admission does not know the model, and a worker-dispatched agent run executes
later and possibly elsewhere, so a figure derived at `POST /runs` time is stale and there is no column to carry it.
The HTTP agent routes derive at admission through a new DEFAULTED port method
`AllowanceAdmissionPort::admit_for_model(subject, run_id, model)` (the default delegates to `admit`, so no
implementor breaks). The worker RE-derives at dispatch for agent-kind runs, with no persisted figure. A zero or
unreadable figure at dispatch records `Halted` without calling the LLM (`allowance_exhausted` or
`ledger_unavailable`; assumption A8: a halt is a resume point). A model that lost its price row between admission
and dispatch records `Failed` with a typed error naming the model, also without calling the LLM, because that is a
configuration incoherence and not a resume point. The figure is not persisted on the run row.

**A5. What figures the halt reports.** When the derived budget halts the loop, the `halt_reason` reports the
binding ceiling (the smallest remaining) with `balance` equal to the ceiling and that ceiling's window. This is a
conservative bound, because the loop does not know the true post-spend balance, and the docs say so.

**D-10. An unpriced model under an allowance is refused with `422`.** A principal with a ceiling calling an agent
whose model has no `treasurer.pricing` row is refused with `422`, code `model_unpriced`, `details: { "model": <name> }`
and no `Retry-After`: a configuration incoherence, not quota exhaustion, so a client must not retry it as pacing.
This applies on the HTTP agent routes (`ApiError::model_unpriced`) and on `POST /runs` for agent-kind assistants
(`RunSubmissionError::ModelUnpriced`). Principals with no ceiling are unaffected and Phase 38 D-08's warn-once
stays.

**D-11 with G12. Tightest wins, one budget, a tie goes to the Treasurer.** One `TokenBudget` per run, whose
effective limit is the smaller of the operator's `agent_runtime.token_budget.max_tokens` (when enabled) and the
Treasurer-derived figure. The derived figure is a per-run input to the same `after_model` comparison, carried on
`RunScope` (`derived_token_budget`) and read through the `ModelCallContext` scratch; it never lives on the shared
middleware struct (limits.rs D-03) and is not a second middleware, loop check or stop path. A tie goes to the
Treasurer, so the stop is the D-12 halt; an operator win keeps today's `StopReason::TokenBudget`. The shared
boot-time `PaladinExecutionService` (the worker's agent-kind path and every engine node) installs `TokenBudget` in
Treasurer-only mode, with the operator figure forced off, so engine nodes are never capped, avoiding the hazard
ADR-0052 rejected. Per-agent HTTP services install it with the operator's figure plus the derived one, and
`configuration.md` documents that the operator budget now takes effect on the HTTP agent routes (it was inert
before: `build_chain` has zero production callers). `ModelCallLimit`, `ToolCallLimit` and the Commissary are
untouched.

**D-12 with G17. A new typed stop reason, neither successful nor a failure.** `StopReason` (already
`#[non_exhaustive]`, so the variant is not a break) gains `AllowanceHalted(AllowanceRefusal)` with wire label
`allowance_halted`, `is_limit() == true` and `is_successful() == false`. The worker's `run_agent` maps a result with
that stop reason to `RunStatus::Halted` plus the `halt_reason` column instead of today's unconditional `Completed`.
`POST /agents/{id}/execute` answers `200` with `stop_reason: "allowance_halted"` and a `halt_reason` object, and
partial output is kept with the truncation notice exactly as `TokenBudget` does today. `stop_reason_label` (whose
`_ => "unknown"` arm would silently mislabel the variant) and its `stop_reason_labels_are_stable` test are extended.
`ExecuteResponse`, a frozen v0.9 schema, gains `halt_reason` behind a new sanctioned golden exception (G9).

**G2 with the option-b amendment. The scope of D-12's stream clause.** A true streamed `execute/stream` call is one
provider call with no `after_model`, so the server cannot halt it mid-flight. It is admission-only: D-10 and the
zero-budget refusal still apply, and its overshoot is that one whole call. The buffered fallback of `execute/stream`
and `POST /agents/{id}/jobs` go through `execute_scoped` and carry the derived budget (`jobs` is already admitted
since 41-04; this phase adds the budget). The operator selected option-b, so D-12's clause "the agent stream's
terminal `done` carries the same object" holds for BOTH: the buffered fallback's `done` IS the serialized
`ExecuteResponse` and so carries `stop_reason: "allowance_halted"` and `halt_reason`; and a true stream's terminal
`done` carries an informational `halt_reason` object when the terminal chunk's usage crossed the derived figure.
That object reports a crossing, not a halt the server performed. A stream whose usage did not cross the figure keeps
`{ "done": true, "usage" }` byte-identical for existing SDK smoke tests. The WINDOWS.md row for G2 (plan 42-12)
records that a single streamed call is not cut mid-flight. Under the rejected option-a the clause would have held
for the buffered fallback only.

### (f) The drain rule (D-15)

Recorded under (b): a drain emits no `done`. Restated here because it is the one place a `Halted` engine outcome is
deliberately not terminal for the caller: the run stays `Running` and is requeued, the bus drops the drain's
`RunFinished { Halted }`, and the degraded polling path converges on the row. Assumption A7 notes a halt at the
exact instant shutdown begins could be mis-suppressed or mis-emitted; the row remains the truth.

### (g) Operator notices: a mid-run warn and a halt rung (D-17, D-18; G5; Pitfall 9)

**D-17. The boundary check also evaluates `warn_at`.** The same evaluation that halts also detects a `warn_at`
crossing and claims the once-per-window notice through the Phase 41 store, emitting the one
`TraceEvent::AllowanceWarning` through the run's own trace emitter and the operator `allowance_warning` webhook,
exactly as admission does. A long run no longer waits for the next admission to warn.

**D-18 with G5. A spend halt notifies the operator once per window (one-way).** A spend halt claims a notice with a
new `notice_kind = 'halt'` and enqueues one operator delivery with the new `RunEventKind::AllowanceHalted`
(`allowance_halted`), reusing the twelve-key payload with `event` set, `run_id` the halted run and `warn_at` the
ceiling's configured threshold (no thirteenth key). Migration `014_add_treasury_notice_kind.sql` (both backends)
adds `treasury_notices.notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (notice_kind IN ('warning','halt'))` and
drops and recreates `idx_treasury_notices_once` with `notice_kind` appended; the adapters' `NOTICE_INSERT` arbiter
list moves with it and `notices_for_run` returns warning rows only, so a halt notice is never replayed as a warning
trace event. Both webhook adapters parse `allowance_halted` and `WebhookDeliveryService`'s operator signing branch
accepts either event. A `ledger_unavailable` halt emits no operator notice (the store that would dedupe it is the
one that is down) and is logged at `error`. A raised ceiling re-arms the halt notice like the warning, and twenty
runs halting in one window produce one operator notice. SQLite acceptance of `ADD COLUMN ... NOT NULL DEFAULT ...
CHECK (...)` is assumption A3; the first red test of plan 42-10 proves it and the fallback is the table-rebuild
idiom.

### (h) Caller legs, registers and the vocabulary guard (D-19, D-21; G9, G15; ALLOW-05)

**D-19.** The caller's `Halted` run webhook payload gains an optional `halt_reason` key (signed once over the stored
bytes, unchanged), and `HeraldTraceSink` folds `RunFinished.halt_reason` into one herald line rendered by the
markdown, JSON and table heralds.

**D-21 with G9, G15.** Registers and docs land in the same commits as the code: `MIGRATION.md` 9.2, 9.4 and 9.6,
the semver-checks allowlist keyed on `ALLOW-03` and `PLAT-09`, `WINDOWS.md`, `CHANGELOG.md` and `make
api-surface-update`. WINDOWS.md gains rows for D-08 (agent-kind runs have no checkpoint), D-01 (the over-admission
race stays accepted), G2 (the streamed call) and G15 (an allowance cannot bound spend on unpriced engine nodes, so
operators must price every model a metered principal can reach).

**ALLOW-05 vocabulary guard.** A root integration test `tests/treasurer_vocabulary_guard.rs` walks the repository
with `std::fs` (no new crate) and fails when `Treasurer` appears under `examples/`, `benches/`, `fixtures/`, any
`tests/fixtures/`, or any `crates/*/examples` or `crates/*/benches` path, or when `GarrisonTreasury` appears outside
an explicit allowlist of files that document the guardrail. It proves it can fail by scanning a planted temporary
tree first.

## Considered Options

Group (a), the boundary check:

- **Reserve a hold per superstep** (rejected, D-01): closes the same-instant over-admission race but needs an
  estimate rule for a superstep's cost (a pre-flight price prediction the research ruled out) and pulls a
  reserve/settle/release attachment into Phase 42, against D-00b. It can be added later beside the check without
  changing it, and is the deferred mitigation for the accepted race.
- **Check-only now, hold for concurrency later** (rejected as a Phase 42 deliverable, D-01): the same destination
  as the deferral above, without committing this phase to it.
- **A debounced probe like `DbCancellationProbe`, or a local short-circuit** (rejected, D-02): a cached `Continue`
  misses spend by other runs sharing the scope, which is the case that matters.
- **Fail open with a log line, or bounded retries then halt** (rejected, D-03): fail-open turns a database blip
  into unbounded spend; retries delay the same decision and add a timing parameter nobody asked for.
- **Re-evaluating a child's halt at the parent** (rejected, G11): a window can roll between the two reads.

Group (b), the halt cause and terminal status:

- **A `Halted { reason }` payload variant of `RunFinishStatus`** (rejected, G4): drops `Copy`, rewrites stored
  `run_traces` JSON and breaks `*status ==` comparisons in `herald.rs` and `paladin-eval`.
- **The worker publishes `done` itself after `map_outcome`** (rejected, D-05/G1): reverses the single
  `map_trace_event` mapping and leaves persisted traces wrong.
- **The bus sink consults the repository** (rejected, D-05): a database read inside a trace sink, and the sink is
  documented as lossy.
- **Reading only the engine's cause for a caller cancel** (rejected, G1): regresses the same-instance cancel the
  worker handles today.
- **Out of scope for a drain, or a new `requeued` wire status** (rejected, D-15): leaves `done: halted` on a run
  that is still `Running`, or adds a status no persisted row ever holds; deferred as an idea.

Group (c), the persisted reason:

- **Reusing the `error` text column, or trace and events only** (rejected, D-06): a halt is not a failure, and a
  client that was not streaming could not learn why it stopped.
- **`error` for a ledger outage, or the live path only** (rejected, D-16): `error` is reserved for `Failed`, and a
  degraded reader must see the same terminal payload.
- **A discriminator named `kind`** (rejected, G6): shadows the limit kind.
- **One combined migration `013` for the run column and the notice rung** (rejected, plan item 3): two one-way
  schema doors in one file and one plan.

Group (d), resume:

- **Re-enqueue the same run id, or a first-class `POST /runs/{id}/resume` with a `Halted -> Queued` edge**
  (rejected, D-07): changes Phase 27's absorbing-state machine and adds a route, where the fork lifecycle already
  re-checks admission and records `fork_from`. Deferred as an idea.
- **Automatic resubmission when the window resets** (deferred, D-07): a scheduling feature, not a resume contract.
- **Fixing the skipped superstep number on resume** (rejected, G13): moves existing contract tests for a cosmetic
  gain.

Group (e), the derived budget:

- **Completion price, or an observed blend re-derived per call** (rejected, D-09): neither can promise the bound a
  dearest-axis figure gives; the blend is a deferred idea.
- **Skip derivation and run unmetered, or reject at boot** (rejected, D-10): silently unbounded, and a boot check
  cannot see a model chosen per request.
- **Two budgets side by side, or the Treasurer budget replacing the operator one** (rejected, D-11): two
  overshoot rules, or an operator limit that silently stops working.
- **Reusing `TokenBudget` and inferring the halt at the worker, or a typed error with the run `Failed`** (rejected,
  D-12): the first cannot tell whose figure won, and the second makes a resume point a failure.
- **Persisting the derived figure on the run row** (rejected, G8): an extra column outside D-06, stale after queue
  delay.
- **A `429`-shaped unpriced refusal** (rejected, D-10): carries pacing semantics and a `Retry-After` for a
  configuration incoherence.
- **An operator `TokenBudget` on the shared boot-time service** (rejected, G12): re-creates the hazard ADR-0052
  rejected, capping engine nodes and returning a successful partial result into the Battlefield.
- **A true streamed `done` that stays byte-identical with no halt information** (option-a at the gate, not
  selected): a streaming client whose single call exhausted the allowance would learn it only at its next
  admission. The operator selected option-b instead.

Group (g), notices:

- **A new `limit_kind` value for the halt rung** (rejected, G5): the `CHECK` constraint and unique index make it a
  table rebuild on SQLite anyway, and it conflates what was crossed with what was done about it.
- **Operator notice per halted run, or caller webhook and trace only** (rejected, D-18): twenty runs halting in one
  window would page the operator twenty times; the caller's own `Halted` webhook already fires per run.
- **A thirteenth payload key for the halt kind** (rejected, G5): the twelve-key operator payload is a documented
  contract.

Cross-cutting:

- **Two ADRs (halt contract, terminal status), or amending ADR-0052 and ADR-0056** (rejected, D-20): the decisions
  are one design and later plans cite one record; ADR-0052 and ADR-0056 stay as accepted, with ADR-0056 gaining
  only a dated pointer.

## Code Locations

Owning plans are named; the files are from 42-PATTERNS.md's classification table.

- `crates/paladin-core/src/platform/container/allowance.rs` - `HaltReason`, `AllowanceRefusal::details_json`,
  `Eq` on `AllowanceRefusal`; plan 42-02
- `crates/paladin-ports/src/output/spend_guard.rs` - `SpendGuard`, `SpendDecision`, `NeverHalts`; plan 42-02
- `crates/paladin-battalion/src/engine/{mod,superstep}.rs` - `HaltCause`, `RunOutcome::Halted.cause`,
  `WarEngine::with_spend_guard`, `ChildEngineResources.spend_guard`, the guard at the top-of-loop boundary check;
  plan 42-02 (hardening and resume proofs, 42-04)
- `src/application/services/treasurer/{mod,evaluate,guard,derive}.rs` - `Treasurer::evaluate`,
  `TreasurerSpendGuard` and its memoised first halt (42-02), `derive.rs` and `Treasurer::derive_budget` (42-07),
  the warn and halt-notice claims (42-11)
- `crates/paladin-storage/migrations/{sqlite,postgres}/013_add_run_halt_reason.sql`,
  `crates/paladin-storage/src/run/{in_memory,sqlite,postgres}.rs`, `.../run/contract_tests.rs`,
  `crates/paladin-core/src/platform/container/run.rs` - the persisted `halt_reason`; plan 42-03
- `crates/paladin-web/src/run_controller.rs` - `RunResponse.halt_reason`, `RunResponse.final_waypoint_id`; plan
  42-03; `docs/src/api-reference/platform-api.md` resume recipe; plan 42-04
- `crates/paladin-core/src/platform/container/trace.rs`, `src/application/services/run/events.rs` -
  `RunFinished.halt_reason`, `run_finish_status`, `map_trace_event`, `terminal_payload`, `replay_stream`; plan 42-05;
  `RunFinishStatus::Cancelled`, the drain rule and `RunEventBusSink::with_shutdown_token`; plan 42-06
- `src/application/services/run/{worker,cancel}.rs` - `map_outcome`, guard attachment, `PerRunCancelProbe`,
  `record_outcome` before `update_status`; plans 42-02, 42-03 and 42-06; the agent-kind halt path; plan 42-09
- `crates/paladin-core/src/platform/container/{execution_result,run_scope}.rs`,
  `src/application/services/paladin/middleware/limits.rs` - `StopReason::AllowanceHalted`, `RunScope`'s derived
  budget, the Treasurer-aware `TokenBudget::after_model`; plan 42-07
- `src/infrastructure/web/{run_api_wiring,agent_host,facade_provisioner}.rs`,
  `crates/paladin-web/src/{agent_controller,error}.rs`, `crates/paladin-web/tests/openapi_golden_v0_9.rs` - wiring
  order, per-agent budget, `ExecuteResponse.halt_reason`, `ApiError::model_unpriced`, the streamed `done` under
  option-b, the golden exception; plan 42-08; the shared-service Treasurer-only mode and
  `RunSubmissionError::ModelUnpriced`; plan 42-09
- `crates/paladin-storage/migrations/{sqlite,postgres}/014_add_treasury_notice_kind.sql`,
  `crates/paladin-storage/src/treasury/`, `src/application/services/run/webhook/` - the notice rung and
  `RunEventKind::AllowanceHalted`; plan 42-10
- `src/infrastructure/telemetry/herald_sink.rs`, `crates/paladin-herald/src/`, `tests/treasurer_vocabulary_guard.rs`,
  `MIGRATION.md`, `WINDOWS.md`, `CHANGELOG.md` - the herald line, the vocabulary guard and the registers; plan 42-12
- `.planning/phases/42-mid-run-halt-sse-terminal-status/42-CONTEXT.md` - D-01..D-21

## Code Conformance

conforms

Re-read against the tree at the close of Phase 42 (plan 42-12): every decision above is built, and each
test named below was found in the tree by `grep` (the first line is the file that holds it). The section was
written as `must change` by plan 42-01, before any Phase 42 code existed, and flipped by this closeout.

- **D-01, D-02, D-03, D-04:** the engine's `spend_guard_tests` module (`crates/paladin-battalion/src/engine/mod.rs`),
  `engine_spend_halt_tracer` (`src/application/services/run/http_surface_tests.rs`: halt at boundary N with no
  node of superstep N dispatched, and no halt without a ceiling),
  `guard_halts_at_exactly_the_ceiling_and_continues_one_nano_below`
  (`src/application/services/treasurer/tests.rs`), `unattributed_run_gets_no_guard_and_reads_no_ledger`
  (`src/application/services/run/worker_tests.rs`), `ledger_unavailable_halt_resumes_after_recovery`
  (`src/application/services/run/http_surface_tests.rs`).
- **G11:** `guard_memoises_its_first_halt` (`src/application/services/treasurer/tests.rs`) and
  `child_battalion_halt_on_spend_halts_the_parent` (`crates/paladin-battalion/src/engine/superstep.rs`).
- **D-05, G1, G4, D-15, G10:** the worker's `map_outcome_*` enumeration rows, one per cause
  (`map_outcome_spend_halt_records_the_reason`, `map_outcome_spend_halt_ignores_a_cancel_flag`,
  `map_outcome_cancel_requested_cause_transitions_to_cancelled`, ... in `src/application/services/run/worker.rs`);
  the `map_trace_event` rows (`map_trace_event_renders_the_halt_reason_on_done`,
  `cancelled_run_finished_maps_to_done_cancelled_without_a_reason` in `src/application/services/run/events.rs`);
  the `run_trace` contract clauses `run_finished_halt_reason_round_trips` and
  `legacy_run_finished_row_reads_back_without_a_halt_reason`
  (`crates/paladin-storage/src/run_trace/contract_tests.rs`, run on all three adapters); and
  `every_halt_cause_maps_to_one_status_on_every_leg` (`src/application/services/run/stream_tests.rs`).
- **D-06, G6, G7, G14:** the run-store contract clauses `halt_reason_round_trips_on_record_outcome`,
  `legacy_row_reads_back_without_a_halt_reason` and `record_outcome_before_status_flip_is_accepted`
  (`crates/paladin-storage/src/run/contract_tests.rs`, on all three adapters).
- **D-07, D-08, G13:** `halted_run_resumes_by_fork_after_window_reset`
  (`src/application/services/run/http_surface_tests.rs`), and the agent-kind row of `.planning/WINDOWS.md`
  (row 64).
- **D-09 through D-12, G2, G8, G12, G16, G17:**
  `derived_budget_composes_with_the_commissary_rationed_rag_context`
  (`src/application/services/paladin/paladin_execution_service.rs`), `agent_execute_halts_on_the_derived_budget`
  (`src/infrastructure/web/agent_host.rs`), `agent_kind_run_halts_on_the_derived_budget`
  (`src/application/services/run/http_surface_tests.rs`),
  `execute_stream_buffered_fallback_done_carries_the_halt_reason` and `stop_reason_labels_are_stable`
  (`crates/paladin-web/src/agent_controller.rs`), and the unit tests of `derive_max_tokens` and
  `dearest_price_per_million` (`src/application/services/treasurer/derive.rs`). The option-b clause of item 12
  (a true stream's `done` carries an informational `halt_reason` only when its terminal usage crossed the
  derived figure) is held by `execute_stream_true_stream_done_is_byte_identical_without_a_crossing` and
  `execute_stream_true_stream_done_carries_halt_reason_when_usage_crossed` in the same file.
- **D-17, D-18, G5:** `mid_run_warn_and_halt_notices_reach_the_operator_once`
  (`src/application/services/run/http_surface_tests.rs`),
  `warning_and_halt_notices_for_one_identity_are_both_recorded` and `notices_for_run_returns_warning_rows_only`
  (`crates/paladin-storage/src/treasury/notice_contract_tests.rs`, on all three adapters) and
  `allowance_halted_operator_row_round_trips` (`crates/paladin-storage/src/webhook/contract_tests.rs`).
- **D-19 (the herald line):** `halt_reason_herald_line_for_a_window_ceiling`,
  `halt_reason_herald_line_for_a_lifetime_ceiling`, `halt_reason_herald_line_for_a_ledger_outage` and
  `halt_reason_herald_line_names_no_tenant_or_key` (`crates/paladin-core/src/platform/container/allowance.rs`),
  `herald_sink_folds_a_halt_reason_into_one_line` (`src/infrastructure/telemetry/herald_sink.rs`), and one
  present-and-absent test per herald in `crates/paladin-herald/src/`.
- **ALLOW-05 vocabulary:** `tests/treasurer_vocabulary_guard.rs`: `treasurer_is_a_framework_only_word` over the
  tree, with the planted-tree control `scanner_reports_a_planted_downstream_use` and the clean-tree control
  `scanner_accepts_a_clean_tree`.

The accepted over-admission race (D-01) and the accepted whole-call overshoot of a true streamed call are
deliberately covered by `.planning/WINDOWS.md` rows (65 and 66) and not by a prevention test; the unpriced
engine node (G15) is row 67.

## Downstream Consumers

- **Phase 43 (rate pacing)** - composes beside the guard at the same `LlmPort` boundary as a sibling of the pricing
  decorator; it adds no ceiling and does not touch the `SpendGuard` port or the derived budget.
- **Phase 46 (docs currency, CURR-23)** - the Treasurer mdBook page reads this ADR, with ADR-0056, for the halt
  contract, the bounded-overshoot wording, the fork-as-resume recipe and the derived-budget arithmetic, and the
  `v0.10` to `v0.11` migration guide carries the MIGRATION.md rows this phase adds.
- **The deferred per-superstep hold** - the mitigation for the accepted over-admission race; it would add a
  `reserve` beside the check-only boundary without changing it, and would need the estimate rule this ADR records as
  missing.
- **Plans 42-02 through 42-12** - each cites this ADR for the resolutions it implements; plans 42-07 and 42-08 and
  42-12 additionally read the recorded option-b outcome for the streamed `done`.
