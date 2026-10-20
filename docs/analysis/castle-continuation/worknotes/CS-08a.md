# CS-08a — Frost's Letter bridge seal

> **Type**: Reference. **Audience**: Castle/Harset coordinators and implementers.
> **Date**: 2026-10-14. **Status**: CS-08a1 integrated at `03100628b`; playable handoff BlockedDependency (HA-1360a/b). **Client**: UATPending.
> **Companions**: [Castle ledger](../README.md), [packet gates](../work-packets.md), [planning packet](../../four-zone-planning/castle-work-packets.md#cs-08--castle-carry-ins-for-harset-missions-1360-and-567), [shared IDs](../../four-zone-planning/id-allocation.md).

## Approval and evidence boundary

Owner decision, 2026-10-14: approve the Cimmeria reconstruction that advances mission 1360 from 4037 to 4038 on Castle-origin `stargate_crossed` key `Harset`, while 1360 and 4037 are active; retain Frost's Letter (3730) until Marsh receives it. The owner also requires Marsh hand-in to require the letter. This seals G-1360's Castle design as **CIMMERIA_RECONSTRUCTION**, never RESTORED_SOURCE. It does not approve an entire chain-ID block or authorize lab use.

This dated contract supersedes only the unresolved mission-1360 bridge fields in CS-00's [contract snapshot](../evidence/castle-contract.csv) and [reconciliation snapshot](../evidence/reconciliation.md#1376-1664-and-carry-ins). Those evidence snapshots remain historical; their bounded negative source finding remains valid. The earlier ignored options/item reports' proposal to tolerate missing possession at receipt is superseded by the owner's requirement.

| Source | Supported fact / limit |
|---|---|
| `deprecated/python/cell/missions/Castle_CellBlock/ArmYourself.py`, letter grant; `deprecated/python/cell/spaces/Castle.py` | Legacy reference grants 3730; no recovered Castle 4037→4038 event. No original-server chronology claimed. |
| `db/resources/Content/Seed/castle_cellblock_chains.sql`, 1003/1121 | Existing seed requests one 3730 with container 0; the grant resolver places it in mission bag 2. Acceptance at 4037 is reconstruction. Execute these seeded chains in the regression; do not replace their grant/accept. |
| `db/resources/Missions/Seed/mission_steps.sql`, 4037/4038; `mission_objectives.sql`, 4650/4651; `mission_tasks.sql`, 5693/5694 | Existing two-step journal data. Step/objective flags are false; tasks are already enabled. Text proves the requested verbs, not an event binding. |
| `crates/cell-content/src/cell/content/event_dispatch/stargate.rs`; `crates/cell-interactions/src/cell/gate_travel/mod.rs` | Crossing carries destination name and origin world/mission context before destination hydration. Normal crossing and existing no-region fallback emit this event. |
| `crates/content-engine/src/loader/condition.rs`; `crates/cell-content/src/cell/content/executor/mission.rs` | Existing world/mission/step guards and advance action suffice for Castle. Advance sends MissionUpdate from resulting state. |
| `db/resources/Content/Seed/harset_opcore_chains.sql`, 6501/6502/6505 | World 68 Marsh consumes one letter and completes 1360 at 4038; bindings survive entry/relog. Current receipt has no supported possession guard and can complete after removal fails. This is the separate recipient defect. |

## Frozen Castle event and chain shape

Exactly one new mission-scoped chain, `scope_type='mission'`, `scope_id=1360`, enabled, priority 0. Trigger: `stargate_crossed`, key `Harset`, scope `player`, `once=false`, sort order 0. Conditions, in order: `world target_id=8 eq` (NULL target_key/value); `mission_status target_id=1360 eq 'active'`; `step_status target_id=1360 target_key='4037' eq 'active'`. Action: exactly `advance_step target_id=1360 target_key='4038' params='{}' delay_ms=0 sort_order=0`.

All archetypes that already accepted 1360 qualify. No 708 prerequisite or Castle item predicate is added. Existing content-chain 1360 completes **mission 708** on the same event; it stays separate. Both chains resolve from pre-action context and must succeed in either action order, including 708 already completed. The Castle bridge emits zero item grant/remove/use messages and no accept, completion, reward, dialog, teleport or binding actions. Cellblock alone grants the letter; Harset alone owns receipt.

The advance means the delivery route was found at the validated crossing notification, before successful destination hydration. If travel later fails, 4038 may remain active in Castle. GM teleport and a player already in Harset at 4037 do not produce this Castle-origin event. No catch-up repair is implied. Persisted mission/step guards prevent repeat advancement after relog/world change; `once` is not the persistence guard. Journal feedback is existing step-4038 text after world load; no extra dialog is displayed during the crossing.

Owner explicitly approved **chain ID 1366**, 2026-10-14, reserved exclusively for CS-08a after the collision recheck recorded below. G-ID is closed for this ID only; the rest of 1366–1400 remains unsigned. This is a tracked content contract, not a seed edit.

Frozen INSERT shape for the implementation packet:

```sql
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1366, '1360 - Castle exit route: advance Frost''s Letter to 4038 (CIMMERIA_RECONSTRUCTION)', 'mission', 1360, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1366, 'stargate_crossed', 'Harset', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1366, 'world', 8, NULL, 'eq', NULL, 0),
  (1366, 'mission_status', 1360, NULL, 'eq', 'active', 1),
  (1366, 'step_status', 1360, '4037', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1366, 'advance_step', 1360, '4038', '{}', 0, 0);
```

## Exact implementation allowlist

| File | Permitted change |
|---|---|
| `db/resources/Content/Seed/castle_carry_in_chains.sql` | One bridge, chain 1366, with the frozen shape above. Provenance in description/comment. |
| `db/database.sql` | One `\ir resources/Content/Seed/castle_carry_in_chains.sql` include beside existing Castle includes. |
| `db/resources/Missions/Seed/mission_steps.sql` | Only step_enabled=true for 4037/4038 of mission 1360; preserve text, indexes, award flags and all other rows. |
| `db/resources/Missions/Seed/mission_objectives.sql` | Only is_enabled=true for 4650/4651 on 4037/4038; preserve all other fields/rows. |
| `crates/cell-content/src/cell/content/chain_replay_tests/mission_1360.rs` | Existing module: seeded bridge resolver/executor/real-crossing replay and `live_db_castle_carry_in` metadata guard. No new module registration needed. |
| `crates/services/src/mission_round_trip_tests.rs` | Extend the existing cross-service live-DB seam with real seeded grant/accept, bridge, persistence and fresh hydration; recipient phase follows its integrated contract. |
| `crates/cell/Cargo.toml` | Coordinator amendment, 2026-10-14: empty `test-support` feature for the production hydration seam. |
| `crates/cell/src/cell/service/mod.rs` | Feature-gated, doc-hidden public wrapper delegating directly to existing `build_restored_missions`; no copied hydration logic. |
| `crates/cell/src/cell/mod.rs` | Feature-gated, doc-hidden re-export of that wrapper through the private service module. |
| `crates/services/Cargo.toml` | Dev-dependencies enable `cimmeria-cell` and `cimmeria-cell-content` test support. |
| `docs/content/mission-chains.md` | Per-mission bridge, provenance, possession dependency and validation limits. No new Bible engine primitive. |
| `docs/analysis/castle-continuation/README.md`, `RESUME.md`, `worknotes/CS-08a.md` | Packet results, dependency/review/lane/revert records and honest UAT state. |

The Castle bridge may be implemented and reviewed as an independent first phase before HA-1360a/b. That phase proves the real Cellblock grant and acceptance, the Castle crossing, persisted step 4038 and letter retention through fresh Harset hydration. The recipient completion, missing-letter refusal and full playable handoff remain blocked until HA-1360a/b is integrated; the Castle phase alone does not complete CS-08a.

No Harset seed/executor change belongs to this allowlist. Mission 567 remains the separate CS-08b evidence gate. Ring chain 1109, callbacks, coordinates, arrival regions and travel timing retain their separate owner. Mission 1360 already exists in the client catalog; catalog insertion is not a dependency. The cell mission loader currently reads step/objective rows without filtering enabled flags; flag edits make metadata consistent and do not prove rendering. Journal delivery remains UATPending; if a resource override is required by evidence, stop for an exact amendment.

## Regression and persistence contract

TESTING.md types: live-DB chain replay with executor traffic and a services live-DB persisted round trip. Use `require_db_or_skip!`, production resource loaders and exact fixture cleanup; reuse the services sentinel allocation without crossing its owned range. Do not inject 4038, manually grant 3730, or hand-build the expected MissionUpdate in the reachability path.

1. Load real seeded 1003/1121, drive Frost's dialog with 622/2113 active, execute their resolved actions and persist actual grant/mission messages through the production CellToBase/base handlers. Assert one owned 3730 and 1360 active at 4037.
2. Hydrate a fresh Castle entity from `query_saved_missions` and production inventory hydration. Drive `fire_stargate_crossed` with origin world 8 and destination `Harset` using the seeded engine. Assert exact one-action bridge, active 4038, completed 4037/objective 4650, active objective 4651, actual post-action MissionUpdate and zero inventory mutations. Persist actual messages; query saved rows; hydrate fresh Harset state. Letter count remains one.
3. After the separate Harset/engine contract is integrated, execute receiving 6502/6501 as applicable through the production interaction/consume path. Assert 1360 completed, letter count zero, exactly one successful recipient consumption, duplicate crossing/receipt/relog has no further mutation, and another player's state/inventory stays unchanged.
4. Missing-letter receipt must refuse: active 4038 and recipient binding remain, no completion/reward/removal success. A possession condition alone is insufficient if removal can fail after resolution; the recipient packet must prove consumption succeeds before mission completion. Inaccessible letter inventory follows that packet's accessible-inventory contract. These assertions are blocked until its exact interface integrates, not waived or changed to current warn-and-complete behavior.
5. Castle negatives: wrong origin (57/68), wrong destination (including `Harset_CmdCenter`), mission not_active/completed/failed, non-active 4037, already active 4038, repeated crossing and fresh-entity relog. Include concurrent active 708/4469 and already-completed 708, with either action order. Metadata guard proves exact 1360 step/objective/task joins and enabled flags.

Revert proof: commit the fix/tests first, remove only the new bridge rows and reload the isolated test DB; unchanged crossing and persisted-reachability guards must fail at 4037 with no reachable receipt. Restore only that file, reload and rerun. Separately revert each new Castle guard to show its adjacent negative fails. Harset owns possession/removal-failure mutations and recipient regression proof. Record commands, exit codes, failed assertion and restored pass; no proof was run in this documentation seal.

## Checks, telemetry and dependencies

Implementation branch/worktree: `castle/cs-08a1-letter-bridge`, `.claude/worktrees/castle-cs-08a1`, from Castle feature SHA `096cc3c3e`. Commit: `feat(castle): bridge Frost's Letter to Harset delivery`. About 40k working tokens; if recipient integration changes the seam enough to exceed the budget, split/reseal before dispatch.

Focused check order after skill reads and authorized seed reload:

```powershell
cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-services --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo nextest run --profile=ci -p cimmeria-cell-content mission_1360
pwsh tools/build-lane/lane.ps1 cargo nextest run --profile=ci -p cimmeria-services frosts_letter
pwsh tools/build-lane/lane.ps1 cargo nextest run --profile=ci -p cimmeria-content-engine --test it
pwsh tools/build-lane/live-db-test.ps1 mission_1360
pwsh tools/build-lane/live-db-test.ps1 castle_carry_in
pwsh tools/build-lane/live-db-test.ps1 frosts_letter
```

Capture real non-skipped DB results; the cumulative feature wave still owes the full execution-contract gate. The content-engine integration suite guards the new seed include. These commands are planned, not executed here.

Correlate existing `gate_travel.cross`, crossing match/destination, `content.resolve`/`condition_failed`, chain matched and `Content: advancing step` with chain ID/name, mission 1360/name, step 4038/name, origin/destination and player identity. Advance logging precedes mutation; actual MissionUpdate plus saved row proves credit. Base inventory evidence must show one Cellblock grant, zero Castle mutation and one committed recipient consume. Missing-letter/refusal and consume-failure telemetry belongs to the Harset/engine packet, with correlated mission/item/player names and persisted unchanged state. No broad new telemetry allowance is implied.

G-REVIEW for the Castle bridge phase is signed 2026-10-14: independent mission, items, and persistence/Rust reviewers found the frozen event/guard/action shape sound. The items review corrected the grant-location wording above; the persistence review requires real seeded 1003/1121 actions and fresh hydration in the packet test. Coordinator accepts those findings and authorizes the independent Castle seed/test phase. The existing mission UPSERT has no version check, so sequential repeat/relog guards do not establish concurrent stale-writer safety; HA-1360a owns that separate receipt requirement. Playable handoff is **BlockedDependency** on Harset/engine **HA-1360a/b**, whose detailed contracts belong in `docs/analysis/harset-continuation/work-packets.md`. Their letter-possession and successful-consume-before-complete interfaces must be committed and recorded by the coordinator before dispatch; this seal does not guess their API. Current 6501 behavior cannot satisfy the owner requirement. G-ID is closed only for allocated chain 1366. Other Castle gates and CS-08b remain open.

Client UAT is **UATPending**: ordinary Castle→Harset crossing, journal 4038 after load, retained letter, Marsh success/refusal, redial/re-cross/relog, simultaneous 708 and a second observer. Explicit owner lab permission is required before any lease/client/server action. Offline tests do not prove journal, cinematic, NPC dialog or visual behavior.

## Seal validation record

Baseline `3222e6b50d5a22602fef2a277e0b0b06014f6a92`, inspected 2026-10-14. First-tuple resource-seed checks for candidate 1366 found zero hits at `origin/main` `19eab88c9`, Castle `3222e6b50`, Cellblock `457b4f75a`, Harset `72ad79648`, Tollana `93eae8cfd`. Content-chain INSERT scans of all four integration worktrees also found zero; live continuation ledgers show no individual chain claim (mission ID 1366 elsewhere is a separate namespace). This is a point-in-time check, not an atomic reservation; recheck immediately before dispatch.

The initial seal changed documentation only; the implementation results below supersede its earlier untested runtime/seed status. Shared status documentation remains FZ-99-owned.

## CS-08a1 implementation record

The coordinator amended the allowlist on 2026-10-14 after independent persistence and testing review: four files expose a feature-gated wrapper around the existing production mission restorer and enable it only for services tests. Gameplay hydration logic is unchanged. The public production base player-load query separately hydrates inventory.

Implemented frozen chain 1366 and its database include, enabled only steps 4037/4038 and objectives 4650/4651, and replaced the former hand-built acceptance-message test with the actual seeded 1003/1121 → base dispatch → saved query → fresh Castle restore → real crossing → actual update persistence → fresh Harset restore path. One 3730 remains in mission bag 2 with the same instance/slot. Repeated crossing and fresh relog emit no mutation; another player's inventory and mission rows remain empty. Castle replay guards cover both wrong origin worlds (57/68), wrong destinations, mission status/step negatives, all eight archetypes, and both 708 action orders with active/already-completed 708.

| Check | Result |
|---|---|
| `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-services --all-targets -- -D warnings` | Exit 0 after fixing the test collector's attempted clone of non-Clone `CellToBaseMsg`; no production change. |
| `pwsh tools/build-lane/lane.ps1 cargo nextest run --profile=ci -p cimmeria-content-engine --test it` | Exit 0; 39 passed. |
| `pwsh tools/build-lane/live-db-test.ps1 -E 'test(mission_1360) \| test(castle_carry_in) \| test(frosts_letter)'` | Exit 0; 19 real DB tests passed after final code edits against the isolated reloaded worktree database (`20261014-141524-63504.log`). |
| `cargo fmt --all -- --check` | Exit 0. |
| Focused `ci` nextest: services `frosts_letter`, cell-content `mission_1360` | Exit 0; 1 and 18 passed respectively. These runs have no DB; the actual DB results are above. |

Implementation snapshot: `65d3e88be`, `feat(castle): bridge Frosts Letter to Harset delivery`. Fresh independent read-only review found no verified issues in that commit. The coordinator owns integration and the cumulative execution-contract gate.

Revert proof started from that committed snapshot, changing only `castle_carry_in_chains.sql`. Removing all bridge INSERTs and running `pwsh tools/build-lane/live-db-test.ps1 -E 'test(seeded_frost_accept_then_real_castle_crossing) | test(frosts_letter_seeded_bridge_round_trip)'` failed with lane exit **100**: both unchanged tests observed **Some(4037)** instead of **Some(4038)** (`20261014-141902-60340.log`). The replay failure is in `assert_delivery`; the services failure is the actual crossing reachability assertion, before any expected update is constructed.

Each individual condition mutation restores the committed file first, removes only the named condition and reloads through `pwsh tools/build-lane/live-db-test.ps1 crossing_guards_and_all_archetypes`:

| Removed condition | Result / failed adjacent negative | Lane log |
|---|---|---|
| `world eq 8` | Exit 100; `must refuse Harset/Harset/1/Some(4037)` failed because a Harset-origin crossing emitted credit. | `20261014-142249-13056.log` |
| `mission_status 1360 eq active` | Exit 100; `must refuse Castle/Harset/0/Some(4037)` failed with a non-active mission at 4037. | `20261014-142635-77552.log` |
| `step_status 1360/4037 eq active` | Exit 100; `must refuse Castle/Harset/1/None` failed with an active mission lacking a current step. | `20261014-143004-9384.log` |

All seed mutations were restored from `65d3e88be`; `git diff --exit-code 65d3e88be -- crates db` returns 0. The restored run of `pwsh tools/build-lane/live-db-test.ps1 -E 'test(mission_1360) | test(castle_carry_in) | test(frosts_letter)'` returns **exit 0, 19 passed** (`20261014-143348-64468.log`). This restores the crossing, persisted reachability, all adjacent negatives and the exact metadata guard together.

CRLF-aware whitespace checks (`git -c core.whitespace=cr-at-eol diff --check`) pass. Scoped Markdown lint has no findings in the updated packet documents or new memory note; `docs/content/mission-chains.md` retains 37 pre-existing warnings outside the edited section. The wrapper unexpectedly linted the whole repository, so the scoped check used the existing `markdownlint-cli2` command directly with `--no-globs`. The implementation code and seed still match the independently reviewed commit; the follow-up commit changes only the packet results documents.

Full cumulative execution-contract checks, concurrent stale-writer safety, Harset receipt/consumption/refusal, live telemetry and client UAT remain outstanding. No lab, push or PR action was taken.

## Castle integration database rerun, 2026-10-14

After the shared PostgreSQL cluster completed crash recovery, the Castle
integration worktree reloaded `sgw_zone_castle` from its own seed: 350 content
chains. `pwsh tools/build-lane/live-db-test.ps1
chain_replay_tests::mission_1360` then reloaded that database in the build
lane and ran **18 real DB tests, 18 passed** (exit 0; lane log
`20261014-170402-93776.log`). The existing Castle release server binary's
`--check-db` also returned `database check ok` against that database. This
rerun verifies the Castle side of the bridge after database recovery. It
does not test the pending Harset recipient, a client crossing, or an aligned
Castle branch lab server.
