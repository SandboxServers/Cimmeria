--
-- NEW CONTENT (Debug Area, DA-09): the System Lords' summit, world 1300.
-- docs/content/debug-area.md#system-lords-summit has the seating and the
-- chatter.
--
-- Id block 13850-13869 (DA-09). Tags `DebugArea_Lords_*` (D-DA6); the chatter
-- lines name their speakers by these tags.
--
-- Centre (282.0, 6.9, -944.0): the east side of the south compound's paved
-- courtyard, 30 m east of the services plaza and 36 m north-east of the Z1
-- arrival. Paved floor (occluder geometry at 6.9, the navmesh 0.2-0.3 m
-- above it) with nothing at head height for 7.5 m round, found by a grid
-- search of ihpet_crater_light.nav and .occ. The palace terrace north of the
-- gallery was tried first and dropped after the lab check: its terrain draws
-- white with magenta streaks in this client and the terrace beyond it is
-- unfinished. Six lords stand on a 4.5 m ring, 60 degrees apart, each facing
-- the centre: Ra to the north (+z) opposite Ba'al, Anat and Nerus on the east
-- side, Morrigan and Athena on the west. Ra's Jaffa stands 3 m behind his
-- lord. Heading 0 faces +z. Reach it with
-- `.gotolocation DebugArea 282 7.2 -953` (behind Ba'al, facing Ra).
--
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13850, 282.00, 6.90, -939.50, 3.1416, 1300, 1400, 'DebugArea_Lords_Ra', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13851, 285.90, 6.90, -941.75, -2.0944, 1300, 1402, 'DebugArea_Lords_Anat', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13852, 285.90, 6.90, -946.25, -1.0472, 1300, 1405, 'DebugArea_Lords_Nerus', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13853, 282.00, 6.90, -948.50, 0.0000, 1300, 1401, 'DebugArea_Lords_Baal', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13854, 278.10, 6.90, -946.25, 1.0472, 1300, 1403, 'DebugArea_Lords_Athena', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13855, 278.10, 6.90, -941.75, 2.0944, 1300, 1404, 'DebugArea_Lords_Morrigan', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13856, 282.00, 6.90, -936.50, 3.1416, 1300, 1406, 'DebugArea_Lords_RaJaffa', NULL, true);

-- Keep default-id inserts (`.savespawn`) past DA-09's reserved block
-- 13850-13869.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 13869), true);
