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
--
-- Dialog ID convention used throughout Mission 638 (and the rest of
-- this seed):
--   2xxx / 3xxx prefix → Tau'ri / Human dialogs.
--   5xxx prefix        → Jaffa dialogs.
--
-- For Mission 638 specifically, the per-archetype mapping is:
--
--   Topic / step                        Human         Jaffa
--   "Free Prisoner 329" topic dialog    2300          5021
--   "Agree to escape" follow-up dialog  2299          5020
--   dialog_set_map row (set_id 603)     2794          5866
--
-- Chains 1011/1012 add the *correct* dialog_set_map row to slot 17 so
-- the prisoner only offers the topic dialog matching the player's
-- archetype. Chains 1018/1019 remove the same marker after the player
-- commits to the escape, so re-talking to the prisoner doesn't re-offer
-- the topic.
--
-- A swap here is a user-visible bug (a Tau'ri player gets the
-- "My symbiote will cure me" Jaffa dialog and vice versa) — issue #216.
-- The chain-replay tests in
-- `crates/services/src/cell/content/chain_replay_tests.rs`
-- (`mission_638_*` group) regression-guard the resolved-action shape
-- for both archetype branches.

-- Chain 1011: enter Region2 when 638 not active + non-Jaffa → accept + Human dialog set
-- dialog_set_map 2794 (set_id 603) → dialog 2300 (Human "Free Prisoner 329")
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1011, '638 - Region2 entry: accept (non-Jaffa)', 'mission', 638, true, 0);

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

-- Chain 1012: enter Region2 when 638 not active + Jaffa → accept + Jaffa dialog set
-- dialog_set_map 5866 (set_id 603) → dialog 5021 (Jaffa "Free Prisoner 329", mentions symbiote)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1012, '638 - Region2 entry: accept (Jaffa)', 'mission', 638, true, 0);

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

-- Chain 1014: dialog choice 5021 (Jaffa talk to 329) while step 2114 active → advance to 2115 + enable door
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1014, '638 - Talk to Prisoner (Jaffa): advance step to 2115', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1014, 'dialog_choice', '5021', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1014, 'step_status', 638, '2114', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1014, 'advance_step', 638, '2115', '{}', 0, 0),
  (1014, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "|", "mask": 256}', 0, 1);

-- Chain 1015: dialog choice 2300 (Human talk to 329) while step 2114 active → advance to 2115 + enable door
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1015, '638 - Talk to Prisoner (Human): advance step to 2115', 'mission', 638, true, 0);

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

-- Chain 1020: dialog choice 5021 (Jaffa) while step 2116 active → show follow-up dialog 5020
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1020, '638 - Post-minigame (Jaffa): show escape dialog', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1020, 'dialog_choice', '5021', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1020, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1020, 'display_dialog', 5020, NULL, '{}', 0, 0);

-- Chain 1021: dialog choice 2300 (Human) while step 2116 active → show follow-up dialog 5020
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1021, '638 - Post-minigame (Human): show escape dialog', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1021, 'dialog_choice', '2300', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1021, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1021, 'display_dialog', 5020, NULL, '{}', 0, 0);

-- Chain 1018: dialog choice 5020 (Jaffa agree escape) while step 2116 active → finish 638, accept 639
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1018, '638 - Agree to help (Jaffa): complete 638, accept 639', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1018, 'dialog_choice', '5020', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1018, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1018, 'display_dialog', 2298, NULL, '{}', 0, 0),
  (1018, 'accept_mission',  639, NULL, '{}', 0, 1),
  (1018, 'complete_mission', 638, NULL, '{}', 0, 2),
  (1018, 'remove_dialog_set', 5866, NULL, '{"slot": 17}', 0, 3);

-- Chain 1019: dialog choice 2299 (Human agree escape) while step 2116 active → finish 638, accept 639
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1019, '638 - Agree to help (Human): complete 638, accept 639', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1019, 'dialog_choice', '2299', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1019, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1019, 'display_dialog', 2298, NULL, '{}', 0, 0),
  (1019, 'accept_mission',  639, NULL, '{}', 0, 1),
  (1019, 'complete_mission', 638, NULL, '{}', 0, 2),
  (1019, 'remove_dialog_set', 2794, NULL, '{"slot": 17}', 0, 3);

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
VALUES
  (1031, 'advance_step', 639, '2145', '{}', 0, 0),
  (1031, 'set_interaction_type', NULL, 'ArmYourself_AmbernolVial', '{"op": "|", "mask": 1073741824}', 0, 1);

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
  (1032, 'advance_step', 639, '2144', '{}', 0, 4);

