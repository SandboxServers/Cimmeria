# CrystalGame rules spec (MG-R2)

Status: draft for review. Source: AS2 disassembly (R0 tool, `tools/minigame-swf`) of the GFX movie embedded in `CrystalGame.upk`. Static analysis only: nothing here was run in the lab or the client. "Confirmed" below means read directly from the disassembly; "hypothesis" means inferred.

CrystalGame is a 14 by 13 "lights out" puzzle over six colours. Clicking a crystal moves it two colours along the cycle and moves each of its four neighbours one colour along. The board starts all green, the server scrambles it with two random clicks, and the player has 16 seconds to make every crystal green again.

## 1. Verdict and evidence

**Verdict: the embedded extension holds the complete rules, and they are small.** Frame 3 of the movie carries the server-side SFS extension (`onUserJoin`, `init`, `tick`, `handleRequest`, `processMove`, `checkForVictory`, `randomizePlayfield`, `initGame`, `sendFullGameState`); the client half follows it in the same frame. The extension has one fixed configuration, ignores `difficulty` and `seed`, and implements none of the cheats the client offers.

| Item | Value |
|---|---|
| Package | `CookedPC\UI\Flash\CrystalGame.upk`, 1,814,466 bytes |
| GFX | offset 7508, length 94203, SWF version 8 |
| md5 (GFX) | `9c1930c0818222986678cecaf280d581` |
| Functions | 233 (`list_functions.py`); 15 extension functions in root frame 3, 47 client functions after them, `finalCallback` in frame 4 |

Locators. Every claim can be rerun with:

```powershell
$out = "$env:TEMP\sgw-gfx"
python -I tools/minigame-swf/extract_gfx.py <client>\CookedPC\UI\Flash\CrystalGame.upk --out $out
python -I tools/minigame-swf/list_functions.py $out\CrystalGame.gfx
python -I tools/minigame-swf/list_functions.py $out\CrystalGame.gfx --function processMove
```

Extension functions (root, frame 3): `onUserJoin(room, user)`, `init()`, `destroy()`, `reloadZone()`, `tick()`, `handleRequest(cmd, params, user, fromRoom)`, `sendTimerUpdate(room)`, `doRandomMove(room)`, `randRange(min, max)`, `checkForVictory(room)`, `updateSlotMsg(room, x, y, num)`, `processMove(room, x, y, u)`, `randomizePlayfield(room)`, `sendFullGameState(room, userlist)`, `initGame(room)`.

Client functions (root, frame 3) that matter to the server: `onPress`, `onPressReset`, `onResponse`, `updateSlot`, `onVictory`, `onFailure`, `startTimer`, `stopTimer`, `timerCallback`, `setupItemCheats`, `activateItemCheat`, `setupAbilityCheats`, `activateAbilityCheat`, `updateAbilityCheats`, `resetGame`. Frame 2 holds the shared online/offline stub (see [spec-protocol-common.md](spec-protocol-common.md) Q1) and hardcodes `_root.gamename.value = 'CrystalGame'`.

### The `Crystal` object model

**There is no `Crystal` class.** The packet asked for it by name; the movie has no constructor, prototype or `DefineFunction` called `Crystal`. The model is:

- **Server**: one room property `gameArray`, an array of `playfieldWidth` arrays of `playfieldHeight` integers, indexed `gameArray[x][y]`. Each value is a colour id 0-5.
- **Client**: one movie clip per cell, named `Crystal<x>_<y>`, attached from a library symbol chosen by `getCrystalImage(num)` (or `getLitCrystalImage(num)` on hover). The exported sprites are `<Colour>Crystal` and `<Colour>CrystalLit`, plus board decoration (`Edge_*`, `H1`-`H6`), `WinText`, `LoseText` and `DisabledShadow`.

| Id | Colour | Numbers mode label `(v + 3) % 6` |
|---|---|---|
| 0 | Red | 3 |
| 1 | Orange | 4 |
| 2 | Yellow | 5 |
| 3 | Green (solved) | 0 |
| 4 | Blue | 1 |
| 5 | Purple | 2 |

Any other value draws White and traces an error. The numbers-mode checkbox (`onPressCheckbox`) labels each crystal with its distance from green, so a solved board reads all zero.

### Shared core with GoauldCrystals

