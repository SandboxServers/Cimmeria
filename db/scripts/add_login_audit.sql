-- Migration: add login_audit table for login event persistence.
-- Safe to run on existing databases (uses IF NOT EXISTS).

CREATE TABLE IF NOT EXISTS login_audit (
    id            BIGSERIAL PRIMARY KEY,
    event_time    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    account_name  VARCHAR(32) NOT NULL,
    account_id    INTEGER,
    ip_address    INET NOT NULL,
    phase         VARCHAR(20) NOT NULL
        CONSTRAINT phase_check CHECK (phase IN ('credential_check', 'shard_selection')),
    outcome       VARCHAR(30) NOT NULL
        CONSTRAINT outcome_check CHECK (outcome IN (
            'success', 'invalid_credentials', 'account_disabled',
            'db_error', 'protocol_mismatch', 'no_shards'
        )),
    shard         VARCHAR(64),
    detail        TEXT
);

CREATE INDEX IF NOT EXISTS idx_login_audit_event_time ON login_audit (event_time DESC);
CREATE INDEX IF NOT EXISTS idx_login_audit_account_time ON login_audit (account_name, event_time DESC);
