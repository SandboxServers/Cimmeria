-- Castle (World 8) content chains — missions 706 and 708.
--
-- Packets CA08 ("Power Behind the Throne") and CA09 ("Secure the
-- Stargate") of the Castle rebuild ledger,
-- docs/analysis/castle-rebuild/work-packets.md. Companion worknote:
-- docs/analysis/castle-rebuild/worknotes/m706-708.md.
--
-- EVIDENCE CLASS: every chain in this file is RECONSTRUCTION. There is
-- no recovered server script for 706 or 708 anywhere in the repository
-- — deprecated/python/cell/spaces/Castle.py stops at mission 701 and
-- never references 702, 703, 704, 706 or 708 (audit.md, "Where The Spec
-- Is Wrong Or Superseded", mission-703 row). What IS original data, and
-- is reproduced here verbatim rather than invented: the mission/step/
-- objective/task rows, the dialog ids and their screens, the dialog-set
-- map rows, the region and spawn rows, and the two item ids. The chain
-- SHAPE — which trigger drives which step, in what order — is the
-- reconstruction. Individual rows carry ORIGINAL_DATA / RECONSTRUCTION
-- markers where the distinction is not obvious from context.
--
-- Chain ID ranges (allocated by the coordinator, work-packets.md
-- "Worker Input And Ownership"):
--   Mission 706 (CA08): 1321-1340 — used: 1321-1323
--   Mission 708 (CA09): 1341-1380 — used: 1341-1365
--
-- ============================================================
-- ENGINE FACTS THIS FILE DEPENDS ON (verified in this worktree)
-- ============================================================
--
-- 1. `advance_step` is unconditional and force-completes every
--    still-active objective of the step being left, by calling
--    `MissionInstance::complete_objective` DIRECTLY
--    (missions/progression.rs:57-66). It deliberately bypasses the
--    module-level `complete_objective`, so it can never trip the
--    `all_required_complete` auto-complete at progression.rs:176-183.
--    That is why "complete the one objective we want ticked, then
--    advance" is safe on a multi-objective step.
--
-- 2. `complete_objective` DOES auto-complete the mission when every
--    non-optional objective of the current step is complete
--    (progression.rs:176-183). Step 2417 carries THREE non-optional
--    objectives (5184 blank, 5185 Marsh, 5186 Moh'katan —
--    mission_objectives.sql:6631/6633/6635), so completing one report
--    objective leaves two active and cannot end 708 early. Step 2415's
--    2794/2795 and step 2416's 2798/2799 are `is_optional = true`
--    (6619/6621/6625/6629) with a required objective still active
--    (2796 hidden, 2797 blank), so the same holds there.
--    CAUTION for future edits: if any of those required objectives is
--    ever flipped to optional, the matching chain below completes the
--    whole mission several steps early. Every route that completes an
--    objective has an EXECUTED guard — it stages the step with the real
--    `is_optional` flags, pushes the chain through `execute_actions`,
--    and asserts the mission is still `MISSION_ACTIVE` afterwards:
--      chain 1343 → diagnosis.rs::guard_route_advances_without_completing_the_mission
--      chain 1345 → diagnosis.rs::panel_route_advances_without_completing_the_mission
--      chain 1346 → crystal.rs::crystal_is_granted_only_once_across_two_officer_deaths
--      chain 1353 → report.rs::reporting_in_advances_the_step_without_completing_the_mission
--      chain 1355 → report.rs::the_jaffa_report_advances_without_completing_the_mission
--    A resolve-only test cannot see any of this: `complete_objective`'s
--    auto-complete branch runs inside the executor, not the resolver.
--
--    BE PRECISE ABOUT WHAT THOSE GUARDS CATCH. They read the objective
--    flags from a hard-coded fixture, NOT from `mission_objectives.sql`.
--    So they catch a chain here growing an extra `complete_objective`
--    row, and they catch a regression in the executor's all-required
--    check — but an `is_optional` flip in `mission_objectives.sql`
--    itself would NOT fail them, because the fixture would keep
--    asserting the old flags. Mirroring a flip into the fixtures is a
--    manual step. See the worknote's Known gaps.
--
-- 3. Conditions for EVERY matching chain are evaluated against one
--    pre-action `ExecutionContext` snapshot, then all matched action
--    lists are concatenated and run in order (chain/mod.rs:288-325).
--    So chain 1350's `step_status 2416 eq active` still passes even
--    though chain 1346 advances to 2417 earlier in the same batch.
--    The corollary is the single-grant guard in (4).
--
-- 4. SINGLE-GRANT GUARD for the Control Crystal (item 2790). The step
--    gate IS the guard: chains 1346-1349 advance out of 2416 in the
--    same action list that grants, and `advance_step` mutates
--    `current_step_id` synchronously in memory (progression.rs:76)
--    before returning. A second kill — including re-killing a
--    respawned officer, which reuses the SAME entity and tag
--    (ticks/npc_respawn/mod.rs:116-165) — is a separate
--    `fire_entity_death` with a fresh context that sees 2417 and
--    resolves nothing. `fire_entity_death` also cannot re-fire for one
--    corpse: kill_credit.rs:56-69 short-circuits on the
--    `was_alive_before` snapshot.
--    THIS GUARD REQUIRES `delay_ms = 0` ON THE `advance_step` ROWS.
--    executor/mod.rs:96-120 QUEUES rather than runs any action with
--    `delay_ms > 0`; a deferred advance would leave 2416 active across
--    the delay window and turn four officers plus a warden plus
--    respawns into an unbounded crystal faucet. Every action row in
--    this file is `delay_ms = 0` on purpose.
--
-- 5. ARCHETYPE GATING MUST LIVE ON THE INTERACT CHAIN, NEVER ON THE
--    `dialog_choice` CHAIN. `fire_dialog_choice`
--    (event_dispatch/dialog.rs:87-93) populates `dialog_id`,
--    `button_id` and mission context — but NOT `archetype`.
--    `Condition::Archetype` reads the missing key as `-1`
--    (conditions.rs:225-230), so on a dialog chain `archetype eq 8` is
--    permanently FALSE (chain dead) and `archetype neq 8` is
--    permanently TRUE — a guard that silently fails open. The
--    Human/Jaffa report branches at step 2417 are therefore
--    discriminated by DIALOG ID (5008 vs 5009) alone, which is
--    server-authoritative for two stacked reasons:
--      (a) #479: `dialogButtonChoice` is rejected unless the server
--          itself displayed that dialog to that player
--          (cell_methods/player/interaction/dialog.rs:36-49).
--      (b) the only way to reach `display_dialog 5008` is chain 1352,
--          which IS archetype-gated (an `interact_tag` chain, where
--          `archetype` is populated — interaction.rs:42-44). Dialog
--          sets 5850/5851 are never `add_dialog_set`-bound by this
--          file, so 5008/5009 never enter a player's
--          `available_interactions` and the client cannot reach them
--          through the unscoped `initialResponse` lookup
--          (interactions/dispatch/initial_response.rs:33-43), which is
--          the one other path that can display an arbitrary dialog.
--
-- 6. ZERO-BUTTON DIALOGS DO FIRE `dialog_choice`. None of 2584, 2586,
--    5003, 5004, 5008, 5009, 5010 or 5011 has a single row in
--    dialog_screen_buttons.sql. The client nevertheless emits
--    `Event_NetOut_DialogButtonChoice` with `ButtonId = -1` from the
--    Done / X close path when a dialog's total button count is zero
--    (client Lua `UI/Core/Dialog/Dialog.lua:64-67,83-85` →
--    `discardAvailableDialog` → native FUN_00d249c0); a dialog that
--    HAS buttons sends nothing on close and only fires on an
--    Accept/Generic click. This is why `Castle.py` could key
--    `dialog.choice::2574` and `::2575` — both also button-less — and
--    why shipped chains 1020/1021 key on 2300/5020. Cimmeria reads the
--    id as a plain i32 and the #479 gate keys only on the open dialog,
--    so `-1` is handled (interaction/dialog.rs:21-22, 36-49).
--    *** DO NOT ADD BUTTONS TO 5003/5004/5008/5009. *** Doing so stops
--    the close path emitting and silently soft-locks steps 2415/2417.
--
-- 7. SHARED-WORLD INTERACTION FLAGS (D-CA15), and the ZERO-BASELINE
--    RULE this file follows. `set_interaction_type` mutates the SHARED
--    `CellEntity.interaction_type_flags` and broadcasts to every
--    witness (executor/world/mod.rs:19-64); there is no per-player
--    interaction state short of a dialog-set bind (design gate GCA1).
--    Setting a bit is harmless to bystanders (an extra cue). CLEARING
--    one is not: `EInteractionNotificationType` is the bitfield that
--    drives the client's right-click cursor and context menu
--    (entity/src/interaction_flags.rs:1-9), so clearing an entity's
--    LAST bit can strip its only affordance out from under another
--    player mid-step. Nothing server-side re-gates it —
--    `handle_interact` reads the template's `NpcInteractionType` and
--    `available_interactions`, never `interaction_type_flags`
--    (interactions/dispatch/interact.rs:93-131), and
--    `fire_interact_tag` runs on ANY interact with a tagged entity
--    (cell_methods/player/interaction/interact.rs:218-238). So the
--    breakage would be client-side and invisible to these tests.
--
--    THE RULE: a mission cue bit may be cleared only if the target's
--    template baseline is non-zero. `entity_templates.interaction_type`
--    IS the spawn-time value of `interaction_type_flags`
--    (space_manager/spawn.rs:127), so the baseline is a one-column
--    lookup, not a judgement call:
--      - `Castle_DHD`      template 162 → 16 (`INT_Dhd`). NON-ZERO.
--        Chain 1357 MAY clear `INT_MinigameLivewire`; the DHD keeps
--        `INT_Dhd` and stays clickable for everyone.
--      - `Castle_AccessPanel` template 147 → 0. Chain 1322 does NOT
--        clear the glow. Two missions use the panel (706 step 2412,
--        708 step 2415) and `accept_mission` can be refused by the
--        offer guard (executor/mission.rs:64-72), so a
--        clear-then-reaccept ordering would leave it permanently dark
--        for the whole zone.
--      - `Castle_ColMarsh` template 10 → 0, `Castle_Mohkatan`
--        template 54 → 0 (entity_templates.sql:25/63). Chains
--        1353/1355 do NOT clear the `!`. Two players can sit on step
--        2417 at once and the first to report in would otherwise make
--        the report NPC unclickable for the second, with no recovery
--        short of a relog.
--      - `Castle_SurrenderGuard`: chains 1343/1345 do NOT clear the
--        `!` either. Its template is seeded by packet CA05 (PR #667)
--        and is not in this DB yet; when it lands, apply THIS rule to
--        it rather than re-deciding. If it ships non-zero the clear
--        may be restored.
--    The cost of the rule is a stale `!` over an NPC the player has
--    already dealt with — the same cosmetic cost this file already
--    accepts for the panel, and strictly preferable to a hard stall.
--    It deviates from packets CA08/CA09's literal wording ("clear the
--    glow" / "clear it on advance"); see the worknote.
--
--    Cellblock precedent 1053/1054 DOES clear `!`-class bit 8388608
--    off `Preparation_ColMarsh` — which is template 10, the very same
--    template as `Castle_ColMarsh` (spawnlist.sql:7 vs :181). That is
--    the same zero-baseline hazard, unexamined there; it is not
--    evidence that the clear is safe. Filed as an out-of-scope finding
--    rather than fixed here.
--
--    The `player_loaded` restore chains 1361-1365 are still required:
--    `interaction_type_flags` is in-memory only, so a server restart
--    drops every bit painted above regardless of whether any chain
--    clears it.
--
-- 8. `content_triggers.scope` / `content_chains.scope_type` /
--    `scope_id` / `once` are read into the loader row structs but
--    never consulted by `build_chains_from_rows`
--    (content-engine/src/loader/mod.rs:45-59, 88-215). They are
--    documentation. Only conditions gate a chain. Values below follow
--    the castle_cellblock_chains.sql convention.
--
-- 9. Multiple `content_triggers` rows on one chain OR together: the
--    loader materializes one in-memory `Chain` per trigger row sharing
--    the id, conditions and actions (loader/mod.rs:170-205, precedent
--    chain 1087). Chains 1350/1351 use this. No two of their trigger
--    rows can match the same event, so no action is ever duplicated.
--
-- ============================================================
-- KNOWN GAPS (do not "fix" without reading the worknote)
-- ============================================================
--
-- - Tags `Castle_SurrenderGuard`, `Castle_BravoOfficer1..3` and
--   `Castle_Muelbach` are CONTRACTS owed by packet CA05 and are not in
--   spawnlist.sql yet. Until they land, `find_entity_by_tag` returns
--   None, `set_interaction_type` debug-logs and no-ops
--   (space_manager/queries.rs:325-332), and the death triggers never
--   fire. The chain-replay guards exercise the chains against a
--   hand-built fixture and pass regardless; the ZONE does nothing
--   until CA05 seeds the rows.
-- - Chain 1357's `add_dialog_set 3073` and chain 1365 are INERT until
--   packet CA02 widens `DialogSetMapEntry.dialog_id` to `Option<i32>`:
--   `spawner/dialogs.rs:45` still drops every `dialog_id IS NULL` row
--   at load, so binding row 3073 is a warn + no-op
--   (executor/dialog.rs:172-177). They are seeded now because the
--   packet asks for them and because they cost nothing when inert.
--   Note that the bind is a TOPIC ("Dial Harset"), not the dial
--   capability: template 162 already ships `interaction_type = 16`
--   (`INT_DHD`, entity_templates.sql:131), so the DHD is dialable from
--   spawn for everyone and the dial itself is NOT gated on step 4462.
--   Chain 1358/1359's `step_status 4462 eq active` gates the mission
--   ADVANCE, which is all mission 708 needs.
-- - `Castle_AccessPanel` at x = 330.49 (spawnlist.sql:133) sits just
--   OUTSIDE point set 2049's bounding box (min x ≈ 331.35,
--   point_set_points.sql:121-127). Harmless today — chain 1321 fires
--   on region entry, not on proximity to the panel — but a future
--   tightening of the region test would bite.

SET search_path = resources, pg_catalog;

-- ============================================================
-- MISSION 706 — Power Behind the Throne (CA08)
-- ============================================================
--
-- Steps (mission_steps.sql:5985/5987, ORIGINAL_DATA):
--   2411 "Get to the Throne Room."            objective 2790 (required)
--   2412 "Hack into the Goa'uld communications grid."
--                        objectives 2791 (use panel) + 2792 (locate panel),
--                        BOTH required
--
-- Accepted by mission 704's delivery chain (packet CA07, chain 1291-1320
-- range — another worker). This file starts at "706 step 2411 active".

-- Chain 1321: enter the Throne Room while step 2411 is active → advance
-- to 2412 and light the Access Panel.
--
-- RECONSTRUCTION. Region 2049 `Castle.ThroneRoom` and the step text
-- ("Get to the Throne Room.") are ORIGINAL_DATA; the enter_region →
-- advance_step wiring is the reconstruction, modelled on chain 1031
-- (639 - Enter Region11 → advance + light the vial), which is the
-- closest recovered-script-backed precedent in the zone family.
--
-- `Castle.ThroneRoom` must stay byte-identical to `point_sets.name`
-- (point_sets.sql:43) — `fire_enter_region` does a literal,
-- case-sensitive match and a typo silently never fires.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1321, '706 - Enter Throne Room: advance to step 2412, light Access Panel', 'mission', 706, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1321, 'enter_region', 'Castle.ThroneRoom', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1321, 'step_status', 706, '2411', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1321, 'advance_step', 706, '2412', '{}', 0, 0),
  -- INT_MissionWorldObject: the quest-object glow. Objective 2792 is
  -- "Locate the access panel behind the Throne", so the glow IS the
  -- objective's affordance. Never cleared — see engine fact (7).
  (1321, 'set_interaction_type', NULL, 'Castle_AccessPanel', '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 1);

-- Chain 1322: use the Access Panel while step 2412 is active → play the
-- Copplemann/Marsh radio scene, complete 706, accept 708.
--
-- RECONSTRUCTION. Dialog 2584 and dialog_set_map row 5711 (set 654,
-- topic "Behind the Throne") are ORIGINAL_DATA, as is the panel spawn
-- (spawnlist.sql:133, template 147).
--
-- `complete_mission` rather than completing 2791 + 2792 separately, per
-- packet CA08 and engine fact (2): `complete_objective` on the LAST
-- non-optional objective of a step routes through the auto-complete
-- branch at progression.rs:183-203, which sends `onMissionUpdate` with
-- the status byte `MISSION_ACTIVE` (see the "// Status sent as
-- 'completed' removal" comment at :200) rather than STATUS_COMPLETED.
-- `complete_mission_direct` (progression.rs:226-309) emits the correct
-- sequence: onObjectiveUpdate(COMPLETED) per objective, then
-- onStepUpdate(COMPLETED), then onMissionUpdate(STATUS_COMPLETED).
--
-- `mission_status 708 eq not_active` does double duty: it is the
-- mandatory accept gate (content-chains.instructions.md, "Mission
-- grants must gate on not_active") AND the discriminator against chain
-- 1344, which triggers on the SAME tag for 708's step-2415 panel
-- diagnosis route.
--
-- Dialog 2584 has eleven screens and ZERO buttons — it fires its
-- `dialog_choice` on close with button_id = -1 (engine fact 6). Nothing
-- in this file keys on `dialog_choice 2584`; the completion is done
-- here, on the interact, so the mission does not depend on the close
-- event. The wire EntityId of `onDialogDisplay` binds to the panel
-- (chain params carry `target_entity_id` from `fire_interact_tag`),
-- so the portrait actor is the panel while the screens' own
-- `speaker_id`s (1110 Copplemann, 261 Marsh) name the voices.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1322, '706 - Use Access Panel: play 2584, complete 706, accept 708', 'mission', 706, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1322, 'interact_tag', 'Castle_AccessPanel', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1322, 'step_status', 706, '2412', 'eq', 'active', 0),
  (1322, 'mission_status', 708, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1322, 'display_dialog',   2584, NULL, '{}', 0, 0),
  (1322, 'complete_mission',  706, NULL, '{}', 0, 1),
  -- `accept_mission` synchronously fires `fire_mission_accepted`
  -- (executor/mission.rs:104-107), which runs chain 1341 before this
  -- list continues. Nothing here depends on that ordering — the panel
  -- glow is never cleared, so 1341's repaint is a defensive idempotent
  -- `|`, not a repair.
  (1322, 'accept_mission',    708, NULL, '{}', 0, 2);

-- Chain 1323: relog restore for step 2412. `interaction_type_flags`
-- live on the in-memory `CellEntity` and do not survive a server
-- restart, so the glow set by chain 1321 has to be repainted on load
-- (content-chains.instructions.md, "Set/clear pairing"; precedent
-- chains 1110/1111). Idempotent: `|` on an already-set bit is a no-op.
--
-- No restore is needed for step 2411 — nothing is painted until the
-- player enters the region.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1323, '706 - Relog at step 2412: restore Access Panel glow', 'mission', 706, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1323, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1323, 'step_status', 706, '2412', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1323, 'set_interaction_type', NULL, 'Castle_AccessPanel', '{"op": "|", "mask": "INT_MissionWorldObject"}', 0, 0);

-- ============================================================
-- MISSION 708 — Secure the Stargate (CA09)
-- ============================================================
--
-- Steps and objectives (mission_steps.sql:5989-5999,
-- mission_objectives.sql:6617-6641 — all ORIGINAL_DATA):
--   2415 "Discover why the Stargate will not dial out."
--          2796 required hidden | 2794 opt (surrender) | 2795 opt (panel)
--   2416 "Retrieve the DHD Control Crystal."
--          2797 required | 2798 opt (Bravo officers) | 2799 opt (Muelbach)
--   2417 "Take the Control Crystal to Checkpoint Alpha."
--          5184 required | 5185 required (Marsh) | 5186 required (Moh'katan)
--   2418 "Repair the DHD."                       5197 required
--   4462 "Use the DHD to dial the Stargate to Harset."  5198 (task 6403)
--   4469 "Enter the active Stargate to leave the Castle."  5200 (task 6404)
--
-- Accepted by chain 1322 above.

-- Chain 1341: 708 accepted → paint the two step-2415 affordances.
--
-- RECONSTRUCTION, precedent chains 1097 and 1106 (mission accept →
-- highlight the next interactable). `mission_accepted` fires from
-- inside the `accept_mission` executor arm, so this runs during chain
-- 1322's action list.
--
-- The surrender guard gets INT_AStoryMissionActive (the "!" cue) rather
-- than the world-object glow because it is an NPC conversation, not a
-- prop; the panel gets the glow it already carries from 706. Both
-- objectives 2794 and 2795 are optional and either satisfies the step,
-- so both affordances are lit at once.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1341, '708 - Accepted: mark surrender guard and Access Panel', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1341, 'mission_accepted', '708', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1341, 'set_interaction_type', NULL, 'Castle_SurrenderGuard', '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0),
  (1341, 'set_interaction_type', NULL, 'Castle_AccessPanel',    '{"op": "|", "mask": "INT_MissionWorldObject"}',  0, 1);

-- ── Step 2415 — two diagnosis routes, either one advances ──
--
-- The step's own text and both optional objectives are ORIGINAL_DATA
-- and spell the two routes out: "(Option #1) Force a guard to surrender
-- and reveal what he knows" (2794) and "(Option #2) Use the Access
-- Panel on the Goa'uld Throne to discover why the gate is
-- malfunctioning" (2795). Dialog 5003 is the guard's interrogation
-- (speaker 1093) and 5004 the panel's diagnostic readout. The
-- interact → display → choice → advance shape is the reconstruction.

-- Chain 1342: talk to the surrendered guard at 2415 → play 5003.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1342, '708 - Interrogate surrender guard: display 5003', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1342, 'interact_tag', 'Castle_SurrenderGuard', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1342, 'step_status', 708, '2415', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1342, 'display_dialog', 5003, NULL, '{}', 0, 0);

-- Chain 1343: close 5003 → tick the surrender objective and advance.
--
-- `complete_objective` BEFORE `advance_step`, and both in the same
-- list: reversed, `mission.complete_objective(2794)` would run against
-- step 2416's objective list, return false, and early-return at
-- progression.rs:155-157 — the client would never see 2794 tick.
--
-- 2794 is optional and 2796 (required, hidden) stays active, so this
-- cannot auto-complete 708 (engine fact 2). `advance_step` then
-- force-completes 2796 — and, unavoidably, the untaken 2795 as well,
-- so a player who interrogated the guard also sees "(Option #2)" tick.
-- That is `advance_step`'s behaviour (progression.rs:57-66), not
-- something this seed can suppress.
--
-- The guard's "!" is cleared here. The panel's glow is NOT — engine
-- fact (7).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1343, '708 - Guard talked (5003): complete 2794, advance to 2416', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1343, 'dialog_choice', '5003', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1343, 'step_status', 708, '2415', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1343, 'complete_objective',  708, '2794', '{}', 0, 0),
  (1343, 'advance_step',        708, '2416', '{}', 0, 1);
  -- No `set_interaction_type ~ INT_AStoryMissionActive` on
  -- `Castle_SurrenderGuard`: zero-baseline rule, engine fact (7).
  -- Packet CA09 asks for the clear; CA05 has not seeded the guard's
  -- template yet, so its baseline is unknown and a clear could strip
  -- the guard's only affordance for a second player still on 2415.

-- Chain 1344: run the panel diagnostic at 2415 → play 5004.
--
-- Same tag as chain 1322 (706's completion). The two are mutually
-- exclusive by step gate: 1322 needs `706 step 2412 active` AND `708
-- not_active`; this one needs `708 step 2415 active`, which can only be
-- true after 706 completed.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1344, '708 - Access Panel diagnostic: display 5004', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1344, 'interact_tag', 'Castle_AccessPanel', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1344, 'step_status', 708, '2415', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1344, 'display_dialog', 5004, NULL, '{}', 0, 0);

-- Chain 1345: close 5004 → tick the panel objective and advance.
-- Mirror of 1343, including its zero-baseline abstention: neither
-- route clears the guard's "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1345, '708 - Panel diagnosed (5004): complete 2795, advance to 2416', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1345, 'dialog_choice', '5004', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1345, 'step_status', 708, '2415', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1345, 'complete_objective',  708, '2795', '{}', 0, 0),
  (1345, 'advance_step',        708, '2416', '{}', 0, 1);
  -- See chain 1343: no guard `!` clear, zero-baseline rule.

-- ── Step 2416 — the Control Crystal, from any of five corpses ──
--
-- Objectives 2798 and 2799 are ORIGINAL_DATA and name the sources:
-- "(Option #1) NID Officers at Checkpoint Bravo may have a Control
-- Crystal" and "(Option #2) Warden Muelbach will certainly have one.
-- She is holed up in the bunker above Checkpoint Bravo." Dialog 5003
-- says the same in prose. Three Bravo officers is packet CA05's default
-- N; if CA05 seeds a different count, add or drop chains in the
-- 1346-1348 block and the matching trigger rows on 1350/1351.
--
-- The crystal is an EXPLICIT grant, not loot: item 2790 has
-- `container_sets {2}` (the mission container) and there is no
-- `mission_reward_groups` row for 708 (audit.md, "Mission items are
-- explicit grants, no loot tables"). Precedent: chain 1032's
-- `add_item 19`. `{"qty": 1}` with no `container` lets
-- `executor/inventory.rs:59-61` resolve the container from
-- `items.container_sets`, which is what we want for a mission item.
--
-- D-CA08: step state is the possession proof, not a `HasItem` check —
-- the cell has no inventory view at all (audit.md, `HasItem` row), and
-- the granting chain advances the step in the same action list.
-- Divergence recorded there: dropping or selling the crystal does not
-- regress the step.
--
-- Single-grant guard: engine fact (4). `mission_status 708 eq active`
-- is redundant with the step gate but kept for parity with chains
-- 1100/1087 and as a cheap second line.

-- Chain 1346: NID officer 1 dies at 2416 → crystal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1346, '708 - Kill Bravo officer 1: grant Control Crystal, advance to 2417', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1346, 'entity_dead_tag', 'Castle_BravoOfficer1', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1346, 'mission_status', 708, NULL,   'eq', 'active', 0),
  (1346, 'step_status',    708, '2416', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1346, 'add_item',           2790, NULL,   '{"qty": 1}', 0, 0),
  (1346, 'complete_objective',  708, '2798', '{}',         0, 1),
  (1346, 'advance_step',        708, '2417', '{}',         0, 2);

-- Chain 1347: NID officer 2 dies at 2416 → crystal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1347, '708 - Kill Bravo officer 2: grant Control Crystal, advance to 2417', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1347, 'entity_dead_tag', 'Castle_BravoOfficer2', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1347, 'mission_status', 708, NULL,   'eq', 'active', 0),
  (1347, 'step_status',    708, '2416', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1347, 'add_item',           2790, NULL,   '{"qty": 1}', 0, 0),
  (1347, 'complete_objective',  708, '2798', '{}',         0, 1),
  (1347, 'advance_step',        708, '2417', '{}',         0, 2);

-- Chain 1348: NID officer 3 dies at 2416 → crystal.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1348, '708 - Kill Bravo officer 3: grant Control Crystal, advance to 2417', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1348, 'entity_dead_tag', 'Castle_BravoOfficer3', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1348, 'mission_status', 708, NULL,   'eq', 'active', 0),
  (1348, 'step_status',    708, '2416', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1348, 'add_item',           2790, NULL,   '{"qty": 1}', 0, 0),
  (1348, 'complete_objective',  708, '2798', '{}',         0, 1),
  (1348, 'advance_step',        708, '2417', '{}',         0, 2);

