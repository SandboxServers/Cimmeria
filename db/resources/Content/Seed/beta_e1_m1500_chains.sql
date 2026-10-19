-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 1500 receiver and Carter (UB-20).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: CIMMERIA_RECONSTRUCTION. The Free Jaffa receiver for 1500 (The
-- SGU, Bra'tac 5024/5025 on Omega). Omega departs the player with S4369 "Report
-- to Beta Site" (O5080, T8046) active; Beta fires it on arrival (D-UB2, confirmed
-- by Omega D-UO2). S4370 "Report to Colonel Carter" (O5081, T6417) is Beta's:
-- Carter's talk dialog is 5026 in dialog_set_maps 5871 (flag 536870912), bound
-- per player to template 2300 (Carter). The step ids and dialog ids are RESTORED_SOURCE.
-- Guards: `chain_replay_tests/beta_e1/m1500.rs`.

-- Chain 23220: player_loaded Beta_Site_Evo_1, Free Jaffa (archetype 7), S4369 active
-- -> advance to S4370 and bind Carter's talk dialog 5871 (P1 receiver).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23220, 'Beta_Site_Evo_1 - Free Jaffa arrival: 1500 S4369 to S4370, bind Carter (UB-20)', 'mission', 1500, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23220, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23220, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23220, 'step_status', 1500, '4369', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23220, 'advance_step', 1500, '4370', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23220, 'add_dialog_set', 5871, NULL, '{"slot": 2300, "mission_id": 1500}', 0, 1);

-- Chain 23221: player_loaded Beta_Site_Evo_1, Free Jaffa, S4370 active -> re-bind
-- Carter's talk dialog on relog (P1 restore; binds are not persisted).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23221, 'Beta_Site_Evo_1 - Free Jaffa restore: bind Carter while S4370 is active (UB-20)', 'mission', 1500, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23221, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23221, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23221, 'step_status', 1500, '4370', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23221, 'add_dialog_set', 5871, NULL, '{"slot": 2300, "mission_id": 1500}', 0, 0);

-- Chain 23222: interact_tag BetaE1_Carter, Free Jaffa, S4370 active -> Carter's
-- welcome dialog 5026, unbind it, complete 1500 (P3 turn-in on interact).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23222, 'Beta_Site_Evo_1 - Carter welcome for the Free Jaffa: dialog 5026, complete 1500 (UB-20)', 'mission', 1500, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23222, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23222, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23222, 'step_status', 1500, '4370', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23222, 'display_dialog', 5026, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23222, 'remove_dialog_set', 5871, NULL, '{"slot": 2300}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23222, 'complete_mission', 1500, NULL, '{}', 0, 2);
