-- ============================================================
-- Harset Loyalist Jaffa mission chains (chain ids 6301-6500)
-- ============================================================
-- Campaign: docs/analysis/harset-rebuild/ (ledger packets H20-H28).
-- Authoring rules: work-packets.md "Worker Input And Ownership";
-- canonical tags: worknotes/harset-tags.md; dialog-set-map ids 120001-120100.
-- Every coordinate in this file must be recovered from the 2009 Python
-- or pinned in M0; mission-scoped hostiles are spawned into the player's
-- own Market/Storage instance, never into world 57 or 68.
-- Sub-allocation per mission is fixed in the ledger table; never reuse a
-- 1xxx-5xxx id.
--
-- Populated so far:
--   1324 Present Yourself  6301-6310  (packet H20)
--   1326 Lan'toc           6331-6345  (packet H22)
-- Worknote for both: docs/analysis/harset-rebuild/worknotes/H20-H22.md
-- Still empty: 1325 (6311-6330, H21), 1343 (6346-6365, H23), 1347
-- (6366-6380, H24), 1348 (6381-6400, H25), 1351 (6401-6415, H26),
-- 1352 (6416-6440, H27), 1353 (6441-6470, H28).
--
-- Base: content/harset-wave2 @ acc12c80 (PR #662 review fixes, main
-- through #680 — includes Castle CA02, which changed how a NULL
-- `dialog_id` dsm row loads; see the dsm 120002 note).
-- ============================================================

SET search_path = resources, pg_catalog;

