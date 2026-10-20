# GoauldCrystals rules spec (MG-R3)

Status: draft for review. Source: the SmartFoxServer 1.x AS1 extension embedded in `GoauldCrystals.gfx`, read with the MG-R0 disassembler (`tools/minigame-swf/as2dis.py`) and a scratch stack decompiler kept outside the repo. Nothing here was run in the lab or against a live client. Offsets cited as `@NNNNN` are byte positions in the output of a scratch decompiler that is not committed; every function named here can be re-read with `python -I tools/minigame-swf/list_functions.py <GoauldCrystals.gfx> --function <name>`, which is the evidence of record.

## 1. Verdict

GoauldCrystals is a real game: a path-building puzzle. The player shuffles coloured crystals until a chain of green crystals connects a start goal on one edge of the board to a finish goal on another edge, before a countdown runs out.

| Field | Value |
|---|---|
| Package | `CookedPC\UI\Flash\GoauldCrystals.upk` (2,356,155 bytes) |
| GFX offset, length | 1637, 154559 |
| GFX md5 | `81c70b6d28b47455c3023ef0a9010ddb` |
| SWF version, functions | 8, 248 |

The movie has the usual layout ([client-packages.md](client-packages.md), [spec-protocol-common.md](spec-protocol-common.md)): sprites 301-310 are the SmartFox AS2 client library, frame 2 holds the offline stubs (`_server`, `zone`, `room`, `user`, whose `sendResponse` ignores its recipients and calls `_root.onResponse(msg,'xml')`), and frame 3 holds the extension and the client UI.

Extension functions (frame 3): `onUserJoin`, `init`, `destroy`, `reloadZone`, `tick`, `handleRequest`, `triggerSound`, `processPlayfieldClick`, `sendSelectedUpdate`, `activateItemCheatRequest`, `processItemCheat`, `sendAbilityCheatInfo`, `activateAbilityCheatRequest`, `processAbilityCheat`, `removeBottomRow`, `removeTopRow`, `removeLeftColumn`, `removeRightColumn`, `addRandomGreenCrystals`, `processReset`, `sendTimerUpdate`, `sendVictoryMsgs`, `sendFullGameState`, `sendUpdatedSquareState`, `initGame`, `nextInt` (anonymous, assigned to the rng object), `populateGameBoard`, `randomizeCrystals`, `updatePowerStatus`, `goalIsPowered`, `placeGoals`.

Client functions (frame 3): `onLocPress`, `setupPlayfieldSize`, `positionObject`, `onGameArrayUpdate`, `updateSquare`, `setupGoal`, `onVictory`, `onFailure`, `onResponse`, timer helpers, item and ability cheat UI, `requestReset`, help panels, `updatePowerArray`.

### Names the packet asked for

- `handleRequest` exists (@3434) and is covered in section 5.
- **There is no `Swap` and no `Crystal` function or class.** The swap is two inline blocks in `processPlayfieldClick` (path move @8340, adjacent swap @8720). A "crystal" is the per-cell object `{color, var, power}` built in `randomizeCrystals` (section 3).
- **There is no `checkForVictory`.** The victory test is in `updatePowerStatus` (@36786) and again in `sendFullGameState` (@24418): the game is won when the finish goal's cell has `power == true`. Section 6 has the excerpt.

### Shared core with CrystalGame: no

CrystalGame (`CrystalGame.gfx`, 233 functions) is a different game, and the two share no rules code:

| | GoauldCrystals | CrystalGame |
|---|---|---|
| Rule | connect two edge goals with a green path | "lights out" mod 6: a click adds 2 to the cell and 1 to each 4-neighbour; win when every cell is 3 |
| Board | `gameArray[x][y] = {color, var, power}`, 4x4 to 14x12 by tier | `gameArray[x][y]` ints 0-5, fixed 14x13 |
| Generation | random goals, then random crystal counts | all cells 3, then 2 random moves |
| Client sends | `playfieldclick`, cheats, `processreset` | `processmove`, cheats, `processreset` |
| Server sends | `squarestate`, `powerstatus`, `selectedupdate`, ... | `updateslot {x,y,num}`, ... |
| Victory call | `(user, 3, timeRemaining/maxTime, numResets+1, instrumentsUsed, consUsed)` | `(user, 30.5, 1, false, false)` (5 args, no code) |

They share only the SFS scaffolding every movie carries: `tick` with `setInterval(100)`, `sendTimerUpdate`, `triggerSound`, the frame-2 offline stubs. Rust already has that scaffolding in `crates/minigame/src/minigame/protocol.rs` and the session loop. **Recommendation: no `games/crystals_core/` module; MG-CG and MG-GC port independently.**

## 2. `joinOK` params read

None. The embedded `initGame` (@24678) hardcodes its inputs as locals before the difficulty switch:

| Local | Value | Meaning |
|---|---|---|
| r16 | 1 | tech competency |
| r17 | 2 | difficulty |
| r19 | 1 | seed |
| r22 | 30 | unused (probably player level) |
| r1, r6 | 1.4, 0.8 | width/height and time multipliers |

