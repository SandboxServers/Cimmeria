-- SGC_W1 Union continuation: 1655 Free Jaffa (packet US-33, chains 3166-3169).
-- Campaign: docs/analysis/union-sgc-continuation/ (index:
-- docs/analysis/union-zone-planning/sgc-work-packets.md).
-- Every chain comment carries one provenance label: RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
SET search_path = resources, pg_catalog;

-- Chain 3166: Woolsey dismisses the liaison (Free Jaffa, 5855).
-- CIMMERIA_RECONSTRUCTION (H9, F6, D-US11 default): 5855 is shown and the press accepts (never keyed: fails R1); no letter item for the Free Jaffa; Harriman's beat is US-32 3160-3165.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3166, 'SGC_W1 - 1655 Woolsey dismisses the liaison (Free Jaffa, 5855)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3166, 'interact_tag', 'SGC_W1_Woolsey', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3166, 'mission_status', 1654, NULL, 'eq', 'completed', 0),
  (3166, 'mission_status', 1655, NULL, 'eq', 'not_active', 1),
  (3166, 'archetype', NULL, NULL, 'eq', '7', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3166, 'display_dialog', 5855, NULL, '{}', 0, 0),
  (3166, 'accept_mission', 1655, NULL, '{}', 0, 1),
  (3166, 'set_interaction_type', NULL, 'SGC_W1_Woolsey', '{"op": "~", "mask": 8388608}', 0, 2),
  (3166, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "|", "mask": 16777216}', 0, 3);

-- Chain 3167: press Hammond on 4943 (Free Jaffa) → his message for Bra'tac, dialog 5875 (RESTORED_SOURCE text).
-- CIMMERIA_RECONSTRUCTION: 5875 has no buttons, so it is closed with dialog_choice -1 (chain 3168).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3167, 'SGC_W1 - 1655 Hammond''s message for Bra''tac (Free Jaffa, 5875)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3167, 'interact_tag', 'SGCW1_GenHammond', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3167, 'step_status', 1655, '4943', 'eq', 'active', 0),
  (3167, 'archetype', NULL, NULL, 'eq', '7', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3167, 'display_dialog', 5875, NULL, '{}', 0, 0);

-- Chain 3168: close 5875 advances to 4944, Hammond's "!" goes and Harriman's "!" comes. No letter (D-US11 default).
-- CIMMERIA_RECONSTRUCTION (H-O6): the step_status row makes it one-shot; the Free Jaffa gets no item 6786.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3168, 'SGC_W1 - 1655 5875 closed: to Harriman (no letter)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3168, 'dialog_choice', '5875', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3168, 'step_status', 1655, '4943', 'eq', 'active', 0),
  (3168, 'archetype', NULL, NULL, 'eq', '7', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3168, 'advance_step', 1655, '4944', '{}', 0, 0),
  (3168, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "~", "mask": 16777216}', 0, 1),
  (3168, 'set_interaction_type', NULL, 'SGC_W1_Harriman', '{"op": "|", "mask": 16777216}', 0, 2);

-- Chain 3169: relog on 4943 → Hammond's "!" is back (Free Jaffa).
-- CIMMERIA_RECONSTRUCTION (H9): the relog restores the marker 3167 would have shown.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3169, 'SGC_W1 - relog on 4943: Hammond "!" (Free Jaffa)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (3169, 'player_loaded', 'SGC_W1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3169, 'step_status', 1655, '4943', 'eq', 'active', 0),
  (3169, 'archetype', NULL, NULL, 'eq', '7', 1),
  (3169, 'world', 58, NULL, 'eq', NULL, 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (3169, 'set_interaction_type', NULL, 'SGCW1_GenHammond', '{"op": "|", "mask": 16777216}', 0, 0);
