# Cellblock continuation: wave 1b coder packets

> **Type:** Work packets (canonical, on `content/cellblock-campaign`)
> **Audience:** `packet-coder` workers for CB-N02b, CB-N02c and CB-N02d, and their reviewers
> **Last updated:** 2026-10-16
> **Companions:** [work-packets.md](work-packets.md) (dispatch rules, waves, the advisor packets and the W1b integration packet), [cb-n02s.md](worknotes/cb-n02s.md) (the audit that sealed these gaps)

Split from work-packets.md to keep both files under the 500-line cap. The contract and repo rules below are copied from work-packets.md so that each packet here is self-contained. CB-N02b, CB-N02c and CB-N02d run in parallel, one worktree each.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/cellblock-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock`) |
| Recorded feature SHA | **`e10fd1f0a86383129d3bcb7b44a0b6cebabecc72`** (W1a integrated, 2026-10-16). History: `14658b3b1` (D-CC1 = B checkpoint), then CB-N02s/CB-N02a cherry-picks. |
| Packet branch / worktree | `cellblock/<id>-<slug>` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-<id>` |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 cellblock/<id>-<slug> cellblock-<id>` (branches from `main`). 2. Ops, or a `packet-coder` if ops refuses ([work-packets.md § W1b-R0](work-packets.md)): `git -C <wt> rebase --onto <feature-sha> HEAD cellblock/<id>-<slug>` (the new branch has no commits, so this moves it onto the feature SHA). 3. Verify `git -C <wt> rev-parse HEAD` = `<feature-sha>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Status vocabulary | Ready, BlockedDependency, BlockedDecision, BlockedEvidence, Writing, Review, Integrated, UATPending, Done |
| Report path | `<wt>\.claude\reports\cellblock-continuation\<packet>-<role>.md` (inside the worker's own worktree, never the root checkout). Before retiring a worktree, the orchestrator copies anything worth keeping to `worknotes/` on the branch. |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (the number if the harness shows it, else `not exposed`), wall-clock start and end (UTC, from `Get-Date -AsUTC`), lane runs, review rounds (filled in by the reviewer). |
| Attribution | Commit trailer lines are given in the dispatch brief; copy them exactly. |
| Relog restore shape | Trigger `player_loaded`, event key `Castle_CellBlock` (exact spelling, capital B), scope `player`, `once` false. Conditions gate on the exact step that needs the state. Context keys the engine fills on login: `mission_<m>_status`, `mission_<m>_step_<s>_status`, `mission_<m>_obj_<o>_status` (values `active`, `completed`; a missing key reads as `not_active`). Precedents: 1045 (`set_interaction_type` restore), 1009 (`set_aggression` re-arm). |
| Test shape (D-CC4) | Seed-only restores are tested at resolve level: load the chain with `load_single_chain_for_test`, register it in a `ChainEngine`, fire a `TriggerType::PlayerLoaded` event with `world_name = "Castle_CellBlock"` and the step params, and assert on `engine.resolve_event(..).actions` filtered by chain id. Test fn names must contain `live_db` (the live-DB group guard fails otherwise). |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...`. Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground and read their logs under `logs/<worktree-name>/`.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps:** soft 500 lines, hard 700. Check `(Get-Content <file>).Count` before and after. If a file you must extend would end above 500, stop and report `blocked`.
6. **Line endings.** Docs under `docs/` are CRLF: after writing one, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`, then after `git add` check `git -C <wt> ls-files --eol <path>` shows `w/crlf`. `castle_cellblock_chains.sql` is stored CRLF (`i/crlf`): keep it that way, and check it the same way. Rust files are `i/lf`; the Edit tool keeps them as they are.
7. **Edit only the files your packet lists.** Any other file needed means `blocked`. Never edit the seed's top-of-file chain-range header or `id-allocation.md` (D-CC8: the integration packet owns them).
8. **Chain IDs** come only from the packet (claimed in [id-allocation.md](../four-zone-planning/id-allocation.md)). Never pick a free ID yourself.
9. Run `git -C <wt> diff --check` before committing.
10. If a lane check fails twice for the same reason, stop and report `failed` with the log path. Do not widen the change to make it pass.

---

## CB-N02b — Keep the 329 cell door button live across relog

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens: one seed chain, one test, one worknote |
| Branch / worktree | `cellblock/cb-n02b-638-door-button-relog` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02b` |
| Chain ID | **1028** (claimed 2026-10-16 in id-allocation.md) |
| Commit | `fix(cellblock): keep the 329 cell door button live across relog` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02b\.claude\reports\cellblock-continuation\CB-N02b-coder.md` |
| UAT row | CB-U03 (relog at 638 step 2115, then click the cell door button): UATPending |

**Fault (gap A of [cb-n02s.md](worknotes/cb-n02s.md)).** Chains 1014/1015 (the Prisoner 329 dialog choice at step 2114) advance mission 638 to step 2115 and set `329_CellDoorButton` (spawn 22, template 22, base flags 0) to `{"op": "|", "mask": 256}`. Chain 1016 starts the Livewire game when the player clicks that button at step 2115; chain 1017 clears the bit at victory and advances to 2116. Interaction flags live in cell memory, and a login builds a fresh instance with flags 0. A relog at step 2115 therefore leaves the button inert and the player stuck in the cell. 1014/1015 split by dialog trigger, not by archetype, so one restore chain covers both archetypes.

**Files (only these):**

- `db/resources/Content/Seed/castle_cellblock_chains.sql`
- `crates/cell-content/src/cell/content/chain_replay_tests/mission_638.rs` (407 lines; must stay under 500)
- `docs/analysis/cellblock-continuation/worknotes/cb-n02b.md` (new)

**Step 0.** Grep the seed for `VALUES (1028,`. If found, stop and report `blocked` (someone else used the ID).

**Seed change.** Insert this block directly after chain 1027's `content_actions` statement (the line `VALUES (1027, 'add_dialog_set', 5866, NULL, '{"slot": 17}', 0, 0);`) and before the `-- ====` banner that opens `-- MISSION 639`. Keep one blank line before and after it.

```sql
-- Chain 1028 (CB-N02b): chains 1014/1015 set the 329_CellDoorButton
-- Livewire bit (mask 256) only on the step 2114 -> 2115 dialog edge, and
-- interaction flags live in cell memory. A relog at step 2115 loads the
-- button with template flags 0, so chain 1016 can never start the Livewire
-- game and the player is stuck in the cell. Re-paint the bit on login while
-- 2115 is active; chain 1017 clears it at 2116. Same shape as chain 1045.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1028, '638 - Zone load: step 2115 active -> restore 329_CellDoorButton Livewire bit (relog)', 'mission', 638, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1028, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1028, 'step_status', 638, '2115', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1028, 'set_interaction_type', NULL, '329_CellDoorButton', '{"op": "|", "mask": 256}', 0, 0);
```

Do not edit the file header (rule 7).

**Test.** Append to the end of `mission_638.rs`. Test type: live-DB chain replay at resolve level (D-CC4; this packet changes no executor code, say so in one sentence in the report). The file already imports `ChainEngine`, `ExecutionContext`, `TriggerEvent`, `TriggerType`, `load_single_chain_for_test` and `require_db_or_skip` at the top; import `Action` inside the fn as the other tests do.

```rust
/// Chain 1028: the cell door button's Livewire bit (mask 256) is set only on
/// the 1014/1015 dialog edge and lives in cell memory. A relog at step 2115
/// must re-paint it; step 2114 and steps after 2115 must not.
#[tokio::test]
async fn live_db_mission_638_login_restores_door_button_bit_only_at_step_2115() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    engine.register_chain(
        load_single_chain_for_test(&pool, 1028)
            .await
            .expect("DB query for chain 1028 must succeed")
            .expect("door button restore chain 1028 must exist"),
    );

    for (params, expect_restore) in [
        (vec![("mission_638_step_2115_status", "active")], true),
        (vec![("mission_638_step_2114_status", "active")], false),
        (
            vec![
                ("mission_638_step_2115_status", "completed"),
                ("mission_638_step_2116_status", "active"),
            ],
            false,
        ),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        for (key, value) in &params {
            ctx.set_param((*key).to_string(), serde_json::json!(*value));
        }
        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);
        let actions: Vec<&Action> = resolved
            .actions
            .iter()
            .filter(|(id, _)| *id == 1028)
            .map(|(_, action)| action)
            .collect();
        if expect_restore {
            assert!(
                matches!(
                    actions.as_slice(),
                    [Action::SetInteractionType { entity_tag, operation, mask: 256 }]
                    if entity_tag == "329_CellDoorButton" && operation == "|"
                ),
                "{params:?}: expected exactly the door button bit; got {actions:?}"
            );
        } else {
            assert!(
                actions.is_empty(),
                "{params:?}: chain 1028 must not fire; got {actions:?}"
            );
        }
    }
}
```

If `fmt --check` fails, run `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content` once and re-check. Do not hand-format.

**Revert proof.** Delete the four 1028 statements from the seed, rerun lane check 4. The test must fail with `door button restore chain 1028 must exist`. Restore the block, rerun, and write the observed failure line in the report.

**Telemetry acceptance.** No new events. After a relog at step 2115, SigNoz shows the DEBUG event `Content: set interaction type` (target `cimmeria_cell_content`) with `chain_id` 1028, `entity_tag` `329_CellDoorButton`, `mask` 256. At any other step, `content.resolve` logs `reason="condition_failed"` for chain 1028 and no set-interaction event follows. Put this in the worknote.

**Lane checks, in order** (each prefixed with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02b;`):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content --filter-expr 'test(mission_638)'`
4. `pwsh tools/build-lane/live-db-test.ps1 mission_638`. The new test must report **passed**, not skipped. Check the log under `logs/cellblock-cb-n02b/`.

**Worknote** `worknotes/cb-n02b.md` (CRLF, under 40 lines): the fault, chain 1028, the telemetry sentence, the CB-U03 steps (Human and Jaffa: reach step 2115, relog, click the cell door button, expect the Livewire game to open), and what is untested (no live client run).

**Docs rows owed:** none beyond the worknote. `docs/content/mission-chains.md` describes the legacy scripts, not seed chain IDs.

**Repo rules:** all ten above.

---

## CB-N02c — Keep the Ambernol vial usable across relog

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens: one seed chain, one test, one worknote |
| Branch / worktree | `cellblock/cb-n02c-639-vial-relog` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02c` |
| Chain ID | **1036** |
| Commit | `fix(cellblock): keep the Ambernol vial usable across relog` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02c\.claude\reports\cellblock-continuation\CB-N02c-coder.md` |
| UAT row | CB-U03 (relog at 639 step 2145, then use the vial): UATPending |

**Fault (gap B).** Chain 1031 (enter `Castle_Cellblock.Region11` while 639 step 2117 is active) advances to step 2145 and sets `ArmYourself_AmbernolVial` (spawn 12, template 18, base flags 0) to `{"op": "|", "mask": 1073741824}`. Chain 1032 (interact with the vial at step 2145) picks it up, destroys the vial entity and advances to 2144. A relog at step 2145 loads the vial with flags 0, and 1031 cannot fire again because 2117 is done. The vial cannot be used and mission 639 stalls. The bit is needed only at 2145: 1032 destroys the vial when it leaves 2145. On a later relog the vial respawns with flags 0, which is harmless.

**Files (only these):**

- `db/resources/Content/Seed/castle_cellblock_chains.sql`
- `crates/cell-content/src/cell/content/chain_replay_tests/mission_639.rs` (313 lines; must stay under 500)
- `docs/analysis/cellblock-continuation/worknotes/cb-n02c.md` (new)

**Step 0.** Grep the seed for `VALUES (1036,`. If found, stop and report `blocked`.

**Seed change.** Insert directly after chain 1031's `content_actions` statement (ends with the line `(1031, 'set_interaction_type', NULL, 'ArmYourself_AmbernolVial', '{"op": "|", "mask": 1073741824}', 0, 1);`) and before the `-- ====` banner that opens `TAKE-COVER OBJECTIVE (C05 ...)`. One blank line before and after.

```sql
-- Chain 1036 (CB-N02c): chain 1031 sets the vial's interact bit
-- (mask 1073741824) only on the Region11 edge while step 2117 is active.
-- A relog at step 2145 loads the vial (spawnlist row 12) with template
-- flags 0, and 1031 cannot fire again because 2117 is done, so the vial
-- cannot be used and 639 stalls. Re-paint the bit on login while 2145 is
-- active; chain 1032 destroys the vial at 2145 -> 2144.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1036, '639 - Zone load: step 2145 active -> restore ArmYourself_AmbernolVial interact bit (relog)', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1036, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (1036, 'step_status', 639, '2145', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1036, 'set_interaction_type', NULL, 'ArmYourself_AmbernolVial', '{"op": "|", "mask": 1073741824}', 0, 0);
```

**Test.** Append to the end of `mission_639.rs`. Same test type and D-CC4 sentence as CB-N02b. This file imports only `load_single_chain_for_test` and `require_db_or_skip` at the top, so put the other `use` lines inside the fn:

```rust
/// Chain 1036: chain 1031 sets the vial's interact bit only on the Region11
/// edge at step 2117. A relog at step 2145 must re-paint it; chain 1032
/// destroys the vial at 2145 -> 2144, so no other step may.
#[tokio::test]
async fn live_db_chain_1036_restores_vial_bit_only_at_step_2145() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::ChainEngine;
    use cimmeria_content_engine::context::ExecutionContext;
    use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    engine.register_chain(
        load_single_chain_for_test(&pool, 1036)
            .await
            .expect("DB query for chain 1036 must succeed")
            .expect("vial restore chain 1036 must exist"),
    );

    for (params, expect_restore) in [
        (vec![("mission_639_step_2145_status", "active")], true),
        (vec![("mission_639_step_2117_status", "active")], false),
        (
            vec![
                ("mission_639_step_2145_status", "completed"),
                ("mission_639_step_2144_status", "active"),
            ],
            false,
        ),
    ] {
        // Same body as CB-N02b's test: build ctx with world_name
        // "Castle_CellBlock" plus params, fire TriggerType::PlayerLoaded,
        // keep the actions of chain 1036, then:
        //   expect_restore -> exactly
        //     [Action::SetInteractionType { entity_tag, operation, mask: 1_073_741_824 }]
        //     with entity_tag == "ArmYourself_AmbernolVial" && operation == "|"
        //   otherwise -> actions.is_empty()
        // Assertion messages: "{params:?}: expected exactly the vial bit; got {actions:?}"
        // and "{params:?}: chain 1036 must not fire; got {actions:?}".
    }
}
```

Copy the loop body from CB-N02b's code block above, changing only the chain id (1036), the mask pattern (`mask: 1_073_741_824`), the tag and the messages.

**Revert proof.** Delete the four 1036 statements, rerun lane check 4: the test must fail with `vial restore chain 1036 must exist`. Restore, rerun, record the failure line.

**Telemetry acceptance.** After a relog at step 2145: DEBUG `Content: set interaction type` with `chain_id` 1036, `entity_tag` `ArmYourself_AmbernolVial`, `mask` 1073741824. Otherwise `content.resolve` `reason="condition_failed"` for 1036. Put this in the worknote.

**Lane checks, in order** (prefix `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02c;`):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content --filter-expr 'test(mission_639)'`
4. `pwsh tools/build-lane/live-db-test.ps1 mission_639`. The new test must report **passed**, not skipped. Log under `logs/cellblock-cb-n02c/`.

**Worknote** `worknotes/cb-n02c.md` (CRLF, under 40 lines): the fault, chain 1036, why the restore cannot outlive 2145 (1032 destroys the vial), the telemetry sentence, the CB-U03 steps (reach 2145, relog, use the vial, expect pickup and the drone fight), and what is untested.

**Docs rows owed:** none beyond the worknote. **Repo rules:** all ten.

---

## CB-N02d — Keep the retrieval drone hostile across relog

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 40k working tokens: one seed chain, one new test file plus one `mod` line, one worknote |
| Branch / worktree | `cellblock/cb-n02d-639-drone-relog` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02d` |
| Chain ID | **1037** |
| Commit | `fix(cellblock): keep the retrieval drone hostile across relog` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02d\.claude\reports\cellblock-continuation\CB-N02d-coder.md` |
| UAT row | CB-U03 (relog at 639 step 2144 before the kill; the drone must engage): UATPending |

**Fault (gap C; degraded play, not a strand).** Chain 1032 arms `ArmYourself_PrisonerRetrievalUnit` (spawnlist row 10, template 4, seeded NEUTRAL) with `set_aggression` level 1 when the player takes the vial, then advances to step 2144. Step 2144 needs objectives 2482 (kill the drone) and 2484 (take cover). A relog at 2144 builds a fresh instance with the drone NEUTRAL again, so it never attacks. Re-arm it on login while 2144 is active and 2482 is still open. Follow chain 1009 (the 622 guard re-arm): aggression level only, no `generate_threat`, so the drone engages through its normal hostile scan. Do not replay sequence 10001; the C05 note in the seed says why.

**Files (only these):**

- `db/resources/Content/Seed/castle_cellblock_chains.sql`
- `crates/cell-content/src/cell/content/chain_replay_tests/mission_639_drone_relog.rs` (new, about 70 lines)
- `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` (one line)
- `docs/analysis/cellblock-continuation/worknotes/cb-n02d.md` (new)

**Step 0.** Grep the seed for `VALUES (1037,`. If found, stop and report `blocked`.

**Seed change.** Insert directly after chain 1032's `content_actions` statement (its last line is `(1032, 'advance_step',    639,  '2144', ...);`) and before the comment `-- Chain 1033 is now defined above ...`. One blank line before and after.

```sql
-- Chain 1037 (CB-N02d): chain 1032 arms the drone (set_aggression 1;
-- spawnlist row 10 seeds it NEUTRAL) when the vial is taken. A relog at
-- step 2144 builds a fresh instance with the drone NEUTRAL again, so it
-- never engages. Re-arm it on login while 2144 is active and the kill
-- objective 2482 is still open. Like chain 1009: aggression level only,
-- no generate_threat, no replay of sequence 10001.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (1037, '639 - Zone load: step 2144 active, kill open -> re-arm ArmYourself_PrisonerRetrievalUnit (relog)', 'mission', 639, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (1037, 'player_loaded', 'Castle_CellBlock', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (1037, 'step_status', 639, '2144', 'eq', 'active', 0),
  (1037, 'objective_status', 639, '2482', 'neq', 'completed', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1037, 'set_aggression', NULL, 'ArmYourself_PrisonerRetrievalUnit', '{"level": 1}', 0, 0);
```

**mod.rs.** Add `mod mission_639_drone_relog;` on its own line directly after `mod mission_639_cover;`.

**Test.** New file `mission_639_drone_relog.rs`. Same test type and D-CC4 sentence as CB-N02b.

```rust
//! Mission 639 relog re-arm of the prisoner-retrieval drone (chain 1037).
//!
//! Chain 1032 arms `ArmYourself_PrisonerRetrievalUnit` (seeded NEUTRAL)
//! when the player takes the Ambernol vial. A relog builds a fresh
//! Castle_CellBlock instance with the drone NEUTRAL again, so chain 1037
//! re-arms it on `player_loaded` while step 2144 is active and kill
//! objective 2482 is open. Like chain 1009 it adds no threat.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn live_db_chain_1037_rearms_drone_only_while_kill_open_at_step_2144() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    engine.register_chain(
        load_single_chain_for_test(&pool, 1037)
            .await
            .expect("DB query for chain 1037 must succeed")
            .expect("drone re-arm chain 1037 must exist"),
    );

    for (params, expect_rearm) in [
        (
            vec![
                ("mission_639_step_2144_status", "active"),
                ("mission_639_obj_2482_status", "active"),
            ],
            true,
        ),
        (
            vec![
                ("mission_639_step_2144_status", "active"),
                ("mission_639_obj_2482_status", "completed"),
            ],
            false,
        ),
        (vec![("mission_639_step_2145_status", "active")], false),
        (
            vec![
                ("mission_639_step_2144_status", "completed"),
                ("mission_639_step_2343_status", "active"),
            ],
            false,
        ),
    ] {
        // Same ctx / event / filter body as CB-N02b's test, with chain id 1037.
        // expect_rearm -> exactly
        //   [Action::SetAggression { entity_tag, level: 1 }]
        //   with entity_tag == "ArmYourself_PrisonerRetrievalUnit"
        //   (a GenerateThreat in the list fails the test: 1009 shape)
        // otherwise -> actions.is_empty()
        // Messages: "{params:?}: expected exactly SetAggression(drone, 1); got {actions:?}"
        // and "{params:?}: chain 1037 must not fire; got {actions:?}".
    }
}
```

Copy the loop body from CB-N02b's code block, changing the chain id, the `matches!` pattern and the messages.

**Revert proof.** Delete the four 1037 statements, rerun lane check 4: the test must fail with `drone re-arm chain 1037 must exist`. Restore, rerun, record the failure line.

**Telemetry acceptance.** After a relog at 2144 with the drone alive: INFO, target `content`, `event="set_aggression"` with `chain_id` 1037 and `tag` `ArmYourself_PrisonerRetrievalUnit`, `to` HOSTILE. If the drone is not spawned yet at login, the WARN `event="set_aggression_tag_miss"` appears instead; that is a finding for the UAT, not for this packet. Put this in the worknote.

**Lane checks, in order** (prefix `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02d;`):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content --filter-expr 'test(mission_639_drone_relog)'`
4. `pwsh tools/build-lane/live-db-test.ps1 mission_639_drone_relog`. The new test must report **passed**, not skipped. Log under `logs/cellblock-cb-n02d/`.

**Worknote** `worknotes/cb-n02d.md` (CRLF, under 40 lines): the fault (degraded, not stranding), chain 1037, why no threat (1009 precedent), the telemetry sentence, the CB-U03 steps (take the vial, relog before killing the drone, expect it to engage when in range and sight), and what is untested.

**Docs rows owed:** none beyond the worknote. **Repo rules:** all ten.

---
