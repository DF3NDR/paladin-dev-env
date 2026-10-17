# Phase 39: Spend Ledger - Context

**Gathered:** 2026-09-27 (`--auto` mode: every gray area was resolved by selecting the
recommended option; no operator answered a question in this session — review before planning
if any decision below is unwelcome)
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 39 delivers the Treasurer's durable spend ledger and nothing else:

1. **`TreasuryLedgerPort`** in `paladin-ports` (sibling of `RunRepositoryPort`) implementing
   ADR-0053's append-only, derive-on-read model: `reserve` / `settle` / `release` rows, `i64`
   nano-unit amounts with an ISO 4217 code, settlement idempotency key
   `(run_id, superstep, attempt)`, superstep-aggregate settlement granularity.
2. **Three adapters** — in-memory, SQLite, Postgres — under `crates/paladin-storage/src/treasury/`,
   backed by a `007` migration in both `crates/paladin-storage/migrations/{sqlite,postgres}/`, all
   passing **one shared contract suite unmodified** (LEDGR-01).
3. **Race-proof reserve-then-settle**: N concurrent draws against a ceiling only N−1 fit → exactly
   N−1 admitted, one refused, on every adapter (LEDGR-02); settlement charged exactly once per
   `(run_id, superstep, attempt)` under lease redelivery, resume, retry and model fallback
   (LEDGR-03).
4. **Queryable spend** (LEDGR-04): a production **settle-only** writer on both run paths so the
   ledger fills in ordinary operation, a `paladin-cli treasury spend` view per tenant / API key /
   run / model over a time window, run cost on `GET /runs/{id}` and the run list derived from the
   ledger, and `PaladinResult.cost` exposed on the HTTP agent response. Herald output and trace
   events already carry cost since Phase 38 (D-10/D-11); this phase adds no new herald field or
   trace variant.

Not in this phase: tenant identity on `Principal` and the `Run` row (Phase 40), allowance
configuration, ceilings and admission refusal (Phase 41 — the ledger takes the ceiling as a
parameter and knows no policy), the mid-run halt and reservation placement at the superstep
boundary (Phase 42), rate pacing (Phase 43), the Treasurer mdBook page (Phase 46 CURR-23), a
hosted spend dashboard or HTTP spend-report endpoint (Out of Scope: FUT-02 territory),
multi-currency FX (FUT-12).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by ADR-0052/0053, Phase 38 and milestone-level decisions — not re-asked)

- **D-00a:** ADR-0053 is implemented as written and **cited, not re-argued**: append-only ledger,
  balance = `SUM` of signed contributions inside the reserving transaction (`reserve` = `+hold`,
  `settle` = `actual − hold` or `actual` when unreserved, `release` = `−hold`); settle/release
  rows are attributed to their reservation's window; amounts are `i64` nano-units carrying the
  currency code; a SUM over mixed currencies is a refusal, never a conversion; settlement key
  `(run_id, superstep, attempt)` with **no `node_id`**; one settlement per superstep attempt
  aggregating every Paladin node dispatched in it; per-node cost stays in `NodeFinished.cost`
  trace events only. Column names, indexes and DDL are this phase's to decide (ADR-0053 §6).
- **D-00b:** ADR-0052 fixes where Phase 42 halts (engine superstep boundary; agent-loop
  `TokenBudget` cutoff). Anything this phase installs at those points is a settle writer only and
  must leave Phase 42's halt hook trivially attachable in the same place.
- **D-00c:** `Cost { nanos: i64, currency: CurrencyCode }` and `CostTally` from Phase 38
  (`crates/paladin-core/src/platform/container/cost.rs`) are the amount type — the ledger never
  introduces a second money type or any `f64`. Display conversion happens once at the edge
  (`nanos as f64 / 1e9`, D-03) and the herald format `0.0450 USD` (D-04) is reused verbatim by
  the CLI table.
