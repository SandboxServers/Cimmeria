-- Phase 3: SGC_W1 content chains
-- Mission 1559 (principal mission in this space, "Orientation")
-- Exercises: complete_objective, objective_status, set_visible, move_entity,
--            mission_completed, dialog_set_open
--
-- Chain ID range: 3001-3050
--
-- Class Start v6 (CS-05, PROJECT_FINAL_S2C; ledger:
-- docs/analysis/class-start-v6/README.md):
--   * 3001, 3017, 3018 carry `archetype neq 7` (OD-CS09): a visiting Free
--     Jaffa (ARCHETYPE_Sholva) is not pulled into the Human tutorial or M1561.
--   * 3008 is the Human pistol pickup (core grant + tutorial 5882); 3029 is
--     the same pickup for everyone else, unchanged (Asgard holding state).
--   * 3030-3037 finish M1562 for the Human classes (Carter, then the SMG
--     on her desk, item 21).
--   * 3041-3044 deliver the M1569 class rewards and signatures on
--     `mission_completed 1569`. M1569's own route is not authored (OD-CS10).

SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 1559 — Orientation (SGC_W1 primary mission)
-- ============================================================

-- Chain 3001: player_loaded when 1559 not active → accept + show Gen Hammond.
-- Not for a Free Jaffa (`archetype neq 7`, OD-CS09): they start on Dakara and
-- only visit SGC_W1. A player with no archetype reads -1 and still gets it.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3001, 'SGC_W1 - Load: accept mission 1559, show Gen Hammond', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3001, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3001, 'mission_status', 1559, NULL, 'eq', 'not_active', 0),
  (3001, 'archetype', NULL, NULL, 'neq', '7', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3001, 'accept_mission', 1559, NULL, '{}', 0, 0),
  (3001, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "|", "mask": 16777216}', 0, 1);

-- Chain 3002: interact SGCW1_GenHammond while step 4612 active → show dialog 5354
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3002, 'SGC_W1 - Interact Hammond: show dialog 5354', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3002, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3002, 'step_status', 1559, '4612', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3002, 'display_dialog', 5354, NULL, '{}', 0, 0);

-- Chain 3003: dialog choice 5354 (accept Gen Hammond briefing) → advance to 4613, hide Hammond, show Teal'c
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3003, 'SGC_W1 - Dialog 5354: advance step, swap NPCs', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3003, 'dialog_choice', '5354', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3003, 'advance_step', 1559, '4613', '{}', 0, 0),
  (3003, 'play_sequence', 10005, NULL, '{}', 0, 1),
  (3003, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 16777216}', 0, 2),
  (3003, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "|", "mask": 16777216}', 0, 3);

-- Chain 3004: interact Teal'c while step 4613 active → show dialog 5355
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3004, 'SGC_W1 - Interact Tealc: show dialog 5355', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3004, 'interact_tag', 'SGC_W1_Tealc', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3004, 'step_status', 1559, '4613', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3004, 'display_dialog', 5355, NULL, '{}', 0, 0);

-- Chain 3005: dialog choice 5355 (agree with Teal'c) → advance 4614, play sequence, hide Teal'c
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3005, 'SGC_W1 - Dialog 5355: advance 4614, play sequences', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3005, 'dialog_choice', '5355', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3005, 'advance_step', 1559, '4614', '{}', 0, 0),
  (3005, 'play_sequence', 2319, NULL, '{}', 0, 1),
  (3005, 'play_sequence', 10002, NULL, '{}', 500, 2),
  (3005, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "~", "mask": 16777216}', 0, 3),
  -- Move Gen Hammond to waypoint alongside player (set_visible / move_entity exercises)
  (3005, 'move_entity', NULL, 'SGCW1_GenHammond',
   '{"destination": "-123.625,1.311,-246.858", "world": "SGC_W1"}', 0, 4);

-- Chain 3006: interact elevator button while step 4614 active → complete objective 5353, show dialog 5357
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3006, 'SGC_W1 - Press elevator button: complete objective 5353', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3006, 'interact_tag', 'SGC_W1_ElevatorButton1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3006, 'step_status', 1559, '4614', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3006, 'complete_objective', 1559, '5353', '{}', 0, 0),
  (3006, 'display_dialog', 5357, NULL, '{}', 0, 1);

