# ADR-0056: Allowance admission model: tumbling UTC windows, check-only admission, every-limit composition, store-deduped notices

## Status

Accepted

**Date:** 2026-10-04

**Phase:** 41 (Admission-Time Allowance Enforcement), plan 41-09. The design was approved at the plan 41-01
Task 1 checkpoint on 2026-10-03 (see Decision, "Checkpoint outcome").

## Context

Phase 41 delivers ALLOW-01 (per-tenant and per-API-key spend allowances over a rolling window, computed
on the store clock), ALLOW-02 (a caller over an allowance is refused at admission, before any run exists)
and ALLOW-04 (a once-per-window warn notice delivered as a trace event, a herald line and a webhook).
The decisions below are the Phase 41 planning decisions D-01, D-03, D-05, D-09, D-10 and D-16
(41-CONTEXT.md), with the alternatives rejected in 41-DISCUSSION-LOG.md.

Four earlier decisions fix the ground this phase stands on:

- **ADR-0052** fixes where Phase 42 halts a run mid-flight: the engine superstep boundary and the
  agent-loop `TokenBudget` cutoff. Phase 41 therefore writes no `reserve` rows, touches neither
  `crates/paladin-battalion/src/engine/{superstep,mod}.rs` nor
  `src/application/services/paladin/middleware/limits.rs`, and wires no `TokenBudget` (D-00b).
- **ADR-0053** fixes the ledger as append-only and derive-on-read: a balance is a `SUM` of signed
  `amount_nanos` contributions, never a stored counter. Admission reads that derived figure; it never
  writes one.
- **ADR-0054** made the tenant server-derived and attached a `RunAttribution { tenant_id, api_key_id }` to
  every run, so a ceiling can name the principal that caused a run.
- **ADR-0050** reserved `Treasurer` as the single officer word for cross-run spend governance. Allowances
  are the Treasurer's; no new officer word is introduced and `Commissary` and `TokenBudget` are not
  renamed (D-00f).

Spend is started from four principal-bearing paths and one principal-less one: `POST /v1/runs`,
`POST /v1/threads/{id}/fork`, `POST /v1/agents/{id}/execute` and `/execute/stream` (plus `jobs`), and
schedule-fired runs. Same-process embedders and tests carry `requested_by: None` and are never gated or
attributed (D-07).

## Decision

**Checkpoint outcome (41-01 Task 1, resolved option-b).** The consolidated design was approved as proposed,
including the defaulted `TreasuryLedgerPort::balance` method (a defaulted trait method, X-10.4, so no
implementor breaks and the engine test doubles stay untouched, which honours D-00b), the `''` and Unix-epoch
sentinels of migration `011`, claim-before-insert with abandon, the C3 Option A webhook row, and the D-08
schedule columns. D-17 was amended in the same decision: the operator webhook payload carries twelve keys,
adding `tenant_id` and `api_key_id` (the tenant id and the API key name, never a key value), so a notice from
the HTTP agent path, where `run_id` is `null`, still identifies the scope that crossed.

**D-01. Tumbling windows aligned to the UTC epoch, read from the store clock only.** For a configured
`period` of `P` seconds and a `store_now()` reading truncated to whole seconds,
`window_start = floor(now / P) * P` and `window_end = window_start + P`. No worker-local clock, no calendar
arithmetic, no trailing window. The reset instant is predictable, the once-per-window notice has the stable
key `window_start`, and a refused caller is told the reset instant (`Retry-After`). The accepted cost is that
a tenant can spend up to twice an allowance across one window boundary (the tail of one window plus the head
of the next); this is documented, not mitigated. Reversibility is costly: the reset instant is part of the
`429`/`Retry-After` contract, the notices table is keyed on `window_start`, and Phase 42's reservations reuse
the same `window_for` function.

**D-03. Every configured limit must fit.** A principal `(tenant, key)` has up to four ceilings, evaluated in
one fixed order: key window, key lifetime, tenant window, tenant lifetime. Each is checked against its own
balance and the first exhausted ceiling names the refusal; the remaining balance reads are skipped. The
absence of an entry means unlimited and the scope is not read at all (no sentinel amount). A lifetime cap is
the same balance function over `[Unix epoch, far future)`; it has no reset instant, so its refusal carries no
`Retry-After`.

