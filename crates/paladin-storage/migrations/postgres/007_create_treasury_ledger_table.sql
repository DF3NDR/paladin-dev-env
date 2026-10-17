-- Migration: Create Treasury Ledger Table (PostgreSQL)
-- Purpose: Append-only Treasurer spend ledger (LEDGR-01..03, ADR-0053, D-01/D-02/D-06/D-11/D-12)
-- Version: 007
-- Date: 2026-09-27
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/007_create_treasury_ledger_table.sql) -- see that
-- file's header for the full ADR-0053 rationale (signed `amount_nanos` contributions,
-- `charged_nanos`/`model_breakdown` for the spend view, `attributed_at`/`recorded_at`'s split
-- purpose, the D-01 `unattributed` sentinel scope). This file differs only in column types:
-- `BIGINT` for the integer columns (`superstep`, `attempt`, `amount_nanos`, `charged_nanos`),
-- native `TIMESTAMPTZ` for `attributed_at`/`recorded_at`, and `JSONB` for `model_breakdown`
-- (mirroring `006`'s TEXT-vs-JSONB split) -- never queried by backend-specific JSON SQL
-- functions (`spend` folds it in Rust identically on every adapter, D-02).
--
-- Serialization on this backend is a transaction-scoped advisory lock keyed on the scope
-- (`pg_advisory_xact_lock(hashtext($1)::bigint)`, taken before the balance SUM inside the same
-- transaction) -- NOT a per-scope lock table (ADR-0053 §5, D-12). `hashtext` is a 32-bit hash:
-- two different scopes can share a lock key and merely serialize against each other, never a
-- correctness failure (T-39-14, accepted).
--
-- Indexes (identical purpose to the SQLite migration's four):
-- - `idx_treasury_ledger_settlement`: the store-enforced settlement idempotency key (D-06,
--   ADR-0053 §4) -- a UNIQUE index on `(run_id, superstep, attempt)` restricted to
--   `kind = 'settle'`. Every settle INSERT's `ON CONFLICT (run_id, superstep, attempt)
--   WHERE kind = 'settle' DO NOTHING` arbiter predicate in `PostgresTreasuryLedger` MUST
--   textually match this index's `WHERE` clause, or Postgres cannot infer it as the conflict
--   target (Pitfall 3) -- kept in sync with the SQLite migration's identical invariant.
-- - `idx_treasury_ledger_scope_window`: serves the per-scope balance SUM and `spend`'s window
--   queries (D-12).
-- - `idx_treasury_ledger_reservation`: serves a settle/release row's lookup of its own
--   reservation; partial, since most rows (every unreserved settle) carry no `reservation_id`.
-- - `idx_treasury_ledger_settled_window`: serves the CLI's all-tenants `spend` window scan
--   (D-09) without touching non-settle rows.

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
    model_breakdown  JSONB NOT NULL DEFAULT '{}'::jsonb,
    attributed_at    TIMESTAMPTZ NOT NULL,
    recorded_at      TIMESTAMPTZ NOT NULL,
    schema_version   TEXT NOT NULL,
    CHECK (
        kind != 'settle'
        OR (run_id IS NOT NULL AND superstep IS NOT NULL AND attempt IS NOT NULL)
    )
);

-- The D-06 idempotency invariant itself: at most one `settle` row per
-- `(run_id, superstep, attempt)`. Reserve rows are deliberately NOT covered -- two reservations
-- for one superstep attempt are legal (a retry after a released hold).
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_ledger_settlement
ON treasury_ledger (run_id, superstep, attempt) WHERE kind = 'settle';

-- Serves the per-scope balance SUM and `TreasuryLedgerPort::spend`'s window queries (D-12).
CREATE INDEX IF NOT EXISTS idx_treasury_ledger_scope_window
ON treasury_ledger (tenant_id, api_key_id, attributed_at);

-- Serves a settle/release row's lookup of its own reservation.
CREATE INDEX IF NOT EXISTS idx_treasury_ledger_reservation
ON treasury_ledger (reservation_id) WHERE reservation_id IS NOT NULL;

-- Serves the CLI's all-tenants `spend` window scan (D-09) without touching reserve/release rows.
CREATE INDEX IF NOT EXISTS idx_treasury_ledger_settled_window
ON treasury_ledger (attributed_at) WHERE kind = 'settle';
