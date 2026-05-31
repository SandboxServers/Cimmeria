-- Phase 1: Castle_CellBlock content chains
-- Missions: 622, 638, 639, 640, 641, 680, 681-687
--
-- Bug fixes baked in:
--   Bug 1: 'Preparation_ColMarshr' -> 'Preparation_ColMarsh' (typo in classic script)
--   Bug 2: step 2113 = mission 622, step 2114 = mission 638 (corrected IDs)
--
-- Chain ID ranges (allocated statically to avoid seed-order sensitivity):
--   Mission 622:  chains 1001-1010
--   Mission 638:  chains 1011-1030
--   Mission 639:  chains 1031-1040
--   Mission 640:  chains 1041-1050
--   Mission 641:  chains 1051-1070
--   Mission 680:  chains 1071-1080
--   Missions 681-687: chains 1081-1104
--   Mission 688:  chains 1105-1111
--   (next free: 1112)

SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 622 — Arm Yourself!
-- ============================================================

-- Chain 1001: zone entry when 622 not yet accepted → accept + show dialog
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1001, '622 - Zone load: accept mission', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1001, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1001, 'mission_status', 622, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1001, 'accept_mission',  622,  NULL, '{}', 0, 0),
  (1001, 'display_dialog',  2982, NULL, '{}', 0, 1),
  (1001, 'add_dialog_set',  5229, NULL, '{"slot": 14, "mission_id": 622}', 0, 2);

-- Chain 1002: zone entry when 622 already completed → play cinematic
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1002, '622 - Zone load: already completed → cinematic', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1002, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1002, 'mission_status', 622, NULL, 'eq', 'completed', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1002, 'play_sequence', 10000, NULL, '{}', 0, 0);

-- Chain 1003: player opens dialog 3995 (body loot) while step 2113 is
-- active → grant pistol to backpack + letter to mission inventory, advance
-- to step 80622 ("Equip the pistol"). The kismet sequence (10000) that
-- opens the stasis-room door and mission completion are deferred to
-- chain 1004, which fires when the player manually equips the pistol.
-- Routing the pistol through a manual equip exercises the bandolier
-- ammo / appearance / fire-animation paths that auto-equip-to-container-3
-- was bypassing.
--
-- Step 80622 is a Cimmeria-introduced step. The client picks up the
-- matching `<Steps StepID="80622">` row from a server-side override of
-- mission 622's `_622` entry in `CookedDataMissions.pak`, shipped to the
-- client via the per-key `InvalidKeys` channel of `onVersionInfo`. Without
-- the PAK override the client UI would silently skip the unknown step
-- and render either nothing or the next client-known step.
--
-- Re-loot guard: the gate `step_status 622 2113 = active` flips false the
-- moment we advance past 2113, so a second dialog open won't re-fire the
-- chain. Same shape as chain 1031 (Ambernol vial pickup).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1003, '622 - Dialog 3995 (loot body): grant items + advance to equip step', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1003, 'dialog_open', '3995', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1003, 'step_status', 622, '2113', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1003, 'set_interaction_type', NULL, 'ArmYourself_FrostBody', '{"op": "~", "mask": 4194304}', 0, 0),
  -- Pistol → backpack (container 1) instead of bandolier (container 3) so
  -- the player must equip it themselves. Letter (3730) keeps its container 0
  -- mission-inventory placement.
  (1003, 'add_item',     55,   NULL,    '{"container": 1, "qty": 1}', 0, 1),
  (1003, 'add_item',     3730, NULL,    '{"container": 0, "qty": 1}', 0, 2),
  (1003, 'advance_step', 622,  '80622', '{}',                          0, 3);

-- Chain 1004: player equips the pistol (item 55) while step 80622 is
-- active → play kismet sequence 10000 (opens the stasis-room door — the
-- only way out of the room) and complete mission 622. Gate on step 80622
-- blocks an early-equip path: the chain only fires once the body has
-- already been looted (chain 1003 is what advances us to step 80622).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1004, '622 - Equip pistol: open stasis door, complete mission', 'mission', 622, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1004, 'item_equipped', '55', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1004, 'step_status', 622, '80622', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1004, 'play_sequence',    10000, NULL, '{}', 0, 0),
  (1004, 'complete_mission', 622,   NULL, '{}', 0, 1);

-- ============================================================
-- MISSION 638 — Speak to Prisoner 329
-- ============================================================
--
-- Dialog ID convention used throughout Mission 638 (and the rest of
-- this seed):
--   2xxx / 3xxx prefix → Tau'ri / Human dialogs.
--   5xxx prefix        → Jaffa dialogs.
--
-- For Mission 638 specifically, the per-archetype mapping is:
--
--   Topic / step                        Human         Jaffa
--   "Free Prisoner 329" topic dialog    2300          5021
--   "Agree to escape" follow-up dialog  2299          5020
--   dialog_set_map row (set_id 603)     2794          5866
--
-- Chains 1011/1012 add the *correct* dialog_set_map row to slot 17 so
-- the prisoner only offers the topic dialog matching the player's
-- archetype. Chains 1018/1019 remove the same marker after the player
-- commits to the escape, so re-talking to the prisoner doesn't re-offer
-- the topic.
--
-- A swap here is a user-visible bug (a Tau'ri player gets the
-- "My symbiote will cure me" Jaffa dialog and vice versa) — issue #216.
-- The chain-replay tests in
-- `crates/services/src/cell/content/chain_replay_tests.rs`
-- (`mission_638_*` group) regression-guard the resolved-action shape
-- for both archetype branches.

-- Chain 1011: enter Region2 when 638 not active + non-Jaffa → accept + Human dialog set
-- dialog_set_map 2794 (set_id 603) → dialog 2300 (Human "Free Prisoner 329")
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1011, '638 - Region2 entry: accept (non-Jaffa)', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1011, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1011, 'mission_status', 638, NULL, 'eq', 'not_active', 0),
  (1011, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1011, 'accept_mission', 638, NULL, '{}', 0, 0),
  (1011, 'add_dialog_set', 2794, NULL, '{"slot": 17}', 0, 1);

-- Chain 1012: enter Region2 when 638 not active + Jaffa → accept + Jaffa dialog set
-- dialog_set_map 5866 (set_id 603) → dialog 5021 (Jaffa "Free Prisoner 329", mentions symbiote)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1012, '638 - Region2 entry: accept (Jaffa)', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1012, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1012, 'mission_status', 638, NULL, 'eq', 'not_active', 0),
  (1012, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1012, 'accept_mission', 638, NULL, '{}', 0, 0),
  (1012, 'add_dialog_set', 5866, NULL, '{"slot": 17}', 0, 1);

-- Chain 1013: enter Region2 always → system message 5040
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1013, '638 - Region2 entry: system message', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1013, 'enter_region', 'Castle_Cellblock.Region2', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1013, 'system_message', 5040, NULL, '{}', 0, 0);

-- Chain 1014: dialog choice 5021 (Jaffa talk to 329) while step 2114 active → advance to 2115 + enable door
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1014, '638 - Talk to Prisoner (Jaffa): advance step to 2115', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1014, 'dialog_choice', '5021', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1014, 'step_status', 638, '2114', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1014, 'advance_step', 638, '2115', '{}', 0, 0),
  (1014, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "|", "mask": 256}', 0, 1);

-- Chain 1015: dialog choice 2300 (Human talk to 329) while step 2114 active → advance to 2115 + enable door
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1015, '638 - Talk to Prisoner (Human): advance step to 2115', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1015, 'dialog_choice', '2300', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1015, 'step_status', 638, '2114', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1015, 'advance_step', 638, '2115', '{}', 0, 0),
  (1015, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "|", "mask": 256}', 0, 1);

