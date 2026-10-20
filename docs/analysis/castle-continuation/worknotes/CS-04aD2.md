# CS-04aD2: medical_q2 (Overlooked in the Cells), design seal

> **Type:** Reference. **Audience:** Castle orchestrator, CS-04a2 s/d/c1/c2 builders and reviewers.
> **Status:** sealed (mission leg by agent 1, spawn leg and verdict by agent 2).
> **Base:** `2fad73404`. Model: `worknotes/CS-04aD.md` §2 (Q1, `medical_q1`).
> **Labels:** `REPO` (read from the tree), `CIMMERIA_AUTHORING` (authored here), `OPEN` (not proven).

## Verdict

- medical_q2: SEALED. Frozen (e) closed: template 417, spawn 13806 at (270.0, 66.98, 1050.0) tag Castle_Pop_Prisoner5, nav component 250 (h 0.00 m, exit 0 with medic 198 and prisoner 200), 23.5 m or more from every hostile (aggro + 5 satisfied). One OPEN: assist-chase of the interior hostiles, not on the quest path.

## Premise (CIMMERIA_AUTHORING)

After the medic's triage rounds (Q1, mission 10300) one prisoner in the Castle cells was missed in the count. Once 10300 is completed, a new prisoner in the cells (template 417, spawn 13806, tag `Castle_Pop_Prisoner5`) carries the offer marker. Clicking him accepts *Overlooked in the Cells* (10301), shows a one-screen monologue and moves the journal to step B, "report to the medic". The Castle medic (template 177, spawn 198, tag `Castle_Pop_Infirmary_Medic1`, the same NPC as Q1) now carries a turn-in marker; clicking the medic shows a second monologue and completes the mission. Zero reward, like Q1. A dedicated template is used so the glyph paints exactly one NPC (CS-04aD.md :328, G9).

Facts it relies on (`REPO`, base `2fad73404`):

- Q1 is in the tree: mission 10300 rows (`missions.sql:2112`), chains 10000-10006 in `castle_side_stories_chains.sql`, dialog 10700, set 10760, map 1000001. Q1's medic chain 10003 is `interact_tag Castle_Pop_Infirmary_Medic1` gated on 704 completed and 10300 `not_active`.
- Prisoner spawns 200-203 are template 176, tags `Castle_Pop_Prisoner1..4` (`spawnlist.sql:629-632`). `live_db_castle_population.rs:88` exempts the `Castle_Pop_Prisoner` prefix from the free-floating guard rule, so `Castle_Pop_Prisoner5` keeps that exemption.
- `interact_tag` keys on `spawnlist.tag` (`event_dispatch/interaction.rs:24`); `add_dialog_set` keys on the template slot (`slot` = template id, G9).
- `complete_objective` auto-completes the mission when every required objective is done (memory `advance-step-vs-complete-objective`; Q1 chain 10005 relies on it). `advance_step` leaves a step without that auto-complete.
- 10301 is `CanFail=false`, `NumRepeats=0`: statuses are `not_active`, `active`, `completed` only.

Not proven here (`UATPending (end UAT, D-CA11)`): the first-press marker on the prisoner and on the medic at B, that the cells are reachable on foot from the arrival with no hostile aggro on the route, relog behaviour at B.

## medical_q2 IDs

Subset of the D-CA20 row (`r4-dispatch-plan.md` §4).

| Namespace | Used | Reserved, unused |
|---|---|---|
| Mission | 10301 | |
| Steps | 10342 (A, find the prisoner), 10343 (B, report) | 10344-10347 |
| Objectives | 10462 (A), 10463 (B) | 10464-10467 |
| Tasks | 10582, 10583 (type 1) | 10584-10587 |
| Dialogs | 10701 (prisoner, on accept), 10702 (medic, on turn-in) | 10703-10704 |
| Dialog sets | 10761 (prisoner offer), 10762 (medic turn-in) | |
| Screens | 10801 (of 10701), 10802 (of 10702) | 10803-10808 |
| Buttons | none | 10920-10923 (G5) |
| Chains | 10010-10017 | 10018-10019 |
| Set maps | 1000002 (set 10761), 1000003 (set 10762) | |
| Template | 417 (agent 2) | |
| Spawn | 13806 (agent 2), tag `Castle_Pop_Prisoner5` | |

No text rows, no event set, no sequence. Prerequisite: 10300 completed.

