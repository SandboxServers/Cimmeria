-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 1426 Amateur Photography (UB-15).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S4245, S4246, S4247), the objective
-- ids (O4913, O4914, O4915; O6326 is inert under HELD-B12 and is never completed),
-- the dialogs (4657 blurb, 4658 offer, 4659 hub object, 4660 done), the dsm ids
-- (5460 blurb, 5461 offer, 5462 done), the Carter template 2300, the case template
-- 2321 and the hub computer template 2322 (UB-02b).
-- CIMMERIA_RECONSTRUCTION for the offer route (1426 is offered once 1425 reaches
-- S4244, or once 1425 is completed, D-UB5 A) and the chain shapes.
-- CIMMERIA_AUTHORING for the camera grant: item 4832 (HELD-B9, UB-00a2 a2 change 2).
-- The grant is on accept (23165, 23166) and the take-back is on upload (23169).
-- Guards: `chain_replay_tests/beta_e1/m1426.rs`.

-- Chain 23160: login, Human, 1425 S4244 active, 1426 not_active -> bind Carter's
-- blurb dsm 5460 (the offer bind, P2, the S4244 path).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23160, 'Beta_Site_Evo_1 - Human at 1425 S4244: bind Carter''s 1426 blurb (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23160, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23160, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23160, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23160, 'step_status', 1425, '4244', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23160, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23160, 'add_dialog_set', 5460, NULL, '{"slot": 2300, "mission_id": 1426}', 0, 0);

-- Chain 23161: login or completing 1425, Human, 1425 completed, 1426 not_active
-- -> bind Carter's blurb dsm 5460 (the offer bind, P2, the 1425 done path).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23161, 'Beta_Site_Evo_1 - Human after 1425: bind Carter''s 1426 blurb (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23161, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23161, 'mission_completed', '1425', 'player', false, 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23161, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23161, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23161, 'mission_status', 1425, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23161, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23161, 'add_dialog_set', 5460, NULL, '{"slot": 2300, "mission_id": 1426}', 0, 0);

-- Chain 23162: dialog_choice 5497 (the 1425 science login hack, button 457), Human,
-- 1425 S4243 active, 1426 not_active -> the in-session offer bind (the bind lands
-- the bind is ready as soon as the science step is done, before the S4244 login).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23162, 'Beta_Site_Evo_1 - science hack: bind Carter''s 1426 blurb (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23162, 'dialog_choice', '5497', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23162, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23162, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23162, 'step_status', 1425, '4243', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23162, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23162, 'add_dialog_set', 5460, NULL, '{"slot": 2300, "mission_id": 1426}', 0, 0);

-- Chain 23163: interact_tag BetaE1_Carter, Human, 1425 S4244 active, 1426 not_active
-- -> the offer 4658.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23163, 'Beta_Site_Evo_1 - Carter click at 1425 S4244: show the 1426 offer (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23163, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23163, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23163, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23163, 'step_status', 1425, '4244', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23163, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23163, 'display_dialog', 4658, NULL, '{}', 0, 0);

-- Chain 23164: interact_tag BetaE1_Carter, Human, 1425 completed, 1426 not_active
-- -> the offer 4658 (the 1425 done path).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23164, 'Beta_Site_Evo_1 - Carter click after 1425: show the 1426 offer (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23164, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23164, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23164, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23164, 'mission_status', 1425, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23164, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23164, 'display_dialog', 4658, NULL, '{}', 0, 0);

-- Chain 23165: dialog_choice 4658 (Accept), Human, 1425 S4244 active, 1426 not_active
-- -> accept 1426, remove the blurb bind, and grant the camera 4832 (HELD-B9).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23165, 'Beta_Site_Evo_1 - accept the 1426 offer at 1425 S4244, camera granted (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23165, 'dialog_choice', '4658', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23165, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23165, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23165, 'step_status', 1425, '4244', 'eq', 'active', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23165, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23165, 'accept_mission', 1426, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23165, 'remove_dialog_set', 5460, NULL, '{"slot": 2300}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23165, 'add_item', 4832, NULL, '{"container": 0, "qty": 1}', 0, 2);

-- Chain 23166: dialog_choice 4658 (Accept), Human, 1425 completed, 1426 not_active
-- -> the same accept, on the 1425 done path, camera granted (HELD-B9).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23166, 'Beta_Site_Evo_1 - accept the 1426 offer after 1425, camera granted (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23166, 'dialog_choice', '4658', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23166, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23166, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23166, 'mission_status', 1425, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23166, 'mission_status', 1426, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23166, 'accept_mission', 1426, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23166, 'remove_dialog_set', 5460, NULL, '{"slot": 2300}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23166, 'add_item', 4832, NULL, '{"container": 0, "qty": 1}', 0, 2);

-- Chain 23167: interact_tag BetaE1_FourPiecesCase, Human, 1426 S4245 active
-- -> S4246 (P5, the photograph). No dialog: the journal update is the feedback.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23167, 'Beta_Site_Evo_1 - Four Pieces case click: photograph taken (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23167, 'interact_tag', 'BetaE1_FourPiecesCase', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23167, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23167, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23167, 'step_status', 1426, '4245', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23167, 'advance_step', 1426, '4246', '{}', 0, 0);

-- Chain 23168: interact_tag BetaE1_HubComputer, Human, 1426 S4246 active -> the
-- upload object dialog 4659 (P5). Disjoint from 23133 (S4242) by step status.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23168, 'Beta_Site_Evo_1 - hub computer click: show the photo upload (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23168, 'interact_tag', 'BetaE1_HubComputer', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23168, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23168, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23168, 'step_status', 1426, '4246', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23168, 'display_dialog', 4659, NULL, '{}', 0, 0);

-- Chain 23169: dialog_choice 4659 (button 458, the upload), Human, 1426 S4246 active
-- -> S4247, the done-side bind 5462, and the camera taken back (HELD-B9).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23169, 'Beta_Site_Evo_1 - photo upload: to Carter, camera taken back (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23169, 'dialog_choice', '4659', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23169, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23169, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23169, 'step_status', 1426, '4246', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23169, 'advance_step', 1426, '4247', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23169, 'add_dialog_set', 5462, NULL, '{"slot": 2300, "mission_id": 1426}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23169, 'remove_item', 4832, NULL, '{"qty": 1}', 0, 2);

-- Chain 23170: login, Human, 1426 S4247 active -> restore the done-side bind 5462
-- (binds are not persisted, P1 restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23170, 'Beta_Site_Evo_1 - Human at 1426 S4247: restore Carter''s turn-in bind (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23170, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23170, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23170, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23170, 'step_status', 1426, '4247', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23170, 'add_dialog_set', 5462, NULL, '{"slot": 2300, "mission_id": 1426}', 0, 0);

-- Chain 23171: interact_tag BetaE1_Carter, Human, 1426 S4247 active -> the done
-- dialog 4660, unbind 5462, and complete 1426 (P3 turn-in, castle 1322 precedent).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23171, 'Beta_Site_Evo_1 - Carter turn-in: complete 1426 (UB-15)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23171, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23171, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23171, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23171, 'step_status', 1426, '4247', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23171, 'display_dialog', 4660, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23171, 'remove_dialog_set', 5462, NULL, '{"slot": 2300}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23171, 'complete_mission', 1426, NULL, '{}', 0, 2);
