# CS-04aD: medical and prisoner side stories, design seal

> **Type:** Reference. **Audience:** Castle orchestrator, mission-systems, CS-00B and CS-04a reviewers.
> **Status:** partial seal. `medical_q1` is SEALED. `medical_q2` and `medical_q3` are blocked (see Verdict). **Base:** `553d8128d` (CS-00Be evidence in `evidence/mission-catalog.md`).

Scope: part 2 section CS-04aD (`r3-packets-2.md`) as re-dispatched under the ruling `CS-04aD-ruling.md`. Docs only: no cargo, no lane, no seed edit, no psql. Every seed was read with `git show 553d8128d:<path>`. Every premise below is `CIMMERIA_AUTHORING` (O14-10/11, ledger D-CS11: no owner approval needed). Facts quoted from seeds are `SHIPPED_DATA`. Client-only claims are `UATPending (end UAT, D-CA11)`; there is no lab (D-CA11).

## Verdict

- `medical_q1` (*Triage Rounds*): **SEALED**. Implementation gated by the CS-00B client proof and W1 restoration UAT (D-CA8).
- `medical_q2` (the overlooked prisoner): **BlockedEvidence (nav proof)**. Implementation gated by the CS-00B client proof and W1 restoration UAT (D-CA8).
- `medical_q3` (the safe message): **BlockedEvidence (nav proof)**: there is no unique-template friendly in the Infirmary besides Ogilvie (section 3), so it needs a new template and spawn, the same blocker as Q2. The spec asks for two or three quests, so Q3 is optional. Implementation gated by the CS-00B client proof and W1 restoration UAT (D-CA8).

## 1. Collision audit

The catalog section 3 script was saved outside the repository as `...\scratchpad\audit-cs00be.ps1` (extracted verbatim from `evidence/mission-catalog.md`) and run with `-Wt` = this worktree. Output for `-Ref 553d8128d -IncludePaks`; the 16 paks not shown printed `entry names in block 0; attributes in block: none`:

```text
== first-column ids in block, per table (ref 553d8128d)
texts 995
sequences 207
event_sets_sequences 172
event_sets 14
error_texts 10
total first-column rows: 1398
dialog_set_maps max seed id: 1000000
dialog_screens.screen_id in block: 10000 10001 10002 10003 10004 10005 10087 10092 10228 10241 10247
CookedDataDialogs: entry names in block 0; attributes in block: ScreenID=11
CookedDataKismetSeqEvent: entry names in block 187; attributes in block: none
CookedDataKismetSetEvent: entry names in block 14; attributes in block: none
ErrorStrings: entry names in block 10; attributes in block: none
TextStrings: entry names in block 995; attributes in block: none
```

`content/cellblock-campaign`, `content/harset-campaign` and `content/tollana-campaign` each printed exactly this (no `-IncludePaks`):

```text
== first-column ids in block, per table (ref <each of the three>)
texts 995
sequences 207
event_sets_sequences 172
event_sets 14
error_texts 10
total first-column rows: 1398
dialog_set_maps max seed id: 1000000
dialog_screens.screen_id in block: 10000 10001 10002 10003 10004 10005 10087 10092 10228 10241 10247
```

Reading: the only occupied namespaces in 10000-10999 are text ids, error text ids, event-set and sequence ids, and 11 screen ids (10000-10247). Q1 claims none of those: no text row, no event set, no play_sequence, and screen 10800 is above 10247.

Cellblock's individual claims, read from `git show content/cellblock-campaign:docs/analysis/four-zone-planning/id-allocation.md`: mission 62050 with 62051-62056, dialog 60150, screen 96110, dialog 60160, screens 200160-200162 and 200170, chains 11000-11005, 11010-11012 and 11020, and the Cellblock block 11000-11499. Every Q1 id is outside all of them. The same file gives Castle `10000-10999` and Tollana `9000-9999`.

Per-id check. A `git grep -E "VALUES \((10000|...|10006|10300|10340|10341|10460|10461|10580|10581|10700|10760|10800|1000001),"` at `553d8128d` over `db/resources`: the only hits are `event_sets` and `event_sets_sequences` rows (event-set ids 10000-10006, a different namespace, listed as occupied in catalog section 2). No `content_chains`, `missions`, `mission_steps`, `mission_objectives`, `mission_tasks`, `dialogs`, `dialog_sets` or `dialog_set_maps` row has any of these ids. A search of the local `castle/*`, `harset`, `tollana`, `cellblock` and `four-zone` branches for `1000001`-`1000009` found no claim.

