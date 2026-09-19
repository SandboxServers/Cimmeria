-- ============================================================
-- Harset Goa'uld mission chains (chain ids 6101-6300)
-- ============================================================
-- Campaign: docs/analysis/harset-rebuild/ (ledger packets H40-H47).
-- Authoring rules: work-packets.md "Worker Input And Ownership";
-- canonical tags: worknotes/harset-tags.md; dialog-set-map ids 120201-120300.
-- Every coordinate in this file must be recovered from the 2009 Python
-- or pinned in M0; mission-scoped hostiles are spawned into the player's
-- own Market/Storage instance, never into world 57 or 68.
-- Sub-allocation per mission is fixed in the ledger table; never reuse a
-- 1xxx-5xxx id.
--
-- Packets landed in this file so far:
--   H41 -- mission 742 "Giving the Walls Ears"  : 6101-6119 (6120 spare)
--   H40 -- mission 1200 "Meet Your Queen"       : 6121-6127 (6128-6130 spare)
-- Base: content/harset-wave2 @ 94e65324.
--
-- Worknotes: docs/analysis/harset-rebuild/worknotes/H41.md and H40.md.
-- Replay guards: crates/services/src/cell/content/chain_replay_tests/
--   mission_742.rs and mission_1200.rs.
-- ============================================================

SET search_path = resources, pg_catalog;

