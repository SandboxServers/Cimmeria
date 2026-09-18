-- Castle (World 8) content chains — mission 701 "Reinforce Copplemann"
--
-- Packets CA01 (arrival + Sgt. Gerschon handoff) and CA03 (mission 701
-- body, steps 2399 → 2400 → 2401 → 2421). Campaign ledger:
-- docs/analysis/castle-rebuild/ (README.md decisions, audit.md
-- "Mission 701 Port Table", work-packets.md CA01/CA03).
--
-- Source of truth: deprecated/python/cell/spaces/Castle.py (recovered
-- Atrea level script). Every chain below names its evidence class:
--   RECOVERED_SCRIPT — a 1:1 port of a Castle.py node
--   RECONSTRUCTION   — no recovered logic; built from original DB data
--   NEW CONTENT      — neither; authored for this rebuild
--
-- Chain ID ranges (allocated in work-packets.md#worker-input-and-ownership;
-- this ledger reserves 1201-1400 for Castle, which is free — the Cellblock
-- file ends at 1112 and effect chains start at 2001):
--   Mission 701 (CA01, arrival + Gerschon):  1201-1230
--   Mission 701 (CA03, body + relog restore): 1231-1260
-- Sibling files castle_702_704_chains.sql and castle_706_708_chains.sql
-- take 1261-1400 and are authored by other packets — do not add them here.
--
-- ============================================================
-- KNOWN GAPS THE COORDINATOR MUST TRACK (all documented in
-- docs/analysis/castle-rebuild/worknotes/m701.md)
-- ============================================================
--
-- GAP 1 — dialog_set_map row 3062 does not load today (defect B3).
-- Castle.py binds interaction-set row 3062 for BOTH the Gerschon offer
-- and the Copplemann in-progress topic. Row 3062 has dialog_id NULL, and
-- `load_dialog_set_maps` (crates/services/src/cell/spawner/dialogs.rs:45)
-- drops every NULL-dialog row, so `add_dialog_set 3062` is a warn + no-op
-- (executor/dialog.rs:172-177). Entity templates 48 (Copplemann) and 149
-- (Gerschon) both carry `interaction_type = 0` and `static_interaction_sets
-- = '{}'`, so the per-player bind is the ONLY source of the client's
-- interaction bit: until sibling packet CA02 widens
-- `DialogSetMapEntry.dialog_id` to Option<i32>, neither NPC is clickable
-- and chains 1202/1203/1231/1233 cannot fire in-client. The chains are
-- authored on 3062 per decision D-CA03 (widened bind) and the coordinator's
-- dispatch note; the replay guards assert resolved actions and are
-- unaffected either way.
--
-- If CA02 closes negative (a flag-only bind is impossible on the wire),
-- exactly these rows change to the sibling ladder — nothing else in this
-- file moves:
--     chain 1201  add_dialog_set 3062 slot 149  ->  3059  (dialog 2572, flags 8388608  INT_AStoryMissionAvaliable "?")
--     chain 1204  remove_dialog_set 3062 slot 149 -> 3059
--     chain 1205  remove_dialog_set 3062 slot 149 -> 3059
--     chain 1204  add_dialog_set 3062 slot 48   ->  3061  (dialog 2574, flags 16777216 INT_AStoryMissionActive "!")
--     chain 1205  add_dialog_set 3062 slot 48   ->  3061
--     chain 1234  remove_dialog_set 3062 slot 48 -> 3061
--     chain 1240  add_dialog_set 3062 slot 48   ->  3061
--     chain 1241  add_dialog_set 3062 slot 48   ->  3061
-- The turn-in rows (3063) already load and do not change. Note that the
-- sibling ladder is arguably MORE correct than the Python: 3062's own flag
-- is 16777216 ("!" mission-active), which is the wrong glyph over an NPC
-- whose mission has not been accepted yet; 3059 carries 8388608 ("?"
-- mission-available). That is a fidelity argument for CA02, not against it.
--
-- GAP 2 (FIXED in this packet) — chain 1205's `display_dialog 5862`.
-- `dialog::display` (executor/dialog.rs) resolves the wire EntityId from
-- params["target_entity_id"], then the player's
-- `last_interaction_target`, then the monologue cache, else warn+bail.
-- `fire_dialog_choice` stamps no target_entity_id, and
-- `last_interaction_target` used to be written ONLY inside
-- `interactions::dispatch::handle_interact`, which
-- cell_methods/player/interaction/interact.rs skips whenever a content
-- chain already handled the interact — which chain 1203 does. Dialog 5862
-- is not a monologue (screens 93218/93220/93222 carry speaker_id 2499,
-- Moh'katan), so the action used to warn and return and the Jaffa radio
-- call never played. Fixed here by pinning `last_interaction_target`
-- before the content-chain dispatch in interact.rs, mirroring python's
-- `SGWPlayer.interact()` which pins first; guarded by
-- `chain_handled_interact_pins_target_for_a_later_chain_dialog`. 5862 now
-- renders with Gerschon as the portrait-lookup entity and Moh'katan's own
-- per-screen speaker ids driving the lines. The same fix is what lets ANY
-- follow-up chain (dialog_choice, minigame victory, deferred drain)
-- display an NPC-speaker dialog — see docs/content/content-engine.md §4.
--
-- GAP 3 — dialog 2576's "Take Missions" button covers only 3 of its 5
-- screens (dialog_screen_buttons rows 2240-2242 → screens 96821/96822/
-- 96823; screens 96824/96825 have none). A player who reads to the last
-- screen gets no button and raises no dialog_choice, so the turn-in
-- (1237/1238/1239) silently does not fire. Same class as the dialog 5861
-- Accept-button gap already filed under D-CA13 (audit.md:38). Per the
-- packet scope this is FILED, not fixed — do not edit dialog data here.
--
-- ============================================================
-- Engine facts these chains rely on (verified against this tree)
-- ============================================================
--  * world_id 8 is named 'Castle' (worlds.sql), so `player_loaded`'s
--    event_key is 'Castle'.
--  * `dialog_choice` matches on dialog_id only; button_id reaches the
--    dispatcher but has no authorable condition
--    (content-engine/src/triggers/matching.rs:136-138). One chain per
--    dialog id is therefore the only available shape.
--  * `fire_dialog_choice` does NOT populate `archetype` into the context
--    (event_dispatch/dialog.rs), so an archetype condition on a
--    dialog_choice chain can never match. The Human/Jaffa split therefore
--    lives on the `interact_tag` chains (1202/1203), and the two accept
--    chains (1204/1205) are keyed by dialog id instead. Same shape as
--    Cellblock chains 1018-1021.
--  * `advance_step` is unconditional and only one step is ever current
--    (missions/progression.rs), so the 2399/2400/2421 interact gates on
--    `Castle_Coppleman` are mutually exclusive and can never co-resolve.
--  * `MissionInstance::complete()` (crates/entity/src/missions.rs:76)
--    moves current_step_id into completed_steps, so `step_status 2421 eq
--    active` self-gates the turn-in to exactly once.
--  * Minigame victory chains are fired by `fire_chain_by_id`, which calls
--    `get_chain_actions` directly and evaluates NO conditions and ignores
--    `content_chains.enabled` — so the step gate must live on the launcher
--    (1233), and `enabled = false` is not a kill switch for chain 1234.
--  * Deferred actions (`content_actions.delay_ms > 0`) are queued on
--    SpaceManager and drained by the cell tick; the queue is scrubbed by
--    `destroy_entity`/`disconnect_entity`. Dying in Castle does NOT scrub
--    it: `resolve_respawn_target` returns a World 8 respawner, so respawn
--    takes the same-world in-place branch
--    (cell_methods/player/combat/respawn.rs:152+) and never destroys the
--    entity. The paths that DO scrub (logout, cross-world hop) are all
--    followed by a `player_loaded`, which chain 1242 re-arms from.

SET search_path = resources, pg_catalog;

-- ============================================================
-- CA01 — Castle arrival and the Sgt. Gerschon handoff (1201-1205)
-- ============================================================
--
-- The player reaches Castle from the Cellblock via chain 1109
-- (castle_cellblock_chains.sql), which completes mission 688 and
-- cross-world teleports to (466.365, 70.397, 991.466) — the ring
-- platform. Sgt. Gerschon (spawn 112, template 149, tag
-- 'Castle_SgtGerschon') stands at (429.638, 70.111, 996.556): 37.08 units
-- away and essentially level with the arrival point (dy = 0.29). That is
-- far outside MAX_INTERACT_DISTANCE (5.0, interactions/dispatch/mod.rs:25)
-- but a short walk on flat ground, so the player lands on the platform and
-- walks to him rather than arriving inside him. Verified by computation,
-- not by an in-client session — UAT M1 confirms he is actually visible and
-- reachable from the landing spot.

-- Chain 1201 [RECOVERED_SCRIPT] — Castle.py `loaded` → n13_trigger_In.
-- Zone load with 701 never accepted → bind the Gerschon offer topic to
-- template 149. See GAP 1: this is a no-op until CA02 lands.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1201, '701 - Zone load: bind Gerschon offer topic (701 not accepted)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1201, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1201, 'mission_status', 701, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1201, 'add_dialog_set', 3062, NULL, '{"slot": 149, "mission_id": 701}', 0, 0);

-- Chain 1202 [RECOVERED_SCRIPT] — Castle.py `entity.interact.tag::
-- Castle_SgtGerschon` → displayDialog(NPC, 2573) when step 2399 is not
-- active. `not_active` (rather than `mission_status 701 eq not_active`)
-- is the Python's own gate and also covers the "accepted but somehow
-- back at the offer" case.
--
-- Human/Tau'ri branch. The archetype split is D-CA13: `archetype neq 8`
-- is every non-Jaffa archetype.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1202, '701 - Gerschon interact (Human): offer dialog 2573', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1202, 'interact_tag', 'Castle_SgtGerschon', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1202, 'step_status', 701, '2399', 'eq',  'not_active', 0),
  (1202, 'archetype',   NULL, NULL,  'neq', '8',          1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1202, 'display_dialog', 2573, NULL, '{}', 0, 0);

