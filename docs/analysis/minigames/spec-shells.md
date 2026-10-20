# Shell games: Hack, Activate, Analyze, Bypass, Converse (MG-R5)

Research spec for the minigames campaign, Wave 0B. It answers the shell half of
[work-packets.md § MG-R5](work-packets.md): the message set and timer of the
five shell movies, whether any of them holds rules (README F1), the client type
table at `0x019d917c` and the handler `0x00e32140`, how the name in
`onStartMinigame` picks a package, what the client does with an unknown name,
and the audio hints. The content half is [content-map.md](content-map.md).

Sections follow Contract C5. Every claim carries a tag, as in
[spec-launch-flow.md](spec-launch-flow.md):

- **Confirmed**: read directly from the movie disassembly, the decompile, a
  seed row, the Rust or the original Python, cited by file and line.
- **Inferred**: a reasoned conclusion that no single source states. G1 should
  treat these as proposals.
- **Unresolved**: no source in this repo answers it. The text says what would.

Movie code was read with `tools/minigame-swf` (`as2dis.py`, `list_functions.py`)
on packages extracted outside the repo. It is client data (D-MG10): excerpts
stay under 20 lines, and no disassembly is committed. Rerun any claim with
`list_functions.py <Name>.gfx --function <name>`.

## Contents

