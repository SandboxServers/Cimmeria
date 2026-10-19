--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): spawns.
-- Campaign: docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
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

-- [UO-11] spawns 63822-63826 (provenance set by UO-11's block)

-- [UO-12] spawns 63827-63829 (provenance set by UO-12's block)

-- [UO-14] ambient 63880-63959 (CIMMERIA_AUTHORING, droppable)

--
-- The rows above load after spawnlist.sql's own sequence footer, so they
-- raise the sequence themselves. Floor 63999 is the top of Omega's block, so a
-- `.savespawn` or GM-made row never takes a reserved id.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 63999), true);
