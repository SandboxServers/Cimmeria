--
-- Debug Area (world 1300) combat zones: DA-04, spawns 13600-13799.
-- Plan: docs/analysis/debug-area/README.md (Z6, Z8, Z9; D-DA8). Reference with
-- the spawn tables: docs/content/debug-area.md. Templates 1370-1377:
-- Entities/Seed/entity_templates_debug_area_combat.sql.
--
-- Loaded right after spawnlist.sql (db/database.sql). Every row is a mobile
-- `mob`: the spawner grounds its Y onto ihpet_crater_light.nav (D-DA5). On the
-- Z9 terrain the navmesh and the occluder's terrain disagree by up to 2 u, and
-- the pit's mesh undulates by about 2 u over its flat water collision plane
-- (y -33.28), so rows are authored near the mesh height and grounding finishes
-- the job. Heading is yaw = atan2(dx, dz). Placement is pinned on the real nav,
-- occluder and cover by crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/
-- (arena.rs, cover.rs, death_respawn.rs) and in the DB by live_db.rs there.
--

-- Z6 NPC-vs-NPC arena: the flat pit floor, centre (250, -32.4, -725), clear of
-- geometry for 40+ u. The crater floor stands 25-40 u above it on the north,
-- east and south; the gentle ramp in is from the south-west (x 190-200,
-- z -755..-790). Fight 1: the Praxis squad on x 238 faces the NID squad on x 262,
-- 24 u apart, inside both sides' 30 u aggro radius. A player watching anywhere
-- within 150 u (AoI) starts it; a player who walks down the ramp and shoots a
-- NID guard joins on the Praxis side. Fight 2, which no player can join, is on
-- the west half, 44+ u from every NID guard (outside their 30 u radius, so a
-- tester can walk up to it): the Green Sniper pair at z -716 against the Yellow
-- Faction pair at z -734..-736, 20-22 u apart. DA-08's ring pad at (210, -725)
-- sits between them, 10-12 u from the nearest; neither side targets players.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13600, 238, -32.4, -721, 1.5708, 1300, 1370, 'DebugArea_Arena_Praxis1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13601, 238, -32.4, -725, 1.5708, 1300, 1371, 'DebugArea_Arena_Praxis2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13602, 238, -32.4, -729, 1.5708, 1300, 1370, 'DebugArea_Arena_Praxis3', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13603, 262, -32.4, -721, -1.5708, 1300, 1372, 'DebugArea_Arena_NID1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13604, 262, -32.4, -725, -1.5708, 1300, 1372, 'DebugArea_Arena_NID2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13605, 262, -32.4, -729, -1.5708, 1300, 1372, 'DebugArea_Arena_NID3', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13610, 214, -32.6, -716, -2.836, 1300, 1373, 'DebugArea_Arena_Green1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13611, 218, -32.35, -716, -2.657, 1300, 1373, 'DebugArea_Arena_Green2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13612, 206, -32.3, -734, 0.507, 1300, 1374, 'DebugArea_Arena_Yellow1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13613, 210, -31.95, -736, 0.291, 1300, 1374, 'DebugArea_Arena_Yellow2', NULL);

-- Z8 cover course, the south compound's west wing (entry doorway at
-- (204, 7.0, -926)). Rifleman 1 stands in the open hall 6.4 u from the nearest
-- marker and has to seek cover when a fight starts (an east-facing slot near it,
-- against a tester coming in from the doorway). Rifleman 2 stands in the room
-- west of the x 151 wall, facing its doorway at z -961..-963, and seeks a slot
-- there. Rifleman 3 is authored 0.8 u behind Mid/Better marker 130000029/1
-- (181.42, 6.58, -963.83, facing north into the hall) on the hall's south wall,
-- so it spawns holding that slot (NA22) and peeks past it (NA23). Its marker is
-- 37+ u from riflemen 1 and 2, beyond the 30 u a seeking NPC looks for cover, so
-- neither takes it while rifleman 3 is dead and it respawns holding it. The three
-- are too far apart to assist each other.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13620, 166, 6.7, -930, 1.5708, 1300, 1375, 'DebugArea_Cover_Rifleman1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13621, 142, 6.7, -958, 1.9415, 1300, 1375, 'DebugArea_Cover_Rifleman2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13622, 181.33, 6.58, -964.62, 0.1133, 1300, 1375, 'DebugArea_Cover_Rifleman3', NULL);

-- Z9 death and respawn test, around respawner 131 (438, 10.4, -916).
-- The four lethal NID Operatives stand 28-31 u east of it, over a rise that
-- blocks the line of sight from the respawner, with a 10 u aggro radius: walk up
-- to them to die. The two respawn-timer targets stand 10 u west of the respawner,
-- seeded NEUTRAL so they wait to be shot: 13640 respawns 10 s after death (its
-- own respawn_secs), 13641 after the template's 30 s.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13630, 466, 13.33, -911, -1.5708, 1300, 1376, 'DebugArea_Death_Operative1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13631, 466, 13.91, -915, -1.5708, 1300, 1376, 'DebugArea_Death_Operative2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13632, 466, 14.25, -919, -1.5708, 1300, 1376, 'DebugArea_Death_Operative3', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13633, 469, 12.86, -913, -1.5708, 1300, 1376, 'DebugArea_Death_Operative4', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs, aggression_override) VALUES (13640, 430, 9.86, -910, 2.2143, 1300, 1377, 'DebugArea_Respawn_Fast', NULL, 10, 3);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, aggression_override) VALUES (13641, 430, 10.03, -922, 0.9273, 1300, 1377, 'DebugArea_Respawn_Slow', NULL, 3);

-- No sequence footer: the base file's footer already floors the sequence past
-- the whole Debug Area reservation (DA-01).
