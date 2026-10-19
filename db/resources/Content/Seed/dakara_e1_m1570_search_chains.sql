-- Dakara_E1 rebuild: mission 1570, steps 4902 and 4901 (the SG-18 search).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-10c).
-- Chain ID range: 8060-8074 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Step 4902 has three client-hinted search regions (D-DK18, DK-09): Louis
-- (objective 5632), Nguyn (5630) and Waters (5631, optional). Entering the
-- second required region moves the player to 4901 and binds the remains.
-- Every chain below is gated on world 61 (Dakara_E1) and its step.

-- Chain 8060 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4902, Louis region entered
-- first of the two required sites: complete objective 5632.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8060, '1570 - 4902: Louis site, first required search region', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8060, 'enter_region', 'Dakara_E1.SG18Louis', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8060, 'world', 61, NULL, 'eq', NULL, 0),
  (8060, 'step_status', 1570, '4902', 'eq', 'active', 1),
  (8060, 'objective_status', 1570, '5632', 'neq', 'completed', 2),
  (8060, 'objective_status', 1570, '5630', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8060, 'complete_objective', 1570, '5632', '{}', 0, 0);

-- Chain 8061 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4902, Louis region entered
-- last (Nguyn's 5630 already done): show 5814 "weak radio signal", advance to
-- 4901, bind the three remains (dsm 7153-7155 on templates 450-452).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8061, '1570 - 4902: Louis site, last required search region', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8061, 'enter_region', 'Dakara_E1.SG18Louis', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8061, 'world', 61, NULL, 'eq', NULL, 0),
  (8061, 'step_status', 1570, '4902', 'eq', 'active', 1),
  (8061, 'objective_status', 1570, '5632', 'neq', 'completed', 2),
  (8061, 'objective_status', 1570, '5630', 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8061, 'display_dialog', 5814, NULL, '{}', 0, 0),
  (8061, 'advance_step', 1570, '4901', '{}', 0, 1),
  (8061, 'add_dialog_set', 7153, NULL, '{"slot": 450}', 0, 2),
  (8061, 'add_dialog_set', 7154, NULL, '{"slot": 451}', 0, 3),
  (8061, 'add_dialog_set', 7155, NULL, '{"slot": 452}', 0, 4);

-- Chain 8062 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4902, Nguyn region entered
-- first of the two required sites: complete objective 5630.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8062, '1570 - 4902: Nguyn site, first required search region', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8062, 'enter_region', 'Dakara_E1.SG18Nguyn', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8062, 'world', 61, NULL, 'eq', NULL, 0),
  (8062, 'step_status', 1570, '4902', 'eq', 'active', 1),
  (8062, 'objective_status', 1570, '5630', 'neq', 'completed', 2),
  (8062, 'objective_status', 1570, '5632', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8062, 'complete_objective', 1570, '5630', '{}', 0, 0);

-- Chain 8063 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4902, Nguyn region entered
-- last (Louis's 5632 already done): as 8061, show 5814, advance to 4901, bind
-- the three remains.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8063, '1570 - 4902: Nguyn site, last required search region', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8063, 'enter_region', 'Dakara_E1.SG18Nguyn', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8063, 'world', 61, NULL, 'eq', NULL, 0),
  (8063, 'step_status', 1570, '4902', 'eq', 'active', 1),
  (8063, 'objective_status', 1570, '5630', 'neq', 'completed', 2),
  (8063, 'objective_status', 1570, '5632', 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8063, 'display_dialog', 5814, NULL, '{}', 0, 0),
  (8063, 'advance_step', 1570, '4901', '{}', 0, 1),
  (8063, 'add_dialog_set', 7153, NULL, '{"slot": 450}', 0, 2),
  (8063, 'add_dialog_set', 7154, NULL, '{"slot": 451}', 0, 3),
  (8063, 'add_dialog_set', 7155, NULL, '{"slot": 452}', 0, 4);

-- Chain 8064 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4902, Waters region entered
-- (optional site): complete objective 5631. advance_step checks it off if skipped.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8064, '1570 - 4902: Waters site, optional search region', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8064, 'enter_region', 'Dakara_E1.SG18Waters', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8064, 'world', 61, NULL, 'eq', NULL, 0),
  (8064, 'step_status', 1570, '4902', 'eq', 'active', 1),
  (8064, 'objective_status', 1570, '5631', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8064, 'complete_objective', 1570, '5631', '{}', 0, 0);

