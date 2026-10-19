-- Union Beta E1 ring pair on world 23 (UB-02d, D-UB4 = A): region 2300 is the
-- hub ring (ring 148, nav 251) and 2301 is the SW ring (nav 70). Each names the
-- other in destination_region_ids. Ids 2302-2309 are reserved.
-- RESTORED_SOURCE: the two ring positions (cooked map prefabs; see the UB-02d packet).
-- CIMMERIA_RECONSTRUCTION: the pairing (the routing is lost).
-- event_set_id 10000 is the generic Region_Teleport_In set the other rings in
-- ring_transport_regions.sql use: Beta has no ring event set (UB-00b). The
-- ceremony has no animation on this route (castle_cellblock_chains.sql:2594
-- note); the ring matinee the client ships may still play. UB-U8 records what
-- the player sees.

SET search_path = resources, pg_catalog;

INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (2300, 23, 146.716, 8.77, 403.273, 'BetaE1_Ring_Hub', 1.70000005, 2.52999997, 10000, 7508, '{2301}', 23080, NULL);
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (2301, 23, -791.600, -51.43, -92.600, 'BetaE1_Ring_SW', 1.70000005, 2.52999997, 10000, 7508, '{2300}', 23081, NULL);

-- Region-sequence footer: the default id lands above the Beta pair.
SELECT pg_catalog.setval('ring_transport_regions_region_id_seq', GREATEST((SELECT MAX(region_id) FROM ring_transport_regions), (SELECT last_value FROM ring_transport_regions_region_id_seq), 2301), true);
