-- Migration: Create Treasury Ledger Table (SQLite)
-- Purpose: Append-only Treasurer spend ledger (LEDGR-01..03, ADR-0053, D-01/D-02/D-06/D-11/D-12)
-- Version: 007
-- Date: 2026-09-27
--
-- `treasury_ledger` is ADR-0053's append-only, derive-on-read ledger: a scope+window balance is
-- a plain `SUM(amount_nanos)` over every row attributed inside that window -- never a running
-- counter maintained anywhere else. `amount_nanos` is the row's SIGNED contribution (`reserve`
-- = `+hold`, `settle` = `actual - hold` or `actual` when unreserved, `release` = `-hold`);
-- `charged_nanos` is the actual charge on a `settle` row only (`0` on `reserve`/`release`) and
-- is what the spend view (LEDGR-04) sums -- these two columns diverge exactly when a settle
-- draws down a reservation (39-02), and agree on every unreserved settle (every Phase 39
-- production row).
--
-- Every ledger row carries `tenant_id`/`api_key_id` NOT NULL (D-01, one-way): until Phase 40
-- records a submitting principal's tenant on the `Run` row, every Phase 39 production writer
-- stamps the literal sentinel `'unattributed'` in both columns (`LedgerScope::unattributed()`);
-- Phase 40 changes only the SOURCE of these values, never this schema.
--
-- `model_breakdown` (D-02, costly to remove) is a JSON object mapping a bare model name to its
-- nano-unit contribution; it is folded in Rust (never with backend-specific JSON SQL
-- functions), never queried by SQL itself, so its column type only needs to round-trip text.
--
-- `attributed_at` is the instant that places a row inside a balance window: the store clock at
-- write time for a reservation or an unreserved settle; a settle/release that references a
-- reservation instead inherits that reservation's own `attributed_at` (ADR-0053 §2, "attributed
-- to their reservation's window"). Planner deviation from D-12's literal wording: D-12 names
-- the covering index on `(tenant_id, api_key_id, window_start)`, but storing a per-row
-- `window_start`/`window_end` cannot express membership in Phase 41's rolling windows (a row
-- written at instant t belongs to every rolling window that contains t) -- the index below is
-- keyed on `attributed_at` instead, serving the same balance-SUM and window-query purpose.
-- `recorded_at` is purely an audit timestamp: when the row was physically written.
--
-- Indexes:
-- - `idx_treasury_ledger_settlement`: the store-enforced settlement idempotency key (D-06,
--   ADR-0053 §4) -- a UNIQUE index on `(run_id, superstep, attempt)` restricted to
--   `kind = 'settle'`. Every settle INSERT's `ON CONFLICT (run_id, superstep, attempt)
--   WHERE kind = 'settle' DO NOTHING` arbiter predicate below MUST textually match this index's
--   `WHERE` clause, or SQLite/Postgres cannot infer it as the conflict target -- kept in sync
--   with `SqliteTreasuryLedger`'s `SETTLE_INSERT` constant (Pitfall 3), mirroring
--   `002_create_runs_table.sql`'s own cross-file busy-set precedent.
-- - `idx_treasury_ledger_scope_window`: serves the per-scope balance SUM and `spend`'s window
--   queries (D-12).
-- - `idx_treasury_ledger_reservation`: serves a settle/release row's lookup of its own
--   reservation (39-02); partial, since most rows (every unreserved settle) carry no
--   `reservation_id`.
-- - `idx_treasury_ledger_settled_window`: serves the CLI's all-tenants `spend` window scan
--   (D-09) without touching non-settle rows.
--
-- Timestamps are TEXT RFC 3339 (mirroring `002`/`006`'s own choice); `model_breakdown` is TEXT
-- holding a JSON object (the Postgres twin, `007` under `migrations/postgres/`, uses JSONB --
-- arrives in 39-03).

CREATE TABLE IF NOT EXISTS treasury_ledger (
    entry_id         TEXT PRIMARY KEY NOT NULL,
    kind             TEXT NOT NULL CHECK (kind IN ('reserve', 'settle', 'release')),
    tenant_id        TEXT NOT NULL,
    api_key_id       TEXT NOT NULL,
    reservation_id   TEXT NULL,
    run_id           TEXT NULL,
    superstep        BIGINT NULL,
    attempt          BIGINT NULL,
    amount_nanos     BIGINT NOT NULL,
    charged_nanos    BIGINT NOT NULL DEFAULT 0,
    currency         TEXT NOT NULL,
    model_breakdown  TEXT NOT NULL DEFAULT '{}',
    attributed_at    TEXT NOT NULL,
    recorded_at      TEXT NOT NULL,
    schema_version   TEXT NOT NULL,
    CHECK (
        kind != 'settle'
        OR (run_id IS NOT NULL AND superstep IS NOT NULL AND attempt IS NOT NULL)
    )
);

-- The D-06 idempotency invariant itself: at most one `settle` row per
-- `(run_id, superstep, attempt)`. Reserve rows are deliberately NOT covered -- two reservations
-- for one superstep attempt are legal (a retry after a released hold, 39-02).
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_ledger_settlement
ON treasury_ledger (run_id, superstep, attempt) WHERE kind = 'settle';

-- Serves the per-scope balance SUM and `TreasuryLedgerPort::spend`'s window queries (D-12).
CREATE INDEX IF NOT EXISTS idx_treasury_ledger_scope_window
ON treasury_ledger (tenant_id, api_key_id, attributed_at);

-- Serves a settle/release row's lookup of its own reservation (39-02's `reserve`/`release`).
CREATE INDEX IF NOT EXISTS idx_treasury_ledger_reservation
ON treasury_ledger (reservation_id) WHERE reservation_id IS NOT NULL;

-- Serves the CLI's all-tenants `spend` window scan (D-09) without touching reserve/release rows.
CREATE INDEX IF NOT EXISTS idx_treasury_ledger_settled_window
ON treasury_ledger (attributed_at) WHERE kind = 'settle';
