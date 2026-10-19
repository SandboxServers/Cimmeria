# Minigame common protocol spec (MG-R6)

Status: draft for review. Source: AS2 disassembly (R0 tool, `tools/minigame-swf`) of the ten minigame GFX movies. Nothing here was run in the lab or the client.

This answers the six MG-R6 questions. SFS 1.x framing and the login, join, `xtReq` flow are covered by `crates/minigame/src/minigame/protocol.rs` and are not repeated.

## Evidence

Each movie is one uncompressed Scaleform GFX embedded in `CookedPC\UI\Flash\<Game>.upk`. The `.gfx` was cut out of the package and disassembled outside the repo.

| Package | Offset | Length | md5 |
|---|---|---|---|
| Livewire.upk | 11232 | 410721 | 7fbe1510a58b120c7776f4b6d78d62f2 |
| Alignment.upk | 4628 | 293868 | f996c4242dec54bfa3cd6689afd7c2e1 |
| ConverseBasicHumanoid.upk | 5649 | 227957 | 12553e9639b3fae147e3f0944c7f77c5 |
| GoauldCrystals.upk | 1637 | 154559 | 81c70b6d28b47455c3023ef0a9010ddb |
| CrystalGame.upk | 7508 | 94203 | 9c1930c0818222986678cecaf280d581 |
| Activate.upk | 1583 | 55840 | 74520bfa6aed2496e3a9109f71737842 |
| Analyze.upk | 1575 | 55806 | 8207148dc178533f5a94af97114fa76a |
| Bypass.upk | 1567 | 55887 | 846b86e358ef93cfa5246d36ecd59093 |
| Converse.upk | 1583 | 55795 | 25931bc75ec2d06289057f530b0af6f8 |
| Hack.upk | 1551 | 55760 | ab34a898ce64a27f9e783142724aa3b6 |

Locators below are function names of the R0 tool: `python -I tools/minigame-swf/list_functions.py <Game.gfx> --function <name>` lists them (the frame-2 script is the exception; read it with `as2dis.py`). The md5 values are of the extracted `.gfx`, not of the `.upk`.

Of the ten movies, five carry a real embedded extension (Livewire, Alignment, CrystalGame, GoauldCrystals, ConverseBasicHumanoid). Activate, Analyze, Bypass, Converse and Hack are near-identical 55 KB shells with a `win_btn` and a stub extension.

## Q1. Does any SWF run its embedded extension locally?

Yes, all ten. It is the same frame-2 code in every movie (the frame-2 script, read with `as2dis.py`):

```text
_root.online = true;
if (_root.ipaddress == '') {
  if (_root.v1 == undefined) {
    if (_root.ipaddress.value == undefined) { _root.online = false; }
  }
}
```

Condition: all three hold: `_root.ipaddress` is the empty string, `_root.v1` is undefined, and `_root.ipaddress.value` is undefined (raw disassembly of the frame-2 script, read after `push 'online', True`). A launch that passes no `ipaddress` at all (undefined, not '') does not satisfy the first test (undefined == '' is false in a v8 movie) and stays online; only an empty-string `ipaddress` goes offline. Not run in the client. Then `online = false`, `_root.sendMessage` calls `_root.handleRequest(cmd, obj, user, 'roomid')` directly and `_server.sendResponse` calls `_root.onResponse(msg, 'xml')`. No socket is opened. In that mode the extension draws all random values from `Math.random`, picks a random techcomp 1-55, difficulty 1-4, CA0..CA4 = 4,3,3,2,2 and a random abilityBitfield.

Online path: `sfs.connect(ipaddress.value, port.value)`, `handleConnection` calls `login(gamename.value, playerid.value, ticket.value)`. `loginSucceeded` goes to frame 3, `loginFailed` goes to frame 4 (failure). Messages arriving before frame 3 registers `onResponse` are queued in `pendingMsgs` and replayed.

D-MG2 ruling: not needed if the launcher always passes a non-empty `ipaddress`; MG-R7 owns that check (see spec-launch-flow.md). If the variable arrives as the empty string the movie plays a local game with no server.

## Q2. `joinOK` room variables by game

Read by the movie (client code or its embedded extension) from the room or user properties after `joinOK`. `seed` is read by the extension only (see Q3); every other variable is read by client code.

