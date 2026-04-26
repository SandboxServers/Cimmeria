-- Phase 3: SGC_W1 content chains
-- Mission 1559 (principal mission in this space, "Orientation")
-- Exercises: complete_objective, objective_status, set_visible, move_entity,
--            mission_completed, dialog_set_open
--
-- Chain ID range: 3001-3050

SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 1559 — Orientation (SGC_W1 primary mission)
-- ============================================================

-- Chain 3001: player_loaded when 1559 not active → accept + show Gen Hammond
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3001, 'SGC_W1 - Load: accept mission 1559, show Gen Hammond', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3001, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3001, 'mission_status', 1559, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3001, 'accept_mission', 1559, NULL, '{}', 0, 0),
  (3001, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "|", "mask": 16777216}', 0, 1);

-- Chain 3002: interact SGCW1_GenHammond while step 4612 active → show dialog 5354
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3002, 'SGC_W1 - Interact Hammond: show dialog 5354', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3002, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3002, 'step_status', 1559, '4612', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3002, 'display_dialog', 5354, NULL, '{}', 0, 0);

-- Chain 3003: dialog choice 5354 (accept Gen Hammond briefing) → advance to 4613, hide Hammond, show Teal'c
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3003, 'SGC_W1 - Dialog 5354: advance step, swap NPCs', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3003, 'dialog_choice', '5354', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3003, 'advance_step', 1559, '4613', '{}', 0, 0),
  (3003, 'play_sequence', 10005, NULL, '{}', 0, 1),
  (3003, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 16777216}', 0, 2),
  (3003, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "|", "mask": 16777216}', 0, 3);

-- Chain 3004: interact Teal'c while step 4613 active → show dialog 5355
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3004, 'SGC_W1 - Interact Tealc: show dialog 5355', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3004, 'interact_tag', 'SGC_W1_Tealc', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3004, 'step_status', 1559, '4613', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3004, 'display_dialog', 5355, NULL, '{}', 0, 0);

-- Chain 3005: dialog choice 5355 (agree with Teal'c) → advance 4614, play sequence, hide Teal'c
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3005, 'SGC_W1 - Dialog 5355: advance 4614, play sequences', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3005, 'dialog_choice', '5355', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3005, 'advance_step', 1559, '4614', '{}', 0, 0),
  (3005, 'play_sequence', 2319, NULL, '{}', 0, 1),
  (3005, 'play_sequence', 10002, NULL, '{}', 500, 2),
  (3005, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "~", "mask": 16777216}', 0, 3),
  -- Move Gen Hammond to waypoint alongside player (set_visible / move_entity exercises)
  (3005, 'move_entity', NULL, 'SGCW1_GenHammond',
   '{"destination": "-123.625,1.311,-246.858", "world": "SGC_W1"}', 0, 4);

-- Chain 3006: interact elevator button while step 4614 active → complete objective 5353, show dialog 5357
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3006, 'SGC_W1 - Press elevator button: complete objective 5353', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3006, 'interact_tag', 'SGC_W1_ElevatorButton1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3006, 'step_status', 1559, '4614', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3006, 'complete_objective', 1559, '5353', '{}', 0, 0),
  (3006, 'display_dialog', 5357, NULL, '{}', 0, 1);

-- Chain 3007: dialog choice 5357 (move to armory) → move player, complete objective 5358, give item 55
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3007, 'SGC_W1 - Move to armory: complete objective 5358, give item', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3007, 'dialog_choice', '5357', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3007, 'move_entity', NULL, NULL,
   '{"destination": "-123.625,1.311,-246.858", "world": "SGC_W1", "use_player": true}', 0, 0),
  (3007, 'complete_objective', 1559, '5358', '{}', 0, 1),
  (3007, 'set_interaction_type', NULL, 'SGC_W1_FirearmBody',
   '{"op": "|", "mask": 16777216}', 0, 2),
  (3007, 'play_sequence', 10013, NULL, '{}', 0, 3);

-- Chain 3008: interact firearm body when 1559 active → complete mission 1559, give item, show dialog
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3008, 'SGC_W1 - Pick up firearm: complete mission 1559', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3008, 'interact_tag', 'SGC_W1_FirearmBody', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3008, 'mission_status', 1559, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3008, 'complete_mission', 1559, NULL, '{}', 0, 0),
  (3008, 'add_item', 55, NULL, '{"container": 1, "qty": 1}', 0, 1),
  (3008, 'display_dialog', 5358, NULL, '{}', 0, 2);

