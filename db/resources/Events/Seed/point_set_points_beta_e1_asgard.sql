-- Union Beta E1 Asgard region centres on world 23 (UB-02c). Each set has one
-- centre point. The Infirmary centre is the seal's INFIRMARY_CENTRE stand
-- point (nav 251, snapped y 15.97). The FireSite centre is the seal's region
-- centre (1022.3, 14.5, 271.2), not a stand-point row.

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: INFIRMARY_CENTRE from placement-seal.md (nav 251, snapped y 15.97)
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23030, 23100, 961.300, 15.97, 284.900, 0, 0, 0);
-- CIMMERIA_RECONSTRUCTION: FireSite centre from placement-seal.md (Regions table, r 12, h 6)
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23031, 23101, 1022.300, 14.50, 271.200, 0, 0, 0);

SELECT pg_catalog.setval('point_set_points_point_id_seq', GREATEST((SELECT last_value FROM point_set_points_point_id_seq), 23101), true);
