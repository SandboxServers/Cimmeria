--
-- Debug Area (world 1300) Z12 combat reference range: NC-03, spawns 14300-14319 (D-NC9).
-- Plan: docs/analysis/npc-combat-reference/README.md. Reference: docs/content/debug-area.md.
-- Templates: 1380/1381 in Entities/Seed/entity_templates_debug_area_reference.sql; 24, 4, 76 are
-- the real NID Guard, Prisoner retrieval unit and ScavDog templates.
--
-- Four pens on the flat navmesh terrace west of the south compound (y about 5.2-5.5),
-- 41-70 m apart and 60+ m from every other world-1300 spawn, respawner and ring pad, so a
-- fight in one pen never pulls another and the reach guards hold. Every row is aggressive
-- (aggression_override NULL = template default) with a 20 s respawn and no loot override.
-- Heading is yaw = atan2(dx, dz); all face east, toward the compound.

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (14300, 100, 5.38, -860, 1.5708, 1300, 24, 'DebugArea_Ref_NID', NULL, 20);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (14301, 60, 5.52, -850, 1.5708, 1300, 4, 'DebugArea_Ref_PRU', NULL, 20);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (14302, 90, 5.13, -900, 1.5708, 1300, 76, 'DebugArea_Ref_ScavDog', NULL, 20);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (14303, 40, 5.38, -890, 1.5708, 1300, 1380, 'DebugArea_Ref_JaffaGuard', NULL, 20);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (14304, 40, 5.25, -900, 1.5708, 1300, 1381, 'DebugArea_Ref_JaffaLieutenant', NULL, 20);

-- Reserve 14305-14319 for the reference range.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 14319), true);
