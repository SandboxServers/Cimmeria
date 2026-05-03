-- Migration: add cell_event_outbox table for durable base→cell content events.
-- Safe to run on existing databases (uses IF NOT EXISTS).
-- See db/sgw/Outbox/Tables/cell_event_outbox.sql for the canonical definition
-- and rationale.

CREATE TABLE IF NOT EXISTS cell_event_outbox (
    id              BIGSERIAL PRIMARY KEY,
    entity_id       INTEGER     NOT NULL,
    event_type      VARCHAR(48) NOT NULL,
    payload         JSONB       NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at    TIMESTAMPTZ,
    attempts        INTEGER     NOT NULL DEFAULT 0,
    last_error      TEXT
);

CREATE INDEX IF NOT EXISTS idx_cell_event_outbox_undelivered
    ON cell_event_outbox (id) WHERE delivered_at IS NULL;
