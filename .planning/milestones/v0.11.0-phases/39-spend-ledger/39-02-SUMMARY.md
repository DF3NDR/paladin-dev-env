---
phase: 39-spend-ledger
plan: 02
subsystem: database
tags: [sqlx, sqlite, tokio, treasurer, ledger, hexagonal-ports, adr-0053]

# Dependency graph
requires:
  - phase: 39-spend-ledger
    provides: "39-01: TreasuryLedgerPort's settle/spend/store_now, the 007 SQLite schema
      (partial unique settlement index, scope+window covering index, reservation lookup
      index, settled-window scan index), SqliteTreasuryLedger's settle/spend/store_now,
      the CLI tracer"
provides:
  - "TreasuryLedgerPort::reserve and ::release (D-04), with per-scope serialized admission
    (LEDGR-02) and full rustdoc"
  - "crates/paladin-storage/src/treasury/contract_tests.rs -- 21 generic async contract
    clause functions plus observed_balance/contract_scope/settle_request helpers, proving
    reserve/release/settle/spend/store_now identically on every adapter (D-11)"
  - "InMemoryTreasuryLedger -- always-on adapter (no feature gate), one tokio::sync::Mutex
    across SUM+insert (D-12), a HashSet<SettlementKey> idempotency twin (D-06)"
  - "SqliteTreasuryLedger::reserve/release and the reserved-settle path under
    Pool::begin_with(\"BEGIN IMMEDIATE\") (D-12), plus a test-only new_shared_file(WAL)
    constructor"
affects: [39-03, 39-04, 39-05, 39-06, 39-07, 39-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Per-scope serialized reserve-then-settle: the balance SUM and the admitting INSERT
      run inside one transaction (SQLite BEGIN IMMEDIATE; in-memory one
      tokio::sync::Mutex held across the whole method body), never a check-then-act
      pair across two separate calls"
    - "A settle/release referencing a reservation inherits that reservation's own
      attributed_at (the store clock at reserve time), never the settle/release call's
      own clock read -- so a hold placed in one window and closed after it ends still
      counts against the window it was placed in"
    - "Contract clauses are pure functions over &dyn TreasuryLedgerPort (Arc<dyn ..> for
      the two concurrency clauses); every clause isolates itself with a unique
      contract_scope tenant id or fresh run ids and filters its own spend() calls by
      them, so a shared database (39-03's Postgres suite) never leaks rows between
      clauses"

key-files:
  created:
    - crates/paladin-storage/src/treasury/contract_tests.rs
    - crates/paladin-storage/src/treasury/in_memory.rs
  modified:
    - crates/paladin-ports/src/output/treasury_ledger_port.rs
    - crates/paladin-storage/src/treasury/mod.rs
    - crates/paladin-storage/src/treasury/sqlite.rs
    - crates/paladin-storage/src/lib.rs

key-decisions:
  - "contract_tests.rs was authored as one file spanning both this plan's tasks' clauses
    (they share observed_balance/contract_scope/settle_request helpers and the file is a
    plain, non-#[cfg(test)] module with no functional cost to including both tasks'
    functions in Task 1's commit) -- Task 1's commit includes the full contract suite
    text; Task 2's commit wires every clause into InMemoryTreasuryLedger and completes
    the SQLite test module. Documented here rather than silently splitting the file
    mid-task."
  - "The real overflow edge (balance.checked_add(hold) returning None, not merely hold
    exceeding the ceiling) is exercised by priming a fresh scope with a small balance
    (hold 1, ceiling 1) then attempting hold i64::MAX against ceiling i64::MAX -- the
    plan's literal 'hold i64::MAX ceiling 0' example does not actually overflow when the
    scope's balance starts at 0, so a second, balance-primed case was added to prove the
    checked-add path specifically."
  - "release's recorded_at (audit timestamp) is a fresh store-clock read distinct from its
    attributed_at (the closed reservation's own instant, kept for balance-window
    consistency) -- mirrors the same attributed_at/recorded_at split the reserved settle
    path uses."

patterns-established:
  - "InMemoryTreasuryLedger mirrors SqliteTreasuryLedger's reserve/release/settle
    semantics field-for-field (foreign-currency probe, checked-add admission, outstanding
    hold via a closed-reservation scan, identical spend fold/order) without sharing any
    storage representation -- proven by the same contract_tests functions passing
    unmodified on both."

requirements-completed: [LEDGR-01, LEDGR-02, LEDGR-03]

coverage:
  - id: D1
    description: "LEDGR-02's N-1-of-N race clause (16 concurrent reserves of hold 1
      against ceiling 15 on a fresh scope yield exactly 15 Ok and 1 Refused) passes on
      InMemoryTreasuryLedger and on SqliteTreasuryLedger over a real on-disk WAL file
      with multiple pooled connections"
    requirement: "LEDGR-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --lib treasury::in_memory::contract_suite::reserve_race_admits_exactly_n_minus_one"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite --lib treasury::sqlite::tests::reserve_race_admits_exactly_n_minus_one_on_disk"
        status: pass
    human_judgment: false
  - id: D2
    description: "reserve/release/settle follow ADR-0053 §2's signed-contribution balance
      math (reserve +hold, reserved settle actual-hold-or-actual depending on whether the
      reservation is still open, release -hold once and idempotent thereafter,
      attribution to the reservation's own window) on every adapter"
    requirement: "LEDGR-02"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite --lib treasury:: (62 tests,
          incl. reserve_then_settle_contributes_actual_minus_hold,
          settle_after_release_charges_actual_only,
          settle_is_attributed_to_its_reservation_window,
          release_returns_the_hold_and_is_idempotent)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Settlement idempotency (LEDGR-03): a duplicate SettlementKey is
      Settled then AlreadySettled with the first amount kept, a bumped attempt is a
      distinct settlement, and ten concurrent settles of one key produce exactly one
      Settled and nine AlreadySettled -- on the in-memory adapter and on a real on-disk
      SQLite file"
    requirement: "LEDGR-03"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite --lib treasury::sqlite::tests::concurrent_duplicate_settles_charge_once_on_disk"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-storage --lib treasury::in_memory::contract_suite (21
          tests incl. duplicate_settle_is_already_settled_and_charges_once,
          bumped_attempt_is_a_distinct_settlement)"
        status: pass
    human_judgment: false
  - id: D4
    description: "spend groups by tenant/api-key/run/model with tenant_id/api_key_id/
      run_ids filters, orders by group ascending then currency ascending, treats an
      empty window as Ok(vec![]), and never combines two currencies into one row --
      identically on the in-memory and SQLite adapters"
    requirement: "LEDGR-01"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-storage --features sqlite --lib treasury:: (incl.
          spend_groups_by_every_dimension_over_a_window,
          spend_orders_groups_then_currencies_ascending,
          spend_splits_currencies_into_separate_rows, spend_window_is_half_open)"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-ports --doc treasury_ledger"
        status: pass
    human_judgment: false

duration: ~30min
completed: 2026-09-27
status: complete
---

# Phase 39 Plan 02: Race-Proof Draws and Idempotent Settlement Summary

**`TreasuryLedgerPort::reserve`/`release` under per-scope `BEGIN IMMEDIATE` serialization
(SQLite) and one `tokio::sync::Mutex` (in-memory), proven by a 21-clause shared contract
suite including the phase's first red test: 16 concurrent reserves against a ceiling of 15
yield exactly 15 admissions and 1 refusal, on both adapters.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 2 (both executed autonomously, no checkpoints)
- **Files modified:** 6 (2 created, 4 modified)

## Accomplishments

- **`TreasuryLedgerPort::reserve`/`release`** (`crates/paladin-ports/src/output/treasury_ledger_port.rs`):
  full rustdoc specifying the admission rule (`balance + hold <= ceiling`, overflow does not
  fit, foreign currency in scope+window is `CurrencyMismatch`, per-scope serialization,
  store-clock attribution), `release`'s idempotent/no-op-after-settle contract, and an
  updated rustdoc mock (now implementing `reserve`/`release`) that still compiles as a
  doctest.
- **The shared contract suite** (`crates/paladin-storage/src/treasury/contract_tests.rs`,
  new): 21 `pub async fn` clauses plus `usd`/`eur`/`contract_scope`/`settle_request`/
  `observed_balance` helpers. `observed_balance` reads a scope+window's balance without
  writing anything, by reserving `i64::MAX` against a ceiling of `0` (always refused) and
  reading the `balance` the error names. Clauses cover: the LEDGR-02 race
  (`reserve_race_admits_exactly_n_minus_one`, `Arc<dyn TreasuryLedgerPort>`), ceiling
  adjacency and checked-add overflow, reserve-then-settle and unreserved-settle balance
  math, release idempotency (including a no-op release after settle and an
  `UnknownReservation` on a never-issued id), settle-after-release, window attribution to
  the reservation's own instant, currency-mismatch refusals (foreign currency in scope,
  hold/ceiling mismatch, settle/reservation mismatch), request validation (negative
  amounts, bad windows, empty scope), two reservations legally sharing one superstep-attempt
  key, duplicate-settle and bumped-attempt idempotency, a 10-way concurrent-duplicate-settle
  race (`Arc`), full spend grouping/filtering/ordering/half-open-window/currency-split
  behavior, settle validation, `store_now` monotonicity, and the `unattributed` sentinel.