-- Chain 1016: interact with door button while step 2115 active → Livewire minigame
--   on_victory_chains: 1017
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1016, '638 - Door button: start Livewire minigame', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1016, 'interact_tag', '329_CellDoorButton', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1016, 'step_status', 638, '2115', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1016, 'start_minigame', NULL, 'Livewire',
   '{"on_victory_chains": [1017]}', 0, 0);

-- Chain 1017: minigame victory (triggered by 1016 on_victory_chains) → advance step, play sequence, disable button
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1017, '638 - Livewire victory: advance to step 2116', 'mission', 638, true, 0);

-- no trigger row — invoked directly by the minigame callback in chain 1016
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1017, 'advance_step', 638, '2116', '{}', 0, 0),
  (1017, 'play_sequence', 1749, NULL, '{}', 0, 1),
  (1017, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "~", "mask": 256}', 0, 2);

-- Chain 1020: dialog choice 5021 (Jaffa) while step 2116 active → show follow-up dialog 5020
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1020, '638 - Post-minigame (Jaffa): show escape dialog', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1020, 'dialog_choice', '5021', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1020, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1020, 'display_dialog', 5020, NULL, '{}', 0, 0);

-- Chain 1021: dialog choice 2300 (Human) while step 2116 active → show follow-up dialog 2299
-- Per the dialog-id mapping at the top of this file (Mission 638 section),
-- 2299 is the *Human* "Agree to escape" follow-up (5020 is the Jaffa one).
-- The earlier archetype-routing fix corrected the *topic* dialog mapping
-- but missed this post-Livewire follow-up, which still routed Human
-- players through the Jaffa "my symbiote will cure me" dialog. Chain-
-- replay test in the mission_638_post_livewire_human group pins this
-- so it doesn't drift again.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1021, '638 - Post-minigame (Human): show escape dialog', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1021, 'dialog_choice', '2300', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1021, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1021, 'display_dialog', 2299, NULL, '{}', 0, 0);

-- Chain 1018: dialog choice 5020 (Jaffa agree escape) while step 2116 active → finish 638, accept 639
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1018, '638 - Agree to help (Jaffa): complete 638, accept 639', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1018, 'dialog_choice', '5020', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1018, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1018, 'display_dialog', 2298, NULL, '{}', 0, 0),
  (1018, 'accept_mission',  639, NULL, '{}', 0, 1),
  (1018, 'complete_mission', 638, NULL, '{}', 0, 2),
  (1018, 'remove_dialog_set', 5866, NULL, '{"slot": 17}', 0, 3);

-- Chain 1019: dialog choice 2299 (Human agree escape) while step 2116 active → finish 638, accept 639
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1019, '638 - Agree to help (Human): complete 638, accept 639', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1019, 'dialog_choice', '2299', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1019, 'step_status', 638, '2116', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1019, 'display_dialog', 2298, NULL, '{}', 0, 0),
  (1019, 'accept_mission',  639, NULL, '{}', 0, 1),
  (1019, 'complete_mission', 638, NULL, '{}', 0, 2),
  (1019, 'remove_dialog_set', 2794, NULL, '{"slot": 17}', 0, 3);

-- ============================================================
-- MISSION 639 — Find Ambernol
-- ============================================================

-- Chain 1031: enter Region11 while step 2117 active → advance to 2145
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1031, '639 - Enter Region11: advance step to 2145', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1031, 'enter_region', 'Castle_Cellblock.Region11', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1031, 'step_status', 639, '2117', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1031, 'advance_step', 639, '2145', '{}', 0, 0),
  (1031, 'set_interaction_type', NULL, 'ArmYourself_AmbernolVial', '{"op": "|", "mask": 1073741824}', 0, 1);

-- ============================================================
-- COVER-SYSTEM DEMO
-- ============================================================
--
-- Chain 9209 — proof-of-trigger that the OnPlayerEnteredCover wire
-- path works end-to-end (DB → loader → trigger match → executor).
-- Fires when ANY player enters ANY cover set inside Castle Cellblock,
-- gated on the prisoner-retrieval-unit (med-bay drone) mission being
-- active. The action is a counter bump — observable in the entity's
-- `counters` map and verifiable from the chain-replay test harness.
--
-- Chain id 9209 chosen to avoid collision with the 1xxx-5xxx range
-- used by authored mission content (1098 is taken by the "search
-- crate" chain in this same file at line ~1289).
--
-- Production med-bay gate (the user's stated use case — "gate the
-- drone attack on player crouching + entering cover") is the natural
-- next step: once cover_set_ids in the med-bay room are catalogued
-- from the generated extractor manifest, swap the wildcard NULL for
-- the specific set_id and add the BSF_CROUCHING state-flag condition
-- to gate on crouched-and-in-cover. v1 lands the trigger plumbing;
-- the per-room tuning is a content-author follow-up.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (9209, 'COVER DEMO: bump counter on player enter cover', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (9209, 'player_entered_cover', NULL, 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (9209, 'step_status', 639, '2145', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (9209, 'increment_counter', NULL, 'cover_demo_entered', '{"amount": 1}', 0, 0);

-- Chain 1032: interact with Ambernol vial while step 2145 active → pick up, destroy, aggro guard, advance
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1032, '639 - Interact vial: pick up ambernol, trigger guard', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1032, 'interact_tag', 'ArmYourself_AmbernolVial', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1032, 'step_status', 639, '2145', 'eq', 'active', 0);

-- Canonical Python ordering from FindAmbernol.py:21-35 + :151-159:
--   1. add_item 19          (inventory.pickedUpItem)
--   2. destroy_entity vial  (destroyCellEntity)
--   3. set_aggression 1     (drone.setAggression — durable behavior bit)
--   4. generate_threat 1000 (drone.threatGenerated — focuses drone on this player)
--   5. display_dialog 2297  (Net'an reaction VO)
--   6. play_sequence 10001  (cinematic / door / etc.)
--   7. advance_step 2144    (mission step transition)
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1032, 'add_item',        19,   NULL,                                '{"container": 0, "qty": 1}',     0, 0),
  (1032, 'destroy_entity',  NULL, 'ArmYourself_AmbernolVial',          '{}',                             0, 1),
  (1032, 'set_aggression',  NULL, 'ArmYourself_PrisonerRetrievalUnit', '{"level": 1}',                   0, 2),
  (1032, 'generate_threat', NULL, 'ArmYourself_PrisonerRetrievalUnit', '{"threat_level": 1000}',         0, 3),
  (1032, 'display_dialog',  2297, NULL,                                '{}',                             0, 4),
  (1032, 'play_sequence',   10001, NULL,                               '{}',                             0, 5),
  (1032, 'advance_step',    639,  '2144',                              '{}',                             0, 6);

-- Chain 1033: entity dead tag for guard (space-scoped) while step 2144 active → advance to 2343
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1033, '639 - Guard killed: advance step to 2343', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1033, 'entity_dead_tag', 'ArmYourself_PrisonerRetrievalUnit', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1033, 'step_status', 639, '2144', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1033, 'advance_step', 639, '2343', '{}', 0, 0);

-- Chain 1034: use item 19 (ambernol) while step 2343 active → complete 639, accept 640.
--   The chain is responsible for consumption via `remove_item`. Mirrors python
--   `FindAmbernol.py:115` which calls `inventory.removeItemByDesign(19, 1, False)`
--   from a per-mission script callback — pure `useItem` events fire without
--   consuming, and per-item handlers decide whether to remove the stack.
--   The previous global consume-on-use behavior was wrong for reusable items
--   (radios, multi-step "use on target" objectives) which silently lost stacks
--   on every use.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1034, '639 - Use ambernol: complete 639, accept 640', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1034, 'item_use', '19', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1034, 'step_status', 639, '2343', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Consume the vial first — the step-status condition is locked in by the
  -- time the chain fires, so subsequent actions can't double-trigger off the
  -- removal's wire updates.
  (1034, 'remove_item',      19,  NULL, '{"qty": 1}', 0, 0),
  (1034, 'complete_mission', 639, NULL, '{}',         0, 1),
  (1034, 'accept_mission',   640, NULL, '{}',         0, 2),
  -- Make the ring switch right-clickable with the Livewire icon so the player can hack it.
  -- The classic Python script (HackTheRings.py) didn't set this bit; the original Atrea editor
  -- presumably wired it implicitly off the Event_EntityInteract node. We do it explicitly here.
  (1034, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 256}', 0, 3);

-- ============================================================
-- MISSION 640 — Hack the Rings
-- ============================================================

-- Chain 1041: interact with ring switch while step 2120 active → Livewire
--   on_victory_chains: 1042
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1041, '640 - Ring switch: start Livewire', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1041, 'interact_tag', 'HackTheRings_Switch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1041, 'step_status', 640, '2120', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1041, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1042]}', 0, 0);

-- Chain 1042: Livewire victory → advance step to 2215, swap switch icon
-- Livewire → RingNetwork, and add the quest-highlight ring (the same cue
-- the player saw on the Ambernol vial) so it's obvious the next thing to
-- do is right-click the ring switch. Highlight cleared by chain 1043
-- when they actually use it; restored on login by chain 1046.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1042, '640 - Livewire victory: advance to 2215', 'mission', 640, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1042, 'advance_step', 640, '2215', '{}', 0, 0),
  (1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "~", "mask": 256}', 0, 1),
  (1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 32}', 0, 2),
  -- Quest-highlight ring (INT_MissionWorldObject = 2^30 = 1073741824).
  (1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 1073741824}', 0, 3);

-- Chain 1043: interact with ring switch while step 2215 active → trigger transporter (regionId=1)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1043, '640 - Ring switch (post-hack): trigger transport to regionId 1', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1043, 'interact_tag', 'HackTheRings_Switch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1043, 'step_status', 640, '2215', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1043, 'trigger_transporter', NULL, NULL, '{"regionId": 1}', 0, 0),
  -- Clear the quest highlight set by chain 1042 — the player has
  -- engaged with the ring, no need to keep the marker.
  (1043, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "~", "mask": 1073741824}', 0, 1);

-- Chain 1045: player_loaded + step 2120 active → restore Livewire bit on the switch.
--   Needed because interaction_type flags are NOT persisted on the entity across server restarts.
--   Without this, a player who logged out mid-mission-640 would find the switch unclickable.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1045, '640 - Restore Livewire bit on login (step 2120)', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1045, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1045, 'step_status', 640, '2120', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1045, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 1046: player_loaded + step 2215 active → restore both the
-- RingNetwork icon AND the quest highlight on the switch. Same
-- restart-resilience rationale as 1045 — the highlight set by chain
-- 1042 doesn't persist across server restarts.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1046, '640 - Restore RingNetwork bit on login (step 2215)', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1046, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1046, 'step_status', 640, '2215', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1046, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 32}', 0, 0),
  (1046, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "|", "mask": 1073741824}', 0, 1);

