-- Union Beta E1 Nine Peaks region and the ring travel ranges on world 23 (UB-02d).
-- The region name is the enter_region key the chain packets use. Centres are in
-- point_set_points_beta_e1_ninepeaks.sql; the loader expands each centre into a
-- client-hinted box (cell-catalog spawner/regions.rs).
-- CIMMERIA_RECONSTRUCTION: NINEPEAKS_CENTRE from placement-seal.md (NinePeaks, r 20, h 6)
-- CIMMERIA_RECONSTRUCTION: travel ranges for ring 2300 and ring 2301 (radius and height copied from the Omega ring set 2057)

SET search_path = resources, pg_catalog;

INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23060, 'Beta_Site_Evo_1.NinePeaks', 'AreaSet', 23, 20, 6, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23080, 'Beta_Site_Evo_1.RingTravelHub', 'AreaSet', 23, 2.53, 1.77, 'Cylinder', 1);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (23081, 'Beta_Site_Evo_1.RingTravelSW', 'AreaSet', 23, 2.53, 1.77, 'Cylinder', 1);

SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT last_value FROM point_sets_set_id_seq), 23081), true);
