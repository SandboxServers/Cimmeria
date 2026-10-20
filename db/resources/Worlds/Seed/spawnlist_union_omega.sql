--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): spawns.
-- Campaign: docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-42).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. Provenance: each block states its own tag
-- (RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-05] cast 63800-63803 (provenance set by UO-05's block)
-- UO-05 (D-UO5, F7): CIMMERIA_RECONSTRUCTION cast spawns in world 80. Coordinates and headings are the UO-01 placements (worknote keys cast.jackson, cast.bratac, cast.tealc, cast.thor), probed on omega_site_cmdcenter.nav.
-- Each stands on the mesh, keeps 1.5 m from the others, and keeps clear of pad 17 and respawner 26 (live_db_the_cast_stand_in_the_command_center).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63800, 2.12, 35.09, 9.99, 1.5478, 80, 63800, 'UnionOmega_Jackson', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63801, 10.00, 34.91, 13.50, 3.9848, 80, 63801, 'UnionOmega_Bratac', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63802, 12.00, 35.32, 13.50, 4.7124, 80, 63802, 'UnionOmega_Tealc', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63803, 2.12, 35.09, 13.50, 2.2733, 80, 63803, 'UnionOmega_Thor', NULL, true);

-- [UO-10] spawns 63820, 63821, 63840 (provenance set by UO-10's block)
-- CIMMERIA_RECONSTRUCTION placements (UO-01 worknote keys svc.banker, svc.registrar, svc.registrar_command, navmesh coordinates, world 18). svc.banker keeps its navmesh point, the only reachable standing point beside the bank prop; its art-area disagreement is recorded in UO-01 Art conflicts and left for the owner to rule (UO-U1).
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63820, -88.06, -48.02, -102.92, 0.0000, 18, 63810, 'UnionOmega_Banker', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63821, 30.00, -51.75, -130.00, 2.7683, 18, 63811, 'UnionOmega_TeamRegistrar', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63840, 28.00, -52.03, -126.00, 2.7450, 18, 63830, 'UnionOmega_CommandRegistrar', NULL, true);

-- [UO-11] begin: spawns 63841-63846
-- [UO-11] end

-- [UO-12] begin: spawns 63853-63857
-- [UO-12] end

-- [UO-27] begin: spawns 63847-63852
-- [UO-27] end

-- [UO-28] begin: spawns 63859-63863

-- CIMMERIA_RECONSTRUCTION, navmesh-sealed (UO-18, UO-19); D-UO16 B, D-UO28
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63859, 216.50, -63.59, -66.17, 0.0000, 18, 63835, 'UnionOmega_JaffaArmorT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63860, 213.50, -63.19, -66.17, 0.0000, 18, 63836, 'UnionOmega_JaffaArmorT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63861, 210.50, -63.63, -66.17, 0.0000, 18, 63837, 'UnionOmega_JaffaArmorT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63862, 207.50, -63.25, -66.17, 0.0000, 18, 63838, 'UnionOmega_JaffaArmorT4', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63863, 48.00, -44.46, -2.00, 4.8595, 18, 63835, 'UnionOmega_JaffaArmorT1_Common', NULL, true);

-- [UO-28] end

-- [UO-29] begin: spawns 63960-63971
-- [UO-29] end

-- [UO-30] begin: spawns 63865-63872
-- [UO-30] end

-- [UO-31] begin: spawns 63804-63808
-- [UO-31] end

-- [UO-32] begin: spawns 63819
-- [UO-32] end

-- [UO-33] begin: spawns 63818
-- [UO-33] end

-- [UO-35] begin: spawns 63810-63817
-- [UO-35] end

-- [UO-36] begin: spawns 63822-63823
-- [UO-36] end

-- [UO-39] begin: spawns 63809, 63824-63827
-- [UO-39] end

-- [UO-40] begin: spawns 63828-63835
-- [UO-40] end

-- [UO-42] begin: spawns 63836-63839
-- [UO-42] end

