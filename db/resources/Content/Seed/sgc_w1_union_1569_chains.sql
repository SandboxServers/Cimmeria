-- SGC_W1 Union continuation: 1569 Ordinance (packet US-23, chains 3110-3122).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION (H3): one crate per required objective of step 4640;
-- rewards stay in chains 3041-3044. Chain 3110 offers 1569 after 1564 and
-- lights the six crates. The offer gates on `mission_status 1569 eq not_active`
-- so a completed 1569 is never re-offered (see the rule in chains 3041-3044).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3110, 'SGC_W1 - offer 1569 after 1564, light the crates', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3110, 'mission_completed', '1564', 'player', false, 0),
  (3110, 'player_loaded', 'SGC_W1', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3110, 'mission_status', 1564, NULL, 'eq', 'completed', 0),
  (3110, 'mission_status', 1569, NULL, 'eq', 'not_active', 1),
  (3110, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3110, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3110, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3110, 'accept_mission', 1569, NULL, '{}', 0, 0),
  (3110, 'set_interaction_type', NULL, 'SGC_W1_Crate1', '{"op": "|", "mask": 1073741824}', 0, 1),
  (3110, 'set_interaction_type', NULL, 'SGC_W1_Crate2', '{"op": "|", "mask": 1073741824}', 0, 2),
  (3110, 'set_interaction_type', NULL, 'SGC_W1_Crate3', '{"op": "|", "mask": 1073741824}', 0, 3),
  (3110, 'set_interaction_type', NULL, 'SGC_W1_Crate4', '{"op": "|", "mask": 1073741824}', 0, 4),
  (3110, 'set_interaction_type', NULL, 'SGC_W1_Crate5', '{"op": "|", "mask": 1073741824}', 0, 5),
  (3110, 'set_interaction_type', NULL, 'SGC_W1_Crate6', '{"op": "|", "mask": 1073741824}', 0, 6);

-- Chain 3111: open Crate1 (objective 5381). The crate has no dialog: the
-- marker flips and the objective is credited. A repeat press does nothing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3111, 'SGC_W1 - open Crate1 (Human): objective 5381', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3111, 'interact_tag', 'SGC_W1_Crate1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3111, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3111, 'objective_status', 1569, '5381', 'neq', 'completed', 1),
  (3111, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3111, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3111, 'complete_objective', 1569, '5381', '{}', 0, 0),
  (3111, 'set_interaction_type', NULL, 'SGC_W1_Crate1', '{"op": "~", "mask": 1073741824}', 0, 1);

