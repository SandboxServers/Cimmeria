-- Phase 1: Castle_CellBlock content chains
-- Missions: 622, 638, 639, 640, 641, 680, 681-687
--
-- Bug fixes baked in:
--   Bug 1: 'Preparation_ColMarshr' -> 'Preparation_ColMarsh' (typo in classic script)
--   Bug 2: step 2113 = mission 622, step 2114 = mission 638 (corrected IDs)
--
-- Chain ID ranges (allocated statically to avoid seed-order sensitivity):
--   Mission 622:  chains 1001-1010
--   Mission 638:  chains 1011-1030
--   Mission 639:  chains 1031-1040
--   Mission 640:  chains 1041-1050
--   Mission 641:  chains 1051-1070
--   Mission 680:  chains 1071-1080
--   Missions 681-687: chains 1081-1130

SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 622 — Arm Yourself!
-- ============================================================

-- Chain 1001: zone entry when 622 not yet accepted → accept + show dialog
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1001, '622 - Zone load: accept mission', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1001, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1001, 'mission_status', 622, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1001, 'accept_mission',  622,  NULL, '{}', 0, 0),
  (1001, 'display_dialog',  2982, NULL, '{}', 0, 1),
  (1001, 'add_dialog_set',  5229, NULL, '{"slot": 14, "mission_id": 622}', 0, 2);

-- Chain 1002: zone entry when 622 already completed → play cinematic
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1002, '622 - Zone load: already completed → cinematic', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1002, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1002, 'mission_status', 622, NULL, 'eq', 'completed', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1002, 'play_sequence', 10000, NULL, '{}', 0, 0);

-- Chain 1003: player opens dialog 3995 (body loot) while 622 active → complete mission
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1003, '622 - Dialog 3995 (loot body): complete mission', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1003, 'dialog_open', '3995', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1003, 'mission_status', 622, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1003, 'set_interaction_type', NULL, 'ArmYourself_FrostBody', '{"op": "~", "mask": 4194304}', 0, 0),
  (1003, 'add_item',  55,   NULL, '{"container": 3, "qty": 1}', 0, 1),
  (1003, 'add_item',  3730, NULL, '{"container": 0, "qty": 1}', 0, 2),
  (1003, 'play_sequence', 10000, NULL, '{}', 0, 3),
  (1003, 'complete_mission', 622, NULL, '{}', 0, 4);

-- ============================================================
-- MISSION 638 — Speak to Prisoner 329
-- ============================================================

-- Chain 1011: enter Region2 when 638 not active + archetype != 8 → accept + SCI dialog set
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1011, '638 - Region2 entry: accept (non-scientist)', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1011, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1011, 'mission_status', 638, NULL, 'eq', 'not_active', 0),
  (1011, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1011, 'accept_mission', 638, NULL, '{}', 0, 0),
  (1011, 'add_dialog_set', 2794, NULL, '{"slot": 17}', 0, 1);

-- Chain 1012: enter Region2 when 638 not active + archetype == 8 → accept + scientist dialog set
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1012, '638 - Region2 entry: accept (scientist)', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1012, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1012, 'mission_status', 638, NULL, 'eq', 'not_active', 0),
  (1012, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1012, 'accept_mission', 638, NULL, '{}', 0, 0),
  (1012, 'add_dialog_set', 5866, NULL, '{"slot": 17}', 0, 1);

-- Chain 1013: enter Region2 always → system message 5040
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1013, '638 - Region2 entry: system message', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1013, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1013, 'system_message', 5040, NULL, '{}', 0, 0);

-- Chain 1014: dialog choice 5021 (non-sci talk to 329) while step 2114 active → advance to 2115 + enable door
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1014, '638 - Talk to Prisoner (non-sci): advance step to 2115', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1014, 'dialog_choice', '5021', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1014, 'step_status', 638, '2114', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1014, 'advance_step', 638, '2115', '{}', 0, 0),
  (1014, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "|", "mask": 256}', 0, 1);

-- Chain 1015: dialog choice 2300 (sci talk to 329) while step 2114 active → advance to 2115 + enable door
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1015, '638 - Talk to Prisoner (sci): advance step to 2115', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1015, 'dialog_choice', '2300', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1015, 'step_status', 638, '2114', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1015, 'advance_step', 638, '2115', '{}', 0, 0),
  (1015, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "|", "mask": 256}', 0, 1);

