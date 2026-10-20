# Cellblock continuation: wave 3 frozen content (CB-N05)

> **Type:** Reference (frozen literals for the W3 packets)
> **Audience:** The CB-N05 packet coders and their reviewers
> **Last updated:** 2026-10-17
> **Companions:** [w3-packets.md](w3-packets.md) (packets, rules), [CB-N05 seal](worknotes/cb-n05-seal.md) (the reviewed design these literals implement), [id-allocation.md § CB-N05](../four-zone-planning/id-allocation.md#cb-n05-individual-claims--2026-10-14)

Every literal below is frozen. Copy it byte for byte. Do not reword, reflow, re-escape or "fix" anything. If a loader, validator or emitter refuses a literal, or produces bytes that differ from it, stop and report `blocked` with both strings; never edit the literal to make a test pass (seal: "If the loader refuses a field combination, stop and return to seal review rather than weaken validation"). Seed comments carry `CIMMERIA_AUTHORING`; player-visible text never does.

## F1. Mission 62050 category-3 XML (CB-N05-1)

Exactly **1678 bytes** UTF-8, one LF after the declaration, no trailing newline. Two cooked-sync fragments (each at most 1390 bytes). The architect's SHA-256 of these bytes: `EE0C4359A1A3972936790877BC898E78EE43A28EE8F091A13E7CE25DF8AF2928` (a reviewer cross-check only; no test needs a hash).

Replace line 13-14 of `crates/resources/src/base/mission_additions.rs` (the doc comment and `pub const MISSION_ADDITIONS: &[MissionAddition] = &[];`) with this block, in the same raw-string style as `TEST_MISSION_XML` (rustc normalizes the working tree's CRLF to LF inside literals; the length test proves it):

```rust
/// CB-N05 *The Remaining Pods* (CIMMERIA_AUTHORING): the exact category-3
/// entry for mission 62050, 1678 bytes, frozen in
/// docs/analysis/cellblock-continuation/w3-frozen.md (F1).
pub const REMAINING_PODS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="The Remaining Pods" MissionID="62050"><HistoryText>Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.</HistoryText><Steps StepEnabled="false" StepID="62051" AwardXP="false" Difficulty="1"><StepDisplayLogText>Ask Prisoner 329 about the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62052" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak to Prisoner 329.</DisplayLogText><Tasks TaskType="1" TaskID="62053" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="62054" AwardXP="false" Difficulty="1"><StepDisplayLogText>Tell Col. Marsh there may be prisoners in the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62055" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Warn Col. Marsh.</DisplayLogText><Tasks TaskType="1" TaskID="62056" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Production authored entries: CB-N05's mission 62050 only.
pub const MISSION_ADDITIONS: &[MissionAddition] = &[MissionAddition {
    mission_id: 62050,
    xml: REMAINING_PODS_XML,
}];
```

Also change the module's line 2 doc comment to: `//! Production entries are added only by a mission packet that sealed its XML (CB-N05: 62050).`

## F2. Mission seed rows (CB-N05-1)

The four files are stored `i/lf w/crlf` and use one blank line between rows. In each file, insert the block directly after the **last** `INSERT` line (and before any trailing blank line, `--` TOC block or `SELECT pg_catalog.setval`), with one blank line before the comment and one blank line between the two `INSERT` rows.

`db/resources/Missions/Seed/missions.sql` (one row):

```sql
-- Cellblock continuation CB-N05 (CIMMERIA_AUTHORING): The Remaining Pods, optional one-time story after 638. Seal: docs/analysis/cellblock-continuation/worknotes/cb-n05-seal.md.
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (62050, 'Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.', false, true, false, false, 1, true, true, false, false, false, 1, 'The Remaining Pods', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`db/resources/Missions/Seed/mission_steps.sql`:

```sql
-- Cellblock continuation CB-N05 (CIMMERIA_AUTHORING): mission 62050 steps A and B.
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (62051, 62050, false, 1, false, 'Ask Prisoner 329 about the other pods.', 0);

INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (62054, 62050, false, 1, false, 'Tell Col. Marsh there may be prisoners in the other pods.', 1);
```

`db/resources/Missions/Seed/mission_objectives.sql`:

```sql
-- Cellblock continuation CB-N05 (CIMMERIA_AUTHORING): mission 62050 objectives A and B.
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (62052, 62051, false, 1, false, false, false, 'Speak to Prisoner 329.');

INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (62055, 62054, false, 1, false, false, false, 'Warn Col. Marsh.');
```

`db/resources/Missions/Seed/mission_tasks.sql`:

```sql
-- Cellblock continuation CB-N05 (CIMMERIA_AUTHORING): mission 62050 tasks A and B.
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (62053, 62052, false, 1, true, 1);

INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (62056, 62055, false, 1, true, 1);
```

No `mission_rewards` or `mission_reward_groups` rows: the mission has no reward.

## F3. Dialog 60150 (CB-N05-2)

Emitted bytes, exactly **305 bytes**, no newline anywhere (the shape `generate_dialog_xml` produced for Frost 3995, pinned by `frost_override_bytes_are_unchanged_by_the_shared_emitter`). Architect SHA-256: `05A0F6B78F33B5313E2A6E1F6978FEB17FB31BC4E2B8BE122E555486A7684DA9`.

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_DIALOG DialogFlags="0" DialogID="60150" KismetEventSetID="0" UIScreenType="2"><Screens SpeakerID="0" ScreenID="96110" Text="Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave."></Screens></COOKED_DIALOG>
```

New file `crates/resources/src/base/dialog_overrides/overrides_cellblock.rs`:

```rust
//! Cellblock authored dialog overrides (CB-N05, CIMMERIA_AUTHORING).
//!
//! Only the shape the client already renders from 3995/3996 is used: one
//! screen, `speaker_id` 0, zero buttons, UI type 2. Multi-screen or
//! NPC-speaker authored dialogs stay quarantined (2026-09-27 client crash).

use super::{DialogOverride, DialogScreen};

/// Mission 62050 *The Remaining Pods*: Prisoner 329's report, shown by
/// chain 11004 after the accept. Closing it (`dialogButtonChoice(60150, -1)`)
/// has no content action. Frozen in w3-frozen.md (F3).
pub(super) const REMAINING_PODS_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 60150,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 96110,
        speaker_id: 0,
        text: "Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave.",
        buttons: &[],
    }],
};
```

In `crates/resources/src/base/dialog_overrides/mod.rs`: add `mod overrides_cellblock;` after `mod patches_cellblock;`; add `#[cfg(test)]` + `mod override_seed_agreement_cellblock_62050;` after the `override_seed_agreement_debug_hub` pair; and append, as the **last** element of `DIALOG_OVERRIDES` (after the 3996 entry, before `];`):

```rust
    // CB-N05 mission 62050: Prisoner 329's report (dialog 60150), the
    // served one-screen speaker-0 shape. See overrides_cellblock.rs.
    overrides_cellblock::REMAINING_PODS_DIALOG,
```

Seed rows, both files `i/lf w/crlf`. Insert directly after the **last** `INSERT` line (the 60160 row added by CB-N04a), with one blank line before the comment; keep the existing blank line and `--` TOC block after it.

`db/resources/Dialogs/Seed/dialogs.sql`:

```sql
-- Cellblock continuation CB-N05: Prisoner 329's report for mission 62050 (CIMMERIA_AUTHORING).
-- Speaker-0 monologue, served by DIALOG_OVERRIDES (overrides_cellblock.rs).
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (60150, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Cellblock - The Remaining Pods: Prisoner 329 report');
```

`db/resources/Dialogs/Seed/dialog_screens.sql`:

```sql
-- Cellblock continuation CB-N05: dialog 60150's only screen (CIMMERIA_AUTHORING).
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60150, 96110, 'Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave.', 0, 0);
```

## F4. Chains 11000-11005 (CB-N05-3)

Append this block at the **end** of `db/resources/Content/Seed/castle_cellblock_chains.sql` (stored `i/crlf`), after the chain 11010 block, with one blank line before it. Do not touch the top-of-file header (W3-INT owns it, D-CC8). `scope_id` is informational only; the engine does not filter on it. The `world` condition keeps `target_id` = 12 (`Castle_CellBlock`) and a NULL value; it reads `ExecutionContext.world_id` and fails closed when that is unset.

```sql
-- =====================================================================
-- CB-N05 (2026-10-17): The Remaining Pods, mission 62050 (CIMMERIA_AUTHORING)
-- =====================================================================
-- Optional one-time story after stock 638. Prisoner 329 offers 62050 through
-- the existing interaction-only dialog-set-map 2795 at slot 17 (per player).
-- A validated Prisoner click accepts; the nested mission_accepted follow-up
-- advances A -> B, removes the offer and shows monologue 60150. A validated
-- Marsh click completes B, which completes the mission. No reward, no
-- dialog_choice or initialResponse path. Core Marsh chains 1051/1052/1152 are
-- unchanged. Seal: docs/analysis/cellblock-continuation/worknotes/cb-n05-seal.md.

-- Chain 11000: 638 completed -> offer 62050 at Prisoner 329
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11000, 'CB-N05 - 638 completed: offer The Remaining Pods at Prisoner 329', 'mission', 62050, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11000, 'mission_completed', '638', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11000, 'world', 12, NULL, 'eq', NULL, 0),
  (11000, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11000, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11000, 'mission_status', 62050, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11000, 'add_dialog_set', 2795, NULL, '{"slot": 17}', 0, 0);

-- Chain 11001: login in Castle_CellBlock -> restore the 62050 offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11001, 'CB-N05 - login: restore The Remaining Pods offer at Prisoner 329', 'mission', 62050, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11001, 'player_loaded', 'Castle_CellBlock', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11001, 'world', 12, NULL, 'eq', NULL, 0),
  (11001, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11001, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11001, 'mission_status', 62050, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11001, 'add_dialog_set', 2795, NULL, '{"slot": 17}', 0, 0);

-- Chain 11002: 62050 abandoned -> restore the offer
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11002, 'CB-N05 - 62050 abandoned: restore The Remaining Pods offer', 'mission', 62050, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11002, 'mission_abandoned', '62050', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11002, 'world', 12, NULL, 'eq', NULL, 0),
  (11002, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11002, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11002, 'mission_status', 62050, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11002, 'add_dialog_set', 2795, NULL, '{"slot": 17}', 0, 0);

-- Chain 11003: validated Prisoner 329 click -> accept 62050 (no other action)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11003, 'CB-N05 - interact Prisoner 329: accept The Remaining Pods', 'mission', 62050, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11003, 'interact_tag', 'Prisoner_329', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11003, 'world', 12, NULL, 'eq', NULL, 0),
  (11003, 'mission_status', 638, NULL, 'eq', 'completed', 1),
  (11003, 'mission_status', 641, NULL, 'eq', 'not_active', 2),
  (11003, 'mission_status', 62050, NULL, 'eq', 'not_active', 3);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11003, 'accept_mission', 62050, NULL, '{}', 0, 0);

-- Chain 11004: nested mission_accepted 62050 -> A to B, drop the offer, report
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11004, 'CB-N05 - 62050 accepted: advance to 62054, remove offer 2795, show dialog 60150', 'mission', 62050, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11004, 'mission_accepted', '62050', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11004, 'world', 12, NULL, 'eq', NULL, 0),
  (11004, 'step_status', 62050, '62051', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (11004, 'advance_step', 62050, '62054', '{}', 0, 0),
  (11004, 'remove_dialog_set', 2795, NULL, '{"slot": 17}', 0, 1),
  (11004, 'display_dialog', 60150, NULL, '{}', 0, 2);

-- Chain 11005: validated Col. Marsh click while B is active -> complete B
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11005, 'CB-N05 - interact Col. Marsh during 62054: complete objective 62055', 'mission', 62050, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11005, 'interact_tag', 'Preparation_ColMarsh', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11005, 'world', 12, NULL, 'eq', NULL, 0),
  (11005, 'mission_status', 640, NULL, 'eq', 'completed', 1),
  (11005, 'step_status', 62050, '62054', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11005, 'complete_objective', 62050, '62055', '{}', 0, 0);
```

Expected resolved actions (content-engine `Action` variants), used by every replay test:

| Chain | Trigger (`TriggerType`, params) | Actions, in order |
|---|---|---|
| 11000 | `MissionCompleted`, `mission_id` = 638 | `AddDialogSet { dialog_set_id: 2795, slot: 17, mission_id: None }` |
| 11001 | `PlayerLoaded`, `world_name` = `Castle_CellBlock` | same |
| 11002 | `MissionAbandoned`, `mission_id` = 62050 | same |
| 11003 | `InteractTag`, `entity_tag` = `Prisoner_329` | `AcceptMission { mission_id: 62050 }` |
| 11004 | `MissionAccepted`, `mission_id` = 62050 | `AdvanceStep { mission_id: 62050, step_id: 62054 }`, `RemoveDialogSet { dialog_set_id: 2795, slot: 17 }`, `DisplayDialog { dialog_id: 60150 }` |
| 11005 | `InteractTag`, `entity_tag` = `Preparation_ColMarsh` | `CompleteObjective { mission_id: 62050, objective_id: 62055 }` |

Context keys (strings `active`, `completed`, `not_active`; a missing key reads `not_active`): `mission_<m>_status`, `mission_<m>_step_<s>_status`; `archetype` is an i64 param; the world is `ctx.world_id = Some(12)`, not a param.

Co-firing stock chains at Marsh (unchanged, must still resolve beside 11005): 1051 (`interact_tag Preparation_ColMarsh`, 641 `not_active`, archetype `neq 8` → `DisplayDialog { 4001 }`), 1052 (same, archetype `eq 8` → `DisplayDialog { 5022 }`). 1152 (`mission_accepted 641` → `DisplayDialog { 4000 }`) must stay intact.

## F5. Spawn and binding facts the tests use

| Fact | Value |
|---|---|
| Prisoner 329 | spawn 6, tag `Prisoner_329`, template 17, (-289.808, 65.474, -113.139), world `Castle_CellBlock` (id 12) |
| Col. Marsh | spawn 7, tag `Preparation_ColMarsh`, template 10, (-191, 54.72, -138.588) |
| Dialog-set-map 2795 | dialog set 603, `dialog_id` NULL, `interaction_flags` 1073741824, topic `Free Prisoner 329` (seed `dialog_set_maps.sql` line 1907). Bound per player: `player.available_interactions.get(&17) == Some(vec![(2795, None, 1_073_741_824i64)])` (observable precedent `chain_replay_tests/mission_701/body.rs` L537-547) |
| Interact range | `MAX_INTERACT_DISTANCE` 5.0 (`crates/cell-world/src/cell/interact_range.rs:13`) |
| `onDialogDisplay` | method index 105; args bytes 0..4 = speaker entity id, 4..8 = dialog id (LE i32). A speaker-0 dialog in `load_monologue_dialog_ids` binds the player as speaker |
| Content logs (no new telemetry) | `Content: accepting mission`, `Content: advancing step`, `Content: removing dialog set`, `Content: displaying dialog`, `Content: adding dialog set`, `Content: complete objective`, `Content: abandoning mission`; resource load INFO `event = cooked_data.mission_catalog_inserted` (mission_id 62050) |
