-- Dakara_E1 rebuild: mission 1648 "Loth'ta's Withdrawal" (offer, Loth'ta, turn-in).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-13).
-- Chain ID range: 8180-8199 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8180 PROJECT_FINAL_RECONSTRUCTION: 1648 offer. When 1647 completes in
-- the tent (world 62), Bra'tac (dsm 6752, dialog 5983 "available") is bound
-- on slot 59. If 1647 completes on Dakara_E1 instead, chain 8181 binds it on
-- entry.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8180, '1648 - Offer: bind Bra''tac dsm 6752 (5983) on 1647 completed in the tent when 1648 not active', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8180, 'mission_completed', '1647', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8180, 'world', 62, NULL, 'eq', NULL, 0),
  (8180, 'mission_status', 1648, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8180, 'add_dialog_set', 6752, NULL, '{"slot": 59}', 0, 0);

-- Chain 8181 PROJECT_FINAL_RECONSTRUCTION: 1648 offer restore. Entering the
-- tent with 1647 completed and 1648 not yet active binds Bra'tac's offer
-- dsm 6752 on slot 59 again (binds are in memory and a relog drops them).
-- Disjoint from 8180: that chain runs on mission_completed, this on
-- player_loaded.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8181, '1648 - Offer restore: bind Bra''tac dsm 6752 (5983) on Dakara_E1_StoryRm when 1647 completed and 1648 not active', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8181, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8181, 'world', 62, NULL, 'eq', NULL, 0),
  (8181, 'mission_status', 1647, NULL, 'eq', 'completed', 1),
  (8181, 'mission_status', 1648, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8181, 'add_dialog_set', 6752, NULL, '{"slot": 59}', 0, 0);

-- Chain 8182 PROJECT_FINAL_RECONSTRUCTION: 1648 accept. Choosing Bra'tac's
-- offer (dialog 5983) accepts 1648, which starts at step 4919 (speak to
-- Loth'ta). The briefing 5828 (dsm 6753, flags 0) plays here and is never
-- bound; the offer binding comes off Bra'tac's slot 59.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8182, '1648 - Accept on offer 5983: accept, brief 5828, unbind dsm 6752', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8182, 'dialog_choice', '5983', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8182, 'world', 62, NULL, 'eq', NULL, 0),
  (8182, 'mission_status', 1647, NULL, 'eq', 'completed', 1),
  (8182, 'mission_status', 1648, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8182, 'accept_mission', 1648, NULL, '{}', 0, 0),
  (8182, 'display_dialog', 5828, NULL, '{}', 0, 1),
  (8182, 'remove_dialog_set', 6752, NULL, '{"slot": 59}', 0, 2);

-- Chain 8183 PROJECT_FINAL_RECONSTRUCTION: 1648 step 4919, Loth'ta bind. In
-- the drops' world (61) at step 4919, Loth'ta (dsm 6754, active, dialog
-- 5829) is bound on slot 440. The accept runs in the tent, so this restore
-- is Loth'ta's only bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8183, '1648 - Step 4919: bind Loth''ta dsm 6754 (5829) on Dakara_E1 at step 4919', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8183, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8183, 'world', 61, NULL, 'eq', NULL, 0),
  (8183, 'step_status', 1648, '4919', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8183, 'add_dialog_set', 6754, NULL, '{"slot": 440}', 0, 0);

-- Chain 8184 PROJECT_FINAL_RECONSTRUCTION: 1648 step 4919, Loth'ta refuses.
-- Clicking Loth'ta at step 4919 plays 5829, gives the message 6781 "Message
-- to Bra'tac", advances to step 4918 and unbinds dsm 6754. Step 4918 is
-- Bra'tac's (world 62), bound by chain 8185 on entry, so no co-firing chain
-- is needed (C5 rule 6).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8184, '1648 - Step 4919: Loth''ta 5829, give item 6781, advance to 4918, unbind dsm 6754', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8184, 'interact_tag', 'Dakara_E1_Lothta', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8184, 'world', 61, NULL, 'eq', NULL, 0),
  (8184, 'step_status', 1648, '4919', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8184, 'display_dialog', 5829, NULL, '{}', 0, 0),
  (8184, 'add_item', 6781, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (8184, 'advance_step', 1648, '4918', '{}', 0, 2),
  (8184, 'remove_dialog_set', 6754, NULL, '{"slot": 440}', 0, 3);

-- Chain 8185 PROJECT_FINAL_RECONSTRUCTION: 1648 step 4918, Bra'tac turn-in
-- bind. In the tent (world 62) at step 4918, Bra'tac (dsm 6755, turn-in,
-- dialog 5830) is bound on slot 59. Chain 8184 advances into this step in
-- world 61, so the bind happens here on entry.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8185, '1648 - Step 4918: bind Bra''tac dsm 6755 (5830) on Dakara_E1_StoryRm at step 4918', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8185, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8185, 'world', 62, NULL, 'eq', NULL, 0),
  (8185, 'step_status', 1648, '4918', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8185, 'add_dialog_set', 6755, NULL, '{"slot": 59}', 0, 0);

-- Chain 8186 PROJECT_FINAL_RECONSTRUCTION: 1648 turn-in. Clicking Bra'tac at
-- step 4918 plays 5830, takes the message 6781, unbinds dsm 6755 and
-- completes 1648. The unbind runs before complete_mission (C5 rule 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8186, '1648 - Step 4918: Bra''tac 5830, take item 6781, unbind dsm 6755, complete 1648', 'mission', 1648, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8186, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8186, 'world', 62, NULL, 'eq', NULL, 0),
  (8186, 'step_status', 1648, '4918', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8186, 'display_dialog', 5830, NULL, '{}', 0, 0),
  (8186, 'remove_item', 6781, NULL, '{"qty": 1}', 0, 1),
  (8186, 'remove_dialog_set', 6755, NULL, '{"slot": 59}', 0, 2),
  (8186, 'complete_mission', 1648, NULL, '{}', 0, 3);
