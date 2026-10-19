-- Dakara_E1 rebuild: mission 1651 "Renewed Attack" (accept, Bra'tac, return).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-30a).
-- Chain ID range: 8290-8309 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8290 RECONSTRUCTION (D-DK26): 1651 has no blurb and no Accept dialog, so
-- it is accepted when 1650 is turned in, in the tent with Bra'tac in scope. The
-- status-report dialog set (dsm 6772, slot 59) is bound at the same time.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8290, '1651 - 1650 completed on Dakara_E1_StoryRm: accept 1651, bind 6772 to Bra''tac (59)', 'mission', 1651, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8290, 'mission_completed', '1650', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8290, 'world', 62, NULL, 'eq', NULL, 0),
  (8290, 'mission_status', 1651, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8290, 'accept_mission', 1651, NULL, '{}', 0, 0),
  (8290, 'add_dialog_set', 6772, NULL, '{"slot": 59}', 0, 1);

-- Chain 8291 RECONSTRUCTION: restore the Bra'tac status-report bind on entering
-- the tent at step 4932 (the bind does not survive a relog or world change).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8291, '1651 - Restore Bra''tac status report on Dakara_E1_StoryRm at 4932: bind 6772 to Bra''tac (59)', 'mission', 1651, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8291, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8291, 'world', 62, NULL, 'eq', NULL, 0),
  (8291, 'step_status', 1651, '4932', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8291, 'add_dialog_set', 6772, NULL, '{"slot": 59}', 0, 0);

-- Chain 8292 RECONSTRUCTION: the status report (dialog 5843, Bra'tac's "You have
-- seen the Hataks?") advances 1651 from 4932 to 4930 (the DHD to the SGC) and
-- unbinds the report offer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8292, '1651 - Status report to Bra''tac at 4932: display 5843, advance to 4930, unbind 6772', 'mission', 1651, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8292, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8292, 'world', 62, NULL, 'eq', NULL, 0),
  (8292, 'step_status', 1651, '4932', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8292, 'display_dialog', 5843, NULL, '{}', 0, 0),
  (8292, 'advance_step', 1651, '4930', '{}', 0, 1),
  (8292, 'remove_dialog_set', 6772, NULL, '{"slot": 59}', 0, 2);

-- Chain 8293 RECONSTRUCTION: back from the SGC at step 5171, the player arrives on
-- Dakara_E1 and walks into the tent; Bra'tac binds his "Did they send any help?"
-- report offer (dsm 6775, slot 59).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8293, '1651 - Return report on Dakara_E1_StoryRm at 5171: bind 6775 to Bra''tac (59)', 'mission', 1651, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8293, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8293, 'world', 62, NULL, 'eq', NULL, 0),
  (8293, 'step_status', 1651, '5171', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8293, 'add_dialog_set', 6775, NULL, '{"slot": 59}', 0, 0);

-- Chain 8294 RECONSTRUCTION: the return report (dialog 5847, "Did they send any
-- help?") unbinds its offer and completes 1651. The unbind comes before the
-- complete. Item 6784 (Naquadah Generator) stays with the player for 1652.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8294, '1651 - Return report to Bra''tac at 5171: display 5847, unbind 6775, complete 1651', 'mission', 1651, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8294, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8294, 'world', 62, NULL, 'eq', NULL, 0),
  (8294, 'step_status', 1651, '5171', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8294, 'display_dialog', 5847, NULL, '{}', 0, 0),
  (8294, 'remove_dialog_set', 6775, NULL, '{"slot": 59}', 0, 1),
  (8294, 'complete_mission', 1651, NULL, '{}', 0, 2);
