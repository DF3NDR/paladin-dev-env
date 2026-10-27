# Phase 42: Mid-Run Halt & SSE Terminal Status - Context

**Gathered:** 2026-10-06
**Status:** Ready for planning
**Mode:** interactive — every decision below was selected by the operator (five gray areas,
seventeen questions, logged in `42-DISCUSSION-LOG.md`). The recommended option was chosen in
all seventeen.

<domain>
## Phase Boundary

This phase delivers the **enforcement half of the Treasurer** — a run already admitted by Phase 41
keeps drawing from the ledger as it goes, and this phase makes it stop cleanly when the next draw
would overspend. Concretely:

1. **Engine-path halt** (ALLOW-03). At every `WarEngine` superstep boundary, beside the existing
   cancellation check, the Treasurer evaluates the caller's ceilings; when one is exhausted the
   engine writes the same `WaypointStatus::Halted` Waypoint the cancellation path writes and the
   run is recorded `Halted` with a typed Treasurer reason, the last checkpoint kept.
2. **Agent-loop halt and derived budget** (ALLOW-03, ALLOW-05). The Treasurer derives a per-run
   `TokenBudget` from the remaining allowance, installs it on the agent loop (the first production
   wiring of that middleware), and a Treasurer cutoff ends the run `Halted` with a typed reason —
   on worker-dispatched `Runnable::Agent` runs and on the HTTP agent routes.
3. **Resume** (ALLOW-03). A halted engine run resumes once the allowance is replenished or the
   window resets, continuing from its halted Waypoint rather than re-executing.
4. **Composition and the vocabulary guard** (ALLOW-05). The derived budget works alongside
   `TokenBudget`, `ModelCallLimit`, `ToolCallLimit` and the Commissary without replacing any of
   them; a guard test keeps `Treasurer` a framework-only word.
5. **SSE terminal status** (PLAT-09). The `done` event matches the persisted status — `cancelled`
   for a caller-cancelled run, `halted` with the Treasurer reason for a spend halt — on both the
   live and the degraded stream paths.

**Not in this phase:** rate pacing (Phase 43), the legacy clean break (Phase 44), the Treasurer
mdBook page (Phase 46, CURR-23), per-superstep reservation holds (deferred, see D-01), automatic
resubmission of halted runs (deferred), and any change to admission semantics (ADR-0056 is cited,
not re-opened).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by ADR-0052/0053/0056 and Phases 38-41 — cited, not re-asked)

- **D-00a:** ADR-0052 fixes the two attachment points and is cited verbatim: the engine halt is
  raised at the `WarEngine` superstep boundary (`crates/paladin-battalion/src/engine/superstep.rs`,
  the top-of-loop check that already consults the `CancellationToken` and the `CancellationProbe`
  and writes a `WaypointStatus::Halted` Waypoint), and the agent-loop halt reuses the `TokenBudget`
  `after_model` cutoff (`src/application/services/paladin/middleware/limits.rs`).
  `AgentRuntimeConfig::build_chain` stays unwired for the engine path; nothing refuses inside the
  pricing decorator.
- **D-00b:** ADR-0053 and Phase 39 are final: the `007` ledger schema, the `reserve`/`settle`/
  `release` row kinds and the settlement key `(run_id, superstep, attempt)` are untouched. The
  engine's per-superstep settle (`SpendHook::settle_boundary`,
  `crates/paladin-battalion/src/engine/settlement.rs`) and the agent loop's `settle_agent_loop_call`
  keep writing exactly as 39-04/39-05 left them.
- **D-00c:** ADR-0056 and Phase 41 are binding: tumbling UTC windows from `store_now()` (41 D-01),
  every-limit composition in the fixed order key-window → key-lifetime → tenant-window →
  tenant-lifetime (41 D-03, `AllowancePolicy::ceilings_for`), the `TreasuryLedgerPort::balance`
  read (41 D-04), the `Treasurer` service as the only policy home (41 D-06 — `paladin-battalion`
  and `paladin-storage` never learn allowances), fail-closed when a ceiling applies (41 D-10), no
  Admin bypass (41 D-09), the `429 allowance_exhausted` contract with `Retry-After` (41 D-12/D-13),
  `AllowanceRefusal` as the one refusal value (41 D-14), store-deduped once-per-window notices
  (41 D-16) and the operator webhook target (41 D-17, twelve-key payload per the 41-01 amendment).
- **D-00d:** Phase 27's run state machine stays as written: `Halted` and `Cancelled` are both
  absorbing terminal statuses, no edge leaves `Halted` (`RunStatus::try_transition`,
  `crates/paladin-core/src/platform/container/run.rs`), and the worker records `Cancelled` only
  when a caller asked (`map_outcome`, `src/application/services/run/worker.rs`).
- **D-00e:** Identity is Phase 40's: `PrincipalRef` / `RunAttribution` on the run row,
  `LedgerScope::from_attribution` as the only attribution→scope mapping, `requested_by: None`
  never gated and never attributed. Vocabulary: `Treasurer` is a framework-only word (ADR-0050).
- **D-00f:** X-03 governs public API: every break needs a `MIGRATION.md` §9.2 row and, when
  marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry; `TraceEvent`, `RunSubmissionError`
  and `Run` are `#[non_exhaustive]`; additive serialized fields use `#[serde(default)]`.