- **`InMemoryTreasuryLedger`** (`crates/paladin-storage/src/treasury/in_memory.rs`, new,
  always compiled, no feature gate): one `tokio::sync::Mutex<LedgerState>` held across each
  method's whole body; `LedgerState.settled: HashSet<SettlementKey>` is the in-memory twin
  of the SQL adapters' partial unique index; entries are only ever pushed (append-only,
  proven by a grep for `entries.(remove|retain|clear|truncate)` printing `0`). All 21
  clauses pass unmodified via `#[cfg(test)] mod contract_suite`.
  `crates/paladin-storage/src/treasury/mod.rs` registers `pub mod in_memory;` and
  `pub mod contract_tests;`; `lib.rs`'s module doc updated to reflect the in-memory
  backend's always-on availability.
- **`SqliteTreasuryLedger::reserve`/`release` and the reserved-settle path**
  (`crates/paladin-storage/src/treasury/sqlite.rs`): every reserve/settle/release
  transaction opens with `Pool::begin_with("BEGIN IMMEDIATE")` and does exactly the
  foreign-currency probe, the balance `SUM`, the admission check, one `INSERT`, and
  commit/rollback -- no other `.await` while the transaction is open (Pitfall 2). New
  consts `BALANCE_QUERY`, `FOREIGN_CURRENCY_QUERY`, `RESERVE_INSERT`, `SELECT_RESERVATION`,
  `RESERVATION_CLOSED_QUERY`, `RELEASE_INSERT`; `SETTLE_INSERT`'s `reservation_id` bind now
  carries the real reservation id instead of an unconditional `NULL`. A test-only
  `new_shared_file` (WAL journal mode, mirrors `SqliteRunRepository`'s D-52 precedent) backs
  `reserve_race_admits_exactly_n_minus_one_on_disk` and
  `concurrent_duplicate_settles_charge_once_on_disk`, proving both concurrency guarantees
  under real multi-connection SQLite concurrency, not a single in-process connection.
- Wired all 21 clauses into `sqlite.rs`'s test module (one `#[tokio::test]` per clause over
  `sqlite::memory:`, the two concurrency clauses `_on_disk`); replaced the pre-existing
  hand-rolled `store_now_is_non_decreasing` test with a call to the shared clause so no
  slightly-diverging duplicate implementation survives.

