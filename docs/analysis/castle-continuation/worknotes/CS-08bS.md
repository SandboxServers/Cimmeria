# CS-08bS: Seal for the Castle leg of mission 567 "Romney's Files"

Agent 1 of 3 (mission design). Labels: REPO (file:line), CIMMERIA_AUTHORING, OPEN. Owner decision OD-2: reconstruct the Castle leg that the cooked client does not ship. Nothing here is restoration. Presentation is `UATPending (end UAT, D-CA11)`. Ids come from the D-CA20 allocation for mission 567 (G6): set maps 1000018-1000019, chains 1394-1397 and 1400, overflow 10240-10249, template 416, spawn 13805. **Exception, needs a coordinator decision (R4a1a review):** D-CA20 allots mission 567 no dialog or screen ids, and the brief says Frozen (c) is stock. The two feedback monologues below use dialogs 10733/10734 and screens 10865/10866, which sit in the unallocated gap between the `repeatable_q1` row (dialogs to 10732, screens to 10864) and the D-CA19 bark blocks (dialogs 10740-10749, screens 10900-10919), free on all five refs. The coordinator must either extend the D-CA20 567 row with those four ids and widen the CS-08b1 allowlist (see ## Frozen (c), Build allowlist delta), or drop the monologues, which removes the visible feedback on a second cabinet click.

## Verdict

**SEALED, with OPEN items listed below, and two coordinator items before CS-08b1 is dispatched: (1) the dialog/screen ids and the CS-08b1 allowlist delta in ## Frozen (c); (2) OPEN-5, the cabinet sits 6.0 u inside Romney's aggro ring. Authority leg: every client forgery shape on the 567 Castle leg is refused by a server gate (see ## Authority); 2698 is granted once per accepted mission and 2012 is reached only through chain 1397, so the only advance-without-item path is a server fault (OPEN-7), plus residual OPEN-10 to OPEN-13.** The 567 Castle leg is a 3-state flow on the shipped steps (REPO `db/resources/Missions/Seed/mission_steps.sql:5199,5201,5203`): offer (Copplemann, slot 48) -> accept -> step 2000 (cabinet click grants item 2698 and advances to 2012) -> crossing to Harset advances to 4039 -> Harset turn-in (6503/6504, owned by the Harset agents).

Design choices, all CIMMERIA_AUTHORING:

1. The offer reuses shipped dialog 2043 (Accept only, one screen). Dialog 2042 stays unused (its More Info button cannot be discriminated from Accept, so the blurb path is not worth a chain). A real set map 1000018 binds dialog 2043 on slot 48, so no tag chain and no NULL carrier is needed.
2. The cabinet is a new template 416 (name moniker 7547) at a new spawn 13805 in Romney's office, with `interact_tag` `Castle_FileCabinet`.
3. Two new monologues (10733 found, 10734 nothing more) give visible feedback on every click, including the second.
4. The 2012 -> 4039 hop is a `stargate_crossed 'Harset'` chain with origin world 8, the same carry-in shape as REPO `castle_carry_in_chains.sql:3-16` (chain 1366).
5. Rewards are zero (REPO `missions.sql:1457`: `reward_naq` 0, `reward_xp` 0, `award_xp` true but no step/objective XP grant is authored).

OPEN items (none block the seal):

- OPEN-1: whether closing dialog 2043 with the X fires `dialog_choice 2043` (it has a button, so I expect no `-1` event). Chain 1396 would then accept on close. Verified in 567-U2.
- OPEN-2: cabinet mesh. Template 416 reuses the supply-chest mesh of template 410 (REPO `entity_templates.sql:1059`). No cabinet mesh name is recoverable from seeds. UAT row 567-U4.
- OPEN-3: `remove_item 2698` and `remove_dialog_set` on a player who has nothing to remove. Both are assumed to log and no-op. 567-U7.
- OPEN-4: all three steps ship `step_enabled false` (REPO `mission_steps.sql:5199-5203`). `step_status` conditions read persisted step state, not the flag, as in the other Castle missions, but 567-U1 proves it live.

## 567 IDs

| Fact | Value | Source |
|---|---|---|
| Mission | 567 "Romney's Files", level 1, non-A-story, shareable, abandonable, rewards 0 | REPO `missions.sql:1457` |
| Steps | 2000 "Retrieve Romney's Files." (index 0), 2012 "Get Romney's Files safely off of the Castle." (1), 4039 "Deliver Romney's Files to Copplemann" (2) | REPO `mission_steps.sql:5199,5201,5203` |
| Objectives / tasks | 2338 (2933/2934/5696), 2350 (6344), 4652 (6345) | evidence `g-567-706-1376.md:20` |
| Item | 2698 "Romney's Files", `container_sets` {2} (INV_Mission, 100 slots), `max_stack_size` 1, `flags` 32768 (no unique or bind bit). "Not deletable, not sellable" is a cooked-client record attribute (evidence `g-567-706-1376.md:24`), not a server seed column; see ## Items | REPO `items.sql:11707`; evidence `g-567-706-1376.md:24` |
| Cabinet name | moniker 7547 "Romney's File Cabinet" | REPO `texts.sql:54162` |
| Offer dialogs | 2042 (blurb, unused), 2043 (offer, button 8 Accept, screen 60106, speaker 258) | REPO `dialog_screens.sql:9469-9471`, `dialog_screen_buttons.sql` (button 1796) |
| Turn-in | map 2817, set 512, dialog 2044 (speaker 968) | REPO `dialog_set_maps.sql` (2817) |
| Giver | Castle Copplemann: template 48, spawn 87 tag `Castle_Coppleman` at (352.69, 70.27, 952.32), world 8. Harset Copplemann is the same template 48 in world 68. | REPO `spawnlist.sql` (87) |
| New (D-CA20) | set map 1000018 (1000019 reserved), template 416, spawn 13805, chains 1394-1397, 1400, 10240-10243 (10244-10249 reserved), dialogs 10733/10734, screens 10865/10866 | CIMMERIA_AUTHORING |

Prerequisites for the offer (REPO): 701 completed (chains 1234 and 1237 remove the slot-48 binds, `castle_701_chains.sql:412,516`) and 703 completed (chains 1272/1273, `castle_702_704_chains.sql:435,467`). 703 completion also accepts 704 (chain 1263), which is not a prerequisite here.

## Collision check

Command shape (PowerShell, anchored per table, over the five refs `2fad73404`, `content/cellblock-campaign`, `content/harset-campaign`, `content/tollana-campaign`, `origin/main`):

```
git grep -n -E "VALUES \((<ids>)," <ref> -- <path>
```

An earlier unanchored regex for 1394-1397/1400 returned noise because those numbers are also ids in other tables (`char_creation_choices`, `dialog_screen_buttons`, `dialogs`, `dialog_sets`, `event_sets`, `items_event_sets`, `missions`, `mission_*`, `texts`, `abilities`, `effects`). They are separate id spaces. The checks below are per table.

Output 1, the 567 regex (content chains and set maps):

```
--- A: chains 1394-1397,1400,10240-10249 (db/resources/Content) : VALUES \((139[4-7]|1400|1024[0-9]),
== 2fad73404 : 0 hit(s)
== content/cellblock-campaign : 0 hit(s)
== content/harset-campaign : 0 hit(s)
== content/tollana-campaign : 0 hit(s)
== origin/main : 0 hit(s)
--- B: set maps 1000018-19 (db/resources/Dialogs) : VALUES \((100001[89]),
== 2fad73404 : 0 hit(s)
== content/cellblock-campaign : 0 hit(s)
== content/harset-campaign : 0 hit(s)
== content/tollana-campaign : 0 hit(s)
== origin/main : 0 hit(s)
--- E: chains 1394|1395|1396|1397|1400 (db/resources/Content) : 0 hit(s) on all five refs
```

Output 2, template 416 and spawn 13805 (Entities and Worlds):

```
--- C: template 416 / spawn 13805 (db/resources/Entities, db/resources/Worlds) : VALUES \((416|13805),
== 2fad73404 : 4 hit(s)   (all four, identical on every other ref:)
   db/resources/Entities/Seed/blueprints.sql:631:            INSERT INTO blueprints (blueprint_id, ...) VALUES (416, 40, false, 781...
   db/resources/Entities/Seed/blueprints_components.sql:3113: INSERT INTO blueprints_components (...) VALUES (416, 5376, 1, 1);
   db/resources/Entities/Seed/blueprints_components.sql:3115: ... VALUES (416, 5379, 1, 1);
   db/resources/Entities/Seed/blueprints_components.sql:3117: ... VALUES (416, 5373, 1, 1);
== content/cellblock-campaign, content/harset-campaign, content/tollana-campaign, origin/main : the same 4 hits
```

All four hits are `blueprint_id` 416, a different id space. No `entity_templates` or `spawnlist` row has 416 or 13805 on any ref. Maxima on `2fad73404`: `entity_templates` 415, `spawnlist` 13804, so 416 and 13805 are the next free ids. (The debug-area spawn files go as high as 14095, but 13805 is itself free.)

Proposed dialog and screen ids (checked the same way, all five refs): `dialogs` 10733/10734 0 hits; `dialog_screens` 10865/10866 0 hits; `crates/resources` Rust overrides 0 hits. Q1 used dialog 10700 and screen 10800 (REPO `castle_side_stories_chains.sql:25`).

## Frozen (a) XML

None (stock). The client ships nothing for the cabinet or the offer binding, and nothing is added.

## Frozen (b) Missions seeds

None (stock). Mission 567, its steps, objectives and tasks stay as shipped.

## Frozen (c) Dialogs seeds and override

Stock except the offer map and two new monologues.

`dialog_set_maps.sql` (appended; flags 134217728 is NON_A_STORY available, REPO `crates/entity/src/interaction_flags.rs`; set 512 is reused, the map is looked up by map id only, REPO `executor/dialog/mod.rs:166-249`, so the shared set 512 does not affect the turn-in map 2817):

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000018, 512, 2043, 'Romney''s Files', 134217728, 1, '{}', '{}', '{}', '{}');
```

`dialogs.sql`:

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10733, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10734, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
```