- **D-00g:** House config shape (`Default` + `validate()` that never clamps + `EnvOverridable`
  for scalars), the 82 % coverage floor (ADR-0006), `make clean-code`, `make api-surface` (+
  `make api-surface-update` and a CHANGELOG entry for an intentional surface change),
  `make security`, and the manual credential-handling review on every commit. No log line, error
  body, trace event, webhook payload or herald line ever carries an API key value.
- **D-00h:** One operator currency; every amount is `Cost { nanos: i64, currency }`; display goes
  through `format_cost` exactly once at the edge; integer arithmetic only, `i128` for products.
- **D-00i:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` — currently
  **0057** — and advance that line in the same commit. Migrations continue the `NNN_verb_noun.sql`
  pair convention; `012_add_treasury_ledger_tenant_index.sql` is the latest on both backends, so
  this phase's migrations start at **`013`**.
- **D-00j:** Streams are priced and settled from the terminal chunk's usage only (Phase 31
  contract, ADR-0052); the agent-loop cutoff therefore fires after a streamed response completes,
  never mid-stream — the research's "settle-on-nonexistent-usage" pitfall is avoided by reusing
  `TokenBudget`'s existing `after_model` mechanism rather than adding a second one.

### Superstep boundary check

- **D-01:** The boundary check is a **check-only balance read**, the same evaluation admission
  performs: for each applicable ceiling (41 D-03 order) read `balance` and halt when
  `balance >= ceiling` before the next superstep starts. **No `reserve` rows are written**;
  settlement stays exactly as 39-04 wrote it. Overshoot is bounded to one superstep's spend, the
  same posture as `TokenBudget`'s one-response overshoot and the research's "no pre-flight exact
  cost prediction". **Consequence recorded, not re-asked:** ADR-0056's Downstream Consumers line
  ("closes the D-05 over-admission race by reserving at the superstep boundary") is superseded —
  two runs admitted in the same instant can both start, and the boundary check halts them at
  their first boundary after exhaustion; ADR-0057 records the race as accepted and ADR-0056 gains
  a dated note pointing to it. — **Reversibility:** reversible — a per-superstep hold can be added
  beside this check later without changing it (deferred idea).
- **D-02:** **Every boundary, no caching.** One `balance` read per applicable ceiling (up to four)
  at every superstep boundary, on the same path the per-superstep settle already takes; a halt
  lands at the first boundary after the ceiling is reached. No `DbCancellationProbe`-style
  `min_interval` debounce and no local short-circuit — other runs sharing the scope are exactly the
  case that matters.
- **D-03:** **Fail closed.** If a boundary balance read fails (`TreasuryLedgerError::Backend`,
  `CurrencyMismatch`, …) for a principal that has at least one configured ceiling, the run halts
  with the distinct reason kind **`ledger_unavailable`** and keeps its checkpoint, mirroring
  41 D-10: a ceiling that cannot be evaluated cannot be passed. A principal with no configured
  ceiling never reads the ledger and is unaffected, so a deployment without allowances keeps
  running through a ledger outage. The halted run is resumable (D-07) once the ledger is back.
- **D-04:** **Policy stays in the Treasurer; the engine consults a port.** `paladin-battalion`
  learns no allowance: the superstep loop consults a new output port (name is the planner's;
  working shape `SpendGuard::check(&ThreadId / scope) -> Decision { Continue, Halt(HaltCause) }`,
  consulted once per boundary beside the `CancellationProbe`, both at the top of the loop), and
  the facade's `Treasurer` implements it from `AllowancePolicy::ceilings_for` + `balance` +
  `store_now`. Unlike the probe it is **not** infallible-by-construction: D-03 makes a failed read
  a halt decision, not a swallowed `false`. Attached per run by the worker exactly where
  `with_cancellation_probe` and `with_treasury_ledger` are attached today, carrying the run's
  `LedgerScope`; `None` (no Treasurer configured, or `requested_by: None`) means no check at all.

### Halt and resume contract

- **D-05:** **The engine outcome carries a typed cause.** `RunOutcome::Halted` gains a cause —
  caller cancel (from the `CancellationProbe` answer), Treasurer spend halt (carrying the
  `AllowanceRefusal` figures), `ledger_unavailable`, and the bare in-process token halt — and
  `RunFinishStatus` gains `Cancelled` and a `Halted { reason }` payload (or equivalent) so the
  engine's own terminal trace event is correct at the source. The worker's `map_outcome` reads the
  cause instead of re-querying `is_cancel_requested`; the live `done` (D-14), the persisted
  `halt_reason` (D-06) and the webhook/trace/herald legs (D-19) all come from this one value.
  Additive under `#[non_exhaustive]` where the enum is, `#[serde(default)]` for stored
  `run_traces` rows; the in-process `CancellationToken` halt keeps today's plain `Halted` cause
  (the worker still distinguishes a drain by `shutting_down`, D-15). Note that `TraceRecord` has a
  hand-written `Serialize`/`Deserialize` since commit `bc9cdf0` (Phase 41 UAT fix: one `run_id`
  key), so the new payload must round-trip through that impl and its `run_trace` contract tests,
  not only through derived serde. — **Reversibility:** costly — `RunFinishStatus` is persisted in
  `run_traces` and read by `paladin-eval`.
