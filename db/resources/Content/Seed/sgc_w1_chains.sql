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
VALUES (3001, 'player_loaded', NULL, 'player', false, 0);

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
  (3007, 'play_sequence', 10013, NULL, '{}', 0, 2);

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

-- Chain 3012: objective_status check — show Hammond dialog only if objective 5353 is not yet complete
--   Exercises: objective_status condition type
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3012, 'SGC_W1 - Hammond waypoint dialog (obj 5353 incomplete only)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3012, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3012, 'objective_status', 1559, '5353', 'neq', 'completed', 0);

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
