-- Union Beta E1: life spawns on world 23 (Beta Site Evo 1), UB-40. Background
-- figures only: SGC personnel packing up the base (template 2308) and the Nine
-- Peaks camp's non-combatants (template 2309). No dialog, no chain, no bind.
-- Anchors: docs/analysis/union-beta-e1-continuation/placement-seal.md (UB-00b,
-- with addenda UB-00b-2 and UB-00b-3). Spawn ids 23160-23172 are in Beta's
-- block; tags start with BetaE1_Life_ and are unique across worlds.
-- Every row is at a sealed LIFE_* anchor and is walk-reachable (see life.rs).
-- The footer at the end keeps the default-id sequence above the Beta block.

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: anchor LIFE_GATE_1..2 from placement-seal.md (gate plaza, PlayerStart side); tone: Carter 3223/98237 and Mimir 1953/95515 (base closing)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23160, 626.000, -60.83, 399.000, 2.941, 23, 2308, 'BetaE1_Life_Gate_1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23161, 642.500, -60.83, 374.500, 5.198, 23, 2308, 'BetaE1_Life_Gate_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor LIFE_CMD_1..2 from placement-seal.md (EM-CommandPost01, cover cell (1000, 500)); tone: Carter 3223/98237 and Mimir 1953/95515 (base closing)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23162, 1000.000, 13.57, 500.000, 1.353, 23, 2308, 'BetaE1_Life_Cmd_1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23163, 1000.000, 13.57, 497.000, 1.196, 23, 2308, 'BetaE1_Life_Cmd_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor LIFE_HUT_1..2 from placement-seal.md (EM-Hut_Med01 barracks); tone: Carter 3223/98237 and Mimir 1953/95515 (base closing)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23164, 1033.700, 7.37, 361.349, 0.994, 23, 2308, 'BetaE1_Life_Hut_1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23165, 1033.700, 7.37, 424.563, 0.974, 23, 2308, 'BetaE1_Life_Hut_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor LIFE_QM_1..2 from placement-seal.md (Quartermaster crates, outside the FireSite region); tone: Carter 3223/98237 and Mimir 1953/95515 (base closing)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23166, 1002.433, 13.44, 281.388, 2.045, 23, 2308, 'BetaE1_Life_QM_1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23167, 999.113, 13.57, 273.732, 1.680, 23, 2308, 'BetaE1_Life_QM_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor LIFE_SCI_1 from placement-seal.md (Science Facility front, reached by the S1 hop); tone: Carter 3223/98237 and Mimir 1953/95515 (base closing)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23168, 622.600, 20.64, 777.000, 0.000, 23, 2308, 'BetaE1_Life_Sci_1', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor LIFE_CAMP_1..3 from placement-seal.md (west camp, Ras'nor's nav 70; reached by the ring pair, not ARRIVAL); tone: Carter 3223/98237 and Mimir 1953/95515 (base closing)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23170, -557.100, 6.77, 145.500, 1.893, 23, 2309, 'BetaE1_Life_Camp_1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23171, -562.100, 6.77, 150.500, 2.058, 23, 2309, 'BetaE1_Life_Camp_2', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23172, -555.100, 6.77, 142.500, 1.670, 23, 2309, 'BetaE1_Life_Camp_3', NULL, true, 30);

-- Spawn-sequence footer: Beta's block top (23199), so the next default id
-- lands above the whole Beta block and every seeded row.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 23199), true);