- **D-06:** **A typed `halt_reason` column on the run row.** Migration pair
  `013_add_run_halt_reason.sql` (both backends) adds a nullable column holding the serialized
  reason (`kind` plus the `AllowanceRefusal` figures: scope, limit kind, balance, ceiling,
  window); `Run` gains `halt_reason: Option<HaltReason>` (`#[serde(default)]`, additive on the
  `#[non_exhaustive]` struct); `GET /runs/{id}` and `GET /runs` rows gain an optional
  `halt_reason` object whose field set equals the Phase 41 `429` `details` object plus `kind`.
  `error` stays `None` — a halt is a resume point, not a failure. Round-tripped by all three run
  adapters under the shared contract suite. — **Reversibility:** one-way — a persisted column.
- **D-07:** **Resume is a new run forked from the halted Waypoint.** `Halted` stays terminal; no
  edge, no new route, no re-enqueue of the same run id. The caller resumes through the existing
  `POST /threads/{id}/fork` lifecycle with `from_waypoint_id` = the halted run's
  `final_waypoint_id`; `RunSubmissionService::fork` already runs `ensure_thread_visible`,
  `authorize_invocation` and `admit_and_persist`, so admission is **re-checked** (a `429
  allowance_exhausted` while still exhausted, with `Retry-After` naming the reset instant) and the
  new run records `fork_from`. The engine's `resume`-from-Waypoint path is what makes the fork
  continue rather than re-execute (ADR-0052: "a consistent restart point `resume` already continues
  from"). Documented in `platform-api.md` as the resume recipe for a halted run.
- **D-08 (consequence, not re-asked):** **Agent-kind runs have no checkpoint.** A worker-dispatched
  `Runnable::Agent` run writes no Waypoint (`run_agent`), so there is nothing to fork from; a
  halted agent-kind run is resumed by a fresh `POST /runs` that re-executes from the start, and the
  HTTP agent routes have no run row at all. ALLOW-03's "continuing from its last checkpoint" is
  met on the engine path and recorded as **not applicable by construction** on the agent loop: one
  `WINDOWS.md` row plus a sentence in the Treasurer docs, not a silent gap.

### Agent-loop budget derivation (ALLOW-05)

- **D-09:** **Dearest axis of the model's price row.** For a principal with at least one
  configured ceiling, `max_tokens = floor(remaining_nanos / price_per_token)` where
  `remaining_nanos = min over applicable ceilings of (ceiling − balance)` (pre-admission balance,
  41 D-05) and `price_per_token` is the **largest** of the agent model's `prompt`, `completion`,
  `cache_write` and `reasoning` per-token prices (`PriceRow`, nanos per 1M tokens, `i128`
  intermediate, integer division). Every token is assumed to be the priciest kind, so the derived
  budget can never let the run overspend the remaining allowance; it stops early on prompt-heavy
  runs, which is the documented trade. A remaining allowance of zero or a derived budget of zero
  is an admission refusal, not a zero-token run. The budget is derived once, at admission
  (`Treasurer::admit` already reads every balance), and travels to the loop per call (see
  Discretion); it is not re-derived per model call.
- **D-10:** **An unpriced model under an allowance is refused at admission.** If the principal has
  a configured ceiling and the agent's model has no `treasurer.pricing` row, `admit` refuses with
  a new typed refusal kind beside `allowance_exhausted` (wire status and `code` are the planner's
  call, `429`-shaped or `422`; the body says which model is unpriced). An allowance that cannot be
  metered is a configuration incoherence caught at the first call, the same posture as 41 D-11's
  boot cross-check. Principals with no ceiling are unaffected and Phase 38 D-08's warn-once stays.
- **D-11:** **Tightest wins, one budget.** One `TokenBudget` per run whose `max_tokens` is the
  smaller of the Treasurer-derived figure and the operator's
  `agent_runtime.token_budget.max_tokens` (when that config is `enabled`). The stop records which
  figure won: a Treasurer win is the D-12 cutoff, an operator win keeps today's
  `StopReason::TokenBudget` stop. One middleware, one `after_model` cutoff, one overshoot rule
  (at most one response). This is ALLOW-05's "works alongside, replaces none": `ModelCallLimit`,
  `ToolCallLimit` and the Commissary are untouched.
- **D-12:** **A new typed stop reason, neither successful nor a failure.** `StopReason` gains a
  variant for the Treasurer cutoff carrying the `AllowanceRefusal` figures (name is the planner's;
  wire label `allowance_halted` in `stop_reason_label`). `is_successful()` is `false` and
  `is_limit()` is `true` for it. The worker's `run_agent` maps a result with that stop reason to
  `RunStatus::Halted` plus the D-06 `halt_reason` column (instead of today's unconditional
  `Completed`); `POST /agents/{id}/execute` answers `200` with `stop_reason: "allowance_halted"`
  and a `halt_reason` object of the same shape as D-06; the agent stream's terminal `done` carries
  the same object. Partial output is kept with the truncation notice, exactly as `TokenBudget`
  does today. — **Reversibility:** costly — `StopReason` is a published `paladin-core` enum; if it
  is not `#[non_exhaustive]` the new variant is a §9.2 `Y` row with an allowlist entry.
- **D-13:** **Coverage: every principal-bearing agent path that Phase 41 admits** — the worker's
  `Runnable::Agent` dispatch (`RunScope.run_id` + `ledger_scope` from the run row) and the HTTP
  `execute` / `execute/stream` handlers (`RunScope` from the `Principal`). The derived budget is
  installed only when `admit` returned a figure; `requested_by: None` / no principal gets no
  budget, exactly as it gets no admission. `POST /agents/{id}/jobs` is addressed under Discretion.

### SSE terminal payload (PLAT-09)

- **D-14:** **`done` carries the corrected status plus the reason object.** For a Treasurer halt:
  `{ "status": "halted", "waypoint_id", "halt_reason": { ... } }`; for a caller cancel:
  `{ "status": "cancelled", "waypoint_id" }`; for the bare in-process token halt: `{ "status":
  "halted", "waypoint_id" }` with no `halt_reason`. `halt_reason` is byte-identical to the D-06
  object on `GET /runs/{id}`, so a streaming client needs no follow-up read. `error` stays
  reserved for `Failed`. Documented in `platform-api.md`'s SSE section and asserted by the
  `map_trace_event` enumeration test (`events.rs`), which gains the new status rows.
- **D-15:** **A drain is not terminal.** A halt whose cause is the worker's own shutdown token
  (the `shutting_down` / `LeaveRunningAndRequeue` arm) produces **no `done`** on the bus: the run
  stays `Running`, is requeued as today, and a connected stream falls through to the degraded
  polling path on reconnect. Today's `done: halted` for a run that is still `Running` is removed
  in the same code region PLAT-09 touches.
- **D-16:** **A ledger-unavailable halt is `done`, on both paths.** `{ "status": "halted",
  "halt_reason": { "kind": "ledger_unavailable" } }` with no figures — a resume point, never an
  `error`. The degraded polling path (`terminal_payload`) builds its `done` from the run row's
  `status` and `halt_reason`, so the live and degraded modes agree byte-for-byte on the terminal
  payload for every status; `waypoint_id` follows each path's existing rule (WINDOWS.md row 33 is
  unchanged by this phase).

