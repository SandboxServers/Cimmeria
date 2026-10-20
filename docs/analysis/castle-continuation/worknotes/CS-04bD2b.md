# CS-04bD2b seal: defense_q3, *Hold the Bravo Route* (mission leg and spawn leg)

> **Type:** How-to (design seal)
> **Audience:** the CS-04b3 s/d/c1/c2 coder packets and their reviewers
> **Last updated:** 2026-10-18
> **Companions:** [CS-04aD](CS-04aD.md) (the Q1 model seal), [CS-04bD](CS-04bD.md) (draft premise; its ids are void, G3), [reconciliation](../evidence/reconciliation.md)
> **Erratum (CS-07R-ID, 2026-10-18):** the giver template and its dialog slot 421 in this seal are now **425**; 421 collided with Tollana's registered template ([CS-07R-id-collision](CS-07R-id-collision.md)). Read every "template 421" and "slot 421" below as 425.

Labels: `REPO` is read from the tree (cited file:line). `CIMMERIA_AUTHORING` is an authored premise, copy or number. `OPEN` is not proven here. Everything client-side is `UATPending (end UAT)`.

**Premise (`CIMMERIA_AUTHORING`).** A sergeant at the Stargate arrival (a new placed copy of the standoff sergeant, template 421, shown as "Sgt. Stanton") cannot move a squad past the hostile officers holding the Bravo and bunker route. After the player has reported the guard-post intelligence (`defense_q1`, mission 10310), he asks for four of the placed officers to be put down: Bravo officers 1, 2 and 3 and Muelbach. The player kills each once. The fourth distinct kill completes the mission. The fight is the existing placed fight (design 1 of CS-04bD, G12): no spawn, despawn or retag of any shared actor.

**Mechanism.** One step with four required objectives, one per target tag. Each target's `entity_dead_tag` chain completes only its own objective and is gated on `objective_status ... neq completed`. So distinct targets count, a respawned officer killed again is inert, and the last completion completes the mission with no counter. `IncrementCounter` is not used: counters live on the cell entity and are not persisted (`crates/entity/src/cell_entity/entity_struct.rs:795-798`), and the counter chain shape has the pre-increment N-1 trap. Objective state is persisted per character, so the count survives relog with no reset chain.

Facts relied on (`REPO`, base `2fad73404`):

- `entity_dead_tag` credits the killer only; a pet credits its owner; an NPC killer credits nobody (`cell-combat/src/cell/abilities/use_ability/kill_credit.rs:98-108`; `cell-content/src/cell/content/event_dispatch/lifecycle/mod.rs` `fire_entity_death`).
- `complete_objective` flips an objective only if it is in the player's `active_objectives` (`crates/entity/src/missions.rs:129-143`); `accept_mission` seeds `active_objectives` with every objective of the mission's first step (`cell-content/src/cell/content/executor/mission.rs:39-50`, `missions/lifecycle.rs:152`).
- When every non-optional active objective is `STATUS_COMPLETED` the mission completes (`missions/progression.rs:463-506`), and the final required objective fires `mission_completed` (`executor/mission.rs:428-459`).
- `objective_status` conditions read `mission_<m>_obj_<o>_status` = `active`/`completed` (`content/mission_context.rs:153-192`); the row needs `target_id` = mission and `target_key` = the objective id as text, or the loader drops it and the chain is UNGATED (`content-engine/src/loader/condition.rs:49-59`).
- The four hostiles are placed spawns, all `respawn_secs` 120, world 8: Muelbach spawn 241 (template 170, 1008/48/414), BravoOfficer1/2/3 spawns 242/243/244 (template 171; 955/25/475, 965/25/485, 968/25.2/477) (`db/resources/Worlds/Seed/spawnlist.sql:519,532,533,537`). Existing chains 1346-1351 key the same four tags with scope `space` (`db/resources/Content/Seed/castle_706_708_chains.sql:520-636`); they are gated on mission 708 and keep firing alongside this quest.
- The placed Stanton (rejected as giver, see Frozen (e)) is spawn 204 `Castle_Standoff_SgtStanton` (template 188, faction 3, level 4, `has_dynamic_properties` true, `static_interaction_sets` `{}`, `aggro_radius` 30; 532/25.97/624, heading 2.219484, `respawn_secs` 120) (`spawnlist.sql:640`, `entity_templates.sql:578`). Template 188 has exactly one spawn in `db/resources/Worlds` (spawn 204). Its tag is in no chain, no dialog and no mission seed (`git grep`, empty). `live_db_castle_standoff.rs:26` only pins the tag-to-template pair.

## Verdict

- `defense_q3`: `SEALED` with the fallback giver (new template 421, new spawn 14096). Placed Stanton (spawn 204) is rejected: it sits on navmesh component 250, which the arrival (component 116) cannot reach on foot, so the offer would be unreachable. Three of the four targets and the new giver are on component 116 (`nav_inspect` exit 0). `Castle_BravoOfficer2` (spawn 243) stands on component 214, a 9-poly island with no walkable link to 116 (closest approaches are through 163 and 205, 1.6-1.8 m jump gaps); it is 14 m from officer 1 and is killable by ranged fire, which the static proof cannot confirm. That is end-UAT row DQ3-U12 and it blocks the packet if it fails. Everything client-side stays `UATPending (end UAT)`.

