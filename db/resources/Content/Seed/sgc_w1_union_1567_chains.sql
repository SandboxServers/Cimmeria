-- SGC_W1 Union continuation: 1567 Clear the Way (packet US-21, chains 3095-3099).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: step text 4636; the five route Jaffa rows 59-63 already on the map.
-- Chain 3095: 1566 completes, so the server offers 1567 to Humans.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3095, 'SGC_W1 - 1566 done: offer 1567 (Human)', 'mission', 1567, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3095, 'mission_completed', '1566', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3095, 'mission_status', 1567, NULL, 'eq', 'not_active', 0),
  (3095, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3095, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3095, 'accept_mission', 1567, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: credit 5377 when the fifth route Jaffa dies while 1567 is on step 4636.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3096, 'SGC_W1 - 1567 all five route Jaffa dead: complete 5377', 'mission', 1567, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3096, 'entity_dead_tag', 'SGC_W1_ClearJaffa1', 'space', false, 0),
  (3096, 'entity_dead_tag', 'SGC_W1_ClearJaffa2', 'space', false, 1),
  (3096, 'entity_dead_tag', 'SGC_W1_ClearJaffa3', 'space', false, 2),
  (3096, 'entity_dead_tag', 'SGC_W1_ClearJaffa4', 'space', false, 3),
  (3096, 'entity_dead_tag', 'SGC_W1_ClearJaffa5', 'space', false, 4);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3096, 'step_status', 1567, '4636', 'eq', 'active', 0),
  (3096, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa1', 'eq', 'dead', 1),
  (3096, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa2', 'eq', 'dead', 2),
  (3096, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa3', 'eq', 'dead', 3),
  (3096, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa4', 'eq', 'dead', 4),
  (3096, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa5', 'eq', 'dead', 5),
  (3096, 'archetype', NULL, NULL, 'gte', '1', 6),
  (3096, 'archetype', NULL, NULL, 'lte', '4', 7);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3096, 'complete_objective', 1567, '5377', '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: backstop for a 1567 accepted when the five Jaffa are already dead.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3097, 'SGC_W1 - 1567 accepted with the five route Jaffa already dead: complete 5377', 'mission', 1567, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3097, 'mission_accepted', '1567', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3097, 'step_status', 1567, '4636', 'eq', 'active', 0),
  (3097, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa1', 'eq', 'dead', 1),
  (3097, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa2', 'eq', 'dead', 2),
  (3097, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa3', 'eq', 'dead', 3),
  (3097, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa4', 'eq', 'dead', 4),
  (3097, 'entity_tag_state', NULL, 'SGC_W1_ClearJaffa5', 'eq', 'dead', 5),
  (3097, 'archetype', NULL, NULL, 'gte', '1', 6),
  (3097, 'archetype', NULL, NULL, 'lte', '4', 7);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3097, 'complete_objective', 1567, '5377', '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: Carter's line 5381 when pressed during 1567 (Human, 1564 on step 4631).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3098, 'SGC_W1 - Interact Carter during 1567 (Human): line 5381', 'mission', 1567, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3098, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3098, 'mission_status', 1567, NULL, 'eq', 'active', 0),
  (3098, 'step_status', 1564, '4631', 'eq', 'active', 1),
  (3098, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3098, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3098, 'display_dialog', 5381, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: re-login during 1567 restores Carter's story marker. Also satisfies
-- the interact-tag linter for 3098 (F9).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3099, 'SGC_W1 - Load during 1567 (Human): restore Col. Carter marker', 'mission', 1567, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3099, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3099, 'mission_status', 1567, NULL, 'eq', 'active', 0),
  (3099, 'step_status', 1564, '4631', 'eq', 'active', 1),
  (3099, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3099, 'archetype', NULL, NULL, 'lte', '4', 3),
  (3099, 'world', 58, NULL, 'eq', NULL, 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3099, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 0);
