# Phase 39: Spend Ledger - Pattern Map

**Mapped:** 2026-09-27
**Files analyzed:** 15 new/modified
**Analogs found:** 15 / 15 (all house-pattern mirrors; RESEARCH.md already did most of this
legwork with exact line numbers — this file adds file classification + Write-tool-ready excerpts)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|--------------------|------|-----------|-----------------|---------------|
| `crates/paladin-core/src/platform/container/treasury_ledger.rs` | model | CRUD (value types) | `crates/paladin-core/src/platform/container/run.rs` (`RunId`, `RunCursor`, `RunStatus`) | exact |
| `crates/paladin-ports/src/output/treasury_ledger_port.rs` | port (interface) | request-response | `crates/paladin-ports/src/output/run_repository_port.rs` | exact |
| `crates/paladin-storage/src/treasury/mod.rs` | module/config | — | `crates/paladin-storage/src/run/mod.rs` | exact |
| `crates/paladin-storage/src/treasury/in_memory.rs` | service/adapter | CRUD | `crates/paladin-storage/src/run/in_memory.rs` | exact |
| `crates/paladin-storage/src/treasury/sqlite.rs` | service/adapter | CRUD + transactional | `crates/paladin-storage/src/run/sqlite.rs` (+ `run_trace/sqlite.rs` for `ON CONFLICT`) | exact |
| `crates/paladin-storage/src/treasury/postgres.rs` | service/adapter | CRUD + transactional | `crates/paladin-storage/src/run/postgres.rs` | exact |
| `crates/paladin-storage/src/treasury/contract_tests.rs` | test | CRUD (contract suite) | `crates/paladin-storage/src/run/contract_tests.rs` | exact |
| `crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql` | migration | batch DDL | `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql` | exact |
| `crates/paladin-storage/migrations/postgres/007_create_treasury_ledger_table.sql` | migration | batch DDL | `crates/paladin-storage/migrations/postgres/006_create_run_traces_table.sql` | exact |
| `src/application/cli/commands/treasury.rs` | route/controller (CLI) | request-response (file I/O read) | `src/application/cli/commands/run.rs` (`RunExportArgs`, `try_build_run_trace_store`) | exact |
| `src/bin/paladin-cli.rs` (edit: `Commands::Treasury`) | route (CLI dispatch) | request-response | existing `Commands` enum in same file | exact |
| `crates/paladin-battalion/src/engine/superstep.rs` (edit) | service (event-driven hook) | event-driven / streaming | same file's `persist_waypoint` call sites + `completed_records` accumulator | exact (in-file) |
| `src/application/services/paladin/paladin_execution_service.rs` (edit) | service | request-response | same file's `cost_tally.record_call` site (line 1556) | exact (in-file) |
| `src/application/services/run/worker.rs` (edit) | service (orchestrator) | event-driven | same file's `bump_attempt`/`run_once` composition | exact (in-file) |
| `crates/paladin-web/src/run_controller.rs`, `agent_controller.rs` (edit) | controller (HTTP) | request-response | same files' existing `RunResponse`/`ExecuteResponse` `From` impls | exact (in-file) |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/treasury_ledger.rs` (model, CRUD)

**Analog:** `crates/paladin-core/src/platform/container/run.rs` and `cost.rs`

Pure, serde-derived value types, `Debug`/`Clone`/`PartialEq`, no I/O. Follow `run.rs`'s
`RunId`/`RunCursor` newtype style and `cost.rs`'s `Cost { nanos: i64, currency: CurrencyCode }`
(D-00c — never fork this type; import it, do not redefine amount fields as anything but
`Cost`/`i64`). New types needed per D-05: `LedgerScope { tenant_id: String, api_key_id: String }`
(with a `LedgerScope::unattributed()` constructor per D-01), `SettlementKey { run_id, superstep,
attempt }`, `ReservationId` (UUIDv7 newtype, mirrors `RunId`), `LedgerEntryKind` (enum:
`Reserve`/`Settle`/`Release`), `SpendRow`, `ReserveRequest`/`SettleRequest`/`SpendQuery`/
`SettleOutcome`.

---

### `crates/paladin-ports/src/output/treasury_ledger_port.rs` (port, request-response)

