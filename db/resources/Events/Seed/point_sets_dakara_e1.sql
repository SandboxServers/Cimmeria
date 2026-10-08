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

SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT last_value FROM point_sets_set_id_seq), 2127), true);