### Mid-run warnings and halt notices

- **D-17:** **The boundary check also evaluates the warn threshold.** The same ceiling evaluation
  that halts also detects a `warn_at` crossing mid-run and claims the once-per-window notice
  through the Phase 41 `TreasuryNoticePort` (store-deduped, 41 D-16); a won claim emits the one
  `TraceEvent::AllowanceWarning` on the run's own stream, the herald line and the operator
  `allowance_warning` webhook exactly as admission does (41 D-17/D-18). A long run no longer waits
  for the next admission to warn; one function per rule — the planner factors admission's
  evaluate-ceilings-and-claim-notices step so both call sites share it.
- **D-18:** **A spend halt notifies the operator once per window.** The halt enqueues the Phase 41
  operator webhook target with a second event, `allowance_halted`, carrying the same twelve keys
  as `allowance_warning` (with `run_id` = the halted run) — deduped through the same notices table
  with a new limit-kind / notice-kind rung, so twenty runs halting in one window produce one
  operator notice, and a raised ceiling re-arms it like the warning. The caller's own `Halted` run
  webhook still fires per halted run. `ledger_unavailable` halts emit **no** operator notice (the
  store that would dedupe it is the one that is down); they are logged at `error`.
- **D-19:** **The reason rides every caller-facing leg.** The caller's `Halted` run webhook payload
  gains an optional `halt_reason` key (the D-06 object; an additive key under `WebhookPayload`'s
  documented key-set discipline, signing unchanged); the `RunFinished` trace event carries the
  D-05 cause; `HeraldTraceSink` folds a halt into one herald line beside the cost line, rendered
  by the markdown, JSON and table heralds (for example `⛔ halted: allowance exhausted —
  25.0000 of 25.0000 USD (api_key, window resets 2026-10-06T00:00:00Z)`), the way 41 D-18
  renders the warning.

### Bookkeeping, ADR and docs

- **D-20:** **One ADR, written first.** ADR-0057 "Mid-run halt contract: check-only boundary,
  typed halt cause, fork-as-resume, derived agent budget" records D-01..D-04, D-05..D-08, D-09..D-12,
  D-15 and D-18 with the rejected alternatives from the discussion log, cites ADR-0052, ADR-0053 and
  ADR-0056 rather than re-opening them, adds the dated note to ADR-0056 named in D-01, and advances
  `PROMOTION.md` to 0058 in the same commit. It is the phase's opening plan (42-01), so every later
  plan cites it.
- **D-21:** Registers and docs land in the same commits as the code: `MIGRATION.md` §9.2 (new
  `StopReason` / `RunOutcome` / `RunFinishStatus` variants and the `halt_reason` field — `Y` with
  allowlist where the enum or struct is not `#[non_exhaustive]`, `N` otherwise), §9.4 (migration
  `013`), §9.6 (`halt_reason` on `GET /runs*`, the `done` payload change, the `Halted` webhook key,
  the `allowance_halted` operator event, the `allowance_halted` stop reason, the unpriced-model
  refusal, and the fork-as-resume recipe); `docs/src/api-reference/platform-api.md`;
  `docs/src/getting-started/configuration.md` (what the derived budget means for
  `agent_runtime.token_budget`); root `CHANGELOG.md` `[Unreleased]`; `WINDOWS.md` rows for D-08
  (agent-kind runs have no checkpoint) and D-01 (over-admission race stays accepted); `make
  api-surface-update` for the new public items; `cargo semver-checks` allowlist entries keyed on
  `ALLOW-03` / `PLAT-09`. The Treasurer mdBook page itself stays Phase 46 (CURR-23).

