-- Migration: Create Treasury Notices Table (SQLite)
-- Purpose: Store-enforced once-per-window allowance warn notices (ALLOW-04, Phase 41 D-15/D-16, C5)
-- Version: 011
-- Date: 2026-10-04
--
-- `treasury_notices` records one row per allowance warn crossing so that "one notice per scope,
-- limit, window and ceiling" holds across every replica sharing the database. The dedup is
-- enforced HERE, by the unique index below plus `INSERT ... ON CONFLICT DO NOTHING` (zero rows
-- affected = the identity was already recorded) -- never by an in-process memory of what was
-- already sent (D-16, mirroring 007's settlement idempotency, D-06).
--
-- Columns:
-- - `notice_id`: the row's own identifier (a UUIDv7 string per claim attempt); an abandoned
--   admission discards exactly the ids it won.
-- - `scope_kind`: `'tenant'` or `'api_key'` -- which identity holds the ceiling.
-- - `tenant_id`: the tenant the notice is held against (a log-safe identifier, D-00g).
-- - `api_key_id`: the API key NAME, NOT NULL, with the empty string `''` for tenant scope. A
--   NULL in a UNIQUE key never conflicts (verified on SQLite 3.45 -- two inserts with a NULL
--   key column both succeed -- and PostgreSQL treats NULLs as distinct before the PG15 opt-in
--   `NULLS NOT DISTINCT`), so a nullable column here would silently allow one tenant-scope
--   notice per claim instead of per window (RESEARCH C5). Real key names are non-empty, so the
--   sentinel can never collide with a real key.
-- - `limit_kind`: `'window'` or `'lifetime'`.
-- - `window_start`: the window's inclusive start, TEXT RFC 3339 through
--   `crate::run::storage_timestamp` like every other persisted instant. A lifetime notice has
--   no window, but the identity still needs a non-null value, so it is stored as the Unix epoch
--   (`1970-01-01T00:00:00Z`); the adapters read both window bounds back as `None` for
--   `limit_kind = 'lifetime'`.
-- - `window_end`: the window's exclusive end, NULL for a lifetime notice. Not part of the
--   identity; kept so the trace event, herald line and operator webhook can be rebuilt from the
--   row without re-deriving the window (41-07, 41-08).
-- - `ceiling_nanos`: the ceiling the balance was measured against, in nano-units. PART of the
--   identity: raising a ceiling re-arms the notice for the same window (D-16).
-- - `currency`, `balance_nanos`: the ceiling's currency and the pre-admission balance that
--   crossed the threshold.
-- - `warn_at`: the configured threshold percent in force when the row was written (0..=100).
--   NOT part of the identity -- changing `warn_at` alone never re-arms a notice.
-- - `run_id`: the admitting run (NULL on the HTTP agent path, which has no run row). The notice
--   is claimed BEFORE the run is inserted so a worker can always read it back; a crash between
--   the claim and the insert can therefore leave a row whose `run_id` no `runs` row carries --
--   at most once per window is the guarantee, never a duplicate (41-01 checkpoint, item 5).
-- - `recorded_at`: when the row was written (store clock, whole seconds).
-- - `schema_version`: the adapter's row schema version, `v1`.
--
-- The table is append-only apart from the `discard` of an ABANDONED admission's own rows (a run
-- that was admitted but never persisted gives its notice back so the next admission re-wins it).
--
-- Indexes:
-- - `idx_treasury_notices_once`: the dedup invariant itself -- UNIQUE over
--   `(scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos)`. Every
--   adapter's `NOTICE_INSERT` `ON CONFLICT (...)` column list MUST textually match this index's
--   column list, or SQLite/PostgreSQL cannot infer it as the conflict target; the adapters'
--   `notice_arbiter_matches_the_migration` tests keep the two in sync (Pitfall 3).
-- - `idx_treasury_notices_run`: serves `notices_for_run` (the worker's first-dispatch read);
--   partial, since agent-path notices carry no run.

CREATE TABLE IF NOT EXISTS treasury_notices (
    notice_id       TEXT PRIMARY KEY NOT NULL,
    scope_kind      TEXT NOT NULL CHECK (scope_kind IN ('tenant', 'api_key')),
    tenant_id       TEXT NOT NULL,
    api_key_id      TEXT NOT NULL,
    limit_kind      TEXT NOT NULL CHECK (limit_kind IN ('window', 'lifetime')),
    window_start    TEXT NOT NULL,
    window_end      TEXT NULL,
    ceiling_nanos   BIGINT NOT NULL,
    currency        TEXT NOT NULL,
    balance_nanos   BIGINT NOT NULL,
    warn_at         INTEGER NOT NULL CHECK (warn_at BETWEEN 0 AND 100),
    run_id          TEXT NULL,
    recorded_at     TEXT NOT NULL,
    schema_version  TEXT NOT NULL
);

-- The once-per-window invariant: at most one notice per (scope, tenant, key, limit, window,
-- ceiling). `api_key_id` is NOT NULL so a tenant-scope notice (`''`) conflicts like any other.
CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_notices_once ON treasury_notices (scope_kind, tenant_id, api_key_id, limit_kind, window_start, ceiling_nanos);

-- Serves `TreasuryNoticePort::notices_for_run`.
CREATE INDEX IF NOT EXISTS idx_treasury_notices_run
ON treasury_notices (run_id) WHERE run_id IS NOT NULL;