## Collision check

Command: the §5 command with the medical_q2 regex `10301|1034[2-7]|1046[2-7]|1058[2-7]|1070[1-4]|1076[12]|1080[1-8]|1092[0-3]|1001[0-9]|100000[23]` over `db/resources/Missions`, `db/resources/Dialogs`, `db/resources/Content`, base `2fad73404`. Output (pasted, every ref returns no hit):

```text
== 2fad73404
== content/cellblock-campaign
== content/harset-campaign
== content/tollana-campaign
== origin/main
```

Templates 417 and spawn 13806 on `db/resources/Entities` and `db/resources/Worlds` with `VALUES \((417|13806),`: the only hits on all five refs are in `blueprints.sql:633` and `blueprints_components.sql:3119-3123` (blueprint ids, a different table). Restricted to the two tables that matter, `git grep -E "(entity_templates|spawnlist) .*VALUES \((417|13806),"` returns no line on any of the five refs. Tag `Castle_Pop_Prisoner5` appears nowhere in `2fad73404`.

## Frozen (a) XML

File `crates/resources/src/base/mission_additions_castle.rs`, `pub const CASTLE_MEDICAL_Q2_XML: &str = r#"..."#;` (no `"#` inside), one line, no newline, registered as `MissionAddition { mission_id: 10301, xml: CASTLE_MEDICAL_Q2_XML }` after the 10300 entry in `MISSION_ADDITIONS` (ascending). Attribute shape copied from Q1 (`CASTLE_MEDICAL_Q1_XML`): same namespaces, flags and element order; changed: `MissionDefn`, `MissionID`, `HistoryText`, step, objective and task ids and texts.

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="3" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Overlooked in the Cells" MissionID="10301"><HistoryText>One prisoner in the Castle cells was missed in the triage count. The medic needs to hear about it.</HistoryText><Steps StepEnabled="false" StepID="10342" AwardXP="false" Difficulty="1"><StepDisplayLogText>Find the overlooked prisoner in the Castle cells.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10462" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak with the overlooked prisoner.</DisplayLogText><Tasks TaskType="1" TaskID="10582" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="10343" AwardXP="false" Difficulty="1"><StepDisplayLogText>Report the overlooked prisoner to the Castle medic.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10463" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Report to the medic in the Castle Infirmary.</DisplayLogText><Tasks TaskType="1" TaskID="10583" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

- **Byte count: 1,732** UTF-8 bytes (`[IO.File]::ReadAllBytes(<scratch file>).Length`, no BOM, no newline). Limit `MAX_MISSION_XML_BYTES` = 355,840; margin 354,108.
- **SHA-256:** `da3378e3b4c42f2ad81ab9dd66f89782c3873dfcf2c711bf0d23c7adfa8a886a`. The scratch file parses as XML with `MissionID` 10301.
- Admission rules met by construction: ids 10301, 10342, 10343, 10462, 10463, 10582, 10583 pairwise unique; `MissionID` equals the registry key; declaration only, no DOCTYPE; child order `HistoryText`, `Steps` (`StepDisplayLogText`, `Objectives` (`DisplayLogText`, `Tasks`)); all ids at most 65535. Steps are declared A then B, matching the `advance_step` order in chain 10014.

## Frozen (b) Missions seeds

Placement: each line goes directly after the corresponding Q1 row (`missions.sql` after the 10300 row; `mission_steps.sql` after step 10341; `mission_objectives.sql` after 10461; `mission_tasks.sql` after 10581), before any trailing `setval`.

```sql
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (10301, 'One prisoner in the Castle cells was missed in the triage count. The medic needs to hear about it.', false, true, false, false, 1, true, true, false, false, false, 3, 'Overlooked in the Cells', 'General', 0, false, false, false, NULL, 0, 0, NULL);
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10342, 10301, false, 1, false, 'Find the overlooked prisoner in the Castle cells.', 0);
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10343, 10301, false, 1, false, 'Report the overlooked prisoner to the Castle medic.', 1);
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10462, 10342, false, 1, false, false, false, 'Speak with the overlooked prisoner.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10463, 10343, false, 1, false, false, false, 'Report to the medic in the Castle Infirmary.');
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10582, 10462, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10583, 10463, false, 1, true, 1);
```

