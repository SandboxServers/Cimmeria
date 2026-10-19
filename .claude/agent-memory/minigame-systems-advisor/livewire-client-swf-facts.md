---
name: livewire-client-swf-facts
description: Verified Livewire client facts from the GFX bytecode and SGW.exe decompile - stage size, click/door handlers, mouse-blocking clips, input routing into the Flash movie, onEndMinigame auto-close, embedded original AS1 server extension
metadata:
  type: reference
---

# Livewire client facts (verified 2026-09-29, AS2 disassembly + Ghidra)

## The movie

- `CookedPC/UI/Flash/Livewire.upk` embeds ONE uncompressed Scaleform `GFX` v8 movie (header "GFX", 410,721 B at file offset 11232). Stage **1280x960**, 24 fps. Rendered into CEGUI `Minigame_Movie_Area` (ExternalWindow, 640x480 client area inside the 660x550 `MinigameWin`, client `Content/UI/Core/Minigame/Minigame.layout`), so the scale is 0.5.
- Placeholder SWFs (Hack/Activate/Analyze/Bypass/Converse, ~55 KB GFX each) are 640x480 and DO have a `win_btn` at stage (470.45, 388.95) that sends `victory`. This resolves the "untested assumption" in [[game-implementation-status]].
- **The GFX embeds the original SmartFox server-side AS1 extension** (`handleRequest`, `processMove`, `setupWires`, `checkForVictory`, `_server.sendResponse`). It is a better parity reference than `deprecated/python/base/minigame/Livewire.py`.

## Input handlers (AS2)

- Wires: `attachMovie(lib, name, depth, {_x,_y})` into `_root.load_mc.wire_mc`. All wires get `onRollOver`, which sends `processover{wirename}`, and `onRollOut`, which sends `processout`. Only libs NOT starting with `p` get `onRelease=wireClick`, which sends `processmove{wirename,timeremaining,countdownupdate}`.
- Door: `_root.start_btn` (stage 363.4,665.25; 79x79) `onRelease` sends `opendoor{open:true}`.
- Mouse-opaque static clips above `load_mc` (depth 387): `wireCover` (depth 451, stage x 160-440 over all wire rows, so **wire terminals at x=275.6 are NOT clickable**), `cover_mc` (the door, disabled after open), `mask_mc` (only a strip at y≈900). The clickable wire band is stage x ≈ 445-1100.
- Wire art is 10-25 stage px thick and wiggles up to ±100 px vertically off its slot y.

## Parity bug (Rust AND Python)

The SWF exports only suffixed libs (`pGray1..4`, `mYellowStripe1..4`, goal/obstacle `…1..4`). The original extension appends `randRange(1,4)` to all four kinds. Rust `games/livewire/setup.rs:239,306` sends bare `pGray`/`mYellowStripe`, so attachMovie fails and those wires are invisible. `:260,326` use `1..4` exclusive, so variant 4 never appears.

## SGW.exe routing

- onStartMinigame handler `0x00e32140`: FlashExternalWindowModule "Minigame" via `FUN_00568d10(FUN_00569230(), L"Minigame")`; GFxMovieView via `FUN_0093ace0`; vtable `+0x2c` SetVariable, `+0x48` Invoke, `+0x84` HandleEvent.
- **onEndMinigame handler `0x00e31a00`** only emits `Event_UI_MinigameVisibility(false)`, so the client auto-closes the window (Lua hides it and calls `endMinigame()`).
- Mouse into the movie: CEGUI ExternalWindow `onMouseMove` `0x011f4a00` normalises the local position and calls manager `+0x08`, which reaches module `vfunc_4 0x0093abc0` and stores the position. `onMouseButtonDown 0x011f4960` reaches module `vfunc_2 0x0093aa70`, which uses the STORED position. **A click lands where the last CEGUI MouseMove was**, so move, wait a frame, then click. There is no `injectMouse*` Lua binding in the client.

See also [[chain-wiring-and-gaps]], [[game-implementation-status]].

## MG-R6 confirmations (2026-10-19)

- Offline mode: frame-2 code sets `_root.online=false` only when `_root.v1` and `_root.ipaddress.value` are both undefined (frame-1 init `_root.ipaddress = new String('')` makes the `== ''` test always true; the host never ran SetVariable before frame 2); then the embedded extension runs locally with Math.random. An empty host gives `.value == ''`, so the movie stays online and the connect fails (frame 4). Not run in the client.
- GoauldCrystals' extension reads `seed` (initGame, placeGoals, randomizeCrystals), so it needs the java.util.Random port too; the earlier "no client code reads seed" holds only for client code.
- Client disconnects itself on `victory`/`failure` (`sfs.disconnect()`), then `FSCommand:closewindow` after 2000 ms. Local timer can call `onFailure` at 0 before the server's tick, so the server may see a drop (Canceled) instead of its own timeout (Defeat).
- Extension host call `__mg.minigameCompleteGame(user, code, timeRatio, numResets, instruments, consumables)`: code 3 = victory, 4 = timeout. These are extension-to-host codes, not cell result ids.
- Original difficulty table, wire variant range 1-4 (inclusive), goal depth bands 1-25/60-85, obstacleTotal x2, getTime method 2 is bitwise XOR: see `docs/analysis/minigames/spec-livewire-parity.md` (16 deltas, 1 informational and 1 no-delta row, vs Rust).
- No SWF needs `t='o'`; arrays as `t='a'` are what the client reads.