- **D-00d:** `Treasurer` is a framework-only word (ADR-0050); Medieval-military vocabulary for
  roles, plain words for units (Phase 30 D-01). The port is `TreasuryLedgerPort`; the storage
  module is `treasury`; no `GarrisonTreasury` anywhere.
- **D-00e:** House storage pattern is mandatory (research: "mirrors `RunRepositoryPort`'s
  three-adapter/contract-test pattern exactly"): feature gates `sqlite` / `postgres`, in-memory
  always on; `sqlx::migrate!("migrations/{sqlite,postgres}")` embedded and applied on
  construction; every statement bound, never formatted; `redact_database_url_password` on
  errors; `storage_timestamp` microsecond truncation on every persisted instant; Postgres suite
  gated on `STORAGE_POSTGRES_TEST_URL` in a module whose path ends in `::postgres` so CI's
  `--lib postgres` filter (ci.yml `postgres-integration` job) picks it up without a workflow edit.
- **D-00f:** X-03 governs public API: this phase is designed to be **additive** (new port, new
  module, new CLI subcommand, additive `#[serde(default)]` DTO fields). Any surface change is
  refreshed with `make api-surface-update` plus a CHANGELOG `[Unreleased]` entry; no MIGRATION
  §9.2 row is expected.
- **D-00g:** Config sub-structs follow `AgentRuntimeConfig` (`Default` + `validate()` +
  `EnvOverridable`, inert when omitted); the ledger's backend selection reuses the existing
  `RunStoreConfig` / `RunStoreBackend` (`src/config/run_store.rs`) rather than a new store config.
- **D-00h:** Shipped tree outranks any document; 82 % coverage floor; `make clean-code`,
  `make api-surface`, `make security` and the manual credential-handling review gate every commit
  (Phase 38 D-00i/D-00j).

### Scope identity before Phase 40

- **D-01:** The `007` schema and the port API are **final now**: every ledger row carries
  `tenant_id TEXT NOT NULL` and `api_key_id TEXT NOT NULL`, and every reserve/settle/query call
  takes a `LedgerScope { tenant_id, api_key_id }` value type. Because `Principal` has no tenant
  and the `Run` row records no submitting principal until Phase 40 (TENANT-01/02), the Phase 39
  production settle writer stamps a documented sentinel scope (`LedgerScope::unattributed()`,
  literal `"unattributed"` for both fields) and Phase 40 replaces only the **source** of the scope
  (the run's recorded principal), never the schema, the port or the queries. Contract tests use
  real scope values. — **Reversibility:** one-way — the `007` migration's columns are persisted;
  reshaping scope later needs a data migration.

### Per-model spend under superstep-aggregate settlement

- **D-02:** One settlement row per `(run_id, superstep, attempt)` stays exactly as ADR-0053 locks
  it; to answer LEDGR-04's "spend per model" without depending on opt-in `run_traces`
  persistence, the settlement row carries an additional **`model_breakdown` JSON column** — a map
  of bare model name → nano-units whose values sum to the row's `amount_nanos` (the empty map for
  reserve/release rows). Balance math reads `amount_nanos` only; the per-model view scans the
  window's settlement rows and folds the breakdowns **in Rust**, never with backend-specific JSON
  SQL functions, so all three adapters behave identically. — **Reversibility:** costly — the
  column is in the persisted schema; dropping it later loses the per-model view.

### Port API shape (policy-free ledger)

- **D-03:** The port knows **no allowance policy**. `reserve` takes the caller's `ceiling` (the
  amount the window may not exceed) and the explicit window bounds `[window_start, window_end)`
  as UTC instants, and admits the hold only if `SUM(window) + hold ≤ ceiling` inside the
  serialized transaction; Phase 41 computes ceilings and windows from allowance config, Phase 39
  never reads such config. A refused reserve is a **typed error variant**
  (`TreasuryLedgerError::Refused { balance, hold, ceiling }`), mirroring
  `RunRepositoryError::ThreadBusy` as an `Err` rather than an `Ok(false)` (X-06).
