-- SGC_W1 Union continuation: 1654 Hammond turn-in (packet US-31, chains 3148-3155).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- CIMMERIA_RECONSTRUCTION (H9, C3): Dakara completes O5961; Hammond's report
-- completes O5962 and so 1654 (final step, F13). Dialog text is RESTORED_SOURCE
-- (5852 Human, 5853 Free Jaffa); both dialogs have zero buttons.
--
-- Mission scope, step 4939 is the only step of 1654. Chains 3148-3152 are the
-- arrival marker, the two reports, the close that completes O5962, and the
-- Woolsey arrival with "?" for the 1655 offer. Chain 3153 stays spare.

-- Chain 3148: arriving on 4939 lights Hammond's turn-in marker (mask 33554432).
-- Not while O5962 is complete, so a relog after the report leaves him unmarked.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3148, 'SGC_W1 - 1654 on 4939: Hammond turn-in marker', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3148, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3148, 'step_status', 1654, '4939', 'eq', 'active', 0),
  (3148, 'objective_status', 1654, '5962', 'neq', 'completed', 1),
  (3148, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3148, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
        '{"op": "|", "mask": 33554432}', 0, 0);

-- Chain 3149: Hammond's report for a Human (dialog 5852).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3149, 'SGC_W1 - 1654 Hammond hears the Dakara report (Human, 5852)', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3149, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3149, 'step_status', 1654, '4939', 'eq', 'active', 0),
  (3149, 'objective_status', 1654, '5962', 'neq', 'completed', 1),
  (3149, 'archetype', NULL, NULL, 'gte', '1', 2),
  (3149, 'archetype', NULL, NULL, 'lte', '4', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3149, 'display_dialog', 5852, NULL, '{}', 0, 0);

-- Chain 3150: Hammond's report for a Free Jaffa (dialog 5853). Archetypes 5
-- and 6 match neither report chain: they never reach the SGC on this mission.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3150, 'SGC_W1 - 1654 Hammond hears the Dakara report (Free Jaffa, 5853)', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3150, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3150, 'step_status', 1654, '4939', 'eq', 'active', 0),
  (3150, 'objective_status', 1654, '5962', 'neq', 'completed', 1),
  (3150, 'archetype', NULL, NULL, 'eq', '7', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3150, 'display_dialog', 5853, NULL, '{}', 0, 0);

-- Chain 3151: closing either report (no buttons, so the close sends -1)
-- clears Hammond's turn-in marker and completes O5962. The holder of 4939 is
-- the only player who can reach this dialog, so no archetype row is needed.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3151, 'SGC_W1 - 1654 report closed: complete O5962', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3151, 'dialog_choice', '5852', 'player', false, 0),
  (3151, 'dialog_choice', '5853', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3151, 'step_status', 1654, '4939', 'eq', 'active', 0),
  (3151, 'objective_status', 1654, '5962', 'neq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3151, 'set_interaction_type', NULL, 'SGCW1_GenHammond',
   '{"op": "~", "mask": 33554432}', 0, 0),
  (3151, 'complete_objective', 1654, '5962', '{}', 0, 1);

-- Chain 3152: 1654 done (final step, F13). Woolsey appears beside Hammond at
-- P02 with "?" for the 1655 offer. The `dead` row on the Woolsey tag only keeps
-- a re-fired load quiet: the spawn is a no-op for a live tag but logs WARN
-- tag_already_live.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3152, 'SGC_W1 - 1654 done: Woolsey arrives with "?"', 'mission', 1654, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3152, 'mission_completed', '1654', 'player', false, 0),
  (3152, 'player_loaded', 'SGC_W1', 'player', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3152, 'mission_status', 1654, NULL, 'eq', 'completed', 0),
  (3152, 'mission_status', 1655, NULL, 'eq', 'not_active', 1),
  (3152, 'entity_tag_state', NULL, 'SGC_W1_Woolsey', 'eq', 'dead', 2),
  (3152, 'world', 58, NULL, 'eq', NULL, 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3152, 'spawn_entity', 47, 'SGC_W1_Woolsey',
   '{"x": 197.5, "y": 1.311, "z": 43.6, "heading": 0}', 0, 0),
  (3152, 'set_interaction_type', NULL, 'SGC_W1_Woolsey',
   '{"op": "|", "mask": 8388608}', 0, 1);
