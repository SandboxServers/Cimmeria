-- Beta Site E1 (world 23, Beta_Site_Evo_1) - mission rewards (UB-50a).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: CIMMERIA_RECONSTRUCTION for every amount (U-D07 Approved
-- 2026-10-19, docs/analysis/union-beta-e1-continuation/reward-proposal.md, § 3
-- XP column). The XP is paid by one `grant_xp` chain per mission on
-- `mission_completed`. The `missions.reward_xp` column stays 0 (double-pay
-- guard, D-UB16). Action `sort_order` 1 is left free for UB-50b2 (`grant_cash`).
-- Guards: `chain_replay_tests/beta_e1/rewards.rs`.

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23480: The SGU (1423), Human, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23480, '1423 reward: 300 XP (UB-50a, U-D07)', 'mission', 1423, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23480, 'mission_completed', '1423', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23480, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23480, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23480, 'mission_status', 1423, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23480, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23481: Closing Shop (381), Human, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23481, '381 reward: 300 XP (UB-50a, U-D07)', 'mission', 381, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23481, 'mission_completed', '381', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23481, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23481, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23481, 'mission_status', 381, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23481, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23482: Inventory (473), Human, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23482, '473 reward: 300 XP (UB-50a, U-D07)', 'mission', 473, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23482, 'mission_completed', '473', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23482, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23482, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23482, 'mission_status', 473, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23482, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23483: Instant IT (478), Human, 400 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23483, '478 reward: 400 XP (UB-50a, U-D07)', 'mission', 478, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23483, 'mission_completed', '478', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23483, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23483, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23483, 'mission_status', 478, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23483, 'grant_xp', NULL, NULL, '{"amount": 400}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23484: SGU Database (1425), Human, 400 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23484, '1425 reward: 400 XP (UB-50a, U-D07)', 'mission', 1425, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23484, 'mission_completed', '1425', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23484, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23484, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23484, 'mission_status', 1425, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23484, 'grant_xp', NULL, NULL, '{"amount": 400}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23485: Amateur Photography (1426), Human, 400 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23485, '1426 reward: 400 XP (UB-50a, U-D07)', 'mission', 1426, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23485, 'mission_completed', '1426', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23485, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23485, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23485, 'mission_status', 1426, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23485, 'grant_xp', NULL, NULL, '{"amount": 400}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23486: The Ruins (1427), Human, 750 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23486, '1427 reward: 750 XP (UB-50a, U-D07)', 'mission', 1427, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23486, 'mission_completed', '1427', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23486, 'archetype', NULL, NULL, 'gte', '1', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23486, 'archetype', NULL, NULL, 'lte', '4', 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23486, 'mission_status', 1427, NULL, 'eq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23486, 'grant_xp', NULL, NULL, '{"amount": 750}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23487: The SGU (1500), Free Jaffa, 250 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23487, '1500 reward: 250 XP (UB-50a, U-D07)', 'mission', 1500, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23487, 'mission_completed', '1500', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23487, 'archetype', NULL, NULL, 'eq', '7', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23487, 'mission_status', 1500, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23487, 'grant_xp', NULL, NULL, '{"amount": 250}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23488: The Nine Peaks Jaffa (1502), Free Jaffa, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23488, '1502 reward: 300 XP (UB-50a, U-D07)', 'mission', 1502, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23488, 'mission_completed', '1502', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23488, 'archetype', NULL, NULL, 'eq', '7', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23488, 'mission_status', 1502, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23488, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23489: The Antidote (1503), Free Jaffa, 350 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23489, '1503 reward: 350 XP (UB-50a, U-D07)', 'mission', 1503, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23489, 'mission_completed', '1503', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23489, 'archetype', NULL, NULL, 'eq', '7', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23489, 'mission_status', 1503, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23489, 'grant_xp', NULL, NULL, '{"amount": 350}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23490: From on High (481), Asgard, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23490, '481 reward: 300 XP (UB-50a, U-D07)', 'mission', 481, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23490, 'mission_completed', '481', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23490, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23490, 'mission_status', 481, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23490, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23491: Reclamation (420), Asgard, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23491, '420 reward: 300 XP (UB-50a, U-D07)', 'mission', 420, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23491, 'mission_completed', '420', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23491, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23491, 'mission_status', 420, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23491, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23492: Mechanical Error (421), Asgard, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23492, '421 reward: 300 XP (UB-50a, U-D07)', 'mission', 421, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23492, 'mission_completed', '421', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23492, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23492, 'mission_status', 421, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23492, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23493: Intriguing Data (1147), Asgard, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23493, '1147 reward: 300 XP (UB-50a, U-D07)', 'mission', 1147, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23493, 'mission_completed', '1147', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23493, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23493, 'mission_status', 1147, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23493, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23494: Incendiary (443), Asgard, 525 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23494, '443 reward: 525 XP (UB-50a, U-D07)', 'mission', 443, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23494, 'mission_completed', '443', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23494, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23494, 'mission_status', 443, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23494, 'grant_xp', NULL, NULL, '{"amount": 525}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23495: Covert Observations (477), Asgard, 300 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23495, '477 reward: 300 XP (UB-50a, U-D07)', 'mission', 477, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23495, 'mission_completed', '477', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23495, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23495, 'mission_status', 477, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23495, 'grant_xp', NULL, NULL, '{"amount": 300}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: U-D07 Approved 2026-10-19 (reward-proposal.md): XP = 50 x level x difficulty factor
-- Chain 23496: Process Support (479), Asgard, 350 XP.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (23496, '479 reward: 350 XP (UB-50a, U-D07)', 'mission', 479, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (23496, 'mission_completed', '479', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23496, 'archetype', NULL, NULL, 'eq', '5', 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (23496, 'mission_status', 479, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (23496, 'grant_xp', NULL, NULL, '{"amount": 350}', 0, 0);
