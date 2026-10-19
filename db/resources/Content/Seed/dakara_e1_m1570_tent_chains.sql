-- Dakara_E1 rebuild: mission 1570, steps 4906 and 4904 (Rak'nor, Bra'tac, Loth'ta).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-10b).
-- Chain ID range: 8045-8059 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8045 PROJECT_FINAL_RECONSTRUCTION: step 4906, a Free Jaffa (arch gte 7)
-- talks to Rak'nor (dialog 6110, dsm 6890 in slot 441): the optional 5936 is
-- completed and the Jaffa bind is removed. Free Jaffa reach 4906 with 5936 done
-- (chain 8040), so this serves Praxis Jaffa and Jaffa who come from the SGC.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8045, '1570 - 4906: Rak''nor 6110 (Jaffa), optional 5936 done', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8045, 'interact_tag', 'Dakara_E1_Raknor', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8045, 'world', 61, NULL, 'eq', NULL, 0),
  (8045, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8045, 'step_status', 1570, '4906', 'eq', 'active', 2),
  (8045, 'objective_status', 1570, '5936', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8045, 'display_dialog', 6110, NULL, '{}', 0, 0),
  (8045, 'complete_objective', 1570, '5936', '{}', 0, 1),
  (8045, 'remove_dialog_set', 6890, NULL, '{"slot": 441}', 0, 2);

-- Chain 8046 PROJECT_FINAL_RECONSTRUCTION: step 4906, a Tau'ri (arch lte 6)
-- talks to Rak'nor (dialog 6111, dsm 6889 in slot 441): as chain 8045.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8046, '1570 - 4906: Rak''nor 6111 (Tau''ri), optional 5936 done', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8046, 'interact_tag', 'Dakara_E1_Raknor', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8046, 'world', 61, NULL, 'eq', NULL, 0),
  (8046, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8046, 'step_status', 1570, '4906', 'eq', 'active', 2),
  (8046, 'objective_status', 1570, '5936', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8046, 'display_dialog', 6111, NULL, '{}', 0, 0),
  (8046, 'complete_objective', 1570, '5936', '{}', 0, 1),
  (8046, 'remove_dialog_set', 6889, NULL, '{"slot": 441}', 0, 2);

-- Chain 8047 PROJECT_FINAL_RECONSTRUCTION: restore Rak'nor's Jaffa bind (6890,
-- slot 441) after a relog or a tent trip, while 4906 is active and 5936 is open.
-- Disjoint from 8045 (click) and from 8075 (bridge, which needs 63100 not active).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8047, '1570 - 4906: restore Rak''nor 6110 (Jaffa) while 5936 is open', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8047, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8047, 'world', 61, NULL, 'eq', NULL, 0),
  (8047, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8047, 'step_status', 1570, '4906', 'eq', 'active', 2),
  (8047, 'objective_status', 1570, '5936', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8047, 'add_dialog_set', 6890, NULL, '{"slot": 441}', 0, 0);

-- Chain 8048 PROJECT_FINAL_RECONSTRUCTION: restore Rak'nor's Tau'ri bind (6889,
-- slot 441), as chain 8047. A Tau'ri arrives from the SGC at 4906.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8048, '1570 - 4906: restore Rak''nor 6111 (Tau''ri) while 5936 is open', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8048, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8048, 'world', 61, NULL, 'eq', NULL, 0),
  (8048, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8048, 'step_status', 1570, '4906', 'eq', 'active', 2),
  (8048, 'objective_status', 1570, '5936', 'neq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8048, 'add_dialog_set', 6889, NULL, '{"slot": 441}', 0, 0);

-- Chain 8049 PROJECT_FINAL_RECONSTRUCTION: Bra'tac's Jaffa bind (dsm 6727, slot 59)
-- in the tent world 62 while step 4906 is active. Template 59 is shared with
-- Harset: the bind is added to the slot, the template is not edited.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8049, '1570 - 4906: Bra''tac 5811 (Jaffa) bind on the tent load', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8049, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8049, 'world', 62, NULL, 'eq', NULL, 0),
  (8049, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8049, 'step_status', 1570, '4906', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8049, 'add_dialog_set', 6727, NULL, '{"slot": 59}', 0, 0);

-- Chain 8050 PROJECT_FINAL_RECONSTRUCTION: Bra'tac's Tau'ri bind (dsm 6726, slot 59),
-- as chain 8049.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8050, '1570 - 4906: Bra''tac 5810 (Tau''ri) bind on the tent load', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8050, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8050, 'world', 62, NULL, 'eq', NULL, 0),
  (8050, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8050, 'step_status', 1570, '4906', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8050, 'add_dialog_set', 6726, NULL, '{"slot": 59}', 0, 0);