## defense_q3 IDs

Subset of the D-CA20 row. Reserved but unused ids are listed as such.

| Namespace | Used | Reserved, unused |
|---|---|---|
| Mission | 10312 | |
| Steps | 10366 | 10367-10371 |
| Objectives | 10486, 10487, 10488, 10489 | 10490-10491 |
| Tasks | 10606, 10607, 10608, 10609 | 10610-10611 |
| Dialogs | 10717 (offer monologue), 10718 (completion monologue) | 10719-10720 |
| Dialog sets | 10769 | 10770 |
| Screens | 10833 (dialog 10717), 10834 (dialog 10718) | 10835-10840 |
| Screen buttons | none | 10936-10939 |
| Chains | 10120-10129 | none |
| Dialog set maps | 1000010 | 1000011 |
| Template, spawn | template 421, spawn 14096 (the giver, see Frozen (e)) | 422-429 |

Template 420 and spawn 13809 are the D-CA20 spare fallback pair and are claimed by CS-05S3 (`repeatable_q1` giver), so this seal takes template 421 (inside the CS-05S2a reserve 420-429; 419 is the `defense_q1` giver of CS-04bD2a) and spawn 14096, the first id above the spawnlist seed maximum 14095, with the five-ref proof below. The two seals therefore cannot collide at build time.

Objective to target: 10486 `Castle_BravoOfficer1`, 10487 `Castle_BravoOfficer2`, 10488 `Castle_BravoOfficer3`, 10489 `Castle_Muelbach`. No text row, no event set, no sequence.

## Collision check

Run in `castle-cs-04bd2b` at `2fad73404` with the defense_q3 regex `10312|1036[6-9]|1037[01]|1048[6-9]|1049[01]|1060[6-9]|1061[01]|1071[7-9]|10720|1076[9]|10770|1083[3-9]|10840|1093[6-9]|1012[0-9]|100001[01]` over `db/resources/Missions`, `db/resources/Dialogs` and `db/resources/Content`. Pasted output (every ref empty):

```text
== HEAD
== content/cellblock-campaign
== content/harset-campaign
== content/tollana-campaign
== origin/main
```

Fallback template 421 and spawn 14096, `git grep -n -E "VALUES \((421|14096),"` over `db/resources/Entities` and `db/resources/Worlds` on the same five refs (blueprint rows excluded; they are an unrelated `blueprint_id` column). Every ref empty:

```text
== HEAD
== content/cellblock-campaign
== content/harset-campaign
== content/tollana-campaign
== origin/main
```

Spawns 13809 (CS-05S3) and 13810 (`db/resources/Worlds/Seed/debug_area_rings.sql:36,43`) are not available, which is why this seal goes above the seed maximum rather than to "13809 + 1". The spawn sequence footer is `GREATEST(MAX(spawn_id), last_value, 13799)`, so 14096 as the new maximum is covered.

## Frozen (a) XML

File `crates/resources/src/base/mission_additions_castle.rs`, `pub const CASTLE_DEFENSE_Q3_XML: &str = r#"..."#;` (the copy contains no `"#`). One line, no newline, Q1's attribute shape. Registered as `MissionAddition { mission_id: 10312, xml: CASTLE_DEFENSE_Q3_XML }`; the registry test finds it by id (G7), never by index.

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="3" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Hold the Bravo Route" MissionID="10312"><HistoryText>Hostile officers hold the Bravo and bunker route. Sergeant Stanton wants four of them put down before the guard change.</HistoryText><Steps StepEnabled="false" StepID="10366" AwardXP="false" Difficulty="1"><StepDisplayLogText>Put down the hostile officers on the Bravo route.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10486" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Defeat the first Bravo officer.</DisplayLogText><Tasks TaskType="1" TaskID="10606" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives><Objectives IsOptional="false" ObjectiveID="10487" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Defeat the second Bravo officer.</DisplayLogText><Tasks TaskType="1" TaskID="10607" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives><Objectives IsOptional="false" ObjectiveID="10488" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Defeat the third Bravo officer.</DisplayLogText><Tasks TaskType="1" TaskID="10608" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives><Objectives IsOptional="false" ObjectiveID="10489" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Defeat Muelbach.</DisplayLogText><Tasks TaskType="1" TaskID="10609" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

- **Byte count: 2,116** UTF-8 bytes, measured with `[IO.File]::ReadAllBytes(<scratch file>).Length` on the exact line above (no BOM, no newline, zero `\n`). Limit `MAX_MISSION_XML_BYTES` = 355,840. Margin 353,724.
- **SHA-256:** `16cb750a68680e45b6265c95cf6bb30948770d8844254a8f6f7757745f40c876`. The scratch file parses as XML with `MissionID` 10312.
- Admission rules met by construction: ids 10312, 10366, 10486-10489 and 10606-10609 are pairwise unique and at most 65535; `MissionID` equals the registry key; no DOCTYPE; child order `HistoryText`, `Steps` (`StepDisplayLogText`, four `Objectives` each with `DisplayLogText` then `Tasks`). Copy has no apostrophe, so the XML and the SQL need no escaping.
- One step, so the declaration order is trivially the seed `index` order. All four objectives are `IsOptional="false"`: the auto-complete rule counts required objectives, and an optional one would not gate completion (revert proof in `### s`).

