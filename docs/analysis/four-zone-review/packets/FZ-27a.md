# FZ-27a: mission 1664 step 5226, the debrief seeds and their per-chain replay

- **Status:** BlockedDependency. Start once FZ-21 and FZ-22 are merged into the campaign branch, and FZ-25 is merged too (or D-PU4 is ruled No, so FZ-25 never ships). FZ-25 edits the same two files and goes first.
- **Wave:** 12, first of three sequential packets (27a, then 27b, then 27c). All three share one worktree and one branch; each packet is one commit with its own packet review loop. The branch merges into the campaign branch once, `--no-ff`, after 27b's review returns `ship`.
- **Depends on:** FZ-20 (merged), FZ-21, FZ-22, FZ-25 (or D-PU4 = No).
- **Fixes:** D-PU3 = B (owner, 2026-10-19). Step 5226 "Report for debriefing" has no credit, so 1664 can never complete. Design: [FZ-26 worknote](../worknotes/FZ-26-step-5226.md). Architect calls: D-FZ14, D-FZ15 in [decisions.md](../decisions.md).
- **Agent:** packet-coder. About 40k working tokens: the SQL is given in full and the harness is copied.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27`, test DB `sgw_fz_27`.
- **Branch:** `four-zone-review/FZ-27-1664-debrief`.
- **Report:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27\.claude\reports\four-zone-review\FZ-27a-coder.md`
- **Return (one line):** `FZ-27a <done|blocked|failed> <sha|-> <report path>`
- **Host:** windows

## Rules (inlined; you do not read CLAUDE.md)

