# ConverseBasicHumanoid rules spec (MG-R4)

Status: draft for review. Source: static AS2 disassembly (R0 tool, `tools/minigame-swf`) of the embedded server extension and the client code in `ConverseBasicHumanoid.gfx`. Nothing here was run in the lab or the client. Labels: **Confirmed** means read directly from the bytecode; **Inferred** means reasoned from it and not run.

The brief guessed the play rules were `Match` and `Swap`. They are not. The game is a timed card contest: each side plays one card per container, and a suit beats the next four suits round a circle of nine. The NPC side is played by the extension itself.

## 1. Verdict

A real game. The extension is complete: deal, a server-side opponent, scoring, hand and deal timers, and the host completion call.

| Package | GFX offset | Length | md5 (extracted `.gfx`) | AS2 functions |
|---|---|---|---|---|
| ConverseBasicHumanoid.upk | 5649 | 227957 | 12553e9639b3fae147e3f0944c7f77c5 | 382 |
| Converse.upk | 1583 | 55795 | 25931bc75ec2d06289057f530b0af6f8 | 183 |

The extension is the root frame-3 script block of the movie. Its functions, by R0 tool name (`python -I tools/minigame-swf/list_functions.py <gfx> --function <name>`):

- Entry points: `handleRequest`, `handleInternalRequest`, `onUserJoin`, `tick`.
- Setup: `initGame`, `createGameType`, `getAccords`, `getVictoryConditions`, `randomNpcCards`, `randomPlayerCards`, `randomIndex`, `randRange`.
- Play: `playerMove`, `updateCards`, `npcMove`, `evalWeight`, `evalWeights`, `maxValue`, `minValue`.
- Hand and score: `newHandMessage`, `processScore`, `scoreMessage`, `clearContainers`, `processReset`.
- End: `checkForVictory`, `failMessage`.
- Cheat and accord: `activateVictory`, `activateAccord`, `checkForCheats`, `turnNpcCards`, `sendTimerUpdate`, `updateMatchScore`.
- Full state: `sendFullGameState`.

### Is Converse a dead parent? (D-MG3)

No. `Converse.upk` is a separate 55 KB shell, the same family as Activate, Analyze, Bypass and Hack (Confirmed):

- Its functions are the SmartFoxClient library (`init14`..`init23`), the frame-2 local bridge (`sendMessage`, `get`, `put`, `sendResponse`, ...), and on frame 3 only `onVictory`, `onFailure`, `onResponse`, the timer functions and the `win_btn` `onRelease`.
- It has no `handleRequest` and no game code.
- ConverseBasicHumanoid does not load it. The string `ConverseBasicHumanoid` in the CBH movie is its own SFS zone name, not a reference to a parent.

So for D-MG3, Converse is an unrelated stub that shares a name prefix. Porting CBH needs nothing from it; Converse belongs with the other shells (MG-R5).

## 2. `joinOK` params read

All are read by the **extension**, not the client, through `room.getVariable(name).getValue()` (Confirmed, `getAccords` and `getVictoryConditions`).

| Name | Type | Use |
|---|---|---|
| `intelligence` | number | Added to the player's accord chance, in percent points. |
| `techcomp` | number | Subtracted from both sides' accord chance (`pclevel - techcomp`). Also read in `getVictoryConditions`, then overwritten (dead). |
| `pclevel` | number | Added to both sides' accord chance. |
| `abilityBitfield` | number (u32) | Bits 15 to 18 add the Trump bonus (section 7). |

Not read: `difficulty`, `seed`, `instcc`, `CA0`..`CA4`. The tier is not taken from `difficulty` (section 4).

Correction to MG-R6: `spec-protocol-common.md` Q2 lists the CBH reader as "client". The reader is the extension's `getAccords`. The client movie never reads these four variables.

Port: map from the C3 session fields: `intelligence`, `tech_competency`, `player_level`, `abilities_mask`.

## 3. Board model

### Cards and suits

A card is a number. Its suit is `floor(card)`, one of 0 to 8. An accord card is a suit plus `.1` or `.2` (for example `4.1`), and plays as its floor suit (Confirmed).

There are no `Card` or `Suit` classes; the brief's names do not exist in the movie. The hand is a sorted Array of numbers.