**No shared rules core. G1 should not slice a shared rules module.** Compared function by function against the GoauldCrystals movie (`GoauldCrystals.upk`, same tool):

- GoauldCrystals has a different and much larger model: `processPlayfieldClick` with a selected-crystal swap (`selectedupdate`), goals (`placeGoals`, `goalIsPowered`, `powerstatus`), `populateGameBoard`, `randomizeCrystals` with an unseeded `Math.random` rng (see [spec-goauldcrystals.md](spec-goauldcrystals.md) section 2; only `seed == 1` on a 4x4 board is special), per-square state (`squarestate`), `processReset`, and real cheat handlers (`processItemCheat`, `processAbilityCheat`, `removeBottomRow`, `removeTopRow`, `removeLeftColumn`, `removeRightColumn`, `addRandomGreenCrystals`).
- CrystalGame has none of those. Its whole rule is `processMove`'s modular increment and `checkForVictory`'s all-green scan; GoauldCrystals has no modular-increment move and no all-green test.
- What they do share is scaffolding every embedded-extension movie has: the frame-2 online/offline stub, the `setInterval(tick, 100)` countdown, `sendTimerUpdate`, the client 50 ms timer and `decimalFormat`, the cheat-button UI pattern, and the crystal art naming. That is generic minigame plumbing (timer, result, cheat inventory) and belongs with the shared session code, not with a "crystal" module.

The nearest thing to reuse is conceptual: CrystalGame's cheat help text (remove the bottom row, remove the right column, turn crystals green) describes what GoauldCrystals' `removeBottomRow`, `removeRightColumn` and `addRandomGreenCrystals` actually do. If a later packet implements CrystalGame cheats (section 7), it can borrow those designs, not code.

## 2. `joinOK` params read

**None.** No extension or client function reads `difficulty`, `seed`, `techcomp`, `pclevel`, `intelligence`, `abilityBitfield`, `instcc` or `CA0`-`CA4` from the login reply; this matches [spec-protocol-common.md](spec-protocol-common.md) Q2. The client takes its cheat inventory from `fullgamestate` instead (section 5). The Rust `joinOK` superset needs no change.

## 3. Board model

- Dimensions: **14 wide by 13 high** (`initGame` puts `playfieldWidth = 14`, `playfieldHeight = 13`). The same for every tier.
- Piece ids: colour 0-5 per cell (section 1 table). 3 is solved.
- Coordinates: `x` 0-13 is the column, `y` 0-12 the row. Client clip `Crystal<x>_<y>`.

### The move (`processMove(room, x, y, u)`)

Confirmed from the disassembly. Pseudocode, cleaned:

```text
g = gameArray; w = playfieldWidth; h = playfieldHeight
g[x][y] = (g[x][y] + 2) % 6;            if u: send updateslot{x, y, num: g[x][y]}
if x > 0:     g[x-1][y] = (g[x-1][y] + 1) % 6;   if u: send updateslot{x-1, y, ...}
if y > 0:     g[x][y-1] = (g[x][y-1] + 1) % 6;   if u: send updateslot{x, y-1, ...}
if x < w - 1: g[x+1][y] = (g[x+1][y] + 1) % 6;   if u: send updateslot{x+1, y, ...}
if y < h - 1: g[x][y+1] = (g[x][y+1] + 1) % 6;   if u: send updateslot{x, y+1, ...}
put gameArray = g
checkForVictory(room)
```

`u` is true for a player click and false for a scramble click, so the scramble sends nothing. A click sends 1 + (number of in-bounds neighbours) `updateslot` messages: 3 in a corner, 4 on an edge, 5 inside, always centre first, then left, up, right, down. The function also reads `timerState` into a local and sets it to 1 when it is 0, but never stores it back: dead code.

The centre step as it appears in the movie (`x` is `r2`, `y` is `r1`, `g` is `r3`):

```text
push r3, r2
getmember
push r1, r3, r2
getmember
push r1
getmember
push 2
add2
setmember
```

The `% 6` (`modulo`) follows in the same shape.

### Initial generation (`initGame`, `randomizePlayfield`, `doRandomMove`)