-- Chain 3112: open Crate2 (objective 5385). The dialog shows before the
-- objective completes, so the last crate's text is not cut short.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3112, 'SGC_W1 - open Crate2 (Human): dialog 5385, objective 5385', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3112, 'interact_tag', 'SGC_W1_Crate2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3112, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3112, 'objective_status', 1569, '5385', 'neq', 'completed', 1),
  (3112, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3112, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3112, 'display_dialog', 5385, NULL, '{}', 0, 0),
  (3112, 'complete_objective', 1569, '5385', '{}', 0, 1),
  (3112, 'set_interaction_type', NULL, 'SGC_W1_Crate2', '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chain 3113: open Crate3 (objective 5386).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3113, 'SGC_W1 - open Crate3 (Human): dialog 5386, objective 5386', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3113, 'interact_tag', 'SGC_W1_Crate3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3113, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3113, 'objective_status', 1569, '5386', 'neq', 'completed', 1),
  (3113, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3113, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3113, 'display_dialog', 5386, NULL, '{}', 0, 0),
  (3113, 'complete_objective', 1569, '5386', '{}', 0, 1),
  (3113, 'set_interaction_type', NULL, 'SGC_W1_Crate3', '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chain 3114: open Crate4 (objective 5387).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3114, 'SGC_W1 - open Crate4 (Human): dialog 5387, objective 5387', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3114, 'interact_tag', 'SGC_W1_Crate4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3114, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3114, 'objective_status', 1569, '5387', 'neq', 'completed', 1),
  (3114, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3114, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3114, 'display_dialog', 5387, NULL, '{}', 0, 0),
  (3114, 'complete_objective', 1569, '5387', '{}', 0, 1),
  (3114, 'set_interaction_type', NULL, 'SGC_W1_Crate4', '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chain 3115: open Crate5 (objective 5388).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3115, 'SGC_W1 - open Crate5 (Human): dialog 5388, objective 5388', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3115, 'interact_tag', 'SGC_W1_Crate5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3115, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3115, 'objective_status', 1569, '5388', 'neq', 'completed', 1),
  (3115, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3115, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3115, 'display_dialog', 5388, NULL, '{}', 0, 0),
  (3115, 'complete_objective', 1569, '5388', '{}', 0, 1),
  (3115, 'set_interaction_type', NULL, 'SGC_W1_Crate5', '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chain 3116: open Crate6 (objective 5389). The sixth required objective
-- completes 1569; mission_completed then fires chains 3041-3044.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3116, 'SGC_W1 - open Crate6 (Human): dialog 5389, objective 5389', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3116, 'interact_tag', 'SGC_W1_Crate6', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3116, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3116, 'objective_status', 1569, '5389', 'neq', 'completed', 1),
  (3116, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3116, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3116, 'display_dialog', 5389, NULL, '{}', 0, 0),
  (3116, 'complete_objective', 1569, '5389', '{}', 0, 1),
  (3116, 'set_interaction_type', NULL, 'SGC_W1_Crate6', '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chains 3117-3122: relog during 1569 relights the crates not yet opened.
-- Each one lights its crate while step 4640 is active and its objective is
-- not completed. Crate1 is 3117 (5381) through Crate6 is 3122 (5389).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES
  (3117, 'SGC_W1 - relog during 1569 (Human): relight Crate1', 'space', NULL, true, 0),
  (3118, 'SGC_W1 - relog during 1569 (Human): relight Crate2', 'space', NULL, true, 0),
  (3119, 'SGC_W1 - relog during 1569 (Human): relight Crate3', 'space', NULL, true, 0),
  (3120, 'SGC_W1 - relog during 1569 (Human): relight Crate4', 'space', NULL, true, 0),
  (3121, 'SGC_W1 - relog during 1569 (Human): relight Crate5', 'space', NULL, true, 0),
  (3122, 'SGC_W1 - relog during 1569 (Human): relight Crate6', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3117, 'player_loaded', 'SGC_W1', 'player', false, 0),
  (3118, 'player_loaded', 'SGC_W1', 'player', false, 0),
  (3119, 'player_loaded', 'SGC_W1', 'player', false, 0),
  (3120, 'player_loaded', 'SGC_W1', 'player', false, 0),
  (3121, 'player_loaded', 'SGC_W1', 'player', false, 0),
  (3122, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3117, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3117, 'objective_status', 1569, '5381', 'neq', 'completed', 1),
  (3117, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3117, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3117, 'world', 58, NULL, 'eq', NULL, 4),
  (3118, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3118, 'objective_status', 1569, '5385', 'neq', 'completed', 1),
  (3118, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3118, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3118, 'world', 58, NULL, 'eq', NULL, 4),
  (3119, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3119, 'objective_status', 1569, '5386', 'neq', 'completed', 1),
  (3119, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3119, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3119, 'world', 58, NULL, 'eq', NULL, 4),
  (3120, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3120, 'objective_status', 1569, '5387', 'neq', 'completed', 1),
  (3120, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3120, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3120, 'world', 58, NULL, 'eq', NULL, 4),
  (3121, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3121, 'objective_status', 1569, '5388', 'neq', 'completed', 1),
  (3121, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3121, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3121, 'world', 58, NULL, 'eq', NULL, 4),
  (3122, 'step_status', 1569, '4640', 'eq', 'active', 0),
  (3122, 'objective_status', 1569, '5389', 'neq', 'completed', 1),
  (3122, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3122, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3122, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3117, 'set_interaction_type', NULL, 'SGC_W1_Crate1', '{"op": "|", "mask": 1073741824}', 0, 0),
  (3118, 'set_interaction_type', NULL, 'SGC_W1_Crate2', '{"op": "|", "mask": 1073741824}', 0, 0),
  (3119, 'set_interaction_type', NULL, 'SGC_W1_Crate3', '{"op": "|", "mask": 1073741824}', 0, 0),
  (3120, 'set_interaction_type', NULL, 'SGC_W1_Crate4', '{"op": "|", "mask": 1073741824}', 0, 0),
  (3121, 'set_interaction_type', NULL, 'SGC_W1_Crate5', '{"op": "|", "mask": 1073741824}', 0, 0),
  (3122, 'set_interaction_type', NULL, 'SGC_W1_Crate6', '{"op": "|", "mask": 1073741824}', 0, 0);
