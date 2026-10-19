-- Dakara_E1 rebuild: mission 1649 "Moh'katan" (offer, Bra'tac, the radio call, turn-in).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-14a).
-- Chain ID range: 8200-8219 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8200 RECONSTRUCTION (D-DK25): 1649 offer by radio. Moh'katan has no
-- offering NPC (blurb 5984 has flags 0), so the offer is narrator text 5831
-- shown as a radio dialog two seconds after the player enters Dakara_E1 once
-- 1648 is completed and 1649 is not yet active. Repeats on each entry until
-- the offer is accepted.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8200, '1649 - Offer: radio dialog 5831 after 2000 ms on Dakara_E1 when 1648 completed and 1649 not active', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8200, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8200, 'world', 61, NULL, 'eq', NULL, 0),
  (8200, 'mission_status', 1648, NULL, 'eq', 'completed', 1),
  (8200, 'mission_status', 1649, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8200, 'display_radio_dialog', 5831, NULL, '{}', 2000, 0);

-- Chain 8201 RECONSTRUCTION (D-DK25): closing the radio offer 5831 accepts
-- 1649. No world condition: a radio window can be closed after a flap trip.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8201, '1649 - Accept on radio offer 5831: accept 1649', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8201, 'dialog_choice', '5831', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8201, 'mission_status', 1648, NULL, 'eq', 'completed', 0),
  (8201, 'mission_status', 1649, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8201, 'accept_mission', 1649, NULL, '{}', 0, 0);

-- Chain 8202 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4923, Bra'tac restore.
-- Binds Bra'tac's daughter talk dsm 6758 (dialog 5832) on slot 59 in the
-- tent (world 62). The accept is in world 61, so this is the first bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8202, '1649 - Step 4923 restore: bind Bra''tac dsm 6758 (5832) on Dakara_E1_StoryRm', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8202, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8202, 'world', 62, NULL, 'eq', NULL, 0),
  (8202, 'step_status', 1649, '4923', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8202, 'add_dialog_set', 6758, NULL, '{"slot": 59}', 0, 0);

-- Chain 8203 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4923, Bra'tac on his
-- daughter (dialog 5832). Shows the talk, advances to 4921 and unbinds dsm
-- 6758 before the talk closes (C5 rule 4).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8203, '1649 - Step 4923: Bra''tac 5832 (daughter), advance to 4921, unbind dsm 6758', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8203, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8203, 'world', 62, NULL, 'eq', NULL, 0),
  (8203, 'step_status', 1649, '4923', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8203, 'display_dialog', 5832, NULL, '{}', 0, 0),
  (8203, 'advance_step', 1649, '4921', '{}', 0, 1),
  (8203, 'remove_dialog_set', 6758, NULL, '{"slot": 59}', 0, 2);

-- Chain 8204 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4921, closing Bra'tac's
-- talk (dialog 5832) makes the radio call to Teal'c (5833, no NPC) and
-- advances to 4922. Both actions run at once; the radio is not delayed.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8204, '1649 - Step 4921: close Bra''tac 5832, radio 5833 to Teal''c, advance to 4922', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8204, 'dialog_choice', '5832', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8204, 'world', 62, NULL, 'eq', NULL, 0),
  (8204, 'step_status', 1649, '4921', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8204, 'display_radio_dialog', 5833, NULL, '{}', 0, 0),
  (8204, 'advance_step', 1649, '4922', '{}', 0, 1);

-- Chain 8205 PROJECT_FINAL_RECONSTRUCTION: recovery on Dakara_E1 at step 4921
-- (the player left before closing 5832). The advance runs at once so the
-- step gate closes; only the radio waits two seconds. DK-14b's 8222 co-fires
-- on this event to bind the terminal and drops.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8205, '1649 - Step 4921 recovery on Dakara_E1: radio 5833 after 2000 ms, advance to 4922', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8205, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8205, 'world', 61, NULL, 'eq', NULL, 0),
  (8205, 'step_status', 1649, '4921', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8205, 'display_radio_dialog', 5833, NULL, '{}', 2000, 0),
  (8205, 'advance_step', 1649, '4922', '{}', 0, 1);

-- Chain 8206 PROJECT_FINAL_RECONSTRUCTION: the same recovery as 8205 in the
-- tent, for a player who left Bra'tac's talk open or closed it before the
-- radio could play.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8206, '1649 - Step 4921 recovery on Dakara_E1_StoryRm: radio 5833 after 2000 ms, advance to 4922', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8206, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8206, 'world', 62, NULL, 'eq', NULL, 0),
  (8206, 'step_status', 1649, '4921', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8206, 'display_radio_dialog', 5833, NULL, '{}', 2000, 0),
  (8206, 'advance_step', 1649, '4922', '{}', 0, 1);

-- Chain 8207 PROJECT_FINAL_RECONSTRUCTION: 1649 step 4920, Bra'tac's turn-in
-- restore. Binds dsm 6765 (dialog 5838, turn-in) on slot 59 in the tent. The
-- advance to 4920 is DK-14b's (8224, in world 61).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8207, '1649 - Step 4920 restore: bind Bra''tac dsm 6765 (5838) on Dakara_E1_StoryRm', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8207, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8207, 'world', 62, NULL, 'eq', NULL, 0),
  (8207, 'step_status', 1649, '4920', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8207, 'add_dialog_set', 6765, NULL, '{"slot": 59}', 0, 0);

-- Chain 8208 PROJECT_FINAL_RECONSTRUCTION: 1649 turn-in. Bra'tac (dialog 5838)
-- takes the orders 6782 and the decoded message 6783, unbinds dsm 6765
-- before the mission completes (C5 rule 5), and completes 1649.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8208, '1649 - Turn-in: Bra''tac 5838, take orders 6782 and message 6783, unbind dsm 6765, complete 1649', 'mission', 1649, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8208, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8208, 'world', 62, NULL, 'eq', NULL, 0),
  (8208, 'step_status', 1649, '4920', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8208, 'display_dialog', 5838, NULL, '{}', 0, 0),
  (8208, 'remove_item', 6782, NULL, '{"qty": 1}', 0, 1),
  (8208, 'remove_item', 6783, NULL, '{"qty": 1}', 0, 2),
  (8208, 'remove_dialog_set', 6765, NULL, '{"slot": 59}', 0, 3),
  (8208, 'complete_mission', 1649, NULL, '{}', 0, 4);