-- Chain 1203 [NEW CONTENT, D-CA13] — Jaffa branch of the same interact.
-- Dialog 5861 is original data (dialogs.sql:9925) with no dialog_set_map
-- row of its own, so it is displayed directly rather than bound. Its
-- Accept button is present on screens 96782-96786 and absent on
-- 96787-96789 (audit.md:38) — filed, not fixed. The accept itself rides
-- on `dialog_choice 5861` (chain 1205), which fires regardless of the
-- Accept affordance, so the missing buttons are cosmetic here.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1203, '701 - Gerschon interact (Jaffa): offer dialog 5861', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1203, 'interact_tag', 'Castle_SgtGerschon', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1203, 'step_status', 701, '2399', 'eq', 'not_active', 0),
  (1203, 'archetype',   NULL, NULL,  'eq', '8',          1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1203, 'display_dialog', 5861, NULL, '{}', 0, 0);

-- Chain 1204 [RECOVERED_SCRIPT] — Castle.py `dialog.choice::2573`:
-- accept 701, drop the Gerschon bind, bind the Copplemann topic to
-- template 48. Action order mirrors the Python's dialogChoiceCb
-- (Castle.py:253-262), where the addDialog on 48 is nested inside the
-- successful removeDialog on 149.
--
-- The `mission_status 701 eq not_active` gate is required of every chain
-- carrying `accept_mission` (.github/instructions/content-chains.instructions.md).
-- It is also the no-re-accept guard: a second click on Gerschon after
-- accepting finds 701 active and resolves nothing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1204, '701 - Dialog 2573 choice (Human): accept 701, rebind topic to Copplemann', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1204, 'dialog_choice', '2573', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1204, 'mission_status', 701, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1204, 'accept_mission',    701,  NULL, '{}',                            0, 0),
  (1204, 'remove_dialog_set', 3062, NULL, '{"slot": 149}',                 0, 1),
  (1204, 'add_dialog_set',    3062, NULL, '{"slot": 48, "mission_id": 701}', 0, 2);

