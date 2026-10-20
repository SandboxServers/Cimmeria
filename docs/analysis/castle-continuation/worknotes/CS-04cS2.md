# CS-04cS2: return_q1 and return_q2 seal (mission leg)

> **Type:** Reference. **Audience:** Castle orchestrator, build packets CS-04c1s/d/c1/c2 and CS-04c2s/d/c1/c2, reviewers.
> **Status:** mission leg sealed (2026-10-18). **Base:** `2fad73404`. The route leg (agent 2, movement-teleport-advisor) is sealed below (Verdict, Route evidence).

Scope: docs only. No seed, Rust or DB edit and no live run. Model: `worknotes/CS-04aD.md` 55-317 (Q1 `medical_q1`, mission 10300). Route evidence: `worknotes/CS-04cS.md` (arrival `stargates.sql:83`, DHD, Marsh and Moh'katan all on navmesh component 116). Labels: `REPO` is read from this tree (file:line), `CIMMERIA_AUTHORING` is authored here, `OPEN` is not proven here.

Design in one paragraph (`CIMMERIA_AUTHORING`). The Harset to Castle crossing itself accepts `return_q1` (G10): the crossing proves the player left Harset for Castle, and the accepted mission row, not a dialog binding, carries the marker across the world transfer. On arrival Marsh is bound per player; clicking him shows one monologue and completes `return_q1`. That completion offers `return_q2` on Moh'katan (click accepts, a monologue follows); the second step is to dial Harset once more at the DHD. Both rewards are zero. Neither quest despawns, retags or re-flags Marsh or Moh'katan, and 708's own bindings are untouched.

## Verdict

- `return_q1`: SEALED. G10 is decided as `stargate_crossed 'Castle'` accept, with the save-across-transfer citation below. Live arrival, marker and dialog are `UATPending (end UAT)`.
- `return_q2`: SEALED. Offered on `return_q1` completion; the step-B trigger is `stargate_dialed 'Harset'` (proven shape: chains 1358/1359). Live dial is `UATPending (end UAT)`.
- Route: SEALED (component); path leg UATPending (RQ-R1, `nav_path` against a running Castle server, CD-9).

### Route evidence (agent 2, movement leg, 2026-10-18)

`nav_inspect` re-run on `data/spaces/castle.nav` through the lane (`cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/castle.nav --quiet --probes <probe file>`; probe file kept outside the repo), **exit 0**, h-tol 2.00 m, v-tol 3.00 m:

| Probe | Position | Poly | Component | h | dy | Result |
|---|---|---|---|---|---|---|
| arrival | (761.677, 63.466, 551.716) (`stargates.sql:83`, 'The Castle', world 8) | 9348 | **116** | 0.00 m | -0.08 | ok |
| dhd | (806.27, 55.10, 517.24) | 8859 | **116** | 0.00 m | -0.08 | ok |
| marsh | (810.732, 55.201, 515.012) (spawn 118, `spawnlist.sql:260`) | 8859 | **116** | 0.00 m | -0.24 | ok |
| mohkatan | (807.478, 55.208, 515.396) (spawn 120, `spawnlist.sql:264`) | 8859 | **116** | 0.00 m | +0.03 | ok |

All four lie on one component, so a walkable surface connects the arrival to Marsh, Moh'katan and the DHD. This is a component proof, not a route proof: component membership says a path exists, not that the server finds one within its search limits. The `nav_path` leg (arrival to Marsh, Marsh to Moh'katan, Moh'katan to DHD) needs a running Castle server and is end-UAT row RQ-R1 (CD-9). It also reconfirms CS-04cS's finding (`REPO`, `CS-04cS.md`) rather than relying on it.

Trigger order and landing (`REPO`):

1. **Chain actions run before the teardown.** Both crossing paths call `fire_stargate_crossed` and await it to the end before the travel is performed: the walk-through path at `crates/cell-interactions/src/cell/gate_travel/mod.rs:425` (`on_stargate_passage`; the transition itself is deferred behind `CROSSING_CINEMATIC_HOLD`, `:518`) and the no-gate-region fallback at `:312`, which then `return`s `perform_gate_travel` at `:321`. In `perform_gate_travel` the cell entity is destroyed only after `tx.send(CellToBaseMsg::GateTravel ...)` succeeds (`:626-636`; the comment at `:621-625` states the send-before-remove ordering). So the 10130 `accept_mission` and its `MissionUpdate` are enqueued before `GateTravel` on the same channel (ordering detail: the mission leg, G10 items 1-4).
2. **The player lands at the stargate row.** `validate_gate_arrival` (`crates/cell-world/src/cell/arrival.rs:187`) takes `gate.desired_arrival()` and runs `resolve_arrival` against the destination navmesh; the validated `arrival.position` is what `GateTravel` carries (`mod.rs:630`). If no standable point exists the transfer is refused (`:575-604`, `arrival_unrecoverable_off_mesh`) and the player stays in Harset, with 10130 already accepted. The Castle arrival probe is on-mesh (h 0.00 m, dy -0.08), so the refusal path is not reachable for this gate (`OPEN` until the live crossing, RQ-R2).
3. **The chain keys on the destination, with the origin world in context.** Event key `'Castle'` is the destination world name (`stargate.rs` module doc, `:7`), and `populate_world_context` (`:95`) sets the ORIGIN world id, matching the mission leg's `world 57` gate on chain 10130.
4. **Movement authority is untouched.** This seal adds no position write: the arrival goes through the existing GateTravel (base-driven world transition and fresh world entry), not an `onPlayerTeleport` or `BASEMSG_FORCED_POSITION` from content. The three quest NPCs are not moved, so there is no AoI refresh obligation of this seal's own. No speed or Z validation is touched.

Route end-UAT rows (movement leg, all `UATPending (end UAT)`):

- RQ-R1: `nav_path` on a running Castle server from the arrival point to Marsh, from Marsh to Moh'katan and from Moh'katan to the DHD (CD-9); each must return a route.
- RQ-R2: after the Harset to Castle crossing the player stands at the stargate (no snap-back, no refusal warn `arrival_unrecoverable_off_mesh`) and Marsh's marker is visible on the first look.

### G10 decision: seal the crossing accept (not fallback 2)

Required: cite (file:line) that a mission accepted inside a `stargate_crossed` chain is saved and restored after the world transfer.

1. **The chain runs before the teardown and before the travel message.** `fire_stargate_crossed` is called from `on_stargate_passage` (`crates/cell-interactions/src/cell/gate_travel/mod.rs:425`) and from the no-gate-region fallback (`:312`); both then reach `perform_gate_travel`, whose `GateTravel` send is at `:627`. The walk-through teardown is deferred behind `CROSSING_CINEMATIC_HOLD` (`:518` `begin_crossing_hold`; `:373` and `:530` explain why). The dispatcher awaits `execute_actions` to the end (`crates/cell-content/src/cell/content/event_dispatch/stargate.rs`, last statement of `fire`).
2. **Accept persists before the travel message.** `accept_or_advance` mutates the entity, then awaits `send_mission_update(..., "accept", tx, ...)` (`crates/cell-content/src/cell/content/executor/mission.rs:82-85`), then fires the nested `mission_accepted` (`:93`). Both `MissionUpdate` and `GateTravel` go through the same cell-to-base `tx` queue, in that order.
3. **Base writes the row.** `CellToBaseMsg::MissionUpdate` is awaited inline by the progression arm (`crates/base-world-entry/src/base/world_entry/cell_dispatch/progression_dispatch.rs:51-75`) into `handle_mission_update` (`crates/base-methods/src/base/world_entry/methods/missions/mod.rs:128-239`, the UPSERT).
4. **The next world entry re-reads the row.** `load_or_lock` calls `load_saved_missions_with_retry` (`crates/base-world-entry/src/base/world_entry_appearance/client_ready/mission_lock.rs:32`, implemented at `crates/base-methods/src/base/world_entry/methods/missions/state_lock.rs:105`) and hydrates the new cell entity from it. It never falls back to "none" (state_lock.rs doc comment on `load_saved_missions_with_retry`).
5. **Precedent in the tree.** Chain 1366 (`db/resources/Content/Seed/castle_carry_in_chains.sql:1-16`, `advance_step` on `stargate_crossed 'Harset'`) and its test `live_db_mission_1360_seeded_frost_accept_then_real_castle_crossing` (`crates/cell-content/src/cell/content/chain_replay_tests/mission_1360.rs:366`) rely on the same crossing-credit path; the seed header says "Crossing credits the route before destination hydration".
6. **Origin world.** The chain context's `world` is the ORIGIN world (`stargate.rs`, `populate_world_context` call), so a Harset to Castle chain gates `world 57`, not `world 8` (worlds.sql: 8 = `Castle`, 57 = `Harset`; the Harset gate is `stargates.sql:65`, world 57). The event key is the destination name `'Castle'`.

Not proven live (`OPEN`, row RQ1-U2): the on-disk ordering race between the `MissionUpdate` UPSERT and the destination hydration read. The cell enqueue order and the sequential `.await` in the base arm make the UPSERT land first by construction; the hold (`CROSSING_CINEMATIC_HOLD`) delays the `GateTravel` further. The c1 test `live_db_castle_side_return_q1_crossing_round_trip` proves the persisted row reloads through the production dispatch and `try_query_saved_missions`.

Fallback 2 (`player_loaded 'Castle'` with 1360 completed, twin on 1664) is NOT frozen. It stays the documented alternative if RQ1-U2 shows a lost row. Note for the orchestrator: neither fallback is needed while the crossing accept holds, so HA-01b (1664) is not a dependency of this seal.

## return_q1 IDs

Mission `10320` ("Report from Harset"). Used: step `10372`; objective `10492`; task `10612`; dialog `10721`; screen `10841`; dialog set `10771`; dialog set map `1000012`; chains `10130`-`10133`.

Reserved, unused: steps `10373`-`10377`; objectives `10493`-`10497`; tasks `10613`-`10617`; dialogs `10722`-`10724`; dialog set `10772`; screens `10842`-`10848`; buttons `10940`-`10943`; chains `10134`-`10139`; set map `1000013`.

## return_q2 IDs

Mission `10321` ("Confirm the Gate Address"). Used: steps `10378`, `10379`; objectives `10498`, `10499`; tasks `10618`, `10619`; dialog `10725`; screen `10849`; dialog set `10773`; dialog set map `1000014`; chains `10140`-`10146`.

Reserved, unused: steps `10380`-`10383`; objectives `10500`-`10503`; tasks `10620`-`10623`; dialogs `10726`-`10728`; dialog set `10774`; screens `10850`-`10856`; buttons `10944`-`10947`; chains `10147`-`10149`; set map `1000015`.

Every id is inside the D-CA20 rows (`r4-dispatch-plan.md` section 4). No text row, `texts.moniker_id`, event set, sequence, template or spawn is used.

## Collision check

Command (section 5, `<BASE>` = `2fad73404`), run 2026-10-18 from this worktree with the return_q1 regex, then the return_q2 regex:

```text
foreach ($r in '2fad73404','content/cellblock-campaign','content/harset-campaign','content/tollana-campaign','origin/main') { "== $r"; git grep -n -E "VALUES \((10320|1037[2-7]|1049[2-7]|1061[2-7]|1072[1-4]|1077[12]|1084[1-8]|1094[0-3]|1013[0-9]|100001[23])," $r -- db/resources/Missions db/resources/Dialogs db/resources/Content }
== 2fad73404
== content/cellblock-campaign
== content/harset-campaign
== content/tollana-campaign
== origin/main

foreach ($r in '2fad73404','content/cellblock-campaign','content/harset-campaign','content/tollana-campaign','origin/main') { "== $r"; git grep -n -E "VALUES \((10321|1037[89]|1038[0-3]|1049[89]|1050[0-3]|1061[89]|1062[0-3]|1072[5-8]|1077[34]|10849|1085[0-6]|1094[4-7]|1014[0-9]|100001[45])," $r -- db/resources/Missions db/resources/Dialogs db/resources/Content }
== 2fad73404
== content/cellblock-campaign
== content/harset-campaign
== content/tollana-campaign
== origin/main
```

Every ref prints its `== ref` header and then no match line: zero collisions on all five refs for both regexes. The anchor VALUES (<id>, covers the single-line seed shape used by Q1 and by these frozen rows; a plain \b<id>\b grep over db/resources/Content (Frozen (d)) backs it up.

## Frozen (a) XML

Files: `crates/resources/src/base/mission_additions_castle.rs`, two new constants (`r#"..."#`; neither copy contains `"#`), registered in `MISSION_ADDITIONS` in mission-id order after 10300 (find by id, never by index: CS-R4P). Each XML is one line with no newline, the declaration only, no DOCTYPE. The attribute shape is Q1's byte for byte except `Level="4"` (708's level; Q1 used 3), `MissionDefn` and `MissionID` (checked by diffing the root element against `CASTLE_MEDICAL_Q1_XML`: only those three differ).

### return_q1: `CASTLE_RETURN_Q1_XML` (mission 10320)

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="4" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Report from Harset" MissionID="10320"><HistoryText>Back from Harset, the gate team wants a report. Find Colonel Marsh at the Castle Stargate.</HistoryText><Steps StepEnabled="false" StepID="10372" AwardXP="false" Difficulty="1"><StepDisplayLogText>Report to Colonel Marsh at the Castle Stargate.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10492" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Report to Colonel Marsh.</DisplayLogText><Tasks TaskType="1" TaskID="10612" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

- **Byte count: 1,235** UTF-8 bytes (no BOM, no newline), measured with `[IO.File]::ReadAllBytes(<scratch file>).Length` on the exact line above. Limit `MAX_MISSION_XML_BYTES` = 355,840; margin 354,605. SHA-256 `05f50b6402cd6776439be21c6146a6a168d6780c6030dd32f8ca1e28ad19c68f`. The scratch file parses as XML with `MissionID` 10320.
- Ids 10320, 10372, 10492, 10612 pairwise unique, all at most 65535, `MissionID` equals the registry key, every required attribute present, child order `HistoryText`, `Steps` (`StepDisplayLogText`, `Objectives` (`DisplayLogText`, `Tasks`)).

### return_q2: `CASTLE_RETURN_Q2_XML` (mission 10321)

```text
<?xml version="1.0" encoding="UTF-8"?><COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="4" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Confirm the Gate Address" MissionID="10321"><HistoryText>Moh'katan wants the Harset address confirmed at the Castle Stargate. Dial Harset once more.</HistoryText><Steps StepEnabled="false" StepID="10378" AwardXP="false" Difficulty="1"><StepDisplayLogText>Speak with Moh'katan at the Castle Stargate.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10498" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak with Moh'katan.</DisplayLogText><Tasks TaskType="1" TaskID="10618" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="10379" AwardXP="false" Difficulty="1"><StepDisplayLogText>Dial Harset on the Castle Stargate.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="10499" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Dial the Harset address.</DisplayLogText><Tasks TaskType="1" TaskID="10619" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>
```

- **Byte count: 1,671** UTF-8 bytes (no BOM, no newline). Limit 355,840; margin 354,169. SHA-256 `c47a5717a2388e48f52afeed698fe919446860b8c7584f416e6eee9d4543c7a7`. Parses as XML with `MissionID` 10321.
- Ids 10321, 10378, 10379, 10498, 10499, 10618, 10619 pairwise unique, all at most 65535. Steps are declared A (10378) then B (10379), matching the `advance_step` order in chain 10143's nested chain 10144 (the client keys steps by declaration order).

`CanAbandon="true"` on both, as Q1. `NumRepeats="0"` (one-time). `AwardXP="false"` everywhere. `CanAbandon` is journal-only: `git grep can_abandon` finds no read in `crates/cell-content` or `crates/entity`, so the server does not refuse an abandon; the abandon paths are handled by chains 10133 and 10146.

## Frozen (b) Missions seeds

Placement: `missions.sql` directly after the `10300` row (`missions.sql:2112` at the base); `mission_steps.sql` after the `10341` row (`:7330`); `mission_objectives.sql` after the `10461` row (`:8124`); `mission_tasks.sql` after the `10581` row (`:8724`). If a sibling seal's rows (10301, 10310, 10312) sit there, keep ascending id order (the integrator sorts; these rows come last).

### return_q1

`missions.sql`:

```sql
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (10320, 'Back from Harset, the gate team wants a report. Find Colonel Marsh at the Castle Stargate.', false, true, false, false, 1, true, true, false, false, false, 4, 'Report from Harset', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`mission_steps.sql`:

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10372, 10320, false, 1, false, 'Report to Colonel Marsh at the Castle Stargate.', 0);
```

`mission_objectives.sql`:

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10492, 10372, false, 1, false, false, false, 'Report to Colonel Marsh.');
```

`mission_tasks.sql`:

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10612, 10492, false, 1, true, 1);
```

