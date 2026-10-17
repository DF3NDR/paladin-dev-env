# Phase 42: Mid-Run Halt & SSE Terminal Status - Research

**Researched:** 2026-10-06
**Domain:** In-repo Rust (hexagonal): `WarEngine` superstep boundary, agent-loop `TokenBudget` middleware, run state machine, SSE terminal events, SQL migrations on two backends
**Confidence:** HIGH on every statement about existing code (each was read in this session); MEDIUM on the design recommendations marked "Recommended" (they resolve gaps CONTEXT leaves open and need the planner's or operator's confirmation, see Assumptions Log and Open Questions)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (locked by ADR-0052/0053/0056 and Phases 38-41 — cited, not re-asked)

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

#### Superstep boundary check

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

#### Halt and resume contract

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

#### Agent-loop budget derivation (ALLOW-05)

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

#### SSE terminal payload (PLAT-09)

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

#### Mid-run warnings and halt notices

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

#### Bookkeeping, ADR and docs

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

### Deferred Ideas (OUT OF SCOPE)

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
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ALLOW-03 | A run in flight whose next draw would overspend halts cleanly (typed Treasurer error, status `Halted`, last checkpoint kept) on the engine path and the agent-loop path; a halted run resumes from its last checkpoint once the allowance is replenished or the window resets | Engine: `SpendGuard` port at the top-of-loop check (`superstep.rs:2217-2244`) writing the same `Halted` Waypoint; typed cause through `RunOutcome::Halted` (Gap G1/G4); worker `map_outcome` + `halt_reason` column (migration 013); resume = `POST /threads/{id}/fork` (`submission.rs:fork`) which re-runs admission; agent loop: derived budget + `StopReason` variant + `run_agent` mapping (Gaps G2/G8); resume numbering quirk (Gap G13) |
| ALLOW-05 | The Treasurer derives the per-run `TokenBudget` from the remaining allowance and works alongside `TokenBudget`, `ModelCallLimit`, `ToolCallLimit` and the Commissary without replacing any; a guard keeps `Treasurer` a framework-only word | Derivation function (Code Example 3) over `PriceRow` axes (Gap G16); `RunScope` carrier + Treasurer-aware `TokenBudget` reading the `ModelCallContext` (Gap G12); vocabulary guard test design (Assumption A1) |
| PLAT-09 | The SSE `done` event matches the persisted status — `Cancelled` for a caller-cancelled run, `Halted` with the Treasurer reason for a spend halt | `map_trace_event` (`events.rs:206-231`), `terminal_payload` (`events.rs:453-484`), third path `replay_stream` (Gap G10); local-token cancel attribution (Gap G1); drain suppression (D-15); wire-shape collision (Gap G6) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Directives extracted from `/home/user/paladin-dev-env/CLAUDE.md` and the three imported `.github/` instruction files. Treat as locked, same authority as CONTEXT decisions.

- **TDD (Red-Green-Refactor):** failing test first. **Coverage floor 82 % workspace line coverage** (ADR-0006), enforced by `cargo llvm-cov --fail-under-lines` in CI. All public APIs need doc tests.
- **Hexagonal dependency direction:** core -> nothing; application/ports -> core; infrastructure adapters -> core + ports. `paladin-battalion` and `paladin-storage` never learn allowances.
- **Ubiquitous language:** Medieval Military terms (Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest, Commissary, Treasurer ...) in code, docs and comments.
- **Before committing a parent task:** `cargo test` -> `cargo fmt --check` -> `cargo clippy` -> `make api-surface` (`make api-surface-update` + CHANGELOG entry for an intentional surface change); conventional-commit message; stop after each major task and wait for go-ahead.
- **Security:** `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings`; **manual credential-handling review** is the primary control (CodeQL is advisory-only, Snyk removed; never add a Snyk step). Redact response bodies **before** truncation; no log line interpolates an API key; clients sending credential headers do not follow redirects; webhook URLs pass the SSRF guard at write and send time; `X-Paladin-Signature` is computed over the exact stored bytes.
- **Rust conventions:** no `unwrap()`/`expect()`/`panic!` in library code; return `Result`; layer-specific `thiserror` enums converted at boundaries; port traits are `Send + Sync` (`#[async_trait]`); `Node<T>` pattern for entities; builders for complex construction; all public items have rustdoc; all public types implement `Debug`; prefer borrowing; keep iterators lazy.
- **Slash-command/MCP:** none apply to this phase's output.
- No `.claude/skills/` or `.agents/skills/` directory exists in the repo (checked), so no project skill patterns apply.

## Summary

Phase 42 adds the enforcement half of the Treasurer to a codebase whose seams are already in place: the engine's top-of-loop boundary check that writes a `WaypointStatus::Halted` Waypoint (`superstep.rs:2217-2244`), the `TokenBudget::after_model` cutoff (`limits.rs:146-171`), the per-run `RunScope` carrier, the Phase 41 `Treasurer` (`admit` evaluates up to four ceilings through `AllowancePolicy::ceilings_for` + `TreasuryLedgerPort::balance` + `store_now`), the worker's pure `map_outcome`, and the SSE mapping functions `map_trace_event` / `terminal_payload`. The CONTEXT decisions are sound in outline and almost entirely implementable as written. This research found **seventeen places where the decisions as worded collide with the code as it is**; they are consolidated in "CONTEXT Gaps the Planner Must Resolve" below, each with a prescriptive recommendation. The five that change the plan's shape are: (G1) a caller cancel on the instance that runs the run fires only the in-process child token, which the engine cannot tell apart from a shutdown drain, so D-05's "read the cause instead of re-querying `is_cancel_requested`" would regress PLAT-09 unless the per-run probe also answers for the local fast path; (G2) the streamed route `execute/stream` is a single model call that **never invokes `after_model`** (`paladin_execution_service.rs:3494-3498`), so D-13's derived budget cannot halt it; (G3) `build_run_api` builds and `spawn`s the worker pool **before** the Treasurer exists (`run_api_wiring.rs:754-843`), so the guard needs the Treasurer built first; (G4) `RunFinishStatus` is a `Copy` unit enum persisted in `run_traces`, so a `Halted { reason }` payload variant would break stored rows and every `*status == X` comparison, and the reason should ride a separate optional `RunFinished.halt_reason` field instead (D-05 permits "or equivalent"); (G5) `treasury_notices` has a `CHECK (limit_kind IN ('window','lifetime'))` and a unique index, so D-18's halt notice needs a new `notice_kind` column and a rebuilt index in migration 013, and the operator-delivery branch in `WebhookDeliveryService` is gated on `event == AllowanceWarning`.

There is no new external dependency. Every capability is a composition of in-tree primitives plus `std`. The phase touches eight crates and both SQL backends, so the dominant planning risks are breadth, register hygiene (MIGRATION.md, allowlist, api-surface, the frozen v0.9 OpenAPI golden gate) and the SSE/status ordering races, not library choice. Mid-run halt has no prior art among the surveyed comparable systems (none have durable multi-step runs), so the design is validated by this repo's own contract/e2e tests rather than by an external reference; the streaming "settle on nonexistent usage" pitfall is neutralised exactly as CONTEXT D-00j says, by reusing `TokenBudget`'s post-response cutoff.

**Primary recommendation:** Open with ADR-0057 (plan 42-01) recording D-01..D-04, D-05..D-08, D-09..D-12, D-15, D-18 **and the resolutions of Gaps G1-G12 below**; then build bottom-up in this order: core types (`HaltReason`, `StopReason` variant, `RunFinished.halt_reason`, `Run.halt_reason`, `RunScope` budget carrier) -> ports (`SpendGuard`, `AllowanceAdmissionPort` model-aware derivation) -> storage migration 013 + three run adapters + notice rung -> engine boundary check + `RunOutcome::Halted.cause` -> Treasurer evaluation factoring + guard impl + derivation -> worker wiring (reordered `build_run_api`) + `map_outcome` -> SSE/webhook/herald legs -> agent-loop budget + HTTP handlers -> docs/registers/guard test. Reuse `TokenBudget`'s cutoff and the Phase 41 evaluation; invent nothing the code already has.

## CONTEXT Gaps the Planner Must Resolve

Each row was found by reading the code named; each carries a recommendation. Rows G1-G12 should be recorded in ADR-0057 or in the plan that owns them. "Owner" is the plan area that must carry the fix.

