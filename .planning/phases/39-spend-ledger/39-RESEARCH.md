# Phase 39: Spend Ledger - Research

**Researched:** 2026-09-27
**Domain:** Durable, race-proof append-only ledger (Rust/sqlx, hexagonal storage adapters) implementing ADR-0053 on top of Phase 38's cost primitives
**Confidence:** HIGH (every claim below is grounded in a direct read of the shipped tree at this session's HEAD, or the vendored sqlx 0.8.6 source in `~/.cargo/registry/src/`; no web-sourced claim is presented as authoritative without a matching in-tree or vendored-source citation)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward (locked by ADR-0052/0053, Phase 38 and milestone-level decisions — not re-asked):** D-00a (ADR-0053 cited, not re-argued: append-only ledger, balance = SUM of signed contributions, settle/release rows attributed to reservation's window, i64 nano-units + currency, settlement key `(run_id, superstep, attempt)` with no `node_id`, one settlement per superstep attempt, per-node cost stays in `NodeFinished.cost` only); D-00b (ADR-0052 fixes where Phase 42 halts; this phase installs settle writers only, must leave Phase 42's halt hook trivially attachable in the same place); D-00c (`Cost`/`CostTally` from Phase 38 is the amount type, never a second money type or `f64`; display conversion happens once at the edge, herald format `0.0450 USD` reused verbatim by the CLI table); D-00d (`Treasurer` is framework-only; port is `TreasuryLedgerPort`; storage module is `treasury`; no `GarrisonTreasury`); D-00e (house storage pattern mandatory: feature gates `sqlite`/`postgres`, in-memory always on, `sqlx::migrate!` embedded, every statement bound, `redact_database_url_password` on errors, `storage_timestamp` microsecond truncation, Postgres suite gated on `STORAGE_POSTGRES_TEST_URL` in a `::postgres`-suffixed module); D-00f (X-03 additive-only design; `make api-surface-update` + CHANGELOG `[Unreleased]` entry for any surface change; no MIGRATION §9.2 row expected); D-00g (config sub-structs follow `AgentRuntimeConfig`; ledger backend selection reuses existing `RunStoreConfig`/`RunStoreBackend`, no new store config); D-00h (shipped tree outranks any document; 82% coverage floor; `make clean-code`/`make api-surface`/`make security` gate every commit).

**Scope identity before Phase 40:** D-01 — the `007` schema and port API are final now: every ledger row carries `tenant_id TEXT NOT NULL` and `api_key_id TEXT NOT NULL`; every reserve/settle/query call takes a `LedgerScope { tenant_id, api_key_id }`. Phase 39's production settle writer stamps a documented sentinel scope (`LedgerScope::unattributed()`, literal `"unattributed"` for both fields); Phase 40 replaces only the source of the scope, never the schema/port/queries. Contract tests use real scope values. Reversibility: one-way.

**Per-model spend under superstep-aggregate settlement:** D-02 — one settlement row per `(run_id, superstep, attempt)` stays exactly as ADR-0053 locks it; the settlement row carries an additional `model_breakdown` JSON column (bare model name → nano-units, summing to `amount_nanos`, empty map for reserve/release). Balance math reads `amount_nanos` only; the per-model view folds breakdowns in Rust, never backend-specific JSON SQL functions. Reversibility: costly.

**Port API shape (policy-free ledger):** D-03 — the port knows no allowance policy; `reserve` takes the caller's `ceiling` and explicit `[window_start, window_end)` UTC bounds, admits the hold only if `SUM(window) + hold ≤ ceiling` inside the serialized transaction; Phase 41 computes ceilings/windows, Phase 39 never reads such config. A refused reserve is a typed `TreasuryLedgerError::Refused { balance, hold, ceiling }` (X-06), never `Ok(false)`. D-04 — method set: `reserve(ReserveRequest) -> Result<ReservationId, _>`, `settle(SettleRequest) -> Result<SettleOutcome, _>` (`SettleOutcome::{Settled, AlreadySettled}`, duplicate key is success), `release(ReservationId) -> Result<(), _>` (idempotent no-op on already-settled/released), `spend(SpendQuery) -> Result<Vec<SpendRow>, _>`, `store_now() -> Result<DateTime<Utc>, _>` (store's own clock, never a worker's). `settle` accepts `reservation: Option<ReservationId>` (unreserved settle); a currency mismatch fails with `CurrencyMismatch`. D-05 — type homes: `LedgerScope`, `SettlementKey`, `ReservationId`, `LedgerEntryKind`, `SpendRow` and request/query structs live in `paladin-core` (`platform/container/treasury_ledger.rs`); the trait and error enum live in `crates/paladin-ports/src/output/treasury_ledger_port.rs` with a compiling mock in its rustdoc.

**Idempotency and the attempt counter:** D-06 — idempotency enforced by the store, not the caller: a unique index on `(run_id, superstep, attempt)` restricted to `kind = 'settle'` (partial unique on both SQL backends, a `HashSet<SettlementKey>` in-memory) with `INSERT ... ON CONFLICT DO NOTHING`; zero rows affected maps to `AlreadySettled`. Reserve rows are not keyed this way. Reversibility: one-way. D-07 — the engine-path `attempt` component is the persisted `runs.attempt` counter (already bumped by `bump_attempt` on lease redelivery and `record_resume` on resume); the worker passes the run's current `attempt` into the settle writer; the engine never invents its own counter. `superstep` is the engine's superstep number from the same coordinates `NodeStarted`/`NodeFinished` carry. Agent loop: `run_id` = Platform API run id when present else the `PaladinExecutionService` execution id, `superstep` = model-call ordinal, `attempt = 1`.

**Production settle writer:** D-08 — Phase 39 wires a settle-only writer (no reservation, no ceiling). Engine path: one `settle` per superstep attempt, aggregating that superstep's `NodeFinished.cost` values and per-model breakdown, issued synchronously at the superstep boundary ADR-0052 names, never through a `TraceSink` (drop-oldest, would silently lose spend). Agent loop: one `settle` per priced model call inside `PaladinExecutionService` after the response is priced. A settle failure is logged at `error` and does not fail the run in this phase. An unpriced call (`cost == None`) writes no row. When no ledger backend is configured the writer is not installed. Reversibility: costly — Phases 41/42 build their reserve and halt on this attachment.

**Query surface (LEDGR-04):** D-09 — CLI reads the store directly through `RunStoreConfig`, exactly as `paladin-cli run export` (no HTTP round-trip). New subcommand group `paladin-cli treasury` with one verb, `spend`, taking `--since`/`--until` (RFC 3339, default last 24h ending at store clock), `--group-by tenant|api-key|run|model` (default `tenant`), optional `--tenant`/`--api-key`/`--run` filters, `--format table|json` (table default). Amounts print as four decimals + currency code; a window spanning two currencies prints one row per currency, never combined. D-10 — Run cost on the HTTP surface is derived from the ledger, not duplicated onto `runs`: `RunResponse` gains additive `#[serde(default)] cost: Option<CostDto>` computed from the ledger's settlements for that `run_id`; `None` when no ledger backend configured or no settled spend. The agent HTTP execute response exposes `PaladinResult.cost` the same way. `RunOutcomeRecord` and migration `002` are untouched.

**Contract suite and migration shape:** D-11 — `crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres,contract_tests}.rs` mirrors `crate::run` one-for-one: one generic async function per contract clause taking `&dyn TreasuryLedgerPort` (`Arc<dyn …>` for the race test). Mandatory clauses: reserve-then-settle balance math (including unreserved settle and settle-in-next-window attribution), `N` concurrent reserves against a ceiling fitting `N−1` → exactly `N−1` `Ok` and one `Refused` (`N ≥ 10`), duplicate settle → `AlreadySettled` with balance unchanged, release idempotency, currency-mismatch refusal, `spend` grouping per dimension over a window, `store_now` monotonic within a test. Migration files are `007_create_treasury_ledger_table.sql` in both directories, table `treasury_ledger`, `006`-style header comments. Timestamps: `TEXT` RFC 3339 on SQLite, `TIMESTAMPTZ` on Postgres; `model_breakdown` `TEXT` on SQLite, `JSONB` on Postgres. D-12 — Postgres takes a transaction-scoped advisory lock keyed on the scope (`pg_advisory_xact_lock(hashtext(tenant || '/' || api_key))`) before the SUM, no per-scope lock table; SQLite opens the reserving transaction with `BEGIN IMMEDIATE` (sqlx 0.8 `Connection::begin_with`); in-memory holds one `tokio::sync::Mutex` across SUM and insert. Indexes: the partial unique settlement index (D-06) and a covering index on `(tenant_id, api_key_id, window_start)`.

### Claude's Discretion

Exact column list/names beyond fixed quantities (e.g. `entry_id` as UUIDv7, `reservation_id`, `hold_nanos` vs `amount_nanos`+`kind`, `recorded_at`, `window_start`, `window_end`, `schema_version`), provided every ADR-0053 quantity is representable and the balance is a plain SUM. How the engine-path writer obtains the superstep's aggregated cost/model breakdown at the boundary (from `TraceDispatcher`'s per-superstep tally, or a small accumulator in the superstep loop) — **this research resolves this: use a small local accumulator, never `TraceDispatcher::total_cost()`, see Pattern 6 below.** Whether `spend` grouping for tenant/api-key/run is pushed to SQL `GROUP BY` while `model` is folded in Rust, or all four folded in Rust after one window scan. `SpendQuery` pagination/row cap; default `--since` window. `CostDto` field names and whether `cost` also appears on `RunListResponse` items (recommended yes). Test fixture shapes; in-memory clock simulation.

