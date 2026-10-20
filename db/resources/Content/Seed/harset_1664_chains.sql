-- Harset mission 1664 "Debriefing" (FZ-20): Sgt. Kaerlia, the Command Center door,
-- relog restores and the abandon tidy-up. Design: docs/analysis/harset-continuation/
-- worknotes/HA-01b-graph-seal.md. Evidence class RECONSTRUCTION; every row PROVISIONAL (O18-13).
--
-- Granted by Castle chain 1398 (castle_1664_chains.sql). There is no chain 1399 and no
-- re-grant here: O18-02 forbids a re-grant after abandonment, so an abandon is final (P0).
-- Steps: 5224 "Speak to Sgt. Kaerlia by the Stargate", 5225 "Make your way to the
-- Command Center", 5226 "Report for debriefing" (FZ-27: D-PU3 = B).
-- Steps 5224/5225 credit with advance_step (complete_objective 6060/6063 would auto-complete 1664).
-- Step 5226 credits with complete_objective 6061 (Marsh, 6018) and 6062 (Moh'katan, 6019), either order; the second auto-completes 1664.
--
-- Chain ids 6008-6020 (moved here from harset_space_chains.sql's reserved block):
--   6008-6011 Kaerlia: K1 credit, K2 later-step bark, K3 no-mission bark, K4 done bark
--   6012-6013 the door control: D1 credit + teleport, D2 teleport only
--   6014-6015 relog restore: R1 Kaerlia cue at 5224, R2 door cue at 5225
--   6016      abandon tidy-up
--   6017      walk-in door credit on 5225 (D-PU4, FZ-25)
--   6018-6019 step 5226 debrief: Marsh, Moh'katan (D-PU3, FZ-27)
--   6020      reserved: re-offer after abandon, only if the owner reverses O18-02
-- Outside that block: 6033-6034 restore the 5226 debrief cues on Command Center login (6021-6032 are HA-01a).
-- Binds are per player and in memory, so each has a player_loaded restore.
-- Every click and login chain carries `world 57 eq`, except the debrief chains 6018/6019/6033/6034 (`world 68 eq`).
-- Disjoint by construction: in each state (no 1664, 5224, 5225, 5226, completed) exactly
-- one of 6008-6011 and one of 6012-6013 matches. Pinned by chain_replay_tests/harset_1664.rs.

-- DSM 120301-120303 (this file owns 120301-120400). NULL dialog_id on all three: each bind
-- only raises one interaction bit for one player. 120301 is the door cue (INT_MissionWorldObject
-- 1073741824, template 261). 120302/120303 are the 5226 debrief cues on Col. Marsh (template 10)
-- and Moh'katan (template 54), INT_NonAStoryMissionTurnIn (536870912).
-- GREATEST keeps the shared 120001+ block from ever lowering.
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions)
VALUES
  (120301, 1992, NULL, 'Command Center door', 1073741824, 1, '{}', '{}', '{}', '{}'),
  (120302, 1992, NULL, 'Report to Col. Marsh', 536870912, 1, '{}', '{}', '{}', '{}'),
  (120303, 1992, NULL, 'Report to Moh''katan', 536870912, 1, '{}', '{}', '{}', '{}');

SELECT pg_catalog.setval(
    'dialog_set_maps_dialog_set_map_id_seq',
    GREATEST(120303, (SELECT last_value FROM dialog_set_maps_dialog_set_map_id_seq)),
    true);

-- 6008 K1: talk to Kaerlia on 5224. Advance first (D-FZ9): the executor keeps going after
-- a failed action, so a lost dialog or bind still leaves the step credited.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6008, '1664 - Kaerlia (step 5224): credit, dialog 6169, move the cue to the door (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6008, 'interact_tag', 'Harset_Kaerlia', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6008, 'world', 57, NULL, 'eq', NULL, 0),
  (6008, 'step_status', 1664, '5224', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6008, 'advance_step', 1664, '5225', '{}', 0, 0),
  (6008, 'display_dialog', 6169, NULL, '{}', 0, 1),
  (6008, 'remove_dialog_set', 7016, NULL, '{"slot": 260}', 0, 2),
  (6008, 'add_dialog_set', 120301, NULL, '{"slot": 261, "mission_id": 1664}', 0, 3);

-- 6009 K2: 1664 active past 5224: point the player at the Command Center.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6009, '1664 - Kaerlia (after 5224): bark the Command Center reminder (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6009, 'interact_tag', 'Harset_Kaerlia', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6009, 'world', 57, NULL, 'eq', NULL, 0),
  (6009, 'mission_status', 1664, NULL, 'eq', 'active', 1),
  (6009, 'step_status', 1664, '5224', 'neq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6009, 'npc_bark', NULL, NULL, '{"screen_id": 117162, "speaker": "Sgt. Kaerlia", "channel": "say"}', 0, 0);

-- 6010 K3: no 1664 (never granted, or abandoned): the stock "keep moving" line.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6010, '1664 - Kaerlia (no 1664): bark keep moving (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6010, 'interact_tag', 'Harset_Kaerlia', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6010, 'world', 57, NULL, 'eq', NULL, 0),
  (6010, 'mission_status', 1664, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6010, 'npc_bark', NULL, NULL, '{"screen_id": 117158, "speaker": "Sgt. Kaerlia", "channel": "say"}', 0, 0);

-- 6011 K4: 1664 completed: the same line.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6011, '1664 - Kaerlia (1664 completed): bark keep moving (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6011, 'interact_tag', 'Harset_Kaerlia', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6011, 'world', 57, NULL, 'eq', NULL, 0),
  (6011, 'mission_status', 1664, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6011, 'npc_bark', NULL, NULL, '{"screen_id": 117158, "speaker": "Sgt. Kaerlia", "channel": "say"}', 0, 0);

-- 6012 D1: the door control on 5225: drop the cue, credit, then travel (the teleport
-- destroys the cell entity, so it must be the last action). Destination copied from 6006.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6012, '1664 - Command Center door (step 5225): credit and travel to Harset_CmdCenter (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6012, 'interact_tag', 'Harset_CmdDoor', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6012, 'world', 57, NULL, 'eq', NULL, 0),
  (6012, 'step_status', 1664, '5225', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6012, 'remove_dialog_set', 120301, NULL, '{"slot": 261}', 0, 0),
  (6012, 'advance_step', 1664, '5226', '{}', 0, 1),
  (6012, 'cross_world_teleport', NULL, 'Harset_CmdCenter', '{"x": 0.0, "y": 0.355, "z": -20.0}', 0, 2);

-- 6013 D2: any other state: the door is a door (parity with walk-in chain 6006).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6013, '1664 - Command Center door (not 5225): travel to Harset_CmdCenter (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6013, 'interact_tag', 'Harset_CmdDoor', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6013, 'world', 57, NULL, 'eq', NULL, 0),
  (6013, 'step_status', 1664, '5225', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6013, 'cross_world_teleport', NULL, 'Harset_CmdCenter', '{"x": 0.0, "y": 0.355, "z": -20.0}', 0, 0);

-- 6014 R1: login in Harset on 5224: restore Kaerlia's cue (dsm 7016, set 1992, dialog 6169).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6014, '1664 - Harset login (step 5224): restore the Kaerlia cue (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6014, 'player_loaded', 'Harset', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6014, 'world', 57, NULL, 'eq', NULL, 0),
  (6014, 'step_status', 1664, '5224', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6014, 'add_dialog_set', 7016, NULL, '{"slot": 260, "mission_id": 1664}', 0, 0);

-- 6015 R2: login in Harset on 5225: restore the door cue.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6015, '1664 - Harset login (step 5225): restore the door cue (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6015, 'player_loaded', 'Harset', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6015, 'world', 57, NULL, 'eq', NULL, 0),
  (6015, 'step_status', 1664, '5225', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6015, 'add_dialog_set', 120301, NULL, '{"slot": 261, "mission_id": 1664}', 0, 0);

-- 6016: abandon, any world: drop every cue. Nothing else (P0; no re-grant).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6016, '1664 - abandoned: drop the Kaerlia, door and debrief cues (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6016, 'mission_abandoned', '1664', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6016, 'mission_status', 1664, NULL, 'eq', 'not_active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6016, 'remove_dialog_set', 7016, NULL, '{"slot": 260}', 0, 0),
  (6016, 'remove_dialog_set', 120301, NULL, '{"slot": 261}', 0, 1),
  (6016, 'remove_dialog_set', 120302, NULL, '{"slot": 10}', 0, 2),
  (6016, 'remove_dialog_set', 120303, NULL, '{"slot": 54}', 0, 3);

-- 6017: walking through the walk-in door (region chain 6006) on 5225 also credits it.
-- Priority 10 so it runs before 6006's teleport removes the player from this cell.
-- No teleport here: 6006 does it.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6017, '1664 - walk-in door (step 5225): credit and drop the door cue (PROVISIONAL)', 'mission', 1664, true, 10);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6017, 'enter_region', 'Harset.CommandCenterTransition', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6017, 'world', 57, NULL, 'eq', NULL, 0),
  (6017, 'step_status', 1664, '5225', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6017, 'remove_dialog_set', 120301, NULL, '{"slot": 261}', 0, 0),
  (6017, 'advance_step', 1664, '5226', '{}', 0, 1);

