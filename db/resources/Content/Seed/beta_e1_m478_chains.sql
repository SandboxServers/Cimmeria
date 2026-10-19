-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 478 Instant IT (UB-13).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the step ids (S1746, S1747, S1749; there is no
-- S1748), the objective ids (O2066 region, O2067 workstation, O2069 Carter), the
-- dialogs (1995 done, 3223 offer, 3224 object, 1998 blurb), the dsm ids (2460 blurb,
-- 3878 offer, 3883 object, 3884 done), the workstation template 2323, the Carter
-- template 2300 and the point set Beta_Site_Evo_1.AdminBuilding (23001).
-- CIMMERIA_RECONSTRUCTION for the chain shapes and the P10 and P5 fallbacks.
-- Guards: `chain_replay_tests/beta_e1/m478.rs`.

-- Chain 23100: login or completing 473, Human, 473 completed, 478 not_active
-- -> bind Carter's blurb dsm 2460 (the offer bind, P2). Two trigger rows.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23100, 'Beta_Site_Evo_1 - Human after 473: bind Carter''s 478 blurb (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23100, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23100, 'mission_completed', '473', 'player', false, 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23100, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23100, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23100, 'mission_status', 473, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23100, 'mission_status', 478, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23100, 'add_dialog_set', 2460, NULL, '{"slot": 2300, "mission_id": 478}', 0, 0);

-- Chain 23101: interact_tag BetaE1_Carter, Human, 473 completed, 478 not_active
-- -> display the offer 3223.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23101, 'Beta_Site_Evo_1 - Carter click: show the 478 offer (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23101, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23101, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23101, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23101, 'mission_status', 473, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23101, 'mission_status', 478, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23101, 'display_dialog', 3223, NULL, '{}', 0, 0);

-- Chain 23102: dialog_choice 3223 (Accept), same gates. Accepts 478 and removes the blurb bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23102, 'Beta_Site_Evo_1 - accept the 478 offer (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23102, 'dialog_choice', '3223', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23102, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23102, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23102, 'mission_status', 473, NULL, 'eq', 'completed', 2);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23102, 'mission_status', 478, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23102, 'accept_mission', 478, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23102, 'remove_dialog_set', 2460, NULL, '{"slot": 2300}', 0, 1);

-- Chain 23103: enter_region Beta_Site_Evo_1.AdminBuilding, Human, S1746 active -> S1747 (P10).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23103, 'Beta_Site_Evo_1 - AdminBuilding entered: to the workstation (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23103, 'enter_region', 'Beta_Site_Evo_1.AdminBuilding', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23103, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23103, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23103, 'step_status', 478, '1746', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23103, 'advance_step', 478, '1747', '{}', 0, 0);

-- Chain 23104: interact_tag BetaE1_AdminWorkstation, Human, S1746 active -> S1747 and
-- the worm offer (P10 fallback: a player already inside the building never enters).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23104, 'Beta_Site_Evo_1 - Workstation clicked before S1747: to the worm (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23104, 'interact_tag', 'BetaE1_AdminWorkstation', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23104, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23104, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23104, 'step_status', 478, '1746', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23104, 'advance_step', 478, '1747', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23104, 'display_dialog', 3224, NULL, '{}', 0, 1);

-- Chain 23105: interact_tag BetaE1_AdminWorkstation, Human, S1747 active -> the worm dialog 3224.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23105, 'Beta_Site_Evo_1 - Workstation clicked at S1747: show the worm (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23105, 'interact_tag', 'BetaE1_AdminWorkstation', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23105, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23105, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23105, 'step_status', 478, '1747', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23105, 'display_dialog', 3224, NULL, '{}', 0, 0);

-- Chain 23106: dialog_choice 3224 (the type-4 button, UB-00a1 Q6), Human, S1747 active
-- -> S1749, and bind Carter's done dsm 3884 (P5 keyed on pick:3224).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23106, 'Beta_Site_Evo_1 - worm shut down: to Carter (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23106, 'dialog_choice', '3224', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23106, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23106, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23106, 'step_status', 478, '1747', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23106, 'advance_step', 478, '1749', '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23106, 'add_dialog_set', 3884, NULL, '{"slot": 2300, "mission_id": 478}', 0, 1);

-- Chain 23107: player_loaded, Human, S1749 active -> bind Carter's done dsm 3884 (restore).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23107, 'Beta_Site_Evo_1 - Human at S1749 relog: restore Carter''s 478 turn-in bind (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23107, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23107, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23107, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23107, 'step_status', 478, '1749', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23107, 'add_dialog_set', 3884, NULL, '{"slot": 2300, "mission_id": 478}', 0, 0);

-- Chain 23108: interact_tag BetaE1_Carter, Human, S1749 active -> the done dialog 1995,
-- unbind 3884 and complete the mission (P3, complete_mission not the last objective).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23108, 'Beta_Site_Evo_1 - Carter turn-in: complete 478 (UB-13)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23108, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23108, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23108, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23108, 'step_status', 478, '1749', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23108, 'display_dialog', 1995, NULL, '{}', 0, 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23108, 'remove_dialog_set', 3884, NULL, '{"slot": 2300}', 0, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23108, 'complete_mission', 478, NULL, '{}', 0, 2);