-- Chain 3007: dialog choice 5357 (move to armory) → move player, complete objective 5358, give item 55
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3007, 'SGC_W1 - Move to armory: complete objective 5358, give item', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3007, 'dialog_choice', '5357', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3007, 'move_entity', NULL, NULL,
   '{"destination": "-123.625,1.311,-246.858", "world": "SGC_W1", "use_player": true}', 0, 0),
  (3007, 'complete_objective', 1559, '5358', '{}', 0, 1),
  (3007, 'set_interaction_type', NULL, 'SGC_W1_FirearmBody',
   '{"op": "|", "mask": 16777216}', 0, 2),
  (3007, 'play_sequence', 10013, NULL, '{}', 0, 3);

-- Chain 3008: a Human class (Soldier 1, Commando 2, Scientist 3,
-- Archaeologist 4) picks up the pistol while 1559 is active → complete 1559,
-- pistol 55, the CORE_TUTORIAL abilities, the corpse line, then the one-time
-- tutorial 5882 "Equipping a Weapon" (OD-CS04, L1). The abilities are sent
-- before the tutorial that tells the player to place Pistol Shot; the base
-- handles cell messages in order. 5883 "Combat" follows from chain 7101
-- (tutorial_chains.sql) on the first hostile combat after 5882.
-- `archetype lt 5` also admits a player with no archetype (-1), who gets the
-- Human branch rather than a dead body that does nothing. The pistol arrives
-- with 0 rounds (OD-CS13).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3008, 'SGC_W1 - Pick up firearm (Human): complete 1559, pistol, core abilities, tutorial 5882', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3008, 'interact_tag', 'SGC_W1_FirearmBody', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3008, 'mission_status', 1559, NULL, 'eq', 'active', 0),
  (3008, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3008, 'complete_mission', 1559, NULL, '{}', 0, 0),
  (3008, 'add_item', 55, NULL, '{"container": 1, "qty": 1}', 0, 1),
  (3008, 'grant_ability', NULL, NULL,
   '{"ability_ids": [592, 594, 597, 1218], "source_kind": "tutorial", "source_id": 1559}', 0, 2),
  (3008, 'display_dialog', 5358, NULL, '{}', 0, 3),
  (3008, 'show_tutorial', NULL, NULL, '{"tutorial_id": 5882}', 0, 4);

-- Chain 3029: the same pickup for every other archetype, exactly as chain
-- 3008 was before CS-05: complete 1559, pistol 55, the corpse line. No
-- ability grant and no tutorial. This is the Asgard holding state (OD-CS09,
-- NON_CANONICAL_BLOCKED_LEGACY): char_def 9 keeps the universal kit and
-- today's M1559. Remove it with the holding state when the Asgard start
-- exists (blockers B1-B3).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3029, 'SGC_W1 - Pick up firearm (non-Human, Asgard holding state): complete mission 1559', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3029, 'interact_tag', 'SGC_W1_FirearmBody', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3029, 'mission_status', 1559, NULL, 'eq', 'active', 0),
  (3029, 'archetype', NULL, NULL, 'gte', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3029, 'complete_mission', 1559, NULL, '{}', 0, 0),
  (3029, 'add_item', 55, NULL, '{"container": 1, "qty": 1}', 0, 1),
  (3029, 'display_dialog', 5358, NULL, '{}', 0, 2);

-- Chain 3009: mission_completed event for 1559 → play cinematic, move airman NPC
--   Exercises: mission_completed event type
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3009, 'SGC_W1 - Mission 1559 completed: play outro', 'mission', 1559, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3009, 'mission_completed', '1559', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3009, 'play_sequence', 10012, NULL, '{}', 0, 0),
  (3009, 'move_entity', NULL, 'SGCW1_AirmanWalking',
   '{"destination": "237.326,1.312,17.921", "world": "SGC_W1"}', 0, 1),
  (3009, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton1',
   '{"op": "|", "mask": 256}', 0, 2);