-- Chain 3009: mission_completed event for 1559 → play cinematic, move airman NPC
--   Exercises: mission_completed event type
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3009, 'SGC_W1 - Mission 1559 completed: play outro', 'mission', 1559, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3009, 'mission_completed', '1559', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3009, 'play_sequence', 10012, NULL, '{}', 0, 0),
  (3009, 'move_entity', NULL, 'SGCW1_AirmanWalking',
   '{"destination": "237.326,1.312,17.921", "world": "SGC_W1"}', 0, 1),
  (3009, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton1',
   '{"op": "|", "mask": 256}', 0, 2);

-- Chain 3010: dialog choice 5358 (post-mission chat) → play sequence
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3010, 'SGC_W1 - Dialog 5358 choice: play outro sequence', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3010, 'dialog_choice', '5358', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3010, 'play_sequence', 10012, NULL, '{}', 0, 0);

-- Chain 3011: dialog choice 5356 (security office interaction) → move airman
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3011, 'SGC_W1 - Dialog 5356: move airman to waypoint', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3011, 'dialog_choice', '5356', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3011, 'move_entity', NULL, 'SGCW1_AirmanWalking',
   '{"destination": "237.326,1.312,17.921", "world": "SGC_W1"}', 0, 0),
  (3011, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton1',
   '{"op": "|", "mask": 256}', 0, 1);

-- Chain 3012: show Hammond waypoint dialog only during step 4614 and while
--   objective 5353 is still incomplete. The original script fires this from a
--   waypoint callback after Teal'c advances the mission, so we gate it to the
--   same step here.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3012, 'SGC_W1 - Hammond waypoint dialog (obj 5353 incomplete only)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3012, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3012, 'step_status', 1559, '4614', 'eq', 'active', 0),
  (3012, 'objective_status', 1559, '5353', 'neq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3012, 'display_dialog', 5356, NULL, '{}', 0, 0);

-- Chain 3013: dialog_set_open event (SecurityOffice dialog set)
--   Exercises: dialog_set_open event type
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3013, 'SGC_W1 - Security office dialog set open: set_visible on door', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3013, 'dialog_set_open', 'SGC_W1_SecurityOffice', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3013, 'set_visible', NULL, 'SGC_W1_SecurityDoor', '{"visible": true}', 0, 0);

-- Resume/re-login hydration: player_loaded only accepts mission 1559 when it is
-- not active, so an in-progress tutorial needs its interaction flags restored
-- from the saved step state when the player logs back in.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3014, 'SGC_W1 - Load active 4612: restore Hammond interaction', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3014, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3014, 'step_status', 1559, '4612', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3014, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
        '{"op": "|", "mask": 16777216}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3015, 'SGC_W1 - Load active 4613: restore Tealc interaction', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3015, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3015, 'step_status', 1559, '4613', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3015, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3015, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "|", "mask": 16777216}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3016, 'SGC_W1 - Load active 4614: restore elevator and firearm interactions', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3016, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3016, 'step_status', 1559, '4614', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3016, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3016, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton1',
   '{"op": "|", "mask": 256}', 0, 1),
  (3016, 'set_interaction_type', NULL, 'SGC_W1_FirearmBody',
   '{"op": "|", "mask": 16777216}', 0, 2);

-- ============================================================
-- MISSIONS 1561/1562 - Security Office continuation
-- ============================================================

-- Chain 3017: killing the bomb-carrying Jaffa opens Hammond radio dialog 5359.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3017, 'SGC_W1 - JaffaBomb death: show dialog 5359', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3017, 'entity_dead_tag', 'SGC_W1_JaffaBomb', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3017, 'display_dialog', 5359, NULL, '{}', 0, 0);

-- Chain 3018: accepting Hammond's radio prompt starts mission 1561.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3018, 'SGC_W1 - Dialog 5359: accept mission 1561', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3018, 'dialog_choice', '5359', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3018, 'mission_status', 1561, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3018, 'accept_mission', 1561, NULL, '{}', 0, 0),
  (3018, 'set_interaction_type', NULL, 'SGCW1_AirmanBody',
   '{"op": "|", "mask": 16777216}', 0, 1);

