--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): the
-- holder dialog for Omega's authored bark lines. Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-42).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. Dialog sequences already stand above 100000, so this
-- file has no sequence footer.
--
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-24] dialog 63900, the holder of Omega's bark lines (CIMMERIA_AUTHORING). It is never
-- displayed: npc_bark reads a screen's text and speaks it as a chat line. DUIST_None, no buttons.
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (63900, 0, NULL, 'DUIST_None', NULL, NULL, 'Union Omega authored barks (never displayed)');

-- [UO-33] begin: medic line, screen 63905
-- [UO-33] end

-- [UO-35] begin: guide lines, screens 63901-63904
-- [UO-35] end

-- [UO-36] begin: inn and battlemaster lines, screens 63906-63907
-- [UO-36] end