## Task Commits

1. **Task 1: LEDGR-02 first red -- race clause, reserve/release on the port, and SQLite
   BEGIN IMMEDIATE reservations** - `4bdef11f` (test)
2. **Task 2: Complete the contract suite and add InMemoryTreasuryLedger -- idempotency,
   spend windows/grouping/ordering, validation, store clock** - `5af5e783` (feat)

**Plan metadata:** pending (this commit)

## Files Created/Modified

- `crates/paladin-ports/src/output/treasury_ledger_port.rs` - `reserve`/`release` trait
  methods with full rustdoc, updated error-variant docs, extended rustdoc mock
- `crates/paladin-storage/src/treasury/contract_tests.rs` - 21-clause shared contract
  suite plus helpers (new)
- `crates/paladin-storage/src/treasury/in_memory.rs` - `InMemoryTreasuryLedger` (new)
- `crates/paladin-storage/src/treasury/mod.rs` - `validate_reserve`, registers
  `in_memory`/`contract_tests` modules
- `crates/paladin-storage/src/treasury/sqlite.rs` - `reserve`/`release`/reserved-`settle`
  under `BEGIN IMMEDIATE`, `new_shared_file`, full test wiring
- `crates/paladin-storage/src/lib.rs` - `treasury` module doc updated (in-memory backend
  always available)