| Game | Variables read | Reader |
|---|---|---|
| Livewire | abilityBitfield, CA0..CA4, difficulty, instcc, pclevel, techcomp; seed | client; seed: extension |
| Alignment | same set as Livewire | client; seed: extension |
| ConverseBasicHumanoid | abilityBitfield, intelligence, pclevel, techcomp | client |
| CrystalGame | none | - |
| GoauldCrystals | seed (extension: initGame, placeGoals, randomizeCrystals) | extension |
| Hack | none | - |
| Activate | none | - |
| Analyze | none | - |
| Bypass | none | - |
| Converse | none | - |

Rust `joinOK` (`crates/minigame/src/minigame/server/session_loop.rs` 196-224) sends abilityBitfield, seed, difficulty, techcomp, pclevel, intelligence, instcc=-1 and CA0..CA4=0 for every game. That is a superset of what each game reads, so no change is needed for compatibility. instcc and CA0..CA4 are the item and ability cheat inventory; Rust hardcodes them, so cheats are never offered (see the parity doc).

## Q3. Does any SWF derive state from `seed`?

No client code does. `seed` is consumed by the server-side extension only (Livewire `initGame`, `processItemCheat`, `processAbilityCheat`; Alignment `initGame`, `doCascade`, `processItemCheat`, `processAbilityCheat`), which does `new Packages.java.util.Random()` then `setSeed(seed)`. In online mode the client renders what the server sends (`fullgamestate`, `create`, `destroy`). Offline it uses `Math.random`. CrystalGame, ConverseBasicHumanoid and the shells use `Math.random` only. GoauldCrystals' extension also reads `seed` (initGame, placeGoals, randomizeCrystals), so MG-R-series work on that game needs the `java.util.Random` port too.

Answer for MG-25: no PRNG port needed for client compatibility. Only an exact reproduction of the original boards would need a `java.util.Random` LCG port (48-bit LCG, multiplier 0x5DEECE66D, `nextInt(n)`). The original helpers were `randRange(rng,min,max) = floor(rng.nextInt(max-min+1)) + min` and `randomIndex(rng,arr) = arr[rng.nextInt(arr.length)]`. Rust uses `StdRng::seed_from_u64` (`games/livewire/mod.rs` 132), which is fine because no client reproduces the board.

## Q4. End of game

### Commands

| Game | Client sends | Server sends |
|---|---|---|
| Livewire | opendoor, closedoor, processover, processout, processmove, processreset, activateitemcheat, activateabilitycheat | victory, failure, fullgamestate, create, destroy, timerupdate, ... |
| Alignment | swappieces, cascade, checkForCascade, callhint, resethinttimer, processreset, doscore, dovictory, cheats | victory, failure, updatescore, fullgamestate, timerupdate, ... |
| CrystalGame | processmove, processreset, cheats | victory, failure, updateslot, fullgamestate, timerupdate |
| GoauldCrystals | playfieldclick, processreset, cheats | victory, failure, squarestate, powerstatus, selectedupdate, fullgamestate, timerupdate, ... |
| ConverseBasicHumanoid | newhand, playermove, activatevictory, processreset | victory, failure, tie (handled by client), newdeal, updatecards, updatescore, fullgamestate, timerupdate, ... |
| Hack, Activate, Analyze, Bypass, Converse | `victory {}` from `win_btn` | victory, failure, fullgamestate |

Victory and failure are server-sent in every game. The client never reports completion to the server except:

- Shells: `win_btn` sends `victory`, and online the client waits for the server to answer `victory` or `failure`. Trap: if MG-25 or a shell implementation echoes the client's `victory`, the player wins by clicking a button.
- Alignment: the client sends `doscore` and `dovictory`. These are client-reported scores and a win trigger. A server-authoritative Alignment must ignore `dovictory` and compute score from `swappieces`.
- ConverseBasicHumanoid: `activatevictory` is the cheat or debug trigger, to be gated like the other cheats.

### Client behaviour on victory and failure

Livewire `onVictory` and `onFailure` both call `_global.sfs.disconnect()`, stop the timers and go to frame 4. Frame 4 shows the result and after 2000 ms `finalCallback` calls `getURL('FSCommand:closewindow','')`. So the client drops the socket itself right after the result. There is no abort message; closing the window is a socket drop.

Race: the client's local timer calls `onFailure()` when `timeRemaining <= 0` (`onTimerUpdate`). That disconnects before the server's own tick declares defeat, so the server can see a drop and report Canceled(0) instead of Defeat(2). The session code (MG-25 result path) should treat a drop within a small window after the server's computed deadline as a timeout defeat.

