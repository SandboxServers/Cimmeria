--
-- Union Omega (worlds 18 `Omega_Site` and 80 `Omega_Site_CmdCenter`): re-offer
-- chains (after an abandon). Campaign:
-- docs/analysis/union-zone-planning/omega-work-packets.md (packets UO-01 to UO-14).
--
-- Omega's id block is 63800-63999; this file's chain ids are 63840-63849. Each packet writes only below its own marker.
-- Provenance: each block states its own tag (RESTORED_SOURCE,
-- CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING).
--

SET search_path = resources, pg_catalog;

-- [UO-09] chains 63840-63845 (provenance set by UO-09's block)
--
-- UO-09 [CIMMERIA_RECONSTRUCTION]; missions 1655, 1423 and 1500, dialogs 4641,
-- 5025 and 6168, maps 63802 and 63803 are RESTORED_SOURCE (dialog 4641 and 5025
-- carry the single Accept button on their final screen, which is why the
-- Accept is keyed on dialog_choice). D-UO10: missions 1423 and 1500 are
-- can_abandon true, and an abandon removes the instance, so nothing would offer
-- the mission again after 1655 completes. The fix is a "?" re-offer from the
-- same NPC with the same shipped dialog.
--
-- Six chains, two families. 63840-63842 are Jackson's re-offer of The SGU
-- (mission 1423, map 63802, Accept 4641). 63843-63845 are Bra'tac's re-offer of
-- the Jaffa mission (mission 1500, map 63803, Accept 5025). Each family binds on
-- a Command Center load (player_loaded), on the abandon of its own mission
-- (mission_abandoned, which fires after the instance is removed, F22) and on the
-- Accept. The bind chains run remove_dialog_set (sort 0) before add_dialog_set
-- (sort 1), since add_dialog_set has no dedupe, so a re-fire never stacks a
-- second entry.
--
-- No gate grant: the player already holds gate 6 from 63822, and Bra'tac's
-- family grants nothing new. Re-offering 1655 itself is SGC's concern, not
-- Omega's (handoff note).
--
-- World gate: the abandon chains (63841, 63844) require world 80. An abandon in
-- world 23 (Beta) binds nothing; the next Command Center load (63840, 63843)
-- binds it.
--
-- Mission guard: every chain requires mission_status 1655 completed, so the
-- re-offer cannot fire while 1655 is still active. The delivery chains of
-- UO-07 and UO-08 require 1655 active, so the two families never both match one
-- event.
--
-- Human gate: archetype gte 1 and lte 4 (Asgard 5 and Free Jaffa 7 never match).
-- Jaffa gate: archetype eq 7.

-- 63840 [CIMMERIA_RECONSTRUCTION]: a Command Center load with 1655 done and 1423 not held re-offers The SGU (Jackson, map 63802).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63840, 'Union Omega - Human Command Center load, 1655 done, 1423 not held: bind Jackson re-offer (map 63802)', 'mission', 1423, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63840, 'player_loaded', 'Omega_Site_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63840, 'world',          80,   NULL, 'eq',  NULL,         0),
  (63840, 'mission_status', 1655, NULL, 'eq',  'completed',  1),
  (63840, 'mission_status', 1423, NULL, 'eq',  'not_active', 2),
  (63840, 'archetype',      NULL, NULL, 'gte', '1',          3),
  (63840, 'archetype',      NULL, NULL, 'lte', '4',          4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63840, 'remove_dialog_set', 63802, NULL, '{"slot": 63800}',                     0, 0),
  (63840, 'add_dialog_set',    63802, NULL, '{"slot": 63800, "mission_id": 1423}', 0, 1);

-- 63841 [CIMMERIA_RECONSTRUCTION]: abandoning The SGU in the Command Center re-binds Jackson's re-offer at once (world 23 binds nothing).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63841, 'Union Omega - Human abandon of 1423 in Command Center, 1655 done: bind Jackson re-offer (map 63802)', 'mission', 1423, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63841, 'mission_abandoned', '1423', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63841, 'world',          80,   NULL, 'eq',  NULL,         0),
  (63841, 'mission_status', 1655, NULL, 'eq',  'completed',  1),
  (63841, 'mission_status', 1423, NULL, 'eq',  'not_active', 2),
  (63841, 'archetype',      NULL, NULL, 'gte', '1',          3),
  (63841, 'archetype',      NULL, NULL, 'lte', '4',          4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63841, 'remove_dialog_set', 63802, NULL, '{"slot": 63800}',                     0, 0),
  (63841, 'add_dialog_set',    63802, NULL, '{"slot": 63800, "mission_id": 1423}', 0, 1);

-- 63842 [CIMMERIA_RECONSTRUCTION]: the re-offer's Accept on 4641 takes The SGU back. 63822 stays silent (its 1655-active guard).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63842, 'Union Omega - Human re-offer Accept on 4641, 1655 done, 1423 not held: accept 1423', 'mission', 1423, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63842, 'dialog_choice', '4641', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63842, 'world',          80,   NULL, 'eq',  NULL,         0),
  (63842, 'mission_status', 1655, NULL, 'eq',  'completed',  1),
  (63842, 'mission_status', 1423, NULL, 'eq',  'not_active', 2),
  (63842, 'archetype',      NULL, NULL, 'gte', '1',          3),
  (63842, 'archetype',      NULL, NULL, 'lte', '4',          4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63842, 'remove_dialog_set', 63802, NULL, '{"slot": 63800}', 0, 0),
  (63842, 'accept_mission',    1423,  NULL, '{}',              0, 1);

-- 63843 [CIMMERIA_RECONSTRUCTION]: a Command Center load with 1655 done and 1500 not held re-offers the Jaffa mission (Bra'tac, map 63803).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63843, 'Union Omega - Free Jaffa Command Center load, 1655 done, 1500 not held: bind Bra''tac re-offer (map 63803)', 'mission', 1500, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63843, 'player_loaded', 'Omega_Site_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63843, 'world',          80,   NULL, 'eq',  NULL,         0),
  (63843, 'mission_status', 1655, NULL, 'eq',  'completed',  1),
  (63843, 'mission_status', 1500, NULL, 'eq',  'not_active', 2),
  (63843, 'archetype',      NULL, NULL, 'eq',  '7',          3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63843, 'remove_dialog_set', 63803, NULL, '{"slot": 63801}',                     0, 0),
  (63843, 'add_dialog_set',    63803, NULL, '{"slot": 63801, "mission_id": 1500}', 0, 1);

-- 63844 [CIMMERIA_RECONSTRUCTION]: abandoning the Jaffa mission in the Command Center re-binds Bra'tac's re-offer at once.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63844, 'Union Omega - Free Jaffa abandon of 1500 in Command Center, 1655 done: bind Bra''tac re-offer (map 63803)', 'mission', 1500, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63844, 'mission_abandoned', '1500', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63844, 'world',          80,   NULL, 'eq',  NULL,         0),
  (63844, 'mission_status', 1655, NULL, 'eq',  'completed',  1),
  (63844, 'mission_status', 1500, NULL, 'eq',  'not_active', 2),
  (63844, 'archetype',      NULL, NULL, 'eq',  '7',          3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63844, 'remove_dialog_set', 63803, NULL, '{"slot": 63801}',                     0, 0),
  (63844, 'add_dialog_set',    63803, NULL, '{"slot": 63801, "mission_id": 1500}', 0, 1);

-- 63845 [CIMMERIA_RECONSTRUCTION]: the re-offer's Accept on 5025 takes the Jaffa mission back.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63845, 'Union Omega - Free Jaffa re-offer Accept on 5025, 1655 done, 1500 not held: accept 1500', 'mission', 1500, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63845, 'dialog_choice', '5025', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (63845, 'world',          80,   NULL, 'eq',  NULL,         0),
  (63845, 'mission_status', 1655, NULL, 'eq',  'completed',  1),
  (63845, 'mission_status', 1500, NULL, 'eq',  'not_active', 2),
  (63845, 'archetype',      NULL, NULL, 'eq',  '7',          3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (63845, 'remove_dialog_set', 63803, NULL, '{"slot": 63801}', 0, 0),
  (63845, 'accept_mission',    1500,  NULL, '{}',              0, 1);
