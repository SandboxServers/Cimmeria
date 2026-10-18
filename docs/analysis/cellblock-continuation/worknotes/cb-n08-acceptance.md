# CB-N08a: escape acceptance reconciliation

> **Type:** Reference (worknote)
> **Audience:** The close-out writer (CB-N08b), the campaign reviewer (CB-N08-R) and the owner
> **Last updated:** 2026-10-17
> **Companions:** [README.md](README.md), [owner-lab-checklist.md](../owner-lab-checklist.md), [route-baseline.md](../route-baseline.md#bounded-fresh-acceptance-rows), [presentation-handoff.md](../presentation-handoff.md), [CA-U scenarios](../../cellblock-asset-audit/uat-guide.md), [w4-packets.md § CB-N08a](../w4-packets.md#cb-n08a--uat-reconciliation)

Static reconciliation at branch `content/cellblock-campaign`, tip `3a6b3cc64` (W5-INT). No lab was driven, no server started, no build run.

## Evidence rule

- **Owner lab not run (O14-09).** Owner directive, 2026-10-17: no agent drives the lab; the owner runs CB-U03, U05, U08, U09, U10, U11 and U12 and the route-baseline rows U01, U02, U04, U06, U07 himself from [owner-lab-checklist.md](../owner-lab-checklist.md). He has run none yet. Every client-dependent row below is `UNTESTED: owner lab not run (O14-09)`.
- **The earlier 2026-10-17 CB-U08 agent runs are invalid** (two servers listened on auth port 13001) and are not evidence for any row.
- **Static evidence** (live-DB, executor replay, resolve, cooked-sync, round-trip and seed-agreement tests, named in the last section) shows the server emits the right state, dialogs, catalog rows and grants. It never shows what the client drew, so it never turns a client row into `pass`.
- **Decided:** D-CC18 (CB-N06 included, labeled new Cimmeria content) and D-CC19 (CB-N07b omitted; the CB-N07a Tier 0 bark stays).
- None of the tests below were run in this packet. They are cited by name from the integrated branch; their pass state is the lane results recorded in the wave reports, not a new claim.

## Acceptance matrix

State is `UNTESTED: owner lab not run (O14-09)` in every row unless a different state is written.

### Escape scenarios (CB-N08 contract)

| Scenario | Covering owner row | Static server evidence (not a client pass) | State |
|---|---|---|---|
| Fresh Human full run, stasis pod to 688 exit | CB-U09 step 1, CB-U01/02/04/06/07 | Chain-replay families `mission_622*`, `mission_638`, `mission_639*`, `mission_640` to `mission_688` (pre-existing); 638 and 639 extended this campaign | UNTESTED: owner lab not run (O14-09) |
| Fresh Jaffa full run | CB-U09 step 2, CB-U10 | `live_db_jaffa_region2_entry_resolves_exactly_one_dialog_bind_and_accept`, `live_db_chain_1012_routes_jaffa_to_jaffa_dialog_set` (`mission_638.rs`); `live_db_mission_62050_replay_marsh_completes_beside_core_archetype_8` | UNTESTED: owner lab not run (O14-09) |
| Another archetype where class branches differ | CB-U10 and CB-U12 (Human and Loyalist Jaffa only); CB-U07 (class crate) | Archetype 1 and 8 replays for 62050 and 62060 (`executor.rs`); class-crate reward split belongs to class-start CS-04 | UNTESTED: owner lab not run (O14-09). No third archetype is on any owner row: a gap to accept or schedule. |
| Skip both optional stories (62050 and 62060) | CB-U10 step 7, CB-U12 step 8, CB-U09 step 1 | `live_db_mission_62050_replay_duplicate_and_skip`; `live_db_mission_62060_replay_abandon_and_641`; `live_db_mission_62050_no_dialog_path_moves_the_mission`, `live_db_mission_62060_no_dialog_path_moves_the_mission` | UNTESTED: owner lab not run (O14-09) |
| Accept each story | CB-U10 step 2, CB-U12 step 3 | `live_db_mission_62050_replay_offer_accept_and_report`; `live_db_mission_62060_replay_offer_accept_shows_the_choice_archetype_1` and `_8`; `live_db_mission_62050_dispatch_prisoner_click_in_range_accepts` | UNTESTED: owner lab not run (O14-09) |
| Take each 62060 branch once (aid, sabotage) | CB-U12 steps 4 and 5 | `live_db_mission_62060_replay_aid_grants_once`, `live_db_mission_62060_replay_sabotage_grants_once`, `live_db_mission_62060_one_branch_per_click`, `live_db_mission_62060_round_trip_aid_then_panel_grants_two_once` | UNTESTED: owner lab not run (O14-09) |
| Relog before and after key transitions | CB-U03 (638/639 restores), CB-U10 step 4, CB-U11 step 3, CB-U12 step 6 | `live_db_mission_638_login_restores_only_active_archetype_dialog`, `live_db_mission_638_login_restores_door_button_bit_only_at_step_2115`, `live_db_chain_1036_restores_vial_bit_only_at_step_2145`, `live_db_chain_1037_rearms_drone_only_while_kill_open_at_step_2144`, `live_db_mission_62050_progress_survives_relog_per_character`, `live_db_mission_62060_round_trip_relog_keeps_the_outcome_per_character`, `live_db_mission_62050_replay_relog_restores_offer_once` | UNTESTED: owner lab not run (O14-09) |
| Death before and after key transitions | No owner row exists | None in the campaign test set (death is not exercised by any CB test) | UNTESTED: no owner row and no test. Gap. |
| Two-client AoI | CB-U08 step 4 | `live_db_cellblock_ambience_barks_fire_once_in_their_phase` (per-player bark scope, server side only); `live_db_cellblock_ambience_cast_rows_are_placed_safely` | UNTESTED: owner lab not run (O14-09) |
| Boots Livewire | CB-U01 | Pre-existing `mission_689.rs`, `mission_689_hidden_frames.rs` (not changed by this campaign) | UNTESTED: owner lab not run (O14-09) |
| Marsh escort | CB-U04 | Pre-existing `mission_680.rs` (not changed by this campaign). `live_db_mission_62050_marsh_completes_b` shows the core Marsh briefing is not displaced by 62050. | UNTESTED: owner lab not run (O14-09) |
| Combat and flank | CB-U05, CB-U02 | `other_cover_guards_spawn_holding_their_marker`, `other_cover_guards_take_cover_stance_against_a_threat_in_front`, `other_cover_guards_release_when_flanked` (`npc_ai_cover_seed_guards.rs`, spawns 28, 32, 33 on the real navmesh and cover seed). Pre-existing `mission_681_686_flank.rs`. | UNTESTED: owner lab not run (O14-09) |
| First-press feedback | CB-U03, CB-U10 step 1, CB-U11, CB-U12 step 2 | `live_db_mission_62050_offer_gates`, `live_db_mission_62060_offer_gates` show the offer is bound at login. The client drawing it on the first press is not shown by any test. | UNTESTED: owner lab not run (O14-09) |
| Castle boundary | CB-U07 | Pre-existing `mission_688.rs`; ring chain 1109 and Castle CS-08 own the transfer | UNTESTED: owner lab not run (O14-09) |
| Barracks alarm bark (D-CC19 Tier 0) | CB-U11 | `live_db_cellblock_barracks_response_barks_once_while_2355_is_active`, `..._refuses_the_crate_and_cleared_phases`, `..._refuses_another_room` | UNTESTED: owner lab not run (O14-09) |
| Cooked-sync reaches a stock client (62050 and 62060) | CB-U10 step 8, CB-U12 step 10 | `remaining_pods_mission_reaches_a_stock_client`, `remaining_pods_dialog_reaches_a_stock_client`, `prisoner_choice_mission_reaches_a_stock_client`, `prisoner_choice_dialog_reaches_a_stock_client`. These show the served bytes are the sealed bytes; a stock client opening dialog 60150 or 60151 without a crash is CB-U10/U12. | UNTESTED: owner lab not run (O14-09) |

### Presentation rows CA-U01 to CA-U21

The asset-audit guide ([cellblock-asset-audit/uat-guide.md](../../cellblock-asset-audit/uat-guide.md)) defines these. CB-N08 joins them at the route boundary; the asset and ring owners keep their own packets. This campaign changed no presentation, so no CA-U row has campaign evidence. Visual rows cannot pass from a dispatched sequence or a log ([presentation-handoff.md](../presentation-handoff.md)).

| Row | Subject | State |
|---|---|---|
| CA-U01 | Wake-up, corpse pickup, first weapon | UNTESTED: owner lab not run (O14-09) |
| CA-U02 | 329 door and minigame | UNTESTED: owner lab not run (O14-09) |
| CA-U03 | Ambernol on the counter | UNTESTED: owner lab not run (O14-09) |
| CA-U04 | Desk cover | UNTESTED: owner lab not run (O14-09) |
| CA-U05 | Internal ring controls | UNTESTED: owner lab not run (O14-09) |
| CA-U06 | Preparation locker weapon | UNTESTED: owner lab not run (O14-09) |
| CA-U07 | Preparation_Terminal and pods | UNTESTED: owner lab not run (O14-09) |
| CA-U08 | Escape barrier | UNTESTED: owner lab not run (O14-09) |
| CA-U09 | Mess Hall table and cover | UNTESTED: owner lab not run (O14-09) |
| CA-U10 | Straegis scene | UNTESTED: owner lab not run (O14-09) |
| CA-U11 | Straegis from an observer | UNTESTED: owner lab not run (O14-09) |
| CA-U12 | Aftermath blood | UNTESTED: owner lab not run (O14-09) |
| CA-U13 | Aftermath crate | UNTESTED: owner lab not run (O14-09) |
| CA-U14 | Armory terminal and doors | UNTESTED: owner lab not run (O14-09) |
| CA-U15 | Armory exit switch, World 12 to 8 | UNTESTED: owner lab not run (O14-09) |
| CA-U16 | Repeat and invalid inputs | UNTESTED: owner lab not run (O14-09). The server half is covered by the dispatch refusals (`live_db_mission_62050_dispatch_far_click_refused`, `..._wrong_and_missing_entity_refused`, `..._cross_space_refused`, `..._forged_dialog_choice_refused`, `..._stale_initial_response_refused`, `live_db_mission_62060_round_trip_far_click_refused`). |
| CA-U17 | Relog around transitions | UNTESTED: owner lab not run (O14-09) |
| CA-U18 | Stream out and in, late observer | UNTESTED: owner lab not run (O14-09) |
| CA-U19 | Two testers, different phases | UNTESTED: owner lab not run (O14-09) |
| CA-U20 | Complete tutorial through World 12 to 8 | UNTESTED: owner lab not run (O14-09) |
| CA-U21 | Editor or frontend change | NOT APPLICABLE: this campaign changed no editor or frontend code (`git diff --stat main...HEAD -- crates` touches no `cimmeria-app`, content-editor or scene-editor path) |

### Class-start and ring travel

| Surface | Owner | State |
|---|---|---|
| Class gear and M687 five-way crate reward | Class-start CS-04 (integrated #1278), CS-08 | UNTESTED: owner lab not run (O14-09). CS-08's earlier partial pass covers only the opening pistol, reload and relog, not the full route. |
| Armory ring, chain 1109, World 12 to 8 | Ring campaign (Phase 1 built, UATPending) | UNTESTED: owner lab not run (O14-09). Installed client-patch lineage unproven. |

## Route-baseline rows

Definitions: [route-baseline.md § bounded fresh acceptance rows](../route-baseline.md#bounded-fresh-acceptance-rows). The owner has given no result for any of them.

| Row | What it proves | State |
|---|---|---|
| CB-U01 | Boots lock, native Livewire, unlock, relog | UNTESTED: owner lab not run (O14-09) |
| CB-U02 | Pistol 55 and ability 592: grant, equip, reload, two shots, guard damage | UNTESTED: owner lab not run (O14-09). A 2026-10-15 fresh Human run recorded in the README (CB-N02 row) saw grant, equip, reload and guard damage, and #1667 did not reproduce; that was one run on an earlier build and does not close the row. |
| CB-U04 | Rings, Marsh first/repeat/relog escort, barks, terminal and pods | UNTESTED: owner lab not run (O14-09) |
| CB-U06 | Straegis scene, Marsh departure, dialogs 2516 then 5859 | UNTESTED: owner lab not run (O14-09) |
| CB-U07 | Class crate once, terminal 2356 to 80688, Castle arrival, 1360 retained | UNTESTED: owner lab not run (O14-09) |

## Owner rows for the new content

| Row | Subject | State |
|---|---|---|
| CB-U03 | Relog restores 638/639 interactions (chains 1026-1028, 1036, 1037) | UNTESTED: owner lab not run (O14-09) |
| CB-U05 | Mess Hall and Hallway05 flank and cover | UNTESTED: owner lab not run (O14-09) |
| CB-U08 | Ambient cast placement, once-only bark, AoI | UNTESTED: owner lab not run (O14-09); the 2026-10-17 agent runs are invalid |
| CB-U09 | Core run 30-45 minutes; optional beats 10 minutes together (O14-14) | UNTESTED: owner lab not run (O14-09). No pacing number exists. |
| CB-U10 | Mission 62050 in the native client | UNTESTED: owner lab not run (O14-09) |
| CB-U11 | Barracks alarm bark, once, only during 687 step 2355 | UNTESTED: owner lab not run (O14-09) |
| CB-U12 | Mission 62060 (new Cimmeria content) in the native client | UNTESTED: owner lab not run (O14-09) |

## Spec check: `castle-cellblock.toml`

One row, T01/T02. Literals checked against the seeds on this branch:

| Literal in the spec | Seed evidence | Result |
|---|---|---|
| Dialog 2982, text "The last thing you remember" | `db/resources/Dialogs/Seed/dialogs.sql` has dialog 2982; `dialog_screens.sql` line 13017 holds screen 96106 starting "The last thing you remember is going into stasis"; chain 1001 `display_dialog` 2982 (`castle_cellblock_chains.sql` line 236) | Current |
| Mission 622 on step 2113 | `mission_steps.sql` line 5681: step 2113, mission 622, index 0; the SigNoz filter `622` and `2113` matches the `Content: accepting mission` line | Current |
| Frost's body searchable, the Guard's not yet | Chain 1003 on step 2113 (Frost), 1004 gated on step 80622 (Guard); `mission_622.rs` header | Current |

No literal is stale, so `castle-cellblock.toml` is unchanged. No lab row was added (owner directive).

## Issue #1667 (first-guard pistol damage)

`gh issue view 1667 --json state,title` (read-only, 2026-10-17): state **OPEN**, title "Pistol: no fire animation, no recoil, and no damage to the guard after pickup".

The contract treats it as a known blocker for CB-N02 and CB-N08. Against that, the README records a 2026-10-15 fresh Human run in which it did not reproduce. The issue stays open until the owner's CB-U02 run decides it. No new fix packet is opened here.

## Blockers

No CB-U10 or CB-U12 failure exists, because those rows have not been run. An `UNTESTED` row is not a blocker. The one real item is #1667 above (open, pre-existing, tracked).

## Test inventory

Source: `git diff --stat main...HEAD -- crates` (36 files, 5,511 insertions, 15 deletions). Counts are `#[test]` and `#[tokio::test]` attributes in the file as it stands; for the files marked "extended" the count is the whole file, not only the campaign's additions.

### New test files (72 tests)

| File | Tests | live_db |
|---|---:|---:|
| `crates/base-session/src/base/cooked_sync/tests/resync_cellblock.rs` | 2 | 0 (no DB) |
| `crates/base-session/src/base/cooked_sync/tests/resync_cellblock_62060.rs` | 2 | 0 (no DB) |
| `crates/cell-content/src/cell/content/chain_replay_tests/cellblock_ambience.rs` | 3 | 3 |
| `crates/cell-content/src/cell/content/chain_replay_tests/cellblock_barracks_response.rs` | 3 | 3 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_62050/executor.rs` | 6 | 6 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_62050/resolve.rs` | 5 | 5 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_62050/rows.rs` | 2 | 2 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_62060/executor.rs` | 6 | 6 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_62060/resolve.rs` | 7 | 7 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_62060/rows.rs` | 2 | 2 |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_639_drone_relog.rs` | 1 | 1 |
| `crates/cell-methods/src/cell/cell_methods/player/interaction/interact/mission_62050_tests.rs` | 7 | 7 |
| `crates/cell/src/cell/service/tests/npc_ai_cover_seed_guards.rs` | 3 | 0 (no DB) |
| `crates/resources/src/base/dialog_overrides/override_seed_agreement_cellblock_62050.rs` | 4 | 0 (no DB) |
| `crates/resources/src/base/dialog_overrides/override_seed_agreement_cellblock_62060.rs` | 4 | 0 (no DB) |
| `crates/resources/src/base/resources/tests/mission_additions_cellblock.rs` | 5 | 0 (no DB) |
| `crates/resources/src/base/resources/tests/mission_additions_cellblock_62060.rs` | 5 | 0 (no DB) |
| `crates/services/src/mission_62050_round_trip_tests.rs` | 2 | 2 |
| `crates/services/src/mission_62060_round_trip_tests.rs` | 3 | 3 |
| **Total** | **72** | **47** |

`overrides_cellblock.rs` (39 lines) holds the two served dialog constants and has no tests.

### Extended files

| File | Tests in file | Change |
|---|---:|---|
| `chain_replay_tests/mission_638.rs` | 8 (all `live_db_`) | Login-restore and archetype-routing guards for chains 1011, 1012, 1026-1028 |
| `chain_replay_tests/mission_639.rs` | 7 (all `live_db_`) | Chains 1036 (vial restore) and neighbours |
| `chain_replay_tests/na13_chain_armed_aggro.rs` | 4 (all `live_db_`) | `live_db_once_chains_match_the_seeded_set` now pins the nine seeded fire-once chains `[1008, 1026, 1027, 1044, 11001, 11010, 11020, 11031, 11032]` |
| `resources/tests/mission_additions.rs` | 12 | Empty-additions assertion became `[62050, 62060]` |
| `resources/tests/dialog_overrides.rs` | 6 | Overridden dialog ids now include 60150 and 60151 |
| `cell-methods/.../interaction/interact.rs` | 4 | Module line for `mission_62050_tests` and two seed-tag exemptions |
| `content-engine/tests/it/interact_tag_linter.rs` | 8 | Two exemption lines for the per-player bind chains 11003 and 11033-11037 |

### Naming check

Every test in a file that touches the database (`PgPool`, `require_db_or_skip!`) has `live_db` in its function name: 47 of 47 in the new files, and all of `mission_638.rs`, `mission_639.rs` and `na13_chain_armed_aggro.rs`. Files with no `live_db` tests (cooked-sync, seed-agreement, mission-additions, cover-guard tests) use no database.

### Mutation spot-check not done here

This packet is static and ran no tests, so no revert proof was repeated. The revert proofs are in the per-packet reports of the integrating waves.

## Gaps for CB-N08b

- **Death** before and after key transitions: no owner row and no test.
- **Third archetype:** only Human (1) and Jaffa (8) are exercised; class-branch differences (CB-U07) rest on the owner run.
- **Pacing (O14-14):** no number exists until the owner runs CB-U09.
- **Closing a row:** every row above stays `UNTESTED` until the owner records a result in the README row.
