---
phase: 39-spend-ledger
plan: 01
subsystem: database
tags: [sqlx, sqlite, treasurer, ledger, cli, hexagonal-ports, adr-0053]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer
    provides: "Cost/CurrencyCode/PriceTable fixed-point value types, ExecutionMetadata's
      cost_display() D-04 format, additive cost fields on LlmResponse/PaladinResult/trace events"
provides:
  - "paladin-core::platform::container::treasury_ledger -- LedgerScope, ReservationId,
    SettlementKey, LedgerEntryKind, ReserveRequest/SettleRequest/SettleOutcome,
    SpendGroupBy/SpendQuery/SpendRow, SettlementContext, format_cost"
  - "paladin-ports::output::treasury_ledger_port -- TreasuryLedgerPort (settle/spend/store_now
    this plan) and TreasuryLedgerError, with a compiling rustdoc mock"
  - "The 007 treasury_ledger SQLite migration and SqliteTreasuryLedger adapter, store-enforced
    settlement idempotency"
  - "paladin-cli treasury spend -- reads the configured RunStoreConfig store directly, no server"
affects: [39-02, 39-03, 39-04, 39-05, 39-06, 39-07, 39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Treasury storage module mirrors crate::run's house pattern one-for-one (mod.rs +
      per-backend adapter + shared pub(crate) validation function)"
    - "Settlement idempotency is store-enforced: a partial unique index on
      (run_id, superstep, attempt) WHERE kind = 'settle' plus
      INSERT ... ON CONFLICT ... DO NOTHING, never an application-side check"
    - "A settle write reads the store's own clock (SELECT/strftime, never Utc::now()) and
      truncates it through crate::run::storage_timestamp before binding, reused not duplicated"

key-files:
  created:
    - crates/paladin-core/src/platform/container/treasury_ledger.rs
    - crates/paladin-ports/src/output/treasury_ledger_port.rs
    - crates/paladin-storage/src/treasury/mod.rs
    - crates/paladin-storage/src/treasury/sqlite.rs
    - crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql
    - src/application/cli/commands/treasury.rs
  modified:
    - crates/paladin-core/src/platform/container/mod.rs
    - crates/paladin-ports/src/output/mod.rs
    - crates/paladin-storage/src/lib.rs
    - src/application/cli/commands/mod.rs
    - src/bin/paladin-cli.rs

key-decisions:
  - "Task 1 checkpoint (007 schema/index design) resolved as option-a (approve as proposed) by
    the operator on 2026-09-27, before any migration file was written -- see Checkpoint outcome
    below"
  - "The engine-path and agent-loop production settle writers, the in-memory adapter, the
    Postgres adapter, and reserve/release are all out of scope for this plan by design (39-02
    through 39-07 own them); this plan proves core -> ports -> SQLite -> CLI end to end on
    settle/spend/store_now alone"

patterns-established:
  - "TreasuryLedgerPort mirrors RunRepositoryPort's error-enum shape (X-06), rustdoc-mock
    convention, and Backend/Serialization boundary-conversion pattern exactly"
  - "SqliteTreasuryLedger mirrors SqliteRunRepository's new()/wrap()/wrap_error() shape and
    run_trace::sqlite's ON CONFLICT ... DO NOTHING idempotency precedent, extended with the
    kind = 'settle' partial-index arbiter predicate"

requirements-completed: [LEDGR-01, LEDGR-04]

coverage:
  - id: D1
    description: "TreasuryLedgerPort domain types and trait exist in paladin-core/paladin-ports,
      mirroring RunRepositoryPort's house shape, with a compiling rustdoc mock"
    requirement: "LEDGR-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai-core --lib treasury_ledger"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ports --doc treasury_ledger"
        status: pass
    human_judgment: false
  - id: D2
    description: "The 007 SQLite migration and SqliteTreasuryLedger adapter settle a spend with
      store-enforced idempotency (duplicate settle -> AlreadySettled, never a double charge)"
    requirement: "LEDGR-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite --lib treasury:: (14 tests, incl.
          duplicate_settle_is_already_settled_and_counted_once,
          settle_arbiter_predicate_matches_the_migration)"
        status: pass
    human_judgment: false
  - id: D3
    description: "paladin-cli treasury spend reads settlements back from the SQLite ledger with
      no server -- per-model, per-run, per-currency rows in table and JSON formats"
    requirement: "LEDGR-04"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-ai --features cli --lib application::cli::commands::treasury
          (4 tests, incl. treasury_spend_tracer_reads_settlements_from_sqlite_ledger,
          treasury_spend_prints_one_row_per_currency)"
        status: pass
      - kind: integration
        ref: "APP_RUN_STORE_BACKEND=sqlite APP_RUN_STORE_PATH=... paladin-cli treasury spend
          --format json against a fresh migrated file prints exactly []"
        status: pass
    human_judgment: false
  - id: D4
    description: "Task 1's blocking checkpoint:decision on the one-way 007 schema/index design
      was answered by the operator before any migration file was written"
    human_judgment: true
    rationale: "Operator sign-off on a one-way schema decision is inherently a human judgment
      call, not something a test can classify; recorded in Checkpoint outcome below."

