# CB-N05 seal: The Remaining Pods

> **Type:** How-to (implementation seal)
> **Audience:** Cellblock coordinator, mission/authority/testing reviewers and packet coder
> **Last updated:** 2026-10-14
> **Companions:** [Cellblock ledger](../README.md), [packet contract](../../four-zone-planning/cellblock-work-packets.md#cb-n05--the-remaining-pods-first-authored-catalog-proof), [ID claims](../../four-zone-planning/id-allocation.md#cb-n05-individual-claims--2026-10-14), [mission chain reference](../../../content/mission-chains.md), [CB-N05a infrastructure](../../four-zone-planning/cb-n05a-seal.md)

**Review state: reviewed design seal at Cellblock integration base `71b470774d522786e53a8c5152f19b5e23697a68`.** Mission, authority, testing, packet and focused re-review closed on 2026-10-14. Coder dispatch remains `BlockedDependency` until [CB-U09](../route-baseline.md#bounded-fresh-acceptance-rows) measures a fresh normal-player 30–45 minute core run and shows this beat fits inside the 10-minute optional budget. O14-10 delegates bounded authored story and exact copy; O14-11 keeps the provenance label out of the player UI; O14-14 sets that pacing gate. This is `CIMMERIA_AUTHORING` in the seed comments and ledger. The 2009 client and deprecated server do not supply this quest. The setting uses the existing Prisoner 329 and Col. Marsh route; it does not assert a recovered line of dialogue.

## Reviewed source join

| Source at the stated base | Fact used here | Limit |
|---|---|---|
| Stock `CookedDataMissions.pak` and CB-N05a's `mission_additions.rs` / `apply_mission_additions.rs` | 62050–62056 are absent; a complete category-3 entry can be inserted and transferred. The validator accepts literal boolean `false` for `AwardXP`, icon flags and `IsOverrideMission` and requires these attributes. | Resource validation and a transport test do not prove rendered journal behavior. |
| `db/resources/Content/Seed/castle_cellblock_chains.sql` plus `db/resources/Dialogs/Seed/dialog_set_maps.sql` | The existing 638 completion route, Prisoner template 17/map 2795 and Marsh 1051/1052/1152 give the bounded trigger and co-firing join. | Seed references do not prove the player can click the marker or safely reach both NPCs. |
| `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs` and `crates/cell-content/src/cell/content/executor/mission.rs` | Target range/space validation precedes `interact_tag`; a real accept emits `mission_accepted`, and advance/complete actions execute server-side. | Negative method and DB round-trip tests must prove those edges with the new seed. |
| Stock `CookedDataDialogs.pak`, served `DIALOG_OVERRIDES` 3995/3996 and their seed rows | 60150/96110 are absent; one-screen, zero-button, speaker-0 UI type 2 is the served shape. | Prior authored multi-screen/NPC-speaker entries crashed the client; native 60150 rendering remains unproven. |

## Player result and content

The player can hear one more concern from Prisoner 329 after completing stock mission 638, then tell Col. Marsh while still in world 12. Both Human and Loyalist Jaffa stock paths reach 638. The optional conversation does not pause the core 638→641 route, add a kill, change the 641 briefing or require Castle travel. There is no XP, naquadah, item or repeat reward. The current Goa'uld 5–7 holding route is outside the first client proof; these six chains add no archetype exclusion.

| Field | Exact authored value |
|---|---|
| Mission 62050 title / `MissionDefn` | `The Remaining Pods` |
| `HistoryText` | `Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.` |
| Step A 62051, index 0 / `StepDisplayLogText` | `Ask Prisoner 329 about the other pods.` |
| Objective A 62052 / `DisplayLogText` | `Speak to Prisoner 329.` |
| Task A 62053 | Enabled task type 1, attached only to objective A |
| Step B 62054, index 1 / `StepDisplayLogText` | `Tell Col. Marsh there may be prisoners in the other pods.` |
| Objective B 62055 / `DisplayLogText` | `Warn Col. Marsh.` |
| Task B 62056 | Enabled task type 1, attached only to objective B |
| Dialog 60150, screen 96110 / exact visible text | `Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave.` |

Both objectives are required, nonhidden and have zero XP. Use the stock mission-638 shape for the two steps (`step_enabled=false`), two objectives (`is_enabled=false`, `is_hidden=false`, `is_optional=false`) and enabled type-1 tasks. The mission is level 1, difficulty 1, enabled, visible, story, not shareable, `MissionLabel=General`, `award_xp=false`, `can_abandon=true`, `can_fail=false`, `can_repeat_on_fail=false`, `num_repeats=0`, `is_override_mission=false`, all three show-icon flags `false`, `script_name=NULL`, `script_spaces=NULL`, `reward_xp=0`, `reward_naq=0`, and has no reward-group or item rows. The complete category-3 XML in `MISSION_ADDITIONS` must agree: `AwardXP="false"`, `CanAbandon="true"`, `NumRepeats="0"`, `IsOverrideMission="false"`, `ShowFactionChangeIcon="false"`, `ShowInstanceIcon="false"`, `ShowPVPIcon="false"` and the other root/child fields above. `apply_mission_additions.rs::attributes` permits `false` and requires these root boolean attributes; `script_name` and `script_spaces` are seed-only and have no supported XML attributes. Match the four seed row families and exact text; keep step declaration order A then B. If the loader refuses a field combination, stop and return to seal review rather than weaken validation.

Dialog 60150 is a **one-screen, zero-button, speaker-0 monologue**, `DUIST_DefaultDialog` (`ui_screen_type=2`, `dialog_flags=0`), with only screen 96110. Seed `dialogs` and `dialog_screens` and append the full category-5 entry to `DIALOG_OVERRIDES` in `crates/resources/src/base/dialog_overrides/mod.rs`; the new agreement test pins their exact match. The seed alone cannot make new text render in the client. Existing served dialogs 3995/3996 show the supported one-screen speaker-0 shape. The multi-screen/nonzero-speaker authored debug-hub category-5 entries remain quarantined after a client crash; this packet does not assume their shape is safe. The native client must prove 60150 renders and survives relog before this gate passes. The close event (`dialogButtonChoice(60150, -1)`) has **no content action**.

## Individual IDs and interaction surface

The [shared ID table](../../four-zone-planning/id-allocation.md#cb-n05-individual-claims--2026-10-14) claims mission 62050, step/objective/task 62051–62056, dialog 60150, screen 96110 and chains 11000–11005 by namespace. Stock mission nested IDs, stock dialog root and XML screen, four active zone seeds and resource registries had zero candidate hits at the dated scan. Recheck all four branches immediately before coding. Existing dialog-set-map **2795** already belongs to dialog set 603, has a null `dialog_id`, and carries an interaction flag. It is reused at NPC template slot 17, not newly claimed or changed. Its null dialog lets the validated `interact_tag` chain answer the first press without a bogus dialog or client button dependency. Verify that its marker is visible and clickable in the real client; the source shape alone is not that proof.

The native `interact` method validates target existence, same space and range before tag-chain dispatch (`crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`). The offer is a player-specific `add_dialog_set(2795, slot 17)` binding. Keep a player-specific `remove_dialog_set(2795, slot 17)` on acceptance; do not change the global NPC interaction flags or the legacy Prisoner dialog maps 2794/5866.

| Chain | Trigger | Required conditions | Executed actions, in order |
|---|---|---|---|
| 11000 | `mission_completed 638` | world 12; 638 completed; 641 not active; 62050 not active | Add map 2795 to slot 17 for this player. |
| 11001 | `player_loaded Castle_CellBlock` | Same four gates as 11000 | Restore map 2795 once for this player. |
| 11002 | `mission_abandoned 62050` | Same four gates, evaluated after mission removal | Restore map 2795 once for this player. |
| 11003 | `interact_tag Prisoner_329` | world 12; 638 completed; 641 not active; 62050 not active; validated in-range target | Accept 62050 only. No direct dialog or objective action here. |
| 11004 | nested `mission_accepted 62050` | world 12; A 62051 active | Advance to B 62054; remove map 2795 from slot 17; display dialog 60150, in that order. `advance_step` marks A's required objective complete before activating B. |
| 11005 | `interact_tag Preparation_ColMarsh` | world 12; 640 completed; B 62054 active; validated in-range target | Complete objective B 62055 only. Its sole required objective auto-completes 62050. |

No chain keys on `dialog_open`, `dialog_choice` or `initialResponse` for 62050. A forged close (`button_id=-1`) or other button ordinal therefore cannot accept or complete it. An `initialResponse` naming an old/stale map cannot reach either mission action. Core Marsh chains 1051/1052 still display the stock briefing on their own conditions; 1152 still displays its stock 641 prompt on acceptance. Chain 11005 adds no optional Marsh dialog or suppression. If the tag dispatcher does not execute both matching core and optional chains, stop and reseal rather than moving this action to a client dialog claim.

The offer stops once 641 is active. A player who skips the Prisoner click may continue 688 without the mission. A player who accepted before that transition can finish B at Marsh while 640 is completed. On abandonment, the offer returns only while 638 remains completed and 641 is not active; a completed one-time mission cannot reopen. The `mission_accepted` follow-up fires only after a real accepted transition; repeat clicks cannot replay the A→B/dialog path. Persistence is per character, not per account.

## Coder allowlist and proof

The exact allowed files are in the [CB-N05 packet](../../four-zone-planning/cellblock-work-packets.md#cb-n05--the-remaining-pods-first-authored-catalog-proof). Add no runtime handler, new verb, Castle recipient, item, reward row or client patch. A discovered handler flaw becomes a separate reviewed packet. Seed comments and the ledger carry `CIMMERIA_AUTHORING`; the dialog and journal do not.

1. Add exact mission/step/objective/task, dialog/screen and six chain rows. Register the complete mission and dialog entries. Keep each SQL chain ID, trigger, condition and action in the table above. Reuse existing mission/content logs for chain fire, step/objective advancement and refusal; any missing telemetry that needs a logger edit returns to separate review.
2. Add production resource guards for exact 62050 XML, IDs and copy and 60150/96110 seed/override agreement. In `crates/base-session/src/base/cooked_sync/tests/resync.rs`, start from independently loaded stock category-3 and category-5 client models and drive the real task/transport for both. Assert exactly one `(3, 62050)` and one `(5, 60150)` fragment head and bodies equal to **independent literal expected** mission XML and dialog XML; the expected bodies cannot come from the mutated server cache.
3. Replay 11000–11005 through the real loader **and executor**. Assert emitted mission update and dialog call, nested `mission_accepted` A→B, 2795 add/remove, Marsh B completion, and co-firing of 1051/1052/1152 where each stock gate applies. Include skip, duplicate, abandoned, completed, relog, wrong world, and both stock archetype routes. A resolve-only list of actions is insufficient.
4. Register `#[cfg(test)] mod mission_62050_tests;` inside the 442-line `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs` and put the test in `crates/cell-methods/src/cell/cell_methods/player/interaction/interact/mission_62050_tests.rs`. Rust resolves that child path from the file module. Do not add to the already 871-line `interaction/mod.rs`; its inline tests already exceed the 700-line cap. Use `require_db_or_skip!()` and public `build_engine(Some(&pool))`; name DB tests `live_db_mission_62050_*` so the live-DB group and packet filter both select them. Drive the real method and tagged target: in-range positive; far, wrong, missing and cross-space targets negative. Send forged dialog close/ordinal and stale `initialResponse`; assert zero 62050 mission actions. No private loader or dev feature change is needed. In `crates/services/src/mission_62050_round_trip_tests.rs`, use exact sentinel account/player IDs `0x7000_8F00`–`0x7000_8F03` for two characters, clean by exact ID and drive cell mission updates through the production cell-to-base dispatch to `sgw_mission`, then reload both players. Name this guard `live_db_mission_62050_*` too. Pin A→B, B completion, abandonment/reaccept, repeats and isolation. This is a true DB round trip, not a seed-only assertion. Revert each new registry, binding, condition and completion action in turn to prove a named guard fails.
5. Before dispatch, record CB-U09's normal first-run core duration and measured optional room, then confirm no other zone has completed the first authored catalog proof. If another zone has, the zero-reward exception has expired; keep dispatch blocked until a bounded normal reward or a specific approved exception is resealed and reviewed. After implementation, run [CB-U10](../route-baseline.md#bounded-fresh-acceptance-rows): first/repeat/relog/abandon/skip on native Human and Jaffa, journal title and A→B text, 60150 rendered text, objective/mission completion, server state, packet/log evidence and elapsed optional time. An observed blank dialog, client crash, missing first-press marker, lost progress or broken core briefing blocks the packet; it is not `UATPending`.

**Packet lane commands, in order:**

```powershell
cargo fmt --all -- --check
pwsh tools/build-lane/reload-db.ps1
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content -p cimmeria-cell-methods -p cimmeria-services --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources -p cimmeria-base-session -p cimmeria-cell-content -p cimmeria-cell-methods -p cimmeria-services
pwsh tools/build-lane/live-db-test.ps1 mission_62050
```

Run the revert guards through the lane, restore only the mutated file, rerun the affected guard, then give the integrated wave the full local CI and fresh-review gate. These commands are packet checks, not a claim that this docs-only seal ran them.

CB-N05a remains `UATPending` until this native first mission proof passes. A documented unavailable lab may leave exact rows `UATPending` under O14-09; an observed gameplay failure blocks merge. The later [CB-N05b evidence hand-in](../../four-zone-planning/cellblock-work-packets.md#cb-n05b--castle-evidence-hand-in-held) waits for HA-1360's atomic item receipt primitive and its own new seal.
