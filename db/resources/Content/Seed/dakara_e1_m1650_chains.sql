-- Dakara_E1 rebuild: mission 1650 "Confront Moh'katan".
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-15).
-- Chain ID range: 8250-8289 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8250 PROJECT_FINAL_RECONSTRUCTION: 1650 offer. Once 1649 is completed
-- in the tent (world 62) and 1650 is not yet active, Bra'tac (dsm 6767, dialog
-- 5985 "available") is bound on slot 59.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8250, '1650 - Offer: bind Bra''tac dsm 6767 (5985) after 1649 completes in Dakara_E1_StoryRm', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8250, 'mission_completed', '1649', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8250, 'world', 62, NULL, 'eq', NULL, 0),
  (8250, 'mission_status', 1650, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8250, 'add_dialog_set', 6767, NULL, '{"slot": 59}', 0, 0);

-- Chain 8251 PROJECT_FINAL_RECONSTRUCTION: 1650 offer restore. On loading the
-- tent with 1649 completed and 1650 not yet active, the offer binding returns
-- to Bra'tac's slot 59, so a relog keeps the offer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8251, '1650 - Offer restore: bind Bra''tac dsm 6767 (5985) on Dakara_E1_StoryRm when 1649 completed and 1650 not active', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8251, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8251, 'world', 62, NULL, 'eq', NULL, 0),
  (8251, 'mission_status', 1649, NULL, 'eq', 'completed', 1),
  (8251, 'mission_status', 1650, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8251, 'add_dialog_set', 6767, NULL, '{"slot": 59}', 0, 0);

-- Chain 8252 PROJECT_FINAL_RECONSTRUCTION: 1650 accept. Choosing the offer
-- (dialog 5985) accepts 1650, which starts at step 4927 (meet Bra'tac at
-- Moh'katan's tent). The briefing 5839 plays and the offer binding comes off
-- Bra'tac's slot 59.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8252, '1650 - Accept on offer 5985: accept, brief 5839, unbind dsm 6767', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8252, 'dialog_choice', '5985', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8252, 'world', 62, NULL, 'eq', NULL, 0),
  (8252, 'mission_status', 1649, NULL, 'eq', 'completed', 1),
  (8252, 'mission_status', 1650, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8252, 'accept_mission', 1650, NULL, '{}', 0, 0),
  (8252, 'display_dialog', 5839, NULL, '{}', 0, 1),
  (8252, 'remove_dialog_set', 6767, NULL, '{"slot": 59}', 0, 2);

-- Chain 8253 PROJECT_FINAL_RECONSTRUCTION: 1650 step 4927 to 4924. Entering
-- Moh'katan's tent (point set 2124 Dakara_E1.MohkatanTent, world 61) completes
-- "Meet Bra'tac at Moh'katan's tent" (D-DK28).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8253, '1650 - Step 4927: enter Dakara_E1.MohkatanTent on Dakara_E1, advance to 4924', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8253, 'enter_region', 'Dakara_E1.MohkatanTent', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8253, 'world', 61, NULL, 'eq', NULL, 0),
  (8253, 'step_status', 1650, '4927', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8253, 'advance_step', 1650, '4924', '{}', 0, 0);

-- Chain 8254 PROJECT_FINAL_RECONSTRUCTION: 1650 step 4924 restore. Inside the
-- tent (world 62, instanced) Moh'katan's confrontation dsm 6768 (dialog 5840)
-- is bound on slot 54 and Ba'al's hologram (template 443) is spawned for this
-- player at B-07. Runs on entry and after a relog; the spawn's tag guard
-- refuses a second hologram while one is live.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8254, '1650 - Step 4924 restore: bind Moh''katan dsm 6768 (5840) and spawn Ba''al hologram 443 on Dakara_E1_StoryRm', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8254, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8254, 'world', 62, NULL, 'eq', NULL, 0),
  (8254, 'step_status', 1650, '4924', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8254, 'add_dialog_set', 6768, NULL, '{"slot": 54}', 0, 0),
  (8254, 'spawn_entity', 443, 'Dakara_E1_StoryRm_BaalHologram', '{"x": 66.2, "y": -0.12, "z": 34.3, "heading": 1.5708, "is_stationary": true}', 0, 1);

