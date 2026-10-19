-- Beta Site E1 (world 23, Beta_Site_Evo_1) - Covert Observations: recording the four subjects (UB-35, mission 477).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: mission 477 step S1744 (objectives O2060 Carter on 2300, O2061 Debeaux on 2304, O2062 Felger on 2301,
-- O2063 Satterfield on 2302) and step S1745 (O2064, the return to Mimir). Mission 477 and its objective texts are
-- RESTORED_SOURCE (mission_objectives.sql, mission_tasks.sql; mission-seal.md 477 table). Dialogs: blurb 1999 (dsm 2457,
-- flag 134217728, C5b), offer 1997 (dsm 2456, Accept, moved by UB-DP), completion 1996 (dsm 4408). Item 2646 Asgard
-- Recording Device. The subject-record mechanism (binds, the partition, the return) is CIMMERIA_RECONSTRUCTION:
-- the four subjects have interaction_type 0, so each is bound per player with dsm 2457 (HELD-B15, Q12).
-- Gate: Asgard (archetype 5) only. Omega UO-16 is HELD (HELD-B16), so this is replay-tested only (D-UB13).
-- F19: the four subjects use a partition (P6, N = 4). No chain completes the last open objective; the return
-- chain advances the step with advance_step.
-- Guards: chain_replay_tests/beta_e1/m477/.

-- Chain 23435: player_loaded and mission_completed 443, Asgard, 443 completed, 477 not_active -> bind the blurb 2457 on Mimir.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23435, 'Beta_Site_Evo_1 - Mimir blurb bind for Covert Observations (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23435, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23435, 'mission_completed', '443', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23435, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23435, 'mission_status', 443, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23435, 'mission_status', 477, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23435, 'add_dialog_set', 2457, NULL, '{"slot": 2305, "mission_id": 477}', 0, 0);

-- Chain 23436: interact_tag BetaE1_Mimir, Asgard, 443 completed, 477 not_active -> show the offer 1997.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23436, 'Beta_Site_Evo_1 - Mimir offers Covert Observations: show the offer (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23436, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23436, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23436, 'mission_status', 443, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23436, 'mission_status', 477, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23436, 'display_dialog', 1997, NULL, '{}', 0, 0);

-- Chain 23437: dialog_choice 1997 (Accept), Asgard, 443 completed, 477 not_active -> accept 477, drop the blurb,
-- give the recording device, bind the four subjects (2300, 2304, 2301, 2302) with dsm 2457.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23437, 'Beta_Site_Evo_1 - accept Covert Observations, bind the four subjects (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23437, 'dialog_choice', '1997', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23437, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23437, 'mission_status', 443, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23437, 'mission_status', 477, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'accept_mission', 477, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'remove_dialog_set', 2457, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'add_item', 2646, NULL, '{"container": 0, "qty": 1}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'add_dialog_set', 2457, NULL, '{"slot": 2300, "mission_id": 477}', 0, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'add_dialog_set', 2457, NULL, '{"slot": 2304, "mission_id": 477}', 0, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'add_dialog_set', 2457, NULL, '{"slot": 2301, "mission_id": 477}', 0, 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23437, 'add_dialog_set', 2457, NULL, '{"slot": 2302, "mission_id": 477}', 0, 6);

-- Chains 23438-23453: the subject partition (P6, N = 4, F19). For subject X (tag, slot), a, b, c are the other three
-- objectives in ascending order. Each subject's chains are tap:<X tag>, Asgard, S1744 active, O<X> active, then:
-- mark-a (O<a> active), mark-b (O<a> completed, O<b> active), mark-c (O<a>, O<b> completed, O<c> active): complete O<X>
-- and unbind the subject. last (all three completed): unbind the subject, advance to S1745 (no complete_objective),
-- and bind the completion dialog 4408 on Mimir (slot 2305).

-- Carter (O2060, slot 2300, tag BetaE1_Carter); a = O2061, b = O2062, c = O2063.

-- Chain 23438: record Carter, mark-a (O2061 active) -> complete O2060, unbind 2300.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23438, 'Beta_Site_Evo_1 - record Carter, mark-a (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23438, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23438, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23438, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23438, 'objective_status', 477, '2060', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23438, 'objective_status', 477, '2061', 'eq', 'active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23438, 'complete_objective', 477, '2060', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23438, 'remove_dialog_set', 2457, NULL, '{"slot": 2300}', 0, 1);