-- Chain 1016: interact with door button while step 2115 active → Livewire minigame
--   on_victory_chains: 1017
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1016, '638 - Door button: start Livewire minigame', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1016, 'interact_tag', '329_CellDoorButton', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1016, 'step_status', 638, '2115', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1016, 'start_minigame', NULL, 'Livewire',
   '{"on_victory_chains": [1017]}', 0, 0);

-- Chain 1017: minigame victory (triggered by 1016 on_victory_chains) → advance step, play sequence, disable button
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1017, '638 - Livewire victory: advance to step 2116', 'mission', 638, true, 0);

-- no trigger row — invoked directly by the minigame callback in chain 1016
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1017, 'advance_step', 638, '2116', '{}', 0, 0),
  (1017, 'play_sequence', 1749, NULL, '{}', 0, 1),
  (1017, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "~", "mask": 256}', 0, 2);

-- Chain 1018: dialog choice 5020 (non-sci agree escape) while step 2116 active → finish 638, accept 639
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1018, '638 - Agree to help (non-sci): complete 638, accept 639', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1018, 'dialog_choice', '5020', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1018, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1018, 'display_dialog', 2298, NULL, '{}', 0, 0),
  (1018, 'accept_mission',  639, NULL, '{}', 0, 1),
  (1018, 'complete_mission', 638, NULL, '{}', 0, 2),
  (1018, 'remove_dialog_set', 2794, NULL, '{"slot": 17}', 0, 3);

-- Chain 1019: dialog choice 2299 (sci agree escape) while step 2116 active → finish 638, accept 639
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1019, '638 - Agree to help (sci): complete 638, accept 639', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1019, 'dialog_choice', '2299', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1019, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1019, 'display_dialog', 2298, NULL, '{}', 0, 0),
  (1019, 'accept_mission',  639, NULL, '{}', 0, 1),
  (1019, 'complete_mission', 638, NULL, '{}', 0, 2),
  (1019, 'remove_dialog_set', 5866, NULL, '{"slot": 17}', 0, 3);

-- ============================================================
-- MISSION 639 — Find Ambernol
-- ============================================================

-- Chain 1031: enter Region11 while step 2117 active → advance to 2145
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1031, '639 - Enter Region11: advance step to 2145', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1031, 'enter_region', 'Castle_Cellblock.Region11', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1031, 'step_status', 639, '2117', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1031, 'advance_step', 639, '2145', '{}', 0, 0);

-- Chain 1032: interact with Ambernol vial while step 2145 active → pick up, destroy, aggro guard, advance
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1032, '639 - Interact vial: pick up ambernol, trigger guard', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1032, 'interact_tag', 'ArmYourself_AmbernolVial', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1032, 'step_status', 639, '2145', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1032, 'add_item',  19, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (1032, 'destroy_entity', NULL, 'ArmYourself_AmbernolVial', '{}', 0, 1),
  (1032, 'set_aggression', NULL, 'ArmYourself_PrisonerRetrievalUnit', '{"level": 1}', 0, 2),
  (1032, 'play_sequence', 10001, NULL, '{}', 0, 3),
  (1032, 'advance_step', 639, '2344', '{}', 0, 4);

-- Chain 1033: entity dead tag for guard (space-scoped) while step 2344 active → advance to 2343
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1033, '639 - Guard killed: advance step to 2343', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1033, 'entity_dead_tag', 'ArmYourself_PrisonerRetrievalUnit', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1033, 'step_status', 639, '2344', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1033, 'advance_step', 639, '2343', '{}', 0, 0);

-- Chain 1034: use item 19 (ambernol) while step 2343 active → remove item, complete 639, accept 640
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1034, '639 - Use ambernol: complete 639, accept 640', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1034, 'item_use', '19', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1034, 'step_status', 639, '2343', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1034, 'remove_item',    19, NULL, '{"qty": 1}', 0, 0),
  (1034, 'complete_mission', 639, NULL, '{}', 0, 1),
  (1034, 'accept_mission',   640, NULL, '{}', 0, 2);

-- ============================================================
-- MISSION 640 — Hack the Rings
-- ============================================================

