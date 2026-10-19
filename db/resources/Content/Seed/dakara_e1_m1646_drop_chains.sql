-- Dakara_E1 rebuild: mission 1646, step 4908 (five drops).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-12c).
-- Chain ID range: 8140-8159 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Step 4908 "Hide the coded orders at the drop sites" has five required
-- objectives (5635, 5939 to 5942), any order. Complete_objective must never
-- close the step's last required objective (C5 rule 3), so the count chains
-- complete at most four of them; the fifth drop leaves the step with
-- advance_step to 4909 and its objective stays open. The counter drops_4908
-- counts the drops done (ruling, DK-08 worknote). It lives in memory and is
-- reset on relog and on every world change, so the rebuild chains
-- 8155-8159 add back one count per drop already completed when the player
-- enters Dakara_E1. Each drop has four chains: a count chain (8140, 8142,
-- ...), its last-drop chain (8141, 8143, ...), a restore bind (8150-8154)
-- and a rebuild increment (8155-8159). Drop k is template 449, 458, 459,
-- 460, 461 (slot T) with dialog set map 6739-6743 (D), tag Dakara_E1_Drop<k>.

-- Chain 8140 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 1 (objective 5635, template 449) while fewer than four drops are counted: display 5821, complete 5635, count the drop, unbind dsm 6739 from slot 449.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8140, '1646 - 4908: drop 1, not last', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8140, 'interact_tag', 'Dakara_E1_Drop1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8140, 'world', 61, NULL, 'eq', NULL, 0),
  (8140, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8140, 'objective_status', 1646, '5635', 'neq', 'completed', 2),
  (8140, 'counter', NULL, 'drops_4908', 'lt', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8140, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8140, 'complete_objective', 1646, '5635', '{}', 0, 1),
  (8140, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 2),
  (8140, 'remove_dialog_set', 6739, NULL, '{"slot": 449}', 0, 3);