### return_q2

`missions.sql`:

```sql
INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (10321, 'Moh''katan wants the Harset address confirmed at the Castle Stargate. Dial Harset once more.', false, true, false, false, 1, true, true, false, false, false, 4, 'Confirm the Gate Address', 'General', 0, false, false, false, NULL, 0, 0, NULL);
```

`mission_steps.sql`:

```sql
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10378, 10321, false, 1, false, 'Speak with Moh''katan at the Castle Stargate.', 0);
INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (10379, 10321, false, 1, false, 'Dial Harset on the Castle Stargate.', 1);
```

`mission_objectives.sql`:

```sql
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10498, 10378, false, 1, false, false, false, 'Speak with Moh''katan.');
INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (10499, 10379, false, 1, false, false, false, 'Dial the Harset address.');
```

`mission_tasks.sql`:

```sql
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10618, 10498, false, 1, true, 1);
INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (10619, 10499, false, 1, true, 1);
```

Agreement with the XML: same ids, same text, `IsHidden` false and `IsOptional` false, steps and objectives `*_enabled` false, tasks enabled, `award_xp` false, `can_abandon` true, `num_repeats` 0, level 4, `script_name` and `script_spaces` NULL.

## Frozen (c) Dialogs seeds and override

No `texts` or `error_texts` rows. One dialog per quest (limit 2), each one screen, speaker 0, zero buttons (G5): a monologue, so `display_dialog` binds the player without an NPC speaker. The copy is `CIMMERIA_AUTHORING`.

