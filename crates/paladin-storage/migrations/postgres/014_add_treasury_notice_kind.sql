-- Migration: Add Treasury Notice Kind (PostgreSQL)
-- Purpose: The halt rung of the once-per-window operator notice store (ALLOW-03, Phase 42 D-18,
--          G5, ADR-0057 group g)
-- Version: 014
-- Date: 2026-10-07
--
-- Same logical change as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/014_add_treasury_notice_kind.sql) -- see that file's
-- header for the full rationale: `notice_kind` (`'warning'` or `'halt'`) is part of the dedup
-- identity so a warning and a halt notice for one scope, limit, window and ceiling are distinct
-- notices; every pre-existing row is a warning (the column default); `notices_for_run` reads
-- warning rows only; and this is a one-way change.
--
-- This file differs only in idempotency spelling: `ADD COLUMN IF NOT EXISTS` makes a re-run a
-- no-op for the column, and the index is dropped and recreated with `notice_kind` appended to its
-- column list. The `ON CONFLICT (...)` column list in `PostgresTreasuryLedger`'s `NOTICE_INSERT`
-- MUST textually match this index's column list (Pitfall 3; the adapter's
-- `notice_arbiter_matches_the_migration` test keeps the two in sync). The inline CHECK follows
-- 011's unnamed-CHECK style.

ALTER TABLE treasury_notices
    ADD COLUMN IF NOT EXISTS notice_kind TEXT NOT NULL DEFAULT 'warning' CHECK (notice_kind IN ('warning', 'halt'));

DROP INDEX IF EXISTS idx_treasury_notices_once;

CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos, notice_kind);
