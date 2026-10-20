# CS-04bD2a seal: `defense_q1` (Change of Watch)

> **Type:** How-to (design seal)
> **Audience:** Castle mission-systems reviewers, npc-ai-spawn reviewers and the `defense_q1` build packets (CS-04b1s, d, c1, c2)
> **Last updated:** 2026-10-18
> **Companions:** [CS-04aD (model seal, Q1)](CS-04aD.md), [CS-04bD (void premise)](CS-04bD.md), [reconciliation](../evidence/reconciliation.md)

**Status: sealed (agent 1 mission leg, agent 2 spawn leg).** Labels: `REPO` (read from the tree, file:line cited), `CIMMERIA_AUTHORING` (authored), `OPEN` (not proven). Everything client-only is `UATPending (end UAT)`. Ids come from the D-CA20 `defense_q1` row only; the CS-04bD ids are void (plan G3).

## Verdict

- `defense_q1`: `SEALED` (mission leg by agent 1; giver by agent 2). The giver is the new template 419 and spawn 13808 in the front courtyard (518.0, 28.18, 640.0), navmesh component 250 (the Infirmary's), 48.8 u from the nearest hostile (margin 25.8 u). The arrival row is component 116, not 250; the arrival-to-250 route is the open G-ROUTE and is an end-UAT row, not claimed here. Every client-visible check stays `UATPending (end UAT)`.

## defense_q1 IDs

All from the D-CA20 `defense_q1` row (`r4-dispatch-plan.md` section 4).

| Namespace | Row | Used | Reserved, unused |
|---|---|---|---|
| Mission | 10310 | 10310 | none |
| Steps | 10354-10359 | 10354 (A), 10355 (B) | 10356-10359 |
| Objectives | 10474-10479 | 10474, 10475 | 10476-10479 |
| Tasks | 10594-10599 | 10594, 10595 | 10596-10599 |
| Dialogs | 10709-10712 | 10709 (one monologue) | 10710-10712 |
| Dialog sets | 10765-10766 | 10765 | 10766 |
| Screens | 10817-10824 | 10817 (sole screen of 10709) | 10818-10824 |
| Buttons | 10928-10931 | none (G5, zero buttons) | 10928-10931 |
| Chains | 10100-10109 | 10100-10106 | 10107-10109 |
| Dialog set maps | 1000006-1000007 | 1000006 | 1000007 |
| Template | 419 | 419 (new, Frozen (e)) | none |
| Spawn | 13808 | 13808 (new, Frozen (e)) | none |
| Tag | n/a | `Castle_DefenseQ1_Sentry` | none |

No text row, no `texts.moniker_id`, no event set, no sequence (G6: copy is inline in the XML, the screen row and the override). `texts.sql` already holds rows whose moniker equals some of these ids (for example 10700, 10760 in Q1's range); a moniker is a different column from every id above and the inline copy reads none of them (`REPO` `db/resources/Texts/Seed/texts.sql:686-1706` for the Q1 equivalents).

### Premise (`CIMMERIA_AUTHORING`)

After the player has reported to the Castle Infirmary (Q1, *Triage Rounds*, mission 10300 completed), a sentry on post in the Castle tells them, in one monologue, when the guard changes over; the player carries the rota back to the infirmary and the mission completes on arrival. The sentry does not talk about the Goa'uld chain of command. It is the CS-04bD premise (guard-post intelligence) with ids, giver and chains rebuilt. Not a chamber guard: chamber guards 257-259 sit on isolated component 468 (`REPO` `evidence/reconciliation.md:56`; plan G11), so the giver is a new friendly sentry (Frozen (e)).

## Collision check

Plan G1/D-CA20 regex for `defense_q1`, run on `HEAD` (= 2fad73404, the base), the three campaign branches and `origin/main`. Command (PowerShell, in `$WT`):

```text
foreach ($r in 'HEAD','content/cellblock-campaign','content/harset-campaign','content/tollana-campaign','origin/main') { "== $r"; git grep -n -E "VALUES \((10310|1035[4-9]|1047[4-9]|1059[4-9]|10709|1071[0-2]|1076[56]|1081[7-9]|1082[0-4]|1092[89]|1093[01]|1010[0-9]|100000[67])," $r -- db/resources/Missions db/resources/Dialogs db/resources/Content }
```

Output (every ref, empty):

```text
== HEAD
== content/cellblock-campaign
== content/harset-campaign
== content/tollana-campaign
== origin/main
```

Template 419 and spawn 13808 (`VALUES \((419|13808),` on `db/resources/Entities db/resources/Worlds`, same five refs): the only hits are `blueprints.sql:637` and `blueprints_components.sql:3131-3135` (`blueprint_id` 419 on every ref), a different table and id space. No `entity_templates` or `spawnlist` row. Template 419 is reserved by D-CA20 and spawn 13808 is the `defense_q1` giver. Chain ids 10100-10109 also have no tuple outside the pasted check (a `git grep -E "\(10[01]0[0-9],"` over `db/resources/Content` on HEAD hits only Q1's 10000-10006 and the bark chains 10200-10201, which are different ids). Note: the check does not scan `db/resources/Texts`; text monikers are not ids used here (G6).

## Frozen (a) XML

File `crates/resources/src/base/mission_additions_castle.rs`, a second constant `pub const CASTLE_DEFENSE_Q1_XML: &str = r#"..."#;` (the copy contains no `"#`). One line, no newline, Q1's attribute shape. Registered as `MissionAddition { mission_id: 10310, xml: CASTLE_DEFENSE_Q1_XML }` in `MISSION_ADDITIONS`, in mission-id order after 10300 (the CD-16 union rule resolves parallel additions).

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="3" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Change of Watch" MissionID="10310"><HistoryText>A Castle sentry knows when the guard changes over. Get the times from the sentry and carry them to the infirmary.</HistoryText><Steps StepEnabled="false" StepID="10354" AwardXP="false" Difficulty="1"><StepDisplayLogText>Speak with the Castle sentry.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10474" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak with the sentry.</DisplayLogText><Tasks TaskType="1" TaskID="10594" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="10355" AwardXP="false" Difficulty="1"><StepDisplayLogText>Report the guard rota at the Castle Infirmary.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10475" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Reach the Castle Infirmary.</DisplayLogText><Tasks TaskType="1" TaskID="10595" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

- **Byte count: 1,684** UTF-8 bytes (`(Get-Item).Length` on the exact line, no BOM, no newline; the scratch file is outside the repo). SHA-256 `eb6b1418648e2026e01e6e1b4a2faa1f61a77750c350021bd96fe60999c8d687`. The line parses as XML with `MissionID` 10310.
- `MAX_MISSION_XML_BYTES` = 355,840 (`mission_additions.rs:21`): margin 354,156.
- Admission by construction (same rules as Q1): ids 10310, 10354, 10355, 10474, 10475, 10594, 10595 pairwise unique and at most 65535; `MissionID` equals the root key; no DOCTYPE; child order `HistoryText`, `Steps` (`StepDisplayLogText`, `Objectives` (`DisplayLogText`, `Tasks`)). Steps are declared A then B, matching `advance_step` in chain 10104 (the client keys steps by declaration order). 10310 is above every stock mission id.

## Frozen (b) Missions seeds

Placement: each line goes directly after the Q1 row named, inside the same file's data block and before any trailing `setval` (a trailing blank line separates the stock block from the authored block, as for Q1).

`db/resources/Missions/Seed/missions.sql`, directly after the 10300 row:

```sql
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (10310, 'A Castle sentry knows when the guard changes over. Get the times from the sentry and carry them to the infirmary.', false, true, false, false, 1, true, true, false, false, false, 3, 'Change of Watch', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`mission_steps.sql`, directly after the 10341 row:

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10354, 10310, false, 1, false, 'Speak with the Castle sentry.', 0);
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10355, 10310, false, 1, false, 'Report the guard rota at the Castle Infirmary.', 1);
```

`mission_objectives.sql`, directly after the 10461 row:

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10474, 10354, false, 1, false, false, false, 'Speak with the sentry.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10475, 10355, false, 1, false, false, false, 'Reach the Castle Infirmary.');
```

`mission_tasks.sql`, directly after the 10581 row:

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10594, 10474, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10595, 10475, false, 1, true, 1);
```

Agreement with the XML: same ids, same text, `IsHidden=false` and `IsOptional=false`, `step_enabled`/`is_enabled` false, `award_xp` false everywhere, `can_abandon` true, `num_repeats` 0 (one-time), level 3 (as Q1 and 1520). No reward rows (Frozen (f)).

## Frozen (c) Dialogs seeds and override

No `texts` or `error_texts` rows. One dialog, one screen, zero buttons, speaker 0 (G5, CD-12). Offers follow Q1: a click accepts (an `interact_tag` chain), then a `mission_accepted` chain shows the monologue.

`dialogs.sql`, directly after the 10700 row:

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10709, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
```

`dialog_screens.sql`, directly after the 10700/10800 row:

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10709, 10817, 'The sentry keeps his eyes on the approach as he talks. "The watch changes at dusk and again before dawn, two to every post. Tell the infirmary the posts are covered, so nobody has to guess when the guards are thin."', 0, 0);
```

No `dialog_screen_buttons` rows. `dialog_sets.sql`, directly after the 10760 row:

```sql
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10765, '');
```

`dialog_set_maps.sql`, directly after the 1000001 row (before the trailing `setval`; the explicit id is above the seed max and 1000002-1000005 belong to the other quests, so the integrator keeps both sides in id order). A NULL-dialog interaction-only flag carrier, the 2795/Q1 shape (flag 1073741824 = `INT_MissionWorldObject`):

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000006, 10765, NULL, 'Speak with the Sentry', 1073741824, 1, '{}', '{}', '{}', '{}');
```

Override constant, appended to `crates/resources/src/base/dialog_overrides/overrides_castle.rs` (G5 shape; the text equals the seed row byte for byte):

```rust
/// Castle defense_q1 dialog 10709: the sentry's monologue, shown by the
/// defense quest after the accept. Frozen in the CS-04bD2a worknote (Frozen (c)).
pub(super) const DEFENSE_Q1_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10709,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10817,
        speaker_id: 0,
        text: "The sentry keeps his eyes on the approach as he talks. \"The watch changes at dusk and again before dawn, two to every post. Tell the infirmary the posts are covered, so nobody has to guess when the guards are thin.\"",
        buttons: &[],
    }],
};
```

`dialog_overrides/mod.rs` (CD-12, at most 4 added lines): `#[cfg(test)]` + `mod override_seed_agreement_castle_10709;` beside `mod override_seed_agreement_castle_10700;` (:59-60), and one `overrides_castle::DEFENSE_Q1_DIALOG,` entry at the END of `DIALOG_OVERRIDES` after `MEDICAL_Q1_DIALOG` (:208; tests read entry 0 as Frost). The agreement test file `override_seed_agreement_castle_10709.rs` is a copy of `..._10700.rs` with ids 10709/10817. Both ids are at most 65535. `tests/dialog_overrides.rs:87` registry list gains 10709 (union rule, CD-16).

