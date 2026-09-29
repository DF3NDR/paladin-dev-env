-- Migration: Enforce All-Or-None Run Attribution (PostgreSQL)
-- Purpose: A run row is either fully attributed (tenant_id AND api_key_id set) or fully
--          unattributed (both NULL) -- never half-attributed (phase 40 review WR-03).
-- Version: 009
-- Date: 2026-09-29
--
-- Migration 008 added `tenant_id` and `api_key_id` as two independent nullable columns, so
-- the schema alone allowed a half-attributed row. `row_to_run` rejects such a row on read
-- (`RunRepositoryError::Serialization`), which the HTTP layer renders as a generic `500` --
-- turning one corrupt row into an existence oracle for that run and a failed `GET /runs` page
-- for its tenant. This constraint stops the row from ever being written.
--
-- A NEW file rather than an edit of 008: migration 008 is already committed and pushed, and
-- this workspace's migrations are append-only (sqlx records a checksum per applied version).
--
-- SQLite deliberately has no counterpart: `ALTER TABLE ... ADD COLUMN` cannot add a
-- cross-column CHECK, and rebuilding the `runs` table is not worth it for a guard the read
-- path already enforces. The SQLite adapter relies on that read-time guard alone.

ALTER TABLE runs
    ADD CONSTRAINT runs_attribution_all_or_none
    CHECK ((tenant_id IS NULL) = (api_key_id IS NULL));