Placement: `dialogs.sql` after the `10700` row (`:10868`); `dialog_screens.sql` after the `10800` row (`:28654`); `dialog_sets.sql` after the `10760` row (`:2369`); `dialog_set_maps.sql` after the `1000001` row (`:9355`).

### return_q1 (dialog 10721, Marsh's report monologue)

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10721, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10721, 10841, 'Colonel Marsh looks up from the gate controls. "Back from Harset, and in one piece. Good. Everything you saw out there goes into my report. The gate team is not done with you yet - Moh''katan is waiting at the DHD with questions about the address."', 0, 0);
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10771, '');
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000012, 10771, NULL, 'Report to Colonel Marsh', 1073741824, 1, '{}', '{}', '{}', '{}');
```

`crates/resources/src/base/dialog_overrides/overrides_castle.rs`, appended:

```rust
/// Castle return_q1 dialog 10721: Marsh's report monologue, shown by the
/// Marsh click that completes the quest. Frozen in the CS-04cS2 worknote (Frozen (c)).
pub(super) const RETURN_Q1_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10721,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10841,
        speaker_id: 0,
        text: "Colonel Marsh looks up from the gate controls. \"Back from Harset, and in one piece. Good. Everything you saw out there goes into my report. The gate team is not done with you yet - Moh'katan is waiting at the DHD with questions about the address.\"",
        buttons: &[],
    }],
};
```

`dialog_overrides/mod.rs` gets three lines: `mod override_seed_agreement_castle_10721;` (beside `:60`), and at the END of `DIALOG_OVERRIDES` (after `overrides_castle::MEDICAL_Q1_DIALOG` at `:208`; tests read entry 0 as Frost) the comment `// Castle return_q1 dialog 10721, Marsh's report monologue (CS-04cS2).` and `overrides_castle::RETURN_Q1_DIALOG,`. The text must equal the seed row byte for byte (`''` in SQL is `'` in Rust).

