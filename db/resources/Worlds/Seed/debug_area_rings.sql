-- NEW CONTENT (Debug Area, DA-08): the eight Debug Area ring stations
-- (world 1300) and the console a player right-clicks at each one.
--
--   35  DebugArea_Ring_Compound     (224, 6.9, -938)  Z1 arrival, Z2 services plaza, Z3 dummies, Z8 cover course
--   36  DebugArea_Ring_FactionYard  (394, -11.13, -738)  Z4 faction yard
--   37  DebugArea_Ring_AiSlope      (81, 0.05, -782)  Z5 patrol, wander, leash and assist
--   38  DebugArea_Ring_ArenaRim     (176, -7.19, -702)  Z6 NPC-vs-NPC arena, west rim
--   39  DebugArea_Ring_ArenaPit     (210, -33.28, -725)  Z6 NPC-vs-NPC arena, pit floor
--   40  DebugArea_Ring_GalleryWest  (127, 23.06, -559)  Z7 enemy gallery, west half
--   41  DebugArea_Ring_GalleryEast  (436, 23.09, -566)  Z7 enemy gallery, east half
--   42  DebugArea_Ring_DeathYard    (437, 11.3, -937)  Z9 death and respawn test, respawner B
--
-- Network: fully connected. Every station lists the other seven, so the
-- destination list a console opens always offers the whole Debug Area. No
-- cross-world destination: the Debug Area is GM-only (D-DA4), and a ring in
-- another world that listed a Debug Area pad would hand ordinary players a
-- way in. GMs leave with .gotolocation.
--
-- Pad row = the cloned base platform's origin (render floor) + 0.537 m, the
-- offset region 1 and region 3 sit above their own base platforms. No
-- mission gate (required_mission_id NULL): the Debug Area is for testing.
-- display_name_id 7508 is the text every shipped ring uses.
--
-- Consoles: template 3 ("Ring Transporter Switch", class being, faction 1,
-- interaction_type 32 INT_RingNetwork as a template default). It renders
-- GP-Props.GP-Ring_Trans_Console00, so the map patch clones no console of its
-- own. Placed where region 3's console sits relative to its base platform
-- (+2.88, +0.44, +1.32), facing the same way. The chains that open the
-- destination list are in Content/Seed/debug_area_ring_chains.sql.
--
-- World 73 (the live Ihpet Crater) loads the same patched map and sees the
-- eight rigs as scenery: no region, console, chain or sequence is seeded for
-- it, so nothing there can open or play them.
--
-- Id blocks (DA-08): ring regions 35-42, spawns 13800-13807.

SET search_path = resources, pg_catalog;

-- Compound
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (35, 1300, 224, 7.437, -938, 'DebugArea_Ring_CompoundRegion', 1.77, 3.53, 13800, 7508, '{36,37,38,39,40,41,42}', 13800, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13800, 226.88, 7.34, -936.68, -2.0617, 1300, 3, 'DebugArea_Ring_Compound', NULL);
-- Faction yard
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (36, 1300, 394, -10.593, -738, 'DebugArea_Ring_FactionYardRegion', 1.77, 3.53, 13801, 7508, '{35,37,38,39,40,41,42}', 13801, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13801, 396.88, -10.69, -736.68, -2.0617, 1300, 3, 'DebugArea_Ring_FactionYard', NULL);
-- AI slope
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (37, 1300, 81, 0.587, -782, 'DebugArea_Ring_AiSlopeRegion', 1.77, 3.53, 13802, 7508, '{35,36,38,39,40,41,42}', 13802, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13802, 83.88, 0.49, -780.68, -2.0617, 1300, 3, 'DebugArea_Ring_AiSlope', NULL);
-- Arena rim
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (38, 1300, 176, -6.653, -702, 'DebugArea_Ring_ArenaRimRegion', 1.77, 3.53, 13803, 7508, '{35,36,37,39,40,41,42}', 13803, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13803, 178.88, -6.75, -700.68, -2.0617, 1300, 3, 'DebugArea_Ring_ArenaRim', NULL);
-- Arena pit
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (39, 1300, 210, -32.743, -725, 'DebugArea_Ring_ArenaPitRegion', 1.77, 3.53, 13804, 7508, '{35,36,37,38,40,41,42}', 13804, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13804, 212.88, -32.84, -723.68, -2.0617, 1300, 3, 'DebugArea_Ring_ArenaPit', NULL);
-- Gallery west
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (40, 1300, 127, 23.597, -559, 'DebugArea_Ring_GalleryWestRegion', 1.77, 3.53, 13805, 7508, '{35,36,37,38,39,41,42}', 13805, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13805, 129.88, 23.5, -557.68, -2.0617, 1300, 3, 'DebugArea_Ring_GalleryWest', NULL);
-- Gallery east
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (41, 1300, 436, 23.627, -566, 'DebugArea_Ring_GalleryEastRegion', 1.77, 3.53, 13806, 7508, '{35,36,37,38,39,40,42}', 13806, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13806, 438.88, 23.53, -564.68, -2.0617, 1300, 3, 'DebugArea_Ring_GalleryEast', NULL);
-- Death yard
INSERT INTO ring_transport_regions (region_id, world_id, x, y, z, tag, height, radius, event_set_id, display_name_id, destination_region_ids, point_set_id, required_mission_id) VALUES (42, 1300, 437, 11.837, -937, 'DebugArea_Ring_DeathYardRegion', 1.77, 3.53, 13807, 7508, '{35,36,37,38,39,40,41}', 13807, NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13807, 439.88, 11.74, -935.68, -2.0617, 1300, 3, 'DebugArea_Ring_DeathYard', NULL);

SELECT pg_catalog.setval('ring_transport_regions_region_id_seq', GREATEST((SELECT MAX(region_id) FROM ring_transport_regions), (SELECT last_value FROM ring_transport_regions_region_id_seq), 42), true);
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 13807), true);
