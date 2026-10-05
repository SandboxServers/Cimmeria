--
-- NEW CONTENT (Debug Area, DA-10): the Visual NPC Lineup's five groups.
-- docs/content/debug-area.md#visual-npc-lineup has the table.
--
-- A spawn set is the spawnlist rows whose `set_name` is its `name` in its
-- `world_id`. Its members never spawn at startup; a GM switches the whole
-- set on and off (the Lineup attendants, `.spawnset`, activateSpawnSet /
-- deactivateSpawnSet). Sets of one `type` in one world are exclusive:
-- showing one switches the others off, so at most one group (44 actors) is
-- ever loaded. The groups follow the rows' families, so each is one stretch
-- of the east wing.
--
-- Id block 1301-1399 is DA-10's (world 1300's sets).
--

SET search_path = resources, pg_catalog;

INSERT INTO spawn_sets (set_id, name, type, world_id, height, radius) VALUES (1301, 'Visual NPC Lineup - Humans', 'visual_lineup', 1300, NULL, NULL); -- 42 actors, spawns 13870-13911
INSERT INTO spawn_sets (set_id, name, type, world_id, height, radius) VALUES (1302, 'Visual NPC Lineup - Jaffa male', 'visual_lineup', 1300, NULL, NULL); -- 44 actors, spawns 13912-13955
INSERT INTO spawn_sets (set_id, name, type, world_id, height, radius) VALUES (1303, 'Visual NPC Lineup - Jaffa female', 'visual_lineup', 1300, NULL, NULL); -- 30 actors, spawns 13956-13985
INSERT INTO spawn_sets (set_id, name, type, world_id, height, radius) VALUES (1304, 'Visual NPC Lineup - Goa''uld, Asgard and children', 'visual_lineup', 1300, NULL, NULL); -- 28 actors, spawns 13986-14013
INSERT INTO spawn_sets (set_id, name, type, world_id, height, radius) VALUES (1305, 'Visual NPC Lineup - Creatures and machines', 'visual_lineup', 1300, NULL, NULL); -- 17 actors, spawns 14014-14030

-- Keep default-id inserts past DA-10's reserved block.
SELECT pg_catalog.setval('spawn_sets_set_id_seq', GREATEST((SELECT MAX(set_id) FROM spawn_sets), (SELECT last_value FROM spawn_sets_set_id_seq), 1399), true);
