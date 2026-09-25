# ADR-0053: Treasury ledger balance model — append-only, derived on read

## Status

Accepted

**Date:** 2026-09-25

## Context

Phase 39 (LEDGR-01..04) writes the `007` migration in both
`crates/paladin-storage/migrations/{sqlite,postgres}/` and a `TreasuryLedgerPort` mirroring
`RunRepositoryPort`; Phases 41/42 draw against it. The Phase 27 worker is concurrent and
redelivers leases, so draws must be race-proof and settlement idempotent. SQLite has no native
decimal type, which is one reason the amount is an integer.

`TraceEvent::NodeStarted`/`NodeFinished` (`crates/paladin-core/src/platform/container/trace.rs`)
carry `superstep`, `node_id` and `attempt`, and show that a single `WarEngine` superstep can
dispatch several Paladin nodes concurrently — parallel vanguard nodes, Aegis node retries and
fallback hops can all land inside one superstep, sharing one attempt number. D-15 locks the
settlement idempotency key `(run_id, superstep, attempt)`; this ADR fixes what one settlement row
under that key covers, resolved at this plan's blocking checkpoint by the operator on 2026-09-25.

## Decision

1. **Balance model (D-14).** An append-only ledger — every reserve, settle and release is an
   immutable row, never updated or deleted by normal operation. A scope's window balance
   (committed spend for a tenant or API key in a window) is a `SUM` over that scope's rows
   attributed to the window, computed inside the same transaction that inserts a new reservation.

2. **Row kinds and signed contributions (D-15).** `reserve` contributes `+hold`; `settle`
   references its reservation (when one exists) and contributes `actual − hold` (a settle with no
   reservation contributes `actual`); `release` contributes `−hold` for a reservation that ended
   without a charge. Committed spend is therefore exactly the plain `SUM` of contributions.
   `settle` and `release` rows are attributed to their reservation's window, so a hold placed in
   one window and settled in the next never splits the window SUM across a boundary. Window
   timestamps come from the store or server clock, never a worker's local clock (ALLOW-01).

3. **Amount unit (D-02, D-15).** Every amount is an `i64` count of nano-units (1e-9) of the
   currency, carried with its ISO 4217 code on every row — the same unit as `Cost` (Phase 38 plan
   38-02). A `SUM` over rows with differing currency codes is a refusal, never a conversion (FX is
   FUT-12). The nano-unit is a finer scale than PRICE-02's micro-unit wording, and for a concrete
   reason: at $0.15 per 1M tokens, one token is 0.15 micro-units and would round to zero per call;
   it is 150 nano-units, which does not.

4. **Settlement idempotency key and settlement granularity (D-15), resolved by the operator at
   this plan's blocking checkpoint (superstep-aggregate, selected 2026-09-25):** the key
   `(run_id, superstep, attempt)` is kept exactly as D-15 locked it — **no `node_id` in the key,
   and D-15 is not amended.** One settlement row covers one superstep attempt in its entirety: on
   the engine path, the ledger settles once per superstep attempt, and that single settlement
   aggregates the `Cost` of every Paladin node the superstep dispatched — parallel vanguard nodes,
   Aegis node retries and fallback hops all roll into that one settlement — so two Paladin nodes
   dispatched in the same superstep can never collide on the key, because they never produce two
   separate settlement rows. `attempt` distinguishes a genuine re-execution of the superstep after
   resume or lease redelivery (real re-spend, charged again) from a duplicate delivery of the same
   settlement (charged once). Per-node cost is not a ledger row under this option — it remains
   visible in `NodeFinished.cost` trace events, not in the ledger; a superstep that fails partway
   settles whatever its completed nodes reported. On the agent loop, `run_id` is the Platform API
   run id when present, else the `PaladinExecutionService` execution id; `superstep` is the
   model-call ordinal within the loop; `attempt` is `1` (the service's own buffered retries of one
   call roll into that call's settlement). The persistence source of the engine-path attempt
   counter is named explicitly as an **open item Phase 39/42 must resolve**, not invented here.
   The milestone research's `(run_id, superstep_seq, node_attempt)` reading was **not adopted**:
   see the rejected `per-node-extended-key` option below for why.

5. **Serialization of SUM-then-reserve (LEDGR-02).** The reservation is admitted only if `SUM +
   hold` fits, and the SUM and the insert are serialized per scope inside one transaction:
   PostgreSQL rejects `FOR UPDATE` on an aggregate query, so the lock is taken on a per-scope row
   (or a transaction-scoped advisory lock keyed on the scope) BEFORE the SUM; SQLite takes the
   write lock up front with `BEGIN IMMEDIATE` (a deferred transaction would let two readers SUM
   before either writes); the in-memory adapter holds one mutex across SUM and insert. Growth is
   mitigated by an index covering (tenant, key, recorded timestamp).

6. **Left to Phase 39 (D-15).** Column names, index definitions and all DDL.

## Considered Options

- **Append-only, derive-on-read** (chosen) — D-14; no drift between a counter and its history, an
  audit trail for free, and the growth cost is mitigated by an index.
- **Running balance in an `allowance_windows` table** (rejected) — a second table whose counter
  can drift from the history that should justify it, a rollover job or lazy-reset logic needed at
  every window boundary, and two writes per draw that must stay transactionally consistent with
  the ledger itself.
- **Full DDL in this ADR** (rejected by D-15) — Phase 39 owns the schema, written against its own
  contract tests, not fixed prematurely here.
- **Per-node settlement, key extended to `(run_id, superstep, node_id, attempt)`** (rejected at
  this plan's checkpoint) — ledger rows would map one-to-one to node attempts, which is queryable
  for per-model views (LEDGR-04) without going through trace events, but this option amends D-15's
  locked key (a locked decision changed after the fact, requiring its own dated amendment note),
  produces more ledger rows, requires Phase 42's halt check at the superstep boundary to sum
  several per-node settlements instead of reading one row, and folds `node_id` into the persisted
  key. The operator selected `superstep-aggregate` instead specifically to avoid amending D-15 and
  to keep the draw, the settle and the halt check at ADR-0052's single superstep-boundary
  checkpoint.

## Code Locations

- `crates/paladin-core/src/platform/container/cost.rs` — the amount unit (`Cost`, `nanos: i64`,
  `CurrencyCode`), Phase 38 plan 38-02
- `crates/paladin-storage/migrations/sqlite/` and `crates/paladin-storage/migrations/postgres/` —
  the future `007` migration implementing this model
- `crates/paladin-ports/src/output/` — the future `TreasuryLedgerPort`, mirroring
  `run_repository_port.rs`
- `crates/paladin-core/src/platform/container/trace.rs` — `NodeStarted`/`NodeFinished`'s
  `superstep`/`attempt` coordinates, the source of the settlement key's non-`run_id` components

## Code Conformance

must change

Phase 39 implements this model in the `007` migration and the `TreasuryLedgerPort` adapters.

## Downstream Consumers

- **Phase 39** — LEDGR-01..04; must cite ADR-0053 and resolve the attempt-counter open item named
  in Decision point 4
- **Phase 41** — admission reserves against the balance this ADR derives on read
- **Phase 42** — mid-run draws and halts; settles once per superstep attempt at the engine's
  superstep boundary, matching ADR-0052's halt point
</content>