`dialog_screens.sql` (speaker 0, one screen each, no buttons, so both are monologues, G5):

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10733, 10865, 'Behind a stack of dusty ledgers you find a grey folder stamped with Romney''s seal. These are the files Copplemann wants. Get them off the Castle.', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10734, 10866, 'The cabinet holds nothing else of use.', 0, 0);
```

`crates/resources/src/base/dialog_overrides/overrides_castle.rs`, appended after the `MEDICAL_Q1_DIALOG` entry in the same registry order (text byte-equal to the seed rows; ids <= 65535 so `every_cooked_override_element_id_fits_in_16_bits` holds):

```rust
DialogOverride { dialog_id: 10733, dialog_flags: 0, kismet_event_set_id: 0, ui_screen_type: 2,
    screens: &[DialogScreen { screen_id: 10865, speaker_id: 0,
        text: "Behind a stack of dusty ledgers you find a grey folder stamped with Romney's seal. These are the files Copplemann wants. Get them off the Castle.",
        buttons: &[] }] },
DialogOverride { dialog_id: 10734, dialog_flags: 0, kismet_event_set_id: 0, ui_screen_type: 2,
    screens: &[DialogScreen { screen_id: 10866, speaker_id: 0,
        text: "The cabinet holds nothing else of use.", buttons: &[] }] },
```

Dialog 2043 is not overridden. It is the shipped record (speaker 258 has a blank name, so the offer shows no speaker label; OPEN, cosmetic, 567-U2).

Placement (each statement is appended after the last INSERT of its file on `2fad73404` and before the trailing `setval`; dialogs.sql :10870 and dialog_screens.sql :28656 are the bark rows 10741/10901, dialog_set_maps.sql :9355 is Q1 map 1000001): `dialog_set_maps.sql`, `dialogs.sql` and `dialog_screens.sql` as above; no `dialog_sets.sql` row (set 512 exists, REPO `dialog_sets.sql:2119`). The override goes in `overrides_castle.rs`, so `dialog_overrides/mod.rs` gets no added line.

Build allowlist delta (coordinator decision): the CS-08b1 brief in r4-dispatch-plan.md lists only `dialog_set_maps.sql` among the Dialogs files and no Rust. To build this seal it must add `db/resources/Dialogs/Seed/dialogs.sql`, `db/resources/Dialogs/Seed/dialog_screens.sql` and `crates/resources/src/base/dialog_overrides/overrides_castle.rs`, and its lane must add `test(dialog_overrides)` and `test(cooked_sync)` (G14), since the override registry feeds the cooked-dialog sync.

## Frozen (d) Chains

New file `db/resources/Content/Seed/castle_567_chains.sql`, registered in `db/database.sql` with `\ir resources/Content/Seed/castle_567_chains.sql` directly after the `castle_hub_barks_chains.sql` line (:401 on `2fad73404`), as the CS-08b1 allowlist says. One statement per line (interact_tag linter), Q1 style. Scope `mission`, scope id 567. All CIMMERIA_AUTHORING.

Ordering rules applied: conditions freeze at resolve time (sibling chains on a key must be pairwise disjoint), and `accept_mission` / `advance_step` run nested dispatch mid action list, so binds are removed before them.

```sql
-- Castle 567 Romney's Files (CS-08bS): chains 1394-1397, 1400, 10240-10243. CIMMERIA_AUTHORING.
-- 1394: 703 completed in world 8 -> offer Copplemann (slot 48)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1394, '567 - 703 completed: offer Romney''s Files on Copplemann (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1394, 'mission_completed', '703', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1394, 'world', 8, NULL, 'eq', NULL, 0), (1394, 'mission_status', 701, NULL, 'eq', 'completed', 1), (1394, 'mission_status', 703, NULL, 'eq', 'completed', 2), (1394, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1394, 'add_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);

-- 1395: Castle login, offer open -> rebind
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1395, '567 - Castle login, offer open: rebind Copplemann (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1395, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1395, 'world', 8, NULL, 'eq', NULL, 0), (1395, 'mission_status', 701, NULL, 'eq', 'completed', 1), (1395, 'mission_status', 703, NULL, 'eq', 'completed', 2), (1395, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1395, 'add_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);

-- 1396: dialog 2043 Accept -> unbind, accept
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1396, '567 - Dialog 2043 accept: unbind offer, accept 567 (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1396, 'dialog_choice', '2043', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1396, 'world', 8, NULL, 'eq', NULL, 0), (1396, 'mission_status', 701, NULL, 'eq', 'completed', 1), (1396, 'mission_status', 703, NULL, 'eq', 'completed', 2), (1396, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1396, 'remove_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0), (1396, 'accept_mission', 567, NULL, '{}', 0, 1);