## Decisions Made

- `contract_tests.rs` was authored as one file spanning both tasks' clauses (see
  key-decisions above) -- functionally harmless since it is a plain, non-test-gated module
  and both tasks' acceptance criteria only positively check for clause presence, never
  absence.
- The plan's literal overflow example (`hold i64::MAX, ceiling 0`) does not exercise
  `checked_add`'s `None` branch when the scope's balance starts at `0` (`0 + i64::MAX`
  never overflows `i64`); a second sub-case primes the scope with a small balance first so
  the real overflow path is proven, not just the "hold exceeds ceiling" path.
- `release`'s `recorded_at` (audit timestamp) is a fresh store-clock read, kept distinct
  from `attributed_at` (the closed reservation's own instant) -- mirrors the same split the
  reserved-settle path already needed, applied consistently rather than collapsing them.

## Deviations from Plan

None - plan executed exactly as written, with the two additions above (the overflow
sub-case and the release recorded_at/attributed_at split) both required to satisfy the
plan's own must-have truths (the overflow edge and D-12's window/window-audit distinction)
rather than optional extras.

## Issues Encountered

- `cargo fmt --check` initially failed after the first draft of `sqlite.rs`'s reserved-
  settle path and `in_memory.rs`'s spend filter chain (long `if let` chains and a
  multi-line closure) -- resolved with `cargo fmt` (Rule 3, blocking); re-verified clippy
  and the full test suite afterward with no behavior change.
- `cargo clippy -D warnings` flagged four `collapsible_if` findings in `in_memory.rs`'s
  `spend` window/tenant/api-key filters -- rewritten as `if let ... && cond { continue; }`
  chains (Rule 1, a real lint, not a style nit under `-D warnings`).

## User Setup Required

None - no external service configuration required; both adapters are embedded (in-memory,
SQLite).

## Next Phase Readiness

- `TreasuryLedgerPort` now exposes its complete Phase 39 surface (`reserve`, `release`,
  `settle`, `spend`, `store_now`) proven identically on the in-memory and SQLite adapters
  by one 21-clause suite; 39-03 twins the same suite on Postgres (transaction-scoped
  advisory lock instead of `BEGIN IMMEDIATE`, `JSONB`/`TIMESTAMPTZ` instead of TEXT) with
  no redesign expected.
- `make api-surface` diverges from the committed baseline as expected -- this plan is
  additive-only (new trait methods, new module) per D-00f, and `.project/current-
  exports.txt`/CHANGELOG updates are explicitly owned by plan 39-08, not touched here.
- No blockers. 39-04/39-05 (the production engine-path and agent-loop settle writers) can
  now also reserve at the superstep/model-call boundary once Phase 42 needs the halt hook;
  this plan's `reserve`/`release` signatures are the ones those phases call unchanged.

---
*Phase: 39-spend-ledger*
*Completed: 2026-09-27*

## Self-Check: PASSED

All 6 created/modified source files plus this SUMMARY verified present on disk; commits
`4bdef11f` and `5af5e783` verified present in git history.