- **D-04:** Method set: `reserve(ReserveRequest) -> Result<ReservationId, _>`,
  `settle(SettleRequest) -> Result<SettleOutcome, _>` where `SettleOutcome::{Settled,
  AlreadySettled}` (a duplicate key is a **success**, never an error — LEDGR-03),
  `release(ReservationId) -> Result<(), _>` (idempotent; releasing an already-settled or
  already-released reservation is a no-op), `spend(SpendQuery) -> Result<Vec<SpendRow>, _>` for
  LEDGR-04, and `store_now() -> Result<DateTime<Utc>, _>` returning the **store's own clock**
  (`SELECT now()` on Postgres, SQLite's `strftime('%Y-%m-%dT%H:%M:%fZ','now')`, `Utc::now()`
  in-memory) so Phase 41 computes window boundaries from the store clock, never a worker's
  (ALLOW-01, ADR-0053 §2). `settle` accepts `reservation: Option<ReservationId>` (ADR-0053 §2's
  unreserved settle) and the row's currency; a settle whose currency differs from its reservation,
  or a SUM that finds a second currency in scope+window, fails with `CurrencyMismatch`.
- **D-05:** Type homes: `LedgerScope`, `SettlementKey { run_id, superstep, attempt }`,
  `ReservationId`, `LedgerEntryKind`, `SpendRow` and the request/query structs live in
  `paladin-core` (`platform/container/treasury_ledger.rs`, pure, serde-derived, `Debug`/`Clone`/
  `PartialEq`); the trait and its error enum live in
  `crates/paladin-ports/src/output/treasury_ledger_port.rs` with a compiling mock in its rustdoc
  like `run_repository_port.rs`.

### Idempotency and the attempt counter