-- Chain 1044: teleport_in regionId=2 while 640 active → complete 640, show ColMarsh indicator
--   BUG FIX: this correctly sets the 'Preparation_ColMarsh' tag (no trailing 'r')
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1044, '640 - Teleport to ringside: complete 640, enable ColMarsh', 'mission', 640, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1044, 'teleport_in', '2', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1044, 'mission_status', 640, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1044, 'complete_mission', 640, NULL, '{}', 0, 0),
  -- BUG FIX: correct tag is 'Preparation_ColMarsh' (classic script had 'Preparation_ColMarshr')
  (1044, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "|", "mask": 8388608}', 0, 1),
  -- Clear RingNetwork bit set by chain 1042 once we've used the rings.
  (1044, 'set_interaction_type', NULL, 'HackTheRings_Switch',
   '{"op": "~", "mask": 32}', 0, 2);

-- ============================================================
-- MISSION 641 — Preparation
-- BUG FIX: subscribe on correct tag 'Preparation_ColMarsh' not 'Preparation_ColMarshr'
-- ============================================================

-- Chain 1051: interact ColMarsh while mission 641 not yet accepted + archetype != 8 (non-sci) → display dialog 4001
--   Gating on `mission_status 641 eq not_active` rather than `step_status 641 2121 eq not_active`:
--   step 2121 is also "not_active" once the player has advanced past it (e.g. picked up the P90 and
--   is on step 3563), which used to make this chain re-fire the briefing dialog and let chain 1053
--   re-accept the mission, resetting progress to step 2121 in a loop.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1051, '641 - Interact ColMarsh (non-sci): show dialog 4001', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1051, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1051, 'mission_status', 641, NULL, 'eq', 'not_active', 0),
  (1051, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1051, 'display_dialog', 4001, NULL, '{}', 0, 0);

-- Chain 1052: interact ColMarsh while mission 641 not yet accepted + archetype == 8 (sci) → display dialog 5022
--   See chain 1051 for why this gates on mission_status rather than step_status.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1052, '641 - Interact ColMarsh (sci): show dialog 5022', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1052, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1052, 'mission_status', 641, NULL, 'eq', 'not_active', 0),
  (1052, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1052, 'display_dialog', 5022, NULL, '{}', 0, 0);

-- Chain 1053: dialog choice 4001 (non-sci accepts briefing) → accept mission 641, highlight P90 locker
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1053, '641 - Dialog choice 4001: accept mission', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1053, 'dialog_choice', '4001', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1053, 'accept_mission', 641, NULL, '{}', 0, 0),
  -- Mark the P90 locker as a quest world object so the client renders the
  -- attention-drawing highlight while step 2121 is active. Cleared in chain
  -- 1055 when the player picks the P90 up; restored on login by chain 1062.
  (1053, 'set_interaction_type', NULL, 'Preparation_SMG1A',
   '{"op": "|", "mask": 1073741824}', 0, 1),
  -- Clear ColMarsh's mission-available bit (set by chain 1044 on
  -- mission-640-complete). Once mission 641 is accepted, the briefing
  -- dialog has been delivered and the icon should clear so it doesn't
  -- linger as a stale "talk to me" indicator. Restored on a server
  -- restart by chain 1063 (gated on 640-complete + 641-not_active).
  (1053, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 2);

-- Chain 1054: dialog choice 5022 (sci accepts briefing) → accept mission 641, highlight P90 locker
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1054, '641 - Dialog choice 5022: accept mission', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1054, 'dialog_choice', '5022', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1054, 'accept_mission', 641, NULL, '{}', 0, 0),
  -- See chain 1053 for the highlight + ColMarsh-clear rationale.
  (1054, 'set_interaction_type', NULL, 'Preparation_SMG1A',
   '{"op": "|", "mask": 1073741824}', 0, 1),
  (1054, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 2);

