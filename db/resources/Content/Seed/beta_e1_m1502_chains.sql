-- Beta Site E1 (world 23, Beta_Site_Evo_1) - 1502 The Nine Peaks Jaffa (UB-21).
-- Plan: docs/analysis/union-zone-planning/beta-work-packets.md. Block 23000-23499.
-- Chains use only per-player binds (D-UB10) and no phasing (D-UB11).
SET search_path = resources, pg_catalog;

-- Provenance: RESTORED_SOURCE for the steps (S4376-S4378, S4481), the dialogs
-- (blurb 5040 in dsm 5885, offer 5041 in dsm 5886, box 5042 in dsm 7151, box 5043
-- in dsm 7152, done 5044 in dsm 5887) and the medallion item 5175. Ras'nor is the
-- offerer, template 2306, so every bind slot is 2306. The box hack minigame is
-- replaced by its button press (CIMMERIA_RECONSTRUCTION): buttons 575 and 576 fire
-- `dialog_choice` (mission-seal.md Q6). Region `Beta_Site_Evo_1.NinePeaks` (set 23060)
-- is the go-to trigger for S4376, with the box click as its fallback (P10).
-- Guards: `chain_replay_tests/beta_e1/m1502.rs`.

-- Chain 23240: offer bind. player_loaded or mission_completed 1500, Free Jaffa,
-- 1500 completed, 1502 not_active -> bind the blurb dsm 5885 to Ras'nor (P2).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23240, 'Beta_Site_Evo_1 - Free Jaffa offer bind: Ras''nor blurb 5885 after 1500 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23240, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23240, 'mission_completed', '1500', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23240, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23240, 'mission_status', 1500, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23240, 'mission_status', 1502, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23240, 'add_dialog_set', 5885, NULL, '{"slot": 2306, "mission_id": 1502}', 0, 0);

-- Chain 23241: offer show. interact_tag BetaE1_RasNor, Free Jaffa, 1500 completed,
-- 1502 not_active -> Ras'nor's offer dialog 5041 (P2).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23241, 'Beta_Site_Evo_1 - Ras''nor offer for the Free Jaffa: dialog 5041 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23241, 'interact_tag', 'BetaE1_RasNor', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23241, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23241, 'mission_status', 1500, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23241, 'mission_status', 1502, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23241, 'display_dialog', 5041, NULL, '{}', 0, 0);

-- Chain 23242: accept. dialog_choice 5041, Free Jaffa, 1500 completed, 1502
-- not_active -> accept 1502, unbind the blurb (P2 accept). S4376 has no bind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23242, 'Beta_Site_Evo_1 - Ras''nor offer accepted: accept 1502, unbind blurb 5885 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23242, 'dialog_choice', '5041', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23242, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23242, 'mission_status', 1500, NULL, 'eq', 'completed', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23242, 'mission_status', 1502, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23242, 'accept_mission', 1502, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23242, 'remove_dialog_set', 5885, NULL, '{"slot": 2306}', 0, 1);

-- Chain 23243: go-to. enter_region Beta_Site_Evo_1.NinePeaks, Free Jaffa, S4376
-- active -> S4377 (P10 region trigger).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23243, 'Beta_Site_Evo_1 - Nine Peaks camp reached: 1502 S4376 to S4377 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23243, 'enter_region', 'Beta_Site_Evo_1.NinePeaks', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23243, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23243, 'step_status', 1502, '4376', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23243, 'advance_step', 1502, '4377', '{}', 0, 0);

-- Chain 23244: go-to fallback. A click on the Jaffa box while S4376 is active
-- (a player already inside the region never fires enter_region) -> S4377 and the
-- first box dialog 5042 (P10 fallback).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23244, 'Beta_Site_Evo_1 - Jaffa box click at S4376: 1502 to S4377, dialog 5042 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23244, 'interact_tag', 'BetaE1_JaffaBox', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23244, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23244, 'step_status', 1502, '4376', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23244, 'advance_step', 1502, '4377', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23244, 'display_dialog', 5042, NULL, '{}', 0, 1);

-- Chain 23245: box click at S4377 -> first box dialog 5042 (P5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23245, 'Beta_Site_Evo_1 - Jaffa box click at S4377: dialog 5042 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23245, 'interact_tag', 'BetaE1_JaffaBox', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23245, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23245, 'step_status', 1502, '4377', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23245, 'display_dialog', 5042, NULL, '{}', 0, 0);

-- Chain 23246: first box button (dialog_choice 5042, button 575) at S4377 ->
-- S4378 and the second box dialog 5043 (P5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23246, 'Beta_Site_Evo_1 - Jaffa box first button at S4377: 1502 to S4378, dialog 5043 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23246, 'dialog_choice', '5042', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23246, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23246, 'step_status', 1502, '4377', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23246, 'advance_step', 1502, '4378', '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23246, 'display_dialog', 5043, NULL, '{}', 0, 1);

-- Chain 23247: box click at S4378 -> second box dialog 5043 (P5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23247, 'Beta_Site_Evo_1 - Jaffa box click at S4378: dialog 5043 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23247, 'interact_tag', 'BetaE1_JaffaBox', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23247, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23247, 'step_status', 1502, '4378', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23247, 'display_dialog', 5043, NULL, '{}', 0, 0);

-- Chain 23248: second box button (dialog_choice 5043, button 576) at S4378 ->
-- the medallion 5175, S4481 and the Ras'nor return bind 5887 (P5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23248, 'Beta_Site_Evo_1 - Jaffa box second button at S4378: medallion 5175, 1502 to S4481, bind 5887 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23248, 'dialog_choice', '5043', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23248, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23248, 'step_status', 1502, '4378', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23248, 'add_item', 5175, NULL, '{"container": 0, "qty": 1}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23248, 'advance_step', 1502, '4481', '{}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23248, 'add_dialog_set', 5887, NULL, '{"slot": 2306, "mission_id": 1502}', 0, 2);

-- Chain 23249: relog restore. player_loaded, Free Jaffa, S4481 active -> re-bind
-- Ras'nor's done dialog 5044 (dsm 5887). Binds are not persisted.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23249, 'Beta_Site_Evo_1 - Free Jaffa restore: bind Ras''nor done dialog 5887 while S4481 is active (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23249, 'player_loaded', 'Beta_Site_Evo_1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23249, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23249, 'step_status', 1502, '4481', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23249, 'add_dialog_set', 5887, NULL, '{"slot": 2306, "mission_id": 1502}', 0, 0);

-- Chain 23250: turn-in. interact_tag BetaE1_RasNor, Free Jaffa, S4481 active ->
-- done dialog 5044, unbind 5887, remove the medallion, complete 1502 (P3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (23250, 'Beta_Site_Evo_1 - Ras''nor takes the medallion: dialog 5044, complete 1502 (UB-21)', 'mission', 1502, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (23250, 'interact_tag', 'BetaE1_RasNor', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23250, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (23250, 'step_status', 1502, '4481', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23250, 'display_dialog', 5044, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23250, 'remove_dialog_set', 5887, NULL, '{"slot": 2306}', 0, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23250, 'remove_item', 5175, NULL, '{"qty": 1}', 0, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (23250, 'complete_mission', 1502, NULL, '{}', 0, 3);
