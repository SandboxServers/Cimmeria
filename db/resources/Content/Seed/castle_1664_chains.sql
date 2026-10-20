-- Castle mission 1664 "Debriefing" grant (FZ-18): chain 1398 only.
-- Rows frozen in docs/analysis/castle-continuation/evidence/1664.md (RECONSTRUCTION).
-- When 708 completes in Castle (world 8), accept 1664; its first step, 5224
-- "Speak to Sgt. Kaerlia by the Stargate", is picked up in Harset
-- (harset_1664_chains.sql, chains 6008-6016).
-- There is deliberately no chain 1399: O18-02 omits it and forbids any re-grant
-- of 1664 after an abandon, so `mission_completed 708` is the only grant edge.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1398, '1664 - Castle exit: accept 1664 when 708 completes (RECONSTRUCTION)', 'mission', 1664, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1398, 'mission_completed', '708', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1398, 'world', 8, NULL, 'eq', NULL, 0), (1398, 'mission_status', 708, NULL, 'eq', 'completed', 1), (1398, 'mission_status', 1664, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1398, 'accept_mission', 1664, NULL, '{}', 0, 0);
