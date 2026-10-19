-- Beta Site E1 (world 23, Beta_Site_Evo_1) - Incendiary: Carter, Satterfield and the Quartermaster fire (UB-34, mission 443).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: mission 443 steps S1482 (O1721, Carter), S1483 (O1722, Satterfield), S1484 (the radio, HELD-B14:
-- no dialog, the step update is the feedback), S1485 (O1724, the suppressant manager), S1486 (O1725, the
-- fire control hub), S3455 (O1726, the turn-in to Mimir). Mission 443 is RESTORED_SOURCE (mission_objectives.sql,
-- mission_tasks.sql; mission-seal.md 443 table). The chains (23405-23417) are CIMMERIA_AUTHORING on that list.
-- The en=false flag on O1725 is inert on the server (mission-seal.md Q3, C4): every step advances with advance_step.
-- Dialogs: blurb 1919 (dsm 4581), offer 1918 (dsm 2352, Accept), Carter 3895 (dsm 3728), Satterfield 3161 (dsm 3780),
-- turn-in 1917 (dsm 4579). Spawns BetaE1_FireSuppressantManager (2328), BetaE1_FireControlHub (2329), from UB-02c.
-- Region: Beta_Site_Evo_1.FireSite (point set 23031, UB-02c). The fires are static map emitters (F18): nothing puts them out.
-- Gate: Asgard (archetype 5) only. Omega UO-16 is HELD (HELD-B16), so this is replay-tested only (D-UB13).
-- F19: every step here has one required objective that advance_step force-completes, so no chain uses complete_objective.
-- Guards: chain_replay_tests/beta_e1/m443.rs.

-- Chain 23405: player_loaded and mission_completed 1147, Asgard, 1147 completed, 443 not_active -> bind the blurb 4581.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23405, 'Beta_Site_Evo_1 - Mimir blurb bind for Incendiary (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23405, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23405, 'mission_completed', '1147', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23405, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23405, 'mission_status', 1147, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23405, 'mission_status', 443, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23405, 'add_dialog_set', 4581, NULL, '{"slot": 2305, "mission_id": 443}', 0, 0);

-- Chain 23406: interact_tag BetaE1_Mimir, Asgard, 1147 completed, 443 not_active -> show the offer 1918.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23406, 'Beta_Site_Evo_1 - Mimir offers Incendiary: show the offer (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23406, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23406, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23406, 'mission_status', 1147, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23406, 'mission_status', 443, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23406, 'display_dialog', 1918, NULL, '{}', 0, 0);

-- Chain 23407: dialog_choice 1918 (Accept), Asgard, 1147 completed, 443 not_active -> accept 443, drop the blurb,
-- bind Carter's dialog 3728 for S1482.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23407, 'Beta_Site_Evo_1 - Accept Incendiary: drop the blurb, bind Carter (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23407, 'dialog_choice', '1918', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23407, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23407, 'mission_status', 1147, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23407, 'mission_status', 443, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23407, 'accept_mission', 443, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23407, 'remove_dialog_set', 4581, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23407, 'add_dialog_set', 3728, NULL, '{"slot": 2300, "mission_id": 443}', 0, 2);

-- Chain 23408: player_loaded, Asgard, S1482 active -> restore Carter's bind 3728 after a relog (no binds persist).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23408, 'Beta_Site_Evo_1 - Carter bind restore for Incendiary (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23408, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23408, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23408, 'step_status', 443, '1482', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23408, 'add_dialog_set', 3728, NULL, '{"slot": 2300, "mission_id": 443}', 0, 0);

