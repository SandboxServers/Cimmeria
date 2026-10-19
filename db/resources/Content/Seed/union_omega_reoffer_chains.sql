--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): re-offer
-- chains (after an abandon). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Omega's id block is 63800-63999; this file's chain ids are 63840-63849. Each packet writes only below its own marker.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-09] chains 63840-63845 (provenance set by UO-09's block)