-- Chain 3010: dialog choice 5358 (post-mission chat) → play sequence
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3010, 'SGC_W1 - Dialog 5358 choice: play outro sequence', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3010, 'dialog_choice', '5358', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3010, 'play_sequence', 10012, NULL, '{}', 0, 0);

-- Chain 3011: dialog choice 5356 (security office interaction) → move airman
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3011, 'SGC_W1 - Dialog 5356: move airman to waypoint', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3011, 'dialog_choice', '5356', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3011, 'move_entity', NULL, 'SGCW1_AirmanWalking',
   '{"destination": "237.326,1.312,17.921", "world": "SGC_W1"}', 0, 0),
  (3011, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton1',
   '{"op": "|", "mask": 256}', 0, 1);

-- Chain 3012: show Hammond waypoint dialog only during step 4614 and while
--   objective 5353 is still incomplete. The original script fires this from a
--   waypoint callback after Teal'c advances the mission, so we gate it to the
--   same step here.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3012, 'SGC_W1 - Hammond waypoint dialog (obj 5353 incomplete only)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3012, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3012, 'step_status', 1559, '4614', 'eq', 'active', 0),
  (3012, 'objective_status', 1559, '5353', 'neq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3012, 'display_dialog', 5356, NULL, '{}', 0, 0);

-- Chain 3013: dialog_set_open event (SecurityOffice dialog set)
--   Exercises: dialog_set_open event type
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3013, 'SGC_W1 - Security office dialog set open: set_visible on door', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3013, 'dialog_set_open', 'SGC_W1_SecurityOffice', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3013, 'set_visible', NULL, 'SGC_W1_SecurityDoor', '{"visible": true}', 0, 0);

-- Resume/re-login hydration: player_loaded only accepts mission 1559 when it is
-- not active, so an in-progress tutorial needs its interaction flags restored
-- from the saved step state when the player logs back in.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3014, 'SGC_W1 - Load active 4612: restore Hammond interaction', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3014, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3014, 'step_status', 1559, '4612', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3014, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
        '{"op": "|", "mask": 16777216}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3015, 'SGC_W1 - Load active 4613: restore Tealc interaction', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3015, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3015, 'step_status', 1559, '4613', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3015, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3015, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "|", "mask": 16777216}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3016, 'SGC_W1 - Load active 4614: restore elevator and firearm interactions', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3016, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3016, 'step_status', 1559, '4614', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3016, 'set_interaction_type', NULL, 'SGC_W1_Tealc',
   '{"op": "~", "mask": 16777216}', 0, 0),
  (3016, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton1',
   '{"op": "|", "mask": 256}', 0, 1),
  (3016, 'set_interaction_type', NULL, 'SGC_W1_FirearmBody',
   '{"op": "|", "mask": 16777216}', 0, 2);

-- ============================================================
-- MISSIONS 1561/1562 - Security Office continuation
-- ============================================================

-- Chain 3017: killing the bomb-carrying Jaffa opens Hammond radio dialog 5359.
-- Not for a Free Jaffa (`archetype neq 7`, OD-CS09), so a visitor's kill does
-- not offer M1561. The archetype is the killer's.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3017, 'SGC_W1 - JaffaBomb death: show dialog 5359', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3017, 'entity_dead_tag', 'SGC_W1_JaffaBomb', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3017, 'archetype', NULL, NULL, 'neq', '7', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3017, 'display_dialog', 5359, NULL, '{}', 0, 0);

-- Chain 3018: accepting Hammond's radio prompt starts mission 1561.
-- Gated `archetype neq 7` as well (OD-CS09): 3017 alone would leave the
-- accept open to a Free Jaffa who was shown 5359 some other way.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3018, 'SGC_W1 - Dialog 5359: accept mission 1561', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3018, 'dialog_choice', '5359', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3018, 'mission_status', 1561, NULL, 'eq', 'not_active', 0),
  (3018, 'archetype', NULL, NULL, 'neq', '7', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3018, 'accept_mission', 1561, NULL, '{}', 0, 0),
  (3018, 'set_interaction_type', NULL, 'SGCW1_AirmanBody',
   '{"op": "|", "mask": 16777216}', 0, 1);

