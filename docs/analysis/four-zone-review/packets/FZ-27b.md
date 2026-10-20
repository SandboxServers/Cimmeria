# FZ-27b: the Praxis offer stands down during the 1664 debrief and comes back after it

- **Status:** BlockedDependency on FZ-27a (committed, packet review `ship`).
- **Wave:** 12, second of three sequential packets. Same worktree and branch as FZ-27a; one commit on top of it, with its own packet review loop. The branch then merges into the campaign branch once, `--no-ff`.
- **Depends on:** FZ-27a; FZ-21 and FZ-22 merged (this packet edits their files).
- **Fixes:** at step 5226 a Human with 1361 not active who clicks Col. Marsh would get both chain 6512 (the Praxis offer, dialog 4457) and chain 6018 (the 1664 report), so the offer dialog would bury the report. 6512 must stand down at 5226; its twin 6511 must carry the same condition (lockstep invariant), and then needs two more trigger rows so the "?" comes back the moment 1664 ends in the Command Center (playtest finding H9). Architect call D-FZ14 under the owner's delegation.
- **Agent:** packet-coder. About 35k working tokens.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27`, test DB `sgw_fz_27`.
- **Branch:** `four-zone-review/FZ-27-1664-debrief`.
- **Report:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27\.claude\reports\four-zone-review\FZ-27b-coder.md`
- **Return (one line):** `FZ-27b <done|blocked|failed> <sha|-> <report path>`
- **Host:** windows

## Rules (inlined; you do not read CLAUDE.md)