-- Chain 1205 [NEW CONTENT, D-CA13] — Jaffa equivalent of 1204, plus the
-- Moh'katan radio call (5862). The 5862 action needs the
-- `last_interaction_target` pin fix that ships with this packet (GAP 2
-- above); without it the radio call warns and never opens.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1205, '701 - Dialog 5861 choice (Jaffa): accept 701, rebind topic, Mohkatan radio', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1205, 'dialog_choice', '5861', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1205, 'mission_status', 701, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1205, 'accept_mission',    701,  NULL, '{}',                            0, 0),
  (1205, 'remove_dialog_set', 3062, NULL, '{"slot": 149}',                 0, 1),
  (1205, 'add_dialog_set',    3062, NULL, '{"slot": 48, "mission_id": 701}', 0, 2),
  (1205, 'display_dialog',    5862, NULL, '{}',                            0, 3);

-- ============================================================
-- CA03 — Mission 701 body, steps 2399 → 2400 → 2401 → 2421 (1231-1243)
-- ============================================================
--
-- Capt. Copplemann is spawn 87, template 48, tag 'Castle_Coppleman'
-- (single 'n' — that is the spelling in both spawnlist.sql:127 and
-- Castle.py:55; the prose spells her "Copplemann"). She is static at
-- (352.692, 70.272, 952.320) and never moves: decision D-CA02 option A.
--
-- What option A drops from Castle.py, and why (audit.md "Where The Spec
-- Is Wrong Or Superseded"): the Python's dialog-2575 node created a
-- second template-48 entity at (354.419, 70.272, 952.801), hid the
-- static one with setVisible(False), walked the clone 63 units to
-- (358.809, 70.156, 889.724) and re-bound the topic in the arrival
-- callback. None of those three primitives works here — `set_visible` on
-- an NPC is routed to the entity's own session and dropped, and is undone
-- by the AoI create packet's unconditional onVisible(1); `move_waypoint`
-- is an instant grid snap with no arrival event; and a per-player clone
-- in a shared world spawns one Copplemann per player, which D-CA15
-- forbids. Chain 1235 substitutes a 10.5-second delay (63 units at the
-- 6.0 u/s NPC speed) after which the mission advances and the turn-in
-- topic appears, with no clone, no hide and no movement.
--
-- No enemy wave is authored anywhere in this mission. That is not an
-- omission: Castle.py:264-286 displays 2574 on the bare step gate with no
-- kill condition, which the audit confirms (spec scenario T05).

-- Chain 1231 [RECOVERED_SCRIPT] — Castle.py n31_propagator_Out, first
-- arm: interact with Copplemann while step 2399 is active → dialog 2574.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1231, '701 - Copplemann interact (step 2399): dialog 2574', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1231, 'interact_tag', 'Castle_Coppleman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1231, 'step_status', 701, '2399', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1231, 'display_dialog', 2574, NULL, '{}', 0, 0);

-- Chain 1232 [RECOVERED_SCRIPT] — Castle.py `dialog.choice::2574` →
-- missions.advance(701, 2400) ("First help me get out of this boot.").
--
-- Dialog 2574 carries ZERO dialog_screen_buttons rows. The shipped
-- Cellblock chains 1018/1020/1021 trigger on dialogs 5020/5021/2300,
-- which are also button-less, so the client evidently still raises
-- `dialogButtonChoice` on a button-less dialog's close. That inference is
-- the single largest untested assumption in this mission and is called
-- out for UAT M1 in the worknote.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1232, '701 - Dialog 2574 choice: advance to step 2400', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1232, 'dialog_choice', '2574', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1232, 'step_status', 701, '2399', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1232, 'advance_step', 701, '2400', '{}', 0, 0);