-- Chain 1055: interact SMG1A while step 2121 active → grant P90 to
-- backpack, advance to step 80641 ("Equip the P90"), clear locker
-- highlight. Step 80641 is a Cimmeria-introduced step; the matching
-- `<Steps StepID="80641">` row is shipped to the client via the
-- mission_overrides patch on `_641` in `CookedDataMissions.pak` + the
-- per-key `InvalidKeys` channel of `onVersionInfo`. The advance to
-- step 3563 ("Speak to Col. Marsh") is deferred to chain 1066, which
-- fires when the player manually equips the P90. Marsh stays
-- non-talkable across the pickup→equip window because chains 1051/1052
-- gate on `mission_status = not_active` (mission already accepted) and
-- chains 1056/1057 gate on step 3563 (we're still on 80641).
--
-- Re-pickup guard: the gate `step_status 641 2121 = active` flips false
-- the moment we advance to 80641, blocking duplicate-grant on a second
-- interact.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1055, '641 - Pick up SMG1A: grant P90 to backpack, advance to equip step', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1055, 'interact_tag', 'Preparation_SMG1A', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1055, 'step_status', 641, '2121', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- P90 → backpack (container 1) instead of bandolier (container 3).
  (1055, 'add_item',     21,  NULL,    '{"container": 1, "qty": 1}', 0, 0),
  (1055, 'advance_step', 641, '80641', '{}',                          0, 1),
  -- Clear the quest-item highlight on the locker the moment the P90 is
  -- grabbed. Mirrors how chain 1042 toggles HackTheRings_Switch off the
  -- minigame icon as soon as the gating action completes.
  (1055, 'set_interaction_type', NULL, 'Preparation_SMG1A',
   '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chain 1066: player equips the P90 (item 21) while step 80641 is active
-- → advance to step 3563 ("Speak to Col. Marsh") and re-set Marsh's
-- mission-available marker. This is the gate that keeps Marsh
-- non-talkable between P90 pickup and equip: chains 1056/1057 fire only
-- on step 3563.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1066, '641 - Equip P90: advance to talk-to-Marsh step', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1066, 'item_equipped', '21', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1066, 'step_status', 641, '80641', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1066, 'advance_step', 641, '3563', '{}', 0, 0),
  -- Set ColMarsh's mission-available marker so the player has a visual cue
  -- to return to him. Cleared again by chain 1058/1059 once the briefing
  -- completes; restored on login by chain 1064 if the player logs out at
  -- 3563.
  (1066, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "|", "mask": 8388608}', 0, 1);

-- Chain 1062: player_loaded + step 2121 active → restore P90 locker highlight on login.
--   Same restart-resilience pattern as chains 1045/1046 for HackTheRings_Switch:
--   interaction_type flags are not persisted on the entity across server restarts,
--   so a player who logged out mid-mission-641 would otherwise find the locker
--   without the quest-item highlight that chain 1053/1054 originally set.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1062, '641 - Restore SMG1A highlight on login (step 2121)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1062, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1062, 'step_status', 641, '2121', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1062, 'set_interaction_type', NULL, 'Preparation_SMG1A',
        '{"op": "|", "mask": 1073741824}', 0, 0);

-- Chain 1063: player_loaded + 640 completed + 641 not_active → restore ColMarsh
-- mission-available bit on login. Chain 1044 sets the bit with `once: true`
-- when the player teleports in post-rings, so a player who logs out at the
-- "talk to ColMarsh to start mission 641" boundary loses the icon on restart
-- (interaction_type flags don't persist on the entity across server restarts).
-- Same restart-resilience pattern as chain 1062 (SMG1A) and chains 1045/1046
-- (HackTheRings_Switch).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1063, '641 - Restore ColMarsh mission-available bit on login (640 done, 641 not started)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1063, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1063, 'mission_status', 640, NULL, 'eq', 'completed', 0),
  (1063, 'mission_status', 641, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1063, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
        '{"op": "|", "mask": 8388608}', 0, 0);

-- Chain 1064: player_loaded + step 3563 active → restore ColMarsh's
-- "talk to me" marker (re-set after P90 pickup by chain 1055). Without this,
-- a player who logs out at step 3563 boundary would log back in to a Marsh
-- with no visual cue that they need to interact with him for the briefing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1064, '641 - Restore ColMarsh marker on login (step 3563)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1064, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1064, 'step_status', 641, '3563', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1064, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
        '{"op": "|", "mask": 8388608}', 0, 0);

-- Chain 1065: player_loaded + step 3564 active → restore Terminal's
-- INT_MinigameLivewire (256) marker (originally set by chain 1058/1059
-- on dialog accept). Mirrors chains 1062/1064 for SMG1A and ColMarsh
-- respectively. The mask matches what the runtime chains set, not the
-- broader INT_MissionWorldObject — the Terminal needs the wrench/minigame
-- cursor, not the round quest-object pulse. See chain 1058's comment.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1065, '641 - Restore Terminal Livewire marker on login (step 3564)', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1065, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1065, 'step_status', 641, '3564', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1065, 'set_interaction_type', NULL, 'Preparation_Terminal',
        '{"op": "|", "mask": 256}', 0, 0);

-- Chain 1056: interact template 'Col Marsh (pet)' while step 3563 active + non-sci → dialog 3999
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1056, '641 - Talk to Col Marsh pet (non-sci): show dialog 3999', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1056, 'interact_template', 'Col Marsh (pet)', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1056, 'step_status', 641, '3563', 'eq', 'active', 0),
  (1056, 'archetype', NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1056, 'display_dialog', 3999, NULL, '{}', 0, 0);

-- Chain 1057: interact template 'Col Marsh (pet)' while step 3563 active + sci → dialog 5023
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1057, '641 - Talk to Col Marsh pet (sci): show dialog 5023', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1057, 'interact_template', 'Col Marsh (pet)', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1057, 'step_status', 641, '3563', 'eq', 'active', 0),
  (1057, 'archetype', NULL, NULL, 'eq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1057, 'display_dialog', 5023, NULL, '{}', 0, 0);

-- Chain 1058: dialog choice 3999 (non-sci briefed) → advance step 3564,
-- swap quest markers from ColMarsh to the Terminal he's sending us to.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1058, '641 - Dialog choice 3999: advance step 3564', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1058, 'dialog_choice', '3999', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1058, 'advance_step', 641, '3564', '{}', 0, 0),
  -- Briefing's done, clear ColMarsh's "talk to me" marker.
  (1058, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 1),
  -- Mark the Terminal as a Livewire minigame entry point. The quest
  -- highlight ring (INT_MissionWorldObject = 1073741824) is the wrong
  -- cue here — the Terminal isn't an inert quest object, it's the
  -- minigame trigger; the player needs to know to right-click it to
  -- start the hack. INT_MinigameLivewire (256) renders the wrench cursor
  -- the rest of the cellblock arc uses for HackTheRings_Switch.
  -- Cleared by chain 1060 when they engage; restored on login by chain 1065.
  (1058, 'set_interaction_type', NULL, 'Preparation_Terminal',
   '{"op": "|", "mask": 256}', 0, 2);

-- Chain 1059: dialog choice 5023 (sci briefed) → advance step 3564.
-- Same marker churn as 1058; see that chain for the rationale.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1059, '641 - Dialog choice 5023: advance step 3564', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1059, 'dialog_choice', '5023', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1059, 'advance_step', 641, '3564', '{}', 0, 0),
  (1059, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
   '{"op": "~", "mask": 8388608}', 0, 1),
  -- INT_MinigameLivewire (256) — see chain 1058's comment for rationale.
  (1059, 'set_interaction_type', NULL, 'Preparation_Terminal',
   '{"op": "|", "mask": 256}', 0, 2);

