# Cellblock continuation: wave 5 frozen content (CB-N06)

> **Type:** Reference (frozen literals for the W5 packets)
> **Audience:** The CB-N06 seal advisors, packet coders and their reviewers
> **Last updated:** 2026-10-17
> **Companions:** [w5-packets.md](w5-packets.md) (packets, rules), [w3-frozen.md](w3-frozen.md) (the CB-N05 pattern these literals follow), [id-allocation.md § CB-N06](../four-zone-planning/id-allocation.md#cb-n06-claims--2026-10-17), [README.md](README.md) (D-CC18, D-CC19)

Every literal below is frozen. Copy it byte for byte. Do not reword, reflow, re-escape or "fix" anything. If a loader, validator or emitter refuses a literal, or produces bytes that differ from it, stop and report `blocked` with both strings; never edit the literal to make a test pass. Seed comments carry `CIMMERIA_AUTHORING`; player-visible text never does.

**Provenance (D-CC18).** Mission 62060 is new Cimmeria content, not 2009 content. The owner included CB-N06 on 2026-10-17 and delegated the quest and prop choice; the W5 architect pre-chose them from static evidence. The seal packet CB-N06s verifies every claim below before any build packet runs; a `change` verdict edits this file (architect pass) and nothing else.

## Design summary

*Before the Patrol Returns* (mission 62060) is optional and one-time. After stock 638, the two friendly Op-CORE prisoners of the CB-N04 ambient cast (spawns 520 and 521, template 176) offer it. One click on either prisoner accepts it; dialog 60151 explains the choice and the `329_CellDoorButton` panel beside Prisoner 329 becomes clickable. Exactly one branch then completes the mission:

| Branch | Player action | Effect |
|---|---|---|
| Aid | Click either prisoner again | Two Health Slappacks (item 2893), objective 62062 completes, mission completes, both choices unbind |
| Sabotage | Click the `329_CellDoorButton` panel | Same reward and completion, same unbind |
| Exit | Ignore the offer, or abandon from the journal | Nothing is lost; the offer stays (or returns) until 641 is accepted |

Reward: two Health Slappacks only (`add_item`, qty 2). No XP (`grant_xp` has no production seed precedent), no naquadah. The core route, Prisoner 329's dialog-set 2795, Marsh's chains and the CB-N07a bark are untouched.

## F1. Mission 62060 category-3 XML (CB-N06-1)

Exactly **1369 bytes** UTF-8, one LF after the declaration, no trailing newline. One cooked-sync fragment (`FRAG_FIRST_AND_LAST`; the limit is 1390). Architect SHA-256: `E7F3F1371CA764DB60E2E084E68DF296766A0871A03EE2CB7D9C253063A20439` (a reviewer cross-check only).

In `crates/resources/src/base/mission_additions.rs`, insert this block directly after the `REMAINING_PODS_XML` const (after its closing `"#;` line and one blank line):

```rust
/// CB-N06 *Before the Patrol Returns* (CIMMERIA_AUTHORING): the exact
/// category-3 entry for mission 62060, 1369 bytes, frozen in
/// docs/analysis/cellblock-continuation/w5-frozen.md (F1).
pub const PRISONER_CHOICE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Before the Patrol Returns" MissionID="62060"><HistoryText>Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.</HistoryText><Steps StepEnabled="false" StepID="62061" AwardXP="false" Difficulty="1"><StepDisplayLogText>Help the prisoners hide, or jam the cell-door panel.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62062" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Help the prisoners or jam the panel.</DisplayLogText><Tasks TaskType="1" TaskID="62063" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;
```

Replace the registry and its doc comment:

```rust
/// Production authored entries: CB-N05's mission 62050 only.
pub const MISSION_ADDITIONS: &[MissionAddition] = &[MissionAddition {
    mission_id: 62050,
    xml: REMAINING_PODS_XML,
}];
```

with:

```rust
/// Production authored entries: CB-N05's 62050 and CB-N06's 62060.
pub const MISSION_ADDITIONS: &[MissionAddition] = &[
    MissionAddition {
        mission_id: 62050,
        xml: REMAINING_PODS_XML,
    },
    MissionAddition {
        mission_id: 62060,
        xml: PRISONER_CHOICE_XML,
    },
];
```

Change line 2 of the module to: `//! Production entries are added only by a mission packet that sealed its XML (CB-N05: 62050; CB-N06: 62060).`

## F2. Mission seed rows (CB-N06-1)

The four files are stored `i/lf w/crlf` and use one blank line between rows. In each file, insert the block directly after the **last** `INSERT` line (the CB-N05 62050 row), with one blank line before the comment; keep whatever followed that line.

`db/resources/Missions/Seed/missions.sql`:

```sql
-- Cellblock continuation CB-N06 (CIMMERIA_AUTHORING): Before the Patrol Returns, optional one-time choice after 638 (new Cimmeria content, D-CC18). Literals: docs/analysis/cellblock-continuation/w5-frozen.md.
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (62060, 'Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.', false, true, false, false, 1, true, true, false, false, false, 1, 'Before the Patrol Returns', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`db/resources/Missions/Seed/mission_steps.sql`:

```sql
-- Cellblock continuation CB-N06 (CIMMERIA_AUTHORING): mission 62060 step.
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (62061, 62060, false, 1, false, 'Help the prisoners hide, or jam the cell-door panel.', 0);
```

`db/resources/Missions/Seed/mission_objectives.sql`:

```sql
-- Cellblock continuation CB-N06 (CIMMERIA_AUTHORING): mission 62060 objective.
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (62062, 62061, false, 1, false, false, false, 'Help the prisoners or jam the panel.');
```

`db/resources/Missions/Seed/mission_tasks.sql`:

```sql
-- Cellblock continuation CB-N06 (CIMMERIA_AUTHORING): mission 62060 task.
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (62063, 62062, false, 1, true, 1);
```

No `mission_rewards` or `mission_reward_groups` rows: the reward is the chain's `add_item`, not a mission reward.

## F3. Dialog 60151 (CB-N06-2)

Emitted bytes, exactly **392 bytes**, no newline anywhere (the 60150 shape). Architect SHA-256: `5E6316F9B3A9BCA2414A8684005E2F1930128DF43DAD4B174C8DE84796EE59E7`.

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_DIALOG DialogFlags="0" DialogID="60151" KismetEventSetID="0" UIScreenType="2"><Screens SpeakerID="0" ScreenID="96111" Text="The two prisoners are too weak to run, and the patrol is coming back. There is time for one thing: help them hide, or jam the cell-door panel by Prisoner 329 so the guards cannot seal the block."></Screens></COOKED_DIALOG>
```

Append to `crates/resources/src/base/dialog_overrides/overrides_cellblock.rs` (after `REMAINING_PODS_DIALOG`, one blank line before):

```rust
/// Mission 62060 *Before the Patrol Returns* (CB-N06): the choice, shown by
/// chain 11035 after the accept. Closing it (`dialogButtonChoice(60151, -1)`)
/// has no content action. Frozen in w5-frozen.md (F3).
pub(super) const PRISONER_CHOICE_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60151,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 96111,
        speaker_id: 0,
        text: "The two prisoners are too weak to run, and the patrol is coming back. There is time for one thing: help them hide, or jam the cell-door panel by Prisoner 329 so the guards cannot seal the block.",
        buttons: &[],
    }],
};
```

In `crates/resources/src/base/dialog_overrides/mod.rs` (641 lines; exactly **5** added lines, result 646):

- after the two lines `#[cfg(test)]` / `mod override_seed_agreement_cellblock_62050;`, add `#[cfg(test)]` / `mod override_seed_agreement_cellblock_62060;`;
- after `    overrides_cellblock::REMAINING_PODS_DIALOG,` (the last element, before `];`), add:

```rust
    // CB-N06 mission 62060: the patrol choice (dialog 60151), the served
    // one-screen speaker-0 shape. See overrides_cellblock.rs.
    overrides_cellblock::PRISONER_CHOICE_DIALOG,
```

Seed rows, both files `i/lf w/crlf`. Insert directly after the **last** `INSERT` line of each file, with one blank line before the comment; keep the existing tail.

`db/resources/Dialogs/Seed/dialogs.sql`:

```sql
-- Cellblock continuation CB-N06: the patrol choice for mission 62060 (CIMMERIA_AUTHORING).
-- Speaker-0 monologue, served by DIALOG_OVERRIDES (overrides_cellblock.rs).
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (60151, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Cellblock - Before the Patrol Returns: the choice');
```

`db/resources/Dialogs/Seed/dialog_screens.sql`:

```sql
-- Cellblock continuation CB-N06: dialog 60151's only screen (CIMMERIA_AUTHORING).
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60151, 96111, 'The two prisoners are too weak to run, and the patrol is coming back. There is time for one thing: help them hide, or jam the cell-door panel by Prisoner 329 so the guards cannot seal the block.', 0, 0);
```

## F4. Chains 11030-11041 (CB-N06-3)

Append at the **end** of `db/resources/Content/Seed/castle_cellblock_chains.sql` (stored `i/crlf`), after the last block (CB-N07a's chain 11020; CB-N05's 11005 if CB-N07a was omitted), with one blank line before it. Do not touch the top-of-file header (W5-INT owns it, D-CC8). Totals: 12 chains, 12 triggers, 31 conditions, 23 actions; `once` true only for 11031 and 11032.

```sql
-- =====================================================================
-- CB-N06 (2026-10-17): Before the Patrol Returns, mission 62060 (CIMMERIA_AUTHORING)
-- =====================================================================
-- New Cimmeria content, not 2009 content (D-CC18). Optional one-time choice
-- after stock 638. The two ambient prisoners (spawns 520/521, template 176)
-- offer 62060 through the existing interaction-only dialog-set-map 3862 at
-- slot 176 (per player). A validated prisoner click accepts; the nested
-- mission_accepted follow-up binds dialog-set-map 7509 on the
-- 329_CellDoorButton panel (slot 22) and shows monologue 60151. Exactly one
-- branch completes objective 62062: a second prisoner click (aid) or a panel
-- click (sabotage). Both grant two Health Slappacks (item 2893) and unbind
-- both choices. No dialog_choice or initialResponse path. Stock panel chains
-- 1016/1017/1028 and Marsh chains 1051/1052/1152 are unchanged.
-- Literals: docs/analysis/cellblock-continuation/w5-frozen.md (F4).

-- Chain 11030: 638 completed -> offer 62060 at the two prisoners
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11030, 'CB-N06 - 638 completed: offer Before the Patrol Returns at the two prisoners', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11030, 'mission_completed', '638', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11030, 'world', 12, NULL, 'eq', NULL, 0),
  (11030, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11030, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11030, 'mission_status', 62060, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11030, 'add_dialog_set', 3862, NULL, '{"slot": 176}', 0, 0);

-- Chain 11031: login in Castle_CellBlock -> restore the 62060 offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11031, 'CB-N06 - login: restore the Before the Patrol Returns offer', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11031, 'player_loaded', 'Castle_CellBlock', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11031, 'world', 12, NULL, 'eq', NULL, 0),
  (11031, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11031, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11031, 'mission_status', 62060, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11031, 'add_dialog_set', 3862, NULL, '{"slot": 176}', 0, 0);

-- Chain 11032: login while step 62061 is active -> restore both choices
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11032, 'CB-N06 - login during 62061: restore the prisoner and panel choices', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11032, 'player_loaded', 'Castle_CellBlock', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11032, 'world', 12, NULL, 'eq', NULL, 0),
  (11032, 'step_status', 62060, '62061', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (11032, 'add_dialog_set', 3862, NULL, '{"slot": 176}', 0, 0),
  (11032, 'add_dialog_set', 7509, NULL, '{"slot": 22}', 0, 1);

-- Chain 11033: validated click on prisoner 1 -> accept 62060
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11033, 'CB-N06 - interact Cellblock_Ambient_Prisoner1: accept Before the Patrol Returns', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11033, 'interact_tag', 'Cellblock_Ambient_Prisoner1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11033, 'world', 12, NULL, 'eq', NULL, 0),
  (11033, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11033, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11033, 'mission_status', 62060, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11033, 'accept_mission', 62060, NULL, '{}', 0, 0);

-- Chain 11034: validated click on prisoner 2 -> accept 62060
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11034, 'CB-N06 - interact Cellblock_Ambient_Prisoner2: accept Before the Patrol Returns', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11034, 'interact_tag', 'Cellblock_Ambient_Prisoner2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11034, 'world', 12, NULL, 'eq', NULL, 0),
  (11034, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11034, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11034, 'mission_status', 62060, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11034, 'accept_mission', 62060, NULL, '{}', 0, 0);

-- Chain 11035: nested mission_accepted 62060 -> bind the panel choice, show the choice
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11035, 'CB-N06 - 62060 accepted: bind the cell-door panel choice, show dialog 60151', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11035, 'mission_accepted', '62060', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11035, 'world', 12, NULL, 'eq', NULL, 0),
  (11035, 'step_status', 62060, '62061', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (11035, 'add_dialog_set', 7509, NULL, '{"slot": 22}', 0, 0),
  (11035, 'display_dialog', 60151, NULL, '{}', 0, 1);

-- Chain 11036: aid branch, prisoner 1 clicked during 62061
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11036, 'CB-N06 - interact Cellblock_Ambient_Prisoner1 during 62061: aid, grant 2 slappacks, complete 62062', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11036, 'interact_tag', 'Cellblock_Ambient_Prisoner1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11036, 'world', 12, NULL, 'eq', NULL, 0),
  (11036, 'step_status', 62060, '62061', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (11036, 'add_item', 2893, NULL, '{"container": 0, "qty": 2}', 0, 0),
  (11036, 'remove_dialog_set', 7509, NULL, '{"slot": 22}', 0, 1),
  (11036, 'remove_dialog_set', 3862, NULL, '{"slot": 176}', 0, 2),
  (11036, 'complete_objective', 62060, '62062', '{}', 0, 3);

-- Chain 11037: aid branch, prisoner 2 clicked during 62061
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11037, 'CB-N06 - interact Cellblock_Ambient_Prisoner2 during 62061: aid, grant 2 slappacks, complete 62062', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11037, 'interact_tag', 'Cellblock_Ambient_Prisoner2', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11037, 'world', 12, NULL, 'eq', NULL, 0),
  (11037, 'step_status', 62060, '62061', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (11037, 'add_item', 2893, NULL, '{"container": 0, "qty": 2}', 0, 0),
  (11037, 'remove_dialog_set', 7509, NULL, '{"slot": 22}', 0, 1),
  (11037, 'remove_dialog_set', 3862, NULL, '{"slot": 176}', 0, 2),
  (11037, 'complete_objective', 62060, '62062', '{}', 0, 3);

-- Chain 11038: sabotage branch, cell-door panel clicked during 62061
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11038, 'CB-N06 - interact 329_CellDoorButton during 62061: sabotage, grant 2 slappacks, complete 62062', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11038, 'interact_tag', '329_CellDoorButton', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11038, 'world', 12, NULL, 'eq', NULL, 0),
  (11038, 'step_status', 62060, '62061', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (11038, 'add_item', 2893, NULL, '{"container": 0, "qty": 2}', 0, 0),
  (11038, 'remove_dialog_set', 7509, NULL, '{"slot": 22}', 0, 1),
  (11038, 'remove_dialog_set', 3862, NULL, '{"slot": 176}', 0, 2),
  (11038, 'complete_objective', 62060, '62062', '{}', 0, 3);

-- Chain 11039: 62060 abandoned -> unbind the panel choice (the prisoner offer stays)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11039, 'CB-N06 - 62060 abandoned: unbind the cell-door panel choice', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11039, 'mission_abandoned', '62060', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (11039, 'world', 12, NULL, 'eq', NULL, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11039, 'remove_dialog_set', 7509, NULL, '{"slot": 22}', 0, 0);

-- Chain 11040: 641 accepted while 62060 was never taken -> withdraw the offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11040, 'CB-N06 - 641 accepted before 62060: withdraw the prisoner offer', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11040, 'mission_accepted', '641', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11040, 'world', 12, NULL, 'eq', NULL, 0),
  (11040, 'mission_status', 62060, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11040, 'remove_dialog_set', 3862, NULL, '{"slot": 176}', 0, 0);

-- Chain 11041: 62060 abandoned after 641 was accepted -> withdraw the offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11041, 'CB-N06 - 62060 abandoned after 641: withdraw the prisoner offer', 'mission', 62060, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11041, 'mission_abandoned', '62060', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11041, 'world', 12, NULL, 'eq', NULL, 0),
  (11041, 'mission_status', 641, NULL, 'neq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11041, 'remove_dialog_set', 3862, NULL, '{"slot": 176}', 0, 0);
```

Expected resolved actions (content-engine `Action` variants), used by every resolve and replay test:

| Chain | Trigger (`TriggerType`, params) | Gate (besides world 12) | Actions, in order |
|---|---|---|---|
| 11030 | `MissionCompleted`, `mission_id` 638 | 638 `completed`, 641 `not_active`, 62060 `not_active` | `AddDialogSet { dialog_set_id: 3862, slot: 176, mission_id: None }` |
| 11031 | `PlayerLoaded`, `world_name` `Castle_CellBlock` | same | same |
| 11032 | `PlayerLoaded`, `world_name` `Castle_CellBlock` | step 62061 `active` | `AddDialogSet { 3862, 176, None }`, `AddDialogSet { dialog_set_id: 7509, slot: 22, mission_id: None }` |
| 11033 | `InteractTag`, `entity_tag` `Cellblock_Ambient_Prisoner1` | 638 `completed`, 641 `not_active`, 62060 `not_active` | `AcceptMission { mission_id: 62060 }` |
| 11034 | `InteractTag`, `entity_tag` `Cellblock_Ambient_Prisoner2` | same | same |
| 11035 | `MissionAccepted`, `mission_id` 62060 | step 62061 `active` | `AddDialogSet { 7509, 22, None }`, `DisplayDialog { dialog_id: 60151 }` |
| 11036 | `InteractTag`, `Cellblock_Ambient_Prisoner1` | step 62061 `active` | `GrantItem { item_id: 2893, count: 2, container_id: Some(0) }`, `RemoveDialogSet { dialog_set_id: 7509, slot: 22 }`, `RemoveDialogSet { dialog_set_id: 3862, slot: 176 }`, `CompleteObjective { mission_id: 62060, objective_id: 62062 }` |
| 11037 | `InteractTag`, `Cellblock_Ambient_Prisoner2` | same | same as 11036 |
| 11038 | `InteractTag`, `329_CellDoorButton` | same | same as 11036 |
| 11039 | `MissionAbandoned`, `mission_id` 62060 | none | `RemoveDialogSet { 7509, 22 }` |
| 11040 | `MissionAccepted`, `mission_id` 641 | 62060 `not_active` | `RemoveDialogSet { 3862, 176 }` |
| 11041 | `MissionAbandoned`, `mission_id` 62060 | 641 `neq not_active` | `RemoveDialogSet { 3862, 176 }` |

Context keys (strings `active`, `completed`, `not_active`; a missing key reads `not_active`): `mission_<m>_status`, `mission_<m>_step_<s>_status`; the world is `ctx.world_id = Some(12)`, not a param. One click resolves every chain against the context **before** any action runs (`fire_interact_tag`, `cell-content/src/cell/content/event_dispatch/interaction.rs:55`), so the accept click (11033) cannot also fire an aid branch (11036): 62060 reads `not_active` at resolve time.

Stock chains that share a trigger (unchanged; each test that touches them asserts they still resolve as before):

- `329_CellDoorButton`: 1016 (Livewire, gated on 638 step 2115; disjoint once 638 is completed), 1017 and 1028 (bit set and restore at step 2115).
- `mission_accepted 641`: 1152 (`DisplayDialog { 4000 }`) co-fires with 11040.
- `Cellblock_Ambient_*`: no stock chain uses these tags; CB-N04a's 11010 is an `enter_region` bark.

## F5. Spawn and binding facts the tests use

| Fact | Value |
|---|---|
| Prisoner 1 | spawn 520, tag `Cellblock_Ambient_Prisoner1`, template 176, (-271.0, 65.45, -135.0), world 12, `is_stationary` true |
| Prisoner 2 | spawn 521, tag `Cellblock_Ambient_Prisoner2`, template 176, (-254.0, 65.45, -135.0) |
| Cell-door panel | spawn 22, tag `329_CellDoorButton`, template 22, (-287.290009, 67.2600021, -115.040001); 3.2 m from Prisoner 329 (spawn 6, (-289.808, 65.474, -113.139)) |
| Template 176 elsewhere | spawns 193, 199-203 in world 8 (Castle). The per-player bind lives on the cell entity, which is rebuilt on a world change, so it never reaches world 8 (seal check G6) |
| Dialog-set-map 3862 | set 71, `dialog_id` NULL, `interaction_flags` 1073741824, topic `Mend Slave Cages` (`dialog_set_maps.sql` line 3495). Unused by any chain before CB-N06 |
| Dialog-set-map 7509 | set 2124, `dialog_id` NULL, `interaction_flags` 1073741824, topic `Remove Safeguards to Prevent a Power Overload` (line 8721). Unused by any chain before CB-N06 |
| Bind observables | `player.available_interactions.get(&176) == Some(&vec![(3862, None, 1_073_741_824i64)])`; `.get(&22) == Some(&vec![(7509, None, 1_073_741_824i64)])` (precedent `chain_replay_tests/mission_701/body.rs` L537-547); unbound reads `None` or an empty `Vec` |
| Item 2893 | `Health Slappack TC1`, max stack 10, container sets `{1,17}` (`items.sql` line 4879) |
| Interact range | `MAX_INTERACT_DISTANCE` 5.0 (`crates/cell-world/src/cell/space_manager/interact_range.rs:13`) |
| `onDialogDisplay` | method index 105; args bytes 0..4 = speaker entity id (the player, for a speaker-0 monologue), 4..8 = dialog id (LE i32) |
| Content logs (no new telemetry) | `Content: accepting mission`, `Content: adding dialog set`, `Content: displaying dialog`, `Content: granting item` (or the executor's existing grant line), `Content: removing dialog set`, `Content: complete objective`, `Content: abandoning mission`; resource load `event = cooked_data.mission_catalog_inserted` with `mission_id` 62060 |