-- Chain 1233 [RECOVERED_SCRIPT] — Castle.py n37_trigger_In: interact with
-- Copplemann while step 2400 is active → Livewire (cut her out of the
-- security boot). Precedent: Cellblock chains 1016/1017.
--
-- The Python built `Minigame('', 1, gameIds[0], 1, 0xffff, cb)`:
-- difficulty 1, techCompetency 1, and 0xffff as the ARCHETYPE permission
-- mask (deprecated/python/cell/Minigame.py:5-23) — not a seed and not a
-- timeout, and an all-archetypes value, so it is a no-op for a mission
-- every archetype can take. It never calls setSeed(), so the server picks
-- the seed, which is what this implementation already does. Difficulty 1
-- matches the current hardcode (executor/mod.rs:278), so when sibling
-- packet CA04 adds the `difficulty` param this row needs no change.
--
-- Defect B4 applies: if the SWF never connects, the session is never
-- expired and a second click hits the duplicate-register reject with no
-- client feedback at all. There is no seed-level mitigation — that is
-- CA04's fix. A DEFEAT is clean: the session is removed on exit, step
-- 2400 stays active and the bind stays on the player, so the player can
-- simply re-click and replay, matching the Python's empty defeat branch.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1233, '701 - Copplemann interact (step 2400): start Livewire minigame', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1233, 'interact_tag', 'Castle_Coppleman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1233, 'step_status', 701, '2400', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1233, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1234]}', 0, 0);

