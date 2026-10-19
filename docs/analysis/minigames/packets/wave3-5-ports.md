# Waves 3-5: per-game ports

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> **Skeleton until gate G1.** The architect fills each `<G>1`, `<G>2` and `<G>3` section from its spec (C5 § 10, port slicing), replacing every `G1:` line. A port section that still has a `G1:` line is not dispatchable.

## How the ports run

- There are four games, each a chain of three packets. A game's `<G>2` stacks on its `<G>1`, and its `<G>3` on its `<G>2`. Games run in parallel: each one edits only its own folder (C4), plus one line each in the shared files below.
- **Shared-file rule.** `games/mod.rs` and `games/kind.rs` were written by MG-03 and need no edits for a port. The only shared edit is the `PORTED` flip, which lives in the game's own `mod.rs`. A port that thinks it must edit another game's folder, or `protocol.rs`, stops and returns `blocked`.
- **Shared rules core.** If MG-R2 and MG-R3 show that CrystalGame and GoauldCrystals share a core, G1 makes `games/crystals_core/` part of MG-CG1, and MG-GC1 depends on MG-CG1.
- **Determinism.** One `StdRng::seed_from_u64(session.seed as u64)` per game. Never use `thread_rng`.
- **Agents.** `<G>1` and `<G>3` go to packet-coder; `<G>2` (move validation, the heaviest logic) goes to rust-gameserver-dev. Each `<G>2` and `<G>3` gets a server-authority-enforcer pass next to the packet-reviewer.
- **Rules (inline, C1):** PowerShell only, with `Set-Location <wt>;` on every command. Cargo runs only through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`), in the foreground. Edit only the game's folder and the named files. Files stay under the 500/700-line caps, with tests in `tests_*.rs`. Every change needs a test that fails on revert. Docs are CRLF. No `git stash`, prune or push; make one local commit with the brief's attribution. No lab.
- **Lanes, every port:**

  ```powershell
  Set-Location <wt>; cargo fmt -p cimmeria-minigame
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-minigame --all-targets -- -D warnings
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-minigame
  ```

## Template: what each slice contains

Each game section below fills this template.

### `<G>1`: board and setup (wave 3, packet-coder)

- **Worktree and branch:** worktree `mg-<g>1`, branch `minigames/mg-<g>1-setup`, commit `MG-<G>1: <Game> board generation and fullgamestate`.
- **Files:** `games/<game>/setup.rs` (the difficulty table as a `const` array indexed by tier, clamped per D-MG9; board generation; the `fullgamestate`/`joinOK` payload builder) and `games/<game>/tests_setup.rs`. `games/<game>/mod.rs` gains the `<Game>Game` struct and `MinigameInstance` impl: `board_snapshot` and `started` are real, `message` returns `vec![]`, and `create()` keeps returning the placeholder while `PORTED` is false. A `#[cfg(test)] pub(crate) fn create_real(...)` constructor serves the tests.
- **Tests (type 1):**
  - each tier's dimensions and timer match the spec's difficulty table;
  - the same seed gives the same board, and a different seed gives a different one;
  - the spec's golden transcript 1 initial payload matches field by field;
  - for a match-3 game, the generated board has no pre-made match, if the spec says the extension guarantees that.

### `<G>2`: moves and scoring (wave 4, rust-gameserver-dev)

- **Worktree and branch:** worktree `mg-<g>2`, branch `minigames/mg-<g>2-moves`, commit `MG-<G>2: <Game> server-validated moves and scoring`.
- **Files:** `games/<game>/moves.rs` and `games/<game>/tests_moves.rs`. `mod.rs`'s `message()` routes each inbound command in the spec's message table.
- **Validation.** Every inbound field is range-checked. An illegal move (out of bounds, not adjacent, wrong piece, after the game ended, or a stale move id) gets the spec's rejection reply, or none, and a WARN with `reason=` and the session identity fields. It never mutates state.
- **Tests (type 1):**
  - every legal move class, with the exact outbound payload;
  - each illegal class, refused and the board unchanged;
  - scoring per goal type;
  - the golden transcript 1 move sequence.

### `<G>3`: victory, timer, hints, cheats; ported (wave 5, packet-coder)

