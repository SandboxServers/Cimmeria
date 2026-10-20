-- Castle repeatable_q1 Supply Run (CS-05S3): chains 10230-10237. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10230, '10330 - 704 completed: offer the supply officer (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10230, 'mission_completed', '704', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10230, 'world', 8, NULL, 'eq', NULL, 0), (10230, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10230, 'mission_status', 10330, NULL, 'neq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10230, 'add_dialog_set', 1000016, NULL, '{"slot": 424}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10231, '10330 - Login, offer open: rebind the supply officer (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10231, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10231, 'world', 8, NULL, 'eq', NULL, 0), (10231, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10231, 'mission_status', 10330, NULL, 'neq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10231, 'add_dialog_set', 1000016, NULL, '{"slot": 424}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10232, '10330 - Abandoned: swap turn-in for offer (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10232, 'mission_abandoned', '10330', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10232, 'world', 8, NULL, 'eq', NULL, 0), (10232, 'mission_status', 704, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10232, 'remove_dialog_set', 1000017, NULL, '{"slot": 424}', 0, 0), (10232, 'add_dialog_set', 1000016, NULL, '{"slot": 424}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10233, '10330 - Completed: rebind the offer for the next run (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10233, 'mission_completed', '10330', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10233, 'world', 8, NULL, 'eq', NULL, 0), (10233, 'mission_status', 704, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10233, 'add_dialog_set', 1000016, NULL, '{"slot": 424}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10234, '10330 - Supply officer click: offer, accept, show 10729 (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10234, 'interact_tag', 'Castle_HubSupplyOfficer', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10234, 'world', 8, NULL, 'eq', NULL, 0), (10234, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10234, 'mission_status', 10330, NULL, 'neq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10234, 'remove_dialog_set', 1000016, NULL, '{"slot": 424}', 0, 0), (10234, 'accept_mission', 10330, NULL, '{}', 0, 1), (10234, 'display_dialog', 10729, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10235, '10330 - Castle.Infirmary enter at A: advance to B, bind turn-in (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10235, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10235, 'world', 8, NULL, 'eq', NULL, 0), (10235, 'mission_status', 10330, NULL, 'eq', 'active', 1), (10235, 'step_status', 10330, '10384', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10235, 'advance_step', 10330, '10385', '{}', 0, 0), (10235, 'add_dialog_set', 1000017, NULL, '{"slot": 424}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10236, '10330 - Supply officer click at B: turn in, show 10730, complete (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10236, 'interact_tag', 'Castle_HubSupplyOfficer', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10236, 'world', 8, NULL, 'eq', NULL, 0), (10236, 'mission_status', 10330, NULL, 'eq', 'active', 1), (10236, 'step_status', 10330, '10385', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10236, 'remove_dialog_set', 1000017, NULL, '{"slot": 424}', 0, 0), (10236, 'display_dialog', 10730, NULL, '{}', 0, 1), (10236, 'complete_mission', 10330, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10237, '10330 - Login at B: rebind the turn-in (CIMMERIA_AUTHORING)', 'mission', 10330, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10237, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10237, 'world', 8, NULL, 'eq', NULL, 0), (10237, 'mission_status', 10330, NULL, 'eq', 'active', 1), (10237, 'step_status', 10330, '10385', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10237, 'add_dialog_set', 1000017, NULL, '{"slot": 424}', 0, 0);