```text
initGame(room):
  gameInit = true; playfieldWidth = 14; playfieldHeight = 13
  timeRemaining = 16; timerState = 0
  randomizePlayfield(room)
  sendFullGameState(room, allUsers)

randomizePlayfield(room):
  gameArray = w arrays of h cells, every cell 3
  doRandomMove(room); doRandomMove(room)

doRandomMove(room):
  x = randRange(0, w - 1); y = randRange(0, h - 1)
  processMove(room, x, y, false)

randRange(min, max) = floor(Math.random() * (max - min + 1)) + min
```

- The original draws from `Math.random`; it never reads `seed`. The port draws from the session's single `StdRng::seed_from_u64(session.seed)`, in this order: x of the first click, y of the first click, x of the second, y of the second (`rng.gen_range(0..w)`, `rng.gen_range(0..h)`).
- The two clicks may land on the same cell or overlap. A scrambled board is never already solved: two clicks add at most 4 to any cell, and 4 is not a multiple of 6.
- **The client reproduces nothing from `seed`.** It draws exactly what `fullgamestate` and `updateslot` carry.

### Why it is solvable

Clicks commute, and six clicks on one cell are the identity (centre +12, neighbours +6, both 0 mod 6). So undoing one scramble click at (a, b) takes five more clicks at (a, b), and a board scrambled by two clicks is always solvable in at most ten. The player is not told where the scramble clicks were, but the board shows them: the centre of a scramble click is the only cell two steps from green (or the cell summed from two overlaps).

## 4. Difficulty table

**One configuration for every tier.** `initGame` reads no `difficulty`, `techcomp` or level; there is no `CrystalGame.py` in `deprecated/python/base/minigame/` (only `GoauldCrystals.py`), so there is no Python table to compare.

| Tier (`difficulty`) | Width | Height | Scramble clicks | Time (s) | Python |
|---|---|---|---|---|---|
| 1 | 14 | 13 | 2 | 16 | none |
| 2 | 14 | 13 | 2 | 16 | none |
| 3 | 14 | 13 | 2 | 16 | none |
| 4 | 14 | 13 | 2 | 16 | none |
| 5 | 14 | 13 | 2 | 16 | none |

The port keeps the fixed configuration (faithful). A tiered table (more scramble clicks per tier) would be a design addition, not recovery; see section 11.

## 5. Message table

Field types use the Rust `SfsValue` names. All server messages go to every user in the room (`room.getAllUsers()`), spectators included.

### Client to server (`xtReq`, extension `game`)

