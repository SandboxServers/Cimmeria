# Tollana continuation: wave 1 packets

> **Title**: Tollana continuation: wave 1 packets
> **Type**: Work packets (canonical, on `content/tollana-campaign`)
> **Audience**: The Tollana orchestrator, the agents it dispatches for T-02a, T-03a1 to T-03a4, PH-01a, PH-01b and the INT steps, and their reviewers
> **Last updated**: 2026-10-17
> **Companions**: [w1-frozen.md](w1-frozen.md) (every literal these packets copy), [568 chain spec](evidence/T-03a-568-chain-spec.md), [offline placement](evidence/T-01-offline-placement.md), [phasing design](evidence/phasing.md), [README.md](README.md) (findings, decisions, status), [ID register](tollana-id-register.csv)

Wave 1 builds the first playable Tollana mission, **568 "Finding Tollan Technology"**, as an item-less pilot, plus the core of the AoI phasing model that later Tollana content (621's vault safe) needs. Every row is CIMMERIA_RECONSTRUCTION.

- **No lab in this wave (user directive, 2026-10-17).** No agent takes a lab lease, starts a client or drives x64dbg. Every in-client check is UATPending for the user ([§ UATPending](#uatpending-the-users-manual-uat)).
- **Eight build packets in four waves**, each about 40k working tokens or less. The 568 spec's single packet is far bigger, so it splits along file seams: cast seeds (T-02a), Copplemann chains (T-03a1), prop chains and the executor rig (T-03a2), executor walks (T-03a3, T-03a4). Phasing splits into the entity model (PH-01a) and the AoI filter (PH-01b).
- **What is not here** is in [§ Blocked](#blocked-no-packet-this-wave).

## Dispatch plan

| Step | Packet | Agent | Worktree / branch | Base | Runs after |
|---|---|---|---|---|---|
| W1a | [T-02a](#t-02a--copplemann-and-food-unit-cast-pilot) (measurement pilot) | `packet-coder` | `tollana-t02a` / `tollana/t02a-cast` | ARCH-W1 commit | dispatch |
| W1a-R | review T-02a; fixes; re-review | `packet-reviewer`; `packet-coder` | `tollana-t02a` | - | T-02a `done` |
| W1a-INT | [integrate](#int-steps--w1a-int-w1b-int-w1c-int-w1d-int) | `packet-coder` | `zone-tollana` / `content/tollana-campaign` | branch tip | W1a-R `ship` |
| W1b | [T-03a1](#t-03a1--chains-9001-9010-copplemann-accept-turn-in-restore-abandon) | `packet-coder` | `tollana-t03a1` / `tollana/t03a1-copplemann` | W1a-INT commit | W1a-INT `done` |
| W1b | [PH-01a](#ph-01a--phase-model-in-the-entity-crate) | `packet-coder` | `tollana-ph01a` / `tollana/ph01a-phase-model` | W1a-INT commit | W1a-INT `done` |
| W1b-R, W1b-INT | review both; fixes; integrate | as W1a | - | - | both `done` |
| W1c | [T-03a2](#t-03a2--chains-9011-9018-and-the-executor-rig) | `packet-coder` | `tollana-t03a2` / `tollana/t03a2-props` | W1b-INT commit | W1b-INT `done` |
| W1c | [PH-01b](#ph-01b--aoi-phase-filter-and-set_player_phase) | `rust-gameserver-dev` | `tollana-ph01b` / `tollana/ph01b-aoi-filter` | W1b-INT commit | W1b-INT `done` |
| W1c-R, W1c-INT | review both; fixes; integrate | as W1a | - | - | both `done` |
| W1d | [T-03a3](#t-03a3--executor-walk-t6-t12) | `packet-coder` | `tollana-t03a3` / `tollana/t03a3-walk` | W1c-INT commit | W1c-INT `done` |
| W1d | [T-03a4](#t-03a4--relog-abandon-two-player-and-the-chain-doc) | `packet-coder` | `tollana-t03a4` / `tollana/t03a4-relog` | W1c-INT commit | W1c-INT `done` |
| W1d-R, W1d-INT | review both; fixes; integrate and bookkeeping | as W1a | - | - | both `done` |
| user | 568 U1-U7, placement checks, U-PH1..5 | **the user**, never an agent | - | - | W1d-INT |

T-02a runs alone first as the **measurement pilot**: its report's Measurement section calibrates the 40k sizing before the rest dispatch. If it exceeds about 55k working tokens, the orchestrator returns the remaining packets to the architect for a re-slice before W1b.

Packets inside one wave touch disjoint files and run in parallel, each in its own worktree with its own `sgw_<wt>` database. PH-01b goes to `rust-gameserver-dev` because it edits the AoI hot path and must judge bypass sites.

**Stop rules (every packet).** Return `blocked` with the exact error; never improvise.

1. A frozen literal ([w1-frozen.md](w1-frozen.md)) that a loader refuses, or that loads as different values: stop. Never edit the literal or weaken a loader.
2. A needed file outside the packet's **Writes** list, or any Rust change outside test modules in T-02a and T-03a*: stop.
3. Any need for a reward, an item row, `set_interaction_type`, a dialog button, a `db/scripts` migration, an edit to a mission, dialog or `dialog_set_maps` row, or a flip of an `is_enabled` / `step_enabled` flag: stop (owner decisions O1, D-TW1, D-TW4).
4. Two chains co-fire in a state, or a state fires no chain where [F5](w1-frozen.md#f5-expected-shapes-tests-assert-these) promises one: stop.
5. `complete_objective 2312` or the Core's double advance completes mission 568 before turn-in: stop.
6. After `advance_step`, the new step's objective roster is empty: stop (a filter exists that research missed). Do not flip flags.
7. `dialog_choice` does not fire on a `-1` close of 2153 or 2154: stop. Do not add a button.
8. Slot 420 ever holds two live binds, or the `dialog_set_maps cache miss` WARN appears for 2656, 4959 or 6393: stop.
9. Anything needing the lab, a client, a running server or a new position: stop.
10. A lane check fails twice for the same reason: return `failed` with the log path. Do not widen the change.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/tollana-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-tollana`) |
| Base SHA | W1a: **the ARCH-W1 commit** (`git -C <zone-tollana> log -1 --format=%H -- docs/analysis/tollana-continuation/w1-packets.md`). W1b, W1c, W1d: the previous INT step's commit (its return line names it). |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <wt-name>` (branches from `main`). 2. Ops: `git -C <wt> rebase --onto <base> HEAD <branch>`. 3. Verify `git -C <wt> rev-parse HEAD` = `<base>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Report path | `<wt>\.claude\reports\tollana-continuation\<packet>-<role>.md` (role `coder`, `reviewer`, `rust-gameserver-dev`, `int`). Absolute: `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<wt>\.claude\reports\tollana-continuation\...`. |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (or `not exposed`), wall-clock start and end (UTC, `Get-Date -AsUTC`), lane runs, review rounds. |
| Commit trailer | `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM` (or the dispatching session's own lines). |
| Claimed IDs | Templates **420** (Copplemann), **421** (food unit). Spawns **19001-19004**; 19005-19010 reserved, unused. Chains **9001-9018**. Tags `Tollana_Copplemann`, `Tollana_FoodUnit_Core`, `Tollana_FoodUnit_Nest`, `Tollana_FoodUnit_Fungus`. Reused, not claimed: dialogs 2153, 2154, 4211, 4213, 4214; dialog-set-maps 2656, 4959, 6393 (slot 420). No new dialog or item id. [ID register](tollana-id-register.csv). Never pick another id. |
| Mission facts | 568: steps 1963 (objective 2309), 1964 (2310), 1965 (2311 required, 2312 optional), 1966 (4491). All steps have `step_enabled = false`; the loaders do not filter on it. `complete_objective` auto-completes the mission when every non-optional active objective is done, so **never `complete_objective 2311`**; `advance_step` force-completes the old step's objectives without auto-completing. |
| Gameplay contract | Offer only on Tollana while 568 is not active. Any close of 2153 accepts (zero buttons). Fungus or Nest moves 1964 to 1965; the Nest also credits optional 2312; the Core moves 1964 or 1965 to 1966. A close of 2154 at 1966 completes 568. Abandon repaints the offer. Done is silent. No reward (O1). Slot 420 holds at most one bind: S0 2656, S1964/S1965 4959, S1966 6393, done none. |
| Telemetry (owner rule) | Chains: existing executor lines only (`Content: accepting mission`, `Content: advancing step`, `Content: complete objective`, `Content: completing mission`, `Content: adding dialog set`, `Content: removing dialog set`). PH-01b adds one INFO event `phase.changed` ([F6](w1-frozen.md#f6-phase-model-names-ph-01a-ph-01b)). Nothing else. |
| Review (D-CC6) | One read-only `packet-reviewer` per wave; fixes by `packet-coder` (or the packet's own agent) in the owning worktree; a fresh `packet-reviewer` re-reviews; at most 3 rounds. No `packet-review-fixer`. Haiku agents stay under 100k tokens each. Reviewer reports `ship`, `fix` (with findings) or `block`. |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...` (fmt: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`). Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground; read their logs under `logs/<worktree-name>/` and check the test count is non-zero.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps**: soft 500 lines, hard 700, for Rust and new docs. Check `(Get-Content <file>).Count` before and after. Seed SQL and `docs/content/mission-chains.md` are exempt (append only). `space_manager/mod.rs` is 666 lines and `aoi.rs` 657: PH-01b may add at most 2 lines to `mod.rs` and 6 to `aoi.rs`.
6. **Line endings.** New seed files and docs are written with CRLF in the working tree: after writing, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`. Stored `i/lf w/crlf`: `db/database.sql` and every `db/resources/**/Seed/*.sql`. Stored `i/crlf`: `docs/content/mission-chains.md`, `docs/engine/space-management.md`. After `git add`, `git -C <wt> diff --cached --stat <path>` must count only your added lines. Rust files are `i/lf`; the Edit tool keeps them.
7. **Edit only the files your packet lists.** Never edit `README.md`, `RESUME.md`, the ID register or `id-allocation.md` (the INT steps and the orchestrator own them).
8. **IDs come only from the contract's claimed list**; literals only from [w1-frozen.md](w1-frozen.md), copied byte for byte.
9. **Every test must fail when its change is reverted.** Each packet names its revert proof; do it once (revert, run, see the failure, restore) and record the failing assertion in the report. Live-DB tests are `#[tokio::test]`, named `live_db_*`, and start with `let pool = require_db_or_skip!();`.
10. Run `git -C <wt> diff --check` before committing. Commit subjects are conventional (`feat(tollana): ...`, `test(tollana): ...`).

---

## T-02a — Copplemann and food-unit cast (pilot)

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 30k working tokens |
| Branch / worktree | `tollana/t02a-cast` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t02a` |
| Writes | `db/resources/Entities/Seed/entity_templates_tollana.sql` (new); `db/resources/Worlds/Seed/spawnlist_tollana.sql` (new); `db/database.sql` (two lines); `crates/cell-world/src/cell/spawner_tests/tollana_world19_cast.rs` (new); `crates/cell-world/src/cell/spawner_tests/mod.rs` (one `mod` line, one doc bullet); `crates/cell-catalog/src/cell/spawner/tests/live_db_spawnlist_sequence.rs` (the constant and its doc comment) |
| Commit | `feat(tollana): seed Copplemann and the food-unit props on Tollana` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t02a\.claude\reports\tollana-continuation\T-02a-coder.md` |

**Why.** Mission 568 needs a Copplemann on Tollana and three clickable food-duplication units. Template 48 (the Command Center Copplemann) cannot be reused: Harset bindings 6503/6504 key on it. Template 421 is one shared prop template, as Dakara 445-448 are.

**Steps.**

1. Create both seed files exactly as [F1](w1-frozen.md#f1-templates-420-and-421-t-02a) and [F2](w1-frozen.md#f2-spawns-19001-19004-t-02a) give them, then CRLF them (rule 6).
2. `db/database.sql`: add `\ir resources/Entities/Seed/entity_templates_tollana.sql` on the line after `\ir resources/Entities/Seed/entity_templates_debug_area_plaza.sql` (line 325), and `\ir resources/Worlds/Seed/spawnlist_tollana.sql` on the line after `\ir resources/Worlds/Seed/spawnlist_debug_area_plaza.sql` (line 389, before `stargates.sql`). Find them by text, not number. Templates must load before spawns.
3. `live_db_spawnlist_sequence.rs`: change `HIGHEST_RESERVED_SPAWN_ID` from `13869` to `19010`, and append to its doc comment (lines 11-17) `, Tollana 19001-19010 (tollana-continuation T-02a, spawnlist_tollana.sql)` before `. Raise it`.
4. New `tollana_world19_cast.rs`, modelled on `spawner_tests/dakara_e1/templates.rs` and `population.rs` (read them first). Register it with `mod tollana_world19_cast;` in `spawner_tests/mod.rs` (after `mod spawn_records;`) and add a doc bullet `//! - [`tollana_world19_cast`]: live-DB guards for the Tollana 568 cast (templates 420-421, spawns 19001-19004; tollana-continuation T-02a).` before the Harset paragraph. Imports: `use crate::cell::spawner::load_spawn_templates;`, `use crate::test_support::require_db_or_skip;`, `use cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT;`, `use cimmeria_entity::navigation::{NavMesh, PathStatus};`, `use cimmeria_common::Vector3;`. Three tests:
   - `live_db_tollana_templates_load_as_seeded`: `load_spawn_templates(&pool)`; templates 420 and 421 present; assert the [F1](w1-frozen.md#f1-templates-420-and-421-t-02a) "expected loader values" through the `SpawnRecord` fields the Dakara test reads (class, interaction type, name id, mesh/body set; use whatever field names `SpawnRecord` has, read its definition). Then spawn 421 with the Dakara `spawned()` pattern (`crate::test_support::make_space_manager_with_player`, `spawn_npc_from_record`) and assert the entity's interaction type has `INT_MISSION_WORLD_OBJECT`.
   - `live_db_tollana_cast_rows_are_seeded_exactly`: `SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 19001 AND 19010` is 4; each row's `(world_id, template_id, tag, x, y, z, is_stationary, respawn_secs)` equals F2 (coordinates compared with `(a-b).abs() < 0.001`); `SELECT last_value FROM resources.spawnlist_spawn_id_seq` is at least 19010.
   - `live_db_tollana_cast_is_on_tollana_nav`: load `data/spaces/tollana.nav` as `dakara_e1/travel.rs::mesh` does (path from `CARGO_MANIFEST_DIR`, `../../data/spaces/tollana.nav`; a missing mesh panics, never skips). For each of the four rows, `mesh.is_point_valid(&Vector3::new(x, y, z))`; `mesh.find_path(copplemann, prop).status` is `PathStatus::Ok` for each prop; the three props are pairwise at least 3.0 m apart and each within 20.0 m of Copplemann; Copplemann is within 20.0 m of gate 4 at `(211.045, 0.606, -547.644)`. All distances are horizontal (x and z only). Both the primary set and F2-B meet these bounds (the F2-B Fungus unit is 15.7 m from its Copplemann).
5. If the nav test fails on any primary row, swap all four rows to the F2-B fallback set (both the seed and the test constants) and rerun. If F2-B also fails, stop and return `blocked` with both sets' failing rows.
6. Lanes, in order: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world -p cimmeria-cell-catalog --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 tollana_world19_cast`; `pwsh tools/build-lane/live-db-test.ps1 spawnlist_sequence`.
7. Revert proof: comment out the `spawnlist_tollana.sql` include in `database.sql`; `live_db_tollana_cast_rows_are_seeded_exactly` must fail. Restore. Record which coordinate set shipped (primary or F2-B).

**Docs owed.** None here. W1d-INT records the shipped set in the placement CSV.

## T-03a1 — Chains 9001-9010 (Copplemann, accept, turn-in, restore, abandon)

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 40k working tokens |
| Branch / worktree | `tollana/t03a1-copplemann` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a1` |
| Writes | `db/resources/Content/Seed/tollana_568_chains.sql` (new); `db/database.sql` (one line); `crates/content-engine/tests/it/interact_tag_linter.rs` (allowlist, 3 lines); `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` (one `mod` line); `crates/cell-content/src/cell/content/chain_replay_tests/mission_568/mod.rs` (new); `.../mission_568/copplemann.rs` (new) |
| Commit | `feat(tollana): mission 568 Copplemann, accept, turn-in and restore chains` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a1\.claude\reports\tollana-continuation\T-03a1-coder.md` |

**Why.** These ten chains are the mission's skeleton: the Copplemann click per state, accept on any close of 2153 (zero buttons, D-TW2), turn-in on a close of 2154, the progress bind after accept, the per-player bind restore on Tollana entry (binds are rebuilt empty on every world entry) and the abandon repaint.

**Steps.**

1. Create `tollana_568_chains.sql` with the header and the 9001-9010 blocks of [F3](w1-frozen.md#f3-chains-9001-9010-t-03a1) exactly; CRLF it.
2. `db/database.sql`: add `\ir resources/Content/Seed/tollana_568_chains.sql` on the line after `\ir resources/Content/Seed/tutorial_chains.sql` (line 408; find by text).
3. `interact_tag_linter.rs`, fn `allowlist`: before the closing `)` of `matches!` (after the `space_castle_cellblock_chains.sql` 5015 line), add:
   ```rust
        // tollana_568_chains.sql — mission 568 (tollana-continuation T-03a1).
        // reason: Copplemann's bit comes from the per-player dialog bind on slot 420.
        | ("tollana_568_chains.sql", 9001..=9003) // Tollana_Copplemann: per-player bind (2656/4959/6393), not a zone-wide flag
   ```
4. `chain_replay_tests/mod.rs`: add `mod mission_568;` after `mod mission_567;`.
5. `mission_568/mod.rs`: a `//!` header (what 568 is, the spec link, D-TW11: shapes and gates are resolve-level here, state transitions are executor-level in `rig.rs` siblings). Copy the helper shapes of `mission_1361/mod.rs` lines 57-157 (read it): `load` (use its panic text), `resolve_one`, `actions_of`, `with_tag`, `with_dialog`, `with_world_name`, plus:
   - `const TOLLANA: i32 = 19;` and `const STEPS: [&str; 4] = ["1963", "1964", "1965", "1966"];`
   - `#[derive(Clone, Copy, Debug)] enum S { S0, S1964, S1965, S1966, Done }` and `const ALL: [S; 5]`.
   - `fn ctx(world_id: i32, s: S) -> ExecutionContext`: `world_id = Some(world_id)`; S0 sets `mission_568_status` = `"not_active"`; S1964/S1965/S1966 set it `"active"`, the current step `mission_568_step_<id>_status` = `"active"` and every earlier step `"completed"`; Done sets the mission and all four steps `"completed"`.
   - `fn with_mission(ctx, id: i32)` setting param `mission_id` (int).
   - `async fn fired(engine: &ChainEngine, tt: TriggerType, ctx: &ExecutionContext) -> Vec<i64>`: build a `TriggerEvent { trigger_type: tt, source_entity: None, target_entity: None, params: ctx.params.clone() }`, `engine.resolve_event(&event, ctx)`, collect the chain ids of `resolved.actions` in `9001..=9018`, sort, dedup.
   - Mark helpers `pub(super)` so siblings reuse them; the full engine is `super::super::engine_loader::build_engine(Some(&pool)).await`.
6. `mission_568/copplemann.rs` (register `mod copplemann;` in `mission_568/mod.rs`). Tests:
   - `live_db_568_chains_9001_to_9010_load_with_exact_shapes`: for each id, `load(&pool, id)`; `chain.enabled`; `chain.conditions.len()` equals F5's `cond`; `chain.actions` equals F5's list (`Action` is `PartialEq`); the trigger matches with `matches!(chain.trigger, Trigger::OnInteractTag { ref entity_tag } if entity_tag == "Tollana_Copplemann")`, `Trigger::OnDialogChoice { dialog_id: 2153 }`, `Trigger::OnMissionAccepted { mission_id: 568 }`, `Trigger::OnPlayerLoaded { world_name: Some(ref w) } if w == "Tollana"`, `Trigger::OnMissionAbandoned { mission_id: 568 }` (`Trigger` is `cimmeria_content_engine::triggers::Trigger`; it has no `PartialEq`).
   - `live_db_568_copplemann_click_is_disjoint_and_exhaustive`: full engine; for each `S`, `fired(InteractTag, with_tag(ctx(TOLLANA, s), "Tollana_Copplemann"))` equals the F5 fired-chain row (S0 `[9001]`, S1964 and S1965 `[9002]`, S1966 `[9003]`, Done `[]`).
   - `live_db_568_dialog_choice_gates`: `DialogChoice` with `with_dialog(.., 2153)`: S0 `[9004]`, every other state `[]`, S0 in world 1 `[]`; with 2154: S1966 `[9005]`, S1964 `[]`, S1966 in world 1 `[]`.
   - `live_db_568_mission_accepted_fires_9006_only_on_tollana`: `MissionAccepted` with `mission_id` 568: world 19 `[9006]` in every state; world 1 `[]`.
   - `live_db_568_restore_and_abandon_gates`: `PlayerLoaded` with `world_name` `Tollana`: S0 `[9007]`, S1964 and S1965 `[9008]`, S1966 `[9009]`, Done `[]`, S0 in world 1 `[]`. `MissionAbandoned` with `mission_id` 568 in S0 (the evaluator sees post-abandon state): `[9010]`; world 1 `[]`.
   - Plain `#[test] fn seed_has_no_forbidden_actions()`: read `concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources/Content/Seed/tollana_568_chains.sql")`, normalize `\r\n`; assert it contains none of `'grant_xp'`, `'grant_cash'`, `'add_item'`, `'remove_item'`, `'set_interaction_type'`, `'complete_objective', 568, '2311'`; and every `content_actions` row's `delay_ms` is 0 (regex-free: each line starting `VALUES (` after a `content_actions` line ends with `, 0, <n>);`).
7. Lanes, in order: fmt; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-content-engine --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine` (the linters, including `dialog_button_linter`, unchanged and green); `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content mission_568`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_568`.
8. Revert proof: delete the `step_status '1966' neq` condition row of 9002 from the seed; the disjointness test must fail (9002 and 9003 co-fire at S1966). Restore.

**Docs owed.** None here (T-03a4 writes the chain doc).

## T-03a2 — Chains 9011-9018 and the executor rig

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 40k working tokens |
| Branch / worktree | `tollana/t03a2-props` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a2` |
| Writes | `db/resources/Content/Seed/tollana_568_chains.sql` (append); `crates/content-engine/tests/it/interact_tag_linter.rs` (one allowlist line); `.../chain_replay_tests/mission_568/mod.rs` (two `mod` lines); `.../mission_568/props.rs` (new); `.../mission_568/rig.rs` (new) |
| Commit | `feat(tollana): mission 568 food-unit chains and executor rig` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a2\.claude\reports\tollana-continuation\T-03a2-coder.md` |

**Why.** The three props carry all of 568's progression. The rig drives the real executor (`fire_interact_tag`, `fire_dialog_choice`, `fire_player_loaded`, `execute_actions`) against the seeded mission defs, so later tests assert real per-player mission state and binds (D-TW11), as `mission_681_686_backstop.rs` does.

**Steps.**

1. Append [F4](w1-frozen.md#f4-chains-9011-9018-t-03a2-appended-to-the-same-file) to the seed after chain 9010; keep CRLF; `diff --stat` counts only added lines.
2. Linter allowlist, after the T-03a1 line: `| ("tollana_568_chains.sql", 9011..=9018) // Tollana_FoodUnit_*: template 421 default INT_MissionWorldObject (1073741824), as Dakara templates 445-448`.
3. `mission_568/props.rs`: `live_db_568_chains_9011_to_9018_load_with_exact_shapes` (as T-03a1's shape test, F5 rows 9011-9018) and `live_db_568_props_are_disjoint_per_state`: for each prop tag and each `S`, `fired(InteractTag, ..)` equals the F5 fired-chain row (Core: S0 9013, S1964 9011, S1965 9012, S1966 9013, Done 9013; Nest: 9016, 9014, 9015, 9016, 9016; Fungus: 9018, 9017, 9018, 9018, 9018).
4. `mission_568/rig.rs`, modelled on `mission_681_686_backstop.rs` lines 20-175 (read it). Exact shape:
   ```rust
   pub(super) const PLAYER: u32 = 7568;
   pub(super) const PLAYER_ID: i32 = 56_800;
   pub(super) const PLAYER_B: u32 = 7569;
   pub(super) const PLAYER_B_ID: i32 = 56_900;
   pub(super) const SLOT: i32 = 420;
   pub(super) struct Rig { pub mgr: SpaceManager, pub engine: ChainEngine, tx: mpsc::Sender<CellToBaseMsg>, rx: mpsc::Receiver<CellToBaseMsg> }
   pub(super) async fn rig(pool: &sqlx::PgPool) -> Rig
   ```
   `rig()`: `SpaceManager::new(1)`; `parse_spaces_xml(r#"<?xml version="1.0"?><Spaces><Space WorldName="Tollana" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#)`; `create_startup_spaces(r#"<?xml version="1.0"?><Spaces><Space WorldName="Tollana" /></Spaces>"#)`; `mgr.worlds.get_mut("Tollana").expect("parsed").world_id = Some(19);`; `mgr.mission_defs = crate::cell::spawner::load_mission_defs(pool)`, `mgr.step_objectives = crate::cell::spawner::load_step_objectives(pool)`, `mgr.dialog_set_maps = crate::cell::spawner::load_dialog_set_maps(pool)` (all three are required: without the second `advance_step` loads an empty roster, without the third `add_dialog_set` cache-misses); create both players at `[206.0, -0.58, -543.0]` with `is_player = true`, `player_id = Some(..)`, `connect_entity`; `mpsc::channel(8192)`; `build_engine(Some(pool)).await`. Methods (each `async` where it calls the executor, each draining `rx` afterwards):
   - `click(who: u32, tag: &str) -> bool`: `fire_interact_tag(who, pid(who), tag, 0, &engine, &tx, &mut mgr)`.
   - `close(who, dialog_id: i32)`: `fire_dialog_choice(who, pid(who), dialog_id, -1, ..)`.
   - `enter(who)`: clear `available_interactions` on the entity, then `fire_player_loaded(who, pid(who), "Tollana", ..)`.
   - `abandon(who)`: `execute_actions(ResolvedActions { actions: vec![(0, Action::AbandonMission { mission_id: 568 })], action_delays: vec![0], params: HashMap::new() }, who, pid(who), &tx, &mut mgr, &engine)` (it fires `mission_abandoned`, so 9010 runs).
   - `binds(who) -> Vec<i32>`: the dialog-set-map ids (`.0`) of `available_interactions.get(&SLOT)`, empty if none.
   - `status(who) -> Option<i8>`, `step(who) -> Option<i32>` (`current_step_id`), `completed(who) -> Vec<i32>` (`completed_objectives`), via `missions.get_mission(568)`.
   - `fn drain(&mut self)`: `while self.rx.try_recv().is_ok() {}`.
   Imports: `crate::cell::content::{fire_dialog_choice, fire_interact_tag, fire_player_loaded}`; `super::super::super::executor::execute_actions` (the backstop sits one level higher and uses `super::super::executor`; `rig.rs` is inside `mission_568/`, so it needs one more `super`); `cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED}`. Copy the remaining imports (`SpaceManager`, `CellToBaseMsg`, `ChainEngine`, `ResolvedActions`, `Action`, `mpsc`, `HashMap`) from the backstop file.
5. In `rig.rs`, one smoke test `live_db_568_rig_accept_on_2153_close` (spec T5): `enter(PLAYER)` gives binds `[2656]`; `click(PLAYER, "Tollana_Copplemann")` is `true`; `close(PLAYER, 2153)`; then `status == Some(MISSION_ACTIVE)`, `step == Some(1964)`, `completed` contains 2309 and not 2310, binds `== [4959]`.
6. Register `mod props;` and `mod rig;` in `mission_568/mod.rs`.
7. Lanes: fmt; clippy as T-03a1; `nextest run -p cimmeria-content-engine`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_568`.
8. Revert proof: delete 9004's `remove_dialog_set 2656` row from the seed locally; the smoke test must fail on binds (`[2656, 4959]`, not `[4959]`). Restore and confirm the seed's `git diff` shows only your appended block.

**Docs owed.** None here.

## T-03a3 — Executor walk (T6-T12)

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens |
| Branch / worktree | `tollana/t03a3-walk` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a3` |
| Writes | `.../chain_replay_tests/mission_568/walk.rs` (new); `.../mission_568/mod.rs` (one `mod walk;` line) |
| Commit | `test(tollana): walk mission 568 through the executor` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a3\.claude\reports\tollana-continuation\T-03a3-coder.md` |

**Why.** Shape tests prove what each chain says; these prove what the executor does with them, including the auto-complete trap.

**Steps.** Use only `rig.rs` (do not edit it). Each test starts `let mut r = rig(&pool).await; r.enter(PLAYER).await; r.click(..Copplemann).await; r.close(PLAYER, 2153).await;` to reach S1964 (factor this into a local `async fn accepted(pool) -> Rig`). Tests:

- `live_db_568_reclose_2153_when_active_is_a_no_op` (T6): `close(PLAYER, 2153)` again; status, step, completed and binds unchanged.
- `live_db_568_core_at_1964_reaches_1966` (T7): click Core; `step == Some(1966)`, `status == Some(MISSION_ACTIVE)` (**not** completed), completed contains 2310 and 2311 (each `advance_step` force-completes the old step's objectives), binds `[6393]`. Record in the report whether 2312 is in `completed`; do not assert it.
- `live_db_568_core_at_1965_reaches_1966` (T8): click Fungus, then Core; step 1966, binds `[6393]`, status active.
- `live_db_568_nest_credits_2312_once` (T9): click Nest; step 1965, completed contains 2312, not 2311, status active; click Nest again; step, status and completed unchanged.
- `live_db_568_fungus_advances_1964_only` (T10): click Fungus; step 1965, completed contains neither 2311 nor 2312; click Fungus again: unchanged.
- `live_db_568_turn_in_completes_and_unbinds` (T11): Core, then `click(Copplemann)` is `true`, then `close(PLAYER, 2154)`; `status == Some(MISSION_COMPLETED)`, binds empty.
- `live_db_568_done_state_is_silent` (T12): after T11's walk, `click(Copplemann)` returns `false`; clicking each prop leaves status completed and binds empty.

Lanes: fmt; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_568`. Revert proof: in the seed, replace 9011's `advance_step 1966` row with `complete_objective 568 '2311'` (local only, never committed); T7 must fail on status (mission completed early). Restore and confirm `git diff` on the seed is empty.

**Docs owed.** None.

## T-03a4 — Relog, abandon, two-player and the chain doc

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens |
| Branch / worktree | `tollana/t03a4-relog` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a4` |
| Writes | `.../chain_replay_tests/mission_568/relog_abandon.rs` (new); `.../mission_568/two_player.rs` (new); `.../mission_568/mod.rs` (two `mod` lines); `docs/content/mission-chains.md` (one new section) |
| Commit | `test(tollana): mission 568 relog, abandon and two-player guards` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-t03a4\.claude\reports\tollana-continuation\T-03a4-coder.md` |

**Steps.** Use only `rig.rs` (do not edit it).

1. `relog_abandon.rs`:
   - `live_db_568_slot_420_never_holds_two_binds` (T13): walk enter, accept, Nest, Core, turn-in; after every step assert `binds(PLAYER).len() <= 1` and equals the F5 bind ladder (`[2656]`, `[4959]`, `[4959]`, `[6393]`, `[]`).
   - `live_db_568_entry_repaints_each_state` (T14): walk to S0, S1964, S1965 (Fungus), S1966 (Core), Done; at each, `enter(PLAYER)` and assert binds `[2656]`, `[4959]`, `[4959]`, `[6393]`, `[]`.
   - `live_db_568_abandon_then_reaccept` (T15): accept, Fungus (1965), `abandon`; `status(PLAYER)` is `None` (abandon drops the mission), binds `[2656]`; click Copplemann, close 2153: step 1964, binds `[4959]`. Separately: accept, Core (1966), abandon: binds `[2656]`.
2. `two_player.rs`, `live_db_568_two_players_are_isolated` (T16): both players `enter`; A accepts and clicks Core; A is at step 1966 with binds `[6393]`; B's `status` is `None` and B's binds are `[2656]`; B clicks Core: B's state unchanged (`None`, `[2656]`); B can still accept (close 2153 after a click) and reaches 1964.
3. `docs/content/mission-chains.md`: insert a `### Tollana (World 19)` section immediately before `### General (Debug/Test)` (find by text). Content: one table of chains 9001-9018 (id, trigger, gate, effect, one line each, from F5), a line "All CIMMERIA_RECONSTRUCTION; item-less pilot; no reward (owner O1); never `complete_objective 2311`", and links to the spec and [w1-frozen.md](../analysis/tollana-continuation/w1-frozen.md). The file is stored `i/crlf`: write CRLF, check `diff --stat` counts only added lines.
4. Lanes: fmt; clippy `-p cimmeria-cell-content`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::mission_568`; `python tools/docs-gen/regen.py --check` if it exists (read `tools/README.md`; never hand-edit a `<!-- gen:` block).
5. Revert proof: delete chain 9010's `add_dialog_set 2656` row locally; T15 must fail (binds empty after abandon). Restore; seed `git diff` empty.

**Docs owed.** `docs/content/mission-chains.md` (doc-update map row for new content chains); list it in the report.

## PH-01a — Phase model in the entity crate

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 25k working tokens |
| Branch / worktree | `tollana/ph01a-phase-model` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-ph01a` |
| Writes | `crates/entity/src/cell_entity/phase.rs` (new); `crates/entity/src/cell_entity/mod.rs` (two lines) |
| Commit | `feat(entity): add the phase key and visibility model` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-ph01a\.claude\reports\tollana-continuation\PH-01a-coder.md` |

**Why.** Phasing makes an entity visible only to players in its phase, through AoI membership (design: [phasing.md](evidence/phasing.md)). D-TW9 keeps the state in `CellEntity.extensions` (the type-keyed feature store, `cell_entity/extensions.rs`: `get::<T>()`, `get_mut`, `insert`, `remove`), so no edit to the 850-line `entity_struct.rs`. This packet is the decision-independent core (D-TW10); persistence, verbs and spawn columns wait on owner decisions.

**Steps.**

1. `phase.rs` with the exact names of [F6](w1-frozen.md#f6-phase-model-names-ph-01a-ph-01b): `PhaseKey`, `PhaseKeyError`, `PhaseState`, `phase_visible`, `viewer_phases`. Semantics:
   - `PhaseKey::new`: empty is `Empty`; more than 64 bytes is `TooLong(len)`; the first char outside `[a-z0-9_]` is `BadChar(c)`.
   - `viewer_phases(e)`: `e.extensions.get::<PhaseState>()` → its `player_phases.clone()`, else empty.
   - `phase_visible(viewer, other)`: `if other.is_player { return true; }` with the comment `// D-PH1 (owner, open): players are never phased in v1.`; then `match other.extensions.get::<PhaseState>().and_then(|s| s.entity_phase.as_ref()) { None => true, Some(k) => viewer.contains(k) }`.
   - `//!` header: what a phase is, the link to `docs/analysis/tollana-continuation/evidence/phasing.md`, and that the AoI filter lives in `cimmeria-cell-world` (PH-01b).
2. `cell_entity/mod.rs`: add `mod phase;` after `mod pet;` and `pub use phase::{phase_visible, viewer_phases, PhaseKey, PhaseKeyError, PhaseState};` after the `pub use pet::...` line.
3. Unit tests in `phase.rs` (`#[cfg(test)] mod tests`), entities from `CellEntity::new(EntityId(1), SpaceId(100), Vector3::new(10.0, 0.0, 20.0))` (`use cimmeria_common::{EntityId, SpaceId, Vector3};`):
   - key validation: `"svarog_vault"` ok; `""` Empty; 65 chars TooLong(65); `"Svarog Vault"` BadChar('S').
   - truth table: unphased NPC visible to an empty viewer; phased NPC hidden from an empty viewer, visible to a viewer holding its key, hidden from a viewer holding another key; a player with `entity_phase` set is still visible (D-PH1).
   - `viewer_phases` is empty without a `PhaseState` and equals the inserted set with one.
4. Lanes: fmt; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-entity --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-entity phase`.
5. Revert proof: make `phase_visible` return `true` unconditionally; the "hidden from an empty viewer" test must fail. Restore.

**Docs owed.** None (PH-01b owns the engine doc row).

## PH-01b — AoI phase filter and `set_player_phase`

| Field | Value |
|---|---|
| Agent | `rust-gameserver-dev` |
| Size | About 45k working tokens (domain agent) |
| Branch / worktree | `tollana/ph01b-aoi-filter` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-ph01b` |
| Writes | `crates/cell-world/src/cell/space_manager/phase.rs` (new); `space_manager/aoi.rs` (at most 6 lines); `space_manager/mod.rs` (at most 2 lines); `space_manager/tests/phase.rs` (new); `space_manager/tests/mod.rs` (one line); `docs/engine/space-management.md` (one section) |
| Commit | `feat(cell-world): filter AoI by phase and recompute on phase change` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\tollana-ph01b\.claude\reports\tollana-continuation\PH-01b-rust-gameserver-dev.md` |

**Why.** A phased entity must neither enter nor stay in the AoI of a viewer outside its phase, and a phase change must take effect at once, not on the next tick. Leaving AoI already sends `LeftAoI` and clears the witness entry, so the client sees a normal leave and, on re-entry, a fresh create.

**Steps.**

1. `aoi.rs`, `compute_player_aoi` (line 264): add `viewer_phases(e)` (from `cimmeria_entity::cell_entity`) as a sixth element of the extraction tuple at lines 273-286; after `if !other.is_introducible() { continue; }` (lines 310-312) add `if !phase_visible(&viewer_phases, other) { continue; }`. An entity already in `previous_aoi` that the filter drops then leaves through the existing leave loop (lines 333-359); do not add a second leave path.
2. Bypass sites: grep `push_introduction`, `introduction_events`, `EnteredAoI` and `witnesses.insert` across `crates/cell*`. `introduction_events` (line 73, the respawn rebuild for an existing witness) works off an existing witness set and is expected to stay as is; list every site with one line of reasoning in the report. If more than two sites introduce an entity without passing through `compute_player_aoi`, stop and return `blocked` with the list.
3. `space_manager/phase.rs`: `PhaseError` and `set_player_phase` per [F6](w1-frozen.md#f6-phase-model-names-ph-01a-ph-01b). Missing entity → `PlayerMissing`; not `is_player` → `NotAPlayer`. Insert a default `PhaseState` if absent; add or remove the key; if the set did not change return `Ok(vec![])` without recomputing; otherwise return `self.compute_aoi_changes_for_player(player_id)` and log `tracing::info!(event = "phase.changed", player_id, phase = %phase, member, entered, left, "phase changed")` where `entered`/`left` count `EnteredAoI`/`LeftAoI` in the result. Register `mod phase;` in `space_manager/mod.rs` and re-export `PhaseError` if `mod.rs` re-exports siblings.
4. `tests/phase.rs` (register `mod phase;` in `tests/mod.rs`), on `make_manager()` with the existing APIs (`create_entity`, `connect_entity`, `spawn_npc`, `compute_aoi_changes`, `get_witnesses_of`, `get_entity_mut`). Phase a prop by inserting `PhaseState { entity_phase: Some(key), ..Default::default() }` into its `extensions`. Tests:
   - `phased_prop_is_not_introduced_without_the_phase`: player A and a phased prop 10 m apart; `compute_aoi_changes()` yields no `EnteredAoI` for the prop.
   - `joining_the_phase_introduces_once`: `set_player_phase(A, key, true)` yields exactly one `EnteredAoI { entity_id: prop }`; a second call returns `Ok(vec![])`; ten further ticks add nothing.
   - `leaving_the_phase_sends_one_leave`: then `set_player_phase(A, key, false)` yields exactly one `LeftAoI` for the prop, and `get_witnesses_of(prop)` no longer holds A.
   - `two_players_see_only_their_phase`: A in the phase, B not, same spot; after 50 ticks `get_witnesses_of(prop)` is `[A]`; A and B still witness each other.
   - `unphased_entities_are_unaffected`: an unphased NPC enters for both players as before.
   - `set_player_phase_rejects_non_players`: an NPC id gives `NotAPlayer`, an unknown id `PlayerMissing`.
5. `docs/engine/space-management.md` (stored `i/crlf`): add a short `## Phases` section: the rule (`phase_visible` in the entity crate, applied in `compute_player_aoi`), `set_player_phase` recomputes at once, the `phase.changed` event, the bypass-site list, and "not yet: persistence, content verbs, spawn column, refusal gates (PH-02..PH-04, owner decisions D-PH1..D-PH7)".
6. Lanes: fmt; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-entity -p cimmeria-cell-world --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world`.
7. Revert proof: delete the filter line in `aoi.rs`; `phased_prop_is_not_introduced_without_the_phase` and `two_players_see_only_their_phase` must fail. Restore.

**Docs owed.** `docs/engine/space-management.md` (engine behavior row of the doc-update map).

## INT steps — W1a-INT, W1b-INT, W1c-INT, W1d-INT

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 15k working tokens each (W1d-INT about 25k) |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-tollana` (`content/tollana-campaign`) |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-tollana\.claude\reports\tollana-continuation\W1x-INT-int.md` |

**Steps (each INT).**

1. Confirm `git -C <zone-tollana> status --porcelain` is empty. For each packet of the wave in dispatch-plan order, `git -C <zone-tollana> cherry-pick -x <commit>` for its coder commit and every fix commit, oldest first (the orchestrator's brief lists the SHAs). On a conflict that is only adjacent `mod` lines in `mission_568/mod.rs` or adjacent allowlist lines, keep both lines in sorted order and continue; any other conflict: `git cherry-pick --abort` and return `blocked`.
2. Run the union of the wave's lanes from the zone worktree (fmt, each clippy, each nextest, each live-DB filter).
3. Return the new tip SHA.

**W1d-INT bookkeeping (only after its cherry-picks pass).** In `docs/analysis/tollana-continuation/`: in `tollana-id-register.csv` set the status of every claimed row to `SEEDED` and note the integration SHA in `remaining_gate`; in `tollana-placement-contract.csv` record which coordinate set T-02a shipped. Keep both files CRLF. Commit `docs(tollana): record W1 integration`. Do not touch README.md or RESUME.md (the orchestrator owns them).

---

## Blocked: no packet this wave

| Item | Status | Reason |
|---|---|---|
| T-03b, T-03c (569, 570) | BlockedEvidence | 569/570 direction conflicts, plus combat and item contracts unsealed. |
| T-04 to T-11 | BlockedEvidence | Mission chronology unresolved for every other Tollana record. |
| T-05a, T-05b (1364, 1366) | BlockedEvidence | Lineage and grant/recipient contracts. |
| T-09 (621 vault) | BlockedDependency | T-08e and the vault placement; 618 → Moh'katan and 621 → Ba'al are proposals only. |
| T-11s | BlockedEvidence | No side child admitted. |
| T-12, T-13 | BlockedDependency | CB-N05a client proof of authored catalog entries. |
| T-14 close-out | BlockedDependency | Campaign implementation, review and the user's UAT. |
| Item-backed 568 (repair fragments) | BlockedDependency | Harset HA-1360a possession and consume primitive. |
| PH-02 refusal gates | BlockedDependency | PH-01b; also wants D-PH1. |
| PH-03 persistence | BlockedDecision | D-PH3 (table or derived), D-PH4 (per character or per world). |
| PH-04 verbs and spawn phase | BlockedDecision | D-PH5 (spawn column or tag); persistence call needs PH-03. |
| PH-05 wire and two-client proof | BlockedDependency | PH-01 to PH-04. |
| A real Decline on 2153 | BlockedDecision | A button strands the accept (linter R2); owner design call. |
| 568 reward | BlockedDecision | Owner O1; ships as none. |

## UATPending: the user's manual UAT

No agent runs these. They go to the user after W1d-INT, on a server built from `content/tollana-campaign`.

| Id | Check |
|---|---|
| P1-P9 | Placement checks 1-9 of [T-01 offline placement](evidence/T-01-offline-placement.md): Copplemann and the three units stand on the floor near gate 4, face sensibly, are reachable on foot, and are clickable from the path. |
| P10 | Template 421's mesh: swap ViewScreen02 for the food-unit mesh if a better client mesh is found (D-TW7). |
| U1 | Offer glyph on Copplemann; 2153 text and speaker. |
| U2 | Each unit's dialog (4211, 4213, 4214) and the journal change 1964 → 1965 → 1966. |
| U3 | Speaker label on 2153/2154 (screen speaker 1100 versus template speaker 968). |
| U4 | Relog at each state keeps the right glyph. |
| U5 | Abandon, then re-accept. |
| U6 | Turn-in, then the silent completed state. |
| U7 | The journal shows step objectives although `step_enabled` is false. |
| T18 | Double-click Copplemann before closing 2153: one accept (the executor rig cannot cover the `open_dialog_id` gate, D-TW11). |
| U-PH1..U-PH5 | [phasing.md § 11](evidence/phasing.md), once a content verb exists (PH-04); not testable from W1 alone. |