-- 1397: cabinet click at step 2000 -> grant 2698, advance to 2012, feedback
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1397, '567 - Castle file cabinet at 2000: take Romney''s Files, advance to 2012 (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1397, 'interact_tag', 'Castle_FileCabinet', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1397, 'world', 8, NULL, 'eq', NULL, 0), (1397, 'mission_status', 567, NULL, 'eq', 'active', 1), (1397, 'step_status', 567, '2000', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1397, 'add_item', 2698, NULL, '{"qty": 1, "container": 2}', 0, 0), (1397, 'advance_step', 567, '2012', '{}', 0, 1), (1397, 'display_dialog', 10733, NULL, '{}', 0, 2);

-- 1400: cabinet click at any other state -> visible feedback (disjoint with 1397 by the step-2000 condition)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (1400, '567 - Castle file cabinet, nothing to take: feedback (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (1400, 'interact_tag', 'Castle_FileCabinet', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (1400, 'world', 8, NULL, 'eq', NULL, 0), (1400, 'step_status', 567, '2000', 'neq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (1400, 'display_dialog', 10734, NULL, '{}', 0, 0);

-- 10240: leaving world 8 with the offer unaccepted -> unbind (never live in world 68)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10240, '567 - Leave Castle, offer unaccepted: unbind Copplemann offer (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10240, 'stargate_crossed', NULL, 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10240, 'world', 8, NULL, 'eq', NULL, 0), (10240, 'mission_status', 567, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10240, 'remove_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);

-- 10241: Castle -> Harset crossing at 2012 -> 4039
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10241, '567 - Castle exit route: advance Romney''s Files to 4039 (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10241, 'stargate_crossed', 'Harset', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10241, 'world', 8, NULL, 'eq', NULL, 0), (10241, 'mission_status', 567, NULL, 'eq', 'active', 1), (10241, 'step_status', 567, '2012', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10241, 'advance_step', 567, '4039', '{}', 0, 0);

-- 10242: abandon in any world -> clear turn-in bind and the item
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10242, '567 - Abandoned: clear the turn-in bind and Romney''s Files (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10242, 'mission_abandoned', '567', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10242, 'mission_status', 567, NULL, 'eq', 'not_active', 0);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10242, 'remove_dialog_set', 2817, NULL, '{"slot": 48}', 0, 0), (10242, 'remove_item', 2698, NULL, '{"qty": 1}', 0, 1);

-- 10243: abandon in world 8 -> reopen the offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10243, '567 - Abandoned in Castle: rebind Copplemann offer (CIMMERIA_AUTHORING)', 'mission', 567, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10243, 'mission_abandoned', '567', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10243, 'world', 8, NULL, 'eq', NULL, 0), (10243, 'mission_status', 701, NULL, 'eq', 'completed', 1), (10243, 'mission_status', 703, NULL, 'eq', 'completed', 2), (10243, 'mission_status', 567, NULL, 'eq', 'not_active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10243, 'add_dialog_set', 1000018, NULL, '{"slot": 48}', 0, 0);
```

Disjointness proof per shared trigger key:

| Key | Chains | Why disjoint |
|---|---|---|
| `mission_completed 703` | 1394 plus the existing 703 chains | 1394 only adds a bind on a disjoint map; no existing chain reads or edits map 1000018 |
| `dialog_choice 2043` | 1396 | only chain on 2043 (REPO grep: no other chain keys 2043) |
| `interact_tag Castle_FileCabinet` | 1397, 1400 | step 2000 `eq active` vs `neq active` |
| `stargate_crossed NULL` | 10240 and the Harset-keyed chains | 10240 needs 567 `not_active`; 10241 needs 567 `active`; 1366 and 1360 are other missions |
| `stargate_crossed Harset` | 10241, 1366, 1360 | 567 step 2012 active vs 1360 steps 4037; all additive and independent |
| `mission_abandoned 567` | 10242, 10243 | 10243 only adds a bind; 10242 only removes 2817 and the item. Additive, no shared state |

Order notes: in 1397 the item is granted first because `add_item` is fire-and-forget; the dialog runs last. In 1396 the unbind runs before `accept_mission` because accept dispatches `mission_accepted` nested. Both are the Q1 lessons (CS-04aD).

Relog restore, per step (the only persisted binds are the offer and the Harset turn-in):

| State at login | Restore |
|---|---|
| 567 not_active, 701+703 done, in Castle | 1395 rebinds the offer |
| Step 2000 | none needed: the cabinet is always interactable and 1397/1400 branch on step state |
| Step 2012 | none needed (item 2698 persists in the inventory; 1400 answers the cabinet) |
| Step 4039, in Harset | chain 6504 (`player_loaded Harset_CmdCenter`) binds 2817; owned by the Harset agents, see Harset contract |
| Step 4039, in Castle | intentionally no bind: turn-in dialog 2044 must not be offered in world 8, where chain 6503 (world 68 only) would not complete the mission |

The step-4039 row rests on the crossing happening before destination hydration, the same ordering the 1366 header states (REPO `castle_carry_in_chains.sql:2`). The crossing from Castle lands in Harset surface (world 57, where the stargate is); 6504 fires later, on the `Harset_CmdCenter` (world 68) load, by which time 10241 has long since run and step 4039 is active (REPO `harset_opcore_chains.sql:366-414`, with the H9 residual note on 6503/6504). Nothing here depends on 6504 firing at the moment of crossing.

## Frozen (e) Templates and spawns

`entity_templates.sql` (modelled column for column on template 410; NORMAL_LOOT 4611686018427387904 as 410, no aggro or assist columns, so it is inert):

```sql
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed) VALUES (416, 'EM_Earth_Military.EM-Crate_Wooden01', 'GLB_Components.WorldObject_Small', NULL, 0, 4611686018427387904, NULL, NULL, NULL, NULL, 7547, NULL, NULL, NULL, 'Castle file cabinet', 'spawnable', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL);
```

Placement: the template row goes directly after the template 415 row (`entity_templates.sql:1081` on `2fad73404`, the Castle quartermaster), with a one-line `-- Romney's file cabinet (CS-08bS, CIMMERIA_AUTHORING)` comment above it; the spawn row goes directly after the spawn 13804 row (`spawnlist.sql:624`), with the same comment. Both are before the trailing `setval` footers, which already take `GREATEST(MAX(id), ...)` (`entity_templates.sql:1297`), so 416 and 13805 need no footer edit.