-- Chain 1349: Warden Muelbach dies at 2416 → crystal + her effects.
--
-- Item 2136 is named "Muelbach..." in items.sql:11846 and carries a
-- copy-pasted Ambernol description (audit.md, "Mission items" row), so
-- its ROLE is a reconstruction: it is granted here as the
-- identifying/keepsake drop that distinguishes the Muelbach route from
-- the officer route, mirroring how item 2135 identifies Romney on
-- mission 703. Nothing consumes it. If a client UAT shows it is
-- meaningless, drop this one action row; the rest of the chain stands.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1349, '708 - Kill Warden Muelbach: grant Control Crystal + 2136, advance to 2417', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1349, 'entity_dead_tag', 'Castle_Muelbach', 'space', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1349, 'mission_status', 708, NULL,   'eq', 'active', 0),
  (1349, 'step_status',    708, '2416', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1349, 'add_item',           2790, NULL,   '{"qty": 1}', 0, 0),
  (1349, 'add_item',           2136, NULL,   '{"qty": 1}', 0, 1),  -- RECONSTRUCTION: identifying drop
  (1349, 'complete_objective',  708, '2799', '{}',         0, 2),
  (1349, 'advance_step',        708, '2417', '{}',         0, 3);

-- Chains 1350/1351: whichever corpse yielded the crystal, light the
-- step-2417 report NPC for the killer's faction.
--
-- Split into their own chains rather than folded into 1346-1349 so the
-- archetype gate is expressed once per faction instead of once per
-- corpse. `entity_dead_tag` populates `archetype` from the KILLER
-- (lifecycle.rs:78-83), so the gate is real here — unlike on a
-- `dialog_choice` chain (engine fact 5). The four trigger rows OR
-- together (engine fact 9) and the step gate still reads `2416 active`
-- even though 1346-1349 advance in the same batch (engine fact 3).
--
-- Only ONE of the two RESOLVES for a given death — the killer's own
-- archetype decides which. But the bit it then sets is zone-wide
-- (engine fact 7), so a Jaffa's kill lights Moh'katan for every Tau'ri
-- in AoI as well, and vice versa. Packet CA09's "never expose the other
-- faction's" is therefore enforced on the DIALOG, not on the cue:
-- chains 1352/1354 carry the archetype gate, so a Tau'ri who walks up
-- to a lit Moh'katan gets nothing. The cue is advisory; the dialog is
-- authoritative. Per-player cues need design gate GCA1.

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1350, '708 - Crystal obtained (Tau''ri): mark Col. Marsh', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (1350, 'entity_dead_tag', 'Castle_BravoOfficer1', 'space', false, 0),
  (1350, 'entity_dead_tag', 'Castle_BravoOfficer2', 'space', false, 1),
  (1350, 'entity_dead_tag', 'Castle_BravoOfficer3', 'space', false, 2),
  (1350, 'entity_dead_tag', 'Castle_Muelbach',      'space', false, 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1350, 'step_status', 708,  '2416', 'eq',  'active', 0),
  (1350, 'archetype',   NULL, NULL,   'neq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1350, 'set_interaction_type', NULL, 'Castle_ColMarsh', '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1351, '708 - Crystal obtained (Jaffa): mark Moh''katan', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (1351, 'entity_dead_tag', 'Castle_BravoOfficer1', 'space', false, 0),
  (1351, 'entity_dead_tag', 'Castle_BravoOfficer2', 'space', false, 1),
  (1351, 'entity_dead_tag', 'Castle_BravoOfficer3', 'space', false, 2),
  (1351, 'entity_dead_tag', 'Castle_Muelbach',      'space', false, 3);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1351, 'step_status', 708,  '2416', 'eq', 'active', 0),
  (1351, 'archetype',   NULL, NULL,   'eq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1351, 'set_interaction_type', NULL, 'Castle_Mohkatan', '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

-- ── Step 2417 — report to Checkpoint Alpha, archetype-split ──
--
-- Objectives 5185 "Report to Col. Marsh at Checkpoint Alpha" and 5186
-- "Report to Moh'katan at Checkpoint Alpha" are ORIGINAL_DATA, as are
-- dialogs 5008 (Marsh: "You saw my future self die", the canonical
-- bridge to the Cellblock's dead Marsh) and 5009 (Moh'katan, speaker
-- 2499). Both objectives are REQUIRED, so the untaken branch can only
-- be closed by `advance_step`'s force-completion — never complete both
-- by hand and never try to finish 2417 with `complete_objective`.
--
-- Archetype 8 is Jaffa; the split follows precedent 1098/1099.
-- The `dialog_choice` halves carry NO archetype condition, by design —
-- engine fact (5) explains why one would be worse than useless, and
-- why dialog-id reachability is the real gate.
--
-- Precedent 1107 is the shape being copied: interact → advance +
-- swap highlights, with the "don't complete the last required
-- objective" caveat called out in its own seed comment.

-- Chain 1352: Tau'ri reports to Marsh.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1352, '708 - Report to Col. Marsh (Tau''ri): display 5008', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1352, 'interact_tag', 'Castle_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1352, 'step_status', 708,  '2417', 'eq',  'active', 0),
  (1352, 'archetype',   NULL, NULL,   'neq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1352, 'display_dialog', 5008, NULL, '{}', 0, 0);

-- Chain 1353: Marsh briefing closed → tick 5185, advance to 2418, hand
-- the DHD its Livewire affordance.
--
-- 5008's last screen is Marsh ordering "get the DHD working", so
-- lighting INT_MinigameLivewire on the DHD here is the direct
-- consequence of the dialog. The DHD keeps template 162's INT_DHD bit
-- (16) throughout, so its flags become 272 for the duration of step
-- 2418 and the client offers both affordances; chain 1356 wins the
-- interact while 2418 is active because `fire_interact_tag`
-- short-circuits `handle_interact`
-- (cell_methods/player/interaction/interact.rs:160-200). Flagged for
-- UAT — D-CA09 already marks the DHD Livewire provisional.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1353, '708 - Marsh briefed (5008): complete 5185, advance to 2418', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1353, 'dialog_choice', '5008', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1353, 'step_status', 708, '2417', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1353, 'complete_objective',  708, '5185', '{}', 0, 0),
  (1353, 'advance_step',        708, '2418', '{}', 0, 1),
  -- NO `~ INT_AStoryMissionActive` on `Castle_ColMarsh`: template 10's
  -- baseline is 0 (entity_templates.sql:25), so the clear packet CA09
  -- asks for would drop Marsh to flags 0 and make him unclickable for
  -- every other player still on step 2417. Zero-baseline rule, engine
  -- fact (7). The DHD clear in chain 1357 is the permitted case —
  -- template 162 keeps `INT_Dhd`.
  (1353, 'set_interaction_type', NULL, 'Castle_DHD',      '{"op": "|", "mask": "INT_MinigameLivewire"}',    0, 2);

