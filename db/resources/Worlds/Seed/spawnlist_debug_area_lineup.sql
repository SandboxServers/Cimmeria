--
-- NEW CONTENT (Debug Area, DA-10): the NPC lineup, world 1300.
-- docs/content/debug-area.md#npc-lineup has the rows and the walk.
--
-- Id block 13870-14099 (DA-10). Tags `DebugArea_Lineup_<source template id>`
-- (`DebugArea_Lineup_<body set>` for the six template-less body sets; D-DA6).
--
-- The east wing of the south compound: an open-air walled ruin (x 297-392,
-- z -969 to -881) east of the courtyard, entered through the doorway at its
-- north-west corner (about (300, 6.6, -897)), 80 m east of the Compound ring
-- pad (224, 7.44, -938). Every spot is on the navmesh and on the occluder's
-- terrain top (y 6.56-6.80), found by a 1 m grid search of
-- ihpet_crater_light.nav and .occ with 1 m clear on each side.
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
-- Reach it with `.gotolocation DebugArea 300 6.6 -897`.
--
-- Humans, male
-- 1410 <- source 10 Col Marsh (pet)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13870, 305.00, 6.58, -888.00, 3.1416, 1300, 1410, 'DebugArea_Lineup_10', NULL, true);
-- 1411 <- source 15 Cellblock Guard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13871, 307.25, 6.58, -888.00, 3.1416, 1300, 1411, 'DebugArea_Lineup_15', NULL, true);
-- 1412 <- source 24 NID Guard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13872, 309.50, 6.58, -888.00, 3.1416, 1300, 1412, 'DebugArea_Lineup_24', NULL, true);
-- 1413 <- source 25 Interaction Debug NPC - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13873, 311.75, 6.58, -888.00, 3.1416, 1300, 1413, 'DebugArea_Lineup_25', NULL, true);
-- 1414 <- source 26 TestAvatar
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13874, 314.00, 6.58, -888.00, 3.1416, 1300, 1414, 'DebugArea_Lineup_26', NULL, true);
-- 1415 <- source 28 test avatar set - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13875, 316.25, 6.58, -888.00, 3.1416, 1300, 1415, 'DebugArea_Lineup_28', NULL, true);
-- 1416 <- source 29 General Hammond
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13876, 318.50, 6.58, -888.00, 3.1416, 1300, 1416, 'DebugArea_Lineup_29', NULL, true);
-- 1417 <- source 31 Airman
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13877, 323.00, 6.58, -893.00, 3.1416, 1300, 1417, 'DebugArea_Lineup_31', NULL, true);
-- 1418 <- source 47 Mr Woolsey
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13878, 325.25, 6.58, -893.00, 3.1416, 1300, 1418, 'DebugArea_Lineup_47', NULL, true);
-- 1419 <- source 51 Daniel Jackson
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13879, 327.50, 6.58, -893.00, 3.1416, 1300, 1419, 'DebugArea_Lineup_51', NULL, true);
-- 1420 <- source 52 Jack O'Neill
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13880, 329.75, 6.80, -893.00, 3.1416, 1300, 1420, 'DebugArea_Lineup_52', NULL, true);
-- 1421 <- source 53 Nerus
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13881, 332.00, 6.58, -893.00, 3.1416, 1300, 1421, 'DebugArea_Lineup_53', NULL, true);
-- 1422 <- source 55 Warrick
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13882, 334.25, 6.58, -893.00, 3.1416, 1300, 1422, 'DebugArea_Lineup_55', NULL, true);
-- 1423 <- source 56 Goldam
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13883, 336.50, 6.58, -893.00, 3.1416, 1300, 1423, 'DebugArea_Lineup_56', NULL, true);
-- 1424 <- source 57 Major Davis
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13884, 338.75, 6.58, -893.00, 3.1416, 1300, 1424, 'DebugArea_Lineup_57', NULL, true);
-- 1425 <- source 58 Walter Harriman
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13885, 341.00, 6.58, -893.00, 3.1416, 1300, 1425, 'DebugArea_Lineup_58', NULL, true);
-- 1426 <- source 146 NID Guard - Castle outside
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13886, 343.25, 6.58, -893.00, 3.1416, 1300, 1426, 'DebugArea_Lineup_146', NULL, true);
-- 1427 <- source 149 Sgt. Gerschon
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13887, 345.50, 6.80, -893.00, 3.1416, 1300, 1427, 'DebugArea_Lineup_149', NULL, true);
-- 1428 <- source 150 HumanMale - Not For Use
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13888, 306.00, 6.58, -902.00, 0.0000, 1300, 1428, 'DebugArea_Lineup_150', NULL, true);
-- 1429 <- source 151 Lucian - Blue Faction Scientist
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13889, 308.25, 6.58, -902.00, 0.0000, 1300, 1429, 'DebugArea_Lineup_151', NULL, true);
-- 1430 <- source 152 Lucian - Slum Dweller
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13890, 310.50, 6.58, -902.00, 0.0000, 1300, 1430, 'DebugArea_Lineup_152', NULL, true);
-- 1431 <- source 168 Castle_Zuritska
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13891, 312.75, 6.58, -902.00, 0.0000, 1300, 1431, 'DebugArea_Lineup_168', NULL, true);
-- 1432 <- source 172 Castle_SurrenderGuard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13892, 315.00, 6.58, -902.00, 0.0000, 1300, 1432, 'DebugArea_Lineup_172', NULL, true);
-- 1433 <- source 174 Castle_OpCoreSoldier
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13893, 317.25, 6.58, -902.00, 0.0000, 1300, 1433, 'DebugArea_Lineup_174', NULL, true);
-- 1434 <- source 175 Castle_OpCoreSoldier_Wander
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13894, 319.50, 6.58, -902.00, 0.0000, 1300, 1434, 'DebugArea_Lineup_175', NULL, true);
-- 1435 <- source 176 Castle_OpCoreSoldier_Unarmed
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13895, 321.75, 6.58, -902.00, 0.0000, 1300, 1435, 'DebugArea_Lineup_176', NULL, true);
-- 1436 <- source 178 Castle_SgtStanton
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13896, 324.00, 6.58, -902.00, 0.0000, 1300, 1436, 'DebugArea_Lineup_178', NULL, true);
-- 1437 <- source 179 Castle_Ogilvie
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13897, 326.25, 6.58, -902.00, 0.0000, 1300, 1437, 'DebugArea_Lineup_179', NULL, true);
-- 1438 <- source 215 Opheltes
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13898, 328.50, 6.58, -902.00, 0.0000, 1300, 1438, 'DebugArea_Lineup_215', NULL, true);
-- 1439 <- source 300 Debug Hub - Vendor
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13899, 330.75, 6.58, -902.00, 0.0000, 1300, 1439, 'DebugArea_Lineup_300', NULL, true);
-- 1440 <- source 302 Debug Hub - Dialog
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13900, 333.00, 6.58, -902.00, 0.0000, 1300, 1440, 'DebugArea_Lineup_302', NULL, true);
-- 1441 <- source 314 Debug Hub - Crafting Supplies
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13901, 335.25, 6.56, -902.00, 0.0000, 1300, 1441, 'DebugArea_Lineup_314', NULL, true);
-- 1442 <- HM_Mesh.HM_BodySet: no template uses it, dressed from body_components
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13902, 337.50, 6.58, -902.00, 0.0000, 1300, 1442, 'DebugArea_Lineup_HM_BodySet', NULL, true);
-- Humans, female
-- 1443 <- source 33 Sam Carter
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13903, 339.75, 6.58, -902.00, 0.0000, 1300, 1443, 'DebugArea_Lineup_33', NULL, true);
-- 1444 <- source 48 CaptCoppleman
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13904, 342.00, 6.58, -902.00, 0.0000, 1300, 1444, 'DebugArea_Lineup_48', NULL, true);
-- 1445 <- source 49 Oma Desala
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13905, 344.25, 6.58, -902.00, 0.0000, 1300, 1445, 'DebugArea_Lineup_49', NULL, true);
-- 1446 <- source 50 Vala Mal Doran
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13906, 346.50, 6.58, -902.00, 0.0000, 1300, 1446, 'DebugArea_Lineup_50', NULL, true);
-- 1447 <- source 153 HumanFemale Template - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13907, 348.75, 6.58, -902.00, 0.0000, 1300, 1447, 'DebugArea_Lineup_153', NULL, true);
-- 1448 <- source 170 Castle_Muelbach
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13908, 351.00, 6.58, -902.00, 0.0000, 1300, 1448, 'DebugArea_Lineup_170', NULL, true);
-- 1449 <- source 177 Castle_Medic
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13909, 353.25, 6.58, -902.00, 0.0000, 1300, 1449, 'DebugArea_Lineup_177', NULL, true);
-- 1450 <- source 219 Storage Lo'taur
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13910, 312.00, 6.58, -909.00, 3.1416, 1300, 1450, 'DebugArea_Lineup_219', NULL, true);
-- 1451 <- source 370 Debug Hub - Banker
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13911, 314.25, 6.58, -909.00, 3.1416, 1300, 1451, 'DebugArea_Lineup_370', NULL, true);
-- Jaffa, male
-- 1452 <- source 30 Teal'c
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13912, 316.50, 6.58, -909.00, 3.1416, 1300, 1452, 'DebugArea_Lineup_30', NULL, true);
-- 1453 <- source 34 SGC Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13913, 318.75, 6.58, -909.00, 3.1416, 1300, 1453, 'DebugArea_Lineup_34', NULL, true);
-- 1454 <- source 59 Bra'tak
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13914, 321.00, 6.58, -909.00, 3.1416, 1300, 1454, 'DebugArea_Lineup_59', NULL, true);
-- 1455 <- source 82 Bull Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13915, 323.25, 6.58, -909.00, 3.1416, 1300, 1455, 'DebugArea_Lineup_82', NULL, true);
-- 1456 <- source 83 Asian Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13916, 325.50, 6.58, -909.00, 3.1416, 1300, 1456, 'DebugArea_Lineup_83', NULL, true);
-- 1457 <- source 84 Cat Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13917, 327.75, 6.58, -909.00, 3.1416, 1300, 1457, 'DebugArea_Lineup_84', NULL, true);
-- 1458 <- source 85 Cobra Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13918, 330.00, 6.58, -909.00, 3.1416, 1300, 1458, 'DebugArea_Lineup_85', NULL, true);
-- 1459 <- source 86 Croc Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13919, 332.25, 6.58, -909.00, 3.1416, 1300, 1459, 'DebugArea_Lineup_86', NULL, true);
-- 1460 <- source 87 Demon Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13920, 302.00, 6.58, -914.00, 0.0000, 1300, 1460, 'DebugArea_Lineup_87', NULL, true);
-- 1461 <- source 88 Dragon Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13921, 304.25, 6.58, -914.00, 0.0000, 1300, 1461, 'DebugArea_Lineup_88', NULL, true);
-- 1462 <- source 89 Eagle Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13922, 306.50, 6.58, -914.00, 0.0000, 1300, 1462, 'DebugArea_Lineup_89', NULL, true);
-- 1463 <- source 90 Falcon Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13923, 308.75, 6.58, -914.00, 0.0000, 1300, 1463, 'DebugArea_Lineup_90', NULL, true);
-- 1464 <- source 91 Horse Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13924, 311.00, 6.58, -914.00, 0.0000, 1300, 1464, 'DebugArea_Lineup_91', NULL, true);
-- 1465 <- source 92 Hyena Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13925, 313.25, 6.58, -914.00, 0.0000, 1300, 1465, 'DebugArea_Lineup_92', NULL, true);
-- 1466 <- source 93 Jackal Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13926, 315.50, 6.80, -914.00, 0.0000, 1300, 1466, 'DebugArea_Lineup_93', NULL, true);
-- 1467 <- source 94 Mayan Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13927, 317.75, 6.80, -914.00, 0.0000, 1300, 1467, 'DebugArea_Lineup_94', NULL, true);
-- 1468 <- source 95 Morrigan Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13928, 320.00, 6.58, -914.00, 0.0000, 1300, 1468, 'DebugArea_Lineup_95', NULL, true);
-- 1469 <- source 96 Naga Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13929, 322.25, 6.58, -914.00, 0.0000, 1300, 1469, 'DebugArea_Lineup_96', NULL, true);
-- 1470 <- source 97 Praxis Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13930, 324.50, 6.58, -914.00, 0.0000, 1300, 1470, 'DebugArea_Lineup_97', NULL, true);
-- 1471 <- source 98 Praxis Jaffa 2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13931, 326.75, 6.80, -914.00, 0.0000, 1300, 1471, 'DebugArea_Lineup_98', NULL, true);
-- 1472 <- source 99 Ra Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13932, 329.00, 6.80, -914.00, 0.0000, 1300, 1472, 'DebugArea_Lineup_99', NULL, true);
-- 1473 <- source 100 Standard Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13933, 331.25, 6.58, -914.00, 0.0000, 1300, 1473, 'DebugArea_Lineup_100', NULL, true);
-- 1474 <- source 101 Savarog Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13934, 333.50, 6.58, -914.00, 0.0000, 1300, 1474, 'DebugArea_Lineup_101', NULL, true);
-- 1475 <- source 102 Tiki Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13935, 335.75, 6.58, -914.00, 0.0000, 1300, 1475, 'DebugArea_Lineup_102', NULL, true);
-- 1476 <- source 105 Unas_1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13936, 306.00, 6.58, -937.00, 3.1416, 1300, 1476, 'DebugArea_Lineup_105', NULL, true);
-- 1477 <- source 106 Unas_2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13937, 308.25, 6.58, -937.00, 3.1416, 1300, 1477, 'DebugArea_Lineup_106', NULL, true);
-- 1478 <- source 107 Unas_3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13938, 310.50, 6.58, -937.00, 3.1416, 1300, 1478, 'DebugArea_Lineup_107', NULL, true);
-- 1479 <- source 108 Unas_4
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13939, 312.75, 6.58, -937.00, 3.1416, 1300, 1479, 'DebugArea_Lineup_108', NULL, true);
-- 1480 <- source 109 Unas_5
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13940, 315.00, 6.80, -937.00, 3.1416, 1300, 1480, 'DebugArea_Lineup_109', NULL, true);
-- 1481 <- source 110 Unas_6
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13941, 317.25, 6.70, -937.00, 3.1416, 1300, 1481, 'DebugArea_Lineup_110', NULL, true);
-- 1482 <- source 111 Viking Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13942, 319.50, 6.58, -937.00, 3.1416, 1300, 1482, 'DebugArea_Lineup_111', NULL, true);
-- 1483 <- source 142 Ra Jaffa 2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13943, 321.75, 6.58, -937.00, 3.1416, 1300, 1483, 'DebugArea_Lineup_142', NULL, true);
-- 1484 <- source 143 Ra's Officer
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13944, 324.00, 6.58, -937.00, 3.1416, 1300, 1484, 'DebugArea_Lineup_143', NULL, true);
-- 1485 <- source 155 JaffaMale Template - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13945, 326.25, 6.58, -937.00, 3.1416, 1300, 1485, 'DebugArea_Lineup_155', NULL, true);
-- 1486 <- source 160 Praxis Jaffa Guard
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13946, 328.50, 6.80, -937.00, 3.1416, 1300, 1486, 'DebugArea_Lineup_160', NULL, true);
-- 1487 <- source 163 Petbe
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13947, 330.75, 6.58, -937.00, 3.1416, 1300, 1487, 'DebugArea_Lineup_163', NULL, true);
-- 1488 <- source 200 Mala'c
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13948, 333.00, 6.58, -937.00, 3.1416, 1300, 1488, 'DebugArea_Lineup_200', NULL, true);
-- 1489 <- source 202 Bra'hin
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13949, 335.25, 6.58, -937.00, 3.1416, 1300, 1489, 'DebugArea_Lineup_202', NULL, true);
-- 1490 <- source 203 Ra's Jaffa Infiltrator
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13950, 337.50, 6.58, -937.00, 3.1416, 1300, 1490, 'DebugArea_Lineup_203', NULL, true);
-- 1491 <- source 206 Angry Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13951, 316.00, 6.58, -942.00, 0.0000, 1300, 1491, 'DebugArea_Lineup_206', NULL, true);
-- 1492 <- source 221 Petbe (hostile)
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13952, 318.25, 6.58, -942.00, 0.0000, 1300, 1492, 'DebugArea_Lineup_221', NULL, true);
-- 1493 <- source 1310 Debug Area - Training Dummy L1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13953, 320.50, 6.58, -942.00, 0.0000, 1300, 1493, 'DebugArea_Lineup_1310', NULL, true);
-- 1494 <- source 1371 DebugArea Arena Praxis Jaffa
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13954, 322.75, 6.58, -942.00, 0.0000, 1300, 1494, 'DebugArea_Lineup_1371', NULL, true);
-- 1495 <- AR_J_Ra.BS_RaJaff: no template uses it, dressed from body_components
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13955, 325.00, 6.58, -942.00, 0.0000, 1300, 1495, 'DebugArea_Lineup_BS_RaJaff', NULL, true);
-- Jaffa, female
-- 1496 <- source 54 Moh'Katan
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13956, 327.25, 6.58, -942.00, 0.0000, 1300, 1496, 'DebugArea_Lineup_54', NULL, true);
-- 1497 <- source 112 Asian Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13957, 329.50, 6.58, -942.00, 0.0000, 1300, 1497, 'DebugArea_Lineup_112', NULL, true);
-- 1498 <- source 113 Bull Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13958, 331.75, 6.58, -942.00, 0.0000, 1300, 1498, 'DebugArea_Lineup_113', NULL, true);
-- 1499 <- source 114 Cat Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13959, 334.00, 6.58, -942.00, 0.0000, 1300, 1499, 'DebugArea_Lineup_114', NULL, true);
-- 1500 <- source 115 Cobra Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13960, 336.25, 6.58, -942.00, 0.0000, 1300, 1500, 'DebugArea_Lineup_115', NULL, true);
-- 1501 <- source 116 Croc Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13961, 338.50, 6.58, -942.00, 0.0000, 1300, 1501, 'DebugArea_Lineup_116', NULL, true);
-- 1502 <- source 117 Demon Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13962, 329.00, 6.58, -947.00, 3.1416, 1300, 1502, 'DebugArea_Lineup_117', NULL, true);
-- 1503 <- source 118 Dragon Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13963, 331.25, 6.58, -947.00, 3.1416, 1300, 1503, 'DebugArea_Lineup_118', NULL, true);
-- 1504 <- source 119 Eagle Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13964, 333.50, 6.58, -947.00, 3.1416, 1300, 1504, 'DebugArea_Lineup_119', NULL, true);
-- 1505 <- source 120 Falcon Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13965, 335.75, 6.58, -947.00, 3.1416, 1300, 1505, 'DebugArea_Lineup_120', NULL, true);
-- 1506 <- source 121 Horse Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13966, 338.00, 6.58, -947.00, 3.1416, 1300, 1506, 'DebugArea_Lineup_121', NULL, true);
-- 1507 <- source 122 Hyena Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13967, 340.25, 6.58, -947.00, 3.1416, 1300, 1507, 'DebugArea_Lineup_122', NULL, true);
-- 1508 <- source 123 Jackal Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13968, 342.50, 6.58, -947.00, 3.1416, 1300, 1508, 'DebugArea_Lineup_123', NULL, true);
-- 1509 <- source 124 Mayan Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13969, 344.75, 6.58, -947.00, 3.1416, 1300, 1509, 'DebugArea_Lineup_124', NULL, true);
-- 1510 <- source 125 Morrigan Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13970, 347.00, 6.58, -947.00, 3.1416, 1300, 1510, 'DebugArea_Lineup_125', NULL, true);
-- 1511 <- source 126 Naga Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13971, 349.25, 6.58, -947.00, 3.1416, 1300, 1511, 'DebugArea_Lineup_126', NULL, true);
-- 1512 <- source 127 Praxis Jaffa 2 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13972, 351.50, 6.58, -947.00, 3.1416, 1300, 1512, 'DebugArea_Lineup_127', NULL, true);
-- 1513 <- source 128 Praxis Jaffa 1 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13973, 315.00, 6.58, -952.00, 0.0000, 1300, 1513, 'DebugArea_Lineup_128', NULL, true);
-- 1514 <- source 129 Standard Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13974, 317.25, 6.58, -952.00, 0.0000, 1300, 1514, 'DebugArea_Lineup_129', NULL, true);
-- 1515 <- source 130 Svarog Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13975, 319.50, 6.58, -952.00, 0.0000, 1300, 1515, 'DebugArea_Lineup_130', NULL, true);
-- 1516 <- source 131 Tiki Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13976, 321.75, 6.58, -952.00, 0.0000, 1300, 1516, 'DebugArea_Lineup_131', NULL, true);
-- 1517 <- source 132 Unas 1 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13977, 324.00, 6.58, -952.00, 0.0000, 1300, 1517, 'DebugArea_Lineup_132', NULL, true);
-- 1518 <- source 133 Unas 2 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13978, 326.25, 6.58, -952.00, 0.0000, 1300, 1518, 'DebugArea_Lineup_133', NULL, true);
-- 1519 <- source 134 Unas 3 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13979, 328.50, 6.58, -952.00, 0.0000, 1300, 1519, 'DebugArea_Lineup_134', NULL, true);
-- 1520 <- source 135 Unas 4 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13980, 330.75, 6.58, -952.00, 0.0000, 1300, 1520, 'DebugArea_Lineup_135', NULL, true);
-- 1521 <- source 136 Unas 5 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13981, 333.00, 6.58, -952.00, 0.0000, 1300, 1521, 'DebugArea_Lineup_136', NULL, true);
-- 1522 <- source 137 Unas 6 Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13982, 335.25, 6.58, -952.00, 0.0000, 1300, 1522, 'DebugArea_Lineup_137', NULL, true);
-- 1523 <- source 138 Viking Jaffa Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13983, 307.00, 6.58, -956.00, 3.1416, 1300, 1523, 'DebugArea_Lineup_138', NULL, true);
-- 1524 <- source 139 Clothed Jaffa Female 1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13984, 309.25, 6.58, -956.00, 3.1416, 1300, 1524, 'DebugArea_Lineup_139', NULL, true);
-- 1525 <- source 154 JaffaFemale Template - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13985, 311.50, 6.58, -956.00, 3.1416, 1300, 1525, 'DebugArea_Lineup_154', NULL, true);
-- Goa'uld, male
-- 1526 <- source 17 Prisoner 329
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13986, 313.75, 6.58, -956.00, 3.1416, 1300, 1526, 'DebugArea_Lineup_17', NULL, true);
-- 1527 <- source 41 Ra
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13987, 316.00, 6.58, -956.00, 3.1416, 1300, 1527, 'DebugArea_Lineup_41', NULL, true);
-- 1528 <- source 42 Ba'al
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13988, 318.25, 6.58, -956.00, 3.1416, 1300, 1528, 'DebugArea_Lineup_42', NULL, true);
-- 1529 <- source 60 Ra 2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13989, 320.50, 6.58, -956.00, 3.1416, 1300, 1529, 'DebugArea_Lineup_60', NULL, true);
-- 1530 <- source 61 Ra 3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13990, 327.00, 6.58, -956.00, 3.1416, 1300, 1530, 'DebugArea_Lineup_61', NULL, true);
-- 1531 <- source 62 Ra 4
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13991, 329.25, 6.58, -956.00, 3.1416, 1300, 1531, 'DebugArea_Lineup_62', NULL, true);
-- 1532 <- source 63 Ra 5
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13992, 331.50, 6.58, -956.00, 3.1416, 1300, 1532, 'DebugArea_Lineup_63', NULL, true);
-- 1533 <- source 158 GoauldMale Template - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13993, 333.75, 6.58, -956.00, 3.1416, 1300, 1533, 'DebugArea_Lineup_158', NULL, true);
-- 1534 <- source 167 Sandbox Ba'al
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13994, 336.00, 6.58, -956.00, 3.1416, 1300, 1534, 'DebugArea_Lineup_167', NULL, true);
-- 1535 <- source 210 Haughty Goa'uld
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13995, 338.25, 6.58, -956.00, 3.1416, 1300, 1535, 'DebugArea_Lineup_210', NULL, true);
-- 1536 <- source 211 Ashrak Assassin
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13996, 340.50, 6.58, -956.00, 3.1416, 1300, 1536, 'DebugArea_Lineup_211', NULL, true);
-- 1537 <- source 353 Lo'Taur Servant
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13997, 326.00, 6.58, -961.00, 0.0000, 1300, 1537, 'DebugArea_Lineup_353', NULL, true);
-- 1538 <- source 1400 Debug Area - Ra
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13998, 328.25, 6.58, -961.00, 0.0000, 1300, 1538, 'DebugArea_Lineup_1400', NULL, true);
-- 1539 <- source 1401 Debug Area - Ba'al
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13999, 330.50, 6.58, -961.00, 0.0000, 1300, 1539, 'DebugArea_Lineup_1401', NULL, true);
-- Goa'uld, female
-- 1540 <- source 43 Anat
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14000, 332.75, 6.58, -961.00, 0.0000, 1300, 1540, 'DebugArea_Lineup_43', NULL, true);
-- 1541 <- source 44 Athena
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14001, 335.00, 6.58, -961.00, 0.0000, 1300, 1541, 'DebugArea_Lineup_44', NULL, true);
-- 1542 <- source 45 Morrigan
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14002, 337.25, 6.58, -961.00, 0.0000, 1300, 1542, 'DebugArea_Lineup_45', NULL, true);
-- 1543 <- source 157 GoauldFemale Template - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14003, 339.50, 6.58, -961.00, 0.0000, 1300, 1543, 'DebugArea_Lineup_157', NULL, true);
-- 1544 <- source 1403 Debug Area - Athena
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14004, 341.75, 6.58, -961.00, 0.0000, 1300, 1544, 'DebugArea_Lineup_1403', NULL, true);
-- 1545 <- source 1404 Debug Area - Morrigan
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14005, 344.00, 6.58, -961.00, 0.0000, 1300, 1545, 'DebugArea_Lineup_1404', NULL, true);
-- Asgard
-- 1546 <- source 64 Thor
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14006, 346.25, 6.58, -961.00, 0.0000, 1300, 1546, 'DebugArea_Lineup_64', NULL, true);
-- 1547 <- source 156 Asgard Template - DO NOT USE
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14007, 348.50, 6.58, -961.00, 0.0000, 1300, 1547, 'DebugArea_Lineup_156', NULL, true);
-- 1548 <- NPC_Asgard.BS_Degenerated_Asgard: no template uses it, dressed from body_components
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14008, 350.75, 6.58, -961.00, 0.0000, 1300, 1548, 'DebugArea_Lineup_BS_Degenerated_Asgard', NULL, true);
-- Children
-- 1549 <- source 66 Nox Child Male
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14009, 353.00, 6.58, -961.00, 0.0000, 1300, 1549, 'DebugArea_Lineup_66', NULL, true);
-- 1550 <- source 67 Nox Child Female
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14010, 355.25, 6.58, -961.00, 0.0000, 1300, 1550, 'DebugArea_Lineup_67', NULL, true);
-- 1551 <- source 68 Blix
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14011, 357.50, 6.58, -961.00, 0.0000, 1300, 1551, 'DebugArea_Lineup_68', NULL, true);
-- 1552 <- source 140 NPC Child 1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14012, 359.75, 6.58, -961.00, 0.0000, 1300, 1552, 'DebugArea_Lineup_140', NULL, true);
-- 1553 <- source 141 NPC Child 2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14013, 362.00, 6.58, -961.00, 0.0000, 1300, 1553, 'DebugArea_Lineup_141', NULL, true);
-- Creatures
-- 1554 <- source 74 AMBRat
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14014, 348.00, 6.58, -909.00, 3.1416, 1300, 1554, 'DebugArea_Lineup_74', NULL, true);
-- 1555 <- source 76 ScavDog
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14015, 352.00, 6.58, -909.00, 3.1416, 1300, 1555, 'DebugArea_Lineup_76', NULL, true);
-- 1556 <- source 73 Lenny
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14016, 356.00, 6.58, -909.00, 3.1416, 1300, 1556, 'DebugArea_Lineup_73', NULL, true);
-- 1557 <- MOB_Lenny.BS_MOB_LennyBaby: no template uses it, dressed from body_components
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14017, 360.00, 6.58, -909.00, 3.1416, 1300, 1557, 'DebugArea_Lineup_BS_MOB_LennyBaby', NULL, true);
-- 1558 <- source 72 Horden
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14018, 366.00, 6.58, -909.00, 3.1416, 1300, 1558, 'DebugArea_Lineup_72', NULL, true);
-- 1559 <- source 71 Carnosaur
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14019, 374.00, 6.58, -909.00, 3.1416, 1300, 1559, 'DebugArea_Lineup_71', NULL, true);
-- 1560 <- source 75 Rhinolion
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14020, 382.00, 6.58, -909.00, 3.1416, 1300, 1560, 'DebugArea_Lineup_75', NULL, true);
-- 1561 <- source 80 Twilla Tree
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14021, 362.00, 6.58, -937.00, 3.1416, 1300, 1561, 'DebugArea_Lineup_80', NULL, true);
-- Machines
-- 1562 <- MOB_AN_Android.BS_AN_Android: no template uses it, dressed from body_components
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14022, 347.00, 6.58, -942.00, 0.0000, 1300, 1562, 'DebugArea_Lineup_BS_AN_Android', NULL, true);
-- 1563 <- MOB_CA_DroneTank.BS_MOB_DroneTank: no template uses it, dressed from body_components
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14023, 351.00, 6.58, -942.00, 0.0000, 1300, 1563, 'DebugArea_Lineup_BS_MOB_DroneTank', NULL, true);
-- 1564 <- source 4 Prisoner retrieval unit
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14024, 355.00, 6.58, -942.00, 0.0000, 1300, 1564, 'DebugArea_Lineup_4', NULL, true);
-- 1565 <- source 69 Malfunctioning Drone
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14025, 359.00, 6.58, -942.00, 0.0000, 1300, 1565, 'DebugArea_Lineup_69', NULL, true);
-- 1566 <- source 81 Ancient Drone
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14026, 368.00, 6.58, -937.00, 3.1416, 1300, 1566, 'DebugArea_Lineup_81', NULL, true);
-- 1567 <- source 78 Straegis Fighter
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14027, 372.00, 6.58, -937.00, 3.1416, 1300, 1567, 'DebugArea_Lineup_78', NULL, true);
-- 1568 <- source 77 Straegis Beacon
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14028, 376.00, 6.58, -937.00, 3.1416, 1300, 1568, 'DebugArea_Lineup_77', NULL, true);
-- 1569 <- source 70 BattleWalker
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14029, 382.00, 6.58, -937.00, 3.1416, 1300, 1569, 'DebugArea_Lineup_70', NULL, true);
-- 1570 <- source 79 Straegis Titan
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14030, 368.50, 6.58, -922.00, 0.0000, 1300, 1570, 'DebugArea_Lineup_79', NULL, true);

-- Keep default-id inserts (`.savespawn`) past DA-10's reserved block
-- 13870-14099.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 14099), true);