-- Chain 1033: entity dead tag for guard (space-scoped) while step 2144 active → advance to 2343
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1033, '639 - Guard killed: advance step to 2343', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1033, 'entity_dead_tag', 'ArmYourself_PrisonerRetrievalUnit', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1033, 'step_status', 639, '2144', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1033, 'advance_step', 639, '2343', '{}', 0, 0);

-- Chain 1034: use item 19 (ambernol) while step 2343 active → complete 639, accept 640.
--   The chain is responsible for consumption via `remove_item`. Mirrors python
--   `FindAmbernol.py:115` which calls `inventory.removeItemByDesign(19, 1, False)`
--   from a per-mission script callback — pure `useItem` events fire without
--   consuming, and per-item handlers decide whether to remove the stack.
--   The previous global consume-on-use behavior was wrong for reusable items
--   (radios, multi-step "use on target" objectives) which silently lost stacks
--   on every use.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1034, '639 - Use ambernol: complete 639, accept 640', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1034, 'item_use', '19', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1034, 'step_status', 639, '2343', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Consume the vial first — the step-status condition is locked in by the
  -- time the chain fires, so subsequent actions can't double-trigger off the
  -- removal's wire updates.
  (1034, 'remove_item',      19,  NULL, '{"qty": 1}', 0, 0),
  (1034, 'complete_mission', 639, NULL, '{}',         0, 1),
  (1034, 'accept_mission',   640, NULL, '{}',         0, 2),
  -- Make the ring switch right-clickable with the Livewire icon so the player can hack it.
  -- The classic Python script (HackTheRings.py) didn't set this bit; the original Atrea editor
  -- presumably wired it implicitly off the Event_EntityInteract node. We do it explicitly here.
  (1034, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 256}', 0, 3);

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

-- Chain 1042: Livewire victory → advance step to 2215, swap switch icon Livewire → RingNetwork
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1042, '640 - Livewire victory: advance to 2215', 'mission', 640, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1042, 'advance_step', 640, '2215', '{}', 0, 0),
  (1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "~", "mask": 256}', 0, 1),
  (1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 32}', 0, 2);

-- Chain 1043: interact with ring switch while step 2215 active → trigger transporter (regionId=1)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1043, '640 - Ring switch (post-hack): trigger transport to regionId 1', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1043, 'interact_tag', 'HackTheRings_Switch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1043, 'step_status', 640, '2215', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1043, 'trigger_transporter', NULL, NULL, '{"regionId": 1}', 0, 0);

-- Chain 1045: player_loaded + step 2120 active → restore Livewire bit on the switch.
--   Needed because interaction_type flags are NOT persisted on the entity across server restarts.
--   Without this, a player who logged out mid-mission-640 would find the switch unclickable.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1045, '640 - Restore Livewire bit on login (step 2120)', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1045, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1045, 'step_status', 640, '2120', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1045, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 1046: player_loaded + step 2215 active → restore RingNetwork bit on the switch.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1046, '640 - Restore RingNetwork bit on login (step 2215)', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1046, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1046, 'step_status', 640, '2215', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1046, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "|", "mask": 32}', 0, 0);

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
   '{"op": "|", "mask": 8388608}', 0, 1),
  -- Clear RingNetwork bit set by chain 1042 once we've used the rings.
  (1044, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "~", "mask": 32}', 0, 2);

-- ============================================================
-- MISSION 641 — Preparation
-- BUG FIX: subscribe on correct tag 'Preparation_ColMarsh' not 'Preparation_ColMarshr'
-- ============================================================

-- Chain 1051: interact ColMarsh while mission 641 not yet accepted + archetype != 8 (non-sci) → display dialog 4001
--   Gating on `mission_status 641 eq not_active` rather than `step_status 641 2121 eq not_active`:
--   step 2121 is also "not_active" once the player has advanced past it (e.g. picked up the P90 and
--   is on step 3563), which used to make this chain re-fire the briefing dialog and let chain 1053
--   re-accept the mission, resetting progress to step 2121 in a loop.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1051, '641 - Interact ColMarsh (non-sci): show dialog 4001', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1051, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1051, 'mission_status', 641, NULL, 'eq', 'not_active', 0),
  (1051, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1051, 'display_dialog', 4001, NULL, '{}', 0, 0);

