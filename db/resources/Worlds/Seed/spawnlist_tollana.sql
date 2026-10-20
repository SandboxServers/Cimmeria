-- Tollana (World 19) mission cast, campaign tollana-continuation W1 (T-02a).
--
-- CIMMERIA_RECONSTRUCTION: offline navmesh candidates beside gate 4
-- (211.045, 0.606, -547.644; stargates.sql). In-client placement is
-- UATPending. Evidence: docs/analysis/tollana-continuation/evidence/
-- T-01-offline-placement.md. Reserve 19001-19010; 19005-19010 unused.

SET search_path = resources, pg_catalog;

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19001, 206.00, -0.58, -543.00, 2.3150, 19, 420, 'Tollana_Copplemann', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19002, 199.50, -0.93, -540.00, 2.0250, 19, 421, 'Tollana_FoodUnit_Core', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19003, 195.50, -1.01, -540.00, 2.0250, 19, 421, 'Tollana_FoodUnit_Nest', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (19004, 197.50, -1.01, -536.54, 2.0250, 19, 421, 'Tollana_FoodUnit_Fungus', NULL, true, 30);

SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 19010), true);
