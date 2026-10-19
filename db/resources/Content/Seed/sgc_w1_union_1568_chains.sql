-- SGC_W1 Union continuation: 1568 Gate Room (packet US-22, chains 3100-3109).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: step text 4637-4639; terminal and Gate Room Jaffa already on the map.
-- Chain 3100: offer 1568 after 1564 completes, or on load after it (Human only).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3100, 'SGC_W1 - offer 1568 after 1564', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3100, 'mission_completed', '1564', 'player', false, 0),
  (3100, 'player_loaded', 'SGC_W1', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3100, 'mission_status', 1564, NULL, 'eq', 'completed', 0),
  (3100, 'mission_status', 1568, NULL, 'eq', 'not_active', 1),
  (3100, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3100, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3100, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3100, 'accept_mission', 1568, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3101 - region Gate Control while 4637 active (Human): advance to 4638, monologue 5382, light the terminal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3101, 'SGC_W1 - reach Gate Control (4637 -> 4638)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3101, 'enter_region', 'SGC_W1.GateControl', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3101, 'step_status', 1568, '4637', 'eq', 'active', 0),
  (3101, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3101, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3101, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3101, 'advance_step', 1568, '4638', '{}', 0, 0),
  (3101, 'display_dialog', 5382, NULL, '{}', 0, 1),
  (3101, 'set_interaction_type', NULL, 'SGC_W1_GateTerminal1',
   '{"op": "|", "mask": 1073741824}', 0, 2);

-- CIMMERIA_RECONSTRUCTION: chain 3102 - terminal press while 4638 active (Human): show 5383, clear the glow, advance to 4639.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3102, 'SGC_W1 - terminal shuts the gate (4638 -> 4639)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3102, 'interact_tag', 'SGC_W1_GateTerminal1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3102, 'step_status', 1568, '4638', 'eq', 'active', 0),
  (3102, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3102, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3102, 'display_dialog', 5383, NULL, '{}', 0, 0),
  (3102, 'set_interaction_type', NULL, 'SGC_W1_GateTerminal1',
   '{"op": "~", "mask": 1073741824}', 0, 1),
  (3102, 'advance_step', 1568, '4639', '{}', 0, 2);

-- CIMMERIA_RECONSTRUCTION: chain 3103 - the three Gate Room Jaffa die while 4639 active: complete 5380.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3103, 'SGC_W1 - Gate Room Jaffa dead', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3103, 'entity_dead_tag', 'SGC_W1_GroomJaffa1', 'space', false, 0),
  (3103, 'entity_dead_tag', 'SGC_W1_GroomJaffa2', 'space', false, 1),
  (3103, 'entity_dead_tag', 'SGC_W1_GroomJaffa3', 'space', false, 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3103, 'step_status', 1568, '4639', 'eq', 'active', 0),
  (3103, 'entity_tag_state', NULL, 'SGC_W1_GroomJaffa1', 'eq', 'dead', 1),
  (3103, 'entity_tag_state', NULL, 'SGC_W1_GroomJaffa2', 'eq', 'dead', 2),
  (3103, 'entity_tag_state', NULL, 'SGC_W1_GroomJaffa3', 'eq', 'dead', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3103, 'complete_objective', 1568, '5380', '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3104 - terminal backstop: the Gate Room is already clear while 4638 active. Runs after 3102 (chain id order, equal priority).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3104, 'SGC_W1 - terminal backstop: Gate Room already clear', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3104, 'interact_tag', 'SGC_W1_GateTerminal1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3104, 'step_status', 1568, '4638', 'eq', 'active', 0),
  (3104, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3104, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3104, 'entity_tag_state', NULL, 'SGC_W1_GroomJaffa1', 'eq', 'dead', 3),
  (3104, 'entity_tag_state', NULL, 'SGC_W1_GroomJaffa2', 'eq', 'dead', 4),
  (3104, 'entity_tag_state', NULL, 'SGC_W1_GroomJaffa3', 'eq', 'dead', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3104, 'complete_objective', 1568, '5380', '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3105 - 1568 done (Human): stage the departure. Player at P15, Carter at P16, Hammond at P14, dialog 5384.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3105, 'SGC_W1 - 1568 done: stage the departure', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3105, 'mission_completed', '1568', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3105, 'archetype', NULL, NULL, 'gte', '1', 0),
  (3105, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3105, 'move_entity', NULL, NULL,
   '{"destination": "182.0,1.311,43.5", "world": "SGC_W1", "use_player": true}', 0, 0),
  (3105, 'move_entity', NULL, 'SGC_W1_SamCarter',
   '{"destination": "179.5,1.311,45.5", "world": "SGC_W1"}', 0, 1),
  (3105, 'move_entity', NULL, 'SGCW1_GenHammond',
   '{"destination": "199.989,1.311,43.608", "world": "SGC_W1"}', 0, 2),
  (3105, 'display_dialog', 5384, NULL, '{}', 0, 3);

-- CIMMERIA_RECONSTRUCTION: chain 3106 - relog on 4638 (Human): relight the terminal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3106, 'SGC_W1 - relog on 4638: relight the terminal', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3106, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3106, 'step_status', 1568, '4638', 'eq', 'active', 0),
  (3106, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3106, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3106, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3106, 'set_interaction_type', NULL, 'SGC_W1_GateTerminal1',
  '{"op": "|", "mask": 1073741824}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3107 - relog after 1568, before 1570 (Human): Carter back at P16.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3107, 'SGC_W1 - relog after 1568, before 1570: Carter at P16', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3107, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3107, 'mission_status', 1568, NULL, 'eq', 'completed', 0),
  (3107, 'mission_status', 1570, NULL, 'eq', 'not_active', 1),
  (3107, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3107, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3107, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3107, 'move_entity', NULL, 'SGC_W1_SamCarter',
  '{"destination": "179.5,1.311,45.5", "world": "SGC_W1"}', 0, 0);