**Analog:** `crates/paladin-ports/src/output/run_repository_port.rs`

**Trait shape** (mirror lines 265-369 of the analog, confirmed exact target shape in
RESEARCH.md Code Examples):
```rust
#[async_trait]
pub trait TreasuryLedgerPort: Send + Sync {
    async fn reserve(&self, req: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError>;
    async fn settle(&self, req: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError>;
    async fn release(&self, reservation: ReservationId) -> Result<(), TreasuryLedgerError>;
    async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError>;
    async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError>;
}
```

**Error enum pattern** (`RunRepositoryError`, this exact file, `pub enum RunRepositoryError`):
```rust
pub enum RunRepositoryError {
    #[error("run not found: {run_id}")]
    NotFound { run_id: RunId },
    #[error("illegal run status transition from {from} to {to}")]
    IllegalTransition { from: RunStatus, to: RunStatus },
    #[error("thread busy: {thread_id}")]
    ThreadBusy { thread_id: ThreadId },
    #[error("run repository backend error: {source}")]
    Backend { #[source] source: Box<dyn std::error::Error + Send + Sync> },
    #[error("run serialization error: {message}")]
    Serialization { message: String },
    // ...
}
```
`TreasuryLedgerError` mirrors this exactly: a typed `Refused { balance, hold, ceiling }` variant
(X-06, same style as `ThreadBusy`, never `Ok(false)`), `CurrencyMismatch`, `Backend { source }`,
`Serialization { message }`. Module doc header must include a compiling rustdoc mock exactly like
`run_repository_port.rs`'s own header pattern (module-doc-driven "WHOLE contract" framing, see
the opening `//!` block quoted above from the file's first 60 lines).

---

### `crates/paladin-storage/src/treasury/mod.rs` (module, config)

**Analog:** `crates/paladin-storage/src/run/mod.rs`

```rust
//! Run storage adapters.
//! Implementations of `paladin_ports::output::run_repository_port::RunRepositoryPort`,
//! mirroring `crate::waypoint`'s module layout (D-03).

use chrono::{DateTime, SubsecRound, Utc};

pub mod in_memory;
pub mod contract_tests;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "postgres")]
pub mod postgres;

pub(crate) const STORAGE_TIMESTAMP_SUBSEC_DIGITS: u16 = 6;

pub(crate) fn storage_timestamp(ts: DateTime<Utc>) -> DateTime<Utc> {
    ts.trunc_subsecs(STORAGE_TIMESTAMP_SUBSEC_DIGITS)
}
```
Per RESEARCH.md Assumption A3: `storage_timestamp` is currently `pub(crate)` scoped to
`crate::run`. Either promote it to a shared `crate::storage_util` module both `run` and
`treasury` import, or duplicate the ~3-line function into `treasury/mod.rs` verbatim (a compile
error surfaces immediately if visibility is wrong — low risk either way). Apply it to every
`window_start`/`window_end`/`recorded_at` write, exactly as `run/postgres.rs` applies it to
`submitted_at`/`started_at`/`finished_at`.

---

### `crates/paladin-storage/src/treasury/sqlite.rs` (service/adapter, CRUD + transactional)

**Analog:** `crates/paladin-storage/src/run/sqlite.rs` (adapter shell, `new_shared_file`,
`wrap_error`) + `crates/paladin-storage/src/run_trace/sqlite.rs` (`ON CONFLICT ... DO NOTHING`
idempotency query style) + `crates/paladin-storage/src/waypoint/redact.rs` (credential redaction)

**Reserve transaction (BEGIN IMMEDIATE — NEW mechanic, sqlx-verified in RESEARCH.md Pattern 2):**
```rust
// Verified against sqlx-core-0.8.6/src/pool/mod.rs:389-397 (Pool<DB>::begin_with)
let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| self.wrap_error(e))?;

let balance: i64 = sqlx::query_scalar(
    "SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger \
     WHERE tenant_id = ? AND api_key_id = ? AND window_start >= ? AND window_end <= ? AND currency = ?"
)
.bind(&scope.tenant_id).bind(&scope.api_key_id)
.bind(window_start).bind(window_end).bind(currency.as_str())
.fetch_one(&mut *tx).await.map_err(|e| self.wrap_error(e))?;

if balance + hold > ceiling {
    tx.rollback().await.map_err(|e| self.wrap_error(e))?;
    return Err(TreasuryLedgerError::Refused { balance, hold, ceiling });
}

sqlx::query("INSERT INTO treasury_ledger (...) VALUES (...)")
    .execute(&mut *tx).await.map_err(|e| self.wrap_error(e))?;
tx.commit().await.map_err(|e| self.wrap_error(e))?;
```
**Pitfall (RESEARCH.md Pitfall 2):** keep the transaction body to exactly SUM + admit-check +
one INSERT + commit/rollback — no other `.await` inside, or the write lock stalls every other
writer against that file.