## Frozen (d) Chains

File `db/resources/Content/Seed/castle_side_stories_chains.sql` (D-CA17), appended after the last existing block at build time (CD-16 union). One statement per line (interact_tag linter). Scope `mission`, scope id 10310. `add_dialog_set`/`remove_dialog_set` `target_id` is the map id 1000006 and `slot` is the template id 419, per `executor/dialog/mod.rs`. The tag key is `Castle_DefenseQ1_Sentry` (Frozen (e)).

```sql
-- Castle defense_q1 (CS-04bD2a): chains 10100-10106. CIMMERIA_AUTHORING.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10100, '10310 - 10300 completed: offer the sentry (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10100, 'mission_completed', '10300', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10100, 'world', 8, NULL, 'eq', NULL, 0), (10100, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10100, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10100, 'add_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10101, '10310 - Login, offer open: rebind the sentry (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10101, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10101, 'world', 8, NULL, 'eq', NULL, 0), (10101, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10101, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10101, 'add_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10102, '10310 - Abandoned: rebind the sentry (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10102, 'mission_abandoned', '10310', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10102, 'world', 8, NULL, 'eq', NULL, 0), (10102, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10102, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10102, 'add_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10103, '10310 - Sentry click: accept (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10103, 'interact_tag', 'Castle_DefenseQ1_Sentry', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10103, 'world', 8, NULL, 'eq', NULL, 0), (10103, 'mission_status', 10300, NULL, 'eq', 'completed', 1), (10103, 'mission_status', 10310, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10103, 'accept_mission', 10310, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10104, '10310 - Accepted: advance to B, unbind sentry, show 10709 (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10104, 'mission_accepted', '10310', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10104, 'world', 8, NULL, 'eq', NULL, 0), (10104, 'step_status', 10310, '10354', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10104, 'advance_step', 10310, '10355', '{}', 0, 0), (10104, 'remove_dialog_set', 1000006, NULL, '{"slot": 419}', 0, 1), (10104, 'display_dialog', 10709, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10105, '10310 - Castle.Infirmary enter at B: complete (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10105, 'enter_region', 'Castle.Infirmary', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10105, 'world', 8, NULL, 'eq', NULL, 0), (10105, 'mission_status', 10310, NULL, 'eq', 'active', 1), (10105, 'step_status', 10310, '10355', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10105, 'complete_objective', 10310, '10475', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10106, '10310 - Login stuck at A: heal to B (CIMMERIA_AUTHORING)', 'mission', 10310, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10106, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10106, 'world', 8, NULL, 'eq', NULL, 0), (10106, 'mission_status', 10310, NULL, 'eq', 'active', 1), (10106, 'step_status', 10310, '10354', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10106, 'advance_step', 10310, '10355', '{}', 0, 0);
```