-- ============================================================
-- HOW THESE CHAINS PAINT AND ROUTE A CLICK
-- ============================================================
--
-- Read this before adding a Harset talk chain. Four engine facts decide
-- the whole shape and none of them is obvious from the seed rows.
--
-- 1. THE QUEST ICON COMES FROM `add_dialog_set`, NEVER FROM
--    `set_interaction_type`. Worlds 57 and 68 are SHARED persistent
--    spaces (D-H04: Harset_CmdCenter is `Instanced="false"`), and
--    `set_interaction_type` mutates `CellEntity::interaction_type_flags`
--    and fans the new value to every witness — so one player clearing
--    Moh'katan's icon would un-click him for every other player still on
--    the step. `add_dialog_set` writes the firing player's
--    `available_interactions[template_id]` and pushes
--    `base_flags | entry.interaction_flags` to that player only
--    (`cell/content/executor/dialog.rs`). Every NPC in this file is a
--    shared-hub NPC, so every bit here is a per-player dialog-set bind.
--
--    Templates 42 (Ba'al), 54 (Moh'katan) and 204 (Ra's Former Jaffa)
--    all carry `entity_templates.interaction_type = 0`, so without a
--    bind the client renders them as scenery and never sends the click.
--    A bind is MANDATORY for every clickable beat, not cosmetic. It also
--    needs `entity_templates.has_dynamic_properties = true` on the
--    target template, or the deferred push at
--    `cell/space_manager/aoi.rs` never reaches a player who was already
--    loaded when the bind happened; 42, 54 and 204 all have it.
--
-- 2. A CLICK TAKES ONE OF TWO ROUTES, AND THE ROUTE DECIDES WHETHER A
--    LATER `display_dialog` CAN WORK.
--
--    `cell/cell_methods/player/interaction/interact.rs` fires
--    `fire_interact_tag` FIRST and only falls through to
--    `interactions::handle_interact` when no chain matched
--    (`matched = !resolved.actions.is_empty()`).
--
--      Route A — an `interact_tag` chain matched. `fire_interact_tag`
--      stamps `target_entity_id` into the chain context, so a
--      `display_dialog` in THAT chain resolves its NPC. But
--      `handle_interact` never runs, so the player's
--      `last_interaction_target` pin is NOT written — its only write
--      site repo-wide is `cell/interactions/dispatch/interact.rs`.
--
--      Route B — no chain matched. `handle_interact` pins
--      `last_interaction_target`, opens the first entry of
--      `available_interactions[template]` THAT CARRIES A DIALOG
--      (interaction-only rows are stepped over), and fires `dialog_open`.
--      If no bound entry has a dialog, the click does nothing at all.
--
--    CONSEQUENCE, and the reason chains 6302/6332 were deleted before
--    this file shipped: a `display_dialog` on a FOLLOW-UP trigger
--    (`dialog_choice`, `dialog_open`) has no `target_entity_id` of its
--    own and can only resolve its NPC through the pin. So a
--    `dialog_choice` chain may display a dialog only if the dialog it
--    hangs off was opened by ROUTE B. An `interact_tag` chain that
--    displays the offer and a `dialog_choice` chain that displays the
--    follow-up cannot both work: the first defeats the second, silently,
--    with a `warn!` and no wire frame (`executor/dialog.rs`). This is
--    exactly why the shipped Castle pattern (chain 1012 binds, the
--    native path opens, 1014/1015 and 1020/1021 hang off the choice)
--    is the one copied here.
--
--    So: the OFFER dialogs (4357, 4373) are opened by Route B — the bind
--    alone — and the accept chains hang off `dialog_choice`. Every other
--    beat, where nothing needs to display a second dialog afterwards,
--    uses an `interact_tag` chain and Route A.
--
-- 3. TWO INVARIANTS THE ENGINE WILL NOT ENFORCE FOR YOU.
--
--    (a) BIND SIDE: at most one dsm is bound to a given template at any
--        time, and the chain that binds the next one removes the
--        previous one in the same action list.
--        `interactions/dispatch/interact.rs` picks the FIRST BOUND ENTRY
--        THAT HAS A DIALOG (`entries.iter().find_map(|&(_, dialog_id, _)|
--        dialog_id)` — it steps over interaction-only rows, see the
--        dsm 120002 note below) — so the OLDEST dialog-bearing bind wins
--        — and Moh'katan (template 54) accumulates
--        binds from 1324, 1325, 1326, 1343, 1347, 1352 … across packets.
--        A leaked bind leaves a stale icon, and in any state that has no
--        matching `interact_tag` chain (every offer state, by rule 2) it
--        also routes the click to the WRONG DIALOG.
--
--    (b) TRIGGER SIDE: at most one `interact_tag` chain per tag may have
--        satisfiable conditions at any point in mission-state space.
--        `ChainEngine::resolve_event` appends the actions of EVERY chain
--        whose conditions pass and never breaks, so two matching chains
--        both push a `display_dialog`; the second `send_dialog_display`
--        overwrites `open_dialog_id` and the first dialog's button is
--        then rejected by the #479 gate. The player sees two stacked
--        dialogs and the top one does nothing. Conditions for all chains
--        are frozen before any action runs, so an action cannot flip a
--        sibling's gate within the same dispatch — the exclusion has to
--        be structural.
--
--        Every Moh'katan offer in this campaign therefore gates on its
--        predecessor's completion. H21/H23/H24/H27 must extend that
--        ladder, not fork it.
--
-- 4. BINDS DO NOT SURVIVE A CROSS-WORLD HOP, AND `player_loaded` IS THE
--    ONLY RESTORE SITE. `available_interactions` is in-memory on the cell
--    entity, which is destroyed and recreated when the player crosses
--    from 57 to 68 through the Command Center door (chain 6006).
--    `fire_player_loaded` runs on initial login, gate travel AND the
--    cross-world path (`base_messages/player_init/mod.rs`), so a
--    `player_loaded` chain gated on the active step is both the "paint it
--    when you walk in" chain and the "re-paint it after a relog" chain —
--    one row does both jobs. Every bind in this file has exactly one
--    matching `player_loaded` chain for the state it belongs to.
--
--    A `player_loaded` chain is not enough ON ITS OWN when the state it
--    paints becomes true while the player is already standing in the
--    world — `player_loaded` is an edge, and there is no second edge to
--    catch. That case needs a level-triggered partner; chain 6341 is the
--    one instance of it in this file and its comment explains the shape.
--    The
--    world-57 NPCs restore on `player_loaded 'Harset'`, the world-68 ones
--    on `player_loaded 'Harset_CmdCenter'`. A useful side effect: because
--    the entity is recreated, `available_interactions` starts empty on
--    every world entry, so a restore chain cannot accumulate duplicate
--    binds even though `add_dialog_set` does not dedupe.
--
-- Four smaller rules that bite in this file:
--
--   * DIALOGS 4375 AND 4376 HAVE NO BUTTONS AT ALL, AND CHAINS 6337/6338
--     DEPEND ON THAT. A dialog whose screens carry zero
--     `dialog_screen_buttons` rows sends `Event_NetOut_DialogButtonChoice`
--     with `ButtonId = -1` when the player closes it (Done / Decline /
--     the window X); a dialog that HAS buttons sends nothing on close and
--     fires only on a button click. So keying `dialog_choice` on a
--     button-less dialog is correct and precedented (`Castle.py` did it
--     for 2574/2575; shipped chains 1020/1021 do it for 2300/5020) — and
--     ADDING A BUTTON TO 4375 OR 4376 SILENTLY KILLS the chain that
--     closes step 3960, stranding the mission with no error anywhere.
--     Check by `screen_id` (`dialog_screen_buttons`' THIRD column), never
--     by dialog id — a grep on the dialog id matches button ids and lies.
--     Today: 4375 owns screens 80986-80988 and 4376 owns 80989-80993,
--     none of which appears in `dialog_screen_buttons.sql`.
--
--   * `complete_objective` on a step's LAST NON-OPTIONAL objective calls
--     `mission.complete()` (`cell/missions/progression.rs`), completing
--     the whole mission rather than advancing. Every step in 1324 and
--     1326 has exactly one non-optional objective, so `complete_objective`
--     is never used here — `advance_step` moves between steps (it
--     force-completes the outgoing step's objectives on the way) and
--     `complete_mission` ends the terminal step.
--   * `fire_dialog_choice` populates world and mission context but NOT
--     `archetype` (`content/event_dispatch/dialog.rs`), and
--     `Condition::Archetype` reads a missing `archetype` param as -1. An
--     `archetype eq 8` row on a `dialog_choice` chain would therefore
--     fail closed and the chain would never fire. The archetype gate
--     lives on the chains that put the dialog on screen instead; a Human
--     can never reach the button because the server never displays the
--     dialog to them, and `dialogButtonChoice` is rejected outright
--     unless `CellEntity::open_dialog_id` matches (#479).
--   * A COMPLETED STEP DOES NOT PERSIST AS `completed`. `advance_step`
--     writes `completed_step_ids: vec![]` through the outbox
--     (`content/executor/mission.rs`), so after a relog an earlier step
--     reads `not_active`, not `completed`. Never gate a restore chain on
--     `step_status ... eq 'completed'`; gate on the step that IS active.
--
-- ============================================================
-- DIALOG-SET-MAP ROWS AUTHORED BY THIS FILE (120001-120100)
-- ============================================================
--
-- The 2009 data ships dialog sets 1391 ("Present Yourself", mission 1324)
-- and 1393 ("Lan'toc", mission 1326) complete with their dialogs. Two
-- binds they need do not exist, and both are authored below rather than
-- substituted, because the alternative in each case is an NPC the client
-- will not let the player click.
--
-- Shipped rows this file consumes unchanged (evidence:
-- db/resources/Dialogs/Seed/dialog_set_maps.sql):
--
--   5149  set 1391  dialog 4357  flags 134217728  INT_NonAStoryMissionAvaliable
--   5151  set 1391  dialog 4363  flags 268435456  INT_NonAStoryMissionActive
--   5159  set 1393  dialog 4373  flags 134217728  INT_NonAStoryMissionAvaliable
--   5161  set 1393  dialog 4377  flags 536870912  INT_NonAStoryMissionTurnIn
--
-- Shipped rows deliberately NOT bound: 5150 (dialog 4358) and 5160
-- (dialog 4374) both carry `interaction_flags = 0`. They are the long
-- conversations behind the offer, displayed by a chain action, not
-- indicators — binding a flags-0 row merges 0 into the NPC's flags and
-- leaves it unclickable.

-- dsm 120001 — Moh'katan's 1324 turn-in indicator.
--
-- Shipped row 6791 already binds dialog 4365 (the "It went well, I
-- assume." debrief) to set 1391, but with `interaction_flags = 0`, which
-- would merge nothing onto template 54's own `interaction_type = 0` and
-- leave Moh'katan unclickable at the one moment the mission requires the
-- player to click him. The sibling set 1393 carries 536870912
-- (INT_NonAStoryMissionTurnIn) on exactly this role (row 5161, dialog
-- 4377), so the flag value is recovered from the shipped data rather than
-- chosen. Row 6791 is left untouched.
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions)
VALUES (120001, 1391, 4365, 'Present Yourself', 536870912, 1, '{}', '{}', '{}', '{}');

-- dsm 120002 — the Former-Ra Jaffa's 1326 "!" indicator.
--
-- Dialogs 4375 (the oath) and 4376 (the refusal) have screens but no
-- dialog_set_maps row at all, so template 204 has no bit from any source.
-- One bind is enough for BOTH Former-Ra Jaffa spawns:
-- `send_interaction_update_if_visible` fans the merged flags to every
-- entity of the bound template in the player's AoI, so binding once on
-- slot 204 lights every template-204 NPC the player can see.
--
-- `dialog_id` IS NULL ON PURPOSE — this is an interaction-only row.
--
-- An earlier draft put 4375 (the oath) on the row, on the belief that a
-- NULL `dialog_id` was dropped at load and would make the bind a silent
-- cache miss. That WAS true, and stopped being true with Castle packet
-- CA02: `cell/spawner/dialogs.rs::load_dialog_set_maps` now KEEPS such
-- rows as `DialogSetMapEntry { dialog_id: None, .. }`, precisely so a
-- bind can raise an indicator with no dialog behind it (Castle.py binds
-- seven of them — 3062, 3071, 3073, 5828, 5829, 5846, 5863). The bit is
-- all this row was ever for; which dialog opens is decided by chains
-- 6335/6336 (Route A), never by the row.
--
-- Naming a dialog here was also actively dangerous, and that is the
-- reason for the change rather than mere tidiness. Both Lan'toc Jaffa
-- are template 204, so ONE bind lights BOTH (that is intended — either
-- one satisfies the step). But the click routes per-NPC: if a future
-- edit ever leaves this bind live in a state where 6335/6336 do not
-- match, the native Route B path opens the first dialog-bearing entry on
-- slot 204 — which would have been 4375 — on BOTH NPCs, and the Jaffa
-- who is supposed to REFUSE swears allegiance instead. With `dialog_id`
-- NULL, `find_map` steps over this row and Route B opens nothing at all:
-- a dead click instead of the wrong outcome. That is the 2026-09-18
-- Castle playtest lesson (two NPCs of one identity, one of them painted
-- with the wrong cue) applied before it can bite.
--
-- CONSTRAINT FOR H14/M0: template 204 must carry ONLY these two mission
-- Jaffa in world 57. `send_interaction_update_if_visible` fans the merged
-- flags to every template-204 entity in the player's AoI, so an ambient
-- Jaffa reusing slot 204 would light up with a mission cue and then do
-- nothing when clicked. Template 205 (`Harset_SuspiciousJaffa`) and 206
-- (`Harset_AngryJaffa`) are the identical body set and are what ambient
-- Ra's-Jaffa population should use.
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions)
VALUES (120002, 1393, NULL, 'Lan''toc', 268435456, 1, '{}', '{}', '{}', '{}');

-- Keep the sequence ahead of the ids this file writes without ever
-- lowering it. Four Harset seed files share the 120001+ block and load in
-- alphabetical order (goauld, jaffa, opcore, space), so a bare
-- `setval(..., 120002)` here would undo a higher value already set by
-- harset_goauld_chains.sql.
SELECT pg_catalog.setval(
    'dialog_set_maps_dialog_set_map_id_seq',
    GREATEST(120002, (SELECT last_value FROM dialog_set_maps_dialog_set_map_id_seq)),
    true);

-- ============================================================
-- MISSION 1324 — PRESENT YOURSELF (packet H20, chains 6301-6310)
-- ============================================================
--
-- Level 6, `mission_label = 'Harset'`, Loyalist Jaffa (archetype 8). Two
-- steps, one required objective each:
--
--   3953  "Talk to Ba'al."        objective 4543
--   3954  "Return to Moh'katan."  objective 4544
--
-- Both NPCs live in world 68 Harset_CmdCenter: Moh'katan is template 54
-- (tag `CmdCenter_Mohkatan`), Ba'al is template 42 (tag
-- `CmdCenter_Baal`). NEITHER SPAWN ROW EXISTS YET — packet H12 seeds
-- them after milestone M0 pins the coordinates. These chains bind by
-- template and trigger by tag, so they are correct the moment those rows
-- land; until then the mission is authored but unreachable, and the
-- packet is UAT-gated on M0.
--
-- Evidence: dialog set 1391 and its four dialogs are shipped 2009 data.
-- The step/objective ids are `mission_steps.sql:950,952` and
-- `mission_objectives.sql:997,999`. Nothing in this section is
-- RECOVERED_SCRIPT — no 2009 script survives for 1324 (only mission 742
-- has one) — so the chain shapes are RECONSTRUCTION from the dialog data
-- and the step text. The dialogs, their flags and their speakers are not.
--
-- CHAIN ID 6302 IS DELIBERATELY UNUSED. It held an
-- `interact_tag 'CmdCenter_Mohkatan'` chain that displayed the offer
-- blurb 4357. That is Route A (header rule 2), and it would have stopped
-- `handle_interact` writing the `last_interaction_target` pin, which is
-- the only thing chain 6303's `display_dialog 4358` could have resolved
-- its NPC through — so the mission would have been accepted and the
-- briefing conversation would silently never have played. The offer now
-- rides Route B: chain 6301's bind alone opens 4357. The id is left as a
-- hole rather than reused so this note stays attached to it.

-- Chain 6301: Moh'katan offers 1324.
--
-- The ONLY chain in the offer state. It paints the "?" on arrival in the
-- Command Center, re-paints it after a relog, and — because nothing
-- matches the click in this state — its bind is also what opens dialog
-- 4357 through `handle_interact`, pinning `last_interaction_target` on
-- the way so chain 6303 can display the follow-up.
--
-- Gated on archetype 8 because only Loyalist Jaffa are sent to present
-- themselves, and on `mission_status 1324 eq not_active` so the icon
-- disappears the moment the mission is accepted (and never returns once
-- it is completed).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6301, '1324 - Moh''katan offer: bind the "?" dialog set on Command Center entry', 'mission', 1324, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6301, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6301, 'world', 68, NULL, 'eq', NULL, 0),
  (6301, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6301, 'mission_status', 1324, NULL, 'eq', 'not_active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6301, 'add_dialog_set', 5149, NULL, '{"slot": 54}', 0, 0);

-- Chain 6303: the Accept button on 4357 -> accept 1324, move the icon to
-- Ba'al.
--
-- `mission_status 1324 eq not_active` is the required accept gate
-- (content-chains.instructions.md "Mission grants must gate on
-- not_active"); `cell/missions/lifecycle.rs::accept_mission` refuses a
-- re-accept authoritatively as well, but the condition stops the other
-- three actions firing spuriously.
--
-- No archetype condition — see the header's third small rule. The Human
-- path is closed one step earlier: only chain 6301's archetype-gated
-- bind puts 4357 on screen, and `handle_dialog_button_choice` drops any
-- choice whose dialog id is not the player's pinned `open_dialog_id`.
--
-- FIDELITY NOTE — "More Info" is indistinguishable from "Accept", and it
-- consumes the click. Screen 58166 of dialog 4357 carries button 9
-- "More Info" and button 8 "Accept"
-- (`dialog_screen_buttons.sql:6545,6547`). `fire_dialog_choice` passes
-- `button_id` into the context but no authorable condition reads it, so
-- both buttons fire this chain. Worse, `handle_dialog_button_choice`
-- CLEARS `open_dialog_id` before firing, so a player who clicks More
-- Info first has already accepted the mission and their subsequent
-- Accept click is rejected by the #479 gate and appears to do nothing.
-- Net outcome is benign — mission accepted, conversation shown — but
-- "nothing happens when I click Accept" is what a UAT will report.
-- Recovering the real two-button behaviour needs a `button_id`
-- condition in the loader; recorded in worknotes/H20-H22.md.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6303, '1324 - Accept: accept the mission, show 4358, move the icon to Ba''al', 'mission', 1324, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6303, 'dialog_choice', '4357', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6303, 'world', 68, NULL, 'eq', NULL, 0),
  (6303, 'mission_status', 1324, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6303, 'accept_mission', 1324, NULL, '{}', 0, 0),
  (6303, 'display_dialog', 4358, NULL, '{}', 0, 1),
  (6303, 'remove_dialog_set', 5149, NULL, '{"slot": 54}', 0, 2),
  (6303, 'add_dialog_set', 5151, NULL, '{"slot": 42}', 0, 3);

-- Chain 6304: step 3953 — click Ba'al -> the council scene, then advance.
--
-- Route A: `fire_interact_tag` stamps `target_entity_id`, so this
-- chain's own `display_dialog` resolves Ba'al without the pin. Nothing
-- needs to display a second dialog afterwards, so the route is safe here.
--
-- Dialog 4363 is the 10-screen council (`dialog_screens.sql:17909-17927`,
-- screens 81723-81732; speakers 942 Ba'al, 941 Marsh, 944 Anat, plus the
-- player's own 0). `advance_step` rather than
-- `complete_objective 4543`: 4543 is the only non-optional objective of
-- 3953, so completing it would complete the whole mission and orphan the
-- return step.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6304, '1324 - Step 3953: Ba''al council dialog 4363, advance to 3954', 'mission', 1324, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6304, 'interact_tag', 'CmdCenter_Baal', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6304, 'world', 68, NULL, 'eq', NULL, 0),
  (6304, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6304, 'step_status', 1324, '3953', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6304, 'display_dialog', 4363, NULL, '{}', 0, 0),
  (6304, 'advance_step', 1324, '3954', '{}', 0, 1),
  (6304, 'remove_dialog_set', 5151, NULL, '{"slot": 42}', 0, 2),
  (6304, 'add_dialog_set', 120001, NULL, '{"slot": 54}', 0, 3);

-- Chain 6305: step 3954 — click Moh'katan -> debrief 4365, complete.
--
-- Terminal step, so `complete_mission` is correct here.
--
-- ACTION ORDER IS LOAD-BEARING: the `remove_dialog_set` runs BEFORE the
-- `complete_mission`. `Action::CompleteMission` awaits
-- `fire_mission_completed` inside the executor arm
-- (`content/executor/mission.rs`), so any chain triggered by
-- `mission_completed '1324'` — H21 will add one to paint 1325's offer on
-- Moh'katan — executes its actions in the middle of this list. Removing
-- first means the next mission's bind is the only one on slot 54 when
-- the dust settles, without depending on `retain`'s behaviour against a
-- bind that did not exist when this chain was written.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6305, '1324 - Step 3954: Moh''katan debrief 4365, complete the mission', 'mission', 1324, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6305, 'interact_tag', 'CmdCenter_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6305, 'world', 68, NULL, 'eq', NULL, 0),
  (6305, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6305, 'step_status', 1324, '3954', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6305, 'display_dialog', 4365, NULL, '{}', 0, 0),
  (6305, 'remove_dialog_set', 120001, NULL, '{"slot": 54}', 0, 1),
  (6305, 'complete_mission', 1324, NULL, '{}', 0, 2);
-- GC3: grant_xp goes here once the reward formula gate is answered
-- (D-H10 / Cellblock GC3). `missions.reward_xp` and `reward_naq` are 0 on
-- 1324 as on all 1,040 mission rows, and `grant_xp` has an executor arm
-- since PR #618 with zero seed rows — only the formula is missing. One
-- action row on this chain closes it; do not hardcode a constant.

-- Chain 6306: relog restore for step 3953 (Ba'al's "!").
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6306, '1324 - Restore: re-bind Ba''al''s active icon while step 3953 is open', 'mission', 1324, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6306, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6306, 'world', 68, NULL, 'eq', NULL, 0),
  (6306, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6306, 'step_status', 1324, '3953', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6306, 'add_dialog_set', 5151, NULL, '{"slot": 42}', 0, 0);

-- Chain 6307: relog restore for step 3954 (Moh'katan's turn-in "?").
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6307, '1324 - Restore: re-bind Moh''katan''s turn-in icon while step 3954 is open', 'mission', 1324, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6307, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6307, 'world', 68, NULL, 'eq', NULL, 0),
  (6307, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6307, 'step_status', 1324, '3954', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6307, 'add_dialog_set', 120001, NULL, '{"slot": 54}', 0, 0);