### Extension to host report

The extension calls `__mg.minigameCompleteGame(user, code, timeRatio, numResets, instrumentsUsed, consumablesUsed)`.

| Event | Call |
|---|---|
| Victory | `(user, 3, timeRemaining/initialTime, numResets+1, origInstcc != -1, consUsed)` |
| Timeout | `(user, 4, 0, numResets+1, false, false)` |

Locations (functions containing the call): Livewire `checkForVictory` and `tick`; Alignment `doVictory` and `tick`; ConverseBasicHumanoid `checkForVictory` and `failMessage`; GoauldCrystals `sendVictoryMsgs` and `tick`; CrystalGame `checkForVictory` and `tick`. CrystalGame is a quirk: timeout is `(user, 4, 0, 1, false, false)` but victory is a 5-argument call `(user, 30.5, 1, false, false)` (it omits the code and passes a constant 30.5 as time). A host that reads positional args would mis-read it; the Rust server never calls this, so it only matters if an implementation ports the call.

### Result codes

The cell-side `handleMinigameResults(ticket, resultId, timeRemaining, numAttempts, instrumentsUsed, consumablesUsed)` exists (`cell/SGWPlayer.py` 1926; `MinigamePlayer.def` 368) with Python `MINIGAME_RESULT` Canceled=0, Victory=1, Defeat=2, NotStarted=3. Rust (`server/result_dispatch.rs` 15-17) uses 0, 1, 2 only.

The packet's 0-4 mapping and the extension's own codes do not line up:

| Source | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| Packet (cell) | canceled | victory | failure | interrupted / not started | defeated |
| Python Constants | Canceled | Victory | Defeat | NotStarted | - |
| Extension host call | - | - | - | victory | timeout |

The extension's 3 and 4 are an extension-to-host enum that a host translates; they are not cell result ids. Proposed mapping for MG-25 and the result path: extension 3 maps to cell 1, extension 4 maps to cell 2, socket drop before any server verdict maps to 0, never-started (ticket issued, no `opendoor`) maps to 3. Timeout maps to cell 2 (Defeat) for the current consumers; C3 code 4 is reserved for a future distinguishing consumer.

## Q5. Who owns the countdown?

Both, with the server authoritative.

- Server: the extension runs `setInterval('tick', 100)` (Livewire `startTimer`, `tick`). `timerState` is 0 stopped, 1 counting, 2 stop-time cheat, 3 reverse-time cheat. At zero it sends `failure` and the code 4 host call. `sendTimerUpdate` sends `timeRemaining - 0.25`; the server value starts at countdown + 0.25.
- Client: Livewire runs a local 50 ms `timerCallback` (using `getTimer` deltas, `timerBeep` each 1000 ms). It resyncs on `timerupdate`, `addtime` (+timeDiff), `cuttime` (x0.75), `reverttime` (= timeTotal) and `countdownupdate` (new countdownRate), and acts locally at 0 (see the Q4 race). The shells run a local 50 ms timer too, with a 12 s default.

Consequence: the server must send `timerupdate` on every state change and a `countdownupdate` when the rate changes (Livewire obstacle and moving wires speed the countdown), or the client display drifts from the server's deadline.

## Q6. `t='o'` versus `t='a'`

The client serializer emits `a` for `instanceof Array` and `o` otherwise. The deserializer builds `[]` for `t='a'` and `{}` for `t='o'`. The client reads `wireNames`, `wireLibs`, `wireSounds`, `wireDepths`, `wireXs`, `wireYs`, `glowPoints` and `wirearray` as arrays (`.length`, indexing), which works with a numeric-keyed `t='a'` object. Rust emits `<obj o=... t='a'>` for everything (`protocol.rs` 426), which satisfies these reads. A nested object such as `wireObj` in `create` is not read by the client.

No SWF was found that needs `t='o'`. MG-01 (list type) should still keep an explicit object type for future games, but nothing blocks on it.

## Open items

- Alignment and Converse server logic were read for command sets and `minigameCompleteGame` sites only; their rules belong to MG-R-series packets.
- Findings are static analysis of the decompile. The pseudo-decompiler garbles ternaries; the `t='a'`/`'o'` choice and `getTime` were checked against the raw disassembly.