-- Chain 23409: interact_tag BetaE1_Carter, Asgard, S1482 active -> show 3895, drop the bind 3728, advance to S1483,
-- bind Satterfield's dialog 3780.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23409, 'Beta_Site_Evo_1 - Carter sends you to Satterfield: advance to S1483 (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23409, 'interact_tag', 'BetaE1_Carter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23409, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23409, 'step_status', 443, '1482', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23409, 'display_dialog', 3895, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23409, 'remove_dialog_set', 3728, NULL, '{"slot": 2300}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23409, 'advance_step', 443, '1483', '{}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23409, 'add_dialog_set', 3780, NULL, '{"slot": 2302, "mission_id": 443}', 0, 3);

-- Chain 23410: player_loaded, Asgard, S1483 active -> restore Satterfield's bind 3780 after a relog.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23410, 'Beta_Site_Evo_1 - Satterfield bind restore for Incendiary (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23410, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23410, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23410, 'step_status', 443, '1483', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23410, 'add_dialog_set', 3780, NULL, '{"slot": 2302, "mission_id": 443}', 0, 0);

-- Chain 23411: interact_tag BetaE1_Satterfield, Asgard, S1483 active -> show 3161, drop the bind 3780, advance to S1484.
-- Satterfield stands at least 2 m outside FireSite (UB-02c), so the radio step starts outside the region.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23411, 'Beta_Site_Evo_1 - Satterfield sends you to the fire: advance to S1484 (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23411, 'interact_tag', 'BetaE1_Satterfield', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23411, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23411, 'step_status', 443, '1483', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23411, 'display_dialog', 3161, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23411, 'remove_dialog_set', 3780, NULL, '{"slot": 2302}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23411, 'advance_step', 443, '1484', '{}', 0, 2);

-- Chain 23412: enter_region Beta_Site_Evo_1.FireSite, Asgard, S1484 active -> advance to S1485 (the radio call, HELD-B14).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23412, 'Beta_Site_Evo_1 - Reach the fire: advance to S1485 (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23412, 'enter_region', 'Beta_Site_Evo_1.FireSite', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23412, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23412, 'step_status', 443, '1484', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23412, 'advance_step', 443, '1485', '{}', 0, 0);

-- Chain 23413: interact_tag BetaE1_FireSuppressantManager, Asgard, S1484 active -> advance to S1485 (the fallback
-- for a player who already stands in FireSite when S1484 starts).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23413, 'Beta_Site_Evo_1 - Suppressant manager at S1484 (fallback): advance to S1485 (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23413, 'interact_tag', 'BetaE1_FireSuppressantManager', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23413, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23413, 'step_status', 443, '1484', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23413, 'advance_step', 443, '1485', '{}', 0, 0);

-- Chain 23414: interact_tag BetaE1_FireSuppressantManager, Asgard, S1485 active -> advance to S1486 (O1724, contain the fire).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23414, 'Beta_Site_Evo_1 - Suppressant manager: contain the fire, advance to S1486 (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23414, 'interact_tag', 'BetaE1_FireSuppressantManager', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23414, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23414, 'step_status', 443, '1485', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23414, 'advance_step', 443, '1486', '{}', 0, 0);

-- Chain 23415: interact_tag BetaE1_FireControlHub, Asgard, S1486 active -> advance to S3455 (O1725, en=false ignored),
-- bind Mimir's turn-in dialog 4579.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23415, 'Beta_Site_Evo_1 - Fire Control Hub: advance to S3455, bind the turn-in (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23415, 'interact_tag', 'BetaE1_FireControlHub', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23415, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23415, 'step_status', 443, '1486', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23415, 'advance_step', 443, '3455', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23415, 'add_dialog_set', 4579, NULL, '{"slot": 2305, "mission_id": 443}', 0, 1);

-- Chain 23416: player_loaded, Asgard, S3455 active -> restore the turn-in bind 4579 after a relog.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23416, 'Beta_Site_Evo_1 - Mimir turn-in bind restore for Incendiary (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23416, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23416, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23416, 'step_status', 443, '3455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23416, 'add_dialog_set', 4579, NULL, '{"slot": 2305, "mission_id": 443}', 0, 0);

-- Chain 23417: interact_tag BetaE1_Mimir, Asgard, S3455 active -> turn-in 1917, drop the bind 4579, complete 443.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23417, 'Beta_Site_Evo_1 - Mimir turn-in for Incendiary: complete the mission (UB-34)', 'mission', 443, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23417, 'interact_tag', 'BetaE1_Mimir', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23417, 'archetype', NULL, NULL, 'eq', '5', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23417, 'step_status', 443, '3455', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23417, 'display_dialog', 1917, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23417, 'remove_dialog_set', 4579, NULL, '{"slot": 2305}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23417, 'complete_mission', 443, NULL, '{}', 0, 2);
