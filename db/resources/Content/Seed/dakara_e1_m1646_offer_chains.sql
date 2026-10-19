-- Dakara_E1 rebuild: mission 1646, offer, accept and turn-in.
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-12b).
-- Chain ID range: 8120-8139 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8120 PROJECT_FINAL_RECONSTRUCTION: 1646 offer. Once 1645 is completed
-- and 1646 is not yet active, Moh'katan (dsm 6737, dialog 5981 "available")
-- is bound on entering the tent (world 62). The same restore runs on every
-- load until the offer is taken, so a relog keeps the offer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8120, '1646 - Offer: bind Moh''katan dsm 6737 (5981) on Dakara_E1_StoryRm when 1645 completed and 1646 not active', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8120, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8120, 'world', 62, NULL, 'eq', NULL, 0),
  (8120, 'mission_status', 1645, NULL, 'eq', 'completed', 1),
  (8120, 'mission_status', 1646, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8120, 'add_dialog_set', 6737, NULL, '{"slot": 54}', 0, 0);

-- Chain 8121 PROJECT_FINAL_RECONSTRUCTION: 1646 accept. Choosing the offer
-- (dialog 5981) accepts 1646, which starts at step 4908 (the five drops,
-- DK-12c). The briefing 5819 plays, the coded orders 6780 are handed over
-- and kept (mission 1649 decodes them later, DK-14b), and the offer binding
-- comes off Moh'katan's slot 54.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8121, '1646 - Accept on offer 5981: accept, brief 5819, give coded orders 6780, unbind dsm 6737', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8121, 'dialog_choice', '5981', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8121, 'world', 62, NULL, 'eq', NULL, 0),
  (8121, 'mission_status', 1645, NULL, 'eq', 'completed', 1),
  (8121, 'mission_status', 1646, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8121, 'accept_mission', 1646, NULL, '{}', 0, 0),
  (8121, 'display_dialog', 5819, NULL, '{}', 0, 1),
  (8121, 'add_item', 6780, NULL, '{"container": 0, "qty": 1}', 0, 2),
  (8121, 'remove_dialog_set', 6737, NULL, '{"slot": 54}', 0, 3);

-- Chain 8122 PROJECT_FINAL_RECONSTRUCTION: 1646 turn-in restore. After the
-- fifth drop (DK-12c advances 4908 to 4909 in world 61), entering the tent
-- binds Moh'katan's turn-in dsm 6744 (dialog 5820) on slot 54. Step 4909 is
-- owned here; the advancing chain is DK-12c's (C5 rule 6 does not apply:
-- the bind happens in world 62, where Moh'katan stands).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8122, '1646 - Turn-in restore: bind Moh''katan dsm 6744 (5820) on Dakara_E1_StoryRm at step 4909', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8122, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8122, 'world', 62, NULL, 'eq', NULL, 0),
  (8122, 'step_status', 1646, '4909', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8122, 'add_dialog_set', 6744, NULL, '{"slot": 54}', 0, 0);

-- Chain 8123 PROJECT_FINAL_RECONSTRUCTION: 1646 turn-in. Clicking Moh'katan
-- at step 4909 plays the turn-in 5820, unbinds dsm 6744 and completes 1646.
-- The unbind runs before complete_mission (C5 rule 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8123, '1646 - Turn-in: Moh''katan 5820, unbind dsm 6744, complete 1646 at step 4909', 'mission', 1646, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8123, 'interact_tag', 'Dakara_E1_StoryRm_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8123, 'world', 62, NULL, 'eq', NULL, 0),
  (8123, 'step_status', 1646, '4909', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8123, 'display_dialog', 5820, NULL, '{}', 0, 0),
  (8123, 'remove_dialog_set', 6744, NULL, '{"slot": 54}', 0, 1),
  (8123, 'complete_mission', 1646, NULL, '{}', 0, 2);
