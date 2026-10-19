# Livewire parity audit (MG-R6)

Status: draft for review. Compares the original server-side extension embedded in `Livewire.upk` (offset 11232, 410721 bytes, md5 7fbe1510a58b120c7776f4b6d78d62f2; see [spec-protocol-common.md](spec-protocol-common.md) for the evidence table) with `crates/minigame/src/minigame/games/livewire/setup.rs` and `mod.rs`.

Original line numbers refer to a scratch pseudo-decompile outside the repo (`Livewire 6802` and so on); they are locators only. Rust line numbers are in the repo at the time of this audit. Each delta ends with a proposed change for MG-LW1.

## Original difficulty table (`setGameType`, Livewire 6802-6870)

`tc` is techcomp. Difficulty 1-4:

| Item | D1 | D2 | D3 | D4 |
|---|---|---|---|---|
| countdown (s) | 20 | 17 | 15 | 12 |
| timeRemaining | countdown + 0.25 | same | same | same |
| countdownRate | 1/countdown | same | same | same |
| paramTimer | floor(30+tc*0.588235) | floor(20+tc*0.588235) | floor(20+tc*0.392157) | floor(12+tc*0.372549) |
| playfield wires | floor(6+tc*0.0755) | floor(6+tc*0.0943) | floor(6+tc*0.0943) | floor(6+tc*0.1509) |
| goals | 2 | 4 | 4 | 6 |
| moving wires | floor(2+tc*0.1887) | floor(2+tc*0.1887) | floor(3+tc*0.2453) | floor(5+tc*0.3208) |
| movTimer | tc/2.5 | tc/2 | tc/1.5 | 0 (unset) |
| obstacleTotal | floor(2+tc*0.1887)*2 | same | floor(3+tc*0.2453)*2 | same as D3 |
| obsTimer | 5+5*L | 6+5*L | 7+5*L | 9+5*L |

`L = minigamelevel = floor((tc-1)/5)`. Difficulty 5 or any other value has no branch: every field stays 0. `modNum = max(4, 2+difficulty)`. The first RNG draw is `randRange(rng,1,3)` for the timer method.

`getTime(method, tc, paramTimer, obsTimer, goalTotal, pclevel)`: method 1 = `(paramTimer*tc)/1.7`; method 2 = `(17 + tc*4.5) ^ 1.25` where AS `^` is bitwise XOR, not a power; method 3 = `6*((obsTimer/10)*goalTotal) + (21+pclevel)`.

## Wire-variant range and library names (`setupWires`, Livewire 6542-6740)

- Playfield and moving wires: lib is a base name plus `randRange(1,4)` (1 to 4 inclusive), for example `pGray3`.
- Goal wires `gX` and obstacle wires `oX`: also base plus `randRange(1,4)`.
- Electrodes: no suffix.
- SWF exports (ExportAssets) variants 1-4 for pGray, pRust, pBronze, pGreen, mYellowStripe, mPurpleStripe, mOrangeStripe, mBlackStripe, gGreenBlack, gRedSilver, gBlackGreen, oRed, oGreen, oBlack. Also pOrangeSilver 1-2, pPurpleBlack 1-2, mMicroChip 1-2, pProcessorFan 1. LED libs and mGear have no suffix. A bare base name does not exist in the movie, so `attachMovie` fails and the wire is invisible.
- Depth: playfield and moving wires `randRange(1,100)`; goals `twoRandRange(1,25,60,85)`; playfield electrodes 30% of the count with depths from 101 up. Electrode x/y are floored (x 375-900, y 250-700). Glow point is `attach1..5` from y in {391, 497, 577, 657, 783}.
- The wire list is in generation order (pe, p, g, me, m, o).

## Deltas

