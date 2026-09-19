-- Harset OP-CORE Human mission chains (chain ids 6501-6800)
-- Worlds: 57 Harset, 68 Harset_CmdCenter
--
-- Campaign: docs/analysis/harset-rebuild/ (ledger packets H30-H37).
-- Authoring rules: work-packets.md "Worker Input And Ownership";
-- canonical tags: worknotes/harset-tags.md; dialog-set-map ids 120101-120200.
-- Every coordinate in this file must be recovered from the 2009 Python
-- or pinned in M0; mission-scoped hostiles are spawned into the player's
-- own Market/Storage instance, never into world 57 or 68.
-- Sub-allocation per mission is fixed in the ledger table; never reuse a
-- 1xxx-5xxx id.
--
-- Chain ID ranges owned by this file (statically allocated in
-- docs/analysis/harset-rebuild/work-packets.md to avoid seed-order
-- sensitivity):
--   1360 / 567:  6501-6510   (H30; 6501-6505 used, 6506-6510 free)
--   1361:        6511-6530   (H31; 6511-6527 used, 6528-6530 free)
--   1362:        6531-6550   1363: 6551-6570   1365: 6571-6590
--   1371:        6591-6605   1372: 6606-6620   1374: 6621-6645
--   1375:        6646-6665   1377: 6666-6680   1410: 6681-6690
--   1580:        6691-6710
--
-- Evidence class for everything below: RECOVERED_DATA. No Harset mission
-- script survives (README.md "Purpose And Evidence Boundary" — only 742
-- has one), so the chains themselves are NEW AUTHORING, but every dialog
-- id, dialog-set-map id, step id, objective id and item id is read out of
-- the shipped 2009 tables and is cited inline. Nothing here invents text,
-- an item or a coordinate. This file seeds NO coordinates at all.
--
-- ZERO new dialog_set_maps rows. All ten bindings these two packets need
-- already exist in the shipped data, so the reserved 120101-120200 range
-- is untouched. The evidence tables are in worknotes/H30-H31.md.
--
-- Packets: Harset H30, H31. Base: content/harset-wave2 @ 94e65324.

SET search_path = resources, pg_catalog;

-- ============================================================
-- AUTHORING NOTES THAT APPLY TO EVERY CHAIN IN THIS FILE
-- ============================================================
--
-- (A) MULTI-CHAIN DISPATCH — the trap this file is shaped around.
--     `ChainEngine::resolve_event` (crates/content-engine/src/chain/mod.rs)
--     loops EVERY registered chain for the trigger type and APPENDS the
--     actions of each one whose conditions pass. There is no first-match
--     break, and `priority` only orders the bucket — it never excludes a
--     sibling. Conditions for all chains are evaluated BEFORE any action
--     runs, so a `complete_mission` in chain A cannot gate chain B in the
--     same event.
--
--     Consequence: any two chains sharing an `interact_tag` key must be
--     made pairwise disjoint BY CONDITION, or one right-click runs both.
--     Three chains key on 'CmdCenter_Marsh' (6501 letter turn-in, 6512
--     Praxis offer, 6527 Praxis turn-in). Their disjointness is enforced
--     by the extra conditions marked "DISJOINTNESS" below and is pinned
--     by `marsh_interact_chains_are_pairwise_disjoint` in
--     crates/services/src/cell/content/chain_replay_tests/mission_1361.rs.
--     Do not remove one of those conditions without re-running that test.
--
--     Two chains key on 'CmdCenter_Mohkatan' (6515 step 4040, 6520 step
--     4042). They are disjoint for free: a mission has exactly one
--     `current_step_id`, so only one `step_status ... eq active` can hold.
--
-- (B) WHY THE NPC DIALOG COMES FROM A CHAIN AND THE ICON FROM A BIND.
--     Worlds 57 and 68 are SHARED (spaces.xml `Instanced="false"`, D-H04).
--     `set_interaction_type` mutates the entity's own
--     `interaction_type_flags` and broadcasts to every witness, so using
--     it for a mission indicator would light the NPC up for EVERY player
--     in the hub. `add_dialog_set` writes the acting player's
--     `available_interactions` and pushes InteractionType to that witness
--     only, so it is per-player — which is what a mission indicator has to
--     be. Hence: the bit and the "!"/"?" icon come from `add_dialog_set`,
--     the dialog and the state change come from the chain.
--
--     That is why the seven `interact_tag` chains in this file carry no
--     `set_interaction_type` action and are allowlisted in
--     crates/content-engine/tests/interact_tag_linter.rs. The bit is real;
--     it just arrives per-player instead of per-entity.
--
-- (C) WHEN A BEAT MUST USE THE BIND PATH INSTEAD OF `interact_tag`.
--     `fire_interact_tag` runs BEFORE `interactions::handle_interact` and
--     SHORT-CIRCUITS it when it matches
--     (cell_methods/player/interaction/interact.rs, the `if !handled`
--     fall-through). `interactions::handle_interact` is the ONLY code that
--     opens a bound dsm's dialog: it reads
--     `available_interactions[template_id]` and sends `onDialogDisplay`.
--
--     So an `interact_tag` chain on an NPC whose beat depends on a BOUND
--     dialog suppresses that dialog entirely. Two beats depend on one:
--     Hansen's 4459 and Anat's 4462 each carry the button whose
--     `dialog_choice` drives the step (6518, 6525). Give either NPC an
--     `interact_tag` chain and the button-bearing dialog never renders, so
--     the `dialog_choice` chain can never fire and the step dead-ends.
--
--     Both therefore use the BIND path: no `interact_tag` chain, the dsm
--     bind alone makes the NPC clickable, `handle_interact` opens the
--     dialog, and the logic hangs off `dialog_choice`. Same shape as
--     Castle chains 1011/1012 -> 1014/1015/1020/1021.
--
--     Every other beat in this file displays a BUTTONLESS blurb with no
--     follow-up, so `interact_tag` is safe and is preferred there because
--     it is explicitly step-gated rather than depending on bind lifecycle.
--
--     SPEAKER RESOLUTION IS NO LONGER A CONSTRAINT HERE (corrected
--     2026-09-19). `display_dialog` resolves the wire EntityId from
--     `params["target_entity_id"]` and falls back to
--     `last_interaction_target` (executor/dialog.rs), and
--     `fire_dialog_choice` stamps neither. An earlier draft of this note
--     said the pin was written inside `interactions::handle_interact` and
--     was therefore lost whenever a chain short-circuited it, and filed an
--     engine follow-up to move it. That follow-up is CLOSED: the pin now
--     happens in `cell_methods/player/interaction/interact.rs` BEFORE any
--     chain dispatch, right after the `interact_target_in_range` gate, so
--     every right-click pins its target regardless of which path claims
--     it. Do not re-file it.
--
-- (D) ONE DIALOG-CARRYING BIND PER TEMPLATE SLOT.
--     `interactions::handle_interact` scans
--     `available_interactions[template_id]` with `find_map`, taking the
--     first entry whose `dialog_id` is non-NULL (interaction-only binds —
--     `dialog_set_maps.dialog_id IS NULL` — are skipped so a flag-only
--     indicator cannot swallow the click). A second LIVE dialog-carrying
--     bind on one slot is therefore permanently unreachable. This file
--     never holds two binds on one slot at once; the proof is in
--     `mission_1361.rs` (`no_template_slot_ever_holds_two_binds_at_once`).
--     Slot 10 (Marsh) is the one at risk — dsm 5356, 5254 and 5253 all
--     target it — and is kept single by the same DISJOINTNESS conditions
--     as (A).
--
-- (E) `once` is dead (agent-memory content-engine-once-semantics). Every
--     one-shot guard here is a `step_status` / `mission_status` condition
--     that the chain's own `advance_step` / `complete_mission` flips false.
--
-- (F) `advance_step` force-completes the current step's active objectives
--     (cell/missions/progression.rs:56-65) and `complete_mission` ->
--     `complete_mission_direct` completes them all before
--     `MissionInstance::complete()`. So no chain in this file emits
--     `complete_objective`: every objective (4651, 4652, 4654-4657, 5572,
--     5573) is non-optional and is closed by the step transition that
--     follows it. Using `complete_objective` on the last required
--     objective of a MID-mission step would complete the whole mission
--     (agent-memory dialog-set-engine-gaps section 3).
--
-- (G) `world` condition column layout: the world id goes in `target_id`
--     (an integer FK to resources.worlds), NOT in `value` — the opposite
--     of `archetype`, which parses its id out of `value`. This matters
--     more than a normal typo: `convert_condition` returning None DROPS
--     THAT ONE CONDITION ROW AND KEEPS THE CHAIN, so a misplaced world id
--     does not disable the chain, it publishes it ungated in every world.
--     With world-57 and world-68 chains in the same file that is a live
--     cross-world misfire. Shape:
--       (chain_id, 'world', <world_id>, NULL, 'eq', NULL, sort_order)
--
-- (H) `scope_type` / `scope_id` are authoring metadata only — nothing
--     reads them at resolve time; the sole consumer is the admin content
--     editor, which groups chains by them. Every chain still carries them.

