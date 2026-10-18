-- Smoke for db/persistent/seed-accounts-stage.sql and seed-accounts.sql (PP-17).
-- Runs against a scratch copy of account on search_path; rolls back.
\set ON_ERROR_STOP on
BEGIN;
--#include meta.sql
SET LOCAL cimmeria.image_id = 'pp17-smoke';
CREATE SCHEMA pp17_probe;
CREATE TABLE pp17_probe.account (LIKE public.account INCLUDING DEFAULTS);
SET LOCAL search_path = pp17_probe, public;
-- Simulate the fresh seed load: the seed rows only.
INSERT INTO pp17_probe.account SELECT * FROM public.account WHERE account_id BETWEEN 2 AND 15;
--#include seed-accounts-stage.sql

DO $$
BEGIN
    IF (SELECT count(*) FROM cimmeria_seed.account) = 0
       OR (SELECT count(*) FROM cimmeria_seed.account)
          <> (SELECT count(*) FROM public.account WHERE account_id BETWEEN 2 AND 15) THEN
        RAISE EXCEPTION 'stage: cimmeria_seed.account must hold exactly the seed rows, got %',
            (SELECT count(*) FROM cimmeria_seed.account);
    END IF;
    IF (SELECT count(*) FROM pp17_probe.account) <> 0 THEN
        RAISE EXCEPTION 'stage: account must be empty after the stage';
    END IF;
END $$;

-- Carry-over simulation.
-- Carry every seed row except ids 5, 6, 7 and 8 unchanged.
INSERT INTO pp17_probe.account SELECT * FROM cimmeria_seed.account WHERE account_id NOT IN (5, 6, 7, 8);
-- Id 5 carried with a changed value (accesslevel, the sixth column).
INSERT INTO pp17_probe.account
SELECT account_id, account_name, password, password_hash_v2, password_algo, 99, enabled, test_account
FROM cimmeria_seed.account WHERE account_id = 5;
-- Id 6 missing on purpose: the top-up must restore it.
-- Id 7 held by a different name: seed row 7 is blocked by id.
INSERT INTO pp17_probe.account
SELECT account_id, 'pp17-sentinel-a', password, password_hash_v2, password_algo, accesslevel, enabled, test_account
FROM cimmeria_seed.account WHERE account_id = 7;
-- Id 8's name held by another id: seed row 8 is blocked by name; id 8 is absent.
INSERT INTO pp17_probe.account
SELECT 2147000020, account_name, password, password_hash_v2, password_algo, accesslevel, enabled, test_account
FROM cimmeria_seed.account WHERE account_id = 8;

--#include seed-accounts.sql

DO $$
DECLARE
    seed6 record;
BEGIN
    SELECT * INTO seed6 FROM public.account WHERE account_id = 6;
    IF NOT EXISTS (SELECT 1 FROM pp17_probe.account a
                   WHERE a.account_id = 6 AND a.account_name = seed6.account_name
                     AND a.accesslevel = seed6.accesslevel) THEN
        RAISE EXCEPTION 'top-up: account 6 was not restored with its seed values';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pp17_probe.account WHERE account_id = 5 AND accesslevel = 99) THEN
        RAISE EXCEPTION 'top-up: account 5 must keep accesslevel 99 (never upserted)';
    END IF;
    IF (SELECT count(*) FROM pp17_probe.account WHERE account_id = 7) <> 1
       OR NOT EXISTS (SELECT 1 FROM pp17_probe.account
                      WHERE account_id = 7 AND account_name = 'pp17-sentinel-a') THEN
        RAISE EXCEPTION 'top-up: account 7 must stay the sentinel-a row';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pp17_probe.account WHERE account_id = 2147000020) THEN
        RAISE EXCEPTION 'top-up: account 2147000020 vanished';
    END IF;
    IF EXISTS (SELECT 1 FROM pp17_probe.account WHERE account_id = 8) THEN
        RAISE EXCEPTION 'top-up: account 8 must not be inserted (name blocked)';
    END IF;
    IF (SELECT count(*) FROM cimmeria_meta.reconcile_log
        WHERE image_id = 'pp17-smoke' AND action = 'insert_seed_account'
          AND (detail->>'account_id')::int = 6) <> 1
       OR (SELECT count(*) FROM cimmeria_meta.reconcile_log
           WHERE image_id = 'pp17-smoke' AND action = 'insert_seed_account') <> 1 THEN
        RAISE EXCEPTION 'log: expected exactly one insert_seed_account for id 6';
    END IF;
    IF (SELECT count(*) FROM cimmeria_meta.reconcile_log
        WHERE image_id = 'pp17-smoke' AND action = 'skip_seed_account') <> 2
       OR (SELECT count(*) FROM cimmeria_meta.reconcile_log
           WHERE image_id = 'pp17-smoke' AND action = 'skip_seed_account'
             AND detail->'blocked_by' = '["id"]'::jsonb
             AND detail->'seed'->>'account_id' = '7') <> 1
       OR (SELECT count(*) FROM cimmeria_meta.reconcile_log
           WHERE image_id = 'pp17-smoke' AND action = 'skip_seed_account'
             AND detail->'blocked_by' = '["name"]'::jsonb
             AND detail->'seed'->>'account_id' = '8') <> 1 THEN
        RAISE EXCEPTION 'log: expected two skip_seed_account rows (id 7 by id, id 8 by name)';
    END IF;
    IF (SELECT count(*) FROM cimmeria_meta.reconcile_log
        WHERE image_id = 'pp17-smoke'
          AND action NOT IN ('insert_seed_account', 'skip_seed_account')) <> 0 THEN
        RAISE EXCEPTION 'log: unexpected extra actions';
    END IF;
    IF to_regnamespace('cimmeria_seed') IS NOT NULL THEN
        RAISE EXCEPTION 'drop: cimmeria_seed must be gone';
    END IF;
    IF (SELECT count(*) FROM pp17_probe.account) <> 14 THEN
        RAISE EXCEPTION 'final: expected 14 account rows, got %', (SELECT count(*) FROM pp17_probe.account);
    END IF;
END $$;

ROLLBACK;