1. [Verdict](#1-verdict)
2. [`joinOK` params](#2-joinok-params)
3. [Board model](#3-board-model)
4. [Difficulty](#4-difficulty)
5. [Message table](#5-message-table)
6. [Scoring, win and loss](#6-scoring-win-and-loss)
7. [Cheats](#7-cheats)
8. [Hints and timers](#8-hints-and-timers)
9. [Golden transcripts](#9-golden-transcripts)
10. [Port slicing](#10-port-slicing)
11. [Open questions](#11-open-questions)
12. [Client type table and handler `0x00e32140`](#12-client-type-table-and-handler-0x00e32140)
13. [Corrections to other docs](#13-corrections-to-other-docs)

## 1. Verdict

**The five shells hold no rules (Confirmed; README F1 and M1 confirmed).** Each
movie is one button (`win_btn`) that sends `victory`, a countdown text field
and the SmartFox plumbing every SGW movie shares. There is no board, no move
message and no client-side scoring. Whatever game the designers meant for
Activate, Analyze, Bypass, Converse and Hack was never built into these movies.

| Package | GFX offset | Length | Movie md5 | Functions |
| --- | --- | --- | --- | --- |
| `Activate.upk` | 1,583 | 55,840 | `74520bfa6aed2496e3a9109f71737842` | 183 |
| `Analyze.upk` | 1,575 | 55,806 | `8207148dc178533f5a94af97114fa76a` | 183 |
| `Bypass.upk` | 1,567 | 55,887 | `846b86e358ef93cfa5246d36ecd59093` | 183 |
| `Converse.upk` | 1,583 | 55,795 | `25931bc75ec2d06289057f530b0af6f8` | 183 |
| `Hack.upk` | 1,551 | 55,760 | `ab34a898ce64a27f9e783142724aa3b6` | 183 |

Offsets, lengths and hashes are from [client-packages.md](client-packages.md)
(Confirmed). All five are SWF version 9.

- **The five movies are one template (Confirmed).** Their action code differs
  only in the literal assigned to `_root.gamename.value` on frame 2.
  Activate also calls `displayText('Game successfully loaded!')`, which adds
  two constant-pool entries (49 against 47). Most of the 183 functions are the
  shared SmartFoxClient 1.x library, not game code.
- **`Converse` is not `ConverseBasicHumanoid` (Confirmed).**
  `ConverseBasicHumanoid.upk` is a separate movie with real rules (382
  functions, md5 `12553e9639b3fae147e3f0944c7f77c5`); MG-R4 owns it.
- **Server today (Confirmed).** `crates/minigame/src/minigame/games/mod.rs:17-40`
  routes all five names to `PlaceholderGame`, which is what D-MG3 = A keeps:
  client-declared, hardened by the 5 s floor (D-MG13).

Shell functions, by frame (Confirmed, `list_functions.py Hack.gfx`):

| Frame | Functions | Role |
| --- | --- | --- |
| 2 | `handleConnection`, `handleExtensionResponse`, `displayText`, `playSound`, `connectAndLogin`, `sendMessage`, `registerOnResponse`, local `zone`/`room`/`user`/`_server.sendResponse` | Connect, log in, buffer early messages, offline stand-in |
| 3 | `onVictory`, `onFailure`, `onResponse`, `startTimer`, `timerCallback`, `onTimerUpdate`, `decimalFormat`, `win_btn.onRelease` | The "game" |
| 4 | `finalCallback` | Close the window |

## 2. `joinOK` params

**Not applicable (Confirmed).** The shells read no variable from the login
response: `handleExtensionResponse` looks only at `_cmd`. This matches
[spec-protocol-common.md](spec-protocol-common.md) Q2, which records that Rust
sends a superset of the parameters other movies read. The shell ignores all of
them.

What the shell does read is what the host window sets before `gotoAndPlay`
(§12): `_root.ipaddress.value`, `_root.port.value`, `_root.playerid.value`,
`_root.ticket.value` and `_root.gamename.value` (Confirmed, decompile
`14_standalone_named.c:311580-311607`). Frame 2 then overwrites
`_root.gamename.value` with the movie's own literal (`'Hack'` in `Hack.gfx`),
so the zone the movie logs into is always its own name, whatever the URL said
(Confirmed).

## 3. Board model

**Not applicable (Confirmed).** There is no board. The only state is
`gameActive`, `victory`, `timeRemaining`, `timerState` and `timerId`.

## 4. Difficulty

**Not applicable (Confirmed).** No shell reads a difficulty, level or seed.
`maxCheats = 0` is set on frame 3 and never read.

## 5. Message table

All traffic is SmartFox extension messages on extension `game`
(`sfs.sendXtMessage('game', cmd, obj)`) with XML responses (Confirmed).
`onResponse` ignores any response whose type is not `'xml'` (Confirmed).

| Dir | `_cmd` | Fields read or sent | Shell action | Source |
| --- | --- | --- | --- | --- |
| C→S | login | zone = `gamename`, user = `playerid`, pass = `ticket` | sent from `handleConnection(true)` | frame 2 |
| S→C | `loginSucceeded` | none | `gotoAndStop(3)` (game frame) | frame 2 `handleExtensionResponse` |
| S→C | `loginFailed` | none | `gotoAndStop(4)` (end frame, closes in 2 s) | frame 2 |
| S→C | any other, before frame 3 | whole object | pushed to `pendingMsgs`, replayed on frame 3 | frame 2 |
| S→C | `fullgamestate` | `timerState`, `timeRemaining` | set `timerState`; call `setupTimer()` (undefined, a no-op); `onTimerUpdate(timeRemaining)` | frame 3 `onResponse` |
| S→C | `timerupdate` | `timerState`, `timeRemaining` | set both; `onTimerUpdate`; `startTimer()` if `timerId == 0` | frame 3 |
| S→C | `victory` | none | `onVictory()` | frame 3 |
| S→C | `failure` | none | `onFailure()` | frame 3 |
| C→S | `victory` | `{}` | sent by `win_btn.onRelease` | frame 3 |

- **The shell sends nothing but `victory` (Confirmed).** There is no abort
  message from the movie; closing the window ends the socket, and the server's
  `aborted` path (`placeholder.rs`) handles that.
- **A failed connect is silent (Confirmed).** `handleConnection(false)` does
  nothing: the window stays on frame 2 with no text and never closes by
  itself.
- `spec-protocol-common.md` Q4 lists the same command set and warns that
  echoing `victory` lets a player win with one click. That is the point of the
  D-MG13 floor.

Frame 3 `onResponse` dispatch, abridged (Confirmed, `Hack.gfx` frame 3):

```text
if (type != 'xml') return
switch (data._cmd)
  'fullgamestate': timerState = data.timerState; setupTimer(); onTimerUpdate(data.timeRemaining)
  'victory':       onVictory()
  'failure':       onFailure()
  'timerupdate':   timerState = data.timerState; timeRemaining = data.timeRemaining
                   onTimerUpdate(timeRemaining); if (timerId == 0) startTimer()
```

## 6. Scoring, win and loss

- **No score (Confirmed).** Nothing is counted.
- **Win (Confirmed).** The server sends `victory`; `onVictory` stops the timer,
  sets `victory = true` and `gameActive = false`, and goes to frame 4. Frame 4
  starts a 2000 ms `setInterval` whose callback fires `FSCommand:closewindow`.
- **Loss (Confirmed).** The server sends `failure`; `onFailure` does the same
  with `victory = false`. The movie never decides a loss itself while online
  (§8).
- **The shells never disconnect (Confirmed).** Unlike Livewire, whose
  `onVictory` calls `sfs.disconnect()` (spec-protocol-common.md), the shell
  handlers do not. The socket closes when the host window closes on
  `closewindow` (Inferred from the host tearing the movie down; not traced).
- **What the movie's `victory` flag means to the client (Unresolved).** No
  FSCommand reports it to the host, so the client UI never learns the result
  from the movie; the result reaches the player only through the server's
  `onMinigameResult`/cell path. A Ghidra pass over the FSCommand handler of
  `FlashExternalWindowModule` would confirm there is no hidden channel.

## 7. Cheats

**Not applicable (Confirmed).** No shell reads `instcc`, `CA0`-`CA4` or
`consUsed` (see spec-launch-flow.md §5), and `maxCheats` is dead. `playSound`
is defined but never called.

## 8. Hints and timers

### The countdown

| Item | Value | Tag |
| --- | --- | --- |
| Local default `timeRemaining` | 12 | Confirmed, frame 3 init |
| Tick | `setInterval(timerCallback, 50)` | Confirmed |
| `timerState` 1 | count down: `timeRemaining -= dt / 1000` | Confirmed |
| `timerState` 2 | frozen | Confirmed |
| `timerState` 3 | count up: `timeRemaining += dt / 1000` | Confirmed |
| `timerState` 0 or other | no change | Confirmed |
| At `timeRemaining <= 0` | clamp to 0, clear the interval; call `onFailure()` **only when offline** | Confirmed |
| Display | `timer_txt.text = decimalFormat(n, 2)` | Confirmed |

`spec-protocol-common.md` Q5 records the same 12 s default and 50 ms tick.

- **Online, the timer cannot lose the game (Confirmed).** The zero branch calls
  `onFailure` only when `online` is false. So the shells do not share
  Livewire's race in which the local timer and the server both end the game.
- **The timer never starts today (Confirmed).** It starts only on
  `timerupdate`, and `PlaceholderGame` never sends one (`needs_tick` is
  false). `fullgamestate` alone calls the undefined `setupTimer()`.
- **Today's `fullgamestate` breaks the timer text (Inferred).**
  `placeholder.rs` sends `fullgamestate {gameStarted:'false'}` with no
  `timerState` or `timeRemaining`, so `onTimerUpdate(undefined)` runs and
  `timer_txt` most likely shows `NaN` text. A lab look would confirm the exact
  string.

### Audio hints

[audio-voice-inventory.md:515](../../client/audio-voice-inventory.md) lists
`activateMG.fev`, 22 events, described as an "Activation minigame (lock picking
/ repair / search)": activate, repair and search variants of choose, ready,
rotate, timeout and match, plus achieve, burst, cash, pop, start, sweep and xp
(Confirmed). `hackMG.fev` (17 events) belongs to Livewire, and `genMG.fev`
holds six generic lines (Fail1, GoodJob1/2, FoundIt1, SeriesDone1, StepDone1)
(Confirmed, lines 516-518).

- **The designed Activate game was a rotate-and-match puzzle in three skins:
  activate, repair, search (Inferred from the event names).** The shell plays
  none of these sounds: `playSound` is never called (Confirmed).
- No bank names Analyze, Bypass or Converse (Confirmed by the same table), so
  nothing in the audio says what those games were.

## 9. Golden transcripts

Server lines use the SmartFox XML extension format that
[spec-protocol-common.md](spec-protocol-common.md) defines; only `_cmd` and the
fields are shown. All three are Inferred end to end (each step is Confirmed in
the movie; no capture of a shell run exists).

### (a) Online win, as MG-R5 recommends the server behave

```text
C  connect ip:port ; loadPolicyFile if gfxVersion defined
C  login zone='Hack' user=<playerid> pass=<ticket>
S  xt loginSucceeded                       -> frame 3
S  xt fullgamestate {timerState:0, timeRemaining:60}   -> timer_txt '60.00'
S  xt timerupdate   {timerState:1, timeRemaining:60}   -> countdown starts
   ... player waits at least 5 s (D-MG13) ...
C  xt victory {}                           (win_btn)
S  xt victory                              -> onVictory, frame 4
C  FSCommand closewindow after 2000 ms     -> window and socket close
```

### (b) Login rejected

```text
C  connect ; login zone='Analyze' user=<playerid> pass=<bad ticket>
S  xt loginFailed                          -> frame 4
C  FSCommand closewindow after 2000 ms
```

### (c) Server ends the game (abort or early click)

```text
S  xt loginSucceeded ; xt fullgamestate {timerState:0, timeRemaining:60}
C  xt victory {}                           (clicked before the 5 s floor)
   server rejects the early victory and keeps the session
C  window closed by the player             -> server 'aborted' path
S  xt failure  (if the socket is still open) -> onFailure, frame 4
```

What `PlaceholderGame` does with a victory before the floor is defined in
`placeholder.rs` (D-MG13); (c) shows only the movie's side.

## 10. Port slicing

D-MG3 = A keeps the shells client-declared, so there is no rules port. The
server-side changes are small:

1. **Send a timer the shell can show (Inferred, recommended).** `started()`
   should send `fullgamestate {timerState:0, timeRemaining:N}` instead of
   `{gameStarted:'false'}`, and one `timerupdate {timerState:1,
   timeRemaining:N}` if a countdown is wanted. A server-side timeout at N
   would then send `failure`, which the shell already handles.
2. **Keep the 5 s floor (Confirmed, D-MG13).**
3. **Reject unknown and wrong-case names at the content loader or at
   `start_minigame` (Inferred, recommended).** See §12: a bad name gives the
   player no window and no feedback.
4. **Regression guard:** a wire test that `PlaceholderGame::started()` emits
   `timerState` and `timeRemaining` would fail if (1) were reverted.

## 11. Open questions

- **Q-S1 (Unresolved).** Exact `timer_txt` text for `onTimerUpdate(undefined)`.
  A lab screenshot of any shell under today's server answers it.
- **Q-S2 (Unresolved).** Does the UE3 package lookup in `0x0093aa20` match
  names case-insensitively? If it does, `hack` loads `Hack.upk` but gets the
  title "Mini-game" and the server's placeholder. A Ghidra read of
  `0x0093aa20` or a lab launch with a lower-case name answers it.
- **Q-S3 (Unresolved).** The two arguments of the `"%s.%s"` `_snprintf` at
  `14_standalone_named.c:311409` were dropped by the decompiler;
  `gamename.gamename` is inferred. A disassembly read of the call site settles
  it.
- **Q-S4 (Unresolved).** Whether any FSCommand from the movie besides
  `closewindow`, `displaytext` and `playsound` reaches game code (§6).

## 12. Client type table and handler `0x00e32140`

### How the name picks the package

`onStartMinigame` carries one `WSTRING` URL,
`http://unused/{host}/{port}/{gameName}/{entityId}/{ticket}`, built in
`crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs:126-130`
(Confirmed). The base does not validate `gameName` (Confirmed).

`FlashExternalWindowModule__unknown_00e32140`
(`docs/reverse-engineering/decompiled/14_standalone_named.c:311164`) then:

| Step | Lines | Behaviour | Tag |
| --- | --- | --- | --- |
| 1 | 311257 | Read the `URL` property | Confirmed |
| 2 | 311276 | Find the `Minigame` window | Confirmed |
| 3 | 311405 | If a movie is already loaded, unload it (`FUN_0093a990`) | Confirmed |
| 4 | 311409-311422 | Format `"%s.%s"` and load it with `0x0093aa20` | Confirmed; arguments Inferred `gamename.gamename` (Q-S3) |
| 5 | 311423-311425 | If `0x0093ace0` reports failure, retry as `"/%s.swf"` | Confirmed |
| 6 | 311458-311471 | Still failed: log `"Failed to open movie: %s"` and return | Confirmed |
| 7 | 311580-311607 | Set `_root.*.value` variables, `gotoAndPlay("Start")` via vtable `+0x48`, then `+0x70(0,0)` | Confirmed |
| 8 | 311623-311812 | Pick the window title by exact compare; set it with `FUN_00438f30` | Confirmed |
| 9 | 311815-311825 | Show the window (`FUN_00e3a760(..., 1)`) | Confirmed |

So the package is the URL's `gameName` used as both package and object name
(Inferred), and the movie, not the host, then decides which SmartFox zone it
logs into (§2).

### The title table (the "type table")

`minigame-architecture.md` calls `0x019d917c`-`0x019d9340` a table of the ten
minigame types. **The handler does not walk a table (Inferred from the
decompile).** It is a chain of byte compares whose length is `strlen + 1`, so
the NUL takes part and every match is exact and case-sensitive (Confirmed).
The address range holds the chain's string literals.

| `gameName` | Window title | Tag |
| --- | --- | --- |
| `Livewire` | `Livewire` (via `FUN_00438520`) | Confirmed |
| `CrystalGame` | `Crystal Game` | Confirmed |
| `Alignment` | `Alignment` | Confirmed |
| `Activate` | `Placeholder Activate` | Confirmed |
| `Analyze` | `Placeholder Analyze` | Confirmed |
| `Bypass` | `Placeholder Bypass` | Confirmed |
| `Hack` | `Hack` | Confirmed |
| `Converse` | `Converse` | Confirmed |
| `ConverseBasicHumanoid` | `Converse: Humanoid Basic` | Confirmed |
| `GoauldCrystals` | `Goa'uld Crystals` | Confirmed |
| anything else | `Mini-game` (line 311803) | Confirmed |

The client titles three shells "Placeholder", but not Hack or Converse. The
2009 team knew those three were stubs (Inferred).

### What the client does with an unknown name

| Case | Client | Server today | Tag |
| --- | --- | --- | --- |
| No such package | Logs `Failed to open movie: <name>` and returns; **no window opens** | `mod.rs` logs "Unknown minigame type, using placeholder", registers the session and waits | Client Confirmed; server Confirmed |
| A package outside the ten | Loads and runs it, title `Mini-game` | Placeholder | Confirmed |
| Wrong case, for example `livewire` | Title `Mini-game`; whether the package loads is Q-S2 | `GameKind` matches case-sensitively, so placeholder | Title Confirmed; load Unresolved |

In the first case the player sees nothing and the server holds a session that
no client will join (Inferred: it lives until the session timeout).

## 13. Corrections to other docs

- **`docs/reverse-engineering/findings/minigame-architecture.md:38-57`** says
  Activate, Analyze and Bypass are "Placeholder — no SWF". All five shell
  packages exist with movies (client-packages.md). The "Placeholder" in their
  window titles is the client's label, not a missing file. The table also
  reads as a lookup table; it is a strcmp chain (§12). The fix belongs to the
  doc's owner; this packet edits only its two files.
