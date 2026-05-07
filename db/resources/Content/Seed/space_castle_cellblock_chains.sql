-- Content chains for space: space:8
-- Auto-exported by Cimmeria Content Editor

SET search_path = resources, pg_catalog;

-- Chain 5000: Castle_CellBlock → Event_Loaded (node 13)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5000, 'Castle_CellBlock → Event_Loaded (node 13)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5000, 'player_loaded', NULL, 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5000, 'mission_status', 622, NULL, 'eq', 'not_active', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5000, 'mission_status', 622, NULL, 'eq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5000, 'accept_mission', 622, NULL,
   '{}', 0, 0),
  (5000, 'display_dialog', 2982, NULL,
   '{}', 0, 1),
  (5000, 'add_dialog', 5229, NULL,
   '{"entity_template":14,"mission_id":622}', 0, 2),
  (5000, 'play_sequence', 10000, NULL,
   '{}', 0, 3),
  (5000, 'launch_ability', 1372, NULL,
   '{}', 0, 4);

-- Chain 5001: Castle_CellBlock → Event_Loaded (node 13)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5001, 'Castle_CellBlock → Event_Loaded (node 13)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5001, 'player_loaded', NULL, 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5001, 'mission_status', 622, NULL, 'eq', 'not_active', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5001, 'mission_status', 622, NULL, 'eq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5001, 'accept_mission', 622, NULL,
   '{}', 0, 0),
  (5001, 'display_dialog', 2982, NULL,
   '{}', 0, 1),
  (5001, 'add_dialog', 5229, NULL,
   '{"entity_template":14,"mission_id":622}', 0, 2),
  (5001, 'play_sequence', 10000, NULL,
   '{}', 0, 3),
  (5001, 'accept_mission', 622, NULL,
   '{}', 0, 4),
  (5001, 'display_dialog', 2982, NULL,
   '{}', 0, 5),
  (5001, 'add_dialog', 5229, NULL,
   '{"entity_template":14,"mission_id":622}', 0, 6),
  (5001, 'display_dialog', 2982, NULL,
   '{}', 0, 7),
  (5001, 'display_dialog', 2982, NULL,
   '{}', 0, 8),
  (5001, 'launch_ability', 1372, NULL,
   '{}', 0, 9),
  (5001, 'launch_ability', 1372, NULL,
   '{}', 0, 10),
  (5001, 'add_dialog', 5229, NULL,
   '{"entity_template":14,"mission_id":622}', 0, 11),
  (5001, 'play_sequence', 10000, NULL,
   '{}', 0, 12),
  (5001, 'play_sequence', 10000, NULL,
   '{}', 0, 13);

-- Chain 5002: Castle_CellBlock → Event_GenericRegion (node 116)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5002, 'Castle_CellBlock → Event_GenericRegion (node 116)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5002, 'enter_region', 'Castle_Cellblock.Region8', 'player', true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5002, 'generate_threat', NULL, 'ArmYourself_NIDGuard',
   '{"threat_level":5000}', 0, 0);

-- Chain 5003: Castle_CellBlock → Event_GenericRegion (node 116)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5003, 'Castle_CellBlock → Event_GenericRegion (node 116)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5003, 'enter_region', 'Castle_Cellblock.Region8', 'player', true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5003, 'generate_threat', NULL, NULL,
   '{"threat_level":5000}', 0, 0);

-- Chain 5004: Castle_CellBlock → Event_GenericRegion (node 123)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5004, 'Castle_CellBlock → Event_GenericRegion (node 123)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5004, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5004, 'mission_status', 638, NULL, 'eq', 'not_active', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5004, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5004, 'accept_mission', 638, NULL,
   '{}', 0, 0),
  (5004, 'add_dialog', 5866, NULL,
   '{"entity_template":17,"mission_id":0}', 0, 1);

-- Chain 5005: Castle_CellBlock → Event_GenericRegion (node 123)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5005, 'Castle_CellBlock → Event_GenericRegion (node 123)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5005, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5005, 'mission_status', 638, NULL, 'eq', 'not_active', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5005, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5005, 'archetype', NULL, NULL, 'eq', '8', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5005, 'archetype', NULL, NULL, 'eq', '8', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5005, 'accept_mission', 638, NULL,
   '{}', 0, 0),
  (5005, 'add_dialog', 5866, NULL,
   '{"entity_template":17,"mission_id":0}', 0, 1),
  (5005, 'accept_mission', 638, NULL,
   '{}', 0, 2),
  (5005, 'add_dialog', 5866, NULL,
   '{"entity_template":17,"mission_id":0}', 0, 3),
  (5005, 'add_dialog', 5866, NULL,
   '{"entity_template":17,"mission_id":0}', 0, 4),
  (5005, 'add_dialog', 5866, NULL,
   '{"entity_template":17,"mission_id":0}', 0, 5),
  (5005, 'add_dialog', 2794, NULL,
   '{"entity_template":17,"mission_id":0}', 0, 6);