| # | File:line | Rust | Original | Proposed MG-LW1 change |
|---|---|---|---|---|
| 1 | setup.rs:240, 306 | playfield and moving wire lib sent without a suffix | base + `randRange(1,4)` | Append a 1-4 suffix. Highest visible impact: these wires do not render today. |
| 2 | setup.rs:261, 327 | goal and obstacle variant `random_range(1..4)` gives 1-3 | 1-4 inclusive | Use `1..=4`. |
| 3 | setup.rs:259 | goal depth `find_random_depth(1,100)` | two bands 1-25 and 60-85 | Port `twoRandRange(1,25,60,85)` (two `nextInt` draws then `nextInt(2)` to choose the band). |
| 4 | setup.rs:221-222, 287-288 | electrode x/y not floored | floored integers | Floor both. |
| 5 | setup.rs:150-152 | obstacleTotal missing the `*2` | `floor(...)*2` | Multiply by 2. |
| 6 | setup.rs:152 | obstacle_timer uses `tc/5.0` | `floor((tc-1)/5)` in `obsTimer = base + 5*L` | Replace with `base + 5*floor((tc-1)/5)`, base 5/6/7/9. |
| 7 | setup.rs:88 | `move_timer_divider` 1.0 for D4 | 0 (unset) | Use 0 so D4 moving wires do not change the rate. |
| 8 | setup.rs:160-161 | method 0 uses `param_timer + tc/1.7` | `(paramTimer*tc)/1.7` | Multiply, not add. |
| 9 | setup.rs:160-161 | method 1 uses `17 + tc*4.5^1.25` | `(17 + tc*4.5) ^ 1.25` as bitwise XOR | Decide per D-MG2-style call: keep the original XOR result for parity, or keep a sane power. The original behaviour is XOR on truncated ints; record the choice in a comment. |
| 10 | setup.rs:140 | `.min(3)` clamp for D5 | no D5 branch, all zero | Deliberate deviation; keep, but note that content difficulty 1-5 maps 5 to 4's table. |
| 11 | mod.rs:371 | `activateitemcheat` and `activateabilitycheat` do nothing | cheat handlers, timer states 2 and 3, `cuttime`, `addtime`, `reverttime`, `flash`, `converttoseconds`, `itemcheatstate`, `abilitycheatstate` | Out of scope for the first fix; either implement or keep ignoring while `instcc`/`CA` stay hardcoded (delta 12). |
| 12 | mod.rs:113-114 | instcc -1 and CA0..4 = 0 hardcoded | read from user vars; item cheats consume CA counts and validate via `__mg.minigameValidateItem` | Keep hardcoded until a cheat source exists. `tech_competency` is also hardcoded 1 (see chain wiring memory). |
| 13 | mod.rs:118, 357-363 | `consUsed` and `origInstcc` not tracked | reported in the victory host call | Track both when delta 11 lands. |
| 14 | mod.rs:433-439 | `aborted()` sends a `failure` command | The original has no abort message; the client drops the socket (see common spec Q4) | Do not send `failure` on abort; report Canceled(0). Closing the window is a socket drop and the client is gone. |
| 15 | setup.rs:346-366 | wires sorted by depth | generation order | Informational: the client attaches by explicit depth. Leave. |
| 16 | setup.rs:408 | `timerState` in `fullgamestate` is the string `"0.0"` | the live numeric state (0-3) | Send an integer; `1` while counting. |
| 17 | mod.rs fullgamestate | extra `abilityBitfield`, `instcc`, `CA0..4`, `gameReadOut` | original sends the same user fields and `gameReadOut` | No delta; fields match. |
| 18 | mod.rs processmove | not verified against the original effects | goal: +1 and destroy; `p`: ignored unless `playfieldActive`; `o`: rate += rate*obsTimer/100; `m`: rate += rate*movTimer/100; each sends `countdownupdate`, then `checkForVictory` | Verify Rust applies rate changes and `countdownupdate`; if not, add (client display depends on it, see common spec Q5). |

## Cheat handling summary

Original: item cheats are `activateitemcheat{cheatNum}`. If `cheatNum <= instcc` the cheat is applied free and `instcc` is set to -1; else if `CA<n> > 0` the counts are decremented and `itemcheatstate` is sent (online the host validates through `minigameValidateItem`). Ability cheats are bits of `abilityBitfield`, cleared on use, answered with `abilitycheatstate`. Cheat 0 adds 2 s and edits the highest-depth obstacle wire. Cheats are driven by user vars the game server sets from the player's real items, so they are server-validated in the original; the client only requests.

Rust: both commands are no-ops, which is safe from a server-authority view (a client cannot claim a cheat). No delta that weakens authority.

## Result

Deltas 1, 2 and 14 are the ones with player-visible impact today. Deltas 3-8 and 16 are board and timing fidelity. Deltas 9-13 are cheat-related or judgement calls. There is no server-authority regression: Rust validates moves against its own board, and the original did the same.
