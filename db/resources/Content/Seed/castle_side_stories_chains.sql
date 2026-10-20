-- Castle Q1 medical (CS-04aD): chains 10000-10006. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10000, '10300 - 704 completed: offer the medic (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10000, 'mission_completed', '704', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10000, 'world', 8, NULL, 'eq', NULL, 0), (10000, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10000, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10000, 'add_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10001, '10300 - Login, offer open: rebind the medic (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10001, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10001, 'world', 8, NULL, 'eq', NULL, 0), (10001, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10001, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10001, 'add_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10002, '10300 - Abandoned: rebind the medic (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10002, 'mission_abandoned', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10002, 'world', 8, NULL, 'eq', NULL, 0), (10002, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10002, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10002, 'add_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10003, '10300 - Medic click: accept (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10003, 'interact_tag', 'Castle_Pop_Infirmary_Medic1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10003, 'world', 8, NULL, 'eq', NULL, 0), (10003, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10003, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10003, 'accept_mission', 10300, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10004, '10300 - Accepted: advance to B, unbind medic, show 10700 (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10004, 'mission_accepted', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10004, 'world', 8, NULL, 'eq', NULL, 0), (10004, 'step_status', 10300, '10340', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10004, 'advance_step', 10300, '10341', '{}', 0, 0), (10004, 'remove_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 1), (10004, 'display_dialog', 10700, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10005, '10300 - Castle.CastleFrontCourtyard enter at B: complete (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10005, 'enter_region', 'Castle.CastleFrontCourtyard', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10005, 'world', 8, NULL, 'eq', NULL, 0), (10005, 'mission_status', 10300, NULL, 'eq', 'active', 1), (10005, 'step_status', 10300, '10341', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10005, 'complete_objective', 10300, '10461', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10006, '10300 - Login stuck at A: heal to B (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10006, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10006, 'world', 8, NULL, 'eq', NULL, 0), (10006, 'mission_status', 10300, NULL, 'eq', 'active', 1), (10006, 'step_status', 10300, '10340', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10006, 'advance_step', 10300, '10341', '{}', 0, 0);
-- Castle Q2 medical (CS-04aD2): chains 10010-10017. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10010, '10301 - 10300 completed: offer the prisoner (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10010, 'mission_completed', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10010, 'world', 8, NULL, 'eq', NULL, 0), (10010, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10010, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10010, 'add_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10011, '10301 - Login, offer open: rebind the prisoner (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10011, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10011, 'world', 8, NULL, 'eq', NULL, 0), (10011, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10011, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10011, 'add_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10012, '10301 - Abandoned: rebind the prisoner, unbind the medic (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10012, 'mission_abandoned', '10301', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10012, 'world', 8, NULL, 'eq', NULL, 0), (10012, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10012, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10012, 'add_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 0), (10012, 'remove_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10013, '10301 - Prisoner click: accept (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10013, 'interact_tag', 'Castle_Pop_Prisoner5', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10013, 'world', 8, NULL, 'eq', NULL, 0), (10013, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10013, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10013, 'accept_mission', 10301, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10014, '10301 - Accepted: advance to B, swap markers, show 10701 (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10014, 'mission_accepted', '10301', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10014, 'world', 8, NULL, 'eq', NULL, 0), (10014, 'step_status', 10301, '10342', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10014, 'advance_step', 10301, '10343', '{}', 0, 0), (10014, 'remove_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 1), (10014, 'add_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 2), (10014, 'display_dialog', 10701, NULL, '{}', 0, 3);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10015, '10301 - Medic click at B: unbind, show 10702, complete (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10015, 'interact_tag', 'Castle_Pop_Infirmary_Medic1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10015, 'world', 8, NULL, 'eq', NULL, 0), (10015, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10015, 'mission_status', 10301, NULL, 'eq', 'active', 2), (10015, 'step_status', 10301, '10343', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10015, 'remove_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 0), (10015, 'display_dialog', 10702, NULL, '{}', 0, 1), (10015, 'complete_objective', 10301, '10463', '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10016, '10301 - Login stuck at A: heal to B, bind the medic (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10016, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10016, 'world', 8, NULL, 'eq', NULL, 0), (10016, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10016, 'mission_status', 10301, NULL, 'eq', 'active', 2), (10016, 'step_status', 10301, '10342', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10016, 'advance_step', 10301, '10343', '{}', 0, 0), (10016, 'add_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10017, '10301 - Login at B: rebind the medic (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10017, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10017, 'world', 8, NULL, 'eq', NULL, 0), (10017, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10017, 'mission_status', 10301, NULL, 'eq', 'active', 2), (10017, 'step_status', 10301, '10343', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10017, 'add_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 0);

-- Castle defense_q1 (CS-04bD2a): chains 10100-10106. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10100, '10310 - 10300 completed: offer the sentry (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10100, 'mission_completed', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10100, 'world', 8, NULL, 'eq', NULL, 0), (10100, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10100, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10100, 'add_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10101, '10310 - Login, offer open: rebind the sentry (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10101, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10101, 'world', 8, NULL, 'eq', NULL, 0), (10101, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10101, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10101, 'add_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10102, '10310 - Abandoned: rebind the sentry (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10102, 'mission_abandoned', '10310', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10102, 'world', 8, NULL, 'eq', NULL, 0), (10102, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10102, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10102, 'add_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10103, '10310 - Sentry click: accept (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10103, 'interact_tag', 'Castle_DefenseQ1_Sentry', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10103, 'world', 8, NULL, 'eq', NULL, 0), (10103, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10103, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10103, 'accept_mission', 10310, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10104, '10310 - Accepted: advance to B, unbind sentry, show 10709 (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10104, 'mission_accepted', '10310', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10104, 'world', 8, NULL, 'eq', NULL, 0), (10104, 'step_status', 10310, '10354', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10104, 'advance_step', 10310, '10355', '{}', 0, 0), (10104, 'remove_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 1), (10104, 'display_dialog', 10709, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10105, '10310 - Castle.Infirmary enter at B: complete (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10105, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10105, 'world', 8, NULL, 'eq', NULL, 0), (10105, 'mission_status', 10310, NULL, 'eq', 'active', 1), (10105, 'step_status', 10310, '10355', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10105, 'complete_objective', 10310, '10475', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10106, '10310 - Login stuck at A: heal to B (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10106, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10106, 'world', 8, NULL, 'eq', NULL, 0), (10106, 'mission_status', 10310, NULL, 'eq', 'active', 1), (10106, 'step_status', 10310, '10354', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10106, 'advance_step', 10310, '10355', '{}', 0, 0);

-- Castle defense_q3 (CS-04bD2b): chains 10120-10129. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10120, '10312 - 10310 completed: offer the giver (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10120, 'mission_completed', '10310', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10120, 'world', 8, NULL, 'eq', NULL, 0), (10120, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10120, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10120, 'add_dialog_set', 1000010, NULL, '{"slot": 425}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10121, '10312 - Login, offer open: rebind the giver (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10121, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10121, 'world', 8, NULL, 'eq', NULL, 0), (10121, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10121, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10121, 'add_dialog_set', 1000010, NULL, '{"slot": 425}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10122, '10312 - Abandoned: rebind the giver (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10122, 'mission_abandoned', '10312', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10122, 'world', 8, NULL, 'eq', NULL, 0), (10122, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10122, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10122, 'add_dialog_set', 1000010, NULL, '{"slot": 425}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10123, '10312 - Giver click: accept (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10123, 'interact_tag', 'Castle_DefenseQ3_Giver', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10123, 'world', 8, NULL, 'eq', NULL, 0), (10123, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10123, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10123, 'accept_mission', 10312, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10124, '10312 - Accepted: unbind the giver, show 10717 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10124, 'mission_accepted', '10312', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10124, 'world', 8, NULL, 'eq', NULL, 0), (10124, 'step_status', 10312, '10366', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10124, 'remove_dialog_set', 1000010, NULL, '{"slot": 425}', 0, 0), (10124, 'display_dialog', 10717, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10125, '10312 - Kill Bravo officer 1: complete 10486 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10125, 'entity_dead_tag', 'Castle_BravoOfficer1', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10125, 'world', 8, NULL, 'eq', NULL, 0), (10125, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10125, 'step_status', 10312, '10366', 'eq', 'active', 2), (10125, 'objective_status', 10312, '10486', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10125, 'complete_objective', 10312, '10486', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10126, '10312 - Kill Bravo officer 2: complete 10487 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10126, 'entity_dead_tag', 'Castle_BravoOfficer2', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10126, 'world', 8, NULL, 'eq', NULL, 0), (10126, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10126, 'step_status', 10312, '10366', 'eq', 'active', 2), (10126, 'objective_status', 10312, '10487', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10126, 'complete_objective', 10312, '10487', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10127, '10312 - Kill Bravo officer 3: complete 10488 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10127, 'entity_dead_tag', 'Castle_BravoOfficer3', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10127, 'world', 8, NULL, 'eq', NULL, 0), (10127, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10127, 'step_status', 10312, '10366', 'eq', 'active', 2), (10127, 'objective_status', 10312, '10488', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10127, 'complete_objective', 10312, '10488', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10128, '10312 - Kill Muelbach: complete 10489 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10128, 'entity_dead_tag', 'Castle_Muelbach', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10128, 'world', 8, NULL, 'eq', NULL, 0), (10128, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10128, 'step_status', 10312, '10366', 'eq', 'active', 2), (10128, 'objective_status', 10312, '10489', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10128, 'complete_objective', 10312, '10489', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10129, '10312 - Completed: show 10718 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10129, 'mission_completed', '10312', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10129, 'world', 8, NULL, 'eq', NULL, 0), (10129, 'mission_status', 10312, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10129, 'display_dialog', 10718, NULL, '{}', 0, 0);

-- Castle return arc (CS-04cS2): return_q1 chains 10130-10133. CIMMERIA_AUTHORING.
-- Crossing accept: stargate_crossed 'Castle' runs in the ORIGIN world (57); the accept is persisted before GateTravel is sent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10130, '10320 - Harset to Castle crossing: accept Report from Harset (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10130, 'stargate_crossed', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10130, 'world', 57, NULL, 'eq', NULL, 0), (10130, 'mission_status', 708, NULL, 'eq', 'completed', 1), (10130, 'mission_status', 10320, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10130, 'accept_mission', 10320, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10131, '10320 - Arrival or login at A: bind Marsh (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10131, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10131, 'world', 8, NULL, 'eq', NULL, 0), (10131, 'mission_status', 10320, NULL, 'eq', 'active', 1), (10131, 'step_status', 10320, '10372', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10131, 'add_dialog_set', 1000012, NULL, '{"slot": 10}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10132, '10320 - Marsh click at A: report, unbind, complete (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10132, 'interact_tag', 'Castle_ColMarsh', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10132, 'world', 8, NULL, 'eq', NULL, 0), (10132, 'mission_status', 10320, NULL, 'eq', 'active', 1), (10132, 'step_status', 10320, '10372', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10132, 'display_dialog', 10721, NULL, '{}', 0, 0), (10132, 'remove_dialog_set', 1000012, NULL, '{"slot": 10}', 0, 1), (10132, 'complete_objective', 10320, '10492', '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10133, '10320 - Abandoned: unbind Marsh (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10133, 'mission_abandoned', '10320', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10133, 'world', 8, NULL, 'eq', NULL, 0), (10133, 'mission_status', 10320, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10133, 'remove_dialog_set', 1000012, NULL, '{"slot": 10}', 0, 0);

-- Castle return_q2 (CS-04c2 / CS-04cS2): chains 10140-10146. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10140, '10321 - 10320 completed: offer Moh''katan (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10140, 'mission_completed', '10320', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10140, 'world', 8, NULL, 'eq', NULL, 0), (10140, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10140, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10140, 'add_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10141, '10321 - Login, offer open: rebind Moh''katan (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10141, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10141, 'world', 8, NULL, 'eq', NULL, 0), (10141, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10141, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10141, 'add_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10142, '10321 - Moh''katan click: accept (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10142, 'interact_tag', 'Castle_Mohkatan', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10142, 'world', 8, NULL, 'eq', NULL, 0), (10142, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10142, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10142, 'accept_mission', 10321, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10143, '10321 - Accepted: advance to B, unbind Moh''katan, show 10725 (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10143, 'mission_accepted', '10321', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10143, 'world', 8, NULL, 'eq', NULL, 0), (10143, 'step_status', 10321, '10378', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10143, 'advance_step', 10321, '10379', '{}', 0, 0), (10143, 'remove_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 1), (10143, 'display_dialog', 10725, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10144, '10321 - Harset dialed at B: complete (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10144, 'stargate_dialed', 'Harset', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10144, 'world', 8, NULL, 'eq', NULL, 0), (10144, 'mission_status', 10321, NULL, 'eq', 'active', 1), (10144, 'step_status', 10321, '10379', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10144, 'complete_objective', 10321, '10499', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10145, '10321 - Login stuck at A: heal to B (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10145, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10145, 'world', 8, NULL, 'eq', NULL, 0), (10145, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10145, 'mission_status', 10321, NULL, 'eq', 'active', 2), (10145, 'step_status', 10321, '10378', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10145, 'advance_step', 10321, '10379', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10146, '10321 - Abandoned: rebind Moh''katan (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10146, 'mission_abandoned', '10321', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10146, 'world', 8, NULL, 'eq', NULL, 0), (10146, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10146, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10146, 'add_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 0);