The defaults before the switch are width 7, height 7, time 60, obstacles 0. On the first join the extension also writes test values into the user: `abilityBitfield=1023`, `instcc=0`, `origInstcc=0`, `origAbilityBitfield=1023`, `CA0..CA4 = 5,4,3,2,1`.

This is the offline test build, so the hardcoded values are a developer's test bench, not a protocol. **The port takes `difficulty`, `techcomp`, `seed`, `instcc`, `CA0..CA4` and `abilityBitfield` from the session**, the same values Rust already sends in `joinOK` (`server/session_loop.rs`), so the client and the server agree on the cheat inventory shown in `fullgamestate`.

The client reads nothing from `joinOK` either: everything it draws comes from `fullgamestate`.

### Correction to spec-protocol-common Q3

[spec-protocol-common.md](spec-protocol-common.md) Q3 says GoauldCrystals' extension seeds a `java.util.Random` from `seed`. It does not. `initGame` builds its own rng object whose `nextInt(maxInt)` is `Math.floor(Math.random() * maxInt)`, unseeded. `seed` is used for exactly one thing: `seed == 1` on a 4x4 board selects the fixed tutorial board and goals (section 3). No `java.util.Random` port is needed for this game. (Not edited here: that file belongs to MG-R6.)

## 3. Board model

`gameArray[x][y]`, `x` in `0..playfieldWidth`, `y` in `0..playfieldHeight`. Each cell is an object:

| Field | Type | Values |
|---|---|---|
| `color` | Number | 0 empty, 1 green, 2 yellow, 3 orange, 4 red |
| `var` | Number | 0..2, visual variant only; no rule reads it |
| `power` | Bool | true when the cell is on the powered green path |

Colours as the rules treat them:

- **Green (1)** conducts power and moves through empty cells.
- **Yellow (2)** can only swap with an adjacent cell of colour 2 or less; a yellow that lands on a new cell after a swap turns orange.
- **Orange (3)** and **red (4)** never move without a cheat.
- **Empty (0)** is where greens travel.

Goals sit on the board edge. Side 0 is top (`x=offset, y=0`), 1 right (`x=w-1, y=offset`), 2 bottom (`x=offset, y=h-1`), 3 left (`x=0, y=offset`). Goal cells are ordinary cells; they are not reserved.

### Dimensions

The dimensions come from the difficulty table (section 4) and run from 4x4 to 14x12. The client's board holds at most 14x12 (`setupPlayfieldSize`: 14 columns for an even width, 13 for odd; 12 rows for an even height, 11 for odd), which is exactly the largest tier.

### Generation

```text
initGame(room, user, firstTime):
  user.consUsed = false; user.consCheatBonus = false
  (w, h, time, obstacles) = difficulty table            # section 4
  room: gameInit=true, playfieldWidth=w, playfieldHeight=h, obstacleNumber=obstacles,
        timeRemaining=maxTime=time, timerState=0, selected=(-1,-1), seed, rng
  if firstTime: user.abilityBitfield, instcc, origInstcc, origAbilityBitfield, CA0..CA4 = session values
  else:         user.abilityBitfield = origAbilityBitfield; user.instcc = origInstcc   # CA is NOT restored
  room: activeItemCheat=-1, yellowsWithoutOrange=0, redsToYellows=0, redsToRemove=0,
        itemCheatsAddTime=false, redsToRevert=undefined
  populateGameBoard(); sendFullGameState()

populateGameBoard:
  placeGoals()                                           # once
  repeat: randomizeCrystals(); updatePowerStatus(send=false)
  until finish goal is not powered                       # never hand out a solved board

placeGoals:
  if seed == 1 and w == 4 and h == 4:
    start = side 3, offset 0 -> (0,0); finish = side 1, offset 2 -> (3,2); return
  minDist = max(w, h) - 1
  start: side = nextInt(4); offset = nextInt(w) for sides 0/2 else nextInt(h)
  up to 100 times: draw finish the same way; stop when euclid(start, finish) >= minDist
  (after 100 misses it traces a warning and keeps the last draw)

randomizeCrystals:
  every cell = {color 0, var 0, power false}
  if seed == 1 and w == 4 and h == 4: fixed tutorial board (below), var 0 everywhere; return
  greens  = |sx-fx| + |sy-fy| + 3
  yellows = (w + h) * 0.5 + 1              # may be fractional
  oranges = 0
  reds    = w * h * 0.25 + obstacles       # may be fractional
  while w*h - h < greens+yellows+oranges+reds:
    if reds > 0: reds -= 1
    if still over and yellows > 0: yellows -= 1
    if still over and oranges > 0: oranges -= 1
  for colour in green, yellow, orange, red:
    while count > 0: x = nextInt(w); y = nextInt(h)
                     if cell empty: color = colour; var = nextInt(3); count -= 1
```

A fractional count places `ceil(count)` crystals, because the loop runs while the count is above 0. The capacity loop keeps at least `h` cells empty in the budget; it never touches greens, and greens can never exceed the budget on any tier (at most `w + h + 1`).

Tutorial board (`seed == 1`, 4x4; rows are `y`, columns are `x`):