-- Chain 1041: interact with ring switch while step 2120 active → Livewire
--   on_victory_chains: 1042
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1041, '640 - Ring switch: start Livewire', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1041, 'interact_tag', 'HackTheRings_Switch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1041, 'step_status', 640, '2120', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1041, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1042]}', 0, 0);

-- Chain 1042: Livewire victory → advance step to 2215
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1042, '640 - Livewire victory: advance to 2215', 'mission', 640, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1042, 'advance_step', 640, '2215', '{}', 0, 0);

-- Chain 1043: interact with ring switch while step 2215 active → trigger transporter (regionId=1)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1043, '640 - Ring switch (post-hack): trigger transport to regionId 1', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1043, 'interact_tag', 'HackTheRings_Switch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1043, 'step_status', 640, '2215', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1043, 'trigger_transporter', NULL, NULL, '{"regionId": 1}', 0, 0);

-- Chain 1044: teleport_in regionId=2 while 640 active → complete 640, show ColMarsh indicator
--   BUG FIX: this correctly sets the 'Preparation_ColMarsh' tag (no trailing 'r')
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1044, '640 - Teleport to ringside: complete 640, enable ColMarsh', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1044, 'teleport_in', '2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1044, 'mission_status', 640, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1044, 'complete_mission', 640, NULL, '{}', 0, 0),
  -- BUG FIX: correct tag is 'Preparation_ColMarsh' (classic script had 'Preparation_ColMarshr')
  (1044, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "|", "mask": 8388608}', 0, 1);

-- ============================================================
-- MISSION 641 — Preparation
-- BUG FIX: subscribe on correct tag 'Preparation_ColMarsh' not 'Preparation_ColMarshr'
-- ============================================================

-- Chain 1051: interact ColMarsh while step 2121 not active + archetype != 8 (non-sci) → display dialog 4001
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1051, '641 - Interact ColMarsh (non-sci): show dialog 4001', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1051, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1051, 'step_status', 641, '2121', 'eq', 'not_active', 0),
  (1051, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1051, 'display_dialog', 4001, NULL, '{}', 0, 0);

-- Chain 1052: interact ColMarsh while step 2121 not active + archetype == 8 (sci) → display dialog 5022
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1052, '641 - Interact ColMarsh (sci): show dialog 5022', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1052, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1052, 'step_status', 641, '2121', 'eq', 'not_active', 0),
  (1052, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1052, 'display_dialog', 5022, NULL, '{}', 0, 0);

-- Chain 1053: dialog choice 4001 (non-sci accepts briefing) → accept mission 641
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1053, '641 - Dialog choice 4001: accept mission', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1053, 'dialog_choice', '4001', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1053, 'accept_mission', 641, NULL, '{}', 0, 0);

-- Chain 1054: dialog choice 5022 (sci accepts briefing) → accept mission 641
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1054, '641 - Dialog choice 5022: accept mission', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1054, 'dialog_choice', '5022', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1054, 'accept_mission', 641, NULL, '{}', 0, 0);

-- Chain 1055: interact SMG1A while step 2121 active → give weapon, advance to step 3563
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1055, '641 - Pick up SMG1A: give weapon, advance step', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1055, 'interact_tag', 'Preparation_SMG1A', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1055, 'step_status', 641, '2121', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1055, 'add_item',    21, NULL, '{"container": 3, "qty": 1}', 0, 0),
  (1055, 'advance_step', 641, '3563', '{}', 0, 1);

-- Chain 1056: interact template 'Col Marsh (pet)' while step 3563 active + non-sci → dialog 3999
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1056, '641 - Talk to Col Marsh pet (non-sci): show dialog 3999', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1056, 'interact_template', 'Col Marsh (pet)', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1056, 'step_status', 641, '3563', 'eq', 'active', 0),
  (1056, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1056, 'display_dialog', 3999, NULL, '{}', 0, 0);

-- Chain 1057: interact template 'Col Marsh (pet)' while step 3563 active + sci → dialog 5023
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1057, '641 - Talk to Col Marsh pet (sci): show dialog 5023', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1057, 'interact_template', 'Col Marsh (pet)', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1057, 'step_status', 641, '3563', 'eq', 'active', 0),
  (1057, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1057, 'display_dialog', 5023, NULL, '{}', 0, 0);

