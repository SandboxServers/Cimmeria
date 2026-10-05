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
-- Humans, male
-- 1410 Colonel Marsh #10 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13870, 305.00, 6.58, -888.00, 3.1416, 1300, 1410, 'DebugArea_VisualLineup_10', NULL, true);
-- 1411 Cellblock Guard #15 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13871, 307.25, 6.58, -888.00, 3.1416, 1300, 1411, 'DebugArea_VisualLineup_15', NULL, true);
-- 1412 NID Guard #24 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13872, 309.50, 6.58, -888.00, 3.1416, 1300, 1412, 'DebugArea_VisualLineup_24', NULL, true);
-- 1413 Interaction Debug NPC #25 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13873, 311.75, 6.58, -888.00, 3.1416, 1300, 1413, 'DebugArea_VisualLineup_25', NULL, true);
-- 1414 TestAvatar #26 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13874, 314.00, 6.58, -888.00, 3.1416, 1300, 1414, 'DebugArea_VisualLineup_26', NULL, true);
-- 1415 test avatar set - DO NO #28 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13875, 316.25, 6.58, -888.00, 3.1416, 1300, 1415, 'DebugArea_VisualLineup_28', NULL, true);
-- 1416 General Hammond #29 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13876, 318.50, 6.58, -888.00, 3.1416, 1300, 1416, 'DebugArea_VisualLineup_29', NULL, true);
-- 1417 Airman #31 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13877, 323.00, 6.58, -893.00, 3.1416, 1300, 1417, 'DebugArea_VisualLineup_31', NULL, true);
-- 1418 Mr. Woolsey #47 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13878, 325.25, 6.58, -893.00, 3.1416, 1300, 1418, 'DebugArea_VisualLineup_47', NULL, true);
-- 1419 Dr. Daniel Jackson #51 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13879, 327.50, 6.58, -893.00, 3.1416, 1300, 1419, 'DebugArea_VisualLineup_51', NULL, true);
-- 1420 Placeholder Gen. Jack O #52 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13880, 329.75, 6.80, -893.00, 3.1416, 1300, 1420, 'DebugArea_VisualLineup_52', NULL, true);
-- 1421 Nerus #53 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13881, 332.00, 6.58, -893.00, 3.1416, 1300, 1421, 'DebugArea_VisualLineup_53', NULL, true);
-- 1422 Warrick #55 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13882, 334.25, 6.58, -893.00, 3.1416, 1300, 1422, 'DebugArea_VisualLineup_55', NULL, true);
-- 1423 Goldam #56 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13883, 336.50, 6.58, -893.00, 3.1416, 1300, 1423, 'DebugArea_VisualLineup_56', NULL, true);
-- 1424 Major Davis #57 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13884, 338.75, 6.58, -893.00, 3.1416, 1300, 1424, 'DebugArea_VisualLineup_57', NULL, true);
-- 1425 Sgt. Harriman #58 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13885, 341.00, 6.58, -893.00, 3.1416, 1300, 1425, 'DebugArea_VisualLineup_58', NULL, true);
-- 1426 NID Guard #146 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13886, 343.25, 6.58, -893.00, 3.1416, 1300, 1426, 'DebugArea_VisualLineup_146', NULL, true);
-- 1427 Sgt. Gerschon #149 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13887, 345.50, 6.80, -893.00, 3.1416, 1300, 1427, 'DebugArea_VisualLineup_149', NULL, true);
-- 1428 HumanMale - Not For Us #150 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13888, 306.00, 6.58, -902.00, 0.0000, 1300, 1428, 'DebugArea_VisualLineup_150', NULL, true);
-- 1429 Blue Faction Scientist #151 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13889, 308.25, 6.58, -902.00, 0.0000, 1300, 1429, 'DebugArea_VisualLineup_151', NULL, true);
-- 1430 Lucian Slum Dweller #152 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13890, 310.50, 6.58, -902.00, 0.0000, 1300, 1430, 'DebugArea_VisualLineup_152', NULL, true);
-- 1431 Nerus #166 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13891, 312.75, 6.58, -902.00, 0.0000, 1300, 1431, 'DebugArea_VisualLineup_166', NULL, true);
-- 1432 Dr. Zuritska #168 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13892, 315.00, 6.58, -902.00, 0.0000, 1300, 1432, 'DebugArea_VisualLineup_168', NULL, true);
-- 1433 NID Guard #172 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13893, 317.25, 6.58, -902.00, 0.0000, 1300, 1433, 'DebugArea_VisualLineup_172', NULL, true);
-- 1434 Op-CORE Soldier #174 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13894, 319.50, 6.58, -902.00, 0.0000, 1300, 1434, 'DebugArea_VisualLineup_174', NULL, true);
-- 1435 Op-CORE Soldier #175 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13895, 321.75, 6.58, -902.00, 0.0000, 1300, 1435, 'DebugArea_VisualLineup_175', NULL, true);
-- 1436 Op-CORE Soldier #176 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13896, 324.00, 6.58, -902.00, 0.0000, 1300, 1436, 'DebugArea_VisualLineup_176', NULL, true);
-- 1437 Sgt. Stanton #178 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13897, 326.25, 6.58, -902.00, 0.0000, 1300, 1437, 'DebugArea_VisualLineup_178', NULL, true);
-- 1438 Ogilvie #179 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13898, 328.50, 6.58, -902.00, 0.0000, 1300, 1438, 'DebugArea_VisualLineup_179', NULL, true);
-- 1439 Opheltes #215 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13899, 330.75, 6.58, -902.00, 0.0000, 1300, 1439, 'DebugArea_VisualLineup_215', NULL, true);
-- 1440 Basic Equipment Quarte #300 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13900, 333.00, 6.58, -902.00, 0.0000, 1300, 1440, 'DebugArea_VisualLineup_300', NULL, true);
-- 1441 Airman Lance #302 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13901, 335.25, 6.56, -902.00, 0.0000, 1300, 1441, 'DebugArea_VisualLineup_302', NULL, true);
-- 1442 Common Materials Compo #314 BS_HumanMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13902, 337.50, 6.58, -902.00, 0.0000, 1300, 1442, 'DebugArea_VisualLineup_314', NULL, true);
-- 1443 (no template) HM_BodySet
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13903, 339.75, 6.58, -902.00, 0.0000, 1300, 1443, 'DebugArea_VisualLineup_NoTemplate_HM_BodySet', NULL, true);
-- Humans, female
-- 1444 Samantha Carter #33 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13904, 342.00, 6.58, -902.00, 0.0000, 1300, 1444, 'DebugArea_VisualLineup_33', NULL, true);
-- 1445 Capt. Copplemann #48 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13905, 344.25, 6.58, -902.00, 0.0000, 1300, 1445, 'DebugArea_VisualLineup_48', NULL, true);
-- 1446 Oma Desala #49 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13906, 346.50, 6.58, -902.00, 0.0000, 1300, 1446, 'DebugArea_VisualLineup_49', NULL, true);
-- 1447 Vala Mal Doran #50 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13907, 348.75, 6.58, -902.00, 0.0000, 1300, 1447, 'DebugArea_VisualLineup_50', NULL, true);
-- 1448 HumanFemale Template #153 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13908, 351.00, 6.58, -902.00, 0.0000, 1300, 1448, 'DebugArea_VisualLineup_153', NULL, true);
-- 1449 Warden Muelbach #170 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13909, 353.25, 6.58, -902.00, 0.0000, 1300, 1449, 'DebugArea_VisualLineup_170', NULL, true);
-- 1450 Castle Medic #177 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13910, 312.00, 6.58, -909.00, 3.1416, 1300, 1450, 'DebugArea_VisualLineup_177', NULL, true);
-- 1451 Storage Lotaur #219 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13911, 314.25, 6.58, -909.00, 3.1416, 1300, 1451, 'DebugArea_VisualLineup_219', NULL, true);
-- 1452 Storage Officer #370 BS_HumanFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13912, 316.50, 6.58, -909.00, 3.1416, 1300, 1452, 'DebugArea_VisualLineup_370', NULL, true);
-- Jaffa, male
-- 1453 Teal'c #30 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13913, 318.75, 6.58, -909.00, 3.1416, 1300, 1453, 'DebugArea_VisualLineup_30', NULL, true);
-- 1454 Jaffa #34 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13914, 321.00, 6.58, -909.00, 3.1416, 1300, 1454, 'DebugArea_VisualLineup_34', NULL, true);
-- 1455 Bra'tac #59 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13915, 323.25, 6.58, -909.00, 3.1416, 1300, 1455, 'DebugArea_VisualLineup_59', NULL, true);
-- 1456 Bull Jaffa #82 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13916, 325.50, 6.58, -909.00, 3.1416, 1300, 1456, 'DebugArea_VisualLineup_82', NULL, true);
-- 1457 Asian Jaffa #83 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13917, 327.75, 6.58, -909.00, 3.1416, 1300, 1457, 'DebugArea_VisualLineup_83', NULL, true);
-- 1458 Cat Jaffa #84 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13918, 330.00, 6.58, -909.00, 3.1416, 1300, 1458, 'DebugArea_VisualLineup_84', NULL, true);
-- 1459 Cobra Jaffa #85 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13919, 332.25, 6.58, -909.00, 3.1416, 1300, 1459, 'DebugArea_VisualLineup_85', NULL, true);
-- 1460 Croc Jaffa #86 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13920, 302.00, 6.58, -914.00, 0.0000, 1300, 1460, 'DebugArea_VisualLineup_86', NULL, true);
-- 1461 Demon Jaffa #87 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13921, 304.25, 6.58, -914.00, 0.0000, 1300, 1461, 'DebugArea_VisualLineup_87', NULL, true);
-- 1462 Dragon Jaffa #88 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13922, 306.50, 6.58, -914.00, 0.0000, 1300, 1462, 'DebugArea_VisualLineup_88', NULL, true);
-- 1463 Eagle Jaffa #89 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13923, 308.75, 6.58, -914.00, 0.0000, 1300, 1463, 'DebugArea_VisualLineup_89', NULL, true);
-- 1464 Falcon Jaffa #90 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13924, 311.00, 6.58, -914.00, 0.0000, 1300, 1464, 'DebugArea_VisualLineup_90', NULL, true);
-- 1465 Horse Jaffa #91 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13925, 313.25, 6.58, -914.00, 0.0000, 1300, 1465, 'DebugArea_VisualLineup_91', NULL, true);
-- 1466 Hyena Jaffa #92 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13926, 315.50, 6.80, -914.00, 0.0000, 1300, 1466, 'DebugArea_VisualLineup_92', NULL, true);
-- 1467 Jackal Jaffa #93 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13927, 317.75, 6.80, -914.00, 0.0000, 1300, 1467, 'DebugArea_VisualLineup_93', NULL, true);
-- 1468 Mayan Jaffa #94 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13928, 320.00, 6.58, -914.00, 0.0000, 1300, 1468, 'DebugArea_VisualLineup_94', NULL, true);
-- 1469 Morrigan Jaffa #95 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13929, 322.25, 6.58, -914.00, 0.0000, 1300, 1469, 'DebugArea_VisualLineup_95', NULL, true);
-- 1470 Naga Jaffa #96 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13930, 324.50, 6.58, -914.00, 0.0000, 1300, 1470, 'DebugArea_VisualLineup_96', NULL, true);
-- 1471 Praxis Jaffa #97 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13931, 326.75, 6.80, -914.00, 0.0000, 1300, 1471, 'DebugArea_VisualLineup_97', NULL, true);
-- 1472 Praxis Jaffa 2 #98 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13932, 329.00, 6.80, -914.00, 0.0000, 1300, 1472, 'DebugArea_VisualLineup_98', NULL, true);
-- 1473 Ra Jaffa #99 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13933, 331.25, 6.58, -914.00, 0.0000, 1300, 1473, 'DebugArea_VisualLineup_99', NULL, true);
-- 1474 Standard Jaffa #100 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13934, 333.50, 6.58, -914.00, 0.0000, 1300, 1474, 'DebugArea_VisualLineup_100', NULL, true);
-- 1475 Savarog Jaffa #101 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13935, 335.75, 6.58, -914.00, 0.0000, 1300, 1475, 'DebugArea_VisualLineup_101', NULL, true);
-- 1476 Tiki Jaffa #102 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13936, 306.00, 6.58, -937.00, 3.1416, 1300, 1476, 'DebugArea_VisualLineup_102', NULL, true);
-- 1477 Unas_1 #105 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13937, 308.25, 6.58, -937.00, 3.1416, 1300, 1477, 'DebugArea_VisualLineup_105', NULL, true);
-- 1478 Unas_2 #106 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13938, 310.50, 6.58, -937.00, 3.1416, 1300, 1478, 'DebugArea_VisualLineup_106', NULL, true);
-- 1479 Unas_3 #107 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13939, 312.75, 6.58, -937.00, 3.1416, 1300, 1479, 'DebugArea_VisualLineup_107', NULL, true);
-- 1480 Unas_4 #108 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13940, 315.00, 6.80, -937.00, 3.1416, 1300, 1480, 'DebugArea_VisualLineup_108', NULL, true);
-- 1481 Unas_5 #109 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13941, 317.25, 6.70, -937.00, 3.1416, 1300, 1481, 'DebugArea_VisualLineup_109', NULL, true);
-- 1482 Unas_6 #110 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13942, 319.50, 6.58, -937.00, 3.1416, 1300, 1482, 'DebugArea_VisualLineup_110', NULL, true);
-- 1483 Viking Jaffa #111 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13943, 321.75, 6.58, -937.00, 3.1416, 1300, 1483, 'DebugArea_VisualLineup_111', NULL, true);
-- 1484 Ra Jaffa 2 #142 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13944, 324.00, 6.58, -937.00, 3.1416, 1300, 1484, 'DebugArea_VisualLineup_142', NULL, true);
-- 1485 Ra's Officer #143 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13945, 326.25, 6.58, -937.00, 3.1416, 1300, 1485, 'DebugArea_VisualLineup_143', NULL, true);
-- 1486 JaffaMale Template - D #155 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13946, 328.50, 6.80, -937.00, 3.1416, 1300, 1486, 'DebugArea_VisualLineup_155', NULL, true);
-- 1487 Praxis Jaffa Guard #160 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13947, 330.75, 6.58, -937.00, 3.1416, 1300, 1487, 'DebugArea_VisualLineup_160', NULL, true);
-- 1488 Petbe #163 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13948, 333.00, 6.58, -937.00, 3.1416, 1300, 1488, 'DebugArea_VisualLineup_163', NULL, true);
-- 1489 Mala'c #200 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13949, 335.25, 6.58, -937.00, 3.1416, 1300, 1489, 'DebugArea_VisualLineup_200', NULL, true);
-- 1490 Bra'hin #202 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13950, 337.50, 6.58, -937.00, 3.1416, 1300, 1490, 'DebugArea_VisualLineup_202', NULL, true);
-- 1491 Ra's Jaffa #203 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13951, 316.00, 6.58, -942.00, 0.0000, 1300, 1491, 'DebugArea_VisualLineup_203', NULL, true);
-- 1492 Angry Jaffa #206 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13952, 318.25, 6.58, -942.00, 0.0000, 1300, 1492, 'DebugArea_VisualLineup_206', NULL, true);
-- 1493 Petbe #221 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13953, 320.50, 6.58, -942.00, 0.0000, 1300, 1493, 'DebugArea_VisualLineup_221', NULL, true);
-- 1494 Jaffa #1310 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13954, 322.75, 6.58, -942.00, 0.0000, 1300, 1494, 'DebugArea_VisualLineup_1310', NULL, true);
-- 1495 Praxis Jaffa Guard #1371 BS_JaffaMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13955, 325.00, 6.58, -942.00, 0.0000, 1300, 1495, 'DebugArea_VisualLineup_1371', NULL, true);
-- 1496 (no template) BS_RaJaff
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13956, 327.25, 6.58, -942.00, 0.0000, 1300, 1496, 'DebugArea_VisualLineup_NoTemplate_BS_RaJaff', NULL, true);
-- Jaffa, female
-- 1497 Moh'Katan #54 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13957, 329.50, 6.58, -942.00, 0.0000, 1300, 1497, 'DebugArea_VisualLineup_54', NULL, true);
-- 1498 Asian Jaffa Female #112 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13958, 331.75, 6.58, -942.00, 0.0000, 1300, 1498, 'DebugArea_VisualLineup_112', NULL, true);
-- 1499 Bull Jaffa Female #113 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13959, 334.00, 6.58, -942.00, 0.0000, 1300, 1499, 'DebugArea_VisualLineup_113', NULL, true);
-- 1500 Cat Jaffa Female #114 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13960, 336.25, 6.58, -942.00, 0.0000, 1300, 1500, 'DebugArea_VisualLineup_114', NULL, true);
-- 1501 Cobra Jaffa Female #115 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13961, 338.50, 6.58, -942.00, 0.0000, 1300, 1501, 'DebugArea_VisualLineup_115', NULL, true);
-- 1502 Croc Jaffa Female #116 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13962, 329.00, 6.58, -947.00, 3.1416, 1300, 1502, 'DebugArea_VisualLineup_116', NULL, true);
-- 1503 Demon Jaffa Female #117 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13963, 331.25, 6.58, -947.00, 3.1416, 1300, 1503, 'DebugArea_VisualLineup_117', NULL, true);
-- 1504 Dragon Jaffa Female #118 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13964, 333.50, 6.58, -947.00, 3.1416, 1300, 1504, 'DebugArea_VisualLineup_118', NULL, true);
-- 1505 Eagle Jaffa Female #119 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13965, 335.75, 6.58, -947.00, 3.1416, 1300, 1505, 'DebugArea_VisualLineup_119', NULL, true);
-- 1506 Falcon Jaffa Female #120 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13966, 338.00, 6.58, -947.00, 3.1416, 1300, 1506, 'DebugArea_VisualLineup_120', NULL, true);
-- 1507 Horse Jaffa Female #121 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13967, 340.25, 6.58, -947.00, 3.1416, 1300, 1507, 'DebugArea_VisualLineup_121', NULL, true);
-- 1508 Hyena Jaffa Female #122 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13968, 342.50, 6.58, -947.00, 3.1416, 1300, 1508, 'DebugArea_VisualLineup_122', NULL, true);
-- 1509 Jackal Jaffa Female #123 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13969, 344.75, 6.58, -947.00, 3.1416, 1300, 1509, 'DebugArea_VisualLineup_123', NULL, true);
-- 1510 Mayan Jaffa Female #124 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13970, 347.00, 6.58, -947.00, 3.1416, 1300, 1510, 'DebugArea_VisualLineup_124', NULL, true);
-- 1511 Morrigan Jaffa Femal #125 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13971, 349.25, 6.58, -947.00, 3.1416, 1300, 1511, 'DebugArea_VisualLineup_125', NULL, true);
-- 1512 Naga Jaffa Female #126 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13972, 351.50, 6.58, -947.00, 3.1416, 1300, 1512, 'DebugArea_VisualLineup_126', NULL, true);
-- 1513 Praxis Jaffa 2 Femal #127 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13973, 315.00, 6.58, -952.00, 0.0000, 1300, 1513, 'DebugArea_VisualLineup_127', NULL, true);
-- 1514 Praxis Jaffa 1 Femal #128 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13974, 317.25, 6.58, -952.00, 0.0000, 1300, 1514, 'DebugArea_VisualLineup_128', NULL, true);
-- 1515 Standard Jaffa Femal #129 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13975, 319.50, 6.58, -952.00, 0.0000, 1300, 1515, 'DebugArea_VisualLineup_129', NULL, true);
-- 1516 Svarog Jaffa Female #130 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13976, 321.75, 6.58, -952.00, 0.0000, 1300, 1516, 'DebugArea_VisualLineup_130', NULL, true);
-- 1517 Tiki Jaffa Female #131 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13977, 324.00, 6.58, -952.00, 0.0000, 1300, 1517, 'DebugArea_VisualLineup_131', NULL, true);
-- 1518 Unas 1 Female #132 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13978, 326.25, 6.58, -952.00, 0.0000, 1300, 1518, 'DebugArea_VisualLineup_132', NULL, true);
-- 1519 Unas 2 Female #133 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13979, 328.50, 6.58, -952.00, 0.0000, 1300, 1519, 'DebugArea_VisualLineup_133', NULL, true);
-- 1520 Unas 3 Female #134 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13980, 330.75, 6.58, -952.00, 0.0000, 1300, 1520, 'DebugArea_VisualLineup_134', NULL, true);
-- 1521 Unas 4 Female #135 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13981, 333.00, 6.58, -952.00, 0.0000, 1300, 1521, 'DebugArea_VisualLineup_135', NULL, true);
-- 1522 Unas 5 Female #136 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13982, 335.25, 6.58, -952.00, 0.0000, 1300, 1522, 'DebugArea_VisualLineup_136', NULL, true);
-- 1523 Unas 6 Female #137 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13983, 307.00, 6.58, -956.00, 3.1416, 1300, 1523, 'DebugArea_VisualLineup_137', NULL, true);
-- 1524 Viking Jaffa Female #138 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13984, 309.25, 6.58, -956.00, 3.1416, 1300, 1524, 'DebugArea_VisualLineup_138', NULL, true);
-- 1525 Clothed Jaffa Female #139 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13985, 311.50, 6.58, -956.00, 3.1416, 1300, 1525, 'DebugArea_VisualLineup_139', NULL, true);
-- 1526 JaffaFemale Template #154 BS_JaffaFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13986, 313.75, 6.58, -956.00, 3.1416, 1300, 1526, 'DebugArea_VisualLineup_154', NULL, true);
-- Goa'uld, male
-- 1527 Prisoner 329 #17 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13987, 316.00, 6.58, -956.00, 3.1416, 1300, 1527, 'DebugArea_VisualLineup_17', NULL, true);
-- 1528 Ra #41 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13988, 318.25, 6.58, -956.00, 3.1416, 1300, 1528, 'DebugArea_VisualLineup_41', NULL, true);
-- 1529 Ba'al #42 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13989, 320.50, 6.58, -956.00, 3.1416, 1300, 1529, 'DebugArea_VisualLineup_42', NULL, true);
-- 1530 Ra #60 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13990, 327.00, 6.58, -956.00, 3.1416, 1300, 1530, 'DebugArea_VisualLineup_60', NULL, true);
-- 1531 Ra #61 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13991, 329.25, 6.58, -956.00, 3.1416, 1300, 1531, 'DebugArea_VisualLineup_61', NULL, true);
-- 1532 Ra #62 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13992, 331.50, 6.58, -956.00, 3.1416, 1300, 1532, 'DebugArea_VisualLineup_62', NULL, true);
-- 1533 Ra #63 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13993, 333.75, 6.58, -956.00, 3.1416, 1300, 1533, 'DebugArea_VisualLineup_63', NULL, true);
-- 1534 GoauldMale Template #158 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13994, 336.00, 6.58, -956.00, 3.1416, 1300, 1534, 'DebugArea_VisualLineup_158', NULL, true);
-- 1535 Ba'al #167 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13995, 338.25, 6.58, -956.00, 3.1416, 1300, 1535, 'DebugArea_VisualLineup_167', NULL, true);
-- 1536 Haughty Goa'uld #210 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13996, 340.50, 6.58, -956.00, 3.1416, 1300, 1536, 'DebugArea_VisualLineup_210', NULL, true);
-- 1537 Ashrak Assassin #211 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13997, 326.00, 6.58, -961.00, 0.0000, 1300, 1537, 'DebugArea_VisualLineup_211', NULL, true);
-- 1538 Lo'Taur Servant #353 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13998, 328.25, 6.58, -961.00, 0.0000, 1300, 1538, 'DebugArea_VisualLineup_353', NULL, true);
-- 1539 Ra #1400 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13999, 330.50, 6.58, -961.00, 0.0000, 1300, 1539, 'DebugArea_VisualLineup_1400', NULL, true);
-- 1540 Ba'al #1401 BS_GoauldMale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14000, 332.75, 6.58, -961.00, 0.0000, 1300, 1540, 'DebugArea_VisualLineup_1401', NULL, true);
-- Goa'uld, female
-- 1541 Anat #43 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14001, 335.00, 6.58, -961.00, 0.0000, 1300, 1541, 'DebugArea_VisualLineup_43', NULL, true);
-- 1542 Athena #44 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14002, 337.25, 6.58, -961.00, 0.0000, 1300, 1542, 'DebugArea_VisualLineup_44', NULL, true);
-- 1543 Morrigan #45 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14003, 339.50, 6.58, -961.00, 0.0000, 1300, 1543, 'DebugArea_VisualLineup_45', NULL, true);
-- 1544 GoauldFemale Templa #157 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14004, 341.75, 6.58, -961.00, 0.0000, 1300, 1544, 'DebugArea_VisualLineup_157', NULL, true);
-- 1545 Athena #1403 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14005, 344.00, 6.58, -961.00, 0.0000, 1300, 1545, 'DebugArea_VisualLineup_1403', NULL, true);
-- 1546 Morrigan #1404 BS_GoauldFemale
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14006, 346.25, 6.58, -961.00, 0.0000, 1300, 1546, 'DebugArea_VisualLineup_1404', NULL, true);
-- Asgard
-- 1547 Thor #64 BS_Asgard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14007, 348.50, 6.58, -961.00, 0.0000, 1300, 1547, 'DebugArea_VisualLineup_64', NULL, true);
-- 1548 Asgard Template - DO NOT #156 BS_Asgard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14008, 350.75, 6.58, -961.00, 0.0000, 1300, 1548, 'DebugArea_VisualLineup_156', NULL, true);
-- 1549 (no template) BS_Degenerated_Asgard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14009, 353.00, 6.58, -961.00, 0.0000, 1300, 1549, 'DebugArea_VisualLineup_NoTemplate_BS_Degenerated_Asgard', NULL, true);
-- Children
-- 1550 Nox Child Male #66 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14010, 355.25, 6.58, -961.00, 0.0000, 1300, 1550, 'DebugArea_VisualLineup_66', NULL, true);
-- 1551 Nox Child Female #67 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14011, 357.50, 6.58, -961.00, 0.0000, 1300, 1551, 'DebugArea_VisualLineup_67', NULL, true);
-- 1552 Blix #68 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14012, 359.75, 6.58, -961.00, 0.0000, 1300, 1552, 'DebugArea_VisualLineup_68', NULL, true);
-- 1553 NPC Child 1 #140 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14013, 362.00, 6.58, -961.00, 0.0000, 1300, 1553, 'DebugArea_VisualLineup_140', NULL, true);
-- 1554 NPC Child 2 #141 NPC_Child_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14014, 364.25, 6.58, -961.00, 0.0000, 1300, 1554, 'DebugArea_VisualLineup_141', NULL, true);
-- Creatures
-- 1555 Rat #74 BS_MOB_Rat
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14015, 348.00, 6.58, -909.00, 3.1416, 1300, 1555, 'DebugArea_VisualLineup_74', NULL, true);
-- 1556 ScavDog #76 BS_MOB_ScavDog
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14016, 352.00, 6.58, -909.00, 3.1416, 1300, 1556, 'DebugArea_VisualLineup_76', NULL, true);
-- 1557 Lenny #73 BS_MOB_Lenny
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14017, 356.00, 6.58, -909.00, 3.1416, 1300, 1557, 'DebugArea_VisualLineup_73', NULL, true);
-- 1558 (no template) BS_MOB_LennyBaby
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14018, 360.00, 6.58, -909.00, 3.1416, 1300, 1558, 'DebugArea_VisualLineup_NoTemplate_BS_MOB_LennyBaby', NULL, true);
-- 1559 Horden #72 BS_MOB_Horden
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14019, 366.00, 6.58, -909.00, 3.1416, 1300, 1559, 'DebugArea_VisualLineup_72', NULL, true);
-- 1560 Carnosaur #71 BS_MOB_Carnosaur
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14020, 374.00, 6.58, -909.00, 3.1416, 1300, 1560, 'DebugArea_VisualLineup_71', NULL, true);
-- 1561 Rhinolion #75 BS_MOB_Rhinolion00
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14021, 382.00, 6.58, -909.00, 3.1416, 1300, 1561, 'DebugArea_VisualLineup_75', NULL, true);
-- 1562 Twilla Vines #80 BS_MOB_TwillaTree
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14022, 362.00, 6.58, -937.00, 3.1416, 1300, 1562, 'DebugArea_VisualLineup_80', NULL, true);
-- Machines
-- 1563 (no template) BS_AN_Android
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14023, 347.00, 6.58, -942.00, 0.0000, 1300, 1563, 'DebugArea_VisualLineup_NoTemplate_BS_AN_Android', NULL, true);
-- 1564 (no template) BS_MOB_DroneTank
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14024, 351.00, 6.58, -942.00, 0.0000, 1300, 1564, 'DebugArea_VisualLineup_NoTemplate_BS_MOB_DroneTank', NULL, true);
-- 1565 Prisoner retrieval #4 BS_MOB_DroneFlyer
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14025, 355.00, 6.58, -942.00, 0.0000, 1300, 1565, 'DebugArea_VisualLineup_4', NULL, true);
-- 1566 Malfunctioning Dron #69 MOB_Goauld_Drone
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14026, 359.00, 6.58, -942.00, 0.0000, 1300, 1566, 'DebugArea_VisualLineup_69', NULL, true);
-- 1567 Agnos Drone #81 MOB_AncientDrone_BS
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14027, 368.00, 6.58, -937.00, 3.1416, 1300, 1567, 'DebugArea_VisualLineup_81', NULL, true);
-- 1568 Straegis Figh #78 BS_MOB_StraegisFighter
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14028, 372.00, 6.58, -937.00, 3.1416, 1300, 1568, 'DebugArea_VisualLineup_78', NULL, true);
-- 1569 Straegis Beaco #77 BS_MOB_StraegisBeacon
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14029, 376.00, 6.58, -937.00, 3.1416, 1300, 1569, 'DebugArea_VisualLineup_77', NULL, true);
-- 1570 BattleWalker #70 BS_MOB_BattleWalker
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14030, 382.00, 6.58, -937.00, 3.1416, 1300, 1570, 'DebugArea_VisualLineup_70', NULL, true);
-- 1571 Straegis Titan #79 BS_MOB_StraegisTitan
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14031, 368.50, 6.58, -922.00, 0.0000, 1300, 1571, 'DebugArea_VisualLineup_79', NULL, true);

-- Keep default-id inserts (`.savespawn`) past DA-10's reserved block
-- 13870-14099.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 14099), true);