-- 6018: report to Col. Marsh on 5226 (Command Center, world 68). Bark first, drop the cue, then
-- credit 6061 last. Either order with 6019; the second credit auto-completes 1664.
-- Guards: 1360/4038 (6501, the letter) and 1361/4694 (6527, the Praxis turn-in) also key on
-- Marsh; 6512 (the Praxis offer) is gated off 5226 in harset_opcore_chains.sql (FZ-27b).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6018, '1664 - Col. Marsh (step 5226): bark, drop the cue, credit objective 6061 (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6018, 'interact_tag', 'CmdCenter_Marsh', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6018, 'world', 68, NULL, 'eq', NULL, 0),
  (6018, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6018, 'objective_status', 1664, '6061', 'neq', 'completed', 2),
  (6018, 'step_status', 1360, '4038', 'neq', 'active', 3),
  (6018, 'step_status', 1361, '4694', 'neq', 'active', 4);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6018, 'npc_bark', NULL, NULL, '{"screen_id": 200180, "speaker": "Col. Marsh", "channel": "say"}', 0, 0),
  (6018, 'remove_dialog_set', 120302, NULL, '{"slot": 10}', 0, 1),
  (6018, 'complete_objective', 1664, '6061', '{}', 0, 2);

-- 6019: report to Moh'katan on 5226. Same shape as 6018, objective 6062. Guards: the Jaffa
-- chains 6305 (1324/3954) and 6340 (1326/4603) and the Praxis chains 6515 (1361/4040) and
-- 6520 (1361/4042) also key on Moh'katan.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6019, '1664 - Moh''katan (step 5226): bark, drop the cue, credit objective 6062 (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6019, 'interact_tag', 'CmdCenter_Mohkatan', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6019, 'world', 68, NULL, 'eq', NULL, 0),
  (6019, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6019, 'objective_status', 1664, '6062', 'neq', 'completed', 2),
  (6019, 'step_status', 1324, '3954', 'neq', 'active', 3),
  (6019, 'step_status', 1326, '4603', 'neq', 'active', 4),
  (6019, 'step_status', 1361, '4040', 'neq', 'active', 5),
  (6019, 'step_status', 1361, '4042', 'neq', 'active', 6);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6019, 'npc_bark', NULL, NULL, '{"screen_id": 200181, "speaker": "Moh''katan", "channel": "say"}', 0, 0),
  (6019, 'remove_dialog_set', 120303, NULL, '{"slot": 54}', 0, 1),
  (6019, 'complete_objective', 1664, '6062', '{}', 0, 2);

-- 6033: Command Center login on 5226 with 6061 not yet credited: put the cue on Marsh.
-- Templates 10 and 54 ship interaction_type 0, so without a bind the client never sends the
-- click. 5226 is only ever reached through a teleport (6012, 6006, 6017), so this login chain
-- also covers the arrival. No 1360/1361 guards here: the cue must show whatever else is live.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6033, '1664 - Command Center login (step 5226): restore the Marsh debrief cue (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6033, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6033, 'world', 68, NULL, 'eq', NULL, 0),
  (6033, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6033, 'objective_status', 1664, '6061', 'neq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6033, 'add_dialog_set', 120302, NULL, '{"slot": 10, "mission_id": 1664}', 0, 0);

-- 6034: the same for Moh'katan and 6062.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6034, '1664 - Command Center login (step 5226): restore the Moh''katan debrief cue (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6034, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6034, 'world', 68, NULL, 'eq', NULL, 0),
  (6034, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6034, 'objective_status', 1664, '6062', 'neq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6034, 'add_dialog_set', 120303, NULL, '{"slot": 54, "mission_id": 1664}', 0, 0);
