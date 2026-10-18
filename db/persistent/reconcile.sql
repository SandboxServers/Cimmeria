-- reconcile.sql: make carried-over player data consistent with a freshly seeded
-- resources schema (campaign persistent-player-data, decision D-PP9).
-- Runs after player data is carried over and before PKs, FKs and indexes are
-- created, so it never relies on them and never touches them. It also works in
-- a full test DB. Public tables are named unqualified (search_path is public in
-- production); resources.* and cimmeria_meta.* are qualified. Needs
-- cimmeria_meta (meta.sql) and `SET [LOCAL] cimmeria.image_id`; an unset
-- setting raises, which is wanted. Every change is logged in
-- cimmeria_meta.reconcile_log.

-- Block 1: delete items whose type_id is missing from resources.items.
DO $reconcile_items$
DECLARE
    img text := current_setting('cimmeria.image_id');
    t   text;
BEGIN
    -- sgw_inventory INHERITS sgw_inventory_base: ONLY keeps each delete and its
    -- log row on the table that actually holds the row. The base is listed first on
    -- purpose: without ONLY it would also delete (and log, truncated to the base
    -- columns) the child rows, which the smoke detects.
    FOREACH t IN ARRAY ARRAY['sgw_inventory_base', 'sgw_inventory',
                             'sgw_gate_mail_item', 'sgw_organization_vault_items']
    LOOP
        EXECUTE format($q$
            WITH del AS (
                DELETE FROM ONLY %1$I x
                WHERE NOT EXISTS (SELECT 1 FROM resources.items i WHERE i.item_id = x.type_id)
                RETURNING x.*
            )
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            SELECT %2$L, 'delete_dangling_item', %1$L, to_jsonb(del) FROM del
        $q$, t, img);
    END LOOP;
END
$reconcile_items$;

-- Block 2: move players in removed worlds to Castle_CellBlock (fail closed if it is missing).
DO $reconcile_worlds$
DECLARE
    img       text := current_setting('cimmeria.image_id');
    fb_id     integer;
    r         record;
BEGIN
    SELECT w.world_id INTO fb_id FROM resources.worlds w WHERE w.world = 'Castle_CellBlock';
    IF fb_id IS NULL THEN
        RAISE EXCEPTION 'reconcile: fallback world Castle_CellBlock is missing from resources.worlds';
    END IF;
    FOR r IN
        SELECT p.player_id, p.world_id, p.world_location, p.pos_x, p.pos_y, p.pos_z
        FROM sgw_player p
        WHERE NOT EXISTS (SELECT 1 FROM resources.worlds w WHERE w.world = p.world_location)
           OR (p.world_id IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM resources.worlds w WHERE w.world_id = p.world_id))
        ORDER BY p.player_id
    LOOP
        UPDATE sgw_player
           SET world_id = fb_id, world_location = 'Castle_CellBlock',
               pos_x = -334.231, pos_y = 73.472, pos_z = -228.026
         WHERE player_id = r.player_id;
        INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
        VALUES (img, 'relocate_player', 'sgw_player', jsonb_build_object(
            'player_id', r.player_id,
            'old', jsonb_build_object('world_id', r.world_id, 'world_location', r.world_location,
                                      'pos_x', r.pos_x, 'pos_y', r.pos_y, 'pos_z', r.pos_z),
            'new', jsonb_build_object('world_id', fb_id, 'world_location', 'Castle_CellBlock',
                                      'pos_x', -334.231, 'pos_y', 73.472, 'pos_z', -228.026)));
    END LOOP;
END
$reconcile_worlds$;

-- Block 3: count (never delete) ids in columns that have no FK to resources.
DO $reconcile_refs$
DECLARE
    img text := current_setting('cimmeria.image_id');
    r   record;
    n   bigint;
BEGIN
    FOR r IN
        SELECT * FROM (VALUES
            ('sgw_player',  'known_stargates',   true,  'stargates',     'stargate_id'),
            ('sgw_player',  'abilities',         true,  'abilities',     'ability_id'),
            ('sgw_player',  'trained_abilities', true,  'abilities',     'ability_id'),
            ('sgw_player',  'discipline_ids',    true,  'disciplines',   'discipline_id'),
            ('sgw_player',  'blueprint_ids',     true,  'blueprints',    'blueprint_id'),
            ('sgw_player',  'known_respawners',  true,  'respawners',    'respawner_id'),
            ('sgw_mission', 'mission_id',        false, 'missions',      'mission_id'),
            ('sgw_mission', 'current_step_id',   false, 'mission_steps', 'step_id')
        ) AS v(src_table, col, is_array, res_table, res_col)
    LOOP
        IF r.is_array THEN
            EXECUTE format('SELECT count(*) FROM %I s, LATERAL unnest(s.%I) AS v(id) '
                           'WHERE NOT EXISTS (SELECT 1 FROM resources.%I r WHERE r.%I = v.id)',
                           r.src_table, r.col, r.res_table, r.res_col) INTO n;
        ELSE
            EXECUTE format('SELECT count(*) FROM %I s WHERE s.%I IS NOT NULL '
                           'AND NOT EXISTS (SELECT 1 FROM resources.%I r WHERE r.%I = s.%I)',
                           r.src_table, r.col, r.res_table, r.res_col, r.col) INTO n;
        END IF;
        IF n > 0 THEN
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'count_dangling_ref', r.src_table,
                    jsonb_build_object('column', r.col, 'resource', r.res_table, 'count', n));
        END IF;
    END LOOP;
END
$reconcile_refs$;

-- Block 4: raise every sequence owned by a public column to at least max(column).
-- (max over an inheritance parent includes its children, which is wanted. A
-- non-integer owner column raises: loud and correct.)
DO $reconcile_seq$
DECLARE
    img text := current_setting('cimmeria.image_id');
    r   record;
    mx  bigint;
    cur bigint;
BEGIN
    FOR r IN
        SELECT s.oid::regclass AS seq, t.oid::regclass AS tbl, a.attname AS col, t.relname AS tname
        FROM pg_class s
        JOIN pg_depend d ON d.classid = 'pg_class'::regclass AND d.objid = s.oid AND d.deptype = 'a'
        JOIN pg_class t ON t.oid = d.refobjid
        JOIN pg_namespace n ON n.oid = t.relnamespace AND n.nspname = 'public'
        JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = d.refobjsubid
        WHERE s.relkind = 'S'
        ORDER BY s.oid
    LOOP
        EXECUTE format('SELECT max(%I)::bigint FROM %s', r.col, r.tbl) INTO mx;
        EXECUTE format('SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM %s', r.seq) INTO cur;
        IF mx IS NOT NULL AND mx > cur THEN
            PERFORM setval(r.seq, mx, true);
            INSERT INTO cimmeria_meta.reconcile_log (image_id, action, table_name, detail)
            VALUES (img, 'sequence_fixup', r.tname,
                    jsonb_build_object('sequence', r.seq::text, 'column', r.col, 'old', cur, 'new', mx));
        END IF;
    END LOOP;
END
$reconcile_seq$;