-- ============================================================
-- H30 / MISSION 1360 -- Frost's Letter, step 4038 (chains 6501-6502)
-- ============================================================
--
-- Harset owns step 4038 ONLY. Step 4037 and the mission accept are
-- Cellblock-side: chain 1121 accepts 1360 on the Frost loot dialog
-- (castle_cellblock_chains.sql), chain 1003 grants the letter (item 3730).
-- The mission survives the Cellblock -> Castle -> Harset hops the same way
-- any active mission survives a relog (`query_saved_missions` rebuilds it
-- from `sgw_mission` on the far side).
--
-- Step/objective ids read from resources.mission_steps /
-- mission_objectives: 1360 has step 4037 (objective 4650, index 0) and
-- step 4038 "Give Cpl. Frost's Letter to Col. Marsh." (objective 4651,
-- index 1). 4651 is non-optional and is closed by `complete_mission` per
-- note (F).
--
-- DIALOG EVIDENCE. dialog_set_maps row 5356 -> dialog 4576, set 1432
-- ("Letters Home"), interaction_flags 268435456
-- (INT_NonAStoryMissionActive, the "!" talk-to-advance icon), min_level 1,
-- no alignment/faction gate. Dialog 4576 is `DUIST_DefaultDialog`, three
-- screens, speaker 941 (Colonel Marsh), and carries NO buttons — it is a
-- pure blurb, so a right-click plays it and dismisses:
--   [941] "I want you to deliver that letter we found on Frost in the
--         Castle. His wife deserves it."
--   [  0] "What about your family?"
--   [  0] "I don't have family, soldier. I just have the corps."
-- This is the only Frost-letter dialog on speaker 941 anywhere in the
-- table, and there is no `DUIST_DefaultBlurb` for 1360 (the packet asked
-- for "the DUIST blurb if one exists" — 4576 is it).
--
-- Row 5356 lives in Copplemann's "Letters Home" set, which belongs to a
-- DIFFERENT, out-of-scope mission (collect letters -> Beta Site, cut by
-- D-H02). That is fine: `add_dialog_set` binds a dsm ROW to a template
-- slot, not a whole set, and the row's screens are Marsh's own lines.