Agreement with the XML: same ids, text, `IsHidden=false`, `IsOptional=false`, `step_enabled`/`is_enabled` false, `award_xp=false`, `can_abandon=true`, `can_fail=false`, `num_repeats=0`, level 3. No `mission_rewards` or `mission_reward_groups` row.

## Frozen (c) Dialogs seeds and override

Both dialogs are one-screen, speaker-0, zero-button monologues (G5, `DUIST_DefaultDialog`, flags 0). The copy contains no apostrophe, so the SQL needs no escaping. Placement: after the Q1 rows (10700 / 10800 / 10760 / 1000001).

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10701, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10702, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10701, 10801, 'The prisoner slumps against the wall, pale and shaking. "Nobody has come down here since the fighting started. I need a medic, please. Tell the one in the infirmary that I am still here."', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10702, 10802, 'The medic nods and pulls a kit from the shelf. "Someone down in the cells, overlooked? I will send help at once. Thank you for telling me."', 0, 0);
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10761, '');
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10762, '');
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000002, 10761, NULL, 'Speak with the Prisoner', 1073741824, 1, '{}', '{}', '{}', '{}');
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000003, 10762, NULL, 'Report to the Medic', 1073741824, 1, '{}', '{}', '{}', '{}');
```

Both maps are NULL-dialog interaction-only flag carriers (Q1 shape, flags 1073741824 = `MISSION_WORLD_OBJECT`). Dialog ids are never reachable from a map, so a click never opens them; only `display_dialog` from a chain does. No `dialog_screen_buttons` rows.

`crates/resources/src/base/dialog_overrides/mod.rs`: append at the END of `DIALOG_OVERRIDES`, after the Q1 entry (10700), two `DIALOG_OVERRIDES` entries (`overrides_castle::MEDICAL_Q2_PRISONER_DIALOG,` and `overrides_castle::MEDICAL_Q2_MEDIC_DIALOG,`) referencing the constants below, which live in `overrides_castle.rs` next to Q1's `pub(super) const MEDICAL_Q1_DIALOG` (same visibility and doc-comment form) (CD-12 budget: one `#[cfg(test)]`, one `mod` and one entry per dialog, 4 lines at most in `mod.rs`). The constants, text byte-equal to the seed rows:

```rust
/// Castle medical_q2 dialog 10701: the overlooked prisoner's monologue, shown by the
/// medical quest after the accept. Frozen in the CS-04aD2 worknote (Frozen (c)).
pub(super) const MEDICAL_Q2_PRISONER_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10701,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10801,
        speaker_id: 0,
        text: "The prisoner slumps against the wall, pale and shaking. \"Nobody has come down here since the fighting started. I need a medic, please. Tell the one in the infirmary that I am still here.\"",
        buttons: &[],
    }],
};

/// Castle medical_q2 dialog 10702: the Medic's reply, shown by the medical quest at the
/// turn-in. Frozen in the CS-04aD2 worknote (Frozen (c)).
pub(super) const MEDICAL_Q2_MEDIC_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10702,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10802,
        speaker_id: 0,
        text: "The medic nods and pulls a kit from the shelf. \"Someone down in the cells, overlooked? I will send help at once. Thank you for telling me.\"",
        buttons: &[],
    }],
};
```

If Q1's override is declared inline in `mod.rs` rather than as a constant, the builder follows that form; the shape and text above are what is frozen. All ids at most 65535 (`every_cooked_override_element_id_fits_in_16_bits`); `shipped_overrides_carry_no_buttons` holds.

## Frozen (d) Chains

Append to `db/resources/Content/Seed/castle_side_stories_chains.sql` after the last Q1 line (chain 10006), one statement per line (interact_tag linter). Scope `mission`, scope id 10301. `slot` is the template id; `target_id` of the dialog-set actions is the map id. Every chain carries `mission_status 10300 completed` so it is provably disjoint from Q1's 10000-10006 (all of which require 10300 `not_active` or `active`) without relying on an invariant.