-- Chain 8065 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Louis remains, first or
-- second pickup: dog tags 6224 (speaker 0), give item 6778, objective 5930,
-- count the pickup, mark Louis visited, unbind 7153 from template 450.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8065, '1570 - 4901: Louis remains, first or second pickup', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8065, 'interact_tag', 'Dakara_E1_SG18_Louis', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8065, 'world', 61, NULL, 'eq', NULL, 0),
  (8065, 'step_status', 1570, '4901', 'eq', 'active', 1),
  (8065, 'counter', NULL, 'sg18_louis', 'eq', '0', 2),
  (8065, 'counter', NULL, 'sg18_remains', 'lt', '2', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8065, 'display_dialog', 6224, NULL, '{}', 0, 0),
  (8065, 'add_item', 6778, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8065, 'complete_objective', 1570, '5930', '{}', 0, 2),
  (8065, 'increment_counter', NULL, 'sg18_remains', '{"amount": 1}', 0, 3),
  (8065, 'increment_counter', NULL, 'sg18_louis', '{"amount": 1}', 0, 4),
  (8065, 'remove_dialog_set', 7153, NULL, '{"slot": 450}', 0, 5);

-- Chain 8066 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Louis remains, third
-- pickup: as 8065, but advance to 4905 (3 remains collected, the last
-- required objective 5929 is closed by advance_step, not complete_objective)
-- and reset the search counters and markers.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8066, '1570 - 4901: Louis remains, third pickup, leave 4901', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8066, 'interact_tag', 'Dakara_E1_SG18_Louis', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8066, 'world', 61, NULL, 'eq', NULL, 0),
  (8066, 'step_status', 1570, '4901', 'eq', 'active', 1),
  (8066, 'counter', NULL, 'sg18_louis', 'eq', '0', 2),
  (8066, 'counter', NULL, 'sg18_remains', 'gte', '2', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8066, 'display_dialog', 6224, NULL, '{}', 0, 0),
  (8066, 'add_item', 6778, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8066, 'advance_step', 1570, '4905', '{}', 0, 2),
  (8066, 'reset_counter', NULL, 'sg18_remains', '{}', 0, 3),
  (8066, 'reset_counter', NULL, 'sg18_louis', '{}', 0, 4),
  (8066, 'reset_counter', NULL, 'sg18_nguyn', '{}', 0, 5),
  (8066, 'reset_counter', NULL, 'sg18_waters', '{}', 0, 6),
  (8066, 'remove_dialog_set', 7153, NULL, '{"slot": 450}', 0, 7);

-- Chain 8067 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Nguyn remains, first or
-- second pickup: as 8065 with dog tags 6225, marker sg18_nguyn, template 451.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8067, '1570 - 4901: Nguyn remains, first or second pickup', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8067, 'interact_tag', 'Dakara_E1_SG18_Nguyn', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8067, 'world', 61, NULL, 'eq', NULL, 0),
  (8067, 'step_status', 1570, '4901', 'eq', 'active', 1),
  (8067, 'counter', NULL, 'sg18_nguyn', 'eq', '0', 2),
  (8067, 'counter', NULL, 'sg18_remains', 'lt', '2', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8067, 'display_dialog', 6225, NULL, '{}', 0, 0),
  (8067, 'add_item', 6778, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8067, 'complete_objective', 1570, '5930', '{}', 0, 2),
  (8067, 'increment_counter', NULL, 'sg18_remains', '{"amount": 1}', 0, 3),
  (8067, 'increment_counter', NULL, 'sg18_nguyn', '{"amount": 1}', 0, 4),
  (8067, 'remove_dialog_set', 7154, NULL, '{"slot": 451}', 0, 5);