-- Chain 1234 [RECOVERED_SCRIPT] — Castle.py n37_propagator_Won. No
-- trigger row: invoked by id from chain 1233's on_victory_chains through
-- `fire_chain_by_id`, which evaluates no conditions (the step gate lives
-- on 1233) and also ignores `content_chains.enabled`, so do not try to
-- disable this chain with that column.
--
-- `display_dialog 2575` reaches the monologue branch of
-- executor/dialog.rs: `fire_chain_by_id` passes empty params, and
-- `last_interaction_target` is unset because a chain-handled interact
-- skips `handle_interact`. Dialog 2575 is a single screen with
-- speaker_id = 0, so it IS in the monologue cache and binds the player as
-- the wire EntityId — which is exactly the Python's
-- `displayDialog(None, 2575)`.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1234, '701 - Livewire victory: unbind topic, dialog 2575, advance to step 2401', 'mission', 701, true, 0);

-- no trigger row — invoked directly by the minigame callback in chain 1233
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1234, 'remove_dialog_set', 3062, NULL, '{"slot": 48}', 0, 0),
  (1234, 'display_dialog',    2575, NULL, '{}',           0, 1),
  (1234, 'advance_step',      701,  '2401', '{}',         0, 2);

-- Chain 1235 [RECONSTRUCTION, D-CA02 option A] — the escort substitute.
-- Castle.py's `dialog.choice::2575` node spawned a clone, hid the static
-- Copplemann, walked the clone to the safe zone and re-bound the topic in
-- the waypoint callback. Option A keeps the static NPC where she is and
-- represents the walk as a delay: 10500 ms ≈ the 63.23-unit straight-line
-- distance from (354.419, 70.272, 952.801) to (358.809, 70.156, 889.724)
-- at the 6.0 u/s NPC movement speed (10.54 s). Both actions carry the delay so the
-- turn-in "?" appears at the same moment the step flips, rather than the
-- player being told to talk to her before she has "arrived".
--
-- The turn-in binds row 3063 (dialog 2576, flags 33554432
-- INT_AStoryMissionTurnIn "?"), not the Python's 3062. 3062 is the
-- in-progress "!" row and would show the wrong glyph on a step whose only
-- remaining action is a turn-in; 3063 is original data authored for
-- exactly this purpose and, unlike 3062, actually loads today. This is the
-- one place the port deliberately diverges from Castle.py.
--
-- Deferred actions carry no conditions — they were resolved at trigger
-- time and fire unconditionally when they elapse. That is safe here only
-- because nothing else can advance 2401. See the engine-facts block above
-- for why death does not strand this timer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1235, '701 - Dialog 2575 choice: escort walk (deferred advance to 2421 + turn-in topic)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1235, 'dialog_choice', '2575', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1235, 'step_status', 701, '2401', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1235, 'advance_step',   701,  '2421', '{}',                             10500, 0),
  (1235, 'add_dialog_set', 3063, NULL,   '{"slot": 48, "mission_id": 701}', 10500, 1);