### return_q2 (dialog 10725, Moh'katan's confirmation monologue)

```sql
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (10725, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (10725, 10849, 'Moh''katan studies the DHD, then turns to you. "The Harset address is in the gate''s memory, but I will not trust it on your word alone. Dial Harset once more, and I will see for myself that the lock holds."', 0, 0);
INSERT INTO dialog_sets (dialog_set_id, name) VALUES (10773, '');
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000014, 10773, NULL, 'Speak with Moh''katan', 1073741824, 1, '{}', '{}', '{}', '{}');
```

```rust
/// Castle return_q2 dialog 10725: Moh'katan's confirmation monologue, shown by
/// the nested accept chain. Frozen in the CS-04cS2 worknote (Frozen (c)).
pub(super) const RETURN_Q2_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10725,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10849,
        speaker_id: 0,
        text: "Moh'katan studies the DHD, then turns to you. \"The Harset address is in the gate's memory, but I will not trust it on your word alone. Dial Harset once more, and I will see for myself that the lock holds.\"",
        buttons: &[],
    }],
};
```

`mod.rs`: `mod override_seed_agreement_castle_10725;`, the comment `// Castle return_q2 dialog 10725, Moh'katan's confirmation monologue (CS-04cS2).` and `overrides_castle::RETURN_Q2_DIALOG,`, appended after the return_q1 entry. Each "d" packet adds at most 4 lines to `mod.rs` (CD-12): three here. `tests/dialog_overrides.rs:87` served-id list gains 10721 / 10725 in ascending position (CD-16 union rule).

Both flag carriers are the NULL-dialog interaction-only shape (flags 1073741824 = `INT_MissionWorldObject`, the 2795 / Q1 `1000001` shape), so the bind lights the first-press marker and the click fires the `interact_tag` chain.

## Frozen (d) Chains

File: NEW `db/resources/Content/Seed/castle_return_chains.sql` (`CIMMERIA_AUTHORING`, one file per quest group so it cannot conflict with the siblings' files; D-CA17). It must be included in `db/database.sql` directly after the `castle_side_stories_chains.sql` line (`db/database.sql:400`): `\ir resources/Content/Seed/castle_return_chains.sql`. One statement per line (interact_tag linter). Slot ids are template ids: 10 (Marsh, spawn 118) and 54 (Moh'katan, spawn 120); the set-map id goes in `target_id`.

