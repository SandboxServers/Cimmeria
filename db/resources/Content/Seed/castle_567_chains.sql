-- Castle 567 Romney's Files (CS-08bS): chains 1394-1397, 1400, 10240-10243. CIMMERIA_AUTHORING.
-- All nine chains ship ENABLED, together with Harset 6503/6504 (FZ-12, decision D-FZ1).
-- 1397 grants item 2698 at step 2000; Harset 6503 takes it back at step 4039. Disable both
-- legs together or not at all: either one alone strands a player holding 2698.
-- 1394: 703 completed in world 8 -> offer Copplemann (slot 48)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1394, '567 - 703 completed: offer Romney''s Files on Copplemann (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1394, 'mission_completed', '703', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1394, 'world', 8, NULL, 'eq', NULL, 0), (1394, 'mission_status', 701, NULL, 'eq', 'completed', 1), (1394, 'mission_status', 703, NULL, 'eq', 'completed', 2), (1394, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1394, 'add_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);

-- 1395: Castle login, offer open -> rebind
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1395, '567 - Castle login, offer open: rebind Copplemann (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1395, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1395, 'world', 8, NULL, 'eq', NULL, 0), (1395, 'mission_status', 701, NULL, 'eq', 'completed', 1), (1395, 'mission_status', 703, NULL, 'eq', 'completed', 2), (1395, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1395, 'add_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);

-- 1396: dialog 2043 Accept -> unbind, accept
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1396, '567 - Dialog 2043 accept: unbind offer, accept 567 (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1396, 'dialog_choice', '2043', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1396, 'world', 8, NULL, 'eq', NULL, 0), (1396, 'mission_status', 701, NULL, 'eq', 'completed', 1), (1396, 'mission_status', 703, NULL, 'eq', 'completed', 2), (1396, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1396, 'remove_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0), (1396, 'accept_mission', 567, NULL, '{}', 0, 1);

-- 1397: cabinet click at step 2000 -> grant 2698, advance to 2012, feedback
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1397, '567 - Castle file cabinet at 2000: take Romney''s Files, advance to 2012 (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1397, 'interact_tag', 'Castle_FileCabinet', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1397, 'world', 8, NULL, 'eq', NULL, 0), (1397, 'mission_status', 567, NULL, 'eq', 'active', 1), (1397, 'step_status', 567, '2000', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1397, 'add_item', 2698, NULL, '{"qty": 1, "container": 2}', 0, 0), (1397, 'advance_step', 567, '2012', '{}', 0, 1), (1397, 'display_dialog', 10733, NULL, '{}', 0, 2);

-- 1400: cabinet click at any other state -> visible feedback (disjoint with 1397 by the step-2000 condition)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1400, '567 - Castle file cabinet, nothing to take: feedback (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1400, 'interact_tag', 'Castle_FileCabinet', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1400, 'world', 8, NULL, 'eq', NULL, 0), (1400, 'step_status', 567, '2000', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1400, 'display_dialog', 10734, NULL, '{}', 0, 0);

-- 10240: leaving world 8 with the offer unaccepted -> unbind (never live in world 68)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10240, '567 - Leave Castle, offer unaccepted: unbind Copplemann offer (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10240, 'stargate_crossed', NULL, 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10240, 'world', 8, NULL, 'eq', NULL, 0), (10240, 'mission_status', 567, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10240, 'remove_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);

-- 10241: Castle -> Harset crossing at 2012 -> 4039
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10241, '567 - Castle exit route: advance Romney''s Files to 4039 (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10241, 'stargate_crossed', 'Harset', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10241, 'world', 8, NULL, 'eq', NULL, 0), (10241, 'mission_status', 567, NULL, 'eq', 'active', 1), (10241, 'step_status', 567, '2012', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10241, 'advance_step', 567, '4039', '{}', 0, 0);

-- 10242: abandon in any world -> clear turn-in bind and the item
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10242, '567 - Abandoned: clear the turn-in bind and Romney''s Files (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10242, 'mission_abandoned', '567', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10242, 'mission_status', 567, NULL, 'eq', 'not_active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10242, 'remove_dialog_set', 2817, NULL, '{"slot": 48}', 0, 0), (10242, 'remove_item', 2698, NULL, '{"qty": 1}', 0, 1);

-- 10243: abandon in world 8 -> reopen the offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10243, '567 - Abandoned in Castle: rebind Copplemann offer (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10243, 'mission_abandoned', '567', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10243, 'world', 8, NULL, 'eq', NULL, 0), (10243, 'mission_status', 701, NULL, 'eq', 'completed', 1), (10243, 'mission_status', 703, NULL, 'eq', 'completed', 2), (10243, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10243, 'add_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);