### Containers

Each game type has `containerTotal` containers per side. One hand fills each container once from each side. A deal is `contestTotal` hands, so a deal uses exactly `cardTotal = contestTotal * containerTotal` cards per side (Confirmed, `createGameType`).

State per side: the hand, `EmptyContainers` (0 empty, 1 used, per container), `ContestPlays` (cards played this hand, in play order) and `TotalPlays` (cards played this deal).

### Generation (Confirmed, `initGame` and `random*Cards`)

```text
deck    = suits of this game type, each twice        # createGameType
accords = x.1 and x.2 for each suit
npc    = draw(cardTotal, npcAccord)                   # NPC first, from the shared deck
player = draw(cardTotal, playerAccord)

draw(n, hasAccord):
  hand = []
  if hasAccord: a = take random from accords; n = n - 1
  repeat n: hand.push(take random from deck)          # splice: no replacement
  if hasAccord: hand.insert(randRange(0, len(hand) - 1), a)
  hand.sort()                                         # AS2 string sort; same as numeric for 0..8.x
```

`randomIndex` is `floor(random * len)`; `randRange(min, max)` is `floor(random * (max - min + 1)) + min`. The original draws from `Math.random` in this path (the embedded copy is the offline one). No client reproduces the deal from `seed`: the client renders `fullgamestate` (Confirmed). The port draws everything from `StdRng::seed_from_u64(session.seed)`, in this order: NPC hand, then player hand.

## 4. Difficulty table

The game ignores `difficulty`. `initGame` always sets `skillLevel = 'beginner'` and `npcType = 'humanoid'` (Confirmed). The game type steps up on each new deal instead.

| Game type | Reached | cardTotal | containerTotal | contestTotal | Deck suits (each twice) |
|---|---|---|---|---|---|
| logic3 | first deal | 3 | 1 | 3 | 0, 4, 8 |
| mediation5 | every later deal | 4 | 2 | 2 | 0, 3, 4, 5, 8 |
| courtesy7 | unreachable | 6 | 2 | 3 | 0, 2, 3, 4, 5, 7, 8 |
| humor9 | unreachable | 9 | 3 | 3 | 0 to 8 |

Accord pool: `s.1` and `s.2` for each deck suit (logic3: 0.1, 0.2, 4.1, 4.2, 8.1, 8.2).

Progression (Confirmed, `initGame` with `isFirstTime == false`): logic3 becomes mediation5, and mediation5 stays mediation5. The code has a courtesy7-to-humor9 step, but nothing reaches courtesy7.

Global numbers (Confirmed):

| Value | Number |
|---|---|
| Countdown (`initialTime`) | 30 s; server value starts at 30.25 |
| NPC first move after a deal | 3 s |
| NPC move after a hand | 1.5 s; 0.5 s when answering the player into an empty container |
| Hand length (`handTimer`) | 6 s |
| Extension tick | 100 ms, wall-clock delta |

No Python reference exists for this game.

## 5. Message table

### Client to server (`handleRequest`, Confirmed)

Messages from a spectator are ignored.

| Command | Fields (`SfsValue`) | Original action | Port validation and reply |
|---|---|---|---|
| `playermove` | `num` Number, `suit` Number, `container` Number or absent | `playerMove` | Running; contest not full; `suit` is in the hand (by value); `container` absent or in range and empty. Reply `updatecards`. Invalid: no reply, WARN. |
| `newhand` | none | `newHandMessage` (ends the hand now) | Running. Reply as for a hand timeout. |
| `processreset` | none | `processReset` | Only after `newdeal`. Reply `fullgamestate`. Otherwise ignore. |
| `activatevictory` | `victoryNum` Number, `pointCost` Number | `activateVictory(..., 'player')` | See section 7. |
| `failure` | none | `failMessage`: sends `failure`, completes with code 4 | The client never sends it. Port: treat as Defeat (code 4, cell result 2), or drop it with a stated reason. |

Correction to MG-R6: the CBH row of `spec-protocol-common.md`'s message table leaves `failure` out of "Client sends". `handleRequest` accepts it (Confirmed).