-- ============================================================
-- SHARED AUTHORING NOTES FOR THIS FILE
-- ============================================================
--
-- NPC QUEST ICONS ARE NOT `set_interaction_type` ROWS. Every
-- `add_dialog_set` in this file pushes the bound `dialog_set_maps` row's
-- own `interaction_flags` to the NPC automatically
-- (executor/dialog.rs:356, `base_flags | entry.interaction_flags`), and
-- `remove_dialog_set` re-folds the remaining entries
-- (executor/dialog.rs:216). So the "!"/"?" markers on Petbe, Anat, Nerus,
-- Ba'al and the Royal Guard ride the dsm rows and need no action of their
-- own. `set_interaction_type` is used ONLY for the three bug baskets,
-- which are addressed by spawn tag rather than by dialog binding.
--
-- This is not a style choice. `set_interaction_type` resolves its target
-- with `find_entity_by_tag` (executor/world/mod.rs:28) and spawnlist rows
-- 222 (Anat) and 223 (Petbe) both carry a NULL `tag`, so a
-- `set_interaction_type` row aimed at either NPC could never resolve.
-- Templates are the only handle these two have.
--
-- `add_dialog_set <dsm_id>` puts the ENTITY TEMPLATE id in
-- `params.slot` -- not a UI slot. `handle_interact` looks the player's
-- bindings up by the target's `template_id`
-- (dispatch/interact.rs:95-101) and takes `entries.first()`, so the
-- FIRST binding on a template wins and per-player bindings outrank the
-- entity's static interaction type. Two live bindings on one template is
-- therefore a content bug, not a merge: see the 742/1200 Anat note at
-- chain 6118.
--
-- WORLD CONDITIONS. Per the campaign rule every chain that fires in a
-- shared space carries a `world` condition. Applied here to exactly the
-- chains whose trigger is PLACE-bound -- the three basket `interact_tag`
-- chains and every `player_loaded` restore chain. Deliberately NOT
-- applied to the `mission_accepted`, `item_use` or `dialog_choice`
-- chains: those fire on player state rather than on location, and a
-- world gate on them would break legitimate play (the disguise may be
-- double-clicked in the Command Center; 742 may be accepted in either
-- world). Recorded in worknotes/H41.md, IR-2.
--
-- ONE-SHOT GUARDS ARE CONDITIONS, NOT `once`. `content_triggers.once` is
-- loaded and then dropped on the floor (agent-memory
-- mission-systems-advisor/content-engine-once-semantics.md); every
-- one-shot in this file is a `step_status` / `mission_status` /
-- `objective_status` gate that the chain's own `advance_step` flips
-- false.
--
-- ============================================================
-- KNOWN BLOCKER: PER-OBJECTIVE STATE IS NOT PERSISTED  (H41-B1)
-- ============================================================
--
-- Every `objective_status` condition in this file is correct WITHIN a
-- session and evaluates FALSE after any relog, gate hop or cross-world
-- teleport, because objective ids never reach the database. Found while
-- authoring H40/H41; it is an engine defect, not a Harset one, and it
-- breaks Castle chain 1109 (`objective_status 688 2734 eq completed`) in
-- exactly the same way. Four sites, all verified by reading:
--
--   1. executor/mission.rs:84-85 (accept) and :241-242 (advance_step)
--      send `completed_objective_ids: vec![]` and
--      `active_objective_ids: vec![step_id]` -- the STEP id is written
--      into the OBJECTIVE array. :172-173 (complete) sends both empty.
--   2. `Action::CompleteObjective` (executor/mission.rs:273-291) sends
--      NO `MissionUpdate` at all -- only the `onObjectiveUpdate` wire
--      frame. Objective completion is a client checkmark plus in-memory
--      cell state and nothing else.
--   3. Hydration faithfully restores the wrong thing:
--      base_messages/player_init/mod.rs:172-188 rebuilds
--      `active_objectives` from `saved.active_objective_ids` with
--      `hidden: false, optional: false` HARDCODED.
--
-- So a player who relogs on 742 step 2504 comes back with
-- `active_objectives = [{ objective_id: 2504 }]` -- the step id wearing
-- an objective's clothes. 2913/2914/2915 appear in neither
-- `active_objectives` nor `completed_objectives`, `populate_mission_
-- context` emits no `mission_742_obj_29xx_status` key, and
-- `Condition::ObjectiveStatus` takes its `unwrap_or("not_active")`
-- fallback (conditions.rs:271).
--
-- AFFECTED HERE: 6104-6109 (the baskets), 6113-6115 (their restore),
-- 6124 and 6127 (1200's hidden optional). Everything gated only on
-- `step_status` is durable and unaffected -- `current_step_id` and
-- `completed_step_ids` both round-trip -- which is 6101-6103, 6110-6112,
-- 6116-6119 and 6121-6123, 6125-6126.
--
-- NOT WORKED AROUND, DELIBERATELY. `step_status` is the only durable
-- mission predicate today and it cannot distinguish which of step
-- 2504's three baskets is planted; `Condition::HasItem` has no populator
-- and no loader arm, and `CellEntity.counters` are session-only too.
-- There is no correct seed-only shape, and inventing extra step ids
-- would need a MissionOverride and would force the baskets into a fixed
-- order -- losing the any-order behaviour the 2009 script had. The rows
-- below are therefore authored against the semantics the engine is
-- MEANT to have; they start working the moment the persistence fix
-- lands and need no edit here.
--
-- The fix (owner: rust-gameserver-dev + database-persistence, tracked as
-- H41-B1 in worknotes/H41.md): populate both objective arrays from
-- `MissionInstance.active_objectives` / `.completed_objectives` after
-- each mutation, add a `MissionUpdate` send to the `complete_objective`
-- arm, and carry `hidden`/`optional` through hydration -- the last one
-- matters on its own, because a restored optional objective currently
-- counts toward `all_required_complete` (progression.rs:176-180) and can
-- complete a mission early.

-- ============================================================
-- H41 -- MISSION 742 "Giving the Walls Ears"  (chains 6101-6119)
-- ============================================================
--
-- THE ONE RESTORE IN THE HARSET CAMPAIGN. Harset kept exactly three
-- surviving 2009 scripts and this is the only mission among them, so
-- unlike every other Harset mission packet this file section is a port,
-- not a reconstruction:
--
--   deprecated/python/cell/missions/Harset/GivingTheWallsEars.py  (261 lines)
--   deprecated/data-scripts/scripts/missions/Harset/GivingTheWallsEars.script
--
-- The `.script` is the ground truth and the `.py` is its compiler
-- output; where they disagree the `.script` wins, because the compiler
-- emitted literal `None` for every unconnected input port and that is
-- what produced the dead nodes the audit catalogued. Node ids below are
-- `.script` `<Node Id="N">` values. Evidence class: RECOVERED_SCRIPT for
-- every chain except 6118/6119 (see their own comment).
--
-- Step chain (resources.mission_steps):
--   2502 Get a disguise from Petbe
--   2503 Put on the Disguise
--   2504 Hide the listening devices in the Jaffa area in Harset
--        objectives 2913 / 2914 / 2915, all non-optional
--   2505 Report to Anat
--   2506 Give the device Map to Nerus
--
-- Cast and where they stand:
--   Petbe  template 163, spawn 223, world 57 (NULL tag)
--   Anat   template  43, spawn 222, world 68 (NULL tag)
--   Nerus  template  53, world 68 -- spawn row is H12's, still pending M0
--   baskets template 164: spawn 224 `FirstBug` (world 57) exists;
--          `SecondBug` and `ThirdBug` are H14's, pending M0
--
-- THE ONE DELIBERATE DEVIATION FROM THE PYTHON (decision D-H05).
-- The 2009 bug-planting step hung off `Event_DialogSetMap` (node 17) on
-- dialog-set-map 1000000, i.e. "the player opened the bound topic on a
-- basket". That shape cannot be ported:
--   * the `dialog_set_open` trigger is authorable but has ZERO dispatch
--     sites repo-wide -- no `fire_*` ever constructs it, so a chain on it
--     is dead on arrival (audit defect H-B4);
--   * dsm 1000000 has a NULL `dialog_id` and `load_dialog_set_maps`
--     (spawner/dialogs.rs:28) skips those rows outright, so
--     `add_dialog_set 1000000` would take the cache-miss warn branch and
--     bind nothing;
--   * even if both were fixed, the trigger carries no target tag, and
--     the Python only got one by reading `args['target']` and calling
--     `Act_GetProperty` (node 18) on it -- there is no chain-level
--     equivalent.
-- So the three baskets become three `interact_tag` chains. The player
-- right-clicks the basket instead of opening a topic on it. The
-- substitution is exact in one respect worth recording: dsm 1000000
-- carries `interaction_flags = 1073741824` = `INT_MissionWorldObject`,
-- which is precisely the bit chain 6103 paints on the three tags. The
-- 2009 data itself is the authority for the mask; nothing was invented.

-- ------------------------------------------------------------
-- Chain 6101: mission accepted -> bind Petbe, grant three Scarabs
-- `.script` node 1 (Event_MissionUpdate 742, port "Started") -> node 7
-- (Act_AddDialog dsm 3129 on template 163) and node 52 (Act_GiveItems
-- design 2820 quantity 3).
-- ------------------------------------------------------------
--
-- `mission_accepted` is the right trigger and it cannot double-grant.
-- The event is fired from the executor's combined
-- `AcceptMission | AdvanceMission` branch (executor/mission.rs:24-27),
-- but `advance_step` is a SEPARATE executor arm (`Action::AdvanceStep`,
-- executor/mod.rs:213) and `AdvanceMission` has no seed verb at all, so
-- no `advance_step` row in this file can re-enter it. The branch also
-- skips the follow-up event entirely when the #411 offer guard refuses a
-- re-accept (executor/mission.rs:64-72), so a second accept of 742
-- grants nothing.
--
-- No world condition: 742 may legitimately be accepted in world 68 (Anat
-- offers it, chain 6119) while Petbe stands in world 57. The binding is
-- stored per-player in `available_interactions` and is world-agnostic;
-- the icon push is best-effort and the AoI create cascade re-applies it
-- when the player reaches Petbe.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6101, '742 - Accepted: bind Petbe''s disguise topic and grant three Scarab listening devices', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6101, 'mission_accepted', '742', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6101, 'mission_status', 742, NULL, 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6101, 'add_dialog_set', 3129, NULL, '{"slot": 163, "mission_id": 742}', 0, 0),
  (6101, 'add_item', 2820, NULL, '{"qty": 3}', 0, 1);