-- Chain 23439: record Carter, mark-b (O2061 completed, O2062 active) -> complete O2060, unbind 2300.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23439, 'Beta_Site_Evo_1 - record Carter, mark-b (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23439, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23439, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23439, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23439, 'objective_status', 477, '2060', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23439, 'objective_status', 477, '2061', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23439, 'objective_status', 477, '2062', 'eq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23439, 'complete_objective', 477, '2060', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23439, 'remove_dialog_set', 2457, NULL, '{"slot": 2300}', 0, 1);

-- Chain 23440: record Carter, mark-c (O2061, O2062 completed, O2063 active) -> complete O2060, unbind 2300.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23440, 'Beta_Site_Evo_1 - record Carter, mark-c (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23440, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23440, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23440, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23440, 'objective_status', 477, '2060', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23440, 'objective_status', 477, '2061', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23440, 'objective_status', 477, '2062', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23440, 'objective_status', 477, '2063', 'eq', 'active', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23440, 'complete_objective', 477, '2060', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23440, 'remove_dialog_set', 2457, NULL, '{"slot": 2300}', 0, 1);

-- Chain 23441: record Carter, last (O2061, O2062, O2063 completed) -> unbind 2300, advance to S1745, bind 4408 on Mimir.
-- No complete_objective here (F19).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23441, 'Beta_Site_Evo_1 - record Carter, last subject, return to Mimir (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23441, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23441, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23441, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23441, 'objective_status', 477, '2060', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23441, 'objective_status', 477, '2061', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23441, 'objective_status', 477, '2062', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23441, 'objective_status', 477, '2063', 'eq', 'completed', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23441, 'remove_dialog_set', 2457, NULL, '{"slot": 2300}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23441, 'advance_step', 477, '1745', '{}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23441, 'add_dialog_set', 4408, NULL, '{"slot": 2305, "mission_id": 477}', 0, 2);

-- Debeaux (O2061, slot 2304, tag BetaE1_Debeaux); a = O2060, b = O2062, c = O2063.

-- Chain 23442: record Debeaux, mark-a (O2060 active) -> complete O2061, unbind 2304.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23442, 'Beta_Site_Evo_1 - record Debeaux, mark-a (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23442, 'interact_tag', 'BetaE1_Debeaux', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23442, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23442, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23442, 'objective_status', 477, '2061', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23442, 'objective_status', 477, '2060', 'eq', 'active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23442, 'complete_objective', 477, '2061', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23442, 'remove_dialog_set', 2457, NULL, '{"slot": 2304}', 0, 1);

-- Chain 23443: record Debeaux, mark-b (O2060 completed, O2062 active) -> complete O2061, unbind 2304.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23443, 'Beta_Site_Evo_1 - record Debeaux, mark-b (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23443, 'interact_tag', 'BetaE1_Debeaux', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23443, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23443, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23443, 'objective_status', 477, '2061', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23443, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23443, 'objective_status', 477, '2062', 'eq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23443, 'complete_objective', 477, '2061', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23443, 'remove_dialog_set', 2457, NULL, '{"slot": 2304}', 0, 1);

-- Chain 23444: record Debeaux, mark-c (O2060, O2062 completed, O2063 active) -> complete O2061, unbind 2304.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23444, 'Beta_Site_Evo_1 - record Debeaux, mark-c (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23444, 'interact_tag', 'BetaE1_Debeaux', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23444, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23444, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23444, 'objective_status', 477, '2061', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23444, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23444, 'objective_status', 477, '2062', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23444, 'objective_status', 477, '2063', 'eq', 'active', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23444, 'complete_objective', 477, '2061', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23444, 'remove_dialog_set', 2457, NULL, '{"slot": 2304}', 0, 1);

