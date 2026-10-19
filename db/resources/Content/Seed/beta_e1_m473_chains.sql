-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 473 Inventory (UB-12).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S1723-S1726), the objective ids
-- (O2036 region, O2037 Arc-42, O2038 Arc-66, O2039 Felger, O2040 Carter), the
-- dialogs (2014 done, 2015 mid, 2017 blurb, 2018 offer), the dsm ids (2438 done,
-- 2439 mid, 2440 blurb, 2441 offer), the crate tags and the crate template 2325.
-- CIMMERIA_RECONSTRUCTION for the chain shapes and the P6 crate partition (F19).
-- Guards: `chain_replay_tests/beta_e1/m473.rs`.

-- Chain 23070: login or completing 381, Human, 381 completed, 473 not_active
-- -> bind Felger's blurb dsm 2440 (the offer bind, P2). Two trigger rows.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23070, 'Beta_Site_Evo_1 - Human after 381: bind Felger''s 473 blurb (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23070, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23070, 'mission_completed', '381', 'player', false, 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23070, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23070, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23070, 'mission_status', 381, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23070, 'mission_status', 473, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23070, 'add_dialog_set', 2440, NULL, '{"slot": 2301, "mission_id": 473}', 0, 0);

-- Chain 23071: interact_tag BetaE1_Felger, Human, 381 completed, 473 not_active
-- -> display the offer 2018.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23071, 'Beta_Site_Evo_1 - Felger click: show the 473 offer (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23071, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23071, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23071, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23071, 'mission_status', 381, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23071, 'mission_status', 473, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23071, 'display_dialog', 2018, NULL, '{}', 0, 0);

-- Chain 23072: dialog_choice 2018 (Accept), same gates. Accepts 473 and removes the blurb bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23072, 'Beta_Site_Evo_1 - accept the 473 offer (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23072, 'dialog_choice', '2018', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23072, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23072, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23072, 'mission_status', 381, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23072, 'mission_status', 473, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23072, 'accept_mission', 473, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23072, 'remove_dialog_set', 2440, NULL, '{"slot": 2301}', 0, 1);

-- Chain 23073: enter_region Beta_Site_Evo_1.Storage, Human, S1723 active -> S1724 (P10).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23073, 'Beta_Site_Evo_1 - Storage entered: to the crate check (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23073, 'enter_region', 'Beta_Site_Evo_1.Storage', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23073, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23073, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23073, 'step_status', 473, '1723', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23073, 'advance_step', 473, '1724', '{}', 0, 0);

-- Chain 23074: interact_tag BetaE1_Crate_Arc42, Human, S1723 active -> S1724 (P10 fallback).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23074, 'Beta_Site_Evo_1 - Arc-42 crate clicked in Storage: to the crate check (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23074, 'interact_tag', 'BetaE1_Crate_Arc42', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23074, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23074, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23074, 'step_status', 473, '1723', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23074, 'advance_step', 473, '1724', '{}', 0, 0);

-- Chain 23075: interact_tag BetaE1_Crate_Arc66, Human, S1723 active -> S1724 (P10 fallback).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23075, 'Beta_Site_Evo_1 - Arc-66 crate clicked in Storage: to the crate check (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23075, 'interact_tag', 'BetaE1_Crate_Arc66', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23075, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23075, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23075, 'step_status', 473, '1723', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23075, 'advance_step', 473, '1724', '{}', 0, 0);

-- Chains 23076-23079: the P6 partition of S1724 (O2037 Arc-42, O2038 Arc-66).
-- Each crate click completes one objective. The second objective is completed
-- by advance_step, never complete_objective (F19: that would complete 473).

-- Chain 23076: Arc-42 click, S1724 active, O2037 open, O2038 open -> complete O2037 (A-mark).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23076, 'Beta_Site_Evo_1 - Arc-42 crate, both open: mark O2037 (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23076, 'interact_tag', 'BetaE1_Crate_Arc42', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23076, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23076, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23076, 'step_status', 473, '1724', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23076, 'objective_status', 473, '2037', 'neq', 'completed', 3);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23076, 'objective_status', 473, '2038', 'neq', 'completed', 4);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23076, 'complete_objective', 473, '2037', '{}', 0, 0);