| # | Gap (what CONTEXT says vs what the code does) | Evidence | Recommendation | Owner |
|---|----|----|----|----|
| G1 | **Local-token caller cancel is indistinguishable from a drain at the engine.** D-05 makes `map_outcome` read the cause "instead of re-querying `is_cancel_requested`" and says the bare in-process token halt keeps the plain cause. But `RunSubmissionService::cancel` writes the durable flag then calls `LocalRunTokens::cancel_if_local`, which cancels the per-run **child** token; the engine then reports a *token* halt. The probe is debounced (`min_probe_interval_ms` default 1000) so it may still answer `false`. The shutdown token is that child's **parent**, so a drain also shows as a token halt. Today the worker resolves this with `is_cancel_requested` + `shutting_down` (`worker.rs:1236-1237`, `486-513`) and the engine's `RunFinished` is wrong (`done: halted`) for the common same-instance cancel (`worker.rs:1244-1255`). | `cancel.rs:109-117,121-134`; `superstep.rs:2217-2224`; `worker.rs:1062-1075,1236-1260`; `submission.rs:cancel` (`request_cancel` then `cancel_if_local`) | (a) Keep re-querying `is_cancel_requested` + `shutting_down` for the bare **Token** cause (defence in depth; never regress to reading only the cause). (b) Make the engine report `CancelRequested` for the same-instance cancel by giving the per-run engine a probe that ORs the debounced DB probe with the local signal: `PerRunCancelProbe { db, local: child_token, shutdown: coordinator_token }` answering `db.is_cancelled(t) \|\| (local.is_cancelled() && !shutdown.is_cancelled())`. Built in the worker's per-run engine build (`worker.rs:1062-1075`) where the child token already exists. (c) For D-15 (drain emits no `done`): the engine still emits `RunFinished{Halted, no reason}` for a Token cause, so suppress at the sink: give `RunEventBusSink` the coordinator's shutdown token and have it drop a `RunFinished{Halted}` with no `halt_reason` while that token is cancelled. | worker + events |
| G2 | **`execute/stream` cannot halt on a derived budget.** D-13 lists the HTTP `execute` / `execute/stream` handlers. The streamed path is one provider call with no reasoning loop; `after_model` and `around_tool` "are never invoked here", only `before_model` once. | `paladin_execution_service.rs:3494-3498` (comment), `3517-3518` | Cover `execute` (buffered, loop) and the **buffered fallback** of `execute/stream` and `jobs` with the derived budget (all go through `execute_scoped` -> `execute_internal`). For a real streamed call, admission is the only gate; record one `WINDOWS.md` row ("a single streamed call is not cut mid-flight; overshoot is that one call"). Optional, cheap: if the terminal chunk's usage crossed the derived figure, add the `halt_reason` object to the stream's final `done` (informational, no behavioural halt). Confirm with the operator (Open Question 2). | agent-loop + web |
| G3 | **Pool is built before the Treasurer.** The guard must be attached per run, but `RunWorkerPool` is `Arc`ed and `spawn`ed at `run_api_wiring.rs:754-796` before the Treasurer is built at `:799-832`. | `run_api_wiring.rs:754-843` | Reorder `build_run_api`: build `treasury_notices`, then the `Treasurer` (needs `ledger`, `notices`, `webhook_repository`, all available), then the pool with `.with_treasurer(Arc<Treasurer>)`. Hold `Arc<Treasurer>` (concrete) and coerce to `Arc<dyn AllowanceAdmissionPort>` for the submission service and agent state. | wiring |
| G4 | **`RunFinishStatus` cannot take a payload without breaking stored data.** It is `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]` + `rename_all = "snake_case"`, **not** `#[non_exhaustive]`, persisted as the string `"halted"` in `run_traces`, compared by value in `herald.rs:757` and matched exhaustively in `events.rs:208-211` and `paladin-eval/src/assertion.rs:423-430`. A `Halted { reason }` struct variant changes the JSON of every stored halted row and drops `Copy`. | `trace.rs:159-170`; `herald.rs:757`; `assertion.rs:423` | Keep `RunFinishStatus` a unit enum; **add `Cancelled`** (X-03 row; unit variant on a non-`non_exhaustive` enum is a compile break for exhaustive downstream matches, so mark the enum `#[non_exhaustive]` in the same change and record it). Carry the reason as a new field on `TraceEvent::RunFinished`: `#[serde(default, skip_serializing_if = "Option::is_none")] halt_reason: Option<HaltReason>`, exactly the `cost` precedent (MIGRATION.md §9.2 rows 208-210). Old stored rows read back with `None`. `paladin-eval`'s `run_status_matches` is a `matches!` over pairs, so it stays correct; add `RunStatusValue::Cancelled` only if scenario files need it. | core |
| G5 | **Notices table cannot hold a halt rung as D-18 words it.** `treasury_notices.limit_kind` has `CHECK (limit_kind IN ('window','lifetime'))`; the unique index `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)` would collide a halt notice with the warn notice for the same ceiling and window. Adapters' `NOTICE_INSERT ... ON CONFLICT (...)` column list must textually match the index. `notices_for_run` feeds the worker's first-dispatch warning emission and must not return halt rows. `WebhookDeliveryService` chooses the operator signing key with `delivery.event == RunEventKind::AllowanceWarning`; the webhook adapters parse the event string with an explicit match. | `011_create_treasury_notices.sql`; `service.rs:240`; `webhook/sqlite.rs:62-74`, `webhook/postgres.rs:61`; `treasurer/mod.rs:220` | Migration 013 (both backends) adds `notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (notice_kind IN ('warning','halt'))` to `treasury_notices`, drops and recreates `idx_treasury_notices_once` with `notice_kind` appended, and the adapters' `NOTICE_INSERT` arbiter list and the `notice_arbiter_matches_the_migration` tests move with it. `notices_for_run` filters `notice_kind = 'warning'`. Add `RunEventKind::AllowanceHalted` (`"allowance_halted"`), extend both webhook adapters' event parse and `service.rs:240` to `matches!(event, AllowanceWarning \| AllowanceHalted)`. Keep the twelve-key payload (reuse `AllowanceWarningPayload` with `event` set; `warn_at` = the ceiling's configured threshold) and do **not** add a thirteenth key. | storage + treasurer + webhook |
| G6 | **`kind` means two things.** D-06/D-14/D-16 put a discriminator named `kind` (`ledger_unavailable`) in `halt_reason`, and also say its field set equals the 429 `details` object. That object already has `kind` = the **limit kind** (`"window"`/`"lifetime"`). | `error.rs:184-191` | Name the discriminator `reason` (`"allowance_exhausted"` \| `"ledger_unavailable"`); keep `kind` = limit kind so the figures stay byte-compatible with the 429 body. Record in ADR-0057. Example: `{"reason":"allowance_exhausted","scope":"api_key","kind":"window","balance":"25.0000 USD","ceiling":"25.0000 USD","window_start":"...","window_end":"..."}` and `{"reason":"ledger_unavailable"}`. | core + web |
| G7 | **Callers cannot find the fork point.** D-07's recipe says fork from the halted run's `final_waypoint_id`, but `RunResponse` has no waypoint field (`run_controller.rs:397-435`; grep finds `waypoint_id` only in thread-route and SSE code). The live `done` carries `waypoint_id: null` (WINDOWS.md row 33). | `run_controller.rs:397-455` | Add an additive `final_waypoint_id: Option<String>` to `RunResponse` beside `halt_reason` (new-in-0.10 type, N/A row, same OpenAPI regeneration). The recipe then works from `GET /runs/{id}` alone; the alternative (`GET /threads/{id}/history`, latest `halted` summary) stays documented. | web |
| G8 | **Admission does not know the model, and a derived budget cannot ride admission to a worker.** `AllowanceAdmissionPort::admit(subject, run_id)` has no model; D-09/D-10 need the model's price row. A worker-dispatched agent run executes later, possibly on another instance, after queue delay, so a figure derived at `POST /runs` time is stale and there is no column to carry it. | `allowance_admission_port.rs`; `submission.rs:admit_and_persist`; `run_agent` | Add **one shared evaluation** in the Treasurer (`evaluate(subject) -> Evaluation { evaluated_at, rows: Vec<(Ceiling, balance, window)> }`) used by `admit`, by the boundary guard and by a new `derive_budget(subject, model) -> Result<Option<DerivedBudget>, BudgetRefusal>`. Admission calls `derive_budget` only to enforce D-10 (unpriced -> refuse) and D-09's "derived budget of zero -> refuse" for agent-kind assistants and the HTTP agent routes (add an optional `model: Option<&str>` input through a new defaulted trait method so existing implementors do not break). The **worker re-derives at dispatch** inside `run_agent` (fresh, no persisted figure). If the dispatch-time derivation is zero (allowance spent while queued), do not call the LLM: record `Halted` with the refusal figures. | treasurer + ports + worker |
| G9 | **Frozen v0.9 OpenAPI golden gate.** `ExecuteResponse` is a v0.9 schema. Adding `halt_reason` (and the new `allowance_halted` string value) changes it; `openapi_golden_v0_9.rs` strips only fields it names (`token_count`, `usage`, `cost`) and the 429 entries. | `openapi_golden_v0_9.rs:306-326` | Extend `strip_known_v0_10_execute_response_divergence` to also strip `halt_reason` and any new component schema, add the "Phase 42 exception" doc block (same pattern as the Phase 31/39/41 blocks), regenerate `crates/paladin-web/openapi.json` via `make openapi`, and cover the field with the existing crate-wide `constructible_struct_adds_field` suppression/allowlist row (MIGRATION.md row 207 precedent). | web |
| G10 | **A third `done` path exists.** `replay_stream` (persisted-trace replay) also maps `RunFinished` records through `map_trace_event` and uses `terminal_payload` as a fallback. Historical `run_traces` rows for caller-cancelled runs say `halted`. | `events.rs:627-720` | In `replay_stream`, when a `RunFinished` record is mapped, fetch the run row and, if terminal, emit `terminal_payload(&run)` instead (the row is the truth; trace sinks are lossy observations). This also makes the replay path agree with the live and degraded paths for pre-phase traces. | events |
| G11 | **A spend halt is not sticky, a cancel is.** A child (`NodeSpec::Battalion`) run inherits the guard like the probe (`ChildEngineResources`); on a child Halt the parent contributes an empty delta and relies on **its own** next boundary re-answering the same. For a cancel token that is guaranteed; for a balance it is not (a window can roll between the two reads, letting the parent continue with a silently empty child delta). | `superstep.rs:1511-1523` | The per-run guard instance memoises its first `Halt` (an `Arc<OnceLock<..>>` shared by the child clones) and returns it on every later call. This does not conflict with D-02 ("no caching" refers to `Continue` answers). | guard impl |
| G12 | **Operator `TokenBudget` on the shared service re-creates the hazard ADR-0052 rejected.** The worker's `Runnable::Agent` path and every engine node share ONE boot-time `PaladinExecutionService` (`facade_provisioner.rs:174-199`). Installing an `enabled` operator `TokenBudget` there would also cap engine nodes and return a *successful* partial result into the Battlefield. | ADR-0052 "Considered Options"; `facade_provisioner.rs:195-199` | Construct the shared service's `TokenBudget` in **Treasurer-only mode** (operator config forced `enabled: false`); it acts only when the per-call `RunScope` carries a derived figure, which only the worker's agent-kind dispatch sets. Per-agent HTTP services (`build_agent_with_llm`) get the operator config plus the scope figure (D-11 tightest-wins). Document in configuration.md that `agent_runtime.token_budget` now takes effect for the HTTP agent routes (it was inert before: `build_chain` has zero production callers). | provisioner + agent_host |
| G13 | **Resume numbering quirk (cosmetic).** The top-of-loop `Halted` Waypoint is written with `superstep = superstep_number` (the superstep about to run), but `resume`/`fork` continue at `latest.superstep + 1`, so one superstep number is skipped (a first-boundary halt resumes at 2). Existing cancel-halt behaviour, not new. | `superstep.rs:2226-2244`; `mod.rs:2397-2406`, `2833-2847` | Do not "fix" it in this phase (it would move existing contract tests). Tests must not assert absolute superstep numbers across a halt/fork; they assert vanguard, Battlefield and "nodes of completed supersteps are not re-dispatched". Note it in ADR-0057. | tests |
| G14 | **`done` can precede the row.** The engine emits `RunFinished` (live `done`) before the worker writes the status; the worker then does `update_status` then `record_outcome` as two statements, so a degraded poller can read `Halted` with `halt_reason = NULL` and build a `done` without the reason. | `worker.rs:1258-1272`; sqlite `record_outcome` has no status guard (`sqlite.rs:459-492`) | For halting outcomes call `record_outcome` **before** `update_status` (it has no status guard on any adapter, so this is legal), so the row is complete the instant the status flips. E2E tests that compare `done` with `GET /runs/{id}` must poll until the status is terminal. | worker |
| G15 | **Unpriced engine nodes are unmetered.** D-10 refuses an unpriced *agent* model; a graph Paladin node on an unpriced model settles nothing (`cost: None`), so the boundary check never sees it. | `settlement.rs` (`SuperstepCharge::Nothing`); Phase 38 D-08 | Out of scope to fix; record one `WINDOWS.md` row ("an allowance cannot bound spend on unpriced engine nodes; operators must price every model a metered principal can reach"). | docs |
| G16 | **D-09 arithmetic details.** (a) `PriceRow` has a fifth axis, `cache_read`, defaulting to `prompt` but configurable higher; D-09 omits it. (b) A max price of `0` makes `floor(x / 0)` undefined. (c) `TokenBudget` compares `cumulative_tokens > max_tokens` (strict) over `u32`, so a run can finish at `max_tokens` plus one response's tokens; "can never overspend" is really "overspend bounded by one response". | `cost.rs:236-342, 375-403`; `limits.rs:156`; `context.rs:264` | Take the max over **all five** axes using the same defaults `cost_of_call` uses; a max price of 0 yields `None` (no Treasurer budget; the model is free); compute `i128` `remaining_nanos * 1_000_000 / price`, then `u32::try_from(..).unwrap_or(u32::MAX)`; word the guarantee as "bounded by one response". Sub-counts are contained in `prompt`/`completion` (`token_usage.rs:17-26`), so total tokens at the max price is a valid upper bound. | treasurer |
| G17 | **`StopReason` carries `Eq`.** `StopReason` derives `Eq`; `AllowanceRefusal` derives only `PartialEq` (every field is `Eq`: `Cost` is `Eq`, `DateTime` is `Eq`). | `execution_result.rs:152`; `allowance.rs:113`; `cost.rs:139` | Add `Eq` to `AllowanceRefusal` (additive) and give the variant a payload of that type (or of the new `HaltReason` if it derives `Eq`). Variant on an already `#[non_exhaustive]` enum is a non-break (no allowlist row, `N`). Extend `stop_reason_label` (it has a `_ => "unknown"` arm that would silently label the new variant `unknown`) and its `stop_reason_labels_are_stable` test. | core + web |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Allowance policy (ceilings, windows, thresholds, derive budget) | Facade application service (`Treasurer`, `src/application/services/treasurer/`) | — | D-00c/41 D-06: the only policy home; `paladin-battalion` and `paladin-storage` never learn allowances |
| Boundary mechanism (consult a port, write `Halted` Waypoint) | Engine (`paladin-battalion` `superstep.rs`) | Ports (`paladin-ports` output port) | Engine owns the checkpoint; it consults a port exactly like `CancellationProbe` (D-04) |
| Spend facts (balance, store clock, notices) | Storage adapters (`paladin-storage` treasury, notices) | Ports | Derive-on-read `balance` (ADR-0053); store clock is the only clock (ALLOW-01) |
| Halt cause / halt reason types | Core (`paladin-core` beside `allowance.rs`) | — | Needed by ports, engine, storage, web and facade; no layer may own it alone |
| Run status + `halt_reason` persistence | Storage (three `RunRepositoryPort` adapters + migration 013) | Core `Run` | Run row is the truth; `done`, webhook, herald only observe it |
| Terminal status mapping (`map_outcome`) | Facade worker (`worker.rs`) | Core state machine | Pure function with enumeration tests; owns the Cancelled-vs-Halted-vs-drain decision (G1) |
| SSE `done` (live / degraded / replay) | Facade events service (`events.rs`) | Core `HaltReason` wire builder | One shared JSON builder keeps all three paths byte-identical (D-14/D-16) |
| HTTP surface (`halt_reason` on runs, `allowance_halted` stop reason, DTOs, OpenAPI) | `paladin-web` | Facade handlers/state | Edge formats money once with `format_cost` (D-00h) |
| Agent-loop cutoff | Facade middleware (`TokenBudget`) | Core `RunScope` as carrier | Reuse the one existing mechanism (D-00a/D-11) |
| Operator notice / halt webhook | Facade `Treasurer` + durable `WebhookDeliveryService` | Storage (notice dedup) | Store-deduped once-per-window; never a second HTTP client |
| Resume | Existing `POST /threads/{id}/fork` -> `RunSubmissionService::fork` | `WarEngine::fork` | Re-runs admission; continues from the Halted Waypoint's vanguard (D-07) |

## Standard Stack

### Core

No new external crates. The phase composes existing in-tree crates and the workspace's existing dependencies.

| Library / crate | Version | Purpose | Why Standard |
|-----------------|---------|---------|--------------|
| `paladin-ai-core` (`paladin-core` dir) | workspace (0.10.1 -> 0.11.0) | `HaltReason`, `StopReason` variant, `RunFinished.halt_reason`, `Run.halt_reason`, `RunScope` budget carrier | Types every layer needs; precedent: `AllowanceRefusal` lives in `allowance.rs` [VERIFIED: codebase] |
| `paladin-ports` | workspace | `SpendGuard` output port; model-aware derivation on the admission port | Mirrors `CancellationProbe` placement [VERIFIED: codebase `cancellation_probe.rs`] |
| `paladin-battalion` | workspace | Boundary check, `RunOutcome::Halted.cause`, `with_spend_guard` | Owns `superstep.rs` [VERIFIED: codebase] |
| `paladin-storage` | workspace | Migration `013`, three run adapters, notice rung | sqlx 0.8 with sqlite + postgres [VERIFIED: `Cargo.toml:44`] |
| `paladin-ai` (facade) | workspace | `Treasurer` (guard impl, derivation), worker, events, webhook, herald sink | Policy home (41 D-06) [VERIFIED: codebase] |
| `paladin-web` | workspace | `RunResponse.halt_reason`/`final_waypoint_id`, `ExecuteResponse.halt_reason`, `stop_reason_label`, OpenAPI | `utoipa` DTO edge [VERIFIED: codebase] |
| `tokio` / `tokio-util` | 1 / 0.7 | async, `CancellationToken` for the per-run probe wrapper | Already used by `cancel.rs` [VERIFIED: `Cargo.toml:26,155`] |
| `async-trait` | 0.1.88 | `SpendGuard` trait | House port pattern [VERIFIED: `Cargo.toml:27`] |
| `serde` / `serde_json` | 1 / 1.0 | `HaltReason` wire + column encoding | Existing [VERIFIED: `Cargo.toml:21-22`] |
| `thiserror` | 2 | New refusal/guard error variants | House error pattern [VERIFIED: `Cargo.toml:25`] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::fs` recursion | std | Vocabulary guard test scanner | Do NOT add `walkdir`/`ignore`; keep the dependency graph unchanged |
| `chrono` | 0.4.38 | Window arithmetic via the existing `window_for` | Reuse, do not re-derive [VERIFIED: `treasurer/window.rs`] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `RunFinished.halt_reason` field | `RunFinishStatus::Halted { reason }` | Payload variant drops `Copy`, rewrites stored `run_traces` rows' JSON and breaks `*status ==` comparisons (G4) |
| Per-run probe wrapper (G1) | Worker publishes `done` itself after `map_outcome` | Reverses D-14's "one `map_trace_event` mapping" and leaves persisted traces wrong |
| Derive at dispatch (G8) | Persist the derived figure on the run row | Extra column outside D-06, stale after queue delay |
| New `notice_kind` column (G5) | New `limit_kind` value | CHECK constraint + unique index make it a table rebuild on SQLite anyway |

**Installation:** none. `cargo build` only.

**Version verification:** not applicable (no new package). Toolchain: `cargo 1.97.1`, `rustc 1.97.1`, pinned nightly `nightly-2026-09-20` present for `make api-surface` [VERIFIED: local `cargo --version`, `rustup toolchain list`].

## Package Legitimacy Audit

No external package is installed or recommended by this phase, so the Package Legitimacy Gate (`gsd-tools query package-legitimacy check`) has nothing to evaluate.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| — | — | — | — | — | n/a | No new dependency; vocabulary guard uses `std::fs` |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                       ENGINE PATH (graph / workflow runs)
 POST /runs ──> RunSubmissionService.submit ──> Treasurer.admit ──(429 allowance_exhausted)
      │                                              │ ok
      │                                              v
      │                                   run row (Queued) + queue
      v
 RunWorkerPool.run_once ── builds per-run WarEngine ──┬─ with_cancellation_probe(PerRunCancelProbe)   [G1]
                                                      ├─ with_treasury_ledger(ledger, SettlementContext)
                                                      └─ with_spend_guard(TreasurerSpendGuard{subject,run_id})   [NEW, D-04]
                                  │
                                  v
   superstep loop ── TOP OF LOOP (each boundary) ─────────────────────────────────────────────┐
     1 token_cancelled?  2 probe.is_cancelled?   ──yes──> Halted Waypoint, cause=Cancel/Token │
     3 guard.check()  ──Halt(spend)/Halt(ledger_unavailable)──> Halted Waypoint, cause=Spend   │
        │ Continue (also may emit AllowanceWarning via notices, D-17)                          │
        v                                                                                     │
     dispatch nodes ──> join ──> SpendHook.settle_boundary (ledger.settle, unchanged) ──> merge ┘
                                  │ Completed / Failed / AwaitingInput / Halted
                                  v
   engine emits RunFinished{status, halt_reason?} ──> RunEventBusSink ──> map_trace_event ──> SSE `done` (live)
                                  │                       (drain: suppressed, D-15)
                                  v
   worker.map_outcome(cause, cancel_requested, shutting_down)
        ├─ Cancelled              ──> record_outcome THEN update_status(Cancelled)         [G14]
        ├─ Halted + halt_reason   ──> record_outcome THEN update_status(Halted) + caller webhook(+halt_reason)
        │                              + operator `allowance_halted` (store-deduped, not for ledger_unavailable)
        └─ drain                  ──> nack, stay Running (no done)
                                  │
   SSE degraded/replay ── terminal_payload(run row: status + halt_reason) ── byte-identical to live   [G10]

   RESUME:  GET /runs/{id} (status halted, halt_reason, final_waypoint_id[G7])
            POST /threads/{id}/fork {from_waypoint_id} ──> RunSubmissionService.fork
               ensure_thread_visible → authorize → Treasurer.admit (429 + Retry-After while exhausted)
               → new run (fork_from) → worker Fork dispatch → WarEngine.fork continues from Halted vanguard

                       AGENT-LOOP PATH (Runnable::Agent runs, HTTP execute / jobs / buffered stream)
 admit(subject, model) ──> derive_budget(remaining, dearest price)  ──refuse: unpriced / zero (D-10, D-09)
        │ (worker: re-derive at dispatch, G8)
        v
 RunScope{ run_id, ledger_scope, treasurer_budget } ──> PaladinExecutionService.execute_scoped
        └─> execute_internal ──> after_model: TokenBudget (min(derived, operator)) ──crossed──> StopReason::AllowanceHalted(figures) | TokenBudget
 worker.run_agent: AllowanceHalted ──> Halted + halt_reason ; else Completed ;  HTTP: 200 stop_reason "allowance_halted" + halt_reason
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
├── allowance.rs        # + HaltReason (reason: AllowanceExhausted(AllowanceRefusal-figures) | LedgerUnavailable), wire_json(), Eq on AllowanceRefusal
├── trace.rs            # RunFinishStatus + Cancelled (#[non_exhaustive]); RunFinished.halt_reason
├── execution_result.rs # StopReason::AllowanceHalted(..); is_limit
├── run.rs              # Run.halt_reason; RunEventKind::AllowanceHalted
└── run_scope.rs        # treasurer_budget: Option<TreasurerBudget> (+ with_ builder)
crates/paladin-ports/src/output/
└── spend_guard.rs      # SpendGuard, SpendDecision (mirror cancellation_probe.rs)
crates/paladin-battalion/src/engine/
├── mod.rs              # RunOutcome::Halted{waypoint, cause}; with_spend_guard; run_finish_status -> (status, halt_reason)
└── superstep.rs        # guard consulted beside the probe; ChildEngineResources carries it
crates/paladin-storage/
├── migrations/{sqlite,postgres}/013_add_run_halt_reason.sql   # runs.halt_reason + treasury_notices.notice_kind + index rebuild
└── src/run/{in_memory,sqlite,postgres}.rs, contract_tests.rs; src/treasury/* (notice arbiter)
src/application/services/
├── treasurer/{mod.rs,evaluate.rs,guard.rs,derive.rs}   # shared evaluation; SpendGuard impl; derivation
├── run/{worker.rs,events.rs,cancel.rs,webhook/*}       # map_outcome, per-run probe, done paths, payloads
└── paladin/{middleware/limits.rs,paladin_execution_service.rs}
tests/treasurer_vocabulary_guard.rs                     # ALLOW-05 guard (auto-discovered root test)
```

### Pattern 1: Port consulted at the boundary, policy in the facade (D-04)

**What:** A new `SpendGuard` output port mirrors `CancellationProbe` in placement but returns a decision that can carry a failed read, instead of a swallowed `bool`.
**When to use:** the one engine attachment point; nothing else in the engine learns about money.
**Example:**
```rust
// Source: codebase pattern crates/paladin-ports/src/output/cancellation_probe.rs (placement),
// designed for this phase. Names are the planner's (CONTEXT Discretion).
#[async_trait]
pub trait SpendGuard: Send + Sync {
    /// Called once per superstep boundary. A failed read is a decision, not a swallowed `false` (D-03).
    async fn check(&self, thread: &ThreadId) -> SpendDecision;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpendDecision {
    Continue,
    Halt(HaltReason), // AllowanceExhausted{figures} | LedgerUnavailable
}
```

### Pattern 2: Boundary check beside the probe, same `Halted` path

**What:** Consult the guard at the same `if` that already handles cancellation, preserving the cancel > spend priority and recording the cause.
**Example:**
```rust
// Source: crates/paladin-battalion/src/engine/superstep.rs:2217-2244 (existing), extended.
let token_cancelled = cancellation.as_ref().is_some_and(CancellationToken::is_cancelled);
let probe_cancelled = match probe { Some(p) => p.is_cancelled(&thread).await, None => false };
let cause = if probe_cancelled {
    Some(HaltCause::CancelRequested)
} else if token_cancelled {
    Some(HaltCause::Token)
} else if let Some(g) = guard {
    match g.check(&thread).await {
        SpendDecision::Continue => None,
        SpendDecision::Halt(reason) => Some(HaltCause::Spend(reason)),
    }
} else { None };
if let Some(cause) = cause {
    let waypoint = build_waypoint(/* unchanged args, WaypointStatus::Halted */);
    persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;
    return Ok(RunOutcome::Halted { waypoint: waypoint.waypoint_id, cause });
}
```

### Pattern 3: One shared evaluation for admission, boundary and derivation (D-17, "one function per rule")

**What:** Factor `admit`'s loop (`treasurer/mod.rs:279-366`) into `evaluate(subject)` returning each ceiling's `(Ceiling, balance, window)` plus one `evaluated_at`; `admit` refuses on the first `balance >= ceiling`; the guard halts on the same predicate and also claims warn notices; `derive_budget` takes `min(ceiling - balance)`. Do not fork the loop.

### Pattern 4: Typed cause -> exactly one `OutcomeAction` (the `map_outcome` table)

```rust
// Source: worker.rs:458-519 (existing shape), extended for the cause. Pure + enumeration-tested.
fn map_outcome(outcome: &RunOutcome, cancel_requested: bool, shutting_down: bool) -> OutcomeAction {
    match outcome {
        // ... Completed / Failed / AwaitingInput unchanged ...
        RunOutcome::Halted { waypoint, cause } => match cause {
            HaltCause::CancelRequested => Cancelled(waypoint),
            HaltCause::Spend(reason) => Halted(waypoint, Some(reason.clone())),
            // Bare token: keep the re-query (G1 a). Drain wins, then cancel flag, else plain Halted.
            HaltCause::Token if shutting_down && !cancel_requested => LeaveRunningAndRequeue,
            HaltCause::Token if cancel_requested => Cancelled(waypoint),
            HaltCause::Token => Halted(waypoint, None),
        },
    }
}
```
Note: today's order is `cancel_requested` first, then `shutting_down` (`worker.rs:487-519`); keep that precedence for the `Token` arm.

### Pattern 5: Terminal payload from one builder

`HaltReason::wire_json(&self) -> serde_json::Value` in `paladin-core` (it already owns `format_cost`). `map_trace_event`, `terminal_payload`, the `GET /runs` DTO, the webhook key and the agent response all call it, so "byte-identical" (D-14) is structural, not tested-into-existence. Persist the *structured* `HaltReason` (serde) in the column; render wire strings only at the edge (D-00h).

### Anti-Patterns to Avoid

- **A second budget mechanism.** Do not add a Treasurer-specific middleware or loop check; extend `TokenBudget` (D-00a/D-11). The ADR-0052 rejected options (wire `build_chain` into the engine; refuse inside the pricing decorator) stay rejected.
- **Policy in the engine.** No allowance, ceiling or currency type enters `paladin-battalion`; the port carries only `HaltReason` (a core type) and a `ThreadId`.
- **Caching `Continue` answers or debouncing the boundary read** (D-02 forbids it; the probe's `min_interval` pattern is the thing to *not* copy).
- **`unwrap()`/`expect()` in the guard or derivation** (clippy `-D warnings`, CLAUDE.md). A failed read is a typed `HaltReason::LedgerUnavailable`.
- **Putting the derived figure on `TokenBudget`'s struct** (per-run scratch on a shared middleware, `limits.rs` D-03). It rides `RunScope` -> `ModelCallContext`.
- **Floating point for money.** `format_cost` is the one f64 use, at the edge; the guard, derivation and notices use integers and `i128`.
- **Emitting `done` for a drain** (D-15) or for a run whose row is still `Running`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Ceiling order, window math, `Retry-After` | New window/ceiling code in the guard | `AllowancePolicy::ceilings_for`, `window_for`, `AllowanceRefusal::retry_after_secs`, `store_now` | One clock and one order (41 D-01/D-03); a second implementation drifts |
| Once-per-window dedup | In-memory "already notified" sets as the truth | `TreasuryNoticePort::record` (`INSERT ... ON CONFLICT DO NOTHING`) with the new rung | Store-enforced across replicas (41 D-16). An in-process memo is allowed **only** to skip repeat writes (Pitfall 9), never as the dedup |
| Per-run token cutoff | A new middleware or loop check | `TokenBudget::after_model` + `FinalResult` | The one overshoot rule; ADR-0052 |
| Halt checkpoint | A new Waypoint status or writer | `WaypointStatus::Halted` + `build_waypoint` + `persist_waypoint` | `resume`/`fork` already continue from it |
| Resume | `POST /runs/{id}/resume`, a `Halted -> Queued` edge | `POST /threads/{id}/fork` | Deferred by D-07; admission is re-checked for free |
| Webhook signing/delivery for the operator halt notice | A new HTTP client or HMAC code | `OperatorNoticeTarget` + `WebhookDeliveryService` (`sign_webhook_body`) | Signed once over stored bytes, SSRF-guarded, no redirects (security.instructions.md) |
| Money display | Local formatting | `format_cost`, once, at the edge | D-00h |
| Terminal SSE payload per path | Three hand-built JSON objects | One `HaltReason::wire_json` + `terminal_payload` | Byte-identical live/degraded/replay |
| Cancel/drain classification | New engine concepts | Existing `LocalRunTokens`, `DbCancellationProbe`, coordinator token, composed in a wrapper (G1) | Mechanism exists; only the composition is new |
| Vocabulary scan | A new dependency | `std::fs` directory walk in a root integration test | Keeps the dependency graph and `make deny` unchanged |

**Key insight:** every hard part of this phase (windows, ceilings, dedup, checkpoint, resume, signing, cutoff) already exists and is tested. The risk is composing them correctly across crates, not building them.

## Runtime State Inventory

Not a rename/refactor/migration-of-strings phase, so the full 5-category inventory does not apply. The phase does add persisted state, recorded here for the planner:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | New nullable `runs.halt_reason` (JSON text/JSONB); new `treasury_notices.notice_kind` + rebuilt unique index; stored `run_traces` rows gain an optional field | Migration `013` on both backends; pre-existing rows read back `NULL` / `'warning'` / `None` (additive, schema version stays `v1`) |
| Live service config | None — no config key added (CONTEXT code_context); operator webhook target reused | None |
| OS-registered state | None | None — verified by reading the phase scope |
| Secrets/env vars | None new; the operator webhook secret stays on `WebhookDeliveryService` | None |
| Build artifacts | `.project/current-exports.txt` (api-surface baseline), `crates/paladin-web/openapi.json`, `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml` | Regenerate in the same commits (`make api-surface-update`, `make openapi`) |

## Common Pitfalls

### Pitfall 1: Reading only the engine's cause for a caller cancel (G1)
**What goes wrong:** A same-instance `POST /runs/{id}/cancel` fires the child `CancellationToken`; the engine reports a token halt; the worker maps it to `Halted` (or the SSE says `halted`) while the row should be `Cancelled`.
**Why it happens:** the local fast path bypasses the debounced probe by design (`cancel.rs:14-18`), and the shutdown token is the child token's parent.
**How to avoid:** `PerRunCancelProbe` OR-ing the local token (unless shutting down) into the probe answer, and keep `is_cancel_requested` + `shutting_down` as the resolver for the residual Token cause.
**Warning signs:** an e2e test that cancels a running graph run on the same instance and finds `done.status == "halted"`.

### Pitfall 2: Persisting a payload variant in `RunFinishStatus` (G4)
**What goes wrong:** stored `run_traces` rows (`"status":"halted"`) stop deserialising; `herald.rs:757`'s `*status == RunFinishStatus::Failed` and `paladin-eval` break; `Copy` is lost.
**How to avoid:** unit `Cancelled` variant + `RunFinished.halt_reason` field. Add a `run_trace` contract clause that a pre-phase JSON row still reads back and a round trip through the hand-written `TraceRecord` impl keeps `halt_reason` (the impl re-reads one flat map and `serde_json::from_value`s the rest, so a nested object under a `halt_reason` key is safe; keep the discriminator nested, never flattened into the record, or it would collide with the `TraceEvent` tag key `kind`).

### Pitfall 3: `kind` collision in `halt_reason` (G6)
**What goes wrong:** a discriminator named `kind` overwrites or shadows the limit kind, so the object no longer equals the 429 `details`.
**How to avoid:** discriminator `reason`; `kind` stays the limit kind.

### Pitfall 4: Notices table rebuild on SQLite (G5)
**What goes wrong:** adding a halt rung through a new `limit_kind` value violates the `CHECK`; adding a column without rebuilding the unique index lets a halt notice collide with (or be suppressed by) the warn notice; the adapters' `ON CONFLICT (...)` list stops matching the index.
**How to avoid:** `ALTER TABLE ... ADD COLUMN notice_kind ... DEFAULT 'warning'`, then `DROP INDEX IF EXISTS idx_treasury_notices_once; CREATE UNIQUE INDEX ...` with `notice_kind` last; update both `NOTICE_INSERT` constants and run the existing `notice_arbiter_matches_the_migration` tests. SQLite permits `ADD COLUMN` with a constant default; the CHECK on an added column is the part to prove in the contract suite [ASSUMED: SQLite accepts `ADD COLUMN ... NOT NULL DEFAULT 'warning' CHECK (...)` in the pinned bundled version; verify in the first red test]. `notices_for_run` must filter `notice_kind = 'warning'` or the worker will emit a halt notice as a warning trace event.

### Pitfall 5: Operator delivery branch is keyed on one event (G5)
**What goes wrong:** an `allowance_halted` delivery reaches `WebhookDeliveryService` as a non-operator event, the service tries a run lookup for the correlation run id and dead-letters it, and the webhook adapters fail to parse `"allowance_halted"` from the `event` column.
**How to avoid:** extend `service.rs:240` and both adapters' event parse functions; add a webhook contract clause beside `contract_tests.rs:300-340`.

### Pitfall 6: Worker pool constructed before the Treasurer (G3)
**What goes wrong:** the guard cannot be attached to an already-spawned pool.
**How to avoid:** reorder `build_run_api` as in G3; keep the existing wiring tests (`build_run_api_wires_a_treasurer_only_when_allowances_are_configured`, `wired_treasurer_refuses_an_exhausted_key_through_the_run_router`) green.

### Pitfall 7: Child-run halt re-evaluated non-stickily (G11)
**What goes wrong:** a child engine halts on spend, the parent re-reads after a window roll, passes, and continues with an empty child delta.
**How to avoid:** memoise the first `Halt` in the per-run guard and share it with child clones.

### Pitfall 8: Status written before the reason (G14)
**What goes wrong:** the degraded `done` for a freshly `Halted` row has no `halt_reason`; a client compares and sees a mismatch.
**How to avoid:** `record_outcome` before `update_status` for halting outcomes; e2e tests poll to terminal before comparing.

### Pitfall 9: A balance read and a notice write on every boundary
**What goes wrong:** up to four `balance` reads (required, D-02) **plus**, once past the warn threshold, one `INSERT ... ON CONFLICT DO NOTHING` per ceiling per boundary for the rest of the run.
**How to avoid:** keep D-02's reads, but memoise in the guard "this ceiling's notice already claimed or lost for this window" so the store write happens once per ceiling per window per run. The store remains the dedup truth; the memo only skips redundant writes.

### Pitfall 10: `execute/stream` has no `after_model` (G2)
**What goes wrong:** a plan task "install the derived budget on the streamed route and test that it halts" cannot pass.
**How to avoid:** scope the derived-budget tests to `execute` and the buffered paths; assert the streamed route is admission-only plus the optional informational `halt_reason` on `done`.

### Pitfall 11: Frozen v0.9 OpenAPI gate (G9)
**What goes wrong:** `cargo test -p paladin-web --test openapi_golden_v0_9` fails after adding `halt_reason` to `ExecuteResponse`.
**How to avoid:** extend `strip_known_v0_10_execute_response_divergence` and its docs in the same commit as the DTO change; regenerate `openapi.json`.

### Pitfall 12: Exhaustive-pattern churn from `RunOutcome::Halted { cause }`
**What goes wrong:** the field addition breaks every pattern that names the variant's fields without `..`.
**Known sites (grep, 2026-10-06):** `engine/mod.rs:6445,6648,6810` (tests) and `:372`; `paladin-eval/src/runner.rs:935`; `examples/graceful_shutdown.rs:236,294`; `worker.rs:486` and its tests `:1944,1962,1970`; `superstep.rs:2241,3826` (constructors). Sites already using `..` are unaffected.
**How to avoid:** fix them in the same task as the type change; record the break in MIGRATION.md §9.2 (type is new-in-0.10 -> `N/A` row by the precedent at rows 208-240).

### Pitfall 13: Reserved-word and register hygiene
**What goes wrong:** `Treasurer` appears in a fixture/example/benchmark file or beside the `GarrisonTreasury` term; `make api-surface` drifts; `check-migration-allowlist.sh` finds a §9.2 row without an allowlist entry (or vice versa).
**How to avoid:** the guard test (ALLOW-05) plus the register steps in D-21 in every commit that adds a public item.

### Pitfall 14: Overspend wording
**What goes wrong:** docs/ADR promise "can never overspend". The derived budget is checked after a response, so overshoot is at most one response (plus, on the engine path, one superstep). Say exactly that.

## Code Examples

### Example 1: `HaltReason` and its single wire builder (core)

```rust
// Source: designed for this phase on the AllowanceRefusal shape (allowance.rs:113-128)
// and the 429 details object (paladin-web error.rs:184-191).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
#[non_exhaustive]
pub enum HaltReason {
    /// A ceiling was reached; the figures are the refused ceiling's own.
    AllowanceExhausted {
        scope_kind: AllowanceScopeKind,
        limit_kind: AllowanceLimitKind,
        balance: Cost,
        ceiling: Cost,
        window: Option<(DateTime<Utc>, DateTime<Utc>)>,
    },
    /// A ceiling could not be evaluated (fail closed, D-03); no figures.
    LedgerUnavailable,
}

impl HaltReason {
    /// The one wire object: byte-identical on GET /runs, SSE `done`, the webhook key.
    pub fn wire_json(&self) -> serde_json::Value {
        match self {
            Self::AllowanceExhausted { scope_kind, limit_kind, balance, ceiling, window } => json!({
                "reason": "allowance_exhausted",
                "scope": scope_kind.as_str(),
                "kind": limit_kind.as_str(),
                "balance": format_cost(balance),
                "ceiling": format_cost(ceiling),
                "window_start": window.map(|(s, _)| s.to_rfc3339_opts(SecondsFormat::Secs, true)),
                "window_end": window.map(|(_, e)| e.to_rfc3339_opts(SecondsFormat::Secs, true)),
            }),
            Self::LedgerUnavailable => json!({ "reason": "ledger_unavailable" }),
        }
    }
}
```

### Example 2: Migration `013` (both backends; sketch)

```sql
-- Source: header/style of 008_add_run_attribution_columns.sql and 011_create_treasury_notices.sql.
-- SQLite (Postgres: same intent, JSONB for the run column, `DROP INDEX IF EXISTS` + `CREATE UNIQUE INDEX`).
ALTER TABLE runs ADD COLUMN halt_reason TEXT NULL;          -- serialized HaltReason JSON; NULL = none recorded
ALTER TABLE treasury_notices
    ADD COLUMN notice_kind TEXT NOT NULL DEFAULT 'warning'
    CHECK (notice_kind IN ('warning', 'halt'));
DROP INDEX IF EXISTS idx_treasury_notices_once;
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices
    (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos, notice_kind);
```
Both backends need every `runs` SELECT/INSERT constant touched: sqlite has four SQL constants at `sqlite.rs:48-87` and postgres four at `postgres.rs:56-92` plus the row mapper (`sqlite.rs:270-340`, `postgres.rs:250-320`) and `record_outcome` (`sqlite.rs:479`, `postgres.rs:509`). `RunOutcomeRecord` gains `halt_reason: Option<HaltReason>` (it derives `Default`, so literals in tests need `..Default::default()`).

### Example 3: Budget derivation (treasurer)

```rust
// Source: designed on cost.rs PriceRow accessors (cost.rs:321-342) and cost_of_call defaults (cost.rs:375-403).
fn dearest_price_per_million(row: &PriceRow) -> i64 {
    let prompt = row.prompt();
    [
        prompt,
        row.completion(),
        row.cache_read().unwrap_or(prompt),
        row.cache_write().unwrap_or(prompt),
        row.reasoning().unwrap_or(row.completion()),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

/// `None` = unbounded (free model). `Some(0)` is the caller's refusal (D-09).
fn derive_max_tokens(remaining_nanos: i64, price_per_million: i64) -> Option<u32> {
    if price_per_million <= 0 {
        return None;
    }
    let tokens = i128::from(remaining_nanos.max(0)) * 1_000_000 / i128::from(price_per_million);
    Some(u32::try_from(tokens).unwrap_or(u32::MAX))
}
```

### Example 4: Treasurer-aware `TokenBudget::after_model` (D-11/D-12)

```rust
// Source: limits.rs:146-171 (existing), extended. The derived figure arrives on the per-call
// context (never a field on the middleware, limits.rs D-03).
let operator = self.config.enabled.then_some(self.config.max_tokens);
let derived = cx.treasurer_budget().map(|b| b.max_tokens);
let (limit, treasurer_won) = match (derived, operator) {
    (Some(d), Some(o)) if d <= o => (d, true),
    (Some(_), Some(o)) => (o, false),
    (Some(d), None) => (d, true),
    (None, Some(o)) => (o, false),
    (None, None) => return Ok(MiddlewareFlow::Continue),
};
if cx.cumulative_tokens > limit {
    resp.content.push_str(TOKEN_BUDGET_NOTICE);
    let stop = match (treasurer_won, cx.treasurer_budget()) {
        (true, Some(b)) => StopReason::AllowanceHalted(b.halt_figures()),
        _ => StopReason::TokenBudget,
    };
    return Ok(MiddlewareFlow::Finish(FinalResult::new(resp.content.clone(), stop)));
}
Ok(MiddlewareFlow::Continue)
```
A tie goes to the Treasurer (`d <= o`): at equal limits the allowance, not the operator knob, is the binding constraint.

### Example 5: Degraded/replay `done` from the row (events)

```rust
// Source: events.rs:453-484 (existing terminal_payload), extended.
RunStatus::Halted => {
    let mut payload = json!({ "status": "halted", "waypoint_id": run.final_waypoint_id });
    if let Some(reason) = &run.halt_reason {
        payload["halt_reason"] = reason.wire_json();
    }
    (RunStreamEventKind::Done, payload)
}
```

### Example 6: Engine test shape (first red test, CONTEXT specifics (a))

```rust
// Source: pattern of cancellation_probe_tests, crates/paladin-battalion/src/engine/mod.rs:6598-6670.
struct HaltAtCall { calls: AtomicUsize, halt_at: usize, reason: HaltReason }
#[async_trait]
impl SpendGuard for HaltAtCall {
    async fn check(&self, _t: &ThreadId) -> SpendDecision {
        let n = self.calls.fetch_add(1, SeqCst) + 1;
        if n >= self.halt_at { SpendDecision::Halt(self.reason.clone()) } else { SpendDecision::Continue }
    }
}
// engine.with_spend_guard(guard) over four_node_chain_graph(): assert RunOutcome::Halted{cause: Spend(..)},
// Waypoint status Halted whose vanguard is boundary N's, and that no node of superstep N was dispatched.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Cancel and halt collapsed: engine `RunFinished{Halted}` for a cancelled run; SSE `done: "halted"` documented as a "deliberate simplification" (`worker.rs:1248-1255`) | Distinct `Cancelled` status + typed halt reason end to end | This phase (PLAT-09) | The comment block at `worker.rs:1244-1255` must be rewritten, not just left stale |
| Ledger observational only (settle at the boundary, never gates; `settlement.rs` module doc) | Same hook site also authoritative via a check-only guard | This phase (D-01) | The module doc in `settlement.rs` ("until Phase 41/42 make it authoritative through this same boundary hook") must be updated |
| `TokenBudget` code-complete, unit-tested, wired nowhere in production (ADR-0052) | First production caller: per-agent HTTP services and the shared service in Treasurer-only mode | This phase (ALLOW-05) | `agent_runtime.token_budget` starts to have an effect on HTTP agent routes; documented behaviour change (G12) |
| ADR-0056: Phase 42 reserves at the superstep boundary | ADR-0057 supersedes with a check-only boundary; race accepted | This phase (D-01) | Dated note on ADR-0056 |

**Deprecated/outdated:**
- ADR-0056's "Downstream Consumers: Phase 42 closes the D-05 over-admission race by reserving at the superstep boundary" — superseded by D-01.
- The `worker.rs` D-14 comment claiming `RunFinishStatus` "carries no `cancelled` variant" — made false by this phase.

**Prior art:** none usable. CONTEXT's roadmap research flag states no surveyed comparable system has durable multi-step runs; this phase is validated only against this repo's contract and e2e tests. No external documentation was needed (Context7/web lookups were not run: there is no external library API in scope).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The ALLOW-05 vocabulary guard is best a root integration test (`tests/treasurer_vocabulary_guard.rs`, auto-discovered, `std::fs` walk) with a path allowlist: `Treasurer` may appear only in framework paths (`src/`, `crates/*/src`, `docs/`, `MIGRATION.md`, `CHANGELOG.md`, `.planning/`, `.project/`), and fails if it (or `GarrisonTreasury`) appears in `examples/`, `benches/`, `fixtures/`, `tests/fixtures/`, or any file that also names `GarrisonTreasury`. ADR-0050 calls the guardrail "a cross-repo convention, not a lint enforced in this repo", so "downstream use" has no in-repo definition yet | Phase Requirements, Don't Hand-Roll | The guard is too loose or too tight; needs operator confirmation of the allowlist (Open Question 4) |
| A2 | `cargo semver-checks` would classify every type this phase changes other than `StopReason`, `ExecuteResponse` and `RunOutcome`-like pre-0.10 types as "absent at the v0.9.0 baseline" (N/A rows, no allowlist entry), by the precedent rows at MIGRATION.md 208, 210, 240. The tool is not installed here, so it was not run | CONTEXT Gaps (G4/G9/G17), Environment | An unexpected `Y` row needs an allowlist entry in the same commit or the CI set-equality gate fails |
| A3 | SQLite accepts `ALTER TABLE ... ADD COLUMN notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (...)` and `DROP INDEX` + `CREATE UNIQUE INDEX` inside an sqlx migration on the bundled version | Pitfall 4, Example 2 | Migration fails on SQLite; fall back to the table-rebuild idiom (create new, copy, drop, rename) |
| A4 | Four balance reads per boundary per run (plus one store-clock read) are acceptable load; migration 012's tenant index serves them | Pitfall 9 | Ledger load grows with concurrent allowance-bound runs; D-02 already locks "no caching", so the fix would be a new decision |
| A5 | For an agent-loop halt the `HaltReason` figures should report `balance = ceiling` (the headroom the derived budget was sized to is spent), `ceiling` and window of the **binding** ceiling, because the loop does not know the true post-spend balance | Gap G8, Open Question 5 | A client may read `balance` as the exact ledger balance; the docs must say it is the conservative bound |
| A6 | The `PerRunCancelProbe` (OR of debounced DB probe and un-debounced local token, minus shutdown) fully resolves caller-cancel attribution on the same instance; a cancel landing on another instance reaches the engine only through the DB probe within `min_probe_interval_ms` | Gap G1 | A cross-instance cancel can still surface as the Token cause (`map_outcome` then re-queries the flag, so the **row** is right; only the live `done` could lag for that boundary) |
| A7 | The shutdown-token check inside `RunEventBusSink` is enough to implement D-15 (no `done` for a drain) | Gap G1 (c) | A halt at the exact instant shutdown begins could be mis-suppressed or mis-emitted; the degraded path still converges on the row |
| A8 | Worker agent-kind runs that hit a zero derived budget at dispatch should end `Halted` with the refusal figures without calling the LLM (admission-time refusal is impossible once the run is `Running`) | Gap G8 | A different outcome (e.g. `Failed`) would contradict "a halt is a resume point, not a failure" |

## Open Questions (RESOLVED)

All seven questions are resolved by the consolidated design gate, 42-01 Task 1 (a blocking
`checkpoint:decision`): each recommendation below is a numbered item of that gate's proposed design,
the operator confirms it (option-a/option-b) or redirects it (option-c), the selection is recorded
verbatim in `42-01-SUMMARY.md` under "Checkpoint decision", and ADR-0057 (42-01 Task 2) records the
confirmed answer. The per-question marker names the gate item and the plan that implements it.

1. **How exactly does a same-instance caller cancel reach the engine as `CancelRequested`? (G1)**
   - What we know: the engine sees the child token; the probe is debounced; the shutdown token is the child's parent; today's worker re-queries the flag.
   - What's unclear: whether the operator prefers the per-run probe wrapper (recommended) or a sink-level fix.
   - Recommendation: the wrapper in the worker's per-run engine build, with the residual-Token re-query kept. Record in ADR-0057.
   - **RESOLVED: 42-01 Task 1 item 5** -- the per-run `PerRunCancelProbe` wrapper with the residual-`Token` re-query kept in `map_outcome`; implemented by 42-06.

2. **Should `execute/stream` carry any derived-budget behaviour? (G2)**
   - What we know: one provider call, no `after_model`; terminal chunk usage is optional (default usage with a warning when absent, `paladin_execution_service.rs:3667-3680`).
   - What's unclear: whether informational `halt_reason` on the stream's `done` is wanted.
   - Recommendation: admission-only for real streams plus one `WINDOWS.md` row; add the informational object only if the SDK smoke tests stay byte-identical for non-halt streams.
   - **RESOLVED: 42-01 Task 1 item 12** -- a true streamed call is admission-only with a byte-identical `done` (option-a), or additionally carries the informational `halt_reason` when its terminal usage crossed the derived figure (option-b, the gate's flagged alternative); the buffered fallback's `done` carries `halt_reason` either way. Implemented by 42-08; the WINDOWS.md row lands in 42-12.

3. **Operator `agent_runtime.token_budget` on the worker's agent-kind path (G12).**
   - What we know: the shared service must not honour the operator budget (ADR-0052 hazard); D-11 wants tightest-wins.
   - What's unclear: whether worker agent-kind runs should additionally enforce the operator figure.
   - Recommendation: no; Treasurer-only mode on the shared service, documented. Revisit only if an operator asks.
   - **RESOLVED: 42-01 Task 1 item 10** -- Treasurer-only mode on the shared boot-time service; the operator figure applies on the HTTP agent routes only. Implemented by 42-08 (per-agent services) and 42-09 (shared service).

4. **Allowlist definition for the vocabulary guard (A1).** Needs operator confirmation of which paths count as "downstream" in this repo.
   - **RESOLVED: 42-01 Task 1 item 16** -- the downstream path set (`examples/`, `benches/`, `fixtures/`, any `tests/fixtures/`, `crates/*/examples/`, `crates/*/benches/`) and the explicit allowlist for the downstream fixture term are confirmed at the gate; implemented by 42-12 Task 2.

5. **Which figures does an agent-loop `halt_reason` report (A5)?** Recommendation: the binding ceiling's figures with `balance = ceiling`, documented as a conservative bound.
   - **RESOLVED: 42-01 Task 1 item 10** -- the binding ceiling (smallest remaining) with `balance` equal to the ceiling, documented as a conservative bound; implemented by 42-07 (`DerivedTokenBudget.halt_figures`).

6. **Expose `final_waypoint_id` on `GET /runs` rows (G7)?** Recommendation yes (additive). If rejected, the fork recipe must use thread history and the UAT runbook must say so.
   - **RESOLVED: 42-01 Task 1 item 7** -- yes, additive `final_waypoint_id` (and `halt_reason`) on `RunResponse`; implemented by 42-03, the fork recipe by 42-04.

7. **Which refusal wire shape for an unpriced model (D-10)?** CONTEXT leaves `429`-shaped vs `422` to the planner. Recommendation: `422` with `code: "model_unpriced"` and the model name in `details` — it is a configuration incoherence, not quota exhaustion, so it must not carry `Retry-After` and a client must not retry it as pacing. Reuse `ApiError`'s existing 422 helper if present; otherwise add one beside `allowance_exhausted`.
   - **RESOLVED: 42-01 Task 1 item 11** -- `422`, code `model_unpriced`, `details: { "model": <name> }`, no `Retry-After`; the Treasurer-level refusal lands in 42-07, the HTTP agent routes in 42-08, `POST /runs` for agent-kind assistants in 42-09.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / `rustc` | all tasks | yes | 1.97.1 | — |
| Pinned nightly `nightly-2026-09-20` | `make api-surface` | yes | installed | — |
| `cargo-deny` | `make security` | yes | 0.20.2 | — |
| `cargo-audit` | `make security` | yes | installed (0.22.2 reported) | — |
| `cargo-llvm-cov` | coverage floor (82 %) | no | — | CI `coverage` job is authoritative; local runs use targeted `cargo test` |
| `cargo-semver-checks` | D-27 empirical diagnostics for MIGRATION.md rows | no | — | CI `semver` job (baseline 0.9.0); install locally only if a plan task needs the empirical lint id (`cargo install cargo-semver-checks`) |
| PostgreSQL server | Postgres leg of the run/notice/webhook contract suites | no (`pg_isready`: no response) | — | CI `postgres-integration` job is authoritative (fails on any `SKIP:` line); locally prove SQLite + in-memory only and say so |
| Docker daemon | `make test-integration-docker` | no (socket absent) | — | Not needed: no Redis/MinIO dependency in this phase |
| `sqlite3` CLI | none (sqlx bundles SQLite) | no | — | not required |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** `cargo-llvm-cov`, `cargo-semver-checks`, a PostgreSQL server (all covered by CI jobs; plan tasks must not mark Postgres clauses "passed locally").

## Validation Architecture

> `workflow.nyquist_validation` is absent from `.planning/config.json` (it holds only `workflow._auto_chain_active`, `worktree_skip_hooks`, `test_gate_timeout`), so this section is included.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` — native Rust `#[cfg(test)]` modules, doctests, `#[tokio::test]`; shared storage contract suites are plain modules |
| Config file | none — workspace `Cargo.toml`; root `tests/*.rs` are auto-discovered (no `autotests = false`) |
| Quick run command | per crate: `cargo test -p paladin-battalion --lib engine::tests::spend_guard_tests`, `cargo test -p paladin-ai --lib application::services::treasurer`, `cargo test -p paladin-ai --lib application::services::run::worker`, `cargo test -p paladin-ai --lib application::services::run::events`, `cargo test -p paladin-web --lib` |
| Full suite command | `cargo test --workspace` (unit + doctests); Postgres leg only in CI |
| Estimated runtime | ~1-3 min per incremental single-crate run; ~10-15 min cold workspace (41-VALIDATION precedent) |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ALLOW-03 | Guard `Halt` at boundary N writes a `Halted` Waypoint (vanguard = N's), `RunOutcome::Halted{cause: Spend}`, no node of superstep N dispatched | unit (engine, TDD first red test) | `cargo test -p paladin-battalion --lib engine::tests::spend_guard_tests` | no, Wave 0 (new mod beside `cancellation_probe_tests`) |
| ALLOW-03 | No guard / no ceiling -> no check and no ledger read; ledger outage with a ceiling -> `ledger_unavailable` halt, checkpoint kept; sticky halt in child runs (G11) | unit | same | no, Wave 0 |
| ALLOW-03 | `map_outcome` table: every cause -> exactly one `OutcomeAction` (incl. Token + cancel_requested, Token + shutting_down) | unit | `cargo test -p paladin-ai --lib application::services::run::worker::tests::map_outcome` | yes (extend) |
| ALLOW-03 | `halt_reason` round-trips on in-memory, SQLite, Postgres run adapters; legacy rows read `None` | contract | `cargo test -p paladin-storage --features sqlite --lib run::` (Postgres: CI) | yes (extend `contract_tests.rs`) |
| ALLOW-03 | Halted engine run -> `GET /runs/{id}` (`status: halted`, `halt_reason`, `error: null`, `final_waypoint_id`) -> fork -> `429` + `Retry-After` while exhausted -> window reset (scripted store clock) -> fork -> `Completed`, completed supersteps not re-dispatched | e2e (HTTP surface) | `cargo test -p paladin-ai --lib application::services::run::http_surface_tests` | yes (extend) |
| ALLOW-03 | Agent-kind run halted by the derived budget -> `Halted` + `halt_reason`, partial output kept; zero derived budget at dispatch -> `Halted` without an LLM call | unit/e2e (worker) | `cargo test -p paladin-ai --lib application::services::run::worker_tests` | yes (extend) |
| ALLOW-05 | Derivation: dearest of five axes, `i128`, floor, free model -> `None`, zero remaining -> refusal, unpriced model + ceiling -> typed refusal, no ceiling -> unaffected | unit (TDD) | `cargo test -p paladin-ai --lib application::services::treasurer` | partly (extend `treasurer/tests.rs`) |
| ALLOW-05 | Derived budget N halts a mock-scripted run after the crossing response, `is_successful() == false`, content + notice kept; same run under the operator budget stops `Completed`/`token_budget`; tie -> Treasurer; `ModelCallLimit`/`ToolCallLimit` untouched | unit | `cargo test -p paladin-ai --lib application::services::paladin::middleware::limits` | yes (extend) |
| ALLOW-05 | `StopReason` variant: `is_limit`, `is_successful`, serde, `stop_reason_label` | unit | `cargo test -p paladin-ai-core --lib execution_result` / `cargo test -p paladin-web --lib agent_controller::tests::stop_reason_labels_are_stable` | yes (extend) |
| ALLOW-05 | `Treasurer` stays framework-only | integration (guard) | `cargo test -p paladin-ai --test treasurer_vocabulary_guard` | no, Wave 0 |
| PLAT-09 | `map_trace_event`: `RunFinished{Cancelled}` -> `done`/`"cancelled"`; `RunFinished{Halted, halt_reason}` -> `done` + object; Halted without reason (drain) -> no `done` at the sink | unit | `cargo test -p paladin-ai --lib application::services::run::events` | yes (extend enumeration test) |
| PLAT-09 | Live == degraded == replay `done` for completed/cancelled/halted/ledger_unavailable; cancel on the same instance yields `cancelled` (G1) | e2e/stream tests | `cargo test -p paladin-ai --lib application::services::run::stream_tests` | yes (extend) |
| PLAT-09 | `TraceRecord` round trip with `halt_reason`; pre-phase `RunFinished` JSON still deserialises | unit + `run_trace` contract | `cargo test -p paladin-ai-core --lib trace` ; `cargo test -p paladin-storage --features sqlite --lib run_trace::` | yes (extend) |
| PLAT-09 | Frozen v0.9 OpenAPI unchanged except sanctioned divergences; `openapi.json` regenerated | golden | `cargo test -p paladin-web --test openapi_golden_v0_9` ; `make openapi` + `git diff --exit-code crates/paladin-web/openapi.json` | yes (extend) |
| D-18 | Halt notice dedup: 20 halts in a window -> 1 operator delivery; raised ceiling re-arms; `ledger_unavailable` -> none; warn notice unaffected; `notices_for_run` excludes halt rows | contract + unit | `cargo test -p paladin-storage --features sqlite --lib treasury::` ; `cargo test -p paladin-ai --lib application::services::run::webhook` | yes (extend) |
| D-19 | Caller `Halted` webhook carries `halt_reason`, signature over the stored bytes unchanged; herald line in markdown/json/table | unit | `cargo test -p paladin-ai --lib application::services::run::webhook` ; `cargo test -p paladin-herald --lib` | yes (extend) |
| D-21 | Registers: MIGRATION.md 9.2/9.4/9.6 rows, allowlist set-equality, api-surface, CHANGELOG | gate | `./scripts/check-migration-allowlist.sh` ; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | yes |

### Sampling Rate
- **Per task commit:** the task's targeted `cargo test -p <crate> --lib <module>` + `cargo clippy -p <crate> -- -D warnings` + `cargo fmt --check`; register-touching tasks add `./scripts/check-migration-allowlist.sh` and `make api-surface`.
- **Per wave merge:** `cargo test --workspace`.
- **Phase gate:** `cargo fmt --check`, `cargo test --workspace`, `make clean-code`, `make api-surface`, `make check-gates`, `make security`, `make openapi` with a clean diff, `cargo test -p paladin-web --test openapi_golden_v0_9`, CI `coverage` (>= 82 %) and `postgres-integration` (no `SKIP:`), before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/paladin-battalion/src/engine/mod.rs` — new `spend_guard_tests` module (mock `SpendGuard`), covers ALLOW-03
- [ ] `tests/treasurer_vocabulary_guard.rs` — covers ALLOW-05's guard (allowlist per A1)
- [ ] `crates/paladin-ports/src/output/spend_guard.rs` — port + doctest (public API needs doc tests)
- [ ] `crates/paladin-storage/src/run/contract_tests.rs` — `halt_reason` clauses; `treasury/notice_contract_tests.rs` — halt-rung clauses
- [ ] Postgres leg of all new contract clauses is CI-only (no local server): plan tasks must say so rather than claim local green
- [ ] No new framework install needed

## Security Domain

> `security_enforcement` is absent from config, treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (unchanged) | existing opaque-bearer / API-key auth (ADR-0040) |
| V3 Session Management | no | — |
| V4 Access Control | yes | Fork re-runs `ensure_thread_visible` + `authorize_invocation` + admission; `halt_reason` / `final_waypoint_id` are served only through `load_visible_run` tenant scoping; no Admin bypass (41 D-09); the guard takes a `RunAttribution`, never a role |
| V5 Input Validation | yes | `HaltReason` read back through typed serde (a corrupt column is a `Serialization` error, not a panic); SQL only via bound parameters and `&'static str` constants (41 T-41-03 precedent); `from_waypoint_id` already parsed by `parse_waypoint_id` |
| V6 Cryptography | yes (reuse only) | HMAC signing stays `sign_webhook_body` over the exact stored bytes; never hand-roll |
| V7 Error handling & logging | yes | No log line, error body, trace event, webhook payload or herald line carries an API key value; `ledger_unavailable` logged at `error` with the backend error text but never a key; `AllowanceRefusal`/`HaltReason` carry no tenant id or key name |
| V8 Data protection | yes | `halt_reason` figures are the ceiling's own (tenant-scope ceiling figures are visible to keys of that tenant, same as the 429 `details`); operator notice payload carries `api_key_id` (the **name**) only, as in Phase 41 |
| V13 API & web service | yes | `429`/`422` bodies and `Retry-After` follow the Phase 41 contract; OpenAPI regenerated and golden-gated |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Over-admission race: two runs admitted in the same instant both start | Elevation / Tampering (of the budget) | Accepted and recorded (D-01); boundary check halts each at its first boundary after exhaustion; overshoot bounded to one superstep |
| Allowance bypass through an ungated spend path (`/agents/{id}/jobs`) | Elevation of privilege | Gate `jobs` with admission and the derived budget (Discretion recommendation); else a `WINDOWS.md` row |
| Spend on an unpriced model escapes the ledger | Repudiation / Tampering | D-10 refuses unpriced *agent* models under an allowance; engine nodes recorded as G15 residual |
| Ledger outage halts every metered run | Denial of service (self-inflicted) | Intended fail-closed (D-03); only principals with a ceiling are affected; halted runs are resumable; error-level log |
| Per-boundary DB load amplification | Denial of service | Bounded reads (<= 4 + clock) and a write memo (Pitfall 9); no debounce by decision D-02 |
| Information disclosure of another scope's figures | Information disclosure | Figures are the refused/halted ceiling's own; reads go through tenant-scoped `load_visible_run`; no tenant id / key name in the object |
| Webhook SSRF / credential forwarding for the operator `allowance_halted` delivery | Spoofing / Information disclosure | Same SSRF-guarded, no-redirect, signed-once delivery service; extend only the event allow-list (Pitfall 5); never add a second HTTP client |
| Replay of a drained run's `done` as terminal | Spoofing (status) | Suppress at the sink (D-15); replay/degraded paths trust the row (G10) |
| Stored-data downgrade/upgrade mismatch on `run_traces` | Tampering | `#[serde(default)]` optional field; legacy-row clause in the `run_trace` contract suite |

## Sources

### Primary (HIGH confidence) — codebase, read in this session
- `.planning/phases/42-mid-run-halt-sse-terminal-status/42-CONTEXT.md` — locked decisions (copied above)
- `.planning/decisions/0052-mid-run-treasurer-enforcement.md`, `0056-allowance-admission-model.md`, `0050-treasurer-reservation.md`, `PROMOTION.md` (next free ADR 0057)
- `crates/paladin-battalion/src/engine/superstep.rs` (boundary check 2190-2244, second Halted site 3770-3830, settle 3455-3470, child run 1440-1523), `engine/mod.rs` (`RunOutcome` 305-340, `run_finish_status` 368-375, `resume` 2271-2440, `fork` 2851-3010, `cancellation_probe_tests` 6598-6670), `engine/settlement.rs`
- `crates/paladin-ports/src/output/cancellation_probe.rs`, `run_repository_port.rs` (`RunOutcomeRecord`), `treasury_ledger_port.rs` (`TreasuryLedgerError`, `balance`), `input/allowance_admission_port.rs`
- `crates/paladin-core/src/platform/container/{allowance,trace,run,run_scope,execution_result,cost,treasury_ledger,token_usage}.rs`
- `src/application/services/run/{worker,events,cancel,submission}.rs`, `run/webhook/{mod,service}.rs`, `treasurer/{mod,policy}.rs`, `paladin/middleware/limits.rs`, `paladin/paladin_execution_service.rs` (1275-1330, 1524-1900, 3494-3720)
- `src/infrastructure/web/{run_api_wiring,agent_host,facade_provisioner}.rs`, `src/infrastructure/telemetry/herald_sink.rs`, `src/config/agent_runtime.rs`
- `crates/paladin-web/src/{agent_controller,run_controller,error}.rs`, `crates/paladin-web/tests/openapi_golden_v0_9.rs`
- `crates/paladin-storage/migrations/{sqlite,postgres}/*`, `src/run/{sqlite,postgres,in_memory}.rs`
- `MIGRATION.md` rows 207-240 (register precedents), `.cargo/semver-checks-allowlist.toml`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/phases/41-admission-time-allowance-enforcement/41-VALIDATION.md`
- Local tool probes: `cargo --version`, `rustup toolchain list`, `cargo llvm-cov/semver-checks` (absent), `pg_isready`, `docker ps`

### Secondary (MEDIUM confidence)
- none (no web or Context7 lookups; no external library API is in scope)

### Tertiary (LOW confidence)
- SQLite DDL behaviour in Assumption A3 (general knowledge, to be proven by the first red migration test)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependency; every crate and version read from the manifests.
- Architecture: HIGH for the attachment points and data flow (read in code); MEDIUM for resolutions G1, G2, G8, G12 (design choices that need ADR-0057 confirmation).
- Pitfalls: HIGH — each is anchored to a file and line read this session; A3/A4/A6/A7 are the only inferred items.

**Research date:** 2026-10-06
**Valid until:** 2026-11-05 (30 days; code moves quickly on this branch — re-check `superstep.rs`, `worker.rs`, `run_api_wiring.rs` line anchors if Phase 43 lands first)
