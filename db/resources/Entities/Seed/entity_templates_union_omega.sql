--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): entity
-- templates. Campaign: docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. Provenance: each block states its own tag
-- (RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-05] Command Center cast templates 63800-63803 (CIMMERIA_RECONSTRUCTION; clones of 51, 59, 30, 64)

-- [UO-10] banker 63810, registrars 63811 (Team) and 63830 (Command) (provenance set by UO-10's block)

-- [UO-11] Tau'ri vendors 63812-63816 (provenance set by UO-11's block)

-- [UO-12] Jaffa vendors 63817-63819 (provenance set by UO-12's block)

-- [UO-14] ambient 63860-63899 (CIMMERIA_AUTHORING, droppable)

--
-- The rows above load after entity_templates.sql's own sequence footer, so
-- they raise the sequence themselves. Floor 63999 is the top of Omega's block,
-- so a `.savespawn` or GM-made template never takes a reserved id.
SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 63999), true);