## 2. Q1 `medical_q1`: *Triage Rounds*

### Premise (`CIMMERIA_AUTHORING`)

The Castle Infirmary is overrun with wounded after the opening battle. The Castle medic asks the player to walk out to the front courtyard and see who still needs help. The player is told once, in a one-screen narrator dialog, then goes there; arriving completes the mission. It is the smallest surface that exercises every part of the client proof: a journal entry, one dialog, a step advance, a region completion and a relog.

Facts it relies on (`SHIPPED_DATA`, base `553d8128d`):

- Spawn 198 (`spawnlist.sql`): `Castle_Pop_Infirmary_Medic1`, template 177, world 8, `(352.0, 70.18, 885.0)`, heading 1.570796. It is the only spawn of template 177 (a search for `, 177,` in the spawn rows finds spawn 198 only), so no other NPC is touched by a template-keyed binding. The tag `Castle_Pop_Infirmary_Medic1` appears in no chain, no dialog and no mission seed.
- Template 177 (`entity_templates.sql`): `template_name` `Castle_Medic`, body set `BS_HumanFemale`, level 4, faction 1 (friendly, the same faction as the Castle's other friendly NPCs), `event_set_id` 570, `has_dynamic_properties` true. The last is what lets a per-player interaction update reach the client. No trainer list, no vendor lists.
- Region `Castle.CastleFrontCourtyard` (`point_sets.sql` 2050, `AreaSet`, `BoundingBox`, world 8) with points `(468.94, 28.18, 674.42)`, `(468.94, 28.18, 631.82)`, `(509.87, 28.18, 631.82)` and `(510.25, 36.25, 674.42)`. No chain in the base tree keys on it (`git grep "Castle.CastleFrontCourtyard"` over `db/resources/Content` is empty). The region trigger shape is proven by chain 1367 (`enter_region`, `Castle.Infirmary`). The set has exactly four points, the count `is_point_in_region` requires (`crates/cell-catalog/src/cell/spawner/regions.rs:107-110`); a region with any other count never fires.
- Mission 704 is a Castle prerequisite already used as the offer gate by chain 1367 (`castle_1520_chains.sql`).
- The pin: `interact.rs` calls `pin_interaction_target` before any tag dispatch (the code comment cites the python `SGWPlayer.interact()`), and an `interact_target_in_range` gate returns early. So a click is range-checked at this base and the follow-up `display_dialog` resolves a speaker. Q1's dialog is still an all-speaker-0 monologue, the proven shape.

Nav evidence (`SHIPPED_DATA`, section 3 table): the medic spawn (198) and the courtyard centre both resolve onto navmesh component 250 of the tracked `data/spaces/castle.nav`, so the NPC navmesh connects the two; the straight-line distance is about 270 m. That is not a proof of the player route, and the client does not walk the server navmesh.

Not proven here (`UATPending (end UAT, D-CA11)`): that the courtyard is reachable on foot from the Infirmary, that no hostile aggro cluster sits on the route, that the first-press marker on the medic shows, and which icon the client draws for it. The offer bit is `INT_MissionWorldObject` (1073741824), as map 2795 uses for the NPC Prisoner 329 (`docs/content/interaction-flags.md` names it a quest-item glow); the side-quest-available bit (134217728) has no Castle precedent in the seeds.

### IDs (all from the CS-00Be sub-blocks)

| Namespace | Id | Use |
|---|---|---|
| Mission | 10300 | *Triage Rounds* |
| Step | 10340, 10341 | A (speak), B (courtyard), declaration order A then B |
| Objective | 10460, 10461 | one required objective per step |
| Task | 10580, 10581 | type 1, one per objective |
| Dialog | 10700 | one screen, zero buttons |
| Screen | 10800 | sole screen of dialog 10700 |
| Dialog set | 10760 | new `dialog_sets` row |
| Dialog set map | 1000001 | explicit id above the seed max 1000000 (not in the block, per catalog section 2) |
| Chains | 10000-10006 | seven chains |

No text row, no `texts.moniker_id`, no event set, no sequence, no template, no spawn.

### Frozen (a): the exact XML

File `crates/resources/src/base/mission_additions_castle.rs`, `pub const CASTLE_MEDICAL_Q1_XML: &str = r#"..."#;` (the copy contains no `"#`). The XML is one line with no newline. It differs from `TEST_MISSION_XML` only by having no newline after the declaration, which the brief requires and the admission rules allow (declaration only, no DOCTYPE, child order preserved). Registered as `MissionAddition { mission_id: 10300, xml: CASTLE_MEDICAL_Q1_XML }` in `MISSION_ADDITIONS`; CS-00B code must change the `is_empty()` assert at `tests/mission_additions.rs:125` (catalog section 1).

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="3" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Triage Rounds" MissionID="10300"><HistoryText>The Castle Infirmary is overflowing with wounded. The medic wants someone to check the front courtyard.</HistoryText><Steps StepEnabled="false" StepID="10340" AwardXP="false" Difficulty="1"><StepDisplayLogText>Speak with the medic in the Castle Infirmary.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10460" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak with the medic.</DisplayLogText><Tasks TaskType="1" TaskID="10580" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="10341" AwardXP="false" Difficulty="1"><StepDisplayLogText>Check the Castle front courtyard for wounded.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10461" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Reach the Castle front courtyard.</DisplayLogText><Tasks TaskType="1" TaskID="10581" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

- **Byte count: 1,692** UTF-8 bytes, measured with `[IO.File]::ReadAllBytes(<scratch file>).Length` on the exact line above, no BOM, no newline. Limit `MAX_MISSION_XML_BYTES` = 355,840. Margin 354,148. The file's SHA-256 is `37ca8762e8a5ca46ec46037fc7a18b74313155ca86260dd8608c202b95a9b3dc`; the scratch file parses as XML with `MissionID` 10300.
- Admission rules met by construction: ids 10300, 10340, 10341, 10460, 10461, 10580, 10581 pairwise unique; `MissionID` equals the root key; every required attribute present for its node; no DOCTYPE; child order `HistoryText`, `Steps` (`StepDisplayLogText`, `Objectives` (`DisplayLogText`, `Tasks`)); all element ids at most 65535.
- Not a stock key: 10300 is above every stock mission id, and the pak audit above shows no `MissionID` attribute in the block.
- Steps are declared A then B, matching the `advance_step` order in chain 10004 (the client keys steps by declaration order).

### Frozen (b): `db/resources/Missions/Seed/`

`missions.sql`:

```sql
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (10300, 'The Castle Infirmary is overflowing with wounded. The medic wants someone to check the front courtyard.', false, true, false, false, 1, true, true, false, false, false, 3, 'Triage Rounds', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`mission_steps.sql` (columns `step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index`):

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10340, 10300, false, 1, false, 'Speak with the medic in the Castle Infirmary.', 0);
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10341, 10300, false, 1, false, 'Check the Castle front courtyard for wounded.', 1);
```

`mission_objectives.sql` (columns `objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text`):

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10460, 10340, false, 1, false, false, false, 'Speak with the medic.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10461, 10341, false, 1, false, false, false, 'Reach the Castle front courtyard.');
```

`mission_tasks.sql` (columns `task_id, objective_id, award_xp, difficulty, is_enabled, task_type`):

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10580, 10460, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10581, 10461, false, 1, true, 1);
```

Agreement with the XML: same ids, same text, `IsHidden=false` and `IsOptional=false` for both objectives, `step_enabled`/`is_enabled` false for steps and objectives as in the fixture and stock shape. `award_xp=false` everywhere; `can_abandon=true`, `num_repeats=0` (one-time), level 3 as mission 1520. `script_name` and `script_spaces` are seed-only.

**Rewards: none.** `mission_rewards.sql` and `mission_reward_groups.sql` get no row for 10300. `missions.reward_naq = 0`, `reward_xp = 0`.

### Frozen (c): Dialogs seeds and the override

No `texts` rows and no `error_texts` rows. The copy is inline in the XML (journal) and in the screen row (dialog), as in CB-N05.

`dialogs.sql` (`DUIST_DefaultDialog`, flags 0, the 3995 shape):

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10700, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
```

`dialog_screens.sql`:

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10700, 10800, 'The medic barely glances up from her work. "Every cot is full and more wounded keep coming in. Make your way out to the front courtyard and see who still needs help. Watch yourself out there."', 0, 0);
```

No `dialog_screen_buttons` rows (zero buttons). A zero-button speaker-0 dialog is a monologue, so `display_dialog` binds the player and needs no NPC speaker.

`dialog_sets.sql`:

```sql
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10760, '');
```

`dialog_set_maps.sql` (a NULL-dialog interaction-only flag carrier, the 2795 shape: flags 1073741824 = `MISSION_WORLD_OBJECT`):

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000001, 10760, NULL, 'Speak with the Medic', 1073741824, 1, '{}', '{}', '{}', '{}');
```

`crates/resources/src/base/dialog_overrides/mod.rs`, appended at the END of `DIALOG_OVERRIDES` (tests read entry 0 as Frost). The text must equal the seed row byte for byte:

```rust
DialogOverride {
    dialog_id: 10700,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10800,
        speaker_id: 0,
        text: "The medic barely glances up from her work. \"Every cot is full and more wounded keep coming in. Make your way out to the front courtyard and see who still needs help. Watch yourself out there.\"",
        buttons: &[],
    }],
},
```

Both ids are at most 65535 (the 16-bit guard `every_cooked_override_element_id_fits_in_16_bits`). The shape is the shipped 3995/3996 one-screen speaker-0 zero-button entry, not the quarantined debug-hub shape.

### Frozen (d): chains

File `db/resources/Content/Seed/castle_side_stories_chains.sql` (D-CA17). One statement per line (interact_tag_linter). Scope `mission`, scope id 10300. The order of the `add_dialog_set`/`remove_dialog_set` `target_id` is the map id and `slot` is the template id (177), per `executor/dialog/mod.rs`; the binding is per player (`available_interactions`).

```sql
-- Castle Q1 medical (CS-04aD): chains 10000-10006. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10000, '10300 - 704 completed: offer the medic (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10000, 'mission_completed', '704', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10000, 'world', 8, NULL, 'eq', NULL, 0), (10000, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10000, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10000, 'add_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10001, '10300 - Login, offer open: rebind the medic (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10001, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10001, 'world', 8, NULL, 'eq', NULL, 0), (10001, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10001, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10001, 'add_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10002, '10300 - Abandoned: rebind the medic (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10002, 'mission_abandoned', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10002, 'world', 8, NULL, 'eq', NULL, 0), (10002, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10002, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10002, 'add_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10003, '10300 - Medic click: accept (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10003, 'interact_tag', 'Castle_Pop_Infirmary_Medic1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10003, 'world', 8, NULL, 'eq', NULL, 0), (10003, 'mission_status', 704, NULL, 'eq', 'completed', 1), (10003, 'mission_status', 10300, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10003, 'accept_mission', 10300, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10004, '10300 - Accepted: advance to B, unbind medic, show 10700 (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10004, 'mission_accepted', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10004, 'world', 8, NULL, 'eq', NULL, 0), (10004, 'step_status', 10300, '10340', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10004, 'advance_step', 10300, '10341', '{}', 0, 0), (10004, 'remove_dialog_set', 1000001, NULL, '{"slot": 177}', 0, 1), (10004, 'display_dialog', 10700, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10005, '10300 - Castle.CastleFrontCourtyard enter at B: complete (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10005, 'enter_region', 'Castle.CastleFrontCourtyard', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10005, 'world', 8, NULL, 'eq', NULL, 0), (10005, 'mission_status', 10300, NULL, 'eq', 'active', 1), (10005, 'step_status', 10300, '10341', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10005, 'complete_objective', 10300, '10461', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10006, '10300 - Login stuck at A: heal to B (CIMMERIA_AUTHORING)', 'mission', 10300, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10006, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10006, 'world', 8, NULL, 'eq', NULL, 0), (10006, 'mission_status', 10300, NULL, 'eq', 'active', 1), (10006, 'step_status', 10300, '10340', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10006, 'advance_step', 10300, '10341', '{}', 0, 0);
```

Why this shape (the traps, applied):

- **Sibling disjointness.** Per event key the chains are pairwise disjoint on the `10300` status: 10001 (`not_active`) and 10006 (`active`) share `player_loaded Castle`; 10000/10002/10003 each sit alone on their key. The 1520 relog chains 1381-1383 also fire on `player_loaded Castle`; they gate on 1520 and do not overlap.
- **One `interact_tag` chain.** 10003 is the only chain on the tag. A click when the map is already removed has no chain, so it falls through to the normal dialog path (no mission action).
- **Accept gates the offer; the advance is nested.** 10003 accepts only. The A-to-B advance and the dialog run in 10004 on the real `mission_accepted` transition, so a repeat click cannot replay it. `advance_step` force-completes A's required objective and activates B. It does not complete the mission.
- **Action order in 10004.** `advance_step` first, then unbinding, then the dialog. The step condition is evaluated once, before the actions run.
- **No `dialog_choice` chain.** The close (`button -1`) has no action. A forged close or button ordinal cannot accept or complete anything.
- **Completion.** B has one required objective, so `complete_objective 10461` auto-completes the mission.
- **Once only.** After completion `mission_status 10300 = not_active` is false for chains 10000-10003, and `num_repeats = 0`.
- **Not gated on archetype.** Unlike 1367, the medic offer is open to every archetype; the story is itemless and no archetype-specific content is touched. `UATPending (end UAT, D-CA11)` for whether archetype 5 (excluded from 1520) behaves.

### Frozen (f): the reward

| Field | Q1 | Comparator |
|---|---|---|
| `reward_naq`, `reward_xp` | 0, 0 | mission 1520 (`missions.sql` row: `reward_naq 0, reward_xp 0`); mission 1521 has no reward rows either |
| `mission_reward_groups`, `mission_rewards` rows | none | no row keyed to 1520 or 1521 in `mission_reward_groups.sql` or `mission_rewards.sql` (the seed's reward groups key to unrelated missions, for example group 1 of mission 1001) |
| `award_xp` (mission, steps, objectives, tasks) | false | 1520 has `award_xp true` at mission level and zero numeric reward; Q1 chooses `false` and 0 |

Zero is the "zero or modest" choice. The first authored Castle mission is a catalog proof, and a reward adds a second thing for the client proof to validate (an item or XP grant path) for no benefit. A later packet may add a modest XP reward without changing ids.

### Behavior

| Situation | Result | Telemetry / evidence |
|---|---|---|
| 704 completed, Q1 `not_active`, in world 8, in range, click the medic | Q1 accepted; A completes; B active; map 1000001 removed; dialog 10700 shown; journal shows B | `Content: adding dialog set` (info, the offer bind, `interaction_only=true`), accept row, mission update, `dialog.remove_set` (info) |
| Click out of range | no chain, generic chat line | `interaction.out_of_range` row (`reject_interact_out_of_range`) |
| Click with 704 not completed, wrong world, or Q1 already active/completed | no accept | `content.resolve` debug row, `reason = "condition_failed"`, with `failed_condition_index` (`chain/mod.rs`) |
| Cache miss on map 1000001 | glyph never appears | warn `dialog_set_maps cache miss for add_dialog_set` |
| Forged dialog close or button, stale `initialResponse` | zero Q1 mission action (nothing keys on `dialog_choice`) | no `10003`/`10004` row |
| Walk into the courtyard at step B | `complete_objective 10461`, mission completed | chain fire row, mission update |
| Walk into the courtyard before accepting, or after completion | nothing | `condition_failed` on 10005 |
| Walk into the courtyard again at B (re-entry) | idempotent: first entry completes, later entries fail the Q1 `active` gate | `condition_failed` |
| Relog before 704 or while Q1 is `not_active` | map rebound (10001) | `Content: adding dialog set` |
| Relog at B | state restores from `sgw_mission`; no binding needed; the player must (re-)enter the courtyard | none. A player who logs in already standing inside the region must step out and in (`enter_region` fires on entry). `UATPending (end UAT, D-CA11)` |
| Relog at A (a crash between accept and the nested advance) | 10006 heals to B | chain fire row |
| Relog after completion | no offer, no bind | `condition_failed` on 10001 |
| Abandon at A or B | mission removed, map rebound (10002) | chain fire row |
| Abandon after completion | impossible; a completed one-time mission cannot reopen | none |
| Two characters on one account | independent: `available_interactions` and `sgw_mission` are per character | none |

The offer binding is per player (`available_interactions[177]`), not global. Other players' views of the medic are untouched even though the update is addressed through the template slot.

### Loader and the proof that the seed rows load

- **Seed loader:** `crates/cell-catalog/src/cell/spawner/missions.rs` loads `resources.mission_steps` and `resources.mission_objectives` at startup (queries at `:57` and `:125`). Its live-DB test file is `crates/cell-catalog/src/cell/spawner/tests/live_db_content_loaders.rs`. The "s" packet's live-DB test is `live_db_castle_side_medical_q1_seed_loads`: after `reload-db.ps1`, assert the loader returns mission 10300 with steps 10340 then 10341 (by `index`), objectives 10460/10461 and tasks 10580/10581, and exactly the XML flags.
- **Chain loader:** `build_engine(Some(&pool))` loads `content_chains` and its children; the replay tests are under `crates/cell-content/src/cell/content/chain_replay_tests/` beside `mission_1520.rs`.

### Test list

**"s" packet (catalog plus seeds)**, module `castle_side_medical_q1`:

1. `castle_side_medical_q1_xml_admitted`: `CASTLE_MEDICAL_Q1_XML` passes `apply_mission_additions::prepare`; assert it is 1,692 bytes, one line, and at most `MAX_MISSION_XML_BYTES`.
2. `castle_side_medical_q1_ids_unique_and_in_block`: ids 10300/10340/10341/10460/10461/10580/10581 pairwise unique, inside the sub-blocks, none a stock key.
3. `castle_side_medical_q1_xml_matches_seed`: parse the XML and compare ids, flags, text and `IsHidden`/`IsOptional` with the mission, step, objective and task seed rows.
4. `castle_side_medical_q1_dialog_override_matches_seed`: override 10700/10800 equals the `dialogs`/`dialog_screens` rows (exact text, zero buttons, speaker 0, `ui_screen_type 2`); both ids at most 65535.
5. `castle_side_medical_q1_registers_one_addition`: the production `MISSION_ADDITIONS` has exactly this entry (replaces the `is_empty()` assert).
6. `live_db_castle_side_medical_q1_seed_loads`: the loader test above.
7. `castle_side_medical_q1_no_reward_rows`: no `mission_rewards`/`mission_reward_groups` row for 10300; `reward_naq = reward_xp = 0`.
8. Revert proofs: change a seed id, a flag, the override text, the XML byte count in turn; each named guard fails.

**"c" packet (chains plus replay)**, module `castle_side_medical_q1`:

1. `castle_side_medical_q1_chains_load`: chains 10000-10006 load with the exact triggers, conditions and action order above.
2. `castle_side_medical_q1_accept_advances_and_shows_dialog`: replay the medic tag click through the real executor; assert accept, A to B, map 1000001 removed, dialog 10700 emitted, in that order. A resolve-only list of actions is insufficient.
3. `castle_side_medical_q1_courtyard_completes_only_at_b`: `enter_region` at B completes the mission; at A, not-active, or completed it does nothing.
4. `castle_side_medical_q1_repeat_click_noop`: a second click after accept fires nothing.
5. `castle_side_medical_q1_relog_each_step`: not-active rebinds (10001); A heals (10006); B needs nothing; completed binds nothing.
6. `castle_side_medical_q1_abandon_rebinds`: abandon at A and at B rebinds via 10002; abandon after completion cannot happen.
7. `castle_side_medical_q1_wrong_world_and_missing_704_refused`: `condition_failed` for each.
8. `castle_side_medical_q1_forged_close_is_inert`: a forged `button -1` and a stale `initialResponse` cause zero Q1 mission actions.
9. `castle_side_medical_q1_siblings_disjoint`: no event matches two chains whose conditions both hold (10001 and 10006 on `player_loaded Castle`).
10. `live_db_castle_side_medical_q1_round_trip`: accept, advance, complete and abandon persist through the production cell-to-base dispatch to `sgw_mission` and reload per character (two characters, exact sentinel ids, cleaned by exact id).
11. `castle_side_medical_q1_out_of_range_click_refused`: the real `interact` method, far target negative, `interaction.out_of_range` row.
12. Revert proofs: remove each chain, condition and the nested action in turn; each named guard fails.

### End-UAT rows (Q1 is the CS-00B client proof)

All `UATPending (end UAT, D-CA11)` until a lab or colo run; an observed blank dialog, crash, lost progress or missing marker blocks the packet.

| Row | Check |
|---|---|
| Q1-U1 journal | after accept, the journal shows *Triage Rounds*, history text, and step B text "Check the Castle front courtyard for wounded." with objective "Reach the Castle front courtyard." |
| Q1-U2 dialog | dialog 10700 renders its text on the first screen with no buttons, closes without a crash, and no blank panel appears |
| Q1-U3 marker | the medic shows the first-press marker after 704 completes and clicking it accepts on the first press (visible feedback) |
| Q1-U4 advance | step A completes and B becomes active at accept; the journal updates without a reload |
| Q1-U5 complete | entering the courtyard completes the mission; the journal marks it done; the marker does not return |
| Q1-U6 relog | relog at not-active (marker returns), at B (state restored, then walk in), and after completion (no marker) |
| Q1-U7 abandon | abandon at B returns the marker; re-accept works; a second character is unaffected |
| Q1-U8 route | the courtyard is reachable on foot from the Infirmary with no hostile aggro on the route |
| Q1-U9 races | click twice quickly; click while out of range (chat line appears) |

## 3. Q2 and Q3

### `medical_q2` (the overlooked prisoner): BlockedEvidence (nav proof)

Premise (`CIMMERIA_AUTHORING`): one prisoner in the Castle cells was missed in the triage count; the player tells the medic or a guard. Not frozen, because it needs a new template and spawn.

The blocker, cited:

- The cell prisoners are spawns 200-203 (`Castle_Pop_Prisoner1-4`), world 8, at `(234, 66.98, 1048)`, `(284, 66.98, 1048)`, `(290, 66.98, 1050)` and `(236, 66.98, 1024)`. They are template 176 (`Castle_OpCoreSoldier_Unarmed`, faction 1, `has_dynamic_properties` true).
- Template 176 also backs spawn 199 (`Castle_Pop_Infirmary_Sick1`, `(349.0, 70.48, 883.5)`) and spawn 193 (`Castle_Pop_Armory_Wounded1`), six spawns in all.
- `add_dialog_set` keys on the template id (`slot`), and the interaction update goes to every entity of that template. A glyph on one prisoner would paint the other five for that player, and a click on the five unbound ones would hit a NULL-dialog flag carrier with no chain and no feedback. That violates "every button press gets visible feedback".
- A one-NPC glyph needs a new template (a copy of 176 with its own id) and a new spawn at a nav-valid position. That is item 4(e), which needs a `nav_inspect` proof of the new position. `nav_inspect` is a shipped binary (`crates/navmesh-extractor/src/bin/nav_inspect.rs`, `docs/engine/navmesh-build-pipeline.md` section 5), and the lane runs it offline; the table below is that run. The blocker is therefore not the tool. It is that the proof needs a chosen position for a new template and spawn, and this packet may not design one (Q1 forbids new spawns; the ruling says give Q2 and Q3 `BlockedEvidence (nav proof)` rather than design a placement). Template and spawn ids are free outside the Castle block (catalog section 2: max 1595 and 14095), but none is reserved for this quest.

Missing fact that would unblock it: a chosen position for the new spawn, then its nav proof, or an owner/architect decision to accept the all-six glyph with a feedback line on the unbound prisoners (no such decision is recorded).

### Nav evidence (`nav_inspect`, offline)

Run through the lane at base `553d8128d` on the tracked `data/spaces/castle.nav` (SHA-256 `CB499C1A5724E297D4855EF656958E01343C0FACF631BE9BD34084747191E9A1`): `pwsh tools/build-lane/lane.ps1 cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/castle.nav --quiet --probes <scratch probe file>`. Probe coordinates are the spawn rows (`spawnlist.sql` 197-203) and the region 2050 points (`point_set_points.sql`). Output (mesh header omitted):

```text
components  549 (total walkable XZ area 814506.8 m^2)
probes (h-tol 2.00 m, v-tol 3.00 m)
  medic198     poly=14230  component=250  h=0.00 m  dy=-0.01 m  ok
  ogilvie197   poly=14205  component=250  h=0.00 m  dy=-0.00 m  ok
  sick199      poly=14230  component=250  h=0.00 m  dy=+0.19 m  ok
  prisoner200  poly=16872  component=250  h=0.00 m  dy=-0.00 m  ok
  prisoner201  poly=17075  component=250  h=0.00 m  dy=-0.00 m  ok
  prisoner202  poly=17141  component=250  h=0.84 m  dy=-0.00 m  ok
  prisoner203  poly=16878  component=250  h=0.59 m  dy=-0.00 m  ok
  court_c      poly=11047  component=250  h=0.40 m  dy=-0.00 m  ok
  court_p2318  poly=11276  component=250  h=0.02 m  dy=-0.00 m  ok
  court_p2319  poly=10414  component=315  h=1.60 m  dy=-51.20 m  OUT OF TOLERANCE
  court_p2321  poly=10623  component=250  h=0.64 m  dy=-0.26 m  ok
  court_p2322  poly=11050  component=316  h=0.00 m  dy=-15.73 m  OUT OF TOLERANCE
nav_inspect: one or more probes were out of tolerance
```

`status=failed exit=2` is the tool's own "a probe had no polygon in tolerance" exit, caused by two of the four region corners (`p2319` and `p2322`); `court_c` is the centre `(489.4, 28.18, 653.1)` of the four points and is on the mesh. Corners of a bounding box need not lie on walkable floor, so this is not a defect. Reading: the Infirmary NPCs, the four cell prisoners and the courtyard centre are all on component 250, so every existing spawn named in this seal is on the mesh and mutually connected. A new Q2 or Q3 spawn would need its own probe at its chosen position; that position is the missing input.

### `medical_q3` (the safe message): not authored

The only other unique-template friendly in the Infirmary is Ogilvie (spawn 197, template 179, `(362, 70.18, 886)`), who already carries mission 1520's step-A talk binding (`add_dialog_set 5827`, slot 179, chains 1380/1381). A second flag carrier on the same slot while 1520 is active makes two interaction rows on one NPC with one `interact_tag`, and the tag dispatch would need to disambiguate by mission status. That is a design change to 1520's surface, outside this packet. Other Castle friendlies are used by 701-708 chains (Coppleman 48, Gerschon 149, Marsh 10, Zuritska 168, Romney 169). Verdict `BlockedEvidence (nav proof)`: a second flag carrier needs a new template and spawn, which this packet may not place. Revisit after the CS-00B client proof shows the glyph and relog behavior of Q1.

## 4. Handoff

- Q1 downstream gate: implementation is BlockedDependency on the CS-00B client proof and W1 restoration UAT (D-CA8). The sealed rows above are the exact input to the CS-04a "s" and "c" packets.
- Files the packets touch, from D-CA17: `crates/resources/src/base/mission_additions_castle.rs`, `crates/resources/src/base/mission_additions.rs` (registration), `crates/resources/src/base/dialog_overrides/mod.rs`, `db/resources/Missions/Seed/{missions,mission_steps,mission_objectives,mission_tasks}.sql`, `db/resources/Dialogs/Seed/{dialogs,dialog_screens,dialog_sets,dialog_set_maps}.sql`, `db/resources/Content/Seed/castle_side_stories_chains.sql`, and the tests named above.
- Not edited by this packet: `id-allocation.md` (CS-07 should add the Q1 ids: mission 10300, steps 10340-10341, objectives 10460-10461, tasks 10580-10581, dialog 10700, dialog set 10760, screen 10800, map 1000001, chains 10000-10006), `cb-n05-seal.md`.
- Text ids: Q1 claims none. The ruling's side note stands: CB-N05 proved copy can live inline in the XML and in `dialog_screens` plus `DIALOG_OVERRIDES`, so a "text ids" blocker for other Castle quests is a misread.
