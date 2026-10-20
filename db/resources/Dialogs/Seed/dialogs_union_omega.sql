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
-- CIMMERIA_AUTHORING (UO-15; H-LBL: no visible label)
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63901, 'Three vendors sell Biomedical Components: Human Biomedical Components in the SGC wing, Common Biomedical Components in the Common Area, Jaffa Biomedical Components in the Free Jaffa wing.', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63902, 'Three vendors sell Electronics Components: Human Electronics Components in the SGC wing, Common Electronics Components in the Common Area, Jaffa Electronics Components in the Free Jaffa wing.', 0, 1);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63903, 'Three vendors sell Power Systems Components: Human Power Systems Components in the SGC wing, Common Power Systems Components in the Common Area, Jaffa Power Systems Components in the Free Jaffa wing.', 0, 2);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63904, 'Three vendors sell Materials Components: Human Materials Components in the SGC wing, Common Materials Components in the Common Area, Jaffa Materials Components in the Free Jaffa wing.', 0, 3);
-- [UO-35] end

-- [UO-36] begin: inn and battlemaster lines, screens 63906-63907
-- [UO-36] end
