-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 1427 The Ruins (UB-16).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S4248, S4249, S4250), the objective
-- ids (O4919, O6327 is inert under HELD-B12 and is never completed, O4920, O4921),
-- the dialogs (4661 blurb, 4662 offer, 4663 post-it object, 4664 Carter mid), the dsm
-- ids (5464 blurb, 5466 Carter mid), the Carter template 2300 and the ruin note
-- template 2335 (UB-02b places it at the Barrak'Ka pin, sealed by UB-00b).
-- CIMMERIA_RECONSTRUCTION for the offer route (1427 is offered once 1426 is completed,
-- P2) and the chain shapes. The arc stops at S4250 (D-UB5 A): nothing binds or
-- completes S4250 on Beta, and no chain touches Woolsey's dialog 4670 or dsm 5468.
-- Guards: `chain_replay_tests/beta_e1/m1427.rs`.

-- Chain 23190: login or completing 1426, Human, 1426 completed, 1427 not_active
-- -> bind Carter's blurb dsm 5464 (the offer bind, P2).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23190, 'Beta_Site_Evo_1 - Human after 1426: bind Carter''s 1427 blurb (UB-16)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23190, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23190, 'mission_completed', '1426', 'player', false, 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23190, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23190, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23190, 'mission_status', 1426, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23190, 'mission_status', 1427, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23190, 'add_dialog_set', 5464, NULL, '{"slot": 2300, "mission_id": 1427}', 0, 0);

-- Chain 23191: interact_tag BetaE1_Carter, Human, 1426 completed, 1427 not_active
-- -> the offer 4662.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23191, 'Beta_Site_Evo_1 - Carter click after 1426: show the 1427 offer (UB-16)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23191, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23191, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23191, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23191, 'mission_status', 1426, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23191, 'mission_status', 1427, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23191, 'display_dialog', 4662, NULL, '{}', 0, 0);

-- Chain 23192: dialog_choice 4662 (Accept), Human, 1426 completed, 1427 not_active
-- -> accept 1427 and remove the blurb bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23192, 'Beta_Site_Evo_1 - accept the 1427 offer after 1426 (UB-16)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23192, 'dialog_choice', '4662', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23192, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23192, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23192, 'mission_status', 1426, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23192, 'mission_status', 1427, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23192, 'accept_mission', 1427, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23192, 'remove_dialog_set', 5464, NULL, '{"slot": 2300}', 0, 1);

-- Chain 23193: interact_tag BetaE1_RuinNote, Human, 1427 S4248 active -> the post-it
-- dialog 4663, S4249, and Carter's mid-step bind 5466 (P5, object click).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23193, 'Beta_Site_Evo_1 - Barrak''Ka post-it click: show the note, report to Carter (UB-16)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23193, 'interact_tag', 'BetaE1_RuinNote', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23193, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23193, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23193, 'step_status', 1427, '4248', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23193, 'display_dialog', 4663, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23193, 'advance_step', 1427, '4249', '{}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23193, 'add_dialog_set', 5466, NULL, '{"slot": 2300, "mission_id": 1427}', 0, 2);

-- Chain 23194: login, Human, 1427 S4249 active -> restore Carter's mid-step bind 5466.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23194, 'Beta_Site_Evo_1 - Human at 1427 S4249: restore the report bind (UB-16)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23194, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23194, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23194, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23194, 'step_status', 1427, '4249', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23194, 'add_dialog_set', 5466, NULL, '{"slot": 2300, "mission_id": 1427}', 0, 0);

-- Chain 23195: interact_tag BetaE1_Carter, Human, 1427 S4249 active -> the report
-- dialog 4664, the bind 5466 removed, and S4250 (the arc stops there, D-UB5 A).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23195, 'Beta_Site_Evo_1 - Carter click at 1427 S4249: report, advance to S4250 (UB-16)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23195, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23195, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23195, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23195, 'step_status', 1427, '4249', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23195, 'display_dialog', 4664, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23195, 'remove_dialog_set', 5466, NULL, '{"slot": 2300}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23195, 'advance_step', 1427, '4250', '{}', 0, 2);
