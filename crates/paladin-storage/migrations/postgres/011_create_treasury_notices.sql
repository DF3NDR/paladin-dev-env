-- Migration: Create Treasury Notices Table (PostgreSQL)
-- Purpose: Store-enforced once-per-window allowance warn notices (ALLOW-04, Phase 41 D-15/D-16, C5)
-- Version: 011
-- Date: 2026-10-04
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/011_create_treasury_notices.sql) -- see that file's
-- header for the full rationale of every column, including why `api_key_id` is `NOT NULL` with
-- the empty string `''` for tenant scope (a NULL in a UNIQUE key never conflicts: PostgreSQL
-- treats NULLs as distinct in unique indexes before the PG15 opt-in `NULLS NOT DISTINCT`, and
-- this schema supports earlier versions), why a lifetime notice stores the Unix epoch as its
-- `window_start`, why `window_end` and `warn_at` are columns but not part of the identity, and
-- that the table is append-only apart from the `discard` of an abandoned admission's own rows.
--
-- This file differs only in column types: native `TIMESTAMPTZ` for `window_start`,
-- `window_end` and `recorded_at` (the SQLite twin stores RFC 3339 TEXT). The epoch sentinel is
-- `1970-01-01 00:00:00+00`.
--
-- Indexes (identical purpose to the SQLite migration's two):
-- - `idx_treasury_notices_once`: the dedup invariant itself -- UNIQUE over
--   `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)`. The
--   `ON CONFLICT (...)` column list in `PostgresTreasuryLedger`'s `NOTICE_INSERT` MUST textually
--   match this index's column list, or PostgreSQL cannot infer it as the conflict target; the
--   adapter's `notice_arbiter_matches_the_migration` test keeps the two in sync (Pitfall 3).
-- - `idx_treasury_notices_run`: serves `notices_for_run`; partial, since agent-path notices
--   carry no run.

CREATE TABLE IF NOT EXISTS treasury_notices (
    notice_id       TEXT PRIMARY KEY NOT NULL,
    scope_kind      TEXT NOT NULL CHECK (scope_kind IN ('tenant', 'api_key')),
    tenant_id       TEXT NOT NULL,
    api_key_id      TEXT NOT NULL,
    limit_kind      TEXT NOT NULL CHECK (limit_kind IN ('window', 'lifetime')),
    window_start    TIMESTAMPTZ NOT NULL,
    window_end      TIMESTAMPTZ NULL,
    ceiling_nanos   BIGINT NOT NULL,
    currency        TEXT NOT NULL,
    balance_nanos   BIGINT NOT NULL,
    warn_at         INTEGER NOT NULL CHECK (warn_at BETWEEN 0 AND 100),
    run_id          TEXT NULL,
    recorded_at     TIMESTAMPTZ NOT NULL,
    schema_version  TEXT NOT NULL
);

-- The once-per-window invariant: at most one notice per (scope, tenant, key, limit, window,
-- ceiling). `api_key_id` is NOT NULL so a tenant-scope notice (`''`) conflicts like any other.
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos);

-- Serves `TreasuryNoticePort::notices_for_run`.
CREATE INDEX IF NOT EXISTS idx_treasury_notices_run
ON treasury_notices (run_id) WHERE run_id IS NOT NULL;
