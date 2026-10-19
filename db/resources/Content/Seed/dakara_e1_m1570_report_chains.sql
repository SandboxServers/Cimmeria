-- Dakara_E1 rebuild: mission 1570, steps 4905 and 4903 (the report to Bra'tac).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-10d).
-- Chain ID range: 8160-8179 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8164 PROJECT_FINAL_RECONSTRUCTION (step 4903, Bra'tac's turn-in): on
-- entering the story room (world 62) while 4903 is active, bind Bra'tac's
-- turn-in dialog 6733 to slot 59 (dialog 5817). The player reaches 4903 out on
-- Dakara (the radio), then walks into the tent. A player already in the tent
-- when 4903 begins gets the bind from DK-10d-A's chains 8170/8171 instead; the
-- conditions here are read before any action runs, so the two never both fire.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8164, '1570 - step 4903: enter the story room, bind Bra''tac''s turn-in dialog 6733@59', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8164, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8164, 'world', 62, NULL, 'eq', NULL, 0),
  (8164, 'step_status', 1570, '4903', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8164, 'add_dialog_set', 6733, NULL, '{"slot": 59}', 0, 0);

-- Chain 8165 PROJECT_FINAL_RECONSTRUCTION (step 4903, Bra'tac's turn-in): a
-- click on Bra'tac in the story room (world 62) while 4903 is active shows the
-- turn-in dialog 5817, removes the bind from 8164 (slot 59) and completes 1570.
-- The unbind comes before the complete, because completing fires the
-- mission_completed chains inline (wave-3.md, shared contract). Mission 1570
-- has no reward (D-DK7).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8165, '1570 - step 4903: Bra''tac''s turn-in, unbind 6733@59, complete 1570', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8165, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8165, 'world', 62, NULL, 'eq', NULL, 0),
  (8165, 'step_status', 1570, '4903', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8165, 'display_dialog', 5817, NULL, '{}', 0, 0),
  (8165, 'remove_dialog_set', 6733, NULL, '{"slot": 59}', 0, 1),
  (8165, 'complete_mission', 1570, NULL, '{}', 0, 2);

