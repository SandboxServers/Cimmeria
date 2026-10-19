-- Beta Site E1 (world 23, Beta_Site_Evo_1) - Intriguing Data: the Analytical Assistance Machine (UB-33, mission 1147).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: mission 1147 steps S3458 (O4020, the machine) and S3459 (O4021, the turn-in to Mimir).
-- Mission 1147 is RESTORED_SOURCE (mission_objectives.sql, mission_tasks.sql; mission-seal.md 1147 table).
-- The chains (23390-23395) are CIMMERIA_AUTHORING on that RESTORED_SOURCE step list.
-- Dialogs: blurb 3903 (dsm 4588), offer 3904 (dsm 4589, Accept), turn-in 3905 (dsm 4590).
-- Spawn BetaE1_AnalyticalMachine (2331, deck, static INT_MissionWorldObject) and NPC Mimir (2305), from UB-02c.
-- Gate: Asgard (archetype 5) only. Omega UO-16 is HELD (HELD-B16), so this is replay-tested only (D-UB13).
-- F19: 1147 has one objective per step, so no chain here uses complete_objective.
-- Guards: chain_replay_tests/beta_e1/m1147.rs.

-- Chain 23390: player_loaded and mission_completed 421, Asgard, 421 completed, 1147 not_active -> bind the blurb 4588.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23390, 'Beta_Site_Evo_1 - Mimir blurb bind for Intriguing Data (UB-33)', 'mission', 1147, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23390, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23390, 'mission_completed', '421', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23390, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23390, 'mission_status', 421, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23390, 'mission_status', 1147, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23390, 'add_dialog_set', 4588, NULL, '{"slot": 2305, "mission_id": 1147}', 0, 0);

-- Chain 23391: interact_tag BetaE1_Mimir, Asgard, 421 completed, 1147 not_active -> show the offer 3904.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23391, 'Beta_Site_Evo_1 - Mimir offers Intriguing Data: show the offer (UB-33)', 'mission', 1147, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23391, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23391, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23391, 'mission_status', 421, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23391, 'mission_status', 1147, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23391, 'display_dialog', 3904, NULL, '{}', 0, 0);

-- Chain 23392: dialog_choice 3904 (Accept), Asgard, 421 completed, 1147 not_active -> accept 1147, drop the blurb.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23392, 'Beta_Site_Evo_1 - Accept Intriguing Data: drop the blurb (UB-33)', 'mission', 1147, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23392, 'dialog_choice', '3904', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23392, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23392, 'mission_status', 421, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23392, 'mission_status', 1147, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23392, 'accept_mission', 1147, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23392, 'remove_dialog_set', 4588, NULL, '{"slot": 2305}', 0, 1);

-- Chain 23393: interact_tag BetaE1_AnalyticalMachine, Asgard, S3458 active -> advance to S3459, bind the turn-in 4590.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23393, 'Beta_Site_Evo_1 - Analytical Assistance Machine: advance to S3459, bind the turn-in (UB-33)', 'mission', 1147, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23393, 'interact_tag', 'BetaE1_AnalyticalMachine', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23393, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23393, 'step_status', 1147, '3458', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23393, 'advance_step', 1147, '3459', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23393, 'add_dialog_set', 4590, NULL, '{"slot": 2305, "mission_id": 1147}', 0, 1);

-- Chain 23394: player_loaded, Asgard, S3459 active -> restore the turn-in bind 4590 after a relog (no binds persist).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23394, 'Beta_Site_Evo_1 - Mimir turn-in bind restore for Intriguing Data (UB-33)', 'mission', 1147, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23394, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23394, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23394, 'step_status', 1147, '3459', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23394, 'add_dialog_set', 4590, NULL, '{"slot": 2305, "mission_id": 1147}', 0, 0);

-- Chain 23395: interact_tag BetaE1_Mimir, Asgard, S3459 active -> turn-in 3905, drop the bind 4590, complete 1147.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23395, 'Beta_Site_Evo_1 - Mimir turn-in for Intriguing Data: complete the mission (UB-33)', 'mission', 1147, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23395, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23395, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23395, 'step_status', 1147, '3459', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23395, 'display_dialog', 3905, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23395, 'remove_dialog_set', 4590, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23395, 'complete_mission', 1147, NULL, '{}', 0, 2);
