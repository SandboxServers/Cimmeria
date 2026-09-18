-- Castle (World 8) content chains — missions 702, 703 and 704
-- Packets CA06 (702, 703) and CA07 (704) of the Castle rebuild ledger
-- (docs/analysis/castle-rebuild/work-packets.md).
--
-- EVIDENCE BOUNDARY — read before changing anything in this file.
--
-- There is NO recovered server script for missions 702-708. `Castle.py`
-- (deprecated/python/cell/spaces/Castle.py) accepts 702 at dialog 2576 and
-- then stops: it never references 702's steps, 703, 704, 706 or 708
-- anywhere. Every chain below is therefore a RECONSTRUCTION built from
-- original shipped data — mission steps/objectives/tasks
-- (db/resources/Missions/Seed/), dialogs and dialog screens
-- (db/resources/Dialogs/Seed/), items (db/resources/Items/) — not a port of
-- recovered logic. Individual rows are labelled:
--   ORIGINAL_DATA   — the id/value is a literal fact from the shipped seed.
--   RECONSTRUCTION  — the wiring choice is this packet's, justified inline.
-- No row in this file is RECOVERED_SCRIPT.
--
-- Chain ID ranges (allocated by the Castle ledger, work-packets.md
-- "Worker Input And Ownership"):
--   CA06 (missions 702 + 703): 1261-1290
--     702: 1261-1265   703: 1271-1273
--     (next free: 1266-1270, 1274-1290)
--   CA07 (mission 704):        1291-1320
--     704: 1291-1302
--     (next free: 1303-1320)
--
-- Chains appear in PLAYER-FLOW order, not id order: 1299 (Zuritska's arrival
-- instruction) sits between 1291 and 1292 because that is when the player
-- meets it. Same convention as castle_cellblock_chains.sql, which places
-- 1020/1021 ahead of 1018/1019.
--
-- ============================================================
-- EXTERNAL CONTRACTS — these do not exist yet in this file's branch
-- ============================================================
--
-- Packet CA05 (Castle map recon and story-actor authoring) owns the
-- spawnlist rows, entity templates and point sets these chains name. Until
-- CA05 lands, the region triggers below reference `point_sets.name` rows
-- that are not seeded, and `crates/content-engine/tests/interact_tag_linter.rs
-- ::every_chain_region_key_matches_a_seeded_point_set` fails for exactly
-- those two keys. That is an expected cross-packet integration gap, not a
-- typo in this file. The contracted names, byte-exact:
--
--   Spawnlist tags:  Castle_Zuritska_Cell     (Zuritska in the detention cell)
--                    Castle_Zuritska_Comms    (Zuritska at the Level-5 workstation)
--                    Castle_Romney            (NID Interrogator Romney, hostile)
--                    Castle_CommsTerminal     (the Communications Room terminal)
--   Point sets:      Castle.InterrogationBlock
--                    Castle.CommsRoom
--
-- Three CA05 row-level requirements these chains silently depend on. Each
-- was verified against current engine code, and each breaks a mission if
-- CA05 takes the column defaults:
--
--   1. `spawnlist.respawn_secs` on the `Castle_Romney` row (DB CHECK >= 3).
--      NO row in the shipped `spawnlist.sql` sets it and no
--      `entity_templates` row does either, so
--      `COALESCE(s.respawn_secs, t.respawn_secs)` (`spawner/npcs.rs:149`)
--      is NULL for every NPC in the game and `mark_npc_dead` skips the
--      `respawn_at` stamp (`cell/combat/state.rs`). Without an explicit
--      value Romney is ONE-SHOT: the first player to kill him takes the
--      badge and mission 703 is uncompletable for everyone else until a
--      server restart.
--   2. `entity_templates.move_speed` on Zuritska's template. The COALESCE
--      default is 0.6 units/tick = 6.0 u/s against a player run speed of
--      8.125 u/s (`spawner/npcs.rs`), and `npc_ai/follow.rs` only re-paths
--      when `nav_path` is empty, so at the default she diverges
--      monotonically and never re-enters the 2-5 unit follow band. The only
--      non-default row in the seed is template 10 "Col Marsh (pet)" at 0.9;
--      Zuritska needs at least that.
--   3. `wander_radius` 0/NULL and `patrol_path_id` NULL on Zuritska's
--      template. `npc_ai/dispatch.rs` promotes an Idle NPC to Patrol or
--      Wander when either is set, so she would wander off both before the
--      escort starts and again the tick after chain 1291 clears the follow.
--
-- D-CA07 is why Zuritska is two static actors rather than one that moves:
-- Castle is a shared persistent world and a single Zuritska cannot be in
-- the cell for one player and at the workstation for another.
--
-- ============================================================
-- SHARED-WORLD CAVEAT (D-CA15) — flagged for the coordinator
-- ============================================================
--
-- `set_interaction_type` is GLOBAL on the entity, not per player: the arm
-- mutates `CellEntity::interaction_type_flags` and fans the new value to
-- every witness (crates/services/src/cell/content/executor/world/mod.rs:19-65),
-- and docs/content/interaction-flags.md:190 says so explicitly. In an
-- instanced zone (the Castle_CellBlock precedent) that is harmless. In
-- Castle, which is persistent and shared, it means:
--
--   * Player A entering the Interrogation Block lights the `!` over
--     Zuritska's cell actor for EVERY player in the zone, including
--     players who have not accepted 702.
--   * Player A reaching the Communications Room CLEARS that bit for every
--     player, so a player B who is mid-step 2419 or mid-escort loses the
--     ability to click the actor until B's restore or repair chain re-arms
--     it.
--
-- The chains' own `step_status` conditions mean nothing FIRES for the
-- wrong player — a spurious cursor is cosmetic, and a cleared bit is a
-- click that the client never sends. So this is an interactivity/indicator
-- defect, not a mission-state corruption. It is nonetheless a real
-- player-visible bug in a shared zone and is called out in
-- docs/analysis/castle-rebuild/worknotes/m702-704.md.
--
-- The per-player fix is the `add_dialog_set` bind (per-player
-- `available_interactions`, see executor/dialog.rs), which is packet CA02's
-- widened NULL-`dialog_id` bind under D-CA03. Once CA02 lands, the
-- `set_interaction_type` pairs below should be re-authored onto dialog-set
-- binds. Until then this is the only primitive that makes these actors
-- clickable at all, and the `interact_tag` linter requires one of the two.
--
-- Each `set` therefore also gets a REGION RE-ENTRY repair chain (1265,
-- 1300, 1301) beside its `player_loaded` restore, so a player whose bit was
-- cleared by someone else's progress gets it back by walking out and in
-- rather than by relogging. `|` is idempotent, so a spurious re-fire costs
-- nothing.
--
-- `set_follow_target` has the same shape and no per-player alternative at
-- all: there is ONE `Castle_Zuritska_Cell` entity in the zone and
-- `follow_target_id` is a single field on it. If player B frees Zuritska
-- while player A is mid-escort, she re-points at B and A's escort visually
-- ends. Mission state is unaffected — A's step 2405 still advances on A's
-- own `Castle.CommsRoom` entry — so this too is presentation, not
-- progression. A per-player escort needs owner-scoped entities, which is
-- design gate GCA1, not a content packet. Note that chain 1296 widens this:
-- ANY player sitting on step 2405 who loads into Castle re-points her at
-- themselves, and 2405 is a story step players will park on across
-- sessions. Same severity class, much higher firing rate.
--
-- ESCORT REPAIR, and why the cell actor's `!` outlives mission 702.
--
-- `AiState::Follow` is preemptable into Fighting by any threat
-- (`combat/threat/aggro.rs`), and `npc_ai_leash` ends at `AiState::Idle`
-- and never returns to Follow (`npc_ai/leash.rs`). One stray point of
-- splash damage to Zuritska on the way to Level 5 ends the escort
-- permanently; GC1b-0 stops her being teleported back to her cell, but
-- nothing restarts the follow.
--
-- The repair is a click. Chain 1302 (`interact_tag Castle_Zuritska_Cell`
-- gated on 704 step 2405) re-issues `set_follow_target`, which means the
-- actor has to stay clickable for the whole of step 2405 — so chain 1263
-- does NOT clear the `!` it inherits from 1261, and chain 1291 clears it on
-- Comms Room arrival instead. Chain 1296 restores both the follow and the
-- bit on relog. The bit's lifecycle across the two missions is therefore:
--
--   1261  set    (702 step 2402 → 2419: entering the Interrogation Block)
--   1263  kept   (702 completes, 704 accepted, escort starts)
--   1291  clear  (704 step 2405 → 2406: reaching the Comms Room)
--
-- with restores at 1264 (702/2419) and 1296 (704/2405) and a re-entry
-- repair at 1265 (702/2419 only).
--
-- ============================================================
-- OTHER ENGINE FACTS THESE CHAINS RELY ON
-- ============================================================
--
-- * `advance_step` is unconditional and force-completes the current step's
--   objectives (missions/progression.rs:20-129), so 2402's objectives 2778
--   and 2779 complete when 1261 advances to 2419. Never hand-complete every
--   objective of a multi-objective step.
-- * `complete_mission` on an active mission completes it and fires the
--   `mission_completed` follow-up once (executor/mission.rs). Objective
--   4653 (the only objective on 702's last step 2419) is completed by the
--   mission completion, NOT by a separate `complete_objective` — doing both
--   would be redundant and `complete_objective` on a last non-optional
--   objective completes the mission anyway.
-- * Victory chains fired by `start_minigame` run through `fire_chain_by_id`
--   with `ResolvedActions::default()` — NO conditions are evaluated
--   (event_dispatch/mod.rs:53-82). The step gate therefore lives on the
--   LAUNCHER chain (1292), never on the victory chain (1293).
-- * `entity_dead_tag` fires per killer, with the killer's own mission
--   context (event_dispatch/lifecycle.rs:66-90); the killer must have a
--   `player_id` (abilities/use_ability/kill_credit.rs:88-105).
-- * `add_item` with `"container": 0` falls through to the item's own
--   `container_sets[1]` (executor/inventory.rs:17,55 — `filter(|&c| c > 0)`),
--   which is 2 (mission container) for items 2135 and 5029. Same shape as
--   chain 1003 in castle_cellblock_chains.sql.
-- * Interaction flags are NOT persisted across a relog or a server restart,
--   which is what every `player_loaded Castle` restore chain below exists
--   for (precedent: chains 1045/1046/1062 in castle_cellblock_chains.sql).
-- * `display_dialog` needs an NPC to bind the client's portrait lookup to
--   (`executor/dialog.rs:47-105`). It resolves, in order: the chain's
--   `target_entity_id` param (stamped ONLY by `interact_tag` /
--   `interact_template` triggers), then the player's
--   `last_interaction_target` pin, then a monologue fallback for dialogs
--   whose every screen has `speaker_id = 0`. If none resolves it WARNS AND
--   RETURNS — the dialog never reaches the client. Consequences for this
--   file: 2577 (chain 1262) and 2581 (1294) are interact-driven and fine;
--   2580 (1293) is a single screen with `speaker_id = 0`
--   (`dialog_screens.sql:11779`) so the monologue fallback carries it; but
--   4866 carries `speaker_id = 1113` on two of its three screens
--   (`dialog_screens.sql:20516,20520`), so it CANNOT be displayed from a
--   region-entry chain. The mission-701 packet is restoring the
--   `last_interaction_target` pin for chain-handled interacts
--   (`cell_methods/player/interaction/interact.rs` skips it today), which
--   would give a region chain *a* binding — but the pin resolves to
--   whatever the player last clicked, which here is the CELL Zuritska, and
--   it is empty after a relog. So 4866 is authored on `interact_tag` as
--   chain 1299 (click-to-play), and chain 1291 arms the workstation actor
--   as well as the terminal so that click is available. The engine fix that
--   would make an arrival play correct is a `target_tag` param on
--   `display_dialog`, tracked as a follow-up packet candidate.
-- * A killing blow delivered by a damage-over-time pulse does NOT fire
--   `entity_death`: `cell/effects/pulsing/tick.rs::fire_pulse` mutates the
--   health stat with no alive→dead detection, and `mark_npc_dead` is only
--   reachable from `abilities/damage_apply/mod.rs` and `abilities/death.rs`.
--   A Romney finished by a bleed tick therefore fires neither 1272 nor
--   1273. Engine gap, not this packet's to fix; recorded because 703's only
--   completion path is an `entity_dead_tag` chain.

SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 702 — Rescue Dr. Zuritska (CA06)
-- ============================================================
--
-- ORIGINAL_DATA: mission 702 "Rescue Dr. Zuritska", steps 2402 ("Make your
-- way to the Interrogation Block.", index 0) → 2419 ("Zuritska is in one of
-- the detention cells. Locate him and free him!", index 1)
-- (mission_steps.sql:5971,5973). Objectives 2778 (required) and 2779
-- (optional, hidden) on 2402; 4653 on 2419 via task 5701 (`task_type 1`, no
-- payload) (mission_objectives.sql:6593-6597). Dialog 2577, speaker 1114
-- (Zuritska), screens 96826-96830 — "You save me... And you kill Romney."
-- then "You must get to the Communications Room, immediately."
-- (dialog_screens.sql:11769-11777).
--
-- 702 is accepted by mission 701's dialog-2576 chain (packet CA01/CA03,
-- another worker). These chains start from `702 step 2402 active`.
--
-- RECONSTRUCTION: dialog 2577's content is the sole acceptance evidence for
-- mission 704 — it is Zuritska, on being freed, telling the player to go to
-- the Communications Room, which is verbatim 704 step 2405's log text. No
-- other dialog, dialog_set_map row or `accepts_mission_id` in the seed
-- offers 704, so the 2577 choice is the only possible accept point.

-- Chain 1261: enter the Interrogation Block while 702 is on its travel step
-- → advance to the "find and free him" step and light Zuritska's cell actor.
--
-- RECONSTRUCTION: step 2402's own log text ("Make your way to the
-- Interrogation Block") is a travel instruction with no other completion
-- verb available — region entry is the only primitive in the engine that
-- expresses it, and `Castle.py` uses the same `client_hinted_region` shape
-- for its three named Castle regions.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1261, '702 - Enter Interrogation Block: advance to 2419, light Zuritska', 'mission', 702, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1261, 'enter_region', 'Castle.InterrogationBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1261, 'step_status', 702, '2402', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: 2402 → 2419 is the shipped step order (index 0 → 1).
  (1261, 'advance_step', 702, '2419', '{}', 0, 0),
  -- RECONSTRUCTION: `!` main-story-active indicator, INT_AStoryMissionActive
  -- (16777216). Matched clear is in chain 1263; restore is chain 1264.
  (1261, 'set_interaction_type', NULL, 'Castle_Zuritska_Cell',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 1);

-- Chain 1262: click the caged Zuritska while 2419 is active → play 2577.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1262, '702 - Interact Zuritska (cell): display dialog 2577', 'mission', 702, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1262, 'interact_tag', 'Castle_Zuritska_Cell', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1262, 'step_status', 702, '2419', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: dialog 2577 is the only Zuritska-rescue dialog in the seed.
  (1262, 'display_dialog', 2577, NULL, '{}', 0, 0);

-- Chain 1263: finish 2577 → free Zuritska. Completes 702, accepts 704, and
-- starts 704's escort by pointing the cell actor at the freeing player.
--
-- Objective 4653 is completed BY `complete_mission 702`, not by a separate
-- `complete_objective` — see the engine-facts block at the top of the file.
--
-- The `mission_status 704 eq not_active` gate is the accept-guard the
-- content-chain review rules require on every `accept_mission` chain
-- (.github/instructions/content-chains.instructions.md, "Mission grants must
-- gate on not_active"). It is also what stops a second run of this chain
-- (e.g. a replayed dialog choice) from re-arming the follow.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1263, '702 - Dialog 2577 choice: complete 702, accept 704, start escort', 'mission', 702, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1263, 'dialog_choice', '2577', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1263, 'step_status', 702, '2419', 'eq', 'active', 0),
  (1263, 'mission_status', 704, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- NOTE: this chain deliberately does NOT clear the `!` on
  -- `Castle_Zuritska_Cell`. The indicator set by chain 1261 stays lit
  -- through 704 step 2405 so the player can re-click Zuritska to restart a
  -- broken escort (chain 1302); chain 1291 clears it on Comms Room arrival.
  -- See the escort-repair note in the shared-world block at the top of this
  -- file for why.
  --
  -- ORIGINAL_DATA: 2419 is 702's last step and 4653 its only objective, so
  -- the rescue completes the mission.
  (1263, 'complete_mission', 702, NULL, '{}', 0, 0),
  -- RECONSTRUCTION: dialog 2577 is the only place in the shipped data that
  -- points the player at the Communications Room (704 step 2405).
  (1263, 'accept_mission', 704, NULL, '{}', 0, 1),
  -- RECONSTRUCTION, presentation only: 704 step 2405 is "Escort Dr. Zuritska
  -- to the Communications Room down on Level 5." `use_player: true` resolves
  -- the follow target to the triggering player (executor/world/mod.rs:135-192)
  -- — the only way to follow a player, since players carry no spawnlist tag.
  -- PROVISIONAL: Castle has no `castle.nav`, so `find_path` returns None and
  -- the follower walks a straight line to the player
  -- (space_manager/spatial.rs:49, npc_ai/follow.rs:110). Expect clipping
  -- through Castle interior geometry until packet CA14 produces the navmesh.
  (1263, 'set_follow_target', NULL, 'Castle_Zuritska_Cell', '{"use_player": true}', 0, 2);

-- Chain 1264: relog restore for 702 step 2419. Interaction flags do not
-- survive a relog or a server restart, so without this a player who logs out
-- between entering the block and freeing Zuritska finds the actor inert and
-- the mission stuck. Precedent: chains 1006/1007/1045/1046 in
-- castle_cellblock_chains.sql.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1264, '702 - Restore Zuritska cell indicator on login (step 2419)', 'mission', 702, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1264, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1264, 'step_status', 702, '2419', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1264, 'set_interaction_type', NULL, 'Castle_Zuritska_Cell',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

-- Chain 1265: region RE-ENTRY repair for 702 step 2419.
--
-- `set_interaction_type` is global on the entity (see the shared-world
-- caveat at the top of this file), so another player's rescue clears
-- Zuritska's `!` for everyone — including a player still on 2419, who then
-- cannot click her at all. Chain 1264 repairs that only on a relog. This
-- chain repairs it on walking back into the Interrogation Block, which is a
-- few seconds rather than a reconnect.
--
-- Disjoint from chain 1261, which fires on the same region but gates on
-- step 2402: at most one of the two matches any given entry.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1265, '702 - Re-arm Zuritska cell indicator on region re-entry (step 2419)', 'mission', 702, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1265, 'enter_region', 'Castle.InterrogationBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1265, 'step_status', 702, '2419', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1265, 'set_interaction_type', NULL, 'Castle_Zuritska_Cell',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

-- ============================================================
-- MISSION 703 — Payback (CA06)
-- ============================================================
--
-- ORIGINAL_DATA: mission 703 "Payback", steps 2403 ("Locate NID Interrogator
-- Romney in the Castle.", index 0) → 2404 ("Romney must be somewhere in the
-- Interrogation Block. Locate and eliminate him.", index 1)
-- (mission_steps.sql:5975,5977). Objective 2780 on 2403, 2781 on 2404, both
-- required and neither optional (mission_objectives.sql:6599-6601). No
-- mission-specific dialog: 703 is a kill mission end to end. Item 2135
-- "Romney's NID Badge" (items.sql:11321, `container_sets {2}`) is the only
-- item in the seed that names Romney; its description text is a copy-paste
-- of the Ambernol vial's and is not evidence of anything.
--
-- 703 is accepted with 702 by mission 701's dialog-2576 chain under D-CA05
-- (another worker). These chains start from `703 step 2403 active`.
--
-- RECONSTRUCTION: 2135 is an EXPLICIT GRANT on the death chain, not loot.
-- There are no `mission_reward_groups` rows for 701-708 and no loot table
-- carries 2135 (audit.md, "Mission items are explicit grants, no loot
-- tables"). Granting it from the chain is the only mechanism available and
-- it gives the kill a tangible proof-of-identity, which is what the item
-- name is for. Nothing in the mission data requires the badge, so a future
-- decision to drop the grant costs nothing.

-- Chain 1271: enter the Interrogation Block while 703 is on its "locate"
-- step → advance to the "eliminate him" step. Same region as chain 1261;
-- both fire on the same entry when the player holds both missions, which is
-- the intended shape (702 and 703 are accepted together under D-CA05).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1271, '703 - Enter Interrogation Block: advance to 2404', 'mission', 703, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1271, 'enter_region', 'Castle.InterrogationBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1271, 'step_status', 703, '2403', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: 2403 → 2404 is the shipped step order (index 0 → 1).
  (1271, 'advance_step', 703, '2404', '{}', 0, 0);

-- Chain 1272: Romney dies while the player is on the kill step → complete
-- 703 and grant the badge.
--
-- `entity_dead_tag` carries the KILLER's mission context
-- (event_dispatch/lifecycle.rs:78-83), so this is per killer: Romney
-- respawning on the ordinary spawner timer lets the next player have the
-- same kill, and a player who is not on 2404 gets nothing. 2781 is 2404's
-- only objective, so completing the mission completes it.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1272, '703 - Romney killed on step 2404: complete 703, grant badge', 'mission', 703, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1272, 'entity_dead_tag', 'Castle_Romney', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1272, 'step_status', 703, '2404', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- RECONSTRUCTION: explicit grant, mission container (container_sets[1] = 2).
  (1272, 'add_item', 2135, NULL, '{"container": 0, "qty": 1}', 0, 0),
  (1272, 'complete_mission', 703, NULL, '{}', 0, 1);

-- Chain 1273: Romney dies while the player is STILL on the locate step 2403.
--
-- RECONSTRUCTION, and the reason this chain exists: Castle is an open
-- persistent world and nothing forces the player through
-- `Castle.InterrogationBlock` before reaching Romney. Region entry is
-- client-hinted, so a player can also simply miss the volume. Without this
-- chain, killing Romney off the expected path leaves 703 permanently stuck
-- on 2403 with a dead target that only respawns on the spawner timer. The
-- action list advances and completes in one go.
--
-- This chain and 1272 can never both fire for one death: the engine
-- snapshots the killer's mission context ONCE in `fire_entity_death` and
-- evaluates every chain against that single snapshot, so `2403 active` and
-- `2404 active` are mutually exclusive at resolve time — actions run after
-- resolution and cannot open the other chain's gate.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1273, '703 - Romney killed on step 2403 (region skipped): advance, complete, grant', 'mission', 703, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1273, 'entity_dead_tag', 'Castle_Romney', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1273, 'step_status', 703, '2403', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Advance first so the mission log shows the kill step before completion
  -- and 2780 is force-completed by the step transition rather than orphaned.
  (1273, 'advance_step', 703, '2404', '{}', 0, 0),
  (1273, 'add_item', 2135, NULL, '{"container": 0, "qty": 1}', 0, 1),
  (1273, 'complete_mission', 703, NULL, '{}', 0, 2);

-- ============================================================
-- MISSION 704 — Hack Communications (CA07)
-- ============================================================
--
-- ORIGINAL_DATA: mission 704 "Hack Communications", steps 2405 ("Escort Dr.
-- Zuritska to the Communications Room down on Level 5.", index 0) → 2406
-- ("Use the Communications Terminal to download specifications for the
-- Castle's security system.", index 1) → 2407 ("Deliver the Data crystal to
-- Zuritska.", index 2) (mission_steps.sql:5979-5983). Objectives: 2782
-- (required) and 2783 (optional, hidden) on 2405; 2784 on 2406 via task
-- 6341; 5151 on 2407 (mission_objectives.sql:6603-6609). Dialogs, all
-- speaker 1113 (Zuritska at the workstation): 4866 screens 96892-96894 ("I
-- will patch into communication network... While I do this you use terminal
-- there."), 2580 screen 96896 (the terminal read-out, which names the Throne
-- Room override node — the hand-off to 706), 2581 screens 96900-96906 (the
-- delivery briefing, ending "You must go..." to the Throne Room)
-- (dialog_screens.sql:11779-11793, 20516-20520). Item 5029 "Data Crystal"
-- (items.sql:11105, `container_sets {2}`).
--
-- D-CA08: step state is the possession proof, not `HasItem`. The cell has no
-- inventory view at all (CellEntity carries only bandolier and loot), so a
-- `HasItem` gate is unimplementable; the granting chain advances the step in
-- the same action list, which makes the step the proof. Accepted divergence:
-- dropping or selling the crystal does not regress step 2407.
--
-- D-CA09: the Communications Terminal minigame is Livewire, PROVISIONAL.
-- The original game id for 2406 is unrecovered; Livewire is the only real
-- minigame implemented (minigame/games/mod.rs:11-23) and is already used
-- three times in Castle_CellBlock. Same policy as Cellblock D-CB14.

-- Chain 1291: reach the Communications Room while escorting → advance to
-- the terminal step, stand Zuritska's cell actor down, play Zuritska's
-- workstation line, and arm the terminal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1291, '704 - Enter Comms Room: advance to 2406, end escort, arm terminal', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1291, 'enter_region', 'Castle.CommsRoom', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1291, 'step_status', 704, '2405', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: 2405 → 2406 is the shipped step order (index 0 → 1).
  (1291, 'advance_step', 704, '2406', '{}', 0, 0),
  -- Clear the escort. An empty params object means no `target_tag` and no
  -- `use_player`, which the loader turns into
  -- `SetFollowTarget { target_tag: None, use_player: None }`
  -- (loader/action.rs, `set_follow_target`); the arm resolves that to None,
  -- clears `follow_target_id`, drops the NPC to Idle and clears `nav_path`
  -- (executor/world/mod.rs:186-192).
  --
  -- The clear must precede the walk home: `set_follow_target` drops the NPC
  -- to Idle and clears `nav_path`, so running it after `move_waypoint` would
  -- throw the freshly-issued path away. Action order inside a chain is the
  -- `sort_order` below, and the executor runs them in that order.
  (1291, 'set_follow_target', NULL, 'Castle_Zuritska_Cell', '{}', 0, 1),
  -- Walk the cell actor home, closing the gap the first draft of this chain
  -- left open. The coordinate is spawn_id 238 `Castle_Zuritska_Cell`,
  -- template 168, world 8 — ORIGINAL_DATA relative to this packet, authored
  -- by CA05 (docs/analysis/castle-rebuild/worknotes/ca05.md, "Spawnlist
  -- rows"), which is also where the `Castle.InterrogationBlock` box that
  -- contains it is defined. Writing it as a literal rather than reading the
  -- spawn row is forced: `MoveWaypoint` takes a parsed `[f32; 3]` and the
  -- content engine has no "walk to your spawn" verb.
  --
  -- If CA05's spawn position ever moves, this row must move with it; the
  -- live-DB guard `chain_1291_walks_zuritska_back_to_her_seeded_spawn` pins
  -- the two together so the drift fails a test instead of stranding her
  -- inside a wall.
  --
  -- No `speed` param: the default 1.0 multiplier keeps her at the template's
  -- own `move_speed` (0.9, set by CA05 so the follow can keep pace), which
  -- is the speed the player just watched her walk at.
  (1291, 'move_waypoint', NULL, 'Castle_Zuritska_Cell',
   '{"destination": "268.0,66.79,1042.59"}', 0, 2),
  -- RECONSTRUCTION (D-CA09, provisional): INT_MinigameLivewire (256) is the
  -- hackable-console cursor. Matched clear is in chain 1293; restore is 1297,
  -- re-entry repair is 1300.
  (1291, 'set_interaction_type', NULL, 'Castle_CommsTerminal',
   '{"op": "|", "mask": "INT_MinigameLivewire"}', 0, 3),
  -- The workstation Zuritska becomes clickable HERE rather than at the
  -- Livewire victory, because dialog 4866 ("While I do this you use terminal
  -- there") is delivered by chain 1299 on a click rather than on arrival —
  -- see the `display_dialog` note in the engine-facts block at the top of
  -- this file, and chain 1299's own comment. Matched clear is in chain 1295;
  -- restores are 1297/1298, repairs are 1300/1301.
  (1291, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 4),
  -- Matched clear for the `!` chain 1261 set on the CELL actor. It is
  -- cleared HERE rather than at the rescue (chain 1263) so the actor stays
  -- clickable for the whole of step 2405 and chain 1302 can restart a broken
  -- escort. Arriving in the Comms Room is what ends the escort, so it is
  -- also what ends the affordance. Restore is 1296; there is no region
  -- re-entry repair for this one (see the note on 1302).
  (1291, 'set_interaction_type', NULL, 'Castle_Zuritska_Cell',
   '{"op": "~", "mask": "INT_AStoryMissionActive"}', 0, 5);

-- Chain 1302: click Zuritska during the escort → she follows again.
--
-- RECONSTRUCTION, and the reason it exists: `AiState::Follow` is
-- preemptable into Fighting by any threat (`combat/threat/aggro.rs`), and
-- `npc_ai_leash` ends at `AiState::Idle` and never returns to Follow
-- (`npc_ai/leash.rs`). One stray point of splash damage to Zuritska on the
-- way down to Level 5 therefore ends the escort permanently. GC1b-0 stops
-- her being teleported back to her cell, but nothing restarts the follow.
-- Before this chain the only re-fire was 1296 on `player_loaded`, i.e. the
-- player had to relog.
--
-- The click is the "follow me again" affordance, which is why chain 1263 no
-- longer clears the cell actor's `!` and chain 1291 clears it instead: the
-- indicator has to survive the whole of step 2405 for this to be reachable.
--
-- Disjoint from chain 1262 on the same tag: 1262 gates on 702 step 2419,
-- this on 704 step 2405, and 1263 closes the first as it opens the second.
--
-- KNOWN GAP, deliberate: `set_interaction_type` is global on the entity, so
-- another player arriving in the Comms Room (chain 1291) clears this
-- player's affordance too. The only repair is the relog restore (1296).
-- The region re-entry repair used for the other bits (1265/1300/1301) does
-- not fit here — the player spends step 2405 in the corridor between the
-- Interrogation Block and the Comms Room, which has no point set to trigger
-- on.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1302, '704 - Interact Zuritska (cell) on 2405: re-arm the escort follow', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1302, 'interact_tag', 'Castle_Zuritska_Cell', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1302, 'step_status', 704, '2405', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1302, 'set_follow_target', NULL, 'Castle_Zuritska_Cell', '{"use_player": true}', 0, 0);

-- Chain 1299: click the workstation Zuritska while the terminal step is
-- active → she tells the player to use the terminal.
--
-- RECONSTRUCTION: dialog 4866 is CLICK-TO-PLAY, not played on arrival. The
-- shipped data gives no ordering evidence either way, and only an
-- `interact_tag` trigger stamps `target_entity_id`, which is what binds the
-- client's portrait to the workstation Zuritska whose speaker id (1113) the
-- dialog actually carries. Authoring it on chain 1291's region entry instead
-- would bind it through the player's `last_interaction_target` pin, which
-- resolves to the CELL Zuritska they just freed and is empty entirely after
-- a relog — a wrong actor or a silent drop. Authoring it in both places
-- would play the line twice on a clean run. The `!` on this actor (set by
-- chain 1291, restored by 1297, repaired by 1300) is what tells the player
-- to click.
--
-- The engine-side fix that would let this play on arrival is a `target_tag`
-- param on `display_dialog`, tracked as a follow-up packet candidate.
--
-- Mutually exclusive with chain 1294 on the same tag: 1299 gates on step
-- 2406, 1294 on 2407.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1299, '704 - Interact Zuritska (comms) on 2406: display dialog 4866', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1299, 'interact_tag', 'Castle_Zuritska_Comms', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1299, 'step_status', 704, '2406', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: 4866, speaker 1113, screens 96892-96894.
  (1299, 'display_dialog', 4866, NULL, '{}', 0, 0);

-- Chain 1292: use the terminal while 2406 is active → launch Livewire.
--
-- The step gate lives HERE and not on the victory chain, because 1293 is
-- fired by id with `ResolvedActions::default()` and evaluates no
-- conditions — a condition row there would read as a guard while guarding
-- nothing. Precedent: chains 1016/1017 and 1041/1042 in
-- castle_cellblock_chains.sql.
--
-- What actually stops a second Data Crystal, precisely, because the two
-- races have DIFFERENT answers and it is easy to credit the wrong one:
--
--   * SEQUENTIAL (win, then click the terminal again). The step gate. 1293
--     advances to 2407 as part of the victory action list, so this chain's
--     `step_status 2406 active` is already false by the time the player can
--     click again. Pinned by
--     `chain_1292_does_not_resolve_on_the_delivery_step`.
--   * CONCURRENT (two clicks before any win). NOT the step gate — 2406 is
--     still active for both, so this chain resolves twice and emits two
--     `StartMinigame` messages, each carrying `on_victory_chains: [1293]`.
--     The guard is the minigame registry: `MinigameRegistry::register`
--     (crates/services/src/minigame/session.rs) returns `None` when
--     `sessions` already holds an entry for the entity, so the second
--     launch never becomes a session and can never report a victory. Its
--     own test `duplicate_session_rejected` pins that.
--
-- The registry guard is therefore load-bearing for this mission, which
-- makes CA04's session-lifecycle work (PR #652: expire never-connected
-- sessions after 180 s, abort on SWF close) load-bearing too — it is what
-- stops an abandoned launch from locking the terminal until relog. Recorded
-- in worknotes/m702-704.md as an input to that packet's design.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1292, '704 - Comms terminal: start Livewire minigame', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1292, 'interact_tag', 'Castle_CommsTerminal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1292, 'step_status', 704, '2406', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1292, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1293]}', 0, 0);

-- Chain 1293: Livewire victory (invoked by 1292's on_victory_chains) →
-- terminal read-out, grant the crystal, advance to the delivery step, swap
-- the cursors from the terminal to the workstation Zuritska.
--
-- NO TRIGGER ROW and NO CONDITION ROWS by design — see the note on 1292.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1293, '704 - Livewire victory: grant Data Crystal, advance to 2407', 'mission', 704, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: 2580 is the terminal read-out and names the Throne Room
  -- override node, which is 706's premise.
  (1293, 'display_dialog', 2580, NULL, '{}', 0, 0),
  -- ORIGINAL_DATA: item 5029 "Data Crystal" is 2407's subject ("Deliver the
  -- Data crystal to Zuritska"). RECONSTRUCTION: the grant point. Exactly one
  -- grant is guaranteed by 1292's `step_status 2406 active` gate plus this
  -- chain's own `advance_step` to 2407, which shuts that gate.
  (1293, 'add_item', 5029, NULL, '{"container": 0, "qty": 1}', 0, 1),
  -- ORIGINAL_DATA: 2406 → 2407 is the shipped step order (index 1 → 2).
  (1293, 'advance_step', 704, '2407', '{}', 0, 2),
  -- Matched clear for chain 1291's terminal bit.
  (1293, 'set_interaction_type', NULL, 'Castle_CommsTerminal',
   '{"op": "~", "mask": "INT_MinigameLivewire"}', 0, 3),
  -- Re-assert the `!` on the workstation Zuritska for the delivery. Chain
  -- 1291 already set it for the 4866 instruction, so this is idempotent on
  -- the happy path — it is kept because it is also the repair for the
  -- shared-world case where another player's chain 1295 cleared the bit
  -- while this player was inside the minigame. Matched clear is in 1295;
  -- restore is 1298, re-entry repair is 1301.
  (1293, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 4);

-- Chain 1294: click the workstation Zuritska while 2407 is active → play the
-- delivery briefing.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1294, '704 - Interact Zuritska (comms): display dialog 2581', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1294, 'interact_tag', 'Castle_Zuritska_Comms', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1294, 'step_status', 704, '2407', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- ORIGINAL_DATA: 2581 is the crystal-delivery briefing and ends by sending
  -- the player to the Throne Room access panel, which is mission 706.
  (1294, 'display_dialog', 2581, NULL, '{}', 0, 0);

-- Chain 1295: finish 2581 → hand the crystal over, complete 704, start 706.
--
-- `remove_item` is explicit because nothing consumes items implicitly any
-- more (.github/instructions/content-chains.instructions.md, "Inventory
-- consumption"). Per D-CA08 this chain does NOT gate on holding 5029 — the
-- step gate is the possession proof; a player who dropped the crystal still
-- completes, and `RemoveInventoryItemByType` on a stack that is not there is
-- a no-op on the base side.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1295, '704 - Dialog 2581 choice: consume crystal, complete 704, accept 706', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1295, 'dialog_choice', '2581', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1295, 'step_status', 704, '2407', 'eq', 'active', 0),
  (1295, 'mission_status', 706, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Consume first: the crystal is handed to Zuritska, and consuming before
  -- the state moves keeps a mid-list failure from leaving a completed
  -- mission plus a stale mission item.
  (1295, 'remove_item', 5029, NULL, '{"qty": 1}', 0, 0),
  -- Matched clear for the workstation bit, which 1291 sets on arrival and
  -- 1293/1297/1300/1301 re-assert. This is the only chain that clears it.
  (1295, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "~", "mask": "INT_AStoryMissionActive"}', 0, 1),
  -- ORIGINAL_DATA: 2407 is 704's last step and 5151 its only objective.
  (1295, 'complete_mission', 704, NULL, '{}', 0, 2),
  -- RECONSTRUCTION: 706 "Power Behind the Throne" is the next story mission
  -- and dialog 2581 is the only place the Throne Room access panel is named.
  -- 706's own chains are packet CA08 (another worker).
  (1295, 'accept_mission', 706, NULL, '{}', 0, 3);

-- Chain 1296: relog restore for 704 step 2405 — re-arm the escort follow
-- AND the cell actor's `!`.
--
-- `follow_target_id` is per-entity runtime state and the player's entity id
-- changes across a relog, so a stale id would point Zuritska at nothing (the
-- follow handler clears it and drops to Idle on the next tick). Re-issuing
-- `use_player` on load rebinds her to the returning player's new entity id.
--
-- The `!` is restored alongside it because it is the affordance chain 1302
-- needs: without it a returning player cannot click Zuritska to restart a
-- broken escort, which is the whole point of 1302. This is also the ONLY
-- repair for that bit once another player's Comms Room arrival has cleared
-- it globally — see the known-gap note on chain 1302.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1296, '704 - Restore Zuritska escort + cell indicator on login (step 2405)', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1296, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1296, 'step_status', 704, '2405', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1296, 'set_follow_target', NULL, 'Castle_Zuritska_Cell', '{"use_player": true}', 0, 0),
  (1296, 'set_interaction_type', NULL, 'Castle_Zuritska_Cell',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 1);

-- Chain 1297: relog restore for 704 step 2406 — re-arm BOTH the terminal
-- and the workstation Zuritska, because step 2406 has two interactables:
-- the terminal (chain 1292's Livewire) and Zuritska herself (chain 1299's
-- instruction dialog).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1297, '704 - Restore Comms terminal + workstation bits on login (step 2406)', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1297, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1297, 'step_status', 704, '2406', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1297, 'set_interaction_type', NULL, 'Castle_CommsTerminal',
   '{"op": "|", "mask": "INT_MinigameLivewire"}', 0, 0),
  (1297, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 1);

-- Chain 1298: relog restore for 704 step 2407 — re-arm the workstation
-- Zuritska so the crystal can still be delivered after a relog.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1298, '704 - Restore Zuritska comms indicator on login (step 2407)', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1298, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1298, 'step_status', 704, '2407', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1298, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

-- Chain 1300: region RE-ENTRY repair for 704 step 2406 — the Comms Room
-- twin of chain 1265. Repairs both of the step's bits after another
-- player's Livewire victory (chain 1293) or delivery (1295) cleared them
-- globally, without waiting for a relog.
--
-- Disjoint from chain 1291, which fires on the same region but gates on
-- step 2405.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1300, '704 - Re-arm terminal + workstation on region re-entry (step 2406)', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1300, 'enter_region', 'Castle.CommsRoom', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1300, 'step_status', 704, '2406', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1300, 'set_interaction_type', NULL, 'Castle_CommsTerminal',
   '{"op": "|", "mask": "INT_MinigameLivewire"}', 0, 0),
  (1300, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 1);

-- Chain 1301: region RE-ENTRY repair for 704 step 2407 — the delivery bit.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1301, '704 - Re-arm workstation indicator on region re-entry (step 2407)', 'mission', 704, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1301, 'enter_region', 'Castle.CommsRoom', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1301, 'step_status', 704, '2407', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1301, 'set_interaction_type', NULL, 'Castle_Zuritska_Comms',
   '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);