-- Chain 1058: dialog choice 3999 (non-sci briefed) → advance step 3564
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1058, '641 - Dialog choice 3999: advance step 3564', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1058, 'dialog_choice', '3999', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1058, 'advance_step', 641, '3564', '{}', 0, 0);

-- Chain 1059: dialog choice 5023 (sci briefed) → advance step 3564
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1059, '641 - Dialog choice 5023: advance step 3564', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1059, 'dialog_choice', '5023', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1059, 'advance_step', 641, '3564', '{}', 0, 0);

-- Chain 1060: interact terminal while step 3564 active → Livewire
--   on_victory_chains: 1061
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1060, '641 - Terminal: start Livewire', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1060, 'interact_tag', 'Preparation_Terminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1060, 'step_status', 641, '3564', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1060, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1061]}', 0, 0);

-- Chain 1061: Livewire victory → display final dialog, complete 641, accept 680
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1061, '641 - Livewire victory: complete 641, accept 680', 'mission', 641, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1061, 'display_dialog', 3998, NULL, '{}', 0, 0),
  (1061, 'complete_mission', 641, NULL, '{}', 0, 1),
  (1061, 'accept_mission',   680, NULL, '{}', 0, 2);

-- ============================================================
-- MISSION 680 — Escape the Cellblock
-- ============================================================

-- Chain 1071: interact ring switch while step 2344 active → trigger transporter to regionId=2
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1071, '680 - Ring switch: trigger transport (region 2)', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1071, 'interact_tag', 'Preparation_RingSwitch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1071, 'step_status', 680, '2344', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1071, 'trigger_transporter', NULL, NULL, '{"regionId": 2}', 0, 0);

-- Chain 1072: teleport_in regionId=3 → advance step to 2345
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1072, '680 - Teleport to topside: advance step 2345', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1072, 'teleport_in', '3', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1072, 'advance_step', 680, '2345', '{}', 0, 0);

-- Chain 1073: enter Region9 when 681 not yet active → accept mission 681
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1073, '680 - Enter Region9: accept 681', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1073, 'enter_region', 'Castle_CellBlock.Region9', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1073, 'mission_status', 681, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1073, 'accept_mission', 681, NULL, '{}', 0, 0);

-- ============================================================
-- MISSIONS 681-687 — Hallway Controller chain
-- Counter-based kill tracking for mess hall
-- ============================================================

-- Chain 1081: enter Region3 (exit) when 681 completed and 682 not active → accept 682
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1081, '681→682 - Leave Region3: accept 682', 'mission', 681, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1081, 'exit_region', 'Castle_Cellblock.Region3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1081, 'mission_status', 681, NULL, 'eq', 'completed', 0),
  (1081, 'mission_status', 682, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1081, 'accept_mission', 682, NULL, '{}', 0, 0);

-- Chain 1082: enter Region4 when 683 completed and 684 not active → accept 684
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1082, '683→684 - Enter Region4: accept 684', 'mission', 683, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1082, 'enter_region', 'Castle_Cellblock.Region4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1082, 'mission_status', 683, NULL, 'eq', 'completed', 0),
  (1082, 'mission_status', 684, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1082, 'accept_mission', 684, NULL, '{}', 0, 0);

-- Chain 1083: enter Region5 when 685 completed and 686 not active → accept 686
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1083, '685→686 - Enter Region5: accept 686', 'mission', 685, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1083, 'enter_region', 'Castle_Cellblock.Region5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1083, 'mission_status', 685, NULL, 'eq', 'completed', 0),
  (1083, 'mission_status', 686, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1083, 'accept_mission', 686, NULL, '{}', 0, 0);

-- Chain 1084: enter Region6 when 686 completed and 687 not active → accept 687
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1084, '686→687 - Enter Region6: accept 687', 'mission', 686, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1084, 'enter_region', 'Castle_Cellblock.Region6', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1084, 'mission_status', 686, NULL, 'eq', 'completed', 0),
  (1084, 'mission_status', 687, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1084, 'accept_mission', 687, NULL, '{}', 0, 0);

-- Chain 1085: mess hall counter — entity_dead (Hallway01 guard tag) increments kill counter
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1085, '681 - Kill guard in hallway 01: increment kill counter', 'mission', 681, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1085, 'entity_dead_tag', 'Hallway01_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1085, 'mission_status', 681, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1085, 'increment_counter', NULL, 'hallway01_kills', '{"amount": 1}', 0, 0);