-- Chain 8068 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Nguyn remains, third
-- pickup: as 8066 with dog tags 6225, marker sg18_nguyn, template 451.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8068, '1570 - 4901: Nguyn remains, third pickup, leave 4901', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8068, 'interact_tag', 'Dakara_E1_SG18_Nguyn', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8068, 'world', 61, NULL, 'eq', NULL, 0),
  (8068, 'step_status', 1570, '4901', 'eq', 'active', 1),
  (8068, 'counter', NULL, 'sg18_nguyn', 'eq', '0', 2),
  (8068, 'counter', NULL, 'sg18_remains', 'gte', '2', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8068, 'display_dialog', 6225, NULL, '{}', 0, 0),
  (8068, 'add_item', 6778, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8068, 'advance_step', 1570, '4905', '{}', 0, 2),
  (8068, 'reset_counter', NULL, 'sg18_remains', '{}', 0, 3),
  (8068, 'reset_counter', NULL, 'sg18_louis', '{}', 0, 4),
  (8068, 'reset_counter', NULL, 'sg18_nguyn', '{}', 0, 5),
  (8068, 'reset_counter', NULL, 'sg18_waters', '{}', 0, 6),
  (8068, 'remove_dialog_set', 7154, NULL, '{"slot": 451}', 0, 7);

-- Chain 8069 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Waters remains, first or
-- second pickup: as 8065 with dog tags 6226, marker sg18_waters, template 452.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8069, '1570 - 4901: Waters remains, first or second pickup', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8069, 'interact_tag', 'Dakara_E1_SG18_Waters', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8069, 'world', 61, NULL, 'eq', NULL, 0),
  (8069, 'step_status', 1570, '4901', 'eq', 'active', 1),
  (8069, 'counter', NULL, 'sg18_waters', 'eq', '0', 2),
  (8069, 'counter', NULL, 'sg18_remains', 'lt', '2', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8069, 'display_dialog', 6226, NULL, '{}', 0, 0),
  (8069, 'add_item', 6778, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8069, 'complete_objective', 1570, '5930', '{}', 0, 2),
  (8069, 'increment_counter', NULL, 'sg18_remains', '{"amount": 1}', 0, 3),
  (8069, 'increment_counter', NULL, 'sg18_waters', '{"amount": 1}', 0, 4),
  (8069, 'remove_dialog_set', 7155, NULL, '{"slot": 452}', 0, 5);

-- Chain 8070 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Waters remains, third
-- pickup: as 8066 with dog tags 6226, marker sg18_waters, template 452.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8070, '1570 - 4901: Waters remains, third pickup, leave 4901', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8070, 'interact_tag', 'Dakara_E1_SG18_Waters', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8070, 'world', 61, NULL, 'eq', NULL, 0),
  (8070, 'step_status', 1570, '4901', 'eq', 'active', 1),
  (8070, 'counter', NULL, 'sg18_waters', 'eq', '0', 2),
  (8070, 'counter', NULL, 'sg18_remains', 'gte', '2', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8070, 'display_dialog', 6226, NULL, '{}', 0, 0),
  (8070, 'add_item', 6778, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8070, 'advance_step', 1570, '4905', '{}', 0, 2),
  (8070, 'reset_counter', NULL, 'sg18_remains', '{}', 0, 3),
  (8070, 'reset_counter', NULL, 'sg18_louis', '{}', 0, 4),
  (8070, 'reset_counter', NULL, 'sg18_nguyn', '{}', 0, 5),
  (8070, 'reset_counter', NULL, 'sg18_waters', '{}', 0, 6),
  (8070, 'remove_dialog_set', 7155, NULL, '{"slot": 452}', 0, 7);

-- Chain 8071 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4901, Dakara_E1 loaded (a relog
-- or a return from the tent): restore the three remains binds (dsm 7153-7155 on
-- templates 450-452). The counters reset too, so all three re-arm.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8071, '1570 - 4901: restore the three remains binds on Dakara_E1 load', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8071, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8071, 'world', 61, NULL, 'eq', NULL, 0),
  (8071, 'step_status', 1570, '4901', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8071, 'add_dialog_set', 7153, NULL, '{"slot": 450}', 0, 0),
  (8071, 'add_dialog_set', 7154, NULL, '{"slot": 451}', 0, 1),
  (8071, 'add_dialog_set', 7155, NULL, '{"slot": 452}', 0, 2);
