-- SGC_W1 Union continuation: 1565 and 1566 Tactics (packet US-20, chains 3085-3094).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION: step text 4634/4635; optional flank O5375 left uncredited (D-US8).
--
-- Mission 1565 (step 4634, required objective 5374, optional 5375) is Carter's
-- fight on the way to the Failsafe room (step 4631). The static pair is
-- SGC_W1_TacticsJaffa1/2 (spawnlist rows 57/58, template 34). Mission 1566
-- (step 4635, objective 5376) follows when 1565 completes: its pair
-- SGC_W1_Tactics2Jaffa1/2 is spawned by chain 3088 at the sealed placement
-- points P11/P12 (placement.md), 4 m apart on the Failsafe-side corridor.
--
-- Human classes only (`archetype gte 1` and `lte 4`). Everyone else never
-- gets 1565 (test: live_db_m1565_never_offered_to_a_non_human).
--
-- Chain 3085 is the offer: Carter's radio report (dialog 5375) on step 4630.
-- The close of 5375 advances 1564 to 4631 in chain 3064 (US-15); this chain
-- reads 4630 because all chains one event matches share one pre-action
-- snapshot.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3085, 'SGC_W1 - Dialog 5375 (Human): offer 1565 with the radio report', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3085, 'dialog_choice', '5375', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3085, 'step_status',   1564, '4630', 'eq',  'active',     0),
  (3085, 'mission_status', 1565, NULL,  'eq',  'not_active', 1),
  (3085, 'archetype',     NULL, NULL,   'gte', '1',          2),
  (3085, 'archetype',     NULL, NULL,   'lte', '4',          3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3085, 'accept_mission', 1565, NULL, '{}', 0, 0);

-- Chain 3086: the first pass of the 1565 pair. One `entity_dead_tag` row per
-- tag (precedent: chain 1350). Both tags must be dead when either fires, so
-- the first kill finds the other still alive and does nothing (the `entity_tag_state`
-- rows are the guard, and the trigger rows OR together).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3086, 'SGC_W1 - 1565 pair dead: complete objective 5374', 'mission', 1565, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3086, 'entity_dead_tag', 'SGC_W1_TacticsJaffa1', 'space', false, 0),
  (3086, 'entity_dead_tag', 'SGC_W1_TacticsJaffa2', 'space', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3086, 'step_status',      1565, '4634',                   'eq', 'active', 0),
  (3086, 'entity_tag_state', NULL, 'SGC_W1_TacticsJaffa1',   'eq', 'dead',   1),
  (3086, 'entity_tag_state', NULL, 'SGC_W1_TacticsJaffa2',   'eq', 'dead',   2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3086, 'complete_objective', 1565, '5374', '{}', 0, 0);

-- Chain 3087: backstop for a pair already dead when 1565 is accepted (the
-- player killed them on an earlier attempt, or before the offer). It fires
-- on `mission_accepted` and runs the same three conditions as 3086.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3087, 'SGC_W1 - 1565 accepted with the pair already dead: complete objective 5374', 'mission', 1565, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3087, 'mission_accepted', '1565', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3087, 'step_status',      1565, '4634',                 'eq', 'active', 0),
  (3087, 'entity_tag_state', NULL, 'SGC_W1_TacticsJaffa1', 'eq', 'dead',   1),
  (3087, 'entity_tag_state', NULL, 'SGC_W1_TacticsJaffa2', 'eq', 'dead',   2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3087, 'complete_objective', 1565, '5374', '{}', 0, 0);

-- Chain 3088: 1565 completed → offer 1566 and spawn its pair. The same two
-- spawn rows are repeated in 3092 for a relog; the spawn executor is
-- idempotent by tag. 1566 is accepted only from this completion, so 1565
-- and 1566 are never both active.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3088, 'SGC_W1 - 1565 done: offer 1566, spawn the second Jaffa pair', 'mission', 1565, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3088, 'mission_completed', '1565', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3088, 'mission_status', 1566, NULL, 'eq',  'not_active', 0),
  (3088, 'archetype',      NULL, NULL, 'gte', '1',         1),
  (3088, 'archetype',      NULL, NULL, 'lte', '4',         2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3088, 'accept_mission', 1566, NULL, '{}', 0, 0),
  (3088, 'spawn_entity', 34, 'SGC_W1_Tactics2Jaffa1', '{"x": 120.0, "y": 1.18, "z": -180.0, "heading": 0}', 0, 1),
  (3088, 'spawn_entity', 34, 'SGC_W1_Tactics2Jaffa2', '{"x": 116.0, "y": 1.18, "z": -176.0, "heading": 0}', 0, 2);

