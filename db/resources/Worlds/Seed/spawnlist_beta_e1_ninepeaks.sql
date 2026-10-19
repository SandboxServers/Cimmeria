-- Union Beta E1: Nine Peaks camp spawns and the SW ring spawn on world 23 (Beta Site Evo 1).
-- Campaign: docs/analysis/union-zone-planning/beta-packets/UB-02bcd.md (UB-02d).
-- Anchors: docs/analysis/union-beta-e1-continuation/placement-seal.md (UB-00b).
-- Nav component 70 (the west valley) has no walk route to ARRIVAL (seal Q1), so
-- these rows are reached by beam or by the ring pair in ring_transport_regions_beta_e1.sql.
-- Spawn ids 23110-23115 are Beta's block; the footer at the end keeps the
-- default-id sequence above the whole block. Tags are unique across worlds
-- (interact_tag triggers do not filter by world) and start with BetaE1_.

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: anchor RAS_NOR from placement-seal.md (Ras'nor pin on nav 70)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23110, -545.100, 6.77, 141.500, 4.957, 23, 2306, 'BetaE1_RasNor', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor SICK_JAFFA_1 from placement-seal.md (SickJaffa 1 pin on nav 70)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23111, -547.100, 6.80, 138.500, 0.588, 23, 2307, 'BetaE1_SickJaffa_1', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor SICK_JAFFA_2 from placement-seal.md (SickJaffa 2 pin on nav 70)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23112, -554.100, 6.77, 142.500, 1.681, 23, 2307, 'BetaE1_SickJaffa_2', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor SICK_JAFFA_3 from placement-seal.md (SickJaffa 3 pin on nav 70)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23113, -552.100, 6.77, 138.500, 1.166, 23, 2307, 'BetaE1_SickJaffa_3', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor JAFFA_BOX from placement-seal.md (JaffaBox pin on nav 70)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23114, -546.100, 6.77, 146.500, 2.944, 23, 2326, 'BetaE1_JaffaBox', NULL, true, 30);
-- CIMMERIA_RECONSTRUCTION: anchor SW_RING from placement-seal.md (SW ring pin on nav 70, snapped y -51.43); the ring pair is in ring_transport_regions_beta_e1.sql. Heading not applicable.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (23115, -791.600, -51.43, -92.600, 0.000, 23, 3, 'BetaE1_Ring_SW', NULL, true, 30);

-- Spawn-sequence footer: Beta's block top (23199), so the next default id
-- lands above the whole Beta block and every seeded row.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 23199), true);
