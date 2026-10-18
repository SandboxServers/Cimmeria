# Castle continuation: wave R2 packets (no-lab content restoration)

> **Type:** Reference. **Audience:** Castle orchestrator, ops, packet workers and reviewers.
> **Last updated:** 2026-10-17.
> **Companions:** [overlay ledger](README.md), [resume card](RESUME.md), [R1 packets and isolation](work-packets.md), [R2 architect report](architect-report-r2.md), branch ledger `content/castle-campaign:docs/analysis/castle-continuation/{README.md,work-packets.md,resume-packets.md}`, [Castle packet menu](../four-zone-planning/castle-work-packets.md).

Main-checkout overlay, never committed to `main` (D-CA2). CS-INT2a copies it to the branch as `docs/analysis/castle-continuation/r2-packets.md`.

The user's direction on 2026-10-17 was to abandon lab work for now, go back to content restoration, and leave UAT to the user at the end (D-CA11). R2 restores missions 1520 and 1521 server-side and seals the remaining docs gates, all without the lab.

## Dispatch rules (R2)

- **No lab.** No R2 packet starts a game client, takes a lab lease, calls a `lab_*`, `client_*` or `server_*` MCP tool, or starts a Castle server. A claim only the client can prove is written as `UATPending (end UAT, D-CA11)` and goes into the branch file `docs/analysis/castle-continuation/uat/end-uat.md`.
- One packet, one worker, one worktree (`C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\castle-<packet>`), and one test DB (`sgw_castle_<packet>`, for example `sgw_castle_cs_02c1`). Ops makes each worktree from the main checkout: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`, one ops dispatch per worktree.
- Sizes:
  - a packet-coder packet takes about 40k working tokens;
  - a specialist seal takes about 50k. If it runs over, it commits what is sealed and returns `blocked`, naming the remainder.
- Review follows D-CA6 for each wave:
  1. one read-only `packet-reviewer` for the whole wave;
  2. fixes by a fresh `packet-coder` (for code and seeds) or a fresh domain agent (for seals), in the owning worktree;
  3. a fresh re-review, at most 3 rounds.
- Integration is by cherry-pick onto `content/castle-campaign` in `zone-castle` (CS-INT2a, CS-INT2). Retire each packet worktree only after its commit is integrated. ops refuses `--force`, so the owner runs `pwsh tools/build-lane/rm-worktree.ps1 <name> --force` (D-CA14).
- Cellblock is live. Never touch any of these:
  - `zone-cellblock` or any `cellblock-*` worktree;
  - `sgw_zone_cellblock`, any `sgw_cellblock_*` DB, or `sgw_castle_cs_01b` (it holds Cellblock's test character `Cellutb`);
  - Cellblock's server;
  - `docs/analysis/cellblock-continuation/`;
  - `.claude/state/active-campaign`.

### Repo rules every R2 packet carries

1. PowerShell only, never bash. Use absolute paths, and start every shell call with `Set-Location <worktree>;`.
2. Never run `cargo` directly. Run `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>` in the foreground, and read the lane's failures file instead of rerunning. `cargo fmt --all -- --check` does not compile and may run directly.
3. Live-DB tests: `pwsh tools/build-lane/live-db-test.ps1 <filter> -p <crate>`. It reloads this worktree's own DB from the seeds.
4. Git:
   - no `git stash`, no `git worktree prune`, no push, no PR;
   - stage by explicit path, never `git add -A`;
   - commit once with the packet's subject and the attribution lines your brief gives (default `Co-Authored-By: Claude <noreply@anthropic.com>`).
5. Edit only the packet's allowlist. If a file outside it is needed, stop and return `blocked`.
6. Files under `docs/` are CRLF. After writing one, run ``$t=[IO.File]::ReadAllText($p) -replace "(?<!`r)`n","`r`n"; [IO.File]::WriteAllText($p,$t)`` with an absolute `$p`, then `git diff --check`.
7. File caps: 500 lines soft, 700 hard. Split a new file before it reaches 500. Do not grow a file that is already over 700.
8. Seeds are the source of truth. Edit `db/resources/`, never `db/scripts/`. Only the `c1` packets edit seeds.
9. Never write an IP address other than loopback, a password or a token into any file.
10. Report and return: write the full report at the path in your brief, then return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

## Contract excerpt (R2)

- **Branch.** The feature branch is `content/castle-campaign` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle`. `FEATURE_SHA` is `da2b82c70` until CS-INT2a returns; after that it is the CS-INT2a SHA, which RESUME.md records. Packet branches are named `castle/<packet-lower>-<slug>`.
- **Statuses:** Ready, BlockedDependency, BlockedDecision, BlockedEvidence, Writing, Review, Integrated, UATPending, Done.
- **Provenance.** Each claim carries one label: `SHIPPED_DATA`, `RECOVERED_SCRIPT`, `CLIENT_MAP`, `CIMMERIA_RECONSTRUCTION` (bounded and delegated under O14-03), `ORAL_HISTORY` or `CIMMERIA_AUTHORING`. A claim without proof is `UNRESOLVED (<gate>)`. A blank field or a guessed mechanic never counts as a resolution.
  - **Label form (decided, R2a review N1).** The reconstruction label is written `CIMMERIA_RECONSTRUCTION` everywhere: seed descriptions end in `(CIMMERIA_RECONSTRUCTION)`, the chain 1366 form (`castle_carry_in_chains.sql:4`) and the entry seal's. Older text and label lists that say `RECONSTRUCTION` mean the same label. No seed row ever carries `(RECONSTRUCTION)`.