-- ------------------------------------------------------------
-- Chain 6102: Petbe hands over the Jaffa disguise
-- `.script` node 8 (Event_DialogChoice 2638) -> node 9 (Act_GiveItems
-- 2819 x1); node 9's "Added" port -> node 10 (Act_RemoveDialog dsm 3129,
-- template 163) and node 11 (Act_AdvanceMission step 2503).
-- ------------------------------------------------------------
--
-- Dialogs 2636-2640 have ZERO `dialog_screen_buttons` rows, yet the 2009
-- script subscribes to `dialog.choice::2638`, `::2639` and `::2640`. The
-- client therefore emits a choice for the closing button of a
-- `DUIST_DefaultDialog` that declares no buttons of its own, and
-- `dialog_choice` is the faithful port. (The server still gates the
-- event on `CellEntity::open_dialog_id == dialog_id`, so a forged choice
-- for a dialog that was never displayed is dropped -- CAT-J-01 / #479.)
--
-- The Python ordered the grant BEFORE the unbind and the advance,
-- because both hung off `Act_GiveItems`'s "Added" success port. The
-- `sort_order` below preserves that: a failed grant cannot be modelled
-- as a branch here, but keeping the order means a reader of the seed
-- sees the same sequence the graph did.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6102, '742 - Petbe: hand over the Jaffa Disguise, clear his topic, advance to step 2503', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6102, 'dialog_choice', '2638', 'player', false, 0);

-- One-shot guard: this flips false the instant the chain's own
-- `advance_step` runs, so a second pass through Petbe's dialog grants no
-- second disguise.
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6102, 'step_status', 742, '2502', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6102, 'add_item', 2819, NULL, '{"qty": 1}', 0, 0),
  (6102, 'remove_dialog_set', 3129, NULL, '{"slot": 163}', 0, 1),
  (6102, 'advance_step', 742, '2503', '{}', 0, 2);