**Settle idempotency query** (mirrors `run_trace/sqlite.rs`'s `APPEND_QUERY` const, adds the
`kind='settle'` partial-index arbiter Pattern 4 requires):
```rust
const SETTLE_QUERY: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, tenant_id, api_key_id, kind, run_id, superstep, attempt, \
       amount_nanos, currency, model_breakdown, recorded_at, window_start, window_end) \
    VALUES (?, ?, ?, 'settle', ?, ?, ?, ?, ?, ?, ?, ?, ?) \
    ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING";

let result = sqlx::query(SETTLE_QUERY) /* .bind(...) x13 */
    .execute(&self.pool).await.map_err(|e| self.wrap_error(e))?;
Ok(if result.rows_affected() == 0 { SettleOutcome::AlreadySettled } else { SettleOutcome::Settled })
```
**Pitfall 3 (arbiter mismatch):** the `WHERE kind = 'settle'` predicate must TEXTUALLY match the
migration's `CREATE UNIQUE INDEX ... WHERE kind = 'settle'` — define as one Rust `const` string
fragment reused in both places, or comment the invariant loudly (see `002_create_runs_table.sql`'s
own precedent for a cross-file string that must agree in four places).

**`store_now()` (SQLite clock, Pattern 5):**
```rust
let now: DateTime<Utc> = sqlx::query_scalar(
    "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')"
).fetch_one(&self.pool).await.map_err(|e| self.wrap_error(e))?;
Ok(now)
```

**Redaction pattern** (`crates/paladin-storage/src/waypoint/redact.rs`, reused verbatim via
`crate::waypoint::redact::redact_database_url_password`):
```rust
/// redact before any truncation — bounding a diagnostic string first can
/// slice a password in half and leak the surviving prefix.
fn extract_url_password(database_url: &str) -> Option<String> { /* url crate + manual fallback */ }
```

**Test-helper for the race clause** (`SqliteRunRepository::new_shared_file`, `run/sqlite.rs:137-157`):
a `#[cfg(test)] async fn new_shared_file` opening a REAL on-disk file with `SqliteJournalMode::Wal`
is required for the LEDGR-02 clause — `sqlite::memory:` cannot exhibit the race.

---

### `crates/paladin-storage/src/treasury/postgres.rs` (service/adapter, CRUD + transactional)

**Analog:** `crates/paladin-storage/src/run/postgres.rs`

**Reserve transaction (advisory lock — NEW mechanic, Pattern 3):**
```rust
let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;
let scope_key = format!("{}/{}", scope.tenant_id, scope.api_key_id);
sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
    .bind(&scope_key).execute(&mut *tx).await.map_err(|e| self.wrap_error(e))?;
let balance: i64 = sqlx::query_scalar(
    "SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger WHERE tenant_id = $1 ..."
).bind(&scope.tenant_id) /* ... */ .fetch_one(&mut *tx).await.map_err(|e| self.wrap_error(e))?;
// same admit-or-refuse logic as sqlite.rs, then commit/rollback.
```
`store_now()`: `SELECT now()` (decodes directly to `chrono::DateTime<Utc>`, same feature set
`run/postgres.rs` already uses for every `TIMESTAMPTZ` column).

**Note (documented limitation, per house convention of naming known gaps loudly — see the
webhook SSRF DNS-rebinding precedent in `security.instructions.md`):** `hashtext()` has a
vanishingly small collision chance across distinct scope strings — accepted tradeoff of ADR-0053
§5's no-lock-table design; document in rustdoc, do not test for it.

---

### `crates/paladin-storage/src/treasury/contract_tests.rs` (test, CRUD contract suite)

