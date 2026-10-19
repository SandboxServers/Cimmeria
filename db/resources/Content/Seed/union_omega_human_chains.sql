--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): Human
-- induction chains (Jackson). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Omega's id block is 63800-63999; this file's chain ids are 63820-63829. Each packet writes only below its own marker.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-07] chains 63820-63822 (provenance set by UO-07's block)
--
-- UO-07 [CIMMERIA_RECONSTRUCTION]; mission 1655, mission 1423, dialog 4641, map
-- 63800 and gate 6 are RESTORED_SOURCE. Three chains, in play order: 63820 binds
-- Jackson's delivery map (63800) on the first beam at S4940, 63821 binds it again
-- on a Command Center load at S4942 (a relog or a second beam), and 63822 is the
-- Accept on 4641 at S4942: it completes 1655, accepts 1423 and grants Beta's
-- gate (gate 6). Beta's receiver (23020) takes over from there (D-UO2).
-- F9: on the first beam both events see S4940, so the bind is keyed on the beam,
-- not on S4942 at player_loaded. Each bind runs remove_dialog_set (sort 0) before
-- add_dialog_set (sort 1), since add_dialog_set has no dedupe.
-- F5: remove_dialog_set (sort 0) runs before complete_mission (sort 1 on 63822),
-- because complete_mission awaits fire_mission_completed mid-list.
-- F21: no chain here completes an objective of 1655. UO-04's 63801 advances the
-- step on the beam; these chains complete 1655 only on the Accept.
-- Mission guard: every chain requires mission_status 1655 active, so the Accept
-- cannot fire again once UO-09's re-offer has completed 1655.
-- Human gate: archetype gte 1 and lte 4 (Asgard 5 and Free Jaffa 7 never match).

-- 63820 [CIMMERIA_RECONSTRUCTION]: the first beam (S4940) binds Jackson's delivery map for Humans.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63820, 'Union Omega - Human beam at 1655 S4940: bind Jackson (map 63800)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63820, 'teleport_in', '17', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63820, 'world',          80,   NULL,   'eq',  NULL,         0),
  (63820, 'mission_status', 1655, NULL,   'eq',  'active',     1),
  (63820, 'step_status',    1655, '4940', 'eq',  'active',     2),
  (63820, 'mission_status', 1423, NULL,   'eq',  'not_active', 3),
  (63820, 'archetype',      NULL, NULL,   'gte', '1',          4),
  (63820, 'archetype',      NULL, NULL,   'lte', '4',          5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63820, 'remove_dialog_set', 63800, NULL, '{"slot": 63800}',                     0, 0),
  (63820, 'add_dialog_set',    63800, NULL, '{"slot": 63800, "mission_id": 1655}', 0, 1);

-- 63821 [CIMMERIA_RECONSTRUCTION]: relog or second beam at S4942 rebinds Jackson's delivery map.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63821, 'Union Omega - Human Command Center load at 1655 S4942: bind Jackson (map 63800)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63821, 'player_loaded', 'Omega_Site_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63821, 'world',          80,   NULL,   'eq',  NULL,         0),
  (63821, 'mission_status', 1655, NULL,   'eq',  'active',     1),
  (63821, 'step_status',    1655, '4942', 'eq',  'active',     2),
  (63821, 'mission_status', 1423, NULL,   'eq',  'not_active', 3),
  (63821, 'archetype',      NULL, NULL,   'gte', '1',          4),
  (63821, 'archetype',      NULL, NULL,   'lte', '4',          5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63821, 'remove_dialog_set', 63800, NULL, '{"slot": 63800}',                     0, 0),
  (63821, 'add_dialog_set',    63800, NULL, '{"slot": 63800, "mission_id": 1655}', 0, 1);

-- 63822 [CIMMERIA_RECONSTRUCTION]: Accept on 4641 completes 1655, starts 1423 and grants Beta (gate 6).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63822, 'Union Omega - Human Accept on 4641 at 1655 S4942: complete 1655, accept 1423, gate 6', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63822, 'dialog_choice', '4641', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63822, 'world',          80,   NULL,   'eq',  NULL,         0),
  (63822, 'mission_status', 1655, NULL,   'eq',  'active',     1),
  (63822, 'step_status',    1655, '4942', 'eq',  'active',     2),
  (63822, 'mission_status', 1423, NULL,   'eq',  'not_active', 3),
  (63822, 'archetype',      NULL, NULL,   'gte', '1',          4),
  (63822, 'archetype',      NULL, NULL,   'lte', '4',          5);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63822, 'remove_dialog_set',      63800, NULL, '{"slot": 63800}', 0, 0),
  (63822, 'complete_mission',       1655,  NULL, '{}',              0, 1),
  (63822, 'accept_mission',         1423,  NULL, '{}',              0, 2),
  (63822, 'grant_stargate_address', 6,     NULL, '{}',              0, 3);

-- D-UO11 (HELD): if the SGC grants item 6786, add (63822, 'remove_item', 6786, NULL, '{"qty": 1}', 0, 4).