-- ------------------------------------------------------------
-- Chain 6103: wearing the disguise opens the Jaffa area
-- `.script` node 12 (Event_Item design 2819, Event=5 "use",
-- Fire Only Once=true) -> node 13 (Act_AdvanceMission 2504); node 13's
-- "Out" -> node 16 (Act_Dialog 2637) and node 14 (Act_AddDialog dsm
-- 1000000 on template 164).
-- ------------------------------------------------------------
--
-- Node 14 is the D-H05 substitution point: the 2009 bind of the
-- NULL-dialog indicator row 1000000 onto template 164 becomes three
-- explicit `set_interaction_type` rows, one per basket tag, carrying the
-- exact mask that dsm row declares (`INT_MissionWorldObject`,
-- 1073741824).
--
-- The disguise is NOT consumed. There is no `Act_RemoveItems` for 2819
-- anywhere in the `.script`; it is worn for the whole Jaffa-area
-- sequence, and `UseInventoryItem` no longer auto-consumes
-- (content-chains.instructions.md, "Inventory consumption"), so omitting
-- `remove_item` is both faithful and sufficient. The 2009 `Fire Only
-- Once = true` is honoured by the `step_status 2503 eq active` gate,
-- which the chain's own `advance_step` flips -- a second double-click
-- re-resolves nothing and cannot re-display 2637 or re-paint the bits.
--
-- `display_dialog 2637` has no NPC to bind to: `item_use` does not stamp
-- `target_entity_id`. Every screen of 2637 carries `speaker_id = 0`, so
-- it is in the monologue cache and the executor binds the player as the
-- context entity (executor/dialog.rs:68-79) rather than bailing. That
-- matches the Python exactly -- node 16's NPC port is unconnected and
-- the compiler emitted `displayDialog(None, 2637)`.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6103, '742 - Use the Jaffa Disguise: advance to step 2504, blurb 2637, light up the three bug baskets', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6103, 'item_use', '2819', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6103, 'step_status', 742, '2503', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6103, 'advance_step', 742, '2504', '{}', 0, 0),
  (6103, 'display_dialog', 2637, NULL, '{}', 0, 1),
  (6103, 'set_interaction_type', NULL, 'FirstBug',  '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 2),
  (6103, 'set_interaction_type', NULL, 'SecondBug', '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 3),
  (6103, 'set_interaction_type', NULL, 'ThirdBug',  '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 4);

-- ============================================================
-- Chains 6104-6109: planting the three devices (step 2504)
-- ============================================================
--
-- `.script` nodes 17-32. The Python read the interacted entity's tag
-- (node 18), compared it against the three string constants (nodes
-- 19/20/21 vs 22/23/24), checked that the matching objective was still
-- Pending (nodes 50/49/47 `Act_GetMissionObj`), completed it (nodes
-- 25/26/27), and ticked a `Counter_Int` with A=3 (node 28). The
-- counter's "Value == A" port advanced to step 2505 (node 31) and bound
-- Anat (node 32); both "Value < A" and "Value == A" fed node 30,
-- `Act_RemoveItems` design 2820 quantity 1.
--
-- TWO ENGINE FACTS SHAPE THE PORT.
--
-- (1) THE COUNTER IS REPLACED BY THE OBJECTIVES THEMSELVES. Content-engine
-- counters live in `CellEntity.counters` and are NOT persisted
-- (content-engine.md section 8) -- a player who plants two devices, logs
-- out and comes back would resume at zero and the third basket would
-- never reach the threshold. Per-objective status IS persisted and is
-- populated into the context as `mission_742_obj_<id>_status`
-- (mission_context.rs:137-170), so "this is the third basket" is
-- expressed as "my objective is active and the other two are already
-- completed". That is order-independent by construction: whichever
-- basket is clicked last matches its own final chain.
--
-- (2) `complete_objective` ON THE LAST REQUIRED OBJECTIVE ENDS THE WHOLE
-- MISSION. `cell/missions/progression.rs:176-183` calls
-- `mission.complete()` as soon as every non-optional objective of the
-- current step is complete. Step 2504's three objectives are all
-- non-optional, so a naive three-way `complete_objective` would finish
-- 742 at step 2504 and skip both 2505 and 2506 -- the player would never
-- report to Anat or reach Nerus.
--
-- THE SPLIT. Each basket therefore gets two chains:
--
--   * a PARTIAL chain at priority 0 (6104-6106) that completes its own
--     objective, consumes one Scarab and clears its own glow;
--   * a FINAL chain at priority 10 (6107-6109) that fires only when the
--     other two objectives are already completed, and does the step
--     advance and the Anat bind.
--
-- On the third click both resolve. Bucket order is descending priority
-- (chain.rs:86) and `execute_actions` runs the resolved list in that
-- order, so the FINAL chain's `advance_step 2505` runs first. That call
-- force-completes the one still-active objective and emits its
-- `onObjectiveUpdate(COMPLETED)` (progression.rs:57-66, 114-128), so the
-- client still ticks all three. The PARTIAL chain's `complete_objective`
-- then runs against an objective that is no longer in
-- `active_objectives`; `MissionInstance::complete_objective`
-- (crates/entity/src/missions.rs:101-113) returns false, and the
-- executor's `progression::complete_objective` early-returns on false
-- (progression.rs:155-157) -- no duplicate wire message, and critically
-- no second `all_required_complete` check. Its `remove_item` and glow
-- clear still run, which is what makes the third basket consume its
-- Scarab like the other two.
--
-- This ordering is load-bearing, so mission_742.rs asserts the resolved
-- action ORDER on the third click, not just the set.
--
-- ON H-B15 (the "orphaned" Scarab removal). The audit records that the
-- per-bug `Act_RemoveItems(2820)` is orphaned and the player keeps all
-- three devices. That is half right and the half that matters is the
-- other one: node 30 IS wired to the counter -- `.script` lines 468-469
-- connect both `Value < A` and `Value == A` to its `In` port -- so the
-- removal was meant to fire on all three ticks. What is unconnected is
-- node 30's `Player` INPUT port (`Flags="2"`, no incoming Connection),
-- which is why the compiler emitted `if None and (2820 or None)` /
-- `None.inventory.removeItemByDesign(...)` at
-- GivingTheWallsEars.py:128-135. The 2009 build shipped a data bug, not
-- a design decision: three granted, three consumed was the intent.
-- DECISION: consume one 2820 per basket. Recorded in worknotes/H41.md
-- and flagged for the H99 correction to mission-chains.md:1017, which
-- states the consumption as fact and was therefore accidentally right
-- about the intent and wrong about the shipped behaviour.
--
-- World gate: the baskets are world-57 entities and
-- `set_interaction_type` resolves tags only within the acting player's
-- own space, so these six chains carry `world eq 57`.

-- Chain 6104: FirstBug partial -- objective 2913.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6104, '742 - Plant device in FirstBug basket: complete objective 2913, consume one Scarab', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6104, 'interact_tag', 'FirstBug', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6104, 'world', 57, NULL, 'eq', NULL, 0),
  (6104, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6104, 'objective_status', 742, '2913', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6104, 'complete_objective', 742, '2913', '{}', 0, 0),
  (6104, 'remove_item', 2820, NULL, '{"qty": 1}', 0, 1),
  (6104, 'set_interaction_type', NULL, 'FirstBug', '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 2);

-- Chain 6105: SecondBug partial -- objective 2914.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6105, '742 - Plant device in SecondBug basket: complete objective 2914, consume one Scarab', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6105, 'interact_tag', 'SecondBug', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6105, 'world', 57, NULL, 'eq', NULL, 0),
  (6105, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6105, 'objective_status', 742, '2914', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6105, 'complete_objective', 742, '2914', '{}', 0, 0),
  (6105, 'remove_item', 2820, NULL, '{"qty": 1}', 0, 1),
  (6105, 'set_interaction_type', NULL, 'SecondBug', '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 2);

-- Chain 6106: ThirdBug partial -- objective 2915.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6106, '742 - Plant device in ThirdBug basket: complete objective 2915, consume one Scarab', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6106, 'interact_tag', 'ThirdBug', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6106, 'world', 57, NULL, 'eq', NULL, 0),
  (6106, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6106, 'objective_status', 742, '2915', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6106, 'complete_objective', 742, '2915', '{}', 0, 0),
  (6106, 'remove_item', 2820, NULL, '{"qty": 1}', 0, 1),
  (6106, 'set_interaction_type', NULL, 'ThirdBug', '{"op": "~", "mask": "INT_MissionWorldObject"}', 0, 2);

-- ------------------------------------------------------------
-- Chains 6107-6109: the third basket, whichever it is
-- `.script` node 28 "Value == A" -> node 31 (Act_AdvanceMission 2505)
-- -> node 32 (Act_AddDialog dsm 3130 on template 43, Anat).
-- ------------------------------------------------------------
--
-- PRIORITY 10 IS REQUIRED, not decorative -- see the long note above.
-- Each of these three is the same chain with the roles of the three
-- objectives rotated, because AND-only conditions cannot express "the
-- other two, whichever they are".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6107, '742 - Third device planted (FirstBug last): advance to step 2505 and bind Anat''s report topic', 'mission', 742, true, 10);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6107, 'interact_tag', 'FirstBug', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6107, 'world', 57, NULL, 'eq', NULL, 0),
  (6107, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6107, 'objective_status', 742, '2913', 'eq', 'active', 2),
  (6107, 'objective_status', 742, '2914', 'eq', 'completed', 3),
  (6107, 'objective_status', 742, '2915', 'eq', 'completed', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6107, 'advance_step', 742, '2505', '{}', 0, 0),
  (6107, 'add_dialog_set', 3130, NULL, '{"slot": 43, "mission_id": 742}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6108, '742 - Third device planted (SecondBug last): advance to step 2505 and bind Anat''s report topic', 'mission', 742, true, 10);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6108, 'interact_tag', 'SecondBug', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6108, 'world', 57, NULL, 'eq', NULL, 0),
  (6108, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6108, 'objective_status', 742, '2914', 'eq', 'active', 2),
  (6108, 'objective_status', 742, '2913', 'eq', 'completed', 3),
  (6108, 'objective_status', 742, '2915', 'eq', 'completed', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6108, 'advance_step', 742, '2505', '{}', 0, 0),
  (6108, 'add_dialog_set', 3130, NULL, '{"slot": 43, "mission_id": 742}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6109, '742 - Third device planted (ThirdBug last): advance to step 2505 and bind Anat''s report topic', 'mission', 742, true, 10);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6109, 'interact_tag', 'ThirdBug', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6109, 'world', 57, NULL, 'eq', NULL, 0),
  (6109, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6109, 'objective_status', 742, '2915', 'eq', 'active', 2),
  (6109, 'objective_status', 742, '2913', 'eq', 'completed', 3),
  (6109, 'objective_status', 742, '2914', 'eq', 'completed', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6109, 'advance_step', 742, '2505', '{}', 0, 0),
  (6109, 'add_dialog_set', 3130, NULL, '{"slot": 43, "mission_id": 742}', 0, 1);

-- ------------------------------------------------------------
-- Chain 6110: Anat takes the report and hands over the Scarab Map
-- `.script` node 33 (Event_DialogChoice 2639) -> node 34
-- (Act_RemoveDialog dsm 3130, template 43); node 34's "Successful" port
-- -> node 40 (Act_GiveItems 2864), node 41 (Act_AdvanceMission 2506) and
-- node 42 (Act_AddDialog dsm 3131 on template 53, Nerus).
-- ------------------------------------------------------------
--
-- The Python gated the whole downstream branch on the unbind SUCCEEDING
-- (`if ... removeDialog(43, dialogSet): self.n34_propagator_Successful()`
-- at GivingTheWallsEars.py:241-242). `remove_dialog_set` has no failure
-- branch to gate on, so the `step_status 2505 eq active` condition does
-- the same job: it is true exactly once, and the chain's own
-- `advance_step` retires it.
--
-- Nerus's bind happens here rather than on arrival because the player is
-- standing in world 68 at this moment -- Anat (spawn 222) and Nerus are
-- both Command Center NPCs, so the icon push lands immediately. Chain
-- 6117 re-applies it on a later world entry.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6110, '742 - Anat: take the report, grant the Scarab Map, advance to step 2506, bind Nerus', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6110, 'dialog_choice', '2639', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6110, 'step_status', 742, '2505', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6110, 'remove_dialog_set', 3130, NULL, '{"slot": 43}', 0, 0),
  (6110, 'add_item', 2864, NULL, '{"qty": 1}', 0, 1),
  (6110, 'advance_step', 742, '2506', '{}', 0, 2),
  (6110, 'add_dialog_set', 3131, NULL, '{"slot": 53, "mission_id": 742}', 0, 3);

-- ------------------------------------------------------------
-- Chain 6111: Nerus takes the Scarab Map -- mission complete
-- `.script` node 43 (Event_DialogChoice 2640) -> node 44
-- (Act_RemoveItems 2864 x1) and node 45 (Act_UpdateMission, port
-- "Complete").
-- ------------------------------------------------------------
--
-- `complete_mission` routes through `complete_mission_direct`, which
-- force-completes step 2506's objective 2917 and the mission in one
-- call -- the Python's explicit `missions.complete(742)` has the same
-- shape, so `complete_objective` is deliberately NOT used here.
--
-- The unbind is ordered BEFORE the completion so the "!" leaves Nerus
-- even if the outbox drops the later frame.
--
-- GC3: grant_xp
-- (742 has `reward_xp = 0` / `reward_naq = 0` like all 36 Harset
-- missions; the reward action lands here once GC3's formula is decided
-- -- README decision D-H10.)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6111, '742 - Nerus: take the Scarab Map and complete the mission', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6111, 'dialog_choice', '2640', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6111, 'step_status', 742, '2506', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6111, 'remove_item', 2864, NULL, '{"qty": 1}', 0, 0),
  (6111, 'remove_dialog_set', 3131, NULL, '{"slot": 53}', 0, 1),
  (6111, 'complete_mission', 742, NULL, '{}', 0, 2);

-- ============================================================
-- Chains 6112-6117: relog / world-entry restore for 742
-- ============================================================
--
-- Nothing this file paints survives a reconnect. Chain-set
-- `interaction_type` bits live on the in-memory `CellEntity` and dialog
-- bindings live in the per-player `available_interactions` map; neither
-- is persisted. `fire_player_loaded` has exactly one production call
-- site (base_messages/player_init/mod.rs:432) and it covers initial
-- login, gate travel AND cross-world teleport, so these chains double as
-- the repaint for a walk through the Command Center door -- not just for
-- a relog.
--
-- They are split by world because what they restore is world-bound: the
-- basket bits can only be set while the player is in world 57 (tag
-- lookup is space-scoped) and Anat and Nerus stand in world 68. The
-- `player_loaded` trigger carries the world in its `event_key` and the
-- `world` condition restates it; the pair is deliberate belt-and-braces
-- per the campaign rule, and the trigger key alone would already be
-- sufficient.
--
-- The three basket restores are separate chains rather than one, because
-- a player who planted one device and logged out must get back exactly
-- the two glows they had -- re-lighting a basket whose objective is
-- already complete would offer a fourth plant.
--
-- Step 2503 has no restore chain: between the disguise grant and its
-- use, 742 owns no bit and no binding. The disguise itself is a normal
-- inventory item and persists on its own.

-- Chain 6112: step 2502 -- Petbe still owes the player a disguise.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6112, '742 - Restore (Harset): re-bind Petbe''s disguise topic while step 2502 is active', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6112, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6112, 'world', 57, NULL, 'eq', NULL, 0),
  (6112, 'step_status', 742, '2502', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6112, 'add_dialog_set', 3129, NULL, '{"slot": 163, "mission_id": 742}', 0, 0);

-- Chains 6113-6115: step 2504 -- re-light only the baskets still owed.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6113, '742 - Restore (Harset): re-light FirstBug while objective 2913 is active', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6113, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6113, 'world', 57, NULL, 'eq', NULL, 0),
  (6113, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6113, 'objective_status', 742, '2913', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6113, 'set_interaction_type', NULL, 'FirstBug', '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6114, '742 - Restore (Harset): re-light SecondBug while objective 2914 is active', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6114, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6114, 'world', 57, NULL, 'eq', NULL, 0),
  (6114, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6114, 'objective_status', 742, '2914', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6114, 'set_interaction_type', NULL, 'SecondBug', '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6115, '742 - Restore (Harset): re-light ThirdBug while objective 2915 is active', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6115, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6115, 'world', 57, NULL, 'eq', NULL, 0),
  (6115, 'step_status', 742, '2504', 'eq', 'active', 1),
  (6115, 'objective_status', 742, '2915', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6115, 'set_interaction_type', NULL, 'ThirdBug', '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

-- Chain 6116: step 2505 -- Anat is waiting for the report.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6116, '742 - Restore (Command Center): re-bind Anat''s report topic while step 2505 is active', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6116, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6116, 'world', 68, NULL, 'eq', NULL, 0),
  (6116, 'step_status', 742, '2505', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6116, 'add_dialog_set', 3130, NULL, '{"slot": 43, "mission_id": 742}', 0, 0);

-- Chain 6117: step 2506 -- Nerus is waiting for the map.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6117, '742 - Restore (Command Center): re-bind Nerus''s map topic while step 2506 is active', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6117, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6117, 'world', 68, NULL, 'eq', NULL, 0),
  (6117, 'step_status', 742, '2506', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6117, 'add_dialog_set', 3131, NULL, '{"slot": 53, "mission_id": 742}', 0, 0);

-- ============================================================
-- Chains 6118-6119: the 742 offer (README decision D-H20)
-- ============================================================
--
-- These two chains are NOT in the `.script`. They restore, as chains,
-- the only two pieces of 2009 mission-offer data that survive anywhere
-- in the shipped database -- and that Cimmeria cannot read:
--
--   * `resources.entity_interactions` row 35 (template 43 Anat, dsm
--     3127, `missions_not_accepted = {742}`) -- the ONLY row in that
--     table in the whole game. `grep -rn "entity_interactions" crates/
--     --include=*.rs` returns ZERO hits: no Rust code loads the table.
--   * `resources.dialogs.accepts_mission_id = 742` on dialog 2636 -- the
--     ONLY non-NULL value of that column in the game. It likewise has no
--     Rust consumer.
--
-- So before these chains, nothing in the server could put mission 742
-- into the accepted state and every other chain in this section was
-- unreachable in play. The two dead rows tell us exactly what the 2009
-- offer looked like -- Anat offers it, via dsm 3127 / dialog 2636, only
-- to a player who has not accepted it -- and that is what is rebuilt
-- here. Evidence class: RECONSTRUCTION from shipped-but-unread data.
-- The dead table and dead column are handed to H99 for documentation in
-- content-engine.md.
--
-- THE `mission_status 1200 eq completed` GATE IS NOT DECORATION. Anat is
-- template 43 and `handle_interact` takes `available_interactions[43]
-- .first()` -- the first binding wins and the second is unreachable. A
-- Goa'uld who could hold the 742 offer (dsm 3127) and the 1200 step-3585
-- topic (dsm 4751) at the same time would silently lose one of the two
-- Anat beats. Requiring 1200 to be COMPLETE before the offer appears
-- makes the two states disjoint, which is also the narrative order: 1200
-- IS the mission in which the player is introduced to Anat. The
-- `archetype eq 6` gate follows from that -- 1200 is Goa'uld-only, so
-- only a Goa'uld can ever satisfy the 1200 gate anyway, but stating it
-- keeps the chain readable and fails closed if 1200's own gating is ever
-- widened.

-- Chain 6118: Anat offers 742 to a Goa'uld who has finished 1200.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6118, '742 - Offer: bind Anat''s "Giving the Walls Ears" topic for a Goa''uld who has completed 1200', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6118, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6118, 'world', 68, NULL, 'eq', NULL, 0),
  (6118, 'archetype', NULL, NULL, 'eq', '6', 1),
  (6118, 'mission_status', 742, NULL, 'eq', 'not_active', 2),
  (6118, 'mission_status', 1200, NULL, 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6118, 'add_dialog_set', 3127, NULL, '{"slot": 43}', 0, 0);

-- Chain 6119: the player accepts.
--
-- `mission_status 742 eq not_active` is the campaign-mandated gate on
-- every `accept_mission` chain. It is also the only gate this chain
-- needs: `dialog_choice` is server-gated on the dialog having actually
-- been displayed to this player (`CellEntity::open_dialog_id`), and
-- dialog 2636 can only be displayed through chain 6118's binding, which
-- already carries the archetype and 1200 gates.
--
-- The unbind is ordered AFTER the accept so that the `mission_accepted`
-- event chain 6101 keys on has already been queued; both are executed in
-- `sort_order` within one `execute_actions` pass and neither depends on
-- the other's effects.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6119, '742 - Offer accepted: start the mission and retire Anat''s offer topic', 'mission', 742, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6119, 'dialog_choice', '2636', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6119, 'mission_status', 742, NULL, 'eq', 'not_active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6119, 'accept_mission', 742, NULL, '{}', 0, 0),
  (6119, 'remove_dialog_set', 3127, NULL, '{"slot": 43}', 0, 1);