-- Chain 5006: Castle_CellBlock → Event_GenericRegion (node 128)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5006, 'Castle_CellBlock → Event_GenericRegion (node 128)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5006, 'enter_region', 'Castle_Cellblock.Region11', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5006, 'system_message', 5180, NULL,
   '{}', 0, 0);

-- Chain 5007: Castle_CellBlock → Event_GenericRegion (node 128)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5007, 'Castle_CellBlock → Event_GenericRegion (node 128)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5007, 'enter_region', 'Castle_Cellblock.Region11', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5007, 'system_message', 5180, NULL,
   '{}', 0, 0);

-- Chains 5008 and 5009 removed: duplicates of chain 1011's system_message(5040)
-- on Castle_Cellblock.Region2 (node 130). They caused triple chat spam and
-- client freezes on every region crossing.

-- Chain 5010: Castle_CellBlock → Event_GenericRegion (node 135)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5010, 'Castle_CellBlock → Event_GenericRegion (node 135)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5010, 'enter_region', 'Castle_Cellblock.Region10', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5010, 'system_message', 5181, NULL,
   '{}', 0, 0);

-- Chain 5011: Castle_CellBlock → Event_GenericRegion (node 135)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5011, 'Castle_CellBlock → Event_GenericRegion (node 135)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5011, 'enter_region', 'Castle_Cellblock.Region10', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5011, 'system_message', 5181, NULL,
   '{}', 0, 0);

-- Chain 5012: Castle_CellBlock → Event_Teleport (node 141)
-- DISABLED: auto-generated converter incorrectly emitted `accept_mission 640` where the original
-- Python (`Castle_CellBlock.py` teleportCb) calls `missions.complete(640)`. Mission 640's
-- teleport_in flow is already correctly handled by curated chain 1044. Re-enabling this would
-- re-accept the just-completed mission and loop the player back to "hack the rings".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5012, 'Castle_CellBlock → Event_Teleport (node 141)', 'space', 8, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5012, 'teleport_in', '2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5012, 'mission_status', 640, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5012, 'accept_mission', 640, NULL,
   '{}', 0, 0),
  (5012, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"mask":8388608,"op":"|"}', 0, 1);

-- Chain 5013: Castle_CellBlock → Event_Teleport (node 141)
-- DISABLED: same root cause as 5012 — auto-generated converter emitted `accept_mission 640`
-- where the original Python (`Castle_CellBlock.py` teleportCb) calls `missions.complete(640)`.
-- Mission 640's teleport_in flow is already correctly handled by curated chain 1044. Re-enabling
-- this would re-accept the just-completed mission and loop the player back to "hack the rings".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5013, 'Castle_CellBlock → Event_Teleport (node 141)', 'space', 8, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5013, 'teleport_in', '2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5013, 'mission_status', 640, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5013, 'accept_mission', 640, NULL,
   '{}', 0, 0),
  (5013, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"mask":8388608,"op":"|"}', 0, 1),
  (5013, 'accept_mission', 640, NULL,
   '{}', 0, 2),
  (5013, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"mask":8388608,"op":"|"}', 0, 3);

-- Chain 5014: Castle_CellBlock → Event_EntityInteract (node 147)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5014, 'Castle_CellBlock → Event_EntityInteract (node 147)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5014, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5014, 'step_status', 641, '2121', 'eq', 'not_active', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5014, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5014, 'display_dialog', 5022, NULL,
   '{}', 0, 0);

-- Chain 5015: Castle_CellBlock → Event_EntityInteract (node 147)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5015, 'Castle_CellBlock → Event_EntityInteract (node 147)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5015, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5015, 'step_status', 641, '2121', 'eq', 'not_active', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5015, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5015, 'archetype', NULL, NULL, 'eq', '8', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5015, 'display_dialog', 5022, NULL,
   '{}', 0, 0),
  (5015, 'display_dialog', 5022, NULL,
   '{}', 0, 1),
  (5015, 'display_dialog', 4001, NULL,
   '{}', 0, 2),
  (5015, 'display_dialog', 5022, NULL,
   '{}', 0, 3);

-- Chain 5016: Castle_CellBlock → Event_GenericRegion (node 155)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5016, 'Castle_CellBlock → Event_GenericRegion (node 155)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5016, 'enter_region', 'Castle_Cellblock.Region3', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5016, 'system_message', 5182, NULL,
   '{}', 0, 0);

-- Chain 5017: Castle_CellBlock → Event_GenericRegion (node 155)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5017, 'Castle_CellBlock → Event_GenericRegion (node 155)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5017, 'enter_region', 'Castle_Cellblock.Region3', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5017, 'system_message', 5182, NULL,
   '{}', 0, 0);

-- Chain 5018: Castle_CellBlock → Event_GenericRegion (node 157)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5018, 'Castle_CellBlock → Event_GenericRegion (node 157)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5018, 'enter_region', 'Castle_Cellblock.Region12', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5018, 'system_message', 5184, NULL,
   '{}', 0, 0);

