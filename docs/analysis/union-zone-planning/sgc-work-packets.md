# SGC_W1 Union work packets

> **Type:** How-to (dispatch contract)
> **Audience:** Union SGC main-session coordinator, packet coders, review-fixers and the specialists named per packet
> **Last updated:** 2026-10-18 (architect pass v3 against `origin/main` @ `d1bba0400`; supersedes the v2 allocation)
> **Status:** Planning. Nothing dispatched. Wave S0 is gated (see [Dispatch gate](#dispatch-gate-and-ownership)).
> **Companions:** [SGC campaign](sgc.md), [Union execution contract](execution-contract.md), [Dakara packets](dakara-work-packets.md), [Omega packets](omega-work-packets.md), [packet folder](sgc-packets/)

## Dispatch gate and ownership

Start `docs/analysis/union-sgc-continuation/` only after the four-zone continuations and FZ-99; work the **Human path first**, then the Free Jaffa return variants. The four-zone [execution contract](../four-zone-planning/execution-contract.md) supplies one campaign branch, one reviewed PR after integrated waves, one worktree and one test database per packet, bounded coder briefs, a fresh review per wave and lab approval by the user.

SGC owns only `SGC_W1`, world **58**. Worlds 86 (`SGC`) and 87 (`SGC_W2`) are separate and never receive world-58 content by name similarity. Dakara (worlds 61/62) owns its local steps; Omega (18/80) owns first arrival.

**Old packet names.** Older Union drafts cite US-01 to US-06. They map to the new packets: old US-01 → US-10 guards; US-02a → US-12/US-15/US-16; US-02b → US-13, US-20 to US-23; **US-03 → US-24 (C1) and US-30 (C2 arrival, C2b)**; **US-04 → US-31 (1654) and US-32/US-33 (1655, C4)**; US-05/06 → US-40, US-U1, US-U2, US-99.

## Short answer

Missions 1563-1571 (the SGC crisis after the opening), the SGC halves of 1651, 1654 and 1655, and the two departures C1 (to Dakara) and C4 (to Omega) have **no chains today**. Every step, objective and dialog exists in the seeds (RESTORED_SOURCE). No legacy python names a grant edge for any of them, so every offer, position and encounter below is CIMMERIA_RECONSTRUCTION, and the story path needs no new line of text. Authored ambience (US-40) is separate, droppable and HELD.

The campaign is 24 packets in five waves:

| Wave | Packets | What |
|---|---|---|
| S0 | US-00, US-01, US-02 | Specialist seals: ledger and IDs, placement points, meshes and sequences (docs only) |
| S1 | US-10, US-11, US-14 | Scaffold, world seeds, crossing log test |
| S2 | US-12, US-13, US-15, US-20, US-21, US-22 | Missions 1563, 1571, 1564, 1565/1566, 1567, 1568 |
| S3 | US-16, US-23, US-24, US-30, US-31, US-32 | Escort, 1569, 1570 and C1, 1651, 1654, 1655 Human and C4 |
| S4 | US-33, US-34, US-40 (HELD), US-U1, US-U2, US-99 | Free Jaffa 1655, return state, authored ambience, lab UATs, close-out |

Each code packet owns **one chain file and one test file**, so no two packets in a wave edit the same seed or test file. US-10 creates all of them empty and registers them, so later packets never touch `db/database.sql`, the linter's file list or the test `mod.rs` lists.

## Findings (verified 2026-10-18 at `d1bba0400`)

| # | Finding | Evidence | Packets |
|---|---|---|---|
| F1 | No "talk" interaction bit exists. Every SGC_W1 template has `interaction_type 0`, so an NPC is clickable only while a chain sets a marker on it. Every press chain therefore needs a marker set by an earlier chain or a `player_loaded` restore. | `crates/entity/src/interaction_flags.rs`; `entity_templates.sql` rows 29-58 | all |
| F2 | Carter's flavor line 5394 has no press to hang on (no marker outside a story beat). It is dropped from the story path and parked in US-40. Hammond's 5395 and Teal'c's 5396 play after the 1570 accept instead. | F1 | US-24, US-40 |
| F3 | `fire_mission_completed` runs **after** the mission state changes, so one chain may carry a `mission_completed X` trigger and a `player_loaded` trigger together with a `mission_status X eq completed` condition. That is the restore pattern every offer chain uses. | `event_dispatch/mission.rs:105` | all |
| F4 | `delay_ms` is measured from fire time, not from the previous row, and a deferred action is queued, so a deferred `advance_step` leaves the old step gate open for the whole window (`assert_no_deferred_actions`, the mission 708 faucet). Every departure therefore advances its **first** step at 0 ms (the gate closes at once), plays sequence 10047 at 0 ms and 10060 at 2500 ms, then advances the arrival step and teleports at 3000 ms. A relog inside the window leaves the intermediate step active; one restore chain and one recovery press per departure finish the crossing without the dial. | `executor/mod.rs:96-120`; `chain_replay_tests/mod.rs:96-128` | US-24, US-30, US-32 |
| F5 | The cell hints every point set with `flags & 1` to the client, so new cylinder regions need no client change. | `send_client_hinted_regions` | US-11, US-15, US-22 |
| F6 | Dialog 5371 has one type-4 "Receive Item" button (display only, never key it). 5855 has Accept on screens 0-2 only and fails linter R1, so it is never keyed. 5370 has Accept on screens 0-2 and 5-9, including its final screen, so keying it passes R1. | `dialog_screen_buttons.sql` | US-12, US-33 |
| F7 | `entity_dead_tag` fires once per killer with the **killer's** mission context; the killer must be a player. A kill by a damage-over-time tick fires nothing (engine gap, castle note). | `castle_702_704_chains.sql:174-212` | US-20, US-21, US-22 |
| F8 | Chain 3005 moves Hammond (`SGCW1_GenHammond`) to the armory (-123.625, 1.311, -246.858) during 1559. Nothing brings him back. US-22 walks him home on the 1568 completion; relogs spawn him at home (spawn 52). | `sgc_w1_chains.sql:102` | US-22, US-24 |
| F9 | The interact-tag linter is per file: every `interact_tag` key in a `*_chains.sql` file needs a `set_interaction_type` row for the same tag **in that file**. | `crates/content-engine/tests/it/interact_tag_linter.rs:301` | all |
| F10 | `dialog_button_linter/seed_model.rs:14` lints only the files in `CHAIN_FILES: [&str; 5]`; `sgc_w1_chains.sql` is not in it. US-10 adds the 14 new files. | `seed_model.rs:14` | US-10 |
| F11 | Static rows 57/58 (template 34) and 59-63 (template 35) are untagged hostile Jaffa on the 1564 route; 65-67 are the tagged Gate Room Jaffa `SGC_W1_GroomJaffa1-3`; 71 and 77 are the 1562 lab attackers. | `spawnlist.sql:125-171` | US-11, US-20, US-21, US-34 |
| F12 | Carter hands over her notes herself: dialog 5370 screen 9 and dialog 5371 both read "(Colonel Carter gives you her notes on the SGC's computer protocols.)". No notes prop is needed, so template 5801, spawn 15802 and point P03 are reserved and unused. | `dialog_screens.sql` (5370, 5371) | US-11, US-12 |
| F13 | **complete_objective trap.** Completing the last open required objective of the **current** step completes the whole mission (and fires `mission_completed`); it never advances to the next step. Mid-mission chains use `advance_step` (which force-completes the old step); `complete_objective` is only safe on the final step, or while another required objective of the same step stays open. Every SGC packet follows this: 1563 4629, 1564 4633, 1565-1567, 1568 4639, 1569 4640, 1571 and 1654 4939 are final steps. 1651 4931 is not final and has **two** required objectives, O6037 (Harriman) and O5956: Carter may complete O5956 while O6037 is open, and Harriman's press must `advance_step '4929'`, never complete O6037 (H10). | `crates/cell-content/src/cell/missions/progression.rs:463-506` | US-12, US-15, US-20-US-23, US-30, US-31 |

## Rules every packet carries

Each packet file repeats the rules it needs, because `packet-coder` skips CLAUDE.md. The common set:

1. PowerShell only. Start every command with `Set-Location <worktree>;`. Use absolute paths. No bash.
2. Every compiling `cargo` call goes through the lane, in the foreground, in this order:
   - `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`
   - `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
   - `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::union_sgc::<module>`: this reloads the worktree's own `sgw_<worktree>` database from `db/database.sql`, then runs the live-DB tier
   - `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine`: the seed linters

   Never point anything at the database `sgw`.
3. Edit only the packet's allowlist. Never touch `dakara*`, `omega*` or `beta*` files, `db/scripts/`, or a chain outside the packet's ID range.
4. Seeds are the source of truth. Edit `db/resources/` only, with no migrations.
5. Files have a soft cap of 500 lines and a hard cap of 700. Split a test file before it passes 500.
6. `docs/**/*.md` is CRLF. After writing a doc, run `$p='<abs>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`.
7. No `git stash`, no `git worktree prune`, no push. Make one local commit with `git add <each path>`.
8. Tests are live-DB chain-replay tests ([TESTING.md](../../../TESTING.md) live-DB tier):
   - Name each test `live_db_…`; its first line is `let pool = require_db_or_skip!();`.
   - Each test fails when its seed rows are reverted, and its doc comment names that revert.
   - Negative cases cover the wrong archetype, the wrong step and the wrong world.
9. If a linter flags a chain, fix the chain. Never edit a linter or its allowlist.
10. Tag every new chain's comment with exactly one label: RESTORED_SOURCE, CIMMERIA_RECONSTRUCTION or CIMMERIA_AUTHORING.
11. No lab or live client. A step that needs one is a lab UAT packet, which asks the user first.

## Contract

### K1. Chain row shapes (verified 2026-10-18)

Copy these shapes exactly; `sgc_w1_chains.sql:535-690` (chains 3030-3037) is the house precedent. Every chain uses `scope_type 'mission'` with `scope_id` = its mission.

```sql
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (3051, 'SGC_W1 - <what> (Human)', 'mission', 1563, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES
  (3051, 'mission_completed', '1562', 'player', false, 0),
  (3051, 'player_loaded', 'SGC_W1', 'player', false, 1);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (3051, 'mission_status',   1563, NULL,   'eq',  'not_active', 0),
  (3051, 'step_status',      1564, '4630', 'eq',  'active',     1),
  (3051, 'objective_status', 1651, '5956', 'neq', 'completed',  2),
  (3051, 'world',            58,   NULL,   'eq',  NULL,         3),
  (3051, 'archetype',        NULL, NULL,   'gte', '1',          4),
  (3051, 'archetype',        NULL, NULL,   'lte', '4',          5),
  (3051, 'entity_tag_state', NULL, 'SGC_W1_TacticsJaffa1', 'eq', 'dead', 6);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (3051, 'display_dialog',       5370, NULL,   '{}', 0, 0),
  (3051, 'accept_mission',       1563, NULL,   '{}', 0, 1),
  (3051, 'advance_step',         1563, '4629', '{}', 0, 2),
  (3051, 'complete_objective',   1563, '5369', '{}', 0, 3),
  (3051, 'set_interaction_type', NULL, 'SGC_W1_SamCarter', '{"op": "|", "mask": 16777216}', 0, 4),
  (3051, 'play_sequence',        10047, NULL,  '{}', 0, 5),
  (3051, 'spawn_entity',         34,   'SGC_W1_Tactics2Jaffa1', '{"x": 60.0, "y": 1.311, "z": -120.0, "heading": 0}', 0, 6),
  (3051, 'despawn_entity',       NULL, 'SGC_W1_GroomJaffa1', '{}', 0, 7),
  (3051, 'add_item',             6784, NULL,   '{"container": 2, "qty": 1}', 0, 8),
  (3051, 'move_entity',          NULL, 'SGC_W1_SamCarter', '{"destination": "170.5,-10.9,12.5", "world": "SGC_W1"}', 0, 9),
  (3051, 'move_entity',          NULL, NULL,   '{"destination": "182.0,1.311,43.5", "world": "SGC_W1", "use_player": true}', 0, 10),
  (3051, 'cross_world_teleport', NULL, 'Dakara_E1', '{"x": 96.87, "y": -16.75, "z": 243.23}', 0, 11);
```

- Triggers and their `event_key`: `interact_tag` (spawn tag), `dialog_choice` (dialog id), `enter_region` (`point_sets.name`, byte-exact), `entity_dead_tag` (tag; scope `'space'` as in castle chain 1272), `mission_completed` and `mission_accepted` (mission id), `player_loaded` (`'SGC_W1'`). Several trigger rows on one chain OR together; conditions AND together.
- Masks: 8388608 is "?" (offer), 16777216 is "!" (story active), 33554432 is turn-in, 1073741824 is a mission world object (the glow that makes a prop pressable). Clear a mask with `"op": "~"`.
- `step_status`: target_id = mission, target_key = step, value `not_active`/`active`/`completed`. `advance_step`: target_key = the **new** step; it force-completes the current step's objectives. Use `complete_objective` for a final objective, so the mission completes and `mission_completed` fires.
- `spawn_entity`: target_id = template, target_key = tag, x/y/z required, heading optional. It is one-shot (no respawn) and skips a tag that is already alive.
- `add_item`: weapons go to container 1; mission items 6784 and 6786 go to container 2.
- `move_entity` moves an NPC by tag, or the player with target_key NULL and `"use_player": true` (precedent chain 3007).
- Escort (castle chains 1263/1291/1296/1302): start with `set_follow_target` on the NPC tag and `{"use_player": true}`; stop with `set_follow_target` `{}` and then `move_waypoint` `{"destination": "x,y,z"}`.
- "All dead" (castle chain 1350): one chain with N `entity_dead_tag` trigger rows plus an `entity_tag_state … 'dead'` condition per tag. A corpse is out of the live set when the chain evaluates.
- Every chain matched by one event is evaluated against **one** pre-action snapshot, and their actions run in chain-id order. Chains in one batch never see each other's actions.

### K2. Guards

| Guard | Rule |
|---|---|
| Human | The Union Human gate (Union execution contract, Human gate): two `archetype` rows, `gte '1'` and `lte '4'`, on every 1563-1571 chain and every Human-only 1654/1655 chain. A missing archetype reads as -1 (`conditions/mod.rs:333-338`) and fails `gte '1'`, so a GM-created, unstamped player never enters the Union Human lines. This replaces the CS-05 form `lt '5'`, which passes -1. |
| Free Jaffa | `archetype eq 7`. Never widen chain 3029 (`gte 5`). |
| Both races | 1651, the 1654 marker and the 1655 Harriman beat carry no archetype row. Step state is the gate, because only the holder of the step can match. |
| World | `world` with target_id 58 on every `player_loaded`, `enter_region` and crossing chain. |
| One-shot | Accept chains gate on `mission_status … not_active`. Credit chains gate on the step being `active`. |
| Dispatchers | Every dispatcher stamps the player's archetype (CS-01b, `dialog_archetype_tests.rs`). The "engine fact 5" comment in `castle_706_708_chains.sql` is stale; don't copy it. |

### K3. Dialog rules

- **Close.** Closing a zero-button dialog sends `dialog_choice (id, -1)`. Closing a dialog with any button sends nothing.
- **Key.** A `dialog_choice` chain matches the dialog id alone, so every button fires it. Never key a blurb (5369, 5373, 5390, 5391, 5988): its More Info button would accept.
- **Binding.** A chain not fired by an interact can show only a **monologue** (every screen speaker 0: 5382, 5383, 5384). Any other dialog binds `last_interaction_target`, the NPC last clicked; a `dialog_choice` chain shows a dialog of the NPC just clicked.
- **R1.** A keyed dialog has zero buttons or a button on its final screen.
- **First press.** Every interact chain shows a dialog or flips a marker. A re-press after a lost close re-shows the dialog (precedent chain 3034).

| Dialog | Buttons | Keyed? |
|---|---|---|
| 5370 | Accept on screens 0-2 and 5-9 (final screen included) | yes, 1563 accept |
| 5374, 5392, 5393, 5854 | Accept on every screen or the final screen | yes, accepts |
| 5845 | type-4 "Receive Item" on all 5 screens | yes, 1651 generator |
| 5371 | one type-4 "Receive Item" | **never**: display only |
| 5855 | Accept on screens 0-2 only (fails R1) | **never** |
| 5372, 5375-5389, 5394-5396, 5844, 5852, 5853, 5856, 5857, 5875 | zero buttons | on close (-1) where a packet says so |

### K4. Linter obligations

- `every_interact_tag_chain_has_set_interaction_type` (F9): each packet's `player_loaded` restore chain satisfies it.
- `every_chain_region_key_matches_a_seeded_point_set`: region keys byte-match a name in `db/resources/Events/Seed/point_sets.sql` (the main file, one INSERT per line).
- `every_chain_seed_file_is_registered_in_database_sql`: one `\ir resources/Content/Seed/<file>` line per chain file (US-10).
- `dialog_button_linter` R1-R4 over `CHAIN_FILES` (US-10 adds the files).

### K5. Crossings (frozen with Dakara UD-00 and Omega UO-00)

| Row | Mission and step | Direction | Departs | Departure rows, in order | State on arrival | Receiver | Coordinate |
|---|---|---|---|---|---|---|---|
| C1 | 1570 4641 → 4642 → 4906 | 58 → Dakara_E1 61 | **SGC US-24** | `advance_step 1570 '4642'` @0, `play_sequence 10047` @0, `play_sequence 10060` @2500, `advance_step 1570 '4906'` @3000, `cross_world_teleport 'Dakara_E1'` @3000 | 1570/4906 active | DK-10b | gate 25 pin (96.87, -16.75, 243.23) |
| C2 | 1651 4930 | 61 → 58 | DK-30 (D-UD2) | `advance_step 1651 '4928'`, `cross_world_teleport 'SGC_W1'` | 1651/4928 active | **SGC US-30** | **H3 = P13 (172.79, -10.9, 12.0)** |
| C2b | 1651 4931 → 4929 → 5171 | 58 → 61 | **SGC US-30** | Carter's 5845 grants item 6784 and completes O5956 (safe: O6037 stays open, F13); Harriman's press on 4931 with O5956 done: `advance_step '4929'` @0 (force-completes O6037), sequences @0/@2500, `advance_step '5171'` @3000, teleport @3000; restore and recovery chains on 4929 | 1651/5171 active, item 6784 held | DK-30 | gate 25 pin |
| C3 | 1654 4939 | 61 → 58 | DK-33 | `complete_objective 1654 '5961'` (never `advance_step`), teleport | 1654/4939 active, O5962 open | **SGC US-31** | H3 |
| C4 | 1655 4944 → 4941 → 4940 | 58 → Omega_Site 18 | **SGC US-32** | `advance_step 1655 '4941'` @0, sequences @0/@2500, `advance_step 1655 '4940'` @3000, `cross_world_teleport 'Omega_Site'` @3000 | 1655/4940 active; Humans hold 6786 (D-US11) | Omega | Omega-published (UO-00); candidate gate 5 (34.705, -50.634, -149.092) |

- **H3, the SGC arrival point (CIMMERIA_RECONSTRUCTION):** the Gate Room floor in front of gate 27. The gate row (172.79, -8.47, 6.014) sits 2.4 m above the floor (y -10.9). SGC_W1 is instanced and `validate_gate_arrival` skips instanced worlds, so nothing at runtime catches a bad y. US-01 seals P13 before US-30 uses it.
- Telemetry exists for every crossing: INFO `Content: cross-world teleporting entity` (entity, account, player, world, position, chain_id, chain_name) and ERROR `CrossWorldTeleport: cell→base GateTravel send failed` with `reason = "base_channel_closed"`. US-14 adds the tests that pin both rows.
- Answers to Dakara:
  - **H1:** yes, a visible dial (sequences 10047/10060) before C1, C2b and C4, subject to D-US4.
  - **H3:** P13 above.
  - **H5:** a returning player finds no Gate Room Jaffa: US-34 despawns `SGC_W1_GroomJaffa1-3` once 1568 is completed and for every Free Jaffa. Hammond (`SGCW1_GenHammond`) is home at spawn 52, and Carter stands in the Gate Room (P08) during 1651/4931.
  - **H-RELOG:** every packet's `player_loaded` chains restore markers and NPC positions per step; US-34's matrix test proves it.

### K6. Telemetry every runtime packet names in its acceptance

| Event (log body) | Level | Attributes |
|---|---|---|
| `fire_chain_by_id: executing` | INFO | chain_id, chain_name, entity_id, action_count |
| `Content: accepting mission` / `accept_mission refused by offer guard` | INFO | mission_id, mission_name |
| `Content: advancing step` | INFO | step_id, step_name |
| `Content: complete objective` / `final required objective completed the mission` | INFO | objective_id, objective_name |
| `Content: displaying dialog` | INFO | dialog_id, chain_id, npc_entity_id |
| `DisplayDialog: no NPC entity id …` | WARN | dialog_id, chain_id: an NPC dialog keyed on a non-interact chain |
| `Content: playing sequence` / `PlaySequence: cell→base send failed` | INFO / WARN | sequence_id, sequence_name, chain_id |
| `Content: cross-world teleporting entity` / `… GateTravel send failed` | INFO / ERROR | world, position, chain_id; `reason=base_channel_closed` |
| `no chains matched` | DEBUG | event: a press that did nothing |
| `spawn_entity: …` | WARN | chain_id, template_id, tag |

The SigNoz check for UAT: filter `service.name = cimmeria-cell` and `chain_id IN (<packet range>)`; expect one `fire_chain_by_id: executing` per press and no WARN from that chain range.

## ID allocation (US-00 records it after a collision re-check)

D-US1 is decided by the architect (Dakara D-UD6 precedent). The coordinator reviews the row in the campaign PR.

| Kind | Block | Sub-use |
|---|---|---|
| chains | 3051-3199 | per packet, table below; 3178-3179 spare; 3180-3199 US-40 (HELD) |
| spawnlist | 15800-15849 | 15800 Harriman, 15801-15802 reserved (F12), 15803 Carter terminal, 15804 Failsafe Device, 15805-15810 crates |
| point_sets / point ids | 15800-15819 / 15800-15899 | 15800 `SGC_W1.FailsafeRoom`, 15801 `SGC_W1.GateControl` |
| entity_templates | 5800-5809 | 5800 Failsafe Device, 5801 reserved (F12), 5802 pressable crate, 5803 Carter terminal |
| dialogs | 58000-58049 | US-40 only, HELD |

| Packet | Chain file (`db/resources/Content/Seed/`) | Chains | Test module (`chain_replay_tests/union_sgc/`) |
|---|---|---|---|
| US-12 | `sgc_w1_union_1563_chains.sql` | 3051-3057 (3058-3059 spare) | `virus_1563.rs` |
| US-15 | `sgc_w1_union_1564_chains.sql` | 3060-3071 (3072 spare) | `failsafe_1564.rs` |
| US-16 | `sgc_w1_union_escort_chains.sql` | 3073-3078 (3079 spare) | `escort_1564.rs` |
| US-13 | `sgc_w1_union_1571_chains.sql` | 3080-3081 (3082-3084 spare) | `aux_1571.rs` |
| US-20 | `sgc_w1_union_tactics_chains.sql` | 3085-3093 (3094 spare) | `tactics_1565_1566.rs` |
| US-21 | `sgc_w1_union_1567_chains.sql` | 3095-3099 | `clear_way_1567.rs` |
| US-22 | `sgc_w1_union_1568_chains.sql` | 3100-3109 | `gate_room_1568.rs` |
| US-23 | `sgc_w1_union_1569_chains.sql` | 3110-3122 | `ordnance_1569.rs` |
| US-24 | `sgc_w1_union_1570_chains.sql` | 3123-3131 (3132 spare) | `depart_1570.rs` |
| US-30 | `sgc_w1_union_1651_chains.sql` | 3133-3142 (3143-3147 spare) | `generator_1651.rs` |
| US-31 | `sgc_w1_union_1654_chains.sql` | 3148-3152 (3153-3155 spare) | `turnin_1654.rs` |
| US-32 | `sgc_w1_union_1655_human_chains.sql` | 3156-3165 | `letter_1655_human.rs` |
| US-33 | `sgc_w1_union_1655_jaffa_chains.sql` | 3166-3169 (3170 spare) | `letter_1655_jaffa.rs`, `disjoint.rs` |
| US-34 | `sgc_w1_union_return_chains.sql` | 3171-3175 (3176-3177 spare) | `return_state.rs` |
| US-40 | `sgc_w1_union_authored_chains.sql` (creates and registers its own) | 3180-3199 | `authored.rs` |

These blocks are clear of the other three Union zones. Each zone's own ledger is the source (recheck there; this list is a copy):

| Zone | Blocks | Source |
|---|---|---|
| Dakara | the DK campaign's 8001-8499 (chains), templates 440-479, spawns 8000-8099, point sets 2123-2199, respawners 611-619; Union block 63500-63799 for every table | `dakara-work-packets.md` § ID block |
| Omega | 63800-63999 for every table (chains 63801-63999, templates and spawns 63800-63899 and 63880-63959, dialog sets and maps 63800-63803, item lists 63802-63819, item list items 638000-639999); respawner 180 | `omega-work-packets.md` § ID map (D-UO1) |
| Beta | chains 23000-23499, spawns 23000-23199, point sets 23000-23099, points 23000-23299, templates 2300-2349, respawners 2300-2309, authored rows 64000-64499 | `beta-work-packets.md` § ID block (D-UB9) |

The older SGC list (Omega 12000-12399, 63000-63099, 64000-64199, templates 1700-1739, spawns 15000-15099, point sets 2300-2329, items 9300-9329) is superseded: its 64000-64199 overlapped Beta's authored block, and its point-set range 2300-2329 overlapped Beta's templates and respawners 2300-2349 and 2300-2309 (different tables, so only a documentation clash). `id-allocation.md` is coordinator-owned; US-00 files a request for these rows and does not edit that file.

### Placement points (US-01 seals each; all CIMMERIA_RECONSTRUCTION)

Packets use the values US-01 records in `docs/analysis/union-sgc-continuation/placement.md`. Until then these are the proposals.

| Point | Use | Proposed (x, y, z) | Source |
|---|---|---|---|
| P01 | Harriman, Control Room chair, heading 3.1416 | (172.082, -6.399, 46.335) | chair mesh yaw -180 |
| P02 | Woolsey, next to Hammond | (197.5, 1.311, 43.6) | Hammond spawn 52 |
| P03 | reserved: dropped by F12 (Carter hands the notes over herself) | — | — |
| P04 | Carter virus terminal | (-49.48, 2.32, 36.38) | ViewScreen02 mesh |
| P05 | Carter at the Failsafe, end of the escort | (147.0, 1.30, -197.0) | Failsafe mesh (151.18, 1.28, -193.38) |
| P06 | Failsafe Device | (151.184, 1.28, -193.376) | EM-SGC_Failsafe00 mesh |
| P07 | FailsafeRoom region centre (cylinder r 8, h 6) | (150.0, 1.3, -196.0) | Failsafe room |
| P08 | Carter in the Gate Room (1651) | (170.5, -10.9, 12.5) | Gate Room floor |
| P09 | GateControl region centre (cylinder r 10, h 6) | (173.0, -6.4, 55.0) | Control Room x 156-190, z 44-70 |
| P10a-f | crates, pool A at y -10.95 | (257.157, 41.74), (259.131, 42.326), (260.237, 43.42), (255.923, 43.406), (257.726, 43.679), (254.369, 43.338) | requisitions room |
| P11, P12 | 1566 Jaffa pair, between rows 57/58 and the Failsafe room | (60.0, 1.311, -120.0), (64.0, 1.311, -124.0) | placeholders: US-01 chooses |
| P13 | H3 arrival | (172.79, -10.9, 12.0) | K5 |
| P14 | Hammond home (spawn 52; no seal needed) | (199.989, 1.311, 43.608) | spawnlist row 52 |
| P15 | player after 1568 (Hammond's office) | (182.0, 1.311, 43.5) | briefing room floor |
| P16 | Carter after 1568 | (179.5, 1.311, 45.5) | beside P15 |

## Waves and packets

| Wave | Packet | Role | Size | Scope | Depends on | Status | File |
|---|---|---|---|---|---|---|---|
| S0 | US-00 | `mission-systems-advisor` | docs ~30k | Ledger, ID check, crossings handshake | dispatch gate | BlockedDependency | [US-00](sgc-packets/US-00.md) |
| S0 | US-01 | `movement-teleport-advisor` + `npc-ai-spawn-advisor` | docs ~35k | Seal P01-P16 and H3 | US-00 | BlockedDependency | [US-01](sgc-packets/US-01.md) |
| S0 | US-02 | `game-archaeology-specialist` | docs ~35k | Meshes, name_ids, sequences 10047/10060, crate room | US-00 | BlockedDependency | [US-02](sgc-packets/US-02.md) |
| S1 | US-10 | `rust-gameserver-dev` (Sonnet; owns the sealed test rig) | ~35k | Scaffold: 14 empty chain files, `\ir`, CHAIN_FILES, test module, docs anchor | US-00 | BlockedDependency | [US-10](sgc-packets/US-10.md) |
| S1 | US-11 | `packet-coder` | ~35k | Templates, spawns, point sets, Jaffa tags | US-10, US-01, US-02 | BlockedDependency | [US-11](sgc-packets/US-11.md) |
| S1 | US-14 | `packet-coder` | ~20k | Crossing log tests (`executor/tests/teleport.rs`) | US-00 | BlockedDependency | [US-14](sgc-packets/US-14.md) |
| S2 | US-12 | `packet-coder` | ~40k | 1563 Virus | US-10, US-11 | BlockedDependency | [US-12](sgc-packets/US-12.md) |
| S2 | US-13 | `packet-coder` | ~20k | 1571 auxiliary credit | US-10 | BlockedDependency | [US-13](sgc-packets/US-13.md) |
| S2 | US-15 | `packet-coder` | ~40k | 1564 Self-Destruct, without the walk | US-10, US-11 | BlockedDependency | [US-15](sgc-packets/US-15.md) |
| S2 | US-20 | `packet-coder` | ~40k | 1565 + 1566 Tactics | US-10, US-11 | BlockedDependency | [US-20](sgc-packets/US-20.md) |
| S2 | US-21 | `packet-coder` | ~30k | 1567 Clear the Way | US-10, US-11 | BlockedDependency | [US-21](sgc-packets/US-21.md) |
| S2 | US-22 | `packet-coder` | ~40k | 1568 Gate Room | US-10, US-11 | BlockedDependency | [US-22](sgc-packets/US-22.md) |
| S3 | US-16 | `rust-gameserver-dev`, movement sign-off | ~40k | 1564 escort on 4631 | US-15 | BlockedDependency | [US-16](sgc-packets/US-16.md) |
| S3 | US-23 | `packet-coder` | ~35k | 1569 Ordinance crates | US-10, US-11 | BlockedDependency | [US-23](sgc-packets/US-23.md) |
| S3 | US-24 | `packet-coder`, movement sign-off | ~40k | 1570 offer and C1 | US-10, US-14, US-02, D-US4 default | BlockedDependency | [US-24](sgc-packets/US-24.md) |
| S3 | US-30 | `packet-coder`, movement + items sign-off | ~40k | 1651 C2 arrival, generator, C2b | US-10, US-11, US-01, UD-00 | BlockedDependency | [US-30](sgc-packets/US-30.md) |
| S3 | US-31 | `packet-coder` | ~30k | 1654 Hammond turn-in, both races | US-10, UD-00 | BlockedDependency | [US-31](sgc-packets/US-31.md) |
| S3 | US-32 | `packet-coder`, movement sign-off | ~40k | 1655 Human and C4 | US-10, US-11, UO-00, D-US11 default | BlockedDependency | [US-32](sgc-packets/US-32.md) |
| S4 | US-33 | `packet-coder` | ~35k | 1655 Free Jaffa + disjointness suite | US-31, US-32 | BlockedDependency | [US-33](sgc-packets/US-33.md) |
| S4 | US-34 | `packet-coder` | ~30k | Return state and relog matrix | US-11, US-20, US-21, US-22, US-30, D-US5 default | BlockedDependency | [US-34](sgc-packets/US-34.md) |
| S4 | US-U1 | lab UAT (ask the user) | lab | Human 1562 → 1570 → Dakara arrival | S3 integrated | UATPending | [US-U1](sgc-packets/US-U1.md) |
| S4 | US-U2 | lab UAT (ask the user) | lab | 1651 round trip, 1654, 1655 for both races | US-33, US-34 | UATPending | [US-U2](sgc-packets/US-U2.md) |
| S4 | US-40 | `packet-coder` | ~30k | Authored ambience, droppable | **D-US2, D-US3** | **BlockedDecision (HELD)** | [US-40](sgc-packets/US-40.md) |
| S4 | US-99 | `documentation-writer` | docs ~30k | Close-out: status docs, unified UAT | all others | BlockedDependency | [US-99](sgc-packets/US-99.md) |

**Specialist vs Haiku.** S0, US-16 and US-99 go to the named domain agents. Every other code packet is a Haiku `packet-coder` packet: the SQL is written out row by row in the packet, and the tests are named with their setup and assertions. One `packet-review-fixer` reviews each wave (S1 three packets, S2 six, S3 six, S4 two or three). Add `server-authority-enforcer` as a read-only second pass on US-23, US-30 and US-32 (item grants), and `movement-teleport-advisor` on US-16, US-22, US-24, US-30 and US-32 (moves and crossings).

**Docs during the campaign.** US-10 adds a "Union continuation" subsection to `docs/content/mission-chains.md` with one placeholder line per packet, each separated by a blank line. Each runtime packet rewrites **only its own line**, so parallel packets merge cleanly. The per-packet UAT steps go in the packet's report; US-99 folds them into the zone UAT row of `docs/guides/unified-uat.md`.

## Decisions

| ID | Question | Owner | Default if unanswered | Status |
|---|---|---|---|---|
| D-US1 | ID blocks above | Architect | as listed | Decided; US-00 records it after the re-check |
| D-US2 | Label policy for reconstructed and authored content shown to players | Owner | — | **BlockedDecision**: US-40 copy is HELD |
| D-US3 | U-D07 scale of authored content | Owner | none authored | **BlockedDecision**: US-40 HELD |
| D-US4 | Visible dial before each crossing (sequences 10047/10060) | Owner (maintainer) | play both sequences; drop the rows if no | **BlockedDecision**, non-blocking: US-24, US-30 and US-32 carry the rows behind one comment block |
| D-US5 | Return state: what a player returning through C2/C3 sees | Owner | no Gate Room Jaffa once 1568 is done; no story Jaffa for a Free Jaffa; Carter in the Gate Room during 4931 | **BlockedDecision**, non-blocking: US-34 |
| D-US6 | Walk into gate 27 to cross, instead of crossing on Harriman's interact | Owner | cross on the Harriman interact after the dial | **BlockedDecision**, non-blocking |
| D-US7 | Virus hack minigame or plain interact (O5369) | Owner | plain interact | **BlockedDecision**, non-blocking: US-12 |
| D-US8 | 1565 flank objective O5375: untracked, or credit it with `player_flanked_npc` (precedent `mission_681_686_flank.rs`) | Owner | untracked (optional objective) | **BlockedDecision**, non-blocking: US-20 |
| D-US9 | Edit the gate 27 row, or carry coordinates in the chains | Architect | chains carry H3; the row is untouched | Decided |
| D-US10 | Static rows 57/58 and 59-63 duplicate the 1565/1567 Jaffa. **A**: chain-spawn story Jaffa beside them. **B**: tag the static rows as the story Jaffa. **C**: delete the rows. Rows 71/77 (1562 attackers) stay in every option. | Owner | **B** | **BlockedDecision**, non-blocking: US-11 step 5, US-20, US-21 |
| D-US11 | 1655 letter item at departure | Owner + Omega | 6786 to Humans on Hammond's 5856; nothing to the Free Jaffa (H-O6) | **BlockedDecision**, shared with Omega UO-00: US-32 |
| D-UD2 | Who owns the 1651/4930 departure | Dakara (D-UD2) | DK-30 | SGC accepts the DK default |

## HELD items and how they were resolved

| HELD | Question | Resolution | Answered by |
|---|---|---|---|
| H1 offers | Who grants 1563-1569? | 1563 on Carter's 5370 after 1562; 1564 on Carter's 5374 after 1563; **1565 on the close of 5375** (the 1564 radio report); 1566 on the 1565 completion; **1567 on the 1566 completion**; **1568 and 1569 on the 1564 completion**. RECONSTRUCTION from the step text order. | architect |
| H1 1567 | Does 1567 gate 1568? | No: `story=false`, auxiliary | architect |
| H2 | 1571 offer | Auto-accept and credit on `mission_completed 1559` for Humans, with no second weapon | architect; US-13 returns `blocked` if the offer guard refuses it |
| H3 | 1569 completion and the 1570 offer | All six required objectives (seed `is_optional=false`); five crates show 5385-5389, the sixth (O5381) only flips its marker. 1570 is offered by Hammond **or** Teal'c ("?") once 1568 and 1569 are both done, and accepted on 5392 or 5393. | architect |
| H4 | 1651/4930 | DK-30 (D-UD2) | Dakara |
| H5 | Harriman | Template 58 exists (speaker 843); new spawn 15800 at P01; O6037 says "Control Room" | architect + US-01 |
| H6 | Radio reports O5370, O5373 | No radio object: zero-button 5375 (Hammond + Carter) and 5378 shown at Carter, keyed on close | architect |
| H7 | Jaffa positions, counts and tags | 1565: rows 57/58 tagged (D-US10 B); 1566: two chain spawns at P11/P12; 1567: rows 59-63 tagged; 1568: existing `SGC_W1_GroomJaffa1-3` | architect + US-01 |
| H8 | Regions | Two new cylinder sets 15800/15801 (F5); the flank region is D-US8 | US-01 |
| H9 | 1654 completion, 1655 offer | Hammond 5852 (Human) or 5853 (Free Jaffa) close completes O5962 and so 1654; Woolsey spawns at P02 with "?" on `mission_completed 1654` | architect |
| H10 | 1651 generator | 5845 type-4 button → `add_item 6784` (container 2) and `complete_objective 1651 '5956'` (O6037 stays open, so the step holds). Harriman's press then runs `advance_step 1651 '4929'`. Never `complete_objective 6037`: it would be the last open required objective of 4931 and would complete the whole mission (F13) | architect |
| H11 | Gate 27 geometry | 2.4 m above the floor; H3 on the floor; D-US9 | architect + US-01 |
| HELD-A | Hidden optional objectives O5830, O5831, O5355, O5356, and the 1565 flank O5375 | Left uncredited (hidden or optional). No source says what O5830/O5831/O5355/O5356 are. | open; no owner action |
| HELD-B | All new player-facing text, and Carter's 5394 | US-40 only, behind D-US2/D-US3 | owner |
| H-REPRESS | A second press of Hammond's 5392/5393 (US-24) inside the 3 s dial window, or any press that lands while the 3131 recovery chain also matches, can run `cross_world_teleport` twice. Is a second `Travel` acceptable, or must the C1 chain be guarded? | US-24 characterises the double teleport in a test and does not fix it. A guard needs a design call: either a once-per-window condition the engine does not have, or a state the first press sets and the second press reads (a new step or objective, which changes the 1570 step ladder). | owner + `movement-teleport-advisor` |

## Seal each implementation child

US-00 supplies the exact chain/mission/dialog IDs, actor tag, map coordinate with placement class, file allowlist, test name and revert proof, telemetry events, lane commands and source build before dispatch. A mission step from a cooked record is `RESTORED_SOURCE`; a chosen offer, position or encounter may be `CIMMERIA_RECONSTRUCTION`; new briefing text is `CIMMERIA_AUTHORING`. Use `db/resources/` seeds and a worktree database. Runtime packets update `docs/content/mission-chains.md` and the zone UAT row; status docs wait for campaign close-out. The specialist signs travel/teleport, mission and item packets; server authority review checks every client-fed credit or item grant.

**Stop the SGC campaign** if the Human cannot leave via an accepted 1570, if a Free Jaffa gains Human tutorial credit, if the 1651 generator/state round trip is lost, or if Omega cannot distinguish the two 1655 letter recipients. A passing SGC-only fixture is an integration checkpoint, not route completion.