-- ============================================================
-- H40 -- MISSION 1200 "Meet Your Queen"  (chains 6121-6127)
-- ============================================================
--
-- NEW authoring, but every dialog and every icon mask below is
-- RECOVERED: dialog set 1268 ("Meet Your Queen") survives complete in
-- `dialog_set_maps` / `dialog_screens` / `dialog_screen_buttons` and
-- maps one-to-one onto the two steps. Nothing here invents text.
--
--   dsm 4750 -> dialog 4019  speaker 942 Ba'al, flags 134217728
--               (INT_NonAStoryMissionAvailable). Buttons 8 "Accept"
--               (type 2) and 9 "More Info" (type 1). "Get an audience
--               with Queen Anat."
--   dsm 4754 -> dialog 4074  flags 0. The "More Info" body -- Ba'al on
--               the state of the alliance, closing "Pay a visit to Queen
--               Anat - she is your mother afterall".
--   dsm 4817 -> dialog 4452  flags 268435456
--               (INT_NonAStoryMissionActive). "Identify yourself." ...
--               "You will have to much to convince me to let you pass."
--               Button 170 (type 4) "Convince Anat's Royal Jaffa to
--               grant you an audience." -- STEP 3584.
--   dsm 4751 -> dialog 5435  speaker 944 Anat, flags 268435456. "Do you
--               find me... Beautiful?... You hesitate..." Button 171
--               (type 4) "Flatter Anat." -- STEP 3585.
--   dsm 6360 -> dialog 5436  speaker 942 Ba'al, flags 268435456. "She is
--               like that... I will give you some suggestions to keep in
--               mind for the future..." -- the hidden optional objective
--               5399 "Ask Ba'al for help."
--
-- Steps: 3584 (convince the Royal Guard, objective 4139) -> 3585 (speak
-- to Anat, objective 4140 required + 5399 hidden and optional).
--
-- Cast: Royal Guard template 209 tag `CmdCenter_RoyalGuard`, Ba'al
-- template 42 tag `CmdCenter_Baal`, Anat template 43 spawn 222 -- all
-- world 68. The two tags are H12's spawn rows and are still pending M0;
-- the chains below bind by TEMPLATE, so they do not depend on the tags
-- and will start working the moment the spawns land.
--
-- SCOPE NOTE ON THE ACCEPT. Per the packet, 1200 is accepted on arrival
-- rather than from Ba'al's offer dialog. Dialog 4019's Accept / More
-- Info buttons and dsm 4750/4754 are therefore recovered evidence that
-- this authoring does NOT consume; they are recorded in
-- worknotes/H40.md as the alternative offer-driven shape, for the
-- coordinator to rule on if the arrival accept plays badly in UAT.

