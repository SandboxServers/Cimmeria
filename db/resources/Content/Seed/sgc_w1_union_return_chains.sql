-- SGC_W1 Union continuation: return state and relogs (packet US-34, chains 3171-3175).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION (D-US5 default, H5): SGC_W1 is a per-player instance rebuilt on every entry; despawn story Jaffa whose missions are spent. Rows 71/77 untouched.

-- CIMMERIA_RECONSTRUCTION: chain 3171 - return with 1568 completed (Human): the Gate Room Jaffa are spent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3171, 'SGC_W1 - return: Gate Room clear after 1568', 'mission', 1568, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3171, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3171, 'archetype', NULL, NULL, 'gte', '1', 0),
  (3171, 'archetype', NULL, NULL, 'lte', '4', 1),
  (3171, 'mission_status', 1568, NULL, 'eq', 'completed', 2),
  (3171, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3171, 'despawn_entity', NULL, 'SGC_W1_GroomJaffa1', '{}', 0, 0),
  (3171, 'despawn_entity', NULL, 'SGC_W1_GroomJaffa2', '{}', 0, 1),
  (3171, 'despawn_entity', NULL, 'SGC_W1_GroomJaffa3', '{}', 0, 2);

-- CIMMERIA_RECONSTRUCTION: chain 3172 - return as a Free Jaffa: no story Jaffa at all (archetype 7 only, never gte 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3172, 'SGC_W1 - return: no story Jaffa for a Free Jaffa', 'mission', 1568, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3172, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3172, 'archetype', NULL, NULL, 'eq', '7', 0),
  (3172, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3172, 'despawn_entity', NULL, 'SGC_W1_TacticsJaffa1', '{}', 0, 0),
  (3172, 'despawn_entity', NULL, 'SGC_W1_TacticsJaffa2', '{}', 0, 1),
  (3172, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa1', '{}', 0, 2),
  (3172, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa2', '{}', 0, 3),
  (3172, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa3', '{}', 0, 4),
  (3172, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa4', '{}', 0, 5),
  (3172, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa5', '{}', 0, 6),
  (3172, 'despawn_entity', NULL, 'SGC_W1_GroomJaffa1', '{}', 0, 7),
  (3172, 'despawn_entity', NULL, 'SGC_W1_GroomJaffa2', '{}', 0, 8),
  (3172, 'despawn_entity', NULL, 'SGC_W1_GroomJaffa3', '{}', 0, 9);

-- CIMMERIA_RECONSTRUCTION: chain 3173 - return with 1565 completed (Human): the Tactics pair is spent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3173, 'SGC_W1 - return: Tactics pair spent after 1565', 'mission', 1568, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3173, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3173, 'archetype', NULL, NULL, 'gte', '1', 0),
  (3173, 'archetype', NULL, NULL, 'lte', '4', 1),
  (3173, 'mission_status', 1565, NULL, 'eq', 'completed', 2),
  (3173, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3173, 'despawn_entity', NULL, 'SGC_W1_TacticsJaffa1', '{}', 0, 0),
  (3173, 'despawn_entity', NULL, 'SGC_W1_TacticsJaffa2', '{}', 0, 1);

-- CIMMERIA_RECONSTRUCTION: chain 3174 - return after 1564 with no 1565-1567 still owed: the route pair and the Clear group are spent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3174, 'SGC_W1 - return: route spent after 1564', 'mission', 1568, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3174, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3174, 'archetype', NULL, NULL, 'gte', '1', 0),
  (3174, 'archetype', NULL, NULL, 'lte', '4', 1),
  (3174, 'mission_status', 1564, NULL, 'eq', 'completed', 2),
  (3174, 'mission_status', 1565, NULL, 'neq', 'active', 3),
  (3174, 'mission_status', 1566, NULL, 'neq', 'active', 4),
  (3174, 'mission_status', 1567, NULL, 'neq', 'active', 5),
  (3174, 'world', 58, NULL, 'eq', NULL, 6);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3174, 'despawn_entity', NULL, 'SGC_W1_TacticsJaffa1', '{}', 0, 0),
  (3174, 'despawn_entity', NULL, 'SGC_W1_TacticsJaffa2', '{}', 0, 1),
  (3174, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa1', '{}', 0, 2),
  (3174, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa2', '{}', 0, 3),
  (3174, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa3', '{}', 0, 4),
  (3174, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa4', '{}', 0, 5),
  (3174, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa5', '{}', 0, 6);

-- CIMMERIA_RECONSTRUCTION: chain 3175 - return with 1567 completed (Human): the Clear group is spent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3175, 'SGC_W1 - return: Clear group spent after 1567', 'mission', 1568, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3175, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3175, 'archetype', NULL, NULL, 'gte', '1', 0),
  (3175, 'archetype', NULL, NULL, 'lte', '4', 1),
  (3175, 'mission_status', 1567, NULL, 'eq', 'completed', 2),
  (3175, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3175, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa1', '{}', 0, 0),
  (3175, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa2', '{}', 0, 1),
  (3175, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa3', '{}', 0, 2),
  (3175, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa4', '{}', 0, 3),
  (3175, 'despawn_entity', NULL, 'SGC_W1_ClearJaffa5', '{}', 0, 4);
