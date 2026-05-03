-- cell_event_outbox — durable queue for base→cell content notifications.
--
-- Bridges the gap between "base committed a DB change" and "cell fires the
-- corresponding content event over the in-process mpsc channel". Without
-- this, an mpsc send failure (receiver dropped — cell task panic / shut
-- down, or panic between commit and send) would silently strand chains
-- that gate on the event — most notably mission progression on
-- `OnItemUse`. See issue #96.
--
-- (Note: `tokio::mpsc::Sender::send().await` does not error on a full
-- channel — it backpressures. The failure mode this protects against is
-- specifically a torn-down receiver, not saturation.)
--
-- Lifecycle:
--   1. Base writes a row inside the same tx as (or alongside) the
--      DB-visible state change.
--   2. Base immediately attempts an in-process `cell_tx.send(...)`.
--   3. On success → row is marked `delivered_at = NOW()`.
--   4. On failure (receiver dropped) → row stays undelivered;
--      a background drainer retries on a periodic timer + on startup.
--
-- The cell-side handler must be idempotent because retries can fan out
-- after a crash + restart. `OnItemUse` already is — chain conditions like
-- `step_status = active` self-gate, and the chain's own
-- `Action::RemoveItem` is the consumption path.
--
-- attempts/last_error are advisory diagnostics for operators; the drainer
-- does not gate retries on attempts today (single-operator deployment).
-- A future hardening step can add a dead-letter threshold here.
CREATE TABLE cell_event_outbox (
    id              BIGSERIAL PRIMARY KEY,
    entity_id       INTEGER     NOT NULL,
    event_type      VARCHAR(48) NOT NULL,
    payload         JSONB       NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at    TIMESTAMPTZ,
    attempts        INTEGER     NOT NULL DEFAULT 0,
    last_error      TEXT
);

-- Drainer scans only undelivered rows, ordered by id. Partial index keeps
-- the working set small even after months of delivered traffic.
CREATE INDEX idx_cell_event_outbox_undelivered
    ON cell_event_outbox (id) WHERE delivered_at IS NULL;