## Frozen (b) Missions seeds

Placement: each row directly after the current last data row of its file, before any trailing `setval`, in ascending id order. On this base that is the 10300 row (`missions.sql:2112`, `setval` at :2113), the 10341 row (`mission_steps.sql`, last line), the 10461 row (`mission_objectives.sql`, last line) and the 10581 row (`mission_tasks.sql`, last line). Sibling seals' rows may already be present at build time; insert after the highest id below this quest's.

`missions.sql`:

```sql
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (10312, 'Hostile officers hold the Bravo and bunker route. Sergeant Stanton wants four of them put down before the guard change.', false, true, false, false, 1, true, true, false, false, false, 3, 'Hold the Bravo Route', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`mission_steps.sql`:

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10366, 10312, false, 1, false, 'Put down the hostile officers on the Bravo route.', 0);
```

`mission_objectives.sql`:

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10486, 10366, false, 1, false, false, false, 'Defeat the first Bravo officer.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10487, 10366, false, 1, false, false, false, 'Defeat the second Bravo officer.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10488, 10366, false, 1, false, false, false, 'Defeat the third Bravo officer.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10489, 10366, false, 1, false, false, false, 'Defeat Muelbach.');
```

`mission_tasks.sql`:

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10606, 10486, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10607, 10487, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10608, 10488, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10609, 10489, false, 1, true, 1);
```

Agreement with the XML: same ids, text, `IsHidden`/`IsOptional` false for all four objectives, `award_xp` false everywhere, `can_abandon` true, `num_repeats` 0 (one-time), level 3 (as Q1). `script_name`/`script_spaces` are seed-only. No `mission_rewards`/`mission_reward_groups` row for 10312.

## Frozen (c) Dialogs seeds and override

No `texts`/`error_texts` rows (G6): the copy is inline. Both dialogs are one-screen, zero-button, speaker-0 monologues (G5), so a non-interact chain may display them. No `dialog_screen_buttons` rows; button ids 10936-10939 stay reserved.

Placement: after the highest existing row of each file, before the `setval` (dialogs :10872 after the 10741 row; dialog_screens after screen row 10901; dialog_sets after the 10760 row; dialog_set_maps after the 1000001 row at :9355).

`dialogs.sql`:

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10717, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10718, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
```

`dialog_screens.sql`:

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10717, 10833, 'Sergeant Stanton checks the load in his rifle. "The Bravo officers hold the route and I cannot move a squad past them. Put all four down and the road is ours."', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10718, 10834, 'The last officer falls and the Bravo route goes quiet. Sergeant Stanton will hold it until the next wave arrives.', 0, 0);
```

`dialog_sets.sql`:

```sql
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10769, '');
```

`dialog_set_maps.sql` (the NULL-dialog interaction-only flag carrier, the Q1/2795 shape; flags 1073741824 = `MISSION_WORLD_OBJECT`):

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000010, 10769, NULL, 'Speak with Sergeant Stanton', 1073741824, 1, '{}', '{}', '{}', '{}');
```

`crates/resources/src/base/dialog_overrides/overrides_castle.rs`, appended after `MEDICAL_Q1_DIALOG` (text equals the seed rows byte for byte; ids at most 65535):

```rust
/// Castle defense_q3 dialog 10717: Stanton's offer monologue, shown after
/// the accept. Frozen in the CS-04bD2b worknote (Frozen (c)).
pub(super) const DEFENSE_Q3_OFFER_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10717,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10833,
        speaker_id: 0,
        text: "Sergeant Stanton checks the load in his rifle. \"The Bravo officers hold the route and I cannot move a squad past them. Put all four down and the road is ours.\"",
        buttons: &[],
    }],
};

/// Castle defense_q3 dialog 10718: the completion monologue, shown when the
/// fourth objective completes the mission.
pub(super) const DEFENSE_Q3_DONE_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10718,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10834,
        speaker_id: 0,
        text: "The last officer falls and the Bravo route goes quiet. Sergeant Stanton will hold it until the next wave arrives.",
        buttons: &[],
    }],
};
```

`dialog_overrides/mod.rs` (budget G4/CD-12: exactly four added lines, no comment lines). At the END of `DIALOG_OVERRIDES`, after `overrides_castle::MEDICAL_Q1_DIALOG,` (:208):

```rust
    overrides_castle::DEFENSE_Q3_OFFER_DIALOG,
    overrides_castle::DEFENSE_Q3_DONE_DIALOG,
```

and, among the `#[cfg(test)]` mods after `override_seed_agreement_castle_10700` (:60):

```rust
#[cfg(test)]
mod override_seed_agreement_castle_defense_q3;
```