- **Chain IDs.** Only the 1366 row is seeded. 1367-1377 are claimed, but no seed has them yet: "entry chains ship only with a completable branch".

  | Block | Owner | Use |
  |---|---|---|
  | 1366 | CS-08a1 | seeded (Frost's Letter exit route) |
  | 1367, 1368, 1371-1373 | 1520 (CS-02c1) | 1367 offer, 1368 credit 5171, 1371 dispenser to Livewire, 1372 victory, 1373 completion |
  | 1369, 1370, 1374-1377 | 1521 (CS-03c1) | 1369 offer, 1370 advance 4456, 1374 tank to Livewire, 1375 victory, 1376 second tank use, 1377 completion |
  | 1378-1385 | CS-02S | any extra 1520 chain (Ogilvie, dialog closes, relog restores) |
  | 1386-1393 | CS-03S | any extra 1521 chain |
  | 1394-1397 | CS-00R2 | a sealed G-567 graph for CS-08b |
  | 1398-1400 | reserve | CS-06 (1664) and fixes; the architect assigns them |

- **Other new IDs** (template candidates 412/413, spawn candidates 13801/13802, any dialog set) must pass a no-collision check. Run `git -C <wt> grep -n -w <id> <ref> -- db/resources` on `FEATURE_SHA`, `content/cellblock-campaign`, `content/harset-campaign` and `content/tollana-campaign`, and paste the output into the seal.
- **Seed shape** (chain 1366, `db/resources/Content/Seed/castle_carry_in_chains.sql:1-16`):

  ```sql
  INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (<id>, '<mission> - <what> (<LABEL>)', 'mission', <mission>, true, 0);
  INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (<id>, '<event>', '<key>', 'player', false, 0);
  INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
    (<id>, 'world', 8, NULL, 'eq', NULL, 0), (<id>, 'mission_status', <m>, NULL, 'eq', 'active', 1), (<id>, 'step_status', <m>, '<step>', 'eq', 'active', 2);
  INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (<id>, 'advance_step', <m>, '<step>', '{}', 0, 0);
  ```

- **Trigger vocabulary in use:**
  - `dialog_choice` (the key is the dialog id), `enter_region` (`Castle.Infirmary`), `interact_tag` (a spawn tag), `player_loaded`, `mission_accepted`, `entity_dead_tag`, `stargate_crossed`, `stargate_dialed`.
  - A chain with no trigger gets the loader's synthetic `__direct_invoke_<id>` event (`crates/cell-content/src/cell/content/loader/mod.rs:257-263`). Minigame callbacks use it.
  - Status values are `not_active`, `active` and `completed`; unpopulated state reads as `not_active`.
- **The `step_enabled` and `is_enabled` flags are not read at runtime.** Only `console_authoring` tests read them, and mission 708's step 2417 is `false` yet works. Missions 1520/1521 ship in `CookedDataMissions.pak` and need no catalog insertion (`crates/resources/src/base/resources/apply_overrides.rs:236-245`). No R2 packet flips those flags.
- **Minigame pattern.** Chain 1233 runs `('start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1234]}', 0, 0)`. Chain 1234 has no trigger and runs `remove_dialog_set`, `display_dialog` and `advance_step` (`castle_701_chains.sql:371-414`). The dialog-accept pattern is at `:245-278`; the difficulty params are in `chain_replay_tests/start_minigame_difficulty.rs`.
- **b2-lite** (D-CA12; CS-02L sealed it `change`, and its edits are folded in below). The minigame victory callback becomes player-bound. There is no client wire change; `BaseToCellMsg` is internal.
  - **Trust claim** (CS-02L minigame edit 1). Castle callbacks fire only for the ticket holder, once, after the server counted all `g` goal wires on a `Livewire` board. This is not proof of skill: the win check has no ordering, timing or geometry rule, so a scripted client can win instantly. Residual **R-LW1**, accepted, the same as for the ten legacy callbacks. This rests on the D-CA12 reading "player-bound and server-counted, not skill-proof", which awaits the owner's confirmation (RESUME.md, Open decisions).
  - `crates/wire/src/cell/messages/base_to_cell.rs:459-464` gains `player_id: i32` after `entity_id`, documented as "character id the Base receipt was minted for".
  - Base forwards `receipt.player_id` (`crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs:291`).
  - The Cell (`crates/cell/src/cell/service/base_messages/minigame.rs`) handles the result. On a victory it checks the player **once, before the chain loop**, and refuses the whole message on a failure:
    - non-victory: nothing fires (unchanged);
    - entity missing: WARN `reason="entity_missing"` with `entity_id`, `expected_player_id` and `chain_ids`, and **nothing fires** (today it fires with player 0);
    - entity not a player, `player_id == 0`, or `entity.player_id != Some(player_id)`: WARN `reason="player_mismatch"` with `entity_id`, `expected_player_id` (the message's `player_id`), `actual_player_id` (the entity's, `0` when `None`) and `chain_ids`, and nothing fires. These are the only field names; there is no `held_player_id`;
    - otherwise, for each chain: if `engine.chain_has_conditions(id)`, call `content::fire_guarded_minigame_chain_by_id(id, entity_id, engine, tx, space_mgr)`, which evaluates world and mission conditions; else call `content::fire_chain_by_id(id, entity_id, player_id, engine, tx, space_mgr)`. The guarded path keeps its own `is_player` and non-zero check as defence in depth. No seeded callback chain (1017, 1025, 1042, 1061, 1234, 1293, 13005, 1357, 3024, 7005) has conditions, so legacy behavior is unchanged.
  - **A guarded miss is visible** (CS-02L authority edit 4). Today `fire_guarded_minigame_chain_by_id` (`crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs:62-64`) returns with no row when `resolved.actions.is_empty()`. It gains an INFO row `reason="guarded_callback_not_matched"` with `chain_id` and `entity_id` just before that return (CS-MGF2). This makes a stale step, a wrong world, or a conditioned chain that also has a real trigger visible from telemetry alone.
  - **New accessor** (CS-02L authority edit 1). It goes in the existing sibling `crates/content-engine/src/chain/step_anchors.rs` (138 lines), inside its `impl ChainEngine` block at `:38`. `chain/mod.rs` is 537 lines and must not grow.

    ```rust
    /// Whether chain `chain_id` has at least one condition row. `false` for an unknown id.
    pub fn chain_has_conditions(&self, chain_id: i64) -> bool {
        self.chains_by_trigger.values().flatten().any(|c| c.id == chain_id && !c.conditions.is_empty())
    }
    ```

  - Every Castle callback chain (1372, 1375) carries `world 8` plus the exact `step_status ... active` condition, with `delay_ms = 0` on every action. This is enforced, not assumed: a delayed action is queued and runs later with no re-check (`executor/mod.rs:96-120`), and a lost condition row would silently demote a callback to the unguarded legacy path.
  - **Seed guard** (CS-02L authority edit 3 and minigame edit 2). A live-DB test that fails if reverted, for each starter and victory pair: 1371/1372 (mission 1520, step 4453) in CS-02c1, and 1374/1375 (mission 1521, step 4456) in CS-03c1. The c2 packets re-run it. It asserts:
    - (a) the victory chain loaded with `load_single_chain_for_test` has non-empty `conditions`. `chain_has_conditions` lands in CS-MGF1, a parallel branch, so assert the field it reads instead of calling it;
    - (b) those conditions include `Condition::World { world_id: 8, .. }` and the exact `Condition::StepStatus { mission_id, step_id, expected_status: StepStatusValue::Active, .. }` for the pair's mission and step;
    - (c) `SELECT count(*) FROM content_actions WHERE chain_id = $1 AND delay_ms <> 0` returns 0 for the victory chain;
    - (d) `SELECT count(*) FROM content_triggers WHERE chain_id = $1` returns 0 for the victory chain (the synthetic `__direct_invoke_<id>` event only);
    - (e) the starter's loaded `Action::StartMinigame` has `minigame_type == "Livewire"` (exact `==`, never a case-insensitive match), `difficulty == 1` and `on_victory_chains == vec![<victory>]`. Its raw `content_actions.params` also sets the difficulty explicitly (`params->>'difficulty' = '1'`), because the loader defaults an omitted value to 1. Pattern: `chain_replay_tests/livewire_pairs.rs:169`.
- **Placement rules** (`db/resources/Worlds/Seed/spawnlist.sql:573-591`, guarded by `crates/cell-catalog/src/cell/spawner/tests/live_db_castle_population.rs` and `crates/entity/tests/castle_navmesh.rs::every_castle_spawn_is_on_the_mesh`):
  - on the navmesh, with `y` at floor height;
  - no hostile aggro radius plus 3 reaches a mission actor;
  - friendlies sit at least 5 u outside any aggro radius.

  `castle_navmesh.rs` already checks every new world-8 spawnlist row.
- **Nav tool:** `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data\spaces\castle.nav --probe NAME=X,Y,Z`. Exit codes: 0 ok, 2 off tolerance, 3 in another component. `data/spaces/castle.nav` is tracked. The routable-test pattern is `mesh.find_path(&from, &to).into_waypoints()`, with the last waypoint within 2.0 of the target.
- **Replay harness.** `crates/cell-content/src/cell/content/chain_replay_tests/` holds live-DB tests that use `super::super::engine_loader::load_single_chain_for_test` and `crate::test_support::require_db_or_skip`.
  - Templates to copy: `livewire_pairs.rs` (start and callback), `region_transition_accepts.rs` (region offer), `mission_1360.rs` (step-guarded advance).
  - Register each new module in `chain_replay_tests/mod.rs`.
- **Shared append points** (decided, R2a review N3). CS-02c1 and CS-03c1 append to the same places, so CS-03c1 is stacked on CS-02c1 and always goes second:
  - `db/database.sql`: the CS-02S line `\ir resources/Content/Seed/castle_1520_chains.sql` goes directly after `:397` (`\ir resources/Content/Seed/castle_carry_in_chains.sql`). The CS-03S line `\ir resources/Content/Seed/castle_symbiote_chains.sql` goes directly after the CS-02S line;
  - `db/resources/Entities/Seed/entity_templates.sql`: template 412 at the end of the file, then 413 after it;
  - `db/resources/Worlds/Seed/spawnlist.sql`, world-8 block: spawn 13801 where CS-02S places it (after spawn 199), then 13802 where CS-03S places it;
  - `MISSION_ACTORS` (`crates/cell-catalog/src/cell/spawner/tests/live_db_castle_population.rs:72`, 517 lines): `"Castle_MedDispenser"` (9 to 10) from CS-02c1, then `"Castle_SymbioteTank"` (10 to 11) from CS-03c1.
- **Interact-tag linter.** `crates/content-engine/tests/it/interact_tag_linter.rs` reads only lines that start with `(` or `VALUES`, so the frozen one-line `INSERT ... VALUES (...)` rows pass it. Keep every frozen row on one line. A multi-line reformat makes chain 1371 (and 1374/1376) fail and needs an `allowlist` entry. The linter file is 753 lines, over the 700 hard cap, so a packet that finds it needs that entry returns `blocked` rather than growing the file.

### Base check (R2)

Step 0 of every worktree packet. Use `<BASE>`: the `FEATURE_SHA` for a fresh packet, or for a stacked packet the predecessor's reviewed tip that the brief names.

```powershell
Set-Location <wt>; git status --porcelain            # must be empty
git -C <wt> rev-list --count origin/main..HEAD       # must be 0
git -C <wt> rebase --onto <BASE> origin/main <branch>
git -C <wt> rev-parse HEAD                           # must equal <BASE>
```

If any value differs, stop and return `blocked`.

## Wave R2-0: housekeeping (now)

### CS-01d: stop the paused Castle lab server

**Agent:** packet-coder. **Worktree:** the existing `castle-cs-01b`. There is no branch change, no commit and no file edit. **Gate:** none (D-CA13).

1. Run `Get-Process -Id 36540 | Select-Object Id,Path,StartTime`. `Path` must be `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\castle-cs-01b\cimmeria-server.exe`. Otherwise stop and return `blocked`, because Cellblock's server runs from `zone-cellblock` and must never be stopped.
2. Run `Stop-Process -Id 36540 -Confirm:$false`.
3. `Get-NetTCPConnection -State Listen -LocalPort 18081,18443,8452,30100 -ErrorAction SilentlyContinue` and `Get-NetUDPEndpoint -LocalPort 32932 -ErrorAction SilentlyContinue` must both be empty.
4. Never run `reload-db.ps1` or `live-db-test.ps1` here, and never drop `sgw_castle_cs_01b`. It holds `Cellutb` and the end-UAT state.

Report: `...\castle-cs-01b\.claude\reports\castle\CS-01d-coder.md`. Return `CS-01d done - <report>`.

## Wave R2a: seals (docs only, five in parallel)

All five are based on `FEATURE_SHA=da2b82c70`. None compiles or touches seeds. The only lane use allowed is the `nav_inspect` and `castle_navmesh` runs named below.

### CS-00R2: G-567, G-706 and the 1376 lineage

**Agent:** game-archaeology-specialist. **Branch / worktree:** `castle/cs-00r2-lineage` / `castle-cs-00r2`.

**Allowlist:**
- `docs/analysis/castle-continuation/evidence/g-567-706-1376.md` (new);
- the status cell of the rows at `docs/analysis/castle-continuation/evidence/reconciliation.md:29,72,94,101` only.

**Inputs** (branch): `evidence/reconciliation.md:29,72,94,101`, `evidence/castle-contract.csv` row 4, `work-packets.md:57` and `:146-157`. Check `docs/` and `.claude/agent-memory/` before using Ghidra.

1. **G-567 (Romney's Files, Castle leg).** Mission 567 has step 2000, objective 2338, tasks 2933/2934/5696, item 2698 and file-cabinet moniker 7547.
   - Prove who grants item 2698 and what moves step 2000 to 2012.
   - Harset keeps 4039; 6503 and 6504 stay disabled until the grant is proved.
   - Verdict: either `SEALED`, with exact frozen rows for chains taken from 1394-1397 (seed shape above), or `BlockedEvidence`, naming the missing proof.
2. **G-706.** Confirm or refute the hypothesis at `reconciliation.md:29`: the exporter's title join drops 706 because its Castle title is blank. Recommend keeping or dropping the integrated 706 reconstruction.
3. **1376 lineage.** The moniker is `DN_ms_B00_Castle_opc_02EscortZuritskaRestart`; only step 4098 is seeded, and it is disabled. Decide whether it is a restart variant of mission 704 that the client offers after an escort failure.
   - Verdict: `hold` (no chain), or a sealed restart graph for a later packet.
   - This packet writes no seeds.

**Tests:** none (docs). **Check:** `git diff --check`. **Commit:** `docs(castle): seal G-567, G-706 and 1376 lineage (CS-00R2)`. **Report:** `...\castle-cs-00r2\.claude\reports\castle\CS-00R2-archaeology.md`.

### CS-00Be: CS-00B evidence half

**Agent:** mission-systems-advisor. **Branch / worktree:** `castle/cs-00be-catalog` / `castle-cs-00be`.

**Allowlist:**
- `docs/analysis/castle-continuation/evidence/mission-catalog.md` (new);
- `docs/analysis/castle-continuation/uat/end-uat.md` (new; this packet creates it).

1. **The #1734 API.** Record how an authored mission enters the client catalog (`MISSION_ADDITIONS`, `MissionOverride`), with file:line. Note that the production `MISSION_ADDITIONS` is empty.
2. **The Castle authored ID reservation (10000-10999).** Propose sub-blocks for missions, steps, objectives, dialogs and chains, and cite the matching block in `docs/analysis/four-zone-planning/id-allocation.md`. Do not edit that file; CS-07 writes it.
3. **The collision audit.** Every reserved block must be absent from `db/resources` on all four refs (contract excerpt) and from the pak exports. Paste the commands and their counts.
4. **`uat/end-uat.md`.** Create it with a header, a "How to run" note (the CS-01b stack recipe, `work-packets.md` § The Castle server instance) and two sections:
   - "CS-00B client proof": journal, dialog, advance and relog of the first authored quest, UATPending;
   - "CS-01 walk": point to `uat/CS-01-walk.md`. The R1 walk reached setup, the GM 688 grant and the Castle landing; walking was blocked by lab input.

**Tests:** none (docs). **Check:** `git diff --check`. **Commit:** `docs(castle): CS-00B evidence half - catalog API, ID reservation, collision audit`. **Report:** `...\castle-cs-00be\.claude\reports\castle\CS-00Be-mission.md`.

### CS-02S: mission 1520 frozen-row seal

**Agent:** mission-systems-advisor. **Branch / worktree:** `castle/cs-02s-1520-seal` / `castle-cs-02s`. **DB:** `sgw_castle_cs_02s` (for read-only psql only).

**Allowlist:**
- `docs/analysis/castle-continuation/worknotes/CS-02S.md` (new);
- an appended `## R2 seal` section in `docs/analysis/castle-continuation/evidence/g-med.md`.

**Inputs** (branch):
- `evidence/g-med.md` (E1-E10 and the candidates);
- `work-packets.md:81-125` (the entry seal) and `:159-213` (the graph candidates);
- the seeds: `castle_701_chains.sql:245-278` and `:371-414`, `spawnlist.sql:573-617`, `entity_templates.sql:555` and `:953-955`, `mission_steps.sql:2052-2064`, `mission_objectives.sql:2126-2140`;
- `crates/entity/tests/castle_navmesh.rs` and `live_db_castle_population.rs`.

Answer each point with its provenance label and a file:line:

1. **Dispenser template 412** (a clone of 147, `being`, `GLB_Components.WorldObject_Small`, `Castle_MedDispenser`, name_id 8896). Give the exact INSERT and the collision-audit output.
2. **Spawn 13801.**
   - The coordinate: world 8, region 2051 `Castle.Infirmary`, component 250, `y` at the floor, clear of the population rules. Label: CIMMERIA_AUTHORING unless a client source pins it.
   - Prove it with a `nav_inspect` probe (exit 0) and a path from Ogilvie `(362.0, 70.18, 886.0)` to the spot.
   - Spawn 13801 is not in the `INSERT` column list until this seal fixes it.
3. **Interaction scope.** Does `set_interaction_type` change the entity for every observer, or for one player? Cite the executor.
   - If the change is global, the design must still be correct with two players at different steps. For example: a static `interaction_type` on template 412, plus chain conditions on step 4453, with no per-player toggling or bit-30 clearing.
   - Record the client-side highlight (bit 30) as end-UAT only.
4. **Ogilvie at 4452** (spawn 197, template 179, `interaction_type 0`). Seal how the player starts the talk: a dialog set or an `interact_tag` trigger. Give the 4983 open and close chains, the hidden credit 5172 and the advance from 4452 to 4453.
5. **The game.** Chain 1371 is `start_minigame Livewire {"on_victory_chains":[1372]}`, gated on step 4453 active. Seal the difficulty params.
6. **Victory chain 1372.** It has no trigger. Conditions: world 8, `mission_status 1520 active` and `step_status 1520 '4453' active`. Every action has `delay_ms 0`. It credits 5173, advances to 4454 and displays 4984.
7. **The 4984 close.** It displays 4985 at once. This is the RECONSTRUCTION of the G-MED radio call, under O14-03.
8. **The 4985 close (chain 1373).** It credits 5174 and completes 1520. Cite whether advancing past the last step completes the mission or whether an explicit completion action is needed.
9. **Relog.** Say what a relog at 4453 or 4454 needs (`player_loaded` chains from 1378-1385), or write "none" and give the reason.
10. **Entry chains 1367 and 1368.** Give the exact rows from the entry seal: world 8, `Castle.Infirmary`, archetypes 1-4 and 6 only, 1520 not active, 704 completed. 1368 credits 5171 while 4452 is active.
11. **The population guard.** Do `live_db_castle_population.rs` and its D-CA06 comment ("nothing here is chain-spawned or tagged") reject a tagged, chain-referenced dispenser? If they do, name the minimal edit, which CS-02c1 makes.

**Output** (`worknotes/CS-02S.md`):
- **Frozen rows:** exact SQL for every chain, template 412 and spawn 13801, plus the `\ir` line placed after `db/database.sql:397`.
- **Replay test list:** event, preconditions, expected actions. Mark each row either for CS-02c1 (entry) or for CS-02c2 (graph).
- **End UAT rows.**
- **The verdict line:** `SEALED`, `BlockedEvidence (<gate>)` or `BlockedDecision (<question>)`.

**Lane:** `nav_inspect` only (contract excerpt). **Commit:** `docs(castle): seal mission 1520 frozen rows (CS-02S)`. **Report:** `...\castle-cs-02s\.claude\reports\castle\CS-02S-mission.md`.

### CS-03S: mission 1521 frozen-row seal (Option B skeleton)

**Agent:** mission-systems-advisor. **Branch / worktree:** `castle/cs-03s-1521-seal` / `castle-cs-03s`. **DB:** `sgw_castle_cs_03s`.

**Allowlist:**
- `docs/analysis/castle-continuation/worknotes/CS-03S.md` (new);
- an appended `## R2 seal` section in `evidence/g-sym.md`.

**Scope.** Seal the mission-state skeleton that both D-CA-SYM options share: no item grant, no debuff and no item consumption. Option A's deltas (`add_item 5169 {"container":2}`, optionally `apply 1926` on accept) are listed separately for CS-03d and are **not** frozen.

1. **The tank template (candidate 413) and spawn (candidate 13802)**, with the collision audit. The precedent is Harset template 245 (`Ga-Props.GA-PuzzleStation00`, flags 4, `entity_templates.sql:953-955`).
2. **Placement and G-ROUTE.** The chamber at `(398.83, 55.04, 929-941)` is on navmesh component 468, which is isolated. Either:
   - (a) place the tank where `nav_inspect` and a path from the Infirmary (component 250) prove it reachable, and label that CIMMERIA_AUTHORING; or
   - (b) keep the chamber spot, mark the route `UATPending (end UAT)` and give the seal `BlockedEvidence (G-ROUTE)`.

   Never treat (b) as playable.
3. **Chains 1369, 1370 and 1374-1377**, plus any extras from 1386-1393:
   - 1369 offers 1521 to archetype 8 only;
   - 1370 advances to 4456 while 4455 is active (never `complete_objective 5175`);
   - 1374 starts Livewire from the tank at 4456;
   - victory chain 1375 has no trigger, is guarded on world 8 and `step_status 1521 '4456' active`, uses `delay_ms 0`, credits 5176, advances to 4457 and displays 4990;
   - 1376 is the second tank use at 4457, which credits 5177 and advances to 4458;
   - 1377 shows the opener, then the 4991 close credits 5178 and completes 1521.

   For the opener, check the UIScreenType of 4991. Opening it immediately on the 4458 advance replaces the radio, which #334 makes client-inert, as a RECONSTRUCTION under O14-03.
4. Seal the relog and population-guard points, the same as CS-02S points 9 and 11.

**Output** (`worknotes/CS-03S.md`) has the same sections as CS-02S's, plus "Option A deltas (not frozen)". **Lane:** `nav_inspect` only. **Commit:** `docs(castle): seal mission 1521 frozen rows (CS-03S)`. **Report:** `...\castle-cs-03s\.claude\reports\castle\CS-03S-mission.md`.

### CS-02L: b2-lite trust seal

**Agents:** server-authority-enforcer, then minigame-systems-advisor, in the same worktree, one after the other. **Branch / worktree:** `castle/cs-02l-b2-lite` / `castle-cs-02l`.

**Allowlist:** `docs/analysis/castle-continuation/worknotes/CS-02L.md` (new; the advisor appends its own section).

**Question.** The branch `work-packets.md:197-271` requires the full CS-02b2 (callback-seam rework) before any conditioned victory chain ships. Does b2-lite (contract excerpt) replace it safely for Castle callbacks 1372 and 1375?

Weigh these points:
- Base already rechecks entity, player and session instance before and after the close (b0/b1, `cell_dispatch/minigame.rs:244-284`).
- Re-entry by the same player can only advance that player's own mission.
- The guarded path evaluates world and step state at fire time.
- `entity_missing` stops firing with player 0.
- No legacy callback chain has conditions.
- The interaction between `delay_ms` and staleness.
- Any spoof path from client data to `on_victory_chains`.

**Verdict:** `accept`, `change` (with the exact contract edits) or `reject` (with the reason). A reject makes CS-MGF1/2 BlockedDecision, and the architect splits the full b2 into b2-prep/b2a/b2b.

**Tests:** none. **Commit:** `docs(castle): b2-lite minigame callback trust seal (CS-02L)`. **Reports:** `...\castle-cs-02l\.claude\reports\castle\CS-02L-authority.md` and `CS-02L-minigame.md`.

## CS-INT2a: integrate R2a

**Agent:** packet-coder, in `zone-castle` (DB `sgw_zone_castle`). **Gate:** the R2a review gate (D-CA6).

1. Cherry-pick the reviewed commits, each with its fix commits, in this order: CS-00R2, CS-00Be, CS-02S, CS-03S, CS-02L.
2. Cherry-pick CS-01b's `009ab0536` (`uat/CS-01-preflight.md`). The lab is paused, so it no longer waits for CS-01c.
3. Copy this overlay file to the branch as `docs/analysis/castle-continuation/r2-packets.md`.
4. Update the branch README rows from the overlay README (R2 findings, D-CA11..D-CA14, the R2 status rows).
5. **Label check** (contract excerpt, label form). `git grep -n -F "(RECONSTRUCTION)'" -- docs/analysis/castle-continuation/worknotes/CS-02S.md docs/analysis/castle-continuation/worknotes/CS-03S.md` must print nothing, because the c1 packets copy those rows verbatim. The trailing quote limits it to SQL descriptions; CS-02S's section headings such as `### 6. Victory chain 1372 (RECONSTRUCTION)` are prose and stay. If it prints a row, stop and return `blocked`. Both seals' round-2 label commits (`cd4ba0d72`, `96ab77cf7`) already use `CIMMERIA_RECONSTRUCTION`, so this is a check, not an edit.
6. **Final CRLF.** `evidence/g-med.md` and `evidence/g-sym.md` end without a line break after their `## R2 seal` text. Append one CRLF to each.
7. Make sure every file is CRLF and that `git diff --check` is clean.

**Commit:** `docs(castle): integrate R2a seals`. **Report:** `...\zone-castle\.claude\reports\castle\CS-INT2a-coder.md`.

The new tip becomes `FEATURE_SHA` for R2b; RESUME.md records it. A seal with a verdict other than `SEALED` (or `accept` for CS-02L) blocks only its dependants (see the gates below).

## Wave R2b (CS-MGF1 and CS-02c1 in parallel on the CS-INT2a SHA; CS-03c1 stacked on CS-02c1)

### CS-MGF1: carry `player_id` on the minigame result; add the accessor

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-mgf1-player-id` / `castle-cs-mgf1`. **Gate:** CS-02L `accept` (or `change`, with its edits applied here). CS-02L returned `change`; its edits are applied in this file (2026-10-17). The verdict stays conditional on the D-CA12 reading in RESUME.md.

**Allowlist:**
- `crates/wire/src/cell/messages/base_to_cell.rs`;
- `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`;
- `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame_result_tests.rs`;
- `crates/cell/src/cell/service/base_messages/mod.rs`;
- `crates/cell/src/cell/service/base_messages/minigame.rs`;
- `crates/cell/src/cell/service/base_messages/tests/minigame.rs`;
- `crates/content-engine/src/chain/step_anchors.rs` (the accessor; 138 lines);
- `crates/content-engine/src/chain/step_anchors_tests.rs` (the unit tests; 151 lines, already wired at `chain/mod.rs:24-25`).

`crates/content-engine/src/chain/mod.rs` (537 lines) is **not** in the allowlist and must not grow.

**Changes.** This packet is behavior-neutral in the Cell:

1. Add the `player_id: i32` field to the message (contract excerpt).
2. Base sends `player_id,` at `:291`.
3. In `base_messages/mod.rs:434-448`, destructure `player_id` and pass it as the second argument. The handler signature becomes `(entity_id: u32, player_id: i32, result_code: u8, on_victory_chains: Vec<i64>, tx, space_mgr, engine)`.
4. The handler only logs it as `receipt_player_id = player_id` in the "Minigame result" row. It changes no firing logic.
5. Add `player_id: 100` to the four constructors in `tests/minigame.rs` (lines 52, 142, 171 and 225).
6. Add `ChainEngine::chain_has_conditions` exactly as the contract excerpt gives it, inside the existing `impl ChainEngine` block of `chain/step_anchors.rs` (`:38`). Leave `chain/mod.rs` unchanged.

**Tests:**
- Unit, base-world-entry: in `minigame_result_forwards_receipt_to_cell_once` (`minigame_result_tests.rs:113`), add `player_id: 1` to the `matches!` pattern. The fixture registers player 1 at `:66-67`. It fails if Base forwards anything else.
- Unit, content-engine, in `chain/step_anchors_tests.rs`, each test name containing `chain_has_conditions`: three cases.
  - `chain_has_conditions` is true for a registered chain with one condition.
  - It is false for a chain with none.
  - It is false for an unknown id.

  It fails if the accessor ignores `conditions`.

**Lane, in order:**
1. `cargo fmt --all -- --check`;
2. `lane.ps1 cargo clippy -p cimmeria-wire -p cimmeria-content-engine -p cimmeria-base-world-entry -p cimmeria-cell -p cimmeria-base -- -D warnings`;
3. `lane.ps1 cargo nextest run -p cimmeria-content-engine -p cimmeria-base-world-entry -p cimmeria-cell -p cimmeria-base -E 'test(minigame) | test(chain_has_conditions)'`.

**Commit:** `feat(minigame): carry the receipt player id to the Cell and expose chain_has_conditions (CS-MGF1)`. **Report:** `...\castle-cs-mgf1\.claude\reports\castle\CS-MGF1-coder.md`.

### CS-02c1: mission 1520 seeds and entry tests

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-02c1-1520-seeds` / `castle-cs-02c1`. **DB:** `sgw_castle_cs_02c1`. **Gate:** the CS-02S verdict is `SEALED`.

**Allowlist:**
- `db/resources/Content/Seed/castle_1520_chains.sql` (new);
- `db/database.sql`, one `\ir resources/Content/Seed/castle_1520_chains.sql` line directly after `:397` (contract excerpt, shared append points);
- `db/resources/Entities/Seed/entity_templates.sql` (template 412, at the end of the file);
- `db/resources/Worlds/Seed/spawnlist.sql` (spawn 13801, after spawn 199);
- `crates/cell-catalog/src/cell/spawner/tests/live_db_castle_population.rs`: the population-guard edit CS-02S point 11 names, `"Castle_MedDispenser"` added to `MISSION_ACTORS` (`:72`, length 9 to 10). Nothing else in that file (517 lines);
- `crates/cell-content/src/cell/content/chain_replay_tests/mission_1520_entry.rs` (new);
- `crates/cell-content/src/cell/content/chain_replay_tests/mission_1520_seed_guard.rs` (new; the seed guard);
- `chain_replay_tests/mod.rs`, two `mod` lines;
- `crates/content-engine/tests/it/interact_tag_linter.rs`, fallback only (contract excerpt, interact-tag linter): needed only if a frozen row is reformatted multi-line. Allowlist reason for `castle_1520_chains.sql` chain 1371: its glow is the per-player bind (`add_dialog_set 5828`, slot 412), not a `set_interaction_type`. The file is over the hard cap, so if it would need the entry, return `blocked` instead.

**Steps:**
1. Copy the frozen SQL from branch `worknotes/CS-02S.md` **verbatim**, one row per line, placed at the shared append points (contract excerpt). If any row is ambiguous, or any description does not end in a label from the contract excerpt (`(CIMMERIA_RECONSTRUCTION)` for reconstructions), return `blocked`.
2. Write the entry replay tests that CS-02S marks "CS-02c1". The menu requires them to cover:
   - the real region dispatcher (`enter_region Castle.Infirmary`);
   - replay of the offer on re-entry;
   - duplicate hints;
   - the wrong world, a missing 704 prerequisite, and archetypes 5, 7 and 8;
   - relog.

   Each test asserts the exact actions emitted. Each must fail if the matching seed row is removed.
3. Write `mission_1520_seed_guard.rs`: the seed guard (a)-(e) from the contract excerpt for starter 1371 and victory chain 1372 (mission 1520, step 4453), using `require_db_or_skip!()`. It must fail if a condition row of 1372 is removed, if any 1372 action gets `delay_ms` above 0, if 1372 gains a trigger row, or if 1371's game name, difficulty or `on_victory_chains` changes.

**Lane, in order:**
1. `cargo fmt --all -- --check`;
2. `lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-cell-catalog -- -D warnings`;
3. `live-db-test.ps1 mission_1520_entry -p cimmeria-cell-content`;
4. `live-db-test.ps1 mission_1520_seed_guard -p cimmeria-cell-content`;
5. `live-db-test.ps1 live_db_castle_population -p cimmeria-cell-catalog`;
6. `lane.ps1 cargo nextest run -p cimmeria-entity --test castle_navmesh`;
7. `lane.ps1 cargo nextest run -p cimmeria-content-engine --test it -E 'test(interact_tag)'`.

**Commit:** `feat(castle): seed mission 1520 Infirmary chains (CS-02c1)`. **Report:** `...\castle-cs-02c1\.claude\reports\castle\CS-02c1-coder.md`.

### CS-03c1: mission 1521 seeds and entry tests (Option B)

This packet is the same shape as CS-02c1, with these changes:
- **Agent:** packet-coder. **Branch / worktree:** `castle/cs-03c1-1521-seeds` / `castle-cs-03c1`. **DB:** `sgw_castle_cs_03c1`.
- **Gate:** the CS-03S verdict is `SEALED`. A G-ROUTE `BlockedEvidence` keeps this packet BlockedEvidence.
- **Base: stacked on CS-02c1** (contract excerpt, shared append points). Dispatch after CS-02c1 returns `done`; `<BASE>` is the CS-02c1 commit. If the R2b review changes CS-02c1, the CS-03c1 fix pass first rebases onto the fixed CS-02c1 tip.
- **Allowlist:**
  - `db/resources/Content/Seed/castle_symbiote_chains.sql` (new; the file name CS-03S freezes);
  - `db/database.sql`, one `\ir resources/Content/Seed/castle_symbiote_chains.sql` line directly after the CS-02c1 line;
  - `db/resources/Entities/Seed/entity_templates.sql` (template 413, directly after 412);
  - `db/resources/Worlds/Seed/spawnlist.sql` (spawn 13802, in the world-8 block where CS-03S places it);
  - `crates/cell-catalog/src/cell/spawner/tests/live_db_castle_population.rs`: `"Castle_SymbioteTank"` added to `MISSION_ACTORS` after `"Castle_MedDispenser"` (length 10 to 11). Nothing else in that file;
  - `chain_replay_tests/mission_1521_entry.rs` (new);
  - `chain_replay_tests/mission_1521_seed_guard.rs` (new; the seed guard);
  - `chain_replay_tests/mod.rs`, two `mod` lines;
  - `crates/content-engine/tests/it/interact_tag_linter.rs`, fallback only, as in CS-02c1. Allowlist reason for `castle_symbiote_chains.sql` chains 1374 and 1376: a static `interaction_type` 256 on template 413, not a `set_interaction_type`. The file is over the hard cap, so if it would need the entry, return `blocked` instead.
- **Steps:** copy the frozen SQL from branch `worknotes/CS-03S.md` verbatim, one row per line, with the same label check as CS-02c1. Re-check, against the CS-02c1 spawnlist row, that the tank (356.0, 70.18, 896.0) is at least 4.0 u from the dispenser 13801 (the gap is exactly 4.0 u; CS-03S lines 41 and 117). If it is less, return `blocked`.
- **Entry tests:** archetype 8 only, step 4455 to 4456, and never `complete_objective 5175`.
- **Seed guard:** `mission_1521_seed_guard.rs` asserts (a)-(e) from the contract excerpt for starter 1374 and victory chain 1375 (mission 1521, step 4456), with the same revert conditions as CS-02c1.
- **Checks:** `mission_1521_entry` and `mission_1521_seed_guard`, plus the same population, navmesh and interact-tag linter runs as CS-02c1.
- **Commit:** `feat(castle): seed mission 1521 skeleton chains (CS-03c1)`. **Report:** `...\castle-cs-03c1\.claude\reports\castle\CS-03c1-coder.md`.

## Wave R2c (four; each stacked on its predecessor's reviewed tip)

Dispatch after the R2b review gate. A stacked packet's step 0 uses the predecessor's reviewed tip as `<BASE>`.

### CS-MGF2: the player-bound victory guard (base = CS-MGF1 tip)

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-mgf2-guard` / `castle-cs-mgf2`.

**Allowlist:**
- `crates/cell/src/cell/service/base_messages/minigame.rs`;
- `crates/cell/src/cell/service/base_messages/tests/minigame.rs` (253 lines at `FEATURE_SHA`; stay under 500);
- `crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs` (73 lines; the INFO row only).

**Change.** Replace the victory body with the b2-lite branch list from the contract excerpt. The player check runs once, before the chain loop:
- `space_mgr.get_entity(entity_id)`. If it is `None`, WARN `reason="entity_missing"` with `entity_id`, `expected_player_id = player_id` and `chain_ids`, and the message "Minigame victory for an entity the cell does not hold; no chain fires", then return.
- If `!entity.is_player || player_id == 0 || entity.player_id != Some(player_id)`, WARN `reason="player_mismatch"` with `entity_id`, `expected_player_id = player_id`, `actual_player_id = entity.player_id.unwrap_or(0)` and `chain_ids`, then return. Use exactly these field names.
- Then route each chain through `engine.chain_has_conditions(*chain_id)`, picking `fire_guarded_minigame_chain_by_id` or `fire_chain_by_id`.
- In `direct_invoke.rs`, before the `return` at `:62-64` (`resolved.actions.is_empty()`), add an INFO row `reason="guarded_callback_not_matched"` with `chain_id` and `entity_id`. Change nothing else in that file.

**Tests** (unit, `tests/minigame.rs`):
1. Update `minigame_victory_for_unknown_entity_warns` (`:162-197`): the new WARN text, and no onStatUpdate emitted (today it fires).
2. Add `minigame_victory_for_another_player_fires_nothing`: the entity has `player_id Some(100)` and the message carries 200; assert no chain output and the WARN.
3. Add `minigame_victory_with_player_zero_fires_nothing`: the entity has `player_id Some(100)` and the message carries 0; assert no chain output.
4. Add `minigame_victory_conditioned_chain_is_guarded`. Register a chain with a `world` condition the entity's space fails, and assert nothing fires. Then assert it fires when the condition holds.
5. Add `minigame_victory_step_gated_chain_grants_once`. Register a chain with a `step_status <m> <step> eq active` condition. With the step not active, nothing fires. With it active, the chain fires once. Then set the held entity's step to `completed` with the same setter the test used to make it active, send the identical message again, and assert nothing fires (single grant).
6. Keep `minigame_result_victory_fires_on_victory_chains` (an unconditioned legacy chain still fires for the matching player: the legacy-parity guard).

The Base side (`receipt.player_id` pinned in `minigame_result_tests.rs`) is CS-MGF1's test. Each test fails if its branch is reverted.

**Lane:** `fmt`, then `lane.ps1 cargo clippy -p cimmeria-cell -p cimmeria-cell-content -- -D warnings`, then `lane.ps1 cargo nextest run -p cimmeria-cell -p cimmeria-cell-content -E 'test(minigame)'`.

**Commit:** `fix(minigame): bind victory callbacks to the receipt's player and guard conditioned chains (CS-MGF2)`. **Report:** `...\castle-cs-mgf2\.claude\reports\castle\CS-MGF2-coder.md`.

### CS-02c2: mission 1520 graph replay tests (base = CS-02c1 tip)

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-02c2-1520-graph` / `castle-cs-02c2`. **DB:** `sgw_castle_cs_02c2`.

**Allowlist:**
- `chain_replay_tests/mission_1520.rs` (new);
- `chain_replay_tests/mod.rs`, one line;
- `crates/entity/tests/castle_navmesh.rs` (a constant and one routable test; stay under 500).

**Tests** (live-DB replay), using CS-02S's "CS-02c2" list:
- Ogilvie talk, 4983 close, then 4452 to 4453.
- The dispenser at 4453 starts Livewire carrying `[1372]`, and the dispenser at any other step does nothing.
- 1372 through `fire_guarded_minigame_chain_by_id` at 4453 advances and shows 4984. The same call at 4454 or with 1520 completed emits nothing.
- The 4984 close shows 4985; the 4985 close completes 1520.
- The relog rows.

The navmesh test checks that Ogilvie to dispenser is routable.

**Seed guard** (CS-02L authority edit 3). The 1372 cases above (at 4454, or with 1520 completed, the guarded call emits nothing) are the behavioural half of the guard; each must fail if 1372's `step_status` row is removed. The static half is CS-02c1's `mission_1520_seed_guard.rs`, which the lane filter below re-runs.

**Lane:** `fmt`, `clippy -p cimmeria-cell-content -p cimmeria-entity`, `live-db-test.ps1 mission_1520 -p cimmeria-cell-content` (it matches `mission_1520`, `mission_1520_entry` and `mission_1520_seed_guard`), `lane.ps1 cargo nextest run -p cimmeria-entity --test castle_navmesh`.

**Commit:** `test(castle): replay the mission 1520 Infirmary graph (CS-02c2)`. **Report:** `...\castle-cs-02c2\.claude\reports\castle\CS-02c2-coder.md`.

### CS-03c2: mission 1521 graph replay tests (base = CS-03c1 tip)

This packet is CS-02c2's shape for 1521 (`mission_1521.rs`, tank routable from the Infirmary), using CS-03S's list. Its seed-guard half is the same: the 1375 cases at a step other than 4456 emit nothing, and the lane filter `mission_1521` re-runs CS-03c1's `mission_1521_seed_guard`. **Branch / worktree:** `castle/cs-03c2-1521-graph` / `castle-cs-03c2`. **DB:** `sgw_castle_cs_03c2`. **Commit:** `test(castle): replay the mission 1521 skeleton graph (CS-03c2)`. **Report:** `...\castle-cs-03c2\.claude\reports\castle\CS-03c2-coder.md`.

### CS-DOC2: mission-chains rows and end-UAT rows (base = CS-INT2a SHA)

**Agent:** documentation-writer. **Branch / worktree:** `castle/cs-doc2-1520-1521-docs` / `castle-cs-doc2`.

**Allowlist:**
- `docs/content/mission-chains.md` (the 1520 and 1521 rows only);
- `docs/analysis/castle-continuation/uat/end-uat.md` (append the "1520" and "1521" sections from the seals' "End UAT rows").

Mark what is tested (replay and navmesh) versus never tested (the client highlight, the Livewire UI, journal text). Facts come only from `worknotes/CS-02S.md` and `CS-03S.md`.

**Commit:** `docs(castle): mission 1520/1521 chain and end-UAT rows (CS-DOC2)`. **Report:** `...\castle-cs-doc2\.claude\reports\castle\CS-DOC2-docs.md`.

## CS-INT2: integrate R2b and R2c

**Agent:** packet-coder, in `zone-castle`. **Gate:** the R2c review gate.

Victory chains 1372 and 1375 carry conditions. Today's unguarded `fire_chain_by_id` would ignore them, so the seed packets never integrate without CS-MGF2. If CS-02L rejects, CS-02c1/03c1/02c2/03c2 wait for the full b2 instead.

1. Cherry-pick, in order: CS-MGF1, CS-MGF2, CS-02c1, CS-03c1, CS-02c2, CS-03c2, CS-DOC2, each with its fix commits.
2. **Conflict rule:** a conflict whose two sides are both pure additions (`db/database.sql` include lines, adjacent template, spawn or `mod` rows) keeps both, in ID order. Any other conflict means `blocked`.
3. Run, in order:
   - `pwsh tools/build-lane/reload-db.ps1`;
   - `live-db-test.ps1 'mission_1520 | mission_1521' -p cimmeria-cell-content`;
   - `live-db-test.ps1 live_db_castle_population -p cimmeria-cell-catalog`;
   - `lane.ps1 cargo nextest run -p cimmeria-cell -p cimmeria-base-world-entry -p cimmeria-content-engine -p cimmeria-entity`.
4. Update the branch README rows with each SHA.

**Commit:** `chore(castle): integrate R2 missions 1520/1521 and b2-lite`. **Report:** `...\zone-castle\.claude\reports\castle\CS-INT2-coder.md`.

## Held (not dispatched in R2)

| Packet | Status | Unblocks when |
|---|---|---|
| CS-03d (1521 Option A: item 5169, optional debuff 1926) | BlockedDecision (D-CA-SYM) | The owner picks A; the architect then freezes the Option A deltas from CS-03S. Option B needs no packet. |
| CS-08b (567 Castle leg) | BlockedEvidence (G-567) | CS-00R2 returns `SEALED`; seeds use 1394-1397. |
| CS-08a2 | BlockedDependency | Harset HA-1360a/b is integrated on `content/harset-campaign` and its interface is recorded. |
| CS-06 (1664 exit grant) | BlockedDependency | G-1664, HA-D05 sealed and Kaerlia placement; IDs 1398-1400. |
| CS-01c, CS-00B client proof, all client-only rows | UATPending (D-CA11) | The user runs `uat/end-uat.md` at the end. |
| CS-04a/b/c, CS-05, CS-07 | BlockedDependency | Unchanged (README). |

## Dispatch briefs (R2)

Every brief starts with: "Read `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\castle-continuation\r2-packets.md` § <section> (after CS-INT2a, the branch copy `r2-packets.md`). Follow its repo rules and the step 0 base check. `<BASE>=<sha>`. No lab."

In this table, `...` is `C:\Users\Steve\source\projects\Cimmeria`, `mk` is `pwsh tools/build-lane/mk-worktree.ps1`, and every worktree is under `...\.claude\worktrees\`.

| Wave | Packet | ops call | Agent | Worktree | BASE |
|---|---|---|---|---|---|
| R2-0 | CS-01d | none | packet-coder | `castle-cs-01b` | n/a |
| R2a | CS-00R2 | `mk castle/cs-00r2-lineage castle-cs-00r2` | game-archaeology-specialist | `castle-cs-00r2` | `da2b82c70` |
| R2a | CS-00Be | `mk castle/cs-00be-catalog castle-cs-00be` | mission-systems-advisor | `castle-cs-00be` | `da2b82c70` |
| R2a | CS-02S | `mk castle/cs-02s-1520-seal castle-cs-02s` | mission-systems-advisor | `castle-cs-02s` | `da2b82c70` |
| R2a | CS-03S | `mk castle/cs-03s-1521-seal castle-cs-03s` | mission-systems-advisor | `castle-cs-03s` | `da2b82c70` |
| R2a | CS-02L | `mk castle/cs-02l-b2-lite castle-cs-02l` | server-authority-enforcer, then minigame-systems-advisor | `castle-cs-02l` | `da2b82c70` |
| gate | CS-INT2a | none | packet-coder | `zone-castle` | branch tip |
| R2b | CS-MGF1 | `mk castle/cs-mgf1-player-id castle-cs-mgf1` | packet-coder | `castle-cs-mgf1` | CS-INT2a |
| R2b | CS-02c1 | `mk castle/cs-02c1-1520-seeds castle-cs-02c1` | packet-coder | `castle-cs-02c1` | CS-INT2a |
| R2b | CS-03c1 | `mk castle/cs-03c1-1521-seeds castle-cs-03c1` | packet-coder | `castle-cs-03c1` | CS-02c1 commit (after CS-02c1 returns) |
| R2c | CS-MGF2 | `mk castle/cs-mgf2-guard castle-cs-mgf2` | packet-coder | `castle-cs-mgf2` | CS-MGF1 reviewed tip |
| R2c | CS-02c2 | `mk castle/cs-02c2-1520-graph castle-cs-02c2` | packet-coder | `castle-cs-02c2` | CS-02c1 reviewed tip |
| R2c | CS-03c2 | `mk castle/cs-03c2-1521-graph castle-cs-03c2` | packet-coder | `castle-cs-03c2` | CS-03c1 reviewed tip |
| R2c | CS-DOC2 | `mk castle/cs-doc2-1520-1521-docs castle-cs-doc2` | documentation-writer | `castle-cs-doc2` | CS-INT2a |
| gate | CS-INT2 | none | packet-coder | `zone-castle` | branch tip |

The report path is `<worktree>\.claude\reports\castle\<packet>-<role>.md`. The return is `<packet> <done|blocked|failed> <sha|-> <report>`. CS-02L returns once per agent.

**Review groups (D-CA6):**
- R2a: five docs packets, one reviewer.
- R2b: three packets.
- R2c: four packets.

Copy each report worth keeping to the branch `worknotes/` before the owner retires the worktree.