1. Use PowerShell only, with absolute paths. Start every call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-27;`.
2. Every compiling `cargo` call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never run `cargo` directly. Live-DB tests: `pwsh tools/build-lane/live-db-test.ps1 <filter>` against `sgw_fz_27`. Never touch `sgw`.
3. Seeds only in `db/resources/`; no `db/scripts/*.sql` migration.
4. **Seed the rows exactly as written.** Step 5226 credits with `complete_objective` 6061 and 6062: completing the second objective auto-completes 1664, which is the point. Steps 5224 and 5225 keep `advance_step`; do not touch chains 6008-6015 or 6017. If a test shows the rows misbehave, return `blocked` with the trace; do not redesign them.
5. Live-DB test names start with `live_db_` and open with `let pool = require_db_or_skip!();`. Type 2 dispatcher tests and a type 6 seed pin (TESTING.md); each must fail when its row is reverted.
6. The interact-tag linter reads one row per line: keep each `'interact_tag'` trigger tuple on one line, as written.
7. New `.rs` files are converted to CRLF after Write: `$p='<absolute path>'; [IO.File]::WriteAllText($p, ([IO.File]::ReadAllText($p) -replace "(?<!\r)\n", "`r`n"))`. Existing files: Edit tool only.
8. File caps: soft 500 lines, hard 700. `harset_1664.rs` is already over 500, so the new tests go in the new file; only the seed pin changes there.
9. Edit only the listed files. Forbidden: `git stash`, `git worktree prune`, fetch, push, rebase, `git add -A`.
10. Read with the Read tool, search with Grep. At most 5 fix rounds per check.

## Ids (collision-checked 2026-10-19 across 892 refs; do not change)

| Kind | Id | Use |
|---|---|---|
| chain | 6018 | Col. Marsh report: bark, drop cue, credit objective 6061 |
| chain | 6019 | Moh'katan report: bark, drop cue, credit objective 6062 |
| chain | 6033 | `player_loaded 'Harset_CmdCenter'` at 5226: Marsh cue (dsm 120302, slot 10) |
| chain | 6034 | same: Moh'katan cue (dsm 120303, slot 54) |
| dialog_set_map | 120302, 120303 | NULL-dialog binds, flag 536870912 (INT_NonAStoryMissionTurnIn) |
| dialog | 60170 | bark holder, never displayed |
| dialog_screen | 200180, 200181 | the two bark lines (PROVISIONAL, D-FZ15) |

## Edits

**A. `db/resources/Content/Seed/harset_1664_chains.sql`.**

1. Header, replace these lines exactly:
   - `-- Command Center", 5226 "Report for debriefing" (no completion chain yet: D-PU3).` becomes `-- Command Center", 5226 "Report for debriefing" (FZ-27: D-PU3 = B).`
   - `-- Credit only with advance_step. complete_objective 6060/6063 would auto-complete 1664.` becomes two lines: `-- Steps 5224/5225 credit with advance_step (complete_objective 6060/6063 would auto-complete 1664).` and `-- Step 5226 credits with complete_objective 6061 (Marsh, 6018) and 6062 (Moh'katan, 6019), either order; the second auto-completes 1664.`
   - `--   6018-6019 reserved: step 5226 debrief (D-PU3)` becomes `--   6018-6019 step 5226 debrief: Marsh, Moh'katan (D-PU3, FZ-27)`.
   - After the line `--   6020      reserved: re-offer after abandon, only if the owner reverses O18-02`, add `-- Outside that block: 6033-6034 restore the 5226 debrief cues on Command Center login (6021-6032 are HA-01a).`
   - `-- Every click and login chain carries `world 57 eq`.` becomes ``-- Every click and login chain carries `world 57 eq`, except the debrief chains 6018/6019/6033/6034 (`world 68 eq`).``
2. DSM block. Replace the three comment lines starting `-- DSM 120301 (this file owns 120301-120400)` with:

   ```sql
   -- DSM 120301-120303 (this file owns 120301-120400). NULL dialog_id on all three: each bind
   -- only raises one interaction bit for one player. 120301 is the door cue (INT_MissionWorldObject
   -- 1073741824, template 261). 120302/120303 are the 5226 debrief cues on Col. Marsh (template 10)
   -- and Moh'katan (template 54), INT_NonAStoryMissionTurnIn (536870912).
   -- GREATEST keeps the shared 120001+ block from ever lowering.
   ```

   Replace the single-row `VALUES (120301, ...)` line with:

   ```sql
   VALUES
     (120301, 1992, NULL, 'Command Center door', 1073741824, 1, '{}', '{}', '{}', '{}'),
     (120302, 1992, NULL, 'Report to Col. Marsh', 536870912, 1, '{}', '{}', '{}', '{}'),
     (120303, 1992, NULL, 'Report to Moh''katan', 536870912, 1, '{}', '{}', '{}', '{}');
   ```

   and in the `setval` below it change `GREATEST(120301,` to `GREATEST(120303,`.
3. Chain 6016 (abandon): change its description to `'1664 - abandoned: drop the Kaerlia, door and debrief cues (PROVISIONAL)'`, change the comment above it to `-- 6016: abandon, any world: drop every cue. Nothing else (P0; no re-grant).`, and extend its action `VALUES` (keep the two rows; the second loses its `;`):

   ```sql
     (6016, 'remove_dialog_set', 120301, NULL, '{"slot": 261}', 0, 1),
     (6016, 'remove_dialog_set', 120302, NULL, '{"slot": 10}', 0, 2),
     (6016, 'remove_dialog_set', 120303, NULL, '{"slot": 54}', 0, 3);
   ```

4. Append at the end of the file (after 6016, or after FZ-25's 6017 if it is there):

```sql

-- 6018: report to Col. Marsh on 5226 (Command Center, world 68). Bark first, drop the cue, then
-- credit 6061 last. Either order with 6019; the second credit auto-completes 1664.
-- Guards: 1360/4038 (6501, the letter) and 1361/4694 (6527, the Praxis turn-in) also key on
-- Marsh; 6512 (the Praxis offer) is gated off 5226 in harset_opcore_chains.sql (FZ-27b).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6018, '1664 - Col. Marsh (step 5226): bark, drop the cue, credit objective 6061 (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6018, 'interact_tag', 'CmdCenter_Marsh', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6018, 'world', 68, NULL, 'eq', NULL, 0),
  (6018, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6018, 'objective_status', 1664, '6061', 'neq', 'completed', 2),
  (6018, 'step_status', 1360, '4038', 'neq', 'active', 3),
  (6018, 'step_status', 1361, '4694', 'neq', 'active', 4);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6018, 'npc_bark', NULL, NULL, '{"screen_id": 200180, "speaker": "Col. Marsh", "channel": "say"}', 0, 0),
  (6018, 'remove_dialog_set', 120302, NULL, '{"slot": 10}', 0, 1),
  (6018, 'complete_objective', 1664, '6061', '{}', 0, 2);

-- 6019: report to Moh'katan on 5226. Same shape as 6018, objective 6062. Guards: the Jaffa
-- chains 6305 (1324/3954) and 6340 (1326/4603) and the Praxis chains 6515 (1361/4040) and
-- 6520 (1361/4042) also key on Moh'katan.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6019, '1664 - Moh''katan (step 5226): bark, drop the cue, credit objective 6062 (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6019, 'interact_tag', 'CmdCenter_Mohkatan', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6019, 'world', 68, NULL, 'eq', NULL, 0),
  (6019, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6019, 'objective_status', 1664, '6062', 'neq', 'completed', 2),
  (6019, 'step_status', 1324, '3954', 'neq', 'active', 3),
  (6019, 'step_status', 1326, '4603', 'neq', 'active', 4),
  (6019, 'step_status', 1361, '4040', 'neq', 'active', 5),
  (6019, 'step_status', 1361, '4042', 'neq', 'active', 6);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (6019, 'npc_bark', NULL, NULL, '{"screen_id": 200181, "speaker": "Moh''katan", "channel": "say"}', 0, 0),
  (6019, 'remove_dialog_set', 120303, NULL, '{"slot": 54}', 0, 1),
  (6019, 'complete_objective', 1664, '6062', '{}', 0, 2);

-- 6033: Command Center login on 5226 with 6061 not yet credited: put the cue on Marsh.
-- Templates 10 and 54 ship interaction_type 0, so without a bind the client never sends the
-- click. 5226 is only ever reached through a teleport (6012, 6006, 6017), so this login chain
-- also covers the arrival. No 1360/1361 guards here: the cue must show whatever else is live.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6033, '1664 - Command Center login (step 5226): restore the Marsh debrief cue (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6033, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6033, 'world', 68, NULL, 'eq', NULL, 0),
  (6033, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6033, 'objective_status', 1664, '6061', 'neq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6033, 'add_dialog_set', 120302, NULL, '{"slot": 10, "mission_id": 1664}', 0, 0);

-- 6034: the same for Moh'katan and 6062.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (6034, '1664 - Command Center login (step 5226): restore the Moh''katan debrief cue (PROVISIONAL)', 'mission', 1664, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (6034, 'player_loaded', 'Harset_CmdCenter', 'player', false, 0);
INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (6034, 'world', 68, NULL, 'eq', NULL, 0),
  (6034, 'step_status', 1664, '5226', 'eq', 'active', 1),
  (6034, 'objective_status', 1664, '6062', 'neq', 'completed', 2);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (6034, 'add_dialog_set', 120303, NULL, '{"slot": 54, "mission_id": 1664}', 0, 0);
```

**B. `db/resources/Dialogs/Seed/dialogs.sql`:** after the 60160 row (Grep `VALUES (60160, 0,`), add a blank line and:

```sql
-- Harset mission 1664 step 5226 (FZ-27): holder for the Marsh and Moh'katan debrief barks.
-- Never displayed; npc_bark reads its screens' text. Seed-only, no override.
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (60170, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Harset - mission 1664 debrief bark holder (never displayed)');
```

**C. `db/resources/Dialogs/Seed/dialog_screens.sql`:** after the 200170 row (Grep `VALUES (60160, 200170,`), add a blank line and:

```sql
-- Harset mission 1664 step 5226 (FZ-27): debrief bark lines (CIMMERIA_AUTHORING, PROVISIONAL).
-- Speaker 0: npc_bark supplies the chat prefix from its own params. Order-neutral text.
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60170, 200180, 'Report received. Good work out there.', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60170, 200181, 'Your report is filed.', 0, 1);
```

At the end of the same file change `SELECT pg_catalog.setval('dialog_screens_screen_id_seq', 200170, true);` to `200181`.

**D. `crates/content-engine/tests/it/interact_tag_linter/allowlist.rs`:** after the line `| ("harset_1664_chains.sql", 6012..=6013) // Harset_CmdDoor: ...`, insert (same indentation):

```rust
        | ("harset_1664_chains.sql", 6018..=6019) // CmdCenter_Marsh / CmdCenter_Mohkatan: per-player dsm 120302/120303 binds (INT_NonAStoryMissionTurnIn) by chains 6033/6034
```

**E. `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs`:** add `mod harset_1664_debrief;` right after `mod harset_1664;`.

**F. `crates/cell-content/src/cell/content/chain_replay_tests/harset_1664.rs`, the seed pin only** (`live_db_mission_1664_seed_never_completes_or_regrants`):

- The action query becomes `SELECT chain_id, action_type, target_key, delay_ms FROM resources.content_actions WHERE chain_id BETWEEN 6008 AND 6020 OR chain_id IN (6033, 6034)`, typed `Vec<(i32, String, Option<String>, i32)>`.
- `complete_mission` and `accept_mission` stay forbidden everywhere. `complete_objective` is allowed only as `(6018, Some("6061"))` and `(6019, Some("6062"))`; assert exactly those two `complete_objective` rows exist (collect them, sort, compare). The `delay_ms == 0` check stays for every row.
- The chain-id query adds `OR chain_id IN (6033, 6034)`. Expected ids: first run `git grep -n "VALUES (6017," db/resources/Content/Seed/harset_1664_chains.sql`. If it prints a line, expect `6008..=6019` then `6033, 6034`; if not, expect `6008..=6016`, `6018, 6019, 6033, 6034`. Say which in the report.
- After the 120301 check, pin 120302 and 120303 the same way: each `(1992, None, 536870912)`.
- Doc comment: "The seed never grants, never completes a mission directly and never delays; its only `complete_objective` rows are the two 5226 reports (6018/6061, 6019/6062). It holds chains 6008-6016 and 6018-6019 (plus 6017 once FZ-25 lands), and the debrief restores 6033-6034. Revert proof: add a `complete_objective` on 6008, or set a `delay_ms` on 6012, and the action scan fails; set 120302's `dialog_id` and the map check fails."

**G. New `crates/cell-content/src/cell/content/chain_replay_tests/harset_1664_debrief.rs`** (about 330 lines). Module doc: mission 1664 step 5226 (FZ-27): the Marsh and Moh'katan reports, the cue restore on Command Center login and the abandon tidy-up, through the real dispatchers; harness copied from `harset_1664` rather than shared.

Harness: copy from `harset_1664.rs` lines 11-238 and the `abandon` helper (Grep `async fn abandon(`): the `use` lines (add `STATUS_COMPLETED` to the `cimmeria_entity::missions` import), `ENTITY`, `PLAYER`, `TARGET`, `MISSION`, `HARSET`, `CMD_CENTER`, `ON_DIALOG_DISPLAY`, `ON_PLAYER_COMMUNICATION`, `engine_with`, `staged`, `completed`, `all_binds`, `read_wstring`, `decode_bark_text`, `screen`, `traffic`, `log_in`, `click`, `abandon`. Changes:

- Constants: `MARSH: &str = "CmdCenter_Marsh"`, `MOHKATAN: &str = "CmdCenter_Mohkatan"`, `CHAINS: [i32; 5] = [6016, 6018, 6019, 6033, 6034]`. Drop `KAERLIA` and `DOOR`.
- `traffic` drops the `MissionUpdate` arm and the step-dedup lines: labels are only `dialog N`, `bark TEXT` and `travel W`, so a report click's trace is exactly its bark.
- `fn debrief(done: &[i32]) -> MissionInstance`: 1664 at step 5226 (`current_step_id = Some(5226)`) with objectives 6061 and 6062, each `MissionObjective { objective_id, status: if done.contains(&objective_id) { STATUS_COMPLETED } else { STATUS_ACTIVE }, hidden: false, optional: false }`.
- `fn mission_1664(step, objective)` copied as is, for the other states.
- `fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance>`: `mgr.get_entity(ENTITY).unwrap().missions.get_mission(MISSION)` (as in `mission_1664_flow.rs`).
- `fn status_of(mgr) -> (i8, Option<i32>)`: `mission_of(mgr).map(|m| (m.status, m.current_step_id))`, panicking with a message if 1664 is gone.
- `fn objective_done(mgr, id) -> bool`: true when 1664 holds objective `id` with `STATUS_COMPLETED` in `active_objectives`, or `id` is in `completed_objectives`.

Tests (each doc comment names its revert proof):

1. `live_db_mission_1664_debrief_cues_restore_on_command_center_login`. Engine `CHAINS`. Stage `CMD_CENTER`, `log_in(.., CMD_CENTER)`, assert `all_binds`:
   - `debrief(&[])`: `[(10, vec![120302]), (54, vec![120303])]`;
   - `debrief(&[6061])`: `[(54, vec![120303])]`; `debrief(&[6062])`: `[(10, vec![120302])]`;
   - no 1664, `mission_1664(5224, 6063)`, `mission_1664(5225, 6060)`, `completed(MISSION)`: `[]`;
   - stage `HARSET` with `debrief(&[])`, `log_in(.., HARSET)`: `[]`.
   Revert proof: delete chain 6033 and the first case loses slot 10.
2. `live_db_mission_1664_marsh_then_mohkatan_completes_the_mission`. `marsh = screen(&pool, 200180)`, `mohkatan = screen(&pool, 200181)`. Import `MISSION_COMPLETED` (already in the copied `use`). Stage `CMD_CENTER` with `debrief(&[])`, log in, `click(.., MARSH)`: `handled`, trace exactly `[format!("bark {marsh}")]`, `objective_done(6061)`, 1664 still active at 5226, `all_binds == [(54, vec![120303])]`. Then `click(.., MOHKATAN)`: `handled`, trace `[format!("bark {mohkatan}")]`, `status_of == (MISSION_COMPLETED, None)`, `all_binds` empty.
3. `live_db_mission_1664_mohkatan_then_marsh_completes_the_mission`: the reverse order, same assertions mirrored (after the first click 6062 done and only slot 10 left).
4. `live_db_mission_1664_a_reported_npc_answers_nothing_new`. After the Marsh click of test 2's setup, click `MARSH` again: `handled == false`, empty trace, 1664 still active. Revert proof: drop 6018's `objective_status` row and the re-click barks again.
5. `live_db_mission_1664_debrief_clicks_are_gated`. Not handled, empty trace, for: Marsh and Moh'katan clicks in each of no 1664, 5224, 5225 and completed (stage `CMD_CENTER`); `debrief(&[])` staged in `HARSET` (world gate); `debrief(&[])` plus a 1361 instance at step 4694 (`MissionInstance::new(1361, 4694, vec![])` with `current_step_id = Some(4694)`) blocks the Marsh click; `debrief(&[])` plus 1324 at 3954 blocks the Moh'katan click. Revert proof: drop 6018's `world` row and the Harset click credits.
6. `live_db_mission_1664_abandon_at_5226_drops_both_debrief_cues`. Stage `CMD_CENTER` with `debrief(&[])`, log in (both cues bound), `abandon_mission(MISSION)` on the entity's missions as `harset_1664.rs` does, then `abandon(..)`: `all_binds` empty. Revert proof: drop 6016's 120302 row and slot 10 survives.

## Checks, in order

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; if it fails, `pwsh tools/build-lane/lane.ps1 cargo fmt --all`, then check again.
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/live-db-test.ps1 harset_1664` (runs `harset_1664`, `harset_1664_debrief` and `harset_1664_lifecycle`). All pass, none skipped.
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine --test it interact_tag_linter`
5. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources seed_id_uniqueness`
6. `pwsh tools/build-lane/live-db-test.ps1 live_db_namebook_every_seed_row_resolves_or_is_a_pinned_gap`. If it fails only on ids from this packet (dsm 120302/120303, dialog 60170, screens 200180/200181), bless as FZ-20 did (`$env:NAMEBOOK_GAPS_BLESS='1'` in the same call), check the gaps diff adds only those ids, and rerun. Anything else: `blocked`.
7. `pwsh tools/build-lane/live-db-test.ps1 mission_1361` and `pwsh tools/build-lane/live-db-test.ps1 mission_1664_flow`. Both pass unchanged (their contexts carry no 1664 at 5226, or check only slots 260/261). A failure here is `blocked`: 27b owns those files.
8. `pwsh tools/build-lane/live-db-test.ps1 chain_replay`

## Revert proof (record it in the report)

1. Delete 6018's `complete_objective` row, rerun check 3: tests 2 and 3 fail. Restore. Then the same for 6019's.
2. Delete chain 6033's rows, rerun check 3: test 1 fails. Restore.
3. `git diff --stat` shows only the intended paths (seven, or eight with the namebook gaps file).

## Commit

```text
feat(content): mission 1664 step 5226, Marsh and Moh'katan debrief credit (FZ-27a)

Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012n1ut2hRgmMP5C6ZXG9HEV
```

Stage the paths explicitly.
