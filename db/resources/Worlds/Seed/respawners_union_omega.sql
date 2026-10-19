--
-- Union Omega (world 18 `Omega_Site`): respawners. Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. respawners has no sequence, so there is no footer.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-03] respawner 180 `Omega Site Gate Plaza` (provenance set by UO-03's block)