**D-05. Admission is a check only: no hold, no ledger write.** For each ceiling the Treasurer reads the
balance and refuses when `balance >= ceiling` (a balance exactly at the ceiling is exhausted). Nothing about a
refused submit reaches the ledger. Two admissions in the same instant may both pass; that over-admission race
is accepted here and is closed by Phase 42's reservation at the superstep boundary (ADR-0052), which adds a
`reserve` beside this check without changing it. No test in this phase claims to prevent the race.

**D-09. No role bypass.** Allowances bind every principal regardless of role. The Treasurer's
`admit(&RunAttribution, ..)` receives an identity and no role, so an Admin key with an entry is refused and
warned like any other; unlimited means no entry. Spend is money, not visibility, and an Admin credential must
not be an unbounded spend path. ADR-0054's Admin read bypass is unrelated and unchanged.

**D-10. Fail closed when an allowance applies.** If any required balance read fails (`Backend`,
`CurrencyMismatch`, a store-clock failure) for a principal with at least one configured ceiling,
`submit`/`fork` return `RunSubmissionError::Backend` (`500`) and the agent handlers return a generic
`500 allowance check failed` with no backend detail; no run is persisted or enqueued. A principal with no
configured ceiling never reads the ledger, so a deployment without allowances keeps working through a ledger
outage. Phase 39 D-08 (a settle failure does not fail a run) remains true for settlement; admission is
authoritative, not observational.

**D-16. Once-per-window notices are enforced by the store.** Migration `011` creates `treasury_notices` with
a unique index over `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)` and the
write is `INSERT .. ON CONFLICT DO NOTHING`; zero rows affected is `AlreadyRecorded`, never an error. No
nullable column sits in the key: `''` stands for tenant scope in `api_key_id` and the Unix epoch for a
lifetime `window_start` (the `''`/epoch sentinels). Including `ceiling_nanos` means raising an allowance
re-arms the notice for the same window. The claim is made before the run is inserted and given back on
abandon (claim-before-insert with abandon): an admission that never persists a run discards exactly its own
notices so the next admission re-wins. The result is at most one notice per scope, limit, window and ceiling
across every replica and restart, and exactly one on the non-failure path; a notice-store failure is logged
and never blocks the run, because a notice observes the run and never gates it.

**D-18. The observation legs.** The warn crossing is observed three ways from one store win: a durable
notice row, one `TraceEvent::AllowanceWarning` and one `allowance:` herald line, and one operator webhook.
The webhook is delivered through the existing durable, SSRF-guarded, no-redirect `webhook_deliveries` queue
and signed with an operator secret held on the delivery service; the delivery row names a correlation run id
no run owns (C3 Option A), so `GET /runs/{id}/webhook-deliveries` never lists operator notices. The trace
event is emitted by the worker on the run's first dispatch, before `RunStarted`, from the stored notice.
On the HTTP agent routes the trace event is emitted only when an agent-path trace emitter is wired, and the
herald line appears only on the streamed final chunk: non-streaming `execute` and `jobs` produce no
`ExecutionMetadata` (41-07). The trace and herald legs are therefore best-effort there by design, while the
durable notice row and the operator webhook always fire.

## Considered Options

- **Trailing (sliding) windows and GCRA** (rejected, D-01): the smoothest limiting and the most literal
  "rolling" reading, but no stable window key for once-per-window dedup and no reset instant to report in
  `Retry-After`; a retrofit would change the `429` contract, the notices key and Phase 42's window function.
- **Calendar-aligned windows (`hourly | daily | weekly | monthly`)** (rejected, D-01/D-02): variable month
  length and an enum grammar, and `weekly` needs an anchor. The duration-string grammar (`1h`, `7d`) was
  chosen over an enum period and over integer seconds plus integer nano-units, which no operator writes.
- **Most-specific-limit-wins, and tenant-window-only / key-lifetime-only composition** (rejected, D-03): a
  key allowance that silently shadows the tenant's defeats per-tenant governance; the narrower shapes drop
  half of ALLOW-01.
- **Reusing `spend()` for the tenant-wide figure, or leaving the choice open** (rejected, D-04): `spend()`
  sums settled `charged_nanos` only and ignores reservations once Phase 42 places them. One additive
  `balance(BalanceQuery)` read over `amount_nanos` serves both phases.
