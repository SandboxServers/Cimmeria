lab-fixture: server_stage_fixture cannot stage mission 1360 at step 4037 (starter UAT praxis SP-DL1)

## Problem

`server_stage_fixture` with `missions: [{id: 1360, step: 4037}]` always fails on a character that loads in Castle_CellBlock: the server logs `lab fixture not applied`, reason `write_failed`, error `mission 1360 was not written: stored None, wanted Some((1, Some(4037), [], [], 0))`. The whole fixture then leaves out mission 1360 (and in the first two attempts the other missions and item did not apply either until the 1360 entry was removed). Any UAT row that needs mission 1360 at step 4037 (Frost's Letter, SP-DL1 clause 3 "advance 4037") is BLOCKED. @Cadacious

Cause from the code: the fixture goes through the generic mission update. For mission 1360 that path calls `persist_active_letter_mission` (`crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in.rs`), which only accepts the active snapshot when `active_objective_ids == [4650]`, no completed steps, no completed objectives, repeats 0. The fixture writes the step with empty objective lists, so the guard returns false and the caller logs `Mission 1360 generic update refused ... invalid_or_stale_snapshot` (`missions/mod.rs` line ~181). The guard is intentional for real play; the defect is that the fixture/stage path does not supply the objective 4650 the letter mission needs. Not a game-path bug.

## Evidence

- Server log, player 108 (Fxdlone): WARN `Mission 1360 generic update refused ... invalid_or_stale_snapshot` and ERROR `lab fixture not applied` at 14:16:55Z and 14:18:05Z on 2026-11-03. Same pair for player 109 at 14:22:57Z.
- `missions_restored` at 14:18:48Z after the 688-only fixture: active `622:2113,689:68900h,688:80688`, no 1360.
- Fixture call args: `missions: [{id:688, step:80688}, {id:1360, step:4037}], items: [{container:mission, id:3730, count:1}], position [-54.88,26.08,-159.84], world Castle_CellBlock`.
- Driver reports: `.claude/reports/starter-uat-praxis/drv-SP-DL1-a1.md`, `-a2.md`, `-a3.md`. No screenshots relevant.

## Acceptance criteria

- `server_stage_fixture` with `{id:1360, step:4037}` ends in `lab fixture applied`, and `SELECT * FROM public.sgw_mission WHERE player_id=<p>` shows mission 1360 status 1, current_step_id 4037, active_objective_ids [4650].
- After a relog the character loads with 1360 active at 4037 (`missions_restored` lists `1360:4037`).
- A fixture that cannot write one mission reports which mission failed in the tool result and does not silently drop it from the staged set.

## Test type

Unit/live-DB test in the lab fixture crate: stage `{1360, 4037}` and assert the row. Revert proof: removing the objective seeding makes it fail with `stored None`.

## Docs to update

`docs/guides/` lab fixture / `spec_fixture` docs: note that 1360 at 4037 carries objective 4650. SP-DL1 verify brief.

## Client impact

Free.

## Domain advisor

mission-systems-advisor.

## Implementation plan

1. In the lab fixture writer, when the mission is 1360 at step 4037, fill `active_objective_ids = [4650]` (or call the letter-mission persist with its expected snapshot); similarly for the step-B state (4038 per `ACTIVE_STEP`) if the fixture is asked for it.
2. Surface the failing mission id in the `server_stage_fixture` result instead of only the server log.
3. Add the live-DB test above and re-run SP-DL1 LEG-1 with 1360 staged.