-- Chain 3019: re-login while mission 1561 step 4620 is active restores the
-- Airman corpse interaction.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3019, 'SGC_W1 - Load active 4620: restore AirmanBody interaction', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3019, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3019, 'step_status', 1561, '4620', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3019, 'set_interaction_type', NULL, 'SGCW1_AirmanBody',
        '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3020: searching the Airman corpse gives the radio and advances to step 4621.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3020, 'SGC_W1 - Interact AirmanBody: recover radio', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3020, 'interact_tag', 'SGCW1_AirmanBody', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3020, 'step_status', 1561, '4620', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3020, 'display_dialog', 5362, NULL, '{}', 0, 0),
  (3020, 'advance_step', 1561, '4621', '{}', 0, 1),
  (3020, 'add_item', 5168, NULL, '{"container": 2, "qty": 1}', 0, 2);

-- Chain 3021: using the radio at step 4621 advances the bomb-defusal objective.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3021, 'SGC_W1 - Use radio step 4621: advance to bomb defusal', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3021, 'item_use', '5168', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3021, 'step_status', 1561, '4621', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3021, 'display_dialog', 5363, NULL, '{}', 0, 0),
  (3021, 'advance_step', 1561, '4622', '{}', 0, 1),
  (3021, 'set_interaction_type', NULL, 'SGC_W1_NaqBomb',
   '{"op": "|", "mask": 256}', 0, 2);

-- Chain 3022: re-login during step 4622 restores the Naquadah bomb interaction.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3022, 'SGC_W1 - Load active 4622: restore NaqBomb interaction', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3022, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3022, 'step_status', 1561, '4622', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3022, 'set_interaction_type', NULL, 'SGC_W1_NaqBomb',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 3023: interacting with the bomb starts Livewire. Victory directly invokes
-- chain 3024, matching SecurityOffice.py's minigame victory callback.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3023, 'SGC_W1 - Interact NaqBomb: start Livewire', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3023, 'interact_tag', 'SGC_W1_NaqBomb', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3023, 'step_status', 1561, '4622', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3023, 'start_minigame', NULL, 'Livewire',
        '{"on_victory_chains": [3024]}', 0, 0);

-- Triggerless direct chain used by the Livewire victory callback.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3024, 'SGC_W1 - Livewire victory: advance mission 1561 to 4623', 'mission', 1561, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3024, 'advance_step', 1561, '4623', '{}', 0, 0);

-- Chain 3025: using the radio after defusing the bomb completes mission 1561.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3025, 'SGC_W1 - Use radio step 4623: complete mission 1561', 'mission', 1561, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3025, 'item_use', '5168', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3025, 'step_status', 1561, '4623', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3025, 'complete_mission', 1561, NULL, '{}', 0, 0),
  (3025, 'display_dialog', 5365, NULL, '{}', 0, 1);

-- Chain 3026: dialog 5365 starts mission 1562 and enables the next elevator.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3026, 'SGC_W1 - Dialog 5365: accept mission 1562', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3026, 'dialog_choice', '5365', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3026, 'mission_status', 1562, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3026, 'accept_mission', 1562, NULL, '{}', 0, 0),
  (3026, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton2',
   '{"op": "|", "mask": 256}', 0, 1);

-- Chain 3027: re-login while mission 1562 step 4624 is active restores ElevatorButton2.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3027, 'SGC_W1 - Load active 4624: restore ElevatorButton2 interaction', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3027, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3027, 'step_status', 1562, '4624', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3027, 'set_interaction_type', NULL, 'SGC_W1_ElevatorButton2',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 3028: second elevator moves the player toward Carter's lab and advances
-- mission 1562 to its scripted dead-end step.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3028, 'SGC_W1 - Interact ElevatorButton2: go to Carter lab', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3028, 'interact_tag', 'SGC_W1_ElevatorButton2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3028, 'step_status', 1562, '4624', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3028, 'advance_step', 1562, '4625', '{}', 0, 0),
  (3028, 'move_entity', NULL, NULL,
   '{"destination": "-53.86,1.311,62.136", "world": "SGC_W1", "use_player": true}', 0, 1),
  (3028, 'display_dialog', 5366, NULL, '{}', 0, 2),
  (3028, 'play_sequence', 10009, NULL, '{}', 0, 3);

