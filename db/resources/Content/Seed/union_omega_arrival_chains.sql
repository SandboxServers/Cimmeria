--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): arrival
-- chains (the Command Center beam, Modi). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Omega's id block is 63800-63999; this file's chain ids are 63801-63819. Each packet writes only below its own marker.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-04] chains 63801, 63810, 63811 (provenance set by UO-04's block)
--
-- UO-04 [CIMMERIA_RECONSTRUCTION]; mission 1655, dialog 6168 and map 7015 are
-- RESTORED_SOURCE. Three chains, in play order: 63810 binds Modi's shipped
-- Shutdown topic on hub entry at S4940, 63811 clears that bind when his dialog
-- closes, and 63801 is the ring beam into the Command Center that advances
-- S4940 to S4942 (D-UO3).
-- F21 guard: no chain here completes an objective of 1655. O5645 and O5646 are
-- optional, and a step whose objectives are all optional completes the mission on
-- the first objective completed. S4940 also lists required O5647 and O5644, which
-- would hold it open today; the guard keeps Omega safe if that changes. Omega
-- advances the step only, and the mission completes on the Accept.
-- Every bind runs remove_dialog_set (sort 0) before add_dialog_set (sort 1), since
-- add_dialog_set has no dedupe and a re-fire must not stack a second entry.

-- 63810 [CIMMERIA_RECONSTRUCTION]: hub entry at S4940 binds Modi's shipped Shutdown topic.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63810, 'Union Omega - hub entry at 1655 S4940: bind Modi (map 7015)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63810, 'player_loaded', 'Omega_Site', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63810, 'world',       18,   NULL,   'eq', NULL,     0),
  (63810, 'step_status', 1655, '4940', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63810, 'remove_dialog_set', 7015, NULL, '{"slot": 65}',                     0, 0),
  (63810, 'add_dialog_set',    7015, NULL, '{"slot": 65, "mission_id": 1655}', 0, 1);

-- 63811 [CIMMERIA_RECONSTRUCTION]: closing Modi's 6168 clears his bind. F21: it completes no objective.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63811, 'Union Omega - Modi dialog 6168 closed at 1655 S4940: clear his bind only', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63811, 'dialog_choice', '6168', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63811, 'world',       18,   NULL,   'eq', NULL,     0),
  (63811, 'step_status', 1655, '4940', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63811, 'remove_dialog_set', 7015, NULL, '{"slot": 65}', 0, 0);

-- 63801 [CIMMERIA_RECONSTRUCTION, D-UO3]: the ring beam into the Command Center proves arrival; S4940 -> S4942.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63801, 'Union Omega - beam into the Command Center (pad 17): 1655 S4940 -> S4942', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63801, 'teleport_in', '17', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63801, 'world',       80,   NULL,   'eq', NULL,     0),
  (63801, 'step_status', 1655, '4940', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63801, 'remove_dialog_set', 7015, NULL,   '{"slot": 65}', 0, 0),
  (63801, 'advance_step',      1655, '4942', '{}',           0, 1);