-- ------------------------------------------------------------
-- 6308-6310 — ARRIVAL DIALOG 6169: NOT AUTHORED, AND WHY
-- ------------------------------------------------------------
--
-- The H20 packet asks for the Harset arrival briefing (dialog 6169,
-- "You're on Harset now - once the site of one of Ra's archeological
-- digs … report in at the command center for debriefing") to play for
-- every archetype on the first `player_loaded Harset`, gated on missions
-- 1324, 1361 and 1200 all being `not_active`. It belongs HERE rather
-- than in the OP-CORE or Goa'uld file because it is faction-neutral and
-- would otherwise be written three times; the other two lanes are told
-- not to author it.
--
-- It is not authored as an executable chain, because BOTH delivery paths
-- are dead against today's data and a chain that resolves and then does
-- nothing is worse than a documented hole:
--
--   * `display_dialog 6169` on a `player_loaded` trigger BAILS. The
--     executor needs an NPC entity id for the client's portrait lookup
--     and resolves it from `params.target_entity_id` (stamped only by
--     `interact_tag` / `interact_template`), then the player's
--     `last_interaction_target` pin, then a monologue fallback for
--     dialogs whose every screen has `speaker_id = 0`. A fresh world
--     entry has none of the three, and 6169 is not a monologue — 3 of its
--     5 screens speak as 3219 (`dialog_screens.sql:27649-27659`). The
--     result is a `warn!` and no wire frame: the chain looks wired and
--     the voice line never plays.
--   * `add_dialog_set 7016` (set 1992, dialog 6169, flags 16777216
--     INT_AStoryMissionActive) needs a `slot`, and `slot` is an
--     `entity_templates.template_id`. Speaker 3219 has an empty `name`
--     (`speakers.sql:505`), NO entity template references it, and nothing
--     anywhere in the repo references set 1992 or dsm 7016. There is no
--     NPC to bind it to.
--
-- To land this, milestone M0 / packet H14 must produce: an
-- `entity_templates` row for the arrival NPC carrying `speaker_id = 3219`
-- (the 2009 "keep moving, lots of wounded coming through" gate medic)
-- and `has_dynamic_properties = true`, plus a `spawnlist` row for it in
-- world 57 near the Stargate with a tag registered in
-- worknotes/harset-tags.md. Then chain 6308 is:
--
--   6308  player_loaded 'Harset'
--         + world eq 57
--         + mission_status 1324 eq not_active
--         + mission_status 1361 eq not_active
--         + mission_status 1200 eq not_active
--         -> add_dialog_set 7016 {"slot": <the new template id>}
--
-- and, because the bind alone opens the dialog through Route B, no
-- second chain is needed to display it — only one to remove the bind
-- once the player has heard it. No archetype condition: the briefing is
-- the same for all three factions, which is exactly why the three
-- `mission_status` gates are the guard (one per faction's first mission)
-- rather than an archetype test.
--
-- Precedent for shipping the hole rather than a silent no-op: H10's
-- chain 6007, disabled with its replay guard asserting the disabled
-- state until M0 re-pins it (harset_space_chains.sql).
--
-- 6309 and 6310 are unallocated.

-- ============================================================
-- MISSION 1326 — LAN'TOC (packet H22, chains 6331-6345)
-- ============================================================
--
-- Level 10, Loyalist Jaffa. Two steps, one required objective each:
--
--   3960  "Present the Lan'toc."   objective 4551
--   4603  "Return to Moh'katan."   objective 4620
--
-- Lan'toc is a loyalty rite. Moh'katan (template 54, world 68) sends the
-- player to Jaffa in the Harset Jaffa Zone (world 57) who were formerly
-- loyal to Ra; each one either swears allegiance (dialog 4375) or refuses
-- (dialog 4376, "Then you must leave the city"). Presenting the rite once
-- satisfies step 3960 whichever answer comes back.
--
-- TWO NPCs, NOT A PLAYER CHOICE. The outcome is the NPC's, not the
-- player's: neither outcome dialog has a branchable button, no dialog in
-- the shipped data forks between them, and the engine has no randomness
-- primitive and no authorable condition that could pick between two
-- dialogs on one tag. Both spawns are template 204 "Ra's Former Jaffa"
-- with distinct tags — `Harset_FormerRaJaffa` swears,
-- `Harset_FormerRaJaffa2` refuses — which also reads as Moh'katan's
-- briefing intends ("Convert as many as you wish", dialog 4374 screen
-- 118959). NEITHER SPAWN ROW EXISTS YET; packet H14 seeds both after M0.
--
-- FIDELITY NOTE: the refusing Jaffa is NOT removed from the city, and
-- cannot be. "Then you must leave" is dialog text only. World 57 is a
-- shared persistent hub and campaign guardrail D-H03 forbids
-- `destroy_entity`, `set_visible`, `set_aggression` and `generate_threat`
-- on a shared-hub NPC — there is no per-witness visibility overlay, so
-- hiding him for one player would hide him for everyone, including
-- players who have not reached the mission. The promise is unkeepable BY
-- CONSTRUCTION for as long as the hub is shared; this is not a TODO, and
-- it must not be "fixed" with a per-witness visibility hack.
--
-- PREREQUISITE GATE. The offer chains carry `mission_status 1324 eq
-- completed` AND `mission_status 1325 eq completed`. Moh'katan hands out
-- 1324, 1325 and 1326 (and later 1343, 1347, 1352) from the same template
-- and the same tag, and two offer chains matching one right-click would
-- both push a `display_dialog` (header rule 3b). Gating each offer on its
-- predecessors' completion keeps exactly one live at a time and matches
-- the 2009 level curve (1324 L6, 1325 L8, 1326 L10). The 1324 row is
-- redundant in normal play — finishing 1325 implies finishing 1324 — but
-- nothing enforces that ordering (a GM grant does not), and one condition
-- row is cheaper than a disjointness argument.
--
-- The cost is that 1326 is unreachable until packet H21 lands 1325 —
-- recorded as a dependency in worknotes/H20-H22.md rather than worked
-- around, because the alternative (gating on 1324 alone) makes 1325 and
-- 1326 collide the moment H21 ships.
--
-- CHAIN ID 6332 IS DELIBERATELY UNUSED, for the same reason as 6302: an
-- `interact_tag` chain displaying the offer blurb 4373 would have taken
-- Route A and left chain 6333's `display_dialog 4374` with no NPC to
-- resolve. The offer rides Route B — chain 6331's bind alone.

-- Chain 6331: Moh'katan offers 1326. The only chain in the offer state;
-- its bind also opens dialog 4373 through `handle_interact`.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6331, '1326 - Moh''katan offer: bind the "?" dialog set on Command Center entry', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6331, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6331, 'world', 68, NULL, 'eq', NULL, 0),
  (6331, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6331, 'mission_status', 1326, NULL, 'eq', 'not_active', 2),
  (6331, 'mission_status', 1325, NULL, 'eq', 'completed', 3),
  (6331, 'mission_status', 1324, NULL, 'eq', 'completed', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6331, 'add_dialog_set', 5159, NULL, '{"slot": 54}', 0, 0);

-- Chain 6341: the same bind, on the edge that 6331 cannot see.
--
-- EDGE-TRIGGER RACE (2026-09-18 Castle playtest, finding H9). Chain 6331
-- fires on `player_loaded`, which is an EDGE: it runs on login, gate
-- travel and the cross-world hop, and never again while the player stands
-- still. But 1326's gate flips to satisfied the instant 1325 completes,
-- and 1325 is turned in TO MOH'KATAN, IN WORLD 68 — the player is already
-- standing in the Command Center when the condition becomes true. With
-- 6331 alone the "?" would not appear until they walked out of the
-- Command Center and back in, which reads to a player as the next mission
-- simply not existing. This is the same shape as the Castle step that was
-- unreachable because the player was already inside the region when the
-- step activated.
--
-- `mission_completed` is the level trigger that closes it.
-- `fire_mission_completed` (`content/event_dispatch/mission.rs`) runs
-- AFTER `complete_mission_direct` has flipped the status and populates
-- world, archetype and the full mission context, so this chain can carry
-- the identical condition set to 6331 and be evaluated against the
-- post-completion state — including `mission_status 1325 eq completed`,
-- which is what has just become true.
--
-- The two chains are deliberately NOT merged into one chain with two
-- trigger rows. N trigger rows on one chain materialize N in-memory
-- `Chain`s sharing one condition set, which would work, but
-- `load_single_chain_for_test` returns only the first expansion — a test
-- written the obvious way would then guard the `player_loaded` row and
-- silently ignore this one. Two chain ids keep both honest.
--
-- Double-binding is not a hazard: the executor's `add_dialog_set` pushes
-- without dedupe, but `available_interactions` is rebuilt empty on every
-- world entry, so 6331 and 6341 can never both be live in one session —
-- and even if they were, both name dsm 5159, `find_map` takes the first
-- and `remove_dialog_set`'s `retain` drops every copy.
--
-- H21 OWES THE MIRROR OF THIS: when 1324 completes (chain 6305, also in
-- world 68) 1325's offer needs the same `mission_completed '1324'`
-- partner, or 1325 inherits exactly this bug. Chain 6305's action order
-- already reserves the slot for it.
--
-- ONE EDGE IN THIS FAMILY IS STILL OPEN, AND NO SEED ROW CAN CLOSE IT:
-- MISSION ABANDON. `abandonMission` is a client-callable cell method
-- (index 52, `cell_methods/missionary.rs`) and `missions::abandon_mission`
-- removes the mission row and fires NOTHING into the content engine —
-- there is no `mission_abandoned` trigger in `loader/trigger.rs` and no
-- dispatcher in `content/event_dispatch/`. So a player who abandons 1324
-- or 1326 while standing in the Command Center flips that mission back to
-- `not_active`, re-satisfying chain 6301's or 6331's gate, with no edge
-- left to fire: the offer icon does not come back until they cross a
-- world boundary. The same abandon strands whatever bind was live — drop
-- 1324 on step 3953 and Ba'al keeps dsm 5151, so he shows a stale "!" and
-- Route B replays the council dialog 4363 on click, because chain 6304's
-- step gate no longer matches.
--
-- Both symptoms self-heal on the next world transition, which rebuilds
-- `available_interactions` empty and re-fires every `player_loaded`
-- chain. Nothing here is Harset-specific: every offer chain in every lane
-- has it, and the fix is a Rust one (a `fire_mission_abandoned`
-- dispatcher plus a `mission_abandoned` trigger), which belongs to an H0x
-- packet — seed packets own no Rust paths. Recorded in
-- worknotes/H20-H22.md for the coordinator; do not work around it here.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6341, '1326 - Moh''katan offer: bind the "?" the moment 1325 completes (edge closer for 6331)', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6341, 'mission_completed', '1325', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6341, 'world', 68, NULL, 'eq', NULL, 0),
  (6341, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6341, 'mission_status', 1326, NULL, 'eq', 'not_active', 2),
  (6341, 'mission_status', 1325, NULL, 'eq', 'completed', 3),
  (6341, 'mission_status', 1324, NULL, 'eq', 'completed', 4);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6341, 'add_dialog_set', 5159, NULL, '{"slot": 54}', 0, 0);