| Command | Fields | Original handling | Port handling |
|---|---|---|---|
| `processmove` | `x`: Number, `y`: Number | If `timerState == 0`: set it to 1, set `lastTime`, send `timerupdate`. Then `processMove(room, x, y, true)`. No bounds check, no game-over check | Ignore (no reply) unless both are integers with 0 ≤ x < 14, 0 ≤ y < 13 and the game is running (not won, not timed out). Then as the original |
| `processreset` | `x`, `y` (the reset button's own `xPos`/`yPos`, probably undefined) | Silently ignored: `handleRequest` only matches `processmove` | Ignore, no reply (open question 11.3) |
| `activateitemcheat` | `cheatNum`: Number | Silently ignored | Ignore, no reply (section 7) |
| `activateabilitycheat` | `cheatNum`: Number | Silently ignored | Ignore, no reply (section 7) |

### Server to client (`xtRes`)

| `_cmd` | Fields | When |
|---|---|---|
| `fullgamestate` | `playfieldWidth`: Number, `playfieldHeight`: Number, `gameArray`: list of `playfieldWidth` lists of `playfieldHeight` Numbers (`[x][y]`), `timeRemaining`: Number, `timerState`: Number, `instcc`: Number, `CA0`-`CA4`: Number, `abilityBitfield`: Number | On a non-spectator join, after `initGame`, to all users; on a spectator join, to that user only. Only sent once `gameInit` is true |
| `updateslot` | `x`: Number, `y`: Number, `num`: Number (the new colour) | One per changed cell of a player click |
| `timerupdate` | `timeRemaining`: Number, `timerState`: Number | Once, on the first `processmove` (state 0 to 1). Not sent per tick |
| `victory` | none | Every cell is 3 after a click |
| `failure` | none | The countdown reaches 0 |

The client also handles `itemcheatstate`, `abilitycheatstate {abilityBitfield}` and `resetgame`, which this extension never sends.

Original `fullgamestate` cheat fields are hardcoded debug values: `instcc = 1`, `CA0 = 10`, `CA1 = 5`, `CA2 = 2`, `CA3 = 1`, `CA4 = 0`, `abilityBitfield = 32767` (all 15 ability buttons). The port must not copy them: they would show every cheat button, and none of them works. Send the same "no cheats" values the Rust `joinOK` uses (`instcc = -1`, `CA0`-`CA4 = 0`, `abilityBitfield = 0`). With these values the ability bar is empty and the item bar shows five greyed slots labelled 0 that do nothing when pressed (`setupItemCheat`, total 0 branch); that is the intended no-cheat look, confirm in the lab (11.2).

### Spectators

`onUserJoin` sends a spectator only `fullgamestate`. A spectator's `processmove` is not filtered by the original; the port should accept moves only from the playing user.

## 6. Scoring, win and loss

- **Goal**: every cell equals 3 (green). `checkForVictory` scans `x` then `y` and returns at the first cell that is not 3, so it runs after every click, the scramble clicks included (harmless: those can never solve the board).
- **Timer**: 16 seconds, starting on the first player click, not on join. No pause, no score, no move limit.
- **Victory** (original): send `victory {}` to all users, then `__mg.minigameCompleteGame(user0, 30.5, 1, false, false)`. That call has five arguments where every other game passes six (`(user, code, timeRatio, numResets, instrumentsUsed, consumablesUsed)`); hypothesis: a missing comma in `3, 0.5`. It also **does not stop the timer**: `timerState` stays 1, so the original `tick` later sends `failure` and the timeout call as well. Both are original bugs.
- **Timeout** (original `tick`): `timerState = 0`, send `failure {}`, then `minigameCompleteGame(user, 4, 0, 1, false, false)` for each non-spectator.
- **Port**: on victory, stop the timer, send `victory {}` and report extension code 3, which maps to cell result 1 (Victory). On timeout send `failure {}` and report extension code 4, which maps to cell result 2 (Defeat) under the [spec-protocol-common.md](spec-protocol-common.md) Q4 mapping (spec-protocol-common Q4: extension code 4 maps to cell 2; C3 code 4 stays reserved; the same rule applies to [spec-goauldcrystals.md](spec-goauldcrystals.md)). After either, ignore further `processmove`.
- **Client**: `onVictory` and `onFailure` stop the local timer and go to frame 4, which shows `WinText` or `LoseText` and after 2000 ms calls `getURL('FSCommand:closewindow', '')`. The window close drops the socket. Online, the client never declares failure itself at 0 (its `timerCallback` calls `onFailure` only offline), so there is no Livewire-style client/server race.

## 7. Cheats

The client offers both cheat bars; **the extension implements none of them** (section 5). Descriptions are the client's help text.

### Ability cheats (`abilityBitfield`, 15 bits)

The client draws one button per set bit and sends `activateabilitycheat {cheatNum: bit}`.

| Bit | Group | Help text |
|---|---|---|
| 0 | Time | +20 s |
| 1 | Time | -10 s, and remove the bottom row |
| 2 | Time | Stop the timer for 15 s |
| 3 | Time | Timer counts up for 10 s |
| 4 | Playfield | Remove the bottom row |
| 5 | Playfield | Remove the right column |
| 6 | Playfield | For 10 s, a click changes only the clicked crystal, by 1 |
| 7 | Moving Parts | Remove the right column |
| 8 | Moving Parts | As bit 6 |
| 9 | Moving Parts | Replace some non-green crystals with green |
| 10 | Tools | Use the instrument a second time |
| 11 | Tools | Consumables also add 5 s |
| 12 | Obstacles | Replace some crystals with green |
| 13 | Obstacles | As bit 6 |
| 14 | Obstacles | Replace many crystals with green, lose 10 s |

The groups are the client's own labels in each help string: Time bits 0-3, Playfield 4-6, Moving Parts 7-9, Tools 10-11, Obstacles 12-14. The three-per-group layout does not hold; the help text of bit 3 is a Time effect.

### Item cheats (`instcc`, `CA0`-`CA4`)

Five slots; the client sends `activateitemcheat {cheatNum: slot}`. The displayed count is `CA<i> + (instcc >= i ? 1 : 0)`, with a border when `instcc >= i`.

| Slot | Help index | Help text |
|---|---|---|
| 0 | 20 | Add 20 s |
| 1 | 21 | The next crystal clicked turns green |
| 2 | 22 | For 10 s, a click changes only the clicked crystal, by 1 |
| 3 | 23 | Remove the bottom row |
| 4 | 24 | Remove the right column |

**Port policy**: CG1-CG3 send the "no cheats" values and ignore both requests, which is exactly the original's behaviour minus the misleading buttons. Implementing them is design work with no recovered behaviour behind it (11.4).

## 8. Hints and timers the server must run

- **Countdown**: 16 s, idle (state 0) until the first valid `processmove`, then counting (state 1). With the C6 250 ms tick, subtract 0.25 per tick and fail when `timeRemaining <= 0`, which is tick 64 after the first click. The original subtracted real elapsed time every 100 ms; either is within the client's display tolerance because the client counts down on its own after `timerupdate`.
- **On start**: send `timerupdate {timeRemaining: 16, timerState: 1}` before that click's `updateslot`s. The original sends no further `timerupdate`: no cheat changes the rate.
- **Stop**: on victory, set state 0 (the fix for the original bug).
- **Hints**: none. The game has no hint, beep or reveal message.
- `needs_tick()` is true only while the countdown is running.

## 9. Golden transcripts

The two scramble clicks come from `StdRng::seed_from_u64(seed)` in the order x1, y1, x2, y2 (`gen_range(0..14)`, `gen_range(0..13)`), a stream only the Rust port can compute. The seeds below are therefore **placeholders to be resolved by MG-CG1**: its test `tests_setup.rs::finds_seed_for_clicks` searches seeds 0..100000 for the first seed whose scramble clicks are the ones named in T1 and T2 and writes the numbers it finds into the `game CrystalGame difficulty N seed <n>` headers of `crates/minigame/tests/scripts/crystal_game/*.mgs` and into this table:

| Transcript | Seed |
|---|---|
| T1 | `TBD (CG1)` |
| T2 | `TBD (CG1)` |

The clicks can collide (T2 clicks the same cell twice, probability 1/182 per seed), so 100k seeds is ample for both. Grids are given as the non-green cells only; every other cell is 3.

### T1: corner and interior, solved

Scramble clicks (0, 0) and (6, 6). After the scramble: (0,0) = 5, (1,0) = 4, (0,1) = 4; (6,6) = 5, (5,6) = 4, (6,5) = 4, (7,6) = 4, (6,7) = 4.

```text
game CrystalGame difficulty 1 seed <T1 seed>
start
< fullgamestate {CA0:0, CA1:0, CA2:0, CA3:0, CA4:0, abilityBitfield:0, gameArray:[...], instcc:-1, playfieldHeight:13, playfieldWidth:14, timeRemaining:16, timerState:0}
> processmove x=0 y=0
< timerupdate {timeRemaining:16, timerState:1}
< updateslot {num:1, x:0, y:0}
< updateslot {num:5, x:1, y:0}
< updateslot {num:5, x:0, y:1}
```

Four more clicks on (0, 0) send, per click, the centre then (1,0) then (0,1): `num` 3/0/0, 5/1/1, 1/2/2, then 3/3/3. Five clicks on (6, 6) follow, each sending centre, (5,6), (6,5), (7,6), (6,7): centre 1, 3, 5, 1, 3 and neighbours 5, 0, 1, 2, 3. After the last click's fifth `updateslot`:

```text
< victory {}
```

No `failure` follows, however many ticks pass (the port stops the timer; the original did not). Ten clicks in total.

### T2: edge click, timeout

Scramble clicks (13, 5) and (13, 5): the same cell twice. After the scramble: (13,5) = 1, (12,5) = 5, (13,4) = 5, (13,6) = 5.

```text
game CrystalGame difficulty 3 seed <T2 seed>
start
< fullgamestate {..., timeRemaining:16, timerState:0}
tick 100
> processmove x=13 y=5
< timerupdate {timeRemaining:16, timerState:1}
< updateslot {num:3, x:13, y:5}
< updateslot {num:0, x:12, y:5}
< updateslot {num:0, x:13, y:4}
< updateslot {num:0, x:13, y:6}
tick 63
tick 1
< failure {}
```

The first `tick 100` proves the timer does not run before the first click. (13, 5) is on the right edge: there is no x + 1 neighbour, so the order is centre, left, up, down. Note the centre is green after one click while the neighbours are not: the scramble used the same cell twice, so four more clicks were needed. After `failure`, a further `> processmove x=13 y=5` produces no output.

### T3: validation (port only)

`> processmove x=14 y=0`, `> processmove x=-1 y=0`, `> processmove x=3`, `> processreset`, `> activateitemcheat cheatNum=0` and `> activateabilitycheat cheatNum=0`, each sent before the first valid click, produce no output and do not start the timer.

## 10. Port slicing

Module layout per C4: `crates/minigame/src/minigame/games/crystal_game/`. Line estimates are code plus doc comments.

| Slice | Files | Content | Lines |
|---|---|---|---|
| CG1 setup | `setup.rs` (~110), `mod.rs` (~110), `tests_setup.rs` (~130) | `Board { w: 14, h: 13, cells: Vec<u8> }` indexed `[x][y]`; `press(x, y) -> Vec<(x, y, num)>` (the pure move, returning changed cells in the original order; shared with CG2); `scramble(rng)` drawing x, y, x, y; `fullgamestate` builder with `SfsValue::list` of lists and the "no cheats" fields; `CrystalGame` implementing `MinigameInstance` with `started()` emitting `fullgamestate`. Tests: dimensions, all-3 then two presses, draw order against a fixed seed, never-solved after scramble, list shape `[x][y]`, seed search for the T1 and T2 scramble clicks (`finds_seed_for_clicks`) | ~350 |
| CG2 moves | `moves.rs` (~100), `mod.rs` (+30), `tests_moves.rs` (~150) | `processmove` parsing and validation (integers, bounds, game running, playing user only); `updateslot` emission in centre, left, up, right, down order; corner, edge and interior counts (3/4/5); silently ignore `processreset` and both cheat commands. Tests: per-position message counts and order, `(v+2)%6` and `(v+1)%6` wrap, six presses are the identity, out-of-bounds and malformed input ignored | ~280 |
| CG3 rules | `rules.rs` (~120), `mod.rs` (+40), `tests_rules.rs` (~160), goldens T1-T3 as `.mgs` | Countdown state machine (0 idle, 1 counting), `timerupdate` on first click, `tick` at 250 ms, `failure` at 0; `checkForVictory` with the timer stopped; result codes (extension 3 to cell 1, 4 to cell 2); moves ignored after the end; `needs_tick`; flip `PORTED = true`. Tests: T1-T3 transcripts, no timer before first click, no `failure` after `victory` (fails if the stop is reverted), tick-64 timeout | ~320 |

Each slice fits about 40k working tokens. CG2 depends on CG1's `press`; CG3 on CG2's handler and on MG-05 / MG-22 for the result path.

## 11. Open questions

1. **Victory call arguments.** `(user0, 30.5, 1, false, false)` is read here as a typo for `(user, 3, 0.5, 1, false, false)`. The port does not need the answer (it reports victory directly), but a host-call-compatible port would. Unresolvable from the movie alone.
2. **"No cheats" display.** `abilityBitfield = 0` draws no ability buttons (read from `setupAbilityCheats`). The item bar always draws five slots; with count 0 they are the greyed `CGCheat<i>0` clips with quantity 0 and no click handler. Does a lab look show that acceptable, or does a real value need `fullgamestate` to omit `CA0-4`? One lab look settles it.
3. **Reset.** The client has a reset button that sends `processreset`, which the original ignores, so the button does nothing. Faithful is "ignore"; a server-authoritative improvement would rescramble (new `fullgamestate` or `resetgame`) and count `numResets`. Needs an advisor call before CG2.
4. **Cheats.** The client UI for 15 ability and 5 item cheats exists with help text, but no recovered behaviour. Implementing them is a design decision (D-MG series), not part of CG1-CG3.
5. **Difficulty.** The original is one fixed puzzle (two scramble clicks, 16 s) at every tier. Whether to add a tier table (for example more scramble clicks at higher tiers) is a design decision; this spec recommends faithful for the port.
6. **Spectator clicks.** The original accepts `processmove` from anyone in the room. The port restricts it to the player; confirm with the minigame advisor that no co-op mode relied on it.