```sql
-- Castle Q2 medical (CS-04aD2): chains 10010-10017. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10010, '10301 - 10300 completed: offer the prisoner (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10010, 'mission_completed', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10010, 'world', 8, NULL, 'eq', NULL, 0), (10010, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10010, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10010, 'add_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10011, '10301 - Login, offer open: rebind the prisoner (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10011, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10011, 'world', 8, NULL, 'eq', NULL, 0), (10011, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10011, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10011, 'add_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10012, '10301 - Abandoned: rebind the prisoner, unbind the medic (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10012, 'mission_abandoned', '10301', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10012, 'world', 8, NULL, 'eq', NULL, 0), (10012, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10012, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10012, 'add_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 0), (10012, 'remove_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10013, '10301 - Prisoner click: accept (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10013, 'interact_tag', 'Castle_Pop_Prisoner5', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10013, 'world', 8, NULL, 'eq', NULL, 0), (10013, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10013, 'mission_status', 10301, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10013, 'accept_mission', 10301, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10014, '10301 - Accepted: advance to B, swap markers, show 10701 (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10014, 'mission_accepted', '10301', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10014, 'world', 8, NULL, 'eq', NULL, 0), (10014, 'step_status', 10301, '10342', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10014, 'advance_step', 10301, '10343', '{}', 0, 0), (10014, 'remove_dialog_set', 1000002, NULL, '{"slot": 417}', 0, 1), (10014, 'add_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 2), (10014, 'display_dialog', 10701, NULL, '{}', 0, 3);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10015, '10301 - Medic click at B: unbind, show 10702, complete (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10015, 'interact_tag', 'Castle_Pop_Infirmary_Medic1', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10015, 'world', 8, NULL, 'eq', NULL, 0), (10015, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10015, 'mission_status', 10301, NULL, 'eq', 'active', 2), (10015, 'step_status', 10301, '10343', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10015, 'remove_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 0), (10015, 'display_dialog', 10702, NULL, '{}', 0, 1), (10015, 'complete_objective', 10301, '10463', '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10016, '10301 - Login stuck at A: heal to B, bind the medic (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10016, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10016, 'world', 8, NULL, 'eq', NULL, 0), (10016, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10016, 'mission_status', 10301, NULL, 'eq', 'active', 2), (10016, 'step_status', 10301, '10342', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10016, 'advance_step', 10301, '10343', '{}', 0, 0), (10016, 'add_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 1);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10017, '10301 - Login at B: rebind the medic (CIMMERIA_AUTHORING)', 'mission', 10301, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10017, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10017, 'world', 8, NULL, 'eq', NULL, 0), (10017, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10017, 'mission_status', 10301, NULL, 'eq', 'active', 2), (10017, 'step_status', 10301, '10343', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10017, 'add_dialog_set', 1000003, NULL, '{"slot": 177}', 0, 0);
```

Why this shape (the traps, applied):

- **Sibling disjointness (trap: one event fires every matching chain).** Per event key, chains that can both resolve are pairwise disjoint on the (10300, 10301, step) state:
  - `player_loaded Castle`: 10011 (10301 `not_active`), 10016 (active, step 10342), 10017 (active, step 10343). Q1's 10001 (10300 `not_active`) and 10006 (10300 `active`) cannot hold with 10300 `completed`. The 701-708 and 1520 relog chains gate on their own missions.
  - `interact_tag Castle_Pop_Infirmary_Medic1`: Q1's 10003 needs 10300 `not_active`; 10015 needs 10300 `completed`. Different values of the same mission status, so never both. The c2 test asserts this over all nine (10300, 10301) pairs.
  - `interact_tag Castle_Pop_Prisoner5`: 10013 alone. `mission_completed 10300`: 10010 alone. `mission_abandoned 10301`: 10012 alone. `mission_accepted 10301`: 10014 alone.
- **The dialog-set slot.** Slot 177 carries Q1's map 1000001 only while 10300 is `not_active` (bound by 10000-10002, removed by 10004) and Q2's 1000003 only while 10300 is `completed` and 10301 is `active` at B. The two never coexist in one player's `available_interactions[177]`.
- **One interact_tag chain per tag and state.** A click when no chain resolves (for example the medic after completion) falls through to the normal dialog path; no mission action.
- **Accept gates the offer; the advance is nested.** 10013 accepts only. The A-to-B advance, the marker swap and the dialog run in 10014 on the real `mission_accepted` transition, so a repeat click cannot replay it (10013 requires `not_active`).
- **Action order (trap: step-status timing, nested dispatch).** Conditions freeze at resolve time. In 10014 `advance_step` goes first, then the bind swap, then the dialog. In 10015 the completion is last: `complete_objective` can auto-complete the mission and dispatch the nested `mission_completed 10301` event mid action list, so the unbind and the dialog run first. No chain listens on `mission_completed 10301`.
- **No `dialog_choice` chain.** Both dialogs are buttonless; a forged choice or close has no chain.
- **Completion.** B has one required objective, so `complete_objective 10463` completes the mission; `advance_step` is used only to leave A.
- **Once only.** After completion `mission_status 10301 not_active` is false for 10010-10013, `num_repeats = 0`.
- **Failure of the nested accept.** If `accept_mission` is refused (already active), 10014 never fires and the prisoner shows nothing new; the click still gets the generic reply.
- **Open (OPEN):** whether a second `add_dialog_set` on slot 177 (Q1's, then Q2's) replaces or stacks in the client's interaction list. Not reachable here (the two never coexist); end-UAT row MQ2-U4 observes it.

