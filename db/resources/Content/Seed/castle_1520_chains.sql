-- Castle mission 1520 (Ambernol, Castle Infirmary), CS-02c1: owner-approved CIMMERIA_RECONSTRUCTION, CS-02S.
-- Entry 1367-1368; dispenser 1371-1373; Ogilvie talk and relog 1378-1383. Each statement stays on one line (interact_tag_linter).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1367, '1520 - Castle.Infirmary enter: offer Ambernol mission (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1367, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1367, 'world', 8, NULL, 'eq', NULL, 0), (1367, 'mission_status', 704, NULL, 'eq', 'completed', 1), (1367, 'mission_status', 1520, NULL, 'eq', 'not_active', 2), (1367, 'archetype', NULL, NULL, 'gte', '1', 3), (1367, 'archetype', NULL, NULL, 'lte', '6', 4), (1367, 'archetype', NULL, NULL, 'neq', '5', 5);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1367, 'accept_mission', 1520, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1368, '1520 - Castle.Infirmary enter: credit objective 5171 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1368, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1368, 'world', 8, NULL, 'eq', NULL, 0), (1368, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1368, 'step_status', 1520, '4452', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1368, 'complete_objective', 1520, '5171', '{}', 0, 0);

-- 1371: dispenser click while 4453 is active starts Livewire
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1371, '1520 - Med Dispenser interact (step 4453): start Livewire (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1371, 'interact_tag', 'Castle_MedDispenser', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1371, 'world', 8, NULL, 'eq', NULL, 0), (1371, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1371, 'step_status', 1520, '4453', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1371, 'start_minigame', NULL, 'Livewire', '{"difficulty": 1, "on_victory_chains": [1372]}', 0, 0);

-- 1372: Livewire victory (no trigger; fired through the b2-lite guarded seam)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1372, '1520 - Livewire victory: advance to 4454, show 4984 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1372, 'world', 8, NULL, 'eq', NULL, 0), (1372, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1372, 'step_status', 1520, '4453', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1372, 'remove_dialog_set', 5828, NULL, '{"slot": 412}', 0, 0), (1372, 'advance_step', 1520, '4454', '{}', 0, 1), (1372, 'display_dialog', 4984, NULL, '{}', 0, 2);

-- 1373: 4985 close completes the mission
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1373, '1520 - Dialog 4985 close: complete 1520 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1373, 'dialog_choice', '4985', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1373, 'world', 8, NULL, 'eq', NULL, 0), (1373, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1373, 'step_status', 1520, '4454', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1373, 'complete_mission', 1520, NULL, '{}', 0, 0);

-- 1378: Ogilvie talk closed (4983): advance to 4453, hand the player the dispenser glow
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1378, '1520 - Dialog 4983 close (Ogilvie): advance to 4453, bind dispenser (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1378, 'dialog_choice', '4983', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1378, 'world', 8, NULL, 'eq', NULL, 0), (1378, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1378, 'step_status', 1520, '4452', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1378, 'remove_dialog_set', 5827, NULL, '{"slot": 179}', 0, 0), (1378, 'advance_step', 1520, '4453', '{}', 0, 1), (1378, 'add_dialog_set', 5828, NULL, '{"slot": 412}', 0, 2);

-- 1379: 4984 close shows the 4985 radio call
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1379, '1520 - Dialog 4984 close: show radio call 4985 (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1379, 'dialog_choice', '4984', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1379, 'world', 8, NULL, 'eq', NULL, 0), (1379, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1379, 'step_status', 1520, '4454', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1379, 'display_dialog', 4985, NULL, '{}', 0, 0);

-- 1380: bind Ogilvie when 1520 is accepted
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1380, '1520 - Accepted: bind Ogilvie talk (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1380, 'mission_accepted', '1520', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1380, 'world', 8, NULL, 'eq', NULL, 0), (1380, 'step_status', 1520, '4452', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1380, 'add_dialog_set', 5827, NULL, '{"slot": 179}', 0, 0);

-- 1381 / 1382 / 1383: relog restores (one step each)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1381, '1520 - Login at 4452: rebind Ogilvie (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1381, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1381, 'world', 8, NULL, 'eq', NULL, 0), (1381, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1381, 'step_status', 1520, '4452', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1381, 'add_dialog_set', 5827, NULL, '{"slot": 179}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1382, '1520 - Login at 4453: rebind dispenser (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1382, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1382, 'world', 8, NULL, 'eq', NULL, 0), (1382, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1382, 'step_status', 1520, '4453', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1382, 'add_dialog_set', 5828, NULL, '{"slot": 412}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1383, '1520 - Login at 4454: complete (radio call cannot be shown) (CIMMERIA_RECONSTRUCTION)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1383, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1383, 'world', 8, NULL, 'eq', NULL, 0), (1383, 'mission_status', 1520, NULL, 'eq', 'active', 1), (1383, 'step_status', 1520, '4454', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1383, 'complete_mission', 1520, NULL, '{}', 0, 0);
