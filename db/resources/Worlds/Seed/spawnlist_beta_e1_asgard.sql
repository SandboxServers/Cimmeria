-- Union Beta E1: Asgard base, deck, infirmary and fire site on world 23 (Beta Site Evo 1).
-- Campaign: docs/analysis/union-zone-planning/beta-packets/UB-02bcd.md (UB-02c).
-- Anchors: docs/analysis/union-beta-e1-continuation/placement-seal.md (UB-00b).
-- Spawn ids 23060-23071 are inside Beta's block (23000-23199). Mimir and the
-- deck objects (nav 355) are reachable only by beam; the ground rows sit on
-- nav 251. The footer at the end keeps the default-id sequence above the
-- whole Beta block. Tags start with BetaE1_ (unique across worlds).

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: anchor MIMIR from placement-seal.md (MIMIR pin on nav 355, deck; beam only)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23060, 696.440, 157.77, 67.360, 0.927, 23, 2305, 'BetaE1_Mimir', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor ASGARD_SCANNER_1 from placement-seal.md (ASGARD_SCANNER_1 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23061, 740.480, -19.43, 68.640, 1.577, 23, 2330, 'BetaE1_AsgardScanner_1', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor ASGARD_SCANNER_2 from placement-seal.md (ASGARD_SCANNER_2 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23062, 746.480, -19.54, 62.640, 0.003, 23, 2330, 'BetaE1_AsgardScanner_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor ASGARD_SCANNER_3 from placement-seal.md (ASGARD_SCANNER_3 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23063, 746.480, -19.80, 74.640, 3.138, 23, 2330, 'BetaE1_AsgardScanner_3', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor ASGARD_SCANNER_4 from placement-seal.md (ASGARD_SCANNER_4 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23064, 752.480, -20.20, 68.640, 4.706, 23, 2330, 'BetaE1_AsgardScanner_4', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor ASGARD_SCANNER_5 from placement-seal.md (ASGARD_SCANNER_5 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23065, 740.480, -19.43, 62.640, 0.790, 23, 2330, 'BetaE1_AsgardScanner_5', NULL, true, 30);

-- CIMMERIA_RECONSTRUCTION: anchor ANALYTICAL_MACHINE from placement-seal.md (ANALYTICAL_MACHINE pin on nav 355, deck)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23066, 696.440, 157.77, 64.360, 0.588, 23, 2331, 'BetaE1_AnalyticalMachine', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor CONSCIOUSNESS_SIM from placement-seal.md (CONSCIOUSNESS_SIM pin on nav 355, deck)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23067, 699.440, 157.93, 67.360, 0.322, 23, 2334, 'BetaE1_ConsciousnessSimulator', NULL, true, 30);

-- CIMMERIA_RECONSTRUCTION: anchor HOLOGRAM_EMITTER from placement-seal.md (HOLOGRAM_EMITTER pin on nav 251, EM-Infirmary01 interior, HELD-B3)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23068, 961.300, 15.97, 284.900, 5.016, 23, 2332, 'BetaE1_HologramEmitter', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor DISABLED_DRONE from placement-seal.md (DISABLED_DRONE pin on nav 251, EM-Infirmary01 interior, HELD-B3)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23069, 959.800, 15.97, 284.900, 5.078, 23, 2333, 'BetaE1_DisabledDrone', NULL, true, 30);

-- CIMMERIA_RECONSTRUCTION: anchor FIRE_SUPPRESSANT_MANAGER from placement-seal.md (FIRE_SUPPRESSANT_MANAGER pin on nav 251, Quartermaster fire cluster)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23070, 1019.327, 13.37, 271.180, 1.564, 23, 2328, 'BetaE1_FireSuppressantManager', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor FIRE_CONTROL_HUB from placement-seal.md (FIRE_CONTROL_HUB pin on nav 251, Quartermaster fire cluster)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23071, 1022.327, 13.37, 268.180, 6.274, 23, 2329, 'BetaE1_FireControlHub', NULL, true, 30);

-- Spawn-sequence footer: Beta's block top (23199), so the next default id
-- lands above the whole Beta block and every seeded row.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 23199), true);