-- Chain 6333: the Accept button on 4373 -> accept 1326, show the briefing.
--
-- Same "More Info is indistinguishable from Accept" fidelity note as
-- chain 6303; screen 80981 carries the same button pair
-- (`dialog_screen_buttons.sql:6555,6557`).
--
-- OPEN FIDELITY QUESTION: dialog 4374's LAST screen (118960) also carries
-- an Accept button (`dialog_screen_buttons.sql:6559`) and nothing
-- consumes it. 4374 is the only one of the nine H20/H22 dialogs with a
-- button on a non-blurb screen, which reads as evidence the 2009 accept
-- for 1326 sat on `dialog_choice '4374'`, with 4373's More Info opening
-- 4374 first. Not restructured on that reading without UAT: moving the
-- accept to 4374 would strand any player who clicks Accept on the blurb.
-- See worknotes/H20-H22.md.
--
-- No bind is added here. The next indicator belongs to the Former-Ra
-- Jaffa in world 57, and a bind made while the player stands in world 68
-- dies on the cross-world hop — chain 6334 paints it on arrival instead.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6333, '1326 - Accept: accept the mission and show Moh''katan''s briefing 4374', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6333, 'dialog_choice', '4373', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6333, 'world', 68, NULL, 'eq', NULL, 0),
  (6333, 'mission_status', 1326, NULL, 'eq', 'not_active', 1),
  (6333, 'mission_status', 1325, NULL, 'eq', 'completed', 2),
  (6333, 'mission_status', 1324, NULL, 'eq', 'completed', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6333, 'accept_mission', 1326, NULL, '{}', 0, 0),
  (6333, 'display_dialog', 4374, NULL, '{}', 0, 1),
  (6333, 'remove_dialog_set', 5159, NULL, '{"slot": 54}', 0, 2);

