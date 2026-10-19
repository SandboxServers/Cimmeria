-- Dakara_E1 rebuild: mission 1652 "Fireball" (the two ring controls).
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packets DK-31b, DK-31c).
-- Chain ID range: 8320-8349 (work-packets.md, C3). Evidence labels: README.md, "Evidence Labels".

SET search_path = resources, pg_catalog;

-- Chain 8320 PROJECT_FINAL_RECONSTRUCTION: 1651 done in the story room, so the
-- 1652 offer (dsm 6776, dialog 5986, available) is bound to Bra'tac (template 59).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8320, '1652 - 1651 completed on Dakara_E1_StoryRm: bind offer 6776 to Bra''tac (59)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8320, 'mission_completed', '1651', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8320, 'world', 62, NULL, 'eq', NULL, 0),
  (8320, 'mission_status', 1652, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8320, 'add_dialog_set', 6776, NULL, '{"slot": 59}', 0, 0);

-- Chain 8321 PROJECT_FINAL_RECONSTRUCTION: restore the 1652 offer bind on entering
-- the story room after 1651 is done (the bind does not survive a relog or world change).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8321, '1652 - Restore offer on Dakara_E1_StoryRm after 1651: bind 6776 to Bra''tac (59)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8321, 'player_loaded', 'Dakara_E1_StoryRm', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8321, 'world', 62, NULL, 'eq', NULL, 0),
  (8321, 'mission_status', 1651, NULL, 'eq', 'completed', 1),
  (8321, 'mission_status', 1652, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8321, 'add_dialog_set', 6776, NULL, '{"slot": 59}', 0, 0);

-- Chain 8322 PROJECT_FINAL_RECONSTRUCTION: accepting the offer (dialog 5986) accepts
-- 1652 and shows the briefing 5846. The offer bind is removed on accept.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8322, '1652 - Accept offer 5986: accept 1652, briefing 5846, unbind offer 6776', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8322, 'dialog_choice', '5986', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8322, 'world', 62, NULL, 'eq', NULL, 0),
  (8322, 'mission_status', 1651, NULL, 'eq', 'completed', 1),
  (8322, 'mission_status', 1652, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8322, 'accept_mission', 1652, NULL, '{}', 0, 0),
  (8322, 'display_dialog', 5846, NULL, '{}', 0, 1),
  (8322, 'remove_dialog_set', 6776, NULL, '{"slot": 59}', 0, 2);

-- Chain 8323 PROJECT_FINAL_RECONSTRUCTION: step 4933 (the western Ha'tak), bind the
-- western ring control (dsm 6887, dialog 6108, active) on entering Dakara_E1 (the accept is in world 62).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8323, '1652 - Step 4933 on Dakara_E1: bind western ring control 6887 to Dakara_E1_RingControl_XMinus (455)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8323, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8323, 'world', 61, NULL, 'eq', NULL, 0),
  (8323, 'step_status', 1652, '4933', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8323, 'add_dialog_set', 6887, NULL, '{"slot": 455}', 0, 0);

-- Chain 8324 PROJECT_FINAL_RECONSTRUCTION: step 4934 (the eastern Ha'tak), restore the
-- eastern ring control (dsm 6888, dialog 6109, active) on entering Dakara_E1.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8324, '1652 - Step 4934 on Dakara_E1: bind eastern ring control 6888 to Dakara_E1_RingControl_XPlus (462)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8324, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8324, 'world', 61, NULL, 'eq', NULL, 0),
  (8324, 'step_status', 1652, '4934', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8324, 'add_dialog_set', 6888, NULL, '{"slot": 462}', 0, 0);

-- Chain 8325 PROJECT_FINAL_RECONSTRUCTION: click the western control at 4933, the
-- western bomb (dialog 6108). Advance to 4934, which lights the eastern control in the
-- same world: the western bind is removed and the eastern one is bound.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8325, '1652 - Step 4933 click Dakara_E1_RingControl_XMinus: display 6108, advance to 4934, unbind 6887, bind 6888', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8325, 'interact_tag', 'Dakara_E1_RingControl_XMinus', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8325, 'world', 61, NULL, 'eq', NULL, 0),
  (8325, 'step_status', 1652, '4933', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8325, 'display_dialog', 6108, NULL, '{}', 0, 0),
  (8325, 'advance_step', 1652, '4934', '{}', 0, 1),
  (8325, 'remove_dialog_set', 6887, NULL, '{"slot": 455}', 0, 2),
  (8325, 'add_dialog_set', 6888, NULL, '{"slot": 462}', 0, 3);

-- Chain 8326 PROJECT_FINAL_RECONSTRUCTION: click the eastern control at 4934, the
-- eastern bomb (dialog 6109). The generator (item 6784) is spent, the bind is removed
-- before the mission completes (C5 rule 5).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8326, '1652 - Step 4934 click Dakara_E1_RingControl_XPlus: display 6109, take 6784, unbind 6888, complete 1652', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8326, 'interact_tag', 'Dakara_E1_RingControl_XPlus', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8326, 'world', 61, NULL, 'eq', NULL, 0),
  (8326, 'step_status', 1652, '4934', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8326, 'display_dialog', 6109, NULL, '{}', 0, 0),
  (8326, 'remove_item', 6784, NULL, '{"qty": 1}', 0, 1),
  (8326, 'remove_dialog_set', 6888, NULL, '{"slot": 462}', 0, 2),
  (8326, 'complete_mission', 1652, NULL, '{}', 0, 3);