1. Use PowerShell only, with absolute paths. Start every call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27;`.
2. Every compiling `cargo` call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never run `cargo` directly. Live-DB tests: `pwsh tools/build-lane/live-db-test.ps1 <filter>` against `sgw_fz_27`. Never touch `sgw`.
3. Seeds only in `db/resources/`; no `db/scripts/*.sql` migration. Seed the rows exactly as written.
4. **6511 and 6512 must carry identical condition lists** (`live_db_offer_bind_and_offer_dialog_carry_identical_conditions` pins it). Every condition row added to one is added to the other with the same values and sort order.
5. Live-DB test names start with `live_db_` and open with `let pool = require_db_or_skip!();`. Type 2 dispatcher tests (TESTING.md); each must fail when its row is reverted.
6. New `.rs` files are converted to CRLF after Write: `$p='<absolute path>'; [IO.File]::WriteAllText($p, ([IO.File]::ReadAllText($p) -replace "(?<!\r)\n", "`r`n"))`. Existing files: Edit tool only.
7. File caps: soft 500 lines, hard 700. `mission_1664_flow.rs` is already near 450, so the whole-engine debrief tests go in a new file.
8. Edit only the listed files. Forbidden: `git stash`, `git worktree prune`, fetch, push, rebase, `git add -A`.
9. Read with the Read tool, search with Grep. At most 5 fix rounds per check.

## Edits

**A. `db/resources/Content/Seed/harset_opcore_chains.sql`, chains 6511 and 6512.**

1. In 6511's comment block, after the paragraph that ends `` Pinned by `chain_6511_offers_the_praxis_the_moment_the_letter_is_handed_over`. `` and before `-- 6512 needs no second trigger`, insert:

   ```sql
   --
   -- THIRD AND FOURTH ROWS (FZ-27): `mission_completed '1664'` and `mission_abandoned '1664'`.
   -- 6511 and 6512 stand down while mission 1664 is on step 5226, so a Marsh click there is
   -- the 1664 report (chain 6018, harset_1664_chains.sql), not the Praxis offer. That gate
   -- reopens in world 68, inside the click that completes 1664 (or the abandon), with no
   -- world entry to come: H9 again. These two rows hand the "?" back in the same event.
   -- Both events populate world, archetype and mission context after the mutation.
   ```

2. 6511's trigger `VALUES` become:

   ```sql
   VALUES
     (6511, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0),
     (6511, 'mission_completed', '1360', 'player', false, 1),
     (6511, 'mission_completed', '1664', 'player', false, 2),
     (6511, 'mission_abandoned', '1664', 'player', false, 3);
   ```

3. 6511's last condition row `(6511, 'step_status', 1360, '4038', 'neq', 'active', 4);` loses its `;` and is followed by:

   ```sql
     -- Stand down during the mission 1664 debrief (FZ-27): a Marsh click on 5226 is 6018's report.
     (6511, 'step_status', 1664, '5226', 'neq', 'active', 5);
   ```

4. 6512's last condition row `(6512, 'step_status', 1360, '4038', 'neq', 'active', 4);` loses its `;` and is followed by `  (6512, 'step_status', 1664, '5226', 'neq', 'active', 5);`.

**B. `crates/cell-content/src/cell/content/chain_replay_tests/mission_1361/handoff.rs`.**

1. `resolve_expansion_as_if_enabled`: replace `assert_eq!(registered, 1, "chain {chain_id} must have exactly one {tt:?} trigger row; found {registered}");` with `assert!(registered >= 1, "chain {chain_id} must have at least one {tt:?} trigger row; found {registered}");`, and add one doc line: "6511 has two `mission_completed` rows (1360 and 1664); the event's `mission_id` picks between them."
2. `live_db_chain_6511_offers_the_praxis_the_moment_the_letter_is_handed_over`: `expansions.len()` expected `2` becomes `4`; the message becomes "chain 6511 must carry four trigger rows: `player_loaded` for the world entry, `mission_completed 1360` for the letter hand-over, and `mission_completed 1664` / `mission_abandoned 1664` for the end of the 1664 debrief. Got: {:?}". Update the doc comment's "two trigger rows" to "four trigger rows" and add one sentence on the 1664 pair.
3. New test `live_db_chain_6511_hands_the_offer_back_when_1664_ends`. `expected` is the same `AddDialogSet { dialog_set_id: 5254, slot: 10, mission_id: Some(1361) }`. Base context: `world_id = Some(CMD_CENTER)`, `archetype = ARCHETYPE_SOLDIER`, `mission_1360_step_4038_status = "completed"`.
   - `MissionCompleted` with `mission_id = 1664`, `mission_1664_status = "completed"`, `mission_1664_step_5226_status = "completed"`: `actions_of(..) == expected`.
   - `MissionAbandoned` with `mission_id = 1664`, `mission_1664_status = "not_active"`: `== expected`.
   - Negatives, each `resolved.actions.is_empty()`: the `PlayerLoaded` expansion with `world_name = "Harset_CmdCenter"` and `mission_1664_step_5226_status = "active"` (the new guard); the `MissionCompleted` 1664 context with `archetype = ARCHETYPE_JAFFA`; the same with `world_id = Some(HARSET)`.
   Doc comment: H9 for the debrief; revert proof: delete 6511's `mission_completed '1664'` row and the first assertion fails.

**C. `crates/cell-content/src/cell/content/chain_replay_tests/mission_1361/disjointness.rs`.**

1. `live_db_marsh_interact_chains_are_pairwise_disjoint`: wrap the `for (m1360, s4038, s4694) in states` loop in `for at_5226 in [false, true]`. When `at_5226`, also set `mission_1664_status`, `mission_1664_step_5226_status`, `mission_1664_obj_6061_status` and `mission_1664_obj_6062_status`, each `"active"`. Add `at_5226` to the failure message. After the `firing.len() <= 1` assert, add an anti-vacuity check: when `at_5226` and the state is `("not_active", "not_active", "not_active")` or `("completed", "completed", "not_active")`, assert `firing == vec![6018]` ("the 1664 report must claim the click; if not, the 1664 params are wrong and the 5226 half proves nothing"). Add to the doc comment: chain 6018 (mission 1664's Marsh report) is the fourth Marsh chain; revert proof: drop the 5226 row from 6511 and 6512 and the `("completed", "completed", "not_active")` state at 5226 resolves 6512 and 6018 together.
2. `live_db_mohkatan_interact_chains_are_pairwise_disjoint`: same `for at_5226 in [false, true]` wrap around `for step in STEPS`, same four params when `at_5226`. Anti-vacuity: when `at_5226` and `step` is `"4041"`, assert `ids == vec![6019]`. Doc comment: 6019 is the third Moh'katan chain; revert proof: drop 6019's `1361/4040` row and step 4040 at 5226 resolves 6515 and 6019.
3. Do **not** touch `live_db_no_template_slot_ever_holds_two_binds_at_once`. It counts every `AddDialogSet`, and NULL-dialog binds (120101, 120302) legally share slot 10 with a dialog bind, because `handle_interact` only takes the first bind that carries a dialog.

**D. New `crates/cell-content/src/cell/content/chain_replay_tests/mission_1664_debrief_flow.rs`** (about 230 lines). Register `mod mission_1664_debrief_flow;` in `chain_replay_tests/mod.rs` right after `mod mission_1664_flow;`. Module doc: mission 1664's debrief on the whole seeded engine (FZ-27): every other seeded chain on `player_loaded 'Harset_CmdCenter'`, `interact_tag 'CmdCenter_Marsh'`/`'CmdCenter_Mohkatan'`, `mission_completed` and `mission_abandoned` is loaded, so a co-firing chain shows up as an extra trace label or bind. Harness copied from `mission_1664_flow.rs`, not shared.

Harness: copy from `mission_1664_flow.rs` its `use` lines (drop `fire_mission_completed` if unused), constants (drop `CASTLE`, `HARSET`, `KAERLIA`, `DOOR`, `KAERLIA_SLOT`, `DOOR_SLOT`), `staged`, `completed`, `mission_of`, `all_binds`, `slot_ids`, `read_wstring`, `decode_bark_text`, `screen`, `traffic`, `log_in`, `click`, `abandon`. Changes:

- Constants: `MARSH: &str = "CmdCenter_Marsh"`, `MOHKATAN: &str = "CmdCenter_Mohkatan"`, `MARSH_SLOT: i32 = 10`, `MOHKATAN_SLOT: i32 = 54`.
- `traffic` drops the `MissionUpdate` arm and the step-dedup lines (labels: `dialog N`, `bark TEXT`, `travel W`).
- `fn debrief() -> MissionInstance`: 1664 at 5226 (`current_step_id = Some(5226)`) with objectives 6061 and 6062 both `STATUS_ACTIVE`, `hidden: false`, `optional: false`.
- The engine is `build_engine(Some(&pool)).await` in every test.

Tests:

1. `live_db_mission_1664_debrief_hands_the_praxis_offer_back`. `marsh = screen(&pool, 200180)`, `mohkatan = screen(&pool, 200181)`. Stage `CMD_CENTER` with `vec![completed(708), completed(1360), debrief()]`, `log_in(.., CMD_CENTER)`:
   - `slot_ids(MARSH_SLOT) == vec![120302]` (no 5254: 6511 stands down) and `slot_ids(MOHKATAN_SLOT) == vec![120303]`;
   - `click(.., MARSH)`: `handled`, trace exactly `[format!("bark {marsh}")]` (no `dialog 4457`); `slot_ids(MARSH_SLOT)` empty;
   - `click(.., MOHKATAN)`: `handled`, trace exactly `[format!("bark {mohkatan}")]`; `mission_of(..).unwrap().status == MISSION_COMPLETED`; `slot_ids(MARSH_SLOT) == vec![5254]` (the offer is back in the same click); `slot_ids(MOHKATAN_SLOT)` empty;
   - `click(.., MARSH)` once more: `handled`, trace `["dialog 4457"]` (the Praxis offer, now reachable).
   Revert proof: drop the 5226 row from 6511 and 6512 and the first Marsh trace gains `dialog 4457`; drop 6511's `mission_completed '1664'` row and slot 10 is empty after the Moh'katan click.
2. `live_db_mission_1664_abandon_at_5226_hands_the_praxis_offer_back`. Same stage and login, then `abandon(..)`: `slot_ids(MARSH_SLOT) == vec![5254]` and `slot_ids(MOHKATAN_SLOT)` empty. Revert proof: drop 6511's `mission_abandoned '1664'` row and slot 10 is empty.

**E. `crates/cell-content/src/cell/content/chain_replay_tests/harset_1664_lifecycle.rs`** (FZ-21), doc text only, in `live_db_mission_1664_relog_after_the_door_binds_nothing`:

- Replace the doc sentences from `In Harset_CmdCenter at 5226 the login binds` through `not), not a revert proof of a current row.` with: "In Harset_CmdCenter at 5226 none of chains 6008-6016 binds anything; the debrief cues there come from chains 6033/6034, which this file does not load, and are pinned in `harset_1664_debrief`."
- Change the message `"a Command Center login at 5226 must bind nothing; got {:?}"` to `"chains 6008-6016 must bind nothing on a Command Center login at 5226; got {:?}"`.

## Checks, in order

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; if it fails, `pwsh tools/build-lane/lane.ps1 cargo fmt --all`, then check again.
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/live-db-test.ps1 mission_1361`. Expected to pass after the edits above. If `live_db_the_praxis_offer_is_human_only`, `live_db_the_praxis_offer_waits_for_the_letter_to_be_delivered` or `live_db_chain_6511_second_trigger_is_gated_as_tightly_as_the_first` fail, they need at most an expected-list or context update (an unset 5226 reads `not_active`, so they should pass unchanged): fix only that, name it in the report.
4. `pwsh tools/build-lane/live-db-test.ps1 mission_1664` (runs `mission_1664_flow` and `mission_1664_debrief_flow`).
5. `pwsh tools/build-lane/live-db-test.ps1 harset_1664`
6. `pwsh tools/build-lane/live-db-test.ps1 mission_letter_receipt` (the services tests fire 6511 through the letter receipt).
7. `pwsh tools/build-lane/live-db-test.ps1 chain_replay`

## Revert proof (record it in the report)

1. Delete the 5226 row from both 6511 and 6512, rerun checks 3 and 4: the Marsh disjointness test and debrief-flow test 1 fail. Restore.
2. Delete 6511's `mission_completed '1664'` row, rerun checks 3 and 4: the new handoff test and debrief-flow test 1 fail. Restore.
3. `git diff --stat` shows only the intended paths (six).

## Commit

```text
feat(content): Praxis offer stands down during the 1664 debrief and returns after it (FZ-27b)

Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012n1ut2hRgmMP5C6ZXG9HEV
```

Stage the paths explicitly.
