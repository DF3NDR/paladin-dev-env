-- Migration: Add Treasury Ledger Tenant-Window Index (SQLite)
-- Purpose: Serve tenant-wide allowance balance sums (ALLOW-01, Phase 41, RESEARCH Pitfall 14)
-- Version: 012
-- Date: 2026-10-04
--
-- Additive index only: `007_create_treasury_ledger_table.sql` is byte-untouched and its final
-- schema (ADR-0053) is unchanged. A tenant-wide allowance ceiling is checked at admission with
-- `balance`, whose tenant-scope form is `SUM(amount_nanos) ... WHERE tenant_id = ? AND
-- attributed_at >= ? AND attributed_at < ?` with NO `api_key_id` predicate. `007`'s
-- `idx_treasury_ledger_scope_window` is keyed `(tenant_id, api_key_id, attributed_at)`, so for
-- such a query it can only prefix-scan on `tenant_id` and must then visit every key's rows to
-- filter the window; `(tenant_id, attributed_at)` lets the planner range-scan exactly the
-- window's rows for the tenant. The per-key form keeps using `007`'s index.
--
-- The index is derived data: it holds no information a table scan could not recompute, so it can
-- be dropped without data loss. `CREATE INDEX IF NOT EXISTS` makes the migration idempotent.

CREATE INDEX IF NOT EXISTS idx_treasury_ledger_tenant_window ON treasury_ledger (tenant_id, attributed_at);
