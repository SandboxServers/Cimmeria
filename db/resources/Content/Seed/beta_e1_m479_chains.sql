-- Beta Site E1 (world 23, Beta_Site_Evo_1) - Process Support: the parallel simulation (UB-36, mission 479).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: mission 479 steps S1750 (O2070, en=false, on the Analytical Machine 2331), S1751 (O2071, the
-- Consciousness Simulator 2334), S1752 (O2072, the Analytical Machine again) and S3644 (O4235, en=false, the
-- return to Mimir). Each step has one objective, so every step advances with advance_step (F19) and only the
-- Mimir turn-in completes the mission (complete_mission, P3). Mission 479 and its objective texts are
-- RESTORED_SOURCE (mission_objectives.sql, mission_steps.sql; mission-seal.md 479 table). Dialogs: blurb 1994
-- (dsm 2462, flag 134217728), offer 3228 (dsm 2461, Accept, moved by UB-DP), done 3227 (dsm 3527). The
-- spawns and the bind mechanism are CIMMERIA_RECONSTRUCTION: the Machine and the Simulator are static
-- INT_MissionWorldObject templates, and Mimir is bound per player (D-UB10).
-- Gate: Asgard (archetype 5) only. Omega UO-16 is HELD (HELD-B16), so this is replay-tested only (D-UB13).
-- Guards: chain_replay_tests/beta_e1/m479.rs.

-- Chain 23460: player_loaded and mission_completed 477, Asgard, 477 completed, 479 not_active -> bind the blurb 2462 on Mimir.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23460, 'Beta_Site_Evo_1 - Mimir blurb bind for Process Support (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23460, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23460, 'mission_completed', '477', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23460, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23460, 'mission_status', 477, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23460, 'mission_status', 479, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23460, 'add_dialog_set', 2462, NULL, '{"slot": 2305, "mission_id": 479}', 0, 0);

-- Chain 23461: interact_tag BetaE1_Mimir, Asgard, 477 completed, 479 not_active -> show the offer 3228.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23461, 'Beta_Site_Evo_1 - Mimir offers Process Support: show the offer (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23461, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23461, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23461, 'mission_status', 477, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23461, 'mission_status', 479, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23461, 'display_dialog', 3228, NULL, '{}', 0, 0);

-- Chain 23462: dialog_choice 3228 (Accept), Asgard, 477 completed, 479 not_active -> accept 479, drop the blurb.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23462, 'Beta_Site_Evo_1 - accept Process Support, drop the blurb (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23462, 'dialog_choice', '3228', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23462, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23462, 'mission_status', 477, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23462, 'mission_status', 479, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23462, 'accept_mission', 479, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23462, 'remove_dialog_set', 2462, NULL, '{"slot": 2305}', 0, 1);

-- Chain 23463: interact_tag BetaE1_AnalyticalMachine, Asgard, S1750 active -> advance to S1751 (P5, F19: no complete_objective).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23463, 'Beta_Site_Evo_1 - Analytical Machine processes the data (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23463, 'interact_tag', 'BetaE1_AnalyticalMachine', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23463, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23463, 'step_status', 479, '1750', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23463, 'advance_step', 479, '1751', '{}', 0, 0);

-- Chain 23464: interact_tag BetaE1_ConsciousnessSimulator, Asgard, S1751 active -> advance to S1752 (P5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23464, 'Beta_Site_Evo_1 - Consciousness Simulator runs the simulation (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23464, 'interact_tag', 'BetaE1_ConsciousnessSimulator', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23464, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23464, 'step_status', 479, '1751', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23464, 'advance_step', 479, '1752', '{}', 0, 0);

-- Chain 23465: interact_tag BetaE1_AnalyticalMachine, Asgard, S1752 active -> advance to S3644 and bind the done dialog 3527 on Mimir.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23465, 'Beta_Site_Evo_1 - Analytical Machine analyses the results (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23465, 'interact_tag', 'BetaE1_AnalyticalMachine', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23465, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23465, 'step_status', 479, '1752', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23465, 'advance_step', 479, '3644', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23465, 'add_dialog_set', 3527, NULL, '{"slot": 2305, "mission_id": 479}', 0, 1);

-- Chain 23466: player_loaded, Asgard, S3644 active -> bind the done dialog 3527 on Mimir again (restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23466, 'Beta_Site_Evo_1 - Mimir done-dialog bind restore for Process Support (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23466, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23466, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23466, 'step_status', 479, '3644', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23466, 'add_dialog_set', 3527, NULL, '{"slot": 2305, "mission_id": 479}', 0, 0);

-- Chain 23467: interact_tag BetaE1_Mimir, Asgard, S3644 active -> the turn-in (P3). Completes the mission; no complete_objective.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23467, 'Beta_Site_Evo_1 - Mimir turn-in completes Process Support (UB-36)', 'mission', 479, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23467, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23467, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23467, 'step_status', 479, '3644', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23467, 'display_dialog', 3227, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23467, 'remove_dialog_set', 3527, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23467, 'complete_mission', 479, NULL, '{}', 0, 2);