### Deferred Ideas (OUT OF SCOPE)

Reservation placement and ceilings in production (Phase 41 admission, Phase 42 mid-run — this phase ships mechanism + settle-only writer). Real tenant/API-key scope on ledger rows (Phase 40 records the submitting principal; the `unattributed` sentinel is replaced then). HTTP spend-report endpoint/dashboard (Out of Scope, FUT-02 territory; CLI is the operator surface). Ledger retention/pruning of old rows (not requested; index mitigates growth). `SpendSettled` trace event (considered and rejected: `RunFinished.cost`/`NodeFinished.cost` already satisfy LEDGR-04's trace-events clause; a new variant would be pure duplication — reconsider only if Phase 42's halt needs a ledger-side event). Multi-currency/FX (FUT-12, v2; mixed currencies are a typed refusal). The RustFS-for-MinIO todo was reviewed and explicitly not folded into this phase (Phase 45 scope).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LEDGR-01 | A `TreasuryLedgerPort` with in-memory, SQLite and Postgres adapters passes one shared contract-test suite, backed by a `007` migration in both `crates/paladin-storage/migrations/{sqlite,postgres}/` | Architecture Patterns 1/4/5, Recommended Project Structure, Validation Architecture table row LEDGR-01 — full adapter/contract-suite house-pattern mirror of `crate::run` identified with exact files to create |
| LEDGR-02 | Draws reserve then settle atomically; concurrent draws never overspend (N draws race, N−1 fit → exactly N−1 succeed, on every adapter) | Architecture Patterns 2/3 (sqlx-verified `Pool::begin_with("BEGIN IMMEDIATE")` and `pg_advisory_xact_lock` exact call shapes), Pitfall 2, Validation Architecture row LEDGR-02 (on-disk race test precedent, `SqliteTreasuryLedger::new_shared_file`) |
| LEDGR-03 | Settlement is idempotent, keyed on run/superstep/attempt; lease redelivery, resume, retries, model fallback never charge twice | Architecture Pattern 4 (partial unique index + `ON CONFLICT ... WHERE ... DO NOTHING`, cross-backend arbiter-matching caveat), Pitfall 3, Code Examples |
| LEDGR-04 | Operator can view spend per tenant, API key, run and model over a time window from the CLI; spend appears in herald output and trace events (already true since Phase 38 — no new work here) | Architecture Patterns 6/7 (settle-writer attachment sites, the model-identity gap on the engine path), Pattern for CLI wiring (`RunStoreConfig`/`TableFormatter` reuse), Pitfalls 1/5/6, Recommended Project Structure (`treasury.rs` CLI command, `RunApiState` field, DTO edits) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Extracted directives this phase's plan must satisfy (same authority as locked CONTEXT.md decisions):