Why this shape (the traps, applied; Q1 is the proven pattern):

- **Sibling disjointness.** Per event key, the 10310 chains are pairwise disjoint on the 10310 status: `player_loaded Castle` carries 10101 (`not_active`) and 10106 (`active`); every other key (`mission_completed 10300`, `mission_abandoned 10310`, `interact_tag Castle_DefenseQ1_Sentry`, `mission_accepted 10310`, `enter_region Castle.Infirmary`) has exactly one 10310 chain. Chains of other quests share two trigger keys, `player_loaded Castle` (Q1 10001/10006, 1381-1383) and `enter_region Castle.Infirmary` (1367/1368/1369, bark chains 10200/10201, once=true): all dispatch (one event fires every matching chain), so each must gate on its own mission and none may act on 10310; this one gates on 10310 only. c2 pins it.
- **Frozen condition order.** Conditions are evaluated once at resolve time; in 10104 `advance_step` runs first, then the unbind, then the dialog, so no later action depends on a step status the earlier action flipped.
- **One `interact_tag` chain.** 10103 is the only chain on the tag, and it accepts only; a click when the map is removed has no chain and falls through to the normal path. `interact_tag` short-circuits `handle_interact`, so a follow-up `dialog_choice` chain would find no pinned target; none is authored, and 10103 does not display the dialog itself (10104 does, as Q1). No `dialog_choice` chain exists, so a forged close or button ordinal accepts nothing.
- **Completion.** B has one required objective; `complete_objective 10475` auto-completes the mission.
- **Nested dispatch.** 10100 fires on Q1's completion event, which Q1's own chain 10005 raises mid action list. The action here is only an `add_dialog_set`, independent of any later Q1 action.
- **Once only.** After completion `mission_status 10310 = not_active` is false for 10100-10103, and `num_repeats = 0` (offer guard `lifecycle.rs:89-100`).
- **Not archetype-gated** (as Q1); `UATPending` for archetype 5.

