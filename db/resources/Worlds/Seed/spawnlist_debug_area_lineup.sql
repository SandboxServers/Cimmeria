--
-- NEW CONTENT (Debug Area, DA-10): Z10, the Visual NPC Lineup, world 1300.
-- docs/content/debug-area.md#visual-npc-lineup has the rows and the walk.
--
-- Id block 13870-14099 (DA-10). Tags `DebugArea_VisualLineup_<source
-- template id>` (`DebugArea_VisualLineup_NoTemplate_<body set>` for the six
-- template-less body sets; D-DA6).
--
-- The east wing of the south compound: an open-air walled ruin (x 297-392,
-- z -969 to -881) east of the courtyard, entered through the doorway at its
-- north-west corner (about (300, 6.6, -897)). Every spot is on the navmesh
-- and on the occluder's terrain top (y 6.56-6.80), found by a 1 m grid
-- search of ihpet_crater_light.nav and .occ with 1 m clear on each side.
--
-- Rows run east-west and face a walkway: N1 (z -888) and N2 (-893) face
-- south and N3 (-902) north across the walkway the doorway opens onto; N4
-- (-909) faces south and N5 (-914) north; through the gap at x 331-349 to
-- the south half, S1 (-937) south / S2 (-942) north, S3 (-947) / S4 (-952),
-- S5 (-956) / S6 (-961). Humanoids stand 2.25 m apart. The creatures stand
-- in E1 (z -909, x 348-382, facing south) and the machines in E3 (-937,
-- x 362-382) and E4 (-942, x 347-359), 4 m apart (8 m for the large bodies);
-- the Straegis Titan stands alone in the open court at (368.5, -922).
-- Heading 0 faces +z. All stationary; none respawns (none can die).
-- Reach it with `.gotolocation DebugArea 300 6.8 -897`.
--
-- Nothing here spawns at startup. Every row belongs to one of five spawn
-- sets (`set_name`, spawn_sets_debug_area_lineup.sql), and a GM shows one
-- set at a time for the whole world: the Lineup attendants at the doorway,
-- `.spawnset on <id>`, or activateSpawnSet(id). All 161 at once ran the
-- 32-bit client out of memory (docs/content/debug-area.md, Arrival load).
--
-- Humans, male
-- 1410 Colonel Marsh #10 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13870, 305.00, 6.58, -888.00, 3.1416, 1300, 1410, 'DebugArea_VisualLineup_10', 'Visual NPC Lineup - Humans', true);
-- 1411 Cellblock Guard #15 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13871, 307.25, 6.58, -888.00, 3.1416, 1300, 1411, 'DebugArea_VisualLineup_15', 'Visual NPC Lineup - Humans', true);
-- 1412 NID Guard #24 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13872, 309.50, 6.58, -888.00, 3.1416, 1300, 1412, 'DebugArea_VisualLineup_24', 'Visual NPC Lineup - Humans', true);
-- 1413 Interaction Debug NPC #25 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13873, 311.75, 6.58, -888.00, 3.1416, 1300, 1413, 'DebugArea_VisualLineup_25', 'Visual NPC Lineup - Humans', true);
-- 1414 TestAvatar #26 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13874, 314.00, 6.58, -888.00, 3.1416, 1300, 1414, 'DebugArea_VisualLineup_26', 'Visual NPC Lineup - Humans', true);
-- 1415 test avatar set - DO NO #28 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13875, 316.25, 6.58, -888.00, 3.1416, 1300, 1415, 'DebugArea_VisualLineup_28', 'Visual NPC Lineup - Humans', true);
-- 1416 General Hammond #29 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13876, 318.50, 6.58, -888.00, 3.1416, 1300, 1416, 'DebugArea_VisualLineup_29', 'Visual NPC Lineup - Humans', true);
-- 1417 Airman #31 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13877, 323.00, 6.58, -893.00, 3.1416, 1300, 1417, 'DebugArea_VisualLineup_31', 'Visual NPC Lineup - Humans', true);
-- 1418 Mr. Woolsey #47 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13878, 325.25, 6.58, -893.00, 3.1416, 1300, 1418, 'DebugArea_VisualLineup_47', 'Visual NPC Lineup - Humans', true);
-- 1419 Dr. Daniel Jackson #51 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13879, 327.50, 6.58, -893.00, 3.1416, 1300, 1419, 'DebugArea_VisualLineup_51', 'Visual NPC Lineup - Humans', true);
-- 1420 Placeholder Gen. Jack O #52 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13880, 329.75, 6.80, -893.00, 3.1416, 1300, 1420, 'DebugArea_VisualLineup_52', 'Visual NPC Lineup - Humans', true);
-- 1421 Nerus #53 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13881, 332.00, 6.58, -893.00, 3.1416, 1300, 1421, 'DebugArea_VisualLineup_53', 'Visual NPC Lineup - Humans', true);
-- 1422 Warrick #55 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13882, 334.25, 6.58, -893.00, 3.1416, 1300, 1422, 'DebugArea_VisualLineup_55', 'Visual NPC Lineup - Humans', true);
-- 1423 Goldam #56 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13883, 336.50, 6.58, -893.00, 3.1416, 1300, 1423, 'DebugArea_VisualLineup_56', 'Visual NPC Lineup - Humans', true);
-- 1424 Major Davis #57 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13884, 338.75, 6.58, -893.00, 3.1416, 1300, 1424, 'DebugArea_VisualLineup_57', 'Visual NPC Lineup - Humans', true);
-- 1425 Sgt. Harriman #58 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13885, 341.00, 6.58, -893.00, 3.1416, 1300, 1425, 'DebugArea_VisualLineup_58', 'Visual NPC Lineup - Humans', true);
-- 1426 NID Guard #146 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13886, 343.25, 6.58, -893.00, 3.1416, 1300, 1426, 'DebugArea_VisualLineup_146', 'Visual NPC Lineup - Humans', true);
-- 1427 Sgt. Gerschon #149 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13887, 345.50, 6.80, -893.00, 3.1416, 1300, 1427, 'DebugArea_VisualLineup_149', 'Visual NPC Lineup - Humans', true);
-- 1428 HumanMale - Not For Us #150 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13888, 306.00, 6.58, -902.00, 0.0000, 1300, 1428, 'DebugArea_VisualLineup_150', 'Visual NPC Lineup - Humans', true);
-- 1429 Blue Faction Scientist #151 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13889, 308.25, 6.58, -902.00, 0.0000, 1300, 1429, 'DebugArea_VisualLineup_151', 'Visual NPC Lineup - Humans', true);
-- 1430 Lucian Slum Dweller #152 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13890, 310.50, 6.58, -902.00, 0.0000, 1300, 1430, 'DebugArea_VisualLineup_152', 'Visual NPC Lineup - Humans', true);
-- 1431 Dr. Zuritska #168 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13891, 312.75, 6.58, -902.00, 0.0000, 1300, 1431, 'DebugArea_VisualLineup_168', 'Visual NPC Lineup - Humans', true);
-- 1432 NID Guard #172 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13892, 315.00, 6.58, -902.00, 0.0000, 1300, 1432, 'DebugArea_VisualLineup_172', 'Visual NPC Lineup - Humans', true);
-- 1433 Op-CORE Soldier #174 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13893, 317.25, 6.58, -902.00, 0.0000, 1300, 1433, 'DebugArea_VisualLineup_174', 'Visual NPC Lineup - Humans', true);
-- 1434 Op-CORE Soldier #175 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13894, 319.50, 6.58, -902.00, 0.0000, 1300, 1434, 'DebugArea_VisualLineup_175', 'Visual NPC Lineup - Humans', true);
-- 1435 Op-CORE Soldier #176 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13895, 321.75, 6.58, -902.00, 0.0000, 1300, 1435, 'DebugArea_VisualLineup_176', 'Visual NPC Lineup - Humans', true);
-- 1436 Sgt. Stanton #178 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13896, 324.00, 6.58, -902.00, 0.0000, 1300, 1436, 'DebugArea_VisualLineup_178', 'Visual NPC Lineup - Humans', true);
-- 1437 Ogilvie #179 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13897, 326.25, 6.58, -902.00, 0.0000, 1300, 1437, 'DebugArea_VisualLineup_179', 'Visual NPC Lineup - Humans', true);
-- 1438 Opheltes #215 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13898, 328.50, 6.58, -902.00, 0.0000, 1300, 1438, 'DebugArea_VisualLineup_215', 'Visual NPC Lineup - Humans', true);
-- 1439 Basic Equipment Quarte #300 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13899, 330.75, 6.58, -902.00, 0.0000, 1300, 1439, 'DebugArea_VisualLineup_300', 'Visual NPC Lineup - Humans', true);
-- 1440 Airman Lance #302 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13900, 333.00, 6.58, -902.00, 0.0000, 1300, 1440, 'DebugArea_VisualLineup_302', 'Visual NPC Lineup - Humans', true);
-- 1441 Common Materials Compo #314 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13901, 335.25, 6.56, -902.00, 0.0000, 1300, 1441, 'DebugArea_VisualLineup_314', 'Visual NPC Lineup - Humans', true);
-- 1442 (no template) HM_BodySet
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13902, 337.50, 6.58, -902.00, 0.0000, 1300, 1442, 'DebugArea_VisualLineup_NoTemplate_HM_BodySet', 'Visual NPC Lineup - Humans', true);
-- Humans, female
-- 1443 Samantha Carter #33 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13903, 339.75, 6.58, -902.00, 0.0000, 1300, 1443, 'DebugArea_VisualLineup_33', 'Visual NPC Lineup - Humans', true);
-- 1444 Capt. Copplemann #48 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13904, 342.00, 6.58, -902.00, 0.0000, 1300, 1444, 'DebugArea_VisualLineup_48', 'Visual NPC Lineup - Humans', true);
-- 1445 Oma Desala #49 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13905, 344.25, 6.58, -902.00, 0.0000, 1300, 1445, 'DebugArea_VisualLineup_49', 'Visual NPC Lineup - Humans', true);
-- 1446 Vala Mal Doran #50 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13906, 346.50, 6.58, -902.00, 0.0000, 1300, 1446, 'DebugArea_VisualLineup_50', 'Visual NPC Lineup - Humans', true);
-- 1447 HumanFemale Template #153 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13907, 348.75, 6.58, -902.00, 0.0000, 1300, 1447, 'DebugArea_VisualLineup_153', 'Visual NPC Lineup - Humans', true);
-- 1448 Warden Muelbach #170 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13908, 351.00, 6.58, -902.00, 0.0000, 1300, 1448, 'DebugArea_VisualLineup_170', 'Visual NPC Lineup - Humans', true);
-- 1449 Castle Medic #177 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13909, 353.25, 6.58, -902.00, 0.0000, 1300, 1449, 'DebugArea_VisualLineup_177', 'Visual NPC Lineup - Humans', true);
-- 1450 Storage Lotaur #219 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13910, 312.00, 6.58, -909.00, 3.1416, 1300, 1450, 'DebugArea_VisualLineup_219', 'Visual NPC Lineup - Humans', true);
-- 1451 Storage Officer #370 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13911, 314.25, 6.58, -909.00, 3.1416, 1300, 1451, 'DebugArea_VisualLineup_370', 'Visual NPC Lineup - Humans', true);
-- Jaffa, male
-- 1452 Teal'c #30 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13912, 316.50, 6.58, -909.00, 3.1416, 1300, 1452, 'DebugArea_VisualLineup_30', 'Visual NPC Lineup - Jaffa male', true);
-- 1453 Jaffa #34 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13913, 318.75, 6.58, -909.00, 3.1416, 1300, 1453, 'DebugArea_VisualLineup_34', 'Visual NPC Lineup - Jaffa male', true);
-- 1454 Bra'tac #59 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13914, 321.00, 6.58, -909.00, 3.1416, 1300, 1454, 'DebugArea_VisualLineup_59', 'Visual NPC Lineup - Jaffa male', true);
-- 1455 Bull Jaffa #82 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13915, 323.25, 6.58, -909.00, 3.1416, 1300, 1455, 'DebugArea_VisualLineup_82', 'Visual NPC Lineup - Jaffa male', true);
-- 1456 Asian Jaffa #83 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13916, 325.50, 6.58, -909.00, 3.1416, 1300, 1456, 'DebugArea_VisualLineup_83', 'Visual NPC Lineup - Jaffa male', true);
-- 1457 Cat Jaffa #84 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13917, 327.75, 6.58, -909.00, 3.1416, 1300, 1457, 'DebugArea_VisualLineup_84', 'Visual NPC Lineup - Jaffa male', true);
-- 1458 Cobra Jaffa #85 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13918, 330.00, 6.58, -909.00, 3.1416, 1300, 1458, 'DebugArea_VisualLineup_85', 'Visual NPC Lineup - Jaffa male', true);
-- 1459 Croc Jaffa #86 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13919, 332.25, 6.58, -909.00, 3.1416, 1300, 1459, 'DebugArea_VisualLineup_86', 'Visual NPC Lineup - Jaffa male', true);
-- 1460 Demon Jaffa #87 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13920, 302.00, 6.58, -914.00, 0.0000, 1300, 1460, 'DebugArea_VisualLineup_87', 'Visual NPC Lineup - Jaffa male', true);
-- 1461 Dragon Jaffa #88 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13921, 304.25, 6.58, -914.00, 0.0000, 1300, 1461, 'DebugArea_VisualLineup_88', 'Visual NPC Lineup - Jaffa male', true);
-- 1462 Eagle Jaffa #89 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13922, 306.50, 6.58, -914.00, 0.0000, 1300, 1462, 'DebugArea_VisualLineup_89', 'Visual NPC Lineup - Jaffa male', true);
-- 1463 Falcon Jaffa #90 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13923, 308.75, 6.58, -914.00, 0.0000, 1300, 1463, 'DebugArea_VisualLineup_90', 'Visual NPC Lineup - Jaffa male', true);
-- 1464 Horse Jaffa #91 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13924, 311.00, 6.58, -914.00, 0.0000, 1300, 1464, 'DebugArea_VisualLineup_91', 'Visual NPC Lineup - Jaffa male', true);
-- 1465 Hyena Jaffa #92 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13925, 313.25, 6.58, -914.00, 0.0000, 1300, 1465, 'DebugArea_VisualLineup_92', 'Visual NPC Lineup - Jaffa male', true);
-- 1466 Jackal Jaffa #93 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13926, 315.50, 6.80, -914.00, 0.0000, 1300, 1466, 'DebugArea_VisualLineup_93', 'Visual NPC Lineup - Jaffa male', true);
-- 1467 Mayan Jaffa #94 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13927, 317.75, 6.80, -914.00, 0.0000, 1300, 1467, 'DebugArea_VisualLineup_94', 'Visual NPC Lineup - Jaffa male', true);
-- 1468 Morrigan Jaffa #95 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13928, 320.00, 6.58, -914.00, 0.0000, 1300, 1468, 'DebugArea_VisualLineup_95', 'Visual NPC Lineup - Jaffa male', true);
-- 1469 Naga Jaffa #96 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13929, 322.25, 6.58, -914.00, 0.0000, 1300, 1469, 'DebugArea_VisualLineup_96', 'Visual NPC Lineup - Jaffa male', true);
-- 1470 Praxis Jaffa #97 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13930, 324.50, 6.58, -914.00, 0.0000, 1300, 1470, 'DebugArea_VisualLineup_97', 'Visual NPC Lineup - Jaffa male', true);
-- 1471 Praxis Jaffa 2 #98 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13931, 326.75, 6.80, -914.00, 0.0000, 1300, 1471, 'DebugArea_VisualLineup_98', 'Visual NPC Lineup - Jaffa male', true);
-- 1472 Ra Jaffa #99 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13932, 329.00, 6.80, -914.00, 0.0000, 1300, 1472, 'DebugArea_VisualLineup_99', 'Visual NPC Lineup - Jaffa male', true);
-- 1473 Standard Jaffa #100 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13933, 331.25, 6.58, -914.00, 0.0000, 1300, 1473, 'DebugArea_VisualLineup_100', 'Visual NPC Lineup - Jaffa male', true);
-- 1474 Savarog Jaffa #101 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13934, 333.50, 6.58, -914.00, 0.0000, 1300, 1474, 'DebugArea_VisualLineup_101', 'Visual NPC Lineup - Jaffa male', true);
-- 1475 Tiki Jaffa #102 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13935, 335.75, 6.58, -914.00, 0.0000, 1300, 1475, 'DebugArea_VisualLineup_102', 'Visual NPC Lineup - Jaffa male', true);
-- 1476 Unas_1 #105 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13936, 306.00, 6.58, -937.00, 3.1416, 1300, 1476, 'DebugArea_VisualLineup_105', 'Visual NPC Lineup - Jaffa male', true);
-- 1477 Unas_2 #106 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13937, 308.25, 6.58, -937.00, 3.1416, 1300, 1477, 'DebugArea_VisualLineup_106', 'Visual NPC Lineup - Jaffa male', true);
-- 1478 Unas_3 #107 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13938, 310.50, 6.58, -937.00, 3.1416, 1300, 1478, 'DebugArea_VisualLineup_107', 'Visual NPC Lineup - Jaffa male', true);
-- 1479 Unas_4 #108 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13939, 312.75, 6.58, -937.00, 3.1416, 1300, 1479, 'DebugArea_VisualLineup_108', 'Visual NPC Lineup - Jaffa male', true);
-- 1480 Unas_5 #109 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13940, 315.00, 6.80, -937.00, 3.1416, 1300, 1480, 'DebugArea_VisualLineup_109', 'Visual NPC Lineup - Jaffa male', true);
-- 1481 Unas_6 #110 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13941, 317.25, 6.70, -937.00, 3.1416, 1300, 1481, 'DebugArea_VisualLineup_110', 'Visual NPC Lineup - Jaffa male', true);
-- 1482 Viking Jaffa #111 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13942, 319.50, 6.58, -937.00, 3.1416, 1300, 1482, 'DebugArea_VisualLineup_111', 'Visual NPC Lineup - Jaffa male', true);
-- 1483 Ra Jaffa 2 #142 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13943, 321.75, 6.58, -937.00, 3.1416, 1300, 1483, 'DebugArea_VisualLineup_142', 'Visual NPC Lineup - Jaffa male', true);
-- 1484 Ra's Officer #143 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13944, 324.00, 6.58, -937.00, 3.1416, 1300, 1484, 'DebugArea_VisualLineup_143', 'Visual NPC Lineup - Jaffa male', true);
-- 1485 JaffaMale Template - D #155 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13945, 326.25, 6.58, -937.00, 3.1416, 1300, 1485, 'DebugArea_VisualLineup_155', 'Visual NPC Lineup - Jaffa male', true);
-- 1486 Praxis Jaffa Guard #160 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13946, 328.50, 6.80, -937.00, 3.1416, 1300, 1486, 'DebugArea_VisualLineup_160', 'Visual NPC Lineup - Jaffa male', true);
-- 1487 Petbe #163 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13947, 330.75, 6.58, -937.00, 3.1416, 1300, 1487, 'DebugArea_VisualLineup_163', 'Visual NPC Lineup - Jaffa male', true);
-- 1488 Mala'c #200 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13948, 333.00, 6.58, -937.00, 3.1416, 1300, 1488, 'DebugArea_VisualLineup_200', 'Visual NPC Lineup - Jaffa male', true);
-- 1489 Bra'hin #202 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13949, 335.25, 6.58, -937.00, 3.1416, 1300, 1489, 'DebugArea_VisualLineup_202', 'Visual NPC Lineup - Jaffa male', true);
-- 1490 Ra's Jaffa #203 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13950, 337.50, 6.58, -937.00, 3.1416, 1300, 1490, 'DebugArea_VisualLineup_203', 'Visual NPC Lineup - Jaffa male', true);
-- 1491 Angry Jaffa #206 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13951, 316.00, 6.58, -942.00, 0.0000, 1300, 1491, 'DebugArea_VisualLineup_206', 'Visual NPC Lineup - Jaffa male', true);
-- 1492 Petbe #221 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13952, 318.25, 6.58, -942.00, 0.0000, 1300, 1492, 'DebugArea_VisualLineup_221', 'Visual NPC Lineup - Jaffa male', true);
-- 1493 Jaffa #1310 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13953, 320.50, 6.58, -942.00, 0.0000, 1300, 1493, 'DebugArea_VisualLineup_1310', 'Visual NPC Lineup - Jaffa male', true);
-- 1494 Praxis Jaffa Guard #1371 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13954, 322.75, 6.58, -942.00, 0.0000, 1300, 1494, 'DebugArea_VisualLineup_1371', 'Visual NPC Lineup - Jaffa male', true);
-- 1495 (no template) BS_RaJaff
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13955, 325.00, 6.58, -942.00, 0.0000, 1300, 1495, 'DebugArea_VisualLineup_NoTemplate_BS_RaJaff', 'Visual NPC Lineup - Jaffa male', true);
-- Jaffa, female
-- 1496 Moh'Katan #54 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13956, 327.25, 6.58, -942.00, 0.0000, 1300, 1496, 'DebugArea_VisualLineup_54', 'Visual NPC Lineup - Jaffa female', true);
-- 1497 Asian Jaffa Female #112 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13957, 329.50, 6.58, -942.00, 0.0000, 1300, 1497, 'DebugArea_VisualLineup_112', 'Visual NPC Lineup - Jaffa female', true);
-- 1498 Bull Jaffa Female #113 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13958, 331.75, 6.58, -942.00, 0.0000, 1300, 1498, 'DebugArea_VisualLineup_113', 'Visual NPC Lineup - Jaffa female', true);
-- 1499 Cat Jaffa Female #114 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13959, 334.00, 6.58, -942.00, 0.0000, 1300, 1499, 'DebugArea_VisualLineup_114', 'Visual NPC Lineup - Jaffa female', true);
-- 1500 Cobra Jaffa Female #115 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13960, 336.25, 6.58, -942.00, 0.0000, 1300, 1500, 'DebugArea_VisualLineup_115', 'Visual NPC Lineup - Jaffa female', true);
-- 1501 Croc Jaffa Female #116 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13961, 338.50, 6.58, -942.00, 0.0000, 1300, 1501, 'DebugArea_VisualLineup_116', 'Visual NPC Lineup - Jaffa female', true);
-- 1502 Demon Jaffa Female #117 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13962, 329.00, 6.58, -947.00, 3.1416, 1300, 1502, 'DebugArea_VisualLineup_117', 'Visual NPC Lineup - Jaffa female', true);
-- 1503 Dragon Jaffa Female #118 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13963, 331.25, 6.58, -947.00, 3.1416, 1300, 1503, 'DebugArea_VisualLineup_118', 'Visual NPC Lineup - Jaffa female', true);
-- 1504 Eagle Jaffa Female #119 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13964, 333.50, 6.58, -947.00, 3.1416, 1300, 1504, 'DebugArea_VisualLineup_119', 'Visual NPC Lineup - Jaffa female', true);
-- 1505 Falcon Jaffa Female #120 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13965, 335.75, 6.58, -947.00, 3.1416, 1300, 1505, 'DebugArea_VisualLineup_120', 'Visual NPC Lineup - Jaffa female', true);
-- 1506 Horse Jaffa Female #121 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13966, 338.00, 6.58, -947.00, 3.1416, 1300, 1506, 'DebugArea_VisualLineup_121', 'Visual NPC Lineup - Jaffa female', true);
-- 1507 Hyena Jaffa Female #122 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13967, 340.25, 6.58, -947.00, 3.1416, 1300, 1507, 'DebugArea_VisualLineup_122', 'Visual NPC Lineup - Jaffa female', true);
-- 1508 Jackal Jaffa Female #123 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13968, 342.50, 6.58, -947.00, 3.1416, 1300, 1508, 'DebugArea_VisualLineup_123', 'Visual NPC Lineup - Jaffa female', true);
-- 1509 Mayan Jaffa Female #124 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13969, 344.75, 6.58, -947.00, 3.1416, 1300, 1509, 'DebugArea_VisualLineup_124', 'Visual NPC Lineup - Jaffa female', true);
-- 1510 Morrigan Jaffa Femal #125 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13970, 347.00, 6.58, -947.00, 3.1416, 1300, 1510, 'DebugArea_VisualLineup_125', 'Visual NPC Lineup - Jaffa female', true);
-- 1511 Naga Jaffa Female #126 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13971, 349.25, 6.58, -947.00, 3.1416, 1300, 1511, 'DebugArea_VisualLineup_126', 'Visual NPC Lineup - Jaffa female', true);
-- 1512 Praxis Jaffa 2 Femal #127 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13972, 351.50, 6.58, -947.00, 3.1416, 1300, 1512, 'DebugArea_VisualLineup_127', 'Visual NPC Lineup - Jaffa female', true);
-- 1513 Praxis Jaffa 1 Femal #128 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13973, 315.00, 6.58, -952.00, 0.0000, 1300, 1513, 'DebugArea_VisualLineup_128', 'Visual NPC Lineup - Jaffa female', true);
-- 1514 Standard Jaffa Femal #129 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13974, 317.25, 6.58, -952.00, 0.0000, 1300, 1514, 'DebugArea_VisualLineup_129', 'Visual NPC Lineup - Jaffa female', true);
-- 1515 Svarog Jaffa Female #130 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13975, 319.50, 6.58, -952.00, 0.0000, 1300, 1515, 'DebugArea_VisualLineup_130', 'Visual NPC Lineup - Jaffa female', true);
-- 1516 Tiki Jaffa Female #131 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13976, 321.75, 6.58, -952.00, 0.0000, 1300, 1516, 'DebugArea_VisualLineup_131', 'Visual NPC Lineup - Jaffa female', true);
-- 1517 Unas 1 Female #132 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13977, 324.00, 6.58, -952.00, 0.0000, 1300, 1517, 'DebugArea_VisualLineup_132', 'Visual NPC Lineup - Jaffa female', true);
-- 1518 Unas 2 Female #133 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13978, 326.25, 6.58, -952.00, 0.0000, 1300, 1518, 'DebugArea_VisualLineup_133', 'Visual NPC Lineup - Jaffa female', true);
-- 1519 Unas 3 Female #134 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13979, 328.50, 6.58, -952.00, 0.0000, 1300, 1519, 'DebugArea_VisualLineup_134', 'Visual NPC Lineup - Jaffa female', true);
-- 1520 Unas 4 Female #135 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13980, 330.75, 6.58, -952.00, 0.0000, 1300, 1520, 'DebugArea_VisualLineup_135', 'Visual NPC Lineup - Jaffa female', true);
-- 1521 Unas 5 Female #136 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13981, 333.00, 6.58, -952.00, 0.0000, 1300, 1521, 'DebugArea_VisualLineup_136', 'Visual NPC Lineup - Jaffa female', true);
-- 1522 Unas 6 Female #137 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13982, 335.25, 6.58, -952.00, 0.0000, 1300, 1522, 'DebugArea_VisualLineup_137', 'Visual NPC Lineup - Jaffa female', true);
-- 1523 Viking Jaffa Female #138 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13983, 307.00, 6.58, -956.00, 3.1416, 1300, 1523, 'DebugArea_VisualLineup_138', 'Visual NPC Lineup - Jaffa female', true);
-- 1524 Clothed Jaffa Female #139 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13984, 309.25, 6.58, -956.00, 3.1416, 1300, 1524, 'DebugArea_VisualLineup_139', 'Visual NPC Lineup - Jaffa female', true);
-- 1525 JaffaFemale Template #154 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13985, 311.50, 6.58, -956.00, 3.1416, 1300, 1525, 'DebugArea_VisualLineup_154', 'Visual NPC Lineup - Jaffa female', true);
-- Goa'uld, male
-- 1526 Prisoner 329 #17 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13986, 313.75, 6.58, -956.00, 3.1416, 1300, 1526, 'DebugArea_VisualLineup_17', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1527 Ra #41 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13987, 316.00, 6.58, -956.00, 3.1416, 1300, 1527, 'DebugArea_VisualLineup_41', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1528 Ba'al #42 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13988, 318.25, 6.58, -956.00, 3.1416, 1300, 1528, 'DebugArea_VisualLineup_42', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1529 Ra #60 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13989, 320.50, 6.58, -956.00, 3.1416, 1300, 1529, 'DebugArea_VisualLineup_60', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1530 Ra #61 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13990, 327.00, 6.58, -956.00, 3.1416, 1300, 1530, 'DebugArea_VisualLineup_61', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1531 Ra #62 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13991, 329.25, 6.58, -956.00, 3.1416, 1300, 1531, 'DebugArea_VisualLineup_62', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1532 Ra #63 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13992, 331.50, 6.58, -956.00, 3.1416, 1300, 1532, 'DebugArea_VisualLineup_63', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1533 GoauldMale Template #158 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13993, 333.75, 6.58, -956.00, 3.1416, 1300, 1533, 'DebugArea_VisualLineup_158', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1534 Ba'al #167 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13994, 336.00, 6.58, -956.00, 3.1416, 1300, 1534, 'DebugArea_VisualLineup_167', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1535 Haughty Goa'uld #210 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13995, 338.25, 6.58, -956.00, 3.1416, 1300, 1535, 'DebugArea_VisualLineup_210', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1536 Ashrak Assassin #211 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13996, 340.50, 6.58, -956.00, 3.1416, 1300, 1536, 'DebugArea_VisualLineup_211', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1537 Lo'Taur Servant #353 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13997, 326.00, 6.58, -961.00, 0.0000, 1300, 1537, 'DebugArea_VisualLineup_353', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1538 Ra #1400 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13998, 328.25, 6.58, -961.00, 0.0000, 1300, 1538, 'DebugArea_VisualLineup_1400', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1539 Ba'al #1401 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13999, 330.50, 6.58, -961.00, 0.0000, 1300, 1539, 'DebugArea_VisualLineup_1401', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- Goa'uld, female
-- 1540 Anat #43 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14000, 332.75, 6.58, -961.00, 0.0000, 1300, 1540, 'DebugArea_VisualLineup_43', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1541 Athena #44 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14001, 335.00, 6.58, -961.00, 0.0000, 1300, 1541, 'DebugArea_VisualLineup_44', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1542 Morrigan #45 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14002, 337.25, 6.58, -961.00, 0.0000, 1300, 1542, 'DebugArea_VisualLineup_45', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1543 GoauldFemale Templa #157 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14003, 339.50, 6.58, -961.00, 0.0000, 1300, 1543, 'DebugArea_VisualLineup_157', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1544 Athena #1403 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14004, 341.75, 6.58, -961.00, 0.0000, 1300, 1544, 'DebugArea_VisualLineup_1403', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1545 Morrigan #1404 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14005, 344.00, 6.58, -961.00, 0.0000, 1300, 1545, 'DebugArea_VisualLineup_1404', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- Asgard
-- 1546 Thor #64 BS_Asgard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14006, 346.25, 6.58, -961.00, 0.0000, 1300, 1546, 'DebugArea_VisualLineup_64', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1547 Asgard Template - DO NOT #156 BS_Asgard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14007, 348.50, 6.58, -961.00, 0.0000, 1300, 1547, 'DebugArea_VisualLineup_156', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1548 (no template) BS_Degenerated_Asgard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14008, 350.75, 6.58, -961.00, 0.0000, 1300, 1548, 'DebugArea_VisualLineup_NoTemplate_BS_Degenerated_Asgard', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- Children
-- 1549 Nox Child Male #66 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14009, 353.00, 6.58, -961.00, 0.0000, 1300, 1549, 'DebugArea_VisualLineup_66', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1550 Nox Child Female #67 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14010, 355.25, 6.58, -961.00, 0.0000, 1300, 1550, 'DebugArea_VisualLineup_67', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1551 Blix #68 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14011, 357.50, 6.58, -961.00, 0.0000, 1300, 1551, 'DebugArea_VisualLineup_68', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1552 NPC Child 1 #140 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14012, 359.75, 6.58, -961.00, 0.0000, 1300, 1552, 'DebugArea_VisualLineup_140', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- 1553 NPC Child 2 #141 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14013, 362.00, 6.58, -961.00, 0.0000, 1300, 1553, 'DebugArea_VisualLineup_141', 'Visual NPC Lineup - Goa''uld, Asgard and children', true);
-- Creatures
-- 1554 Rat #74 BS_MOB_Rat
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14014, 348.00, 6.58, -909.00, 3.1416, 1300, 1554, 'DebugArea_VisualLineup_74', 'Visual NPC Lineup - Creatures and machines', true);
-- 1555 ScavDog #76 BS_MOB_ScavDog
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14015, 352.00, 6.58, -909.00, 3.1416, 1300, 1555, 'DebugArea_VisualLineup_76', 'Visual NPC Lineup - Creatures and machines', true);
-- 1556 Lenny #73 BS_MOB_Lenny
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14016, 356.00, 6.58, -909.00, 3.1416, 1300, 1556, 'DebugArea_VisualLineup_73', 'Visual NPC Lineup - Creatures and machines', true);
-- 1557 (no template) BS_MOB_LennyBaby
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14017, 360.00, 6.58, -909.00, 3.1416, 1300, 1557, 'DebugArea_VisualLineup_NoTemplate_BS_MOB_LennyBaby', 'Visual NPC Lineup - Creatures and machines', true);
-- 1558 Horden #72 BS_MOB_Horden
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14018, 366.00, 6.58, -909.00, 3.1416, 1300, 1558, 'DebugArea_VisualLineup_72', 'Visual NPC Lineup - Creatures and machines', true);
-- 1559 Carnosaur #71 BS_MOB_Carnosaur
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14019, 374.00, 6.58, -909.00, 3.1416, 1300, 1559, 'DebugArea_VisualLineup_71', 'Visual NPC Lineup - Creatures and machines', true);
-- 1560 Rhinolion #75 BS_MOB_Rhinolion00
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14020, 382.00, 6.58, -909.00, 3.1416, 1300, 1560, 'DebugArea_VisualLineup_75', 'Visual NPC Lineup - Creatures and machines', true);
-- 1561 Twilla Vines #80 BS_MOB_TwillaTree
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14021, 362.00, 6.58, -937.00, 3.1416, 1300, 1561, 'DebugArea_VisualLineup_80', 'Visual NPC Lineup - Creatures and machines', true);
-- Machines
-- 1562 (no template) BS_AN_Android
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14022, 347.00, 6.58, -942.00, 0.0000, 1300, 1562, 'DebugArea_VisualLineup_NoTemplate_BS_AN_Android', 'Visual NPC Lineup - Creatures and machines', true);
-- 1563 (no template) BS_MOB_DroneTank
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14023, 351.00, 6.58, -942.00, 0.0000, 1300, 1563, 'DebugArea_VisualLineup_NoTemplate_BS_MOB_DroneTank', 'Visual NPC Lineup - Creatures and machines', true);
-- 1564 Prisoner retrieval #4 BS_MOB_DroneFlyer
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14024, 355.00, 6.58, -942.00, 0.0000, 1300, 1564, 'DebugArea_VisualLineup_4', 'Visual NPC Lineup - Creatures and machines', true);
-- 1565 Malfunctioning Dron #69 MOB_Goauld_Drone
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14025, 359.00, 6.58, -942.00, 0.0000, 1300, 1565, 'DebugArea_VisualLineup_69', 'Visual NPC Lineup - Creatures and machines', true);
-- 1566 Agnos Drone #81 MOB_AncientDrone_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14026, 368.00, 6.58, -937.00, 3.1416, 1300, 1566, 'DebugArea_VisualLineup_81', 'Visual NPC Lineup - Creatures and machines', true);
-- 1567 Straegis Figh #78 BS_MOB_StraegisFighter
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14027, 372.00, 6.58, -937.00, 3.1416, 1300, 1567, 'DebugArea_VisualLineup_78', 'Visual NPC Lineup - Creatures and machines', true);
-- 1568 Straegis Beaco #77 BS_MOB_StraegisBeacon
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14028, 376.00, 6.58, -937.00, 3.1416, 1300, 1568, 'DebugArea_VisualLineup_77', 'Visual NPC Lineup - Creatures and machines', true);
-- 1569 BattleWalker #70 BS_MOB_BattleWalker
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14029, 382.00, 6.58, -937.00, 3.1416, 1300, 1569, 'DebugArea_VisualLineup_70', 'Visual NPC Lineup - Creatures and machines', true);
-- 1570 Straegis Titan #79 BS_MOB_StraegisTitan
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14030, 368.50, 6.58, -922.00, 0.0000, 1300, 1570, 'DebugArea_VisualLineup_79', 'Visual NPC Lineup - Creatures and machines', true);