- **TDD (Red-Green-Refactor):** write the failing test first, per clause of the contract suite — the LEDGR-02 N-1-of-N race clause is named in CONTEXT.md as "the phase's first red test."
- **Coverage floor:** 82% workspace line coverage (ADR-0006), gated by `cargo llvm-cov --fail-under-lines` in CI's `coverage` job — no separate unit/integration target.
- **Dependencies flow inward only:** core → (nothing); ports → core; storage adapters → core + ports. `paladin-battalion` must receive the ledger writer as an injected port/hook and must never import `paladin-storage` (Architecture Pattern 6).
- **Ubiquitous language:** Medieval Military terms for domain roles (`Treasurer` is framework-only per ADR-0050); plain words for units — matches CONTEXT.md D-00d exactly.
- **Before committing a parent task:** `cargo test` → `cargo fmt --check` → `cargo clippy -- -D warnings` → `make api-surface` (refresh with `make api-surface-update` + CHANGELOG entry if the surface changed) → conventional-commit message. Stop after each major task for go-ahead.
- **Security:** `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new/modified code, plus the manual credential-handling review (Security Domain section below) — `codeql.yml` is advisory-only, not a gate.
- **No `unwrap()`/`expect()`/`panic!` in library code** — every fallible path in `paladin-core`/`paladin-ports`/`paladin-storage` returns `Result`; prefer borrowing over cloning; keep iterators lazy.
- **All public APIs need doc tests** — `TreasuryLedgerPort`'s rustdoc mock (D-05) must compile as a doctest, mirroring `run_repository_port.rs`'s existing one exactly.

## Summary

Phase 39 is a **house-pattern phase**: `RunRepositoryPort` (`crates/paladin-ports/src/output/run_repository_port.rs`) and its three adapters (`crates/paladin-storage/src/run/{mod,in_memory,sqlite,postgres,contract_tests}.rs`) are a complete, working template for `TreasuryLedgerPort` — same crate layout, same error-enum shape (X-06), same `sqlx::migrate!` embedding, same `redact_database_url_password` wrapping, same `storage_timestamp` microsecond-truncation contract, same `--lib postgres` CI auto-discovery. The one genuinely new mechanic this phase introduces — and the one with no in-tree precedent — is **per-scope serialized reserve-then-settle**: `Pool::begin_with("BEGIN IMMEDIATE")` on SQLite (confirmed to exist on sqlx 0.8.6 by reading the vendored `sqlx-core-0.8.6` source directly, not by assumption) and `pg_advisory_xact_lock` inside a `pool.begin()` transaction on Postgres (no existing adapter in this codebase calls it — the assistant/run_trace/run_schedule/waypoint adapters that already use `pool.begin()` all do plain sequential writes, never a lock-then-aggregate). Both mechanics are fully specified by ADR-0053 §5 and CONTEXT.md D-12; this research confirms the exact sqlx call shapes so the planner does not have to guess.

The second load-bearing finding is a **gap the codebase does not yet close**: the engine-path settle writer (D-08) needs a per-superstep cost total AND a per-model breakdown at the exact point `superstep.rs` calls `persist_waypoint` (line ~3845/3900/3929), but neither `NodeExecutionRecord` (`crates/paladin-core/src/platform/container/waypoint.rs:563-604`) nor `TraceEvent::NodeFinished` (`crates/paladin-core/src/platform/container/trace.rs:233-262`) nor `PaladinResult` (`crates/paladin-core/src/platform/container/execution_result.rs:51-74`) carries a `model` field — only `cost: Option<Cost>` and `usage: TokenUsage`. Phase 38 threaded `Cost` everywhere `TokenUsage` already went, but never threaded `model` anywhere. The engine-path settle writer can build a per-superstep cost TOTAL for free (accumulate `cost` alongside the existing `completed_records.push(NodeExecutionRecord{...})` calls, a small local accumulator, never `TraceDispatcher::total_cost()` which is a whole-run cumulative figure, not a per-superstep one — see Architecture Patterns below) — but it CANNOT build a non-trivial `model_breakdown` without either (a) an additive `model: Option<String>` field threaded through the same closures `cost` already flows through, or (b) accepting an engine-path `model_breakdown` that only ever has one entry (`"mixed"` or the last-seen model) until a follow-up phase threads model identity through. The agent-loop settle site has no such gap: at `paladin_execution_service.rs:1556` (`cost_tally.record_call(response.cost.as_ref())`), `response.model` (an `LlmResponse` field) is already in scope in the same statement, so `model_breakdown` there is trivial (`{response.model: response.cost.nanos}`).

The third finding is a **pre-existing regression test that names Phase 39 explicitly and must be updated, not just satisfied**: `crates/paladin-web/src/agent_controller.rs:1908-1939`'s `execute_response_carries_no_cost_field` test asserts the HTTP agent execute response's serialized JSON contains no `"cost"` key, with a doc comment reading "Exposing spend over this unscoped, authenticated HTTP surface is Phase 39 LEDGR-04, not this phase." D-10 requires exposing `PaladinResult.cost` on this exact response. The planner must invert or replace this test, not merely add a new one beside it.

**Primary recommendation:** Mirror `crate::run` byte-for-byte for the port/adapter/contract-test/migration shell (Sections below give the exact file list); spend the phase's actual design effort on (1) the SQLite `BEGIN IMMEDIATE` / Postgres advisory-lock reserve transaction (novel, but fully sqlx-verified below), (2) a small per-superstep `Vec<(NodeId, Cost)>` accumulator in `superstep.rs` for the engine-path settle write (never `TraceDispatcher`), and (3) the `execute_response_carries_no_cost_field` test inversion plus the `ExecuteResponse`/`RunResponse`/`RunListResponse` additive DTO fields.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `TreasuryLedgerPort` trait + error enum | API / Backend (ports) | — | Pure interface, mirrors `RunRepositoryPort`; owned by `paladin-ports`, no I/O |
| Domain types (`LedgerScope`, `SettlementKey`, `ReservationId`, `SpendRow`, requests) | API / Backend (core) | — | Pure, serde-derived value types in `paladin-core`, consumed by ports/adapters/facade alike (D-05) |
| In-memory / SQLite / Postgres ledger adapters | Database / Storage | — | `paladin-storage`, feature-gated exactly like `run`/`run_schedule`/`webhook` |
| `007` migration (both dialects) | Database / Storage | — | Schema DDL; SQLite TEXT/JSON vs Postgres TIMESTAMPTZ/JSONB per house convention |
| Engine-path settle writer | API / Backend (facade, `paladin-battalion` boundary) | — | Injected `Arc<dyn TreasuryLedgerPort>`-backed hook at the superstep boundary; `paladin-battalion` never imports `paladin-storage` (hexagonal boundary, ADR-0052) |
| Agent-loop settle writer | API / Backend (facade, `PaladinExecutionService`) | — | Synchronous call after `cost_tally.record_call` inside the reasoning loop |
| `RunResponse`/`RunListResponse`/`ExecuteResponse` `cost` field | API / Backend (`paladin-web` controllers) | — | Derived read from the ledger at request time, never persisted on `runs` (D-10) |
| `paladin-cli treasury spend` | CLI (facade) | Database / Storage (direct read) | Reads the configured store directly via `RunStoreConfig`, no HTTP round-trip (D-09, mirrors `run export`) |
| Herald / trace cost display | Browser-adjacent (CLI/terminal output only; no browser tier in this project) | — | Already wired in Phase 38; this phase adds no new herald field or trace variant (explicit non-goal) |

This project has no browser/SSR/CDN tiers in the traditional web-app sense — it is a Rust backend + CLI + HTTP API framework. The table above maps to this project's actual tiers (core/ports/storage/facade/web/CLI) rather than forcing a browser-app shape.

## Standard Stack

### Core

No new external dependency is required for this phase — every mechanic below is already resolved in `Cargo.lock` at the versions cited.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `sqlx` (workspace) | 0.8.6 `[VERIFIED: Cargo.lock + vendored source]` | SQLite/Postgres adapters, `Pool::begin_with`, migrations | Already the workspace's only SQL layer; `chrono`, `uuid`, `json`, `migrate`, `macros` features already enabled at the workspace level (`Cargo.toml:44`) |
| `chrono` (workspace) | already resolved | `DateTime<Utc>` timestamps, `store_now()` | Used by every existing storage adapter; `SubsecRound::trunc_subsecs` already used by `crate::run::storage_timestamp` |
| `async-trait` (workspace) | already resolved | `TreasuryLedgerPort` trait | Every port in `paladin-ports` uses it |
| `thiserror` (workspace) | already resolved | `TreasuryLedgerError` | Matches `RunRepositoryError`'s derive shape exactly |
| `serde`/`serde_json` (workspace) | already resolved | `model_breakdown` JSON column (SQLite TEXT via `serde_json::to_string`, Postgres JSONB via `::jsonb` cast) | Matches `run_traces.record`'s TEXT-vs-JSONB precedent (`006_create_run_traces_table.sql`) |
| `comfy-table` (workspace, already a direct dep) | resolved at `Cargo.lock:810` `[VERIFIED: Cargo.lock]` | `paladin-cli treasury spend --format table` | Already wired at `src/application/cli/formatters/table.rs`'s `TableFormatter` — reuse it verbatim, do not add a table crate |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `uuid` (workspace) | already resolved | `ReservationId` (mirrors `RunId`'s UUIDv7 convention, see `run.rs`) | If the planner picks a UUIDv7 `entry_id`/`reservation_id` (Claude's Discretion in CONTEXT.md) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `pg_advisory_xact_lock` on Postgres | A per-scope lock row + `SELECT ... FOR UPDATE` | Rejected by ADR-0053 §5 and CONTEXT.md D-12: needs a second table and an upsert; the advisory lock needs no schema at all |
| `BEGIN IMMEDIATE` on SQLite | A deferred `BEGIN` (default `pool.begin()`) | Rejected by ADR-0053 §5: a deferred transaction lets two readers SUM before either writes, defeating the whole point of the serialization |
| `rust_decimal`/`bigdecimal` for the ledger's persisted amount | `i64` nano-units | `sqlx` deliberately has no `rust_decimal`/`bigdecimal` support for SQLite (maintainer decision, cited in `.planning/research/SUMMARY.md`); `i64` is the only representation that works identically on all three backends, and it is already `Cost`'s shape from Phase 38 |

**Installation:** none — no `Cargo.toml` change to any dependency version or feature flag is required. Only the `postgres`/`sqlite` feature gates on the new `treasury` module need to be wired identically to `run`'s (`crates/paladin-storage/Cargo.toml:19-24`, unchanged).

**Version verification:** `sqlx 0.8.6` confirmed via `Cargo.lock:5459-5461` (`sqlx`), `:5472-5475` (`sqlx-core`), `:5592-5595` (`sqlx-postgres`), `:5631-5634` (`sqlx-sqlite`) — all four crates pinned at exactly `0.8.6`. `comfy-table` confirmed present at `Cargo.lock:810`. No package-legitimacy check is needed because zero new external crates enter the dependency graph in this phase (see Package Legitimacy Audit below).

## Package Legitimacy Audit

**Not applicable — zero new external packages are introduced by this phase.** Every crate this phase's code will import (`sqlx`, `chrono`, `async-trait`, `thiserror`, `serde`, `serde_json`, `uuid`, `comfy-table`) is already a resolved workspace or facade dependency, confirmed present in `Cargo.lock` above. `gsd-tools query package-legitimacy check` was not run because there is no candidate package name to check.

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none.

## Architecture Patterns

### System Architecture Diagram

```
                         ┌─────────────────────────────┐
                         │   TreasuryLedgerPort (trait)  │   crates/paladin-ports
                         │ reserve / settle / release /  │
                         │  spend / store_now             │
                         └──────────────┬────────────────┘
                                        │ implemented by
              ┌─────────────────────────┼──────────────────────────┐
              ▼                         ▼                          ▼
   InMemoryTreasuryLedger      SqliteTreasuryLedger        PostgresTreasuryLedger
   (tokio::sync::Mutex          (Pool::begin_with(          (pool.begin() +
    across SUM+insert)           "BEGIN IMMEDIATE"))         pg_advisory_xact_lock
                                                              (hashtext(scope)) + SUM)
              └─────────────────────────┴──────────────────────────┘
                                        │  crates/paladin-storage/src/treasury/
                                        │  007 migration (sqlite + postgres)
                                        ▼
        ┌───────────────────────────────────────────────────────────────┐
        │                     Two production settle writers                │
        └───────────────────────────────────────────────────────────────┘
              ▲                                              ▲
              │ (A) engine path                              │ (B) agent-loop path
   ┌──────────┴────────────────┐                 ┌───────────┴─────────────────┐
   │ superstep.rs boundary      │                 │ paladin_execution_service.rs │
   │ (persist_waypoint call,    │                 │ (after cost_tally.record_    │
   │  ~line 3845/3900/3929);    │                 │  call(response.cost), one    │
   │ per-superstep Cost/model   │                 │  settle per priced model     │
   │ accumulator (NEW — small   │                 │  call; model = response.     │
   │ local Vec, not             │                 │  model already in scope)     │
   │ TraceDispatcher)           │                 └──────────────────────────────┘
   └────────────────────────────┘
              ▲
              │ run_id = runs.attempt-bearing Run (worker.rs::run_once)
   ┌──────────┴────────────────┐
   │ RunWorkerPool::run_once    │  src/application/services/run/worker.rs
   │ (bump_attempt already      │
   │  called on redelivery)     │
   └─────────────────────────────┘

        ┌───────────────────────────────────────────────────────────────┐
        │                          Read side (LEDGR-04)                    │
        └───────────────────────────────────────────────────────────────┘
   paladin-cli treasury spend ──▶ RunStoreConfig (env-driven, same as      ──▶ spend()
        --format table/json         `run export`'s try_build_run_trace_store)
   GET /runs/{id}, GET /runs  ──▶ RunApiState.treasury_ledger (new field,  ──▶ spend()
        (RunResponse.cost)          mirrors run_repository: Option<Arc<..>>)
   POST /agents/{id}/execute  ──▶ ExecuteResponse::from(PaladinResult)     ──▶ result.cost
        (ExecuteResponse.cost)      (no ledger read needed — same-process
                                     value already on PaladinResult)
```

### Recommended Project Structure

```
crates/paladin-core/src/platform/container/
└── treasury_ledger.rs        # LedgerScope, SettlementKey, ReservationId, LedgerEntryKind,
                               # SpendRow, ReserveRequest/SettleRequest/SpendQuery (D-05)

crates/paladin-ports/src/output/
└── treasury_ledger_port.rs   # TreasuryLedgerPort trait + TreasuryLedgerError (D-05),
                               # compiling rustdoc mock mirroring run_repository_port.rs's

crates/paladin-storage/src/treasury/
├── mod.rs                    # module docs, storage_timestamp reuse note, contract_tests re-export
├── in_memory.rs               # tokio::sync::Mutex<...> across SUM+insert
├── sqlite.rs                  # Pool::begin_with("BEGIN IMMEDIATE"), partial unique index
├── postgres.rs                 # pool.begin() + pg_advisory_xact_lock(hashtext($1)::bigint)
└── contract_tests.rs           # one generic async fn per clause, &dyn TreasuryLedgerPort

crates/paladin-storage/migrations/sqlite/
└── 007_create_treasury_ledger_table.sql

crates/paladin-storage/migrations/postgres/
└── 007_create_treasury_ledger_table.sql

src/application/cli/commands/
└── treasury.rs                # TreasuryCommands::Spend, mirrors run.rs's try_build_run_trace_store

src/infrastructure/web/
└── run_api_wiring.rs (edit)   # RunApiState.treasury_ledger: Option<Arc<dyn TreasuryLedgerPort>>
```

### Pattern 1: Contract-suite-per-clause (house pattern)

**What:** One plain (non-`#[cfg(test)]`) module, `contract_tests.rs`, exporting `pub async fn <clause_name>(port: &dyn TreasuryLedgerPort)` functions; each adapter's own `#[cfg(test)] mod tests` calls every function unchanged against a fresh instance.
**When to use:** Every method and race condition this port must guarantee identically across backends (LEDGR-01).
**Example (from the existing `run` suite, to mirror verbatim):**
```rust
// Source: crates/paladin-storage/src/run/sqlite.rs:760-864
#[tokio::test]
async fn insert_then_get_round_trips_every_field() {
    contract_tests::insert_then_get_round_trips_every_field(&fresh_store().await).await;
}

// The ten-way race clause — the DIRECT template for LEDGR-02's N-1-of-N test,
// requiring a REAL shared on-disk SQLite file (not `sqlite::memory:`) to
// prove the invariant holds under true multi-connection concurrency:
#[tokio::test(flavor = "multi_thread")]
async fn ten_concurrent_inserts_one_thread_exactly_one_accepted_on_disk() {
    let path = std::env::temp_dir().join(format!("paladin_..._{}.sqlite", uuid::Uuid::new_v4()));
    let url = format!("sqlite://{}", path.display());
    let store: Arc<dyn RunRepositoryPort> =
        Arc::new(SqliteRunRepository::new_shared_file(&url).await.unwrap());
    contract_tests::ten_concurrent_inserts_one_thread_exactly_one_accepted(store).await;
    let _ = std::fs::remove_file(&path);
    // ...remove -wal/-shm too
}
```
`SqliteRunRepository::new_shared_file` (`crates/paladin-storage/src/run/sqlite.rs:137-157`) uses `SqliteJournalMode::Wal` — the ledger's SQLite adapter needs the identical `#[cfg(test)] async fn new_shared_file` for its own LEDGR-02 race clause, because `sqlite::memory:` with a single connection cannot exhibit the race `BEGIN IMMEDIATE` is meant to serialize against.

### Pattern 2: SQLite `BEGIN IMMEDIATE` reserve transaction (NEW to this codebase — sqlx-verified)

**What:** `SqlitePool` (a type alias for `sqlx::Pool<Sqlite>`) has an inherent method `pub async fn begin_with(&self, statement: impl Into<Cow<'static, str>>) -> Result<Transaction<'static, DB>, Error>` — confirmed by reading `sqlx-core-0.8.6/src/pool/mod.rs:389-397` directly (not assumed from docs). It acquires a pooled connection and immediately runs the given statement instead of the default `BEGIN`.
**When to use:** The reserve path's SUM-then-insert, so no other connection can begin a competing write transaction until this one commits or rolls back (ADR-0053 §5).
**Example:**
```rust
// Verified against sqlx-core-0.8.6/src/pool/mod.rs:389-397 (Pool<DB>::begin_with)
// and sqlx-core-0.8.6/src/connection.rs:59-66 (Connection::begin_with, the
// per-connection twin `Transaction::begin(conn, Some(statement))` calls).
let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| self.wrap_error(e))?;

let balance: i64 = sqlx::query_scalar(
    "SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger \
     WHERE tenant_id = ? AND api_key_id = ? AND window_start >= ? AND window_end <= ? AND currency = ?"
)
.bind(&scope.tenant_id).bind(&scope.api_key_id)
.bind(window_start).bind(window_end).bind(currency.as_str())
.fetch_one(&mut *tx)
.await
.map_err(|e| self.wrap_error(e))?;

if balance + hold > ceiling {
    tx.rollback().await.map_err(|e| self.wrap_error(e))?;
    return Err(TreasuryLedgerError::Refused { balance, hold, ceiling });
}

sqlx::query("INSERT INTO treasury_ledger (...) VALUES (...)")
    .execute(&mut *tx).await.map_err(|e| self.wrap_error(e))?;
tx.commit().await.map_err(|e| self.wrap_error(e))?;
```
**Caveat found during research (cite this in the plan's pitfalls):** `Connection::begin_with`'s own rustdoc (`connection.rs:53-58`) says it "Returns an error if the connection is already in a transaction or if `statement` does not put the connection into a transaction" — this is a plain single-shot call, not reentrant; never call it from inside an already-open transaction. There is also a well-documented community footgun (found via WebSearch, `[CITED: emschwartz.me/psa-write-transactions-are-a-footgun-with-sqlx-and-sqlite]`): starting a `BEGIN IMMEDIATE` transaction and then `.await`ing inside it while holding the connection can starve the pool if the transaction body does further async I/O before committing — keep the transaction body to the SUM + one INSERT + commit, no external calls in between, exactly as the example above does.

### Pattern 3: Postgres transaction-scoped advisory lock

**What:** `pg_advisory_xact_lock(key bigint)` takes an exclusive, session-independent lock that is automatically released at transaction end (commit or rollback) — no unlock call needed, no lock table.
**When to use:** Immediately after `pool.begin()`, before the SUM, keyed on the scope (tenant + api key) so two concurrent reservations for the SAME scope serialize, while reservations for DIFFERENT scopes never block each other.
**Example:**
```rust
// hashtext() returns integer (int4); pg_advisory_xact_lock has overloads for
// (bigint) and (int, int) — casting to bigint explicitly avoids any overload
// ambiguity (int4 -> int8 is an implicit widening cast in Postgres, but an
// explicit cast is clearer and safer to depend on, per PostgreSQL docs
// `[CITED: postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADVISORY-LOCKS]`).
let mut tx = self.pool.begin().await.map_err(|e| self.wrap_error(e))?;

let scope_key = format!("{}/{}", scope.tenant_id, scope.api_key_id);
sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
    .bind(&scope_key)
    .execute(&mut *tx)
    .await
    .map_err(|e| self.wrap_error(e))?;

let balance: i64 = sqlx::query_scalar(
    "SELECT COALESCE(SUM(amount_nanos), 0) FROM treasury_ledger WHERE tenant_id = $1 ..."
)
.bind(&scope.tenant_id) /* ... */
.fetch_one(&mut *tx).await.map_err(|e| self.wrap_error(e))?;
// ... same admit-or-refuse logic as the SQLite example, then commit/rollback.
```
**Note:** `hashtext` has a (vanishingly rare but nonzero) collision chance across different scope strings; this is an accepted tradeoff of the chosen design (no lock table) per ADR-0053 §5 and CONTEXT.md D-12 — the contract suite does not need to test for it, but the rustdoc should note it as a known, deliberate limitation (mirrors the codebase's own convention of documenting known limitations, e.g. the webhook SSRF guard's DNS-rebinding note in `security.instructions.md`).

### Pattern 4: Partial unique index + `ON CONFLICT ... WHERE ... DO NOTHING` (settlement idempotency, D-06)

**What:** A unique index restricted to `kind = 'settle'` rows, with an `INSERT ... ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING` — zero rows affected means `SettleOutcome::AlreadySettled`.
**When to use:** Every settle write, on both SQL backends.
**Verified syntax (Postgres, `[CITED: postgresql.org/docs/current/sql-insert.html`, confirmed via WebSearch]`):** "If your unique index is a partial one, the predicates you added to `CREATE INDEX` must be all provided in the `ON CONFLICT` clause, or the partial index will not be inferred" — i.e. the arbiter's `WHERE` clause must textually match the index's `WHERE` clause.
**SQLite:** SQLite's `UPSERT` grammar accepts the identical `ON CONFLICT (cols) WHERE expr DO NOTHING` conflict-target shape (SQLite has supported partial-index conflict targets since 3.24.0; this workspace's SQLite driver is well past that). The existing `run_traces` precedent (`APPEND_QUERY` in `crates/paladin-storage/src/run_trace/{sqlite,postgres}.rs:33-36` / `:25-28`) uses the simpler full-PK form (`ON CONFLICT(thread_id, seq) DO NOTHING`) since that table's unique constraint is the primary key, not a partial index — the ledger's `kind='settle'` predicate is the one addition this phase introduces beyond that precedent, and it must be tested on both backends in the contract suite (a real, not hypothetical, portability risk: partial-index arbiter matching is one of the few UPSERT areas where SQLite and Postgres diverge in edge-case behavior).
```sql
-- Migration (both dialects, syntax per-backend):
-- Postgres:
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_ledger_settlement
ON treasury_ledger (run_id, superstep, attempt) WHERE kind = 'settle';
-- SQLite: identical syntax, IF NOT EXISTS supported identically.

-- Write:
INSERT INTO treasury_ledger (run_id, superstep, attempt, kind, ...)
VALUES ($1, $2, $3, 'settle', ...)
ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING;
-- rows_affected() == 0  =>  SettleOutcome::AlreadySettled
-- rows_affected() == 1  =>  SettleOutcome::Settled
```

### Pattern 5: Store-clock query (D-04 `store_now()`)

**What:** Each backend answers `store_now()` from its OWN clock, never the calling process's `Utc::now()` (except the in-memory adapter, which has no other clock to defer to).
**Postgres:** `SELECT now()` returns a `TIMESTAMPTZ`, decodes directly to `chrono::DateTime<Utc>` via the workspace's `sqlx`/`chrono` feature (already used for every `TIMESTAMPTZ` column in this codebase, e.g. `run/postgres.rs`'s bound `submitted_at`).
```rust
let now: DateTime<Utc> = sqlx::query_scalar("SELECT now()")
    .fetch_one(&self.pool).await.map_err(|e| self.wrap_error(e))?;
Ok(now)
```
**SQLite:** `strftime('%Y-%m-%dT%H:%M:%fZ','now')` returns TEXT in an RFC3339-shaped format (`%f` gives fractional seconds as `SS.SSS`); `sqlx`'s chrono decode for SQLite TEXT columns already parses this shape identically to how every existing `DateTime<Utc>` column round-trips in `run/sqlite.rs`.
```rust
let now: DateTime<Utc> = sqlx::query_scalar(
    "SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')"
)
.fetch_one(&self.pool).await.map_err(|e| self.wrap_error(e))?;
Ok(now)
```
**Truncation interaction (verified, `crates/paladin-storage/src/run/mod.rs:29-57`):** `storage_timestamp()` truncates to microsecond precision *toward zero* before ANY value is bound into a Postgres `TIMESTAMPTZ` column — apply this to `window_start`/`window_end`/`recorded_at` on write exactly as `run/postgres.rs` already does for `submitted_at`/`started_at`/`finished_at` (see that file's own module doc, lines 17-23). `store_now()`'s return value itself does NOT need `storage_timestamp()` applied — it's a read, not a write; Phase 41 (the eventual caller) is responsible for truncating whatever it derives from `store_now()` before it binds a `window_start`/`window_end` value.

### Pattern 6: Engine-path settle attachment (superstep boundary) — CONFIRMED site, cost aggregation is NEW work

**Confirmed boundary (ADR-0052, verified in source):** `crates/paladin-battalion/src/engine/superstep.rs` calls `persist_waypoint(waypoint_port, durability, &waypoint, trace).await?;` at three sites inside the superstep loop — line 3845 (terminal `Completed`), 3900 (terminal `Failed` on a starved node), and 3929 (the normal per-superstep checkpoint, the one that matters for every superstep that is NOT the run's last). Immediately after line 3929's `persist_waypoint` call and before `superstep_number += 1` (line 3943) is the settle-writer attachment point: the superstep's outcome (Waypoint) is already durable, so a ledger write here settles "real, checkpointed spend," matching ADR-0052's own halt-point rationale (Phase 42 will halt at this exact place).

**What is NOT available at that point (the gap this research surfaces):**
- `TraceDispatcher::total_cost()` (`crates/paladin-battalion/src/engine/hooks.rs:419-425`) is a **whole-run cumulative total**, folded via `CostTally::record_node` inside `emit()` for every `NodeFinished` the dispatcher has ever seen (confirmed: `total_cost_sums_priced_nodes_and_ignores_neutral_ones` test at `hooks.rs:707-726` asserts a sum across MULTIPLE emitted events, with no superstep-scoping). Using it directly for a per-superstep settle would double-count every superstep after the first (it never resets). **Do not use `TraceDispatcher::total_cost()` for the settle writer.**
- `completed_records: Vec<NodeExecutionRecord>` (accumulated per-superstep at `superstep.rs:2905` and pushed to at lines 3056/3150/3251/3322/3342/3362, sorted and finalized at line 3385, consumed by `build_waypoint` at every `persist_waypoint` call site) IS already scoped correctly per-superstep — but `NodeExecutionRecord` (`crates/paladin-core/src/platform/container/waypoint.rs:563-604`) carries `usage: TokenUsage` and NOT `cost: Option<Cost>`. Phase 38 never added `cost` to this type.
- The per-node `cost` value the engine DOES compute per attempt is a local variable at `superstep.rs:2681` / `2705` / `2754-2758` (destructured as `(paladin_id, usage, cost, outcome/result)`), immediately emitted into `TraceEvent::NodeFinished { cost: cost.clone(), ... }` at line 2787-2803. **This local `cost` variable is available at the exact point `completed_records.push(...)` is called** (lines 3056/3150/3251/3322/3342/3362) — the natural fix is a small local accumulator (e.g. `let mut superstep_cost_total: Option<Cost> = None; let mut superstep_model_breakdown: HashMap<String, i64> = HashMap::new();`) declared alongside `completed_records` (line ~2905) and folded at every one of those six push sites, then read (and reset) right before each `persist_waypoint` call.
- **No `model` string is available at any of those six sites today.** `PaladinResult` (the Paladin-node arm's source of `cost`) has no `model` field (`execution_result.rs:51-74`, confirmed by grep: only `usage`/`cost`, no `model`). Building a genuine per-model breakdown on the engine path requires either (a) an additive `model: Option<String>` field on `PaladinResult` (X-03-additive, needs `make api-surface-update` + CHANGELOG entry per D-00f, no MIGRATION §9.2 row expected since additive) threaded from wherever `LlmResponse.model` is available inside `execute_vanguard_node`'s call chain, or (b) accepting that the engine-path `model_breakdown` column is populated with a single best-effort key (e.g. `paladin.model` off the node's static config, if resolvable without a network round-trip) until a later phase closes the gap. **This is an open design question the plan must resolve explicitly — do not silently ship an always-empty or always-wrong `model_breakdown` for the engine path.**

**How the facade injects the ledger port without `paladin-battalion` importing `paladin-storage` (verified precedent, ADR-0052-cited):** `EngineExecutionPort` (`src/infrastructure/web/facade_provisioner.rs`, `paladin_port_from_settings`) is the existing precedent for handing the engine a facade-composed `Arc<dyn PaladinPort>` without the engine crate knowing about HTTP/storage. The settle writer should follow the SAME shape: a small trait (or a plain callback closure captured by the superstep loop's existing `trace`/`heartbeat` parameters) that the facade constructs from `Arc<dyn TreasuryLedgerPort>` and passes down through `WarEngine::start`/`resume*` the same way `waypoint_port`/`durability`/`trace` already flow in as parameters to the superstep loop function (`superstep.rs`'s `run_superstep_loop`-equivalent signature already takes `waypoint_port`, `durability`, `trace` as `&dyn`/`Arc` parameters — a `settle_writer: Option<&dyn SupersstepSettleWriter>` parameter is a structurally identical addition). `worker.rs::run_once`'s `run.attempt` (already read at `worker.rs:389` for the `attempt` param the resume/redelivery machinery already threads, and bumped via `self.repository.bump_attempt(&run.run_id)` at `worker.rs:853` on redelivery) is exactly the `Run.attempt` D-07 names as the settlement key's `attempt` component — no new counter needed, it is already correctly threaded into the engine's execution context today for other purposes.

### Pattern 7: Agent-loop settle attachment — CONFIRMED site, has NO model gap, but HAS an identity gap

**Confirmed site:** `src/application/services/paladin/paladin_execution_service.rs:1556` — `cost_tally.record_call(response.cost.as_ref());` — inside the reasoning loop, immediately after a model call returns. `response` here is an `LlmResponse` (confirmed at the function's earlier `execute_with_retry_and_temperature` call, line 1533-1542), which HAS a `.model` field. **Immediately after this line is the exact settle-writer call site**: `treasury_ledger.settle(SettleRequest { run_id, superstep: <model-call ordinal>, attempt: 1, model_breakdown: {response.model.clone(): response.cost}, ... })`.

**The identity gap (verified, not assumed):** `execution_id` at this call site is `uuid::Uuid::new_v4()`, self-generated fresh inside `PaladinExecutionService::execute`/`execute_stream_inner` (confirmed at `paladin_execution_service.rs:1073`, `:3061`, `:3206` — THREE separate `Uuid::new_v4()` call sites across the sync/streaming code paths, none of which accept an external run id as a parameter today). CONTEXT.md D-07 requires "`run_id` = Platform API run id when present else the `PaladinExecutionService` execution id" — but **no code path today passes a Platform API run id into `PaladinExecutionService`** for it to prefer. The planner must decide: (a) add an optional `platform_run_id: Option<RunId>` parameter/builder-field to `PaladinExecutionService` so callers hosting it behind a Platform API run (the engine path already resolves `PaladinPort` through a DIFFERENT code path — `EngineExecutionPort`/`execute_scoped` — so this is specifically about the agent-loop HTTP route, `agent_host.rs`'s `build_agent`/`build_agent_with_llm`), or (b) settle only ever uses the self-generated `execution_id` in this phase and Phase 40/41 revisit identity once `Run`/`Principal` carry a tenant (this reads as consistent with D-01's "Phase 40 replaces only the source of the scope, never the schema/port/queries" — the SAME reasoning likely applies to the agent-loop `run_id` component of `SettlementKey`, but this is not explicitly stated in CONTEXT.md and should be raised as a plan-time question, not silently assumed).

### Anti-Patterns to Avoid

- **Using `TraceDispatcher::total_cost()` for the engine-path settle write:** it is a whole-run cumulative tally (see Pattern 6) — using it per-superstep double-counts every superstep after the first.
- **Settling through a `TraceSink`:** explicitly rejected by D-08 and the DISCUSSION-LOG's own considered-and-rejected option — the trace channel is drop-oldest by contract (`TraceDispatcher`'s own module doc, `hooks.rs:1-30`), so a dropped `NodeFinished` under load would silently lose real spend. The settle writer must be a synchronous call inside the superstep loop / reasoning loop, never a trace consumer.
- **Persisting cost on the `runs` table:** explicitly rejected by D-10 — `RunResponse.cost` is DERIVED from the ledger at read time, never a second source of truth.
- **A deferred `BEGIN` on SQLite for the reserve transaction:** lets two readers SUM before either writes (ADR-0053 §5) — must be `BEGIN IMMEDIATE` via `Pool::begin_with`.
- **A single-argument `pg_advisory_xact_lock` call without an explicit `::bigint` cast on an ambiguous expression:** while `hashtext()`'s `int4` result implicitly widens to `bigint` for the one-arg overload, an explicit cast removes any need to reason about Postgres's overload-resolution rules under review.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Money serialization/quantization | A new decimal or float money type | `Cost { nanos: i64, currency: CurrencyCode }` from `crates/paladin-core/src/platform/container/cost.rs` (Phase 38) | D-00c locks this; a second money type would fork the arithmetic Phase 38 already tested 17 ways |
| Reserve/settle race safety | An application-level check-then-insert or a new lock table | `BEGIN IMMEDIATE` (SQLite) / `pg_advisory_xact_lock` (Postgres) / `tokio::sync::Mutex` (in-memory), all inside one transaction with the SUM | ADR-0053 §5 already resolved this design question; re-deriving it risks reintroducing the exact TOCTOU race the ADR was written to close |
| Settlement idempotency | A caller-side "have I already settled this?" check | A partial unique index + `ON CONFLICT ... DO NOTHING`, `rows_affected() == 0` => `AlreadySettled` | D-06; store-enforced idempotency is the only version that survives concurrent redelivery (the exact scenario LEDGR-03 exists to close) |
| CLI table/JSON rendering | A new formatting crate or hand-rolled `println!` alignment | `TableFormatter` (`src/application/cli/formatters/table.rs`, already `comfy-table`-backed) + `serde_json::to_string_pretty` for `--format json` | Already the house pattern; a second table crate would be an unreviewed new dependency for no benefit |
| Contract-test infrastructure | A declarative macro-generated suite | Per-clause generic `async fn` mirroring `crate::run::contract_tests` | The `run` suite's own rationale (named in D-11/CONTEXT.md) is that a failing clause names itself in the test output, not a generated line number |

**Key insight:** Every "Don't Hand-Roll" row above already has a working, tested implementation somewhere in this exact codebase from an earlier phase. The discipline this phase needs is restraint — copy the shape, do not "improve" it, and reserve real design effort for the two genuinely new mechanics (per-scope transaction serialization, engine-path per-superstep cost aggregation).

## Common Pitfalls

### Pitfall 1: Using `TraceDispatcher::total_cost()` as the engine settle source (cumulative, not per-superstep)
**What goes wrong:** Every superstep after the first double-counts (or worse) all prior supersteps' spend, because `total_cost()` folds every `NodeFinished` the dispatcher has ever seen (`hooks.rs:419-425`, confirmed cumulative by its own test at `:707-726`).
**Why it happens:** `total_cost()` looks like exactly the right helper (it exists, it's tested, it's literally about cost) — but it answers "total for the run so far," not "total for this superstep."
**How to avoid:** A local accumulator scoped to the superstep loop iteration, reset every time `persist_waypoint` is called, folded from the same local `cost` variable already available at each `completed_records.push(...)` site (Pattern 6).
**Warning signs:** LEDGR-04's CLI `spend --group-by run` total for a multi-superstep run is larger than the run's own `RunFinished.cost` — a dead giveaway of double-counting.

### Pitfall 2: SQLite `BEGIN IMMEDIATE` held across an `.await` that does unrelated I/O
**What goes wrong:** `BEGIN IMMEDIATE` takes the write lock immediately; if the transaction body awaits something slow before committing (a network call, another lock), every other writer against that SQLite file blocks for the duration — pool exhaustion under load.
**Why it happens:** It is tempting to fold extra logic (e.g. an audit log write, a metrics increment) into the same transaction "since we're already in one."
**How to avoid:** Keep the transaction body to exactly SUM + admit-check + one INSERT + commit/rollback, matching Pattern 2's example precisely; do all other work before beginning or after committing.
**Warning signs:** SQLite adapter tests pass individually but the CI `--test-threads=1` run (or the LEDGR-02 concurrency test itself) shows unexplained slowness or timeouts.

### Pitfall 3: Partial-unique-index `ON CONFLICT` arbiter mismatch between the migration and the write query
**What goes wrong:** If the `WHERE kind = 'settle'` predicate on the `INSERT ... ON CONFLICT` statement does not TEXTUALLY match the migration's `CREATE UNIQUE INDEX ... WHERE kind = 'settle'`, Postgres refuses to infer the partial index as the conflict target and raises `42P10: there is no unique or exclusion constraint matching the ON CONFLICT specification` at runtime, not at migration time.
**Why it happens:** The predicate is written twice (once in DDL, once in every settle-write query) with no compiler check that they agree.
**How to avoid:** Define the predicate string as a single Rust `const` and use it (or a very tightly commented mirror, per this codebase's existing convention of commenting cross-file invariants — see `002_create_runs_table.sql`'s own header note about `idx_runs_thread_active`'s busy-set string appearing in four places) in both the migration file's header comment and the settle query.
**Warning signs:** The SQLite contract-test clause for duplicate settle passes (SQLite is more forgiving here) while the equivalent Postgres clause fails with `42P10` — a real, not hypothetical, cross-backend divergence risk this exact pattern introduces (Pattern 4).

### Pitfall 4: Forgetting `storage_timestamp` truncation on `window_start`/`window_end`/`recorded_at` writes
**What goes wrong:** A `DateTime<Utc>` with nanosecond precision bound into a Postgres `TIMESTAMPTZ` column round-trips with the extra digits silently dropped by the driver/server in a way this codebase does not control — an `assert_eq!` in a contract test comparing an inserted timestamp to a read-back one fails intermittently depending on which backend ran it.
**Why it happens:** `Utc::now()` typically carries nanosecond precision; SQLite/in-memory preserve it, Postgres truncates it — a difference this codebase already solved once (`crate::run::storage_timestamp`) but a new module must remember to reuse.
**How to avoid:** Call `crate::run::storage_timestamp` (or move it to a shared location both `run` and `treasury` import — check whether it should be promoted out of `run/mod.rs` into a crate-level `storage_util` module rather than duplicated) on every timestamp before binding it.
**Warning signs:** A contract test that passes on SQLite/in-memory but fails only on the Postgres CI job with a sub-microsecond timestamp mismatch.

### Pitfall 5: Shipping the engine-path `model_breakdown` without resolving the model-identity gap
**What goes wrong:** LEDGR-04's "spend per model" view silently reports an empty, `"unknown"`, or wrong-but-plausible model key for every engine-path run, while the agent-loop path reports correctly — an inconsistency that will not be caught by a contract test scoped to the ledger port alone (the port itself has no opinion on what the caller puts in `model_breakdown`; the bug is entirely in the ENGINE-PATH CALLER, not the port).
**Why it happens:** `PaladinResult`/`NodeExecutionRecord`/`TraceEvent::NodeFinished` genuinely do not carry a model string today (verified above, Pattern 6) — it is easy to either skip the field or fill it with a placeholder and move on, since nothing forces the question.
**How to avoid:** Make this an explicit plan-time decision (add `model: Option<String>` to `PaladinResult`, additive per D-00f, OR document the engine-path limitation loudly in rustdoc and the phase's SUMMARY), not a silent implementation detail.
**Warning signs:** A UAT/manual verification step that runs an engine-driven multi-model graph and checks `paladin-cli treasury spend --group-by model` for a sane per-model split — if this isn't in the plan's verification steps, the gap will ship unnoticed.

### Pitfall 6: The `execute_response_carries_no_cost_field` test blocks the D-10 change
**What goes wrong:** The plan adds a `cost` field to `ExecuteResponse` and wires `From<PaladinResult>`, but the pre-existing test at `agent_controller.rs:1908-1939` still asserts `!json.contains("\"cost\"")` — CI goes red, or worse, someone "fixes" it by reverting the D-10 change instead of updating the intentionally-named test.
**Why it happens:** The test is real, passes today, and its own doc comment explicitly defers the behavior to "Phase 39 LEDGR-04" — it is easy to miss during a broad `cargo test` pass if the planner doesn't grep for it first.
**How to avoid:** The plan's task list must explicitly name this test file and either invert its assertion or replace it with a new one proving the OPPOSITE (`json.contains("\"cost\"")` when `PaladinResult.cost.is_some()`), and must NOT remove the surrounding doc comment's historical context without replacing it with the new phase's own rationale.
**Warning signs:** none needed — `cargo test -p paladin-web` will fail loudly and immediately if this is missed; the risk is a plan that doesn't anticipate it and burns a cycle.

## Code Examples

### `TreasuryLedgerPort` trait shape (mirrors `RunRepositoryPort` exactly)

```rust
// Source: crates/paladin-ports/src/output/run_repository_port.rs:265-369 (pattern to mirror)
#[async_trait]
pub trait TreasuryLedgerPort: Send + Sync {
    async fn reserve(&self, req: ReserveRequest) -> Result<ReservationId, TreasuryLedgerError>;
    async fn settle(&self, req: SettleRequest) -> Result<SettleOutcome, TreasuryLedgerError>;
    async fn release(&self, reservation: ReservationId) -> Result<(), TreasuryLedgerError>;
    async fn spend(&self, query: SpendQuery) -> Result<Vec<SpendRow>, TreasuryLedgerError>;
    async fn store_now(&self) -> Result<DateTime<Utc>, TreasuryLedgerError>;
}
```

### Settle-write with partial-unique-index idempotency (SQLite)

```rust
// Pattern combines: crates/paladin-storage/src/run_trace/sqlite.rs's APPEND_QUERY shape
// (ON CONFLICT ... DO NOTHING) with a WHERE-scoped arbiter (Pattern 4 above).
const SETTLE_QUERY: &str = "\
    INSERT INTO treasury_ledger \
      (entry_id, tenant_id, api_key_id, kind, run_id, superstep, attempt, \
       amount_nanos, currency, model_breakdown, recorded_at, window_start, window_end) \
    VALUES (?, ?, ?, 'settle', ?, ?, ?, ?, ?, ?, ?, ?, ?) \
    ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING";

let result = sqlx::query(SETTLE_QUERY) /* .bind(...) x13 */
    .execute(&self.pool).await.map_err(|e| self.wrap_error(e))?;

Ok(if result.rows_affected() == 0 {
    SettleOutcome::AlreadySettled
} else {
    SettleOutcome::Settled
})
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `ExecutionMetadata.cost_estimate` reserved, no producer | Real producer on both run paths, `Option<f64>` at the display edge only | Phase 38 (2026-09-26) | The ledger's `i64` nano-unit `Cost` is authoritative; `f64` is display-only, never fed back |
| No cost field on `PaladinResult`/`TraceEvent` | Additive `cost: Option<Cost>` on `LlmResponse`, `PaladinResult`, `NodeFinished`, `RunFinished` | Phase 38 | This phase's engine-path settle writer builds directly on these fields — but NOT on `NodeExecutionRecord`, which Phase 38 did not touch |

**Deprecated/outdated:** none relevant — this is new-construction work, not a migration off an old pattern.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SQLite's `ON CONFLICT (cols) WHERE expr DO NOTHING` accepts a partial-index arbiter identically to Postgres (both confirmed independently via WebSearch/Postgres docs and SQLite's documented UPSERT grammar since 3.24.0; not independently executed against this workspace's exact SQLite version in this research session) | Pattern 4 | If SQLite's driver version has any edge-case divergence, the LEDGR-03 duplicate-settle contract clause could pass differently on the two backends — mitigated because the plan's contract suite runs this exact clause on BOTH backends before considering the phase done |
| A2 | `hashtext()`'s `int4` result implicitly widens to the `bigint`-arg overload of `pg_advisory_xact_lock` without an explicit cast erroring (recommended to cast explicitly regardless, per the pattern above) | Pattern 3 | Low risk — the pattern recommends the explicit `::bigint` cast specifically to avoid depending on this implicit behavior at all |
| A3 | `storage_timestamp` (currently `pub(crate)` inside `crate::run::mod`) is either reused via `crate::run::storage_timestamp` (if visibility allows) or duplicated/promoted for `crate::treasury` — the exact refactor (promote vs. duplicate) is left to the planner | Pitfall 4 | If duplicated without noticing the module is `pub(crate)`-scoped to `run`, a compile error surfaces immediately (not a silent risk) — the only real risk is spending planning time deciding promote-vs-duplicate, not a functional one |

**If this table is empty:** N/A — three low-risk assumptions above, all self-correcting via compile errors or contract-test failures rather than silent behavior differences.

## Open Questions

1. **Where does the engine-path `model_breakdown` come from, given `PaladinResult` has no `model` field?**
   - What we know: The per-node `cost` IS available at every `completed_records.push(...)` site in `superstep.rs`; the model string is NOT, anywhere in that call chain, today.
   - What's unclear: Whether the plan should add `model: Option<String>` to `PaladinResult` (additive, X-03-safe) this phase, or explicitly scope the engine-path `model_breakdown` as best-effort/incomplete and note it in the phase's known-gaps register.
   - Recommendation: Raise this explicitly with the plan's D-nn decision list before writing tasks; do not let it surface only during implementation. Given the coupling to Phase 38's already-established "cost rides beside usage everywhere usage travels" precedent (D-10), extending that precedent to `model` is the more consistent choice — but it is a real scope decision, not a free extension.

2. **Where does the agent-loop `run_id` come from when hosted behind the Platform API?**
   - What we know: `PaladinExecutionService` self-generates a fresh `Uuid::new_v4()` as `execution_id` on every call today; no Platform-API-run-id parameter exists on the service.
   - What's unclear: Whether Phase 39 should add such a parameter (and if so, where the caller — `agent_host.rs` — would source a real Platform API run id from, since the agent HTTP route is not obviously a Platform API "run" today) or whether the self-generated id is accepted as-is for this phase, matching the D-01 "sentinel now, real source later" pattern used for `LedgerScope`.
   - Recommendation: Treat this the same way D-01 treats tenant scope — document a stated, deliberate simplification for Phase 39 (self-generated `execution_id` stands in for `run_id` on the agent-loop path) rather than silently building a half-wired parameter no caller populates.

## Environment Availability

No external service dependency beyond what the existing `run`/`run_schedule`/`webhook` adapters already require:

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| SQLite (embedded, via sqlx) | `treasury::sqlite` adapter, always-on tier | ✓ | bundled via `sqlx/sqlite` feature | — |
| PostgreSQL (Docker, `docker/docker-compose.test.yml`) | `treasury::postgres` adapter contract suite (Tier 2, CI-gated) | ✓ (CI); local availability depends on Docker | matches `postgres-test` service already used by `run::postgres` | Suite skips cleanly when `STORAGE_POSTGRES_TEST_URL` is unset (existing `run::postgres` skip-path precedent) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** Postgres locally — the existing `run::postgres` suite's skip-on-unset-env-var behavior is the precedent to mirror exactly; do not invent a different skip mechanism.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Tokio async tests, `#[tokio::test]`), workspace-standard |
| Config file | none — the pattern is per-crate `#[cfg(test)] mod tests` + a shared `contract_tests.rs` module, no external test-framework config |
| Quick run command | `cargo test -p paladin-storage --lib treasury::` |
| Full suite command | `cargo test -p paladin-storage --features "sqlite postgres"` (in-memory + SQLite; Postgres suite requires `STORAGE_POSTGRES_TEST_URL`, see CI job below) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LEDGR-01 | One shared contract suite passes unmodified against in-memory/SQLite/Postgres | integration (contract) | `cargo test -p paladin-storage --lib treasury::` (in-memory+sqlite); `STORAGE_POSTGRES_TEST_URL=... cargo test -p paladin-storage --features postgres --lib postgres` (postgres, `--lib postgres` substring filter, CI-only by default) | ❌ Wave 0 — `crates/paladin-storage/src/treasury/contract_tests.rs` does not exist yet |
| LEDGR-02 | N=16 concurrent reserves against ceiling=15 → exactly 15 Ok, 1 Refused, on every adapter | integration (concurrency, `#[tokio::test(flavor = "multi_thread")]`) | `cargo test -p paladin-storage --lib treasury::sqlite::ten_concurrent... -- --test-threads=1` (name TBD by planner; mirrors `ten_concurrent_inserts_one_thread_exactly_one_accepted_on_disk`) | ❌ Wave 0 — needs `SqliteTreasuryLedger::new_shared_file` test helper mirroring `run/sqlite.rs:137-157` |
| LEDGR-03 | Duplicate settle (lease redelivery, resume, retry, fallback) charges exactly once | unit + integration (contract clause) | `cargo test -p paladin-storage --lib treasury::` (duplicate-settle clause, all three adapters) | ❌ Wave 0 |
| LEDGR-04 | CLI spend view; herald/trace already carry cost (no new work here per D-08's own scope note) | CLI smoke + unit | `cargo run --bin paladin-cli -- treasury spend --since <ts> --group-by model` (manual/smoke); `cargo test -p paladin-ai --lib application::cli::commands::treasury` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p paladin-storage --lib treasury::` (fast, in-memory + sqlite, no Docker)
- **Per wave merge:** `cargo test -p paladin-storage --features "sqlite postgres"` plus, if Docker available, the full `STORAGE_POSTGRES_TEST_URL`-gated Postgres run
- **Phase gate:** Full suite green (`cargo test --workspace` targeted crates: `paladin-storage`, `paladin-ports`, `paladin-core`, `paladin-battalion`, `paladin-web`, `paladin-ai`) before `/gsd-verify-work`; CI's `postgres-integration` job (`.github/workflows/ci.yml:954-1010`) picks up `treasury::postgres` automatically via its `--lib postgres` substring filter — **no CI workflow edit required**, confirmed by reading the job's own comment (lines 995-999) stating exactly this auto-discovery contract.

### Wave 0 Gaps
- [ ] `crates/paladin-core/src/platform/container/treasury_ledger.rs` — domain types (D-05)
- [ ] `crates/paladin-ports/src/output/treasury_ledger_port.rs` — port trait + error enum
- [ ] `crates/paladin-storage/src/treasury/{mod,in_memory,sqlite,postgres,contract_tests}.rs` — adapters + suite
- [ ] `crates/paladin-storage/migrations/{sqlite,postgres}/007_create_treasury_ledger_table.sql`
- [ ] `SqliteTreasuryLedger::new_shared_file` test helper (mirrors `run/sqlite.rs:137-157`) for the LEDGR-02 on-disk race test
- [ ] `src/application/cli/commands/treasury.rs` + `Commands::Treasury` wiring in `src/bin/paladin-cli.rs`
- [ ] Updated/inverted `execute_response_carries_no_cost_field` test at `crates/paladin-web/src/agent_controller.rs:1908-1939`

*(Framework itself needs no install — `cargo test`/`sqlx::migrate!`/`tokio::test` are already fully set up workspace-wide.)*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (this phase adds no new auth surface; `paladin-cli treasury spend` reads local storage directly, `GET /runs*` already authenticates via existing `Principal` extraction) | — |
| V3 Session Management | no | — |
| V4 Access Control | yes — CLI `treasury spend` runs with the operator's OS-level file access to the configured store, matching `run export`'s existing precedent exactly; the HTTP-surfaced `cost` field on `RunResponse`/`ExecuteResponse` inherits whatever authorization already gates `GET /runs/{id}`/`POST /agents/{id}/execute` — this phase adds no NEW access-control decision, it rides the existing one | Existing `Principal`/role check on the controllers this phase edits (no new gate needed, no new gate should be invented) |
| V5 Input Validation | yes | Every SQL statement is a `&'static str` literal or `sqlx::QueryBuilder` with `push_bind` — never string-formatted from caller input, per the existing house convention (`run/sqlite.rs:36-42`'s own module doc states this explicitly and every adapter in this phase must state and prove the same) |
| V6 Cryptography | no | No new cryptographic operation in this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL injection via a caller-controlled scope/model string landing in `model_breakdown` or a `LIKE`/dynamic filter | Tampering | Bound parameters only (`push_bind`/`?`/`$N`), exactly as every existing adapter in this codebase does; `model_breakdown`'s JSON is built via `serde_json::to_string`, never string-concatenated into SQL |
| Double-spend via a check-then-act race between concurrent workers | Tampering / Repudiation of the balance invariant | `BEGIN IMMEDIATE` (SQLite) / `pg_advisory_xact_lock` (Postgres) / `tokio::sync::Mutex` (in-memory) — this IS the phase's central security property, not an add-on (LEDGR-02) |
| A settle write failure silently corrupting or halting a run | Denial of Service (of the run itself, not the ledger) | D-08 explicitly requires settle failures to be logged at `error` and NEVER fail the run in this phase (the ledger is observational until Phase 41) — the planner must make this a tested behavior (a settle-writer error must not propagate into the superstep loop's own `Result`), not just a documented intention |
| Database URL leaking a password into an error message or log line | Information Disclosure | `redact_database_url_password` (`crates/paladin-storage/src/waypoint/redact.rs`, already reused by every existing adapter's `wrap`/`wrap_error` helper) — apply identically in `treasury::sqlite`/`treasury::postgres`; redact BEFORE any truncation, per `security.instructions.md`'s stated rule |
| Money amounts or scope identifiers appearing in logs beyond what's needed for diagnosis | Information Disclosure (mild — spend amounts are operational data, not secrets, but tenant/API-key identifiers are still principal-adjacent) | Log amounts (nanos, currency) freely for diagnosis (this is the ledger's whole purpose); avoid logging a FULL `LedgerScope` in a way that could be mistaken for an authentication credential — `tenant_id`/`api_key_id` here are opaque labels, not the raw API key value itself (confirm this distinction holds once Phase 40 wires real values in) |

No SSRF, XSS, CSRF, or session-management surface is introduced by this phase — it is a pure storage-and-arithmetic phase with one new CLI subcommand and three additive HTTP response fields.

## Sources

### Primary (HIGH confidence — direct read of the shipped tree or vendored source this session)
- `crates/paladin-ports/src/output/run_repository_port.rs` — port/error/mock shape to mirror
- `crates/paladin-storage/src/run/{mod,sqlite,postgres,in_memory,contract_tests}.rs` — adapter house pattern, `storage_timestamp`, redaction, migration embedding
- `crates/paladin-storage/src/run_trace/{sqlite,postgres}.rs` — `ON CONFLICT ... DO NOTHING` idempotency precedent
- `crates/paladin-storage/src/assistant/{sqlite,postgres}.rs`, `run_schedule/*`, `waypoint/sqlite.rs` — every existing `pool.begin()` transaction precedent in this codebase
- `crates/paladin-storage/migrations/{sqlite,postgres}/{002,006}_*.sql` — DDL/header/partial-index conventions
- `crates/paladin-core/src/platform/container/{cost,trace,execution_result,waypoint}.rs` — `Cost`, `NodeFinished`, `PaladinResult`, `NodeExecutionRecord` field shapes (confirmed absence of `model` field)
- `crates/paladin-battalion/src/engine/{superstep,hooks}.rs` — superstep-boundary `persist_waypoint` call sites, `completed_records` accumulation, `TraceDispatcher::total_cost` cumulative behavior
- `src/application/services/paladin/paladin_execution_service.rs` — agent-loop settle site, `execution_id` self-generation (3 call sites)
- `src/application/services/run/worker.rs` — `run_once`, `bump_attempt`, `run.attempt` plumbing
- `crates/paladin-web/src/{run_controller,agent_controller,agent_auth}.rs` — `RunResponse`, `ExecuteResponse` (and its Phase-39-naming regression test), `Principal`
- `src/config/run_store.rs`, `src/application/cli/commands/run.rs`, `src/bin/paladin-cli.rs`, `src/application/cli/formatters/table.rs` — CLI wiring precedent
- `src/infrastructure/web/run_api_wiring.rs` — `RunApiState` composition
- `Cargo.lock` (sqlx/sqlx-core/sqlx-postgres/sqlx-sqlite @ 0.8.6, comfy-table) and workspace `Cargo.toml:44` (sqlx feature set)
- `~/.cargo/registry/src/index.crates.io-.../sqlx-core-0.8.6/src/{pool/mod.rs,connection.rs,transaction.rs}` — verified `Pool::begin_with`/`Connection::begin_with` exist and their exact signatures, not assumed from documentation
- `.github/workflows/ci.yml:954-1010` — `postgres-integration` job, `--lib postgres` auto-discovery contract, `STORAGE_POSTGRES_TEST_URL`
- `.planning/decisions/{0052,0053}-*.md`, `.planning/phases/39-spend-ledger/{39-CONTEXT,39-DISCUSSION-LOG}.md`, `.planning/phases/38-*/{38-CONTEXT,38-VERIFICATION}.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`

### Secondary (MEDIUM confidence — official docs, cross-checked via WebSearch this session)
- PostgreSQL `INSERT` documentation (partial-unique-index `ON CONFLICT ... WHERE` arbiter matching requirement) `[CITED: postgresql.org/docs/current/sql-insert.html]`
- SQLite's UPSERT grammar supporting the identical `WHERE`-scoped conflict target (`[CITED: sqlite.org lang_upsert]`, general community confirmation via WebSearch, not independently executed against this workspace's exact SQLite build in this session)

### Tertiary (LOW confidence)
- The `BEGIN IMMEDIATE`-held-across-await footgun note (`emschwartz.me` blog post) — directionally correct community guidance, not an official sqlx doc; treated as a pitfall warning, not a load-bearing claim

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new dependencies, every version pinned and verified in `Cargo.lock`
- Architecture: HIGH — every attachment point and every gap (model field, run_id identity) verified by direct source read, not inferred from the ADRs' prose alone
- Pitfalls: HIGH — five of six pitfalls are grounded in a specific file:line in this codebase; the sixth (BEGIN IMMEDIATE footgun) is community-sourced but flagged as such

**Research date:** 2026-09-27
**Valid until:** 30 days (stable, internal-codebase-grounded; the only external-facing claim, sqlx 0.8.6's `begin_with` API, is a stable released API unlikely to change before this phase executes)