-- ------------------------------------------------------------
-- Chain 6121: a Goa'uld arriving in Harset is expected at court
-- ------------------------------------------------------------
--
-- Deliberately accept-only. The Royal Guard binding lives on chain 6125
-- alone -- initial paint and relog restore in one chain -- so that
-- `available_interactions[209]` can never accumulate a duplicate entry.
-- `add_dialog_set` pushes unconditionally with no dedup
-- (executor/dialog.rs:145-151), so binding here AND in the restore chain
-- would stack a second copy on every world entry. Harmless today
-- (`entries.first()` returns the same dialog and `remove_dialog_set`
-- retains-all), but it would become a real bug the moment a second dsm
-- is bound to template 209.
--
-- The arrival is world 57 and the Royal Guard is world 68, so there is
-- nothing to paint at this moment anyway.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6121, '1200 - Goa''uld arrival in Harset: accept "Meet Your Queen"', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6121, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6121, 'world', 57, NULL, 'eq', NULL, 0),
  (6121, 'archetype', NULL, NULL, 'eq', '6', 1),
  (6121, 'mission_status', 1200, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6121, 'accept_mission', 1200, NULL, '{}', 0, 0);

-- ------------------------------------------------------------
-- Chain 6122: the Royal Guard is convinced (step 3584 -> 3585)
-- Dialog 4452, button 170 "Convince Anat's Royal Jaffa to grant you an
-- audience."
-- ------------------------------------------------------------
--
-- Both of step 3585's topics are bound here, because the player is
-- standing in world 68 in front of the Royal Guard and Anat and Ba'al
-- are a few metres away -- the icons appear without waiting for a world
-- re-entry. Chains 6126/6127 re-apply them after a relog.
--
-- `advance_step` (not `complete_objective`) moves 3584 -> 3585: objective
-- 4139 is step 3584's only required objective, so completing it would
-- end the whole mission (progression.rs:176-183).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6122, '1200 - Royal Guard convinced: advance to step 3585 and open Anat and Ba''al', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6122, 'dialog_choice', '4452', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6122, 'step_status', 1200, '3584', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6122, 'remove_dialog_set', 4817, NULL, '{"slot": 209}', 0, 0),
  (6122, 'advance_step', 1200, '3585', '{}', 0, 1),
  (6122, 'add_dialog_set', 4751, NULL, '{"slot": 43, "mission_id": 1200}', 0, 2),
  (6122, 'add_dialog_set', 6360, NULL, '{"slot": 42, "mission_id": 1200}', 0, 3);

