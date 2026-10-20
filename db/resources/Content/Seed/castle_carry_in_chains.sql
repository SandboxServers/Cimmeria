-- Castle carry-ins: owner-approved CIMMERIA_RECONSTRUCTION, CS-08a.
-- Crossing credits the route before destination hydration; Harset owns receipt.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1366, '1360 - Castle exit route: advance Frost''s Letter to 4038 (CIMMERIA_RECONSTRUCTION)', 'mission', 1360, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1366, 'stargate_crossed', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1366, 'world', 8, NULL, 'eq', NULL, 0),
  (1366, 'mission_status', 1360, NULL, 'eq', 'active', 1),
  (1366, 'step_status', 1360, '4037', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1366, 'advance_step', 1360, '4038', '{}', 0, 0);