-- Chain 1060: interact terminal while step 3564 active → Livewire
--   on_victory_chains: 1061
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1060, '641 - Terminal: start Livewire', 'mission', 641, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1060, 'interact_tag', 'Preparation_Terminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1060, 'step_status', 641, '3564', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1060, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1061]}', 0, 0),
  -- Clear the Terminal's INT_MinigameLivewire (256) marker the moment the
  -- minigame starts; player has engaged with it, no need to keep the cue
  -- once they're in the minigame. Same shape as chain 1042 clearing the
  -- Livewire icon off HackTheRings_Switch on victory.
  (1060, 'set_interaction_type', NULL, 'Preparation_Terminal',
   '{"op": "~", "mask": 256}', 0, 1);

-- Chain 1061: Livewire victory → display final dialog, complete 641, accept 680
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1061, '641 - Livewire victory: complete 641, accept 680', 'mission', 641, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1061, 'display_dialog', 3998, NULL, '{}', 0, 0),
  (1061, 'complete_mission', 641, NULL, '{}', 0, 1),
  (1061, 'accept_mission',   680, NULL, '{}', 0, 2),
  -- Make the second ring switch right-clickable with the RingNetwork icon. Same gap as 640:
  -- EscapeTheCellblock.py subscribes to the interact tag but never sets interactionType.
  (1061, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 32}', 0, 3),
  -- Highlight the ring switch as the next quest objective so the player
  -- has a visual cue. Cleared on teleport_in (chain 1072) when they
  -- actually use it; restored on login by chain 1074.
  (1061, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 1073741824}', 0, 4);

-- ============================================================
-- MISSION 680 — Escape the Cellblock
-- ============================================================

-- Chain 1074: player_loaded + step 2344 active → restore both the RingNetwork
-- icon AND the quest highlight on the second switch.
--   Same restart-resilience pattern as chains 1045/1046 for the first ring switch.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1074, '680 - Restore RingSwitch bits on login (step 2344)', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1074, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1074, 'step_status', 680, '2344', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1074, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 32}', 0, 0),
  -- Restore the quest highlight too (originally set by chain 1061 on
  -- mission-680 acceptance).
  (1074, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "|", "mask": 1073741824}', 0, 1);

-- Chain 1071: interact ring switch while step 2344 active → trigger transporter to regionId=2
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1071, '680 - Ring switch: trigger transport (region 2)', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1071, 'interact_tag', 'Preparation_RingSwitch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1071, 'step_status', 680, '2344', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1071, 'trigger_transporter', NULL, NULL, '{"regionId": 2}', 0, 0);

-- Chain 1072: teleport_in regionId=3 → advance step to 2345
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1072, '680 - Teleport to topside: advance step 2345', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1072, 'teleport_in', '3', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1072, 'advance_step', 680, '2345', '{}', 0, 0),
  (1072, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "~", "mask": 32}', 0, 1),
  -- Clear the quest highlight too — player has used the rings, no
  -- longer need the "go here" indicator. Pairs with the set in chain 1061.
  (1072, 'set_interaction_type', NULL, 'Preparation_RingSwitch',
   '{"op": "~", "mask": 1073741824}', 0, 2);

-- Chain 1073: enter Region9 when 681 not yet active → complete 680, accept 681
-- Mission 680 ("Escape the Cellblock") is the umbrella mission whose
-- "Lockdown! Find another way out of the Castle!" step (2345) ends when
-- the player reaches the corridor outside the topside ring room. There
-- is no other completion trigger for 680 — without this, it stays open
-- forever even as the 681-687 sub-missions advance.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1073, '680 - Enter Region9: complete 680, accept 681', 'mission', 680, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1073, 'enter_region', 'Castle_Cellblock.Region9', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1073, 'mission_status', 681, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1073, 'complete_mission', 680, NULL, '{}', 0, 0),
  (1073, 'accept_mission',   681, NULL, '{}', 0, 1);

-- ============================================================
-- MISSIONS 681-687 — Hallway Controller chain
-- Counter-based kill tracking for mess hall
-- ============================================================

-- Chain 1081: enter Region3 (exit) when 681 completed and 682 not active → accept 682
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1081, '681→682 - Leave Region3: accept 682', 'mission', 681, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1081, 'exit_region', 'Castle_Cellblock.Region3', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1081, 'mission_status', 681, NULL, 'eq', 'completed', 0),
  (1081, 'mission_status', 682, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1081, 'accept_mission', 682, NULL, '{}', 0, 0);

-- Chain 1082: enter Region4 when 683 completed and 684 not active → accept 684
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1082, '683→684 - Enter Region4: accept 684', 'mission', 683, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1082, 'enter_region', 'Castle_Cellblock.Region4', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1082, 'mission_status', 683, NULL, 'eq', 'completed', 0),
  (1082, 'mission_status', 684, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1082, 'accept_mission', 684, NULL, '{}', 0, 0);

-- Chain 1083: enter Region5 when 685 completed and 686 not active → accept 686
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1083, '685→686 - Enter Region5: accept 686', 'mission', 685, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1083, 'enter_region', 'Castle_Cellblock.Region5', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1083, 'mission_status', 685, NULL, 'eq', 'completed', 0),
  (1083, 'mission_status', 686, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1083, 'accept_mission', 686, NULL, '{}', 0, 0);

-- Chain 1084: enter Region6 when 686 completed and 687 not active → accept 687
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1084, '686→687 - Enter Region6: accept 687', 'mission', 686, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1084, 'enter_region', 'Castle_Cellblock.Region6', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1084, 'mission_status', 686, NULL, 'eq', 'completed', 0),
  (1084, 'mission_status', 687, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1084, 'accept_mission', 687, NULL, '{}', 0, 0);

-- ─────────────────────────────────────────────────────────────────────
-- Chains 1085-1094: kill-counter chains for missions 681-686.
--
-- Each Hallway/MessHall mission has TWO chains: one increment chain
-- (fires on each guard kill, bumps the counter) and one completion
-- chain (fires when the counter reaches the per-mission threshold).
--
-- Spawn-list reality (from `resources.spawnlist`):
--   MessHall_Guard1, MessHall_Guard2 → 2 spawns total
--   Hallway01_Guard                  → 1 spawn
--   Hallway02_Guard                  → 1 spawn
--   Hallway03_Guard                  → 1 spawn
--   Hallway04_Guard                  → 1 spawn
--   Hallway05_Guard1, Hallway05_Guard2 → 2 spawns total
--
-- The mission→tag wiring follows mission `history_text`, NOT the
-- previous (broken) chain comments. Mission 681 displays as "Mess Hall
-- Controller" so it tracks MessHall_Guard*. Mission 682 displays as
-- "Hallway01 Controller" so it tracks Hallway01_Guard. Etc.
--
-- The previous chain version was off-by-one across the whole sequence
-- (681 tracked Hallway01, 682 tracked Hallway02, …, 686 tracked
-- MessHall_Guard which doesn't exist as an exact tag). It also had
-- thresholds set to 3 (and 5 for messhall) which made every counter
-- impossible to reach against the actual spawn counts, AND was missing
-- increment chains entirely for missions 683-686.
--
-- Tag matching is exact-string (see content-engine triggers.rs
-- `OnEntityDeath` matcher). Rooms with two suffixed guards
-- (Hallway05_Guard1/2, MessHall_Guard1/2) get two increment chains
-- both feeding the same counter.
--
-- Chain-replay tests in `crates/services/src/cell/content/
-- chain_replay_tests.rs` (`mission_681_*` and `mission_686_*` groups)
-- pin both the per-tag increment dispatch and the threshold-reached
-- completion so the wiring can't drift.
-- ─────────────────────────────────────────────────────────────────────

-- ── Mission 681 — Mess Hall Controller (2 guards) ──

-- Chain 1085: kill MessHall_Guard1 → increment messhall_kills
-- Priority 1 (vs completion chain 1087 at priority 0) so the increment
-- runs before the completion's `reset_counter` action on the same
-- entity_dead event. Equal priority would leave ordering undefined and
-- the reset could land first, then the increment re-introduces a
-- non-zero counter that bleeds into the next mission acceptance.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1085, '681 - Kill MessHall_Guard1: increment messhall_kills', 'mission', 681, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1085, 'entity_dead_tag', 'MessHall_Guard1', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1085, 'mission_status', 681, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1085, 'increment_counter', NULL, 'messhall_kills', '{"amount": 1}', 0, 0);

INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on)
VALUES (1085, 'messhall_kills', 2, 'mission_complete');

-- Chain 1086: kill MessHall_Guard2 → increment messhall_kills (same counter)
-- Priority 1 — same ordering rationale as chain 1085.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1086, '681 - Kill MessHall_Guard2: increment messhall_kills', 'mission', 681, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1086, 'entity_dead_tag', 'MessHall_Guard2', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1086, 'mission_status', 681, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1086, 'increment_counter', NULL, 'messhall_kills', '{"amount": 1}', 0, 0);

-- Chain 1087: messhall_kills >= 1 (target-1) → complete 681, accept 682
--
-- Triggered by either MessHall guard death; condition gates on counter.
-- The `gte 1` (target_value 2 minus 1) is load-bearing: chain conditions
-- evaluate against the PRE-increment counter value because resolve runs
-- before execute. On the second kill, counter is at 1 (from the first
-- kill's increment chain), `gte 1` passes, and this completion fires
-- alongside the second increment. See `executor.rs::IncrementCounter`
-- for the full ordering note.
--
-- The two trigger rows OR together — content-engine loader materializes
-- one Chain per trigger row so either MessHall_Guard1 or
-- MessHall_Guard2 death satisfies the trigger. (Pre-fix, the loader
-- silently dropped the 2nd row, leaving Guard2 deaths to never fire
-- this chain.)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1087, '681 - Kill counter reached: complete 681, accept 682', 'mission', 681, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (1087, 'entity_dead_tag', 'MessHall_Guard1', 'space', false, 0),
  (1087, 'entity_dead_tag', 'MessHall_Guard2', 'space', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1087, 'mission_status', 681, NULL, 'eq', 'active', 0),
  (1087, 'counter', NULL, 'messhall_kills', 'gte', '1', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1087, 'complete_mission', 681, NULL, '{}', 0, 0),
  (1087, 'accept_mission',   682, NULL, '{}', 0, 1),
  (1087, 'reset_counter', NULL, 'messhall_kills', '{}', 0, 2);

-- ── Mission 682 — Hallway01 Controller (1 guard) ──

-- Chain 1088: kill Hallway01_Guard → complete 682, accept 683
-- Single-guard hallway: no counter needed, the kill itself completes.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1088, '682 - Kill Hallway01_Guard: complete 682, accept 683', 'mission', 682, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1088, 'entity_dead_tag', 'Hallway01_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1088, 'mission_status', 682, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1088, 'complete_mission', 682, NULL, '{}', 0, 0),
  (1088, 'accept_mission',   683, NULL, '{}', 0, 1);

-- ── Mission 683 — Hallway02 Controller (1 guard) ──

-- Chain 1089: kill Hallway02_Guard → complete 683, accept 684
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1089, '683 - Kill Hallway02_Guard: complete 683, accept 684', 'mission', 683, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1089, 'entity_dead_tag', 'Hallway02_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1089, 'mission_status', 683, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1089, 'complete_mission', 683, NULL, '{}', 0, 0),
  (1089, 'accept_mission',   684, NULL, '{}', 0, 1);

-- ── Mission 684 — Hallway03 Controller (1 guard) ──

-- Chain 1090: kill Hallway03_Guard → complete 684, accept 685
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1090, '684 - Kill Hallway03_Guard: complete 684, accept 685', 'mission', 684, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1090, 'entity_dead_tag', 'Hallway03_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1090, 'mission_status', 684, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1090, 'complete_mission', 684, NULL, '{}', 0, 0),
  (1090, 'accept_mission',   685, NULL, '{}', 0, 1);

-- ── Mission 685 — Hallway04 Controller (1 guard) ──

-- Chain 1091: kill Hallway04_Guard → complete 685, accept 686
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1091, '685 - Kill Hallway04_Guard: complete 685, accept 686', 'mission', 685, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1091, 'entity_dead_tag', 'Hallway04_Guard', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1091, 'mission_status', 685, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1091, 'complete_mission', 685, NULL, '{}', 0, 0),
  (1091, 'accept_mission',   686, NULL, '{}', 0, 1);

-- ── Mission 686 — Hallway05 Controller (2 guards) ──

-- Chain 1092: kill Hallway05_Guard1 → increment hallway05_kills
-- Priority 1 (vs completion chain 1094 at priority 0) — see chain 1085's
-- ordering note for the rationale.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1092, '686 - Kill Hallway05_Guard1: increment hallway05_kills', 'mission', 686, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1092, 'entity_dead_tag', 'Hallway05_Guard1', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1092, 'mission_status', 686, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1092, 'increment_counter', NULL, 'hallway05_kills', '{"amount": 1}', 0, 0);

INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on)
VALUES (1092, 'hallway05_kills', 2, 'mission_complete');

-- Chain 1093: kill Hallway05_Guard2 → increment hallway05_kills
-- Priority 1 — same ordering rationale as chain 1092.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1093, '686 - Kill Hallway05_Guard2: increment hallway05_kills', 'mission', 686, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1093, 'entity_dead_tag', 'Hallway05_Guard2', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1093, 'mission_status', 686, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1093, 'increment_counter', NULL, 'hallway05_kills', '{"amount": 1}', 0, 0);

-- Chain 1094: hallway05_kills >= 1 (target-1) → complete 686, accept 687
-- See chain 1087 for the resolve/execute ordering note that motivates
-- the `target - 1` threshold on counter completion conditions.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1094, '686 - Kill counter reached: complete 686, accept 687', 'mission', 686, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (1094, 'entity_dead_tag', 'Hallway05_Guard1', 'space', false, 0),
  (1094, 'entity_dead_tag', 'Hallway05_Guard2', 'space', false, 1);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1094, 'mission_status', 686, NULL, 'eq', 'active', 0),
  (1094, 'counter', NULL, 'hallway05_kills', 'gte', '1', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1094, 'complete_mission', 686, NULL, '{}', 0, 0),
  (1094, 'accept_mission',   687, NULL, '{}', 0, 1),
  (1094, 'reset_counter', NULL, 'hallway05_kills', '{}', 0, 2);

