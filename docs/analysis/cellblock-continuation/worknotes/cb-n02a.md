# CB-N02a worknote: Keep Prisoner 329 dialog across relog

> **Type:** Worknote
> **Audience:** Cellblock coordinator and CB-U03 UAT
> **Last updated:** 2026-10-16

## Fault

Mission 638 binds Prisoner 329's topic (Human dialog set 2794, Jaffa 5866, slot 17) only on Region2 entry while 638 is `not_active` (chains 1011/1012). `available_interactions` is cell-local, so after relog with 638 active nothing rebinds the topic. The click on Prisoner 329 then produces no reply, and chains 1015/1014 (step 2114) and 1021/1020 (step 2116) both need that topic.

Sealed by CB-N02s (`## Seal check for CB-N02a: sealed`). The door-button mask (`329_CellDoorButton`, step 2115) is a separate gap (CB-N02b) and is not fixed here.

## Chain ids

- Chain 1026: `player_loaded` on `Castle_CellBlock`, `once = true`. Conditions: `mission_status 638 eq active`, `archetype neq 8`. Action: `add_dialog_set 2794`, slot 17. Human branch.
- Chain 1027: same trigger and mission gate, `archetype eq 8`. Action: `add_dialog_set 5866`, slot 17. Jaffa branch.
- Both are in `db/resources/Content/Seed/castle_cellblock_chains.sql` directly after chain 1019. Chain ids were allocated by the coordinator.

## Test

`live_db_mission_638_login_restores_only_active_archetype_dialog` in `crates/cell-content/src/cell/content/chain_replay_tests/mission_638.rs`. It is a resolve-level test (D-CC4): this packet changes no executor code. It fails with `login restore chain 1026 must exist` when the two chains are removed.

## Telemetry

No new events. After relog with 638 active, SigNoz shows `content.resolve` with `chain_id` 1026 or 1027, then a `dialog.add_set` span with dialog set 2794 or 5866.

## CB-U03 steps

On a fresh Human character and a fresh Jaffa character, relog at 2114, 2115 and 2116, then click Prisoner 329 and expect the archetype's topic.

## Untested

No live client run. The resolve-level test and the live-DB seed load were run in the lane; the in-game relog and click were not.