-- Chain 23445: record Debeaux, last (O2060, O2062, O2063 completed) -> unbind 2304, advance to S1745, bind 4408 on Mimir.
-- No complete_objective here (F19).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23445, 'Beta_Site_Evo_1 - record Debeaux, last subject, return to Mimir (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23445, 'interact_tag', 'BetaE1_Debeaux', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23445, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23445, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23445, 'objective_status', 477, '2061', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23445, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23445, 'objective_status', 477, '2062', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23445, 'objective_status', 477, '2063', 'eq', 'completed', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23445, 'remove_dialog_set', 2457, NULL, '{"slot": 2304}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23445, 'advance_step', 477, '1745', '{}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23445, 'add_dialog_set', 4408, NULL, '{"slot": 2305, "mission_id": 477}', 0, 2);

-- Felger (O2062, slot 2301, tag BetaE1_Felger); a = O2060, b = O2061, c = O2063.

-- Chain 23446: record Felger, mark-a (O2060 active) -> complete O2062, unbind 2301.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23446, 'Beta_Site_Evo_1 - record Felger, mark-a (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23446, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23446, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23446, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23446, 'objective_status', 477, '2062', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23446, 'objective_status', 477, '2060', 'eq', 'active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23446, 'complete_objective', 477, '2062', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23446, 'remove_dialog_set', 2457, NULL, '{"slot": 2301}', 0, 1);

-- Chain 23447: record Felger, mark-b (O2060 completed, O2061 active) -> complete O2062, unbind 2301.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23447, 'Beta_Site_Evo_1 - record Felger, mark-b (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23447, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23447, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23447, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23447, 'objective_status', 477, '2062', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23447, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23447, 'objective_status', 477, '2061', 'eq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23447, 'complete_objective', 477, '2062', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23447, 'remove_dialog_set', 2457, NULL, '{"slot": 2301}', 0, 1);

-- Chain 23448: record Felger, mark-c (O2060, O2061 completed, O2063 active) -> complete O2062, unbind 2301.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23448, 'Beta_Site_Evo_1 - record Felger, mark-c (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23448, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23448, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23448, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23448, 'objective_status', 477, '2062', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23448, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23448, 'objective_status', 477, '2061', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23448, 'objective_status', 477, '2063', 'eq', 'active', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23448, 'complete_objective', 477, '2062', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23448, 'remove_dialog_set', 2457, NULL, '{"slot": 2301}', 0, 1);

-- Chain 23449: record Felger, last (O2060, O2061, O2063 completed) -> unbind 2301, advance to S1745, bind 4408 on Mimir.
-- No complete_objective here (F19).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23449, 'Beta_Site_Evo_1 - record Felger, last subject, return to Mimir (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23449, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23449, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23449, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23449, 'objective_status', 477, '2062', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23449, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23449, 'objective_status', 477, '2061', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23449, 'objective_status', 477, '2063', 'eq', 'completed', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23449, 'remove_dialog_set', 2457, NULL, '{"slot": 2301}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23449, 'advance_step', 477, '1745', '{}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23449, 'add_dialog_set', 4408, NULL, '{"slot": 2305, "mission_id": 477}', 0, 2);

-- Satterfield (O2063, slot 2302, tag BetaE1_Satterfield); a = O2060, b = O2061, c = O2062.

-- Chain 23450: record Satterfield, mark-a (O2060 active) -> complete O2063, unbind 2302.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23450, 'Beta_Site_Evo_1 - record Satterfield, mark-a (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23450, 'interact_tag', 'BetaE1_Satterfield', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23450, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23450, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23450, 'objective_status', 477, '2063', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23450, 'objective_status', 477, '2060', 'eq', 'active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23450, 'complete_objective', 477, '2063', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23450, 'remove_dialog_set', 2457, NULL, '{"slot": 2302}', 0, 1);

-- Chain 23451: record Satterfield, mark-b (O2060 completed, O2061 active) -> complete O2063, unbind 2302.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23451, 'Beta_Site_Evo_1 - record Satterfield, mark-b (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23451, 'interact_tag', 'BetaE1_Satterfield', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23451, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23451, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23451, 'objective_status', 477, '2063', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23451, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23451, 'objective_status', 477, '2061', 'eq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23451, 'complete_objective', 477, '2063', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23451, 'remove_dialog_set', 2457, NULL, '{"slot": 2302}', 0, 1);