-- Chain 3089: the second pair. Same shape as 3086, for the 1566 tags and objective 5376.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3089, 'SGC_W1 - 1566 pair dead: complete objective 5376', 'mission', 1566, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3089, 'entity_dead_tag', 'SGC_W1_Tactics2Jaffa1', 'space', false, 0),
  (3089, 'entity_dead_tag', 'SGC_W1_Tactics2Jaffa2', 'space', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3089, 'step_status',      1566, '4635',                    'eq', 'active', 0),
  (3089, 'entity_tag_state', NULL, 'SGC_W1_Tactics2Jaffa1',   'eq', 'dead',   1),
  (3089, 'entity_tag_state', NULL, 'SGC_W1_Tactics2Jaffa2',   'eq', 'dead',   2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3089, 'complete_objective', 1566, '5376', '{}', 0, 0);

-- Chain 3090: Carter's line 5379, pressed while 1565 is active on step 4631.
-- It names its own mission (`mission_status 1565 active`), so it stays quiet
-- once 1565 completes, and 3091 does not share its gate.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3090, 'SGC_W1 - Carter press during 1565 (Human): line 5379', 'mission', 1565, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3090, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3090, 'mission_status', 1565, NULL, 'eq',  'active', 0),
  (3090, 'step_status',    1564, '4631', 'eq', 'active', 1),
  (3090, 'archetype',      NULL, NULL,  'gte', '1',      2),
  (3090, 'archetype',      NULL, NULL,  'lte', '4',      3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3090, 'display_dialog', 5379, NULL, '{}', 0, 0);

-- Chain 3091: Carter's line 5380, pressed while 1566 is active on step 4631.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3091, 'SGC_W1 - Carter press during 1566 (Human): line 5380', 'mission', 1566, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3091, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3091, 'mission_status', 1566, NULL, 'eq',  'active', 0),
  (3091, 'step_status',    1564, '4631', 'eq', 'active', 1),
  (3091, 'archetype',      NULL, NULL,  'gte', '1',      2),
  (3091, 'archetype',      NULL, NULL,  'lte', '4',      3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3091, 'display_dialog', 5380, NULL, '{}', 0, 0);

-- Chain 3092: relog during 1566 (step 4635). A fresh instance has no
-- second pair, so the load respawns it. `tag_already_live` is the expected
-- WARN when the pair is still in the same space (the idempotence check is
-- `find_entity_by_tag`, which counts a corpse still in `space.entities`;
-- that is a US-34 finding, not handled here).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3092, 'SGC_W1 - Load active 1566 step 4635 (Human): respawn the second Jaffa pair', 'mission', 1566, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3092, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3092, 'step_status', 1566, '4635', 'eq',  'active', 0),
  (3092, 'archetype',   NULL, NULL,  'gte', '1',      1),
  (3092, 'archetype',   NULL, NULL,  'lte', '4',      2),
  (3092, 'world',       58,   NULL,  'eq',  NULL,     3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3092, 'spawn_entity', 34, 'SGC_W1_Tactics2Jaffa1', '{"x": 120.0, "y": 1.18, "z": -180.0, "heading": 0}', 0, 0),
  (3092, 'spawn_entity', 34, 'SGC_W1_Tactics2Jaffa2', '{"x": 116.0, "y": 1.18, "z": -176.0, "heading": 0}', 0, 1);

-- Chain 3093: relog on step 4631 (Human): a relog starts from a fresh
-- instance, so Carter's "!" marker is set again on load (as chain 3031 does).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3093, 'SGC_W1 - Load active 1564 step 4631 (Human): restore Col. Carter marker', 'mission', 1564, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3093, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3093, 'step_status', 1564, '4631', 'eq',  'active', 0),
  (3093, 'archetype',   NULL, NULL,  'gte', '1',      1),
  (3093, 'archetype',   NULL, NULL,  'lte', '4',      2),
  (3093, 'world',       58,   NULL,  'eq',  NULL,     3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3093, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 0);