-- Chain 8141 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 1 as the fifth drop (four already counted): display 5821, advance 4908 to 4909, reset the count, unbind dsm 6739 from slot 449. Objective 5635 stays open (C5 rule 3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8141, '1646 - 4908: drop 1, last (advance to 4909)', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8141, 'interact_tag', 'Dakara_E1_Drop1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8141, 'world', 61, NULL, 'eq', NULL, 0),
  (8141, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8141, 'objective_status', 1646, '5635', 'neq', 'completed', 2),
  (8141, 'counter', NULL, 'drops_4908', 'gte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8141, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8141, 'advance_step', 1646, '4909', '{}', 0, 1),
  (8141, 'reset_counter', NULL, 'drops_4908', '{}', 0, 2),
  (8141, 'remove_dialog_set', 6739, NULL, '{"slot": 449}', 0, 3);

-- Chain 8150 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 1 not yet done: bind dsm 6739 to slot 449 (C5 rule 4 restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8150, '1646 - 4908: restore drop 1 bind on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8150, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8150, 'world', 61, NULL, 'eq', NULL, 0),
  (8150, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8150, 'objective_status', 1646, '5635', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8150, 'add_dialog_set', 6739, NULL, '{"slot": 449}', 0, 0);

-- Chain 8155 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 1 already done: add it back to the in-memory count, which a world change resets.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8155, '1646 - 4908: rebuild count for drop 1 on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8155, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8155, 'world', 61, NULL, 'eq', NULL, 0),
  (8155, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8155, 'objective_status', 1646, '5635', 'eq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8155, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 0);

-- Chain 8142 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 2 (objective 5939, template 458) while fewer than four drops are counted: display 5821, complete 5939, count the drop, unbind dsm 6740 from slot 458.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8142, '1646 - 4908: drop 2, not last', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8142, 'interact_tag', 'Dakara_E1_Drop2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8142, 'world', 61, NULL, 'eq', NULL, 0),
  (8142, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8142, 'objective_status', 1646, '5939', 'neq', 'completed', 2),
  (8142, 'counter', NULL, 'drops_4908', 'lt', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8142, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8142, 'complete_objective', 1646, '5939', '{}', 0, 1),
  (8142, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 2),
  (8142, 'remove_dialog_set', 6740, NULL, '{"slot": 458}', 0, 3);

-- Chain 8143 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 2 as the fifth drop (four already counted): display 5821, advance 4908 to 4909, reset the count, unbind dsm 6740 from slot 458. Objective 5939 stays open (C5 rule 3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8143, '1646 - 4908: drop 2, last (advance to 4909)', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8143, 'interact_tag', 'Dakara_E1_Drop2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8143, 'world', 61, NULL, 'eq', NULL, 0),
  (8143, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8143, 'objective_status', 1646, '5939', 'neq', 'completed', 2),
  (8143, 'counter', NULL, 'drops_4908', 'gte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8143, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8143, 'advance_step', 1646, '4909', '{}', 0, 1),
  (8143, 'reset_counter', NULL, 'drops_4908', '{}', 0, 2),
  (8143, 'remove_dialog_set', 6740, NULL, '{"slot": 458}', 0, 3);

-- Chain 8151 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 2 not yet done: bind dsm 6740 to slot 458 (C5 rule 4 restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8151, '1646 - 4908: restore drop 2 bind on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8151, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8151, 'world', 61, NULL, 'eq', NULL, 0),
  (8151, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8151, 'objective_status', 1646, '5939', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8151, 'add_dialog_set', 6740, NULL, '{"slot": 458}', 0, 0);

-- Chain 8156 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 2 already done: add it back to the in-memory count, which a world change resets.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8156, '1646 - 4908: rebuild count for drop 2 on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8156, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8156, 'world', 61, NULL, 'eq', NULL, 0),
  (8156, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8156, 'objective_status', 1646, '5939', 'eq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8156, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 0);

-- Chain 8144 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 3 (objective 5940, template 459) while fewer than four drops are counted: display 5821, complete 5940, count the drop, unbind dsm 6741 from slot 459.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8144, '1646 - 4908: drop 3, not last', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8144, 'interact_tag', 'Dakara_E1_Drop3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8144, 'world', 61, NULL, 'eq', NULL, 0),
  (8144, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8144, 'objective_status', 1646, '5940', 'neq', 'completed', 2),
  (8144, 'counter', NULL, 'drops_4908', 'lt', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8144, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8144, 'complete_objective', 1646, '5940', '{}', 0, 1),
  (8144, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 2),
  (8144, 'remove_dialog_set', 6741, NULL, '{"slot": 459}', 0, 3);

-- Chain 8145 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 3 as the fifth drop (four already counted): display 5821, advance 4908 to 4909, reset the count, unbind dsm 6741 from slot 459. Objective 5940 stays open (C5 rule 3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8145, '1646 - 4908: drop 3, last (advance to 4909)', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8145, 'interact_tag', 'Dakara_E1_Drop3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8145, 'world', 61, NULL, 'eq', NULL, 0),
  (8145, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8145, 'objective_status', 1646, '5940', 'neq', 'completed', 2),
  (8145, 'counter', NULL, 'drops_4908', 'gte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8145, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8145, 'advance_step', 1646, '4909', '{}', 0, 1),
  (8145, 'reset_counter', NULL, 'drops_4908', '{}', 0, 2),
  (8145, 'remove_dialog_set', 6741, NULL, '{"slot": 459}', 0, 3);

-- Chain 8152 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 3 not yet done: bind dsm 6741 to slot 459 (C5 rule 4 restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8152, '1646 - 4908: restore drop 3 bind on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8152, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8152, 'world', 61, NULL, 'eq', NULL, 0),
  (8152, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8152, 'objective_status', 1646, '5940', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8152, 'add_dialog_set', 6741, NULL, '{"slot": 459}', 0, 0);

-- Chain 8157 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 3 already done: add it back to the in-memory count, which a world change resets.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8157, '1646 - 4908: rebuild count for drop 3 on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8157, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8157, 'world', 61, NULL, 'eq', NULL, 0),
  (8157, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8157, 'objective_status', 1646, '5940', 'eq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8157, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 0);

-- Chain 8146 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 4 (objective 5941, template 460) while fewer than four drops are counted: display 5821, complete 5941, count the drop, unbind dsm 6742 from slot 460.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8146, '1646 - 4908: drop 4, not last', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8146, 'interact_tag', 'Dakara_E1_Drop4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8146, 'world', 61, NULL, 'eq', NULL, 0),
  (8146, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8146, 'objective_status', 1646, '5941', 'neq', 'completed', 2),
  (8146, 'counter', NULL, 'drops_4908', 'lt', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8146, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8146, 'complete_objective', 1646, '5941', '{}', 0, 1),
  (8146, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 2),
  (8146, 'remove_dialog_set', 6742, NULL, '{"slot": 460}', 0, 3);

-- Chain 8147 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 4 as the fifth drop (four already counted): display 5821, advance 4908 to 4909, reset the count, unbind dsm 6742 from slot 460. Objective 5941 stays open (C5 rule 3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8147, '1646 - 4908: drop 4, last (advance to 4909)', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8147, 'interact_tag', 'Dakara_E1_Drop4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8147, 'world', 61, NULL, 'eq', NULL, 0),
  (8147, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8147, 'objective_status', 1646, '5941', 'neq', 'completed', 2),
  (8147, 'counter', NULL, 'drops_4908', 'gte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8147, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8147, 'advance_step', 1646, '4909', '{}', 0, 1),
  (8147, 'reset_counter', NULL, 'drops_4908', '{}', 0, 2),
  (8147, 'remove_dialog_set', 6742, NULL, '{"slot": 460}', 0, 3);

-- Chain 8153 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 4 not yet done: bind dsm 6742 to slot 460 (C5 rule 4 restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8153, '1646 - 4908: restore drop 4 bind on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8153, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8153, 'world', 61, NULL, 'eq', NULL, 0),
  (8153, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8153, 'objective_status', 1646, '5941', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8153, 'add_dialog_set', 6742, NULL, '{"slot": 460}', 0, 0);

-- Chain 8158 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 4 already done: add it back to the in-memory count, which a world change resets.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8158, '1646 - 4908: rebuild count for drop 4 on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8158, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8158, 'world', 61, NULL, 'eq', NULL, 0),
  (8158, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8158, 'objective_status', 1646, '5941', 'eq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8158, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 0);

-- Chain 8148 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 5 (objective 5942, template 461) while fewer than four drops are counted: display 5821, complete 5942, count the drop, unbind dsm 6743 from slot 461.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8148, '1646 - 4908: drop 5, not last', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8148, 'interact_tag', 'Dakara_E1_Drop5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8148, 'world', 61, NULL, 'eq', NULL, 0),
  (8148, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8148, 'objective_status', 1646, '5942', 'neq', 'completed', 2),
  (8148, 'counter', NULL, 'drops_4908', 'lt', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8148, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8148, 'complete_objective', 1646, '5942', '{}', 0, 1),
  (8148, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 2),
  (8148, 'remove_dialog_set', 6743, NULL, '{"slot": 461}', 0, 3);

-- Chain 8149 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, drop 5 as the fifth drop (four already counted): display 5821, advance 4908 to 4909, reset the count, unbind dsm 6743 from slot 461. Objective 5942 stays open (C5 rule 3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8149, '1646 - 4908: drop 5, last (advance to 4909)', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8149, 'interact_tag', 'Dakara_E1_Drop5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8149, 'world', 61, NULL, 'eq', NULL, 0),
  (8149, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8149, 'objective_status', 1646, '5942', 'neq', 'completed', 2),
  (8149, 'counter', NULL, 'drops_4908', 'gte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8149, 'display_dialog', 5821, NULL, '{}', 0, 0),
  (8149, 'advance_step', 1646, '4909', '{}', 0, 1),
  (8149, 'reset_counter', NULL, 'drops_4908', '{}', 0, 2),
  (8149, 'remove_dialog_set', 6743, NULL, '{"slot": 461}', 0, 3);

-- Chain 8154 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 5 not yet done: bind dsm 6743 to slot 461 (C5 rule 4 restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8154, '1646 - 4908: restore drop 5 bind on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8154, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8154, 'world', 61, NULL, 'eq', NULL, 0),
  (8154, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8154, 'objective_status', 1646, '5942', 'neq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8154, 'add_dialog_set', 6743, NULL, '{"slot": 461}', 0, 0);

-- Chain 8159 PROJECT_FINAL_RECONSTRUCTION: 1646 step 4908, entering Dakara_E1 with drop 5 already done: add it back to the in-memory count, which a world change resets.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8159, '1646 - 4908: rebuild count for drop 5 on Dakara_E1', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8159, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8159, 'world', 61, NULL, 'eq', NULL, 0),
  (8159, 'step_status', 1646, '4908', 'eq', 'active', 1),
  (8159, 'objective_status', 1646, '5942', 'eq', 'completed', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8159, 'increment_counter', NULL, 'drops_4908', '{"amount": 1}', 0, 0);