- **Worktree and branch:** worktree `mg-<g>3`, branch `minigames/mg-<g>3-rules`, commit `MG-<G>3: <Game> victory, timer and cheats; server-validated`.
- **Files:**
  - `games/<game>/rules.rs` and `games/<game>/tests_rules.rs`;
  - `PORTED = true` in `mod.rs`, with `create()` returning the real game;
  - golden scripts `crates/minigame/tests/scripts/<game>/*.mgs` (at least two, from the spec's transcripts, recorded with `minigame_repl --record` and checked by eye against the spec), plus a test that runs each through `repl::check_script`.
- **Rules.**
  - The win condition is server-decided (`GameOutput::Victory`).
  - The timer runs in `tick()`, and its expiry gives `GameOutput::Failure` (D-MG5 code 2), or code 4 where the spec says "defeated".
  - Hints come from `tick()`.
  - Ability cheats read `session.abilities_mask` (MG-22); item cheats use `GameOutput::ConsumeItem` (MG-23). Skip item cheats if MG-23 was ruled not applicable.
  - A client `victory` for this game is refused by `session_loop.rs`'s server-validated path; the packet adds a test proving that for this game.
- **Tests:**
  - type 1: the win condition, timer expiry, each cheat bit and the client-`victory` refusal;
  - type 1: the golden REPL scripts, run through `repl::check_script`. On revert of the flip, `validation("<Game>")` is `"client_declared"` and the refusal test fails.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the game's row (validation: server) and a "Rules" subsection of at most 15 lines. Doc-update-map row "Game systems".

## Alignment (MG-AL1..3)

- Spec: [spec-alignment.md](../spec-alignment.md). Folder `games/alignment/`. Name `Alignment`.
- G1: the difficulty table, the goal types and the cascade and refill rule (`rng` or deterministic).
- G1: the message table for `processmove`, `processreset`, `callhint`, `doscore` and the cheat timers `stct`, `upct` and `osct`, with the exact field names and types.
- G1: whether the hex geometry needs its own `hex.rs` (AL1) to stay under the file caps.
- G1: the golden transcripts as `.mgs` text.

## CrystalGame (MG-CG1..3)

- Spec: [spec-crystalgame.md](../spec-crystalgame.md). Folder `games/crystal_game/`. Name `CrystalGame`.
- G1: whether a shared `crystals_core` exists (see the shared-core rule above).
- G1: the `processMove` table, the `checkForVictory` rule, the difficulty table and the golden transcripts.

## GoauldCrystals (MG-GC1..3)

- Spec: [spec-goauldcrystals.md](../spec-goauldcrystals.md). Folder `games/goauld_crystals/`. Name `GoauldCrystals`.
- G1: the `handleRequest` dispatch table, `Swap`, the victory test found by MG-R3, the difficulty table checked against `deprecated/python/base/minigame/GoauldCrystals.py`, and the golden transcripts.

## ConverseBasicHumanoid (MG-CH1..3)

- Spec: [spec-conversebasichumanoid.md](../spec-conversebasichumanoid.md). Folder `games/converse_humanoid/`. Name `ConverseBasicHumanoid`.
- G1: the deck model (`Card`, `Suit`), the deal (CH1), the play rules (`Match`, `Swap`) and the server-side opponent (CH2).
- G1: whether the opponent AI is big enough to need its own slice. If it is, add MG-CH2b to the README table and split it here.
- G1: how the Trump passives 778, 779, 792 and 793 map onto `abilities_mask` bits (MG-22's table).
- G1: the golden transcripts.

## MG-LW1: Livewire parity fixes (only if G1 says so)

- Agent: packet-coder · Host: windows · Wave 3 · Depends on: MG-R6, G1
- Worktree `mg-lw1` · Branch `minigames/mg-lw1-parity` · Commit `MG-LW1: Livewire matches the client extension's tables`
- **Gate.** Dispatch only if [spec-livewire-parity.md](../spec-livewire-parity.md) lists a delta; otherwise G1 marks this packet Done as not-applicable.
- G1: one bullet per delta, with file:line, the old value, the new value and the spec evidence.
- **Tests (type 1):** one per delta, asserting the new value. Re-record `tests/scripts/livewire/d1_seed7.mgs` (MG-05) only if setup output changes, and say so in the report.
- **Lanes:** as above. **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the Livewire row, if a visible rule changed.