-- Chain 6501: hand the letter to Marsh.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6501, '1360 - Marsh interact (step 4038): take Frost''s Letter, complete the mission', 'mission', 1360, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6501, 'interact_tag', 'CmdCenter_Marsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  -- Marsh is a shared-hub NPC in world 68; `interact_tag` carries no world
  -- of its own. Note (G) for the column layout.
  (6501, 'world', 68, NULL, 'eq', NULL, 0),
  -- Offer/turn-in guard. `complete_mission` flips this to 'completed', so
  -- a second right-click cannot re-run `remove_item 3730`. Paired with the
  -- step gate below rather than replacing it: the mission-level gate is
  -- the one that still holds if step bookkeeping ever drifts.
  (6501, 'mission_status', 1360, NULL, 'eq', 'active', 1),
  -- The step gate. `MissionInstance::complete()` moves `current_step_id`
  -- into `completed_steps`, so this is the primary re-fire guard.
  (6501, 'step_status', 1360, '4038', 'eq', 'active', 2),
  -- DISJOINTNESS (note A) vs chain 6527, which also keys on
  -- 'CmdCenter_Marsh'. Without this, a player who is on 1361 step 4694
  -- AND still carrying the letter would run both chains on one click and
  -- see only the second `display_dialog` (the later
  -- `send_dialog_display` re-pins `open_dialog_id`, so the first blurb is
  -- silently discarded while its item removal and completion still ran).
  -- Reaching that state is hard given 6512's own gate, but the guard is
  -- one row and the failure it prevents is silent content loss.
  (6501, 'step_status', 1361, '4694', 'neq', 'active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Play Marsh's blurb first. The wire EntityId is resolved from
  -- `params["target_entity_id"]`, which `fire_interact_tag` stamps, so the
  -- portrait binds to Marsh correctly (note C).
  (6501, 'display_dialog', 4576, NULL, '{}', 0, 0),
  -- Consume the letter. `Action::RemoveItem` takes only the design id and
  -- a qty — there is NO `container` param on the remove path (the loader
  -- reads `target_id` + `params.qty` and nothing else), and the by-type
  -- resolver's query carries no container filter
  -- (`SELECT ... FROM sgw_inventory WHERE character_id = $1 AND
  -- type_id = $2 ORDER BY container_id, slot_id LIMIT 1 FOR UPDATE`), so
  -- it finds the letter wherever chain 1003 actually landed it.
  --
  -- If the player no longer holds 3730 this logs a warn and does nothing;
  -- it does NOT abort the chain, so the mission still completes. That is
  -- deliberate — the alternative gate, `has_item`, is NOT AUTHORABLE
  -- (`convert_condition` has no "has_item" arm and nothing populates the
  -- `item_<id>_count` param it reads), and an unknown condition_type is
  -- dropped while the chain is KEPT, i.e. it would fail OPEN and look
  -- authored while enforcing nothing. Never add one.
  (6501, 'remove_item', 3730, NULL, '{"qty": 1}', 0, 1),
  -- Clear the "!" from Marsh for this player.
  (6501, 'remove_dialog_set', 5356, NULL, '{"slot": 10}', 0, 2),
  -- Terminal step of a 2-step mission, so this closes objective 4651 and
  -- the mission together (note F).
  (6501, 'complete_mission', 1360, NULL, '{}', 0, 3);
  -- GC3: grant_xp -- mission 1360 has reward_xp = 0 / reward_naq = 0 like
  -- every Harset mission. The `grant_xp` arm is wired on both sides since
  -- PR #618; only the formula is missing (D-H10). Add the action here when
  -- GC3 lands; do not hardcode a constant.

-- Chain 6502: relog restore for step 4038.
-- `available_interactions` are in-memory on the cell entity and do not
-- survive a relog, a server restart, or a `cross_world_teleport` (which
-- destroys the cell entity and rebuilds it via InitPlayerState on the far
-- side). Without this chain a player who logs out carrying the letter, or
-- who simply walks from Harset into the Command Center, finds Marsh inert.
-- Same shape as Cellblock chains 1006/1007/1045/1046.
--
-- `player_loaded` fires once per world entry from
-- `service/base_messages/player_init`, so this covers relog AND the
-- 57 -> 68 door crossing with one chain.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6502, '1360 - Restore Marsh letter binding on Command Center load (step 4038)', 'mission', 1360, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6502, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  -- Redundant with the trigger's own `world_name` filter, but the campaign
  -- rule is that every chain firing in a shared space carries a `world`
  -- condition, and `populate_world_context` + the player_init call site
  -- both set `world_id` on this path so it evaluates correctly.
  (6502, 'world', 68, NULL, 'eq', NULL, 0),
  (6502, 'step_status', 1360, '4038', 'eq', 'active', 1),
  -- DISJOINTNESS ON THE BIND SIDE (note D). The same row chain 6501
  -- carries on the interact side. Without it, a player who is on 1360
  -- step 4038 AND 1361 step 4694 at the same world entry gets BOTH 6502
  -- (dsm 5356) and 6526 (dsm 5253) binding template slot 10, and
  -- `handle_interact`'s `find_map` would make the second permanently
  -- unreachable.
  --
  -- That state is reachable: 1361 is accepted while 4038 is NOT active
  -- (chains 6511/6512 require `neq active`), but nothing stops the player
  -- from going back to the Castle mid-Praxis, looting Frost, advancing
  -- 1360 to 4038 and returning. Found by
  -- `no_template_slot_ever_holds_two_binds_at_once`.
  --
  -- The precedence chosen here matches the interact side exactly: while
  -- the Praxis turn-in is pending, Marsh wears the Praxis "?" and the
  -- letter waits. Chain 6505 below hands the letter "!" back the instant
  -- 1361 completes, so the player never needs a relog to see it.
  (6502, 'step_status', 1361, '4694', 'neq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6502, 'add_dialog_set', 5356, NULL, '{"slot": 10, "mission_id": 1360}', 0, 0);

-- Chain 6505: hand the letter indicator back when 1361 completes.
--
-- This is the other half of 6502's DISJOINTNESS row. `player_loaded` is
-- the only other thing that binds 5356, and it does not fire again when a
-- player completes 1361 standing still in the Command Center — so without
-- this chain the letter "!" would stay dark until the next world crossing
-- or relog. That is not merely cosmetic: template 10 ships
-- `entity_templates.interaction_type = 0` and
-- `static_interaction_sets = '{}'`, so with no bind on slot 10 the client
-- never registers an interaction on Marsh at all and the right-click that
-- would fire chain 6501 is never sent. The turn-in would be unreachable.
--
-- `fire_mission_completed` (executor/mission.rs, gated on a real
-- active -> completed transition) runs AFTER 6527's
-- `remove_dialog_set 5253`, so slot 10 ends this event holding exactly
-- one bind. It populates world, mission and archetype context, so the
-- gates below evaluate normally.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6505, '1360 - Re-bind Marsh''s letter indicator when 1361 completes (step 4038 still open)', 'mission', 1360, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6505, 'mission_completed', '1361', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6505, 'world', 68, NULL, 'eq', NULL, 0),
  (6505, 'mission_status', 1360, NULL, 'eq', 'active', 1),
  (6505, 'step_status', 1360, '4038', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6505, 'add_dialog_set', 5356, NULL, '{"slot": 10, "mission_id": 1360}', 0, 0);

-- ============================================================
-- H30 / MISSION 567 -- Romney's Files, step 4039 (chains 6503-6504)
-- ============================================================
--
-- *** BOTH CHAINS DISABLED -- see U17. ***
--
-- Step 4039 "Deliver Romney's Files to Copplemann" (objective 4652) needs
-- item 2698 "Romney's Files". That item HAS NO GRANT PATH ANYWHERE in the
-- current seed, verified across every table that could source one:
-- `content_actions` (no add_item on 2698), `loot`, `mission_rewards`,
-- `items_event_sets`, `char_creation_choices`, `item_list_items`,
-- `blueprints` / `blueprints_components`. The Castle ledger's CA06
-- explicitly excludes it ("no acquisition evidence"), and steps 2000 and
-- 2012 (the Castle-side retrieve/extract legs) are likewise unauthored.
--
-- So the beat is authored to the same standard as 1360's and parked. When
-- a Castle packet grants 2698, flip `enabled` to true on BOTH rows below
-- and nothing else changes. Keeping the rows (rather than omitting them)
-- is the ledger's stated preference: it preserves the dialog evidence and
-- makes the handoff a one-word edit.
--
-- DIALOG EVIDENCE. dialog_set_maps row 2817 -> dialog 2044, set 512
-- ("Romney's Files"), interaction_flags 536870912
-- (INT_NonAStoryMissionTurnIn, the "?" turn-in icon). Dialog 2044 is
-- `DUIST_DefaultDialog`, ONE screen, speaker 968 (Copplemann), no buttons:
--   [968] "These files are going to help us out a lot. Nice work."
-- Speaker 968 is the Harset Copplemann; 1110 is a separate "Capt.
-- Copplemann (Castle)" speaker used by the Castle-side dialogs 2042/2043.
-- Template 48 is the Harset Copplemann (tag `CmdCenter_Copplemann`).

-- Chain 6503: hand the files to Copplemann.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6503, '567 - Copplemann interact (step 4039): take Romney''s Files, complete the mission (DISABLED: item 2698 has no grant path, U17)', 'mission', 567, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6503, 'interact_tag', 'CmdCenter_Copplemann', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6503, 'world', 68, NULL, 'eq', NULL, 0),
  (6503, 'mission_status', 567, NULL, 'eq', 'active', 1),
  (6503, 'step_status', 567, '4039', 'eq', 'active', 2);
  -- No DISJOINTNESS row needed: 6503 is the only chain in the repo keyed
  -- on 'CmdCenter_Copplemann'.

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6503, 'display_dialog', 2044, NULL, '{}', 0, 0),
  (6503, 'remove_item', 2698, NULL, '{"qty": 1}', 0, 1),
  (6503, 'remove_dialog_set', 2817, NULL, '{"slot": 48}', 0, 2),
  -- Terminal step of a 3-step mission; closes objective 4652 with it.
  (6503, 'complete_mission', 567, NULL, '{}', 0, 3);
  -- GC3: grant_xp -- same as 6501.

