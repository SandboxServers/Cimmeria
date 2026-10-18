-- cimmeria_meta: bookkeeping for the persistent-player-data rebuild (campaign
-- persistent-player-data). Never referenced from db/database.sql. Runs as the DB
-- owner; safe to run twice. Plain SQL: no psql meta-commands, no transaction control.
CREATE SCHEMA IF NOT EXISTS cimmeria_meta;

CREATE TABLE IF NOT EXISTS cimmeria_meta.db_state (
    id                   smallint PRIMARY KEY CHECK (id = 1),
    state                text NOT NULL CHECK (state IN ('baked', 'live')),
    image_id             text NOT NULL,
    db_source_sha256     text NOT NULL,
    first_seeded_at      timestamptz,
    last_rebuild_at      timestamptz,
    rebuild_count        integer NOT NULL DEFAULT 0,
    last_failed_image_id text,
    last_failed_at       timestamptz,
    last_restore_sha256  text
);

-- action values:
--   'delete_dangling_item'  item row whose type_id is missing from resources.items (full row in detail)
--   'relocate_player'       player moved out of a removed world (old and new location in detail)
--   'count_dangling_ref'    non-FK id column with ids missing from resources (counted, never deleted)
--   'sequence_fixup'        owned sequence raised to max(column)
--   'insert_seed_account'   seed account inserted by seed-accounts.sql
--   'skip_seed_account'     seed account already present, left alone
CREATE TABLE IF NOT EXISTS cimmeria_meta.reconcile_log (
    id         bigserial PRIMARY KEY,
    logged_at  timestamptz NOT NULL DEFAULT now(),
    image_id   text NOT NULL,
    action     text NOT NULL,
    table_name text NOT NULL,
    detail     jsonb NOT NULL
);