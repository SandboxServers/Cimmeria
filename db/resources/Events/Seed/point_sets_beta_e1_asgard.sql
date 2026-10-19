-- Union Beta E1 Asgard regions on world 23 (UB-02c). The region names are the
-- enter_region keys the chain packets use. Centres are in
-- point_set_points_beta_e1_asgard.sql; the loader expands each centre into a
-- client-hinted box (cell-catalog spawner/regions.rs).
-- CIMMERIA_RECONSTRUCTION: Infirmary from placement-seal.md (r 10, h 6; INFIRMARY_CENTRE)
-- CIMMERIA_RECONSTRUCTION: FireSite from placement-seal.md (r 12, h 6; Quartermaster fire cluster)

SET search_path = resources, pg_catalog;

INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23030, 'Beta_Site_Evo_1.Infirmary', 'AreaSet', 23, 10, 6, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23031, 'Beta_Site_Evo_1.FireSite', 'AreaSet', 23, 12, 6, 'Cylinder', 1);

SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT last_value FROM point_sets_set_id_seq), 23031), true);
