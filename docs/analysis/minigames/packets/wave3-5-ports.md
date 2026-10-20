# Waves 3-5: per-game ports

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> Filled at gate G1 (2026-10-19) from the merged specs: [spec-alignment.md](../spec-alignment.md) (MG-R1), [spec-crystalgame.md](../spec-crystalgame.md) (MG-R2), [spec-goauldcrystals.md](../spec-goauldcrystals.md) (MG-R3), [spec-conversebasichumanoid.md](../spec-conversebasichumanoid.md) (MG-R4) and [spec-livewire-parity.md](../spec-livewire-parity.md) (MG-R6). The slicing is D-MG21; the Livewire deltas are D-MG17. Every open question the specs left is ruled below; nothing is left for the coder to choose.

## How the ports run

- **Waves.** Wave 3: AL1, CG1, GC1a, GC1b, CH1, LW1. Wave 4: AL2a, AL2b, CG2, GC2, CH2. Wave 5: AL3a, AL3b, CG3, GC3a, GC3b, CH3.
- **Chains.** Inside a game, each slice depends on the one before it (AL1 → AL2a → AL2b → AL3a → AL3b; CG1 → CG2 → CG3; GC1a → GC1b → GC2 → GC3a → GC3b; CH1 → CH2 → CH3). A slice's worktree is made from `minigames/integration` after its predecessor has merged there. Games are independent of each other and of wave 2, so a game's next slice may start before the rest of its wave is done.
- **Shipping.** Every slice merges into `minigames/integration` through ops `merge-noff.ps1` once its packet review says `ship`; the campaign ships one PR at the end. Merge order inside a wave does not matter (disjoint folders); within a game, the chain order.
- **Agents.** `<G>2` slices (AL2a, AL2b, CG2, GC2, CH2) go to rust-gameserver-dev; every other slice to packet-coder. A server-authority-enforcer pass runs beside the packet-reviewer on every `<G>2` and `<G>3` slice.
- **No shared core.** CrystalGame and GoauldCrystals share nothing but the SFS scaffolding (spec-goauldcrystals § 1). There is no `games/crystals_core/`.
- **Shared files.** `games/mod.rs` and `games/kind.rs` need no edit for a port: `kind.rs` already reads each module's `PORTED`, and MG-22's `ability_bits()` already returns each game's bits once it is server-validated. A port that thinks it must edit another game's folder, `protocol.rs`, `session_loop.rs` or `kind.rs` stops and returns `blocked`.

## Rules for every port slice (inline, C1)

1. PowerShell only, never Bash. Start every command with `Set-Location <wt>;`.
2. Cargo only through `pwsh tools/build-lane/lane.ps1` (`cargo fmt` may run directly), in the foreground.
3. Edit only the files the slice names, all under `crates/minigame/src/minigame/games/<folder>/` and `crates/minigame/tests/scripts/<folder>/`, plus the docs row the final slice owes.
4. Files: 500 lines soft cap, 700 hard. Tests go in `tests_*.rs` files beside the code, registered with `#[cfg(test)] mod tests_x;` in `mod.rs`.
5. Every behaviour has a type 1 test that fails when it is reverted; the report names the revert each test catches.
6. Docs are CRLF. Never hand-edit a `<!-- gen:NAME -->` block.
7. No `git stash`, `git worktree prune`, push or PR. One local commit with the slice's subject and the brief's attribution lines. No lab, no client files.
8. Determinism: one `rng: StdRng` per game, `StdRng::seed_from_u64(session.seed as u64)`, made in the constructor. Never `thread_rng`. Draw in the order the spec gives.
9. Timers run on the session's 250 ms `tick()`: subtract exactly 0.25 per tick, never wall time.

Lanes, every slice:

```powershell
Set-Location <wt>; cargo fmt -p cimmeria-minigame
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-minigame --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-minigame
```

