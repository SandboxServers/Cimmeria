-- Beta Site E1 (world 23, Beta_Site_Evo_1) - Reclamation: Mimir's five scanners (UB-31, mission 420).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: mission 420 steps S1455 (O1689, quota 5) and S1457 (O1691). S1457 is never entered,
-- because the turn-in completes the mission from S1455 (C3, mission-seal.md). Mission 420 is RESTORED_SOURCE
-- (mission_objectives.sql). The collect step is CIMMERIA_RECONSTRUCTION.
-- Dialogs: blurb 1954 (dsm 2344), offer 1953 (Accept), turn-in 3057 (dsm 2343, flag 33554432).
-- Item 2647 Asgard Scanner (stack 5). Objects BetaE1_AsgardScanner_1.._5 are template 2330 (UB-02c).
-- Quota: counter beta_420_scanners (P9). Counters and once reset on relog (Q5), so 23351 clears the scanners.
-- Guards: chain_replay_tests/beta_e1/m420.rs.

-- Chain 23340: player_loaded and mission_completed 481, Asgard, 481 completed, 420 not_active -> bind the blurb 1954 (dsm 2344).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23340, 'Beta_Site_Evo_1 - Mimir blurb bind for Reclamation (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23340, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23340, 'mission_completed', '481', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23340, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23340, 'mission_status', 481, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23340, 'mission_status', 420, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23340, 'add_dialog_set', 2344, NULL, '{"slot": 2305, "mission_id": 420}', 0, 0);

-- Chain 23341: interact_tag BetaE1_Mimir, Asgard, 481 completed, 420 not_active -> show the offer 1953.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23341, 'Beta_Site_Evo_1 - Mimir offers Reclamation: show the offer (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23341, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23341, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23341, 'mission_status', 481, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23341, 'mission_status', 420, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23341, 'display_dialog', 1953, NULL, '{}', 0, 0);

-- Chain 23342: dialog_choice 1953 (Accept), Asgard, 481 completed, 420 not_active -> accept 420, swap the blurb for the turn-in bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23342, 'Beta_Site_Evo_1 - Accept Reclamation: start the scanner quota (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23342, 'dialog_choice', '1953', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23342, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23342, 'mission_status', 481, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23342, 'mission_status', 420, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23342, 'accept_mission', 420, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23342, 'remove_dialog_set', 2344, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23342, 'add_dialog_set', 2343, NULL, '{"slot": 2305, "mission_id": 420}', 0, 2);

-- Chains 23343-23347: interact_tag on scanner n (once per player), S1455 active -> grant one scanner and count it.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23343, 'Beta_Site_Evo_1 - Asgard Scanner 1: collect for Mimir (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23343, 'interact_tag', 'BetaE1_AsgardScanner_1', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23343, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23343, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23343, 'add_item', 2647, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23343, 'increment_counter', NULL, 'beta_420_scanners', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23344, 'Beta_Site_Evo_1 - Asgard Scanner 2: collect for Mimir (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23344, 'interact_tag', 'BetaE1_AsgardScanner_2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23344, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23344, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23344, 'add_item', 2647, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23344, 'increment_counter', NULL, 'beta_420_scanners', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23345, 'Beta_Site_Evo_1 - Asgard Scanner 3: collect for Mimir (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23345, 'interact_tag', 'BetaE1_AsgardScanner_3', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23345, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23345, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23345, 'add_item', 2647, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23345, 'increment_counter', NULL, 'beta_420_scanners', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23346, 'Beta_Site_Evo_1 - Asgard Scanner 4: collect for Mimir (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23346, 'interact_tag', 'BetaE1_AsgardScanner_4', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23346, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23346, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23346, 'add_item', 2647, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23346, 'increment_counter', NULL, 'beta_420_scanners', '{"amount": 1}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23347, 'Beta_Site_Evo_1 - Asgard Scanner 5: collect for Mimir (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23347, 'interact_tag', 'BetaE1_AsgardScanner_5', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23347, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23347, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23347, 'add_item', 2647, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23347, 'increment_counter', NULL, 'beta_420_scanners', '{"amount": 1}', 0, 1);

-- Chain 23348: player_loaded, Asgard, S1455 active -> restore the turn-in bind (binds are not persisted).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23348, 'Beta_Site_Evo_1 - Asgard relog during Reclamation: restore the turn-in bind (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23348, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23348, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23348, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23348, 'add_dialog_set', 2343, NULL, '{"slot": 2305, "mission_id": 420}', 0, 0);

-- Chain 23349: interact_tag BetaE1_Mimir, Asgard, S1455 active, counter >= 5 -> the turn-in. Completes the mission from S1455 (C3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23349, 'Beta_Site_Evo_1 - Mimir turn-in: Reclamation complete (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23349, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23349, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23349, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23349, 'counter', NULL, 'beta_420_scanners', 'gte', '5', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23349, 'display_dialog', 3057, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23349, 'remove_dialog_set', 2343, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23349, 'remove_item', 2647, NULL, '{"qty": 5}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23349, 'complete_mission', 420, NULL, '{}', 0, 3);

-- Chain 23350: interact_tag BetaE1_Mimir, Asgard, S1455 active, counter < 5 -> "not yet": re-show the pitch 1954 (F21, P9).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23350, 'Beta_Site_Evo_1 - Mimir not yet: Reclamation pitch while short of scanners (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23350, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23350, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23350, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23350, 'counter', NULL, 'beta_420_scanners', 'lt', '5', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23350, 'display_dialog', 1954, NULL, '{}', 0, 0);

-- Chain 23351: player_loaded, Asgard, S1455 active -> clear the scanners held by the old quota (Q5, UB-U5b).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23351, 'Beta_Site_Evo_1 - Asgard relog during Reclamation: clear the scanners held (UB-31)', 'mission', 420, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23351, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23351, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23351, 'step_status', 420, '1455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23351, 'remove_item', 2647, NULL, '{"qty": 5}', 0, 0);