-- Chain 5019: Castle_CellBlock → Event_GenericRegion (node 157)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5019, 'Castle_CellBlock → Event_GenericRegion (node 157)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5019, 'enter_region', 'Castle_Cellblock.Region12', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5019, 'system_message', 5184, NULL,
   '{}', 0, 0);

-- Chain 5020: Castle_CellBlock → Event_GenericRegion (node 159)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5020, 'Castle_CellBlock → Event_GenericRegion (node 159)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5020, 'enter_region', 'Castle_Cellblock.Region13', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5020, 'system_message', 5185, NULL,
   '{}', 0, 0);

-- Chain 5021: Castle_CellBlock → Event_GenericRegion (node 159)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5021, 'Castle_CellBlock → Event_GenericRegion (node 159)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5021, 'enter_region', 'Castle_Cellblock.Region13', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5021, 'system_message', 5185, NULL,
   '{}', 0, 0);

-- Chain 5022: Castle_CellBlock → Event_GenericRegion (node 187)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5022, 'Castle_CellBlock → Event_GenericRegion (node 187)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5022, 'enter_region', 'Castle_Cellblock.Region3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5022, 'mission_status', 681, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5022, 'mission_status', 682, NULL, 'eq', 'not_active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5022, 'mission_status', 682, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5022, 'accept_mission', 682, NULL,
   '{}', 0, 0),
  (5022, 'accept_mission', 682, NULL,
   '{}', 0, 1),
  (5022, 'accept_mission', 682, NULL,
   '{}', 0, 2);

-- Chain 5023: Castle_CellBlock → Event_GenericRegion (node 187)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5023, 'Castle_CellBlock → Event_GenericRegion (node 187)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5023, 'exit_region', 'Castle_Cellblock.Region3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5023, 'mission_status', 681, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5023, 'mission_status', 682, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5023, 'accept_mission', 682, NULL,
   '{}', 0, 0);

-- Chain 5024: Castle_CellBlock → Event_GenericRegion (node 211)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5024, 'Castle_CellBlock → Event_GenericRegion (node 211)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5024, 'enter_region', 'Castle_Cellblock.Region4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5024, 'mission_status', 683, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5024, 'mission_status', 684, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5024, 'accept_mission', 684, NULL,
   '{}', 0, 0);

-- Chain 5025: Castle_CellBlock → Event_GenericRegion (node 211)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5025, 'Castle_CellBlock → Event_GenericRegion (node 211)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5025, 'enter_region', 'Castle_Cellblock.Region4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5025, 'mission_status', 683, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5025, 'mission_status', 684, NULL, 'eq', 'not_active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5025, 'mission_status', 684, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5025, 'accept_mission', 684, NULL,
   '{}', 0, 0),
  (5025, 'accept_mission', 684, NULL,
   '{}', 0, 1),
  (5025, 'accept_mission', 684, NULL,
   '{}', 0, 2);

-- Chain 5026: Castle_CellBlock → Event_GenericRegion (node 220)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5026, 'Castle_CellBlock → Event_GenericRegion (node 220)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5026, 'enter_region', 'Castle_Cellblock.Region5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5026, 'mission_status', 685, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5026, 'mission_status', 686, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5026, 'accept_mission', 686, NULL,
   '{}', 0, 0);

-- Chain 5027: Castle_CellBlock → Event_GenericRegion (node 220)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5027, 'Castle_CellBlock → Event_GenericRegion (node 220)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5027, 'enter_region', 'Castle_Cellblock.Region5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5027, 'mission_status', 685, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5027, 'mission_status', 686, NULL, 'eq', 'not_active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5027, 'mission_status', 686, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5027, 'accept_mission', 686, NULL,
   '{}', 0, 0),
  (5027, 'accept_mission', 686, NULL,
   '{}', 0, 1),
  (5027, 'accept_mission', 686, NULL,
   '{}', 0, 2);

-- Chain 5028: Castle_CellBlock → Event_GenericRegion (node 225)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5028, 'Castle_CellBlock → Event_GenericRegion (node 225)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5028, 'enter_region', 'Castle_Cellblock.Region6', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5028, 'mission_status', 686, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5028, 'mission_status', 687, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5028, 'accept_mission', 687, NULL,
   '{}', 0, 0);

-- Chain 5029: Castle_CellBlock → Event_GenericRegion (node 225)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (5029, 'Castle_CellBlock → Event_GenericRegion (node 225)', 'space', 8, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (5029, 'enter_region', 'Castle_Cellblock.Region6', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5029, 'mission_status', 686, NULL, 'eq', 'completed', 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5029, 'mission_status', 687, NULL, 'eq', 'not_active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (5029, 'mission_status', 687, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (5029, 'accept_mission', 687, NULL,
   '{}', 0, 0),
  (5029, 'accept_mission', 687, NULL,
   '{}', 0, 1),
  (5029, 'accept_mission', 687, NULL,
   '{}', 0, 2);

