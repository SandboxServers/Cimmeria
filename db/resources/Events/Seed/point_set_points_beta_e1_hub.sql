-- Union Beta E1 hub region centres on world 23 (UB-02b). Each set has one
-- centre point, the seal's snapped stand point for its anchor.

SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: STORAGE_CENTRE from placement-seal.md (nav 251, snapped y 13.37)
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23000, 23000, 1022.327, 13.37, 271.180, 0, 0, 0);
-- CIMMERIA_RECONSTRUCTION: ADMIN_CENTRE from placement-seal.md (nav 251, snapped y 13.90)
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (23001, 23001, 1018.405, 13.90, 506.851, 0, 0, 0);