-- ============================================================
-- MISSION 687 — Aftermath
--
-- Final mission of the Castle_Cellblock escape. Two steps:
--   2354 — "Search the crate for any useful items."
--          Right-click `Cellblock_WoodenCrate` → archetype-gated reward
--          (Tau'ri stealth set + knife / Jaffa armor + serpent staff)
--          → advance to step 2355.
--   2355 — "Eliminate the guards in the barracks."
--          Three NID Guards tagged `Barracks_Guard1/2/3` (re-tagged from
--          previously-untagged spawn_ids 25/26/36, see
--          db/resources/Worlds/Seed/spawnlist.sql).
--          Counter `barracks_kills` reaches the `target - 1 = 2`
--          threshold on the third kill → complete 687.
--
-- Reference: python/cell/missions/Castle_CellBlock/Aftermath.py covers
-- step 2354 only; step 2355 has no python source (the original game
-- shipped that step uncompleted). The barracks-kill completion is
-- therefore a new design call — three guards already spawn in the room
-- behind the wall from the crate, so we re-tag rather than insert new
-- spawns.
-- ============================================================

-- Chain 1097: mission 687 accepted → highlight Cellblock_WoodenCrate
-- as a quest world object so the right-click cursor renders the
-- attention-pulse. Mirrors how chain 1053 highlights Preparation_SMG1A
-- on mission 641 accept. The matching `mission_accepted` trigger fires
-- from `crates/services/src/cell/content/executor.rs::Action::AcceptMission`
-- right after the mission state is committed (engine extension landed
-- alongside this chain).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1097, '687 - Mission accepted: highlight wooden crate', 'mission', 687, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1097, 'mission_accepted', '687', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1097, 'set_interaction_type', NULL, 'Cellblock_WoodenCrate',
        '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

-- Chain 1098: search crate (Tau'ri / non-Jaffa) → display dialog 3942,
-- grant 5-piece Covert Stealth set + Combat Knife to backpack, advance
-- to step 2355, clear crate highlight. Items go to container 1
-- (backpack) per Aftermath.py — the original `inventory.addItem(1, ...)`
-- path. Player learns to equip themselves rather than auto-equipping;
-- earlier items in the cellblock arc were auto-handled.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1098, '687 - Search crate (non-Jaffa): grant stealth set', 'mission', 687, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1098, 'interact_tag', 'Cellblock_WoodenCrate', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1098, 'step_status', 687, '2354', 'eq', 'active', 0),
  (1098, 'archetype',   NULL, NULL, 'neq', '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1098, 'display_dialog', 3942, NULL,   '{}',                         0, 0),
  (1098, 'add_item',       3347, NULL,   '{"qty": 1, "container": 1}', 0, 1),  -- Covert Stealth Helmet
  (1098, 'add_item',       3359, NULL,   '{"qty": 1, "container": 1}', 0, 2),  -- Covert Stealth Vest
  (1098, 'add_item',       3372, NULL,   '{"qty": 1, "container": 1}', 0, 3),  -- Covert Stealth Pants
  (1098, 'add_item',       3387, NULL,   '{"qty": 1, "container": 1}', 0, 4),  -- Covert Stealth Gloves
  (1098, 'add_item',       3401, NULL,   '{"qty": 1, "container": 1}', 0, 5),  -- Covert Stealth Boots
  (1098, 'add_item',       3325, NULL,   '{"qty": 1, "container": 1}', 0, 6),  -- Combat Knife
  (1098, 'advance_step',   687,  '2355', '{}',                         0, 7),
  (1098, 'set_interaction_type', NULL, 'Cellblock_WoodenCrate',
   '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 8);

-- Chain 1099: search crate (Jaffa) → display dialog 3943, grant
-- Armored Prison Jacket + Serpent Staff to backpack, advance step,
-- clear crate highlight. Same container-1 reasoning as 1098.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1099, '687 - Search crate (Jaffa): grant prison jacket + serpent staff', 'mission', 687, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1099, 'interact_tag', 'Cellblock_WoodenCrate', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1099, 'step_status', 687, '2354', 'eq', 'active', 0),
  (1099, 'archetype',   NULL, NULL, 'eq',  '8', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1099, 'display_dialog', 3943, NULL,   '{}',                         0, 0),
  (1099, 'add_item',       3482, NULL,   '{"qty": 1, "container": 1}', 0, 1),  -- Armored Prison Jacket
  (1099, 'add_item',       2797, NULL,   '{"qty": 1, "container": 1}', 0, 2),  -- Serpent Staff
  (1099, 'advance_step',   687,  '2355', '{}',                         0, 3),
  (1099, 'set_interaction_type', NULL, 'Cellblock_WoodenCrate',
   '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 4);

-- ── Mission 687 step 2355 — Eliminate barracks guards (3 guards) ──
--
-- Same shape as chain 1085-1087 (Mess Hall, 2 guards): one increment
-- chain per guard at priority 1, one completion chain at priority 0
-- with the multi-trigger OR shape. Step gating is on 2355 specifically
-- so a guard kill while step 2354 is active doesn't bleed counter
-- progress into the next step. Three guards → target_value 3, gte 2
-- (target - 1) per the resolve/execute ordering note at chain 1087.

-- Chain 1100: kill Barracks_Guard1 → increment barracks_kills
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1100, '687 - Kill Barracks_Guard1: increment barracks_kills', 'mission', 687, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1100, 'entity_dead_tag', 'Barracks_Guard1', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1100, 'mission_status', 687, NULL,   'eq', 'active', 0),
  (1100, 'step_status',    687, '2355', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1100, 'increment_counter', NULL, 'barracks_kills', '{"amount": 1}', 0, 0);

INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on)
VALUES (1100, 'barracks_kills', 3, 'mission_complete');

-- Chain 1101: kill Barracks_Guard2 → increment barracks_kills
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1101, '687 - Kill Barracks_Guard2: increment barracks_kills', 'mission', 687, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1101, 'entity_dead_tag', 'Barracks_Guard2', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1101, 'mission_status', 687, NULL,   'eq', 'active', 0),
  (1101, 'step_status',    687, '2355', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1101, 'increment_counter', NULL, 'barracks_kills', '{"amount": 1}', 0, 0);

-- Chain 1102: kill Barracks_Guard3 → increment barracks_kills
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1102, '687 - Kill Barracks_Guard3: increment barracks_kills', 'mission', 687, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1102, 'entity_dead_tag', 'Barracks_Guard3', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1102, 'mission_status', 687, NULL,   'eq', 'active', 0),
  (1102, 'step_status',    687, '2355', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1102, 'increment_counter', NULL, 'barracks_kills', '{"amount": 1}', 0, 0);

-- Chain 1103: barracks_kills threshold reached on third kill → complete 687.
-- Three trigger rows OR together (one per guard tag) so any of the
-- three deaths can be the trigger. Counter `gte 2` (target - 1)
-- because chain conditions evaluate against the PRE-increment counter
-- value — see chain 1087 for the full ordering note.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1103, '687 - Kill counter reached: complete 687', 'mission', 687, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (1103, 'entity_dead_tag', 'Barracks_Guard1', 'space', false, 0),
  (1103, 'entity_dead_tag', 'Barracks_Guard2', 'space', false, 1),
  (1103, 'entity_dead_tag', 'Barracks_Guard3', 'space', false, 2);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1103, 'mission_status', 687, NULL,   'eq',  'active', 0),
  (1103, 'step_status',    687, '2355', 'eq',  'active', 1),
  (1103, 'counter',        NULL, 'barracks_kills', 'gte', '2', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1103, 'complete_mission', 687, NULL, '{}', 0, 0),
  (1103, 'reset_counter',    NULL, 'barracks_kills', '{}', 0, 1);

-- Chain 1104: player_loaded + step 2354 active → restore crate highlight
-- on login. interaction_type flags don't persist across server
-- restarts, so a player who logs out mid-Aftermath at step 2354 would
-- otherwise log back in to an unhighlighted crate. Mirrors chains
-- 1062/1064/1065 (SMG1A / ColMarsh / Terminal restore patterns).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1104, '687 - Restore crate highlight on login (step 2354)', 'mission', 687, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1104, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1104, 'step_status', 687, '2354', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1104, 'set_interaction_type', NULL, 'Cellblock_WoodenCrate',
        '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