-- Chain 6504: relog restore for step 4039. Disabled with its sibling so
-- the "?" never appears on Copplemann for a step the player cannot finish.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6504, '567 - Restore Copplemann files binding on Command Center load (step 4039) (DISABLED with 6503, U17)', 'mission', 567, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6504, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6504, 'world', 68, NULL, 'eq', NULL, 0),
  (6504, 'step_status', 567, '4039', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6504, 'add_dialog_set', 2817, NULL, '{"slot": 48, "mission_id": 567}', 0, 0);

-- ============================================================
-- H31 / MISSION 1361 -- Meet The Praxis (chains 6511-6527)
-- ============================================================
--
-- Six strictly ordered talk steps, read from resources.mission_steps
-- (index in parentheses) and mission_objectives:
--   4040 (0) "Talk to Moh'katan."                            obj 4654
--   4041 (1) "Convince Requisitions Officer Hansen to Give
--             you the weapons"                               obj 4655
--   4042 (2) "Deliver the weapons samples to Moh'katan."      obj 4656
--   4043 (3) "Talk to Ba'al."                                obj 4657
--   4693 (4) "Talk to Anat."                                 obj 5572
--   4694 (5) "Return to Marsh."                              obj 5573
-- Every objective is non-optional; `advance_step` closes each one and
-- `complete_mission` closes 5573 (note F). Ordering is enforced by each
-- chain's own `step_status ... eq active` gate: a mission has exactly one
-- `current_step_id`, so exactly one step chain can ever match.
--
-- WORLDS. Steps 4040, 4042, 4043, 4693 and 4694 are in world 68
-- (Harset_CmdCenter); step 4041 (Hansen) is in world 57 (Harset). The
-- player therefore round-trips 68 -> 57 -> 68 across the Command Center
-- door. Because `cross_world_teleport` destroys and rebuilds the cell
-- entity, `available_interactions` do NOT survive the crossing, so a bind
-- for an NPC in the other world CANNOT be made by the chain that advances
-- the step — it has to come from the destination world's own
-- `player_loaded` restore chain. That is why 6515 does not bind Hansen
-- (6516 does) and 6518 does not bind Moh'katan (6519 does), while the
-- same-world hand-offs (6520 -> Ba'al, 6522 -> Anat, 6525 -> Marsh) bind
-- in-chain AND get a restore chain, matching Cellblock chain 1003.
--
-- ARCHETYPE. 1361 is the Human/OP-CORE arrival mission, so the offer is
-- gated `archetype neq 8` (Jaffa) and `neq 6` (Goa'uld) — the two values
-- are from entities/defs/enumerations.xml EArchetype. The Jaffa
-- equivalent is 1324 (packet H20) and the Goa'uld one is 1200 (H40).
-- Only the OFFER chains carry the gate: `fire_dialog_choice` does not
-- populate `archetype` at all, so a gate on 6513 would read the
-- evaluator's missing-param default and be silently always-true. The gate
-- is transitive instead — 4457 can only reach `dialog_choice` if THIS
-- server displayed it to THIS player (the #479 `open_dialog_id` gate in
-- `handle_dialog_button_choice`), and only 6512 displays it.
--
-- THE "WEAPON SAMPLES" ITEM DOES NOT EXIST -- decision H31-D1.
-- Step 4041's fiction is that Hansen hands over samples of Earth weaponry
-- which step 4042 delivers to Moh'katan. No such item is in
-- resources.items, and the spec's Items sheet has no id for it either
-- (audit.md, the 1361 row). An anti-join of every mission-bag item
-- (container_sets @> {2}) against `loot`, `mission_rewards`,
-- `items_event_sets`, `content_actions`, `item_list_items`, `blueprints`,
-- `blueprints_components` and `char_creation_choices` turned up no
-- unreferenced candidate that fits: the two closest by name, 2870
-- "Disassembled Weapon Components" and 7646 "Staff Weapon Blueprints",
-- BOTH carry live `items_event_sets` rows (item_event 130 -> ability 597,
-- and item_event 2628 -> ability 2924), so repurposing either would make
-- a double-click fire someone else's ability. The rest (2728, 3614, 3651,
-- 3685) are free but belong to other missions' fiction.
--
-- So NO item is granted and NO item is removed. The beat is carried by
-- dialogs 4459/4460/4466 and the step log text. Player-visible
-- consequence: no object appears in or leaves the mission bag between
-- 4041 and 4042. That is a strictly smaller gap than a wrong-fiction item
-- sitting in the bag for the length of the quest, and it leaves the door
-- open for a NEW `resources.items` row later (which is what a future
-- packet should add — never a repurposed id).
--
-- DIALOG EVIDENCE (all rows pre-existing; `slot` is the entity template):
--   dsm 5254 -> 4456  flags 134217728 (available "?")  spk 941 Marsh
--   dsm 5231 -> 4457  flags 0                          spk 941 Marsh
--   dsm 6397 -> 4458  flags 268435456 (active "!")     spk 945 Moh'katan
--   dsm 6399 -> 4459  flags 268435456                  spk 950 Hansen
--   dsm 6398 -> 4466  flags 268435456                  spk 945 Moh'katan
--   dsm 6395 -> 4461  flags 268435456                  spk 942 Ba'al
--   dsm 6396 -> 4462  flags 268435456                  spk 944 Anat
--   dsm 5253 -> 4465  flags 536870912 (turn-in "?")    spk 941 Marsh
-- Dialogs 4460 (Hansen relents) and 4463 (Anat accepts the flattery) have
-- NO dsm row — they are outcome dialogs displayed by a `dialog_choice`
-- chain, which is exactly why those two beats must use the bind path
-- (note C).
--
-- BUTTONS. Only three of these dialogs carry a button, and in every case
-- it sits on the dialog's LAST screen:
--   4456 screen 0/0 : id 8 type 2 "Accept" AND id 9 type 1 "More Info"
--   4457 screen 5/5 : id 8 type 2 "Accept"
--   4459 screen 2/2 : id 195 type 4 "Convince Hansen."
--   4462 screen 6/6 : id 194 type 4 "Flatter Anat."
-- The other five (4458, 4461, 4465, 4466, plus 4576/2044 above) have no
-- buttons at all and are pure blurbs.
--
-- 4456 IS DELIBERATELY NEVER DISPLAYED. `Trigger::OnDialogChoice` carries
-- only the dialog id — `button_id` reaches `ctx.params` but NO condition
-- type can read it (`PropertyEquals` has no loader arm). 4456's two
-- buttons are therefore indistinguishable at the trigger, so a chain keyed
-- on it would accept the mission when the player clicked "More Info".
-- 6512 displays 4457 instead: one button, on the last screen, unambiguous.
-- In the shipped data 4457 IS the "more info" row for this offer (set
-- 1419, flags 0 — the same pairing as dsm 2641/2642 "Strange Bedfellows"
-- and "Strange Bedfellows (More info)"), so showing it directly loses
-- nothing. Do NOT add a `dialog_choice '4456'` sibling as a safety net;
-- it would trade a narrow hole for a guaranteed mis-accept.

-- ------------------------------------------------------------
-- 1361 ACCEPTANCE (6511-6513) -- DISABLED pending M0.
-- ------------------------------------------------------------
--
-- *** ALL THREE DISABLED. Blocker: the return door is dark. ***
--
-- Step 4041 is at Hansen in world 57 and steps 4040/4042 are in world 68,
-- so the mission REQUIRES a working 68 -> 57 crossing. The only one is
-- chain 6007 (harset_space_chains.sql), which ships `enabled = false`
-- because `NavMesh::is_point_valid` rejects its 2009 arrival coordinate
-- (0, -67.600, -231) and `resolve_recovery_position` has nothing to offer
-- in world 57 — the player would be silently ghosted
-- (`CorrectionSuppressed`) rather than rubber-banded.
--
-- Shipping the acceptance path live against a dark return leg would soft-
-- stick every player who accepts 1361 at step 4041 with no recovery: there
-- is no `fail_objective` executor arm and no chain-authorable abandon. A
-- mission that cannot be started is a strictly better failure than one
-- that cannot be finished.
--
-- The campaign rule forbids inventing the coordinate here (every Harset
-- coordinate is recovered from the Python or pinned in M0), and chain 6007
-- belongs to packet H10's file, so this packet cannot fix it.
--
-- M0 MUST flip 6007 AND these three rows together. The biconditional is
-- pinned by `praxis_acceptance_is_enabled_iff_the_return_door_is` in
-- mission_1361.rs, so the two cannot drift apart silently.
--
-- 6514-6527 stay ENABLED: they are all `step_status`-gated on a mission
-- that cannot be accepted, so they are unreachable today and correct the
-- moment acceptance opens. Keeping them live means M0 flips three
-- booleans, not seventeen.

-- Chain 6511: put the "?" on Marsh for a Human who has not met the Praxis.
-- Doubles as the relog restore — `player_loaded` fires on every world
-- entry, so there is no separate restore chain for the offer.
--
-- 6511 AND 6512 MUST CARRY IDENTICAL CONDITIONS. If the bind can happen
-- while the display chain cannot fire, `fire_interact_tag` matches nothing
-- for 'CmdCenter_Marsh', `handled` stays false, and `handle_interact`
-- falls through to the bound dsm — which would render 4456 and its
-- "More Info" trap. Keep these two lists in lockstep; the invariant is
-- pinned by `offer_bind_and_offer_dialog_carry_identical_conditions`.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6511, '1361 - Offer indicator on Marsh for Humans (DISABLED: needs the 68->57 door, chain 6007/M0)', 'mission', 1361, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6511, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6511, 'world', 68, NULL, 'eq', NULL, 0),
  -- EArchetype 8 = ARCHETYPE_Jaffa, 6 = ARCHETYPE_Goauld
  -- (entities/defs/enumerations.xml:364,366). Two `neq` rows rather than
  -- one `eq`, because "Human" is four archetypes, not one.
  (6511, 'archetype', NULL, NULL, 'neq', '8', 1),
  (6511, 'archetype', NULL, NULL, 'neq', '6', 2),
  -- Canonical offer guard (content-chains.instructions.md).
  (6511, 'mission_status', 1361, NULL, 'eq', 'not_active', 3),
  -- DISJOINTNESS (note A) vs chain 6501, which also keys on Marsh: the
  -- letter is delivered first. Without this, a player arriving from the
  -- Cellblock with 1360 active at 4038 and 1361 untouched — the GUARANTEED
  -- first-visit state — would run 6501 and 6512 on one right-click, and
  -- 6512's `display_dialog 4457` would re-pin `open_dialog_id` and discard
  -- Marsh's letter blurb while 6501's item removal and completion still
  -- ran. Also keeps template slot 10 down to one live bind (note D).
  (6511, 'step_status', 1360, '4038', 'neq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6511, 'add_dialog_set', 5254, NULL, '{"slot": 10, "mission_id": 1361}', 0, 0);

-- Chain 6512: right-click Marsh -> the Praxis briefing.
-- Conditions are a byte-for-byte copy of 6511's; see the note there.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6512, '1361 - Marsh interact: show the Praxis briefing 4457 (DISABLED with 6511)', 'mission', 1361, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6512, 'interact_tag', 'CmdCenter_Marsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6512, 'world', 68, NULL, 'eq', NULL, 0),
  (6512, 'archetype', NULL, NULL, 'neq', '8', 1),
  (6512, 'archetype', NULL, NULL, 'neq', '6', 2),
  (6512, 'mission_status', 1361, NULL, 'eq', 'not_active', 3),
  (6512, 'step_status', 1360, '4038', 'neq', 'active', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6512, 'display_dialog', 4457, NULL, '{}', 0, 0);

