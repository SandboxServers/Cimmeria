-- Castle return-hub barks (CS-05S2b): chains 10200-10201. CIMMERIA_AUTHORING.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10200, 'Castle hub bark: Ogilvie after 1520 (CIMMERIA_AUTHORING)', 'mission', 1520, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10200, 'enter_region', 'Castle.Infirmary', 'player', true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10200, 'world', 8, NULL, 'eq', NULL, 0), (10200, 'mission_status', 1520, NULL, 'eq', 'completed', 1), (10200, 'archetype', NULL, NULL, 'lte', '6', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10200, 'npc_bark', NULL, NULL, '{"screen_id": 10900, "speaker": "Ogilvie", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10201, 'Castle hub bark: Castle Medic after 1521 (CIMMERIA_AUTHORING)', 'mission', 1521, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10201, 'enter_region', 'Castle.Infirmary', 'player', true, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10201, 'world', 8, NULL, 'eq', NULL, 0), (10201, 'mission_status', 1521, NULL, 'eq', 'completed', 1), (10201, 'mission_status', 1520, NULL, 'eq', 'not_active', 2), (10201, 'archetype', NULL, NULL, 'eq', '8', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10201, 'npc_bark', NULL, NULL, '{"screen_id": 10901, "speaker": "Castle Medic", "channel": "say"}', 0, 0);