### Claude's Discretion

- **Port and type names:** the boundary port (`SpendGuard`, `DrawProbe`, …) and its decision enum,
  the `HaltReason` / `HaltCause` type and whether it lives in `paladin-core` beside `allowance.rs`,
  the `StopReason` variant name, and the `halt_reason` column's exact JSON encoding (recommended:
  the `AllowanceRefusal` serde shape plus a `kind` discriminator, so the `429` details, the run
  row, the SSE payload and the webhook key all deserialize to one type).
- **How the derived budget reaches the loop:** `TokenBudget` is a per-service middleware shared by
  every call (ADR-0052's shared `EngineExecutionPort` and `build_agent`-per-agent shapes), and
  per-run scratch must never live on the middleware struct (limits.rs D-03). Recommended: carry the
  per-run figure on `RunScope` (additive `#[serde(default)]` field beside `allowance_warnings`)
  and have a Treasurer-aware `TokenBudget` read it from the `ModelCallContext` scratch, computing
  the D-11 minimum there; the operator config value remains the middleware's own.
- **`POST /agents/{id}/jobs`:** 41-RESEARCH C4 names it as a fourth spend path that Phase 41 did
  not gate. Recommended: this phase gates it with both admission and the derived budget (41 D-07's
  reasoning applies verbatim — leaving it open lets a halted caller spend the same allowance
  through `jobs`); if the planner finds a reason not to, it records a `WINDOWS.md` row rather than
  leaving the gap silent.
- **Where in the worker the boundary port is attached** (the per-run engine build beside
  `with_cancellation_probe` / `with_treasury_ledger`) and whether the `Treasurer` service grows a
  second constructor-injected handle on `RunWorkerPool` or is reached through the existing
  `with_treasury_notices`-style builder.
- **Notice rung encoding** for D-18 (a new `limit_kind` value vs a `notice_kind` column) and whether
  the `allowance_halted` operator payload adds a thirteenth key for the halt `kind`.
- **Agent-stream `done` shape:** today the agent stream ends with `{ done: true, usage }`; the
  planner decides whether `halt_reason` is added to that object or a sibling `stop_reason` field
  joins it, keeping the non-halt shape byte-identical for existing SDK smoke tests.
- **Test topology:** engine tests for halt-at-boundary and no-halt-without-a-ceiling (the
  `cancellation_probe_tests` shape in `engine/mod.rs`), worker `map_outcome` rows for every new
  cause, `map_trace_event`'s enumeration test, the run-store contract clauses for `halt_reason`,
  the HTTP end-to-end proofs (`http_surface_tests.rs`: halt → `GET /runs/{id}.halt_reason` → fork
  → `429` → window reset → fork → `Completed`), and the agent-loop test proving the derived budget
  stops a run `Halted` with partial output while the same run under an operator budget stops
  `Completed` with `token_budget`.
- **Whether the `ledger_unavailable` halt also emits a `warn`/`error` log line naming the backend
  error** (recommended: yes, the figures never include a key value).

</decisions>

<specifics>
## Specific Ideas

- An operator should be able to watch a run that is burning through `ci-runner`'s `2.50 USD`/hour
  allowance, see `⚠ allowance: 82% …` appear mid-run in the herald, then see the run stop at the
  next superstep with `⛔ halted: allowance exhausted — 2.5000 of 2.5000 USD (api_key, window resets
  …)`, `GET /runs/{id}` answer `status: "halted"` with a `halt_reason` object and `error: null`,
  and `POST /threads/{id}/fork` from the halted waypoint answer `429 allowance_exhausted` with
  `Retry-After` until the hour turns — then `202`, and the forked run finish `Completed` without
  repeating the supersteps already paid for.
- The SSE `done` for that halt must equal `GET /runs/{id}`'s status and reason field-for-field,
  whether the client stayed connected (live path) or reconnected (degraded path).
- A caller cancel streams `done: { status: "cancelled" }`; a worker drain streams nothing
  terminal; nobody ever sees `halted` for a run that is still `Running`.
- The phase's first red tests: (a) an engine test where a guard answering `Halt` at boundary N
  yields a `Halted` Waypoint whose `vanguard` is boundary N's and a `RunOutcome::Halted` carrying
  the spend cause, with no node of superstep N dispatched; (b) a `map_outcome` table proving each
  cause maps to exactly one `OutcomeAction`; (c) a `map_trace_event` row proving
  `RunFinished { Cancelled }` maps to `done`/`"cancelled"`; (d) an agent-loop test proving a
  Treasurer-derived budget of N tokens halts a mock-scripted run after the crossing response with
  `is_successful() == false` and the partial content kept.
- `Treasurer` stays a framework-only word: the guard test from ALLOW-05 greps the tree for the
  word outside the framework vocabulary allowlist (ADR-0050, Milestone 14 overview §3) and fails on
  any new downstream use.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked design (cite, do not re-open)
- `.planning/decisions/0052-mid-run-treasurer-enforcement.md` — the two attachment points
  (engine superstep boundary, agent-loop `TokenBudget` cutoff), the rejected alternatives
  (wiring `build_chain` into the engine; refusing inside the pricing decorator), `build_chain`'s
  zero production callers.
- `.planning/decisions/0056-allowance-admission-model.md` — tumbling windows, check-only
  admission, every-limit composition, store-deduped notices; its Downstream Consumers line on
  Phase 42 reservations is superseded by D-01 (dated note required).
- `.planning/decisions/0053-ledger-balance-model.md` — append-only ledger, derive-on-read balance,
  settlement key; nothing here changes.
- `.planning/decisions/0050-treasurer-reservation.md` — the framework-only word and the guard
  ALLOW-05 requires.
- `.planning/decisions/PROMOTION.md` — next free ADR number **0057** (D-20).
- `.planning/phases/41-admission-time-allowance-enforcement/41-CONTEXT.md` — D-01 (windows),
  D-03 (ceiling order), D-04 (`balance`), D-05 (check-only, the race), D-06 (`Treasurer` service
  and injection sites), D-07 (gated paths), D-10 (fail closed), D-12..D-14 (refusal contract and
  `AllowanceRefusal`), D-15..D-18 (warn evaluation, notices table, operator webhook, trace/herald
  legs), Claude's Discretion (names this phase inherits).
- `.planning/phases/41-admission-time-allowance-enforcement/41-RESEARCH.md` — row C4 (the
  ungated `POST /agents/{id}/jobs` path) and the threat table's "over-admission race" row.
- `.planning/phases/39-spend-ledger/39-CONTEXT.md` — D-03 (policy-free `reserve`), D-07
  (engine-path `attempt`), D-08 (settle-only writer; observational ledger).
- `.planning/phases/40-tenant-identity-run-read-scoping/40-CONTEXT.md` — D-04 (`PrincipalRef`),
  D-11/D-12 (read scope, Admin read bypass), D-16 (agent-execute attribution via `RunScope`).
- `.planning/phases/38-design-seams-pricing-cost-producer/38-CONTEXT.md` — D-02 (nano-units,
  `i128` products), D-06 (price row axes and fallbacks — the axes D-09 compares), D-08 (warn-once
  for unpriced models), D-09 (pricing decorator), D-13 (ADR-0052 posture).

### Milestone scope and requirements
- `.planning/ROADMAP.md` §"Phase 42" — goal, success criteria 1-4, research flag (no prior art for
  a resumable mid-run halt; the streaming pitfall and the "reuse `TokenBudget`'s cutoff" rule).
- `.planning/REQUIREMENTS.md` — ALLOW-03, ALLOW-05, PLAT-09 (this phase); ALLOW-01/02/04 and
  PLAT-07/08 (what this phase builds on); "Out of Scope" (no rework of the limit middleware, no
  renames); FUT-04/FUT-12.
- `.planning/research/SUMMARY.md` — §"Phase 42", Pitfall 5 (streaming settle-on-nonexistent-usage),
  the "genuinely open architectural question" ADR-0052 closed, the comparable-systems survey.
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` — R3 (the Treasurer
  "installs the per-run `TokenBudget` mechanism"), §4 out of scope (composes, does not replace).
- `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md` §3 — vocabulary guardrail.

### Breaking-change register and conventions
- `MIGRATION.md` §9.2, §9.4, §9.6 — row columns and the set-equality contract (D-21).
- `.cargo/semver-checks-allowlist.toml` — entry schema (`requirement_id = "ALLOW-03"` / `"PLAT-09"`).
- `.planning/WINDOWS.md` — row 33 (the `done` payload's null `waypoint_id` and reduced content —
  unchanged), row 58 (`/threads/*` not tenant-scoped — the fork route D-07 relies on; note that
  `RunSubmissionService::fork` itself runs `ensure_thread_visible`), and the row format for D-08
  and D-01.

### Code the phase extends (read, do not re-derive)
- `crates/paladin-battalion/src/engine/superstep.rs` — the top-of-loop boundary check
  (`token_cancelled || probe_cancelled` → `build_waypoint(.., WaypointStatus::Halted, ..)` →
  `RunOutcome::Halted`), `ChildEngineResources` (child runs inherit the probe; the guard follows),
  the nested-child `Ok(RunOutcome::Halted { .. })` arm.
- `crates/paladin-battalion/src/engine/settlement.rs` — `SpendHook::settle_boundary` (the
  per-superstep settle the check sits beside; child hooks are no-ops).
- `crates/paladin-battalion/src/engine/mod.rs` — `RunOutcome` (gains the cause), `run_finish_status`,
  `with_cancellation_probe`, `with_treasury_ledger(ledger, SettlementContext)`, the four
  `trace.emit(TraceEvent::RunFinished { .. })` sites, `cancellation_probe_tests`.
- `crates/paladin-ports/src/output/cancellation_probe.rs` — the probe port the new guard port
  mirrors in placement but not in fallibility (D-03/D-04); `src/application/services/run/cancel.rs`
  — `DbCancellationProbe` (the debounce D-02 declines).
- `src/application/services/run/worker.rs` — `map_outcome` (D-05 reads the cause; D-15 drain arm),
  the per-run engine build where probe and ledger are attached (D-04 attachment site), `run_agent`
  (D-12 mapping; writes no Waypoint, D-08), `run_status_to_event_kind`, `enqueue_webhook_delivery`
  (D-19), the D-14 comment block explaining today's `halted`-for-cancelled simplification.
- `src/application/services/run/events.rs` — `map_trace_event` (`RunFinished` → `done`/`error`,
  D-14/D-15) and `terminal_payload` (degraded path, D-16), plus their enumeration tests.
- `crates/paladin-core/src/platform/container/run.rs` — `RunStatus::try_transition` (unchanged),
  `Run` (`#[non_exhaustive]`; gains `halt_reason`), `RunEventKind`, `RunStreamEventKind`.
- `crates/paladin-core/src/platform/container/trace.rs` — `RunFinishStatus` (gains `Cancelled` and
  the halt reason), `TraceEvent::RunFinished`, `AllowanceWarning` (41 D-18 precedent).
- `crates/paladin-core/src/platform/container/execution_result.rs` — `StopReason`, `is_successful`
  / `is_limit` (D-12).
- `src/application/services/paladin/middleware/limits.rs` — `TokenBudget::after_model`,
  `TOKEN_BUDGET_NOTICE`, the scratch-not-struct rule; `src/config/agent_runtime.rs` —
  `TokenBudgetConfig { enabled, max_tokens }` (D-11's operator figure).
- `src/application/services/paladin/paladin_execution_service.rs` — `with_middleware`, the
  `cumulative_tokens = usage.total_tokens` update, `execute_scoped`, `settle_agent_loop_call`,
  `AgentLoopSettlement`; `crates/paladin-core/src/platform/container/run_scope.rs` — `RunScope`
  (`run_id`, `ledger_scope`, `allowance_warnings`; the recommended carrier for the derived budget).
- `src/application/services/treasurer/mod.rs` and `policy.rs` — `Treasurer` (`admit`/`confirm`/
  `abandon`, `with_notices`, `with_operator_webhook`), `AllowancePolicy::ceilings_for`, `Ceiling`;
  `crates/paladin-core/src/platform/container/allowance.rs` — `AllowanceRefusal`,
  `AllowanceWarning`, `AllowanceNotice`, `crosses_warn_threshold`, `NoticeRecord`, `Admission`.
- `crates/paladin-ports/src/input/allowance_admission_port.rs` — `AllowanceAdmissionPort`
  (gains the D-09/D-10 derivation result on `Admission`, or a sibling method);
  `crates/paladin-ports/src/output/treasury_ledger_port.rs` — `balance`, `store_now`, the error
  variants D-03 maps; `crates/paladin-ports/src/output/treasury_notice_port.rs` — `record`,
  `notices_for_run`, `discard` (D-17/D-18 reuse).
- `crates/paladin-core/src/platform/container/cost.rs` — `PriceRow` accessors (D-09's axes),
  `PriceTable::row`, `Cost`; `crates/paladin-llm/src/pricing.rs` — `with_pricing` (unchanged).
- `src/infrastructure/web/agent_host.rs` — `build_agent` / `build_agent_with_llm` (where the
  Treasurer-aware `TokenBudget` is installed per agent); `src/infrastructure/web/facade_provisioner.rs`
  — `paladin_port_from_settings_with_ledger`, the shared `EngineExecutionPort`;
  `src/infrastructure/web/run_api_wiring.rs` — `build_run_api` (where the `Treasurer` is built and
  handed to the submission service, the agent state and now the worker pool).
- `crates/paladin-web/src/agent_controller.rs` — `admit_principal`, the `execute` /
  `execute/stream` / `jobs` handlers, `stop_reason_label`, the stream's `done` event;
  `crates/paladin-web/src/run_controller.rs` — `get_run`, `list_runs`, `load_visible_run`,
  `map_submission_error`; `crates/paladin-web/src/thread_controller.rs` — `fork_thread` (D-07);
  `crates/paladin-web/src/error.rs` — the `429` helper D-10 reuses.
- `src/application/services/run/submission.rs` — `fork` (`ensure_thread_visible` →
  `authorize_invocation` → `admit_and_persist`) and `admit_and_persist` (the admission that D-09
  extends with the derived budget).
- `src/application/services/run/webhook/mod.rs` — `WebhookPayload` and the
  `allowance_warning_payload_*` tests (D-18/D-19 key-set discipline);
  `src/application/services/run/webhook/service.rs` — the durable delivery D-18 enqueues to.
- `src/infrastructure/telemetry/herald_sink.rs` — `HeraldTraceSink` (D-19 fold);
  `crates/paladin-herald/src/{markdown,json,table}_herald.rs` — the three renderers.
- `crates/paladin-storage/migrations/{sqlite,postgres}/012_add_treasury_ledger_tenant_index.sql` —
  the latest number and header style for `013`; `crates/paladin-storage/src/run/contract_tests.rs`
  — the clause home for `halt_reason` round-trips; `crates/paladin-storage/src/treasury/contract_tests.rs`.
- `src/application/services/run/http_surface_tests.rs`, `crates/paladin-web/tests/openapi_golden_v0_9.rs`
  — end-to-end and golden-diff precedents (the `done` payload and `GET /runs` schema change the
  OpenAPI document).
- `docs/src/api-reference/platform-api.md`, `docs/src/getting-started/configuration.md`,
  `config.example.yml`, `CHANGELOG.md` `[Unreleased]` — D-21 targets.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The superstep loop already has the exact seam: one check per boundary, `vanguard` persisted on a
  `WaypointStatus::Halted` Waypoint, `RunOutcome::Halted` returned, `resume` continuing from it —
  the Treasurer guard is a second signal at the same `if`, not a new mechanism (ADR-0052).
- `AllowancePolicy::ceilings_for` + `TreasuryLedgerPort::balance` + `store_now` — admission's
  evaluation is the boundary's evaluation (D-01/D-17); factor, do not fork.
- `TreasuryNoticePort::record` with `INSERT … ON CONFLICT DO NOTHING` — D-17/D-18 reuse the same
  store-enforced once-per-window dedup with a new rung.
- `TokenBudget::after_model` — code-complete, unit-tested, never wired in production; D-11/D-12
  give it its first production caller with one new comparison and one new stop reason.
- `RunScope` — the per-call carrier Phase 39/40/41 already extended additively (`run_id`,
  `ledger_scope`, `allowance_warnings`); the natural home for the derived budget.
- `AllowanceRefusal` — the one refusal shape (41 D-14) that becomes the `halt_reason` figures, the
  `done` payload object, the webhook key and the agent stop reason's payload.
- `map_outcome` / `map_trace_event` / `terminal_payload` — three small pure functions with
  enumeration tests; every status-mapping change in this phase lands in them.
- `fork_thread` → `RunSubmissionService::fork` → `admit_and_persist` — resume needs no new gate.
- `HeraldTraceSink` + `ExecutionMetadata.metadata` + the three heralds' one-line pattern (41 D-18).

### Established Patterns
- **Policy in the facade, mechanism in the engine** (`CancellationProbe` D-15, Treasurer 41 D-06):
  `paladin-battalion` consults a port, never a ledger or an allowance.
- **Checked at a boundary, never mid-step** (ENG-FR-23): cancellation and now spend are observed
  only at the top of the loop, so a halt is always a consistent restart point.
- **Fail closed where a ceiling applies, inert where none does** (41 D-10) — D-03 keeps the rule.
- **Additive `#[serde(default)]` fields and `#[non_exhaustive]` enums** keep Rust-side changes
  non-breaking; the register still records them (D-21).
- **Store-enforced idempotency over application-level checks** (39 D-06, 41 D-16).
- **Trace sinks are lossy observations; money and status are not** — the run row (`status`,
  `halt_reason`) is the truth; `done`, webhook and herald observe it.
- **One shared function per rule** (`load_visible_run`, `LedgerScope::from_attribution`,
  `admit`): the ceiling evaluation gets one home for admission and the boundary.

### Integration Points
- `build_run_api` → `Treasurer` (+ new guard-port impl) → `RunWorkerPool` (per-run engine build:
  `with_cancellation_probe`, `with_treasury_ledger`, **+ the guard**) → superstep boundary →
  `RunOutcome::Halted { cause }` → `run_finish_status` → `RunFinished { Cancelled | Halted {reason} }`
  → `RunEventBusSink` → `map_trace_event` → `done` (D-14/D-15/D-16).
- `map_outcome(cause)` → `update_status(Halted)` + `record_outcome(halt_reason)` →
  `enqueue_webhook_delivery(Halted + halt_reason)` → operator `allowance_halted` (D-18).
- `Treasurer::admit` → `Admission` (+ derived budget) → `RunScope` → `TokenBudget::after_model`
  (min of derived and operator figures) → new `StopReason` → `run_agent` → `Halted` + `halt_reason`;
  HTTP agent handlers → `200` + `stop_reason: "allowance_halted"` + `halt_reason`.
- `GET /runs/{id}` ← `Run.halt_reason`; `POST /threads/{id}/fork` ← halted `final_waypoint_id`
  → `admit_and_persist` → `429` or `202`.
- `TreasurerConfig` / `AllowanceConfig` — no new config keys in this phase (the derived budget has
  no knob; `agent_runtime.token_budget` keeps its meaning).

</code_context>

<deferred>
## Deferred Ideas

- **Per-superstep reservation hold** (`reserve` at the boundary sized by the previous superstep's
  charge or a configured estimate; closes the 41 D-05 over-admission race) — explicitly declined
  for v0.11.0 (D-01); recorded in ADR-0057 with the estimate rule it would need.
- **Automatic resubmission of Treasurer-halted threads** when the window resets or the allowance is
  raised — a scheduler-shaped capability with an attribution question; its own phase.
- **A `requeued` wire status** for drained runs — rejected in favour of D-15 (no terminal event).
- **Re-deriving the agent budget per model call** from the observed prompt/completion blend — D-09
  fixes the figure at admission; revisit only with evidence that prompt-heavy agents stop too early.
- **`POST /runs/{id}/resume` as a first-class route** re-enqueuing the same run id (a `Halted →
  Queued` edge) — rejected for v0.11.0 (D-07); would be an ADR-level state-machine change.
- **Warn-threshold ladder**, **caller-facing allowance warning webhook**, **`paladin-cli treasury
  allowance` view** — carried from Phase 41's deferred list, unchanged.
- **Thread-route read scoping** (WINDOWS.md row 58) — still the Phase 40 deferred item; D-07 leans
  on `RunSubmissionService::fork`'s own `ensure_thread_visible`, not on the route.
- **Rate pacing** — Phase 43 (PACE-01..05). **Treasurer mdBook page** — Phase 46 (CURR-23).

</deferred>

---

*Phase: 42-mid-run-halt-sse-terminal-status*
*Context gathered: 2026-10-06*