-- Chain 6513: the player clicks Accept on 4457.
-- No archetype gate — see the ARCHETYPE note above.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6513, '1361 - Briefing 4457 accepted: accept Meet The Praxis, point at Moh''katan (DISABLED with 6511)', 'mission', 1361, false, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6513, 'dialog_choice', '4457', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6513, 'world', 68, NULL, 'eq', NULL, 0),
  (6513, 'mission_status', 1361, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6513, 'accept_mission', 1361, NULL, '{}', 0, 0),
  -- Drop the "?" from Marsh; 6514's bind puts the "!" on Moh'katan.
  (6513, 'remove_dialog_set', 5254, NULL, '{"slot": 10}', 0, 1),
  -- Same world (68), so the bind is made in-chain as well as by the
  -- restore chain 6514 — matching Cellblock chain 1003's shape.
  (6513, 'add_dialog_set', 6397, NULL, '{"slot": 54, "mission_id": 1361}', 0, 2);

-- ------------------------------------------------------------
-- Step 4040 -- talk to Moh'katan (6514-6515)
-- ------------------------------------------------------------

-- Chain 6514: relog / world-entry restore for step 4040.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6514, '1361 - Restore Moh''katan binding on Command Center load (step 4040)', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6514, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6514, 'world', 68, NULL, 'eq', NULL, 0),
  (6514, 'step_status', 1361, '4040', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6514, 'add_dialog_set', 6397, NULL, '{"slot": 54, "mission_id": 1361}', 0, 0);

