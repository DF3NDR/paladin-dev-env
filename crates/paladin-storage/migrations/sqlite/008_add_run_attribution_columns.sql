-- Migration: Add Run Attribution Columns (SQLite)
-- Purpose: Record the submitting principal's tenant and API key id on every run (TENANT-02, D-08/D-09)
-- Version: 008
-- Date: 2026-09-28
--
-- NULL means "no principal recorded" (pre-v0.11 rows, schedule-fired runs, same-process
-- embedders, tests) -- never the ledger's "unattributed" string sentinel. That sentinel is a
-- treasury_ledger-row concept (`LedgerScope::UNATTRIBUTED`), never written to this table.
-- Role is never persisted here -- roles are config, not data (D-08).
--
-- The new index below is non-unique, so `idx_runs_thread_active` (migration 002) remains this
-- table's only unique index.

ALTER TABLE runs ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE runs ADD COLUMN api_key_id TEXT NULL;

-- Serves the tenant-scoped keyset list (D-12): WHERE tenant_id = ? combined with the existing
-- (submitted_at DESC, run_id DESC) ordering.
CREATE INDEX IF NOT EXISTS idx_runs_tenant_submitted
ON runs(tenant_id, submitted_at DESC, run_id DESC);
