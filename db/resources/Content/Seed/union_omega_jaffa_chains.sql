--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): Jaffa
-- induction chains (Bra'tac). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Omega's id block is 63800-63999; this file's chain ids are 63830-63839. Each packet writes only below its own marker.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-08] chains 63830-63832 (provenance set by UO-08's block)
--
-- UO-08 [CIMMERIA_RECONSTRUCTION]; mission 1500, dialog 5025 and stargate gate 6
-- are RESTORED_SOURCE. Map 63801 (set 63800, template 63801) is UO-05's. Three
-- chains: 63830 binds Bra'tac on the Command Center beam at S4940, 63831 rebinds
-- him on a Command Center load at S4942 (relog or second beam), and 63832 is the
-- Accept on dialog 5025: it removes the bind, completes 1655, accepts 1500 (The
-- SGU) and grants gate 6. Archetype 7 (Free Jaffa) only; archetype 8 (Jaffa) is a
-- different faction and is excluded.
-- F21 guard: no chain here completes an objective of 1655. UO-04's 63801 advances
-- the step on the beam, so 1655 completes only on the Accept (63832).
-- Every bind runs remove_dialog_set (sort 0) before add_dialog_set (sort 1), since
-- add_dialog_set has no dedupe and a re-fire must not stack a second entry.
-- Every chain carries mission_status 1655 eq active (sort 1), so the re-offer
-- chains (UO-09) never meet a second bind or a second completion. All three also
-- carry mission_status 1500 eq not_active (sort 3), so a taken offer never rebinds
-- and cannot be accepted twice.

-- 63830 [CIMMERIA_RECONSTRUCTION]: the Command Center beam at S4940 binds Bra'tac (map 63801, slot 63801).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63830, 'Union Omega - Free Jaffa beam into the Command Center at 1655 S4940: bind Bra''tac (map 63801)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63830, 'teleport_in', '17', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63830, 'world',          80,   NULL,   'eq', NULL,         0),
  (63830, 'mission_status', 1655, NULL,   'eq', 'active',     1),
  (63830, 'step_status',    1655, '4940', 'eq', 'active',     2),
  (63830, 'mission_status', 1500, NULL,   'eq', 'not_active', 3),
  (63830, 'archetype',      NULL, NULL,   'eq', '7',          4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63830, 'remove_dialog_set', 63801, NULL, '{"slot": 63801}',                      0, 0),
  (63830, 'add_dialog_set',    63801, NULL, '{"slot": 63801, "mission_id": 1655}', 0, 1);

-- 63831 [CIMMERIA_RECONSTRUCTION]: a Command Center load at S4942 rebinds Bra'tac after a relog or a second beam.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63831, 'Union Omega - Free Jaffa Command Center load at 1655 S4942: rebind Bra''tac (map 63801)', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63831, 'player_loaded', 'Omega_Site_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63831, 'world',          80,   NULL,   'eq', NULL,         0),
  (63831, 'mission_status', 1655, NULL,   'eq', 'active',     1),
  (63831, 'step_status',    1655, '4942', 'eq', 'active',     2),
  (63831, 'mission_status', 1500, NULL,   'eq', 'not_active', 3),
  (63831, 'archetype',      NULL, NULL,   'eq', '7',          4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63831, 'remove_dialog_set', 63801, NULL, '{"slot": 63801}',                      0, 0),
  (63831, 'add_dialog_set',    63801, NULL, '{"slot": 63801, "mission_id": 1655}', 0, 1);

-- 63832 [CIMMERIA_RECONSTRUCTION]: Bra'tac's Accept on dialog 5025 at S4942 completes 1655, offers The SGU, grants gate 6.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63832, 'Union Omega - Free Jaffa Bra''tac dialog 5025 Accept at 1655 S4942: complete 1655, accept 1500, grant gate 6', 'mission', 1655, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63832, 'dialog_choice', '5025', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63832, 'world',          80,   NULL,   'eq', NULL,         0),
  (63832, 'mission_status', 1655, NULL,   'eq', 'active',     1),
  (63832, 'step_status',    1655, '4942', 'eq', 'active',     2),
  (63832, 'mission_status', 1500, NULL,   'eq', 'not_active', 3),
  (63832, 'archetype',      NULL, NULL,   'eq', '7',          4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63832, 'remove_dialog_set',      63801, NULL, '{"slot": 63801}', 0, 0),
  (63832, 'complete_mission',       1655,  NULL, '{}',              0, 1),
  (63832, 'accept_mission',         1500,  NULL, '{}',              0, 2),
  (63832, 'grant_stargate_address', 6,     NULL, '{}',              0, 3);
