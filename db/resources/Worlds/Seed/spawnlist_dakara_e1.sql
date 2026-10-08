-- Dakara_E1 rebuild: story spawns on worlds 61 and 62.
-- Campaign: docs/analysis/dakara-e1-rebuild/; placement rows in
-- docs/analysis/dakara-e1-rebuild/placements/. All positions here are
-- PROJECT_FINAL_RECONSTRUCTION from labelled map estimates pending M2 UAT.
-- DK-05 adds the static cast to this file in story order.

SET search_path = resources, pg_catalog;

-- DK-04: four clickable flap props, from PL-DK-A-01/A-03 and
-- PL-DK-B-02/B-03. Tags are unique because interact_tag triggers do not
-- implicitly filter by world. The two exterior tents share the one
-- client-shipped interior (world 62).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8000, 121.50, -19.04, 265.20, 4.2487, 61, 445, 'Dakara_E1_TentFlap_ToCommand', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8001, 164.00, -21.33, 276.00, 5.1933, 61, 446, 'Dakara_E1_TentFlap_ToMohkatan', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8002, 70.74, 0.00, 39.40, 3.1416, 62, 447, 'Dakara_E1_StoryRm_TentFlap_FromCommand', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8003, 70.52, 0.05, 34.50, 3.1416, 62, 448, 'Dakara_E1_StoryRm_TentFlap_FromMohkatan', NULL, true);
