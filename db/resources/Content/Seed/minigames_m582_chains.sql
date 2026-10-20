-- Minigames wave 6 (MG-61): mission 582 step 2307, Converse with Marsh.
-- Plan: docs/analysis/minigames/packets/wave6-7-content-close.md (MG-61), decision D-MG22.
-- Block 24000-24001 (C7 reserves 24000-24009 for minigame content; 24020-24029 stay unused).
-- Provenance: mission 582 step 2307 (objective 2675, next step 3274) is RESTORED_SOURCE in
-- mission_steps.sql and mission_objectives.sql. step_enabled is set to true to match this packet;
-- no server code reads the flag yet.
-- Marsh is spawn 344 in world 68 with interact tag CmdCenter_Marsh; dialog 2433 is the prompt.
-- Mission 582 cannot be offered yet: giver Blackstock (NPC 948) has no spawn and set 530 is unbound (M10).
-- The launcher steps aside while Marsh's other missions are on their own steps (1360/4038, 1361/4694).
SET search_path = resources, pg_catalog;

-- Chain 24000: interact_tag CmdCenter_Marsh, world 68, step 2307 active -> prompt, then the Converse minigame.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24000, '582 - Converse with Marsh: display prompt, start ConverseBasicHumanoid (MG-61)', 'mission', 582, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (24000, 'interact_tag', 'CmdCenter_Marsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (24000, 'world', 68, NULL, 'eq', NULL, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (24000, 'mission_status', 582, NULL, 'eq', 'active', 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (24000, 'step_status', 582, '2307', 'eq', 'active', 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (24000, 'step_status', 1360, '4038', 'neq', 'active', 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (24000, 'step_status', 1361, '4694', 'neq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24000, 'display_dialog', 2433, NULL, '{}', 0, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24000, 'start_minigame', NULL, 'ConverseBasicHumanoid', '{"difficulty": 1, "on_victory_chains": [24001]}', 0, 1);

-- Chain 24001: Converse victory (triggered by 24000 on_victory_chains, no trigger row) -> advance
-- mission 582 to step 3274. No complete_objective: 2675 is the only required objective of step 2307,
-- and completing it ends the mission (F19 trap). advance_step closes 2675 as part of the step change.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (24001, '582 - Converse with Marsh victory: advance to step 3274 (MG-61)', 'mission', 582, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (24001, 'advance_step', 582, '3274', '{}', 0, 0);