-- ============================================================
-- MISSION 1562 - Col. Carter and the SGHC 6 (Class Start v6, CS-05)
-- ============================================================
-- PROJECT_FINAL_S2C, a reconstruction: the legacy script stopped at step
-- 4625. What the seed still holds is Carter's spawn (spawnlist 56,
-- `SGC_W1_SamCarter`), her line "There's another weapon on my lab desk"
-- (dialog 5367), the pickup line "You take the submachinegun from Carter's
-- desk." (dialog 5368), steps 4626 / 4627, and the two Jaffa of dialog 5366
-- (spawnlist 71 and 77, untagged, in the corridor outside the lab).
--
-- The desk is read from the cooked map: spawn 81 `SGC_W1_CarterDeskSMG`
-- (template 411) lies on the centre desk of the lab's west wall; the
-- evidence is on that spawn row. Carter's own heading (pi/2, facing +X) is
-- right as seeded: she stands inside her U-shaped lab bench and faces across
-- it to the lab's only doorway, 8 m east of her.
--
-- The lab doors: the map places them open, and chain 3028's
-- `play_sequence 10009` is the Kismet event "Designer 0: Close Doors" of
-- `CartersLabDoors` (event 6000; 10010 / 6001 is "Designer 1: Open Doors").
-- The legacy script closed them on arrival and never opened them, so the
-- lab was sealed. Chain 3030 opens them again for a Human. Opening them on
-- the two Jaffa dying, which is what dialogs 5366 and 5367 suggest, is not
-- authored: objective 5365 (the kill) is optional in the seed, and the fight
-- is an owner decision. The lab-entry region (objective 6036) is not
-- authored either.
--
-- Human classes only (`archetype lt 5`, which also admits a player with no
-- archetype): for everyone else M1562 still stops at step 4625 with the doors
-- closed, Carter unmarked and the SMG unlit and unpressable, as before
-- (Asgard holding state, OD-CS09). SGC_W1 is instanced, so the marker one
-- player sets is never seen by another.

-- Chain 3030: the same ElevatorButton2 press as chain 3028, for a Human →
-- mark Carter as the active story contact, and re-open the lab doors that
-- chain 3028 has just closed. The close is a 2.0 s Matinee track (two keys,
-- 4 m down), so the open is sent 3 s later, after it has finished. A relog
-- needs no door action: the map loads with the doors open and chain 3028
-- only fires on step 4624.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3030, 'SGC_W1 - Interact ElevatorButton2 (Human): mark Col. Carter, re-open her lab doors', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3030, 'interact_tag', 'SGC_W1_ElevatorButton2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3030, 'step_status', 1562, '4624', 'eq', 'active', 0),
  (3030, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3030, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
   '{"op": "|", "mask": 16777216}', 0, 0),
  (3030, 'play_sequence', 10010, NULL, '{}', 3000, 1);

-- Chain 3031: re-login on step 4625 or 4626 restores Carter's marker. On
-- step 4627 the marker is the SMG's (chain 3037).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3031, 'SGC_W1 - Load active 4625 or 4626 (Human): restore Col. Carter marker', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3031, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3031, 'mission_status', 1562, NULL, 'eq', 'active', 0),
  (3031, 'step_status', 1562, '4624', 'neq', 'active', 1),
  (3031, 'step_status', 1562, '4627', 'neq', 'active', 2),
  (3031, 'archetype', NULL, NULL, 'lt', '5', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3031, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
        '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3032: reach Carter (step 4625) → step 4626 "Make certain that Col.
-- Carter is ok." and her dialog 5367.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3032, 'SGC_W1 - Interact Col. Carter step 4625 (Human): advance to 4626, show dialog 5367', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3032, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3032, 'step_status', 1562, '4625', 'eq', 'active', 0),
  (3032, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3032, 'advance_step', 1562, '4626', '{}', 0, 0),
  (3032, 'display_dialog', 5367, NULL, '{}', 0, 1);

-- Chain 3033: closing dialog 5367 (it has no buttons, so the close is the
-- choice) → step 4627 "Take the sub-machine gun from Col. Carter's desk.":
-- the marker moves from Carter to the SMG on her desk, which is lit as a
-- mission object and becomes pressable.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3033, 'SGC_W1 - Dialog 5367 closed (Human): advance 1562 to 4627, light the desk SMG', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3033, 'dialog_choice', '5367', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3033, 'step_status', 1562, '4626', 'eq', 'active', 0),
  (3033, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3033, 'advance_step', 1562, '4627', '{}', 0, 0),
  (3033, 'set_interaction_type', NULL, 'SGC_W1_SamCarter',
   '{"op": "~", "mask": 16777216}', 0, 1),
  (3033, 'set_interaction_type', NULL, 'SGC_W1_CarterDeskSMG',
   '{"op": "|", "mask": 1073741824}', 0, 2);

