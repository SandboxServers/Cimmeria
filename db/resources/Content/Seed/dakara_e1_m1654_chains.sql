-- Dakara_E1 rebuild: mission 1654 "Aftermath" (offer, accept and real gate travel to the SGC, crossing C3).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-33).
-- Chain ID range: 8430-8449 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8430 PROJECT_FINAL_RECONSTRUCTION: 1654 offer at the 1653 report. Completing
-- 1653 in the tent (world 62) binds Bra'tac's offer blurb 5987 (dsm 6780, available)
-- on slot 59. 1654 is not yet active. The 1653 unbind (DK-32a, chain 8363) runs
-- before this completion, so the slot is free.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8430, '1654 - Offer: bind Bra''tac dsm 6780 (5987) on slot 59 when 1653 completes in Dakara_E1_StoryRm', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8430, 'mission_completed', '1653', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8430, 'world', 62, NULL, 'eq', NULL, 0),
  (8430, 'mission_status', 1654, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8430, 'add_dialog_set', 6780, NULL, '{"slot": 59}', 0, 0);

-- Chain 8431 PROJECT_FINAL_RECONSTRUCTION: 1654 offer restore. After 1653 is
-- completed and 1654 is not yet active, entering the tent rebinds the offer
-- dsm 6780 on Bra'tac's slot 59. The same restore runs on every load until
-- the offer is taken, so a relog keeps the offer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8431, '1654 - Offer restore: bind Bra''tac dsm 6780 (5987) on Dakara_E1_StoryRm when 1653 completed and 1654 not active', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8431, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8431, 'world', 62, NULL, 'eq', NULL, 0),
  (8431, 'mission_status', 1653, NULL, 'eq', 'completed', 1),
  (8431, 'mission_status', 1654, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8431, 'add_dialog_set', 6780, NULL, '{"slot": 59}', 0, 0);

-- Chain 8432 PROJECT_FINAL_RECONSTRUCTION: 1654 accept, Tau'ri branch. Choosing
-- the offer (dialog 5987) for archetypes 1 to 6 accepts 1654, unbinds the offer
-- from slot 59 and shows the Tau'ri briefing 5850 (dsm 6781, four Accept screens).
-- SGC address for the C3 crossing (D-DK24, D-DK43): gate 27 is granted (idempotent).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8432, '1654 - Accept on offer 5987, Tau''ri archetypes 1-6: accept, unbind dsm 6780, brief 5850', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8432, 'dialog_choice', '5987', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8432, 'world', 62, NULL, 'eq', NULL, 0),
  (8432, 'mission_status', 1653, NULL, 'eq', 'completed', 1),
  (8432, 'mission_status', 1654, NULL, 'eq', 'not_active', 2),
  (8432, 'archetype', NULL, NULL, 'gte', '1', 3),
  (8432, 'archetype', NULL, NULL, 'lte', '6', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8432, 'accept_mission', 1654, NULL, '{}', 0, 0),
  (8432, 'remove_dialog_set', 6780, NULL, '{"slot": 59}', 0, 1),
  (8432, 'display_dialog', 5850, NULL, '{}', 0, 2),
  (8432, 'grant_stargate_address', 27, NULL, '{}', 0, 3);

-- Chain 8433 PROJECT_FINAL_RECONSTRUCTION: 1654 accept, Free Jaffa branch. The same
-- offer for archetype 7 and up (Free Jaffa 7, Praxis Jaffa 8, D-DK15) shows the
-- Jaffa briefing 5851 (dsm 6782, six Accept screens).
-- SGC address for the C3 crossing (D-DK24, D-DK43): gate 27 is granted (idempotent).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8433, '1654 - Accept on offer 5987, Free Jaffa archetype 7 and up: accept, unbind dsm 6780, brief 5851', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8433, 'dialog_choice', '5987', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8433, 'world', 62, NULL, 'eq', NULL, 0),
  (8433, 'mission_status', 1653, NULL, 'eq', 'completed', 1),
  (8433, 'mission_status', 1654, NULL, 'eq', 'not_active', 2),
  (8433, 'archetype', NULL, NULL, 'gte', '7', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8433, 'accept_mission', 1654, NULL, '{}', 0, 0),
  (8433, 'remove_dialog_set', 6780, NULL, '{"slot": 59}', 0, 1),
  (8433, 'display_dialog', 5851, NULL, '{}', 0, 2),
  (8433, 'grant_stargate_address', 27, NULL, '{}', 0, 3);

-- Chain 8434 PROJECT_FINAL_RECONSTRUCTION: 1654 step 4939, crossing C3. Dialling
-- SGC_W1 by gate, from any world (D-DK24, F41) while 4939 "Report back to
-- Gen. Hammond at the SGC" is active and objective 5961 is not yet done completes
-- 5961 only. Never advance_step: that would also close 5962, which the SGC side
-- (US-31) completes at Hammond. No teleport: the player arrives by the gate itself
-- (merge held for US-31). No chain matches the Dakara_E1_DHD click (F25), so the
-- dial window opens.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8434, '1654 - Step 4939 (crossing C3): crossed to SGC_W1 by gate, complete objective 5961 only', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8434, 'stargate_crossed', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8434, 'step_status', 1654, '4939', 'eq', 'active', 0),
  (8434, 'objective_status', 1654, '5961', 'neq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8434, 'complete_objective', 1654, '5961', '{}', 0, 0);
