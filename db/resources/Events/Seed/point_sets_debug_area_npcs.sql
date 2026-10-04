--
-- Debug Area (world 1300) packet DA-03: the Z5 patrol route. A two-point
-- 'Patrol' / 'Path' set is walked back and forth (point_id order). Set 13200,
-- points 13200-13201: the DA-03 block mirrors its spawn block
-- (docs/analysis/debug-area/README.md). Reference: docs/content/debug-area.md.
--
-- Loaded by db/database.sql after point_sets.sql, whose fixed footer would
-- otherwise lower the sequences again. The footers move both sequences past
-- every seeded row and past the DA-03 block top (sets 13209, points 13299),
-- so the console's .path_add authoring never hands out a seeded id.
--

INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13200, 'DebugArea.Patrol.Slope', 'Patrol', 1300, NULL, NULL, 'Path', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13200, 13200, 28.00, 12.39, -760.00, 0, 0, 0);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13200, 13201, 26.00, 21.83, -644.00, 0, 0, 0);

SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT MAX(set_id) FROM point_sets), (SELECT last_value FROM point_sets_set_id_seq), 13209), true);
SELECT pg_catalog.setval('point_set_points_point_id_seq', GREATEST((SELECT MAX(point_id) FROM point_set_points), (SELECT last_value FROM point_set_points_point_id_seq), 13299), true);