-- Chain 6334: paint the Jaffa Zone "!" on entering Harset, and re-paint
-- it after a relog. One bind covers both template-204 spawns.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6334, '1326 - Step 3960: bind the Former-Ra Jaffa active icon on Harset entry', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6334, 'player_loaded', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6334, 'world', 57, NULL, 'eq', NULL, 0),
  (6334, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6334, 'step_status', 1326, '3960', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6334, 'add_dialog_set', 120002, NULL, '{"slot": 204}', 0, 0);

-- Chain 6335: present the Lan'toc to the Jaffa who swears (dialog 4375).
--
-- Route A. Nothing needs to display a second dialog off this one, so the
-- missing pin does not matter — chains 6337/6338 only advance and
-- unbind.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6335, '1326 - Step 3960: present Lan''toc, this Jaffa swears (4375)', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6335, 'interact_tag', 'Harset_FormerRaJaffa', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6335, 'world', 57, NULL, 'eq', NULL, 0),
  (6335, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6335, 'step_status', 1326, '3960', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6335, 'display_dialog', 4375, NULL, '{}', 0, 0);

-- Chain 6336: present the Lan'toc to the Jaffa who refuses (dialog 4376).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6336, '1326 - Step 3960: present Lan''toc, this Jaffa refuses (4376)', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6336, 'interact_tag', 'Harset_FormerRaJaffa2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6336, 'world', 57, NULL, 'eq', NULL, 0),
  (6336, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6336, 'step_status', 1326, '3960', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6336, 'display_dialog', 4376, NULL, '{}', 0, 0);

