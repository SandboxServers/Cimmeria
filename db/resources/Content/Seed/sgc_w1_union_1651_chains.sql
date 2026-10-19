-- SGC_W1 Union continuation: 1651 SGC half, C2 arrival and C2b (packet US-30, chains 3133-3142).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;
-- Every press and dialog choice is a client event, so every chain here carries a world 58 row (S3 review 2).
-- No archetype row on purpose: 1651 is step-gated for both races (spec, US-30 Flow); a non-Human on 1651 is the Free Jaffa route.

-- CIMMERIA_RECONSTRUCTION (H10, F13): O5956 at Carter while O6037 stays open; Harriman advances 4931 -> 4929 -> 5171; no complete_objective on 6037.
-- Chain 3133: arrival from Dakara on 4928 - Hammond shows "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3133, 'SGC_W1 - 1651 on 4928: Hammond "!"', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3133, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3133, 'step_status', 1651, '4928', 'eq', 'active', 0),
  (3133, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3133, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "|", "mask": 16777216}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3134 - Hammond plans the overload while 4928 active: show 5844.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3134, 'SGC_W1 - 1651 Hammond plans the overload (5844)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3134, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3134, 'step_status', 1651, '4928', 'eq', 'active', 0),
  (3134, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3134, 'display_dialog', 5844, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3135 - 5844 closed while 4928 active: to 4931, Carter in the Gate Room (P08, sealed) with "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3135, 'SGC_W1 - 1651 5844 closed: to the Gate Room', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3135, 'dialog_choice', '5844', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3135, 'step_status', 1651, '4928', 'eq', 'active', 0),
  (3135, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3135, 'advance_step', 1651, '4931', '{}', 0, 0),
  (3135, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "~", "mask": 16777216}', 0, 1),
  (3135, 'move_entity', NULL, 'SGC_W1_SamCarter', '{"destination": "164.5,-10.8,18.0", "world": "SGC_W1"}', 0, 2),
  (3135, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 3);

-- CIMMERIA_RECONSTRUCTION: chain 3136 - Carter briefs the generators while 4931 active and O5956 open: show 5845.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3136, 'SGC_W1 - 1651 Carter briefs the generators (5845)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3136, 'interact_tag', 'SGC_W1_SamCarter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3136, 'step_status', 1651, '4931', 'eq', 'active', 0),
  (3136, 'objective_status', 1651, '5956', 'neq', 'completed', 1),
  (3136, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3136, 'display_dialog', 5845, NULL, '{}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3137 - receive the generators (O5956 only; O6037 keeps 4931 open). One-shot via the objective_status row.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3137, 'SGC_W1 - 1651 receive the generators (O5956)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3137, 'dialog_choice', '5845', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3137, 'step_status', 1651, '4931', 'eq', 'active', 0),
  (3137, 'objective_status', 1651, '5956', 'neq', 'completed', 1),
  (3137, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3137, 'add_item', 6784, NULL, '{"container": 2, "qty": 1}', 0, 0),
  (3137, 'complete_objective', 1651, '5956', '{}', 0, 1),
  (3137, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "~", "mask": 16777216}', 0, 2),
  (3137, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 3);

-- CIMMERIA_RECONSTRUCTION (F4, D-US4 default): advance at 0, dial at 0/2500, arrive at 3000; 3141/3142 cover a disconnect inside the window.
-- Chain 3138: Harriman dials Dakara (C2b) once the generator is held. Pin: gate 25 (crossings.md, C2b row).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3138, 'SGC_W1 - 1651 Harriman dials Dakara (C2b)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3138, 'interact_tag', 'SGC_W1_Harriman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3138, 'step_status', 1651, '4931', 'eq', 'active', 0),
  (3138, 'objective_status', 1651, '5956', 'eq', 'completed', 1),
  (3138, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3138, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "~", "mask": 16777216}', 0, 0),
  (3138, 'advance_step', 1651, '4929', '{}', 0, 1),
  (3138, 'play_sequence', 10047, NULL, '{}', 0, 2),
  (3138, 'play_sequence', 10060, NULL, '{}', 2500, 3),
  (3138, 'advance_step', 1651, '5171', '{}', 3000, 4),
  (3138, 'cross_world_teleport', NULL, 'Dakara_E1', '{"x": 96.87, "y": -16.75, "z": 243.23}', 3000, 5);

-- CIMMERIA_RECONSTRUCTION: chain 3139 - relog on 4931 before the generators: Carter back at P08 with "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3139, 'SGC_W1 - relog on 4931 before the generators: Carter "!"', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3139, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3139, 'step_status', 1651, '4931', 'eq', 'active', 0),
  (3139, 'objective_status', 1651, '5956', 'neq', 'completed', 1),
  (3139, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3139, 'move_entity', NULL, 'SGC_W1_SamCarter', '{"destination": "164.5,-10.8,18.0", "world": "SGC_W1"}', 0, 0),
  (3139, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 1);

-- CIMMERIA_RECONSTRUCTION: chain 3140 - relog on 4931 with the generators: Harriman "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3140, 'SGC_W1 - relog on 4931 with the generators: Harriman "!"', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3140, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3140, 'step_status', 1651, '4931', 'eq', 'active', 0),
  (3140, 'objective_status', 1651, '5956', 'eq', 'completed', 1),
  (3140, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3140, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3141 - relog on 4929 (interrupted dial): Harriman "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3141, 'SGC_W1 - relog on 4929 (interrupted dial): Harriman "!"', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3141, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3141, 'step_status', 1651, '4929', 'eq', 'active', 0),
  (3141, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3141, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 0);

-- CIMMERIA_RECONSTRUCTION: chain 3142 - recovery: Harriman sends you through while 4929 is active (interrupted dial).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3142, 'SGC_W1 - recovery: Harriman sends you through (4929)', 'space', NULL, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3142, 'interact_tag', 'SGC_W1_Harriman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3142, 'step_status', 1651, '4929', 'eq', 'active', 0),
  (3142, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3142, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "~", "mask": 16777216}', 0, 0),
  (3142, 'advance_step', 1651, '5171', '{}', 0, 1),
  (3142, 'cross_world_teleport', NULL, 'Dakara_E1', '{"x": 96.87, "y": -16.75, "z": 243.23}', 0, 2);
