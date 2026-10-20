-- Castle mission 1521 (Moh'katan, symbiote tank), CS-03c1: owner-approved CIMMERIA_RECONSTRUCTION, Option A: 1386 grants 5169 on completion, CS-03S.
-- Entry 1369-1370; tank 1374-1377; completion item 1386. Each statement stays on one line (interact_tag_linter).
-- 1369 offer: archetype 8 only, once 704 is done
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1369, '1521 - Infirmary entry (Jaffa): accept 1521 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1369, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1369, 'world', 8,    NULL, 'eq', NULL,        0), (1369, 'mission_status', 704,  NULL, 'eq', 'completed', 1), (1369, 'mission_status', 1521, NULL, 'eq', 'not_active', 2), (1369, 'archetype', NULL, NULL, 'eq', '8',         3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1369, 'accept_mission', 1521, NULL, '{}', 0, 0);

-- 1370 step 4455 -> 4456 (never complete_objective 5175)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1370, '1521 - on accept: advance 4455 to 4456 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1370, 'mission_accepted', '1521', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1370, 'world', 8, NULL, 'eq', NULL, 0), (1370, 'step_status', 1521, '4455', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1370, 'advance_step', 1521, '4456', '{}', 0, 0);

-- 1374 first tank use: Livewire
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1374, '1521 - tank interact (step 4456): start Livewire (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1374, 'interact_tag', 'Castle_SymbioteTank', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1374, 'world', 8, NULL, 'eq', NULL, 0), (1374, 'step_status', 1521, '4456', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1374, 'start_minigame', NULL, 'Livewire', '{"difficulty": 1, "on_victory_chains": [1375]}', 0, 0);

-- 1375 victory: no trigger row (direct invoke); credits 5176, advance, dialog 4990 (monologue)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1375, '1521 - Livewire victory: credit 5176, advance to 4457, dialog 4990 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1375, 'world', 8, NULL, 'eq', NULL, 0), (1375, 'step_status', 1521, '4456', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1375, 'advance_step',   1521, '4457', '{}', 0, 0), (1375, 'display_dialog', 4990, NULL,   '{}', 0, 1);

-- 1376 second tank use: advance to 4458 and open 4991
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1376, '1521 - tank interact (step 4457): advance to 4458, dialog 4991 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1376, 'interact_tag', 'Castle_SymbioteTank', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1376, 'world', 8, NULL, 'eq', NULL, 0), (1376, 'step_status', 1521, '4457', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1376, 'advance_step',   1521, '4458', '{}', 0, 0), (1376, 'display_dialog', 4991, NULL,   '{}', 0, 1);

-- 1377 close: 4991 closes -> credit 5178 and complete
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1377, '1521 - dialog 4991 closed: complete 1521 (CIMMERIA_RECONSTRUCTION)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1377, 'dialog_choice', '4991', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1377, 'world', 8, NULL, 'eq', NULL, 0), (1377, 'step_status', 1521, '4458', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1377, 'complete_mission', 1521, NULL, '{}', 0, 0);

-- 1386 completion item: on a real active->completed 1521 grant the Immature Goa'uld Symbiote (5169) into the mission bag (container 2), once. mission_completed fires only on the active->completed transition (executor/mission.rs), so a second completion event needs 1521 to be reset first (CS-03dS, OD-1)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1386, '1521 - completed: grant the implant item 5169 (CIMMERIA_AUTHORING)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1386, 'mission_completed', '1521', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1386, 'world', 8, NULL, 'eq', NULL, 0), (1386, 'mission_status', 1521, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1386, 'add_item', 5169, NULL, '{"container": 2}', 0, 0);
