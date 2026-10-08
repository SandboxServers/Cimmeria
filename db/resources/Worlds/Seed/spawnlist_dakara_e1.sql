-- Dakara_E1 rebuild: story spawns on worlds 61 and 62.
-- Campaign: docs/analysis/dakara-e1-rebuild/; placement rows in
-- docs/analysis/dakara-e1-rebuild/placements/. All positions here are
-- PROJECT_FINAL_RECONSTRUCTION from labelled map estimates pending M2 UAT.
-- DK-05 places the static cast after the DK-04 flap props.

SET search_path = resources, pg_catalog;

-- DK-04: four clickable flap props, from PL-DK-A-01/A-03 and
-- PL-DK-B-02/B-03. Tags are unique because interact_tag triggers do not
-- implicitly filter by world. The two exterior tents share the one
-- client-shipped interior (world 62).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8000, 121.50, -19.04, 265.20, 4.2487, 61, 445, 'Dakara_E1_TentFlap_ToCommand', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8001, 164.00, -21.33, 276.00, 5.1933, 61, 446, 'Dakara_E1_TentFlap_ToMohkatan', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8002, 70.74, 0.00, 39.40, 3.1416, 62, 447, 'Dakara_E1_StoryRm_TentFlap_FromCommand', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (8003, 70.52, 0.05, 34.50, 3.1416, 62, 448, 'Dakara_E1_StoryRm_TentFlap_FromMohkatan', NULL, true);

-- DK-05 PROJECT_FINAL_RECONSTRUCTION. B-04/B-05 are inside the shared room;
-- A-06 is the plaza. DK-05-A-01 is a speculative outpost estimate approved
-- by the owner, not a recovered Repository landmark. Friendly, stationary
-- actors respawn after 30 seconds if removed; mission dialog binds are
-- per-player and are deliberately not applied by the spawn rows.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8004, 69.50, 0.00, 27.60, 0.5586, 62, 59, 'Dakara_E1_StoryRm_Bratac', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8005, 73.60, 0.00, 27.60, 5.4578, 62, 54, 'Dakara_E1_StoryRm_Mohkatan', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8006, 103.50, -16.80, 238.50, 3.5322, 61, 441, 'Dakara_E1_Raknor', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8007, 290.00, -17.00, 95.00, 5.3300, 61, 440, 'Dakara_E1_Lothta', NULL, true, 30);