-- Chain 1236 [RECOVERED_SCRIPT, re-authored per D-CA04] — Castle.py
-- n65_propagator_Out, second arm. The Python hung this on the
-- `dialog_set.open::3062` event. That trigger loads and matches in the
-- content engine but has no `fire_dialog_set_open` dispatch site anywhere
-- in services, so it has never fired; D-CA04 retires it for Castle and
-- re-authors the node on the `interact_tag` + step gate the client
-- actually produces.
--
-- The Python's FIRST arm (2401 active → advance 2421) is not reproduced
-- here: option A moved that advance onto chain 1235's timer. Conditions
-- are evaluated against a pre-action snapshot, so a chain that both
-- advanced 2401→2421 and displayed 2576 would have to do it in one action
-- list; splitting it across two `interact_tag` chains would require two
-- clicks. Recorded in the worknote as the one behavioural difference a
-- player could notice.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1236, '701 - Copplemann interact (step 2421): turn-in dialog 2576', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1236, 'interact_tag', 'Castle_Coppleman', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1236, 'step_status', 701, '2421', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1236, 'display_dialog', 2576, NULL, '{}', 0, 0);

-- Chain 1237 [RECOVERED_SCRIPT] — Castle.py n71_trigger_In: on the 2576
-- choice with 2421 active, drop the turn-in topic and complete 701.
-- sort_order is load-bearing and mirrors Castle.py:210-216, where the
-- mission completion is nested inside the successful removeDialog.
--
-- Exactly once, structurally: `MissionInstance::complete()` moves 2421
-- from current_step_id into completed_steps, so the `eq active` gate is
-- false for any subsequent 2576 choice.
--
-- The turn-in is split across chains 1237/1238/1239 rather than being one
-- chain so that each `accept_mission` carries its own `eq not_active`
-- gate (the content-chains review rule) without coupling mission 701's
-- completion to the state of 702/703, and so the 703 reconstruction can
-- be withdrawn with a single `enabled = false` flip. All three match the
-- same event and all conditions are evaluated against the same
-- pre-action snapshot, so the completion in 1237 cannot gate 1238/1239.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1237, '701 - Dialog 2576 choice: unbind turn-in topic, complete 701', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1237, 'dialog_choice', '2576', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1237, 'step_status', 701, '2421', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1237, 'remove_dialog_set', 3063, NULL, '{"slot": 48}', 0, 0),
  (1237, 'complete_mission',  701,  NULL, '{}',           0, 1);

