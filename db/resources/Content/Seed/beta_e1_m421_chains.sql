-- Beta Site E1 (world 23, Beta_Site_Evo_1) - Mechanical Error: the infirmary signal and the disabled drone (UB-32, mission 421).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: mission 421 steps S1460 (O1698, region), S1461 (O1699, emitter), S1462 (O1700, drone),
-- S1464 (O1702, turn-in). There is no S1463 (a seed gap, mission-seal.md). Mission 421 is RESTORED_SOURCE
-- (mission_objectives.sql, mission_tasks.sql). The step chains (23363-23368) are CIMMERIA_AUTHORING on that
-- RESTORED_SOURCE step list; the object and region triggers follow the UB-02c spawns and point set.
-- Dialogs: blurb 1988 (dsm 2349), offer 1987 (dsm 2347, Accept), object 3722 (dsm 4375, speaker 763),
-- turn-in 3901 (dsm 4585). Spawns BetaE1_HologramEmitter (2332) and BetaE1_DisabledDrone (2333) in
-- region Beta_Site_Evo_1.Infirmary (point set 23030), from UB-02c. HELD-B3 is sealed (placement-seal.md).
-- Guards: chain_replay_tests/beta_e1/m421.rs.

-- Chain 23360: player_loaded and mission_completed 420, Asgard, 420 completed, 421 not_active -> bind the blurb 2349.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23360, 'Beta_Site_Evo_1 - Mimir blurb bind for Mechanical Error (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23360, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23360, 'mission_completed', '420', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23360, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23360, 'mission_status', 420, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23360, 'mission_status', 421, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23360, 'add_dialog_set', 2349, NULL, '{"slot": 2305, "mission_id": 421}', 0, 0);

-- Chain 23361: interact_tag BetaE1_Mimir, Asgard, 420 completed, 421 not_active -> show the offer 1987.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23361, 'Beta_Site_Evo_1 - Mimir offers Mechanical Error: show the offer (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23361, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23361, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23361, 'mission_status', 420, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23361, 'mission_status', 421, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23361, 'display_dialog', 1987, NULL, '{}', 0, 0);

-- Chain 23362: dialog_choice 1987 (Accept), Asgard, 420 completed, 421 not_active -> accept 421, drop the blurb.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23362, 'Beta_Site_Evo_1 - Accept Mechanical Error: drop the blurb (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23362, 'dialog_choice', '1987', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23362, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23362, 'mission_status', 420, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23362, 'mission_status', 421, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23362, 'accept_mission', 421, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23362, 'remove_dialog_set', 2349, NULL, '{"slot": 2305}', 0, 1);

-- Chain 23363: enter_region Beta_Site_Evo_1.Infirmary, Asgard, S1460 active -> advance to S1461.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23363, 'Beta_Site_Evo_1 - Infirmary region: find the signal (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23363, 'enter_region', 'Beta_Site_Evo_1.Infirmary', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23363, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23363, 'step_status', 421, '1460', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23363, 'advance_step', 421, '1461', '{}', 0, 0);

-- Chain 23364: interact_tag BetaE1_HologramEmitter, Asgard, S1460 active -> advance to S1461 (the region fallback, P10).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23364, 'Beta_Site_Evo_1 - Hologram emitter before the region fires: advance (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23364, 'interact_tag', 'BetaE1_HologramEmitter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23364, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23364, 'step_status', 421, '1460', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23364, 'advance_step', 421, '1461', '{}', 0, 0);

-- Chain 23365: interact_tag BetaE1_HologramEmitter, Asgard, S1461 active -> show the emitter 3722, advance to S1462.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23365, 'Beta_Site_Evo_1 - Hologram emitter found: show the object and advance (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23365, 'interact_tag', 'BetaE1_HologramEmitter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23365, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23365, 'step_status', 421, '1461', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23365, 'display_dialog', 3722, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23365, 'advance_step', 421, '1462', '{}', 0, 1);

-- Chain 23366: interact_tag BetaE1_DisabledDrone, Asgard, S1462 active -> advance to S1464, bind the turn-in 4585.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23366, 'Beta_Site_Evo_1 - Disabled drone examined: advance and bind the turn-in (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23366, 'interact_tag', 'BetaE1_DisabledDrone', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23366, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23366, 'step_status', 421, '1462', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23366, 'advance_step', 421, '1464', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23366, 'add_dialog_set', 4585, NULL, '{"slot": 2305, "mission_id": 421}', 0, 1);

-- Chain 23367: player_loaded, Asgard, S1464 active -> restore the turn-in bind 4585 after a relog (no binds persist).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23367, 'Beta_Site_Evo_1 - Mechanical Error turn-in bind restore on login (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23367, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23367, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23367, 'step_status', 421, '1464', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23367, 'add_dialog_set', 4585, NULL, '{"slot": 2305, "mission_id": 421}', 0, 0);

-- Chain 23368: interact_tag BetaE1_Mimir, Asgard, S1464 active -> turn-in 3901, drop the bind 4585, complete 421.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23368, 'Beta_Site_Evo_1 - Mimir turn-in for Mechanical Error (UB-32)', 'mission', 421, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23368, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23368, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23368, 'step_status', 421, '1464', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23368, 'display_dialog', 3901, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23368, 'remove_dialog_set', 4585, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23368, 'complete_mission', 421, NULL, '{}', 0, 2);
