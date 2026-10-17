-- Migration: Add Run Schedule Creator Columns (SQLite)
-- Purpose: Record the creating principal's tenant and API key name on every schedule so fired runs are attributed and admitted against it (ALLOW-02, D-08)
-- Version: 010
-- Date: 2026-10-03
--
-- NULL in both columns means "no creator recorded": a schedule created before Phase 41, or by a
-- principal-less embedder. Such a schedule fires unattributed and is never gated by an allowance
-- (recorded as an open row in .planning/WINDOWS.md). Role is never persisted here -- roles are
-- config, not data (Phase 40 D-08). This migration is append-only.
--
-- There is no SQLite `009`. The PostgreSQL-only `009_add_run_attribution_check.sql` exists
-- because SQLite cannot add a cross-column CHECK through ALTER TABLE; numbering is kept aligned
-- across the two backends, so SQLite goes 008 -> 010. The adapter rejects a half-attributed row
-- on read (`RunScheduleRepositoryError::Serialization`) instead of at write time.

ALTER TABLE run_schedules ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE run_schedules ADD COLUMN api_key_id TEXT NULL;