-- Content counters for hallway kill tracking
INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on)
VALUES (1085, 'hallway01_kills', 3, 'mission_complete');

-- Chain 1086: hallway01_kills == 3 → complete mission 681
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1086, '681 - Kill counter reached: complete 681', 'mission', 681, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1086, 'entity_dead_tag', 'Hallway01_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1086, 'mission_status', 681, NULL, 'eq', 'active', 0),
  (1086, 'counter', NULL, 'hallway01_kills', 'gte', '3', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1086, 'complete_mission', 681, NULL, '{}', 0, 0),
  (1086, 'reset_counter', NULL, 'hallway01_kills', '{}', 0, 1);

-- Chain 1087: mess hall (682) guard kill counter
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1087, '682 - Kill guard in hallway 02: increment kill counter', 'mission', 682, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1087, 'entity_dead_tag', 'Hallway02_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1087, 'mission_status', 682, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1087, 'increment_counter', NULL, 'hallway02_kills', '{"amount": 1}', 0, 0);

INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on)
VALUES (1087, 'hallway02_kills', 3, 'mission_complete');

-- Chain 1088: hallway02_kills == 3 → complete mission 682, accept 683
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1088, '682 - Kill counter reached: complete 682, accept 683', 'mission', 682, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1088, 'entity_dead_tag', 'Hallway02_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1088, 'mission_status', 682, NULL, 'eq', 'active', 0),
  (1088, 'counter', NULL, 'hallway02_kills', 'gte', '3', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1088, 'complete_mission', 682, NULL, '{}', 0, 0),
  (1088, 'accept_mission',   683, NULL, '{}', 0, 1),
  (1088, 'reset_counter', NULL, 'hallway02_kills', '{}', 0, 2);

-- Chain 1089: hallway03 kills → complete 683 (similar pattern, condensed)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1089, '683 - Kill counter reached: complete 683, accept 684', 'mission', 683, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1089, 'entity_dead_tag', 'Hallway03_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1089, 'mission_status', 683, NULL, 'eq', 'active', 0),
  (1089, 'counter', NULL, 'hallway03_kills', 'gte', '3', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1089, 'complete_mission', 683, NULL, '{}', 0, 0),
  (1089, 'accept_mission',   684, NULL, '{}', 0, 1),
  (1089, 'reset_counter', NULL, 'hallway03_kills', '{}', 0, 2);

-- Chain 1090: hallway04 kills → complete 684, accept 685
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1090, '684 - Kill counter reached: complete 684, accept 685', 'mission', 684, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1090, 'entity_dead_tag', 'Hallway04_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1090, 'mission_status', 684, NULL, 'eq', 'active', 0),
  (1090, 'counter', NULL, 'hallway04_kills', 'gte', '3', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1090, 'complete_mission', 684, NULL, '{}', 0, 0),
  (1090, 'accept_mission',   685, NULL, '{}', 0, 1),
  (1090, 'reset_counter', NULL, 'hallway04_kills', '{}', 0, 2);

-- Chain 1091: hallway05 kills → complete 685, accept 686
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1091, '685 - Kill counter reached: complete 685, accept 686', 'mission', 685, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1091, 'entity_dead_tag', 'Hallway05_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1091, 'mission_status', 685, NULL, 'eq', 'active', 0),
  (1091, 'counter', NULL, 'hallway05_kills', 'gte', '3', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1091, 'complete_mission', 685, NULL, '{}', 0, 0),
  (1091, 'accept_mission',   686, NULL, '{}', 0, 1),
  (1091, 'reset_counter', NULL, 'hallway05_kills', '{}', 0, 2);

-- Chain 1092: final hallway kills → complete 686, accept 687
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1092, '686 - Kill counter reached: complete 686, accept 687', 'mission', 686, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1092, 'entity_dead_tag', 'MessHall_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1092, 'mission_status', 686, NULL, 'eq', 'active', 0),
  (1092, 'counter', NULL, 'messhall_kills', 'gte', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1092, 'complete_mission', 686, NULL, '{}', 0, 0),
  (1092, 'accept_mission',   687, NULL, '{}', 0, 1),
  (1092, 'reset_counter', NULL, 'messhall_kills', '{}', 0, 2);
