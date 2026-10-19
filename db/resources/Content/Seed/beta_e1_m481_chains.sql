-- Beta Site E1 (world 23, Beta_Site_Evo_1) - From on High, receiver, Carter, beam and Mimir (UB-30, mission 481).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: the Asgard arrival is the Omega UO-16 handoff (HELD-B16, not
-- delivered yet). This file is the Beta receiver fixture: chains 23310-23313
-- hold the Carter and Mimir binds, so the arrival replays on Beta once UO-16
-- lands. Mission 481 steps S1760-S1762 and objectives O2079-O2080, O4015 are
-- RESTORED_SOURCE (mission_objectives.sql). Item 3399 is removed on the O2079
-- hand-in (HELD-B1b, mission-seal-held.md Q7): CIMMERIA_AUTHORING.
-- The beam (D-UB12) is CIMMERIA_RECONSTRUCTION; destinations are sealed in
-- placement-seal.md § beam (BEAM_DECK_DEST, BEAM_GROUND_DEST).
-- Guards: chain_replay_tests/beta_e1/m481.rs.

-- Chain 23310: player_loaded, Asgard, S1760 active -> advance to S1761 and bind Carter's talk dialog 1915 (dsm 2465).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23310, 'Beta_Site_Evo_1 - Asgard arrival: From on High receiver, Carter talk (UB-30)', 'mission', 481, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23310, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23310, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23310, 'step_status', 481, '1760', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23310, 'advance_step', 481, '1761', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23310, 'add_dialog_set', 2465, NULL, '{"slot": 2300, "mission_id": 481}', 0, 1);

-- Chain 23311: player_loaded, Asgard, S1761 active -> restore the Carter bind (binds are not persisted).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23311, 'Beta_Site_Evo_1 - Asgard relog at S1761: restore the Carter talk bind (UB-30)', 'mission', 481, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23311, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23311, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23311, 'step_status', 481, '1761', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23311, 'add_dialog_set', 2465, NULL, '{"slot": 2300, "mission_id": 481}', 0, 0);

-- Chain 23312: interact_tag BetaE1_Carter, Asgard, S1761 active -> the O2079 hand-in.
-- Shows 1915, unbinds Carter, removes communique 3399 (HELD-B1b), advances to S1762 and binds Mimir's turn-in 1916 (dsm 2467).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23312, 'Beta_Site_Evo_1 - Carter hand-in: take the communique, send the Asgard to Mimir (UB-30)', 'mission', 481, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23312, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23312, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23312, 'step_status', 481, '1761', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23312, 'display_dialog', 1915, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23312, 'remove_dialog_set', 2465, NULL, '{"slot": 2300}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23312, 'remove_item', 3399, NULL, '{"qty": 1}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23312, 'advance_step', 481, '1762', '{}', 0, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23312, 'add_dialog_set', 2467, NULL, '{"slot": 2305, "mission_id": 481}', 0, 4);

-- Chain 23313: player_loaded, Asgard, S1762 active -> restore the Mimir turn-in bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23313, 'Beta_Site_Evo_1 - Asgard relog at S1762: restore the Mimir turn-in bind (UB-30)', 'mission', 481, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23313, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23313, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23313, 'step_status', 481, '1762', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23313, 'add_dialog_set', 2467, NULL, '{"slot": 2305, "mission_id": 481}', 0, 0);

-- Chain 23314: interact_tag BetaSite_00000007 (ground base pad, spawn 149), ungated beam up to the deck.
-- CIMMERIA_RECONSTRUCTION (D-UB12). Destination sealed in placement-seal.md § beam (BEAM_DECK_DEST).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23314, 'Beta_Site_Evo_1 - Ring pad, ground base: beam up to the Asgard deck (UB-30)', 'space', 23, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23314, 'interact_tag', 'BetaSite_00000007', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23314, 'move_entity', NULL, NULL, '{"destination": "698.98,157.93,70.36", "world": "Beta_Site_Evo_1", "use_player": true}', 0, 0);

-- Chain 23315: interact_tag BetaSite_Beam_000007_Ship (deck pad, spawn 151), ungated beam down to the ground base.
-- CIMMERIA_RECONSTRUCTION (D-UB12). Destination sealed in placement-seal.md § beam (BEAM_GROUND_DEST).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23315, 'Beta_Site_Evo_1 - Ring pad, Asgard deck: beam down to the ground base (UB-30)', 'space', 23, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23315, 'interact_tag', 'BetaSite_Beam_000007_Ship', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23315, 'move_entity', NULL, NULL, '{"destination": "745.05,-19.60,68.75", "world": "Beta_Site_Evo_1", "use_player": true}', 0, 0);

-- Chain 23316: interact_tag BetaSite_00000007, Asgard, S1762 active and O2080 open -> the teleporter objective.
-- Runs beside 23314 on the same click. O4015 stays open, so this never completes the mission (F19, P6).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23316, 'Beta_Site_Evo_1 - Teleporter objective at the ground pad: use the teleporter (UB-30)', 'mission', 481, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23316, 'interact_tag', 'BetaSite_00000007', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23316, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23316, 'step_status', 481, '1762', 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23316, 'objective_status', 481, '2080', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23316, 'complete_objective', 481, '2080', '{}', 0, 0);

-- Chain 23317: interact_tag BetaE1_Mimir, Asgard, S1762 active -> the turn-in. Completes the mission (P3, castle 1322 precedent).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23317, 'Beta_Site_Evo_1 - Mimir turn-in: From on High complete (UB-30)', 'mission', 481, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23317, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23317, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23317, 'step_status', 481, '1762', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23317, 'display_dialog', 1916, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23317, 'remove_dialog_set', 2467, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23317, 'complete_mission', 481, NULL, '{}', 0, 2);
