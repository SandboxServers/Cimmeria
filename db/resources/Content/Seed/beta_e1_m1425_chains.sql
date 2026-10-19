-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 1425 SGU Database (UB-14).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S4242, S4243, S4244), the objective
-- ids (O4910, O4911, O4912; O6325 is inert under HELD-B12 and is never completed),
-- the dialogs (4653 blurb, 4654 offer, 4655 hub object, 5497 science object), the
-- dsm ids (5457 blurb, 5458 offer), the Carter template 2300, the hub computer
-- template 2322 and the science computer template 2324 (UB-02b). The item 4833 is
-- not granted (HELD-B5, UB-00a2 Q8: no step consumes it).
-- CIMMERIA_RECONSTRUCTION for the chain shapes and the P5 object clicks.
-- Mission 1425 stops at S4244 (D-UB5 A): no chain completes it.
-- Guards: `chain_replay_tests/beta_e1/m1425.rs`.

-- Chain 23130: login or completing 478, Human, 478 completed, 1425 not_active
-- -> bind Carter's blurb dsm 5457 (the offer bind, P2). Two trigger rows.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23130, 'Beta_Site_Evo_1 - Human after 478: bind Carter''s 1425 blurb (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23130, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23130, 'mission_completed', '478', 'player', false, 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23130, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23130, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23130, 'mission_status', 478, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23130, 'mission_status', 1425, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23130, 'add_dialog_set', 5457, NULL, '{"slot": 2300, "mission_id": 1425}', 0, 0);

-- Chain 23131: interact_tag BetaE1_Carter, Human, 478 completed, 1425 not_active
-- -> display the offer 4654.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23131, 'Beta_Site_Evo_1 - Carter click: show the 1425 offer (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23131, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23131, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23131, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23131, 'mission_status', 478, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23131, 'mission_status', 1425, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23131, 'display_dialog', 4654, NULL, '{}', 0, 0);

-- Chain 23132: dialog_choice 4654 (Accept), same gates. Accepts 1425 and removes the blurb bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23132, 'Beta_Site_Evo_1 - accept the 1425 offer (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23132, 'dialog_choice', '4654', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23132, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23132, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23132, 'mission_status', 478, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23132, 'mission_status', 1425, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23132, 'accept_mission', 1425, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23132, 'remove_dialog_set', 5457, NULL, '{"slot": 2300}', 0, 1);

-- Chain 23133: interact_tag BetaE1_HubComputer, Human, S4242 active -> the hub
-- object dialog 4655 (P5). The hub download is the first click of the mission.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23133, 'Beta_Site_Evo_1 - hub computer click: show the SGU search (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23133, 'interact_tag', 'BetaE1_HubComputer', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23133, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23133, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23133, 'step_status', 1425, '4242', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23133, 'display_dialog', 4655, NULL, '{}', 0, 0);

-- Chain 23134: dialog_choice 4655 (button 456), Human, S4242 active -> S4243.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23134, 'Beta_Site_Evo_1 - hub search button: to the science computers (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23134, 'dialog_choice', '4655', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23134, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23134, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23134, 'step_status', 1425, '4242', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23134, 'advance_step', 1425, '4243', '{}', 0, 0);

-- Chain 23135: interact_tag BetaE1_ScienceComputer, Human, S4243 active -> the
-- science object dialog 5497 (P5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23135, 'Beta_Site_Evo_1 - science computer click: show the login hack (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23135, 'interact_tag', 'BetaE1_ScienceComputer', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23135, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23135, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23135, 'step_status', 1425, '4243', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23135, 'display_dialog', 5497, NULL, '{}', 0, 0);

-- Chain 23136: dialog_choice 5497 (button 457), Human, S4243 active -> S4244 (Jackson).
-- Mission 1425 stays open at S4244 under D-UB5 A: no complete_mission here.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23136, 'Beta_Site_Evo_1 - science login hack button: to the delivery step (UB-14)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23136, 'dialog_choice', '5497', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23136, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23136, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23136, 'step_status', 1425, '4243', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23136, 'advance_step', 1425, '4244', '{}', 0, 0);
