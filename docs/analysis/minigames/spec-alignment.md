# Alignment: rules spec from the embedded SFS extension

Packet MG-R1. This spec is in the C5 format from [work-packets.md](work-packets.md#c5-spec-doc-format-every-research-packet-writes-to-it), and feeds the Alignment ports MG-AL1 to MG-AL3.

**Evidence base.** The extension's ActionScript in `Alignment.upk`, disassembled and decompiled with the MG-R0 tools (`tools/minigame-swf/`). Everything here is static analysis: no claim was checked against a live client or a live SFS server. The transcripts in section 9 come from a Python model of the decompiled rules, not from a capture.

**Confidence tags:**

- **Confirmed**: read directly from the decompiled extension or client code.
- **Inferred**: follows from that code but was not traced end to end.
- **Open**: section 11.

**Conventions:**

- The board is `gameArray[x][y]`. `x` is the first index and is what the client calls `row`; `y` is the second index, the client's `col`.
- Column parity (`y % 2`) selects the neighbour table.
- Function names are the extension's own. Excerpts are cleaned pseudocode of at most 20 lines (D-MG10).
- To rerun a claim, list a function with `python -I tools/minigame-swf/list_functions.py <Alignment.swf> --function <name>`. Keep the output outside the repo.

## 1. Verdict

A real game (confirmed). The movie embeds a complete SmartFoxServer 1.x server extension that owns the board, the scoring and the timer. The client movie only renders it.

| Field | Value |
|---|---|
| Package | `UI\Flash\Alignment.upk` |
| GFX offset | 4628 |
| GFX length | 293868 |
| md5 (GFX) | `f996c4242dec54bfa3cd6689afd7c2e1` |
| SWF version | 9 |
| Functions | 410 (movie and extension) |

The same numbers are in [client-packages.md](client-packages.md) and [spec-protocol-common.md](spec-protocol-common.md).

**Extension functions, by role:**

| Role | Functions |
|---|---|
| SFS entry points | `init`, `destroy`, `reloadZone`, `handleRequest`, `handleInternalRequest`, `handleInternalEvent`, `onUserJoin`, `tick` |
| Setup | `initGame`, `selectIcons`, `sendFullGameState`, `processReset` |
| Board geometry and matching | `findNeighbors`, `checkForMatches`, `checkNeighborMatches`, `spinCheck`, `checkPowerUpMatches`, `getMatchPower` |
| Moves | `swapPieces`, `processHex`, `processMove` (called, never defined) |
| Cascade | `doCascade`, `checkForCascade`, `getNewType` |
| Score and end | `doScore`, `checkForVictory`, `doVictory` |
| Hints | `getHint`, `doHint`, `clickToHint` |
| Cheats | `activateItemCheatRequest`, `activateAbilityCheatRequest`, `processItemCheat`, `processAbilityCheat`, `checkForMatchCheat`, `handleItemCheatInfo`, `sendItemCheatInfo` |
| Helpers | `sendTimerUpdate`, `randIndex`, `randRange`, `minValue`, `maxValue` |

**Offline mode.** When the host never sets `_root.ipaddress.value`, the movie runs this extension locally: `sendMessage` calls `handleRequest` directly and draws from `Math.random`. See spec-protocol-common Q1. It does not affect the port.

**D-MG3 option B: what `bypasshexspinner` is.** It is not a game. It is the server-to-client message that removes cells without the spinner animation: cascade matches in `checkForCascade`, and the board-clearing cheats (confirmed, `d_rules` and `d_cheats`). The client handles it with `removePieces`.

The Alignment extension sets `gameType = 'activate'` on every init, and `getNewType` has tables for `'activate'`, `'repair'` and `'search'` (confirmed). That suggests the designers meant the Activate shell, and possibly a Repair or Search variant, to be skins of this game. It supports aliasing the Activate shell to Alignment under option B. Nothing in the movie names Bypass, so a Bypass alias has no evidence here (inferred).

## 2. `joinOK` params read

The extension reads these room variables in `initGame` on the first init (confirmed). **The client movie reads none of them directly.** It gets `techcomp`, `difficulty`, `minigamelevel`, `abilityBitfield`, `instcc` and `CA0`-`CA4` from `fullgamestate`.

> **Correction to spec-protocol-common Q2.** Q2 lists Alignment's reader as "client; seed: extension". For Alignment, the reader of every variable is the extension.

| Name | Type sent by Rust | Read as | Use |
|---|---|---|---|
| `techcomp` | Number | `Number(...)` | `minigamelevel = floor((techcomp - 1) / 5)`, which selects the goal totals (section 4). Echoed in `fullgamestate`. |
| `difficulty` | Number | `Number(...)` | The tier, 1-4: icons, time and board size. Any other value leaves the board undefined (shipped bug; the port clamps it). |
| `pclevel` | Number | `Number(...)` | Read and never used online. |
| `seed` | Number | `Number(...)` | `java.util.Random.setSeed(seed)`, used by `selectIcons` only (section 3). |
| `abilityBitfield` | Number | raw | Ability cheats available (section 7). It is stored on the user as `abilityBitfield` and `origAbilityBitfield`. |
| `instcc` | Number | raw | The instrument cheat slot, -1 for none (section 7). Stored as `instcc` and `origInstcc`. |
| `CA0`-`CA4` | Number | raw | Consumable cheat counts per slot (section 7). |

`intelligence` is not read. Rust's `joinOK` (`server/session_loop.rs`) sends all of these. Today it hardcodes `instcc = -1` and `CA0..4 = 0`, so no item cheat is ever offered; that is a launch-flow concern (spec-launch-flow section 5), not a port change.

## 3. Board model

**Dimensions (confirmed, `initGame`):**

| Tier | Size |
|---|---|
| 1 | 8x8 |
| 2 | 8x8 |
| 3 | 9x9 |
| 4 | 10x10 |

Width and height are always equal. Several loops use one where they mean the other, which is harmless only because the boards are square (section 6, bug list).

**Piece ids (confirmed):**

| Value | Meaning |
|---|---|
| 0 | Empty. This is transient, during a cascade. |
| 1 | cash |
| 2 | experience |
| 3 | achievement |
| 4 | power |
| 5 | force |
| 6 | code |
| 7 | protocol |
| 8 | cipher |
| 9 | Blocker. It never matches and cannot be clicked. |
| `t + 0.p` | A power-up on type `t`, with power `p` in 1-6, for example `4.3` (section 6). |
| -1 | `findNeighbors` uses this for an out-of-bounds neighbour. It never appears on the board. |

Matching uses `floor(type)`, so a power-up matches its base type.

**Hex geometry** (confirmed, `findNeighbors`). Neighbours are returned in the order n0-n5:

| Column parity | n0 | n1 | n2 | n3 | n4 | n5 |
|---|---|---|---|---|---|---|
| even `y` | (x-1, y) | (x-1, y+1) | (x, y+1) | (x+1, y) | (x, y-1) | (x-1, y-1) |
| odd `y` | (x-1, y) | (x, y+1) | (x+1, y+1) | (x+1, y) | (x+1, y-1) | (x, y-1) |

**The match rule** (confirmed, `checkForMatches`):

- A cell of type `t != 9` is matched when some *adjacent neighbour pair* is in bounds and both members have `floor` type `t`. In other words, it sits in a triangle of three equal hexes.
- The pairs are (n5, n0), (n4, n5), (n3, n4), (n0, n1), (n1, n2) and (n2, n3).
- `checkNeighborMatches` then flood-fills (BFS) from those seeds through any same-floor-type neighbour. A matched triangle takes its whole connected same-type group.

**Initial generation** (confirmed, `selectIcons`). The original uses `rng` = `java.util.Random(seed)`:

```text
N = width * height; bonusCount = round(N * 0.25); goalCount = round(N * 0.75)
k = 0
for x in 0..width: for y in 0..height:          # x outer, no shuffle
    if k < bonusCount:
        t = randIndex(rng, bonusIcons)
        if randRange(rng, 1, 100) <= 5: t += 0.1  # 5% power-up, power 1
    else:
        t = randIndex(rng, goalIcons)
    board[x][y] = t; k += 1
while (m = checkForMatches(board)) not empty:
    for cell in m:
        board[cell] = randIndex(rng, bonusIcons if cell.type in {1, 2, 3} else goalIcons)
gameArrayInit = copy(board); gameArray = board
```

So the first quarter of the cells, in row-major `x` then `y` order, holds bonus icons and the rest holds goal icons. Re-rolled cells lose their `+0.1`. The comparison is exact (no floor), so a fractional bonus type is re-rolled from `goalIcons`.

**Does the client reproduce anything from `seed`?** No (confirmed). The client renders `gameArray` from `fullgamestate`, and every later change arrives as a message. No `java.util.Random` port is needed for client compatibility.

Under D-MG12 the port draws from one `StdRng::seed_from_u64(session.seed)` for setup, refills and cheats. The original seeded only setup; refills and cheats used `Math.random`, because `doCascade` and both cheat processors force `rng = -1` (confirmed). This is a deliberate and documented divergence that makes REPL transcripts reproducible.

## 4. Difficulty table

The extension's table (confirmed, `initGame`) is shown beside the Python table in `deprecated/python/base/minigame/Alignment.py` lines 14-90, which is keyed by `minigamelevel` 0-10. **They agree on every value.**

| Tier | `bonusIcons` | `goalIcons` | Time (s) | Board | Python |
|---|---|---|---|---|---|
| 1 | [1,2,3] | [4,5,6] | 120 | 8x8 | same |
| 2 | [1,2,3] | [4,5,6,7] | 120 | 8x8 | same |
| 3 | [1,2,3] | [4,5,6,7] | 100 | 9x9 | same |
| 4 | [1,2,3] | [4,5,6,8,7] | 80 | 10x10 | same |

`goalTotals` by `minigamelevel = floor((techcomp - 1) / 5)`:

| Level | Tier 1 | Tiers 2 and 3 | Tier 4 |
|---|---|---|---|
| 0 | 45/15/6 | 63/30/18 | 63/30/23/6 |
| 1 | 45/23/12 | 72/38/18 | 72/38/23/12 |
| 2 | 63/30/18 | 81/38/24 | 81/38/30/18 |
| 3 | 72/38/18 | 90/50/24 | 90/50/30/24 |
| 4 | 81/38/24 | 105/63/30 | 105/63/38/30 |
| 5 | 90/50/24 | 120/75/30 | 120/75/45/30 |
| 6 | 105/63/30 | 135/88/40 | 135/88/53/36 |
| 7 | 120/75/30 | 150/100/48 | 150/100/60/42 |
| 8 | 135/88/40 | 180/125/60 | 180/125/75/48 |
| 9 | 150/100/48 | 198/130/66 | 198/130/83/54 |
| 10 | 180/125/60 | 216/140/72 | 216/140/90/60 |

**How the totals are read:**

- The totals are, in order, power (4), force (5), code (6), and on tier 4 cipher (8). These are the `goalScores` slots in section 6.
- Tiers 2 and 3 show icon 7 (protocol), and tier 4 shows icons 8 and 7, but **protocol has no total and no score slot**. It is displayed and never counts.
- Outside levels 0-10 (`techcomp` < 1 or > 55), `goalTotals` stays empty (confirmed). `checkForVictory` then finds nothing unmet, so the first scored cascade sends `activate` and the game is winnable at once (inferred from the comparison against `undefined`). The port clamps the level to 0-10.

**Time, timer and initial state:**

- The stored `timeRemaining` is the time plus 0.25 s. Every outbound copy subtracts 0.25.
- `timerState` starts at 1, which means running.
- The tier-specific `*Score`/`*Total` room properties (`cipherTotal = 3`, `powerTotal = 21`, ...) are set and never read. They are dead.

## 5. Message table

**Inbound** (client to server). The client sends these on the extension channel with `sendMessage(cmd, obj)`, and `handleRequest` dispatches them (confirmed). A spectator's request is dropped.

"Port validation" is the D-MG2 rule for the port. The original validates nothing.

| Cmd | Fields (`SfsValue`) | Sent by the client when | Original handler | Port validation | Reply |
|---|---|---|---|---|---|
| `swappieces` | `type1`, `type2`, `type3` (Number); `row1`-`row3`, `col1`-`col3` (Number); `xPos`, `yPos` (Number); `orientation` (String, the clicked clip name) | A click on a hex edge (`hexClick`) while `validMove` | Sets `hintTimerPause = true`, then `swapPieces` | Phase `Ready`. The three cells are in bounds, distinct and an adjacent pair triangle (cell 1 plus neighbours `n_a`, `n_b` from section 3's pair list). Ignore the client `type*` and use the board types. No cell is type 9. | `feedback` (on a match only), then `hexspinner`, possibly `timerupdate` |
| `cascade` | none | The last removal tween finishes (`destroyHex`) | `doCascade` | Phase `AwaitCascade` | `hexmove` |
| `checkForCascade` | none | `hexmove` animation done (`checkForMoreMatches`) | `checkForCascade` | Phase `AwaitCheck` | `bypasshexspinner` or `cascadecheckcomplete` |
| `doscore` | none | `cascadeCheckComplete` | `doScore` | Phase `AwaitScore` | `updatescore`, `timerupdate`, maybe `activate` |
| `processreset` | none | Reset button (`resetGame`) | `numResets++`, `initGame(false)` | Phase `Ready` | `fullgamestate` |
| `callhint` | none | Sent by the client: hint bubble click (`callhint`); end of `hintHelpOut`, so before every `swappieces` (`resethinttimer`) | `getHint`, then `timeRemaining -= 5`, `timerupdate` | Phase `Ready` | `hint` (if found), `timerupdate` |
| `resethinttimer` | none | Sent by the client: hint bubble click (`callhint`); end of `hintHelpOut`, so before every `swappieces` (`resethinttimer`) | `hintLastTime = timeRemaining`, `hintTimer = 10` | Any phase but `Ended` | none |
| `processmove` | `cardType`, `target`, `current` | **Never sent by this movie** | Calls `processMove`, which is undefined: a no-op | Reject | none |
| `dovictory` | none | `activate_btn` click after `activate` | `doVictory`, unconditionally | Goals met **and** phase `Ready`. Otherwise ignore it. | `victory`, end code 1 |
| `activateitemcheat` | `cheatNum` (Number 0-4) | Item cheat button | `activateItemCheatRequest` | Phase `Ready`, slot available (section 7) | per cheat, then `itemcheatstate` |
| `activateabilitycheat` | `cheatNum` (Number 0-14) | Ability cheat button | `activateAbilityCheatRequest` | Phase `Ready`, bit set (section 7) | per cheat, then `abilitycheatstate` |

**Port phases** (the D-MG2 state machine; this is a design, not original behaviour):

```text
Ready --swappieces (match)--> AwaitCascade      # hexspinner had matches
Ready --swappieces (no match)--> Ready          # hexspinner spinCount 3, matches []
Ready --board-clearing cheat--> AwaitCascade    # bypasshexspinner
AwaitCascade --cascade--> AwaitCheck            # hexmove
AwaitCheck --checkForCascade--> AwaitCascade    # bypasshexspinner (new matches)
AwaitCheck --checkForCascade--> AwaitScore      # cascadecheckcomplete
AwaitScore --doscore--> Ready                   # updatescore (+ activate)
any --dovictory (goals met) / timeout / abort--> Ended
```

**Outbound** (server to client). `_cmd` is a String. All arrays use `SfsValue::list` (`t='a'`), and a cell object is `{x, y, type}`, all Numbers.

| Cmd | Fields | Sent by | Client action |
|---|---|---|---|
| `fullgamestate` | `numResets`, `timeRemaining` (minus 0.25), `timerState`, `isSpectator` (Bool), `gameArray` (list of lists of Number), `playfieldWidth`, `playfieldHeight`, `goalIcons` (list), `goalTotals` (list), `goalScores` (list), `techcomp`, `difficulty`, `minigamelevel`, `instcc`, `CA0`-`CA4`, `abilityBitfield` | `initGame` (start and reset); spectator join | Rebuilds the board, scores and cheats. The first one starts the client timer. |
| `hexspinner` | `matches` (list of cells), `spinCount` (1, 2 or 3), plus `xPos`, `yPos`, `orientation`, `row1`-`row3`, `col1`-`col3` echoed from the request | `swapPieces` | Animates `spinCount` rotations, applies the rotation locally, removes `matches`. 3 means no match. |
| `feedback` | `spinCount`, `powerUpValues` (list of Number) | `checkPowerUpMatches`, every call (`powerUpValues` empty when no power-up was in the match) | Power-up popups |
| `powerupconvert` | `x`, `y`, `powerUp` (Number, e.g. 0.3) | Cheats | Adds `powerUp` to that cell's type |
| `bypasshexspinner` | `matches` (list of cells) | `checkForCascade`, board-clearing cheats | Removes the cells, then sends `cascade` |
| `hexmove` | `direction` (Number 0-5), `hexesToMove` (list of `{moveTotal, x, y, type}`), `newHexes` (list of cells) | `doCascade` | Slides and refills, then sends `checkForCascade` |
| `cascadecheckcomplete` | none | `checkForCascade` | Rotation indicator, then sends `doscore` |
| `updatescore` | `goalScores`, `goalTotals`, `goalIcons` (lists) | `doScore` | Score bars |
| `activate` | none | `checkForVictory` | Arms `activate_btn`. A click sends `dovictory`. |
| `victory` | none | `doVictory` | Victory frame |
| `failure` | none | `tick` on timeout | Failure frame |
| `timerupdate` | `timeRemaining` (minus 0.25), `timerState` | Many (see section 8) | Resyncs the client countdown |
| `hint` | `hex1`, `hex2`, `hex3` (cells) | `getHint` | Highlights three hexes for 2 s |
| `clicktohint` | none | `tick` idle prompt | Shows the hint-help bubble |
| `itemcheatstate` | `CA0`-`CA4`, `instcc` | Item cheat | Refreshes the item cheat buttons |
| `abilitycheatstate` | `abilityBitfield` | Ability cheat | Refreshes the ability cheat buttons |

**The client-driven loop.** The server never chains cascade steps on its own; each step waits for the client's animation callback:

```text
swappieces → hexspinner → cascade → hexmove → checkForCascade
  → bypasshexspinner → cascade → ...
  → cascadecheckcomplete → doscore → updatescore
```

The port keeps this handshake, because the client waits for each reply, but enforces the order with the phases above.

### `swapPieces` (confirmed)

```text
b = copy(gameArray)
b[c3] = type2; b[c2] = type1; b[c1] = type3            # rotation 1
spin = 1; m = checkForMatches(b)
if m empty:
    b[c3] = type1; b[c2] = type3; b[c1] = type2        # rotation 2
    spin = 2; m = checkForMatches(b)
    if m empty:
        send hexspinner{matches: [], spinCount: 3, echo...}; hintTimerPause = false; return
flood = checkNeighborMatches(m); expanded = checkPowerUpMatches(flood)
currentMatches = flood; spinCount = spin
send hexspinner{matches: expanded, spinCount: spin, echo...}
for cell in expanded: b[cell] = 0
gameArray = b; checkForMatchCheat()
```

Rotation 2 is applied on top of rotation 1's writes, but because all three cells are rewritten from the request's types the result is as if it started from the original board. On a no-match the board is unchanged; the client rotates three times, back to the start.

## 6. Scoring, win and loss

### Power-ups (confirmed, `checkPowerUpMatches`, `getMatchPower`)

For each matched cell whose type has a fraction:

- If `powerUpAddTime` is set: `timeRemaining += 2`, then `timerupdate`.
- `p = round(frac * 10)` is pushed to `powerUpValues`, and the match set grows.

| p | Adds to the match set |
|---|---|
| 1 | The six in-bounds neighbours |
| 2 or 6 | The whole line `y` (every `x`) |
| 3 | Radius 2: the six neighbours plus the twelve cells of the outer ring |
| 4 or 6 | One diagonal, walked both ways with a parity toggle, cells > 0 |
| 5 or 6 | The other diagonal |

`p = 6` is therefore the full star. The expansion is one loop over the match set that re-reads the set's length on every pass (listing: the loop bound is read at each iteration and the loop jumps back to it). Cells appended during the expansion are therefore visited too, so a power-up cell that another power-up pulls in fires its own effect: its own shape, its `timeRemaining += 2` when `powerUpAddTime` is set, and its `powerUpValues` entry. A candidate is appended only when no cell with the same `x` and `y` is already in the set, so no cell is added twice, including one appended earlier in the same expansion. The server always sends `feedback {spinCount, powerUpValues}` from `checkPowerUpMatches`, before `hexspinner` or `bypasshexspinner`; `powerUpValues` is `[]` when nothing fired.

`getMatchPower(type, mp)` decides which power a new cell gets:

| `mp` | Types | Power |
|---|---|---|
| 8 | 1-6 | 0.1 |
| 8 | 7 | 0.2 |
| 8 | 8 | 0.3 |
| 12 | 4 | 0.2 |
| 12 | 5 | 0.4 |
| 12 | 6 | 0.5 |
| 12 | 8 | 0.6 |
| 12 | 1-3 | 0.6 |
| 12 | 7 | Undefined, giving `NaN` (shipped bug) |

Types 1-3 and 8 share the `mp = 12` branch, which tests `(t >= 1 && t <= 3) || t == 8` and returns 0.6; type 7 has no branch.

### `checkForCascade` and `doCascade` (confirmed)

`checkForCascade` decides whether the cascade continues:

```text
scores = currentScores ++ [c.type for c in currentMatches]
m = checkForMatches(gameArray)
if m not empty:
    currentScores = scores; cascadeCount += 1
    flood = checkNeighborMatches(m); currentMatches = checkPowerUpMatches(flood)
    zero those cells; send bypasshexspinner{matches: currentMatches}
else:
    if dropIndex == 5: dropIndex = 0
    elif scores not empty: dropIndex += 1
    currentScores = scores; send cascadecheckcomplete
```

The drop direction therefore rotates 0, 1, 2, 3, 4, 5, 0 once per scored turn. On a cascade step `currentMatches` holds the power-up-expanded set, but on the swap step it holds the flood only (shipped inconsistency). Cells that a swap's power-ups add are therefore never scored; cells that a cascade's power-ups add are.

**`doCascade` gravity**, direction `d = dropDirection[dropIndex]`. Each direction walks lines of cells. Within a line, every non-empty cell after the first gap slides by the number of gaps seen so far, and `{moveTotal, x, y, type}` is pushed at the cell's *original* position.

| `d` | Lines walked | One slide step from (x, y) |
|---|---|---|
| 0 | For each `y` from H-1 to 0: `x` from W-1 to 0 | (x+1, y) |
| 1 | Starts (s, 0) for s = 0..H+3; `y` +1 per step, `x` -1 every second step | even y: (x, y-1); odd y: (x+1, y-1) |
| 2 | Starts (s, 0) for s = -3..W-1; `y` +1 per step, `x` +1 every second step | even y: (x-1, y-1); odd y: (x, y-1) |
| 3 | For each `y` ascending: `x` from 0 to H-1 | (x-1, y) |
| 4 | Starts (s, H) for s = W-1..-4; `y` -1 per step, `x` +1 every second step | even y: (x-1, y+1); odd y: (x, y+1) |
| 5 | Starts (s, **8**) for s = W+3..0; `y` -1 per step, `x` -1 every second step | even y: (x, y+1); odd y: (x+1, y+1) |

Out-of-range positions on a line are skipped. Direction 5 starts every line at `y = 8`, so on a 10x10 board column 9 never slides in direction 5. Its gaps are refilled in place (shipped bug). The toggle phase for direction 4 depends on the board's parity. Directions 1, 2, 4 and 5 should be ported line for line from the listing; this table is the map, not the spec. `hexesToMove` entries are appended in this walk order, so for `d = 0` they run `y` descending and, within a row, `x` descending.

**Refill** (confirmed, same in every direction except the scan order and the clone source):

```text
A = [t for t in distinct int types of currentMatches if 8 <= count(t) <= 11]
B = [t for t in distinct int types of currentMatches if count(t) >= 12]
for each cell == 0, in the direction's scan order:
    if A: t = A.pop(0) + getMatchPower(t, 8)
    elif B: t = B.pop(0) + getMatchPower(t, 12)
    else:
        r28 = randRange(1, 100); r29 = randRange(1, 100)
        if r29 <= 8: t = 9                                    # blocker, 8%
        elif r28 <= 10 and clone_source(cell) > 0: t = clone_source(cell)
        else: t = getNewType()
    board[cell] = t; newHexes.push({x, y, type: t})
```

A match of 8 or more of one type therefore earns exactly one power-up in the refill. `A` and `B` are built by scanning the types 1 to 8 in ascending order, so each list is sorted by type, and `A.pop(0)` (`splice(0, 1)` in the listing) takes the first element.

**Scan order and clone source** (the clone source is out of bounds, so no clone, at the edges):

| `d` | Scan order | Clone source |
|---|---|---|
| 0 | `x` W-1 to 0, `y` ascending | (x+1, y) |
| 1 | `x` W-1 to 0, `y` ascending | even y: (x, y-1); odd y: (x-1, y-1) |
| 2 | `x` ascending, `y` ascending | (x, y-1). On even y, the test reads (x-1, y-1) but the copy is from (x, y-1). |
| 3 | `x` ascending, `y` ascending | (x-1, y) |
| 4 | `x` ascending, `y` from H-1 to 0 | even y: (x-1, y+1); odd y: (x, y+1) |
| 5 | `x` from W-1 to 0, `y` from H-1 to 0 | Only when (x+1, y) > 0: even y: (x, y+1); odd y: (x+1, y+1) |

**`getNewType` reads `difficulty` and then overwrites it with 1** (confirmed; this is a shipped quirk). For `gameType` `'activate'` (always) the tier 1 roll is used:

| `randRange(1, 100)` | Type |
|---|---|
| 1-12 | 1 |
| 13-20 | 2 |
| 21-25 | 3 |
| 26-58 | 4 |
| 59-83 | 5 |
| 84-100 | 6 |

The tier 2-4 tables exist but are unreachable (section 11, question 1). The consequence is that refills never create types 7 or 8. On tier 4, cipher (8) appears only from the initial board and clone copies, so high-level cipher totals are close to unreachable.

**Re-roll pass** (confirmed, only when `cascadeCount > 0`):

```text
for pass in 0..=cascadeCount:
    rerolled_first = false
    for i in (len(newHexes)-1) down to 0:
        while cell newHexes[i] completes a triangle:
            newHexes[i].type = randIndex(bonusIcons ++ goalIcons)
            if i == 0: rerolled_first = true
    if not rerolled_first: break
send hexmove{direction, hexesToMove, newHexes}; gameArray = board
```

The pass stops early whenever element 0 needed no re-roll, even if a later element still did (shipped quirk). So after the first scored step the refills tend not to chain, but a chain is still possible.

### `doScore` (confirmed)

```text
mult = [1, 1.5, 3, 5, 10][min(cascadeCount, 4)]
bonus = [0, 2, 5, 8, 12][min(cascadeCount, 4)]
for each integer type t in currentScores (exact match, so power-ups are skipped):
    n = count of t; s = floor(n * mult)
    slot = {4: 0, 5: 1, 6: 2, 8: 3}.get(t)      # 1, 2, 3 and 7 have no slot
    if slot is not None: goalScores[slot] += s
checkForVictory()                               # sends activate if every slot >= total
send updatescore{goalScores, goalTotals, goalIcons}
cascadeCount = 0; currentScores = []
timeRemaining += bonus; send timerupdate
hintLastTime = timeRemaining; hintTimer = 10; hintTimerPause = false
if stopTimerMatch and timeRemaining > 0: timerState = 1; send timerupdate
```

Types 1-3 (cash, experience, achievement) and 7 (protocol) add to named counters that nothing reads; they have no effect.

### Win

1. `checkForVictory` sends `activate` the first time every `goalScores[i] >= goalTotals[i]`.
2. The player clicks the armed button, and the client sends `dovictory`.
3. `doVictory` sends `victory` and calls `__mg.minigameCompleteGame(user, 3, timeRemaining / initialTime, numResets + 1, instrumentUsed, consUsed)` for the first non-spectator user.
4. `instrumentUsed` is `instcc == -1 && origInstcc != -1` (confirmed). This refines spec-protocol-common Q4, which gives `origInstcc != -1`.

### Loss

- `tick` checks `timeRemaining <= 0 && timerState >= 1`.
- It sets `timerState = 0`, sends `failure`, and calls `minigameCompleteGame(user, 4, 0, numResets + 1, false, false)`.
- There is no move limit and no deadlock check: a board with no legal swap simply runs out the clock (section 8).

### Port result codes (D-MG5)

| Event | `GameOutput` | Cell result code |
|---|---|---|
| Victory | `Victory` | 1 |
| Timeout | `Failure` | 4 (`session_loop` maps `Failure` to defeat) |
| Abort | from `aborted()` | 0 |

Both ends are terminal in the port: no message is processed after them. In the original, a victory does not stop the room timer, so `tick` could still send `failure` later (shipped bug).

## 7. Cheats

### Item cheats: `instcc`, `CA0`-`CA4` and `consUsed`

`activateItemCheatRequest(cheatNum)`, confirmed:

```text
if cheatNum <= instcc:                     # instrument: reusable once per run
    processItemCheat(cheatNum); instcc = -1; send itemcheatstate
elif CA<cheatNum> > 0:                     # consumable
    online:  __mg.minigameValidateItem(user, cheatNum)
             # the host answers with internal itemCheatActivate:
             #   consUsed = true; processItemCheat(cheatNum)
    offline: processItemCheat(cheatNum); CA0..CA<cheatNum> -= 1;
             clamp CA<k> (k > cheatNum) down to stay non-increasing; send itemcheatstate
```

`handleInternalRequest('itemCheatInfo', info)` (`handleItemCheatInfo`) lets the host push fresh slot counts, which are then sent with `sendItemCheatInfo`.

| Cheat | Effect (`processItemCheat`) |
|---|---|
| 0 | `getHint`, then `timeRemaining += 2`, `timerupdate` |
| 1 | One random integer cell gets `+ getMatchPower(t, 8)`; `powerupconvert` |
| 2 | Clears every cell of the least common type 1-8 present (`minValue` ignores zero counts), expands it with `checkPowerUpMatches`, sets `currentMatches`, zeroes the cells, sends `bypasshexspinner` |
| 3 | Clears `ceil(n / 2)` random blockers (9), with repeats possible, from `n` on the board; if `n = 0` it returns without a message; sends `bypasshexspinner` |
| 4 | For each icon in `bonusIcons ++ goalIcons`, one random cell of exactly that integer type becomes a power-up (`getMatchPower(t, 8)`); one `powerupconvert` each |

### Ability cheats: `abilityBitfield`

`activateAbilityCheatRequest(cheatNum)`: if bit `cheatNum` is set, run `processAbilityCheat`, clear the bit, and send `abilitycheatstate` (confirmed). After any ability cheat, if `abilityAddTime` was already true when the cheat started, add 2 s and send `timerupdate`.

| Bit | Effect |
|---|---|
| 0 | `timeRemaining += 5` |
| 1, 9 | `timeRemaining -= 10`, `timerupdate`; three random power-up conversions; `hintLastTime = timeRemaining`, `hintTimer = 20` |
| 2 | `stopTimerMatch = true`: after each matching swap the timer pauses (`timerState = 0`) until `doscore` |
| 3 | `addTimerMatch = true`: +0.5 s per matching swap |
| 4, 6, 7, 12 | Clear the least common type (as item cheat 2) |
| 5, 14 | Clear every power-up cell, expand, `bypasshexspinner` |
| 8 | `addTimerMatchMove = true`: another +0.5 s per matching swap |
| 10 | `instcc = origInstcc`; `sendItemCheatInfo`, which restores the instrument |
| 11 | `abilityAddTime = true`: +2 s after each later ability cheat |
| 13 | `powerUpAddTime = true`: +2 s per power-up matched |

**Cheat behaviour shared by both kinds:**

- **Cheat clears never score.** `processItemCheat` and `processAbilityCheat` count the cleared types into a local variable and never store it in `currentScores` (confirmed). Cascades that follow a clear do score.
- **`checkForMatchCheat`** runs after every matching swap and applies bits 2, 3 and 8 (above), each only while `timeRemaining > 0`.
- **The hoisting bug in both processors** (inferred, not run). The board-copy loop compares against `playfieldWidth` and `playfieldHeight` before the `var playfieldWidth = ...` statements run. Under AS1 function scoping those are `undefined` at that point, so the copy is empty and board-touching cheats work on an empty array. The port reads the dimensions first.

**The bit-to-ability mapping** (which ability grants which bit) is MG-R7's: spec-launch-flow section 4. The cheat sounds are in [spec-launch-flow, cheat sounds section](spec-launch-flow.md#bit-to-cheat-category-confirmed-goauldcrystals-processabilitycheat).

### Timer cheats `stct`, `upct` and `osct`

`tick` handles three cheat timers (confirmed):

| `timerState` / mode | Property | Effect while active | When it runs out |
|---|---|---|---|
| `timerState == 2` | `StopTimeCheatTime` (`stct`, ms) | Freezes the countdown | `timerState = 1`, `timerupdate` |
| `timerState == 3` | `UpTimeCheatTime` (`upct`, ms) | Runs the countdown backwards | `timerState = 1`, `timerupdate` |
| `moveMode == 2` | `OneStepCheatTime` (`osct`, ms) | — | `moveMode = 0` |

**All three are dead code in Alignment.** No path sets `timerState` to 2 or 3, sets `moveMode` to 2, or writes any of the three properties (confirmed by a string search of the extension). They are Livewire's timers, copied over. The port does not implement them.

## 8. Hints and timers

### Countdown

- The original `tick` runs every 100 ms. It subtracts real elapsed time from `timeRemaining` while `timerState == 1`.
- The port's tick is 250 ms. It subtracts 0.25 s per tick while running.
- The client runs its own countdown and is resynced by `timerupdate`.
- The server sends `timerupdate` only on events, not every tick (confirmed): after a swap cheat, after a score, after a hint, after cheats, and when the pause ends. The port does the same.

### Idle hint prompt (`tick`)

When all of these hold, the server sends `clicktohint`, then sets `hintLastTime = timeRemaining` and `hintTimer = 10`:

- `timeRemaining <= hintLastTime - hintTimer`
- `timeRemaining > 0`
- `timerState == 1`
- `!hintTimerPause`

`initGame` sets `hintTimer = 5`, so the first prompt comes 5 s after start; each later one comes 10 s after the last score, prompt or reset. `swappieces` pauses the prompt, and a no-match or `doscore` unpauses it. The client shows the bubble only while its own `timeRemaining > 0`. Every `swappieces` is preceded by a `resethinttimer`.

### `getHint` (`callhint`, item cheat 0)

- `hintIndex` (0-3, reset to 0 by init) picks the scan order, and is advanced *before* the scan:

  | `hintIndex` | Scan order |
  |---|---|
  | 0 | `x` ascending, `y` ascending |
  | 1 | `x` ascending, `y` descending |
  | 2 | `x` descending, `y` descending |
  | 3 | `x` descending, `y` ascending, then the index wraps to 0 |

- It takes the first cell whose n0 is in bounds and where `spinCheck(cell, n0, n1)` finds a matching rotation, and sends `hint {hex1: cell, hex2: n0, hex3: n1}`.
- n1 is not bounds-checked (shipped bug). The port requires n1 in bounds.
- Only the (n0, n1) triangle of each cell is tried, so the hint can miss legal moves that use other pairs.
- If nothing is found, no message is sent, but `callhint` still costs 5 s.

### No deadlock handling

Nothing reshuffles a board with no legal move. The port keeps that by default, as shipped behaviour; section 11 asks whether to add one.

## 9. Transcripts (fixture board, RNG values frozen on first port run)

These are derived from a Python model of the decompiled rules (static, not captured).

**Fixtures.** Both transcripts use a fixed fixture board, not a seeded one:

```text
base[x][y] = [4, 5, 6][(x + 2 * (y % 2)) % 3]
```

On tier 1 (8x8) this board has no match and no swap that creates one. AL1 must provide a test-only constructor that injects a board (`AlignmentGame::with_board`), because a seeded `StdRng` board cannot be predicted before the port exists.

Every refill value that comes from the RNG is written as `<rng>`. The first port run captures those values and freezes them in the `.mgs` script; the deterministic fields are fixed now.

Cells are written `(x, y, type)`.

### Transcript A: one scoring swap (tier 1, `techcomp` 1, level 0)

Setup: tier 1, `goalTotals [45, 15, 6]`, time 120, `seed` 7, and the fixture board with (2, 2) set to 4.

1. **Start.** `fullgamestate` has `timeRemaining 120`, `timerState 1`, `goalScores [0,0,0]`, `goalTotals [45,15,6]`, `goalIcons [4,5,6]`, `difficulty 1`, `minigamelevel 0` and the board.
2. **Hint check** (no message sent). On this board `getHint` with `hintIndex 0` would return `hex1 (1,1,4)`, `hex2 (0,1,6)`, `hex3 (1,2,5)`.
3. **Swap.** `> resethinttimer`, then `> swappieces row1=1 col1=1 type1=4 row2=2 col2=1 type2=5 row3=2 col3=0 type3=6 xPos=0 yPos=0 orientation=e`. This is cell 1 plus its n3 and n4; the triangle is legal.
   - `< feedback` with `spinCount 1` and `powerUpValues []` precedes `< hexspinner`.
   - `< hexspinner` with `spinCount 1` and `matches [(1,3,4), (2,1,4), (2,2,4), (3,0,4), (3,2,4)]` (in BFS order; the test compares as a set), plus the echoed fields.
   - No `timerupdate`, because no cheat flags are set.
4. **Cascade.** `> cascade` gives `< hexmove` with `direction 0` and:
   - `hexesToMove [{1,0,3,6}, {2,1,2,5}, {2,0,2,4}, {1,1,1,6}, {1,0,1,6}, {1,2,0,5}, {1,1,0,5}, {1,0,0,4}]`, each entry `{moveTotal, x, y, type}`, in walk order for `d = 0` (section 6).
   - `newHexes` at (0,0), (0,1), (0,2), (0,3) and (1,2), each `type <rng>`. With no re-roll needed (`cascadeCount 0`), the refill draws two rolls per cell.
5. **Check.** `> checkForCascade` gives `< cascadecheckcomplete`. This holds only if the frozen refill makes no triangle; if it does, re-pick the seed. `dropIndex` becomes 1.
6. **Score.** `> doscore` gives `< updatescore` with `goalScores [5,0,0]`, `goalTotals [45,15,6]`, `goalIcons [4,5,6]`, then `< timerupdate` with `timeRemaining 120`, `timerState 1` (bonus 0).
7. **Next swap.** A second swap's `hexmove` carries `direction 1`.

### Transcript B: no-match swap, hint penalty and reset (tier 1, level 0)

Setup: the plain fixture board.

1. `> resethinttimer`, then `> swappieces row1=1 col1=1 type1=4 row2=1 col2=0 type2=5 row3=0 col3=1 type3=6 ...` gives `< hexspinner` with `spinCount 3`, `matches []` and the echo. The board is unchanged and the phase stays `Ready`.
2. `> callhint`: the plain board has no move, so there is no `hint`. Then `< timerupdate` with `timeRemaining 115`, `timerState 1`.
3. `> processreset` gives `< fullgamestate` with `numResets 1`, `timeRemaining 120` and the initial board. The board is the injected fixture, because reset copies `gameArrayInit` back.

### Port-only guards for AL2 and AL3

These are not transcripts. Each test is written so that it fails when the guard is removed.

- `swappieces` with a non-adjacent triangle, an out-of-bounds cell, a blocker, or lying `type*` fields: the guard rejects it and the board is unchanged.
- `cascade` sent in `Ready`, and `doscore` sent in `AwaitCheck`: both are rejected.
- `dovictory` before `activate`: no `Victory`. After `activate`: `victory` and `Victory`.
- Run `tick` until the time is up: `failure` and `Failure`, and nothing is processed afterwards.

## 10. Port slicing

The layout follows C4: `crates/minigame/src/minigame/games/alignment/`. All randomness comes from one `StdRng` in the game struct, and every array goes through `SfsValue::list`. Line counts are estimates of Rust including doc comments. They are measured against Livewire, whose `mod.rs` has 405 lines and `setup.rs` 373.

### MG-AL1: setup and the state shell (~650 lines)

Files:

| File | Contents | Lines |
|---|---|---|
| `alignment/mod.rs` | `AlignmentGame`; the `Phase` enum (section 5); `MinigameInstance` with `started` (`fullgamestate`), `needs_tick`, `aborted`; `message` dispatch with `processreset` and every other command rejected as not-yet-ported; `with_board` for tests; `PORTED` stays `false` | ~200 |
| `alignment/setup.rs` | Tier table and goal totals (section 4); clamp tier to 1-4 and level to 0-10; `select_icons` (section 3) on `StdRng`; the `fullgamestate` builder; board to `SfsValue` | ~250 |
| `alignment/tests_setup.rs` | The tables match section 4 (spot rows per tier, level clamping); the first quarter of cells is bonus icons; no initial matches; seed determinism; `fullgamestate` keys and `timeRemaining` minus 0.25; reset returns the initial board and counts `numResets` | ~200 |

Fits comfortably.

### MG-AL2: geometry, swap and cascade (~800 lines)

Files:

| File | Contents | Lines |
|---|---|---|
| `alignment/moves.rs` | Neighbours; `check_for_matches`; flood; `spin_check`; the power-up expansion and `get_match_power`; `swap_pieces` with the D-MG2 triangle validation; `check_for_cascade`; refill with A/B lists, blocker and clone rolls; `get_new_type` (tier override behind `const NEW_TYPE_USES_TIER: bool = false`); re-roll pass; `feedback` (sent first, before `hexspinner` or `bypasshexspinner`), `hexspinner`, `bypasshexspinner`, `hexmove`, `cascadecheckcomplete`; wire into `message` with phase checks; unit test: a power-up pulled in by another power-up fires its own effect, and no cell is added twice | ~450 |
| `alignment/moves_gravity.rs` | The six `doCascade` slide walks (section 6), split off to keep `moves.rs` under the 500-line cap | ~150 |
| `alignment/tests_moves.rs` | The neighbour table for both parities; the match pair rule; transcript A steps 3-5 against the fixture; one test per gravity direction on a small hand board; power-up shapes p = 1-6; refill A/B power-up grant; swap validation rejects; phase order rejects | ~350 |

AL2 is the largest slice. If it runs over about 40k tokens, cut it at the line between `moves.rs` and `moves_gravity.rs` plus their tests (AL2a swap and matching, AL2b cascade and refill).

### MG-AL3: score, end, hints, cheats (~700 lines)

Files:

| File | Contents | Lines |
|---|---|---|
| `alignment/rules.rs` | `do_score` and `check_for_victory` (`activate`); `dovictory` gated on goals, giving `victory` and `GameOutput::Victory`; `tick` (countdown, timeout `failure` and `GameOutput::Failure`, the `clicktohint` idle prompt); `get_hint` with the n1 bounds fix; `callhint` and `resethinttimer`; item cheats 0-4 with `instcc`/`CA` bookkeeping (online path: treat a granted slot as validated, set `consUsed`); ability cheats; `check_for_match_cheat`; terminal end state; **flip `PORTED = true`** | ~400 |
| `alignment/tests_rules.rs` | Multiplier and bonus per `cascadeCount`; power-ups not scored; type 7 not scored; `activate` once at goals; `dovictory` gate; timeout; idle prompt timing; hint order rotation; each cheat's effect and its state message; cheat clears not scored; `stopTimerMatch` pause and resume | ~300 |
| `crates/minigame/tests/scripts/alignment/{a_scoring_swap,b_reset}.mgs` | Transcripts A and B (C6), with `<rng>` values frozen from the port | ~40 |

`GameKind::validation()` reaches Alignment only after AL3. The Activate alias (D-MG3 option B) is a separate content and kind packet; it is not part of AL3.

## 11. Open questions

1. **The `getNewType` tier override.** Should refills use the tier tables? The shipped extension forces tier 1, which keeps protocol (7) and cipher (8) out of refills and makes tier 4 cipher totals close to unreachable. The port ships the original behaviour behind `NEW_TYPE_USES_TIER = false`. **Owner decision.**
2. **Resolved:** `getMatchPower` with `mp = 12` gives 0.6 for types 1-3 (the same branch as type 8; section 6).
3. **Deadlock reshuffle.** The original has none. Should the port add one, as a server-side improvement, or keep the run-out-the-clock behaviour?
4. **Cheat 3 with no blockers.** It returns without any message. The client put up its `disable` overlay when it sent the cheat, and it is not clear what removes it; the client may stay locked. The port sends `itemcheatstate` regardless. A lab check is needed once cheats are reachable.
5. **`consUsed` and item validation.** The online path relies on the host (`__mg.minigameValidateItem`) to consume the item. Cimmeria has no item-validation call into the cell yet. AL3 treats a granted `CA` slot as validated. A real consumable spend belongs to the launch-flow and items work (spec-launch-flow section 5).
6. **The hoisting bug.** Whether board-touching cheats were truly broken online in 2009 needs a run under an AS1 VM; it is inferred from scoping rules only. The port fixes it either way.
7. **Gravity details for directions 1, 2, 4 and 5.** The section 6 table is a map. AL2 ports the walks line for line from the listing and verifies each with a hand board; any mismatch with the table is a doc fix here.
8. **Activate, Repair and Search as Alignment skins.** The `gameType` strings are the only evidence. Whether the Activate shell's content expects Alignment's goals and timer is a content question for the D-MG3 alias packet.
