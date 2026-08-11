-- [savespawn] -> db/resources/X.sql
DELETE FROM x;
-- [delspawn] -> db/resources/Worlds/Seed/spawnlist.sql
DELETE FROM resources.spawnlist WHERE spawn_id = 42;
-- [savespawn] -> db/resources/X.sql
DELETE FROM x;
-- ===== CONFIRMED by entity 7 -> commit into db/resources/X.sql =====
DELETE FROM x;
-- [path_clear] -> db/resources/Events/Seed/point_set_points.sql
DELETE FROM resources.point_set_points WHERE set_id = 7;
-- [path_add] -> db/resources/Events/Seed/point_sets.sql
INSERT INTO resources.point_sets (set_id, name, type, world_id, shape, flags)
SELECT 7, 'console-path-7', 'Patrol', w.world_id, 'Path', 1
FROM resources.worlds w WHERE w.world = 'Agnos'
ON CONFLICT (set_id) DO NOTHING;
-- [path_clear] -> db/resources/Events/Seed/point_sets.sql
DELETE FROM resources.point_sets WHERE set_id = 7;
-- [path_add] -> db/resources/Events/Seed/point_set_points.sql
INSERT INTO resources.point_set_points (set_id, x, y, z, yaw, pitch, roll)
VALUES (7, 10, 0, 10, 0, 0, 0);
-- [savespawn] -> db/resources/Worlds/Seed/spawnlist.sql
INSERT INTO resources.spawnlist (x, y, z, heading, world_id, template_id, tag)
SELECT 12, 0, 12, 0, w.world_id, 1, NULL
FROM resources.worlds w WHERE w.world = 'Agnos';
-- [path_add] -> db/resources/Events/Seed/point_sets.sql
INSERT INTO resources.point_sets (set_id, name, type, world_id, shape, flags)
SELECT 10, 'console-path-10', 'Patrol', w.world_id, 'Path', 1
FROM resources.worlds w WHERE w.world = 'Agnos'
ON CONFLICT (set_id) DO NOTHING;
-- [path_add] -> db/resources/Events/Seed/point_set_points.sql
INSERT INTO resources.point_set_points (set_id, x, y, z, yaw, pitch, roll)
VALUES (10, 10, 0, 10, 0, 0, 0);
-- [path_clear] -> db/resources/Events/Seed/point_set_points.sql
DELETE FROM resources.point_set_points WHERE set_id = 10;
-- [path_clear] -> db/resources/Events/Seed/point_sets.sql
DELETE FROM resources.point_sets WHERE set_id = 10;
-- [path_seq] -> db/resources/Events/Seed/point_set_points.sql
UPDATE resources.point_set_points SET sequence_id = 10
WHERE point_id = (SELECT point_id FROM resources.point_set_points
WHERE set_id = 10 ORDER BY point_id OFFSET 10 LIMIT 1);
-- [path_seq] -> db/resources/Events/Seed/point_set_points.sql
UPDATE resources.point_set_points SET sequence_id = NULL
WHERE point_id = (SELECT point_id FROM resources.point_set_points
WHERE set_id = 10 ORDER BY point_id OFFSET 10 LIMIT 1);
-- [path_set_tp] -> db/resources/Events/Seed/point_set_points.sql
UPDATE resources.point_set_points SET teleport_x = 10, teleport_y = 0, teleport_z = 10
WHERE point_id = (SELECT point_id FROM resources.point_set_points
WHERE set_id = 10 ORDER BY point_id OFFSET 10 LIMIT 1);
-- [path_clear_tp] -> db/resources/Events/Seed/point_set_points.sql
UPDATE resources.point_set_points
SET teleport_x = NULL, teleport_y = NULL, teleport_z = NULL, teleport_sequence_id = NULL, teleport_delay = NULL
WHERE point_id = (SELECT point_id FROM resources.point_set_points
WHERE set_id = 10 ORDER BY point_id OFFSET 10 LIMIT 1);
-- [path_seq] -> db/resources/Events/Seed/point_set_points.sql
UPDATE resources.point_set_points SET teleport_sequence_id = 10
WHERE point_id = (SELECT point_id FROM resources.point_set_points
WHERE set_id = 10 ORDER BY point_id OFFSET 10 LIMIT 1);
-- [path_set_tp_delay] -> db/resources/Events/Seed/point_set_points.sql
UPDATE resources.point_set_points SET teleport_delay = 10
WHERE point_id = (SELECT point_id FROM resources.point_set_points
WHERE set_id = 10 ORDER BY point_id OFFSET 10 LIMIT 1);
