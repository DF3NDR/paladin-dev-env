-- Migration: Add Treasury Notice Kind (SQLite)
-- Purpose: The halt rung of the once-per-window operator notice store (ALLOW-03, Phase 42 D-18,
--          G5, ADR-0057 group g)
-- Version: 014
-- Date: 2026-10-07
--
-- `treasury_notices` (011) recorded one row per allowance WARN crossing. A spend halt (Phase 42)
-- must notify the operator once per window as well, and it must stay distinct from the warning
-- for the same scope, limit, window and ceiling. So:
--
-- - `notice_kind`: `'warning'` or `'halt'`. PART of the dedup identity: a warning and a halt
--   notice for one scope, limit, window and ceiling are two different notices, each recorded
--   once; raising the ceiling re-arms both. NOT NULL with the default `'warning'`, so every row
--   written before this migration reads back as a warning notice and no existing claim is
--   disturbed.
-- - `notices_for_run` (the worker's first-dispatch replay of warn crossings) reads WARNING rows
--   only, so a halt row is never re-emitted as a warning.
--
-- Index: `idx_treasury_notices_once` is dropped and recreated with `notice_kind` appended to its
-- column list. Every adapter's `NOTICE_INSERT` `ON CONFLICT (...)` column list MUST textually
-- match this index's column list (Pitfall 3; the adapters' `notice_arbiter_matches_the_migration`
-- tests keep the two in sync). `idx_treasury_notices_run` is untouched.
--
-- This is a one-way schema change (a down-migration would have to drop the column and rebuild the
-- old index, losing every halt notice). `011_create_treasury_notices.sql` is NOT edited:
-- migrations are append-only (sqlx records a checksum per applied version).

ALTER TABLE treasury_notices
    ADD COLUMN notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (notice_kind IN ('warning', 'halt'));

DROP INDEX IF EXISTS idx_treasury_notices_once;

-- The once-per-window invariant, now per notice kind: at most one notice per (scope, tenant, key,
-- limit, window, ceiling, kind). `api_key_id` is NOT NULL so a tenant-scope notice (`''`)
-- conflicts like any other.
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos, notice_kind);
