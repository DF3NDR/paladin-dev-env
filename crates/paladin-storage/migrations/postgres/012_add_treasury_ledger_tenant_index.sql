-- Migration: Add Treasury Ledger Tenant-Window Index (PostgreSQL)
-- Purpose: Serve tenant-wide allowance balance sums (ALLOW-01, Phase 41, RESEARCH Pitfall 14)
-- Version: 012
-- Date: 2026-10-04
--
-- Same index, for the same reason, as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/012_add_treasury_ledger_tenant_index.sql) -- see that
-- file's header. Additive only: `007_create_treasury_ledger_table.sql` is byte-untouched and its
-- final schema (ADR-0053) is unchanged. `attributed_at` is `TIMESTAMPTZ` on this backend; the
-- index definition is otherwise identical. Derived data: it can be dropped without data loss.

CREATE INDEX IF NOT EXISTS idx_treasury_ledger_tenant_window ON treasury_ledger (tenant_id, attributed_at);
