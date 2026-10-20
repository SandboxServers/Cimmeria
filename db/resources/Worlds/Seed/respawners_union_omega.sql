--
-- Union Omega (world 18 `Omega_Site`): respawners. Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-42).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. respawners has no sequence, so there is no footer.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-03] respawner 180 `Omega Site Gate Plaza` (provenance set by UO-03's block)
--
-- CIMMERIA_RECONSTRUCTION (D-UO7): the gate plaza is where an arriving player
-- stands. World 18 had no respawner before this row, so a hub death respawned
-- where the player fell. Coordinates are the UO-01 placements worknote key
-- `hub.respawner` (candidate A, snapped floor y -51.42), 4.0 m from the C4
-- arrival point and 9.4 m from Modi (spawn 78).
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (180, 18, 'Omega Site Gate Plaza', 34.70, -51.42, -138.00);

-- [UO-34] begin: respawner 181
-- [UO-34] end