-- Chain 1238 [RECOVERED_SCRIPT] — Castle.py n71_trigger_In's
-- missions.accept(702), "Rescue Dr. Zuritska".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1238, '701 - Dialog 2576 choice: accept 702 (Rescue Dr. Zuritska)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1238, 'dialog_choice', '2576', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1238, 'step_status',    701, '2421', 'eq', 'active',     0),
  (1238, 'mission_status', 702, NULL,   'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1238, 'accept_mission', 702, NULL, '{}', 0, 0);

-- Chain 1239 [RECONSTRUCTION, D-CA05] — accept 703 "Payback" alongside
-- 702. Castle.py accepts ONLY 702 and never references 703, 704, 706 or
-- 708 anywhere; `accepts_mission_id` is NULL on every dialog in this set.
-- The evidence for accepting 703 here is indirect: dialog 2576's button
-- is "Take Missions" (button_id 71), the only PLURAL button text in the
-- entire dialog seed, and dialog 2577 ("You save me... And you kill
-- Romney") assumes 703 is already live by the Zuritska rescue. Without
-- this chain "Payback" has no acquisition path at all and is unreachable.
--
-- This is the least-supported row in the file (D-CA05 is MEDIUM
-- confidence). It is deliberately its own chain so it can be withdrawn by
-- setting `enabled` to false, with no effect on 701's completion or 702's
-- accept.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1239, '701 - Dialog 2576 choice: accept 703 (Payback) [RECONSTRUCTION D-CA05]', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1239, 'dialog_choice', '2576', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1239, 'step_status',    701, '2421', 'eq', 'active',     0),
  (1239, 'mission_status', 703, NULL,   'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1239, 'accept_mission', 703, NULL, '{}', 0, 0);

-- ============================================================
-- Relog restoration (1240-1243)
-- ============================================================
--
-- Per-player dialog binds live in `available_interactions` on the cell
-- entity and are NOT persisted, so every step that depends on a bind
-- needs a `player_loaded` chain that re-paints it. Precedent: Cellblock
-- chains 1006/1007. Note `add_dialog_set` pushes without de-duplicating,
-- so a player who relogs repeatedly on the same step accumulates
-- duplicate entries; the flags are OR-folded and `remove_dialog_set`
-- retains by map id (clearing every copy), so this is log noise rather
-- than a defect — same as 1006/1007.
--
-- Note there is no restore chain for "701 not accepted": chain 1201
-- already fires on `player_loaded` for that case.

-- Chain 1240 [RECOVERED_SCRIPT] — step 2399 active on login → re-bind the
-- Copplemann in-progress topic (the bind chain 1204/1205 made at accept).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1240, '701 - Restore Copplemann topic on login (step 2399 active)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1240, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1240, 'step_status', 701, '2399', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1240, 'add_dialog_set', 3062, NULL, '{"slot": 48, "mission_id": 701}', 0, 0);

-- Chain 1241 [RECOVERED_SCRIPT] — step 2400 active on login. Same bind as
-- 1240: the Livewire launcher (1233) needs Copplemann clickable.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1241, '701 - Restore Copplemann topic on login (step 2400 active)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1241, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1241, 'step_status', 701, '2400', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1241, 'add_dialog_set', 3062, NULL, '{"slot": 48, "mission_id": 701}', 0, 0);

-- Chain 1242 [RECONSTRUCTION, D-CA02 option A] — step 2401 active on
-- login → re-arm the deferred escort pair from chain 1235.
--
-- This is the recovery path for every case that scrubs the deferred queue
-- mid-walk: logging out, a cross-world hop, a GM despawn. All of them are
-- followed by a `player_loaded` when the player comes back to Castle, so
-- re-arming here is what keeps the mission from stranding on 2401. The
-- cost is that the player waits out the 10.5 s again.
--
-- No double-advance hazard: the queue is scrubbed on the way out, so at
-- most one entry exists per session, and once it fires step 2401 is no
-- longer active so this chain stops matching.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1242, '701 - Re-arm escort timer on login (step 2401 active)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1242, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1242, 'step_status', 701, '2401', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1242, 'advance_step',   701,  '2421', '{}',                             10500, 0),
  (1242, 'add_dialog_set', 3063, NULL,   '{"slot": 48, "mission_id": 701}', 10500, 1);

-- Chain 1243 [RECOVERED_SCRIPT] — step 2421 active on login → re-bind the
-- turn-in topic so the "?" comes back over Copplemann and chain 1236 can
-- fire.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1243, '701 - Restore Copplemann turn-in topic on login (step 2421 active)', 'mission', 701, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1243, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1243, 'step_status', 701, '2421', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1243, 'add_dialog_set', 3063, NULL, '{"slot": 48, "mission_id": 701}', 0, 0);