## Frozen (e) Templates and spawns

Frozen by agent 2 (npc-ai-spawn). Agent 1's no-candidate finding is **confirmed**, and the giver is the new template 419 and spawn 13808, **placed in the Castle front courtyard, not near the arrival**.

**Existing-giver proof (grep over `HEAD` = 2fad73404).** A qualifying existing NPC needs faction 1, exactly one spawn of its template, no dialog-set binding, no `interact_tag` chain on its tag, and navmesh component 250 (see the route note below). None does:

| Candidate | Result |
|---|---|
| Armory soldiers 189-192 (templates 174/175) | two spawns per template (`spawnlist.sql:597-600`), a slot binding paints both |
| Standoff rows 204-211 (templates 187/188/189) | faction 3, standoff combat (`live_db_castle_standoff.rs`) |
| Sgt. Gerschon 112 / 149, Surrender Guard 245 / 172, Copplemann 87 / 48, Romney 240 / 169, Ogilvie 197 / 179, medic 198 / 177, hub quartermaster 13804 / 415 | slot or tag already bound by 701, 706-708, 567, 1520, Q1 or the hub (agent 1's table, re-read) |
| PrJaffaLieuternant 119 / 159 | template 159 has five more spawns in world 57 (`spawnlist.sql:232-236`), and its spawn is on component 116 beside Marsh and Moh'katan (the return_q1/q2 slots) |
| Muelbach 241 / 170, Bravo officers 242-244 / 171 | faction 10 hostiles (the defense_q3 targets) |
| Comms terminal 246 / 173, DHD 2 / 162, chamber guards 257-259 | prop, prop, component 468 (G11) |

`git grep` for `Castle_DefenseQ1_Sentry` and `Castle_GuardPostSentry` over `db/resources` and `crates` on `HEAD`, `content/cellblock-campaign`, `content/harset-campaign`, `content/tollana-campaign` and `origin/main` has no hit, and `VALUES \((419|13808),` over `db/resources/Entities` and `db/resources/Worlds` hits only `blueprints.sql:637` and `blueprints_components.sql:3131-3135` (blueprint 419, another table) on all five refs. So tag, template and spawn are unused everywhere.

**Template 419** (`entity_templates.sql`, directly after the template 415 row): a byte copy of the template 174 row (`Castle_OpCoreSoldier`: faction 1, level 4, name_id 7552 "Op-CORE Soldier", weapon item 21, `has_dynamic_properties` true, no ability set, no loot, no aggro columns) with only `template_id` and `template_name` changed.

```sql
INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, aggro_radius, assist_radius, wander_radius, wander_min_dwell_secs, wander_max_dwell_secs) VALUES (419, NULL, 'BS_HumanMale.BS_HumanMale', '{AR_Global.Prisoner_Boots,AR_H_Clothing00.AR_HM_PL1_PL100,AR_H_Clothing00.AR_HM_PT1_PT100,BS_HumanMale.BS_HM_Boots_00,BS_HumanMale.BS_HM_Feet_00,BS_HumanMale.BS_HM_Hands_00,BS_HumanMale.BS_HM_Head_02,BS_HumanMale.BS_HM_Legs_00,BS_HumanMale.BS_HM_Torso_00,WP-Human.WP_SMG_1A}', 0, 0, 570, 4, 0, 1, 7552, NULL, NULL, NULL, 'Castle_GuardPostSentry', 'mob', NULL, NULL, NULL, NULL, NULL, NULL, NULL, -256, -256, -52773120, 21, '{}', NULL, NULL, true, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
```

**Spawn 13808** (`spawnlist.sql`, directly after the 13804 row, before the Interrogation Block comment; the file's `setval` footer takes `GREATEST(MAX(spawn_id), ...)`, so 13808 needs no footer edit):

```sql
-- Castle defense_q1 sentry (CS-04bD2a, CIMMERIA_AUTHORING): front courtyard, east of the Castle.CastleFrontCourtyard box, on navmesh component 250 with the Infirmary.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (13808, 518.0, 28.18, 640.0, 5.119, 8, 419, 'Castle_DefenseQ1_Sentry', NULL, true);
```

Why that spot. (1) It is where the player stands when the offer opens: Q1 (10300) ends on entering `Castle.CastleFrontCourtyard` (point set 2050, x 468.9-510.3, z 631.8-674.4, y 28.18; chain 10005), so the marker appears next to the player at Q1's completion. (2) It sits 7.7 u outside that box (x 518 against 510.3), so the sentry is not inside Q1's region, and it is nowhere near the `Castle.Infirmary` box (2051, x 344.9-371.0, z 880.2-904.6), so step B cannot complete on arrival at the giver. (3) Heading 5.119 rad (atan2(dx, dz) toward the courtyard centre; heading 0 faces +Z) turns the sentry toward the courtyard. (4) `is_stationary` true, as the hub quartermaster; template 419 has no wander columns, so it never moves, and a faction 1 NPC never fights.

**Navmesh (`nav_inspect data/spaces/castle.nav`, h-tol 2.00 m, v-tol 3.00 m, lane exit 0, probe file `C:\Users\Steve\AppData\Local\Temp\castle-r4\dq1a-final.txt`, `--quiet` removed nothing: the lane log carries it):**

```text
sentry13808          poly=10754  component=250  h=0.00 m  dy=-0.04 m  ok
medic198             poly=14230  component=250  h=0.00 m  dy=-0.01 m  ok
infirmary_c          poly=14435  component=250  h=0.00 m  dy=-0.00 m  ok
courtyard_c          poly=10873  component=250  h=0.40 m  dy=-0.00 m  ok
```

Component 250 for the giver, the medic (198, Q1's giver), the Infirmary box centre (358, 70.18, 892; step B's target) and the courtyard centre (490, 28.18, 652; Q1's target). Exit 0: all four share one component, so the two legs of this quest (courtyard to Infirmary, and the giver to the courtyard) are the same connected region Q1 already uses. **Correction to the plan:** the Castle arrival row (761.677, 63.466, 551.716) is on component **116** (probe `arrival`, ok, h 0.00, dy -0.08), the same as the DHD (806.27, 55.10, 517.24) and Marsh; the Infirmary and the courtyard are on 250. An earlier probe run with the arrival in the same file exits 3 ("probes span 2 components"), which is why it is not in the frozen run. The arrival-to-250 link is the unresolved G-ROUTE (`evidence/reconciliation.md`, the flowchart after line 56), not something this seal claims. The sentry therefore sits on the Infirmary's component, which is what a player who has just finished Q1 in the courtyard needs; DQ1-U8 below is reworded to match. The `nav_path` leg stays an end-UAT row (CD-9).

**Aggro clearance** (rule: `live_db_castle_population.rs` friendlies at or beyond `aggro_radius + 5` of every faction-10 world-8 hostile, 4 u vertical band, `AGGRO_VERTICAL_BAND`; sentry at (518.0, 28.18, 640.0)). Every faction-10 spawn row in `spawnlist.sql` for world 8 was measured; none patrols near the courtyard (the patrol sets 2086-2093 are in the Hall, Transept and Field, 90+ u away):

| Nearest hostile | Spawn | Height | Horizontal distance | In the 4 u band | Need (aggro + 5) |
|---|---|---|---|---|---|
| Castle_PRU4 (template 145) | 111 at (554.3, 24.37, 607.1) | dy 3.81 | **48.8 u** | yes | 23 u (default aggro 18) |
| Castle_NidGuard12 | 105 at (581.2, 22.07, 638.4) | dy 6.1 | 63.2 u | no | 25 u (aggro at most 20) |
| Castle_NidGuard11 | 104 at (589.3, 24.01, 610.7) | dy 4.17 | 76.8 u | no (0.17 u outside) | 25 u |
| Castle_Pop_FieldW_4 | 278 at (600.0, 20.02, 690.0) | dy 8.2 | 96.0 u | no | 23 u |
| Castle_PRU3 | 106 at (613.7, 17.31, 625.1) | dy 10.9 | 96.8 u | no | 23 u |
| Nearest other in-band hostile: Castle_NidGuard15 | 109 at (686.6, 25.18, 475.6) | dy 3.0 | 235.5 u | yes | 25 u |

Result: the nearest hostile of any height is 48.8 u away and the nearest in-band one is the same row, so the clearance margin is 48.8 - 23 = **25.8 u** against the friendlies rule and 48.8 - 18 = 30.8 u against bare aggro. Faction 3 standoff neighbours, none a hostile: Sgt. Stanton 21.3 u, courtyard soldiers 23.9-32.8 u (spawns 204-208). They are friendly to faction 1 (the Castle reaction table gives faction 1 no enemy); the build packet pins their distance in c1 only as a note, not an assertion.

**Requirements the build packets keep (c1 test 5-6):** exactly one spawn of template 419 (13808); the tag `Castle_DefenseQ1_Sentry` unique in `spawnlist`; slot 419 and the tag bound only by map 1000006 and chain 10103; world 8; faction 1; outside the Infirmary box and not on component 468 or 116; at least `aggro_radius + 5` from every faction-10 world-8 hostile (the 48.8 u row above is the tightest). Spawn 13808 is outside `FRIENDLY_SPAWNS` (189-212) in `live_db_castle_population.rs`, so that file's pinned counts (12 friendlies, 24 + 36) stay unchanged and the new live-DB test carries its own clearance assertion. `castle_navmesh.rs::every_castle_spawn_is_on_the_mesh` covers the spot without a database.

## Frozen (f) Reward

Zero. `missions.reward_naq = 0`, `reward_xp = 0`, no `mission_rewards` or `mission_reward_groups` row for 10310, `award_xp` false at every level. Comparator: Q1 and missions 1520/1521 (CS-04aD.md "Frozen (f)"). No economy sign-off needed for zero. The CS-04bD "modest reward" for the quest family stays with `defense_q3`.

## Behavior

State x event -> chain -> actions. Every row has visible feedback on the first press (the click accepts and the dialog and journal update at once).

| State | Event | Chain | Actions / result | Telemetry |
|---|---|---|---|---|
| 10300 not completed | login or any event | none | no marker | none |
| 10300 completed (Q1 courtyard), 10310 `not_active`, world 8 | `mission_completed 10300` | 10100 | bind map 1000006 on slot 419; the sentry shows the offer marker | `Content: adding dialog set` info, `interaction_only=true` |
| 10300 completed, 10310 `not_active`, relog | `player_loaded Castle` | 10101 | rebind | same |
| above, player in range | click the sentry (`interact_tag`) | 10103 | `accept_mission 10310` | accept row |
| accepted (step A active) | `mission_accepted 10310` | 10104 | `advance_step 10355` (A done, B active); unbind; show dialog 10709 | chain fire, `dialog.remove_set` info, mission update |
| B active | `enter_region Castle.Infirmary` | 10105 | `complete_objective 10475`; mission completes | chain fire, mission update |
| B active, relog | `player_loaded Castle` | none | state restores from `sgw_mission`; player must enter the region (it fires on entry); a player who logs in already inside must step out and in | none |
| A active (crash between accept and the nested advance), relog | `player_loaded Castle` | 10106 | `advance_step 10355` | chain fire |
| A or B active | abandon | 10102 | map rebound | chain fire |
| completed | relog, click, region | none | no marker, no accept, no repeat | `content.resolve` debug `reason = "condition_failed"` |
| click, out of range | `interact` | none | generic chat line | `interaction.out_of_range` info (`range_feedback.rs:157`) |
| click with 10300 not completed, wrong world, or 10310 already active/completed | `interact_tag` | 10103 refused | nothing accepted | `content.resolve` debug, `reason = "condition_failed"`, `failed_condition_index` 0 world / 1 prerequisite 10300 / 2 10310 `not_active` (`content-engine/src/chain/mod.rs:498-499`) |
| forged dialog close or button, stale `initialResponse` | `dialog_choice` | none authored | zero 10310 action | no 101xx row |
| cache miss on map 1000006 | `add_dialog_set` | 10100/10101/10102 | glyph never appears | warn `dialog_set_maps cache miss for add_dialog_set` |
| Two characters, one account | any | any | independent (`available_interactions` and `sgw_mission` are per character) | none |

**Refusal telemetry keys.** The CS-04bD proposals `castle_side.defense.refuse.world|prereq|done|target` are **dropped**; no new log field is invented. Each refusal is pinned to an existing reason: wrong world, missing prerequisite and already done all log `reason = "condition_failed"` with `failed_condition_index` (above); a far or wrong-space target logs `interaction.out_of_range`. The `s` packet asserts no new key exists.

## Test list

Module names `castle_side_defense_q1` (cell-content, resources) and `override_seed_agreement_castle_10709`. Revert proof for every test: revert the named item and the named test fails.

### s

Catalog and seeds. Files: `mission_additions_castle.rs`, `mission_additions.rs`, five Missions seeds, `tests/mission_additions*.rs`.

1. `castle_side_defense_q1_xml_admitted`: `CASTLE_DEFENSE_Q1_XML` passes `apply_mission_additions::prepare`; assert 1,684 bytes, one line, at most `MAX_MISSION_XML_BYTES`. Revert: change a byte.
2. `castle_side_defense_q1_ids_unique_and_in_block`: ids 10310/10354/10355/10474/10475/10594/10595 pairwise unique, inside the D-CA20 row, none a stock key. Revert: move one id out of the block.
3. `castle_side_defense_q1_xml_matches_seed`: parse the XML and compare ids, flags, text, `IsHidden`/`IsOptional` with the mission, step, objective and task seed rows. Revert: change a seed text.
4. `castle_side_defense_q1_registers_one_addition`: the id-keyed lookup (CS-R4P) finds 10310 in `MISSION_ADDITIONS`, and the registry stays ascending and unique. Revert: drop the entry.
5. `live_db_castle_side_defense_q1_seed_loads`: after `reload-db.ps1`, the loader returns mission 10310 with steps 10354 then 10355 (by `index`), objectives 10474/10475, tasks 10594/10595. Revert: change a step `index`.
6. `castle_side_defense_q1_no_reward_rows`: no `mission_rewards`/`mission_reward_groups` row for 10310; `reward_naq = reward_xp = 0`. Revert: add a reward row.
7. Regression filters the lane adds: `test(castle_side_defense_q1)`, `test(mission_additions)`, `test(cooked_sync)` (G14: a transfer regression still fails a lane).

### d

Dialog seeds and the override. Files: `overrides_castle.rs`, `override_seed_agreement_castle_10709.rs`, the four `dialog_overrides/mod.rs` lines, three Dialogs seeds.

1. `override_seed_agreement_castle_10709` (a copy of the 10700 test): override 10709/10817 equals the `dialogs`/`dialog_screens` rows (exact text, zero buttons, speaker 0, `ui_screen_type 2`); one served override for 10709. Revert: edit the override text.
2. `castle_side_defense_q1_dialog_ids_fit_16_bits`: 10709 and 10817 at most 65535 (`every_cooked_override_element_id_fits_in_16_bits`). Revert: set an id above 65535.
3. `shipped_overrides_carry_no_buttons` stays green (`dialog_overrides/mod.rs:624`). Revert: add a button.
4. `castle_side_defense_q1_dialog_set_map_is_flag_carrier`: map 1000006 has `dialog_id` NULL, flags 1073741824, set 10765. Revert: set a dialog id.
5. Regression filters: `test(dialog_overrides)`, `test(override_seed_agreement)`, `test(cooked_sync)`.

### c1

Chains, template and spawn, first replay layer. After the d commits are cherry-picked (CD-13). Files: `castle_side_stories_chains.sql`, `entity_templates.sql`, `spawnlist.sql`, new replay module `chain_replay_tests/castle_side_defense_q1.rs`.

1. `castle_side_defense_q1_chains_load`: chains 10100-10106 load with the exact triggers, conditions and action order above. Revert: change one action order.
2. `castle_side_defense_q1_accept_advances_and_shows_dialog`: replay the tag click through the real executor; assert accept, A to B, map 1000006 removed, dialog 10709 emitted, in that order. Revert: delete the nested `advance_step`.
3. `castle_side_defense_q1_infirmary_completes_only_at_b`: `enter_region Castle.Infirmary` at B completes the mission; at A, not-active, or completed it does nothing. Revert: drop the step condition.
4. `castle_side_defense_q1_repeat_click_noop`: a second click after accept fires nothing. Revert: drop the `not_active` condition from 10103.
5. `live_db_castle_side_defense_q1_giver`: exactly one spawn of template 419, tag unique in `spawnlist`, faction 1, world 8, outside the `Castle.Infirmary` box, not on component 468 or 116, at least `aggro_radius + 5` from every faction-10 world-8 hostile (tightest: spawn 111 at 48.8 u), carrying its own clearance assertion because 13808 is outside `FRIENDLY_SPAWNS`. Revert: add a second spawn.
6. `castle_side_defense_q1_slot_419_only_bound_here`: no other chain or map binds slot 419 or tag `Castle_DefenseQ1_Sentry`. Revert: add a second binding.
7. Regression filters: `test(castle_side_defense_q1)`, `test(chain_replay_tests)`, `test(live_db_castle_population)`.

### c2

Relog, abandon, forgery, siblings and persistence on c1's tip.

1. `castle_side_defense_q1_relog_each_step`: not-active rebinds (10101); A heals (10106); B needs nothing; completed binds nothing. Revert: remove 10106.
2. `castle_side_defense_q1_abandon_rebinds`: abandon at A and at B rebinds via 10102; abandon after completion cannot happen. Revert: remove 10102.
3. `castle_side_defense_q1_wrong_world_and_missing_prerequisite_refused`: `condition_failed` with the expected `failed_condition_index` for each. Revert: drop the 10300 condition.
4. `castle_side_defense_q1_forged_close_is_inert`: a forged `button -1` and a stale `initialResponse` cause zero 10310 mission actions. Revert: add a `dialog_choice` chain.
5. `castle_side_defense_q1_siblings_disjoint`: for every (10300, 10310) status pair, no event matches two 10310 chains whose conditions both hold (10101 and 10106 on `player_loaded Castle`); and the Q1 chains (10000-10006, 1381-1383) and 1367/1368/1369/10200/10201 never act on 10310. Revert: make 10101 and 10106 overlap.
6. `castle_side_defense_q1_q1_completion_offers_sentry`: replaying Q1's courtyard completion (chain 10005) in one run fires 10100 and binds map 1000006, and a player who completed 10300 earlier is bound at login (10101). Revert: drop 10100.
7. `live_db_castle_side_defense_q1_round_trip`: accept, advance, complete and abandon persist through the production cell-to-base dispatch to `sgw_mission` and reload per character (two characters, exact sentinel ids, cleaned by exact id). Revert: skip the persist call.
8. `castle_side_defense_q1_out_of_range_click_refused`: the real `interact` method, far target negative, `interaction.out_of_range` row. Revert: remove the range gate call.
9. Regression filters: `test(castle_side_defense_q1)`, `test(castle_side_medical_q1)` (Q1 must stay green), `test(live_db_castle_side_defense_q1)`.

## End-UAT rows

All `UATPending (end UAT)` until a lab or colo run (D-CA11, CD-9). An observed blank dialog, crash, lost progress or missing marker blocks the packet.

| Row | Check |
|---|---|
| DQ1-U1 journal | after accept, the journal shows *Change of Watch*, the history text, and step B "Report the guard rota at the Castle Infirmary." with objective "Reach the Castle Infirmary." |
| DQ1-U2 dialog | dialog 10709 renders its text on the first screen with no buttons, closes without a crash, and no blank panel appears |
| DQ1-U3 marker | the sentry shows the offer marker after 10300 completes, and a click accepts on the first press (visible feedback) |
| DQ1-U4 advance | step A completes and B becomes active at accept; the journal updates without a reload |
| DQ1-U5 complete | entering the Infirmary completes the mission; the journal marks it done; the marker does not return |
| DQ1-U6 relog | relog at not-active (marker returns), at B (restored, then step out and in), and after completion (no marker) |
| DQ1-U7 abandon | abandon at B returns the marker; re-accept works; a second character is unaffected |
| DQ1-U8 route | the path leg: `nav_path` from the sentry to the Infirmary and from the `Castle.CastleFrontCourtyard` box (Q1's end) to the sentry, all component 250 (CD-9; component equality is the static proof), with no hostile aggro on the route. The Castle arrival row is component 116: a player walking from it to the sentry depends on the open G-ROUTE and is not claimed |
| DQ1-U9 races | click twice quickly; click while out of range (chat line appears); accept while standing in the Infirmary doorway |
| DQ1-U10 sentry look | template 419 renders as an Op-CORE soldier with the glyph over this NPC only, standing east of the front courtyard facing it, 21-33 u from the faction 3 standoff group, with no hostile shooting at it |