-- Chain 3034: Carter pressed again while step 4626 is still active (the
-- close of 5367 was lost to a relog or an evicted dialog) → show 5367 again,
-- so the press is never silent and the close can still advance the step.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3034, 'SGC_W1 - Interact Col. Carter step 4626 (Human): show dialog 5367 again', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3034, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3034, 'step_status', 1562, '4626', 'eq', 'active', 0),
  (3034, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3034, 'display_dialog', 5367, NULL, '{}', 0, 0);

-- Chain 3035: the desk step (4627), a press on the SMG on Carter's desk →
-- complete 1562, SGHC 6 SMG (item 21) to the backpack with 0 rounds
-- (OD-CS13), the pickup line, the SMG unlit. complete_mission closes the
-- step gate, so a second press grants nothing; unlit, the prop is scenery
-- again. Same shape as the CellBlock pickup of the same gun (chain 1055).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3035, 'SGC_W1 - Interact desk SMG step 4627 (Human): complete 1562, grant SMG 21', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3035, 'interact_tag', 'SGC_W1_CarterDeskSMG', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3035, 'step_status', 1562, '4627', 'eq', 'active', 0),
  (3035, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3035, 'complete_mission', 1562, NULL, '{}', 0, 0),
  (3035, 'add_item', 21, NULL, '{"container": 1, "qty": 1}', 0, 1),
  (3035, 'display_dialog', 5368, NULL, '{}', 0, 2),
  (3035, 'set_interaction_type', NULL, 'SGC_W1_CarterDeskSMG',
   '{"op": "~", "mask": 1073741824}', 0, 3);

-- Chain 3036: Carter pressed during the desk step (4627) → her dialog 5367
-- again, whose last line is "There's another weapon on my lab desk." She is
-- unmarked on this step, so the press may not be possible at all; if it is,
-- it is answered with the pointer to the desk and grants nothing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3036, 'SGC_W1 - Interact Col. Carter step 4627 (Human): show dialog 5367, the desk hint', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3036, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3036, 'step_status', 1562, '4627', 'eq', 'active', 0),
  (3036, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3036, 'display_dialog', 5367, NULL, '{}', 0, 0);

-- Chain 3037: re-login on the desk step (4627) lights the SMG again.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3037, 'SGC_W1 - Load active 4627 (Human): restore the desk SMG marker', 'mission', 1562, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3037, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3037, 'step_status', 1562, '4627', 'eq', 'active', 0),
  (3037, 'archetype', NULL, NULL, 'lt', '5', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3037, 'set_interaction_type', NULL, 'SGC_W1_CarterDeskSMG',
        '{"op": "|", "mask": 1073741824}', 0, 0);