-- Chain 6515: Moh'katan asks for samples of Earth weaponry, and the step
-- advances to the Hansen leg. Dialog 4458 is a five-screen blurb with no
-- buttons; the ask is screen 2:
--   [945] "There is something you could do to please me, however... I
--          would like to examine some of your earth weaponry... Can you
--          bring me samples of the hardware?"
--
-- No Hansen bind here: Hansen is in world 57 and this chain runs in 68,
-- so the bind would be destroyed by the crossing. Chain 6516 does it.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6515, '1361 - Moh''katan interact (step 4040): ask for weapon samples, advance to 4041', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6515, 'interact_tag', 'CmdCenter_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6515, 'world', 68, NULL, 'eq', NULL, 0),
  -- Disjoint from 6520 (same tag, step 4042) for free: one current step.
  (6515, 'step_status', 1361, '4040', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6515, 'display_dialog', 4458, NULL, '{}', 0, 0),
  (6515, 'remove_dialog_set', 6397, NULL, '{"slot": 54}', 0, 1),
  -- `advance_step` force-completes objective 4654 (note F).
  (6515, 'advance_step', 1361, '4041', '{}', 0, 2);

-- ------------------------------------------------------------
-- Step 4041 -- convince Hansen (6516-6518). BIND PATH, see note (C).
-- ------------------------------------------------------------
--
-- There is deliberately NO `interact_tag` chain for Hansen. The dsm 6399
-- bind alone makes him clickable (its interaction_flags carry the "!"),
-- `handle_interact` opens dialog 4459 AND pins `last_interaction_target`,
-- and the logic hangs off `dialog_choice`. That pin is what lets chain
-- 6518's follow-up `display_dialog 4460` bind Hansen's portrait; an
-- `interact_tag` chain would short-circuit `handle_interact`, leave the
-- pin unset, and make 4460 either abort with a warn or speak from a stale
-- NPC. Castle chains 1011/1012 -> 1014/1015/1020/1021 are the same shape.
--
-- Safe because template 212 never holds a second bind: 6399 is the only
-- dsm this file ever binds to slot 212, so `.first()` is unambiguous.