- **Reserve a minimum hold at admission, or a zero-amount reserve for a serialized `SUM`** (rejected, D-05):
  race-proof today but pulls Phase 42's settle/release attachment forward, and the zero-row variant covers
  only the `(tenant, key)` pair and writes a row per admission.
- **Gating `RunSubmissionService` only, leaving agent execute open** (rejected, D-07): Phase 40 D-16 made the
  agent routes settle under the principal, so an exhausted `POST /runs` caller could spend the same allowance
  through them.
- **Leaving schedule-fired runs ungated with a WINDOWS.md row, or refusing to create a schedule when auth is
  on** (rejected, D-08): the operator folded creator attribution into this phase instead; the removal of a
  working feature was not acceptable. Pre-existing schedules (`created_by` NULL) stay unattributed and
  ungated, recorded as an open `WINDOWS.md` row.
- **An Admin bypass mirroring Phase 40 D-11** (rejected, D-09): an Admin key would become an unbounded spend
  path.
- **`402 Payment Required` and `403 Forbidden` for a refusal** (rejected, D-12): `402` has uneven client and
  proxy handling and reads as billing; `403` conflates authorization with budget and clients do not retry
  it. `429 Too Many Requests` with a dedicated `allowance_exhausted` code and a `Retry-After` was chosen.
- **A message-only refusal body, or figures without the window** (rejected, D-13): the caller could not tell
  when to retry. The body carries six figures and never the tenant, key name or key value.
- **Separate refusal variants per error enum** (rejected, D-14): two definitions to keep aligned; one core
  `AllowanceRefusal` is wrapped per port.
- **Fail-open with an error-level log on a ledger outage** (rejected, D-10): matches the settle posture but
  turns a database blip into unbounded spend.
- **A ladder of warn thresholds, or the run's own caller webhook, or both** (rejected/deferred, D-15/D-17):
  ALLOW-04 reads singular and the audience of an allowance notice is the operator, not the tenant.
- **In-process dedup (a `HashSet` on the facade) and application-level "have I notified" checks** (rejected,
  D-16): per-process only, re-fires on restart and across replicas; store-enforced idempotency is the Phase
  39 D-06 pattern.
- **A log line only, with no herald or trace change** (rejected, D-18): ALLOW-04 names the herald.

## Code Locations

- `src/application/services/treasurer/` (`mod.rs`, `policy.rs`, `window.rs`) - `Treasurer`,
  `AllowancePolicy`, `ScopeAllowance`, `Ceiling`, `window_for`, `OperatorNoticeTarget`; the one admission
  function, plans 41-01, 41-06, 41-08
- `crates/paladin-core/src/platform/container/allowance.rs` - `AllowanceScopeKind`, `AllowanceLimitKind`,
  `AllowanceRefusal`, `AllowanceWarning`, `AllowanceNotice`, `Admission`, `NoticeRecord`, `NoticeOutcome`,
  `LIFETIME_WINDOW_START`, `crosses_warn_threshold`
- `crates/paladin-core/src/platform/container/treasury_ledger.rs` - `BalanceQuery`
- `crates/paladin-ports/src/input/allowance_admission_port.rs` - `AllowanceAdmissionPort`, `AdmissionError`
- `crates/paladin-ports/src/output/treasury_notice_port.rs` - `TreasuryNoticePort`
- `crates/paladin-ports/src/output/treasury_ledger_port.rs` - `TreasuryLedgerPort::balance` (defaulted)
- `crates/paladin-storage/src/treasury/` and
  `crates/paladin-storage/migrations/{sqlite,postgres}/{010_add_run_schedule_created_by,
  011_create_treasury_notices,012_add_treasury_ledger_tenant_index}.sql` - `balance` and the notice store on
  the in-memory, SQLite and PostgreSQL adapters; the schedule creator columns
- `src/config/treasurer.rs` - `AllowanceConfig`, `AllowanceEntryConfig`, `AllowanceWebhookConfig`, the
  duration and decimal grammar, `validate_against`
- `src/application/services/run/submission.rs` - `RunSubmissionService::{submit, fork}` and the shared
  `admit_and_persist` lifecycle (admit, insert, enqueue, confirm or abandon)
