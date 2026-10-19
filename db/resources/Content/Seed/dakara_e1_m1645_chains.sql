-- Dakara_E1 rebuild: mission 1645.
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-11).
-- Chain ID range: 8080-8119 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8080 RECONSTRUCTION (D-DK1 F): 1570 is done in the tent beside Moh'katan
-- (chain 8165), so the 1645 offer (dsm 6736, dialog 5980, "available") appears
-- at once on Moh'katan (template 54, tag Dakara_E1_StoryRm_Mohkatan).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8080, '1645 - 1570 completed on Dakara_E1_StoryRm: bind offer 6736 to Moh''katan (54)', 'mission', 1645, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8080, 'mission_completed', '1570', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8080, 'world', 62, NULL, 'eq', NULL, 0),
  (8080, 'mission_status', 1645, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8080, 'add_dialog_set', 6736, NULL, '{"slot": 54}', 0, 0);

-- Chain 8081 RECONSTRUCTION: restore the 1645 offer bind on entering the tent
-- after 1570 is done (the bind does not survive a relog or world change).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8081, '1645 - Restore offer on Dakara_E1_StoryRm after 1570: bind 6736 to Moh''katan (54)', 'mission', 1645, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8081, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8081, 'world', 62, NULL, 'eq', NULL, 0),
  (8081, 'mission_status', 1570, NULL, 'eq', 'completed', 1),
  (8081, 'mission_status', 1645, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8081, 'add_dialog_set', 6736, NULL, '{"slot": 54}', 0, 0);

-- Chain 8082 RECONSTRUCTION: accepting the offer (dialog 5980) accepts 1645,
-- plays the orders scene 5807 (Moh'katan and Bra'tac), hands over the withdrawal
-- orders (item 6779), and moves past step 5168 to 4907 (deliver to Loth'ta).
-- The offer bind is removed before the mission moves on.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8082, '1645 - Accept offer 5980: accept, orders scene 5807, give item 6779, advance to 4907, unbind offer 6736', 'mission', 1645, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8082, 'dialog_choice', '5980', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8082, 'world', 62, NULL, 'eq', NULL, 0),
  (8082, 'mission_status', 1570, NULL, 'eq', 'completed', 1),
  (8082, 'mission_status', 1645, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8082, 'accept_mission', 1645, NULL, '{}', 0, 0),
  (8082, 'display_dialog', 5807, NULL, '{}', 0, 1),
  (8082, 'add_item', 6779, NULL, '{"container": 0, "qty": 1}', 0, 2),
  (8082, 'advance_step', 1645, '4907', '{}', 0, 3),
  (8082, 'remove_dialog_set', 6736, NULL, '{"slot": 54}', 0, 4);

-- Chain 8083 RECONSTRUCTION: Loth'ta (template 440, Dakara_E1) takes the
-- turn-in of 4907 (dialog 5808). Bind the turn-in dsm 6735 on entering Dakara_E1.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8083, '1645 - Step 4907 on Dakara_E1: bind turn-in 6735 to Loth''ta (440)', 'mission', 1645, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8083, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8083, 'world', 61, NULL, 'eq', NULL, 0),
  (8083, 'step_status', 1645, '4907', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8083, 'add_dialog_set', 6735, NULL, '{"slot": 440}', 0, 0);

-- Chain 8084 RECONSTRUCTION: Loth'ta's turn-in (dialog 5808). The coded orders
-- (item 6779) are taken back, the bind is removed, then the mission completes.
-- Unbind runs before complete_mission (C5 rule 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8084, '1645 - Step 4907 click Loth''ta (5808): take item 6779, unbind 6735, complete 1645', 'mission', 1645, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8084, 'interact_tag', 'Dakara_E1_Lothta', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8084, 'world', 61, NULL, 'eq', NULL, 0),
  (8084, 'step_status', 1645, '4907', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8084, 'display_dialog', 5808, NULL, '{}', 0, 0),
  (8084, 'remove_item', 6779, NULL, '{"qty": 1}', 0, 1),
  (8084, 'remove_dialog_set', 6735, NULL, '{"slot": 440}', 0, 2),
  (8084, 'complete_mission', 1645, NULL, '{}', 0, 3);