-- ------------------------------------------------------------
-- Chain 6123: Anat is flattered -- mission complete
-- Dialog 5435, button 171 "Flatter Anat."
-- ------------------------------------------------------------
--
-- Ba'al's optional topic is retired here too, whether or not the player
-- took it. `complete_mission` force-completes every remaining objective
-- of the current step including the hidden 5399
-- (progression.rs:252-260), so a player who skipped Ba'al is not left
-- with a dangling hidden objective.
--
-- GC3: grant_xp
-- (1200 has `reward_xp = 0` / `reward_naq = 0`; step 3584 is the only
-- Harset step in either of these two missions with `award_xp = t`, so
-- the GC3 formula will need to read the step rather than the mission.
-- Recorded in worknotes/H40.md.)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6123, '1200 - Anat flattered: complete "Meet Your Queen"', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6123, 'dialog_choice', '5435', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (6123, 'step_status', 1200, '3585', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6123, 'remove_dialog_set', 4751, NULL, '{"slot": 43}', 0, 0),
  (6123, 'remove_dialog_set', 6360, NULL, '{"slot": 42}', 0, 1),
  (6123, 'complete_mission', 1200, NULL, '{}', 0, 2);

-- ------------------------------------------------------------
-- Chain 6124: the hidden optional -- ask Ba'al how to handle his sister
-- Dialog 5436, objective 5399.
-- ------------------------------------------------------------
--
-- This can NEVER block completion and can never end the mission early.
-- Objective 5399 is `is_optional = t`, and the auto-complete check in
-- `complete_objective` filters optional objectives out
-- (progression.rs:176-180), so completing it while the required 4140 is
-- still active does nothing but tick the (hidden) objective.
--
-- It is only reachable while step 3585 is active, because objective 5399
-- belongs to step 3585 and `MissionInstance::complete_objective` returns
-- false for anything not in `active_objectives`. A player who talks to
-- Ba'al during step 3584 gets nothing -- which is correct: the advice is
-- about Anat, and the topic is not bound until 6122 runs.
--
-- `dialog_open`, NOT `dialog_choice`. Dialog 5436 has ZERO
-- `dialog_screen_buttons` rows, and unlike 742's button-less dialogs
-- there is no 2009 script subscribing to `dialog.choice::5436` to prove
-- the client emits a choice for it. The objective is "Ask Ba'al for
-- help", which is satisfied by the dialog being shown at all --
-- `fire_dialog_open` is dispatched from
-- cell_methods/player/interaction/interact.rs:210 whenever
-- `handle_interact` resolves a dialog from `available_interactions`,
-- which is exactly this path. Re-opening the dialog cannot double-fire:
-- the `objective_status` gate below closes after the first pass.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6124, '1200 - Ba''al''s advice (hidden optional): complete objective 5399', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6124, 'dialog_open', '5436', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6124, 'step_status', 1200, '3585', 'eq', 'active', 0),
  (6124, 'objective_status', 1200, '5399', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6124, 'complete_objective', 1200, '5399', '{}', 0, 0),
  (6124, 'remove_dialog_set', 6360, NULL, '{"slot": 42}', 0, 1);