-- Chain 8255 PROJECT_FINAL_RECONSTRUCTION: 1650 confrontation. Clicking
-- Moh'katan at step 4924 plays 5840 (Ba'al, Moh'katan and Bra'tac), moves the
-- step to 4926 and swaps Moh'katan's dsm 6768 for the terminal's flag carrier
-- 6760 on prop 454, which lights in the same room. The hologram stays until
-- the terminal is used (8258).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8255, '1650 - Confrontation: Moh''katan 5840, advance to 4926, unbind dsm 6768, bind terminal carrier 6760 on 454', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8255, 'interact_tag', 'Dakara_E1_StoryRm_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8255, 'world', 62, NULL, 'eq', NULL, 0),
  (8255, 'step_status', 1650, '4924', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8255, 'display_dialog', 5840, NULL, '{}', 0, 0),
  (8255, 'advance_step', 1650, '4926', '{}', 0, 1),
  (8255, 'remove_dialog_set', 6768, NULL, '{"slot": 54}', 0, 2),
  (8255, 'add_dialog_set', 6760, NULL, '{"slot": 454}', 0, 3);

-- Chain 8256 PROJECT_FINAL_RECONSTRUCTION: 1650 search order. Closing 5840
-- brings Bra'tac's radio line 5841 "Search the room" (no NPC, DK-06b). The
-- radio dialog is never bound.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8256, '1650 - Step 4926: after 5840, Bra''tac radio 5841 (search the room)', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8256, 'dialog_choice', '5840', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8256, 'world', 62, NULL, 'eq', NULL, 0),
  (8256, 'step_status', 1650, '4926', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8256, 'display_radio_dialog', 5841, NULL, '{}', 0, 0);

-- Chain 8257 PROJECT_FINAL_RECONSTRUCTION: 1650 step 4926 restore. After a
-- relog in the tent the terminal's carrier 6760 is bound on slot 454 again,
-- and Ba'al's hologram is spawned again (the tag guard keeps it single).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8257, '1650 - Step 4926 restore: bind terminal carrier 6760 on 454 and spawn Ba''al hologram 443 on Dakara_E1_StoryRm', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8257, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8257, 'world', 62, NULL, 'eq', NULL, 0),
  (8257, 'step_status', 1650, '4926', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8257, 'add_dialog_set', 6760, NULL, '{"slot": 454}', 0, 0),
  (8257, 'spawn_entity', 443, 'Dakara_E1_StoryRm_BaalHologram', '{"x": 66.2, "y": -0.12, "z": 34.3, "heading": 1.5708, "is_stationary": true}', 0, 1);

-- Chain 8258 PROJECT_FINAL_RECONSTRUCTION: 1650 terminal. Using the terminal
-- at step 4926 gives the Downloaded File 6787 (no dialog), advances to 4925,
-- unbinds the carrier 6760, despawns the hologram and binds Bra'tac's turn-in
-- dsm 6771 on slot 59 (he is in the same room). The unbind and despawn run
-- before the new bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8258, '1650 - Terminal: give file 6787, advance to 4925, unbind 6760 on 454, despawn hologram, bind Bra''tac dsm 6771 on 59', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8258, 'interact_tag', 'Dakara_E1_StoryRm_MohkatanTerminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8258, 'world', 62, NULL, 'eq', NULL, 0),
  (8258, 'step_status', 1650, '4926', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8258, 'add_item', 6787, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (8258, 'advance_step', 1650, '4925', '{}', 0, 1),
  (8258, 'remove_dialog_set', 6760, NULL, '{"slot": 454}', 0, 2),
  (8258, 'despawn_entity', NULL, 'Dakara_E1_StoryRm_BaalHologram', '{}', 0, 3),
  (8258, 'add_dialog_set', 6771, NULL, '{"slot": 59}', 0, 4);

-- Chain 8259 PROJECT_FINAL_RECONSTRUCTION: 1650 step 4925 restore. After a
-- relog at step 4925 Bra'tac's turn-in dsm 6771 is bound on slot 59 again.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8259, '1650 - Step 4925 restore: bind Bra''tac dsm 6771 (5842) on Dakara_E1_StoryRm', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8259, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8259, 'world', 62, NULL, 'eq', NULL, 0),
  (8259, 'step_status', 1650, '4925', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8259, 'add_dialog_set', 6771, NULL, '{"slot": 59}', 0, 0);

-- Chain 8260 PROJECT_FINAL_RECONSTRUCTION: 1650 turn-in. Clicking Bra'tac at
-- step 4925 plays 5842, takes the Downloaded File 6787 back, unbinds dsm 6771
-- and completes 1650. The unbind runs before complete_mission (C5 rule 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8260, '1650 - Turn-in: Bra''tac 5842, take file 6787, unbind dsm 6771, complete 1650 at step 4925', 'mission', 1650, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8260, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8260, 'world', 62, NULL, 'eq', NULL, 0),
  (8260, 'step_status', 1650, '4925', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8260, 'display_dialog', 5842, NULL, '{}', 0, 0),
  (8260, 'remove_item', 6787, NULL, '{"qty": 1}', 0, 1),
  (8260, 'remove_dialog_set', 6771, NULL, '{"slot": 59}', 0, 2),
  (8260, 'complete_mission', 1650, NULL, '{}', 0, 3);
