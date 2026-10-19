-- SGC_W1 Union world spawns (US-11). CIMMERIA_RECONSTRUCTION: positions are
-- the sealed points in docs/analysis/union-sgc-continuation/placement.md
-- (P01, P04, P06, P10a-f; crate y from the sealed P10 row). Chain packets
-- read these tags; SGC_W1_Crate (spawn 64) stays in spawnlist.sql.

SET search_path = resources, pg_catalog;

-- 15801-15802 reserved (F12).

-- CIMMERIA_RECONSTRUCTION: P01, Harriman in the Control Room (template 58).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15800, 172.08, -6.40, 46.70, 3.1416, 58, 58, 'SGC_W1_Harriman', NULL, true);

-- CIMMERIA_RECONSTRUCTION: P04 target, Carter's virus terminal (template 5803).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15803, -49.48, 2.32, 36.38, 0, 58, 5803, 'SGC_W1_CarterTerminal', NULL, true);

-- CIMMERIA_RECONSTRUCTION: P06 target, the Failsafe Device (template 5800).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15804, 151.184, 1.28, -193.376, 0, 58, 5800, 'SGC_W1_FailsafeDevice', NULL, true);

-- CIMMERIA_RECONSTRUCTION: P10a-f, the six requisitions crates (template 5802).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15805, 257.157, -10.85, 41.74, 0, 58, 5802, 'SGC_W1_Crate1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15806, 259.131, -10.85, 42.326, 0, 58, 5802, 'SGC_W1_Crate2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15807, 260.237, -10.85, 43.42, 0, 58, 5802, 'SGC_W1_Crate3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15808, 255.923, -10.85, 43.406, 0, 58, 5802, 'SGC_W1_Crate4', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15809, 257.726, -10.85, 43.679, 0, 58, 5802, 'SGC_W1_Crate5', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (15810, 254.369, -10.85, 43.338, 0, 58, 5802, 'SGC_W1_Crate6', NULL, true);

SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 15810), true);