-- ============================================================
-- Chains 6125-6127: world-entry paint and relog restore for 1200
-- ============================================================
--
-- All three fire on entry to world 68, which is where all three NPCs
-- stand. 6125 is both the FIRST paint of the Royal Guard topic (chain
-- 6121 deliberately does not bind) and its restore; 6126/6127 are pure
-- restores for topics 6122 already bound.

-- Chain 6125: step 3584 -- the Royal Guard is blocking the way.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6125, '1200 - Command Center entry: bind the Royal Guard''s topic while step 3584 is active', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6125, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6125, 'world', 68, NULL, 'eq', NULL, 0),
  (6125, 'step_status', 1200, '3584', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6125, 'add_dialog_set', 4817, NULL, '{"slot": 209, "mission_id": 1200}', 0, 0);

-- Chain 6126: step 3585 -- Anat will see the player now.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6126, '1200 - Restore (Command Center): re-bind Anat''s topic while step 3585 is active', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6126, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6126, 'world', 68, NULL, 'eq', NULL, 0),
  (6126, 'step_status', 1200, '3585', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6126, 'add_dialog_set', 4751, NULL, '{"slot": 43, "mission_id": 1200}', 0, 0);

-- Chain 6127: step 3585 -- Ba'al's advice, only while still unasked.
--
-- The extra `objective_status` gate is what stops a relog from
-- re-offering advice the player already took: 6126 has no such gate
-- because Anat's topic must come back until the mission is finished,
-- but 5399 completes independently.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6127, '1200 - Restore (Command Center): re-bind Ba''al''s advice while objective 5399 is still active', 'mission', 1200, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6127, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6127, 'world', 68, NULL, 'eq', NULL, 0),
  (6127, 'step_status', 1200, '3585', 'eq', 'active', 1),
  (6127, 'objective_status', 1200, '5399', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6127, 'add_dialog_set', 6360, NULL, '{"slot": 42, "mission_id": 1200}', 0, 0);
