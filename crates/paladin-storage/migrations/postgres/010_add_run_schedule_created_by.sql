-- Migration: Add Run Schedule Creator Columns (PostgreSQL)
-- Purpose: Record the creating principal's tenant and API key name on every schedule so fired runs are attributed and admitted against it (ALLOW-02, D-08)
-- Version: 010
-- Date: 2026-10-03
--
-- NULL in both columns means "no creator recorded": a schedule created before Phase 41, or by a
-- principal-less embedder. Such a schedule fires unattributed and is never gated by an allowance
-- (recorded as an open row in .planning/WINDOWS.md). Role is never persisted here -- roles are
-- config, not data (Phase 40 D-08). This migration is append-only.
--
-- A schedule row is either fully attributed (tenant_id AND api_key_id set) or fully
-- unattributed (both NULL) -- never half-attributed. The CHECK below stops such a row from being
-- written; the adapter also rejects one on read.

ALTER TABLE run_schedules ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE run_schedules ADD COLUMN api_key_id TEXT NULL;
ALTER TABLE run_schedules ADD CONSTRAINT run_schedules_created_by_all_or_none CHECK ((tenant_id IS NULL) = (api_key_id IS NULL));