- `crates/paladin-web/src/agent_controller.rs` - `admit_principal` in `execute_agent`,
  `execute_agent_stream` and `enqueue_job`; `crates/paladin-web/src/error.rs` -
  `ApiError::allowance_exhausted`, `with_retry_after`
- `src/application/services/run/worker.rs` - `RunWorkerPool::run_once` / `run_agent` first-dispatch notice
  readback and `AllowanceWarning` emission
- `src/application/services/run/schedule/service.rs` - schedule fire site, `SkipReason::AllowanceExhausted`
- `src/application/services/run/webhook/` - `WebhookDeliveryService::process` operator signing branch,
  `AllowanceWarningPayload`
- `src/infrastructure/web/run_api_wiring.rs` - `build_run_api`, `build_treasury_notices`, the boot SSRF check
- `.planning/phases/41-admission-time-allowance-enforcement/41-CONTEXT.md` - D-01..D-20 and the option-b
  amendment to D-17

## Code Conformance

conforms

Landed in Phase 41 (plans 41-01 through 41-08, 2026-10-03/04). Named proofs by decision:

- **D-01:** `store_clock_selects_the_window_not_the_local_clock`, `instant_at_window_end_starts_a_new_window`,
  `instant_at_window_end_belongs_to_the_next_window`, `retry_after_counts_whole_seconds_to_window_end_from_the_store_clock`
  (`src/application/services/treasurer/`); `balance_window_is_half_open` and
  `balance_counts_a_row_at_window_start_and_excludes_one_at_window_end` (storage).
- **D-03:** `ceilings_are_evaluated_key_window_key_lifetime_tenant_window_tenant_lifetime`,
  `first_exhausted_ceiling_in_order_names_the_refusal`, `refusal_short_circuits_remaining_balance_reads`,
  `tenant_ceiling_sums_every_key_of_the_tenant`, `principal_without_an_entry_never_touches_the_ledger`.
- **D-05:** `balance_exactly_at_the_ceiling_is_refused_and_one_nano_below_is_admitted`,
  `repeated_admission_is_read_only_and_identical`, `concurrent_admissions_write_no_ledger_rows`.
- **D-09:** `admin_principal_is_bound_by_its_allowance` (`src/application/services/run/submission.rs`) and
  the role-free `Treasurer::admit` signature.
- **D-10:** `ledger_failure_fails_closed_for_an_allowanced_principal`,
  `agent_routes_fail_closed_when_the_allowance_check_fails`.
- **D-16:** the ten-clause notice contract (`first_claim_wins_duplicate_is_already_recorded`,
  `tenant_scope_duplicate_dedups`, `lifetime_notice_dedups_per_ceiling`, `raised_ceiling_rearms_the_same_window`,
  `sixteen_concurrent_claims_yield_exactly_one_recorded`) on all three adapters;
  `sixteen_concurrent_crossings_yield_exactly_one_notice`, `abandon_discards_only_this_admissions_notices`,
  `threadbusy_after_a_won_notice_abandons_it_so_the_next_admission_rewins`,
  `notices_store_failure_never_blocks_admission`.
- **D-18 and the end-to-end legs:** `allowance_admission_tracer`, `allowance_warn_path_tracer`,
  `build_run_api_wires_the_allowance_warn_path`,
  `first_dispatch_emits_one_allowance_warning_before_run_started`,
  `operator_delivery_is_signed_with_the_operator_secret_without_a_run_lookup`.

The accepted over-admission race (D-05) and the accepted 2x boundary burst (D-01) are deliberately not
covered by a prevention test; Phase 42 closes the first.

## Downstream Consumers

- **Phase 42 (mid-run enforcement and reservations)** - reuses `window_for`, `TreasuryLedgerPort::balance`
  (a reservation row counts toward the balance automatically, because it reads `amount_nanos`) and the fixed
  ceiling order, and closes the D-05 over-admission race by reserving at the superstep boundary beside the
  admission check; it cites this ADR with ADR-0052 and ADR-0053 rather than re-opening admission.
- **Phase 43 (rate pacing)** - unrelated to allowances; it adds no ceiling and reads none of this model.
- **Phase 46 (docs currency)** - the Treasurer mdBook page (CURR-23) cites this ADR for the window, composition
  and notice semantics; the `v0.10` to `v0.11` migration guide carries MIGRATION.md 9.2, 9.4, 9.5 and 9.6.