`spawnlist.sql` (the 283 shape, no respawn):

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13805, 232.0, 66.79, 1036.0, 1.57079637, 8, 416, 'Castle_FileCabinet', NULL);
```

Position rationale: Romney's office, 12.0 u west of Romney (spawn 240 at 244.0, 66.79, 1036.0, heading 1.5708 facing east), at the same floor height.

nav_inspect (CD-9), run through the lane (`pwsh tools/build-lane/lane.ps1 cargo run -q -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/castle.nav --probes <file> --quiet`). Exit 3 is the tool's informative "probes span 2 components" code, not a failure; the arrival probe is the intentional second component. `Arrival_116` is the stargate arrival point (761.677, 63.466, 551.716), which the plan puts on component 116; it is not spawn 116 (a NidGuar at 294.159, 55.392, 894.443, which lies on component 250). Probe file: `Romney_240 244.0 66.79 1036.0`, `Copplemann_87 352.692 70.272 952.32`, `Arrival_116 761.677 63.466 551.716`, `Cab_D 230.0 66.79 1034.0`, `Cab_E 232.0 66.79 1036.0`, `Cab_F`, `IntBlock1_255 248.0 67.18 1032.5`, `IntBlock2_256 249.0 67.18 1039.5`, `Zuritska_cell_238 268.0 66.79 1042.59`. R4a1a rerun on a reduced set (Romney, arrival, Cab_E): Romney 250, arrival 116, Cab_E 250, exit 3.

```
  file        data/spaces/castle.nav
  header      nverts=40068 npolys=19815 nvp=6 cs=0.3 ch=0.2 border=0
  agent       height=1.8 climb=0.6 radius=0.6
  bounds      x[-15.66, 1200.00]  y[-102.22, 206.39]  z[0.00, 1200.00]
  components  549 (total walkable XZ area 814506.8 m^2)
  probes (h-tol 2.00 m, v-tol 3.00 m)
    Romney_240           poly=16782  component=250  h=0.00 m  dy=-0.39 m  ok
    Copplemann_87        poly=15521  component=250  h=0.00 m  dy=-0.11 m  ok
    Arrival_116          poly=9348   component=116  h=0.00 m  dy=-0.04 m  ok
    Cab_D                poly=16881  component=250  h=0.00 m  dy=-0.39 m  ok
    Cab_E                poly=16883  component=250  h=0.00 m  dy=-0.39 m  ok
    Cab_F                poly=16883  component=250  h=0.00 m  dy=-0.39 m  ok
    IntBlock1_255        poly=16804  component=250  h=0.00 m  dy=-0.00 m  ok
    IntBlock2_256        poly=16804  component=250  h=0.00 m  dy=-0.00 m  ok
    Zuritska_cell_238    poly=17124  component=250  h=0.00 m  dy=-0.31 m  ok
  nav_inspect: probes span 2 components: [("Romney_240", 250), ("Copplemann_87", 250), ("Arrival_116", 116), ("Cab_D", 250), ("Cab_E", 250), ("Cab_F", 250), ("IntBlock1_255", 250), ("IntBlock2_256", 250), ("Zuritska_cell_238", 250)]
