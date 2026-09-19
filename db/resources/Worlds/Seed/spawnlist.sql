--
-- TOC entry 3238 (class 0 OID 63107)
-- Dependencies: 255
-- Data for Name: spawnlist; Type: TABLE DATA; Schema: resources; Owner: -
--

--
-- Harset hub respawn / stationary policy (packet H13, defect H-B7).
--
-- The 14 Harset mob rows -- eight Praxis Jaffa Guards (template 160) on
-- the stargate plaza, four Praxis Jaffa Lieutenants (159) on the two
-- door thresholds, Petbe (163) and Anat (43, world 68) -- carry
-- `respawn_secs = 30` and `is_stationary = true`. Before this, every
-- Harset NPC was one-shot: `mark_npc_dead` only stamps `respawn_at`
-- when `respawn_secs` resolves, so clearing the plaza emptied it until
-- a server restart. 30s is the "typical mob" floor documented on
-- `entity_templates.respawn_secs`, applied uniformly rather than tiered
-- because no Harset template has a `loot_table_id` -- the delay only
-- controls how fast world density is restored, never a farm rate.
--
-- Deliberately per-spawn, not per-template: templates 159 and 160 are
-- shared with the Castle hub (world 8, spawns 119/121/123/124), so a
-- template-level delay would silently change those too. Note the
-- precedence -- `COALESCE(spawnlist.respawn_secs,
-- entity_templates.respawn_secs)` in `cell/spawner/npcs.rs` means these
-- rows override whatever packet H11 later puts on the template.
--
-- `is_stationary` is interim, per decision D-H06: `harset.nav` is split
-- into 1,939 disconnected components and world 68 has no mesh at all,
-- so an NPC chasing across a boundary hits the `no_path` branch in
-- `npc_ai/fight.rs` and freezes mid-pursuit. The flag routes it to the
-- hold-position-and-fire branch, which is correct for a sentry anyway.
-- It gates fight-time pathing ONLY -- patrol, wander and follow never
-- read it -- so it is not a movement lock. Revisit after GH1.
--
-- Ring switches (template 3), the DHD (1) and the merchant basket (164)
-- are props that can never enter combat, so they stay NULL; a delay on
-- them would read as intent to the next author.
--
-- Removed by H13: spawn 1 (template 23, "Loot debug item") and spawn 42
-- (template 25, "Interaction Debug NPC - DO NOT USE"), both standing on
-- the gate plaza in the player's face on arrival. `spawnlist` has no
-- enabled/dev column, so deletion was the only available gate. Both
-- templates remain in `entity_templates.sql` and a GM reconstitutes
-- either on demand with `.spawn 23` / `.spawn 25`.
--

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (28, -95.8899994, 34.5909996, -98.8079987, 2.09426737, 12, 24, 'MessHall_Guard2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (27, -49.6899986, 24.6700001, -127.110001, 3.1414969, 12, 24, 'Cellblock_ArmoryGuard1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (26, -131.479996, 24.6700001, -116.969994, 0, 12, 24, 'Barracks_Guard2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (29, -96.25, 34.5909996, -91.5899963, 2.09426737, 12, 24, 'MessHall_Guard1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (36, -136.384995, 24.6709976, -135.671005, 0, 12, 24, 'Barracks_Guard3', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (25, -118.409996, 24.6700001, -118.349998, 1.57079637, 12, 24, 'Barracks_Guard1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (78, 39.5439987, -51.9140015, -129.919998, 3.13986993, 18, 65, 'Omega_Site_Modi', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (22, -287.290009, 67.2600021, -115.040001, 0, 12, 22, '329_CellDoorButton', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (15, -322.509979, 73.4720001, -209.830002, 1.57079637, 12, 21, 'ArmYourself_GuardBody', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (31, -98.5799942, 39.5499992, -77.0899963, 0, 12, 24, 'Hallway03_Guard', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (34, -52.6599998, 25.763998, -151.12999, 0, 12, 19, 'Cellblock_TerminalX', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (16, -187.714005, 55.848999, -141.503998, 4.71238899, 12, 19, 'Preparation_Terminal', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (12, -234.039993, 66.5199966, -124.699997, 1.57079637, 12, 18, 'ArmYourself_AmbernolVial', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (6, -289.808014, 65.473999, -113.139, 3.1414969, 12, 17, 'Prisoner_329', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (20, -289.464996, 68.5419998, -154.275986, 3.1414969, 12, 15, 'ArmYourself_NIDGuard', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (19, -328.299988, 73.4720001, -210.269989, 1.57079637, 12, 14, 'ArmYourself_FrostBody', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (8, -130.449997, 24.6709976, -92.0699997, 1.57079637, 12, 13, 'Cellblock_WoodenCrate', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (7, -191, 54.7199974, -138.587997, 3.1414969, 12, 10, 'Preparation_ColMarsh', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (11, -201.25, 56.079998, -131.610001, 1.57079637, 12, 8, 'Preparation_SMG1A', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (10, -220.257004, 66.7440033, -121.375, 4.71238899, 12, 4, 'ArmYourself_PrisonerRetrievalUnit', NULL, true);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (79, -54.8799973, 26.0799999, -163.839996, 1.04607904, 12, 3, 'Cellblock_ArmoryRingSwitch', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (17, -193.919998, 56.3199997, -152.160004, 1.10139823, 12, 3, 'Preparation_RingSwitch', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (23, -218.080002, 67.0400009, -122.719994, -0.56776464, 12, 3, 'HackTheRings_Switch', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (86, -61.348999, 34.5909996, -69.0319977, 0, 12, 24, 'Hallway04_Guard', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (222, 61.9690399, 1.77149999, 77.1715775, 3.11704898, 68, 43, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (224, -176.697678, -41.2540016, 125.271324, 5.93957376, 57, 164, 'FirstBug', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (4, -25.7569981, -67.8280029, 15.1359997, 0, 57, 3, 'HarsetRingLeftBottom', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (33, -101.5, 24.6700001, -51.2999992, 1.57079637, 12, 24, 'Hallway05_Guard2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (52, 199.988998, 1.31099999, 43.6080017, 0, 58, 29, 'SGCW1_GenHammond', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (53, 177.320007, 1.31099999, 43.3409996, 0, 58, 30, 'SGC_W1_Tealc', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (54, 178.688995, 1.31099999, 66.5899963, 3.13986993, 58, 31, 'SGC_W1_Airman', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (55, 241, 2.52999997, 18.0960007, 3.13986993, 58, 32, 'SGC_W1_ElevatorButton1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (56, -33.2260017, 1.31099999, 36.4720001, 1.59985006, 58, 33, 'SGC_W1_SamCarter', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (57, -52.0559998, 1.31099999, -44.9900017, 0, 58, 34, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (58, -69.4560013, 1.31099999, -44.0849991, 0, 58, 34, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (59, 210.781006, 1.31099999, -273.390015, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (60, 209.334, 1.31099999, -280.5, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (61, 188.104996, 1.31099999, -258.789001, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (62, 179.442001, 1.31099999, -271.589996, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (63, 123.702003, 1.31099999, -215.981995, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (64, 258.118988, -10.8479996, 40.0859985, 0, 58, 36, 'SGC_W1_Crate', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (65, 172.923004, -8.47599983, 5.34700012, 0, 58, 35, 'SGC_W1_GroomJaffa1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (66, 172.923004, -8.47599983, 5.34700012, 0, 58, 35, 'SGC_W1_GroomJaffa2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (67, 172.923004, -8.47599983, 5.34700012, 0, 58, 35, 'SGC_W1_GroomJaffa3', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (68, 169.498993, -5.62699986, 45.0880013, 0, 58, 37, 'SGC_W1_GateTerminal1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (32, -100.159996, 24.6700001, -43.8950005, 1.57079637, 12, 24, 'Hallway05_Guard1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (30, -128.852997, 39.5519981, -73.5339966, 3.1414969, 12, 24, 'Hallway01_Guard', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (82, -113.485001, 39.5519981, -63.0419998, 0, 12, 24, 'Hallway02_Guard', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (69, -45.5530014, 1.31099999, -271.557007, 2.49991012, 58, 35, 'SGC_W1_JaffaBomb', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (70, -42.1049995, 1.31099999, -273.911011, 0, 58, 38, 'SGC_W1_NaqBomb', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (71, -9.51399994, 1.31099999, 42.1949997, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (72, -12.5530005, 1.31099999, -43.637001, 0, 58, 32, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (73, 172.850998, 1.31099999, 53.2080002, 0, 58, 31, 'SGCW1_AirmanWalking', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (74, -85.6729965, 1.31099999, -248.613998, 0, 58, 39, 'SGC_W1_FirearmBody', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (75, -45.8569984, 1.31099999, -274.438995, 0, 58, 40, 'SGCW1_AirmanBody', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (76, -62.4490013, 2.29999995, -257.394989, 1.59985006, 58, 32, 'SGC_W1_ElevatorButton2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (77, -5.03599977, 1.31099999, 47.5610008, 0, 58, 35, NULL, NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (80, -87.0400009, 46.2399979, -160.319992, 2.59664607, 12, 3, 'Cellblock_UnusedRingSwitch', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (87, 352.691986, 70.2720032, 952.320007, 0, 8, 48, 'Castle_Coppleman', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (37, 4.40700006, -68.9570007, 31.5249996, 3.13996291, 57, 1, 'Harset_DHD', NULL);

-- RECONSTRUCTION (CA05 scope addition, worknotes/ca05.md "Zone-wide hostile respawn timers"):
-- no shipped World 8 spawn row set respawn_secs at all (spawner reads
-- `COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)`, both NULL everywhere),
-- so nothing hostile in Castle ever respawned -- the original SpawnSet timers were never
-- recovered (`spawn_sets.sql`/`spawn_points.sql` are both empty). Every hostile World 8 row
-- (templates 145 Prisoner Retrieval Unit, 146 NID Guard - Castle outside, 148 NID Guard -
-- Castle inside, plus this packet's own Castle_Romney/Castle_Muelbach/Castle_BravoOfficer*
-- above) gets the same respawn_secs=120 for one consistent zone-wide value. Non-hostile
-- World 8 NPCs (Gerschon, Copplemann, Marsh, Moh'katan, the Jaffa guards at Checkpoint
-- Alpha, the DHD, the Access Panel, both Zuritska rows, the comms terminal) are left NULL
-- (no death-gated mission step touches them; a one-shot NPC that "never respawns" is the
-- correct behavior for a unique named character).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (90, 364.778015, 70.2720032, 921.562012, 0, 8, 145, 'Castle_PRU1', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (92, 330.490997, 41.1819992, 653.107971, 0, 8, 147, 'Castle_AccessPanel', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (93, 782.252991, 29.809, 374.493988, 0, 8, 146, 'Castle_NidGuard2', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (94, 850.327026, 29.677, 384.031006, 0, 8, 146, 'Castle_nidGuard3', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (95, 904.536011, 28.5310001, 501.947998, 0, 8, 146, 'Castle_NidGuard4', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (96, 924.85199, 24.2639999, 485.769012, 0, 8, 146, 'Castle_NidGuard5', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (97, 923.387024, 24.2639999, 472.053009, 0, 8, 146, 'Castle_NidGuard6', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (98, 939.864014, 24.2639999, 495.997009, 0, 8, 145, 'Castle_PRU2', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (99, 906.872009, 26.5900002, 535.080994, 0, 8, 146, 'Castle_NidGuard7', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (100, 891.768005, 24.2639999, 453.825989, 0, 8, 146, 'Castle_NidGuard8', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (101, 962.408997, 24.7989998, 469.571991, 0, 8, 146, 'Castle_NidGuard9', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (102, 940.247009, 24.2639999, 526.40802, 0, 8, 146, 'Castle_NidGuard10', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (104, 589.320984, 24.0149994, 610.692993, 0, 8, 146, 'Castle_NidGuard11', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (105, 581.228027, 22.0650005, 638.406982, 0, 8, 146, 'Castle_NidGuard12', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (106, 613.719971, 17.3050003, 625.083984, 0, 8, 145, 'Castle_PRU3', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (107, 648.77301, 20.3920002, 631.796997, 0, 8, 146, 'Castle_NidGuard13', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (108, 623.005005, 19.2490005, 564.64801, 0, 8, 146, 'Castle_NidGuard14', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (109, 686.63501, 25.1779995, 475.608002, 0, 8, 146, 'Castle_NidGuard15', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (110, 758.171997, 29.8799992, 421.436005, 0, 8, 146, 'Castle_NidGuard16', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (111, 554.335022, 24.3710003, 607.067017, 0, 8, 145, 'Castle_PRU4', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (114, 359.778992, 70.2720032, 983.057983, 0, 8, 145, 'Castle_PRU5', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (112, 429.638, 70.1110001, 996.55603, 1.79999995, 8, 149, 'Castle_SgtGerschon', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (115, 326.414001, 70.2720032, 933.495972, 0, 8, 148, 'Castle_NidGuard17Inside', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (89, 381.843994, 70.2720032, 997.200012, 0, 8, 148, 'CastleNidGuardXInside', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (116, 294.158997, 55.3919983, 894.442993, 0, 8, 148, 'CastleNidGuard18Inside', NULL, 120);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (118, 810.731995, 55.2010002, 515.012024, 0, 8, 10, 'Castle_ColMarsh', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (121, 786.973999, 55.3019981, 517.007996, 0, 8, 160, 'Castle_PrJaffaGuard1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (120, 807.478027, 55.2080002, 515.395996, 0.800000012, 8, 54, 'Castle_Mohkatan', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (119, 809.015015, 55.2080002, 514.406006, 0.300000012, 8, 159, 'Castle_PrJaffaLieuternant', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (123, 790.049011, 55.2299995, 515.414978, 0.400000006, 8, 160, 'Castle_PrJaffaGuard2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (124, 788.481995, 55.2229996, 511.394012, 1.29999995, 8, 160, 'Castle_PrJaffaGuard3', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (2, 806.27002, 55.0999985, 517.23999, 2.37899995, 8, 162, 'Castle_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (39, 209.748001, -1.00800002, -552.89801, -2.4000001, 19, 1, 'Tollana_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (40, 629.596008, -60.9129982, 381.289001, 0, 23, 1, 'Beta_Site_Evo_1_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (223, -165.512543, -41.2696266, 99.4154739, 3.21522403, 57, 163, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (129, -171.774002, -27.0149994, 235.979996, 0, 57, 3, 'HarsetRingLeftTop', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (130, 215.817993, -34.125, 39.4179993, 0, 57, 3, 'HarsetRingRight', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (128, -194.459, -40.1669998, 81.2750015, 0, 57, 3, 'HarsetRingLeft', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (135, 34.5750008, -35.7700005, 55.6980019, 0, 18, 3, 'OmegaSiteBeamToCmdCenter', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (136, 191.981995, -48.9300003, -122.780998, 0, 18, 3, 'OmegaSiteBeamRight', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (132, 34.4099998, -42.6500015, -75.4629974, 0, 18, 3, 'OmegaSiteBeamCenter', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (133, -18.5569992, -29.4899998, -207.522003, 0, 18, 3, 'OmegaSiteBeamBottom', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (134, -210.149994, -36.7299995, -92.3310013, 0, 18, 3, 'OmegaSiteBeamLeft', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (137, 6.12099981, 35.2299995, 9.98700047, 0, 80, 3, 'OmegaSiteCmdCenterBeam', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (138, 56.6879997, -182.143997, -28.7070007, 0, 15, 3, 'LuciaRingCenterTop', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (139, -90.9049988, -182.085007, -128.337997, 0, 15, 3, 'LuciaRingCenterLeft', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (140, 227.171005, -182.072998, -128.270004, 0, 15, 3, 'LuciaRingCenterRight', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (141, 67.586998, -163.302994, -142.606003, 0, 15, 3, 'LuciaRingCenter1', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (142, 67.2340012, -146.643997, -375.816986, 0, 15, 3, 'LuciaRingCenter2', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (143, -189.376007, -170.645004, -720.789001, 0, 15, 3, 'LuciaRing_fff8fffe', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (232, 4.69599581, -58.6540871, -188.246613, 0, 57, 159, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (145, 1113.26599, -146.867996, 214.621994, 0, 15, 3, 'LuciaRing_0002000b', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (146, 118.615997, -15.4420004, 739.77301, 0, 15, 3, 'LuciaRing_00070001', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (147, -761.302002, -173.593002, -160.951996, 0, 15, 3, 'LuciaRing_fffefff8', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (233, -5.13134289, -58.6591301, -188.338028, 6.2586422, 57, 159, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (149, 746.554993, -19.2099991, 68.75, 0, 23, 3, 'BetaSite_00000007', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (148, 146.716003, 8.5909996, 403.27301, 0, 23, 3, 'BetaSite_Ring_00040001', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (151, 700.47998, 157.949997, 70.3649979, 0, 23, 3, 'BetaSite_Beam_000007_Ship', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (152, 834.53302, 20.7049999, 773.976013, 0, 23, 3, 'BetaSite_Ring_00070008', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (235, -4.43621397, -67.6082916, -231.103455, 1.57079601, 57, 159, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (236, 4.33233595, -67.6082916, -231.151123, 4.71238899, 57, 159, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (153, 72.1409988, -167.167999, -75.0699997, 2.5999999, 15, 1, 'Lucia_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (154, 245.442993, 8.93599987, -983.388977, 0, 73, 1, 'Ihpet_Crater_Light_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (155, 235.244003, -1.10899997, -571.307983, 0, 19, 3, 'Tollana_Ring_fffa0002', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (156, -766.127014, 9.5, 379.945007, 0, 19, 3, 'Tollana_Ring_0003fff8', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (41, 42.6040001, -51.9140015, -134.313995, 0.5, 18, 1, 'Omega_Site_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (157, 246.192993, 8.94099998, -984.065002, 0, 72, 1, 'Ihpet_Dark_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (158, -42.2970009, -193.567993, 400.56601, 0, 77, 1, 'Menfa_Dark_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (159, 102.723999, -193.828003, 486.110992, 0, 77, 3, 'Menfa_Dark_Ring_00040001', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (160, 25.1140003, 29.4500008, 425.727997, 0, 77, 3, 'Menfa_Dark_Ring_00040000', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (161, 340.713989, 22.4319992, -52.1450005, 0, 77, 3, 'Menfa_Dark_Ring_ffff0003', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (162, 404.408997, 38.651001, -418.958008, 0, 77, 3, 'Menfa_Dark_Ring_fffb0004', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (163, 84.939003, 6.73099995, -160.723999, 0, 77, 3, 'Menfa_Dark_Ring_fffe0000', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (164, -142.886002, 28.9500008, -309.699005, 0, 77, 3, 'Menfa_Dark_Ring_fffcfffe', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (165, -25.1550007, 9.77000046, -184.884003, 0, 77, 3, 'Menfa_Dark_Ring_fffeffff', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (166, 75.8270035, 8.03199959, -13.6389999, 0, 77, 3, 'Menfa_Dark_Ring_ffff0000', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (167, -264.253998, 12.9700003, 441.229004, 0, 77, 3, 'Menfa_Dark_Ring_0004fffd', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (168, -323.920013, 30.2719994, 163.039001, 0, 77, 3, 'Menfa_Dark_Ring_0001fffc', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (169, -469.971008, 18.7520008, 297.895996, 0, 77, 3, 'Menfa_Dark_Ring_0002fffb', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (170, 691.112, -43.1679993, -20.7689991, 0, 77, 3, 'Menfa_Darj_Ring_ffff0006', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (172, 750.802979, -45.9249992, 70.086998, 0, 77, 3, 'Menfa_Dark_Ring_00000007_WTF', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (173, 627.874023, -45.9749985, 69.887001, 0, 77, 3, 'Menfa_Dark_Ring_00000006_WTF', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (175, 352.251007, -142.628006, -526.934021, 0, 77, 3, 'Menfa_Dark_Ring_fffa0003', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (176, -144.205002, -73.0439987, -547.747009, 0, 77, 3, 'Menfa_Dark_Ring_fffafffe_WTF', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (177, -203.425995, -70.2300034, -638.767029, 0, 77, 3, 'Menfa_Dark_Ring_fff9fffd', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (179, -267.266998, -72.7669983, -546.619995, 0, 77, 3, 'Menfa_Dark_Ring_fffafffd_WTF', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (180, -617.122986, -109.262001, -455.718994, 0, 77, 3, 'Menfa_Dark_Ring_fffbfff9', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (181, -767.549988, -45.5880013, 126.154999, 0, 77, 3, 'Menfa_Dark_Ring_0001fff8_WTF', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (182, -704.012024, -42.8699989, 34.9430008, 0, 77, 3, 'Menfa_Dark_Ring_0000fff8', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (184, -644.380005, -45.5810013, 126.182999, 0, 77, 3, 'Menfa_Dark_Ring_0001fff9', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (185, -618.619995, -44.2919998, 464.38501, 0, 77, 3, 'Menfa_Dark_Ring_0004fff9_WTF', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (187, -677.40802, -41.5929985, 372.477997, 0, 77, 3, 'Menfa_Dark_Ring_0003fff9', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (38, 98.086998, -16.7689991, 237.335007, 3.1415, 61, 1, 'Dakara_E1_DHD', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (144, 639.210022, -131.910995, -599.18103, 0, 15, 3, 'LuciaRing_fff90006', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (188, 183.636002, 94.7699966, 857.731018, 0, 15, 3, 'LuciaRing_00080001', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (234, -18.6429996, -68.9227982, 11.3282003, 1.59534001, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (225, -18.7497005, -68.9227982, 19.3561001, 1.54615676, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (231, 18.5478001, -68.9227982, 11.3548994, 4.73683691, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (230, 29.6623993, -68.9227982, 22.3821983, 0, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (229, 21.5998001, -68.9227982, 22.1867008, 6.25854588, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (228, 18.6380997, -68.9227982, 19.4368992, 4.68784523, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (227, -29.5678005, -68.9227982, 22.3317986, 0, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (226, -21.5496998, -68.9227982, 22.1611977, 6.25854588, 57, 160, NULL, NULL, true, 30);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (127, 26.1189976, -67.8280029, 15.4709997, 0, 57, 3, 'HarsetRingRightBottom', NULL);

INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (237, 5.97424412, 0.351500005, 11.9172192, 3.58337903, 2, 166, NULL, NULL);

-- RECONSTRUCTION (CA05, worknotes/ca05.md "Interrogation Block" -- HIGH confidence):
-- anchored on one of 36 CA-Cell_Doorway01_Pf0 prefab instances recovered from
-- Castle-000a0002.umap (raw UE3 X=104258.69, Y=26799.88, Z=6679.10), converted via
-- server.x=rawY/100, server.y=rawZ/100, server.z=rawX/100 (formula confirmed against
-- the existing Castle.ThroneRoom (2049) and the Stargate/DHD prefab, both within a few
-- units of their known seeded positions). Independently corroborated by a second prefab
-- family: 36 `EM-ViewScreen03_Pf0` instances (the per-cell door screens, a variant that
-- appears in NO other Castle tile) occupy the same two tiles at y=69.27 -- i.e. ~2.5 units
-- up the wall from this row's 66.79 floor -- on a grid spanning x[228.32, 333.29] and
-- z[1021.82, 1099.58], which is the corridor bbox point set 2082 uses. Exact spot within the
-- cell is a MEDIUM-confidence placement at the doorway itself.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (238, 268.0, 66.79, 1042.59, 3.14159274, 8, 168, 'Castle_Zuritska_Cell', NULL);

-- Two separable claims here; only the second is a reconstruction.
--
-- ORIGINAL_DATA -- the room's NAME AND FLOOR. Dialog 2576 screen 96821
-- (`dialog_screens.sql:11759`) says "take him to **the Communications room on Level 5**. He
-- has to patch me into the Castle's comm systems". So a Communications room exists, it is on
-- Level 5, and its purpose is the comm systems. None of that is inferred, and the point set's
-- name is not a label this packet invented.
--
-- RECONSTRUCTION (MEDIUM confidence) -- WHERE it is. The dialog gives no coordinate, and no
-- map asset carries the string "Communications" or "Level 5", so the room had to be matched
-- to geometry. It IS located, on a fourth recon pass: the first three searched the map name
-- tables for comm/terminal/workstation and found nothing, and texts.sql moniker 7720
-- `DN_Ob_D_HumanViewScreen_Castle_CommTerminal` supplies the missing search term -- the assets
-- are called *screens*, never "terminals". Scanning all 145 tiles for the screen/monitor
-- families turns up exactly one enclosed room built around a monitor wall, in
-- Castle-00080002.umap:
--   * two `CA-Monitor_Wall00_Pf0` PrefabInstances (`GP-Monitor_Wall00`) side by side at
--     server (268.38, 55.48, 852.20) and (275.04, 55.48, 852.20) -- a double-wide monitor
--     wall, the only one in the Castle that is not set dressing in another named room;
--   * two `CA-SecurityLock00_Pf0` above them at (266.96, 60.29, 851.82) and
--     (276.53, 60.29, 851.82) -- the room is access-controlled, which fits a Level-5
--     communications room and nothing else the Castle has;
--   * three `Ca-StasisCamber_Wallstation00` sharing the same z=852.08 wall at x 264.89 /
--     271.79 / 278.79, floor height y=55.20;
--   * two `CA-normal_room_corner_a_00` corner meshes at (261.40, 62.55, 861.20) and
--     (282.49, 62.55, 861.20), which is what actually bounds the room: x[261.4, 282.5],
--     monitor wall on z=852.2, opposite side near z=861.2.
-- The earlier draft of this row placed the comms actors in the Castle-00090003 room at
-- (388-392, 55.2, 930-944) because it also has a six-screen `EM-ViewScreen02_Pf0` bank.
-- That was wrong: the same room holds four `CA-SymbioteChamber00` and five
-- `Ca-StasisCamber_Wallstation01` at x 398.2-398.8, so it is CA15's symbiote chamber and its
-- screens are the stasis monitors. Do not put mission 704 there.
-- Existing spawn 116 (CastleNidGuard18Inside, (294.16, 55.39, 894.44)) stands on this same
-- y~55.2 interior level ~40 units away, so the floor is real and populated -- and y~55 is one
-- interior level below the Interrogation Block (y~66.79) and above the throne room (y~41-48),
-- which is consistent with "Level 5" but does not prove it, since nothing maps the game's
-- floor numbering onto these heights. MEDIUM not HIGH for that reason: the room's identity is
-- matched by set dressing, so confirm the exact standing spot -- and that this room is the one
-- on Level 5 -- with an in-client `.location` walk before UAT M3. The NAME is not in question.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (239, 271.7, 55.2, 858.0, 0, 8, 168, 'Castle_Zuritska_Comms', NULL);

-- RECONSTRUCTION (CA05, worknotes/ca05.md "Interrogation Block" -- HIGH confidence):
-- second cell-corridor segment in Castle-000a0003.umap (raw X=104258.69, Y=32063.88,
-- Z=6679.10), same conversion formula as Castle_Zuritska_Cell above. Romney is placed in
-- the second half of the corridor (interpreted as "Interrogation Room 02").
-- respawn_secs=120: mission 703 completes on `entity_death Castle_Romney`; the shipped seed
-- sets no respawn on any NID template (146/148 both NULL), which would permanently lock 703
-- for every other player in the shared world after the first kill. No existing NID-guard
-- value to copy, so this uses the packet's own fallback (120s) per the missions 702-704
-- worker's finding (docs/analysis/castle-rebuild/worknotes/ca05.md).
-- UAT CORRECTION (2026-09-18 colo playtest, docs/analysis/playtests/2026-09-18-colo-castle
-- section 8.5): the placement above is WRONG in practice. Tile Castle-000a0003 is an
-- unfinished mirror wing the original developers sealed off -- untextured blocking wall,
-- hole in the floor, solid panel where the connecting door should be -- so (320.64, 66.79,
-- 1042.59) can only be reached with GM `.gotoxyz` or client `ghost`. A raw umap read cannot
-- see that. Romney now stands at the far (dead) end of the ACCESSIBLE wing's cell corridor,
-- ~24 units past Zuritska's door, on a line the playtest PROVED walkable: the player and the
-- escort traversed x 240..277 at z ~1036 (movement.npc waypoints for npc 100112, 00:28:50-
-- 00:29:38 UTC). heading = +PI/2 faces back along the corridor toward the approaching
-- player. MEDIUM confidence on the exact spot; HIGH that it is reachable. Also: heading 0
-- faces +Z, which for a cell on the +Z side of the corridor is the BACK WALL -- hence the
-- PI heading on Castle_Zuritska_Cell above.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (240, 244.0, 66.79, 1036.0, 1.57079637, 8, 169, 'Castle_Romney', NULL, 120);

-- RECONSTRUCTION (CA05, worknotes/ca05.md "Bunker above Checkpoint Bravo" -- MEDIUM-HIGH
-- confidence): objective 2799 (mission_objectives.sql:6629) states outright that Muelbach
-- "is holed up in the bunker above Checkpoint Bravo", so she does NOT stand at the
-- checkpoint with the officers (this row previously placed her at (960, 25, 490), inside
-- the Humvee cluster, which contradicts that line). The Castle tree has exactly one bunker
-- asset elevated over the checkpoint: the `CastleEast_SmallBunkerTunnel` PrefabInstance in
-- Castle-0004000a.umap at server (1004.16, 48.00, 413.60) -- 23 units above and ~85 units
-- from the Checkpoint Bravo cluster (y 24.2-28.5), versus `EM-Bunker_Frost00`(_Pf0) at
-- (985.65, 27.95, 495.64)/(962.06, 24.26, 471.59), which are level with the checkpoint and
-- therefore not "above" it. The tunnel is a lit interior with its floor at exactly y=48.00
-- (the prefab and a co-located TriggerVolume, with EM-Brace_Wall01 at 48.00,
-- EM-Pipe_Floor_Med00 at 49.32, EM-WallLight01_Pf0 wall lights at 54.08 and CA-Cell_Decor03
-- ceiling decor at 56.02 above it), spanning x[1001,1017] z[406,423]; she is placed on that
-- floor midway between the two wall-light pairs (x 1007.2 / 1014.9). Not HIGH because no
-- asset is named "Bravo"/"Checkpoint", so the checkpoint identification the "above" relation
-- is measured against is itself inferred from set dressing.
-- respawn_secs=120: mission 708 step 2416 completes on `entity_death` of Muelbach or any
-- Bravo officer; same shared-world lockout concern and fallback as Castle_Romney above.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (241, 1008.0, 48.0, 414.0, 0, 8, 170, 'Castle_Muelbach', NULL, 120);

-- RECONSTRUCTION (CA05, worknotes/ca05.md "Checkpoint Bravo" -- MEDIUM confidence): inside
-- the Humvee/bunker cluster in Castle-00040009.umap -- three `EM-Humvee00` PrefabInstances
-- at server (950.60, 24.20, 496.75), (960.41, 28.51, 478.33), (973.96, 25.12, 471.08) with
-- `EM-Bunker_Frost00_Pf0` at (962.06, 24.26, 471.59); a parked Humvee alongside bunker set
-- dressing reads as a military checkpoint, though no "Bravo"/"Checkpoint" named asset exists
-- to confirm the label. Objective 2798 ("NID Officers at Checkpoint Bravo may have a Control
-- Crystal", mission_objectives.sql:6625) is what puts the officers HERE and Muelbach in the
-- bunker above -- the two options on step 2416 are deliberately in different places.
-- N=3 officers per packet default. The exact ground height at each spot is not decodable
-- until the terrain decoder lands (CA14), so y is taken from the nearest recovered asset.
-- respawn_secs=120, same reasoning as Castle_Romney above.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (242, 955.0, 25.0, 475.0, 0, 8, 171, 'Castle_BravoOfficer1', NULL, 120);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (243, 965.0, 25.0, 485.0, 0, 8, 171, 'Castle_BravoOfficer2', NULL, 120);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, respawn_secs) VALUES (244, 970.0, 26.0, 478.0, 0, 8, 171, 'Castle_BravoOfficer3', NULL, 120);

-- RECONSTRUCTION (CA05, worknotes/ca05.md "Checkpoint Bravo" -- LOW confidence on the
-- position, this is the weakest coordinate in the packet): objective 2794 ("(Option #1)
-- Force a guard to surrender and reveal what he knows", mission_objectives.sql:6619) names
-- no location at all, and unlike 2798/2799 there is nothing to anchor him to. Placed inside
-- the Checkpoint Bravo cluster because step 2415's other option is the throne-room Access
-- Panel (spawn 92) and putting the two options in different rooms matches how 2416's pair
-- is laid out. A competing reading -- a guard nearer the gate/Checkpoint Alpha, since 2415
-- is about why the gate is malfunctioning -- is equally consistent with the text; resolve it
-- in the in-client pass, not from the map assets. Non-hostile, so no respawn timer.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (245, 958.0, 25.0, 480.0, 0, 8, 172, 'Castle_SurrenderGuard', NULL);

-- RECONSTRUCTION (CA05, worknotes/ca05.md "Communications room" -- MEDIUM confidence):
-- on the y=55.20 comms-room floor, centred between the two monitor walls (x 268.38 and
-- 275.04) and 2.8 units out from the z=852.2 wall they are mounted on, so the interactable
-- prop stands in front of the screens it represents and the player can reach it from the
-- room side. See Castle_Zuritska_Comms above for the full evidence chain. `heading` is 0
-- like every other Castle prop row (spawns 2, 92); the facing that would turn it toward the
-- monitor wall is unverified, so this does not invent one -- fix it in the same in-client
-- `.location` pass that confirms the room.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (246, 271.7, 55.2, 855.0, 0, 8, 173, 'Castle_CommsTerminal', NULL);

--
-- PLACEMENT PASS B (packet H14, world 57 Harset hub exterior).
-- Ledger: docs/analysis/harset-rebuild/placements/B-world57-population-and-regions.md
-- Method: docs/analysis/harset-rebuild/placements/METHOD.md
--
-- Every coordinate below is a GUESS with a recorded evidence class and
-- confidence, not a pin. The ledger row (id PL-B-nn) carries the evidence,
-- the checks run, how to verify it in-client and how to correct it. Correct
-- these rows; do not re-derive them.
--
-- ONE FACT THAT SHAPES THE WHOLE BLOCK. `data/spaces/harset.nav` does not
-- cover the hub's upper quarters at their real floor height. A ring probe
-- around every named Jaffa Zone landmark (the tent rows, the barracks, the
-- high-wall arch, both fountains, the military tent, `FirstBug` itself)
-- found NO on-mesh point within 12 m at the floor y the geometry actually
-- has (-41.3, from `obj_slab`); the nearest mesh polygons sit 7-11 m above
-- or below it. The two AUTHORED rows in that quarter -- Petbe (spawn 223)
-- and `FirstBug` (spawn 224) -- are themselves off-mesh for the same
-- reason, so "off-mesh" here is a statement about the navmesh, not about
-- the placement. World 57 is `navmesh_mode = 'advisory'` (H53), so an
-- off-mesh spawn is not rubber-banded; it does mean NPC pathing there is
-- dead, which is why every row below is `is_stationary = true`.
-- Consequence: rows on the stargate plaza are placed on-mesh on the hub
-- component (187); rows in the Jaffa Zone, at the shield towers and on the
-- palace terrace are placed on the TRUE GEOMETRY FLOOR from `obj_slab` and
-- recorded as off-mesh. See GH1.
--
-- `heading` is atan2(dx, dz) radians, 0 = +Z (cell/service/npc_ai/fight.rs
-- line 633). Every value below is derived from an approach direction or
-- from the landmark the entity faces -- never left at 0, which is the
-- Castle lesson (reconstructed rows all faced walls).
--

-- PL-B-01 / PL-B-02: Hansen and Jacobs. Spec observation "left of the gate,
-- walking outward". Left of the gate is -X (arriving through the gate at
-- z=38 the player walks outward toward -Z, past the two `GA-GuardPost00`
-- prefabs at z=3.5, and left of that heading is -X). Placed just outside
-- the authored plaza guard line (spawns 225/234 at x=-18.6) at the same
-- floor y those rows use, so they read as a pair standing off the walkway.
-- They face the gate (the way a visitor arrives) rather than outward: the
-- 2009 walk cannot be reproduced until GH1 gives world 57 patrol paths.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (300, -24.0, -68.9227982, 14.0, 0.7836, 57, 212, 'Harset_Hansen', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (301, -26.5, -68.9227982, 18.5, 0.935, 57, 213, 'Harset_Jacobs', NULL, true, 30);

-- PL-B-03: Lo'rak in the bazaar. Two merchant clusters exist in world 57:
-- the lower-level one at y=-60 (x 100-126, z 51-94) and the merchant street
-- that runs south out of the stargate plaza at y=-69.2 (`GA-MerchantTent00`
-- through `03`, x 19-46, z -13 to -105). Lo'rak is placed in the second,
-- because it is the one on the hub navmesh component (187) and therefore
-- the one a player reaches on foot from the gate. Faces back up the street
-- toward the plaza exit at (0, 4) -- the direction a customer arrives from.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (302, 22.0, -68.9, -30.0, 5.709, 57, 201, 'Harset_Lorak', NULL, true, 30);

-- PL-B-04 / PL-B-05: the two 1326 Lan'toc Jaffa (chains 6335 and 6336 in
-- harset_jaffa_chains.sql expect exactly these tags). Two individuals of
-- template 204 because the accept/reject outcome belongs to the NPC and the
-- engine has no way to pick between two dialogs on one tag (H22).
-- Placed in the Jaffa Zone's west tent row, between the `JF-Tent03` row at
-- x=-154.5 and `GA-Barracks01` at (-198.1, 84.4), 5 m apart so they read as
-- two men standing in the camp street rather than one entity. Both face the
-- `HarsetRingLeft` pad at (-194.5, -40.2, 81.3), which is how a player
-- arrives in this quarter.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (303, -160.0, -41.28, 84.0, 4.6335, 57, 204, 'Harset_FormerRaJaffa', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (304, -160.0, -41.28, 89.0, 4.4921, 57, 204, 'Harset_FormerRaJaffa2', NULL, true, 30);

-- PL-B-06: the Suspicious Jaffa (1371 / 1322 arrest target). The one
-- Jaffa Zone placement in this block that IS on-mesh: the shipped mesh has
-- a rectangular fragment (component 853, x -163 to -158, z -30 to -21) that
-- coincides with a tent floor at y=-42.1, and `obj_slab` confirms the real
-- floor there too (100 m^2 at y[-42, -41]). A man lurking inside a tent in
-- the south camp is also what the name suggests. Faces the south camp's
-- fountain (`TOL-FluidPlaneCircle_Flat00` at -176.6, -41.0, -5.0), i.e.
-- diagonally out of the tent toward the camp's open ground.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (305, -159.0, -41.28, -26.0, 5.5862, 57, 205, 'Harset_SuspiciousJaffa', NULL, true, 30);

-- PL-B-07 / PL-B-08: `SecondBug` and `ThirdBug`, the other two 742 baskets.
-- The AREA is authored, not guessed: step 2504 is literally "Hide the
-- listening devices in the Jaffa area in Harset", and the existing
-- `FirstBug` (spawn 224) sits in the north Jaffa camp at (-176.7, -41.25,
-- 125.3). Both new baskets go in the same camp so the player does not cross
-- the map three times: one beside `JF-Tent01`/`JF-Tent03` at x=-185, one
-- beside the `JF-Tent03` row at x=-147. `obj_slab` confirms the floor at
-- y[-42, -41] in both columns. Each faces the tent it belongs to.
-- `respawn_secs` is set for consistency with D-H17; a prop never dies, so
-- it never fires.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (306, -186.0, -41.28, 117.5, 2.8993, 57, 164, 'SecondBug', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (307, -147.0, -41.28, 104.5, 3.6932, 57, 164, 'ThirdBug', NULL, true, 30);

-- PL-B-09 / PL-B-10 / PL-B-11: the three shield towers (1240 "examine 3
-- Shield Towers"). Strong landmark evidence: the map contains exactly three
-- `GA-Tow*` instances and the mission wants exactly three towers --
-- `GA-TowTall01` (-226.0, -41.4, 37.7), `GA-TowMed00` (-166.1, -31.1,
-- 234.8), `GA-TowShort01` (0.0, -30.7, 288.8). Template 243 is a console
-- mesh (`GA-PuzzleStation00`), not the tower itself, so each row is the
-- examinable console set 3-4 m off its tower's pivot and facing it. Y is
-- the tower prefab's own origin height in each case, which `obj_slab`
-- confirms as a real surface; tower 1 is the weakest of the three because
-- its column is a hillside with no flat level at all (0.0 m^2 flat<=5deg).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (308, -223.0, -41.36, 37.72, 4.7124, 57, 243, 'Harset_ShieldTower1', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (309, -169.5, -31.13, 234.6, 1.5126, 57, 243, 'Harset_ShieldTower2', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (310, -3.0, -30.72, 285.5, 0.7378, 57, 243, 'Harset_ShieldTower3', NULL, true, 30);

-- PL-B-12: Shield Controls (1374 objective 4087). No landmark names it, so
-- this is reasoned from neighbouring evidence and is the lowest-confidence
-- row in the block. `GA-Props:GA-Viewscreens00` is the map's ONLY Goa'uld
-- control-panel prop -- one instance, at (-88.3, -28.9, 213.9) -- and the
-- shield towers are Goa'uld (`GA-`) tech, so the Goa'uld screen bank is the
-- best candidate for the Goa'uld shield console. `obj_slab` finds a real
-- constructed floor there (128 m^2 at y[-31.0, -30.5], 502 tris), which is
-- the Y used; the prop stands 1.7 m west of the screens and faces them.
-- The competing reading is that the controls are inside the Command Center
-- (world 68) -- if the in-client pass finds nothing here, that is where to
-- look next, and this row should be deleted rather than moved.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (311, -90.0, -30.7, 213.9, 1.5557, 57, 248, 'Harset_ShieldControls', NULL, true, 30);

-- PL-B-13 / PL-B-14: the Bank (1374 objective 4088, 1352) and its banker.
-- `GA-Props:GA-Bank00` has five instances and three of them sit on top of a
-- `CA-Arch:CA-Courtyard_Str00`, which is why the name alone is weak
-- evidence. The tie-break is geometry: of the five, only the Jaffa-Zone
-- courtyard pair at (-187.9 / -184.5, -41.4, 162.3) is ON the navmesh
-- (component 1441, dy -0.13 m), and `obj_slab` gives it a 180 m^2 floor at
-- y[-42, -41]. A walkable courtyard is what a bank needs; the other four
-- instances are 5-68 m off any mesh. Storage Lo'taur (template 219, the
-- banker per the tag registry and audit defect 15) stands 2.5 m off the
-- anchor, facing the courtyard centre.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (312, -184.5, -41.28, 162.27, 4.7492, 57, 248, 'Harset_BankAnchor', NULL, true, 30);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (314, -186.0, -41.28, 160.0, 0.5814, 57, 219, 'Harset_StorageLotaur', NULL, true, 30);

-- PL-B-15: Petbe's quarters exterior search object (1244 step 3619, 1243
-- objective anchor). Two candidate sites, neither decisive:
--   (a) the terrace at (-165, -28.2, 233) carrying the map's only two
--       `HP-Props:HP-Brazier00` instances -- a unique art family appears
--       exactly once in a map when it belongs to one named place -- reached
--       by the `HarsetRingLeftTop` ring pad 7 m away at y=-27.0, with a
--       272 m^2 floor at y[-28.5, -28.0] from `obj_slab`;
--   (b) beside Petbe's own authored spawn 223 at (-165.5, -41.3, 99.4).
-- (a) is seeded: a ring pad implies a destination that matters, and where
-- a Goa'uld stands on duty is not where he sleeps. If the in-client pass
-- finds no building on that terrace, move this row to (b).
-- Note the shield-tower-2 console (spawn 309) stands on the same terrace;
-- a tower in the palace forecourt is coherent, and the two regions
-- deliberately overlap.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary, respawn_secs) VALUES (313, -160.5, -28.25, 232.6, 5.0039, 57, 244, 'Harset_PetbeQuarters', NULL, true, 30);

--
-- TOC entry 3335 (class 0 OID 0)
-- Dependencies: 256
-- Name: spawnlist_spawn_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('spawnlist_spawn_id_seq', 314, true);