-- Chain 6337: the rite was accepted -> step 3960 is satisfied.
--
-- `advance_step`, not `complete_objective 4551`. 4551 is the ONLY
-- non-optional objective of 3960 (`mission_objectives.sql:1009`), so
-- `complete_objective` would trip the all-required-complete branch in
-- `cell/missions/progression.rs` and complete the whole of 1326,
-- orphaning the turn-in step 4603. `advance_step` force-completes 4551 on
-- its way to 4603, so the objective is still completed exactly once.
--
-- The step condition is also the one-shot guard: `once` on
-- `content_triggers` is dead code, and after the advance 3960 is no
-- longer active, so a second Lan'toc on the other Jaffa resolves nothing.
-- It must be a STEP gate rather than a mission gate — 1326 is still
-- active on step 4603, so a `mission_status` gate would leave both Jaffa
-- live all the way to the turn-in.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6337, '1326 - Lan''toc accepted: advance to the return step', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6337, 'dialog_choice', '4375', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6337, 'world', 57, NULL, 'eq', NULL, 0),
  (6337, 'step_status', 1326, '3960', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6337, 'remove_dialog_set', 120002, NULL, '{"slot": 204}', 0, 0),
  (6337, 'advance_step', 1326, '4603', '{}', 0, 1);

-- Chain 6338: the rite was refused -> step 3960 is satisfied all the same.
-- Identical to 6337 except for the dialog the branch hangs off; the
-- mission does not care which answer came back, only that the rite was
-- presented.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6338, '1326 - Lan''toc refused: advance to the return step', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6338, 'dialog_choice', '4376', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6338, 'world', 57, NULL, 'eq', NULL, 0),
  (6338, 'step_status', 1326, '3960', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6338, 'remove_dialog_set', 120002, NULL, '{"slot": 204}', 0, 0),
  (6338, 'advance_step', 1326, '4603', '{}', 0, 1);