Record a golden script (the `<G>3` slices):

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --record crates/minigame/tests/scripts/<folder>/<name>.mgs
```

## Contract excerpt (C4 and C6, as merged)

```rust
// crates/minigame/src/minigame/game.rs
pub trait MinigameInstance: Send {
    fn board_snapshot(&self) -> Option<BoardSnapshot>;
    fn started(&mut self) -> Vec<GameOutput>;
    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput>;
    fn tick(&mut self) -> Vec<GameOutput>;      // every 250 ms if needs_tick() was true at game start (read once)
    fn aborted(&mut self) -> Vec<GameOutput>;
    fn needs_tick(&self) -> bool;
}
pub enum GameOutput { Send(HashMap<String, SfsValue>), Victory, Failure }
```

- `GameOutput::Victory` reports cell result 1 and fires the victory chains; `GameOutput::Failure` reports cell result 2 (defeat) and fires the failure chains. An extension "code 3" is Victory, "code 4" is Failure (spec-protocol-common Q4).
- Each game folder's `mod.rs` has, today, `pub(crate) const PORTED: bool = false;` (line 5) and `pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance>` (line 6) returning the placeholder. Keep `create()` returning the placeholder while `PORTED` is false; tests build the real game with `#[cfg(test)] pub(crate) fn create_real(session: &MinigameSession) -> <Game>`.
- Session fields a game reads (`crates/minigame/src/minigame/session.rs`): `difficulty`, `tech_competency` (always 1 today), `seed`, `abilities_mask` (already filtered to the game's `ability_bits()`), `intelligence`, `player_level`, `entity_id`, `player_id`, `player_name`. Copy Livewire's struct fields for identity (`games/livewire/mod.rs`): `entity_id`, `player_id`, `player_name`, `rng`, `outcome: Option<&'static str>`.
- Build messages with `sfs_vars!` and `SfsValue::{Number, String, Bool, list(..)}` as Livewire does. Every outbound array is `SfsValue::list`.
- Golden test shape (`crates/minigame/src/minigame/repl_tests.rs:104`): `let golden = include_str!("../../../../tests/scripts/<folder>/<name>.mgs"); repl::check_script(golden).expect(..)`. `Repl::new` uses `MinigameSession::offline` (techcomp 1, abilities mask 0); a script cannot set a board or a cheat inventory, so cheat and fixture cases are Rust tests, not scripts.

## Message rules every port follows (D-MG17)

- **Client `victory` is refused.** A server-validated game has no `victory` arm: the command reaches the unknown-command arm, which for `cmd == "victory"` logs WARN `reason = "client_victory_refused"` and returns `vec![]`. A test per game sends `victory` and asserts no output and no `GameOutput::Victory`.
- **Unknown command:** WARN with `entity_id`, `entity_name`, `player_id`, `player_name`, `cmd` and `reason = "unknown_command"`, and `vec![]` (Livewire `mod.rs:373-384`).
- **Item cheats do not exist** (M8, MG-23 not applicable). Every `fullgamestate` sends `instcc = -1` and `CA0`-`CA4 = 0`. `activateitemcheat` logs WARN `reason = "item_cheat_unavailable"` and returns `vec![]`.
- **Ability cheats** read `session.abilities_mask`. A `cheatNum` outside 0-14, or whose bit is not set, logs WARN `reason = "ability_cheat_not_granted"` and returns `vec![]`. CrystalGame implements none (WARN `reason = "ability_cheat_unavailable"`).
- **Invalid input** (a missing field, a non-integer number, out of bounds, the wrong phase, after the end) logs WARN with a specific `reason` and returns `vec![]`. It never changes state.
- **After the end** (`outcome` is set) every command returns `vec![]` and `needs_tick()` is false.
- **`aborted()`** sets `outcome = Some("canceled")` and returns `vec![]`: the client has gone (D-MG17, MG-LW1 delta 14).

---

## Alignment (MG-AL1, AL2a, AL2b, AL3a, AL3b)

Spec [spec-alignment.md](../spec-alignment.md). Folder `games/alignment/`, name `Alignment`, struct `AlignmentGame`. Rulings on its § 11: refills use tier 1 (`const NEW_TYPE_USES_TIER: bool = false`, faithful); no deadlock reshuffle (the clock ends a stuck board, faithful); the n1 bounds fix in `get_hint`; the hoisting fix (cheats read the board dimensions before copying); the dead timer cheats `stct`/`upct`/`osct` are not ported; the tier is clamped to 1-4 and `minigamelevel = floor((techcomp - 1) / 5)` to 0-10; item cheats are gone (M8).

### MG-AL1: setup and the state shell

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-04 (merged) · Status Ready
- Worktree `mg-al1` · Branch `minigames/mg-al1-setup` · Commit `MG-AL1: Alignment board generation and fullgamestate`
- **Files** (spec § 10, lines 592-602):
  - `alignment/mod.rs` (~200 lines): `AlignmentGame` with the session identity fields, `rng`, `tier`, `level`, the board `Vec<Vec<u8>>` indexed `[x][y]`, `board_init` (the copy reset restores), `goal_scores`, `goal_totals`, `goal_icons`, `bonus_icons`, `time_remaining` (stored as time + 0.25, § 4 lines 175-178), `timer_state`, `num_resets`, `hint_index`, `phase: Phase` (§ 5; `Ready` only in this slice), `outcome`. `MinigameInstance`: `started()` sends `fullgamestate`; `message()` handles `processreset` (resend `fullgamestate` from `board_init`, `num_resets += 1`, time reset) and sends every other command to the unknown-command arm; `tick()` returns `vec![]`; `needs_tick()` false until AL3a; `aborted()` per the rules above. `#[cfg(test)] pub(crate) fn with_board(session, board: Vec<Vec<u8>>) -> Self` (§ 9 line 548). `PORTED` stays false.
  - `alignment/setup.rs` (~250 lines): the tier table (§ 4 lines 146-151) and the `goalTotals` table (lines 155-167) as `const` arrays; `clamp_tier(difficulty)` to 1-4 and `level_for(techcomp)` clamped to 0-10, each logging INFO when it clamps; `select_icons` (§ 3, from line 73) on `rng`; board generation with no initial match (§ 3); the `fullgamestate` builder with every key in § 5's server table (line 181 on), `timeRemaining` minus 0.25, `timerState 1`, `difficulty`, `minigamelevel`, `instcc -1`, `CA0`-`CA4 0`, `abilityBitfield` = `session.abilities_mask`.
  - `alignment/tests_setup.rs` (~200 lines).
- **Tests (type 1):** one spot row per tier and per level band against § 4 (level 0 and 10, tier 1 and 4); difficulty 0 and 9 clamp to 1 and 4; techcomp 60 clamps to level 10; the first quarter of cells are bonus icons (§ 3); a generated board has no match; the same seed gives the same board and seed 8 a different one; `fullgamestate` has every § 5 key with `timeRemaining 120` on tier 1; `processreset` resends the initial board with `numResets 1`; `with_board` with the § 9 fixture `base[x][y] = [4,5,6][(x + 2*(y % 2)) % 3]` sends that board; a client `victory` gives no output.
- **Docs:** none (AL3b writes the game row).

### MG-AL2a: geometry, matching and the swap

- Agent: rust-gameserver-dev · Host: windows · Wave 4 · Depends on: MG-AL1 · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-al2a` · Branch `minigames/mg-al2a-swap` · Commit `MG-AL2a: Alignment server-validated swaps and matching`
- **Files:** `alignment/moves.rs` (~450 lines) and `alignment/tests_moves.rs` (~300). In `mod.rs`, route `swappieces` and `resethinttimer` (§ 5, the client table from line 185, and § 5 `swapPieces` at line 245).
- **Content:** the neighbour table for both row parities (§ 3); `check_for_matches` and the flood (§ 6); `spin_check`; the power-up expansion `check_power_up_matches` and `get_match_power` (§ 6 lines 270-302; `mp = 12` gives 0.6 for types 1-3, § 11 item 2); `swap_pieces` with the D-MG2 triangle validation: the three cells must be in bounds, form a legal triangle, hold no blocker (9), and the client's `type1`-`type3` must equal the server's board, else WARN (`reason` `swap_not_triangle`, `swap_out_of_bounds`, `swap_blocker`, `swap_type_mismatch`) and no reply. Swap fields: `row<n>` is `y`, `col<n>` is `x`; confirm against § 3 and say so in the report. Outbound: `feedback` first (`spinCount`, `powerUpValues`), then `hexspinner` (match) or `hexspinner` with `matches []` (no match; the board is unchanged, the phase stays `Ready`) per § 5 and transcript A step 3 / B step 1. A match moves the phase to `AwaitCascade`.
- **Tests (type 1):** both neighbour parities; the match pair rule; transcript A step 3 against the fixture (with (2,2) set to 4: `feedback {spinCount 1, powerUpValues []}` then `hexspinner` with `matches` as a set `{(1,3,4),(2,1,4),(2,2,4),(3,0,4),(3,2,4)}`); transcript B step 1 (`spinCount 3`, `matches []`, board unchanged); power-up shapes p = 1-6; a power-up pulled in by another fires its own effect and no cell is added twice; each swap reject leaves the board equal; a swap while not `Ready` is refused.

### MG-AL2b: cascade, gravity and refill

- Agent: rust-gameserver-dev · Host: windows · Wave 4 · Depends on: MG-AL2a · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-al2b` · Branch `minigames/mg-al2b-cascade` · Commit `MG-AL2b: Alignment cascade, gravity and refill`
- **Files:** `alignment/cascade.rs` (~250: `check_for_cascade`, refill with the A/B lists, blocker and clone rolls, `get_new_type` behind `NEW_TYPE_USES_TIER = false`, the re-roll pass, `hexmove` and `cascadecheckcomplete` builders), `alignment/moves_gravity.rs` (~150: the six `doCascade` slide walks), `alignment/tests_cascade.rs` (~300). Route `cascade` (phase `AwaitCascade` → `AwaitCheck`) and `checkForCascade` (`AwaitCheck` → `Ready` with `dropIndex += 1`, or another cascade) in `mod.rs`.
- **Spec:** § 6 `checkForCascade` and `doCascade`, lines 303-391. Port the six walks line for line from the decompiled listing: `Set-Location <wt>; python tools/minigame-swf/list_functions.py --help` first, then run it with `--function doCascade` on the Alignment movie named in spec-alignment § 1 (the client Flash folder is `C:\Users\Steve\source\projects\SGW\Client\game\Working\SGWGame\CookedPC\UI\Flash\`). Quote at most 20 lines of the listing in the report (no client bytes in git, D-MG10). Where the listing and the § 6 table disagree, the listing wins: note it in the report (the architect fixes the spec at close-out, § 11 item 7).
- **Tests (type 1):** transcript A step 4 against the fixture: `hexmove` with `direction 0` and `hexesToMove` exactly `[{1,0,3,6},{2,1,2,5},{2,0,2,4},{1,1,1,6},{1,0,1,6},{1,2,0,5},{1,1,0,5},{1,0,0,4}]` in that order, and `newHexes` at (0,0), (0,1), (0,2), (0,3), (1,2) (types from the rng; assert positions and count); one test per gravity direction 0-5 on a small hand board; refill A/B power-up grant; `cascade` in `Ready` and `checkForCascade` in `AwaitCascade` are refused; the second swap's `hexmove` carries `direction 1` (step 7).

### MG-AL3a: score, victory, timer and hints

- Agent: packet-coder · Host: windows · Wave 5 · Depends on: MG-AL2b · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-al3a` · Branch `minigames/mg-al3a-rules` · Commit `MG-AL3a: Alignment score, victory, timer and hints`
- **Files:** `alignment/rules.rs` (~300) and `alignment/tests_rules.rs` (~300); route `doscore`, `dovictory`, `callhint` in `mod.rs`; `needs_tick()` true from the start until the game ends (the session loop reads it once, before the first click).
- **Content:** `do_score` (§ 6 lines 392-410: the multiplier and bonus per `cascadeCount`, power-ups and type 7 never score, `updatescore` then `timerupdate`); `check_for_victory` sends `activate` once all goals are met (§ 6 Win, line 411); `dovictory` before `activate` is refused (WARN `reason = "victory_not_earned"`), after it sends `victory` and returns `GameOutput::Victory`; the countdown (§ 8 lines 500-505) with `failure` plus `GameOutput::Failure` at 0 (§ 6 Loss, line 418); the idle prompt `clicktohint` (§ 8 lines 507-516, first at 5 s); `get_hint` with the four scan orders (lines 518-532), the n1 bounds fix, `callhint` costing 5 s with `timerupdate` even when no hint is found.
- **Tests (type 1):** transcript A step 6 (`updatescore {goalScores [5,0,0], goalTotals [45,15,6], goalIcons [4,5,6]}` then `timerupdate {timeRemaining 120, timerState 1}`); multiplier per `cascadeCount`; power-ups and type 7 unscored; `activate` sent once; the `dovictory` gate; the hint on the fixture with (2,2)=4 is `hex1 (1,1,4), hex2 (0,1,6), hex3 (1,2,5)` (§ 9 step 2); the hint index rotates through the four orders; transcript B step 2 (`callhint` on the plain fixture: no `hint`, then `timerupdate {timeRemaining 115}`); the idle prompt at tick 20 and 10 s later; `tick` to 0 gives `failure` and `Failure`, then any command returns nothing.

### MG-AL3b: ability cheats, goldens, ported

- Agent: packet-coder · Host: windows · Wave 5 · Depends on: MG-AL3a, MG-22 (merged) · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-al3b` · Branch `minigames/mg-al3b-ported` · Commit `MG-AL3b: Alignment ability cheats; server-validated`
- **Files:** `alignment/cheats.rs` (~200) and `alignment/tests_cheats.rs` (~200); `PORTED = true` and `create()` returning the real game in `mod.rs`; goldens `crates/minigame/tests/scripts/alignment/a_scoring_swap.mgs` and `b_reset.mgs` plus their `check_script` tests in `tests_rules.rs`.
- **Content:** `activateabilitycheat` per § 7 lines 461-484: bit set → `process_ability_cheat`, clear the bit, send `abilitycheatstate {abilityBitfield}`; bits 0-14 per the table at lines 465-476. Bit 10 has nothing to restore (`instcc` is -1 for good): the bit is spent and `abilitycheatstate` is sent, nothing else. `check_for_match_cheat` (bits 2, 3, 8) after each matching swap; cheat clears never score (line 480); read the board dimensions before copying (the hoisting fix, line 482).
- **Goldens.** Scripts cannot inject the § 9 fixture, so the two `.mgs` files are recorded on the seeded board: `game Alignment difficulty 1 seed 7`. `a_scoring_swap.mgs`: `start`, then the swap the server's own `callhint` names (run `> callhint` once in the recording session, then `> resethinttimer` and `> swappieces` with that hint's three cells), `> cascade`, `> checkForCascade`, `> doscore`. `b_reset.mgs`: `start`, `> callhint`, `> processreset`. Check both by eye against transcripts A and B (message order and keys) and say so in the report. Transcripts A and B on the fixture stay as the Rust tests AL2a, AL2b and AL3a wrote.
- **Tests (type 1):** each ability bit's effect and its `abilitycheatstate`; an unset bit and `cheatNum 15` are refused; `activateitemcheat` gives no output; `ability_bits()` for `GameKind::Alignment` is `0x7FFF` and `validation("Alignment")` is `"server"` (revert the flip and both fail); a client `victory` gives no output; both goldens pass `check_script`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md): the Alignment row says validation server, and a "Rules" subsection of at most 15 lines (goal icons, scoring, timer, hints, cheats bits 0-14, no item cheats). Doc-update-map row "Game systems".

---

## CrystalGame (MG-CG1..3)

Spec [spec-crystalgame.md](../spec-crystalgame.md). Folder `games/crystal_game/`, name `CrystalGame`, struct `CrystalGame`. It is lights-out mod 6 on a fixed 14x13 board, 16 s, at every tier (§ 3, § 4). Rulings on its § 11: one fixed configuration (faithful); no cheats (`abilityBitfield 0`, both cheat commands get a WARN and no reply); moves only from the playing user; **reset rescrambles**: `processreset` sets every cell to 3, draws two new scramble clicks from the same `rng`, sets `timeRemaining 16`, `timerState 0`, `num_resets += 1`, and sends `fullgamestate` (a server-authoritative improvement on a button the original ignored; recorded deviation).

### MG-CG1: board, scramble and fullgamestate

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-04 (merged) · Status Ready
- Worktree `mg-cg1` · Branch `minigames/mg-cg1-setup` · Commit `MG-CG1: CrystalGame board, scramble and fullgamestate`
- **Files** (spec § 10, line 300): `crystal_game/setup.rs` (~110: `Board { w: 14, h: 13, cells: Vec<u8> }` indexed `[x][y]`; `press(x, y) -> Vec<(u8, u8, u8)>` returning the changed cells in the order centre, left, up, right, down (§ 3 lines 69-84); `scramble(&mut rng)` drawing x1, y1, x2, y2 with `gen_range(0..14)` / `gen_range(0..13)` (line 122); the `fullgamestate` builder per § 5 line 161 with `timeRemaining 16`, `timerState 0`, `instcc -1`, `CA0`-`CA4 0`, `abilityBitfield 0`), `crystal_game/mod.rs` (~110: `CrystalGame` and `MinigameInstance`; `started()` sends `fullgamestate`; `message()` answers every command with the unknown-command arm for now), `crystal_game/tests_setup.rs` (~130).
- **Tests (type 1):** dimensions 14x13; an all-3 board then two `press` calls; the draw order for a fixed seed; a scrambled board is never solved; `gameArray` is a list of 14 lists of 13; `press` in a corner, on an edge and inside returns 3, 4 and 5 cells in the § 3 order.
- **Seed search** `finds_seed_for_clicks` (`#[test] #[ignore]`, run once with `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-minigame --run-ignored only finds_seed_for_clicks`): search seeds 0..100000 for the first whose clicks are (0,0) then (6,6) (T1), and the first whose clicks are (13,5) twice (T2). Put the two numbers in the report and as consts `T1_SEED` and `T2_SEED` in `tests_setup.rs` with a non-ignored test asserting each seed's clicks. If a seed is not found, report `blocked`.

### MG-CG2: moves and reset

- Agent: rust-gameserver-dev · Host: windows · Wave 4 · Depends on: MG-CG1 · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-cg2` · Branch `minigames/mg-cg2-moves` · Commit `MG-CG2: CrystalGame server-validated clicks and reset`
- **Files:** `crystal_game/moves.rs` (~100), `mod.rs` (+40), `crystal_game/tests_moves.rs` (~150).
- **Content** (§ 5 lines 148-155): `processmove {x, y}`: both integers with 0 ≤ x < 14 and 0 ≤ y < 13, game running, else WARN (`reason` `bad_coordinates` or `game_over`) and no reply. The first valid click starts the clock: `timerupdate {timeRemaining 16, timerState 1}` before that click's `updateslot`s (§ 8 line 229). Then one `updateslot {x, y, num}` per changed cell in the `press` order. `processreset`: refused (decision: ignored, WARN `reset_refused`, no reply), matching the original; a rescramble would let a client reset before the 16 s limit for unlimited time. `activateitemcheat` and `activateabilitycheat`: WARN, no reply.
- **Tests (type 1):** per-position message counts and order; the `(v+2)%6` and `(v+1)%6` wraps; six presses are the identity; out-of-bounds, negative, missing and fractional coordinates give no output and do not start the clock (spec T3 including `processreset`); `processreset` gives no output, does not touch the board and does not stop or start the clock; both cheat commands give no output.

### MG-CG3: timer, victory, goldens, ported

- Agent: packet-coder · Host: windows · Wave 5 · Depends on: MG-CG2 · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-cg3` · Branch `minigames/mg-cg3-rules` · Commit `MG-CG3: CrystalGame timer and victory; server-validated`
- **Files:** `crystal_game/rules.rs` (~120), `mod.rs` (+40), `crystal_game/tests_rules.rs` (~160); goldens `crates/minigame/tests/scripts/crystal_game/t1_solved.mgs`, `t2_timeout.mgs`, `t3_validation.mgs`; `PORTED = true`.
- **Content** (§ 6 lines 175-182, § 8 lines 226-232): state 0 idle until the first valid click, 1 counting; `tick` subtracts 0.25 and at `<= 0` sends `failure` and returns `GameOutput::Failure` (tick 64 after the first click); after every click, if every cell is 3: set the timer state to 0, send `victory`, return `GameOutput::Victory` (the original's missing timer stop is fixed); `needs_tick()` true from the start until the game ends (the session loop reads it once, before the first click); nothing after the end.
- **Goldens:** T1 and T2 exactly as spec § 9 lines 245-288, using CG1's `T1_SEED` and `T2_SEED` in the `game CrystalGame difficulty N seed <n>` header. T3 as § 9 lines 290-292, including `> processreset` as an ignored command. Record each, then check by eye that the recorded lines equal the spec's.
- **Carried notes:** set `outcome` on victory and on timeout, and stop the timer on victory; the session layer must route only the playing user's packets to `message()` before `PORTED` flips.
- **Tests (type 1):** the three goldens through `check_script`; no timer before the first click (`tick 100` silent); no `failure` after `victory` (fails if the stop is reverted); timeout at tick 64; a client `victory` gives no output; `validation("CrystalGame")` is `"server"` and `ability_bits()` is 0.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md): the CrystalGame row (validation server) and a "Rules" subsection of at most 10 lines (lights-out mod 6, 14x13, 16 s from the first click, reset is ignored, no cheats). Doc-update-map row "Game systems".

---

## GoauldCrystals (MG-GC1a, GC1b, GC2, GC3a, GC3b)

Spec [spec-goauldcrystals.md](../spec-goauldcrystals.md). Folder `games/goauld_crystals/`, name `GoauldCrystals`, struct `GoauldCrystalsGame`. It is a path-building puzzle: move greens through empty cells until a green path powers the finish goal (§ 3, § 6); it is not a match-3. Rulings on its § 11: keep the `seed == 1` 4x4 tutorial board (the goldens rely on it; online seeds are random, so it almost never appears); clamp `L` to 10 and the difficulty to 1-4, logging INFO on a clamp, instead of the 7x7 fallback; send `fullgamestate` once; `rules.rs` plus `cheats.rs`; item cheats are gone (M8), so `activeItemCheat` is always -1 and the armed-mode steps and path mode 2 are not ported; golden A is dropped. The shipped bugs at § 6 lines 294-304 are fixed as that table says.

### MG-GC1a: table, goals, crystals, fullgamestate

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-04 (merged) · Status Ready
- Worktree `mg-gc1a` · Branch `minigames/mg-gc1a-setup` · Commit `MG-GC1a: GoauldCrystals board generation and fullgamestate`
- **Files:** `goauld_crystals/mod.rs` (~180: the struct with session identity, `rng`, `w`, `h`, `board: Vec<Vec<Cell>>` (`Cell { color: u8, var: u8, power: bool }`), the goals (side, offset, x, y for start and finish), `selected: Option<(usize, usize)>`, `time_remaining`, `max_time`, `timer_state`, `ability_bitfield` (from the session), `yellows_without_orange`, `reds_to_yellows`, `reds_to_remove`, `reds_to_revert: Option<Vec<(usize, usize)>>`, `item_cheats_add_time`, `num_resets`, `outcome`; `started()` sends `fullgamestate`; `message()` answers everything with the unknown-command arm), `goauld_crystals/setup.rs` (~300), `goauld_crystals/tests_setup.rs` (~250).
- **Content** (§ 3 lines 65-151, § 4 lines 152-174): the 11x4 difficulty table as final values (`w`, `h`, `time`, `obstacles`) from lines 158-170, indexed `[L][difficulty - 1]` with `L = floor((techcomp - 1) / 5)` clamped to 0-10; `place_goals` (lines 107-113, `min_dist = max(w,h) - 1`, up to 100 draws for the finish, then a WARN `reason = "goal_distance_unmet"` and the last draw); `randomize_crystals` with the fractional counts and the capacity loop (lines 115-131); the tutorial board (lines 133-142) when `seed == 1 && w == 4 && h == 4`; `init_game(first_time)` calling `randomize_crystals` once (GC1b adds the power loop); builders for `fullgamestate` (§ 5 line 211, every key; `activeItemCheat -1`, `instcc -1`, `CA0`-`CA4 0`, `abilityBitfield` from the session, `selectedXPos/YPos -1` when nothing is selected, `gameArray` a list of columns of `{color, var, power}` objects) and `timerupdate {timeRemaining, timerState}`.
- **Tests (type 1):** both ends of every table column (L 0 and 10, difficulty 1 and 4); techcomp 60 and difficulty 7 clamp; the tutorial board and goals for `seed 1, difficulty 2`; goal distance on the smallest and largest tiers; the capacity loop on 4x4 and 14x12 keeps at least `h` cells empty in the budget; ceil placement of fractional counts; `fullgamestate` key set and types.

### MG-GC1b: power search and the unsolved-board loop

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-GC1a · Status BlockedDependency
- Worktree `mg-gc1b` · Branch `minigames/mg-gc1b-power` · Commit `MG-GC1b: GoauldCrystals power path and never-solved boards`
- **Files:** `goauld_crystals/power.rs` (~120: `update_power(&mut board, start) -> Vec<(usize, usize)>`, breadth-first from the start goal if it is green, over 4-neighbours in the order `x-1, y-1, x+1, y+1`, following colour 1 only, setting `power` and returning the visit order (§ 3 lines 144-146); `finish_powered()`; the `powerstatus {powerArray: [{xPos, yPos}, ..]}` builder), `goauld_crystals/tests_power.rs` (~150); `setup.rs`: `populate_game_board` = place goals once, then repeat `randomize_crystals` + `update_power` until the finish is not powered (lines 102-105), called from `init_game`.
- **Tests (type 1):** the tutorial board powers exactly (0,0), (0,1), (1,1), (2,1), (3,1) in that order (§ 9 line 370); a start that is not green powers nothing; a hand board where the finish is powered; 200 seeds on tier L0 difficulty 1 never start with the finish powered (fails if the loop is removed and a seed hits it: assert by forcing the first draw with a hand board through a test hook if no seed in 200 does).

### MG-GC2: clicks, path moves and swaps

- Agent: rust-gameserver-dev · Host: windows · Wave 4 · Depends on: MG-GC1b · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-gc2` · Branch `minigames/mg-gc2-moves` · Commit `MG-GC2: GoauldCrystals server-validated clicks and moves`
- **Files:** `goauld_crystals/moves.rs` (~400), `goauld_crystals/tests_moves.rs` (~340); route `playfieldclick` in `mod.rs`.
- **Content** (§ 5 lines 228-251, steps 1 and 3-9; step 2 and path mode 2 are not ported): validation first (integers, `0 <= x < w`, `0 <= y < h`, game not over, else WARN `bad_coordinates` / `game_over`); clock start with `timerupdate`; `reds_to_yellows` and `reds_to_remove`; select (`selectedupdate`, `triggersound select`); path move by BFS from the selection in the order `x-1, y-1, x+1, y+1` (each neighbour compared with the destination first, then queued if empty); the adjacent swap with the yellow-to-orange rule and `yellows_without_orange`; `invalid` sound; clear the selection; the `reds_to_revert` remap and revert (step 9). After every board change call one `after_board_change()` that runs `update_power` and, if the finish is powered, sets `outcome`, stops the clock, sends `fullgamestate` then `victory`, and returns `GameOutput::Victory`; otherwise sends `powerstatus`. Builders for `squarestate {xPos, yPos, info}` and `triggersound {soundName}`.
- **Tests (type 1):** transcript B steps 1-5 (§ 9 lines 384-392) and C steps 1-3 (lines 397-405) message by message; the path search goes through empties only; path mode never falls back to an adjacent swap; yellow to orange with and without `yellows_without_orange`; out-of-bounds and fractional clicks dropped; a `reds_to_revert` set by hand reverts after a move that does not win; a move that powers the finish gives `fullgamestate`, `victory` and `Victory`, then input is ignored.

### MG-GC3a: timer, timeout and reset

- Agent: packet-coder · Host: windows · Wave 5 · Depends on: MG-GC2 · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-gc3a` · Branch `minigames/mg-gc3a-rules` · Commit `MG-GC3a: GoauldCrystals clock and reset`
- **Files:** `goauld_crystals/rules.rs` (~200), `goauld_crystals/tests_rules.rs` (~200); refuse `processreset` in `rules.rs`; `needs_tick()` true from the start until the game ends (the session loop reads it once, before the first click).
- **Content** (§ 6 lines 281-292, § 8 lines 356-364): timer states 0-3 on the 0.25 s tick (state 2 pause and 3 reverse each 20 ticks, then back to 1 with `timerupdate`); at `<= 0` send `failure` and return `GameOutput::Failure`; `processreset` is refused: WARN `reset_refused`, no reply, state unchanged. This is a deliberate deviation from § 5 line 224 (which sends `resetgame` and a new board), for owner confirmation: a reset refills the clock and the ability bits, the same server-authority hole the CrystalGame ruling refuses.
- **Tests (type 1):** transcript B step 6 (`tick 64` gives `failure`); transcript C step 4 (`tick 8` is silent and leaves `timeRemaining` 14); pause and reverse each end with one `timerupdate`; `processreset` gives no output and changes nothing; a reset after the deadline ends the game as `failure`; nothing after the end; refusals are logged in full for the first eight, then one in a hundred.

### MG-GC3b: ability cheats, goldens, ported

- Agent: packet-coder · Host: windows · Wave 5 · Depends on: MG-GC3a, MG-22 (merged) · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-gc3b` · Branch `minigames/mg-gc3b-ported` · Commit `MG-GC3b: GoauldCrystals ability cheats; server-validated`
- **Files:** `goauld_crystals/cheats.rs` (~300: bits 0-14 per § 7 lines 328-354, one parametrised edge-removal for the four sides, `add_random_greens(n, overwrite)`; each cheat starts the clock if stopped, plays its sound from § 5 line 226, clears the bit and sends `abilitycheatstate`; bit 10 has no instrument to restore, so it is spent with `abilitycheatstate` only; bit 11 sets `item_cheats_add_time`, which has no effect without item cheats), `goauld_crystals/tests_cheats.rs` (~250); `PORTED = true`; goldens `crates/minigame/tests/scripts/goauld_crystals/b_move_timeout.mgs` and `c_path_move.mgs`.
- **Goldens:** transcripts B and C from § 9 (lines 384-408) with the header `game GoauldCrystals difficulty 2 seed 1`. Record each and check every line against the spec's tables.
- **Tests (type 1):** each bit's effect (edge removal skips an edge with a goal and keeps the bit spent when none qualifies; bit 13 reverts after a move that does not win); an unset bit and `cheatNum 15` refused; `activateitemcheat` gives no output; both goldens through `check_script`; a client `victory` gives no output; `ability_bits()` for `GameKind::GoauldCrystals` is `0x7FFF` and `validation("GoauldCrystals")` is `"server"`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md): the GoauldCrystals row (validation server; a path-building puzzle) and a "Rules" subsection of at most 15 lines. Doc-update-map row "Game systems".

---

## ConverseBasicHumanoid (MG-CH1..3)

Spec [spec-conversebasichumanoid.md](../spec-conversebasichumanoid.md). Folder `games/converse_humanoid/`, name `ConverseBasicHumanoid`, struct `ConverseHumanoidGame`. A timed card contest against a server-side NPC (§ 3-§ 6). Rulings on its § 11: keep the unanswered-card fall-through and `newhand` (faithful); pair by play order (faithful); clamp the time ratio to 0; **hold the hand and NPC timers after `newdeal` until `processreset`** (the countdown still runs and ends the game); a client `failure` is a Defeat (`failure` echoed, `GameOutput::Failure`; the client can only hurt itself); `intelligence` and `tech_competency` are used as the session sends them; the NPC victory lists are dropped; there are no item cheats; the Trump bits 15-18 add +5/+10/+15/+25 (§ 7 lines 217-232).

### MG-CH1: deal, accords and fullgamestate

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-04 (merged) · Status Ready
- Worktree `mg-ch1` · Branch `minigames/mg-ch1-setup` · Commit `MG-CH1: ConverseBasicHumanoid deal, accords and fullgamestate`
- **Files:** `converse_humanoid/mod.rs` (~120: the struct with session identity, `rng`, `game_type`, both hands as `Vec<f64>`, `EmptyContainers`, `ContestPlays` and `TotalPlays` per side, scores, `time_remaining` (30.25), `timer_state`, `npc_last_time`, `npc_timer` (3.0), `hand_last_time`, `hand_timer` (6.0), `contests_without_accord`, `num_resets`, `player_victory_used`, `outcome`; `started()` sends `fullgamestate`), `converse_humanoid/setup.rs` (~250), `converse_humanoid/tests_setup.rs` (~150).
- **Content:** the game-type table (§ 4 lines 89-96; logic3 first, mediation5 every later deal); the deck (each suit twice) and accord pools; the seeded deal, NPC hand first then the player's (§ 3 lines 67-83), sorted numerically; `get_accords` with the Trump bits (§ 7 lines 236-245); the player victory conditions `[focusdamage1, focusdamage2]` with costs `[3, 4]` (lines 247-249); the `fullgamestate` builder (§ 5 line 136, without `suitTypes`; `timeRemaining` minus 0.25; `npcType "humanoid"`, `isSpectator false`).
- **Tests (type 1):** the table values; logic3 deals 3 cards a side and mediation5 4; no card is dealt twice beyond its two copies; the accord threshold at the boundary; the Trump bit sum (+5 +10 +15 +25 = 55 with all four bits); a fixed seed gives a fixed deal; `fullgamestate` keys and types.

### MG-CH2: plays, the NPC and hand scoring

- Agent: rust-gameserver-dev · Host: windows · Wave 4 · Depends on: MG-CH1 · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-ch2` · Branch `minigames/mg-ch2-moves` · Commit `MG-CH2: ConverseBasicHumanoid validated plays and NPC opponent`
- **Files:** `converse_humanoid/moves.rs` (~250), `converse_humanoid/tests_moves.rs` (~200). Functions only; CH3 wires them into `message()` and `tick()`.
- **Content:** `player_move(num, suit, container)` (§ 5 lines 118-130: running; the contest not full; the card found by `suit` value and removed; `container` absent or in range and empty; echo the client's `num` in `updatecards`; else WARN and no output); `npc_move` with `eval_weight` and the focus answer (§ 8 lines 279-297, strict argmax, ties to the lowest index); `end_hand` in the order of § 6 lines 167-177 (returning `newdeal`, `emptyhand` or the failure path); `process_score` pairing by play order, the sweep bonus, `updatescore` (lines 179-188); `clear_containers`.
- **Tests (type 1):** the dominance table for all 81 suit pairs against `(p - n + 8) % 9`; the NPC's choice on transcript B's hands (§ 9 line 353); the focus answer; each hand-end branch 1-5; a sweep scores 2 in logic3; a suit not in the hand, a used container and container 5 are refused with no output.

### MG-CH3: timers, deal flow, victory, goldens, ported

- Agent: packet-coder · Host: windows · Wave 5 · Depends on: MG-CH2, MG-22 (merged) · Status BlockedDependency · Extra pass: server-authority-enforcer
- Worktree `mg-ch3` · Branch `minigames/mg-ch3-rules` · Commit `MG-CH3: ConverseBasicHumanoid timers and result; server-validated`
- **Files:** `converse_humanoid/rules.rs` (~200), `converse_humanoid/tests_rules.rs` (~200), the `message()` and `tick()` dispatch in `mod.rs`; `PORTED = true`; goldens `crates/minigame/tests/scripts/converse_humanoid/a_idle_hand.mgs`, `c_rejected.mgs`, and `b_first_deal.mgs` if a seed exists.
- **Content:** the tick (§ 8 lines 265-277) on 0.25 s; timers held between `newdeal` and `processreset`; `newhand`; `processreset` only after `newdeal` (`num_resets += 1`, `contests_without_accord += 1`, next game type, new deal, keep scores and time, `fullgamestate`); `check_for_victory` at the countdown (ties win: `victory` + `GameOutput::Victory`; NPC ahead: `failure` + `GameOutput::Failure`); `fail_message`; the gated `activatevictory` (§ 7 lines 253-257: cost from `playerPointCosts[victoryNum - 1]`, refused out of range, used, or unaffordable); a client `failure` gives `failure` and `GameOutput::Failure`.
- **Goldens:** A and C as § 9 lines 303-370 (seed 7). For B, search seeds 0..100000 in an `#[ignore]` test for a deal of NPC `[0,4,8]` and player `[0,4,8]` with no accords; if found, record B with it; if not, ship A and C only and say so in the report.
- **Tests (type 1):** the goldens through `check_script`; the timers hold after `newdeal` (24 ticks after `newdeal` without `processreset` send nothing); `activatevictory` refusals and one success (`activatevictory` then `updatescore`); timeout with a tie gives Victory and with the NPC ahead gives Failure; a client `victory` gives no output; `ability_bits()` for `GameKind::ConverseBasicHumanoid` is `0x78000` and `validation("ConverseBasicHumanoid")` is `"server"`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md): the ConverseBasicHumanoid row (validation server) and a "Rules" subsection of at most 15 lines (dominance circle, hands and deals, the NPC, Trump passives). Doc-update-map row "Game systems".

---

## MG-LW1: Livewire parity fixes

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-R6, MG-05 (merged) · Status Ready
- Worktree `mg-lw1` · Branch `minigames/mg-lw1-parity` · Commit `MG-LW1: Livewire matches the client extension's tables`
- **Why.** [spec-livewire-parity.md](../spec-livewire-parity.md) lists 18 deltas (lines 37-58). Deltas 1 and 2 make wires invisible today. The rulings (D-MG17) below are final. Files: `games/livewire/setup.rs`, `games/livewire/mod.rs`, `games/livewire/tests.rs` (or a new `tests_parity.rs` if `tests.rs` passes 450 lines), `crates/minigame/src/minigame/server/tests.rs`, `crates/minigame/tests/scripts/livewire/d1_seed7.mgs`.

| # | Where (setup.rs unless named) | Change |
|---|---|---|
| 1 | `PLAYFIELD_WIRES` (line 102) and `MOVING_WIRES` (about 115-120), used at 240 and 306 | Append a `random_range(1..=4)` suffix to each playfield and moving wire lib (`pGray3`). Electrodes keep theirs; LED libs and `mGear` get none. |
| 2 | 261, 327 | Goal and obstacle variant `random_range(1..4)` becomes `random_range(1..=4)`. |
| 3 | 259 (`find_random_depth` at 177) | Goal depth is `twoRandRange(1,25,60,85)`: draw `a` in 1..=25, then `b` in 60..=85, then `random_range(0..2)` picks `a` (0) or `b` (1). Keep the uniqueness check `find_random_depth` does. |
| 4 | 221-222, 287-288 | Floor the electrode x and y. |
| 5 | 150-152 | `obstacle_total` is `floor(..) * 2`. |
| 6 | 152 | `obstacle_timer = base + 5 * ((tc - 1) / 5)` in integers, base 5/6/7/9 for difficulty 1-4. |
| 7 | 88 | The D4 `move_timer_divider` is 0. |
| 8 | 160-161 | Timer method 0 is `(param_timer * tc) / 1.7`. |
| 9 | 160-161 | Keep method 1 as it is; add a comment that the original computes `(17 + tc*4.5) ^ 1.25` as a bitwise XOR on truncated integers and the port keeps a real power on purpose. |
| 10 | 140 | Keep `.min(3)`; add a comment that content difficulty 5 uses difficulty 4's table. |
| 11-13 | mod.rs 113-118, 371 | No cheats (D-MG17): `activateitemcheat` gets WARN `reason = "item_cheat_unavailable"`, `activateabilitycheat` gets WARN `reason = "ability_cheat_unavailable"`; both return `vec![]`. `instcc`/`CA` stay as they are. |
| 14 | mod.rs 433-439 | `aborted()` sets `outcome = Some("canceled")` and returns `vec![]` (no `failure`). |
| 15 | 346-366 | No change. |
| 16 | 408 | `timerState` in `fullgamestate` is the live state, `SfsValue::Number(self.timer_state as f64)` (0 at join), not the string `"0.0"`. |
| 17-18 | | No change. |

- **Adjacent fix.** In `processmove`, `wire.cut = true` (mod.rs:301) runs before the `'p'` prefix check (310-322), so a playfield wire the server ignores is still marked cut. Move the assignment after the check, so an ignored `p` wire stays uncut.
- **Tests (type 1):** one per changed delta (1-8, 11, 14, 16) asserting the new value: every playfield and moving lib ends in 1-4; goal and obstacle variants reach 4 over 200 seeds; goal depths fall in 1-25 or 60-85; electrode x and y are whole; `obstacle_total` and `obstacle_timer` for each difficulty at tc 1 and 6; the D4 divider; method 0's timer; both cheat commands give no output; `aborted()` gives no output; `timerState` is a Number. One for the adjacent fix: a `p` wire processed while the playfield is inactive is still uncut afterwards.
- **Fix the existing tests the change breaks:** `crates/minigame/src/minigame/server/tests.rs` lines 350-378 expect a `failure` on abort; they now expect none. Re-record `d1_seed7.mgs` (the abort `failure` line goes and the setup lines change) with the record command above, and say in the report which lines changed.
- **Lanes:** as above. **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the Livewire row: "an abort sends nothing; the client has closed". Doc-update-map row "Game systems".