The original validates nothing in `playerMove`. A bad `num` or `suit` splices the wrong card or nothing.

`num`: the client sends the card's index at deal time, which is stale after earlier plays. The server splices `playerCardsArray` at `num` (Confirmed). The port must find the card by `suit` value, remove that card, and echo the client's `num` in `updatecards`.

### Server to client

| Command | Fields | Sent by |
|---|---|---|
| `fullgamestate` | `numResets`, `timeRemaining` (minus 0.25), `timerState`, `isSpectator`, `npcType`, `npcScore`, `npcCardsArray`, `npcTotalPlays`, `npcContestPlays`, `playerCardsArray`, `playerScore`, `playerTotalPlays`, `playerContestPlays`, `cardTotal`, `containerTotal`, `playerVictoryConditions`, `gameType` | `sendFullGameState`, at join and after each deal |
| `updatecards` | `user` (`player` or `npc`), `num`, `suit`, `container`, `playerTotalPlays`, `npcTotalPlays` | `updateCards`, after each play |
| `updatescore` | `npcScore`, `playerScore` | `scoreMessage`, at each hand end and after `activatevictory` |
| `emptyhand` | none | `newHandMessage`, hand over, deal goes on |
| `newdeal` | none | `newHandMessage`, deal over |
| `activatevictory` | `pScore`, `nScore`, `victoryNum`, `user` | `activateVictory` |
| `victory` | none | `checkForVictory` |
| `failure` | none | `failMessage` |
| `spectator` | none | `onUserJoin` for a spectator |

Types: arrays go out as `SfsValue::list` of numbers; `npcType` and `gameType` are strings; `isSpectator` is a bool; the rest are numbers.

`fullgamestate` also sends `suitTypes` and `suitTypesTotal`, which are never set (undefined). The port omits them.

Not sent: `tie` (the client has a handler, the server never sends it), and `timerupdate`. `sendTimerUpdate` runs only when a stop-time or up-time cheat ends, and nothing starts one.

Client behaviour (Confirmed, client code): a click sends `playermove` without `container` (`cardRelease`); a drag onto a slot sends it with `container`. The client answers `newdeal` with `processreset`. Its countdown is a local display only and does not call `onFailure` at zero, so the MG-R6 Q4 drop race does not apply to this game.

## 6. Scoring, win and loss

### Dominance (Confirmed, `processScore` and `evalWeight`)

```text
d = (floor(p) - floor(n) + 8) % 9     # p player card, n NPC card
d == 8      -> tie (same suit)
d < 4       -> NPC wins the container
otherwise   -> player wins the container
```

So each suit beats the next four suits round the circle: s beats s+1 to s+4 (mod 9). In logic3, 0 beats 4, 4 beats 8, and 8 beats 0.

### Hand end (Confirmed, `newHandMessage` then `processScore`)

The hand ends when `handTimer` (6 s) runs out or the client sends `newhand`. In order:

