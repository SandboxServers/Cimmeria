-- Union Beta E1: hub cast and objects on world 23 (Beta Site Evo 1).
-- Campaign: docs/analysis/union-zone-planning/beta-packets/UB-02bcd.md (UB-02b).
-- Anchors: docs/analysis/union-beta-e1-continuation/placement-seal.md (UB-00b).
-- Spawn ids 23000-23059 are Beta's block; the footer at the end keeps the
-- default-id sequence above the whole block. Tags are unique across worlds
-- (interact_tag triggers do not filter by world) and start with BetaE1_.

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: anchor CARTER from placement-seal.md (CARTER pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23000, 1011.405, 13.57, 511.851, 2.498, 23, 2300, 'BetaE1_Carter', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor FELGER from placement-seal.md (FELGER pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23001, 1007.405, 13.57, 511.851, 2.246, 23, 2301, 'BetaE1_Felger', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor SATTERFIELD from placement-seal.md (SATTERFIELD pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23002, 1004.144, 13.44, 276.690, 1.864, 23, 2302, 'BetaE1_Satterfield', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor DEBEAUX from placement-seal.md (DEBEAUX pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23003, 1034.146, 13.38, 314.764, 3.142, 23, 2304, 'BetaE1_Debeaux', NULL, true, 30);

-- CIMMERIA_RECONSTRUCTION: anchor XENOCAGE_1 from placement-seal.md (XENOCAGE_1 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23010, 712.389, -66.99, 411.698, 1.277, 23, 2320, 'BetaE1_XenoCage_1', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor XENOCAGE_2 from placement-seal.md (XENOCAGE_2 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23011, 765.860, -63.85, 426.136, 1.277, 23, 2320, 'BetaE1_XenoCage_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor XENOCAGE_3 from placement-seal.md (XENOCAGE_3 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23012, 826.331, -36.48, 446.574, 1.277, 23, 2320, 'BetaE1_XenoCage_3', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor XENOCAGE_4 from placement-seal.md (XENOCAGE_4 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23013, 884.801, -16.69, 464.012, 1.277, 23, 2320, 'BetaE1_XenoCage_4', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor XENOCAGE_5 from placement-seal.md (XENOCAGE_5 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23014, 938.272, -4.48, 481.450, 1.277, 23, 2320, 'BetaE1_XenoCage_5', NULL, true, 30);

-- CIMMERIA_RECONSTRUCTION: anchor CRATE_ARC42 from placement-seal.md (CRATE_ARC42 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23015, 1007.015, 13.37, 275.820, 1.864, 23, 2325, 'BetaE1_Crate_Arc42', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor CRATE_ARC66 from placement-seal.md (CRATE_ARC66 pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23016, 1004.015, 13.43, 275.820, 1.818, 23, 2325, 'BetaE1_Crate_Arc66', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor HUB_COMPUTER from placement-seal.md (HUB_COMPUTER pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23017, 1018.405, 13.90, 506.851, 5.482, 23, 2322, 'BetaE1_HubComputer', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor ADMIN_WORKSTATION from placement-seal.md (ADMIN_WORKSTATION pin on nav 251)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23018, 1018.405, 13.57, 509.851, 5.262, 23, 2323, 'BetaE1_AdminWorkstation', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor SCIENCE_COMPUTER from placement-seal.md (SCIENCE_COMPUTER pin on nav 890)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23019, 649.700, 19.77, 814.100, 1.571, 23, 2324, 'BetaE1_ScienceComputer', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor FOUR_PIECES_CASE from placement-seal.md (FOUR_PIECES_CASE pin on nav 890)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23020, 647.700, 19.77, 814.100, 1.571, 23, 2321, 'BetaE1_FourPiecesCase', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor RUIN_NOTE from placement-seal.md (RUIN_NOTE pin on nav 251; INFERRED Barak'Ka ruin)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23021, 122.200, 27.17, 356.800, 1.249, 23, 2335, 'BetaE1_RuinNote', NULL, true, 30);

-- Spawn-sequence footer: Beta's block top (23199), so the next default id
-- lands above the whole Beta block and every seeded row.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 23199), true);
