-- SGC_W1 Union continuation: 1564 Self-Destruct (packet US-15, chains 3060-3071).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
--
-- The steps (4630-4633), objective 5373 and dialogs 5374, 5375, 5377 and 5378
-- are RESTORED_SOURCE. The chain wiring below (offer, radio report, the
-- FailsafeRoom region, the device and the restores) is CIMMERIA_RECONSTRUCTION.
--
-- Human classes only: `archetype gte 1` and `archetype lte 4` (the Union Human
-- gate). An unstamped player reads -1 and fails it.
SET search_path = resources, pg_catalog;

-- Chain 3060: M1563 completes, or the player loads SGC_W1 with 1563 done and
-- 1564 not yet given → Carter shows the "?" offer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3060, 'SGC_W1 - Carter offers 1564 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3060, 'mission_completed', '1563', 'player', false, 0),
  (3060, 'player_loaded', 'SGC_W1', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3060, 'mission_status', 1563, NULL, 'eq', 'completed', 0),
  (3060, 'mission_status', 1564, NULL, 'eq', 'not_active', 1),
  (3060, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3060, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3060, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3060, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
        '{"op": "|", "mask": 8388608}', 0, 0);

-- Chain 3061: press Carter before 1564 is taken → her offer dialog 5374.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3061, 'SGC_W1 - Interact Col. Carter, 1564 offer (Human): show dialog 5374', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3061, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3061, 'mission_status', 1563, NULL, 'eq', 'completed', 0),
  (3061, 'mission_status', 1564, NULL, 'eq', 'not_active', 1),
  (3061, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3061, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3061, 'display_dialog', 5374, NULL, '{}', 0, 0);

-- Chain 3062: accept 1564 on 5374 → step 4630, Carter's "?" goes and her "!" comes.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3062, 'SGC_W1 - accept 1564 on 5374 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3062, 'dialog_choice', '5374', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3062, 'mission_status', 1563, NULL, 'eq', 'completed', 0),
  (3062, 'mission_status', 1564, NULL, 'eq', 'not_active', 1),
  (3062, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3062, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3062, 'accept_mission', 1564, NULL, '{}', 0, 0),
  (3062, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
   '{"op": "~", "mask": 8388608}', 0, 1),
  (3062, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
   '{"op": "|", "mask": 16777216}', 0, 2);

-- Chain 3063: press Carter on step 4630 → the radio report 5375 (no buttons).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3063, 'SGC_W1 - Carter radio report 5375, 1564 step 4630 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3063, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3063, 'step_status', 1564, '4630', 'eq', 'active', 0),
  (3063, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3063, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3063, 'display_dialog', 5375, NULL, '{}', 0, 0);

-- Chain 3064: close 5375 → step 4631, go to the Failsafe room.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3064, 'SGC_W1 - 5375 closed: 1564 step 4631 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3064, 'dialog_choice', '5375', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3064, 'step_status', 1564, '4630', 'eq', 'active', 0),
  (3064, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3064, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3064, 'advance_step', 1564, '4631', '{}', 0, 0);

-- Chain 3065: enter SGC_W1.FailsafeRoom on 4631 → step 4632, the device glows.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3065, 'SGC_W1 - FailsafeRoom reached: 1564 step 4632 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3065, 'enter_region', 'SGC_W1.FailsafeRoom', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3065, 'step_status', 1564, '4631', 'eq', 'active', 0),
  (3065, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3065, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3065, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3065, 'advance_step', 1564, '4632', '{}', 0, 0),
  (3065, 'set_interaction_type', NULL, 'SGC_W1_FailsafeDevice',
   '{"op": "|", "mask": 1073741824}', 0, 1);

-- Chain 3066: Carter on 4632 → her Failsafe dialog 5377 (no buttons).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3066, 'SGC_W1 - Carter at the Failsafe, 1564 step 4632 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3066, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3066, 'step_status', 1564, '4632', 'eq', 'active', 0),
  (3066, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3066, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3066, 'display_dialog', 5377, NULL, '{}', 0, 0);

-- Chain 3067: press the Failsafe Device on 4632 → step 4633, the glow clears.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3067, 'SGC_W1 - Failsafe Device pressed: 1564 step 4633 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3067, 'interact_tag', 'SGC_W1_FailsafeDevice', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3067, 'step_status', 1564, '4632', 'eq', 'active', 0),
  (3067, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3067, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3067, 'advance_step', 1564, '4633', '{}', 0, 0),
  (3067, 'set_interaction_type', NULL, 'SGC_W1_FailsafeDevice',
   '{"op": "~", "mask": 1073741824}', 0, 1);

-- Chain 3068: Carter on 4633 → her final report 5378 (no buttons).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3068, 'SGC_W1 - Carter final report 5378, 1564 step 4633 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3068, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3068, 'step_status', 1564, '4633', 'eq', 'active', 0),
  (3068, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3068, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3068, 'display_dialog', 5378, NULL, '{}', 0, 0);

-- Chain 3069: close 5378 → Carter's "!" goes and objective 5373 completes 1564.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3069, 'SGC_W1 - 5378 closed: complete 1564 (Human)', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3069, 'dialog_choice', '5378', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3069, 'step_status', 1564, '4633', 'eq', 'active', 0),
  (3069, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3069, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3069, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3069, 'complete_objective', 1564, '5373', '{}', 0, 1);

-- Chain 3070: load SGC_W1 on an active 1564 → restore Carter's "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3070, 'SGC_W1 - Load active 1564 (Human): restore Col. Carter "!"', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3070, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3070, 'mission_status', 1564, NULL, 'eq', 'active', 0),
  (3070, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3070, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3070, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3070, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
        '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3071: load SGC_W1 on step 4632 → restore the device glow.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3071, 'SGC_W1 - Load active 4632 (Human): restore the Failsafe Device glow', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3071, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3071, 'step_status', 1564, '4632', 'eq', 'active', 0),
  (3071, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3071, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3071, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3071, 'set_interaction_type', NULL, 'SGC_W1_FailsafeDevice',
        '{"op": "|", "mask": 1073741824}', 0, 0);
