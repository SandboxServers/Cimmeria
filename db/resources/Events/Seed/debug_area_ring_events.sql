-- NEW CONTENT (Debug Area, DA-08): Kismet sequences, event sets and pad
-- trigger volumes for the eight Debug Area ring stations (world 1300).
-- docs/content/debug-area.md "Ring transports" and
-- docs/gameplay/ring-transport-system.md "Debug Area ring network" describe them.
--
-- Region, base floor height (render floor; the rig's base platform origin) and
-- what each station is for:
--   35  DebugArea_Ring_Compound     (224, 6.9, -938)  Z1 arrival, Z2 services plaza, Z3 dummies, Z8 cover course
--   36  DebugArea_Ring_FactionYard  (394, -11.13, -738)  Z4 faction yard
--   37  DebugArea_Ring_AiSlope      (81, 0.05, -782)  Z5 patrol, wander, leash and assist
--   38  DebugArea_Ring_ArenaRim     (176, -7.19, -702)  Z6 NPC-vs-NPC arena, west rim
--   39  DebugArea_Ring_ArenaPit     (331, -11.12, -693)  Z6 NPC-vs-NPC arena, east shelf
--   40  DebugArea_Ring_GalleryWest  (127, 23.06, -559)  Z7 enemy gallery, west half
--   41  DebugArea_Ring_GalleryEast  (436, 23.09, -566)  Z7 enemy gallery, east half
--   42  DebugArea_Ring_DeathYard    (437, 11.3, -937)  Z9 death and respawn test, respawner B
--
-- The rigs exist only in a patched Ihpet_Crater_Light-fff80002.umap (client
-- patch 010-debug-area-rings, data/client-patches/README.md). Each is a copy
-- of region 3's rig in Castle_CellBlock-fffeffff (sequence 772), cloned by
-- `upk_patch clone-objects` into the chunk's Main_Sequence.Prefabs. Each copy
-- took the next free instance number, so station N's sequence is
-- `...Pf0_Seq` (station 0) or `...Pf0_Seq_<N-1>`.
--
-- Clients resolve a sequence id through their own cooked catalogue, not this
-- table: crates/resources/src/base/sequence_overrides.rs delivers these
-- sixteen ids per key at login. On a client without patch 010 the paths do
-- not resolve and the trip runs without the animation.
--
-- Id blocks (DA-08): sequences 10189-10204, event sets 13810-13817, point
-- sets 13810-13817, points 13810-13817. Set and point 13800 are DA-07's gate
-- volume (point_sets.sql), so this block starts at 13810.

SET search_path = resources, pg_catalog;

-- Compound (region 35)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10189, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10190, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq');
INSERT INTO event_sets (event_set_id, name) VALUES (13810, 'DebugArea Ring Compound');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13810, 10189);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13810, 10190);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13810, 'DebugArea.RingCompound', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13810, 13810, 224, 7.437, -938, 0, 0, 0);
-- Faction yard (region 36)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10191, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10192, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_0');
INSERT INTO event_sets (event_set_id, name) VALUES (13811, 'DebugArea Ring Faction yard');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13811, 10191);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13811, 10192);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13811, 'DebugArea.RingFactionYard', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13811, 13811, 394, -10.593, -738, 0, 0, 0);
-- AI slope (region 37)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10193, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_1');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10194, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_1');
INSERT INTO event_sets (event_set_id, name) VALUES (13812, 'DebugArea Ring AI slope');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13812, 10193);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13812, 10194);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13812, 'DebugArea.RingAiSlope', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13812, 13812, 81, 0.587, -782, 0, 0, 0);
-- Arena rim (region 38)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10195, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_2');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10196, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_2');
INSERT INTO event_sets (event_set_id, name) VALUES (13813, 'DebugArea Ring Arena rim');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13813, 10195);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13813, 10196);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13813, 'DebugArea.RingArenaRim', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13813, 13813, 176, -6.653, -702, 0, 0, 0);
-- Arena pit (region 39)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10197, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_3');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10198, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_3');
INSERT INTO event_sets (event_set_id, name) VALUES (13814, 'DebugArea Ring Arena pit');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13814, 10197);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13814, 10198);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13814, 'DebugArea.RingArenaPit', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13814, 13814, 331, -10.583, -693, 0, 0, 0);
-- Gallery west (region 40)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10199, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_4');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10200, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_4');
INSERT INTO event_sets (event_set_id, name) VALUES (13815, 'DebugArea Ring Gallery west');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13815, 10199);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13815, 10200);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13815, 'DebugArea.RingGalleryWest', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13815, 13815, 127, 23.597, -559, 0, 0, 0);
-- Gallery east (region 41)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10201, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_5');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10202, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_5');
INSERT INTO event_sets (event_set_id, name) VALUES (13816, 'DebugArea Ring Gallery east');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13816, 10201);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13816, 10202);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13816, 'DebugArea.RingGalleryEast', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13816, 13816, 436, 23.627, -566, 0, 0, 0);
-- Death yard (region 42)
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10203, 8000, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_6');
INSERT INTO sequences (sequence_id, event_id, kismet_script_name) VALUES (10204, 8001, 'Ihpet_Crater_Light-fff80002.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq_6');
INSERT INTO event_sets (event_set_id, name) VALUES (13817, 'DebugArea Ring Death yard');
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13817, 10203);
INSERT INTO event_sets_sequences (event_set_id, sequence_id) VALUES (13817, 10204);
INSERT INTO point_sets (set_id, name, type, world_id, radius, height, shape, flags) VALUES (13817, 'DebugArea.RingDeathYard', 'AreaSet', 1300, 3.53, 1.77, 'Cylinder', 1);
INSERT INTO point_set_points (set_id, point_id, x, y, z, yaw, pitch, roll) VALUES (13817, 13817, 437, 11.837, -937, 0, 0, 0);

-- Sequences past every seeded row and past the DA-08 blocks. Each seed file
-- that inserts explicit ids carries its own footer, since files load in turn.
SELECT pg_catalog.setval('event_sets_event_set_id_seq', GREATEST((SELECT MAX(event_set_id) FROM event_sets), (SELECT last_value FROM event_sets_event_set_id_seq), 13817), true);
SELECT pg_catalog.setval('point_sets_set_id_seq', GREATEST((SELECT MAX(set_id) FROM point_sets), (SELECT last_value FROM point_sets_set_id_seq), 13817), true);
SELECT pg_catalog.setval('point_set_points_point_id_seq', GREATEST((SELECT MAX(point_id) FROM point_set_points), (SELECT last_value FROM point_set_points_point_id_seq), 13817), true);
