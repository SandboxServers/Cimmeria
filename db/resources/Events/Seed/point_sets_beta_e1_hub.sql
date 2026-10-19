-- Union Beta E1 hub regions on world 23 (UB-02b). The region names are the
-- enter_region keys the chain packets use. Centres are in
-- point_set_points_beta_e1_hub.sql; the loader expands each centre into a
-- client-hinted box (cell-catalog spawner/regions.rs).
-- CIMMERIA_RECONSTRUCTION: STORAGE_CENTRE from placement-seal.md (Storage, r 20, h 6; EM-Quartermaster01 origin)
-- CIMMERIA_RECONSTRUCTION: ADMIN_CENTRE from placement-seal.md (AdminBuilding, r 14, h 6)

SET search_path = resources, pg_catalog;

INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23000, 'Beta_Site_Evo_1.Storage', 'AreaSet', 23, 20, 6, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23001, 'Beta_Site_Evo_1.AdminBuilding', 'AreaSet', 23, 14, 6, 'Cylinder', 1);

SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT last_value FROM point_sets_set_id_seq), 23001), true);