-- Chain 1354: Jaffa reports to Moh'katan.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1354, '708 - Report to Moh''katan (Jaffa): display 5009', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1354, 'interact_tag', 'Castle_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1354, 'step_status', 708,  '2417', 'eq', 'active', 0),
  (1354, 'archetype',   NULL, NULL,   'eq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1354, 'display_dialog', 5009, NULL, '{}', 0, 0);

-- Chain 1355: Moh'katan briefing closed → tick 5186, advance to 2418.
-- Mirror of 1353. 5009's last screen is "fix the DHD and then dial the
-- Stargate to Harset", the Jaffa equivalent of Marsh's order.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1355, '708 - Moh''katan briefed (5009): complete 5186, advance to 2418', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1355, 'dialog_choice', '5009', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1355, 'step_status', 708, '2417', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1355, 'complete_objective',  708, '5186', '{}', 0, 0),
  (1355, 'advance_step',        708, '2418', '{}', 0, 1),
  -- NO `~ INT_AStoryMissionActive` on `Castle_Mohkatan`: template 54's
  -- baseline is 0 (entity_templates.sql:63). See chain 1353.
  (1355, 'set_interaction_type', NULL, 'Castle_DHD',      '{"op": "|", "mask": "INT_MinigameLivewire"}',    0, 2);

-- ── Step 2418 — repair the DHD ──
--
-- D-CA09: Livewire is PROVISIONAL. The original minigame id for the DHD
-- repair is unrecovered; dialog 2586's "you dial the DHD... It fires"
-- weakly suggests Activate, which is not enough to pick it, and the only
-- alternative is the auto-win placeholder (click-to-win). Same policy as
-- Cellblock D-CB14. Swapping the game later is a one-word edit to chain
-- 1356's `target_key` plus the matching `INT_Minigame*` masks here and
-- in chains 1353/1355/1364.
--
-- Launcher/victory pair shape is precedent 1016/1017. Victory chains
-- are fired by id with `ResolvedActions::default()` and NO condition
-- evaluation (event_dispatch/mod.rs:53-82), which is why the step gate
-- lives on the launcher (1356) and chain 1357 has no trigger row and no
-- conditions.

-- Chain 1356: use the DHD at 2418 → Livewire.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1356, '708 - Repair DHD: start Livewire minigame', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1356, 'interact_tag', 'Castle_DHD', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1356, 'step_status', 708, '2418', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1356, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1357]}', 0, 0);

