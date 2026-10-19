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

-- [UO-10] spawns 63820, 63821, 63840 (provenance set by UO-10's block)

-- [UO-11] spawns 63822-63826 (provenance set by UO-11's block)

-- [UO-12] spawns 63827-63829 (provenance set by UO-12's block)

-- [UO-14] ambient 63880-63959 (CIMMERIA_AUTHORING, droppable)

--
-- The rows above load after spawnlist.sql's own sequence footer, so they
-- raise the sequence themselves. Floor 63999 is the top of Omega's block, so a
-- `.savespawn` or GM-made row never takes a reserved id.
SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 63999), true);