## Frozen (e) Templates and spawns

Agent 2's leg. Labels: `REPO` (read from the tree at base `2fad73404`), `CIMMERIA_AUTHORING` (authored here), `OPEN` (not proven).

### Template 417 (CIMMERIA_AUTHORING)

Copy of template 176 (`REPO`: `entity_templates.sql:552`). Only two things change: `template_id` and `template_name`. `name_id` stays 7552 ("Op-CORE Soldier"): `name_id` resolves to client-shipped text only and beats `entity_templates.name` on a mob plate (agent memory `debug-area-npc-lineup-da10.md`), and no free client moniker exists for a new prisoner name, so a new `name_id` would draw a blank plate. `name` is NULL in 176 and stays NULL. The tag lives on the spawn, not the template. Faction stays 1 (friendly, not damageable by players), `has_dynamic_properties` stays true (the interaction update needs it), no ability set, loot, patrol or wander. Insert after the last `entity_templates` row (line 1283, template 400) and before the sequence comment block (line 1285). The highest id at base is 415 (hub quartermaster); 411-415 are other packets' rows, 416 is the 567 cabinet and 418 is reserved, so 417 is free:

```sql
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, aggro_radius, assist_radius, wander_radius, wander_min_dwell_secs, wander_max_dwell_secs) VALUES (417, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_02,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00}', 0, 0, 570, 4, 0, 1, 7552, NULL, NULL, NULL, 'Castle_Overlooked_Prisoner', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, -256, -256, -52773120, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
```

### Spawn 13806 (CIMMERIA_AUTHORING)

World 8, `(270.0, 66.98, 1050.0)`, heading 3.141593 (same as prisoners 201/202), tag `Castle_Pop_Prisoner5`, no respawn, not stationary-flagged (like 200-203). It stands 14.1 m west of prisoner 201 `(284, 66.98, 1048)` and 36.1 m east of prisoner 200, in the same cell run, so a player entering the cells to see 200-203 sees it. Insert after spawn 203 in `spawnlist.sql` (line 632, 9-column form):

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (13806, 270.0, 66.98, 1050.0, 3.141593, 8, 417, 'Castle_Pop_Prisoner5', NULL);
```

Candidates probed first (all component 250, h at most 0.40 m): `(278,1048)`, `(287,1042)`, `(284,1036)`, `(260,1048)`, `(270,1050)`. `(270, 1050)` won on clearance from the interior hostiles and distance from the 201/202 click targets (the other candidates sit 3-9 m from a stock prisoner, where a mis-click picks the wrong NPC).

### Nav proof (`nav_inspect`, offline, REPO)

Run through the lane at base `2fad73404` on the tracked `data/spaces/castle.nav` (`status=ok exit=0`), probe file kept outside the repo:

```text
pwsh tools/build-lane/lane.ps1 cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/castle.nav --quiet --probes <q2-final.txt>
components  549 (total walkable XZ area 814506.8 m^2)
probes (h-tol 2.00 m, v-tol 3.00 m)
  spawn13806           poly=17399  component=250  h=0.00 m  dy=-0.00 m  ok
  medic198             poly=14230  component=250  h=0.00 m  dy=-0.01 m  ok
  prisoner200          poly=16872  component=250  h=0.00 m  dy=-0.00 m  ok