-- ============================================================
-- MISSION 688 — Secure the Armory
--
-- Sequential follow-up to 687 (Aftermath). Two steps:
--   2356 (canonical) — "Secure the Castle's main armory for Op-CORE."
--                      Pre-terminal: chain 1106 highlights
--                      `Cellblock_TerminalX` (spawn 34, the dedicated
--                      armory terminal — distinct from the
--                      Preparation_Terminal used in mission 641). Has
--                      one required objective (2734 "Use the terminal
--                      to open the armory doors") and one optional
--                      (4647 "Eliminate the NID guards"). On terminal
--                      interact, chain 1107 advances to step 80688.
--   80688 (Cimmeria) — "Use the ring transport to escape." Inserted via
--                      MissionOverride (mirrors the equip-from-inventory
--                      pattern used for missions 622/641). Without this
--                      second step, chain 1107's `complete_objective 688/2734` would
--                      auto-complete the mission via
--                      `cell::missions::complete_objective`'s
--                      all-required-done check (only one required obj),
--                      which kills chain 1109's step gate before the
--                      player can use the ring switch.
--
-- The optional objective (4647) is paired with `Cellblock_ArmoryGuard1`
-- (spawn 27, re-tagged from previously-untagged; only y=24-floor
-- untagged NID Guard left after Aftermath consumed spawns 25/26/36 for
-- `Barracks_Guard1/2/3`). Counter `armory_kills` is cosmetic — the
-- objective is `is_optional=true` so the kill doesn't gate progression.
--
-- The mission climax is interacting with `Cellblock_ArmoryRingSwitch`
-- (spawn 79, re-tagged from `Cellblock_FakeRingSwitch`). Chain 1109
-- both completes the mission and triggers the new region 33 ring
-- transport. The runtime mission gate on
-- `ring_transport_regions.required_mission_id=688` (region 33) is
-- belt-and-suspenders for direct method-call paths; the chain gate
-- (chain 1109's `step_status 688/80688` condition) is the primary
-- check on the happy path.
--
-- Chain ID range: 1105-1111.
-- ============================================================

-- Chain 1105: mission 687 completed → auto-accept mission 688.
-- Mirrors the auto-progression shape of chain 1094 (686 complete →
-- 687 accept) and chain 1087 (681 complete → 682 accept), but uses
-- the dedicated `mission_completed` trigger so we don't have to mirror
-- the source chain's trigger row(s). See `loader/trigger.rs::mission_completed`.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1105, '688 - Auto-accept on 687 complete', 'mission', 688, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1105, 'mission_completed', '687', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1105, 'accept_mission', 688, NULL, '{}', 0, 0);

-- Chain 1106: mission 688 accepted → highlight `Cellblock_TerminalX` so
-- the right-click cursor renders the attention-pulse for objective 2734.
-- Same shape as chain 1097 (687 accept → highlight Cellblock_WoodenCrate).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1106, '688 - Mission accepted: highlight armory terminal', 'mission', 688, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1106, 'mission_accepted', '688', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1106, 'set_interaction_type', NULL, 'Cellblock_TerminalX',
        '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

-- Chain 1107: interact with terminal while step 2356 active → advance
-- to step 80688, clear terminal highlight, and highlight
-- `Cellblock_ArmoryRingSwitch`. `advance_step` internally completes
-- step 2356's active objectives (including obj 2734) without going
-- through the public `complete_objective` helper, so the
-- all-required-done auto-complete check isn't triggered and mission
-- 688 stays MISSION_ACTIVE until chain 1109 fires.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1107, '688 - Interact armory terminal: advance to step 80688, swap highlights', 'mission', 688, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1107, 'interact_tag', 'Cellblock_TerminalX', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1107, 'step_status', 688, '2356', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1107, 'advance_step',         688,  '80688', '{}', 0, 0),
  (1107, 'set_interaction_type', NULL, 'Cellblock_TerminalX',
   '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 1),
  (1107, 'set_interaction_type', NULL, 'Cellblock_ArmoryRingSwitch',
   '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 2);

-- Chain 1108: kill `Cellblock_ArmoryGuard1` while step 2356 active →
-- increment `armory_kills` counter. Optional objective 4647 doesn't gate
-- the mission, so this counter is purely for content-replay test
-- pinning — content authors can later add a "killed all the guards"
-- chain if obj 4647 becomes required.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1108, '688 - Kill Cellblock_ArmoryGuard1: increment armory_kills', 'mission', 688, true, 1);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1108, 'entity_dead_tag', 'Cellblock_ArmoryGuard1', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1108, 'mission_status', 688, NULL,   'eq', 'active', 0),
  (1108, 'step_status',    688, '2356', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1108, 'increment_counter', NULL, 'armory_kills', '{"amount": 1}', 0, 0);

INSERT INTO content_counters (chain_id, counter_name, target_value, reset_on)
VALUES (1108, 'armory_kills', 1, 'mission_complete');

-- Chain 1109: interact with ring switch while step 80688 active →
-- complete mission 688, clear ring highlight, then cross-world
-- teleport directly to Castle's arrival platform.
--
-- We use `cross_world_teleport` instead of `trigger_transporter`
-- because the ring-transport ceremony has nothing to animate on this
-- route: the canonical `GLB-RingTransporterBase` kismet prefab actor
-- doesn't exist on either side at the right location. The Cellblock-
-- side platform (spawn 79) is set-decorated geometry without a wired
-- prefab; the Castle-side platform is also set-decorated only. Routing
-- through the ring FSM would either play no animation or play an
-- existing ring's animation at the wrong world position. Skipping the
-- ceremony entirely is cleaner — player right-clicks the switch,
-- mission completes, world transition happens, player arrives on the
-- Castle platform with no ring sequence. (Documented in the prerelease
-- known-issues list.)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1109, '688 - Interact armory ring (step 80688): complete mission, cross-world teleport to Castle', 'mission', 688, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1109, 'interact_tag', 'Cellblock_ArmoryRingSwitch', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1109, 'step_status', 688, '80688', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1109, 'complete_mission', 688, NULL, '{}', 0, 0),
  (1109, 'set_interaction_type', NULL, 'Cellblock_ArmoryRingSwitch',
   '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 1),
  -- Coords pinned in-game to the visible Castle ring platform
  -- (Castle-00090004 map debug HUD readout). target_key carries the
  -- world name; params carries the spawn x/y/z.
  (1109, 'cross_world_teleport', NULL, 'Castle',
   '{"x": 466.365, "y": 70.397, "z": 991.466}', 0, 2);

-- Chain 1110: player_loaded Castle_CellBlock + step 2356 active →
-- restore terminal highlight on login. interaction_type flags don't
-- persist across server restarts (same restart-resilience pattern as
-- chains 1045/1046/1062/1064/1065/1104).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1110, '688 - Restore terminal highlight on login (step 2356 active)', 'mission', 688, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1110, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1110, 'step_status', 688, '2356', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1110, 'set_interaction_type', NULL, 'Cellblock_TerminalX',
        '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

-- Chain 1111: player_loaded Castle_CellBlock + step 80688 active →
-- restore ring switch highlight on login. Same restart-resilience
-- rationale as chain 1110.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1111, '688 - Restore ring switch highlight on login (step 80688 active)', 'mission', 688, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1111, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1111, 'step_status', 688, '80688', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1111, 'set_interaction_type', NULL, 'Cellblock_ArmoryRingSwitch',
        '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);