```sql
-- Castle return arc (CS-04cS2): return_q1 chains 10130-10133, return_q2 chains 10140-10146. CIMMERIA_AUTHORING.
-- Crossing accept: stargate_crossed 'Castle' runs in the ORIGIN world (57); the accept is persisted before GateTravel is sent.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10130, '10320 - Harset to Castle crossing: accept Report from Harset (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10130, 'stargate_crossed', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10130, 'world', 57, NULL, 'eq', NULL, 0), (10130, 'mission_status', 708, NULL, 'eq', 'completed', 1), (10130, 'mission_status', 10320, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10130, 'accept_mission', 10320, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10131, '10320 - Arrival or login at A: bind Marsh (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10131, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10131, 'world', 8, NULL, 'eq', NULL, 0), (10131, 'mission_status', 10320, NULL, 'eq', 'active', 1), (10131, 'step_status', 10320, '10372', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10131, 'add_dialog_set', 1000012, NULL, '{"slot": 10}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10132, '10320 - Marsh click at A: report, unbind, complete (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10132, 'interact_tag', 'Castle_ColMarsh', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10132, 'world', 8, NULL, 'eq', NULL, 0), (10132, 'mission_status', 10320, NULL, 'eq', 'active', 1), (10132, 'step_status', 10320, '10372', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10132, 'display_dialog', 10721, NULL, '{}', 0, 0), (10132, 'remove_dialog_set', 1000012, NULL, '{"slot": 10}', 0, 1), (10132, 'complete_objective', 10320, '10492', '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10133, '10320 - Abandoned: unbind Marsh (CIMMERIA_AUTHORING)', 'mission', 10320, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10133, 'mission_abandoned', '10320', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10133, 'world', 8, NULL, 'eq', NULL, 0), (10133, 'mission_status', 10320, NULL, 'eq', 'not_active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10133, 'remove_dialog_set', 1000012, NULL, '{"slot": 10}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10140, '10321 - 10320 completed: offer Moh''katan (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10140, 'mission_completed', '10320', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10140, 'world', 8, NULL, 'eq', NULL, 0), (10140, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10140, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10140, 'add_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10141, '10321 - Login, offer open: rebind Moh''katan (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10141, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10141, 'world', 8, NULL, 'eq', NULL, 0), (10141, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10141, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10141, 'add_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10142, '10321 - Moh''katan click: accept (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10142, 'interact_tag', 'Castle_Mohkatan', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10142, 'world', 8, NULL, 'eq', NULL, 0), (10142, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10142, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10142, 'accept_mission', 10321, NULL, '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10143, '10321 - Accepted: advance to B, unbind Moh''katan, show 10725 (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10143, 'mission_accepted', '10321', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10143, 'world', 8, NULL, 'eq', NULL, 0), (10143, 'step_status', 10321, '10378', 'eq', 'active', 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10143, 'advance_step', 10321, '10379', '{}', 0, 0), (10143, 'remove_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 1), (10143, 'display_dialog', 10725, NULL, '{}', 0, 2);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10144, '10321 - Harset dialed at B: complete (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10144, 'stargate_dialed', 'Harset', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10144, 'world', 8, NULL, 'eq', NULL, 0), (10144, 'mission_status', 10321, NULL, 'eq', 'active', 1), (10144, 'step_status', 10321, '10379', 'eq', 'active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10144, 'complete_objective', 10321, '10499', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10145, '10321 - Login stuck at A: heal to B (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10145, 'player_loaded', 'Castle', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10145, 'world', 8, NULL, 'eq', NULL, 0), (10145, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10145, 'mission_status', 10321, NULL, 'eq', 'active', 2), (10145, 'step_status', 10321, '10378', 'eq', 'active', 3);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10145, 'advance_step', 10321, '10379', '{}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (10146, '10321 - Abandoned: rebind Moh''katan (CIMMERIA_AUTHORING)', 'mission', 10321, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (10146, 'mission_abandoned', '10321', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (10146, 'world', 8, NULL, 'eq', NULL, 0), (10146, 'mission_status', 10320, NULL, 'eq', 'completed', 1), (10146, 'mission_status', 10321, NULL, 'eq', 'not_active', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (10146, 'add_dialog_set', 1000014, NULL, '{"slot": 54}', 0, 0);
```

Why this shape (the traps, applied):

- **Sibling disjointness (pairwise, per event key).**
  - `player_loaded 'Castle'`: 10131 (10320 active at 10372), 10141 (10320 completed, 10321 not_active) and 10145 (10320 completed, 10321 active at 10378); 10131 vs 10141/10145 is closed by 10320 active vs completed, and 10141 vs 10145 by 10321 not_active vs active.
  - `interact_tag 'Castle_ColMarsh'`: 10132 vs the live 708 chain 1352 (`step_status 708 '2417' active` AND `archetype neq 8`, `castle_706_708_chains.sql:680-685`). 10132 needs 10320 active, which needs 708 completed, and a completed mission has no active step, so the two never both hold.
  - `interact_tag 'Castle_Mohkatan'`: 10142 vs 1354 (`step_status 708 '2417'`, archetype 8, `:728-733`): same argument through 10320 completed.
  - `stargate_dialed 'Harset'`: 10144 vs 1358/1359 (`step_status 708 '4462' active`): 708 is completed in every state where 10321 can be active.
  - `stargate_crossed 'Castle'`: 10130 is alone on the key (the existing crossing chains 1360 and 1366 key `'Harset'`).
  - `mission_accepted '10321'`, `mission_completed '10320'`, `mission_abandoned '10320'` / `'10321'`: one chain each.