| y \ x | 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| 0 | 1 start | 4 | 2 | 2 |
| 1 | 1 | 1 | 1 | 1 |
| 2 | 0 | 0 | 0 | 4 finish |
| 3 | 0 | 0 | 0 | 1 |

The finish cell is red and reds never move, so the tutorial board cannot be won without a cheat; it teaches the Crystal Extractor (item cheat 0, section 7).

### Power

`updatePowerStatus(room, send)` (@34892-36896) clears every `power` flag. If the start goal cell is green, it runs a breadth-first search from it through 4-neighbours in the order `x-1, y-1, x+1, y+1`, following only colour 1, and sets `power = true` on each visited cell. The visited list, in visit order, is the `powerArray`.

### Seed and the client

The client reproduces nothing from `seed`; it draws whatever `fullgamestate`, `squarestate` and `powerstatus` say. The port draws everything from one `StdRng::seed_from_u64(session.seed as u64)` (C4), which makes REPL transcripts reproducible. It keeps the `seed == 1` tutorial switch so the seeded goldens in section 9 can pin a known board (see open question 1).

## 4. Difficulty table

`L = floor((techcomp - 1) / 5)`. The extension stores base values and multiplies: `w = floor(base_w * 1.4)`, `h = floor(base_h * 1.4)`, `time = floor(base_t * 0.8)` seconds; obstacles are not scaled. The final values below are computed in IEEE doubles, as AS1 does.

Final `w x h, time s, obstacles` (base values in brackets):

| L (techcomp) | Difficulty 1 | Difficulty 2 | Difficulty 3 | Difficulty 4 |
|---|---|---|---|---|
| 0 (1-5) | 4x4, 24, 2 [3,3,30] | 4x4, 16, 2 [3,3,20] | 5x5, 16, 3 [4,4,20] | 7x5, 16, 5 [5,4,20] |
| 1 (6-10) | 5x5, 16, 3 [4,4,20] | 5x5, 16, 3 [4,4,20] | 7x5, 16, 4 [5,4,20] | 7x7, 16, 6 [5,5,20] |
| 2 (11-15) | 7x5, 24, 4 [5,4,30] | 7x5, 20, 4 [5,4,25] | 7x7, 18, 6 [5,5,23] | 8x7, 20, 8 [6,5,25] |
| 3 (16-20) | 7x7, 28, 5 [5,5,35] | 7x7, 24, 5 [5,5,30] | 8x7, 20, 7 [6,5,26] | 8x8, 24, 9 [6,6,30] |
| 4 (21-25) | 8x7, 32, 6 [6,5,40] | 8x7, 25, 6 [6,5,32] | 8x8, 22, 8 [6,6,28] | 9x8, 25, 11 [7,6,32] |
| 5 (26-30) | 8x8, 36, 7 [6,6,45] | 8x8, 28, 7 [6,6,35] | 9x8, 24, 10 [7,6,30] | 9x9, 28, 12 [7,7,35] |
| 6 (31-35) | 9x8, 40, 8 [7,6,50] | 9x8, 29, 8 [7,6,37] | 11x8, 25, 11 [8,6,32] | 11x9, 29, 14 [8,7,37] |
| 7 (36-40) | 11x8, 44, 9 [8,6,55] | 11x8, 32, 9 [8,6,40] | 9x9, 27, 12 [7,7,34] | 11x11, 32, 16 [8,8,40] |
| 8 (41-45) | 9x9, 48, 10 [7,7,60] | 9x9, 33, 10 [7,7,42] | 11x9, 28, 14 [8,7,36] | 12x11, 33, 18 [9,8,42] |
| 9 (46-50) | 11x9, 48, 11 [8,7,60] | 11x9, 36, 11 [8,7,45] | 11x9, 30, 15 [8,7,38] | 12x12, 36, 20 [9,9,45] |
| 10 (51-55) | 11x11, 48, 12 [8,8,60] | 11x11, 40, 12 [8,8,50] | 12x11, 32, 16 [9,8,40] | 14x12, 40, 22 [10,9,50] |

Outside the table (`L > 10`, so techcomp above 55, or a difficulty outside 1-4) the switch falls through to the defaults: 7x7, 60 s, 0 obstacles, with no multiplier applied. The port should clamp `L` to 10 instead and log the clamp (open question 2).

Quirks worth keeping: difficulty 3 at L=7 is smaller than at L=6 (9x9 against 11x8), and difficulty 1 at L=0 gives more time than difficulty 2 (24 s against 16 s). They are in the extension's numbers, not decoder errors.

### Python cross-check

`deprecated/python/base/minigame/GoauldCrystals.py` has no GoauldCrystals rules. It is a copy of the Livewire handler with a match-3 table pasted on top:

