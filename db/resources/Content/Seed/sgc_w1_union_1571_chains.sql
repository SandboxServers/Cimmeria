-- SGC_W1 Union continuation: 1571 auxiliary credit (packet US-13, chains 3080-3084).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 1571 - Secure a Firearm: auxiliary credit (US-13, chains 3080-3081)
-- ============================================================
-- Mission 1571 is the auxiliary record of the armory pickup in 1559. Its one
-- required objective is 5354 (5355 and 5356 are hidden optional objectives
-- and stay uncredited). A Human who completes 1559 is credited 1571 at once;
-- a Human who finished 1559 before this chain existed is credited on load.
-- No `add_item`: the weapon was already granted in 1559.

-- Chain 3080: mission_completed 1559 (Human): accept 1571 and complete 5354
-- CIMMERIA_RECONSTRUCTION (H2): auto-accept and credit on the 1559 completion.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3080, 'SGC_W1 - 1559 done: credit 1571 (Human)', 'mission', 1571, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3080, 'mission_completed', '1559', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3080, 'mission_status', 1571, NULL, 'eq', 'not_active', 0),
  (3080, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3080, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3080, 'accept_mission', 1571, NULL, '{}', 0, 0),
  (3080, 'complete_objective', 1571, '5354', '{}', 0, 1);

-- Chain 3081: player_loaded in SGC_W1 (Human, 1559 done): catch-up credit 1571
-- CIMMERIA_RECONSTRUCTION (H2): a character who finished 1559 before chain 3080.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3081, 'SGC_W1 - 1571 catch-up on load (Human)', 'mission', 1571, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3081, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3081, 'mission_status', 1559, NULL, 'eq', 'completed', 0),
  (3081, 'mission_status', 1571, NULL, 'eq', 'not_active', 1),
  (3081, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3081, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3081, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3081, 'accept_mission', 1571, NULL, '{}', 0, 0),
  (3081, 'complete_objective', 1571, '5354', '{}', 0, 1);

