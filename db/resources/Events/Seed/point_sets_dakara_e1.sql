-- Dakara_E1 discovery regions. DK-04 placed four map-backed estimates.
-- DK-05 adds a speculative Naquadah Repository at the owner's chosen
-- military outpost lead. A distinct Jaffa Command remains unplaced.
-- These client-hinted cylinders are reconstructed, not recovered server
-- data. Centres are in point_set_points_dakara_e1.sql.

SET search_path = resources, pg_catalog;

INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2123, 'Dakara_E1.CommandTent', 'AreaSet', 61, 15, 6, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2124, 'Dakara_E1.MohkatanTent', 'AreaSet', 61, 15, 6, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2125, 'Dakara_E1.StargatePlaza', 'AreaSet', 61, 18, 6, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2126, 'Dakara_E1.SuperweaponCourtyard', 'AreaSet', 61, 20, 8, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2127, 'Dakara_E1.NaqRepository', 'AreaSet', 61, 22, 8, 'Cylinder', 1);

-- DK-09, D-DK18 RECONSTRUCTION: mission 1570 step 4902 "Search the locations
-- Loth'ta indicated" names no site; its objectives become one region around
-- each SG-18 remains (spawns 8008-8010), so the M2 position pass moves a
-- region with its remains. 2128 completes 5632, 2129 completes 5630,
-- 2130 completes the optional 5631 (dakara_e1_m1570_search_chains.sql).
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2128, 'Dakara_E1.SG18Louis', 'AreaSet', 61, 8, 10, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2129, 'Dakara_E1.SG18Nguyn', 'AreaSet', 61, 8, 10, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (2130, 'Dakara_E1.SG18Waters', 'AreaSet', 61, 8, 10, 'Cylinder', 1);

SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT last_value FROM point_sets_set_id_seq), 2130), true);
