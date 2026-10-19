-- Dakara_E1 rebuild: mission 1653 "Superweapon" (courtyard, power supplies, radio, Bra'tac).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-32a).
-- Chain ID range: 8350-8389 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8350 PROJECT_FINAL_RECONSTRUCTION: 1652 completes (done 1652), accept 1653 at once (D-DK27, no Accept screen).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8350, '1653 - Accept on done 1652 (no blurb)', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8350, 'mission_completed', '1652', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8350, 'world', 61, NULL, 'eq', NULL, 0),
  (8350, 'mission_status', 1653, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8350, 'accept_mission', 1653, NULL, '{}', 0, 0);

-- Chain 8351 PROJECT_FINAL_RECONSTRUCTION: step 4936 "Get to the Superweapon Courtyard!", enter the region,
-- advance to 4938 and light both power supplies (carrier 6892 at 456 and 463).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8351, '1653 - Step 4936: enter courtyard, advance to 4938, bind supplies 6892@456 and 6892@463', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8351, 'enter_region', 'Dakara_E1.SuperweaponCourtyard', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8351, 'world', 61, NULL, 'eq', NULL, 0),
  (8351, 'step_status', 1653, '4936', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8351, 'advance_step', 1653, '4938', '{}', 0, 0),
  (8351, 'add_dialog_set', 6892, NULL, '{"slot": 456}', 0, 1),
  (8351, 'add_dialog_set', 6892, NULL, '{"slot": 463}', 0, 2);

-- Chain 8352 PROJECT_FINAL_RECONSTRUCTION: step 4938, western cannon power supply still open; restore its bind on load.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8352, '1653 - Step 4938: restore western supply bind 6892@456 on load (objective 5642 open)', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8352, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8352, 'world', 61, NULL, 'eq', NULL, 0),
  (8352, 'step_status', 1653, '4938', 'eq', 'active', 1),
  (8352, 'objective_status', 1653, '5642', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8352, 'add_dialog_set', 6892, NULL, '{"slot": 456}', 0, 0);

-- Chain 8353 PROJECT_FINAL_RECONSTRUCTION: step 4938, eastern cannon power supply still open; restore its bind on load.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8353, '1653 - Step 4938: restore eastern supply bind 6892@463 on load (objective 5641 open)', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8353, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8353, 'world', 61, NULL, 'eq', NULL, 0),
  (8353, 'step_status', 1653, '4938', 'eq', 'active', 1),
  (8353, 'objective_status', 1653, '5641', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8353, 'add_dialog_set', 6892, NULL, '{"slot": 463}', 0, 0);

-- Chain 8354 PROJECT_FINAL_RECONSTRUCTION: western supply clicked first (5641 still open, so the mission does not complete).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8354, '1653 - Step 4938: click western supply first, objective 5642, unbind 6892@456', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8354, 'interact_tag', 'Dakara_E1_PowerSupply1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8354, 'world', 61, NULL, 'eq', NULL, 0),
  (8354, 'step_status', 1653, '4938', 'eq', 'active', 1),
  (8354, 'objective_status', 1653, '5642', 'neq', 'completed', 2),
  (8354, 'objective_status', 1653, '5641', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8354, 'complete_objective', 1653, '5642', '{}', 0, 0),
  (8354, 'remove_dialog_set', 6892, NULL, '{"slot": 456}', 0, 1);

-- Chain 8355 PROJECT_FINAL_RECONSTRUCTION: western supply clicked second (5641 done). Leave 4938 with advance, never objective.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8355, '1653 - Step 4938: click western supply second, unbind 6892@456, advance to 4937, radio 5849', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8355, 'interact_tag', 'Dakara_E1_PowerSupply1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8355, 'world', 61, NULL, 'eq', NULL, 0),
  (8355, 'step_status', 1653, '4938', 'eq', 'active', 1),
  (8355, 'objective_status', 1653, '5642', 'neq', 'completed', 2),
  (8355, 'objective_status', 1653, '5641', 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8355, 'remove_dialog_set', 6892, NULL, '{"slot": 456}', 0, 0),
  (8355, 'advance_step', 1653, '4937', '{}', 0, 1),
  (8355, 'display_radio_dialog', 5849, NULL, '{}', 0, 2);