-- Chain 8327 PROJECT_FINAL_RECONSTRUCTION: step 4933 on Dakara_E1, show both Ha'taks
-- (unhide sequences 2392 west and 2390 east, ORIGINAL_DATA Kismet, DK-31a). Delayed 3 s so
-- the map has loaded (precedent: chain 3030). Co-fires with 8323; no accept-time unhide,
-- because the accept (8322) is in world 62, which has no Ha'tak (D-DK33).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8327, '1652 - Step 4933 on Dakara_E1: show both Ha''taks (play 2392 west, 2390 east)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8327, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8327, 'world', 61, NULL, 'eq', NULL, 0),
  (8327, 'step_status', 1652, '4933', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8327, 'play_sequence', 2392, NULL, '{}', 3000, 0),
  (8327, 'play_sequence', 2390, NULL, '{}', 3000, 1);

-- Chain 8328 PROJECT_FINAL_RECONSTRUCTION: step 4934 on Dakara_E1, show the eastern Ha'tak
-- only (unhide 2390). The map hides both ships on every load, so the western one, destroyed
-- at 4933, stays gone; there is no explosion replay (D-DK34). Co-fires with 8324.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8328, '1652 - Step 4934 on Dakara_E1: show the eastern Ha''tak (play 2390)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8328, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8328, 'world', 61, NULL, 'eq', NULL, 0),
  (8328, 'step_status', 1652, '4934', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8328, 'play_sequence', 2390, NULL, '{}', 3000, 0);

-- Chain 8329 PROJECT_FINAL_RECONSTRUCTION: click the western control at 4933, explode the
-- western Ha'tak (2393) 3 s after the bomb dialog 6108. Co-fires with 8325, which advances
-- the step; conditions are read before either chain acts.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8329, '1652 - Step 4933 click Dakara_E1_RingControl_XMinus: explode the western Ha''tak (play 2393)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8329, 'interact_tag', 'Dakara_E1_RingControl_XMinus', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8329, 'world', 61, NULL, 'eq', NULL, 0),
  (8329, 'step_status', 1652, '4933', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8329, 'play_sequence', 2393, NULL, '{}', 3000, 0);

-- Chain 8330 PROJECT_FINAL_RECONSTRUCTION: click the eastern control at 4934, explode the
-- eastern Ha'tak (2391) 3 s after the bomb dialog 6109. Co-fires with 8326, which completes
-- 1652; a delayed action survives the completion (it is dropped only on logout).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8330, '1652 - Step 4934 click Dakara_E1_RingControl_XPlus: explode the eastern Ha''tak (play 2391)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8330, 'interact_tag', 'Dakara_E1_RingControl_XPlus', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8330, 'world', 61, NULL, 'eq', NULL, 0),
  (8330, 'step_status', 1652, '4934', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8330, 'play_sequence', 2391, NULL, '{}', 3000, 0);