```

The chosen spot is Cab_E = (232.0, 66.79, 1036.0), on poly 16883, component 250.

Component answer: the cabinet **shares component 250 with Romney's office actors and with Copplemann (spawn 87)**, and does **not** share a component with the stargate arrival point (761.677, 63.466, 551.716; component 116). The brief's test is "shares a component with the arrival or with Romney's office actors": it shares with Romney's office actors (spawns 240, 200, 203, 255, 256, 238) and with the giver, Copplemann. A player who can accept the offer from Copplemann (component 250) is already on the component that holds the cabinet. How the player gets from component 116 to 250 is the same unproven Castle route as for all 701-706 content (CD-9: the path leg is an end-UAT row, 567-U4).

Aggro clearance for the cabinet (XZ distance from 232.0, 1036.0, all world-8 spawns within 45 u, from `spawnlist.sql`). A faction-10 template with a NULL `aggro_radius` uses the default 18 u (`DEFAULT_AGGRO_RADIUS`, `crates/cell-world/.../combat/aggression.rs:24`, assist default 10, vertical band 4); players react as faction 3 and `reaction(3,10)` is hostile. Templates 181 and 182 set aggro 15 / assist 12 explicitly. Templates 168 and 176 are faction 1 (friendly):

| Spawn | Template | Tag | Faction | Distance (u) | Effective aggro / assist | Result |
|---|---|---|---|---|---|---|
| 240 | 169 | Castle_Romney | 10 | 12.0 | 18 (default) / 10 | **inside the ring by 6.0 u** |
| 200 | 176 | Castle_Pop_Prisoner1 | 1 | 12.2 | friendly | clear |
| 203 | 176 | Castle_Pop_Prisoner4 | 1 | 12.6 | friendly | clear |
| 255 | 182 | Castle_Pop_IntBlock_1 | 10 | 16.4 | 15 / 12 | 1.4 u outside, clear |
| 256 | 181 | Castle_Pop_IntBlock_2 | 10 | 17.4 | 15 / 12 | 2.4 u outside, clear |
| 238 | 168 | Castle_Zuritska_Cell | 1 | 36.6 | friendly | clear |

The cabinet is not clear of every aggro ring. Romney (hostile, default radius 18) is 12.0 u away, so a player at the cabinet is 6.0 u inside his ring. He respawns every 120 s (`respawn_secs` 120, `spawnlist.sql:499`), so after 703 completes he is back within two minutes. IntBlock_1 and IntBlock_2 are 5.3 u and 6.1 u from Romney, inside their 12 u assist radius, so Romney's pull is expected to bring both. Against the IntBlocks alone the margins are small (1.4 and 2.4 u), and a player on the east side of the cabinet is closer to IntBlock_1 than the cabinet is.

OPEN-5: 567-U4 records Romney's pull and the IntBlock assist. Moving the cabinet does not remove it on its own: the office is entirely inside Romney's 18 u ring, and a move west (for example Cab_D, 230.0, 1034.0, same component 250, 14.1 u from Romney) gains only 2.1 u. Whether to relocate the cabinet or accept the pull (the player has just killed Romney for 703 and the office is already contested content) is a design call left to the coordinator; the seal keeps Cab_E.

## Frozen (f) Reward

Zero. REPO `missions.sql:1457` has `reward_naq` 0 and `reward_xp` 0; no chain here grants XP, cash or items other than the quest item 2698. XP/cash at turn-in (6503's `GC3` note) stays with the Harset agents and is not a Castle concern.

## Behavior

| # | Event | Chain | Result |
|---|---|---|---|
| 1 | 703 completes in world 8 (701 done) | 1394 | Copplemann gets the offer map 1000018 on slot 48 (flag 134217728) |
| 2 | Castle login, offer open | 1395 | same bind restored |
| 3 | Player talks to Copplemann, dialog 2043, presses Accept | 1396 | bind removed, 567 accepted, step 2000 active |
| 4 | Click cabinet at step 2000 | 1397 | item 2698 x1 into container 2, step 2012 active, monologue 10733 |
| 5 | Click cabinet again (or with no mission, or after) | 1400 | monologue 10734 (visible feedback every click) |
| 6 | Cross to Harset at step 2012 | 10241 | step 4039 active; Harset turn-in is 6503/6504 |
| 7 | Cross anywhere with the offer unaccepted | 10240 | offer unbound; never live in world 68 |
| 8 | Abandon in any world | 10242 | turn-in bind and item 2698 removed |
| 9 | Abandon in Castle | 10243 | offer reopened |

Edge cases:

- Second click on the same event: 1397 and 1400 are resolved from the same pre-action snapshot, so 1400 does not fire on the click that grants (it needs 2000 not active at resolve time).
- A player who drops or loses 2698 (the item is not deletable and not sellable, REPO) cannot re-take it: 1400 answers with "nothing else of use". OPEN-6: abandon and re-accept is the recovery path, which 10242/10243 support.
- Slot 48 in Castle was only used during 701 (REPO `castle_701_chains.sql:48-52,412,516`), so the post-703 offer is separate in time from those binds.
- Dialog 2043 closes with a single Accept; there is no More Info path to discriminate.
- The range gate runs before `fire_interact_tag` (REPO `cell-methods/.../interaction/interact.rs:108-125`); the chain layer itself has no distance check. Corrected by agent 3 in ## Authority.

## Test list

Revert proof means: undo the named line and the named test fails.

### s

- **s1 `castle_567_ids_are_free_on_every_ref`**: the 5-ref collision script returns zero hits for chains 1394-1397/1400/10240-10243 (content_chains), map 1000018, dialogs 10733/10734, screens 10865/10866, and for `entity_templates` 416 / `spawnlist` 13805. Revert proof: change 13805 to 13804 and it reports a hit.
- **s2 `castle_cabinet_is_on_romney_component`**: nav_inspect puts the cabinet and Romney (spawn 240) in the same component, not arrival 116, exit 3 with exactly 2 components. Revert proof: move the cabinet to the arrival point and the components differ from Romney's.
- **s3 `castle_cabinet_clears_aggro`**: for every hostile spawn within 45 u of (232, 1036), distance is at least the effective aggro radius, where a NULL `aggro_radius` reads as the default 18 (`DEFAULT_AGGRO_RADIUS`, `cell-world/.../combat/aggression.rs:24`) and the faction-10 templates 169, 181, 182 are hostile to players. The shipped position fails this for Castle_Romney (spawn 240: 12.0 u against 18), so the test names Romney as the single documented exception (OPEN-5) and asserts the IntBlock rows (15 u, 16.4 and 17.4 u away) are clear. Revert proof: move the cabinet to (240, 1036), 8.7 u from IntBlock_1, and the IntBlock assertion fails.

### d

- **d1 `castle_567_monologues_are_registered`**: the Rust override registry contains 10733 and 10734, each one screen, speaker 0, no buttons, and the text equals the seed row. Revert proof: edit one character in the Rust text and it fails.
- **d2 `every_cooked_override_element_id_fits_in_16_bits`** (existing) still passes with 10865/10866 in the registry. Revert proof: raise a screen id to 70000.
- **d3 `castle_567_offer_map_binds_2043`**: `dialog_set_maps` 1000018 has dialog 2043, flags 134217728 and set 512, and 2817 is unchanged. Revert proof: set the map's dialog to NULL and the test fails.

### c1

- **c1a `castle_567_chains_load_and_have_triggers`**: all of 1394-1397, 1400, 10240-10243 load enabled, scope 567, and each has the expected trigger key (NULL for 10240). Revert proof: delete one trigger insert.
- **c1b `interact_tag_linter` and database.sql registration** (existing `crates/content-engine/tests/it/interact_tag_linter.rs`): chains 1397/1400 keep one statement per line, so `scan_chains` (lines 59-62) never scans them and `Castle_FileCabinet` needs no allowlist entry (Q1 chain 10003 is the precedent; entries are only for multi-line `VALUES`). `castle_567_chains.sql` is registered in `db/database.sql` (agents 2/3 do it). Revert proof: drop the registration line and `every_chain_seed_file_is_registered_in_database_sql` fails.
- **c1c `castle_567_cabinet_chains_are_disjoint`**: for every (step 2000 active, inactive, absent) state exactly one of 1397/1400 resolves. Revert proof: change 1400's `neq` to `eq` and both fire on the grant click.

### c2

- **c2a `castle_567_offer_accept_flow`**: 703 completed -> 1394 binds 1000018 on slot 48; `dialog_choice 2043` -> bind removed then 567 accepted, step 2000 active. Revert proof: swap the 1396 action order and the nested accept sees the bind still present.
- **c2b `castle_567_cabinet_grants_then_advances`**: click at 2000 -> `GrantItem 2698 x1 container 2`, `AdvanceStep 2012`, `DisplayDialog 10733`, in that order; second click -> `DisplayDialog 10734` only. Revert proof: delete the `add_item` row from 1397.
- **c2c `castle_567_crossing_advances_to_4039`**: `stargate_crossed Harset` from world 8 at step 2012 -> step 4039; from world 68 or at step 2000 -> nothing. Revert proof: remove the world-8 condition on 10241.
- **c2d `castle_567_offer_unbinds_on_leaving_and_abandon`**: crossing with 567 not_active -> 10240 unbinds; abandon -> 10242 removes 2817 and the item, 10243 rebinds the offer in world 8 only. Revert proof: delete the 567 `not_active` condition on 10240 and an active-567 crossing wrongly unbinds.
- **c2e `castle_567_relog_restore`**: login at not_active with prerequisites -> 1395 rebinds; no 1395 at step 2000. Revert proof: drop 703 from 1395's conditions.

Regression filters to run (all must stay green): `chain_replay_tests::mission_567` (`live_db_chains_6503_and_6504_are_disabled_pending_the_2698_grant` and the sibling live-DB tests keep asserting 6503/6504 disabled; this seal does not touch them), `interact_tag_linter`, `every_chain_seed_file_is_registered_in_database_sql` (the `database.sql` seed-registration test), `dialog_overrides` (`every_cooked_override_element_id_fits_in_16_bits`), the Castle chain replay modules `mission_701`, `mission_702`, `mission_703`, `mission_706`, `mission_708` and `castle_702_704_executor`, `mission_1360` (shares the `stargate_crossed Harset` key with 1366), and the Castle Q1 tests (`castle_side_medical_q1`).

## End-UAT rows

All `UATPending (end UAT, D-CA11)`.

- **567-U1**: finish 701 and 703. Copplemann shows the offer marker; open dialog 2043; press Accept. Expect 567 in the log at step 2000 (proves OPEN-4 `step_enabled false` is harmless). Visible feedback on the first press of Accept: the mission log gains 567 and the offer marker on Copplemann clears.
- **567-U2**: close 2043 with the X instead of Accept. Expect no accept (OPEN-1). Also record the blank speaker label (cosmetic).
- **567-U3**: relog with the offer open; the marker returns (1395). Cross to Harset with the offer open; Harset Copplemann shows no offer (10240).
- **567-U4**: walk to the cabinet in Romney's office. Expect it is visible, targetable and reachable on the mesh. Record Romney's pull (OPEN-5): he is faction 10 with the default 18 u aggro radius, the cabinet is 12.0 u from him, so he is expected to engage a player who stands at the cabinet, and IntBlock_1/2 (5.3 u and 6.1 u from Romney, assist radius 12) are expected to join. Record whether IntBlock_1/2 pull on their own from the east side (OPEN-5, mesh note OPEN-2).
- **567-U5**: click it at step 2000: item 2698 appears in the bag, monologue 10733 shows, step 2012 is shown. Click again: monologue 10734 shows on the first press.
- **567-U6**: cross to Harset at 2012; the log shows 4039; item still in the bag. Turn-in is 6503/6504 once enabled (Harset agents' row).
- **567-U7**: abandon at 2012 and at 4039. Expect the item removed (OPEN-3), no turn-in marker on Harset Copplemann, offer returned in Castle (10243).
- **567-U8**: re-accept and complete the loop once more, to prove 1397 grants again after an abandon.

## Harset contract

State this leg hands to chains 6503/6504 (cross to world 57, then the Command Center load into world 68; Harset owns receipt):

- Mission 567 `active`, step 2000 and 2012 completed, step 4039 `active`, written by 10241 before destination hydration.
- Item 2698 x1 in container 2 (stack limit 1), or, only if container 2 was full at the grant, x1 in the player's mailbox (see ## Items; the 6503 `remove_item` then finds nothing and the mission still completes).
- The offer map 1000018 is **not** bound in world 68 (10240 / 1396 removed it). Slot 48 in world 68 carries only what Harset binds.
- Map 2817 (set 512, dialog 2044) is **not** bound by Castle. 6504 (`player_loaded Harset_CmdCenter`, `{"slot": 48, "mission_id": 567}`, REPO `harset_opcore_chains.sql`) is the only binder, so the Harset agents must flip 6503 and 6504 to `enabled true` and update the two live-DB tests in `mission_567.rs:71-232` that currently assert them disabled.
- Chain 6503 (`interact_tag CmdCenter_Copplemann`, world 68, step 4039 active) removes 2698 qty 1, removes the 2817 bind and completes 567; this note changes none of it.
- Abandon at 4039 in world 68: 10242 clears the 2817 bind and the item with no world condition, so a stale "turn in" topic does not remain.
- Not done here and left to the Harset agents: XP/cash at completion (zero today), and any change to 1360/1366 (separate mission, same crossing key, additive).

Hand-offs for agents 2 and 3: `add_item` is fire-and-forget (the grant is not confirmed before 1397 advances), item 2698 has stack limit 1, there is no More Info/Accept discrimination (not needed with 2043), the `db/database.sql` registration of `castle_567_chains.sql` is theirs (the `interact_tag` linter needs no allowlist entry while every `INSERT ... VALUES` stays on one line, because `scan_chains` skips lines that do not start with `(` or `VALUES`, `interact_tag_linter.rs:59-62`; Q1 chain 10003 has none), the cabinet click is range-gated upstream of the tag chain (REPO `interact.rs:108-125`; corrected by agent 3), and the cabinet shares NORMAL_LOOT interaction flags with the supply chest, so the loot window path must not open for it (the chain, not a loot table, drives the click).

## Items

Agent 2 of 3 (items leg). Labels: REPO (file:line), CIMMERIA_AUTHORING, OPEN. All REPO paths are under `crates/` unless stated. Classification: content-engine grant and removal of a quest item (not UseInventoryItem, not loot, not vendor).

### Hazard scan

- **Double-consume (UseInventoryItem + `remove_item`): not triggered.** 2698 has no `items_event_sets` row and no `content_triggers (item_use, 2698)` chain, so no `fire_item_use` path exists. Chain 6503 runs from `interact_tag`, so its `remove_item` carries no `instance_id` in params and takes the by-type path (REPO `cell-content/src/cell/content/executor/inventory.rs:118-122,156-183`). One consume path only.
- **Bandolier / propId / equip ordering: not applicable.** 2698 is `container_sets {2}` with no weapon or ammo columns (REPO `db/resources/Items/Seed/items.sql:11707`). Grant precedes advance in 1397 (`add_item` sort 0, `advance_step` sort 1), the same order as the equip-from-inventory pattern.
- **Stack invariants:** `max_stack_size` 1 means the merge fast path never matches (`base-methods/.../inventory/grant/persist.rs:178-190`, `ri.max_stack_size > 1`), so every grant is a new row.

### Action names

The brief's `give_item` is the chain-text name `add_item` (loader REPO `content-engine/src/loader/action.rs:44-48`, params `qty`, `container`), which becomes `Action::GrantItem`. The frozen SQL already uses `add_item`; no correction.

### give_item 2698 with a full inventory

Path: 1397 `add_item` -> `executor/inventory.rs::grant` sends `CellToBaseMsg::GrantItem` with `notify_gm false` and `loot None` (`inventory.rs:52-66`) -> `item_grant_dispatch.rs:48-61` routes to `handle_content_grant_item` -> `grant()` -> `resolve_placement` (container 2 requested, `{2}` allows it: `cell-catalog/src/item_placement.rs:69-85`) -> `persist_grant`.

What "full" means: the target is container 2 (INV_Mission), 100 slots (REPO `entity/src/inventory.rs:91`), not the 40-slot main bag. A full main bag does not affect this grant. Container 2 full is practically unreachable but is handled:

| Outcome of `persist_grant` | Player sees | State after 1397 |
|---|---|---|
| Committed | the bag refresh (`send_full_inventory_update`, `grant_item.rs:517-525`), then monologue 10733 | item in 2, step 2012 |
| `ContainerFull` | line "Your bag was full. Your item is in your mailbox." plus a mail notification; 2698 minted into mail (`grant_item.rs:117-155`) | item in mail, step 2012, monologue 10733 still shows |
| `DatabaseError`, `NoDatabase`, `NotGrantable` | nothing from the grant (content grants send no GM feedback: `grant_item.rs:304-309`, `notify_gm false`); WARN `grant_refused` | no item, step 2012, monologue 10733 still shows |
| `CommitUnknown` | WARN `grant_outcome_unknown`; never retried or mailed (`grant_item.rs:108-115`) | item may or may not exist, step 2012 |

**The chain cannot refuse to advance when the grant fails, and this seal does not claim it can.** The cell side is fire-and-forget: `grant()` returns after the channel send (`inventory.rs:56-86`), and `advance_step` runs next in the same action list. Two engine facts rule out a gate: (a) `Condition::HasItem` reads `ctx.params["item_<id>_count"]` (REPO `content-engine/src/conditions/mod.rs:258-265`), nothing in `crates/` populates that key and the SQL loader has no `has_item` mapping, so no chain can test inventory; (b) the cell's `InventoryItemGranted` handler only logs (`cell/src/cell/service/base_messages/inventory_events.rs:91-116`) and fires no content event, so no chain can wait for a confirmed grant. Making the advance conditional on a committed grant needs Rust (a grant-confirmed trigger or a populated `item_N_count`), which is outside a docs seal and is a maintainer decision.

Accepted residual (CIMMERIA_AUTHORING, OPEN-7): on a database failure the player is at 2012 without the file. Cabinet clicks then answer 1400 ("nothing else of use"), the turn-in at 6503 would still complete the mission (its `remove_item` finds nothing and logs a WARN, `remove_by_type.rs:137-148`, and `complete_mission` runs regardless). Recovery is abandon (10242 no-ops the removal, 10243 reopens the offer) and re-accept, which is the OPEN-6 path. A mailed 2698 (full container 2) has the same shape for 6503: the mission completes without the item in a carried bag, and the mail remains claimable. Both are low-likelihood and the item has no reward value of its own (zero rewards, frozen (f)); the seal accepts them rather than add engine work. 567-U9 records a log check that the grant row appears.

### Duplicate grant on a repeated click

The grant is gated only by step state, not by item presence. 1397 requires `step_status 567/2000 active`; its `advance_step` updates the cell's tracked step synchronously (`executor/mission.rs:290`, `cell::missions::advance_step`) before the next event is resolved, so a second click resolves with 2000 no longer active: 1397 does not match, 1400 does (step 2000 `neq active`), and the second click shows monologue 10734 and grants nothing. The only re-arm of 2000 is abandon plus re-accept.

Where a second 2698 row can still appear (all need the first row to survive past a re-accept, none is reachable by clicking alone):

1. **Abandon removal fails or is skipped.** 10242 removes one row by type, first by `(container_id, slot_id)` (`remove_by_type.rs:111-120`). If that removal fails (a send failure or a DB error, both logged), re-accept plus a cabinet click grants again and the player holds two rows. `max_stack_size` 1 means no merge and no dedupe, and the cell has no unique-item guard (`flags` 32768 is `ELEMENTARY_COMPONENT`, which the code documents as set on every seeded item and distinguishing nothing: `cell-catalog/src/crafting/constants.rs:99-100`). Consequence: 6503 removes one row at turn-in and one stray 2698 stays in container 2. Harmless to the mission, a cosmetic leftover. OPEN-8: accepted.
2. **Mail plus a later grant.** If the first grant was mailed and the player abandons, 10242 finds no carried row and the mailed copy stays claimable; a re-accept then grants a second. Same consequence as 1.
3. **Concurrent clicks.** If two cabinet events are queued before the first runs, they resolve one after the other in the cell loop, the second after step 2012 is recorded, so it matches 1400. Whether the cell loop can interleave two events of one player mid-action-list is agent 3's check (## Authority); this section assumes it cannot.

Order of consume versus grant across a quick abandon and re-accept is safe: removal and grant both travel the single cell-to-base channel in send order, so the removal commits before the next grant.

### `remove_item 2698` in 6503 matches the grant

6503 action 1: `remove_item 2698 {"qty": 1}` (REPO `db/resources/Content/Seed/harset_opcore_chains.sql:384`). Matching, field by field:

| Field | Grant (1397) | Removal (6503, 10242) | Match |
|---|---|---|---|
| Design id | `add_item` target 2698 | `remove_item` target 2698 | yes |
| Quantity | `qty 1`, stack limit 1, so one row of stack 1 | `qty 1`; `count >= stack_size` deletes the row (`remove_by_type.rs:170-180`) | yes |
| Container | 2 | not specified: by-type search over all player-reachable containers ordered by `(container_id, slot_id)`, never buyback, vault only with a session, and a by-type removal never reaches the vault (`remove_by_type.rs:108-120`, `inventory.rs:172-182`) | yes: container 2 is reachable and 2698 is `{2}`-only, so it cannot be anywhere else but 2 (or the mailbox, which is not searched) |
| Path | content grant | by-type (no `instance_id`, chain fired by `interact_tag`) | yes |
| Order in 6503 | n/a | `display_dialog` (0), `remove_item` (1), `remove_dialog_set` (2), `complete_mission` (3) | the removal is fire-and-forget like the grant; completion does not wait on it |

6503 does not need a `container` param and none should be added: `RemoveItem` has no container field (`executor/inventory.rs:93-185`).

### Corrections to the frozen rows

1. **567 IDs, Item row:** the earlier "not deletable" was stated as a server fact. It comes from the cooked client record (evidence `g-567-706-1376.md:24`); the server seed row carries no deletable or sellable column and `flags` 32768 binds nothing. Row amended above. Consequence: the server does not stop the player from moving the item within carried bags, and whether the client UI offers a destroy action on it is OPEN-9 (567-U9). The Behavior edge case "a player who drops or loses 2698 cannot re-take it" stays true (1400 answers) and is the OPEN-6 recovery case.
2. **Harset contract, item line:** amended above to name the mailbox case.
3. No change to chains 1394-1397, 1400, 10240-10243, to ids, or to the frozen SQL. The grant order (item first, step second, dialog last) is correct and stays.

### Tests this leg adds to the list

- **i1 `castle_567_grant_params_match_removal`** (unit, chain replay): 1397's `GrantItem` is `item_id 2698, qty 1, container Some(2)` and 6503's `RemoveItem` is `item_id 2698, qty 1`, same design id and quantity. Revert proof: change either qty to 2, or the item id in 1397, and the test fails.
- **i2 `castle_567_second_click_grants_nothing`** (chain replay, extends c2b): after the first click mutates tracked state to 2012, a second click emits no `GrantItem`. Revert proof: drop the `step_status 2000 active` condition from 1397 and the second click grants again.
- **i3 `live_db_castle_567_item_2698_grants_into_mission_bag`** (live DB, base): `resolve_placement` for 2698 requesting 2 returns container 2, and a second grant creates a second row (no merge), documenting that dedupe is by step state only. Revert proof: set `max_stack_size` 5 in the test fixture and the merge path appears, failing the assertion.
- **i4 `live_db_castle_567_remove_by_type_finds_container_2`** (live DB, base): with a 2698 row in container 2, `RemoveInventoryItemByType {2698, 1}` deletes it. Revert proof: restrict the `ANY($3)` list to exclude 2 and the row survives.
- **i5 `live_db_castle_567_full_container_2_mails_the_file`** (live DB, base; extends `full_bag_tests`): with container 2 filled to 100, a content grant of 2698 commits no carried row and mints one mail. Revert proof: remove the `ContainerFull` branch in `handle_content_grant_item` and no mail exists.

Regression filters: `chain_replay_tests::mission_567`, `grant::full_bag_tests`, `grant::fall_through_tests`, `core::remove_by_type` tests.

### End-UAT rows added

- **567-U9**: at step 2000 click the cabinet and read the base log for `grant_container_chosen` (item 2698, container 2, qty_after 1). Open the bag UI and note whether 2698 appears under the mission tab and whether a destroy or drop action is offered (OPEN-9). `UATPending (end UAT, D-CA11)`.
- **567-U10**: abandon at 2012, re-accept, click the cabinet again. Expect exactly one 2698 afterwards (OPEN-8 stray-row check) and the log pair `RemoveInventoryItemByType: persisted` then `grant_container_chosen`. `UATPending (end UAT, D-CA11)`.

### Hand-offs

- Agent 3 (server-authority-enforcer): the cell-side serialisation of two cabinet events from one player, and whether `interact_tag` has any server-side range gate; neither is settled here.
- Maintainer decision (not a docs seal): a grant-confirmed content trigger or a populated `item_N_count` would let a chain gate `advance_step` on a committed grant (OPEN-7). Not needed for the seal; recorded so it is not rediscovered.
- rust-gameserver-dev: i3 to i5 land in `base-methods` if the maintainer wants them in the build packet.

## Authority

Agent 3 of 3 (server-authority-enforcer). Labels: REPO (file:line), CIMMERIA_AUTHORING, OPEN. Review only: no code changed. Paths are under `crates/` unless stated. Question asked of every input: what does the client assert, and what does the server already know that contradicts a lie.

### Corrections to agents 1 and 2

1. Agent 1's "no distance check on `interact_tag` chains" is wrong as a statement about the click path. `fire_interact_tag` (REPO `cell-content/src/cell/content/event_dispatch/interaction.rs:24-78`) has no range check, but the only caller path, `handle_interact`, refuses out-of-range targets first (REPO `cell-methods/src/cell/cell_methods/player/interaction/interact.rs:108-125`), before the interaction pin (`interact.rs:163`), trainer dispatch and `fire_interact_tag` (`interact.rs:192`). Behavior edge case and Harset hand-off lines are amended above.
2. Agent 2's concurrency question is answered in the table (row F6).

### What the client can assert, and the gate that refuses it

| # | Forgery | Server gate | Result |
|---|---|---|---|
| F1 | Interact on the cabinet (template 416, tag `Castle_FileCabinet`) from out of range, or from another space | `interact_target_in_range` (REPO `interact.rs:108-125`) over `interact_range` (REPO `cell-world/src/cell/space_manager/interact_range.rs:39-72`): target must exist (46-51), share the player's space (57-59, closes the same-coordinates cross-space pin), and be within `MAX_INTERACT_DISTANCE` 5.0 (13, 65-70). Actor alive and present is checked first (`interact.rs:34-44`). Nothing is pinned and no chain fires on refusal. | Refused. Guard tests already exist: `out_of_range_interact_does_not_pin_or_fire_chains`, `interact_on_a_missing_target_does_not_pin_or_fire_chains`, `in_range_interact_pins_and_fires_chains` (REPO `interaction/mod.rs`). |
| F1b | Interact with a negative or forged target id | Negative id rejected (REPO `interact.rs:56-68`); a missing entity fails `interact_range` at `TargetMissing`. | Refused. |
| F1c | Click the cabinet in a state where 1397 is not valid (not on 567, wrong step, second time) | Chain conditions are evaluated server-side from cell-tracked mission state at resolve time (`event_dispatch/interaction.rs:40-55` populates `populate_mission_context` from the entity, never from the packet). 1397 needs `step_status 567/2000 active`; 1400 answers otherwise. The client sends only the target id. | Refused (no grant, monologue 10734). |
| F2 | Forged `dialogButtonChoice` on dialog 2042 or 2043 without the server having offered it | `handle_dialog_button_choice` takes the id from the per-player offered set before any chain fires (REPO `cell-methods/.../interaction/dialog.rs:65-67`, rejection at 68-102, chain at 119-123). The set is filled only when `onDialogDisplay` is sent (REPO `entity/src/cell_entity/offered_dialogs.rs:8-13,58-73`), is bounded at 8 (50), and `take_offered_dialog` is one-shot (85-97). Dialog 2042 is never displayed by any 567 chain, so it can never enter the set. | Refused. 2042 cannot be offered, so a forged 2042 choice is rejected; a forged 2043 choice is accepted only once, after the server showed it. |
| F2b | Replay of a valid 2043 choice | One-shot take (REPO `offered_dialogs.rs:85-97`; unit test `offer_then_take_is_one_shot`). The accept chain 1396 also requires 567 not already held, so a re-run cannot re-accept. | Refused. |
| F2c | Forged `initialResponse` for the Copplemann slot-48 bind (to get 2043 offered without clicking him) | Server matches the asserted id against its own `available_interactions` set map (REPO `cell-interactions/src/cell/interactions/dispatch/initial_response.rs:38-47`); the display, pin and `fire_dialog_open` follow at 65-131. | Partly open: see OPEN-10. |
| F3 | Replay of the Castle to Harset crossing (to re-fire `stargate_crossed 'Harset'`, chain advancing 2012 to 4039) | Address-book gate `player_knows_stargate` (REPO `cell-interactions/src/cell/gate_travel/mod.rs:163-167`); refusals at 170-243; a normal dial needs a server-held dial that is consumed on crossing, one crossing per dial (335-350; region entry needs a dial and `passable`, 451-519, `cancel_gate_dial` at 508). The destination world is taken from the server-held dial, never from the packet; the origin world comes from `populate_world_context` (REPO `cell-content/src/cell/content/event_dispatch/stargate.rs:49-132`). The advance chain additionally requires step 2012 active, so a replay after the hop matches nothing. | Refused. |
| F4 | Complete or turn in 567 without the cabinet step | No client message completes the mission. 6503 (Harset, `interact_tag CmdCenter_Copplemann`, step 4039 active) is range-gated like F1 and step-gated; 4039 is reachable only from 2012 by chain, 2012 only from 2000 by chain 1397. | Refused. |
| F5 | Spoof a GM or give-item path to mint 2698 | Not in this leg: no 567 chain consumes a client privilege bit. The systemic GM access-level plumbing gap is tracked in agent memory and is outside CS-08bS. | Not applicable here. |
| F6 | Two cabinet clicks queued back to back (race for a double grant) | Cell events are dispatched with `&mut SpaceManager` per call, so they are serialised. `advance_step` updates the cell-tracked step synchronously (REPO `cell-content/src/cell/content/executor/mission.rs:264-291`) before the next event is resolved, so the second click resolves with 2000 not active and matches 1400. | Refused structurally. I did not confirm that no `.await` yields inside an action list lets a second event of the same player interleave; see OPEN-11. |

### Confirmation: 2698 is granted once, 2012 is not reached without the item

- **Grants:** the only chain that emits `add_item 2698` is 1397 (frozen). It is gated by `step_status 567/2000 active` and is the same chain that moves the step, so one accepted mission yields one grant. Re-arming 2000 needs abandon (10242) plus re-accept, which removes the item first. No other path emits it in this leg: no vendor, loot table or `items_event_sets` row exists for 2698 (agent 2 hazard scan), and 6503 only removes.
- **Advance:** the only path to 2012 is the `advance_step` action in 1397, sort order after `add_item`. No client message advances a step.
- **The one gap is a server fault, not a client lie (OPEN-7, accepted):** `add_item` is fire-and-forget and chains cannot read inventory (`Condition::HasItem` reads an unpopulated param, REPO `content-engine/src/conditions/mod.rs:258-265`), so a base-side DB failure leaves the player at 2012 without the file. A client cannot induce it, and it cannot be repeated for profit (no reward, zero rewards, frozen (f)).
- **Second 2698 row:** only via a failed abandon removal or a mailed copy plus re-accept (OPEN-8, agent 2). Cosmetic, no value.

### Residuals (none block the seal)

- **OPEN-10 (low, REPO):** `initialResponse` has no range check and the `last_interaction_target` pin persists (`initial_response.rs:65-131`). A player who was once in range of Copplemann can later re-open and accept the offer from range. The set-map lookup still requires the bind to be live for that player (slot 48 bound, 567 not held, mission eligible), so it grants nothing a legitimate in-range interaction would not. Fix owner: interaction layer; guard test `initial_response_out_of_range_does_not_open_567_offer` (fails when a range check is removed).
- **OPEN-11 (REPO, unverified):** serialisation relies on the dispatcher's `&mut SpaceManager`; no test pins "two cabinet events from one player grant once". Add `castle_567_two_cabinet_events_grant_once` (chain replay over the real executor with shared state; fails if 1397 loses its `step_status` condition). This is agent 2's i2 run through the executor rather than the engine alone.
- **OPEN-12 (low, REPO):** if the Castle stargate has no usable region, the no-region fallback fires `stargate_crossed` before `perform_gate_travel` (`gate_travel/mod.rs:287-322`, usable-arrival refusal at 575). A failed travel then leaves the step at 4039 while the player is still in Castle. Self-inflicted state only (needs a dial and the address); 6503 is range-gated in world 68, so the player cannot turn in until they actually cross. Not checked: whether Castle ships a `REGION_FLAG_Stargate` region, which decides whether this path is reachable; verified by 567-U6.
- **OPEN-13 (REPO):** the range gate is only as strong as the movement validator's position (the cell trusts the validated position, not the packet). A speed or teleport bypass would let a client click the cabinet from afar; it would still get only the one grant. Out of scope; owned by the movement leg. Mail and trade carrying 2698 were not verified (the item is `container_sets {2}` only; the trade swap whitelist and mail attach source containers should be confirmed when those flows are implemented).

### Tests required

- `castle_567_two_cabinet_events_grant_once` (OPEN-11), fails if 1397 loses `step_status 2000 active`.
- `initial_response_out_of_range_does_not_open_567_offer` (OPEN-10), fails if the range check is removed.
- Existing guards that cover F1, F2 and F2b stay in the regression filter: `out_of_range_interact_does_not_pin_or_fire_chains`, `offer_then_take_is_one_shot`.

Checks: no client-supplied timestamp, quantity or privilege bit is used by any 567 chain; the grant and removal are keyed by design id with the by-type path as agent 2 documented; no `remove_item` plus `UseInventoryItem` double path exists for 2698; trade and mail rollback are not exercised by this leg.