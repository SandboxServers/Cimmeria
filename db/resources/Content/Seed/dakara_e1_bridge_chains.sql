-- Dakara_E1 rebuild: mission 63100 "Home to Dakara", the Free Jaffa bridge (NEW CONTENT, D-DK1 F).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-10fc).
-- Chain ID range: 8075-8079 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8075 NEW CONTENT (D-DK1 F): first arrival on Dakara_E1 of a Free Jaffa
-- with no Dakara mission yet. Accepts 63100 "Home to Dakara" and binds Rak'nor
-- (spawn 8006, template 441) to his Jaffa dialog set 6890 (dialog 6110). Never
-- again once 63100, 1570 or 1645 is active or completed.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8075, '63100 - first arrival: accept Home to Dakara, bind Rak''nor', 'mission', 63100, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8075, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8075, 'world', 61, NULL, 'eq', NULL, 0),
  (8075, 'archetype', NULL, NULL, 'eq', '7', 1),
  (8075, 'mission_status', 63100, NULL, 'eq', 'not_active', 2),
  (8075, 'mission_status', 1570, NULL, 'eq', 'not_active', 3),
  (8075, 'mission_status', 1645, NULL, 'eq', 'not_active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8075, 'accept_mission', 63100, NULL, '{}', 0, 0),
  (8075, 'add_dialog_set', 6890, NULL, '{"slot": 441}', 0, 1);

-- Chain 8076 NEW CONTENT (D-DK1 F): restore Rak'nor's bind after a relog or a
-- tent trip while 63100 is at its step 63101. Binds are in memory only (C5
-- rule 4). Disjoint from 8075: 8075 needs 63100 not_active.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8076, '63100 - 63101: restore Rak''nor', 'mission', 63100, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8076, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8076, 'world', 61, NULL, 'eq', NULL, 0),
  (8076, 'archetype', NULL, NULL, 'eq', '7', 1),
  (8076, 'step_status', 63100, '63101', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8076, 'add_dialog_set', 6890, NULL, '{"slot": 441}', 0, 0);

-- Chain 8077 NEW CONTENT (D-DK1 F): the conversation with Rak'nor at the
-- Stargate (spawn 8006, tag Dakara_E1_Raknor) completes 63100. The chain shows
-- dialog 6110 itself, so the click never falls back to the dsm dialog. The
-- unbind runs before complete_mission (C5 rule 5), which fires mission_completed
-- chain 8040 inline.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8077, '63100 - 63101: Rak''nor 6110, complete', 'mission', 63100, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8077, 'interact_tag', 'Dakara_E1_Raknor', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8077, 'world', 61, NULL, 'eq', NULL, 0),
  (8077, 'archetype', NULL, NULL, 'eq', '7', 1),
  (8077, 'step_status', 63100, '63101', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8077, 'display_dialog', 6110, NULL, '{}', 0, 0),
  (8077, 'remove_dialog_set', 6890, NULL, '{"slot": 441}', 0, 1),
  (8077, 'complete_mission', 63100, NULL, '{}', 0, 2);