**Analog:** `crates/paladin-storage/src/run/contract_tests.rs`

One plain (non-`#[cfg(test)]`) module exporting `pub async fn <clause_name>(port: &dyn
TreasuryLedgerPort)` functions; each adapter's own `#[cfg(test)] mod tests` calls every function
unchanged. Direct template for the LEDGR-02 race clause:
```rust
// Source: crates/paladin-storage/src/run/sqlite.rs:760-864
#[tokio::test(flavor = "multi_thread")]
async fn ten_concurrent_inserts_one_thread_exactly_one_accepted_on_disk() {
    let path = std::env::temp_dir().join(format!("paladin_..._{}.sqlite", uuid::Uuid::new_v4()));
    let url = format!("sqlite://{}", path.display());
    let store: Arc<dyn RunRepositoryPort> =
        Arc::new(SqliteRunRepository::new_shared_file(&url).await.unwrap());
    contract_tests::ten_concurrent_inserts_one_thread_exactly_one_accepted(store).await;
    let _ = std::fs::remove_file(&path);
}
```
For LEDGR-02: scale to `N=16` reserving `hold=1` against `ceiling=15` → exactly 15 `Ok`, 1
`Refused`. Mandatory clauses per D-11: balance math (incl. unreserved settle, settle-in-next-
window attribution), the N-1-of-N race, duplicate settle → `AlreadySettled` with balance
unchanged, release idempotency, currency-mismatch refusal, `spend` grouping per dimension,
`store_now` monotonic.

---

### `crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql` (migration)

**Analog:** `crates/paladin-storage/migrations/sqlite/006_create_run_traces_table.sql`

Header-comment style to copy verbatim (purpose / version / date / why each index exists):
```sql
-- Migration: Create Run Traces Table (SQLite)
-- Purpose: Durable, append-only per-record trace persistence (OBS-02, D-17)
-- Version: 006
-- Date: 2026-09-08
--
-- <prose explaining the schema's rationale, PK choice, index purpose>

CREATE TABLE IF NOT EXISTS run_traces (
    thread_id       TEXT NOT NULL,
    ...
    PRIMARY KEY (thread_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_run_traces_thread_seq ON run_traces(thread_id, seq);
```
For `007`: table `treasury_ledger`, columns per D-11/D-12 (`entry_id`, `tenant_id`, `api_key_id`,
`kind`, `run_id`, `superstep`, `attempt`, `amount_nanos`, `currency`, `model_breakdown` TEXT,
`recorded_at`/`window_start`/`window_end` TEXT RFC3339), the partial unique settlement index
(D-06, Pattern 4) and a covering index on `(tenant_id, api_key_id, window_start)` (D-12). Postgres
twin: `TIMESTAMPTZ` timestamps, `JSONB model_breakdown` (the `006 record` TEXT-vs-JSONB
precedent).

---

### `src/application/cli/commands/treasury.rs` (route/controller, CLI, file I/O read)

**Analog:** `src/application/cli/commands/run.rs`

```rust
#[derive(Debug, clap::Subcommand)]
pub enum RunCommands { /* ... */ }

pub struct RunExportArgs { /* --thread, --format, etc. */ }

async fn try_build_run_trace_store() -> Result<Option<Arc<dyn RunTracePort>>, CliError> {
    // reads RunStoreConfig from env, constructs the configured backend directly,
    // no HTTP round-trip.
}
```
New `TreasuryCommands::Spend` mirrors this shape one-for-one: `try_build_treasury_ledger_store()`
reads `RunStoreConfig` the same way (D-09 explicitly reuses `RunStoreConfig`, no new config
struct per D-00g), then calls `.spend(SpendQuery { since, until, group_by, tenant, api_key, run,
})`. Render with `TableFormatter` (`src/application/cli/formatters/table.rs`, `comfy-table`-backed,
already wired) for `--format table` and `serde_json::to_string_pretty` for `--format json`.
Amount rendering reuses the `0.0450 USD` format (D-00c/D-04) verbatim — Phase 38's `cost_display`
helper.

---

### `crates/paladin-battalion/src/engine/superstep.rs` (edit — settle-writer hook, event-driven)

**Analog:** same file's existing `persist_waypoint` call sites and `completed_records`
accumulator (in-file pattern, no cross-crate analog because this mechanic is genuinely new).

