-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 1423 receiver and Carter (UB-10).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S4223 "Go to Beta Site", S4224
-- "Report to Carter"), dialog 4643 and dsm 5433 (slot 2300 = Carter).
-- CIMMERIA_RECONSTRUCTION for the chain shapes, which follow the Omega receiver
-- contract (docs/analysis/union-beta-e1-continuation/receiver-contract.md) and the
-- seal (docs/analysis/union-beta-e1-continuation/mission-seal.md, 1423 table).
-- Guards: `chain_replay_tests/beta_e1/m1423.rs`.

-- Chain 23020: player_loaded Beta_Site_Evo_1, Human, 1423 active and S4223 active
-- -> advance to S4224 and bind Carter's talk dialog set. The receiver (P1).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23020, 'Beta_Site_Evo_1 - Human arrival: S4223 to S4224, bind Carter (UB-10)', 'mission', 1423, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23020, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23020, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23020, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23020, 'mission_status', 1423, NULL, 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23020, 'step_status', 1423, '4223', 'eq', 'active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23020, 'advance_step', 1423, '4224', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23020, 'add_dialog_set', 5433, NULL, '{"slot": 2300, "mission_id": 1423}', 0, 1);

-- Chain 23021: player_loaded Beta_Site_Evo_1, Human, S4224 active -> re-bind Carter
-- (restore). Binds are not persisted, so a relog while reporting needs this (P1).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23021, 'Beta_Site_Evo_1 - Human relog while reporting: re-bind Carter (UB-10)', 'mission', 1423, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23021, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23021, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23021, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23021, 'step_status', 1423, '4224', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23021, 'add_dialog_set', 5433, NULL, '{"slot": 2300, "mission_id": 1423}', 0, 0);

-- Chain 23022: interact_tag BetaE1_Carter, Human, S4224 active -> show the report
-- dialog 4643, unbind Carter's set, complete 1423. P3 turn-in on interact. Use
-- `complete_mission`, not a last `complete_objective` (castle 706 note).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23022, 'Beta_Site_Evo_1 - Carter takes the report: complete 1423 (UB-10)', 'mission', 1423, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23022, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23022, 'archetype', NULL, NULL, 'gte', '1', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23022, 'archetype', NULL, NULL, 'lte', '4', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23022, 'step_status', 1423, '4224', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23022, 'display_dialog', 4643, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23022, 'remove_dialog_set', 5433, NULL, '{"slot": 2300}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23022, 'complete_mission', 1423, NULL, '{}', 0, 2);