-- Chain 1357: Livewire victory → the DHD works; advance to the dial step.
--
-- No trigger row — invoked directly by the minigame callback in 1356.
--
-- `add_dialog_set 3073 {"slot": 162}` binds set-656 row 3073 ("Dial
-- Harset", `interaction_flags = 16` = INT_Dhd, `dialog_id` NULL) to the
-- DHD's template. INERT until packet CA02 lands — see "KNOWN GAPS" at
-- the top of this file. It is a TOPIC, not the dial capability:
-- template 162 already carries `interaction_type = 16`, so the DHD has
-- always been dialable and the dial is NOT gated on step 4462. Chain
-- 1358/1359's step condition gates the mission advance, which is the
-- part that matters.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1357, '708 - Livewire victory: DHD repaired, advance to step 4462', 'mission', 708, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1357, 'advance_step',        708, '4462', '{}', 0, 0),
  (1357, 'set_interaction_type', NULL, 'Castle_DHD', '{"op": "~", "mask": "INT_MinigameLivewire"}', 0, 1),
  (1357, 'add_dialog_set',      3073, NULL,   '{"slot": 162}', 0, 2);

-- ── Steps 4462 / 4469 — dial, then walk through ──
--
-- The dial and the crossing are both NATIVE (packet CA10, merged as
-- PR #663): `stargate_dialed` fires when the
-- four-second dial timer arms, `stargate_crossed` fires from
-- `cell::gate_travel::on_stargate_passage` immediately before the
-- world-transition teardown. `event_key` is the DESTINATION world name
-- from `resources.worlds.world`; "Harset" keys both chains so dialling
-- anywhere else does not advance 708 (loader/trigger.rs:58-63).
--
-- Both dispatchers populate `archetype` (event_dispatch/stargate.rs:
-- 96-101), so the 5010/5011 split below is a real gate — unlike on a
-- `dialog_choice` chain.
--
-- `remove_dialog_set` takes the dialog_set_map id in `target_id` plus
-- the template in `slot`, so 3073/162 here undoes 1357's bind exactly
-- (audit.md, "Minigame victory" bullet; precedent 1018's
-- `remove_dialog_set 5866 {"slot": 17}`).
--
-- DISPLAY CAVEAT for 5010/5011: `fire_stargate_dialed` stamps no
-- `target_entity_id`, so `display_dialog` falls back to the player's
-- `last_interaction_target` (executor/dialog.rs:54-90). At step 4462
-- the DHD has no matching interact chain — 1356 requires 2418 — so the
-- click that opens the dial UI falls through to `handle_interact`,
-- which pins the DHD (interactions/dispatch/interact.rs:89-91). The
-- portrait therefore binds to the DHD while the screens' own speakers
-- (261 Marsh / 2499 Moh'katan) name the voices. If UAT shows the final
-- line missing, a null `last_interaction_target` is the cause.

-- Chain 1358: dialled Harset (Tau'ri) → advance to 4469, play 5010.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1358, '708 - Dialled Harset (Tau''ri): advance to 4469, display 5010', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1358, 'stargate_dialed', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1358, 'step_status', 708,  '4462', 'eq',  'active', 0),
  (1358, 'archetype',   NULL, NULL,   'neq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1358, 'advance_step',      708, '4469', '{}',            0, 0),
  (1358, 'display_dialog',   5010, NULL,   '{}',            0, 1),
  (1358, 'remove_dialog_set', 3073, NULL,  '{"slot": 162}', 0, 2);

-- Chain 1359: dialled Harset (Jaffa) → advance to 4469, play 5011.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1359, '708 - Dialled Harset (Jaffa): advance to 4469, display 5011', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1359, 'stargate_dialed', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1359, 'step_status', 708,  '4462', 'eq', 'active', 0),
  (1359, 'archetype',   NULL, NULL,   'eq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1359, 'advance_step',      708, '4469', '{}',            0, 0),
  (1359, 'display_dialog',   5011, NULL,   '{}',            0, 1),
  (1359, 'remove_dialog_set', 3073, NULL,  '{"slot": 162}', 0, 2);

-- Chain 1360: stepped through to Harset → 708 complete.
--
-- Step 4469 is "Enter the active Stargate to leave the Castle"
-- (task 6404, objective 5200) — the last step of the last Castle
-- mission. `complete_mission` rather than `complete_objective 5200`
-- for the wire reason in chain 1322's comment.
--
-- The crossing event is emitted BEFORE the GateTravel teardown (packet
-- CA10), so the player's cell entity still exists when this runs. There
-- is no interaction flag to clear: everything 708 painted is already
-- cleared or belongs to another player's run.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1360, '708 - Crossed to Harset: complete mission 708', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1360, 'stargate_crossed', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1360, 'step_status', 708, '4469', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1360, 'complete_mission', 708, NULL, '{}', 0, 0);

-- ── Relog restore chains ──
--
-- `interaction_type_flags` and `available_interactions` are in-memory
-- only, so every bit painted above has to be repainted on
-- `player_loaded Castle`. One chain per step that owns a bit; step 2416
-- owns none (chains 1350/1351 paint at the 2416→2417 TRANSITION, and
-- 1362/1363 cover the resulting 2417 state), and step 4469 owns none.
-- All are idempotent `|` / re-binds.

-- Chain 1361: relog at step 2415 → repaint both diagnosis affordances.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1361, '708 - Relog at step 2415: restore guard and panel cues', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1361, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1361, 'step_status', 708, '2415', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (1361, 'set_interaction_type', NULL, 'Castle_SurrenderGuard', '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0),
  (1361, 'set_interaction_type', NULL, 'Castle_AccessPanel',    '{"op": "|", "mask": "INT_MissionWorldObject"}',  0, 1);

-- Chain 1362: relog at step 2417 (Tau'ri) → repaint Marsh's "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1362, '708 - Relog at step 2417 (Tau''ri): restore Col. Marsh cue', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1362, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1362, 'step_status', 708,  '2417', 'eq',  'active', 0),
  (1362, 'archetype',   NULL, NULL,   'neq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1362, 'set_interaction_type', NULL, 'Castle_ColMarsh', '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

-- Chain 1363: relog at step 2417 (Jaffa) → repaint Moh'katan's "!".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1363, '708 - Relog at step 2417 (Jaffa): restore Moh''katan cue', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1363, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1363, 'step_status', 708,  '2417', 'eq', 'active', 0),
  (1363, 'archetype',   NULL, NULL,   'eq', '8',      1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1363, 'set_interaction_type', NULL, 'Castle_Mohkatan', '{"op": "|", "mask": "INT_AStoryMissionActive"}', 0, 0);

-- Chain 1364: relog at step 2418 → repaint the DHD's Livewire offer.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1364, '708 - Relog at step 2418: restore DHD Livewire cue', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1364, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1364, 'step_status', 708, '2418', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1364, 'set_interaction_type', NULL, 'Castle_DHD', '{"op": "|", "mask": "INT_MinigameLivewire"}', 0, 0);

-- Chain 1365: relog at step 4462 → re-bind the "Dial Harset" topic.
-- Inert until CA02 — see chain 1357's comment and "KNOWN GAPS".
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1365, '708 - Relog at step 4462: restore Dial Harset topic', 'mission', 708, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1365, 'player_loaded', 'Castle', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1365, 'step_status', 708, '4462', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1365, 'add_dialog_set', 3073, NULL, '{"slot": 162}', 0, 0);
