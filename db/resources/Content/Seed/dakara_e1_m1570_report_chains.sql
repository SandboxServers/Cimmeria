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