The mod.rs file is 640 lines, so this leaves it at 644 (plus other quests' four each, under the 700 hard cap).

## Frozen (d) Chains

File `db/resources/Content/Seed/castle_side_stories_chains.sql`, appended after the Q1 block (D-CA17). One statement per line (interact_tag_linter). Scope `mission`, scope id 10312. `add_dialog_set`/`remove_dialog_set` use `target_id` = the set map id 1000010 and `slot` = the template id 421 (`executor/dialog/mod.rs`); the binding is per player but keyed on the template slot, so it paints every spawn of template 421 for that player (there is one: spawn 14096). The prerequisite 10310 (`defense_q1`) is the CIMMERIA_AUTHORING offer gate; the swap rule is below the block.

```sql
-- Castle defense_q3 (CS-04bD2b): chains 10120-10129. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10120, '10312 - 10310 completed: offer the giver (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10120, 'mission_completed', '10310', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10120, 'world', 8, NULL, 'eq', NULL, 0), (10120, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10120, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10120, 'add_dialog_set', 1000010, NULL, '{"slot": 421}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10121, '10312 - Login, offer open: rebind the giver (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10121, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10121, 'world', 8, NULL, 'eq', NULL, 0), (10121, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10121, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10121, 'add_dialog_set', 1000010, NULL, '{"slot": 421}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10122, '10312 - Abandoned: rebind the giver (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10122, 'mission_abandoned', '10312', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10122, 'world', 8, NULL, 'eq', NULL, 0), (10122, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10122, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10122, 'add_dialog_set', 1000010, NULL, '{"slot": 421}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10123, '10312 - Giver click: accept (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10123, 'interact_tag', 'Castle_DefenseQ3_Giver', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10123, 'world', 8, NULL, 'eq', NULL, 0), (10123, 'mission_status', 10310, NULL, 'eq', 'completed', 1), (10123, 'mission_status', 10312, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10123, 'accept_mission', 10312, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10124, '10312 - Accepted: unbind the giver, show 10717 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10124, 'mission_accepted', '10312', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10124, 'world', 8, NULL, 'eq', NULL, 0), (10124, 'step_status', 10312, '10366', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10124, 'remove_dialog_set', 1000010, NULL, '{"slot": 421}', 0, 0), (10124, 'display_dialog', 10717, NULL, '{}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10125, '10312 - Kill Bravo officer 1: complete 10486 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10125, 'entity_dead_tag', 'Castle_BravoOfficer1', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10125, 'world', 8, NULL, 'eq', NULL, 0), (10125, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10125, 'step_status', 10312, '10366', 'eq', 'active', 2), (10125, 'objective_status', 10312, '10486', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10125, 'complete_objective', 10312, '10486', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10126, '10312 - Kill Bravo officer 2: complete 10487 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10126, 'entity_dead_tag', 'Castle_BravoOfficer2', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10126, 'world', 8, NULL, 'eq', NULL, 0), (10126, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10126, 'step_status', 10312, '10366', 'eq', 'active', 2), (10126, 'objective_status', 10312, '10487', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10126, 'complete_objective', 10312, '10487', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10127, '10312 - Kill Bravo officer 3: complete 10488 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10127, 'entity_dead_tag', 'Castle_BravoOfficer3', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10127, 'world', 8, NULL, 'eq', NULL, 0), (10127, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10127, 'step_status', 10312, '10366', 'eq', 'active', 2), (10127, 'objective_status', 10312, '10488', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10127, 'complete_objective', 10312, '10488', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10128, '10312 - Kill Muelbach: complete 10489 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10128, 'entity_dead_tag', 'Castle_Muelbach', 'space', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10128, 'world', 8, NULL, 'eq', NULL, 0), (10128, 'mission_status', 10312, NULL, 'eq', 'active', 1), (10128, 'step_status', 10312, '10366', 'eq', 'active', 2), (10128, 'objective_status', 10312, '10489', 'neq', 'completed', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10128, 'complete_objective', 10312, '10489', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10129, '10312 - Completed: show 10718 (CIMMERIA_AUTHORING)', 'mission', 10312, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10129, 'mission_completed', '10312', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10129, 'world', 8, NULL, 'eq', NULL, 0), (10129, 'mission_status', 10312, NULL, 'eq', 'completed', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10129, 'display_dialog', 10718, NULL, '{}', 0, 0);
```

Why this shape (the traps, applied):

- **Siblings disjoint.** Every event key carries at most one chain, except the two cases below. `entity_dead_tag` on each of the four tags has exactly one 10312 chain (10125-10128). The 708 chains (1346-1351) share those tags but are gated on mission 708; they fire alongside, independent of this quest (both can credit one kill when a player holds both). `player_loaded Castle` has 10121 only (the 1381-1383 relog chains gate on 1520 and do not overlap, and Q1's 10001/10006 gate on 10300).
- **No shared slot.** Template 421 has exactly one spawn and no other quest binds slot 421, and the tag `Castle_DefenseQ3_Giver` is new, so 10123 is the only `interact_tag` chain on it. Placed Stanton (template 188, spawn 204) is on component 250 and is not a giver here, so `defense_q1` cannot collide on slot 188 through this seal. If CS-04bD2a nevertheless freezes slot 188 for its own giver, nothing in 10120-10129 touches slot 188 (c2 test 7 is `n/a`).
- **Swap rule if 10310 is not built (held).** The offer is then unreachable. The mechanical fix, applied by the build brief and not by a coder on their own: in chains 10120-10123 replace `10310` with `10300` in the trigger key of 10120 and in the `mission_status` target of 10120-10123. No other row changes (the kill and completion chains never mention 10310).
- **One `interact_tag` chain.** 10123 is the only chain on the tag for this quest. An `interact_tag` chain short-circuits `handle_interact`, so `last_interaction_target` is never pinned; nothing here follows with a `dialog_choice`, which is why the monologue is shown by the nested `mission_accepted` chain (10124), exactly as Q1.
- **Accept gates the offer; the dialog is nested.** 10123 accepts only. The unbind and the monologue run in 10124 on the real `mission_accepted` transition (`executor/mission.rs:93-96`), so a repeat click cannot replay them. Action order in 10124: unbind first, dialog second. No `advance_step`: the single step is active at accept and its four objectives are seeded active.
- **Counting by distinct targets.** Each of 10125-10128 completes only its own objective and is gated on that objective's `neq completed`. A respawned officer (120 s) killed again fails the gate (`condition_failed`), so re-kills never double-count, never error and never complete anything. Conditions are frozen at resolve time and the four chains sit on four different keys, so a kill never fires two of them.
- **Completion needs no chain.** The fourth completion makes every required objective `STATUS_COMPLETED`, the mission completes and `mission_completed 10312` fires; 10129 shows 10718. 10129 only displays, so a repeat is harmless; the one-time mission cannot complete twice (`num_repeats` 0).
- **No heal chain.** Q1 needed one because it had an advance between two steps; this quest has one step, so there is no stuck-between-steps state.
- **No `dialog_choice` chain.** A forged close or button has no chain and cannot accept, advance or complete anything.
- **Condition columns.** `step_status` and `objective_status` carry the id as TEXT in `target_key` and the mission in `target_id`; `mission_status` carries the mission in `target_id`. A malformed row would leave the chain ungated, so the c1 `chains_load` test pins all columns.

## Frozen (e) Templates and spawns

Giver: a new friendly template 421 (a column-for-column copy of template 188 with a new `template_name`) and one stationary spawn 14096 on component 116, 22-26 m from Col Marsh and Moh'katan and outside every hostile's reach. The targets are the placed spawns 241-244; they are unchanged.

`nav_inspect` run on this tree at `f902167c0`, `data/spaces/castle.nav` (header: nverts 40068, npolys 19815, 549 components), through the lane. Probe file `C:\Users\Steve\AppData\Local\Temp\castle-r4\cs04bd2b-probes5.txt`, kept out of the repo. h-tol 2.00 m, v-tol 3.00 m. **Exit 0**, all on one component:

```text
Arrival       poly=9348  component=116  h=0.00 m  dy=-0.08 m  ok   (761.677, 63.466, 551.716)
GiverA        poly=8891  component=116  h=0.00 m  dy=-0.01 m  ok   (790.0, 59.7, 530.0)   <- the new giver
Marsh118      poly=8859  component=116  h=0.00 m  dy=-0.24 m  ok
Bravo1_242    poly=8307  component=116  h=0.00 m  dy=+0.22 m  ok
Bravo3_244    poly=8201  component=116  h=0.00 m  dy=+0.12 m  ok
Muelbach_241  poly=7316  component=116  h=0.00 m  dy=-0.18 m  ok
```

Probes that did not pass, kept as evidence (exit 3):

```text
Stanton204    poly=10449 component=250  h=0.00 m  dy=+0.11 m  ok   (532.0, 25.97, 624.0)  <- NOT 116: rejected as giver
Bravo2_243    poly=8420  component=214  h=0.00 m  dy=+0.62 m  ok   (965.0, 25.0, 485.0)   <- island
```

Probes next to Bravo2 (963/960 x 483/485, 970, 967/965 x 481/487/489) snap either to 214 (poly 8420) or to 116 only with a vertical or horizontal miss outside tolerance. `--gaps --gap-pair 214,116` lists 214 (1,188 m^2, 9 polys) linking to 116 only through components 163 (1,123 m^2) and 205 (255 m^2), at 1.58-1.77 m horizontal and about 1 m vertical. So a player cannot be proved to walk onto officer 2's patch; the other three officers are reachable.

Rejection of the placed Stanton: component 250 holds the courtyard, the infirmary and the prisoners (G11, CS-04cS). The arrival is on 116. A character landing at the DHD cannot reach 250 on foot (end-UAT S4), so an offer there would be unreachable.

**Frozen rows** (CIMMERIA_AUTHORING).

`db/resources/Entities/Seed/entity_templates.sql`, one line, directly after the highest-id row below 421 (on this base the row for template 415):

```sql
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, aggro_radius, assist_radius, wander_radius, wander_min_dwell_secs, wander_max_dwell_secs) VALUES (421, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_00,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00,WP-Human.WP_SMG_1A}', 0, 0, 570, 4, 0, 3, 7036, NULL, NULL, NULL, 'Castle_DefenseQ3_Giver', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, -256, -256, -52773120, 21, '{}', NULL, NULL, true, NULL, NULL, true, 30, NULL, NULL, NULL, NULL);
```

It differs from template 188 (`entity_templates.sql:578`) only in `template_id` and `template_name`. Faction 3 (not 10), so it is not damageable and not hostile; `name_id` 7036 renders as "Sgt. Stanton" (`texts.sql:53124`). `OPEN`: the client shows two "Sgt. Stanton" actors in the world (one at the arrival, one in the courtyard); they cannot be seen together because 250 is unreachable. A distinct name needs a new `name_id`, which G6 says is not available as a text row; leave as is.

`db/resources/Worlds/Seed/spawnlist.sql`, one line, directly after the spawn 13804 row (:624):

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (14096, 790.0, 59.7, 530.0, 5.3559, 8, 421, 'Castle_DefenseQ3_Giver', NULL, true);
```

Heading 5.3559 rad faces the arrival (761.7, 551.7). `respawn_secs` is NULL (friendly, undamageable, as Col Marsh spawn 118). The tag is new (unique in the world 8 tag set that `live_db_castle_population` builds). The spawn sequence is covered: the `spawnlist.sql` footer sets `spawnlist_spawn_id_seq` to `GREATEST(MAX(spawn_id), last_value, 13799)`, so 14096 as the new seed maximum is picked up and `live_db_seed_sequences` is unaffected.

**Clearance numbers** (horizontal, giver at (790.0, 530.0); default aggro 18 m, assist 10 m, `live_db_castle_standoff.rs:41`):

| Hostile or group | Distance | Reach | Margin |
|---|---|---|---|
| Bravo officer 1 (spawn 242, 955/475) | 168.1 m | 18 m | 150.1 m |
| Bravo officers 2, 3 (243, 244) | 180.7 m, 185.7 m | 18 m | over 162 m |
| Muelbach (241, 1008/414) | 247.0 m | 18 m | 229.0 m |
| Bunker approach 279, 280 (templates 185/184, aggro 20, assist 12) | 220.5 m, 229.4 m | 20 m | over 200 m |
| Alpha Jaffa 209-211 (template 189, faction 3, not hostile) | 104.0 m | 30 m | n/a |
| Col Marsh and Moh'katan (118, 120; friendly) | 25.6 m, 22.8 m | n/a | no overlap |

So no idle hostile reaches the giver, and the giver's click is not preempted by aggro. The arrival-to-giver leg is on one component (116), 36 m straight line.

**Fights on the route** (REPO seed, `OPEN` as a feel): officers 2 and 3 are 8.5 m apart and pull each other by assist (10 m); officer 1 is 13.2 m from 3 and 14.1 m from 2, so it does not assist and is fought alone unless the player is within its 18 m aggro of 2 or 3. Muelbach (template 170, level 1, faction 10) has bunker soldiers 280 and 279 at 17.8 m and 26.4 m (aggro 20, assist 12): pulling Muelbach brings both (279 and 280 are 9.2 m apart). All are level 1. These are the stock placed hostiles of chains 1346-1351; this quest adds no hostile.

**Respawn and throughput.** All four targets have `respawn_secs` 120 (`spawnlist.sql:519,532,533,537`). Death stamps `respawn_at = now + respawn_secs` and the 1 Hz respawn tick promotes it (`cell-combat/.../combat/state.rs:28-31`, `cell/.../ticks/npc_respawn/mod.rs`). Each target has its own timer, so a wave of four is up again 120 s after its last death at the latest, and an individual officer 120 s after its own death. Credit is the killer's only (`kill_credit.rs:98-108`), so one wave carries four credits for one player. Two players who each need all four need at least two waves: both players clear a wave together and split the credits (P1 takes officers 1, 2; P2 takes officer 3 and Muelbach), then the swap happens on the next wave 120 s later. Worst case about 2-3 minutes with travel, and each player's re-kills are inert (the `objective_status neq completed` gate), so a second player cannot be blocked by the first. A pet credits its owner, which does not help the other player. A group does not share credit. This is acceptable for a one-time story quest; a party member who lands no killing blow has to wait for a respawn.

## Frozen (f) Reward

| Field | defense_q3 | Comparator |
|---|---|---|
| `reward_naq`, `reward_xp` | 0, 0 | Q1 (CS-04aD, Frozen (f)); mission 1520 has both 0 and no reward rows; all twelve stock Castle missions have 0 (CS-04bD:46) |
| `mission_rewards`, `mission_reward_groups` rows | none | no row keyed to 10312 |
| item grant action | none | no `add_item`, `grant_xp` or `grant_cash` in chains 10120-10129 |
| `award_xp` (mission, step, objectives, tasks) | false | as Q1 |

Zero is chosen because a first authored combat quest should not add a reward grant path to the thing being proven. A later packet may add a modest XP grant without changing ids (an `add_item`/`grant_xp` action on 10129). Economy sign-off: none needed for zero.

## Behavior

Start state is "prerequisite 10310 completed, 10312 `not_active`" unless a row says otherwise. Every row is per player.

| State | Event | Chain | Actions / result |
|---|---|---|---|
| 10310 completes | `mission_completed 10310` | 10120 | bind map 1000010 on slot 421; the giver shows the offer glyph |
| Login, 10310 completed, 10312 `not_active` | `player_loaded Castle` | 10121 | rebind (idempotent) |
| 10312 abandoned (any count) | `mission_abandoned 10312` | 10122 | rebind; a re-accept restarts at 0 of 4 |
| Offer open | click the giver (in range) | 10123 then 10124 | accept; unbind; monologue 10717 appears; journal shows the four objectives. This is the first-press feedback |
| Click the giver with 10310 not completed, 10312 active or completed, wrong world | `interact_tag` | none (`condition_failed`) | no mission action; the NPC has no binding so the normal path gives the generic response, same as Q1 |
| Click out of range | `interact_tag` | none | `interaction.out_of_range` row, generic chat line |
| Active, officer N alive and its objective active | killer's `entity_dead_tag` | 10125/6/7/8 | `complete_objective` for that officer; the journal ticks it |
| Active, officer N killed again after respawn | `entity_dead_tag` | none (`condition_failed` on `objective_status`) | inert |
| Active, killed by an NPC or by another player | none for this player | none | credit goes to the killer only; a pet credits its owner |
| Fourth distinct objective completes | `complete_objective` last | engine auto-complete, then 10129 | mission `completed`; monologue 10718; no offer returns |
| Kill before accept, or after completion | `entity_dead_tag` | none | inert |
| Relog at not_active with 10310 completed | `player_loaded Castle` | 10121 | offer rebound |
| Relog while active (any count) | `player_loaded Castle` | none | completed objectives restore from the character row (H50); the remaining kills still count |
| Relog after completion | `player_loaded Castle` | none | no offer, no bind |
| Forged `dialog_choice` or button | none | none | nothing keys on `dialog_choice` |
| Two characters on one account | independent | | state is per character |
| Two players kill the same officer | the killer only | | the other player's count is unchanged; see the Test list |

Shared-actor rules: no chain here despawns, retags or spawns anything. The only per-player state is the dialog-set binding and the mission row. Respawn is the placed spawns' own (120 s each).

Throughput (mission-side fact, not a spawn proof): a kill gives one credit, to one player. A wave of four therefore carries four credits in total; two players who both need all four need at least two waves. Agent 2's answer is in Frozen (e), *Respawn and throughput*.

## Test list

Module names start with `castle_side_defense_q3` so the lane filter `test(castle_)` picks them up.

### s

1. `castle_side_defense_q3_xml_admitted`: the XML passes `apply_mission_additions::prepare`; assert 2,116 bytes, one line, at most `MAX_MISSION_XML_BYTES`. Revert: change one byte of the constant; fails on the byte count and sha.
2. `castle_side_defense_q3_ids_in_block`: ids 10312, 10366, 10486-10489, 10606-10609 pairwise unique, inside the D-CA20 row, none a stock key. Revert: move an id out of its row; fails.
3. `castle_side_defense_q3_is_registered_once`: the registry holds exactly one entry for 10312, found by id; its xml equals the constant. Revert: delete the `MISSION_ADDITIONS` entry; fails.
4. `castle_side_defense_q3_xml_matches_seed`: parse the XML; mission, step, four objectives and four tasks match the seed rows in id, text, `IsHidden`, `IsOptional`, `TaskType`. Revert: edit one display text in the seed; fails.
5. `castle_side_defense_q3_four_required_objectives`: the single step has exactly four non-optional objectives (the auto-complete rule counts required objectives only). Revert: set one `is_optional` true in the seed; fails.
6. `castle_side_defense_q3_reward_matches_seal`: zero reward rows, `reward_naq = reward_xp = 0`. Revert: add a `mission_rewards` row; fails.
7. `castle_side_defense_q3_giver_rows_frozen`: template 421 equals template 188 except `template_id` and `template_name`; faction 3; spawn 14096 is world 8, template 421, tag `Castle_DefenseQ3_Giver`, stationary, `respawn_secs` NULL; the tag is unique among world 8 spawns. Revert: set the faction to 10 or change the tag; fails.
8. `live_db_castle_side_defense_q3_seed_loads`: after `reload-db.ps1`, the loader returns mission 10312 with `step_id` 10366 and the four objectives (what `accept_mission` seeds). Revert: drop one objective row; fails.

### d

1. `castle_side_defense_q3_dialog_override_matches_seed`: for each of 10717 and 10718 the served override equals the `dialogs`/`dialog_screens` rows (exact text, zero buttons, speaker 0, `ui_screen_type` 2); ids at most 65535. Revert: change one character of the override text; fails.
2. `castle_side_defense_q3_dialog_set_seed`: set 10769 and map 1000010 carry the frozen values (NULL dialog, flags 1073741824, topic text). Revert: change the flags; fails.
3. `castle_side_defense_q3_overrides_registered`: both dialogs are in `DIALOG_OVERRIDES`. Revert: remove one line; fails.

### c1

1. `castle_side_defense_q3_chains_load`: chains 10120-10129 load with the exact triggers, condition columns and action order above (including `target_key` text ids and `neq`). Revert: blank one `target_key`; the chain loads ungated and the test fails.
2. `castle_side_defense_q3_offer_binds_after_prereq`: `mission_completed 10310` runs 10120; with 10310 not completed it does not. Revert: drop the 10310 condition; fails.
3. `castle_side_defense_q3_accept_unbinds_and_shows_dialog`: replay a giver click through the real executor; assert accept, then map 1000010 removed, then dialog 10717 emitted, in that order. Revert: delete 10123; fails.
4. `castle_side_defense_q3_each_target_completes_its_objective`: each of the four tags completes exactly its own objective and no other. Revert: swap two objective ids; fails.
5. `castle_side_defense_q3_fourth_kill_completes_and_shows_dialog`: after three objectives, the fourth completes the mission and 10718 is emitted. A resolve-only list of actions is insufficient. Revert: delete 10129; fails.
6. `castle_side_defense_q3_rekill_is_inert`: after an objective is completed, the same tag fires nothing for 10312 (`condition_failed`), and no mission frame is sent. Revert: drop the `objective_status` row; fails.
7. `castle_side_defense_q3_kill_outside_window_is_inert`: before accept and after completion, kills fire nothing. Revert: drop `mission_status 10312 eq active` from 10125; the before-accept kill matches and the test fails.
8. `castle_side_defense_q3_coexists_with_708`: with 708 at step 2416 and 10312 active, one kill fires both the 1346-class chain and this quest's chain with neither blocking the other. Revert: add `mission_status 708 neq active` to 10125; the 708-active case stops matching and the test fails.

### c2

1. `castle_side_defense_q3_relog_each_stage`: not_active with 10310 completed rebinds (10121); active at 0, 2 and 3 of 4 keeps the completed set and the remaining kills still count; completed binds nothing. Revert: delete chain 10121; the not_active relog case binds nothing and fails.
2. `castle_side_defense_q3_abandon_rebinds_and_resets`: abandon at 0..3 of 4 rebinds (10122); a re-accept starts at 0 of 4 (all four objectives active again). Revert: delete chain 10122; the abandon case leaves the giver unbound and fails.
3. `castle_side_defense_q3_refusals`: wrong world, 10310 not completed, already active and already completed each fire nothing. Revert: drop `mission_status 10310 completed` from 10123; the 10310-not-completed case accepts and fails.
4. `castle_side_defense_q3_forged_close_is_inert`: a forged `button -1` and a stale `initialResponse` cause zero 10312 actions. Revert: add a `dialog_choice` chain on 10312; the test fails.
5. `castle_side_defense_q3_siblings_disjoint`: no event matches two 10312 chains whose conditions both hold. Revert: add the trigger `entity_dead_tag Castle_BravoOfficer1` to chain 10126; a fresh kill matches 10125 and 10126 and the test fails.
6. `castle_side_defense_q3_two_player_credit`: two players in range of one kill; the killer's objective completes, the other's stays active; no chain here despawns or retags (assert no `despawn_entity` action in 10120-10129). Revert: add a `despawn_entity` action to chain 10125; the static scan fails.
7. `castle_side_defense_q3_shared_slot_188_disjoint`: `n/a` (the giver is slot 421). Replaced by `castle_side_defense_q3_giver_clearance`: a live-DB check that no idle hostile in world 8 reaches spawn 14096 (aggro radius, default 18, plus 5 m), the same method as `live_db_castle_hub`. Revert: move the spawn to (955, 25, 475); fails.
8. `live_db_castle_side_defense_q3_round_trip`: accept, two kills, relog, two kills, complete, all through the production cell-to-base dispatch to `sgw_mission` (two characters, exact sentinel ids, cleaned by exact id). Revert: omit the `MissionUpdate` on the fourth completion; the reload shows 3 of 4 and the test fails.

Regression filters each lane adds, one call each: `test(mission_706) | test(mission_708)` (the 708 chains 1346-1351 share the four tags; `castle_706_708` is only a file-name stem, so `test(castle_706_708)` selects nothing), `test(mission_1520)`, `test(castle_side_medical_q1)`, `test(cooked_sync)`, `test(mission_addition)`, `test(dialog_overrides) | test(override_seed_agreement)`, `live-db-test.ps1 live_db_content_loaders -p cimmeria-cell-catalog`, `live_db_castle_population`, `live_db_castle_standoff`, `live_db_castle_hub` (they filter by explicit spawn id lists, so 14096 must not change their counts), and the seed linters Q1 ran. If the later build ships Q1's `live_db_castle_` filters, run the `castle_` filter too.

## End-UAT rows

All `UATPending (end UAT)`, listed in `uat/end-uat.md` at the end. An observed blank dialog, crash, lost progress or missing marker blocks the packet.

| Row | Check |
|---|---|
| DQ3-U1 journal | after accept, the journal shows *Hold the Bravo Route*, its history text, the step text and the four objectives |
| DQ3-U2 offer dialog | dialog 10717 renders its text with no buttons, closes without a crash, no blank panel |
| DQ3-U3 marker | the giver shows the first-press marker once 10310 is complete, and one click accepts on the first press |
| DQ3-U4 per-kill tick | each kill ticks its own objective in the journal with no reload; killing the same officer again after respawn changes nothing |
| DQ3-U5 completion | the fourth distinct kill completes the mission; the journal marks it done; dialog 10718 renders; the marker does not return |
| DQ3-U6 relog | relog at 2 of 4 keeps the two ticks and the other two still count; relog before accept keeps the marker; relog after completion shows none |
| DQ3-U7 abandon | abandon at 2 of 4 returns the marker; re-accept starts at 0 of 4 |
| DQ3-U8 second character | a second character is unaffected by the first character's kills |
| DQ3-U9 route | the giver (790, 530) to the officers is walkable on foot with no unexpected aggro; Muelbach (y = 48) is reachable; the `nav_path` leg arrival to giver and giver to officer 1 and Muelbach is checked on a running server (CD-9) |
| DQ3-U10 shared actors | after a kill, the other player still sees the officer die and respawn normally; nothing is despawned or retagged; the arrival giver shows the "Sgt. Stanton" name and the courtyard Stanton is unaffected |
| DQ3-U11 races | click twice quickly; click out of range (chat line appears); kill two officers in one volley |
| DQ3-U12 officer 2 island | officer 2 (965, 25, 485) is on navmesh component 214, which has no walkable link to 116: confirm a player can damage and kill it from 116 (line of sight and range), and that it dies and respawns normally; if it cannot be killed, the quest cannot complete and the packet is blocked |