-- [UO-14] ambient 63880-63959 (CIMMERIA_AUTHORING, droppable)
-- CIMMERIA_AUTHORING (D-UO12, D-UO26): Omega's ambient population, cap 24. Droppable: delete this block, the [UO-14] template block and spawner_tests/union_omega/ambient.rs.
-- Coordinates and headings are the UO-01 placements (worknote keys amb.*, probed on the shipped navmeshes). Spawns 63902-63903 (bridge_e.1, bridge_w.1: no placement, D-UO22) have no row, so 22 of the 24 cap are used.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63880, -146.25, -38.02, -173.73, 5.7596, 18, 63860, 'UnionOmega_Ambient_cafeteria_1', NULL, true); -- amb.cafeteria.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63881, -142.69, -39.23, -165.90, 4.1886, 18, 63860, 'UnionOmega_Ambient_cafeteria_2', NULL, true); -- amb.cafeteria.2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63882, -150.25, -39.08, -166.80, 2.6183, 18, 63860, 'UnionOmega_Ambient_cafeteria_3', NULL, true); -- amb.cafeteria.3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63883, -79.66, -30.82, -210.23, 4.1888, 18, 63860, 'UnionOmega_Ambient_south_bunks_1', NULL, true); -- amb.south_bunks.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63884, -86.49, -31.02, -208.40, 2.6180, 18, 63860, 'UnionOmega_Ambient_south_bunks_2', NULL, true); -- amb.south_bunks.2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63885, -76.20, -30.90, -208.23, 4.1886, 18, 63860, 'UnionOmega_Ambient_south_bunks_3', NULL, true); -- amb.south_bunks.3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63886, -215.63, -35.38, -39.30, 4.1888, 18, 63860, 'UnionOmega_Ambient_bunker_outpost_1', NULL, true); -- amb.bunker_outpost.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63887, -217.46, -35.45, -46.13, 5.7596, 18, 63860, 'UnionOmega_Ambient_bunker_outpost_2', NULL, true); -- amb.bunker_outpost.2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63888, -224.29, -35.05, -44.30, 1.0472, 18, 63860, 'UnionOmega_Ambient_bunker_outpost_3', NULL, true); -- amb.bunker_outpost.3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63889, 99.78, -26.62, -237.58, 5.7596, 18, 63860, 'UnionOmega_Ambient_library_1', NULL, true); -- amb.library.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63890, 101.61, -26.62, -230.75, 4.1888, 18, 63860, 'UnionOmega_Ambient_library_2', NULL, true); -- amb.library.2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63891, 95.78, -26.62, -230.65, 2.6183, 18, 63860, 'UnionOmega_Ambient_library_3', NULL, true); -- amb.library.3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63892, 297.22, -61.16, -106.85, 4.1888, 18, 63861, 'UnionOmega_Ambient_jaffa_market_1', NULL, true); -- amb.jaffa_market.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63893, 297.39, -60.82, -117.14, 5.7594, 18, 63861, 'UnionOmega_Ambient_jaffa_market_2', NULL, true); -- amb.jaffa_market.2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63894, 300.68, -61.23, -104.85, 4.1886, 18, 63861, 'UnionOmega_Ambient_jaffa_market_3', NULL, true); -- amb.jaffa_market.3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63895, 222.69, -63.63, -27.90, 5.7596, 18, 63861, 'UnionOmega_Ambient_arena_1', NULL, true); -- amb.arena.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63896, 215.86, -63.63, -26.07, 1.0472, 18, 63861, 'UnionOmega_Ambient_arena_2', NULL, true); -- amb.arena.2
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63897, 226.25, -63.63, -20.07, 4.1886, 18, 63861, 'UnionOmega_Ambient_arena_3', NULL, true); -- amb.arena.3
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63898, 87.99, 30.82, -21.75, 5.7599, 80, 63860, 'UnionOmega_Ambient_console_ne_1', NULL, true); -- amb.console_ne.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63899, -77.81, 30.82, -16.55, 2.6183, 80, 63860, 'UnionOmega_Ambient_console_nw_1', NULL, true); -- amb.console_nw.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63900, 85.89, 30.82, 36.63, 5.7599, 80, 63860, 'UnionOmega_Ambient_console_se_1', NULL, true); -- amb.console_se.1
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63901, -76.81, 30.82, 37.73, 1.0475, 80, 63860, 'UnionOmega_Ambient_console_sw_1', NULL, true); -- amb.console_sw.1

--
-- The rows above load after spawnlist.sql's own sequence footer, so they
-- raise the sequence themselves. Floor 63999 is the top of Omega's block, so a
-- `.savespawn` or GM-made row never takes a reserved id.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 63999), true);
