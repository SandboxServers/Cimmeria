--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): arrival
-- chains (the Command Center beam, Modi). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (UO-).
--
-- Chain ID range: 63801-63819. Each packet writes only below its own marker.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-04] chains 63801, 63810, 63811 (provenance set by UO-04's block)

