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

-- DK-09 PROJECT_FINAL_RECONSTRUCTION. The SG-18 remains (mission 1570 step
-- 4901) at PL-DK-A-07 to A-09: LOW map estimates, on the navmesh but not on
-- the plaza's component, reached over rough ground (D-DK11). The five drop
-- locations (mission 1646 step 4908) at PL-DK-A-17 to A-21, on the plaza's
-- component (D-DK13). No bind is applied here: every prop's dialog is a
-- per-player bind from its mission chain.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8008, 483.90, -9.10, 460.60, 3.1416, 61, 450, 'Dakara_E1_SG18_Louis', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8009, 503.90, -9.10, 462.00, 0.0000, 61, 451, 'Dakara_E1_SG18_Nguyn', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8010, 440.10, -9.10, 497.80, 1.5708, 61, 452, 'Dakara_E1_SG18_Waters', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8011, -110.00, -16.61, 253.00, 0.0000, 61, 449, 'Dakara_E1_Drop1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8012, 318.00, -17.82, 309.00, 0.0000, 61, 458, 'Dakara_E1_Drop2', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8013, 62.00, -16.56, 413.00, 0.0000, 61, 459, 'Dakara_E1_Drop3', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8014, 26.00, -17.90, 109.00, 0.0000, 61, 460, 'Dakara_E1_Drop4', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (8015, 366.00, -16.26, -31.00, 0.0000, 61, 461, 'Dakara_E1_Drop5', NULL, true, 30);