1. Deal over (both sides' `TotalPlays` equal `cardTotal`): reply `newdeal`.
2. The player's `TotalPlays` list is empty: `failMessage`. (The test is `playerTotalPlays.length == 0 && contestTotal >= 1`; `contestTotal` is always 2 or more, so the second half is always true.)
3. The NPC has no cards and the player played nothing this hand: `failMessage`.
4. The NPC played this hand and the player did not: `failMessage`. An idle hand loses the game.
5. Otherwise: reply `emptyhand`.

For 1 and 5, before the reply: `processScore`, then `clearContainers`.

`processScore` pairs the two `ContestPlays` arrays by play order, index 0 to `containerTotal - 1`, not by container:

- One side played and the other did not: a point to the side that played.
- Same suit, or neither played: no point.
- Otherwise the dominance rule above.
- Sweep: a side that won every container gets +1 more. In logic3 a won hand is worth 2.

It then sends `updatescore`. Scores carry over across deals.

A missing card on the other side is `undefined`, not -1, so the "one side did not play" tests compare against -1 and miss. The comparison falls through to "player wins" (Inferred from AS2 NaN rules). In practice: if the NPC did not answer, the player scores; if the player did not answer, rule 2 (no play yet this deal) or rule 4 fails the game first.

### Deal end

On `newdeal` the client sends `processreset`. `processReset` (Confirmed): if `floor(timeRemaining) > 0`, it adds 1 to `numResets` and to `contestsWithoutAccord`, runs `initGame(false)` (next game type, new accords, new deal, keeps scores and time) and sends `fullgamestate`. Otherwise it calls `checkForVictory`.

### Timeout and result

At `timeRemaining <= 0` the tick sets `timerState = 0` and calls `checkForVictory` (Confirmed):

```text
checkForVictory:
  if npcScore > playerScore: failMessage            # loss
  else:                                             # ties win
    send victory
    minigameCompleteGame(user, 3, timeRemaining / initialTime, numResets + 1, instr, cons)

failMessage:
  timerState = 0
  send failure
  minigameCompleteGame(user, 4, 0, numResets + 1, false, false)
```

`instr` and `cons` come from user properties that nothing sets, so both are false in practice. The time ratio is 0 or below at a timeout; the port clamps it to 0.

Port result: extension code 3 maps to cell result 1 (Victory), code 4 to cell result 2 (Defeat), as `spec-protocol-common.md` Q4 proposes. Victory and failure are both terminal; the port stops ticking. The original keeps its interval running after the end.

## 7. Cheats

### Trump passives 778, 779, 792 and 793 (Confirmed bits, Confirmed names)

They enter through `abilityBitfield`, in `getAccords`:

| Bit | Mask | Accord bonus (percent points) | Ability (Confirmed, abilities.sql descriptions) |
|---|---|---|---|
| 15 | 32768 | +5 | 778 Mediator |
| 16 | 65536 | +10 | 779 Negotiator |
| 17 | 131072 | +15 | 792 Diplomat |
| 18 | 262144 | +25 | 793 Ambassador |

The bonuses add up. The four bits sit above the 0-14 cheat mask (`spec-launch-flow.md` section 4), so other games' `setupAbilityCheats` never see them.

For MG-22: OR bit 15 into `abilities_mask` when the player knows 778, bit 16 for 779, bit 17 for 792 and bit 18 for 793. The order follows the bonuses in the ability descriptions (abilities.sql ids 778, 779, 792, 793; Confirmed): each description's "+N% for Trump in Converse" matches exactly one bit's bonus in `getAccords`.

Correction to MG-R7: `spec-launch-flow.md` section 4 says "The Trump passives are not bits". They are bits 15 to 18 of `abilityBitfield`, read by this extension's `getAccords`.

### Accords (Confirmed, `getAccords`)

```text
npcPct    = 5 + contestsWithoutAccord + (pclevel - techcomp)
playerPct = 5 + trump + intelligence + contestsWithoutAccord + (pclevel - techcomp)
npcAccord    = max(npcPct / 100, 0.01) * cardTotal >= 1
playerAccord = (playerPct / 100) * cardTotal >= 1
```

This is a threshold, not a roll. With an accord, that side's next deal holds one accord card (section 3).

An accord card scores as its floor suit. `activateAccord` and `checkForCheats` (the accord effects, and the code that sets `opponentFaceUp`) are never called, so in the shipped build an accord changes nothing on the server. The client shows an `accord_effect` visual and counts it.

### Victory conditions (Confirmed, `getVictoryConditions` and `activateVictory`)

Names, in order, with point costs: focusdamage1 3, focusdamage2 4, fear 6, disorient 7, confuse 8, defeat 10, fearaoe 10, disorientaoe 12, confuseaoe 14, defeat_disorient_aoe 16. Beginner gets the first two, experienced six, advanced all ten. Only beginner is reachable, so the player's list is `[focusdamage1, focusdamage2]`, costs `[3, 4]`, stored server-side as `playerPointCosts`.

The NPC lists are built from a `randRange(1, 60)` draw, with a cost and name mismatch bug in one branch, and are never used. The port drops them.

`activatevictory` (original): subtract the client's `pointCost` from `playerScore`; return if `npcScore < 0`; send `activatevictory` and `updatescore`. The client button for slot i (1-based) sends `victoryNum = i` and `pointCost` from `vc.pointCost`, which the movie never assigns (Confirmed: read, never written). So the original sends `undefined`, the score becomes NaN, and `npcScore > NaN` is false: the game ends in a victory (Inferred).

On the client, `activatevictory` for `user = player` calls `FSCommand:ConvEffTarget <victoryNum>`, shows the effect and disables the button. `victoryNum == 6` (`defeat`) wins at once; beginner never offers it.

Port: take the cost from `playerPointCosts[victoryNum - 1]`, ignore the client's `pointCost`, refuse when `victoryNum` is out of range, already used, or `playerScore` is below the cost (no reply, WARN). On success subtract, send `activatevictory {nScore, pScore, user=player, victoryNum}` then `updatescore`. This matches `spec-protocol-common.md` Q4's ruling that `activatevictory` is gated.

### Item cheats

`handleInternalRequest` routes `itemCheatInfo` and `itemCheatActivate` to `handleItemCheatInfo` and `processItemCheat`, which are not defined in this movie (Confirmed). The path is dead. `instcc` and `CA0`..`CA4` are not read. The port has no item cheats.

## 8. Hints and timers

No hints. The server runs one 100 ms tick (the port uses the 250 ms session tick):

```text
tick(dt):
  if timerState == 1: timeRemaining -= dt
  if timeRemaining <= 0 and timerState >= 1: timerState = 0; checkForVictory; (the original keeps the interval running; the port stops, see the end-of-game rule)
  if timeRemaining <= npcLastTime - npcTimer:
    npcMove; npcLastTime = timeRemaining; npcTimer = 1.5   # even if npcMove did nothing
  if timeRemaining <= handLastTime - handTimer:
    newHandMessage; handLastTime = timeRemaining; handTimer = 6
```

`timerState` 2 (stop time) and 3 (up time) exist in `tick` but nothing sets them; the port leaves them out.

### NPC opponent (Confirmed, `npcMove`, `evalWeight`)

```text
npcMove:
  if NPC contest full: return
  if focusContainer set:            # player has played into a container the NPC has not
    card = best answer to focusSuit; container = focusContainer
  else:
    card = argmax over hand of evalWeights; container = first empty NPC container
  play card; send updatecards (num = index in the NPC hand)

evalWeight(c, pm):  d = (floor(pm) - floor(c) + 8) % 9
  d == 8 -> +0.5;  d < 4 -> +1;  else -> -1
evalWeights(c) = sum of evalWeight(c, pm) over the player's current hand
```

`maxValue` is strict, so ties go to the lowest index. "Best answer to `focusSuit`" is the argmax of `evalWeight(c, focusSuit)` over the NPC hand. `playerMove` sets the focus, with `npcTimer = 0.5`, when the player plays into a container the NPC has not filled.

The AI fits in CH2 with the move code. No separate CH2b is needed.

## 9. Golden transcripts

Timing, at the 250 ms REPL tick: `timeRemaining = 30.25 - 0.25 * ticks`. The first NPC move is at tick 12; each hand ends 24 ticks after it starts. Card values depend on the seeded deal. MG-CH3 runs each script with `--record`, picks or confirms the seed, and fills the bracketed values. These are untested plans.

### A. Idle hand loses (seed 7, tier 1)

```text
game ConverseBasicHumanoid difficulty 1 seed 7
start
< fullgamestate {...gameType=logic3, numResets=0, timeRemaining=30, ...}
tick 12
< updatecards {container=0, npcTotalPlays={0=<n>}, num=<i>, playerTotalPlays={}, suit=<n>, user=npc}
tick 12
< failure
```

Asserts rule 2 of the hand end (the player has no play this deal, checked before rule 4) and result code 4 (cell Defeat).

### B. Player wins the first deal (seed found by recording, tier 1)

B has no seed yet: MG-CH3 must find one; until then only A is a golden.

Needs a deal of NPC `[0, 4, 8]` and player `[0, 4, 8]` with no accords.

```text
game ConverseBasicHumanoid difficulty 1 seed <s>
start
< fullgamestate {...}
tick 12
< updatecards {container=0, npcTotalPlays={0=0}, num=0, playerTotalPlays={}, suit=0, user=npc}
> playermove num=2 suit=8
< updatecards {container=0, npcTotalPlays={0=0}, num=2, playerTotalPlays={0=8}, suit=8, user=player}
tick 12
< updatescore {npcScore=0, playerScore=2}
< emptyhand
tick 6
< updatecards {container=0, npcTotalPlays={0=0, 1=8}, num=1, playerTotalPlays={0=8}, suit=8, user=npc}
> playermove num=1 suit=4
< updatecards {container=0, npcTotalPlays={0=0, 1=8}, num=1, playerTotalPlays={0=8, 1=4}, suit=4, user=player}
tick 18
< updatescore {npcScore=0, playerScore=4}
< emptyhand
tick 6
< updatecards {container=0, npcTotalPlays={0=0, 1=8, 2=4}, num=0, playerTotalPlays={0=8, 1=4}, suit=4, user=npc}
> playermove num=0 suit=0
< updatecards {container=0, npcTotalPlays={0=0, 1=8, 2=4}, num=0, playerTotalPlays={0=8, 1=4, 2=0}, suit=0, user=player}
tick 18
< updatescore {npcScore=0, playerScore=6}
< newdeal
> processreset
< fullgamestate {...gameType=mediation5, numResets=1, timeRemaining=12, ...}
abort
```

Why: in hand 1 every NPC card weighs 0.5, so it plays index 0 (suit 0); 8 beats 0. In hand 2 the NPC plays 8 (weight 0 against -0.5); 4 beats 8. In hand 3, 0 beats 4. Each hand is a sweep, so it scores 2.

### C. Rejected inputs (any seed, tier 1)

```text
game ConverseBasicHumanoid difficulty 1 seed 7
start
< fullgamestate {...}
> playermove num=0 suit=7
> playermove num=0 suit=<own suit> container=5
> activatevictory victoryNum=1 pointCost=0
> processreset
tick 12
< updatecards {...user=npc}
abort
```

Asserts: a suit not in the hand, an out-of-range container, an unaffordable victory condition (score 0, cost 3) and a `processreset` outside a deal end all give no output.

## 10. Port slicing

Folder `crates/minigame/src/minigame/games/converse_humanoid/`, game name `ConverseBasicHumanoid`. Each slice fits about 40k working tokens.

| Slice | Files | Lines | Content |
|---|---|---|---|
| CH1 | `mod.rs` (~120), `setup.rs` (~250) | ~370 plus ~150 tests | State struct, game-type table, deck and accord pools, seeded deal, `getAccords` with the Trump bits, player victory conditions and costs, `fullgamestate`. Tests: table values, deal sizes, accord threshold, Trump bit sum. |
| CH2 | `moves.rs` (~250) | ~250 plus ~200 tests | `playermove` validation and `updatecards`, `npcMove` with `evalWeight` and the focus answer, hand end, `processScore` with the sweep and the pairing rule, `clearContainers`. Tests: dominance table, AI choice, each hand-end branch. |
| CH3 | `rules.rs` (~200) | ~200 plus goldens | Tick timers, `newhand`, `processreset` and progression, `checkForVictory`, `failMessage`, the gated `activatevictory`, the result mapping, `PORTED = true`, the `.mgs` goldens A to C. |

Order: CH1, then CH2, then CH3. CH2 and CH3 both touch `mod.rs` dispatch; give CH3 the dispatch.

## 11. Open questions

1. `FSCommand:ConvEffTarget` and `ConvEffSelf`: what the host client does with them (a combat effect on the conversation NPC?). No repo match. Needs Ghidra on the GFx FSCommand handler.
2. Units of `intelligence` and `tech_competency`. If either is a large stat, the threshold makes an accord certain (or impossible) every deal. Check the values Rust sends in `joinOK`.
3. Keep or drop the "unanswered card scores for the player" fall-through, and the `newhand` command that ends a hand early (both favour the player)? This spec keeps both as the original behaves.
4. The pairing by play order, not by container, matters only in mediation5. Keep it (faithful) or pair by container (as the client draws it)?
5. The time ratio at a timeout is 0 or below; this spec clamps it to 0.
6. After `newdeal`, if the client never sends `processreset`, the original's tick keeps ending empty hands, and the NaN fall-through scores them for the player (Inferred). The port should hold the timers until `processreset`, or end the game at the countdown.
