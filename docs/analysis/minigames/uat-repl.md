# Minigames REPL UAT

> Type: how-to. Part of the [minigames campaign](README.md). This is the logic UAT that [AGENTS.md](../../../AGENTS.md) asks for: it runs the server-side game code with no client and no lab. Lab runs are in a separate recipe file, and they need the owner's go-ahead first.

## What a script is

Each golden script under `crates/minigame/tests/scripts/<game>/` starts with `game <Name> difficulty <n> seed <s>`, then alternates client lines (`>`, the move the SWF would send) and server lines (`<`, the frame the server must send back). `#` lines are comments.

`minigame_repl --script <file>` replays the `>` lines through the same game code the live server runs, compares the `<` lines it produces, and exits **0** on a match or **1** on a mismatch. Exit 0 is the pass. A human checks the transcript as well, because a script can be recorded wrong; the checks listed below are what the transcript must show.

## Replay command

From the worktree root, one script at a time:

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --script <file>
```

Example, the Livewire golden:

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --script crates/minigame/tests/scripts/livewire/d1_seed7.mgs
```

Run the lane in the foreground and read its `status=` line. Scripts that record a new game are written with `--record`, not by hand.

## Games

Every game below is server-validated (`validation()` is `server`), so the victory checks run in the session test as well: `crates/minigame/src/minigame/server/ported_games_tests.rs` sends each game's first golden move over a loopback session and checks that the client's `victory` is refused.

### Livewire

| Script | Command |
|---|---|
| `livewire/d1_seed7.mgs` (seed 7, difficulty 1) | `--script crates/minigame/tests/scripts/livewire/d1_seed7.mgs` |

- Expected: exit 0. The start frame (`fullgamestate`) lists the wires for seed 7, and the script ends with the client's `abort`.
- Human check: the wire list in `fullgamestate` is the seed's, and the transcript ends after the `abort`.

### Alignment

| Script | Command |
|---|---|
| `alignment/a_scoring_swap.mgs` (seed 7) | `--script crates/minigame/tests/scripts/alignment/a_scoring_swap.mgs` |
| `alignment/b_reset.mgs` (seed 7) | `--script crates/minigame/tests/scripts/alignment/b_reset.mgs` |

- `a_scoring_swap`: a power-up swap, two cascade rounds, then `doscore`. Human check: the `feedback` frame reports the power-up value after the swap, and each `cascade` is answered by a `hexmove` frame and then a `bypasshexspinner` frame.
- `b_reset`: `processreset` is refused. Human check: the refusal sends no reply and changes nothing, so the next hint is the first scan-order hint of seed 7 at the full 120 s clock, and the one after it arrives at 110 s.

### CrystalGame

| Script | Command |
|---|---|
| `crystal_game/t1_solved.mgs` (seed 15435, difficulty 1) | `--script crates/minigame/tests/scripts/crystal_game/t1_solved.mgs` |
| `crystal_game/t2_timeout.mgs` (seed 6331, difficulty 3) | `--script crates/minigame/tests/scripts/crystal_game/t2_timeout.mgs` |
| `crystal_game/t3_validation.mgs` (seed 15435, difficulty 1) | `--script crates/minigame/tests/scripts/crystal_game/t3_validation.mgs` |

- `t1_solved`: the script is named for a solved board. Human check: the transcript's `processmove` clicks each answer with `updateslot` frames for the slots they change.
- `t2_timeout`: the script is named for the clock running out. Human check: the `timerupdate` frames count down, and the end is a defeat, not a victory.
- `t3_validation`: the script is named for the server's move checks. Human check: a move the server refuses sends no `updateslot` frame.

### GoauldCrystals

| Script | Command |
|---|---|
| `goauld_crystals/b_move_timeout.mgs` (seed 1, difficulty 2) | `--script crates/minigame/tests/scripts/goauld_crystals/b_move_timeout.mgs` |
| `goauld_crystals/c_path_move.mgs` (seed 1, difficulty 2) | `--script crates/minigame/tests/scripts/goauld_crystals/c_path_move.mgs` |

- `b_move_timeout`: the script is named for a move that times out. Human check: the `timerupdate` frames count down and the game ends without a victory.
- `c_path_move`: two `playfieldclick` moves. Human check: the first click sends `selectedupdate`, and the second sends `powerstatus` and `squarestate` frames for the squares it touches.

### ConverseBasicHumanoid

| Script | Command |
|---|---|
| `converse_humanoid/a_idle_hand.mgs` (seed 7, difficulty 1) | `--script crates/minigame/tests/scripts/converse_humanoid/a_idle_hand.mgs` |
| `converse_humanoid/b_first_deal.mgs` (seed 2, difficulty 1) | `--script crates/minigame/tests/scripts/converse_humanoid/b_first_deal.mgs` |
| `converse_humanoid/c_rejected.mgs` (seed 7, difficulty 1) | `--script crates/minigame/tests/scripts/converse_humanoid/c_rejected.mgs` |

- `a_idle_hand`: the script is named for a player who does not move. Human check: the `tick` lines drive the NPC frames in the transcript.
- `b_first_deal`: the first `playermove` (`num=2 suit=8`). Human check: the `updatecards` frame after it shows the player's `playerTotalPlays` going up.
- `c_rejected`: the script is named for a refused move. Human check: the server's frames after the refused `playermove` are the ones the script records, with no card moved.

## When a run fails

1. Read the first `<` line that differs. The REPL prints the expected and the produced frame.
2. Check the seed and difficulty on the `game` line against the table above. A wrong seed looks like a mismatch on the first frame.
3. A mismatch on a script that passed before is a regression in the game code, not a script problem. Report it with the script name and the lane log path.
