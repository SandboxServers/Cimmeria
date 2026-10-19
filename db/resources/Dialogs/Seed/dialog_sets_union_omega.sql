--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): dialog
-- set and dialog-set maps. Chains bind these maps with add_dialog_set.
-- Campaign: docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Id block 63800-63999 (Omega's reservation, D-UO1). Each packet writes only
-- below its own marker. Dialog sequences already stand above 100000, so this
-- file has no sequence footer.
--
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-05] dialog set 63800 and maps 63800-63803 (provenance set by UO-05's block)
-- UO-05 (D-UO6): CIMMERIA_RECONSTRUCTION offer set 63800 for the Command Center cast. Dialogs 4641 (Jackson) and 5025 (Bra'tac) are RESTORED_SOURCE.
-- The stock set 1477 is not used: its join is inferred and its flags are 0 (F13).
-- Dialog set 63800: the Command Center offers.
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (63800, 'Omega Command Center offers');

-- 63800: Jackson delivery, flag 16777216 (INT_A_STORY_MISSION_ACTIVE, "!").
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (63800, 63800, 4641, 'The SGU', 16777216, 1, '{}', '{}', '{}', '{}');
-- 63801: Bra'tac delivery, flag 16777216 ("!").
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (63801, 63800, 5025, 'The SGU', 16777216, 1, '{}', '{}', '{}', '{}');
-- 63802: Jackson re-offer, flag 8388608 (INT_A_STORY_MISSION_AVAILABLE, "?").
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (63802, 63800, 4641, 'The SGU', 8388608, 1, '{}', '{}', '{}', '{}');
-- 63803: Bra'tac re-offer, flag 8388608 ("?").
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (63803, 63800, 5025, 'The SGU', 8388608, 1, '{}', '{}', '{}', '{}');

