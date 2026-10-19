-- SGC_W1 Union continuation: 1570 and crossing C1 (packet US-24, chains 3123-3132).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: chain 3123 - 1568 and 1569 done (Human, world 58, 1570 not started): Hammond and Teal'c show "?".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3123, 'SGC_W1 - 1568 and 1569 done: Hammond and Teal''c offer 1570', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3123, 'mission_completed', '1568', 'player', false, 0),
  (3123, 'mission_completed', '1569', 'player', false, 1),
  (3123, 'player_loaded', 'SGC_W1', 'player', false, 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3123, 'mission_status', 1568, NULL, 'eq', 'completed', 0),
  (3123, 'mission_status', 1569, NULL, 'eq', 'completed', 1),
  (3123, 'mission_status', 1570, NULL, 'eq', 'not_active', 2),
  (3123, 'archetype', NULL, NULL, 'gte', '1', 3),
  (3123, 'archetype', NULL, NULL, 'lte', '4', 4),
  (3123, 'world', 58, NULL, 'eq', NULL, 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3123, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "|", "mask": 8388608}', 0, 0),
  (3123, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "|", "mask": 8388608}', 0, 1);

-- CIMMERIA_RECONSTRUCTION: chain 3124 - Hammond press with 1568 and 1569 done and 1570 not started (Human): offer 5392.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3124, 'SGC_W1 - Hammond offers 1570 (5392)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3124, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3124, 'mission_status', 1568, NULL, 'eq', 'completed', 0),
  (3124, 'mission_status', 1569, NULL, 'eq', 'completed', 1),
  (3124, 'mission_status', 1570, NULL, 'eq', 'not_active', 2),
  (3124, 'archetype', NULL, NULL, 'gte', '1', 3),
  (3124, 'archetype', NULL, NULL, 'lte', '4', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3124, 'display_dialog', 5392, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3125 - Teal'c press with 1568 and 1569 done and 1570 not started (Human): offer 5393.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3125, 'SGC_W1 - Teal''c offers 1570 (5393)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3125, 'interact_tag', 'SGC_W1_Tealc', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3125, 'mission_status', 1568, NULL, 'eq', 'completed', 0),
  (3125, 'mission_status', 1569, NULL, 'eq', 'completed', 1),
  (3125, 'mission_status', 1570, NULL, 'eq', 'not_active', 2),
  (3125, 'archetype', NULL, NULL, 'gte', '1', 3),
  (3125, 'archetype', NULL, NULL, 'lte', '4', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3125, 'display_dialog', 5393, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3126 - accept 1570 from Hammond (5392): both "?" clear, Harriman "!", follow-up 5395.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3126, 'SGC_W1 - accept 1570 from Hammond', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3126, 'dialog_choice', '5392', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3126, 'mission_status', 1570, NULL, 'eq', 'not_active', 0),
  (3126, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3126, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3126, 'accept_mission', 1570, NULL, '{}', 0, 0),
  (3126, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 8388608}', 0, 1),
  (3126, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "~", "mask": 8388608}', 0, 2),
  (3126, 'set_interaction_type', NULL, 'SGC_W1_Harriman',
   '{"op": "|", "mask": 16777216}', 0, 3),
  (3126, 'display_dialog', 5395, NULL, '{}', 0, 4);

-- CIMMERIA_RECONSTRUCTION: chain 3127 - accept 1570 from Teal'c (5393): both "?" clear, Harriman "!", follow-up 5396.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3127, 'SGC_W1 - accept 1570 from Teal''c', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3127, 'dialog_choice', '5393', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3127, 'mission_status', 1570, NULL, 'eq', 'not_active', 0),
  (3127, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3127, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3127, 'accept_mission', 1570, NULL, '{}', 0, 0),
  (3127, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 8388608}', 0, 1),
  (3127, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "~", "mask": 8388608}', 0, 2),
  (3127, 'set_interaction_type', NULL, 'SGC_W1_Harriman',
   '{"op": "|", "mask": 16777216}', 0, 3),
  (3127, 'display_dialog', 5396, NULL, '{}', 0, 4);

-- CIMMERIA_RECONSTRUCTION (F4): advance at 0, dial at 0/2500, arrive at 3000; 3130/3131 cover a disconnect inside the window.
-- Chain 3128 - Harriman dials Dakara (C1) while 4641 is active (Human).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3128, 'SGC_W1 - Harriman dials Dakara (C1)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3128, 'interact_tag', 'SGC_W1_Harriman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3128, 'step_status', 1570, '4641', 'eq', 'active', 0),
  (3128, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3128, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3128, 'set_interaction_type', NULL, 'SGC_W1_Harriman',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3128, 'advance_step', 1570, '4642', '{}', 0, 1),
  (3128, 'play_sequence', 10047, NULL, '{}', 0, 2),
  (3128, 'play_sequence', 10060, NULL, '{}', 2500, 3),
  (3128, 'advance_step', 1570, '4906', '{}', 3000, 4),
  (3128, 'cross_world_teleport', NULL, 'Dakara_E1',
   '{"x": 96.87, "y": -16.75, "z": 243.23}', 3000, 5);

-- CIMMERIA_RECONSTRUCTION: chain 3129 - relog on 4641 (Human): relight Harriman.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3129, 'SGC_W1 - relog on 4641: Harriman "!"', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3129, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3129, 'step_status', 1570, '4641', 'eq', 'active', 0),
  (3129, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3129, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3129, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3129, 'set_interaction_type', NULL, 'SGC_W1_Harriman',
  '{"op": "|", "mask": 16777216}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3130 - relog on 4642 (interrupted dial, Human): relight Harriman.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3130, 'SGC_W1 - relog on 4642 (interrupted dial): Harriman "!"', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3130, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3130, 'step_status', 1570, '4642', 'eq', 'active', 0),
  (3130, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3130, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3130, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3130, 'set_interaction_type', NULL, 'SGC_W1_Harriman',
  '{"op": "|", "mask": 16777216}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3131 - recovery: Harriman sends you through while 4642 is active (Human), no dial.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3131, 'SGC_W1 - recovery: Harriman sends you through (4642)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3131, 'interact_tag', 'SGC_W1_Harriman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3131, 'step_status', 1570, '4642', 'eq', 'active', 0),
  (3131, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3131, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3131, 'set_interaction_type', NULL, 'SGC_W1_Harriman',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3131, 'advance_step', 1570, '4906', '{}', 0, 1),
  (3131, 'cross_world_teleport', NULL, 'Dakara_E1',
   '{"x": 96.87, "y": -16.75, "z": 243.23}', 0, 2);