-- Chain 8356 PROJECT_FINAL_RECONSTRUCTION: eastern supply clicked first (5642 still open, so the mission does not complete).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8356, '1653 - Step 4938: click eastern supply first, objective 5641, unbind 6892@463', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8356, 'interact_tag', 'Dakara_E1_PowerSupply2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8356, 'world', 61, NULL, 'eq', NULL, 0),
  (8356, 'step_status', 1653, '4938', 'eq', 'active', 1),
  (8356, 'objective_status', 1653, '5641', 'neq', 'completed', 2),
  (8356, 'objective_status', 1653, '5642', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8356, 'complete_objective', 1653, '5641', '{}', 0, 0),
  (8356, 'remove_dialog_set', 6892, NULL, '{"slot": 463}', 0, 1);

-- Chain 8357 PROJECT_FINAL_RECONSTRUCTION: eastern supply clicked second (5642 done). Leave 4938 with advance, never objective.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8357, '1653 - Step 4938: click eastern supply second, unbind 6892@463, advance to 4937, radio 5849', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8357, 'interact_tag', 'Dakara_E1_PowerSupply2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8357, 'world', 61, NULL, 'eq', NULL, 0),
  (8357, 'step_status', 1653, '4938', 'eq', 'active', 1),
  (8357, 'objective_status', 1653, '5641', 'neq', 'completed', 2),
  (8357, 'objective_status', 1653, '5642', 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8357, 'remove_dialog_set', 6892, NULL, '{"slot": 463}', 0, 0),
  (8357, 'advance_step', 1653, '4937', '{}', 0, 1),
  (8357, 'display_radio_dialog', 5849, NULL, '{}', 0, 2);

-- Chain 8358 PROJECT_FINAL_RECONSTRUCTION: the call is closed (any place; a radio window can be closed after a flap trip).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8358, '1653 - Step 4937: close radio 5849, advance to 4935', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8358, 'dialog_choice', '5849', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8358, 'step_status', 1653, '4937', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8358, 'advance_step', 1653, '4935', '{}', 0, 0);

-- Chain 8359 PROJECT_FINAL_RECONSTRUCTION: the call is closed in the tent; bind Bra'tac now (co-fires with 8358).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8359, '1653 - Step 4937: radio 5849 closed in the tent, bind Bra''tac 6775@59', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8359, 'dialog_choice', '5849', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8359, 'world', 62, NULL, 'eq', NULL, 0),
  (8359, 'step_status', 1653, '4937', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8359, 'add_dialog_set', 6775, NULL, '{"slot": 59}', 0, 0);

-- Chain 8360 PROJECT_FINAL_RECONSTRUCTION: the call was never closed; replay the radio after a Dakara load.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8360, '1653 - Step 4937: recovery, replay radio 5849 after load on Dakara_E1', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8360, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8360, 'world', 61, NULL, 'eq', NULL, 0),
  (8360, 'step_status', 1653, '4937', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8360, 'display_radio_dialog', 5849, NULL, '{}', 2000, 0);

-- Chain 8361 PROJECT_FINAL_RECONSTRUCTION: the same recovery when the player loads in the tent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8361, '1653 - Step 4937: recovery, replay radio 5849 after load on Dakara_E1_StoryRm', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8361, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8361, 'world', 62, NULL, 'eq', NULL, 0),
  (8361, 'step_status', 1653, '4937', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8361, 'display_radio_dialog', 5849, NULL, '{}', 2000, 0);

-- Chain 8362 PROJECT_FINAL_RECONSTRUCTION: step 4935 "Report back to Bra'tac", restore Bra'tac's report bind on load.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8362, '1653 - Step 4935: restore Bra''tac report bind 6775@59 on StoryRm load', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8362, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8362, 'world', 62, NULL, 'eq', NULL, 0),
  (8362, 'step_status', 1653, '4935', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8362, 'add_dialog_set', 6775, NULL, '{"slot": 59}', 0, 0);

-- Chain 8363 PROJECT_FINAL_RECONSTRUCTION: report to Bra'tac in person (5848); unbind before complete (C5 rule 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8363, '1653 - Step 4935: Bra''tac 5848, unbind 6775@59, complete 1653', 'mission', 1653, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8363, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8363, 'world', 62, NULL, 'eq', NULL, 0),
  (8363, 'step_status', 1653, '4935', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8363, 'display_dialog', 5848, NULL, '{}', 0, 0),
  (8363, 'remove_dialog_set', 6775, NULL, '{"slot": 59}', 0, 1),
  (8363, 'complete_mission', 1653, NULL, '{}', 0, 2);