duration: ~30min
completed: 2026-09-27
status: complete
---

# Phase 39 Plan 01: Spend Ledger Tracer Summary

**A settlement travels core -> ports -> SQLite -> CLI end to end: `TreasuryLedgerPort::settle`
persists into the 007 `treasury_ledger` table with store-enforced idempotency, and
`paladin-cli treasury spend` reads it back as `0.0450 USD`-style rows with no server.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 2 (Task 1 checkpoint resolved without pause; Task 2 tracer executed and committed)
- **Files modified:** 11 (6 created, 5 modified)

## Checkpoint outcome

Task 1 was a blocking `checkpoint:decision` gate on the one-way `007_create_treasury_ledger_table`
schema (D-01 scope columns, D-06 partial unique settlement index, D-02 `model_breakdown` column,
D-12's covering index keyed on `attributed_at` rather than a per-row `window_start`). **The
operator answered this checkpoint through the orchestrator before this plan executed: option-a,
"Approve as proposed," confirmed 2026-09-27.** No redirect was requested. The schema in
`007_create_treasury_ledger_table.sql` implements exactly the design presented in the plan's
Task 1 `<context>` block, including the planner's documented deviation from D-12's literal
`window_start` wording (the index is keyed on `attributed_at`, since a per-row `window_start`/
`window_end` cannot express membership in Phase 41's rolling windows). This is recorded here so
39-03 (which twins this DDL on Postgres) and any later phase reading this schema can find the
operator's sign-off without re-deriving it.

## Accomplishments

- **Core domain types** (`crates/paladin-core/src/platform/container/treasury_ledger.rs`):
  `LedgerScope` (with the `unattributed()` sentinel constructor, D-01), `ReservationId` (UUIDv7,
  mirrors `RunId`), `SettlementKey` (`run_id`/`superstep`/`attempt`, ADR-0053 §4, no `node_id`),
  `LedgerEntryKind`, `ReserveRequest`/`SettleRequest`/`SettleOutcome`, `SpendGroupBy`/`SpendQuery`/
  `SpendRow`, `SettlementContext`, and `format_cost` (byte-identical to
  `ExecutionMetadata::cost_display()`, proven by a shared-format unit test). Six unit tests plus
  six doctests, all passing.
- **Port trait** (`crates/paladin-ports/src/output/treasury_ledger_port.rs`): `TreasuryLedgerPort`
  with `settle`/`spend`/`store_now` (this plan's scope; `reserve`/`release` arrive in 39-02) and
  `TreasuryLedgerError` (`Refused`, `CurrencyMismatch`, `UnknownReservation`, `InvalidRequest`,
  `Backend`, `Serialization`), mirroring `RunRepositoryError`'s X-06 shape exactly, with a
  compiling rustdoc mock proving a duplicate settlement key is success, never an error.
- **Storage** (`crates/paladin-storage/src/treasury/{mod,sqlite}.rs`,
  `migrations/sqlite/007_create_treasury_ledger_table.sql`): the approved 007 schema (four
  indexes: the partial unique settlement index, the scope+window covering index, the reservation
  lookup index, and the settled-window scan index) and `SqliteTreasuryLedger` implementing
  `settle`/`spend`/`store_now`. `settle` validates through the shared `validate_settle` (crate-
  private, nine unit tests), rejects any `Some(reservation)` as `UnknownReservation` (no adapter
  can yet hold a `reserve` row), stamps `attributed_at`/`recorded_at` from the store's own clock
  truncated through `crate::run::storage_timestamp`, and relies on the store's own
  `ON CONFLICT (run_id, superstep, attempt) WHERE kind = 'settle' DO NOTHING` for idempotency.
  `spend` folds settle rows in Rust into `(group, currency)` buckets -- never combining
  currencies -- with `Model` grouping unpacking each row's `model_breakdown`. Fourteen unit tests
  pass, including a dedicated arbiter/migration-predicate-agreement test (Pitfall 3) and a
  password-redaction test.
- **CLI** (`src/application/cli/commands/treasury.rs`, wired into `commands/mod.rs` and
  `src/bin/paladin-cli.rs`): `paladin-cli treasury spend --since/--until/--group-by
  tenant|api-key|run|model/--tenant/--api-key/--run/--format table|json`, reading the ledger
  directly through the existing `RunStoreConfig` (no HTTP round-trip, mirrors `run export`).
  Four tests pass, including the phase's tracer: two settlements (`gpt-4` at `45_000_000` nanos,
  `gpt-4o-mini` at `1_500_000` nanos) settle into a fresh on-disk SQLite file and read back as
  `0.0450 USD` / `0.0015 USD` per model and `0.0465 USD` / `2` settlements per run, exactly as the
  plan's must-have truth demands. A mixed-currency scope+run prints two rows, never a combined
  figure.
- The verify command's smoke test (`APP_RUN_STORE_BACKEND=sqlite ... paladin-cli treasury spend
  --format json` against a fresh migrated file) prints exactly `[]`.

## Task Commits

1. **Task 1: Confirm the one-way 007 treasury_ledger schema before it is written** - resolved
   via the operator's pre-recorded decision (option-a); no code change, no separate commit (the
   decision is recorded in this SUMMARY and in the Task 2 commit message).
2. **Task 2: Tracer -- a settlement written through TreasuryLedgerPort into the SQLite ledger is
   shown by `paladin-cli treasury spend`** - `7d368652` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/treasury_ledger.rs` - Ledger domain value types
  (LedgerScope, ReservationId, SettlementKey, LedgerEntryKind, requests/query/row types,
  format_cost)
- `crates/paladin-core/src/platform/container/mod.rs` - Registers `pub mod treasury_ledger;`
  (alphabetical, after `transience`)
- `crates/paladin-ports/src/output/treasury_ledger_port.rs` - `TreasuryLedgerPort` trait and
  `TreasuryLedgerError`
- `crates/paladin-ports/src/output/mod.rs` - Registers `pub mod treasury_ledger_port;`
- `crates/paladin-storage/src/lib.rs` - Registers `pub mod treasury;`
- `crates/paladin-storage/src/treasury/mod.rs` - Module docs, `validate_settle` shared validation
- `crates/paladin-storage/src/treasury/sqlite.rs` - `SqliteTreasuryLedger` adapter
- `crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql` - The approved
  007 schema
- `src/application/cli/commands/treasury.rs` - `paladin-cli treasury spend`
- `src/application/cli/commands/mod.rs` - Registers `pub mod treasury;`
- `src/bin/paladin-cli.rs` - `Commands::Treasury` wiring

## Decisions Made

- Task 1's one-way schema/index design approved as proposed (option-a) by the operator before
  execution -- see Checkpoint outcome above.
- The Postgres arm of `build_postgres_treasury_ledger` in the CLI returns the same
  `CliError::configuration` shape `run.rs`'s `not(feature = "storage-postgres")` arm uses,
  regardless of whether that feature is enabled -- `PostgresTreasuryLedger` does not exist until
  39-03, so there is nothing to build yet either way. This keeps the operator-facing message
  consistent and avoids a half-wired feature-gated arm that would need rewriting in 39-03 anyway.
- `TREASURY_LEDGER_SCHEMA_VERSION` is a private `const` inside `sqlite.rs` (mirroring
  `RUN_SCHEMA_VERSION`'s convention) rather than a public core type, since no cross-crate reader
  needs it in this plan's scope.

## Deviations from Plan

None - plan executed exactly as written. The Task 1 checkpoint was pre-resolved by the operator
per the orchestrator's instructions, so no pause occurred; Task 2 was implemented, tested, and
committed exactly per the plan's `<action>` text.

## Issues Encountered

- The sandbox's root filesystem ran out of disk space mid-build (`No space left on device`)
  during the CLI smoke test, caused by a 7.6 GiB `target/debug/incremental` cache built up from
  prior sessions. Cleared with `rm -rf target/debug/incremental` (safe: incremental compilation
  caches are regenerable and carry no source-of-truth data); freed the build to complete. Not a
  code or plan issue -- purely an environment housekeeping item, not reintroduced by this commit.

## User Setup Required

None - no external service configuration required. SQLite is embedded; the smoke test used a
temp directory, no persistent operator setup needed.

## Next Phase Readiness

- The architecture is proven end-to-end: `TreasuryLedgerPort` compiles, `SqliteTreasuryLedger`
  settles and reads back spend, and the CLI surface exists. 39-02 can add `reserve`/`release`,
  the shared contract-test suite, and the in-memory adapter against this same trait without any
  redesign.
- 39-03 has the approved 007 schema to twin on Postgres (transaction-scoped advisory lock instead
  of `BEGIN IMMEDIATE`, `JSONB` instead of `TEXT` for `model_breakdown`, `TIMESTAMPTZ` instead of
  `TEXT` for timestamps) -- the SQLite migration's header comment documents every index's purpose
  for that port.
- No blockers. The engine-path and agent-loop production settle writers (39-04/39-05) still need
  the model-identity gap noted in `39-RESEARCH.md` Pitfall 5 resolved (no `model` field exists yet
  on `PaladinResult`/`NodeExecutionRecord` for the engine path) -- out of this plan's scope, but
  worth flagging again here since it is the next concrete design decision those plans must make.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-27*

## Self-Check: PASSED

All 6 created files verified present on disk; commit `7d368652` verified present in git history.