-- Chain 6516: bind Hansen on arrival in world 57.
-- This is the chain that makes the 68 -> 57 crossing work: 6515 advanced
-- the step while the player was in the Command Center, and the bind can
-- only be created on the Harset side.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6516, '1361 - Bind Hansen on Harset load (step 4041)', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6516, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6516, 'world', 57, NULL, 'eq', NULL, 0),
  (6516, 'step_status', 1361, '4041', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6516, 'add_dialog_set', 6399, NULL, '{"slot": 212, "mission_id": 1361}', 0, 0);

-- Chain 6517 is INTENTIONALLY ABSENT.
-- The id is left unused so the numbering stays aligned with the step
-- table; Hansen's dialog comes from the 6516 bind via `handle_interact`,
-- not from an `interact_tag` chain. See the BIND PATH note above before
-- "fixing" this by adding one — an `interact_tag` chain here would break
-- chain 6518's `display_dialog 4460`.

-- Chain 6518: the player clicks "Convince Hansen." on dialog 4459.
-- 4459 is three screens, speaker 950 (Hansen), button id 195 type 4 on
-- the last screen:
--   [950] "You want to give some of our weapons to the Jaffa? And I'm
--          just supposed to give these to you?"
--   [  0] "Yes."
--   [  0] "You better tell me one heck of a story for that to ever
--          happen..."   [Convince Hansen.]
-- Outcome dialog 4460, three screens, no dsm row, no buttons:
--   [950] "I cannot believe I am doing this..."
--   [  0] "The Colonel would approve..."
--   [950] "The Colonel will have my hide."
--
-- Per D-H12 this plays as a dialog choice, not a real Converse minigame
-- (Converse is a client SWF with a placeholder auto-win server-side).
-- There is no failure branch: the single button is the only affordance the
-- shipped data offers, so the check always succeeds. Recorded as a
-- fidelity note in worknotes/H30-H31.md rather than invented.
--
-- NO `add_item` here -- see decision H31-D1 above.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6518, '1361 - Hansen convinced (dialog 4459): show 4460, advance to 4042', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6518, 'dialog_choice', '4459', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6518, 'world', 57, NULL, 'eq', NULL, 0),
  (6518, 'step_status', 1361, '4041', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  -- Resolves Hansen through `last_interaction_target`, which
  -- `handle_interact` pinned when it opened 4459 (note C).
  (6518, 'display_dialog', 4460, NULL, '{}', 0, 0),
  (6518, 'remove_dialog_set', 6399, NULL, '{"slot": 212}', 0, 1),
  -- Closes objective 4655. The Moh'katan bind for 4042 is world 68's job
  -- (chain 6519) — this chain is running in world 57.
  (6518, 'advance_step', 1361, '4042', '{}', 0, 2);

-- ------------------------------------------------------------
-- Step 4042 -- deliver the samples to Moh'katan (6519-6520)
-- ------------------------------------------------------------

-- Chain 6519: bind Moh'katan on return to the Command Center.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6519, '1361 - Restore Moh''katan binding on Command Center load (step 4042)', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6519, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6519, 'world', 68, NULL, 'eq', NULL, 0),
  (6519, 'step_status', 1361, '4042', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6519, 'add_dialog_set', 6398, NULL, '{"slot": 54, "mission_id": 1361}', 0, 0);

-- Chain 6520: hand the samples over. Dialog 4466 is a one-screen blurb,
-- speaker 945, no buttons:
--   [945] "You did this without concern for our differences... This
--          impresses me. Because you understand the value of honor, I
--          will request that you are my liasion officer in the future."
-- No `remove_item`: nothing was granted (H31-D1).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6520, '1361 - Moh''katan interact (step 4042): deliver the samples, advance to 4043', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6520, 'interact_tag', 'CmdCenter_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6520, 'world', 68, NULL, 'eq', NULL, 0),
  (6520, 'step_status', 1361, '4042', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6520, 'display_dialog', 4466, NULL, '{}', 0, 0),
  (6520, 'remove_dialog_set', 6398, NULL, '{"slot": 54}', 0, 1),
  (6520, 'advance_step', 1361, '4043', '{}', 0, 2),
  -- Ba'al is in the same world, so bind in-chain as well as via 6521.
  (6520, 'add_dialog_set', 6395, NULL, '{"slot": 42, "mission_id": 1361}', 0, 3);

-- ------------------------------------------------------------
-- Step 4043 -- talk to Ba'al (6521-6522)
-- ------------------------------------------------------------

-- Chain 6521: relog restore for step 4043.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6521, '1361 - Restore Ba''al binding on Command Center load (step 4043)', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6521, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6521, 'world', 68, NULL, 'eq', NULL, 0),
  (6521, 'step_status', 1361, '4043', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6521, 'add_dialog_set', 6395, NULL, '{"slot": 42, "mission_id": 1361}', 0, 0);

