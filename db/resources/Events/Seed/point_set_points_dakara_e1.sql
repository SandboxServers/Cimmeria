-- DK-04 cylinder centres, on world 61's shipped navmesh component 279.
-- PL-DK-A-01/A-03/A-14 supply the tent and courtyard points; the plaza
-- point is the CS-02 start/respawner 610. All are map estimates pending UAT.

SET search_path = resources, pg_catalog;

INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (2123, 2552, 121.50, -19.04, 265.20, 0, 0, 0);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (2124, 2553, 164.00, -21.33, 276.00, 0, 0, 0);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (2125, 2554, 100.00, -17.40, 230.00, 0, 0, 0);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (2126, 2555, 129.40, -21.53, -66.50, 0, 0, 0);
-- DK-05-A-01: speculative military-outpost estimate, not a map label.
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (2127, 2556, 290.00, -17.00, 95.00, 0, 0, 0);

SELECT pg_catalog.setval('point_set_points_point_id_seq', GREATEST((SELECT last_value FROM point_set_points_point_id_seq), 2556), true);