| Python | Extension |
|---|---|
| `difficultyLevels` 1-4: `goalIcons`, `goalTotals`, `timeRemaining` 120/120/100/80, `playfieldSize` 8x8/8x8/9x9/10x10 (lines 52-127) | 11 techcomp bands x 4 difficulties, 4x4 to 14x12, 16-48 s (table above) |
| `updatedDifficulty` reads `countdownRate` and other keys the table does not have (lines 161-177) | n/a |
| handlers `opendoor`, `closedoor`, `processover`, `processout`, `processmove` (Livewire's) | `playfieldclick`, `activateitemcheat`, `activateabilitycheat`, `processreset` |
| `fullgamestate.gameArray` = 64 flat `randrange(1,8)` ints (line 247) | 2-D array of `{color,var,power}` objects |
| `tickRate = 0.25` | `setInterval(..., 100)`, time measured with `getTimer()` deltas |

None of the Python's numbers match the extension, and its messages would not drive this client. It only shows that the legacy server never implemented the game. Do not port from it.

## 5. Message table

Client to server goes through `sendXtMessage('game', cmd, obj)`. `handleRequest` (@3434) ignores spectators, then dispatches on `cmd`. **The extension validates nothing**: no bounds check on `x`, `y`, no type check, no check that a cheat is unlocked in the session, nothing about the board state. The port adds the validation in the "Port validation" column; an invalid request is dropped with a WARN log and no reply. (Rule: never trust the client.)

### Client to server

| `cmd` | Fields (`SfsValue`) | Extension handler | Port validation |
|---|---|---|---|
| `playfieldclick` | `x` Number, `y` Number | `processPlayfieldClick(room, x, y)` | integers, `0 <= x < w`, `0 <= y < h`; game not finished |
| `activateitemcheat` | `cheatNum` Number | `activateItemCheatRequest` | integer 0-4; game not finished |
| `activateabilitycheat` | `cheatNum` Number | `activateAbilityCheatRequest` | integer 0-14; game not finished |
| `processreset` | none | `processReset` | game not finished |

The client builds `playfieldclick` in `onLocPress` from the clicked square's index minus the board's centring offset, so `x`, `y` are board coordinates.

### Server to client

All are broadcast to every user in the room (`room.getAllUsers()`); one player per room in practice.

| `_cmd` | Fields (`SfsValue`) | Sent by | Client handler |
|---|---|---|---|
| `fullgamestate` | `playfieldWidth`, `playfieldHeight`, `startGoalSide`, `startGoalOffset`, `startGoalX`, `startGoalY`, `finishGoalSide`, `finishGoalOffset`, `finishGoalX`, `finishGoalY`, `selectedXPos`, `selectedYPos`, `timeRemaining`, `maxTime`, `timerState`, `activeItemCheat`, `instcc`, `CA0`..`CA4`, `abilityBitfield` (all Number); `gameArray` (list of columns, each a list of cell objects `{color Number, var Number, power Bool}`) | `sendFullGameState` | full redraw, goals, timer, cheat bars |
| `squarestate` | `xPos` Number, `yPos` Number, `info` cell object | `sendUpdatedSquareState` | `updateSquare` |
| `powerstatus` | `powerArray`: list of `{xPos Number, yPos Number}` | `updatePowerStatus(send=true)` when not won | `updatePowerArray` |
| `selectedupdate` | `xPos` Number, `yPos` Number (-1,-1 = none) | `processPlayfieldClick`, `sendSelectedUpdate` | `onSelectedUpdate` |
| `timerupdate` | `timeRemaining` Number (seconds, fractional), `timerState` Number | `sendTimerUpdate` | `onTimerUpdate` |
| `triggersound` | `soundName` String | `triggerSound` | plays the sound |
| `abilitycheatstate` | `abilityBitfield` Number | `sendAbilityCheatInfo` | `updateAbilityCheats` |
| `resetgame` | none | `processReset` | **no handler**; ignored (a `fullgamestate` follows) |
| `victory` | none | `sendVictoryMsgs` | `onVictory` |
| `failure` | none | `tick` | `onFailure` |

`fullgamestate` without a non-spectator in the room sends `instcc=-1`, `CA0..CA4=0`, `abilityBitfield=0`. With `instcc`, `CA0..4` and `abilityBitfield` the port sends the session values (section 2). `timerState`: 0 stopped (not started), 1 running, 2 paused, 3 running backwards.

`processreset` in full: send `resetgame {}` to every user (the client has no handler), then `numResets += 1`, then `initGame(firstTime = false)`: new board, new goals, `timerState = 0`, `timeRemaining = maxTime`, cheat inventory restored as in section 3 (`CA` not restored). The port does the same; `numResets` starts at 0 and is reported as `numResets + 1` in the completion call.

Sound names: `select`, `action`, `invalid`, and the cheat sounds `crystalMG/time/chng`, `crystalMG/time/xchng`, `crystalMG/time/stop`, `crystalMG/time/alter`, `crystalMG/plyfld/addremSpc`, `crystalMG/plyfld/addremCont`, `crystalMG/plyfld/alter`, `crystalMG/mvprts/addrem`, `crystalMG/mvprts/alter`, `crystalMG/mvprts/xchng`, `crystalMG/tools/alter`, `crystalMG/tools/apply`, `crystalMG/obst/addrem`, `crystalMG/obst/alter`, `crystalMG/obst/xchng`.

### `playfieldclick` in full

`processPlayfieldClick` (@3812-10091), in order:

1. If `timerState == 0`: set it to 1, set `lastTime`, send `timerupdate`. The first click starts the clock.
2. **Armed item cheat.** If `activeItemCheat` matches the clicked cell, apply it, set `activeItemCheat = -1` and send `squarestate` for the cell:
   - mode 0 and the cell is red: it becomes empty. If nothing is selected, play `action` and return; otherwise fall through to step 5 with the cell now empty.
   - mode 1 and orange: becomes yellow; `action`; return.
   - mode 3 and empty: becomes green with `var = nextInt(3)`; `updatePowerStatus(send=true)` before the `squarestate`; `action`; return.
   - mode 4 and colour 2 or more: becomes green; `updatePowerStatus(send=true)` first; `action`; return.
   - A cell of the wrong colour leaves the mode armed and the click carries on.
3. If `redsToYellows > 0` and the cell is red: decrement, cell becomes yellow, `squarestate`. Carry on.
4. If `redsToRemove > 0` and the cell is red: decrement, cell becomes empty, `squarestate`. Carry on.
5. **Nothing selected:** select this cell (any colour), send `selectedupdate {xPos, yPos}`, play `select`, return.
6. **Path move** when (destination green and selection empty) or (destination empty and selection green) or mode 2 is armed:
   - Mode 2 clears `activeItemCheat` and swaps the two cells at once.
   - Otherwise breadth-first search from the selection over 4-neighbours in the order `x-1, y-1, x+1, y+1`. Each in-bounds neighbour is first compared with the destination (a match ends the search, whatever its colour), then queued if it is empty and unvisited.
   - On success: swap the two cell objects, `updatePowerStatus(send=true)`, `squarestate` for the destination then the selection, play `action`.
   - A failed search makes no move. The adjacent swap in step 7 is **not** tried in path mode.
7. **Adjacent swap** otherwise, when the Manhattan distance is 1 and both colours are 2 or less: swap; for each of the two cells that now holds a yellow, decrement `yellowsWithoutOrange` if it is above 0, else turn it orange; `updatePowerStatus(send=true)`; `squarestate` destination then selection; play `action`.
8. Clear the selection, send `selectedupdate {-1,-1}`, and play `invalid` if neither step 6 nor 7 moved anything.
9. If the finish is not powered and `redsToRevert` is set (ability 13): move each tracked coordinate along with the swap just made, turn every tracked cell that is still green back to red, clear `redsToRevert`, `updatePowerStatus(send=true)`, send `fullgamestate`.

Moving a yellow therefore costs: each yellow that moves becomes an immovable orange. That is the game's pressure, along with the clock.

## 6. Scoring, win and loss

There is no score. The goal is a powered finish cell.

**Victory test** (`updatePowerStatus`, @36720-36810):

```text
room.properties.put('gameArray', gameArray)
if gameArray[finishGoalX][finishGoalY].power == true:
    sendFullGameState(room, userlist)          # userlist: undefined global here
else if bSendUpdate:
    send {_cmd:'powerstatus', powerArray: visited}
```

`sendFullGameState` (@23331) then repeats the test on the message it built:

```text
send(fullgamestate)
if fullgamestate.gameArray[finishGoalX][finishGoalY].power == true:
    won = true; fullgamestate.timerState = 0
send(fullgamestate)                            # sent twice, every time
if won: sendVictoryMsgs(room)
```

`sendVictoryMsgs` sends `victory {}` and calls `__mg.minigameCompleteGame(user, 3, timeRemaining / maxTime, numResets + 1, instrumentsUsed, consumablesUsed)` for the first non-spectator, where `instrumentsUsed = (instcc == -1 && origInstcc != -1)` and `consumablesUsed = consUsed`. Note the ratio uses `maxTime`; spec-protocol-common's table writes `initialTime` for the same thing.

Victory can be reached by a path move, an adjacent swap, item cheats 3 and 4, ability cheats 1, 7 and 13, row and column removal, or the `redsToRevert` step. Every one of them goes through `updatePowerStatus` or `sendFullGameState`, so the port checks in one place: after any board change, recompute power and, if the finish is powered, end the game.

**Timer.** `tick` runs every 100 ms over every game room:

| `timerState` | Tick behaviour |
|---|---|
| 0 | nothing (before the first click or cheat) |
| 1 | `timeRemaining -= (now - lastTime) / 1000`; at `<= 0`: `timerState = 0`, send `failure {}`, then `minigameCompleteGame(user, 4, 0, numResets + 1, false, false)` |
| 2 | paused; at `now > endStopTime` back to 1 and send `timerupdate` |
| 3 | `timeRemaining += delta`; at `now > endReverseTime` back to 1 and send `timerupdate` |

The client runs its own display timer from `timerupdate` and `fullgamestate`.

**Result codes for the port.** Victory is `GameOutput::Victory` (server result 1). Timeout is `GameOutput::Failure`, which the session loop reports as defeat, cell result 2 (spec-protocol-common Q4: extension code 4 maps to cell 2).

### Shipped bugs the port does not copy

| Bug | Effect in the original | Port |
|---|---|---|
| `updatePowerStatus` passes the undefined global `userlist` | online, the victory `fullgamestate` would go nowhere; offline the stub ignores recipients | broadcast |
| `fullgamestate` is sent twice | double redraw | send once (open question 3) |
| victory sets `timerState = 0` on the message only, not the room | the room clock keeps running; a later click restarts it and can then report a timeout after the win | the first result ends the game; later input is ignored |
| `consUsed` is never set true | `consumablesUsed` always false | keep false; nothing in Rust reads it |
| `activateItemCheatRequest` does not check the session | any `cheatNum` the client sends is honoured against the user properties | validate against the session's `instcc` and `CA` (section 7) |
| `removeXxx` and `addRandomGreenCrystals` use the global `room` | broken with more than one room | operate on the game instance |
| reset restores `instcc` and abilities, not `CA` | consumables spent before a reset stay spent | keep (it is the inventory model) |

## 7. Cheats

### Item cheats (`activateitemcheat {cheatNum}`, 0-4)

`activateItemCheatRequest` (@10257):

- If `cheatNum <= instcc`, the instrument pays: `processItemCheat`, then `instcc = -1` (the instrument is used up), then `fullgamestate`.
- Else, if `CA<cheatNum> > 0`, a consumable pays: decrement `CA0` through `CA<cheatNum>`; then for each `i` from `cheatNum+1` to 4, decrement `CAi` once if it is greater than `CA(i-1)`. Then `processItemCheat` and `fullgamestate`. (CA reads as a cumulative count: `CAk` is the number of consumables that can pay for tier `k`.)
- Else nothing happens and nothing is sent. The port should send nothing too, but log it.

`processItemCheat` starts the clock if it is stopped (`timerupdate`), adds 2 s and sends `timerupdate` if ability 11 is active, then:

| `cheatNum` | If the selected cell qualifies | Else |
|---|---|---|
| 0 (Crystal Extractor) | selected red: clear the selection (`selectedupdate`), cell becomes empty, `squarestate` | arm mode 0 |
| 1 | selected orange: clear the selection (`selectedupdate`), becomes yellow, `squarestate` | arm mode 1 |
| 2 | (no immediate form) | arm mode 2: the next path move swaps any two cells |
| 3 | selected empty: clear the selection silently, becomes green with `var = nextInt(3)`, `updatePowerStatus(send=true)`, `squarestate` | arm mode 3 |
| 4 | selected colour 2 or more: clear the selection silently, becomes green with `var = nextInt(3)`, `updatePowerStatus(send=true)`, `squarestate` | arm mode 4 |

An armed mode is used by the next qualifying click (section 5, step 2).

### Ability cheats (`activateabilitycheat {cheatNum}`, bits 0-14)

`activateAbilityCheatRequest` (@13217) requires bit `cheatNum` of `abilityBitfield`. It runs `processAbilityCheat`, clears the bit, and sends `abilitycheatstate`. `processAbilityCheat` starts the clock if it is stopped, plays the sound, then:

| Bit | Sound | Effect |
|---|---|---|
| 0 | `time/chng` | +5 s, `timerupdate` |
| 1 | `time/xchng` | -5 s (floor 0), `timerupdate`, then 2 random greens on empty cells |
| 2 | `time/stop` | `timerState = 2` for 5000 ms, `timerupdate` |
| 3 | `time/alter` | `timerState = 3` (clock runs backwards) for 5000 ms, `timerupdate` |
| 4 | `plyfld/addremSpc` | remove the first goal-free edge in the order bottom row, top row, left column, right column |
| 5 | `plyfld/addremCont` | the same, in the order top, bottom, right, left |
| 6 | `plyfld/alter` | `yellowsWithoutOrange += 5` |
| 7 | `mvprts/addrem` | 2 random greens on empty cells |
| 8 | `mvprts/alter` | `yellowsWithoutOrange += 5` (same as 6) |
| 9 | `mvprts/xchng` | selected red: becomes yellow (selection cleared) and `redsToYellows += 1`; else `redsToYellows += 2` |
| 10 | `tools/alter` | `instcc = origInstcc` (the instrument is usable again), `fullgamestate` |
| 11 | `tools/apply` | `itemCheatsAddTime = true`: every later item cheat adds 2 s |
| 12 | `obst/addrem` | selected red: becomes empty (selection cleared); else `redsToRemove += 1` |
| 13 | `obst/alter` | `ceil(reds / 3)` random reds turn green (no new `var`) and are recorded in `redsToRevert`; `updatePowerStatus(send=true)`, `fullgamestate`. If the next move does not win, they turn back to red |
| 14 | `obst/xchng` | +5 s, `timerupdate`; selected red: becomes empty and `redsToRemove += 1`; else `redsToRemove += 2` |

An edge "without a goal" means neither goal is on it (for the bottom row, `h-1 > startGoalY` and `h-1 > finishGoalY`). The remove functions shift `gameArray`, shrink the width or height, fix the goal coordinates and offsets, fix or clear the selection, count the greens they deleted and re-add that many with `addRandomGreenCrystals(n, overwrite=true)`. If no edge qualifies, the cheat does nothing (the bit is still spent).

`addRandomGreenCrystals(n, overwrite)` picks empty cells at random without repeats (swap-remove with `nextInt`). If there are not enough empties and `overwrite` is set, it overwrites random non-empty cells. It ends with `updatePowerStatus(send=true)` and `fullgamestate`.

Bits 6 and 8 are identical; 15 bits are defined and the offline test bitfield is 1023 (bits 0-9).

## 8. Hints and timers

There are no hints. The server runs one clock per game:

- Countdown from `maxTime` seconds, started by the first `playfieldclick`, item cheat or ability cheat.
- Pause (ability 2) and reverse (ability 3), each 5000 ms, ended by the tick.
- The tick only reports through `timerupdate` when it leaves state 2 or 3; otherwise the client counts down on its own.

The port runs on the session loop's 250 ms tick (`game.rs`) and should subtract the elapsed 0.25 s per tick, not wall time, so REPL scripts are exact. `tick N` then advances `N * 0.25` s.

## 9. Golden transcripts

All three use the tutorial board: `game GoauldCrystals difficulty 2 seed 1` with techcomp 1-5 gives 4x4, 16 s, and `seed == 1` pins the board and goals (section 3), so no rng draw affects the outputs. The port sends `fullgamestate` once (open question 3). `C` below stands for the full `gameArray`.

Start state: start (0,0) side 3 offset 0, finish (3,2) side 1 offset 2, powered cells (0,0), (0,1), (1,1), (2,1), (3,1). Finish not powered.

### A: Crystal Extractor win (needs `instcc = 0` in the session)

| Step | Input | Expected outputs, in order |
|---|---|---|
| 1 | join | `fullgamestate {..., timerState 0, timeRemaining 16, maxTime 16, activeItemCheat -1, instcc 0, gameArray C}` |
| 2 | `activateitemcheat cheatNum=0` | `timerupdate {timeRemaining 16, timerState 1}`; `fullgamestate {..., activeItemCheat 0, instcc -1}` (nothing selected, so mode 0 is armed) |
| 3 | `playfieldclick x=3 y=2` | `squarestate {xPos 3, yPos 2, info {color 0, power false, var 0}}`; `triggersound {soundName action}` |
| 4 | `playfieldclick x=3 y=3` | `selectedupdate {xPos 3, yPos 3}`; `triggersound {soundName select}` |
| 5 | `playfieldclick x=3 y=2` | path move: green (3,3) into empty (3,2), found as the `y-1` neighbour. `fullgamestate {..., timerState 0, finish powered}`; `victory`. The game ends here. |

In the original extension, step 5 then also sent `squarestate` (3,2) and (3,3), `triggersound action` and `selectedupdate {-1,-1}`, and reported `minigameCompleteGame(user, 3, ratio, 1, true, false)`, where `instrumentsUsed` is true because `instcc` went from 0 to -1. The port stops at `victory`: the client is on its victory frame by then.

### B: no cheats, a move, an invalid move, timeout

| Step | Input | Expected outputs, in order |
|---|---|---|
| 1 | join | `fullgamestate {..., timerState 0, timeRemaining 16}` |
| 2 | `playfieldclick x=2 y=0` | `timerupdate {timeRemaining 16, timerState 1}`; `selectedupdate {xPos 2, yPos 0}`; `triggersound {soundName select}` |
| 3 | `playfieldclick x=2 y=1` | adjacent swap of yellow (2,0) and green (2,1); the yellow on (2,1) turns orange. `powerstatus {powerArray [{xPos 0, yPos 0}, {xPos 0, yPos 1}, {xPos 1, yPos 1}]}`; `squarestate {xPos 2, yPos 1, info {color 3, power false, var 0}}`; `squarestate {xPos 2, yPos 0, info {color 1, power false, var 0}}`; `triggersound {soundName action}`; `selectedupdate {xPos -1, yPos -1}` |
| 4 | `playfieldclick x=0 y=0` | `selectedupdate {xPos 0, yPos 0}`; `triggersound {soundName select}` |
| 5 | `playfieldclick x=3 y=3` | green to green, not adjacent: no move. `selectedupdate {xPos -1, yPos -1}`; `triggersound {soundName invalid}` |
| 6 | `tick 64` | `failure` (16 s at 0.25 s per tick) |

Step 3 shows why the path is now cut: (2,1) is orange, so power stops at (1,1). The `powerArray` order is the breadth-first visit order.

### C: no cheats, a path move, the clock runs down

Derived from `processPlayfieldClick` steps 5-8 on the same board: the selected green (2,1) moves into the empty (2,2) by a one-cell path search (the `y+1` neighbour matches the destination at once). The move cuts nothing that powers the finish, so the game goes on.

| Step | Input | Expected outputs, in order |
|---|---|---|
| 1 | join | `fullgamestate {..., timerState 0, timeRemaining 16}` |
| 2 | `playfieldclick x=2 y=1` | `timerupdate {timeRemaining 16, timerState 1}`; `selectedupdate {xPos 2, yPos 1}`; `triggersound {soundName select}` |
| 3 | `playfieldclick x=2 y=2` | path move, green (2,1) into empty (2,2). `powerstatus {powerArray [{xPos 0, yPos 0}, {xPos 0, yPos 1}, {xPos 1, yPos 1}]}`; `squarestate {xPos 2, yPos 2, info {color 1, power false, var 0}}`; `squarestate {xPos 2, yPos 1, info {color 0, power false, var 0}}`; `triggersound {soundName action}`; `selectedupdate {xPos -1, yPos -1}` |
| 4 | `tick 8` | nothing is sent (state 1 is silent); `timeRemaining` is 14 |

Finish (3,2) stays unpowered, so no `victory`. The tutorial board sets `var 0` on every cell, so the `var` values above are exact.

## 10. Port slicing

Layout per C4: `crates/minigame/src/minigame/games/goauld_crystals/`. Estimates are modelled on Livewire (`mod.rs` 444, `setup.rs` 402, `tests.rs` 272 lines).

| Packet | Files | Lines (est.) | Content |
|---|---|---|---|
| MG-GC1 | `mod.rs` | 230 | `GoauldCrystalsGame` (session fields as Livewire, `rng: StdRng`, board, goals, selection, clock, cheat state, `outcome`), `MinigameInstance` impl with `message` dispatch stubs, `pub const PORTED: bool = false`, `create()` |
| | `setup.rs` | 350 | difficulty table and clamp; `place_goals`; `randomize_crystals` with fractional counts and the capacity loop; tutorial board; `update_power` breadth-first search returning the visit list; builders for `fullgamestate`, `timerupdate`; `init_game(first_time)` |
| | `tests_setup.rs` | 300 | table spot checks (both ends of every column), tutorial board and power set, generated boards never start solved, goal distance, capacity loop on the smallest and largest tiers, `fullgamestate` field set and types |
| MG-GC2 | `moves.rs` | 440 | `playfieldclick` steps 1-9: bounds validation, clock start, armed item modes on click, `redsToYellows` and `redsToRemove`, select, path search and swap, adjacent swap with the yellow-to-orange rule, invalid, `redsToRevert` remap and revert; victory hand-off to one `after_board_change()`; builders for `squarestate` and `powerstatus` (first used by `playfieldclick`) |
| | `tests_moves.rs` | 340 | transcripts B steps 1-5 and C steps 1-3; path search reaches through empties only; path mode never falls back to an adjacent swap; yellow to orange with and without `yellowsWithoutOrange`; out-of-bounds and non-integer clicks dropped |
| MG-GC3 | `rules.rs` | 460 | victory (terminal guard, one `victory` + `GameOutput::Victory`), timer states 0-3 on the 0.25 s tick, timeout, item-cheat payment (`instcc`, `CA` deduction) and `processItemCheat`, ability bits 0-14, one parametrised edge-removal function for the four sides, `add_random_greens`, `processreset`; flips `PORTED` to `true` |
| | `tests_rules.rs` | 380 | transcript A; transcript C step 4 (`tick 8` leaves `timeRemaining` at 14); timeout at `tick 64`; pause and reverse; each ability bit; CA deduction table; input after victory is ignored; golden `.mgs` script for B (`crates/minigame/tests/scripts/goauld_crystals/b_move_timeout.mgs`); transcript A is a Rust unit test that sets `instcc = 0` on the session before `started()` and becomes an `.mgs` once MG-05 or MG-22 adds an inventory directive (open question 5) |

Slices are about 880 (GC1), 780 (GC2) and 840 (GC3) lines including tests; GC1 is the largest because it holds the table, generation, power search and two message builders. If `rules.rs` passes 500 lines, split the ability cheats into a `cheats.rs` beside it (open question 4). MG-GC3 also adds `GoauldCrystals` to `kind.rs`'s ported list.

## 11. Open questions

1. **Tutorial switch.** The extension shows the fixed tutorial board when `seed == 1` and the board is 4x4. Online, sessions get a random seed, so the tutorial would almost never show. Keep the rule (the goldens rely on it), or make the tutorial board a content choice instead (for example, the first GoauldCrystals a character plays)? Needs an owner decision.
2. **Out-of-table techcomp.** The extension falls back to 7x7, 60 s, 0 obstacles for techcomp above 55 or a bad difficulty. The port proposal clamps to the last row instead. Confirm.
3. **Double `fullgamestate`.** The extension always sends it twice. The proposal sends once; the client redraws idempotently, but this has not been checked in the lab.
4. **`rules.rs` size.** If the estimate holds, C4's layout fits; otherwise add `cheats.rs`. The architect decides.
5. **`instcc` and `CA` in the session.** Rust's `joinOK` sends `instcc=-1` and `CA0..4=0`, so no item cheat can ever be paid for. Transcript A needs `instcc = 0`, which the C6 REPL header cannot set today. MG-05 or MG-22 should add a way to set the cheat inventory in a REPL script.
6. **Not verified live.** No transcript here was run against the client. Lab confirmation of the double-send, of the `resetgame` no-op, and of a mid-game `fullgamestate` while the client timer runs is still owed.