-- Chain 1052: interact ColMarsh while mission 641 not yet accepted + archetype == 8 (sci) → display dialog 5022
--   See chain 1051 for why this gates on mission_status rather than step_status.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1052, '641 - Interact ColMarsh (sci): show dialog 5022', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1052, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1052, 'mission_status', 641, NULL, 'eq', 'not_active', 0),
  (1052, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1052, 'display_dialog', 5022, NULL, '{}', 0, 0);

-- Chain 1053: dialog choice 4001 (non-sci accepts briefing) → accept mission 641, highlight P90 locker
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1053, '641 - Dialog choice 4001: accept mission', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1053, 'dialog_choice', '4001', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1053, 'accept_mission', 641, NULL, '{}', 0, 0),
  -- Mark the P90 locker as a quest world object so the client renders the
  -- attention-drawing highlight while step 2121 is active. Cleared in chain
  -- 1055 when the player picks the P90 up; restored on login by chain 1062.
  (1053, 'set_interaction_type', NULL, 'Preparation_SMG1A',
   '{"op": "|", "mask": 1073741824}', 0, 1),
  -- Clear ColMarsh's mission-available bit (set by chain 1044 on
  -- mission-640-complete). Once mission 641 is accepted, the briefing
  -- dialog has been delivered and the icon should clear so it doesn't
  -- linger as a stale "talk to me" indicator. Restored on a server
  -- restart by chain 1063 (gated on 640-complete + 641-not_active).
  (1053, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 2);

-- Chain 1054: dialog choice 5022 (sci accepts briefing) → accept mission 641, highlight P90 locker
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1054, '641 - Dialog choice 5022: accept mission', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1054, 'dialog_choice', '5022', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1054, 'accept_mission', 641, NULL, '{}', 0, 0),
  -- See chain 1053 for the highlight + ColMarsh-clear rationale.
  (1054, 'set_interaction_type', NULL, 'Preparation_SMG1A',
   '{"op": "|", "mask": 1073741824}', 0, 1),
  (1054, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 2);

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
  (1055, 'advance_step', 641, '3563', '{}', 0, 1),
  -- Clear the quest-item highlight on the locker the moment the P90 is
  -- grabbed. Mirrors how chain 1042 toggles HackTheRings_Switch off the
  -- minigame icon as soon as the gating action completes.
  (1055, 'set_interaction_type', NULL, 'Preparation_SMG1A',
   '{"op": "~", "mask": 1073741824}', 0, 2),
  -- Re-set ColMarsh's mission-available marker so the player has a
  -- visual cue to return to him for step 3563 ("Speak to Col. Marsh").
  -- Cleared again by chain 1058/1059 once the briefing completes;
  -- restored on login by chain 1064 if the player logs out at 3563.
  (1055, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "|", "mask": 8388608}', 0, 3);

-- Chain 1062: player_loaded + step 2121 active → restore P90 locker highlight on login.
--   Same restart-resilience pattern as chains 1045/1046 for HackTheRings_Switch:
--   interaction_type flags are not persisted on the entity across server restarts,
--   so a player who logged out mid-mission-641 would otherwise find the locker
--   without the quest-item highlight that chain 1053/1054 originally set.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1062, '641 - Restore SMG1A highlight on login (step 2121)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1062, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1062, 'step_status', 641, '2121', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1062, 'set_interaction_type', NULL, 'Preparation_SMG1A',
        '{"op": "|", "mask": 1073741824}', 0, 0);

-- Chain 1063: player_loaded + 640 completed + 641 not_active → restore ColMarsh
-- mission-available bit on login. Chain 1044 sets the bit with `once: true`
-- when the player teleports in post-rings, so a player who logs out at the
-- "talk to ColMarsh to start mission 641" boundary loses the icon on restart
-- (interaction_type flags don't persist on the entity across server restarts).
-- Same restart-resilience pattern as chain 1062 (SMG1A) and chains 1045/1046
-- (HackTheRings_Switch).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1063, '641 - Restore ColMarsh mission-available bit on login (640 done, 641 not started)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1063, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1063, 'mission_status', 640, NULL, 'eq', 'completed', 0),
  (1063, 'mission_status', 641, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1063, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
        '{"op": "|", "mask": 8388608}', 0, 0);

-- Chain 1064: player_loaded + step 3563 active → restore ColMarsh's
-- "talk to me" marker (re-set after P90 pickup by chain 1055). Without this,
-- a player who logs out at step 3563 boundary would log back in to a Marsh
-- with no visual cue that they need to interact with him for the briefing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1064, '641 - Restore ColMarsh marker on login (step 3563)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1064, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1064, 'step_status', 641, '3563', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1064, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
        '{"op": "|", "mask": 8388608}', 0, 0);

