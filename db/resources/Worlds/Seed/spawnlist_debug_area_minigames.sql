--
-- NEW CONTENT (minigames campaign, MG-74, D-MG23): the Debug Area minigame
-- terminals of world 1300 `DebugArea` (Ihpet_Crater_Light), spawns 14100-14109.
-- Templates are entity_templates_debug_area_minigames.sql (1600-1605); chains
-- are debug_area_minigames_chains.sql (24050-24057).
--
-- A row on the services plaza's north edge (spawnlist_debug_area_plaza.sql),
-- all on navmesh component 17 (nav_inspect), facing south (heading 3.1416).
-- x 252 stays free as the walkway to the dummies.
--
-- Every spawn is stationary and belongs to no mission: the terminals are
-- reusable GM test fixtures.
--

SET search_path = resources, pg_catalog;

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14100, 234.0, 6.66, -903.0, 3.1416, 1300, 1600, 'DebugArea_Minigame_CrystalGame', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14101, 239.0, 6.73, -903.0, 3.1416, 1300, 1601, 'DebugArea_Minigame_Alignment', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14102, 244.0, 6.78, -903.0, 3.1416, 1300, 1602, 'DebugArea_Minigame_GoauldCrystals', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14103, 257.0, 6.86, -903.0, 3.1416, 1300, 1603, 'DebugArea_Minigame_ConverseBasicHumanoid', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14104, 262.0, 6.90, -903.0, 3.1416, 1300, 1604, 'DebugArea_Minigame_Livewire', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14105, 267.0, 6.82, -903.0, 3.1416, 1300, 1605, 'DebugArea_Minigame_Hack', NULL, true);

-- Block 14100-14109 is reserved for the minigame terminals. Keep default-id
-- inserts (`.savespawn`) past it.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 14109), true);