-- ============================================================
-- MISSION 1569 - Ordinance: class rewards and signatures (Class Start v6, CS-05)
-- ============================================================
-- PROJECT_FINAL_S2C. Keyed on `mission_completed 1569` and nothing else, so
-- the SGU route campaign (OD-CS10) can author how 1569 is offered and how
-- its storage crates complete it. Until then nothing accepts or completes
-- 1569, so these chains do not fire in play.
--
-- ONE RULE FOR THAT ROUTE: its offer chain must gate on
-- `mission_status 1569 eq not_active`. These chains are not one-shot. The
-- offer guard refuses a completed mission only when `repeats > num_repeats`,
-- and 1569's `num_repeats` is 1, so the server allows one re-accept after the
-- first completion; a second completion would fire these chains again and
-- grant every item twice (the signature grant is idempotent, the items are
-- not). A completed mission is not `not_active`, so that gate closes it. No
-- existing condition can make a `mission_completed` chain once-per-character
-- by itself: counters reset on relog and `tutorial_shown` takes tutorial
-- dialogs only.
--
-- One chain per Human class (conditions AND, so each `archetype eq N` needs
-- its own chain). Each grants the Gear matrix's M1569 row and the class's
-- one free signature ability (OD-CS06), `source_kind` signature, `source_id`
-- 1569, with the grant's own `archetypes` list as the second gate. Items use
-- container 0, the item's own default container, as a loot pickup does.
-- Every gun arrives with 0 rounds (OD-CS13). Free Jaffa (7) get their gear
-- at the Dakara start (CS-06) and the Asgard (5) is blocked (B1-B3): neither
-- has a chain here.

-- Chain 3041: Soldier → SK37 LMG 3260, Armored BDU Jacket 7373, Quick Burst 598.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3041, 'SGC_W1 - Mission 1569 completed (Soldier): LMG, jacket, signature 598', 'mission', 1569, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3041, 'mission_completed', '1569', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3041, 'archetype', NULL, NULL, 'eq', '1', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3041, 'add_item', 3260, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (3041, 'add_item', 7373, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (3041, 'grant_ability', NULL, NULL,
   '{"ability_ids": [598], "source_kind": "signature", "source_id": 1569, "archetypes": [1]}', 0, 2);

-- Chain 3042: Commando → the Covert Stealth set (reward family A, OD-CS12:
-- 3347, 3359, 3372, 3387, 3401), Combat Knife 3325, Stealth I 646.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3042, 'SGC_W1 - Mission 1569 completed (Commando): stealth set, knife, signature 646', 'mission', 1569, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3042, 'mission_completed', '1569', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3042, 'archetype', NULL, NULL, 'eq', '2', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3042, 'add_item', 3347, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (3042, 'add_item', 3359, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (3042, 'add_item', 3372, NULL, '{"container": 0, "qty": 1}', 0, 2),
  (3042, 'add_item', 3387, NULL, '{"container": 0, "qty": 1}', 0, 3),
  (3042, 'add_item', 3401, NULL, '{"container": 0, "qty": 1}', 0, 4),
  (3042, 'add_item', 3325, NULL, '{"container": 0, "qty": 1}', 0, 5),
  (3042, 'grant_ability', NULL, NULL,
   '{"ability_ids": [646], "source_kind": "signature", "source_id": 1569, "archetypes": [2]}', 0, 6);

-- Chain 3043: Scientist → Deployment Belt 4444, Armored BDU Jacket 7373,
-- Battlefield Heal 948.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3043, 'SGC_W1 - Mission 1569 completed (Scientist): belt, jacket, signature 948', 'mission', 1569, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3043, 'mission_completed', '1569', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3043, 'archetype', NULL, NULL, 'eq', '3', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3043, 'add_item', 4444, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (3043, 'add_item', 7373, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (3043, 'grant_ability', NULL, NULL,
   '{"ability_ids": [948], "source_kind": "signature", "source_id": 1569, "archetypes": [3]}', 0, 2);

-- Chain 3044: Archaeologist → Hologram Emitter 6843, Armored BDU Jacket
-- 7373, Reveal Mini-Games 802.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3044, 'SGC_W1 - Mission 1569 completed (Archaeologist): emitter, jacket, signature 802', 'mission', 1569, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3044, 'mission_completed', '1569', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3044, 'archetype', NULL, NULL, 'eq', '4', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3044, 'add_item', 6843, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (3044, 'add_item', 7373, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (3044, 'grant_ability', NULL, NULL,
   '{"ability_ids": [802], "source_kind": "signature", "source_id": 1569, "archetypes": [4]}', 0, 2);