- **No dialog in the crossing chain.** The player is torn down immediately; a dialog could not be seen and the pin would be wrong. 10130 only accepts.
- **One `interact_tag` chain per NPC of mine.** A click when the bind is already removed resolves no chain of mine and falls through to the normal `handle_interact` path. `interact_tag` short-circuits `handle_interact` (`pin_interaction_target` is not called), so neither quest uses `dialog_choice` or a pinned speaker: both dialogs are speaker-0 monologues.
- **Action order.** 10132: `display_dialog`, `remove_dialog_set`, then `complete_objective` last (nested `mission_completed` dispatch runs mid action list and adds the Moh'katan offer; the step condition is evaluated once, before the actions). 10143: `advance_step` first, then unbind, then the dialog (Q1's 10004).
- **No DHD tag chain.** Step B completes on the dial event, not on a click, because a tag chain on `Castle_DHD` would short-circuit `handle_interact` and break the dial UI (chain 1356 already owns that tag at 708 step 2418).
- **Once only.** After completion `mission_status 10320/10321 = not_active` is false for the accept and offer chains, `num_repeats` is 0 and the offer guard (`crates/cell-content/src/cell/missions/lifecycle.rs:89-100`) refuses a completed non-repeatable mission.
- **Not gated on archetype.** Marsh reports for every archetype (708 split Marsh/Moh'katan by archetype 8; the return arc does not). `UATPending` RQ2-U7.
- **Id grep.** `git grep -n -E "\b(10130|10131|10132|10133|10140|10141|10142|10143|10144|10145|10146)\b" 2fad73404 -- db/resources/Content` finds no chain row (pasted result: empty).

## Frozen (e) Templates and spawns

None for either quest. Marsh is spawn 118 / template 10 (`Castle_ColMarsh`, world 8, `spawnlist.sql:260`) and Moh'katan is spawn 120 / template 54 (`Castle_Mohkatan`, world 8, `spawnlist.sql:264`); both exist and are on navmesh component 116 with the Harset arrival and the DHD (`CS-04cS.md` Point 1; agent 2 re-runs `nav_inspect`).

### G9 check: every spawn row of templates 10 and 54, any world (`REPO`)

| Template | Spawn | World | Tag | Source |
|---|---|---|---|---|
| 10 (Col. Marsh) | 7 | 12 (Cellblock) | `Preparation_ColMarsh` | `spawnlist.sql:90` |
| 10 | 118 | 8 (Castle) | `Castle_ColMarsh` | `spawnlist.sql:260` |
| 10 | 344 | 68 (Harset Command Center) | `CmdCenter_Marsh` | `spawnlist.sql:828` |
| 54 (Moh'katan) | 120 | 8 (Castle) | `Castle_Mohkatan` | `spawnlist.sql:264` |
| 54 | 343 | 68 (Harset Command Center) | `CmdCenter_Mohkatan` | `spawnlist.sql:817` |
| 54 | 8005 | 62 (Dakara E1) | `Dakara_E1_StoryRm_Mohkatan` | `spawnlist_dakara_e1.sql:24` |

The same rows appear on `content/cellblock-campaign`, `content/harset-campaign`, `content/tollana-campaign` and `origin/main` (no branch adds a row of template 10 or 54).

Existing slot binds on those templates (`git grep -E '"slot": ?(10|54)'`): Harset only, `harset_opcore_chains.sql` (slot 10: maps 5356, 5254, 5253; slot 54: maps 6397, 6398) and `harset_jaffa_chains.sql` (slot 54: maps 5149, 120001, 5159, 5161), all gated on Harset worlds. No Castle, Cellblock or Dakara chain binds either slot.

Why the shared slots are safe, and what a binding could do outside world 8:

1. `add_dialog_set` writes the cell entity's `available_interactions` (`crates/entity/src/cell_entity/entity_struct.rs:268`), which `construction.rs:67` initialises EMPTY, and a new cell entity is built on every world entry. A Castle bind is therefore gone after any world transfer; it cannot be live in worlds 12, 68 or 62. The slot is per player, not per spawn row: the only template-10/54 spawns a player can see while the bind is live are 118 and 120 (world 8).
2. Defence in depth: every chain of this seal carries `world 8` except the crossing accept (10130, `world 57`, a single `accept_mission`, no bind). Every bind is also re-added only on `player_loaded 'Castle'` or a world-8 event.
3. The two quests' binds are removed on use (10132, 10144) and on abandon (10133, 10146), so they are not left stale in Castle either.
4. The reverse direction: a Harset bind on slot 10/54 cannot leak into Castle for the same reason (new empty entity), and the Castle tag chains key on `Castle_ColMarsh` / `Castle_Mohkatan`, which exist only in world 8.

The c2 test proves it (Test list, `castle_side_return_c2_no_bind_outside_world_8`).

## Frozen (f) Reward

Zero for both: `missions.reward_naq = 0`, `reward_xp = 0`; no row in `mission_rewards.sql` or `mission_reward_groups.sql` for 10320 or 10321; no `grant_*` action in any chain; `award_xp` false at mission, step, objective and task level. Comparator: mission 1520 and Q1 (`reward_naq 0`, `reward_xp 0`); every Castle mission seed carries a zero reward. A later packet may add a modest XP grant without changing ids. Economy sign-off: not needed (no item, no currency).

## Behavior

State keys: `708 done` = mission 708 completed; `R1`/`R2` = mission 10320/10321 status; `A`/`B` = the quest's active step. Every click gets visible feedback on the first press: the bound marker is the affordance, and the click either shows a monologue (R1, R2 accept) or, with no chain resolved, runs the normal interaction path (generic line).

| State | Event | Chain | Actions / result |
|---|---|---|---|
| world 57, 708 done, R1 not_active | cross to Castle | 10130 | `accept_mission 10320` (row persisted, then travel) |
| world 57, R1 active or completed | cross to Castle | none | `condition_failed` on 10130 |
| world 57, 708 not done | cross to Castle | none | `condition_failed` (a GM-teleported or non-708 player is not offered) |
| world 8, R1 active at A | arrive or login (`player_loaded`) | 10131 | `add_dialog_set 1000012 {slot 10}`: Marsh shows the marker for this player |
| world 8, R1 active at A | click Marsh | 10132 | `display_dialog 10721`; `remove_dialog_set 1000012`; `complete_objective 10492` (mission completes, nested chain 10140) |
| world 8, R1 completed, R2 not_active | `mission_completed 10320` | 10140 | `add_dialog_set 1000014 {slot 54}`: Moh'katan shows the marker |
| world 8, R1 completed, R2 not_active | login (`player_loaded`) | 10141 | same bind |
| world 8, R1 completed, R2 not_active | click Moh'katan | 10142 | `accept_mission 10321` (nested `mission_accepted`) |
| R2 just accepted, A active | `mission_accepted 10321` | 10143 | `advance_step 10379`; `remove_dialog_set 1000014`; `display_dialog 10725` |
| R2 active at B, world 8 | dial Harset at the DHD (`stargate_dialed 'Harset'`) | 10144 | `complete_objective 10499` (mission completes) |
| R2 active at B | dial another gate | none | key mismatch |
| R2 active at A (crash between accept and nested advance), relog | `player_loaded` | 10145 | `advance_step 10379` (heal to B) |
| R1 active, abandon | `mission_abandoned 10320` | 10133 | `remove_dialog_set 1000012`; R1 is `not_active`; the next Harset to Castle crossing re-accepts |
| R2 abandoned after R1 completed | `mission_abandoned 10321` | 10146 | `add_dialog_set 1000014` (rebind) |
| R1 or R2 completed | login | none | no bind, no offer (`condition_failed` on 10131/10141) |
| R1 active, click Marsh with the bind missing (cache miss) | click | 10132 | still resolves (the chain keys on the tag and state, not the bind); warn `dialog_set_maps cache miss for add_dialog_set` on the bind attempt |
| Wrong world (12, 68, 62, 57) | any of the above except 10130 | none | `condition_failed`, zero binds |
| 708 at step 2417 or 4462, R1/R2 not started | click Marsh/Moh'katan, dial Harset | 1352/1354, 1358/1359 only | unchanged 708 behaviour |
| Forged dialog close / button, stale `initialResponse` | `dialog_choice` | none | nothing keys on `dialog_choice` (zero mission action) |
| Click out of range | `interact` | none | `interaction.out_of_range` row |
| Second character on the account | any | independent | `available_interactions` and `sgw_mission` are per character |

Notes: the stale 708 cue is not cleared. Chains 1350/1351 paint `INT_AStoryMissionActive` on `Castle_ColMarsh`/`Castle_Mohkatan` and the zero-baseline rule forbids clearing it (`castle_706_708_chains.sql:715-720`), so after 708 the shared NPC may still show 708's "!" for players who never get 10320. It is an existing 708 artifact, not changed here (`OPEN`, RQ1-U3 records what the client draws). Abandoning R1 leaves the player with no quest until the next Harset to Castle crossing; a login back-fill chain (`player_loaded` accept) is deliberately not added: it would turn abandon into a no-op and offer the quest to players who never saw Harset's return. `OPEN` for the owner if wanted.

## Test list

Module names: `castle_side_return_q1`, `castle_side_return_q2` (s and d), chain replay in `crates/cell-content/src/cell/content/chain_replay_tests/castle_side_return_q1*.rs` and `castle_side_return_q2*.rs` beside `castle_side_medical_q1*.rs`. Every test names a revert proof (flip the named item, the named guard must fail, restore).

### s

return_q1 (CS-04c1s):

1. `castle_side_return_q1_xml_admitted`: `CASTLE_RETURN_Q1_XML` passes `apply_mission_additions::prepare`; 1,235 bytes, one line, at most 355,840. Revert: append a byte; assert fails.
2. `castle_side_return_q1_ids_unique_and_in_block`: ids 10320/10372/10492/10612 pairwise unique, inside the D-CA20 row, none a stock key. Revert: change 10372 to 10341 (Q1's); fails.
3. `castle_side_return_q1_xml_matches_seed`: parse the XML; ids, flags, text, `IsHidden`/`IsOptional`, level 4 equal the four seed rows. Revert: edit one seed text; fails.
4. `castle_side_return_q1_is_registered_once`: exactly one `MISSION_ADDITIONS` entry with `mission_id` 10320, found by id, xml equals the constant; ids strictly ascending (CS-R4P shape). Revert: drop the entry; fails.
5. `live_db_castle_side_return_q1_seed_loads`: after `reload-db.ps1` the loader returns mission 10320, step 10372, objective 10492, task 10612. Revert: skip the seed include; fails.
6. `castle_side_return_q1_no_reward_rows`: no reward rows; `reward_naq = reward_xp = 0`. Revert: add a reward row; fails.

return_q2 (CS-04c2s): the same six with `q2` (1,671 bytes), steps 10378 then 10379 by `index`, objectives 10498/10499, tasks 10618/10619, and `castle_side_return_q2_steps_in_declaration_order` (XML step order equals the seed `index` order and chain 10143's `advance_step 10379`). Revert: swap the two `Steps` blocks; fails.

### d

1. `castle_side_return_q1_dialog_override_matches_seed`: override 10721/10841 equals the `dialogs` and `dialog_screens` rows: exact text, zero buttons, speaker 0, `ui_screen_type` 2, ids at most 65535. Revert: change one character in the override; fails.
2. `castle_side_return_q2_dialog_override_matches_seed`: same for 10725/10849. Revert: add a button; fails.
3. `override_seed_agreement_castle_10721` / `_10725` modules (Q1's `override_seed_agreement_castle_10700` shape): served exactly once, `dialogs.sql` holds the row once. Revert: duplicate the entry; fails.
4. `every_cooked_override_element_id_fits_in_16_bits` and `tests/dialog_overrides.rs:87` (served-id list) still pass with 10721/10725 added in ascending position. Revert: use id 100721; the 16-bit guard fails.
5. `cooked_sync` lane filter (G14): the existing `cooked_sync` guard runs unchanged and a transfer regression still fails (a filter, not a new test; no revert proof needed).

### c1 (return_q1: chains 10130-10133; return_q2: chains 10140-10146)

return_q1:

1. `castle_side_return_q1_chains_load`: chains 10130-10133 load with the exact triggers, conditions and action order frozen above. Revert: drop a condition; fails.
2. `castle_side_return_q1_crossing_accepts_once`: fire `stargate_crossed 'Castle'` from world 57 with 708 completed through the real executor: exactly one accept, step 10372 active, a `MissionUpdate` ("accept") enqueued BEFORE any `GateTravel` and no dialog action; a second crossing fires nothing. Revert: remove the `not_active` guard; the second crossing re-accepts and the test fails.
3. `castle_side_return_q1_crossing_gates`: origin world 8, 708 not completed, R1 active, R1 completed each resolve nothing (`condition_failed`). Revert: flip `world 57` to `world 8`; fails.
4. `castle_side_return_q1_marsh_click_reports_and_completes`: tag click at A: `display_dialog 10721`, `remove_dialog_set 1000012` and `complete_objective` emitted in that order; R1 completed; nested chain 10140 adds `1000014`. Revert: move `complete_objective` first; the order assertion fails.
5. `live_db_castle_side_return_q1_crossing_round_trip`: accept in a crossing, send the `MissionUpdate` through the production cell-to-base dispatch, then `try_query_saved_missions` returns mission 10320 active at step 10372 for that character only (two characters, exact sentinel ids, cleaned by exact id). This is the executable G10 persistence proof. Revert: send the update after the `GateTravel` message; the persisted-before-travel assertion fails.

return_q2:

6. `castle_side_return_q2_chains_load`: chains 10140-10146 load as frozen. Revert: change an action order; fails.
7. `castle_side_return_q2_offer_on_q1_completion`: `mission_completed 10320` binds `1000014` slot 54 only when R2 is `not_active`. Revert: drop the R2 guard; the repeat case fails.
8. `castle_side_return_q2_click_accepts_then_shows_dialog`: tag click accepts; nested chain advances A to B, removes the bind, shows 10725, in that order (real executor, not a resolve-only list). Revert: swap `advance_step` and `display_dialog`; fails.
9. `castle_side_return_q2_dial_completes_only_at_b`: `stargate_dialed 'Harset'` at B completes the mission; at A, not_active, completed, or with another gate key it does nothing. Revert: remove the step gate; the A case fails.
10. `castle_side_return_q2_repeat_click_noop`: a second click after accept fires nothing. Revert: drop the `not_active` guard.

### c2

1. `castle_side_return_c2_no_bind_outside_world_8` (G9): for every trigger of chains 10131/10133/10140/10141/10146 replayed in worlds 12, 68, 62 and 57 with all other conditions satisfied, zero `add_dialog_set` / `remove_dialog_set` resolves; a static scan asserts every chain of `castle_return_chains.sql` has a `world` condition, only 10130 uses 57 and carries exactly one `accept_mission` action. Revert: drop `world 8` from 10141; the world-68 case fails.
2. `castle_side_return_c2_siblings_disjoint`: no event matches two of my chains whose conditions both hold: `player_loaded 'Castle'` (10131, 10141, 10145) over every (R1, R2) state; `interact_tag` Marsh/Moh'katan against 1352/1354 over {708 at 2417 or 4462, 708 completed} x archetype 8 or not; `stargate_dialed 'Harset'` against 1358/1359. Revert: delete `mission_status 10320 completed` from 10145; the (R1 active, R2 active) case matches two chains.
3. `castle_side_return_c2_relog_each_state`: R1 active rebinds Marsh; R1 completed with R2 not_active rebinds Moh'katan; R2 at A heals to B; R2 at B needs nothing; both completed binds nothing. Revert: remove 10145; the heal case fails.
4. `castle_side_return_c2_abandon_rebinds`: abandon R1 unbinds Marsh; abandon R2 at A or B rebinds Moh'katan; R1 abandon then a new crossing accepts again. Revert: remove 10146; fails.
5. `castle_side_return_c2_forged_close_is_inert`: forged `button -1` / stale `initialResponse` cause zero mission actions on 10320/10321. Revert: add a `dialog_choice` chain; fails.
6. `castle_side_return_c2_wrong_world_and_missing_prereq`: each refusal prints `condition_failed` with `failed_condition_index`. Revert: drop `mission_status 708 completed` from 10130; the 708-not-done case accepts.
7. `castle_side_return_c2_out_of_range_click_refused`: the real `interact` method, far target negative, `interaction.out_of_range` row, no chain fires. Revert: remove the `interact_target_in_range` gate at `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs:108`; the far click resolves 10132 and the test fails.
8. `live_db_castle_side_return_c2_q2_round_trip`: accept, advance, complete and abandon persist through the production dispatch to `sgw_mission` and reload per character. Revert: omit the `MissionUpdate` on complete; reload shows B active.

### Regression filters each lane must add

The plan names `castle_706_708` as the regression filter, but `git grep` finds that string only in file comments and chain seed names, in no test name, so `-E 'test(castle_706_708)'` alone selects nothing (a vacuous guard). Use the module and test names that exist (`REPO`):

```text
test(mission_706) | test(mission_708) | test(chain_1360) | test(mission_1360) | test(castle_side_medical_q1)
```

Lanes: s and d add `test(cooked_sync) | test(mission_addition) | test(dialog_override) | test(castle_side_return)`; c1 and c2 add the regression filter above plus `test(castle_side_return)` and `test(castle_702_704)` (it shares `player_loaded Castle`). `live_db_` tests run only in the lane's live-DB mode.

## End-UAT rows

All `UATPending (end UAT)`; an observed blank dialog, crash, lost row or missing marker blocks the packet.

| Row | Check |
|---|---|
| RQ1-U1 journal | after the Harset to Castle crossing the journal shows *Report from Harset*, its history text and step "Report to Colonel Marsh at the Castle Stargate." |
| RQ1-U2 persistence | the accepted mission is present after arrival (G10: row written before the transfer, read at world entry); relog right after arrival keeps it |
| RQ1-U3 marker | Marsh shows the first-press marker for this player on arrival; record what the client draws and whether 708's old "!" is also lit |
| RQ1-U4 click | clicking Marsh shows dialog 10721 on one screen with no buttons, closes without a crash or blank panel; the journal marks R1 done; the marker goes |
| RQ1-U5 relog | relog at A (marker returns), after completion (no marker) |
| RQ1-U6 abandon | abandon at A removes the marker; the next Harset to Castle crossing re-accepts |
| RQ1-U7 no leak | with R1 active, in the Harset Command Center Marsh and Moh'katan show only Harset's own sets; no Castle marker anywhere outside Castle |
| RQ1-U8 route | on foot from the arrival (761.7, 63.5, 551.7) to Marsh at the DHD (component 116; the `nav_path` leg is the CD-9 end-UAT row) |
| RQ1-U9 708 regression | a fresh character at 708 step 2417 still gets 5008 (Tau'ri) / 5009 (Jaffa) from Marsh / Moh'katan unchanged |
| RQ2-U1 offer | Moh'katan shows the marker immediately after R1 completes, with no relog |
| RQ2-U2 accept | clicking accepts on the first press: journal shows *Confirm the Gate Address* at step B, dialog 10725 renders with no buttons, the marker goes |
| RQ2-U3 dial | dialing Harset at the DHD completes R2, the journal updates, and the dial UI is unaffected (no crash, gate opens as before) |
| RQ2-U4 relog | relog while R2 is not_active (marker), at B (state restored), after completion (nothing) |
| RQ2-U5 abandon | abandon at B returns the marker; re-accept works; a second character is unaffected |
| RQ2-U6 races | click Moh'katan twice quickly; click out of range (chat line appears) |
| RQ2-U7 archetype | a Jaffa (archetype 8) and a non-Jaffa character both receive both quests |
| RQ2-U8 no leak | slot 54 is not bound on Moh'katan in the Harset Command Center (68) or Dakara E1 (62) after R2 offers in Castle |
