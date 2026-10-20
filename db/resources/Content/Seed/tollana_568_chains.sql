-- Tollana (World 19) content chains: mission 568 "Finding Tollan Technology".
--
-- CIMMERIA_RECONSTRUCTION, every chain. missions.script_name is NULL for 568
-- and no 2009 chain survives. Spec:
-- docs/analysis/tollana-continuation/evidence/T-03a-568-chain-spec.md;
-- packets: docs/analysis/tollana-continuation/w1-packets.md (T-03a1, T-03a2).
-- Chain ids 9001-9018 (Tollana reserve 9000-9799). 9001-9010 Copplemann,
-- accept, turn-in, restore, abandon; 9011-9018 food-unit props.
-- Item-less pilot (D-TW1). Reward unresolved (owner O1): no grant rows.
-- Never complete_objective 2311: it auto-completes the mission.

SET search_path = resources, pg_catalog;

-- 4.1 Copplemann click (interact_tag Tollana_Copplemann); disjoint by state.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9001, '568 - Copplemann click, not active: offer briefing 2153', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9001, 'interact_tag', 'Tollana_Copplemann', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9001, 'mission_status', 568, NULL, 'eq', 'not_active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9001, 'display_dialog', 2153, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9002, '568 - Copplemann click, in progress (1964/1965): briefing 2153', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9002, 'interact_tag', 'Tollana_Copplemann', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9002, 'mission_status', 568, NULL, 'eq', 'active', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9002, 'step_status', 568, '1966', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9002, 'display_dialog', 2153, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9003, '568 - Copplemann click, step 1966: turn-in 2154', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9003, 'interact_tag', 'Tollana_Copplemann', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9003, 'step_status', 568, '1966', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9003, 'display_dialog', 2154, NULL, '{}', 0, 0);

-- 4.2 Accept, turn-in, progress bind. Action order is load-bearing.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9004, '568 - accept on close of 2153', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9004, 'dialog_choice', '2153', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9004, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9004, 'mission_status', 568, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9004, 'remove_dialog_set', 2656, NULL, '{"slot": 420}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9004, 'accept_mission', 568, NULL, '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9004, 'advance_step', 568, '1964', '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9005, '568 - turn-in on close of 2154', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9005, 'dialog_choice', '2154', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9005, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9005, 'step_status', 568, '1966', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9005, 'remove_dialog_set', 6393, NULL, '{"slot": 420}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9005, 'complete_mission', 568, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9006, '568 - progress bind 4959 after accept', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9006, 'mission_accepted', '568', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9006, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9006, 'add_dialog_set', 4959, NULL, '{"slot": 420, "mission_id": 568}', 0, 0);

-- 4.3 Tollana entry restore (binds are per player and rebuilt empty).

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9007, '568 - Tollana entry restore: offer bind 2656', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9007, 'player_loaded', 'Tollana', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9007, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9007, 'mission_status', 568, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9007, 'add_dialog_set', 2656, NULL, '{"slot": 420}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9008, '568 - Tollana entry restore: progress bind 4959', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9008, 'player_loaded', 'Tollana', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9008, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9008, 'mission_status', 568, NULL, 'eq', 'active', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9008, 'step_status', 568, '1966', 'neq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9008, 'add_dialog_set', 4959, NULL, '{"slot": 420, "mission_id": 568}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9009, '568 - Tollana entry restore: turn-in bind 6393', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9009, 'player_loaded', 'Tollana', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9009, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9009, 'step_status', 568, '1966', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9009, 'add_dialog_set', 6393, NULL, '{"slot": 420, "mission_id": 568}', 0, 0);

-- 4.4 Abandon repaint (shape of Harset 6120).

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9010, '568 - abandon repaint: offer bind 2656', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9010, 'mission_abandoned', '568', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9010, 'world', 19, NULL, 'eq', NULL, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9010, 'mission_status', 568, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9010, 'remove_dialog_set', 4959, NULL, '{"slot": 420}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9010, 'remove_dialog_set', 6393, NULL, '{"slot": 420}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9010, 'add_dialog_set', 2656, NULL, '{"slot": 420}', 0, 2);

-- 4.5 Food-unit props (interact_tag on the prop tag). No binds: template 421
-- carries INT_MissionWorldObject. A = step 1964 active, B = step 1965 active.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9011, '568 - Core at 1964: advance to 1966, turn-in bind', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9011, 'interact_tag', 'Tollana_FoodUnit_Core', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9011, 'step_status', 568, '1964', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'advance_step', 568, '1965', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'advance_step', 568, '1966', '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'remove_dialog_set', 4959, NULL, '{"slot": 420}', 0, 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'add_dialog_set', 6393, NULL, '{"slot": 420, "mission_id": 568}', 0, 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9011, 'display_dialog', 4211, NULL, '{}', 0, 4);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9012, '568 - Core at 1965: advance to 1966, turn-in bind', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9012, 'interact_tag', 'Tollana_FoodUnit_Core', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9012, 'step_status', 568, '1965', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'advance_step', 568, '1966', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'remove_dialog_set', 4959, NULL, '{"slot": 420}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'add_dialog_set', 6393, NULL, '{"slot": 420, "mission_id": 568}', 0, 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9012, 'display_dialog', 4211, NULL, '{}', 0, 3);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9013, '568 - Core, other states: dialog 4211 only', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9013, 'interact_tag', 'Tollana_FoodUnit_Core', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9013, 'step_status', 568, '1964', 'neq', 'active', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9013, 'step_status', 568, '1965', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9013, 'display_dialog', 4211, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9014, '568 - Nest at 1964: advance to 1965, credit 2312', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9014, 'interact_tag', 'Tollana_FoodUnit_Nest', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9014, 'step_status', 568, '1964', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9014, 'advance_step', 568, '1965', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9014, 'complete_objective', 568, '2312', '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9014, 'display_dialog', 4213, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9015, '568 - Nest at 1965: credit 2312', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9015, 'interact_tag', 'Tollana_FoodUnit_Nest', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9015, 'step_status', 568, '1965', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9015, 'complete_objective', 568, '2312', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9015, 'display_dialog', 4213, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9016, '568 - Nest, other states: dialog 4213 only', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9016, 'interact_tag', 'Tollana_FoodUnit_Nest', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9016, 'step_status', 568, '1964', 'neq', 'active', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9016, 'step_status', 568, '1965', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9016, 'display_dialog', 4213, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9017, '568 - Fungus at 1964: advance to 1965', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9017, 'interact_tag', 'Tollana_FoodUnit_Fungus', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9017, 'step_status', 568, '1964', 'eq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9017, 'advance_step', 568, '1965', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9017, 'display_dialog', 4214, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9018, '568 - Fungus, other states: dialog 4214 only', 'mission', 568, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9018, 'interact_tag', 'Tollana_FoodUnit_Fungus', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9018, 'step_status', 568, '1964', 'neq', 'active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9018, 'display_dialog', 4214, NULL, '{}', 0, 0);