-- Chain 6339: paint Moh'katan's turn-in "?" on returning to the Command
-- Center, and re-paint it after a relog.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6339, '1326 - Step 4603: bind Moh''katan''s turn-in icon on Command Center entry', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6339, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6339, 'world', 68, NULL, 'eq', NULL, 0),
  (6339, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6339, 'step_status', 1326, '4603', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6339, 'add_dialog_set', 5161, NULL, '{"slot": 54}', 0, 0);

-- Chain 6340: step 4603 — click Moh'katan -> turn-in 4377, complete.
-- Remove-before-complete for the same reason as chain 6305.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6340, '1326 - Step 4603: Moh''katan turn-in 4377, complete the mission', 'mission', 1326, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6340, 'interact_tag', 'CmdCenter_Mohkatan', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6340, 'world', 68, NULL, 'eq', NULL, 0),
  (6340, 'archetype', NULL, NULL, 'eq', '8', 1),
  (6340, 'step_status', 1326, '4603', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6340, 'display_dialog', 4377, NULL, '{}', 0, 0),
  (6340, 'remove_dialog_set', 5161, NULL, '{"slot": 54}', 0, 1),
  (6340, 'complete_mission', 1326, NULL, '{}', 0, 2);
-- GC3: grant_xp goes here, same gate as chain 6305.

-- 6341 is the offer edge-closer, authored above next to chain 6331 so the
-- two halves of one gate read together. 6342-6345 are unallocated.