-- Chains 8160-8163 and 8166-8171 (DK-10d-A) PROJECT_FINAL_RECONSTRUCTION, step 4905
-- (the radio report to Hammond). Evidence: radio: D-DK10 option B. No NPC speaks the
-- radio call, so each chain shows dialog 5816 (Jaffa) or 5815 (Tau'ri) with
-- display_radio_dialog (DK-06b, EntityId 0, no portrait) and advances 1570 to 4903
-- in the same chain, so progress never waits on the radio's own close. Dialogs 5815
-- and 5816 have no buttons. The dog-tag dialogs (6224 Louis, 6225 Nguyen, 6226
-- Waters) close with button -1 and advance to 4905 on the third pickup (DK-10c's
-- 8066, 8068 and 8070), so a close after the first or second pickup matches nothing.
-- No world condition on 8160-8167: the player may close the dog tag inside the tent.
-- 8168-8171 restore the report after a relog or world change at 4905. Their 2000 ms
-- delay lets the client finish loading before the window opens (precedent: Castle
-- chain 1172); the advance has no delay, so the step gate closes at once. 8170 and
-- 8171 also bind Bra'tac (6733@59); 8164 needs 4903, so it does not fire on that load.

-- Chain 8160 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): Louis's dog tag 6224 closes (Jaffa), radio 5816, advance to 4903.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8160, '1570 - step 4905: Louis''s dog tag 6224 closed (Jaffa), radio 5816, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8160, 'dialog_choice', '6224', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8160, 'archetype', NULL, NULL, 'gte', '7', 0),
  (8160, 'step_status', 1570, '4905', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8160, 'display_radio_dialog', 5816, NULL, '{}', 0, 0),
  (8160, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8161 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): as 8160, Tau'ri, radio 5815.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8161, '1570 - step 4905: Louis''s dog tag 6224 closed (Tau''ri), radio 5815, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8161, 'dialog_choice', '6224', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8161, 'archetype', NULL, NULL, 'lte', '6', 0),
  (8161, 'step_status', 1570, '4905', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8161, 'display_radio_dialog', 5815, NULL, '{}', 0, 0),
  (8161, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8162 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): Nguyen's dog tag 6225 closes (Jaffa), radio 5816, advance to 4903.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8162, '1570 - step 4905: Nguyen''s dog tag 6225 closed (Jaffa), radio 5816, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8162, 'dialog_choice', '6225', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8162, 'archetype', NULL, NULL, 'gte', '7', 0),
  (8162, 'step_status', 1570, '4905', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8162, 'display_radio_dialog', 5816, NULL, '{}', 0, 0),
  (8162, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8163 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): as 8162, Tau'ri, radio 5815.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8163, '1570 - step 4905: Nguyen''s dog tag 6225 closed (Tau''ri), radio 5815, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8163, 'dialog_choice', '6225', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8163, 'archetype', NULL, NULL, 'lte', '6', 0),
  (8163, 'step_status', 1570, '4905', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8163, 'display_radio_dialog', 5815, NULL, '{}', 0, 0),
  (8163, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8166 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): Waters's dog tag 6226 closes (Jaffa), radio 5816, advance to 4903.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8166, '1570 - step 4905: Waters''s dog tag 6226 closed (Jaffa), radio 5816, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8166, 'dialog_choice', '6226', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8166, 'archetype', NULL, NULL, 'gte', '7', 0),
  (8166, 'step_status', 1570, '4905', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8166, 'display_radio_dialog', 5816, NULL, '{}', 0, 0),
  (8166, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8167 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): as 8166, Tau'ri, radio 5815.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8167, '1570 - step 4905: Waters''s dog tag 6226 closed (Tau''ri), radio 5815, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8167, 'dialog_choice', '6226', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8167, 'archetype', NULL, NULL, 'lte', '6', 0),
  (8167, 'step_status', 1570, '4905', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8167, 'display_radio_dialog', 5815, NULL, '{}', 0, 0),
  (8167, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8168 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): restore the report on a load in Dakara (world 61) at 4905, Jaffa; the radio opens after 2000 ms.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8168, '1570 - step 4905: restore the radio report on load in Dakara (Jaffa), radio 5816 after 2 s, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8168, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8168, 'world', 61, NULL, 'eq', NULL, 0),
  (8168, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8168, 'step_status', 1570, '4905', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8168, 'display_radio_dialog', 5816, NULL, '{}', 2000, 0),
  (8168, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8169 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): as 8168, Tau'ri, radio 5815.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8169, '1570 - step 4905: restore the radio report on load in Dakara (Tau''ri), radio 5815 after 2 s, advance to 4903', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8169, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8169, 'world', 61, NULL, 'eq', NULL, 0),
  (8169, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8169, 'step_status', 1570, '4905', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8169, 'display_radio_dialog', 5815, NULL, '{}', 2000, 0),
  (8169, 'advance_step', 1570, '4903', '{}', 0, 1);

-- Chain 8170 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): restore the report on a load in the story room (world 62) at 4905, Jaffa, and bind Bra'tac's turn-in 6733 (8164 needs 4903, so it does not fire here).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8170, '1570 - step 4905: restore the radio report in the story room (Jaffa), radio 5816 after 2 s, advance to 4903, bind Bra''tac''s turn-in 6733@59', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8170, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8170, 'world', 62, NULL, 'eq', NULL, 0),
  (8170, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8170, 'step_status', 1570, '4905', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8170, 'display_radio_dialog', 5816, NULL, '{}', 2000, 0),
  (8170, 'advance_step', 1570, '4903', '{}', 0, 1),
  (8170, 'add_dialog_set', 6733, NULL, '{"slot": 59}', 0, 2);

-- Chain 8171 PROJECT_FINAL_RECONSTRUCTION (step 4905, radio: D-DK10 option B): as 8170, Tau'ri, radio 5815.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8171, '1570 - step 4905: restore the radio report in the story room (Tau''ri), radio 5815 after 2 s, advance to 4903, bind Bra''tac''s turn-in 6733@59', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8171, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8171, 'world', 62, NULL, 'eq', NULL, 0),
  (8171, 'archetype', NULL, NULL, 'lte', '6', 1),
  (8171, 'step_status', 1570, '4905', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8171, 'display_radio_dialog', 5815, NULL, '{}', 2000, 0),
  (8171, 'advance_step', 1570, '4903', '{}', 0, 1),
  (8171, 'add_dialog_set', 6733, NULL, '{"slot": 59}', 0, 2);