**Confirmed attachment site (RESEARCH.md Pattern 6):** immediately after the third
`persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;` call (~line 3929, the
normal per-superstep checkpoint) and before `superstep_number += 1` (line 3943).

**Anti-pattern — do NOT use `TraceDispatcher::total_cost()`** (`engine/hooks.rs:419-425`): it is
a whole-run cumulative tally, not per-superstep — using it here double-counts every superstep
after the first.

**Correct approach:** a small local accumulator declared alongside `completed_records` (~line
2905), folded at each of the six `completed_records.push(...)` sites (lines 3056/3150/3251/
3322/3342/3362) from the same local `cost` variable already destructured there
(`(paladin_id, usage, cost, outcome/result)` at lines 2681/2705/2754-2758), then read and reset
immediately before each `persist_waypoint` call:
```rust
let mut superstep_cost_total: Option<Cost> = None;
let mut superstep_model_breakdown: HashMap<String, i64> = HashMap::new();
// ... at each completed_records.push(...) site:
if let Some(c) = &cost {
    superstep_cost_total = Some(superstep_cost_total.map_or(c.clone(), |t| t.saturating_add(c)));
    // model_breakdown: open design question (RESEARCH.md Open Question 1) —
    // PaladinResult has no `model` field today; resolve explicitly, do not
    // silently ship an empty/wrong breakdown (Pitfall 5).
}
```
**Injection shape (never import `paladin-storage` into `paladin-battalion`):** mirror
`EngineExecutionPort`'s facade-composed injection (`src/infrastructure/web/facade_provisioner.rs`,
`paladin_port_from_settings`) — add a `settle_writer: Option<&dyn SupersstepSettleWriter>`
parameter to the superstep loop's signature, structurally identical to the existing
`waypoint_port`/`durability`/`trace` `&dyn`/`Arc` parameters already threaded through.
`worker.rs::run_once`'s already-read `run.attempt` (worker.rs:389, bumped via
`self.repository.bump_attempt(&run.run_id)` at worker.rs:853) is the D-07 settlement-key
`attempt` — no new counter.

**Failure handling (D-08, Security Domain table):** a settle failure is logged at `error` and
MUST NOT propagate into the superstep loop's own `Result` — test this explicitly, not just
document it.

---

### `src/application/services/paladin/paladin_execution_service.rs` (edit — agent-loop settle site)

**Analog:** same file's `cost_tally.record_call(response.cost.as_ref());` site (line 1556, in-
file pattern).

```rust
// Confirmed site: paladin_execution_service.rs:1556
cost_tally.record_call(response.cost.as_ref());
// NEW: immediately after —
if let Some(cost) = &response.cost {
    let outcome = treasury_ledger.settle(SettleRequest {
        scope: LedgerScope::unattributed(), // D-01 sentinel until Phase 40
        run_id, // self-generated execution_id today — see identity gap below
        superstep: model_call_ordinal,
        attempt: 1,
        amount: cost.clone(),
        model_breakdown: [(response.model.clone(), cost.nanos)].into(),
        reservation: None,
    }).await;
    if let Err(e) = outcome {
        tracing::error!(error = %e, "treasury settle failed (observational in Phase 39)");
    }
}
```
No model-identity gap here (unlike the engine path) — `response.model` is already in scope.
**Open identity gap (RESEARCH.md Open Question 2):** `execution_id` is a fresh `Uuid::new_v4()`
self-generated at three call sites (`:1073`, `:3061`, `:3206`) with no Platform-API-run-id
parameter threaded in today. Recommended per RESEARCH.md: accept the self-generated id as the
`run_id` for this phase (same "sentinel now, real source later" reasoning as D-01), document it
explicitly rather than half-wiring an unused parameter.

---

### `crates/paladin-web/src/agent_controller.rs` (edit — MUST invert existing test)