-- Chain 3019: re-login while mission 1561 step 4620 is active restores the
-- Airman corpse interaction.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3019, 'SGC_W1 - Load active 4620: restore AirmanBody interaction', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3019, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3019, 'step_status', 1561, '4620', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3019, 'set_interaction_type', NULL, 'SGCW1_AirmanBody',
        '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3020: searching the Airman corpse gives the radio and advances to step 4621.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3020, 'SGC_W1 - Interact AirmanBody: recover radio', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3020, 'interact_tag', 'SGCW1_AirmanBody', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3020, 'step_status', 1561, '4620', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3020, 'display_dialog', 5362, NULL, '{}', 0, 0),
  (3020, 'advance_step', 1561, '4621', '{}', 0, 1),
  (3020, 'add_item', 5168, NULL, '{"container": 2, "qty": 1}', 0, 2);

-- Chain 3021: using the radio at step 4621 advances the bomb-defusal objective.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3021, 'SGC_W1 - Use radio step 4621: advance to bomb defusal', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3021, 'item_use', '5168', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3021, 'step_status', 1561, '4621', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3021, 'display_dialog', 5363, NULL, '{}', 0, 0),
  (3021, 'advance_step', 1561, '4622', '{}', 0, 1),
  (3021, 'set_interaction_type', NULL, 'SGC_W1_NaqBomb',
   '{"op": "|", "mask": 256}', 0, 2);

-- Chain 3022: re-login during step 4622 restores the Naquadah bomb interaction.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3022, 'SGC_W1 - Load active 4622: restore NaqBomb interaction', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3022, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3022, 'step_status', 1561, '4622', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3022, 'set_interaction_type', NULL, 'SGC_W1_NaqBomb',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 3023: interacting with the bomb starts Livewire. Victory directly invokes
-- chain 3024, matching SecurityOffice.py's minigame victory callback.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3023, 'SGC_W1 - Interact NaqBomb: start Livewire', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3023, 'interact_tag', 'SGC_W1_NaqBomb', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3023, 'step_status', 1561, '4622', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3023, 'start_minigame', NULL, 'Livewire',
        '{"on_victory_chains": [3024]}', 0, 0);

-- Triggerless direct chain used by the Livewire victory callback.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3024, 'SGC_W1 - Livewire victory: advance mission 1561 to 4623', 'mission', 1561, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3024, 'advance_step', 1561, '4623', '{}', 0, 0);

-- Chain 3025: using the radio after defusing the bomb completes mission 1561.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3025, 'SGC_W1 - Use radio step 4623: complete mission 1561', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3025, 'item_use', '5168', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3025, 'step_status', 1561, '4623', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3025, 'complete_mission', 1561, NULL, '{}', 0, 0),
  (3025, 'display_dialog', 5365, NULL, '{}', 0, 1);

-- Chain 3026: dialog 5365 starts mission 1562 and enables the next elevator.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3026, 'SGC_W1 - Dialog 5365: accept mission 1562', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3026, 'dialog_choice', '5365', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3026, 'accept_mission', 1562, NULL, '{}', 0, 0),
  (3026, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton2',
   '{"op": "|", "mask": 256}', 0, 1);

-- Chain 3027: re-login while mission 1562 step 4624 is active restores ElevatorButton2.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3027, 'SGC_W1 - Load active 4624: restore ElevatorButton2 interaction', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3027, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3027, 'step_status', 1562, '4624', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3027, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton2',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 3028: second elevator moves the player toward Carter's lab and advances
-- mission 1562 to its scripted dead-end step.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3028, 'SGC_W1 - Interact ElevatorButton2: go to Carter lab', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3028, 'interact_tag', 'SGC_W1_ElevatorButton2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3028, 'step_status', 1562, '4624', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3028, 'advance_step', 1562, '4625', '{}', 0, 0),
  (3028, 'move_entity', NULL, NULL,
   '{"destination": "-53.86,1.311,62.136", "world": "SGC_W1", "use_player": true}', 0, 1),
  (3028, 'display_dialog', 5366, NULL, '{}', 0, 2),
  (3028, 'play_sequence', 10009, NULL, '{}', 0, 3);