- **D-06:** Idempotency is **enforced by the store, not by the caller**: a unique index on
  `(run_id, superstep, attempt)` restricted to `kind = 'settle'` (partial unique index on both
  SQLite and Postgres, a `HashSet<SettlementKey>` in-memory) with `INSERT … ON CONFLICT DO
  NOTHING`; zero rows affected maps to `SettleOutcome::AlreadySettled` (the `run_traces
  (thread_id, seq)` / `idx_runs_thread_active` precedent). Reserve rows are not keyed this way —
  two reservations for one superstep attempt are legal (Phase 42's retry after a released hold).
  — **Reversibility:** one-way — the index is part of `007`.
- **D-07 (resolves ADR-0053's named open item):** the engine-path `attempt` component **is the
  persisted `runs.attempt` counter** (`Run.attempt`, D-23) — already bumped by
  `RunRepositoryPort::bump_attempt` on lease redelivery and by `record_resume` on resume, i.e. on
  exactly the events ADR-0053 §4 says must count as a genuine re-execution. The worker passes the
  run's current `attempt` into the settle writer; the engine never invents its own counter.
  `superstep` is the engine's superstep number from the same coordinates `NodeStarted` /
  `NodeFinished` carry. Agent loop: `run_id` = Platform API run id when present else the
  `PaladinExecutionService` execution id, `superstep` = model-call ordinal within the loop,
  `attempt = 1` (ADR-0053 §4, unchanged).

### Production settle writer (what fills the ledger in Phase 39)

- **D-08:** Phase 39 wires a **settle-only** writer (no reservation, no ceiling — those arrive
  with Phases 41/42) so LEDGR-04's operator view shows real spend. Engine path: one `settle` per
  superstep attempt, aggregating that superstep's `NodeFinished.cost` values (and their per-model
  breakdown) and issued **synchronously at the superstep boundary ADR-0052 names** — the same
  place Phase 42 will reserve and halt — never through a `TraceSink`, because the trace channel
  is drop-oldest by contract and a dropped `NodeFinished` would silently lose spend. Agent loop:
  one `settle` per priced model call inside `PaladinExecutionService` after the response is
  priced. A settle failure is **logged at `error` and does not fail the run** in this phase
  (the ledger is observational until Phase 41 makes it authoritative); an unpriced call
  (`cost == None`) writes no row. When no ledger backend is configured (`RunStoreBackend::
  Disabled`) the writer is not installed and nothing changes. — **Reversibility:** costly —
  Phases 41/42 build their reserve and halt on this attachment.

### Query surface (LEDGR-04)

- **D-09:** The CLI reads the store **directly through `RunStoreConfig`**, exactly as
  `paladin-cli run export` opens the trace store (`src/application/cli/commands/run.rs`,
  `try_build_run_trace_store`) — no HTTP round-trip, no server required. New subcommand group
  `paladin-cli treasury` with one verb in this phase, `spend`, taking `--since` / `--until`
  (RFC 3339, default: the last 24 hours ending at the store clock), `--group-by
  tenant|api-key|run|model` (default `tenant`), optional `--tenant` / `--api-key` / `--run`
  filters, and `--format table|json` (table default). Amounts print as four decimals plus the
  currency code (D-04 format); a window that spans two currencies prints one row per currency,
  never a combined figure.
- **D-10:** Run cost on the HTTP surface is **derived from the ledger, not duplicated onto the
  `runs` table**: `RunResponse` gains an additive `#[serde(default)] cost: Option<CostDto>`
  (nanos rendered as the four-decimal string plus currency, plus the raw `nanos`) computed from
  the ledger's settlements for that `run_id`; it is `None` when no ledger backend is configured
  or the run has no settled spend. The agent HTTP execute response exposes `PaladinResult.cost`
  the same way. `RunOutcomeRecord` and migration `002` are untouched.

### Contract suite and migration shape

- **D-11:** `crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres,contract_tests}.rs`
  mirrors `crate::run` one-for-one: one **generic async function per contract clause** taking
  `&dyn TreasuryLedgerPort` (`Arc<dyn …>` for the race test), each adapter invoking every clause
  from its own `#[tokio::test]`s, named per clause. Mandatory clauses: reserve-then-settle
  balance math per ADR-0053 §2 (including unreserved settle and settle-in-next-window attribution),
  `N` concurrent reserves against a ceiling fitting `N−1` → exactly `N−1` `Ok` and one `Refused`
  (LEDGR-02, with `N ≥ 10`), duplicate settle → `AlreadySettled` with the balance unchanged
  (LEDGR-03), release idempotency, currency-mismatch refusal, `spend` grouping per dimension
  over a window, and `store_now` monotonic within a test. Migration files are
  `007_create_treasury_ledger_table.sql` in both directories, table `treasury_ledger`, with the
  header-comment style of `006` (purpose, version, date, why each index exists). Timestamps:
  `TEXT` RFC 3339 on SQLite, `TIMESTAMPTZ` on Postgres; `model_breakdown` `TEXT` on SQLite,
  `JSONB` on Postgres (the `006 record` precedent).
- **D-12:** Serialization per backend, as ADR-0053 §5 prescribes: Postgres takes a
  **transaction-scoped advisory lock keyed on the scope** (`pg_advisory_xact_lock(hashtext(tenant
  || '/' || api_key))`) before the SUM — no per-scope lock table; SQLite opens the reserving
  transaction with **`BEGIN IMMEDIATE`** (sqlx 0.8 `Connection::begin_with`; a deferred
  transaction lets two readers SUM before either writes); in-memory holds one `tokio::sync::Mutex`
  across SUM and insert. Indexes: the partial unique settlement index (D-06) and a covering index
  on `(tenant_id, api_key_id, window_start)` for the balance SUM and the window queries.

### Claude's Discretion

- Exact column list and names beyond those fixed above (e.g. `entry_id` as UUIDv7 text,
  `reservation_id`, `hold_nanos` vs `amount_nanos` + `kind`, `recorded_at`, `window_start`,
  `window_end`, `schema_version`), provided every ADR-0053 quantity is representable and the
  balance is a plain `SUM`.
- How the engine-path writer obtains the superstep's aggregated cost and model breakdown at the
  boundary (from the `TraceDispatcher`'s per-superstep tally, or a small accumulator in the
  superstep loop) — the researcher confirms the exact attachment site in
  `crates/paladin-battalion/src/engine/{superstep,mod}.rs` and how the facade injects an
  `Arc<dyn TreasuryLedgerPort>` without `paladin-battalion` importing storage.
- Whether `spend` grouping for tenant/api-key/run is pushed to SQL `GROUP BY` while `model` is
  folded in Rust, or all four are folded in Rust after one window scan — either is acceptable if
  the contract suite proves identical results on all three adapters.
- `SpendQuery` pagination or a row cap for very large windows; the default `--since` window.
- The `CostDto` field names on the HTTP response and whether `cost` also appears on
  `RunListResponse` items (recommended yes, same derivation, one query per page).
- Test fixture shapes; how the in-memory adapter simulates the store clock.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Locked design (cite, do not re-open)
- `.planning/decisions/0053-ledger-balance-model.md` — the ledger model this phase implements:
  row kinds and signed contributions, nano-unit amounts with currency, settlement key and
  superstep-aggregate granularity, per-backend serialization, the named open item (attempt
  counter source) resolved by D-07 above.
- `.planning/decisions/0052-mid-run-treasurer-enforcement.md` — where the engine path and the
  agent loop can checkpoint; the settle writer (D-08) attaches at the same points.
- `.planning/decisions/PROMOTION.md` — ADR index; next free number 0054 (no new ADR expected
  in this phase; if one becomes necessary it takes 0054 and updates the index in the same commit).
- `.planning/decisions/0049-commissary-design-and-rename.md`,
  `.planning/decisions/0050-treasurer-reservation.md`,
  `.planning/decisions/0051-token-economy-versioning-x03-supersession.md` — vocabulary and X-03.

### Milestone scope and requirements
- `.planning/REQUIREMENTS.md` — LEDGR-01..04 (this phase); TENANT-01/02, ALLOW-01..05 (what the
  port must serve later); Out of Scope table; FUT-12.
- `.planning/ROADMAP.md` — Phase 39 goal and success criteria 1-4; Phase 40/41/42 "Depends on".
- `.planning/PROJECT.md` — *Current Milestone: v0.11.0 Treasurer Spend Governance*.
- `.project/Milestone_14-Treasurer/Epic_1/prd-treasurer-spend-governance.md` — R5 (ledger
  surfaced through heralds, CLI, traces), §4 out of scope.
- `.project/Milestone_14-Treasurer/overview/Milestone-14_Treasurer.md` §3 — the
  `GarrisonTreasury` vocabulary guardrail.

### Prior phase context
- `.planning/phases/38-design-seams-pricing-cost-producer/38-CONTEXT.md` — D-02 (nano-units),
  D-03 (`cost_estimate` stays `f64` at the edge), D-04 (currency rendering), D-09/D-10 (where
  cost travels), D-11 (what was deferred to this phase), D-14/D-15.
- `.planning/phases/38-design-seams-pricing-cost-producer/38-VERIFICATION.md` — what actually
  shipped and where (`cost.rs`, `pricing.rs`, `herald_sink.rs`, `worker.rs::with_herald`).
- `.planning/research/SUMMARY.md` — Phase 39 section, Pitfalls 2/3/4 (double-spend,
  redelivery double-charge, clock skew), stack note that sqlx has no decimal support on SQLite.

### Code the phase extends (read, do not re-derive)
- `crates/paladin-ports/src/output/run_repository_port.rs` — the port shape to mirror: error
  enum style (X-06), rustdoc mock, `bump_attempt` / `record_resume` (the D-07 counter).
- `crates/paladin-storage/src/run/{mod,contract_tests,in_memory,sqlite,postgres}.rs` — the
  adapter/contract-suite house pattern, `storage_timestamp`, `STORAGE_POSTGRES_TEST_URL` gating,
  `MIGRATOR` embedding, `redact_database_url_password`.
- `crates/paladin-storage/migrations/{sqlite,postgres}/006_create_run_traces_table.sql` — the
  migration header style, `ON CONFLICT DO NOTHING` idempotency precedent, TEXT-vs-JSONB and
  TEXT-vs-TIMESTAMPTZ conventions; `002_create_runs_table.sql` — the partial unique index
  precedent and its SQLite `is_unique_violation` caveat.
- `.github/workflows/ci.yml` (`postgres-integration` job) and `docker/docker-compose.test.yml`
  — how `*::postgres` suites run against the live server.
- `crates/paladin-core/src/platform/container/cost.rs` — `Cost`, `CurrencyCode`, `CostTally`.
- `crates/paladin-core/src/platform/container/trace.rs` — `NodeFinished { superstep, attempt,
  cost, … }`, `RunFinished { cost, total_supersteps, … }`.
- `crates/paladin-core/src/platform/container/run.rs` — `Run.attempt` (D-23) and the `Run` row.
- `crates/paladin-battalion/src/engine/{superstep,mod,hooks}.rs` — the superstep boundary
  (`WaypointStatus::Halted` site) and `TraceDispatcher::total_cost` tally.
- `src/application/services/run/worker.rs` — `run_once`, `bump_attempt` call, `with_herald`
  composition (the model for injecting the ledger writer).
- `src/application/services/paladin/paladin_execution_service.rs` — the agent-loop call site
  where `PaladinResult.cost` is produced (agent-loop settle point).
- `src/infrastructure/telemetry/herald_sink.rs` — `HeraldTraceSink` (what NOT to copy for the
  ledger writer: it is a lossy sink; reuse only its wiring shape).
- `src/infrastructure/web/run_api_wiring.rs` and `src/config/run_store.rs` — backend selection
  the ledger adapter reuses.
- `src/application/cli/commands/run.rs` and `src/bin/paladin-cli.rs` — the CLI store-reading
  precedent and the `Commands` enum to extend with `Treasury`.
- `crates/paladin-web/src/run_controller.rs` — `RunResponse`, `RunListResponse` (gain `cost`);
  `crates/paladin-web/src/agent_controller.rs` — the agent execute response.
- `crates/paladin-web/src/agent_auth.rs` — `Principal { id, role }` (no tenant yet; D-01).
- `.project/current-exports.txt`, `CHANGELOG.md` `[Unreleased]` — surface baseline and entries.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crate::run::contract_tests` — per-clause generic async functions over `&dyn Port`, invoked
  unchanged from each adapter's tests; includes a ten-way concurrency clause
  (`ten_concurrent_inserts_one_thread_exactly_one_accepted`) that is the direct template for the
  LEDGR-02 `N−1 of N` race test.
- `storage_timestamp` / `contract_timestamp` — microsecond normalisation so `assert_eq!` stays
  exact on Postgres.
- `sqlx::migrate!` static `MIGRATOR` per adapter, applied on construction; every migration is
  `IF NOT EXISTS` so adapters sharing a database file coexist.
- `Cost` / `CostTally` saturating arithmetic and the `0.0450 USD` renderer (`cost_display`).
- `Run.attempt` + `bump_attempt` / `record_resume` — a persisted, already-correct attempt counter.
- `RunStoreConfig` / `RunStoreBackend` and `try_build_run_trace_store` — config-driven backend
  selection reusable by both the server wiring and the CLI.
- `pool.begin()` transaction precedent in every SQLite/Postgres adapter; `is_unique_violation`
  mapping precedent for constraint-driven outcomes.

### Established Patterns
- Hexagonal boundaries: core (pure types) → ports (trait + error) → storage adapters; the facade
  (`src/`) is the only composition root. `paladin-battalion` must receive the ledger writer as an
  injected port/hook, never import `paladin-storage`.
- Adapters never build SQL from caller strings; every query is a `&'static str` or a
  `QueryBuilder` with `push_bind`.
- Additive serialized fields use `#[serde(default)]` (+ `skip_serializing_if` on trace events).
- Trace sinks are best-effort (drop-oldest, errors are diagnostics) — unsuitable for money.
- Postgres contract suites are discovered by module path (`*::postgres`) in CI.

### Integration Points
- Engine superstep boundary (`engine/superstep.rs` / `engine/mod.rs`) → settle writer →
  `TreasuryLedgerPort` (D-08); `worker.rs::run_once` supplies `run_id`, `attempt`, and the
  configured port.
- `PaladinExecutionService` priced-response site → settle writer (agent loop).
- `run_api_wiring.rs` builds the ledger adapter from `RunStoreConfig` alongside the run repository
  and hands it to `RunWorkerPool` and to the run/agent controllers for the `cost` field.
- `paladin-cli treasury spend` → `RunStoreConfig` → adapter → `spend()`.
- `007` migration files → `MIGRATOR` on both SQL adapters → CI `postgres-integration` job.

</code_context>

<specifics>
## Specific Ideas

- The operator should be able to run `paladin-cli treasury spend --since 2026-09-01T00:00:00Z
  --group-by model` against the same SQLite file the server uses and see one line per model,
  `0.0450 USD` style, with no server running.
- The LEDGR-02 contract clause is the phase's first red test: `N = 16` tasks reserving `hold = 1`
  against `ceiling = 15` on a fresh scope must yield exactly 15 `Ok` and one `Refused`, on all
  three adapters, before any adapter is considered done.
- A redelivered lease that re-runs a superstep already settled must produce a row count of
  exactly one for that `(run_id, superstep, attempt)`, and a resume that bumps `attempt` must
  produce a second, distinct row — both asserted in the contract suite.
- `Refused` errors name the numbers (`balance`, `hold`, `ceiling`) so Phase 41's admission error
  and Phase 42's halt reason can carry them verbatim.

</specifics>

<deferred>
## Deferred Ideas

- **Reservation placement and ceilings in production** — Phase 41 (admission) and Phase 42
  (mid-run); this phase ships the mechanism and a settle-only writer.
- **Real tenant / API-key scope on ledger rows** — Phase 40 records the submitting principal on
  the `Run` row; the `unattributed` sentinel (D-01) is replaced then.
- **HTTP spend-report endpoint / dashboard** — Out of Scope (FUT-02 territory); the CLI is the
  operator surface.
- **Ledger retention / pruning of old rows** — not requested; the index mitigates growth
  (ADR-0053). Revisit if an operator raises table size.
- **`SpendSettled` trace event** — considered for LEDGR-04's "trace events" clause and rejected:
  `RunFinished.cost` / `NodeFinished.cost` already satisfy it and a new variant would be pure
  duplication. Reconsider only if Phase 42's halt needs a ledger-side event.
- **Multi-currency / FX (FUT-12)** — v2; mixed currencies are a typed refusal.

### Reviewed Todos (not folded)
- *Evaluate replacing MinIO with RustFS in the dev/test stack*
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`; `todo.match-phase`
  score 0.6 on the keywords "test, phase, crates, paladin, storage" only). **Auto-mode deviation,
  logged deliberately:** the ≥ 0.4 auto-fold rule was not applied because the match is
  keyword-only — the todo is Phase 45's STORE-01..03 scope (roadmap) and Phase 38 reached the same
  disposition. Folding it here would widen a ledger phase into an object-storage swap.

</deferred>

---

*Phase: 39-spend-ledger*
*Context gathered: 2026-09-27*