-- Chain 8051 PROJECT_FINAL_RECONSTRUCTION: step 4906, a Jaffa talks to Bra'tac
-- (dialog 5811, dsm 6727 in slot 59): leave 4906 for 4904 and remove the bind.
-- Bra'tac's required 5627 is not completed here: the step is left with advance.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8051, '1570 - 4906: Bra''tac 5811 (Jaffa), advance to 4904', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8051, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8051, 'world', 62, NULL, 'eq', NULL, 0),
  (8051, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8051, 'step_status', 1570, '4906', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8051, 'display_dialog', 5811, NULL, '{}', 0, 0),
  (8051, 'advance_step', 1570, '4904', '{}', 0, 1),
  (8051, 'remove_dialog_set', 6727, NULL, '{"slot": 59}', 0, 2);

-- Chain 8052 PROJECT_FINAL_RECONSTRUCTION: step 4906, a Tau'ri talks to Bra'tac
-- (dialog 5810, dsm 6726 in slot 59): as chain 8051.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8052, '1570 - 4906: Bra''tac 5810 (Tau''ri), advance to 4904', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8052, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8052, 'world', 62, NULL, 'eq', NULL, 0),
  (8052, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8052, 'step_status', 1570, '4906', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8052, 'display_dialog', 5810, NULL, '{}', 0, 0),
  (8052, 'advance_step', 1570, '4904', '{}', 0, 1),
  (8052, 'remove_dialog_set', 6726, NULL, '{"slot": 59}', 0, 2);

-- Chain 8053 PROJECT_FINAL_RECONSTRUCTION: Loth'ta's Jaffa bind (dsm 6729, slot 440)
-- in the Dakara world 61 while step 4904 is active. Loth'ta is in world 61, so the
-- bind comes from this load and not from 8051 (C5 rule 6).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8053, '1570 - 4904: Loth''ta 5813 (Jaffa) bind on the Dakara load', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8053, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8053, 'world', 61, NULL, 'eq', NULL, 0),
  (8053, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8053, 'step_status', 1570, '4904', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8053, 'add_dialog_set', 6729, NULL, '{"slot": 440}', 0, 0);

-- Chain 8054 PROJECT_FINAL_RECONSTRUCTION: Loth'ta's Tau'ri bind (dsm 6728, slot 440),
-- as chain 8053.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8054, '1570 - 4904: Loth''ta 5812 (Tau''ri) bind on the Dakara load', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8054, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8054, 'world', 61, NULL, 'eq', NULL, 0),
  (8054, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8054, 'step_status', 1570, '4904', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8054, 'add_dialog_set', 6728, NULL, '{"slot": 440}', 0, 0);

-- Chain 8055 PROJECT_FINAL_RECONSTRUCTION: step 4904, a Jaffa talks to Loth'ta
-- (dialog 5813, dsm 6729 in slot 440): leave 4904 for 4902 and remove the bind.
-- Step 4902 has no binds (D-DK18), so no chain co-fires on this click.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8055, '1570 - 4904: Loth''ta 5813 (Jaffa), advance to 4902', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8055, 'interact_tag', 'Dakara_E1_Lothta', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8055, 'world', 61, NULL, 'eq', NULL, 0),
  (8055, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8055, 'step_status', 1570, '4904', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8055, 'display_dialog', 5813, NULL, '{}', 0, 0),
  (8055, 'advance_step', 1570, '4902', '{}', 0, 1),
  (8055, 'remove_dialog_set', 6729, NULL, '{"slot": 440}', 0, 2);

-- Chain 8056 PROJECT_FINAL_RECONSTRUCTION: step 4904, a Tau'ri talks to Loth'ta
-- (dialog 5812, dsm 6728 in slot 440): as chain 8055.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8056, '1570 - 4904: Loth''ta 5812 (Tau''ri), advance to 4902', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8056, 'interact_tag', 'Dakara_E1_Lothta', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8056, 'world', 61, NULL, 'eq', NULL, 0),
  (8056, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8056, 'step_status', 1570, '4904', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8056, 'display_dialog', 5812, NULL, '{}', 0, 0),
  (8056, 'advance_step', 1570, '4902', '{}', 0, 1),
  (8056, 'remove_dialog_set', 6728, NULL, '{"slot": 440}', 0, 2);
