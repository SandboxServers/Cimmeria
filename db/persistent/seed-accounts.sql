-- Step 5a of the rebuild. Runs after the carry-over of player data: inserts every
-- seed account that is missing, never changes or removes a carried account.
-- A seed row blocked by an existing id or name is logged and skipped.
-- Needs cimmeria_meta (meta.sql), cimmeria_seed.account (seed-accounts-stage.sql)
-- and SET cimmeria.image_id. Plain SQL, no transaction control.
DO $seed_accounts$
DECLARE
    img       text := current_setting('cimmeria.image_id');
    s         record;
    id_hit    boolean;
    name_hit  boolean;
    same_row  boolean;
    blk       text[];
BEGIN
    IF to_regclass('cimmeria_seed.account') IS NULL THEN
        RAISE EXCEPTION 'seed-accounts: cimmeria_seed.account is missing; run seed-accounts-stage.sql first';
    END IF;
    FOR s IN SELECT * FROM cimmeria_seed.account ORDER BY account_id LOOP
        id_hit   := EXISTS (SELECT 1 FROM account a WHERE a.account_id = s.account_id);
        name_hit := EXISTS (SELECT 1 FROM account a WHERE a.account_name = s.account_name);
        same_row := EXISTS (SELECT 1 FROM account a
                            WHERE a.account_id = s.account_id AND a.account_name = s.account_name);
        IF same_row THEN
            CONTINUE;  -- already there (normal rebuild): silent no-op, never changed
        ELSIF id_hit OR name_hit THEN
            blk := ARRAY[]::text[];
            IF id_hit THEN blk := blk || 'id'::text; END IF;
            IF name_hit THEN blk := blk || 'name'::text; END IF;
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'skip_seed_account', 'account', jsonb_build_object(
                'seed', to_jsonb(s),
                'blocked_by', to_jsonb(blk)));
        ELSE
            INSERT INTO account
            SELECT * FROM jsonb_populate_record(NULL::account, to_jsonb(s));
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'insert_seed_account', 'account', to_jsonb(s));
        END IF;
    END LOOP;
END
$seed_accounts$;

DROP SCHEMA cimmeria_seed CASCADE;
