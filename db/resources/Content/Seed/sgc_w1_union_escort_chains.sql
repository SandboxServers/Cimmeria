-- SGC_W1 Union continuation: 1564 escort (packet US-16, chains 3073-3079).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- All six chains are CIMMERIA_RECONSTRUCTION: the step text (4631 "Carter goes
-- with you to the Failsafe room") is the only source; no source gives the path
-- or the stop point. P05 (147.0, 1.30, -197.0) is sealed in
-- docs/analysis/union-sgc-continuation/placement.md (on the mesh, comp 3).
--
-- Escort mechanics, as the Castle precedent (chains 1263, 1291, 1296, 1302):
-- `set_follow_target` {"use_player": true} points the one `SGC_W1_SamCarter`
-- entity at the triggering player; `set_follow_target` {} clears it and drops
-- her to Idle, so it must precede `move_waypoint`, which places her at P05.
-- There is no per-player alternative: Carter is one entity per instance.
--
-- Human classes only: `archetype gte 1` and `archetype lte 4`.

-- Chain 3073: close Carter's radio report (5375) on step 4630 → Carter follows.
-- US-15 chain 3064 advances 4630 → 4631 on the same event; the engine shares
-- one pre-action snapshot, so this chain still sees 4630.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3073, 'SGC_W1 - Carter starts following, 1564 step 4630 → 4631', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3073, 'dialog_choice', '5375', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3073, 'step_status', 1564, '4630', 'eq', 'active', 0),
  (3073, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3073, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3073, 'set_follow_target', NULL, 'SGC_W1_SamCarter', '{"use_player": true}', 0, 0);

-- Chain 3074: enter the Failsafe room on 4631 → she stops and walks to P05.
-- The clear comes first: `set_follow_target` drops her to Idle and clears the
-- nav path, so running it after `move_waypoint` would undo the move.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3074, 'SGC_W1 - FailsafeRoom: Carter stops and walks to P05', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3074, 'enter_region', 'SGC_W1.FailsafeRoom', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3074, 'step_status', 1564, '4631', 'eq', 'active', 0),
  (3074, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3074, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3074, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3074, 'set_follow_target', NULL, 'SGC_W1_SamCarter', '{}', 0, 0),
  (3074, 'move_waypoint', NULL, 'SGC_W1_SamCarter', '{"destination": "147.0,1.30,-197.0"}', 0, 1);

-- Chain 3075: relog on 4631 → Carter follows again and keeps her "!".
-- The `|` mask also satisfies the interact-tag linter for chain 3076's key.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3075, 'SGC_W1 - relog on 4631: Carter follows again', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3075, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3075, 'step_status', 1564, '4631', 'eq', 'active', 0),
  (3075, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3075, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3075, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3075, 'set_follow_target', NULL, 'SGC_W1_SamCarter', '{"use_player": true}', 0, 0),
  (3075, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 1);

-- Chain 3076: press Carter on 4631 → her escort line 5376. It stands aside
-- while 1565, 1566 or 1567 is active (US-20/US-21 show her tactics lines
-- 5379-5381 then), so a press shows one dialog. A never-accepted mission reads
-- as not active.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3076, 'SGC_W1 - Carter escort line 5376 on 4631', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3076, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3076, 'step_status', 1564, '4631', 'eq', 'active', 0),
  (3076, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3076, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3076, 'mission_status', 1565, NULL, 'neq', 'active', 3),
  (3076, 'mission_status', 1566, NULL, 'neq', 'active', 4),
  (3076, 'mission_status', 1567, NULL, 'neq', 'active', 5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3076, 'display_dialog', 5376, NULL, '{}', 0, 0);

-- Chain 3077: relog on 4632 → Carter back at P05 beside the device.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3077, 'SGC_W1 - relog on 4632: Carter at P05', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3077, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3077, 'step_status', 1564, '4632', 'eq', 'active', 0),
  (3077, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3077, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3077, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3077, 'move_entity', NULL, 'SGC_W1_SamCarter',
        '{"destination": "147.0,1.30,-197.0", "world": "SGC_W1"}', 0, 0);

-- Chain 3078: relog on 4633 → Carter back at P05 for her final report.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3078, 'SGC_W1 - relog on 4633: Carter at P05', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3078, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3078, 'step_status', 1564, '4633', 'eq', 'active', 0),
  (3078, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3078, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3078, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3078, 'move_entity', NULL, 'SGC_W1_SamCarter',
        '{"destination": "147.0,1.30,-197.0", "world": "SGC_W1"}', 0, 0);
