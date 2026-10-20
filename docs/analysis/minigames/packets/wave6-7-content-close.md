# Waves 6-7: content, UAT and close-out

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> Content packets read [content-map.md](../content-map.md) (MG-R5) and [spec-launch-flow.md](../spec-launch-flow.md) (MG-R7). Every input below was confirmed against them at G1 (D-MG22); a coder still re-reads the named seed line before editing it.

## Content rules (inline for every wave-6 packet)

- PowerShell only, with `Set-Location <wt>;` on every command.
- Cargo runs only through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`). Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, in the foreground.
- **Seeds are the source of truth.** Edit `db/resources/` only; never add `db/scripts/*.sql` migrations.
- A new seed file gets its `\ir resources/Content/Seed/<file>.sql` line in `db/database.sql`, after the last `Content/Seed` line.
- Chain ids come from the packet's C7 block only. Grep the range in `db/resources/` first; if any id is taken, stop and return `blocked`.
- The chain row shape follows the existing pairs: the `start_minigame` rows in `db/resources/Content/Seed/castle_cellblock_chains.sql` (line 198: `(1024, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1025]}', 0, 0)`) and the `interact_tag` and step-condition rows in `db/resources/Content/Seed/beta_e1_m1147_chains.sql` (chains 23391 and 23393). Read both before writing.
- **Tests:** type 6 chain-replay, in `crates/cell-content/src/cell/content/chain_replay_tests/`, following `livewire_pairs.rs`. That means four assertions: the launcher resolves exactly one `StartMinigame`, it resolves nothing at the wrong step, `execute_actions` emits one `CellToBaseMsg::StartMinigame` with the authored fields, and the victory chain's action list matches the seed. Register the module in `chain_replay_tests/mod.rs`.
- Files: soft cap 500 lines, hard cap 700. Docs are CRLF. No `git stash`, prune or push; make one local commit with the brief's attribution. No lab.
- **Lanes:**

  ```powershell
  Set-Location <wt>; cargo fmt -p cimmeria-cell-content
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
  Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 <the packet's test module name>
  ```

- **Docs.** Add a row to [mission-chains.md](../../../content/mission-chains.md) for each mission touched, and update [zone-audit.md](../../../content/zone-audit.md) if a zone row changes. Doc-update-map row "Seeds / content chains".

Wave 6 ships like every other wave: `ops` merges each packet branch into `minigames/integration` with `merge-noff.ps1`, and the campaign ships one PR at the end. Merge order: MG-62, MG-61, MG-64. MG-62 goes after MG-63b (wave 2) because both append to the same hand-authored `effect_nvps` block.

## MG-61: mission 582 step 2307, Converse with Marsh

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-CH3 merged into `minigames/integration` · Status BlockedDependency
- Worktree `mg-61` · Branch `minigames/mg-61-m582-marsh` · Commit `MG-61: mission 582 step 2307 plays the Marsh conversation minigame`
- **Ids:** chains 24000-24001 (C7 reserves 24000-24009; the rest stay unused).
- **Inputs (content-map.md §6 and §10, confirmed at G1):** Marsh is spawn 344 in world 68, interact tag `CmdCenter_Marsh`. Dialog 2433 is the step's prompt. The game is `ConverseBasicHumanoid` at difficulty 1. Step 2307's objective is 2675 and the next step is 3274. Marsh's existing chains 6501 and 6527 fire on the same tag while missions 1360 (step 4038) and 1361 (step 4694) are active; the launcher must not steal those clicks.
- **Files:**
  1. `db/resources/Missions/Seed/mission_steps.sql:5335`: row 2307 `step_enabled` goes from `false` to `true`. Change nothing else on the row.
  2. New `db/resources/Content/Seed/minigames_m582_chains.sql` (header comment naming D-MG22 and MG-61):
     - launcher **24000**: trigger `interact_tag 'CmdCenter_Marsh'`; conditions `world 68` (Grep `'world',` under `db/resources/Content/Seed/` for the row shape), `mission_status 582 eq active`, `step_status 582 '2307' eq active`, `step_status 1360 '4038' neq active`, `step_status 1361 '4694' neq active`; actions in order `display_dialog 2433`, then `start_minigame` with target key `'ConverseBasicHumanoid'` and params `{"difficulty": 1, "on_victory_chains": [24001]}`.
     - victory chain **24001** (no trigger row): `complete_objective 582 '2675'`, then `advance_step 582 '3274'`. Grep both action names in `db/resources/Content/Seed/` and copy the column use of an existing row; if either action name differs, use the seeded spelling and say so in the report.
     - No failure chain: a loss leaves step 2307 active, and the player clicks Marsh again. The session's own `failure` result is the feedback.
  3. The `\ir resources/Content/Seed/minigames_m582_chains.sql` line in `db/database.sql`.
- **Not in scope.** Mission 582 cannot be offered today: its giver Blackstock (NPC 948) has no spawn and set 530 is unbound (M10, content-map §10). Do not add an offer path; the test sets the mission and step active directly. Dialog 2431 (`accepts_mission_id = 582`, `dialogs.sql:4345`) stays as is.
- **Test:** new `chain_replay_tests/minigames_m582.rs` with the four assertions, plus: the launcher resolves nothing while step 4038 of mission 1360 is active (the guard), and the seeded step 2307 row has `step_enabled = true`. On revert of the seed edit the step-enabled assertion fails; on revert of the chain file the launcher resolves nothing. Lane filter `minigames_m582`.
- **Docs:** [mission-chains.md](../../../content/mission-chains.md), a mission 582 row naming the offer gap (M10).

## MG-62: Converse abilities 1327 and 2089

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-24, MG-63b and MG-CH3 merged into `minigames/integration` · Status BlockedDependency
- Worktree `mg-62` · Branch `minigames/mg-62-converse-abilities` · Commit `MG-62: Converse abilities launch the conversation minigame`
- **Ids:** NVPs 25501-25504 in the hand-authored block MG-63b opened (25500-25509). No chains: spec-launch-flow names no victory or failure outcome for either ability, so the session result is the only outcome.
- **Inputs (spec-launch-flow.md §6, D-MG20):** effect 1643 (ability 1327) and effect 2813 (ability 2089). The `StartMinigame` script (MG-24) reads `minigame_type` and `difficulty` from the effect's NVPs; the caster plays.
- **Files:**
  1. `db/resources/Effects/Seed/effects.sql`: `script_name` (the last column, NULL today) becomes `'StartMinigame'` on effect 1643 (line 2915) and effect 2813 (line 4144). Re-read each line first; change only that column.
  2. `db/resources/Effects/Seed/effect_nvps.sql`, in the `-- minigames hand-authored (25500-25509)` block, after MG-63b's 25500 row:

     ```sql
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25501, 1643, 'minigame_type', 'ConverseBasicHumanoid');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25502, 1643, 'difficulty', '1');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25503, 2813, 'minigame_type', 'ConverseBasicHumanoid');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25504, 2813, 'difficulty', '1');
     ```

     Grep `25501` first; if taken, stop and return `blocked`. If the block header is missing, MG-63b has not merged: return `blocked`.
- **Test (type 3 live-DB):** new `crates/cell-effect-scripts/src/cell/effects/start_minigame_seed_live_db_tests.rs`, copying the shape of MG-63b's `movement_hold_seed_live_db_tests.rs`: effects 1643 and 2813 load with `script_name = "StartMinigame"`, the registry resolves it, and each carries `minigame_type = ConverseBasicHumanoid` and `difficulty = 1`. Register it beside MG-63b's module. On revert the script name is NULL and the first assertion fails.
- **Lanes:** `cargo fmt -p cimmeria-cell-effect-scripts`; `lane.ps1 cargo clippy -p cimmeria-cell-effect-scripts --all-targets -- -D warnings`; `live-db-test.ps1 start_minigame_seed`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch: the two seeded abilities.

## MG-63: Prison Boot 1597 (replaced)

Replaced at G1 by **MG-63a** (movement lock while a minigame or hold is active) and **MG-63b** (the boot's `MovementHold`/`MovementRelease` scripts and seeds), both in wave 2: [wave2-launch.md](wave2-launch.md). Chains 24020-24029 stay unused; chains 1022-1025 already carry the boot.

## MG-64: shell placeholder timer and unknown-name rejection

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-04 (integrated) · Status Ready
- Worktree `mg-64` · Branch `minigames/mg-64-shell-timer` · Commit `MG-64: shell minigames show a countdown and bad game names fail at load`
- **Decision (D-MG3 = A, D-MG22, spec-shells.md §8 and §10).** The shells stay client-declared; no content is wired (content-map finds no seeded objective that names a shell). Two server fixes only: the shells get a timer they can show, and an unknown or wrong-case minigame name is caught at content load, not at launch.
- **Files:**
  1. `crates/minigame/src/minigame/games/placeholder.rs`:
     - `started()` sends `fullgamestate {timerState: 0, timeRemaining: 60}` (replacing `{gameStarted: 'false'}`), then `timerupdate {timerState: 1, timeRemaining: 60}`. Numbers go as `SfsValue::Number`.
     - `needs_tick()` returns true; count ticks (250 ms each). At 240 ticks (60 s), send `failure` and return `GameOutput::Failure`, once.
     - Keep the D-MG13 5 s floor on a client `victory` unchanged.
  2. `crates/content-engine/src/loader/action.rs`, the `"start_minigame"` arm (lines 124-163): a `const MINIGAME_TYPE_NAMES: &[&str]` holding the exact names `cimmeria_minigame` registers (copy them from `crates/minigame/src/minigame/games/kind.rs`; do not add a crate dependency). A target key not in the list (case-sensitive) logs WARN `reason = "unknown_minigame_type"` with the chain id and drops the row, like the existing difficulty check at line 151.
- **Tests:**
  - type 1 in `crates/minigame/src/minigame/games/placeholder_tests.rs`: `started()` emits `timerState` and `timeRemaining` (spec-shells §10 item 4); 239 ticks send nothing and the 240th sends `failure` and returns `Failure`; a `victory` at 4 s is still refused.
  - type 1 in `crates/content-engine/src/loader/tests/action_conversion.rs`, beside the difficulty tests (line 476): `'hack'` and `'NoSuchGame'` are dropped; `'Hack'` and `'Livewire'` load.

  Revert `started()` and the first test fails; revert the name check and `'hack'` loads.
- **Lanes:** `fmt`, `clippy`, then `lane.ps1 cargo nextest run -p <crate>` for `cimmeria-minigame` and `cimmeria-content-engine`, each in turn. No live DB.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the shell rows (60 s server timeout, failure at expiry) and the loader's name check.

## MG-65: objective 4203 upgrade (not applicable)

D-MG6 = A: objective 4203 keeps Harset's D-H13 interact route. Nothing to build; chains 24040-24049 stay unused.

## Wave 7: UAT, docs and close-out

Order: MG-71, MG-73, MG-72, the campaign review loop (skill § 4), then MG-99. Each packet merges into `minigames/integration`; the campaign's one PR opens after MG-99.

### MG-71: cross-game TCP session test and REPL UAT index

- Agent: packet-coder · Host: windows · Wave 7 · Depends on: waves 3-6
- Worktree `mg-71` · Branch `minigames/mg-71-session-uat` · Commit `MG-71: every server-validated game survives a real SFS session`
- **Files:** new `crates/minigame/src/minigame/server/ported_games_tests.rs`, registered from `server/mod.rs` under `#[cfg(test)]`. For each game whose `validation()` is `"server"`, the test drives a real loopback SFS session through the existing listener test harness (copy the setup from `server/listener_tests.rs`, the Hack flow): login with the ticket, `joinOK`, the first golden move, then a client `victory`, which must be refused with no result sent.
- **Docs:** new `docs/analysis/minigames/uat-repl.md` (CRLF). For each game: the golden scripts, the command that replays them (`pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --script <file>`), and what a human checks in the transcript. This is the REPL-style logic UAT that [AGENTS.md](../../../../AGENTS.md) asks for.
- **Lanes:** C2 for `cimmeria-minigame`.

### MG-72: docs sweep

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: waves 1-6
- Worktree `mg-72` · Branch `minigames/mg-72-docs` · Commit `MG-72: minigame docs match the shipped games`
- **Scope:** walk [doc-update-map.md](../../../agents/doc-update-map.md) for the campaign's union of changes.
  - [minigame-system.md](../../../gameplay/minigame-system.md): the game table (validation, tiers, cheats), the launch flow, failure chains and the flags.
  - [minigame-wire-formats.md](../../../reverse-engineering/findings/minigame-wire-formats.md): the per-game message tables, linking the specs.
  - [CAT-K-minigame.md](../../../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md): the status of each finding.
  - `crates/minigame/README.md`, if one exists, or the crate docs in [crates/README.md](../../../../crates/README.md).
  - The `minigame-systems-advisor` memory: `game-implementation-status.md`.
  - The spec fixes the G1 report lists as owed at close-out.
  - Then `python tools/docs-gen/regen.py --skip-crate-graph` and `--check`, and `pwsh tools/lint-md.ps1` on every touched file.
- **Not here:** `docs/gap-analysis*` and `docs/project-status.md` (MG-99 only).

### MG-73: lab UAT recipes (ask-user-first)

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: waves 3-6
- Worktree `mg-73` · Branch `minigames/mg-73-lab-uat` · Commit `MG-73: lab UAT recipes for the minigames`
- **Files:** new `docs/analysis/minigames/uat-lab.md` (CRLF). Add one recipe per playable path:
  - the Livewire cellblock pair;
  - each ported game through a GM launch (Grep the GM commands for a minigame launcher; if there is none, record that as a gap rather than inventing one);
  - mission 582/2307 (GM-set the mission active: it has no offer path, M10);
  - ability 1327 or 2089;
  - Prison Boot: on zone load while mission 689 is open the wearer cannot walk but can use item 3438; a Livewire win frees them; a minigame window holds movement (MG-63a);
  - the shell 5 s refusal and the 60 s server timeout (MG-64);
  - D-MG11's dialog switch, on and off.

  Each recipe gives the steps, the expected client view, the server-log lines (`reason=` values), and the SigNoz query. It is written for a Haiku `lab-driver`, with the gotchas first: stand 4-6 m from targets, clear dialogs first, and verify the driver's claims against the server log.
- **Rule:** the first line of the file says **"Ask the user before any lab use."** This packet writes recipes only and runs nothing.
- **Docs:** [unified-uat.md](../../../guides/unified-uat.md) is MG-99's job; link to it from here.

### Campaign review

Run per the skill § 4 over `minigames/integration` against `main`, in an `ops` worktree named `minigames-review`. Its fixes merge into `minigames/integration` before MG-99.

### MG-99: close-out

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: the campaign review
- Worktree `mg-99` · Branch `minigames/mg-99-close-out` · Commit `MG-99: minigames campaign close-out`
- **Files:**
  - `docs/gap-analysis*` and `docs/project-status.md`, changed once: the minigame rows, saying what is server-validated, what stays client-declared, and what is owed;
  - [unified-uat.md](../../../guides/unified-uat.md): a "Minigames" section pointing at `uat-repl.md` and `uat-lab.md`, with the owner-run lab steps marked ask-first;
  - this campaign's README: every row's final status, the missing-content register with its final verdicts, and the open owner decisions;
  - `RESUME.md`: phase closed.
- **Then (ops):** one PR from `minigames/integration`; after it merges, retire the worktrees (`rm-worktree.ps1 --merged-into <PR> <name>...` for every packet worktree, then `mg-integration`); empty `.claude/state/active-campaign`; post the board handoff.
