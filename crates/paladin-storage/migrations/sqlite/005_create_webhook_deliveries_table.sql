-- Migration: Create Webhook Deliveries Table (SQLite)
-- Purpose: Persist a durable, bounded-retry webhook delivery queue (PLAT-FR-14/15, D-40..D-43)
-- Version: 005
-- Date: 2026-09-08
--
-- `webhook_deliveries` is D-40's whole durability story: a delivery is
-- CLAIMED by the conditional update `UPDATE webhook_deliveries SET
-- status = 'in_flight', updated_at = ? WHERE delivery_id = ? AND status IN
-- ('pending', 'retrying') AND next_attempt_at <= ?` -- exactly one drain
-- service instance's update affects a row, and only that instance sends the
-- HTTP request. A restart between enqueue and send loses nothing: the row
-- is still `pending`/`retrying` on disk, waiting to be claimed.
--
-- No signing-key column exists on this table (prohibition P1): the HMAC
-- signing value lives on the run's own `webhook` column (see
-- `002_create_runs_table.sql`) and is read from there at send time.
--
-- `payload` is the EXACT JSON string that was signed and sent -- never
-- re-serialized (D-41) -- so a redelivered attempt reuses the same bytes.

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    delivery_id           TEXT PRIMARY KEY NOT NULL,
    run_id                TEXT NOT NULL,
    thread_id             TEXT NOT NULL,
    event                 TEXT NOT NULL,
    url                   TEXT NOT NULL,
    payload               TEXT NOT NULL,
    attempt               INTEGER NOT NULL DEFAULT 0,
    max_attempts          INTEGER NOT NULL DEFAULT 5,
    status                TEXT NOT NULL,
    next_attempt_at       TEXT NOT NULL,
    last_response_status  INTEGER NULL,
    last_error            TEXT NULL,
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
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
