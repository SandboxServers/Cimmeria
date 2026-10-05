--
-- Debug Area (world 1300) combat zones: DA-04, spawns 13600-13799.
-- Plan: docs/analysis/debug-area/README.md (Z6, Z8, Z9; D-DA8). Reference with
-- the spawn tables: docs/content/debug-area.md. Templates 1370-1377:
-- Entities/Seed/entity_templates_debug_area_combat.sql.
--
-- Loaded right after spawnlist.sql (db/database.sql). Every row is a mobile
-- `mob`: the spawner grounds its Y onto ihpet_crater_light.nav (D-DA5). On the
-- Z9 terrain the navmesh and the occluder's terrain disagree by up to 2 u, so
-- rows are authored near the mesh height and grounding finishes
-- the job. Heading is yaw = atan2(dx, dz). Placement is pinned on the real nav,
-- occluder and cover by crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/
-- (arena.rs, cover.rs, death_respawn.rs) and in the DB by live_db.rs there.
--

-- Z6 NPC-vs-NPC arena: the east shelf, a flat terrain terrace at y -11.12
-- east of the sunken pit, x 318-395, z -680..-749, 22 u above the pit's water
-- plane. DA-F2 (2026-10-05) moved both fights here from the pit floor, which is
-- a water collision plane (y -33.28, WaterCollisionPrefab_Square) that players
-- sink through to the lakebed at y -52 while NPCs stand on the navmesh on top
-- of it. The shelf is terrain (occluder layer Terrain, navmesh within 0.01 u)
-- and no fluid volume reaches it, but it is the floor of a ruin: its low walls
-- block sight at eye height almost everywhere. The rows below are the only
-- 24 u, 3-against-3 layout on it whose nine sight lines are clear with 1 u to
-- spare on both sides (a grid search on ihpet_crater_light.occ): the strip
-- between the long east-west wall at z -736 and the south edge wall at z -749.
-- The south edge is a 3-14 u drop; the way on is from the east, through the
-- gap at x 390-400, z -738..-744, where DA-08's faction-yard ring pad stands
-- (394, -738). Fight 1: the NID squad on x 354 faces the Praxis squad on x 378,
-- 24 u apart, inside both sides' 30 u aggro radius. The NID guards stay 40 u
-- from that pad, so a player who rings in is not pulled; one who walks west
-- past the Praxis line comes within 30 u of the guards and joins on the Praxis
-- side. A player watching within 150 u (AoI) starts the fight. Fight 2, which
-- no player can join, is in the open room north of the long wall, 39+ u from
-- every NID guard: the Green Sniper pair on x 364 against the Yellow Faction
-- pair on x 343, 21 u apart. Neither fight-2 side targets players.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13600, 378, -11.12, -738, -1.5708, 1300, 1370, 'DebugArea_Arena_Praxis1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13601, 378, -11.12, -742, -1.5708, 1300, 1371, 'DebugArea_Arena_Praxis2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13602, 378, -11.12, -746, -1.5708, 1300, 1370, 'DebugArea_Arena_Praxis3', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13603, 354, -11.12, -738, 1.5708, 1300, 1372, 'DebugArea_Arena_NID1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13604, 354, -11.12, -742, 1.5708, 1300, 1372, 'DebugArea_Arena_NID2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13605, 354, -11.12, -746, 1.5708, 1300, 1372, 'DebugArea_Arena_NID3', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13610, 364, -11.12, -700, -1.5708, 1300, 1373, 'DebugArea_Arena_Green1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13611, 364, -11.12, -696, -1.5708, 1300, 1373, 'DebugArea_Arena_Green2', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13612, 343, -11.12, -700, 1.5708, 1300, 1374, 'DebugArea_Arena_Yellow1', NULL);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13613, 343, -11.12, -696, 1.5708, 1300, 1374, 'DebugArea_Arena_Yellow2', NULL);

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