```

Spawn 13806 is component 250, h 0.00 m (limit 2 m), exit 0 for the set, and shares the component with the medic (198) and prisoner 200. Both turn-in ends of the quest are reachable on one component.

### Hostile aggro clearance (the `live_db_castle_population` rule, REPO)

`live_db_castle_population.rs` (`FRIENDLY_CLEARANCE = 5.0`, `DEFAULT_AGGRO_RADIUS = 18.0`, `AGGRO_VERTICAL_BAND = 4.0`): a population friendly must stand more than `aggro_radius + 5` (plus wander and patrol reach) from every faction-10 hostile idle position inside the 4 m height band. Caged `Castle_Pop_Prisoner*` tags are exempt (`CAGED_PREFIX`, line 88), and `Castle_Pop_Prisoner5` keeps that exemption, so the guard cannot fail it. It is checked anyway, because the player stands at the prisoner to click:

| Hostile (faction 10, world 8) | Position | aggro | Needed (aggro + 5) | Distance to 13806 | Result |
|---|---|---|---|---|---|
| 255 IntBlock_1 (t182) | (248, 67.18, 1032.5) | 15 | 20 | 28.1 | clear (+8.1) |
| 256 IntBlock_2 (t181) | (249, 67.18, 1039.5) | 15 | 20 | 23.5 | clear (+3.5) |
| 240 Castle_Romney (t169, faction 10) | (244, 66.79, 1036) | default 18 (t169 sets none) | 23 | 29.5 | clear (+6.5) |
| 254 HallPRU_1 (t145, patrol 2090) | (256..298, 70.25/70.18, 996) | 18 | 23 | 55.8 at best (256, 996) | clear |
| 252/253 HallPatrolW | (262, 70.18, 929-932) | 15 | 20 | 118+ | clear |

Template 181/182 aggro 15, assist 12 and wander NULL come from the seed (`entity_templates.sql` tail of rows 181/182); 145 has no override so the default 18 applies. The tightest row is IntBlock_2 at +3.5 m beyond the 20 m the rule asks for. A player standing 3 m closer to it (the `PLAYER_STANDOFF` slack) is at 20.5 m, still outside its 15 m aggro. Stock prisoners 203 (236, 1024) and 200 (234, 1048) stand 14.7 m from IntBlock_1 and 17.2 m from IntBlock_2, inside the 20 m rule, and are exempt as caged; 13806 deliberately is not.

`OPEN`: nothing here proves the interior hostiles do not path-chase to 13806 once aroused by a fight elsewhere (assist radius 12 from a pulled neighbour); the pop-up fight is not on the quest path, so this is not a gate.

### Trap checks (agent memory, REPO)

- Faction 0 is an auto-aggro dead zone: 417 is faction 1, not 0.
- The glyph paints every entity of template 417 (`add_dialog_set` keys on the template slot): 417 has exactly one spawn, so one NPC.
- Template 176 is still shared by 199 (Sick1) and 193 (Wounded1) plus 200-203; chains must never bind slot 176 (the chains use slot 417 for the offer and 177 for the turn-in).
- Respawn: none authored (like 200-203), so a killed prisoner does not return. Players cannot damage faction 1, so it cannot be killed by them.

## Frozen (f) Reward

Zero. `missions.reward_naq = reward_xp = 0`, `award_xp=false` at every level, no `mission_rewards` or `mission_reward_groups` row for 10301. No economy sign-off needed. A later packet may add a modest reward without changing ids.

## Behavior

| State (10300 / 10301 / step) | Event | Chain | Result / evidence |
|---|---|---|---|
| 10300 just completed, 10301 `not_active`, world 8 | `mission_completed 10300` | 10010 | prisoner map 1000002 bound; `Content: adding dialog set` |
| completed / `not_active` | click prisoner 13806, in range | 10013 then 10014 | accept; A to B; map 1000002 removed; medic map 1000003 bound; dialog 10701 shown; journal shows B |
| completed / `not_active` | click out of range | none | `interaction.out_of_range` row, generic chat line |
| any other 10300 state | click prisoner | none | `condition_failed` on 10013 |
| completed / active / B | click prisoner again | none | 10013 needs `not_active`; no replay |
| completed / active / B | click medic, in range | 10015 | map 1000003 removed; dialog 10702 shown; objective 10463 done; mission completed |
| completed / active / A (not reachable normally) | click medic | none | 10015 needs B active |
| completed / completed | click prisoner or medic | none | no chain; no marker |
| `not_active` or `active` 10300 | click medic | Q1's 10003 only | 10015 refused on `mission_status 10300 completed` |
| completed / `not_active`, relog | `player_loaded Castle` | 10011 | prisoner map rebound |
| completed / active / A, relog | `player_loaded Castle` | 10016 | heals to B, medic bound |
| completed / active / B, relog | `player_loaded Castle` | 10017 | medic bound |
| completed / completed, relog | `player_loaded Castle` | none | nothing bound |
| active at A or B | abandon | 10012 | prisoner rebound, medic unbound |
| wrong world | any | none | `world 8` condition fails |
| forged dialog close or button, stale `initialResponse` | `dialog_choice` | none | zero 10301 action |
| two characters on one account | any | | independent; `available_interactions` and `sgw_mission` are per character |

Every click that matches a chain gives visible feedback on the first press: accept shows 10701, turn-in shows 10702; a click that matches no chain falls through to the normal interaction reply.

## Test list

Module `castle_side_medical_q2` in each crate that hosts the test. Each test gives name, assertion and revert proof.

### s

Catalog, seeds and the template/spawn (worktree `castle-cs-04ad2`).

1. `castle_side_medical_q2_xml_admitted`: `CASTLE_MEDICAL_Q2_XML` passes `apply_mission_additions::prepare`; 1,732 bytes, one line, at most `MAX_MISSION_XML_BYTES`. Revert: change one byte or add a newline.
2. `castle_side_medical_q2_ids_unique_and_in_block`: ids 10301, 10342/10343, 10462/10463, 10582/10583 pairwise unique, inside the D-CA20 row, none a stock key. Revert: set a task id to 10581 (Q1's).
3. `castle_side_medical_q2_xml_matches_seed`: parse the XML and compare ids, flags, text, `IsHidden`/`IsOptional` with the mission, step, objective and task seed rows. Revert: edit one seed text.
4. `castle_side_medical_q2_registers_addition`: look the 10301 entry up by id in `MISSION_ADDITIONS` (CS-R4P shape, not by index); the registry stays ascending and unique. Revert: drop the entry or put it before 10300.
5. `live_db_castle_side_medical_q2_seed_loads`: after `reload-db.ps1` the loader returns 10301 with steps 10342 then 10343 (by `index`), objectives 10462/10463, tasks 10582/10583. Revert: change a seed id.
6. `castle_side_medical_q2_no_reward_rows`: no `mission_rewards` or `mission_reward_groups` row for 10301; `reward_naq = reward_xp = 0`. Revert: add a reward row.
7. `live_db_castle_side_medical_q2_prisoner_seeded`: template 417 equals template 176 except `template_id` and `template_name`; spawn 13806 is world 8, template 417, tag `Castle_Pop_Prisoner5`, the only spawn of template 417, tag unique across spawns. Revert: point 13806 at template 176.

Regression filters the s lane adds: `castle_side_medical_q2` (resources and cell-catalog), `mission_additions`, `cooked_sync` (G14). CS-R4P (review fix) already makes `fixture_is_exact_stock_transform_and_ids_are_test_only` and `castle_side_medical_q1_is_registered_once` id-based; the s lane must be based on that tip or `mission_additions` fails on the second registry entry.

### d

Dialogs and overrides (worktree `castle-cs-04ad2d`, parallel).

1. `castle_side_medical_q2_dialog_override_matches_seed`: overrides 10701/10801 and 10702/10802 equal the `dialogs`/`dialog_screens` rows byte for byte (text, speaker 0, `ui_screen_type 2`, zero buttons); all element ids at most 65535. Revert: edit one override character.
2. `castle_side_medical_q2_set_maps_are_flag_carriers`: maps 1000002/1000003 have NULL dialog, flags 1073741824, sets 10761/10762 exist; no `dialog_screen_buttons` row for 10701/10702. Revert: give a map a dialog id.
3. `castle_side_medical_q2_dialogs_not_reachable_from_maps`: no map or set routes to 10701 or 10702, so they appear only through `display_dialog`. Revert: add a dialog id to a map.

Regression filters: `dialog_overrides`, `shipped_overrides_carry_no_buttons`, `every_cooked_override_element_id_fits_in_16_bits`, `cooked_sync`.

### c1

Chains and replay (worktree `castle-cs-04ad2`, after the s and d reviews).

1. `castle_side_medical_q2_chains_load`: chains 10010-10017 load with the exact triggers, conditions and action order above. Revert: delete a chain or reorder an action.
2. `castle_side_medical_q2_prisoner_click_accepts_and_shows_dialog`: replay the tag click through the real executor; assert accept, A to B, map 1000002 removed, 1000003 bound on slot 177, dialog 10701 emitted, in that order. Revert: remove the nested `advance_step` or the display action.
3. `castle_side_medical_q2_medic_click_completes_only_at_b`: the medic click at B unbinds, shows 10702 and completes 10301; at A, `not_active` or `completed` it does nothing. Revert: drop the step gate.
4. `castle_side_medical_q2_offer_needs_10300_completed`: the prisoner click and 10010/10011 refuse with 10300 `not_active` or `active`. Revert: drop the 10300 condition.
5. `castle_side_medical_q2_repeat_click_noop`: a second prisoner click after accept fires nothing; a second medic click after completion fires nothing. Revert: relax `not_active`.

Regression filters: `castle_side_medical_q2`, `castle_side_medical_q1` (Q1 must stay green), `chains_load`, the interact_tag linter.

### c2

Disjointness, relog, refusals, persistence (same worktree, on the c1 tip).

1. `castle_side_medical_q2_siblings_disjoint_with_q1`: over every (10300, 10301) status pair in {not_active, active, completed} x {not_active, active, completed} and every 10301 step, resolve `interact_tag Castle_Pop_Infirmary_Medic1` and `player_loaded Castle`; the union of the Q1 chains (10000-10006) and 10010-10017 never resolves two chains of different quests, and within Q2 never two at once. Also assert the slot-177 binding invariant: map 1000001 and map 1000003 are never added for the same state. Revert: remove `mission_status 10300 completed` from 10015.
2. `castle_side_medical_q2_relog_each_state`: not_active rebinds (10011); A heals and binds the medic (10016); B binds the medic (10017); completed binds nothing. Revert: drop each chain in turn.
3. `castle_side_medical_q2_abandon_rebinds`: abandon at A and B fires 10012 (prisoner bound, medic unbound); abandon after completion cannot occur. Revert: drop the remove action.
4. `castle_side_medical_q2_wrong_world_refused`: every Q2 chain refuses outside world 8. Revert: drop a `world` condition.
5. `castle_side_medical_q2_forged_close_is_inert`: forged `button -1` and a stale `initialResponse` cause zero 10301 actions. Revert: add a `dialog_choice` chain.
6. `castle_side_medical_q2_out_of_range_click_refused`: the real `interact` method, far prisoner and far medic targets, `interaction.out_of_range` row. Revert: remove the range gate in the test double.
7. `live_db_castle_side_medical_q2_round_trip`: accept, advance, complete and abandon persist through the production cell-to-base dispatch to `sgw_mission` and reload per character (two characters, exact sentinel ids, cleaned by exact id). Revert: skip persistence of the step change.

Regression filters: `castle_side_medical_q1`, `castle_side_medical_q2`, `mission_1520`, `live_db_castle_population` (the new spawn must satisfy it).

## End-UAT rows

All `UATPending (end UAT, D-CA11)`.

| Row | Check |
|---|---|
| MQ2-U1 journal | after accept the journal shows *Overlooked in the Cells*, the history text and step B "Report the overlooked prisoner to the Castle medic." |
| MQ2-U2 dialogs | 10701 and 10702 render on a single screen with no buttons, close without a crash, no blank panel |
| MQ2-U3 prisoner marker | after Q1 completes, exactly one prisoner shows the first-press marker (the other cell prisoners do not); clicking accepts on the first press |
| MQ2-U4 medic marker | at B the medic shows the turn-in marker; Q1's marker never reappears on the medic (slot 177 swap) |
| MQ2-U5 complete | the medic click shows 10702, completes the mission, the journal marks it done, the marker does not return |
| MQ2-U6 relog | relog at not-active (prisoner marker returns), at B (medic marker returns), after completion (no marker) |
| MQ2-U7 abandon | abandon at B returns the prisoner marker and removes the medic marker; re-accept works; a second character is unaffected |
| MQ2-U8 route | the cells are reachable on foot from the Infirmary with no hostile aggro on the route; the prisoner stands where placed (agent 2's nav proof is static) |
| MQ2-U9 races | click the prisoner twice quickly; click out of range (chat line appears); click the medic before accepting (Q1 reply or nothing, no Q2 effect) |
