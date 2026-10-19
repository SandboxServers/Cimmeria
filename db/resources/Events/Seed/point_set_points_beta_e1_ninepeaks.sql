-- Union Beta E1 Nine Peaks region and ring travel centres on world 23 (UB-02d).
-- Each set has one centre point. The ring centres are the ring positions in
-- ring_transport_regions_beta_e1.sql.

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: NINEPEAKS_CENTRE from placement-seal.md (pin y 6.77 on nav 70). A region centre, not a stand point: the seal measures it 3.50 m off the mesh.
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23060, 23180, -549.100, 6.77, 142.500, 0, 0, 0);
-- CIMMERIA_RECONSTRUCTION: ring 2300 travel centre, the hub ring 148 (placement-seal.md Q1, nav 251; snapped y 8.77, the mesh surface 0.18 m below the pin y 8.591 per nav_inspect)
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23080, 23260, 146.716, 8.77, 403.273, 0, 0, 0);
-- CIMMERIA_RECONSTRUCTION: ring 2301 travel centre, the SW ring (placement-seal.md SW_RING, nav 70, snapped y -51.43)
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23081, 23261, -791.600, -51.43, -92.600, 0, 0, 0);

SELECT pg_catalog.setval('point_set_points_point_id_seq', GREATEST((SELECT last_value FROM point_set_points_point_id_seq), 23261), true);