-- Chain 1065: player_loaded + step 3564 active → restore Terminal quest
-- highlight (originally set by chain 1058/1059 on dialog accept). Mirrors
-- chains 1062/1064 for SMG1A and ColMarsh respectively.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1065, '641 - Restore Terminal quest highlight on login (step 3564)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1065, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1065, 'step_status', 641, '3564', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1065, 'set_interaction_type', NULL, 'Preparation_Terminal',
        '{"op": "|", "mask": 1073741824}', 0, 0);

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

-- Chain 1058: dialog choice 3999 (non-sci briefed) → advance step 3564,
-- swap quest markers from ColMarsh to the Terminal he's sending us to.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1058, '641 - Dialog choice 3999: advance step 3564', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1058, 'dialog_choice', '3999', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1058, 'advance_step', 641, '3564', '{}', 0, 0),
  -- Briefing's done, clear ColMarsh's "talk to me" marker.
  (1058, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 1),
  -- Highlight the Terminal so the player has the same visual cue we
  -- already give for the P90 locker / Ambernol vial. Cleared by chain
  -- 1060 when they interact with it; restored on login by chain 1065.
  (1058, 'set_interaction_type', NULL, 'Preparation_Terminal',
   '{"op": "|", "mask": 1073741824}', 0, 2);

-- Chain 1059: dialog choice 5023 (sci briefed) → advance step 3564.
-- Same marker churn as 1058; see that chain for the rationale.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1059, '641 - Dialog choice 5023: advance step 3564', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1059, 'dialog_choice', '5023', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1059, 'advance_step', 641, '3564', '{}', 0, 0),
  (1059, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 1),
  (1059, 'set_interaction_type', NULL, 'Preparation_Terminal',
   '{"op": "|", "mask": 1073741824}', 0, 2);

-- Chain 1060: interact terminal while step 3564 active → Livewire
--   on_victory_chains: 1061
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1060, '641 - Terminal: start Livewire', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1060, 'interact_tag', 'Preparation_Terminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1060, 'step_status', 641, '3564', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1060, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1061]}', 0, 0),
  -- Clear the Terminal quest marker the moment the minigame starts;
  -- player has reached and engaged with it, no need to keep highlighting it.
  (1060, 'set_interaction_type', NULL, 'Preparation_Terminal',
   '{"op": "~", "mask": 1073741824}', 0, 1);

-- Chain 1061: Livewire victory → display final dialog, complete 641, accept 680
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1061, '641 - Livewire victory: complete 641, accept 680', 'mission', 641, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1061, 'display_dialog', 3998, NULL, '{}', 0, 0),
  (1061, 'complete_mission', 641, NULL, '{}', 0, 1),
  (1061, 'accept_mission',   680, NULL, '{}', 0, 2),
  -- Make the second ring switch right-clickable with the RingNetwork icon. Same gap as 640:
  -- EscapeTheCellblock.py subscribes to the interact tag but never sets interactionType.
  (1061, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 32}', 0, 3),
  -- Highlight the ring switch as the next quest objective so the player
  -- has a visual cue. Cleared on teleport_in (chain 1072) when they
  -- actually use it; restored on login by chain 1074.
  (1061, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 1073741824}', 0, 4);

-- ============================================================
-- MISSION 680 — Escape the Cellblock
-- ============================================================

-- Chain 1074: player_loaded + step 2344 active → restore both the RingNetwork
-- icon AND the quest highlight on the second switch.
--   Same restart-resilience pattern as chains 1045/1046 for the first ring switch.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1074, '680 - Restore RingSwitch bits on login (step 2344)', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1074, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1074, 'step_status', 680, '2344', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1074, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 32}', 0, 0),
  -- Restore the quest highlight too (originally set by chain 1061 on
  -- mission-680 acceptance).
  (1074, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 1073741824}', 0, 1);

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
VALUES
  (1072, 'advance_step', 680, '2345', '{}', 0, 0),
  (1072, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "~", "mask": 32}', 0, 1),
  -- Clear the quest highlight too — player has used the rings, no
  -- longer need the "go here" indicator. Pairs with the set in chain 1061.
  (1072, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "~", "mask": 1073741824}', 0, 2);

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