-- Chain 23452: record Satterfield, mark-c (O2060, O2061 completed, O2062 active) -> complete O2063, unbind 2302.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23452, 'Beta_Site_Evo_1 - record Satterfield, mark-c (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23452, 'interact_tag', 'BetaE1_Satterfield', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23452, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23452, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23452, 'objective_status', 477, '2063', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23452, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23452, 'objective_status', 477, '2061', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23452, 'objective_status', 477, '2062', 'eq', 'active', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23452, 'complete_objective', 477, '2063', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23452, 'remove_dialog_set', 2457, NULL, '{"slot": 2302}', 0, 1);

-- Chain 23453: record Satterfield, last (O2060, O2061, O2062 completed) -> unbind 2302, advance to S1745, bind 4408 on Mimir.
-- No complete_objective here (F19).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23453, 'Beta_Site_Evo_1 - record Satterfield, last subject, return to Mimir (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23453, 'interact_tag', 'BetaE1_Satterfield', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23453, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23453, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23453, 'objective_status', 477, '2063', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23453, 'objective_status', 477, '2060', 'eq', 'completed', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23453, 'objective_status', 477, '2061', 'eq', 'completed', 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23453, 'objective_status', 477, '2062', 'eq', 'completed', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23453, 'remove_dialog_set', 2457, NULL, '{"slot": 2302}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23453, 'advance_step', 477, '1745', '{}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23453, 'add_dialog_set', 4408, NULL, '{"slot": 2305, "mission_id": 477}', 0, 2);

-- Chains 23454-23457: login restores while the subject's objective is open (S1744 active). Each re-adds the subject bind.

-- Chain 23454: login, Asgard, S1744 active, O2060 (Carter) active -> bind 2457 on Carter (2300).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23454, 'Beta_Site_Evo_1 - restore the Carter subject bind (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23454, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23454, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23454, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23454, 'objective_status', 477, '2060', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23454, 'add_dialog_set', 2457, NULL, '{"slot": 2300, "mission_id": 477}', 0, 0);

-- Chain 23455: login, Asgard, S1744 active, O2061 (Debeaux) active -> bind 2457 on Debeaux (2304).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23455, 'Beta_Site_Evo_1 - restore the Debeaux subject bind (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23455, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23455, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23455, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23455, 'objective_status', 477, '2061', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23455, 'add_dialog_set', 2457, NULL, '{"slot": 2304, "mission_id": 477}', 0, 0);

-- Chain 23456: login, Asgard, S1744 active, O2062 (Felger) active -> bind 2457 on Felger (2301).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23456, 'Beta_Site_Evo_1 - restore the Felger subject bind (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23456, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23456, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23456, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23456, 'objective_status', 477, '2062', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23456, 'add_dialog_set', 2457, NULL, '{"slot": 2301, "mission_id": 477}', 0, 0);

-- Chain 23457: login, Asgard, S1744 active, O2063 (Satterfield) active -> bind 2457 on Satterfield (2302).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23457, 'Beta_Site_Evo_1 - restore the Satterfield subject bind (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23457, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23457, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23457, 'step_status', 477, '1744', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23457, 'objective_status', 477, '2063', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23457, 'add_dialog_set', 2457, NULL, '{"slot": 2302, "mission_id": 477}', 0, 0);

-- Chain 23458: login, Asgard, S1745 active -> bind the completion dialog 4408 on Mimir (2305).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23458, 'Beta_Site_Evo_1 - Mimir completion bind for Covert Observations (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23458, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23458, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23458, 'step_status', 477, '1745', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23458, 'add_dialog_set', 4408, NULL, '{"slot": 2305, "mission_id": 477}', 0, 0);

-- Chain 23459: interact_tag BetaE1_Mimir, Asgard, S1745 active -> show the completion 1996, drop the bind and the
-- recording device, complete 477.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23459, 'Beta_Site_Evo_1 - Mimir turn-in for Covert Observations (UB-35)', 'mission', 477, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23459, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23459, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23459, 'step_status', 477, '1745', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23459, 'display_dialog', 1996, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23459, 'remove_dialog_set', 4408, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23459, 'remove_item', 2646, NULL, '{"qty": 1}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23459, 'complete_mission', 477, NULL, '{}', 0, 3);
