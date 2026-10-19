-- SGC_W1 Union continuation: 1655 Human and crossing C4 (packet US-32, chains 3156-3165).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;
-- Chain 3156: Woolsey announces the closure to a Human (5854), once 1654 is done.
-- CIMMERIA_RECONSTRUCTION (H9, D-US11 default): Woolsey offers 1655 to Humans; the letter 6786 on Hammond's 5856; no complete_objective on 1655 (F13).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3156, 'SGC_W1 - 1655 Woolsey announces the closure (Human, 5854)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3156, 'interact_tag', 'SGC_W1_Woolsey', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3156, 'mission_status', 1654, NULL, 'eq', 'completed', 0),
  (3156, 'mission_status', 1655, NULL, 'eq', 'not_active', 1),
  (3156, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3156, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3156, 'display_dialog', 5854, NULL, '{}', 0, 0);

-- Chain 3157: Accept on 5854 accepts 1655, Woolsey's "?" goes and Hammond's "!" comes.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3157, 'SGC_W1 - accept 1655 on 5854 (Human)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3157, 'dialog_choice', '5854', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3157, 'mission_status', 1655, NULL, 'eq', 'not_active', 0),
  (3157, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3157, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3157, 'accept_mission', 1655, NULL, '{}', 0, 0),
  (3157, 'set_interaction_type', NULL, 'SGC_W1_Woolsey', '{"op": "~", "mask": 8388608}', 0, 1),
  (3157, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "|", "mask": 16777216}', 0, 2);

-- Chain 3158: press Hammond on 4943 (Human) → his orders, dialog 5856.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3158, 'SGC_W1 - 1655 Hammond''s orders (Human, 5856)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3158, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3158, 'step_status', 1655, '4943', 'eq', 'active', 0),
  (3158, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3158, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3158, 'display_dialog', 5856, NULL, '{}', 0, 0);

-- Chain 3159: close 5856 grants the letter 6786 once, advances to 4944, Hammond's "!" goes and Harriman's "!" comes.
-- The step_status row makes it one-shot: after the advance a second close matches nothing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3159, 'SGC_W1 - 1655 5856 closed: the letter, then Harriman (Human)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3159, 'dialog_choice', '5856', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3159, 'step_status', 1655, '4943', 'eq', 'active', 0),
  (3159, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3159, 'archetype', NULL, NULL, 'lte', '4', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3159, 'add_item', 6786, NULL, '{"container": 2, "qty": 1}', 0, 0),
  (3159, 'advance_step', 1655, '4944', '{}', 0, 1),
  (3159, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "~", "mask": 16777216}', 0, 2),
  (3159, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 3);

-- Chain 3160: press Harriman on 4944 → his orders, dialog 5857. No archetype row: US-33 sends the Free Jaffa here.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3160, 'SGC_W1 - 1655 Harriman has his orders (5857)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3160, 'interact_tag', 'SGC_W1_Harriman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3160, 'step_status', 1655, '4944', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3160, 'display_dialog', 5857, NULL, '{}', 0, 0);

-- Chain 3161: close 5857 dials Omega Site (C4). The C4 press chain is keyed on the 5857 close, not the interact.
-- CIMMERIA_RECONSTRUCTION (F4, D-US4 default): advance at 0, dial at 0/2500, arrive at 3000; 3164/3165 cover a disconnect inside the window. No archetype row: US-33 sends the Free Jaffa through here.
-- The arrival pin is the one UO-00 publishes (omega-work-packets.md, Receiving: C4); its heading 3.1409 is not an action parameter.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3161, 'SGC_W1 - 1655 Harriman dials Omega Site (C4)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3161, 'dialog_choice', '5857', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3161, 'step_status', 1655, '4944', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3161, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "~", "mask": 16777216}', 0, 0),
  (3161, 'advance_step', 1655, '4941', '{}', 0, 1),
  (3161, 'play_sequence', 10047, NULL, '{}', 0, 2),
  (3161, 'play_sequence', 10060, NULL, '{}', 2500, 3),
  (3161, 'advance_step', 1655, '4940', '{}', 3000, 4),
  (3161, 'cross_world_teleport', NULL, 'Omega_Site', '{"x": 34.70, "y": -51.28, "z": -142.00}', 3000, 5);

-- Chain 3162: relog on 4943 → Hammond's "!" is back (Human).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3162, 'SGC_W1 - relog on 4943: Hammond "!" (Human)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3162, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3162, 'step_status', 1655, '4943', 'eq', 'active', 0),
  (3162, 'archetype', NULL, NULL, 'gte', '1', 1),
  (3162, 'archetype', NULL, NULL, 'lte', '4', 2),
  (3162, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3162, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3163: relog on 4944 → Harriman's "!" is back.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3163, 'SGC_W1 - relog on 4944: Harriman "!"', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3163, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3163, 'step_status', 1655, '4944', 'eq', 'active', 0),
  (3163, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3163, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3164: relog inside the dial window (4941) → Harriman's "!" is back, so the player can finish through him.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3164, 'SGC_W1 - relog on 4941 (interrupted dial): Harriman "!"', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3164, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3164, 'step_status', 1655, '4941', 'eq', 'active', 0),
  (3164, 'world', 58, NULL, 'eq', NULL, 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3164, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 0);

-- Chain 3165: recovery, Harriman sends you through from 4941 (the F4 re-press risk: a press inside the window matches here).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3165, 'SGC_W1 - recovery: Harriman sends you through (4941)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3165, 'interact_tag', 'SGC_W1_Harriman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (3165, 'step_status', 1655, '4941', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3165, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "~", "mask": 16777216}', 0, 0),
  (3165, 'advance_step', 1655, '4940', '{}', 0, 1),
  (3165, 'cross_world_teleport', NULL, 'Omega_Site', '{"x": 34.70, "y": -51.28, "z": -142.00}', 0, 2);