-- Chain 23077: Arc-42 click, S1724 active, O2037 open, O2038 done -> S1725 and bind Felger's mid dsm 2439 (A-last).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23077, 'Beta_Site_Evo_1 - Arc-42 crate, Arc-66 done: advance to S1725 (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23077, 'interact_tag', 'BetaE1_Crate_Arc42', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23077, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23077, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23077, 'step_status', 473, '1724', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23077, 'objective_status', 473, '2037', 'neq', 'completed', 3);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23077, 'objective_status', 473, '2038', 'eq', 'completed', 4);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23077, 'advance_step', 473, '1725', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23077, 'add_dialog_set', 2439, NULL, '{"slot": 2301, "mission_id": 473}', 0, 1);

-- Chain 23078: Arc-66 click, S1724 active, O2038 open, O2037 open -> complete O2038 (B-mark).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23078, 'Beta_Site_Evo_1 - Arc-66 crate, both open: mark O2038 (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23078, 'interact_tag', 'BetaE1_Crate_Arc66', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23078, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23078, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23078, 'step_status', 473, '1724', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23078, 'objective_status', 473, '2038', 'neq', 'completed', 3);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23078, 'objective_status', 473, '2037', 'neq', 'completed', 4);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23078, 'complete_objective', 473, '2038', '{}', 0, 0);

-- Chain 23079: Arc-66 click, S1724 active, O2038 open, O2037 done -> S1725 and bind Felger's mid dsm 2439 (B-last).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23079, 'Beta_Site_Evo_1 - Arc-66 crate, Arc-42 done: advance to S1725 (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23079, 'interact_tag', 'BetaE1_Crate_Arc66', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23079, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23079, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23079, 'step_status', 473, '1724', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23079, 'objective_status', 473, '2038', 'neq', 'completed', 3);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23079, 'objective_status', 473, '2037', 'eq', 'completed', 4);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23079, 'advance_step', 473, '1725', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23079, 'add_dialog_set', 2439, NULL, '{"slot": 2301, "mission_id": 473}', 0, 1);

-- Chain 23080: player_loaded, Human, S1725 active -> restore Felger's mid dsm 2439.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23080, 'Beta_Site_Evo_1 - Human relog on S1725: restore Felger''s mid bind (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23080, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23080, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23080, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23080, 'step_status', 473, '1725', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23080, 'add_dialog_set', 2439, NULL, '{"slot": 2301, "mission_id": 473}', 0, 0);

-- Chain 23081: interact_tag BetaE1_Felger, Human, S1725 active -> mid talk 2015, then S1726 and bind Carter's dsm 2438 (P4).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23081, 'Beta_Site_Evo_1 - Felger mid talk on S1725: to the Carter turn-in (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23081, 'interact_tag', 'BetaE1_Felger', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23081, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23081, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23081, 'step_status', 473, '1725', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23081, 'display_dialog', 2015, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23081, 'remove_dialog_set', 2439, NULL, '{"slot": 2301}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23081, 'advance_step', 473, '1726', '{}', 0, 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23081, 'add_dialog_set', 2438, NULL, '{"slot": 2300, "mission_id": 473}', 0, 3);

-- Chain 23082: player_loaded, Human, S1726 active -> restore Carter's dsm 2438.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23082, 'Beta_Site_Evo_1 - Human relog on S1726: restore Carter''s turn-in bind (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23082, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23082, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23082, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23082, 'step_status', 473, '1726', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23082, 'add_dialog_set', 2438, NULL, '{"slot": 2300, "mission_id": 473}', 0, 0);

-- Chain 23083: interact_tag BetaE1_Carter, Human, S1726 active -> done talk 2014 and complete_mission (P3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23083, 'Beta_Site_Evo_1 - Carter turn-in for 473 (UB-12)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23083, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23083, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23083, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23083, 'step_status', 473, '1726', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23083, 'display_dialog', 2014, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23083, 'remove_dialog_set', 2438, NULL, '{"slot": 2300}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23083, 'complete_mission', 473, NULL, '{}', 0, 2);
