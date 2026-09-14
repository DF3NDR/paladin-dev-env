-- Migration: Create Webhook Deliveries Table (PostgreSQL)
-- Purpose: Persist a durable, bounded-retry webhook delivery queue (PLAT-FR-14/15, D-40..D-43)
-- Version: 005
-- Date: 2026-09-08
--
-- Same logical schema as the SQLite sibling migration
-- (crates/paladin-storage/migrations/sqlite/005_create_webhook_deliveries_table.sql),
-- except the timestamp columns are native TIMESTAMPTZ. `payload` stays
-- plain TEXT (not JSONB) on both backends -- it is the EXACT byte buffer
-- that was signed and sent (D-41), not a value to be queried by shape.
--
-- No signing-key column exists on this table (prohibition P1) -- see the
-- SQLite migration's header comment for the full rationale.

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    delivery_id           TEXT PRIMARY KEY,
    run_id                TEXT NOT NULL,
    thread_id             TEXT NOT NULL,
    event                 TEXT NOT NULL,
    url                   TEXT NOT NULL,
    payload               TEXT NOT NULL,
    attempt               INTEGER NOT NULL DEFAULT 0,
    max_attempts          INTEGER NOT NULL DEFAULT 5,
    status                TEXT NOT NULL,
    next_attempt_at       TIMESTAMPTZ NOT NULL,
    last_response_status  INTEGER NULL,
    last_error            TEXT NULL,
    created_at            TIMESTAMPTZ NOT NULL,
    updated_at            TIMESTAMPTZ NOT NULL,
    schema_version        TEXT NOT NULL
);

-- Serves `WebhookDeliveryRepositoryPort::claim_due`'s documented
-- `(status IN ('pending','retrying'), next_attempt_at <= now)` filter.
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_due
ON webhook_deliveries(status, next_attempt_at);

-- Serves `WebhookDeliveryRepositoryPort::list_for_run`'s documented
-- `(run_id, created_at DESC)` filter/ordering.
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_run
ON webhook_deliveries(run_id, created_at DESC);
