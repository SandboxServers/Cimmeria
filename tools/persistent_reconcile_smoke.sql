-- persistent_reconcile_smoke.sql (PP-01): db/persistent/reconcile.sql against
-- sentinel dangling data. The Rust harness replaces each `--#include <file>`
-- line with that file's text (the SQL cannot \ir). Rolls back at the end.
\set ON_ERROR_STOP on
BEGIN;

--#include meta.sql

SET LOCAL cimmeria.image_id = 'pp01-smoke';

-- Scratch copies of the public tables, first on search_path, so the shared test
-- tables are never locked or modified. sgw_inventory INHERITS sgw_inventory_base
-- like the real schema (LIKE merges the shared base columns).
CREATE SCHEMA pp01_probe;
CREATE TABLE pp01_probe.sgw_inventory_base (LIKE public.sgw_inventory_base INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_inventory (LIKE public.sgw_inventory INCLUDING DEFAULTS)
    INHERITS (pp01_probe.sgw_inventory_base);
CREATE TABLE pp01_probe.sgw_gate_mail_item (LIKE public.sgw_gate_mail_item INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_organization_vault_items (LIKE public.sgw_organization_vault_items INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_player (LIKE public.sgw_player INCLUDING DEFAULTS);
CREATE TABLE pp01_probe.sgw_mission (LIKE public.sgw_mission INCLUDING DEFAULTS);
SET LOCAL search_path = pp01_probe, public;

-- 1a. Dangling item in the inheritance child.
INSERT INTO pp01_probe.sgw_inventory (character_id, type_id, stack_size, slot_id, container_id,
                                      bound, durability, charges, ammo, cur_ammo_type, item_id)
VALUES (2147000010, 2147000001, 1, 0, 1, false, -1, 0, 0, 0, 2147000020);
-- 1b. Dangling item in the inheritance base itself.
INSERT INTO pp01_probe.sgw_inventory_base (item_id, container_id, slot_id, type_id)
VALUES (2147000021, 1, 1, 2147000001);
-- 2. Dangling item in gate mail escrow.
INSERT INTO pp01_probe.sgw_gate_mail_item (mail_id, item_id, type_id, stack_size, charges, durability,
    flags, bound, ammo, cur_ammo_type, ammo_type, ammo_types, source_character_id, escrowed_at)
VALUES (2147000002, 2147000003, 2147000001, 1, 0, -1, 0, false, 0, 0,
        (enum_range(NULL::resources."EAmmoType"))[1],
        ARRAY[(enum_range(NULL::resources."EAmmoType"))[1]], 2147000006, 0);
-- 3. Dangling item in an organization vault.
INSERT INTO pp01_probe.sgw_organization_vault_items (item_id, org_id, org_type, container_id, slot_id,
    type_id, stack_size, charges, durability, flags, bound, ammo, cur_ammo_type, ammo_type, ammo_types,
    deposited_by_player_id)
VALUES (2147000004, 2147000005, 1, 19, 0, 2147000001, 1, 0, -1, 0, false, 0, 0,
        (enum_range(NULL::resources."EAmmoType"))[1],
        ARRAY[(enum_range(NULL::resources."EAmmoType"))[1]], 2147000006);
-- 4. Control row: a valid type_id must survive.
INSERT INTO pp01_probe.sgw_inventory (character_id, type_id, stack_size, slot_id, container_id,
                                      bound, durability, charges, ammo, cur_ammo_type, item_id)
VALUES (2147000010, (SELECT min(item_id) FROM resources.items), 1, 1, 1, false, -1, 0, 0, 0, 2147000022);
-- 5. Player in a removed world, with a removed stargate and ability.
INSERT INTO sgw_player (account_id, player_id, alignment, archetype, gender, player_name, extra_name,
    world_location, bodyset, pos_x, pos_y, pos_z, skin_color_id, world_id, known_stargates, abilities)
VALUES (2147000009, 2147000010, 0, 1, 1, 'pp01-sentinel', '', 'NoSuchWorld', 'BS_HumanMale.BS_HumanMale',
        1, 2, 3, 0, 2147000011, ARRAY[2147000012], ARRAY[2147000013]);
-- 6. Mission row with a removed mission and step.
INSERT INTO sgw_mission (player_id, mission_id, current_step_id, status, completed_step_ids,
    completed_objective_ids, active_objective_ids, failed_objective_ids)
VALUES (2147000010, 2147000014, 2147000015, 1, '{}', '{}', '{}', '{}');
-- 7. Sequence behind its data (DDL is rolled back).
CREATE TABLE public.zz_pp01_seq_probe (id serial PRIMARY KEY);
INSERT INTO public.zz_pp01_seq_probe SELECT generate_series(1, 5);
SELECT setval(pg_get_serial_sequence('public.zz_pp01_seq_probe', 'id'), 1, false);

--#include reconcile.sql
--#include reconcile.sql

DO $$
DECLARE
    n int;
    t text;
BEGIN
    SELECT count(*) INTO n FROM cimmeria_meta.reconcile_log
     WHERE image_id = 'pp01-smoke' AND action = 'delete_dangling_item' AND detail->>'type_id' = '2147000001';
    IF n <> 4 THEN RAISE EXCEPTION 'expected 4 delete rows, got %', n; END IF;
    FOREACH t IN ARRAY ARRAY['sgw_inventory','sgw_inventory_base','sgw_gate_mail_item','sgw_organization_vault_items'] LOOP
        SELECT count(*) INTO n FROM cimmeria_meta.reconcile_log
         WHERE image_id = 'pp01-smoke' AND action = 'delete_dangling_item'
           AND detail->>'type_id' = '2147000001' AND table_name = t;
        IF n <> 1 THEN RAISE EXCEPTION 'expected 1 delete row for %, got %', t, n; END IF;
    END LOOP;
    SELECT count(*) INTO n FROM cimmeria_meta.reconcile_log
     WHERE image_id = 'pp01-smoke' AND action = 'delete_dangling_item' AND NOT (detail ? 'item_id');
    IF n <> 0 THEN RAISE EXCEPTION 'delete log rows without a full row: %', n; END IF;
    SELECT count(*) INTO n FROM ONLY pp01_probe.sgw_inventory WHERE type_id = 2147000001;
    IF n <> 0 THEN RAISE EXCEPTION 'dangling child inventory row remains'; END IF;
    SELECT count(*) INTO n FROM ONLY pp01_probe.sgw_inventory_base WHERE type_id = 2147000001;
    IF n <> 0 THEN RAISE EXCEPTION 'dangling base inventory row remains'; END IF;
    SELECT count(*) INTO n FROM pp01_probe.sgw_gate_mail_item WHERE type_id = 2147000001;
    IF n <> 0 THEN RAISE EXCEPTION 'dangling mail item remains'; END IF;
    SELECT count(*) INTO n FROM pp01_probe.sgw_organization_vault_items WHERE type_id = 2147000001;
    IF n <> 0 THEN RAISE EXCEPTION 'dangling vault item remains'; END IF;
    SELECT count(*) INTO n FROM ONLY pp01_probe.sgw_inventory WHERE item_id = 2147000022;
    IF n <> 1 THEN RAISE EXCEPTION 'control inventory row was deleted'; END IF;

    SELECT count(*) INTO n FROM pp01_probe.sgw_player
     WHERE player_id = 2147000010 AND world_location = 'Castle_CellBlock'
       AND world_id = (SELECT world_id FROM resources.worlds WHERE world = 'Castle_CellBlock')
       AND abs(pos_x + 334.231) < 0.01;
    IF n <> 1 THEN RAISE EXCEPTION 'player was not relocated to Castle_CellBlock'; END IF;
    SELECT count(*) INTO n FROM cimmeria_meta.reconcile_log
     WHERE image_id = 'pp01-smoke' AND action = 'relocate_player'
       AND detail->>'player_id' = '2147000010' AND detail->'old'->>'world_location' = 'NoSuchWorld';
    IF n <> 1 THEN RAISE EXCEPTION 'expected 1 relocate_player row, got %', n; END IF;

    IF (SELECT known_stargates FROM pp01_probe.sgw_player WHERE player_id = 2147000010) <> ARRAY[2147000012] THEN
        RAISE EXCEPTION 'known_stargates was modified';
    END IF;
    FOREACH t IN ARRAY ARRAY['known_stargates','abilities','mission_id','current_step_id'] LOOP
        SELECT count(*) INTO n FROM cimmeria_meta.reconcile_log
         WHERE image_id = 'pp01-smoke' AND action = 'count_dangling_ref'
           AND detail->>'column' = t AND (detail->>'count')::int >= 1;
        IF n < 1 THEN RAISE EXCEPTION 'no count_dangling_ref row for %', t; END IF;
    END LOOP;

    IF nextval(pg_get_serial_sequence('public.zz_pp01_seq_probe', 'id')) <> 6 THEN
        RAISE EXCEPTION 'sequence was not fixed up to max+1';
    END IF;
    SELECT count(*) INTO n FROM cimmeria_meta.reconcile_log
     WHERE image_id = 'pp01-smoke' AND action = 'sequence_fixup' AND table_name = 'zz_pp01_seq_probe';
    IF n <> 1 THEN RAISE EXCEPTION 'expected 1 sequence_fixup row, got %', n; END IF;
END
$$;

ROLLBACK;