-- Lineup attendants (14090-14095): the buttons. A row of six on the
-- courtyard ground north of the doorway path (z -894, x 281-291, 2 m
-- apart, facing south onto the path), left to right: Humans, Jaffa male,
-- Jaffa female, Goa'uld/Asgard/children, Creatures and machines, Clear.
-- Always spawned (no set).
-- 14090 attendant: Show Humans (42)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14090, 281.00, 6.58, -894.00, 3.1416, 1300, 1590, 'DebugArea_LineupAttendant_1301', NULL, true);
-- 14091 attendant: Show Jaffa male (44)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14091, 283.00, 6.58, -894.00, 3.1416, 1300, 1591, 'DebugArea_LineupAttendant_1302', NULL, true);
-- 14092 attendant: Show Jaffa female (30)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14092, 285.00, 6.58, -894.00, 3.1416, 1300, 1592, 'DebugArea_LineupAttendant_1303', NULL, true);
-- 14093 attendant: Show Goa'uld, Asgard and children (28)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14093, 287.00, 6.58, -894.00, 3.1416, 1300, 1593, 'DebugArea_LineupAttendant_1304', NULL, true);
-- 14094 attendant: Show Creatures and machines (17)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14094, 289.00, 6.58, -894.00, 3.1416, 1300, 1594, 'DebugArea_LineupAttendant_1305', NULL, true);
-- 14095 attendant: Clear lineup
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14095, 291.00, 6.58, -894.00, 3.1416, 1300, 1595, 'DebugArea_LineupAttendant_Clear', NULL, true);

-- Keep default-id inserts (`.savespawn`) past DA-10's reserved block
-- 13870-14099.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 14099), true);
