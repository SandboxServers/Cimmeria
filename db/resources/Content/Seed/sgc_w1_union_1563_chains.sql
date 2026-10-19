-- SGC_W1 Union continuation: 1563 Virus (packet US-12, chains 3051-3059).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 1563 - Virus (Carter's terminal, US-12)
-- Carter offers 1563 ("?") once a Human finished 1562, takes the offer
-- ("!"), hands over her notes (step 4629 lights her terminal) and the
-- terminal completes objective 5369. Every marker comes back on relog.
-- Human gate: `archetype gte 1` and `archetype lte 4` (never `lt 5`), so a
-- player with no archetype (-1) fails it too.
-- ============================================================

-- Chain 3051 CIMMERIA_RECONSTRUCTION: Carter offers 1563 ("?") once a Human
-- has finished 1562; the player_loaded row restores it on relog.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3051, 'SGC_W1 - Carter offers 1563 Virus (Human)', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3051, 'mission_completed', '1562', 'player', false, 0),
  (3051, 'player_loaded', 'SGC_W1', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3051, 'mission_status', 1562, NULL, 'eq', 'completed', 0),
  (3051, 'mission_status', 1563, NULL, 'eq', 'not_active', 1),
  (3051, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3051, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3051, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3051, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 8388608}', 0, 0);

-- Chain 3052 CIMMERIA_RECONSTRUCTION: pressing Carter before 1563 is taken
-- shows the offer dialog 5370.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3052, 'SGC_W1 - Carter shows the 1563 offer (Human)', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3052, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3052, 'mission_status', 1562, NULL, 'eq', 'completed', 0),
  (3052, 'mission_status', 1563, NULL, 'eq', 'not_active', 1),
  (3052, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3052, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3052, 'display_dialog', 5370, NULL, '{}', 0, 0);

-- Chain 3053 CIMMERIA_RECONSTRUCTION: accepting offer 5370 starts 1563 and
-- swaps Carter's "?" for "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3053, 'SGC_W1 - accept 1563 on Carter 5370 (Human)', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3053, 'dialog_choice', '5370', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3053, 'mission_status', 1562, NULL, 'eq', 'completed', 0),
  (3053, 'mission_status', 1563, NULL, 'eq', 'not_active', 1),
  (3053, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3053, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3053, 'accept_mission', 1563, NULL, '{}', 0, 0),
  (3053, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "~", "mask": 8388608}', 0, 1),
  (3053, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 2);

-- Chain 3054 RESTORED_SOURCE: pressing Carter at step 4628 shows her notes
-- (dialog 5371, display only) and advances 1563 to step 4629, which lights
-- the terminal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3054, 'SGC_W1 - Carter hands over her notes, 1563 step 4629 (Human)', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3054, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3054, 'step_status', 1563, '4628', 'eq', 'active', 0),
  (3054, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3054, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3054, 'display_dialog', 5371, NULL, '{}', 0, 0),
  (3054, 'advance_step', 1563, '4629', '{}', 0, 1),
  (3054, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "~", "mask": 16777216}', 0, 2),
  (3054, 'set_interaction_type', NULL, 'SGC_W1_CarterTerminal', '{"op": "|", "mask": 1073741824}', 0, 3);

-- Chain 3055 RESTORED_SOURCE: the terminal at step 4629 shows 5372, clears
-- its glow and completes objective 5369, which completes 1563.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3055, 'SGC_W1 - Carter terminal runs the virus, completes 1563 (Human)', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3055, 'interact_tag', 'SGC_W1_CarterTerminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3055, 'step_status', 1563, '4629', 'eq', 'active', 0),
  (3055, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3055, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3055, 'display_dialog', 5372, NULL, '{}', 0, 0),
  (3055, 'set_interaction_type', NULL, 'SGC_W1_CarterTerminal', '{"op": "~", "mask": 1073741824}', 0, 1),
  (3055, 'complete_objective', 1563, '5369', '{}', 0, 2);

-- Chain 3056 CIMMERIA_RECONSTRUCTION: re-login at step 4628 restores Carter's
-- "!" (the 1563 marker).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3056, 'SGC_W1 - restore Carter "!" on 1563 step 4628', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3056, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3056, 'step_status', 1563, '4628', 'eq', 'active', 0),
  (3056, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3056, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3056, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3056, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3057 RESTORED_SOURCE: re-login at step 4629 restores the terminal
-- glow (the 1563 objective marker).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3057, 'SGC_W1 - restore the terminal glow on 1563 step 4629', 'mission', 1563, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3057, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3057, 'step_status', 1563, '4629', 'eq', 'active', 0),
  (3057, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3057, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3057, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3057, 'set_interaction_type', NULL, 'SGC_W1_CarterTerminal', '{"op": "|", "mask": 1073741824}', 0, 0);