-- Chain 6522: Ba'al sizes the player up. Dialog 4461, four screens,
-- speaker 942, no buttons.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6522, '1361 - Ba''al interact (step 4043): advance to 4693', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6522, 'interact_tag', 'CmdCenter_Baal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6522, 'world', 68, NULL, 'eq', NULL, 0),
  (6522, 'step_status', 1361, '4043', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6522, 'display_dialog', 4461, NULL, '{}', 0, 0),
  (6522, 'remove_dialog_set', 6395, NULL, '{"slot": 42}', 0, 1),
  (6522, 'advance_step', 1361, '4693', '{}', 0, 2),
  (6522, 'add_dialog_set', 6396, NULL, '{"slot": 43, "mission_id": 1361}', 0, 3);

-- ------------------------------------------------------------
-- Step 4693 -- talk to Anat (6523-6525). BIND PATH, see note (C).
-- ------------------------------------------------------------
--
-- Like Hansen, Anat's beat ends in a button whose outcome dialog (4463)
-- has no dsm row, so it must come from `dialog_choice` -> `display_dialog`
-- and therefore needs `last_interaction_target`. No `interact_tag` chain.
--
-- ANAT'S SPAWN NEEDS A TAG ANYWAY. spawn 222 (world 68, template 43) is
-- the only Anat row and its `tag` column is EMPTY, so no `interact_tag`
-- chain could address her even if we wanted one. The bind path sidesteps
-- that entirely — `add_dialog_set` targets a TEMPLATE slot, not a tag.
-- H12 should still give spawn 222 the tag `CmdCenter_Anat` (recorded in
-- worknotes/H30-H31.md and in the tag registry) for later packets; nothing in
-- THIS file depends on it.
--
-- Anat also carries the game's only `entity_interactions` row (id 35,
-- template 43, dsm 3127, gated `missions_not_accepted {742}`) — the
-- Goa'uld 742 offer. It cannot collide with the 6523 bind for two
-- independent reasons, either of which alone is sufficient:
--   1. `entity_interactions` has NO Rust consumer at all — `grep -rn
--      entity_interactions crates/` returns nothing, so the row never
--      reaches the runtime. (Recorded as a finding in H30-H31.md: the
--      table is shipped 2009 data with no loader.)
--   2. Even if it were loaded, 742 is Goa'uld-only (archetype 6) and 1361
--      is Human-only, so no character can be in both states.
-- Do not "fix" this by removing the 6523 bind.

-- Chain 6523: relog restore for step 4693.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6523, '1361 - Restore Anat binding on Command Center load (step 4693)', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6523, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6523, 'world', 68, NULL, 'eq', NULL, 0),
  (6523, 'step_status', 1361, '4693', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6523, 'add_dialog_set', 6396, NULL, '{"slot": 43, "mission_id": 1361}', 0, 0);

-- Chain 6524 is INTENTIONALLY ABSENT -- same reason as 6517. Anat's
-- dialog 4462 comes from the 6523 bind via `handle_interact`, which is
-- what pins the target for chain 6525's `display_dialog 4463`.

-- Chain 6525: the player clicks "Flatter Anat." on dialog 4462.
-- 4462 is seven screens, speaker 944, button id 194 type 4 on the last:
--   [944] "You hesitate. Do not lie to me human... Do you find this body
--          pleasing to gaze upon?"   [Flatter Anat.]
-- Outcome dialog 4463, three screens, no dsm row, no buttons:
--   [944] "I believe you... You can cease groveling... I find it
--          unattractive."
-- Another D-H12 dialog-choice social check with no authored failure
-- branch.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6525, '1361 - Anat flattered (dialog 4462): show 4463, advance to 4694', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6525, 'dialog_choice', '4462', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6525, 'world', 68, NULL, 'eq', NULL, 0),
  (6525, 'step_status', 1361, '4693', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6525, 'display_dialog', 4463, NULL, '{}', 0, 0),
  (6525, 'remove_dialog_set', 6396, NULL, '{"slot": 43}', 0, 1),
  (6525, 'advance_step', 1361, '4694', '{}', 0, 2),
  -- Marsh's turn-in "?" — same world, so bind in-chain plus 6526.
  (6525, 'add_dialog_set', 5253, NULL, '{"slot": 10, "mission_id": 1361}', 0, 3);

-- ------------------------------------------------------------
-- Step 4694 -- return to Marsh (6526-6527)
-- ------------------------------------------------------------

-- Chain 6526: relog restore for the turn-in.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6526, '1361 - Restore Marsh turn-in binding on Command Center load (step 4694)', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6526, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6526, 'world', 68, NULL, 'eq', NULL, 0),
  (6526, 'step_status', 1361, '4694', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6526, 'add_dialog_set', 5253, NULL, '{"slot": 10, "mission_id": 1361}', 0, 0);

-- Chain 6527: Marsh debriefs and the mission ends. Dialog 4465, five
-- screens, speaker 941, no buttons:
--   [941] "They are an eclectic bunch, aren't they?"
--   ...
--   [941] "You do not always go into battle with the army you want... You
--          go into battle with the army you have."
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6527, '1361 - Marsh interact (step 4694): debrief, complete Meet The Praxis', 'mission', 1361, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6527, 'interact_tag', 'CmdCenter_Marsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6527, 'world', 68, NULL, 'eq', NULL, 0),
  -- Disjoint from 6512 by construction (6512 needs 1361 `not_active`).
  -- Disjoint from 6501 via 6501's own `step_status 1361/4694 neq active`.
  (6527, 'mission_status', 1361, NULL, 'eq', 'active', 1),
  (6527, 'step_status', 1361, '4694', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6527, 'display_dialog', 4465, NULL, '{}', 0, 0),
  (6527, 'remove_dialog_set', 5253, NULL, '{"slot": 10}', 0, 1),
  -- Terminal step; closes objective 5573 with the mission (note F).
  (6527, 'complete_mission', 1361, NULL, '{}', 0, 2);
  -- GC3: grant_xp -- same as 6501.