**Analog:** the file's own existing `From<PaladinResult> for ExecuteResponse` impl and its
regression test at lines 1908-1939 (quoted in full below — this is the file/lines to edit, not
a pattern to imitate):
```rust
/// D-11, T-38-23: `PaladinResult.cost` is populated (D-10), but
/// `From<PaladinResult> for ExecuteResponse` selects exactly its current
/// fields -- a priced result's serialized `ExecuteResponse` carries no
/// `cost` key. Exposing spend over this unscoped, authenticated HTTP
/// surface is Phase 39 LEDGR-04, not this phase.
#[test]
fn execute_response_carries_no_cost_field() {
    // ... asserts !json.contains("\"cost\"")
}
```
**Action required, not optional (RESEARCH.md Pitfall 6):** D-10 requires exposing
`PaladinResult.cost` on this exact response. This test must be inverted (assert
`json.contains("\"cost\"")` when `PaladinResult.cost.is_some()`) with an updated doc comment
citing Phase 39/D-10, not silently left red or reverted. `RunResponse`/`RunListResponse` in
`run_controller.rs` gain the analogous additive `#[serde(default)] cost: Option<CostDto>` field,
derived from `treasury_ledger.spend()` at request time (D-10 — never persisted on `runs`).

---

## Shared Patterns

### Error enum + `Backend`/`Serialization` boundary conversion
**Source:** `crates/paladin-ports/src/output/run_repository_port.rs` (`RunRepositoryError`)
**Apply to:** `TreasuryLedgerError` in the new port file — same `#[derive(Debug, Error)]`
shape, `#[source]` boxed backend errors, typed refusal variant instead of `Ok(false)` (X-06).

### Credential redaction on every backend error
**Source:** `crates/paladin-storage/src/waypoint/redact.rs` (`redact_database_url_password`)
**Apply to:** `treasury::sqlite`'s and `treasury::postgres`'s `wrap`/`wrap_error` helpers —
redact BEFORE any truncation (security instructions, D-00e).

### Microsecond timestamp truncation before any Postgres bind
**Source:** `crates/paladin-storage/src/run/mod.rs::storage_timestamp`
**Apply to:** every `window_start`/`window_end`/`recorded_at` write in `treasury::sqlite` and
`treasury::postgres` (Pitfall 4).

### Bound parameters only, never string-built SQL
**Source:** `crates/paladin-storage/src/run/sqlite.rs`'s module doc (T-22-17 convention),
mirrored identically in `run_trace/sqlite.rs`'s header.
**Apply to:** all three treasury adapters — every query is a `&'static str` literal with `?`/`$N`
binds, `model_breakdown` serialized via `serde_json::to_string`, never concatenated (Security
Domain V5).

### Partial-unique-index idempotency via `ON CONFLICT ... WHERE ... DO NOTHING`
**Source:** `crates/paladin-storage/src/run_trace/sqlite.rs`'s `APPEND_QUERY` const
**Apply to:** the settle write on both SQL backends, extended with the `kind = 'settle'` arbiter
predicate (must textually match the migration's index `WHERE` clause on both backends — Pitfall
3).

### CLI store-reading via `RunStoreConfig`, no HTTP round-trip
**Source:** `src/application/cli/commands/run.rs::try_build_run_trace_store`
**Apply to:** the new `treasury.rs` CLI command's store construction.

## No Analog Found

None — every file in this phase has a direct, exact in-tree analog (house-pattern phase per
RESEARCH.md's own summary). The two genuinely novel mechanics (per-scope serialized
reserve-then-settle transactions; engine-path per-superstep cost/model accumulation) have no
existing adapter to copy but are fully specified with verified sqlx call shapes in RESEARCH.md
Patterns 2/3/6 (reproduced above) — treat those as the closest available substitute for an
analog.

## Metadata

**Analog search scope:** `crates/paladin-ports/src/output/`, `crates/paladin-storage/src/{run,
run_trace,waypoint,assistant,run_schedule}/`, `crates/paladin-storage/migrations/{sqlite,
postgres}/`, `crates/paladin-core/src/platform/container/`, `crates/paladin-battalion/src/engine/`,
`src/application/{cli/commands,services/run,services/paladin}/`, `crates/paladin-web/src/`
**Files scanned:** ~20 (via RESEARCH.md's prior direct reads plus this session's targeted
excerpt reads)
**Pattern extraction date:** 2026-09-27
**Primary source:** `.planning/phases/39-spend-ledger/39-RESEARCH.md` (exhaustive, line-numbered
codebase citations); this file adds role/data-flow classification and Write-ready excerpts on
top of that research.
