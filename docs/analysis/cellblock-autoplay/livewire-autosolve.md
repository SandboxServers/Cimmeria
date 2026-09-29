# Livewire auto-solve for the live research lab: design

> Type: explanation (design). Audience: whoever picks up the MG packets of the Cellblock autoplay campaign.
> Updated: 2026-09-29. Companions: [README.md](README.md), [work-packets.md](work-packets.md), [livewire-spike/](livewire-spike/README.md), [minigame-system.md](../../gameplay/minigame-system.md).

Status: research and design, 2026-09-29. No code has been written. Every fact below is marked as one of:

- **[V]**: verified against the code, the SWF bytecode, or the SGW.exe decompile.
- **[S]**: verified by offline simulation.
- **[U]**: unverified. It needs the live spike (MG-L0) or the owner.

Spike tooling, committed under [livewire-spike/](livewire-spike/) for packets MG-L4 and MG-L2 to port (see its README):

- `as2dis.py` is an AS2 bytecode disassembler for GFX/SWF files.
- `bounds.py`, `bounds_core.py` and `shapes.py` parse shapes, compute bounding boxes and hit-test points.
- `atlas.py` builds `livewire_atlas.json`.
- `solver_sim.py` and `clearance.py` run the solver simulation.
- `probe_static.py` lists the movie's named root placements (start button, electrodes, covers).

The extracted movies (`Livewire.gfx`, `Hack.gfx`), their disassembly and the generated atlas are **not** committed: they are client assets, or derived from them, and the client is not in git. The spike README says how to regenerate them from your own client copy.

## 0. TL;DR

**The game.** Livewire is won by cutting every goal wire (`g*`).

- A cut is one SmartFox `processmove{wirename}` extension request. The server checks only that the wire exists, is not already cut, and that the game has started. There is no hit-test on the server [V].
- The SWF sends `processmove` only from the wire clip's `onRelease` [V].
- So "solving" means clicking a screen point where the topmost mouse-enabled clip is a goal wire.

**The data we have.**

- The server generated the board, so it knows every wire's library symbol, depth and stage position [V].
- The client movie's wire art can be rasterised offline into a per-library hit atlas [S].
- A simulated solver over the server's board, using that atlas, found an exposed click point for **every goal on 2,400 random boards** (difficulties 1 to 4, with and without the §7 fix) [S].

**Recommended approach (N1): atlas-planned, rollover-verified clicks.**

1. A new read-only lab-mcp tool returns the live board and a click plan.
2. A lab flow moves the virtual cursor to the planned point.
3. It waits for the SWF's own `processover{wirename=g…}` to arrive at the server. That rollover is proof the goal wire is topmost under the cursor.
4. Only then does it left-click, as a real window message. The SWF sends `processmove`, and the server validates it as usual.

**What SGW.exe says about input.** The decompile shows that mouse motion reaches the movie only through a CEGUI MouseMove event on the ExternalWindow [V]. The game feeds CEGUI from `GetCursorPos`, which the lab already virtualises (`focus.rs:8-9`). So the lab's existing cursor path should work [U: one live check].

**Fallbacks.** No minigame GM command exists today [V]. In order of preference:

- X-tier: a server-side inject of `processmove` through the game's own `message()`.
- G-tier: a new `.minigame win` GM command.
- `/gmMissionAdvance`: G-tier, and a step skip, not a win.
- A raw SmartFox client using the ticket: rejected.

**Side finding (parity bug).** The server sends playfield and moving wire library names without the variant suffix the SWF needs. Those wires never render. The Python reference has the same bug; the original AS1 extension embedded in the SWF does not (§7).

## 1. How the client hosts Livewire

### 1.1 The window and the movie

- **[V] CEGUI layout** (client `Content/UI/Core/Minigame/Minigame.layout`):
  - `MinigameWin` is a `ComplexFrame_2`, fixed at 660x550 and centered.
  - It contains `Minigame_Movie_Area`, a `CEGUI::ExternalWindow` with a 640x480 client area (`UnifiedAreaRect {{0,10},{0,50},{1,-10},{1,-20}}`).
- **[V] Lua wiring** (client `Minigame.lua:5-17,69-81`). `Events.MinigameVisibility(true/false)` shows or hides `MinigameWin` and calls `setMinigameActive`. On hide it also calls `Minigame_Movie_Area:deactivate()` and `endMinigame()`. The movie binds through `setExternalWindowID(getExternalWindowIDForName("Minigame"))`.
- **[V] onStartMinigame handler** is `FlashExternalWindowModule` code at `0x00e32140`, subscribed as `MinigameInfo` handler #1 at `0x00e36090` / `0x00e361b0`.
  - It parses the URL, then looks up or creates the `FlashExternalWindowModule` named `L"Minigame"`: `FUN_00568d10(FUN_00569230(), L"Minigame")`.
  - It opens `%s.%s`, and falls back to `/%s.swf`.
  - It gets the `GFxMovieView*` via `FUN_0093ace0(module)`.
  - It calls `SetVariable` (vtable `+0x2c`) for `_root.ipaddress/port/playerid/ticket/gamename.value`, then `Invoke` (`+0x48`) `"gotoAndPlay","Start"`.
  - It emits `MinigameName` (the display name; Livewire maps to L"Livewire") and `Event_UI_MinigameVisibility(true)`.
- **[V] onEndMinigame handler** is `FUN_00e31a00`, handler #2 in the same subscription list, typed `MemberCallback<MinigameInfo, Event_NetIn_onEndMinigame>` (`FUN_00e38910`). It only emits `Event_UI_MinigameVisibility(false)`. **So the client closes the window by itself when the server sends `onEndMinigame`.** The base sends `onEndMinigame` on every result, including Canceled (`crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs:98-119`).
- **[V] The movie.** `Livewire.upk` embeds one uncompressed Scaleform `GFX` v8 movie: 410,721 bytes at file offset 11232, stage 1280x960, 24 fps. It fits the 640x480 area at exactly 0.5 scale. The placeholder SWFs (Hack and the others) use a 640x480 stage, which is 1:1.
- `FlashManager/*.lua` is a window-blink helper, not a Flash host. `docs/client/ui-layout-inventory.md:78` is wrong about this.

### 1.2 How input reaches the movie [V, decompile]

`CEGUI::ExternalWindow` has its own vtable at `0x01abd8a0`. Its overrides forward to the external-window manager at `System+0xc4`, which is `SGWExternalWindowManager` (vtable `0x0183f26c`). The manager dispatches to `FlashExternalWindowModule` (vtable `0x018f9cc4`), which calls `GFxMovieView::HandleEvent` (`+0x84`).

| CEGUI ExternalWindow override | Manager slot | Module | Movie event |
|---|---|---|---|
| `onMouseMove` `0x011f4a00`: window-local position normalised by width and height | `+0x08` (`vfunc_2` `0x00568210`) | `vfunc_4` `0x0093abc0`: stores `pos * viewport` at `+0x28/+0x2c` | MouseMove |
| `onMouseButtonDown` `0x011f4960` | `+0x00` | `vfunc_2` `0x0093aa70`: event type 2, button from args `+0x1c`, **position = the stored `+0x28/+0x2c`** | MouseDown |
| `onMouseButtonUp` `0x011f49b0` | `+0x04` | (same family) | MouseUp |
| key down / key up / char | `+0x0c` / `+0x14` | `vfunc_7` / `vfunc_8` / `vfunc_6` | Key events 5 / 6 / 0xd |

Consequences:

- **The movie's mouse position changes only on a CEGUI MouseMove event.** A button press lands at the last *moved-to* point. The flow must move, let at least one frame pass, and only then click.
- The movie maps local coordinates proportionally (normalised, then multiplied by the viewport). That supports the stage-to-UI mapping in §2.2 [V]. Letterboxing inside the viewport is [U].
- The CEGUI Lua bindings expose no `injectMouse*` function (no such strings in SGW.exe) [V]. A MouseMove therefore has to come from the game's own feed.
  - `crates/client-telemetry/src/bridge/input/focus.rs:8-9` records that "the UI cursor follows the OS cursor". The game polls `GetCursorPos`, which the bridge virtualises.
  - `Supervisor::move_cursor` sets the bridge virtual cursor and *then* calls CEGUI `MouseCursor:setPosition` (`crates/lab/src/supervisor/input.rs:258-266`).
  - [U] Whether the game still injects a MouseMove when Lua has already moved CEGUI's cursor to the same point. If it does not, the flow uses a virtual-cursor-only move and waits one frame. See Q1.

### 1.3 Livewire SWF logic [V, AS2 disassembly]

**Wire creation.**

- Each wire is `attachMovie(wireLibs[i], wireNames[i], wireDepths[i], {_x, _y})` into `_root.load_mc.wire_mc`.
- Every wire gets `onRollOver=wireOver` and `onRollOut=wireOut`.
- Only libraries that do **not** start with `p` get `onRelease=wireClick`. Playfield wires and electrodes are therefore hover-only occluders.

**What the handlers send.**

- `wireOver` sends `processover{wirename}` for names starting p, g, m or o.
- `wireOut` sends `processout`.
- `wireClick` sends `processmove{wirename, timeremaining, countdownupdate}` and then clears its own `onRelease`.

**The door.** `_root.start_btn.onRelease` sends `opendoor{open:true}`.

- `start_btn` sits at stage (363.4, 665.25) and is 79x79.
- The movie calls `openDoor()`, which plays `cover_mc` and sets `cover_mc.enabled=false`, when the server's `opendoor` arrives, or a `timerupdate` with `timerState==1`.

**Static clips that block the mouse.** All three have an `onRollOver` handler that exists only to set `useHandCursor=false`, which makes them mouse-opaque.

| Clip | Depth | What it covers (rasterised) |
|---|---|---|
| `wireCover` | 451 | Stage x 160-440, y 180-940. **This covers every wire's left terminal (x = 275.6).** |
| `cover_mc` | 469 | The door. It is disabled and animated away once open. |
| `mask_mc` | 523 | Only a strip at y ≈ 900. |

`load_mc`, which holds every wire, is at depth 387, below all three. **The clickable band is therefore stage x ≈ 445 to 1100.**

**Wire art** [S, atlas]. Wires are 10-25 stage px thick and run 820-1036 px right from their slot origin at (275.6, {391, 497, 577, 657, 783}), wiggling up to about ±100 px vertically. At the 0.5 scale a wire is only about 5-12 screen px thick, so screen-pixel precision matters.

**The original server.** The SWF also carries the **original SmartFox server-side AS1 extension**: `handleRequest`, `processMove`, `setupWires`, `checkForVictory`, `_server.sendResponse`. It is a better parity reference than `deprecated/python/base/minigame/Livewire.py`.

### 1.4 SmartFox packets the lab watches

Client to server over TCP, null-terminated (shape as in `crates/minigame/src/minigame/protocol.rs:408`):

```xml
<msg t='xt'><body action='xtReq' r='-1'><![CDATA[<dataObj><var n='cmd' t='s'>opendoor</var><obj o='param' t='a'><var n='open' t='b'>1</var></obj></dataObj>]]></body></msg>
<msg t='xt'><body action='xtReq' r='-1'><![CDATA[<dataObj><var n='cmd' t='s'>processover</var><obj o='param' t='a'><var n='wirename' t='s'>g1</var></obj></dataObj>]]></body></msg>
<msg t='xt'><body action='xtReq' r='-1'><![CDATA[<dataObj><var n='cmd' t='s'>processmove</var><obj o='param' t='a'><var n='wirename' t='s'>g1</var>…</obj></dataObj>]]></body></msg>
```

Server to client (`xtRes`):

- `fullgamestate`: `wireNames/Libs/Xs/Ys/Depths`, `glowPoints`, `timeRemaining` (`crates/minigame/src/minigame/games/livewire/setup.rs:345-409`).
- Also `opendoor`, `timerupdate`, `over`/`out`, `destroy{wirename}`, `countdownupdate`, `victory`, `failure`.

### 1.5 Server rules that matter [V]

- **Timer.** Difficulty 1 sets `time_remaining = 20.25` (`setup.rs:152-153`), decremented by 0.25 on every 250 ms tick, and only after `opendoor` (`crates/minigame/src/minigame/games/livewire/mod.rs:134-149,278-317`). That is about **20 s from door-open**. Defeat reports result 2 and fires no chains; a retry means re-interacting.
- **Board size.** With tech_competency hardcoded to 1 (`cell_dispatch/minigame.rs:44`), difficulty 1 has 2 goals, 2 obstacles, 5 playfield wires + 1 playfield electrode, and 2 moving wires (`setup.rs:138-166,200-343`).
- **What `processmove` validates** (`mod.rs:190-261`): the game has started, the wire exists, it is not already cut, and the prefix is g/o/m (or p while the playfield is active). Cutting an o or m wire changes `countdown_rate`, but `tick` ignores it (`mod.rs:231-243,286`), so obstacles cost the lab nothing.
- **One-time ticket.** `authenticate_and_claim` sets `connected=true` under the registry lock (`crates/minigame/src/minigame/session.rs:187-210`), so a second login with the same ticket fails.

## 2. Recommended: N1, atlas-planned and rollover-verified

### 2.1 Server: expose the live session (new, read-only)

**The problem.** Today the board lives only inside the connection task: `run_session` owns `Box<dyn MinigameInstance>` (`crates/minigame/src/minigame/server/mod.rs:127-376`). The `SessionRegistry` holds tickets only (`session.rs:46-86`). Lab-mcp can already reach the registry through `state.base.minigame_registry` (`crates/services/src/orchestrator.rs:293`).

**The change.**

1. Add `MinigameInstance::lab_snapshot(&self) -> Option<serde_json::Value>`, defaulting to `None` (the trait is at `crates/minigame/src/minigame/game.rs:24-40`). Livewire returns:
   - per wire: `name`, `lib`, `depth`, `x`, `y`, `cut`, `kind`, `clickable`;
   - `goal_total`, `goal_cut`, `game_started`, `timer_state`, `time_remaining`, `difficulty`.
2. Add a **per-entity minigame tap**: a bounded ring of `{seq, t_ms, dir: in|out|inject, cmd, params}` plus lifecycle rows (`login_ok`, `started`, `outcome`, `ended`).
   - Write it at the existing call sites in `run_session`: `game.started`, `game.message` and `encode_extension` (`server/mod.rs:214-222,264-305,329-368`).
   - Keep the last snapshot and outcome for about 10 minutes after the session ends.
   - Model it on `cimmeria_services::wire_log::tap`.
3. Add two lab-mcp tools. Both use the `server_` prefix, so the runner grades them as reads (`crates/lab/src/uat/tier.rs:102-121`):
   - `server_minigame_state(entity_id)`: `phase` (`pending|connected|playing|ended`), game, snapshot, **plan** (§2.3), last outcome.
   - `server_minigame_tap_read(entity_id, since_seq)`.

### 2.2 Client: `client_minigame_play` (N1 flow under `crates/lab/src/supervisor/`)

1. **Wait for the window.** `client_wait_for` with Lua `MinigameWin:isVisible()`, and `server_minigame_state.phase == "playing"`.
2. **Map stage to UI.**
   - Read the `Minigame_Movie_Area` rect with `widget_rect_chunk` (`input.rs:52-86`).
   - `ui = (R.left + x * R.width/1280, R.top + y * R.height/960)`. The expected scale is 0.5.
   - Placeholder games use `/640` and `/480` (1:1).
3. **Open the door.** Move to `start_btn` at stage (363.4, 665.25), wait one frame (at least 50 ms), then left-click through `post_button` (`input.rs:228`). Expect `in opendoor` in the tap.
4. **Calibrate once.** Move to electrode `pe1`, whose `x`/`y` come from the board. It sits at depth 101 or more, so it is always topmost, and it is about 47 px across. Expect `in processover{pe1}` within about 150 ms. If nothing arrives, retry at ±2 px offsets, then fail with `reason=calibration`.
5. **For each plan entry:**
   - move to the planned point, wait for `processover{name}`, confirm the current rollover is the intended wire, then left-click;
   - expect `in processmove{name}` and then `out destroy{name}`;
   - on the wrong or no rollover, try the entry's next candidate point (the plan carries 3-5).
6. **Victory.** Expect `out victory`, then run the §4 asserts.
7. **Defeat.** On `out failure` or timeout, report `outcome=defeat` with the probe log. The spec step can retry by re-interacting.

Report `native_level: {tier: "N1"}`. For evidence, take `lab_screenshot_region` of the movie rect before and after, plus the tap rows. Add two capability rows in `crates/lab/src/uat/tools.rs:42-82`:

- `drive("minigame_play", "client_minigame_play", Tier::N1)`
- `read("minigame_state", "server_minigame_state")`

### 2.3 Solver, over the server's board

```text
atlas A[lib]  : local hit points on a 5-px grid (from Livewire.gfx; see MG-L4)
unattached(w) : w.lib not in A        # today: every p*/m* wire (see §7)
blocked(p)    : p.x < 445 or p.x > 1270 or p.y < 110 or p.y > 890
                # wireCover, frame, timer UI
cells(w)      : { round5(w.x + lx, w.y + ly) : (lx, ly) in A[w.lib] }
top(p)        : max-depth live wire w (not cut, not unattached) with p in cells(w)
                # electrodes pe*/me* have depth 101+

plan = []
loop until every goal is cut:
  for g in uncut goals, fewest occluded cells first:
     exposed = [p in cells(g) : !blocked(p) and top(p) == g]
     if exposed: plan += (g, exposed ranked by clearance, top 5); mark g cut; continue loop
  # no goal exposed: cut the cuttable (o/m) wire that tops the most goal cells
  o = argmax over cuttable occluders; plan += (o, …); mark o cut
  if no cuttable occluder: stop with reason=goal_fully_occluded
```

Clearance means the side of the largest square around p that lies entirely inside g and outside every higher-depth wire.

**Simulation** [S] (`solver_sim.py`, `clearance.py`): 300 random boards per difficulty 1-4, generated with the Rust semantics, both as the code is today and with the §7 suffix fixed.

- The solver solved **every board** with exactly `goal_total` clicks. It never had to cut an occluder.
- For about **75%** of goals the best point has 5 stage px (2.5 screen px) of clearance or more. The rest are exposed only at wire edges.

That is why the flow verifies each click with a rollover before pressing, and why the plan carries several candidates. A blind grid without the atlas would need offsets up to ±100 px at 10 px steps, far too many probes for a 20 s timer. **The atlas is required, not optional.**

**Timing budget.** A difficulty-1 board is 2 clicks. At about 3 probes per click and about 150 ms per probe that is roughly 1 s. The Castle playtest humans took 3-12 s (`docs/analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md:33,42,47`).

## 3. Fallbacks, tiered

Runner tiers are defined in `crates/lab/src/uat/tier.rs:8-14`, and a step graded below its row's floor is NATIVE_SHORTFALL.

| # | Mechanism | Exists? | Tier | Loop integrity | Effort and notes |
|---|---|---|---|---|---|
| F1 | GFx `Invoke` on the minigame movie: `FUN_00568d10(FUN_00569230(), L"Minigame")`, then `FUN_0093ace0`, then vtable `+0x48`. It calls the SWF's own `sendMessage(…,'processmove')`. A read-only `Invoke` of `…g1.hitTest(x,y,true)` could also replace rollover probing | No. The anchors are now known [V] | N3 (`client_call_native` floor, `tier.rs:86-92`) | Intact: the SWF sends a real xt message | M. Only if N1 input fails (Q1). A native call from a foreign thread into GFx needs the bridge's main-thread marshalling |
| F2 | `server_minigame_act(entity_id, cmd, params)` in lab-mcp. It injects an xt request into the live session through `MinigameInstance::message`, so the rules still apply. Needs a per-session control `mpsc` in the registry and one `select!` arm in `run_session` | No | X | Intact: server-validated; the SWF gets `destroy`/`victory`, `onEndMinigame` closes the window, chains fire. The tap tags the row `dir=inject` | S. Cheapest reliable safety net |
| F3 | `.minigame win\|lose\|cancel [target]` GM command: `CellToBaseMsg::MinigameControl`, then base, then the same control channel. The task sends `victory` xt and reports through `send_minigame_result(…, "gm_forced")` (`crates/minigame/src/minigame/server/result_dispatch.rs:27-70`) | **No.** None of the 150 dot commands in `crates/cell-console/src/cell/console/registry/commands/` touches minigames, chains or mission advance [V] | G when typed in chat; X through `server_console_exec` (`tier.rs:94`) | Chains fire. Game rules are bypassed by design | S-M. Needs an owner decision: a new GM power, and a Discord line per use |
| F4 | Native `/gmMissionAdvance <missionId> <step>` (`crates/cell-console/src/cell/console/gm/missions.rs:179-250`) | **Yes** | G | **Broken.** It advances the step only. Victory chains 1017, 1042 and 1061 also run dialogs, interaction-bit swaps and mission 641 complete / 680 accept (`docs/analysis/castle-cellblock-rebuild/uat-guide.md:385-416,458-483`) | Only to unblock later rows. Never a PASS for T07, T10, T13 or T25 |
| F5 | A lab-side raw SmartFox client logging in with the ticket from the `onStartMinigame` URL (visible in the Mercury tap) | Possible | X | **Reject.** The ticket is one-time and claimed by the SWF (`session.rs:187-210`). Racing it breaks the real client | Do not build |
| - | Client-declared result: `MinigameComplete` NetOut or `endCurrentMinigame` (CM 25) | Stub (`crates/cell-methods/src/cell/cell_methods/minigame.rs:66-80`) | - | It would be the client self-declaring success | Must never become a win path |

## 4. Open, close and victory detection

**Open.**

- The server logs `Sending onStartMinigame to client` (`crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs:59-64`), then `Minigame started {entity_id, game, room_id}` (`crates/minigame/src/minigame/server/mod.rs:241`).
- `server_minigame_state.phase` becomes `playing`.
- On the client, `MinigameWin:isVisible()` becomes true.

**Victory.** Require all three layers:

1. **Minigame server.** Tap row `out victory`, and the log line `Minigame victory` (`server/mod.rs:276`).
2. **Base, cell and content.**
   - The base logs `Minigame result received result_code=1` (`cell_dispatch/minigame.rs:107`).
   - The cell logs `Minigame result chains=[…]` and fires them (`crates/cell/src/cell/service/base_messages/minigame.rs:22-31`).
   - The chain's effect shows in `server_entity_get` or `server_db_query`. Per the uat-guide:
     - T07 → step 2116 (lines 244-271);
     - T10 → step 2215 (385-416);
     - T13 → mission 641 completed and 680 accepted (458-483);
     - T25 → mission 689 completed (107-143).
3. **Client.** `onEndMinigame` appears in `server_packet_tap_read`. `MinigameWin` hides by itself (`FUN_00e31a00` emits Visibility false) [V]. Assert with `client_wait_for` on `not MinigameWin:isVisible()`.

**Closed without an outcome.** The log shows `Minigame aborted -- client closed without reporting a result` and result code 0 (`server/mod.rs:393-415`), followed by `Minigame session ended` (`:114`).

## 5. Work packets

| Id | Packet | Crates | Effort | Tests |
|---|---|---|---|---|
| MG-L0 | **Live spike, first** (about 1 h, no code). In a Livewire session: `client_cursor_move` to `start_btn`, then `client_input_mouse` click, and check `opendoor` in the log. After that, hover `pe1` and check whether rollover arrives (needs debug-level logging or MG-L1). Answers Q1-Q3 | - | S | - |
| MG-L1 | `lab_snapshot()` + per-entity minigame tap + retained outcome | `cimmeria-minigame` | M | Unit: snapshot shape, ring bounds, retention. Server test: a scripted SFS client's `processover` shows up in the tap, in order |
| MG-L4 | Livewire hit atlas: a generator (port of `shapes.py` + `atlas.py`, or the Rust `swf` crate) run against the user's `CookedPC/UI/Flash/Livewire.upk`, and loaded by the solver | tools/ + `cimmeria-minigame` or lab | M | Atlas bbox matches the GFX bounds; golden point checks per library |
| MG-L2 | `server_minigame_state` / `server_minigame_tap_read` + atlas solver plan | `cimmeria-lab-mcp`, `cimmeria-minigame` | S-M | Solver property test over seeded boards, difficulties 1-4 (every goal planned; points not blocked; top == target) |
| MG-L3 | `client_minigame_play` N1 flow + capability rows | `cimmeria-lab` | M | Unit tests: stage-to-UI mapping, rollover state machine over recorded tap rows. One live run as evidence |
| MG-L5 | `castle-cellblock.toml` rows T25, T07, T10, T13 use `@minigame_play` and the §4 expects | `docs/guides/uat-specs/` | S | `cargo test -p cimmeria-lab` spec validation |
| MG-F2 | `server_minigame_act` (X) through a per-session control channel | `cimmeria-minigame`, `cimmeria-lab-mcp` | S | Injected goal `processmove` gives Victory and chains fire; injected unknown wire is refused and logged |
| MG-F3 | `.minigame win\|lose\|cancel` GM command (owner decision) | `cimmeria-cell-console`, base, minigame | S-M | Console tests; negative log when no session is active |
| MG-P1 | Parity: wire library suffix and variant range (§7), a separate issue for `rust-gameserver-dev` | `cimmeria-minigame` | S | Byte-level `fullgamestate` test: every `wireLibs` entry is a SWF export name |

**Order:** L0, then L1 with L4 in parallel, then L2, then L3, then L5. Build F2 in parallel as the safety net. Placeholder games need only L1-L3 (the plan is "click `win_btn`").

## 6. Open questions

### RE or live checks (MG-L0)

1. **[U] Does `move_cursor` produce a CEGUI MouseMove on the ExternalWindow?**
   - The lab sets the virtual `GetCursorPos` and then Lua `MouseCursor:setPosition` (`input.rs:259-266`).
   - If the game injects only when its polled position differs from CEGUI's cursor, the Lua step could suppress the event.
   - Fallback: a virtual-cursor-only move, then a one-frame wait.
   - Tell-tale: `processover` never arrives, or the click lands at the old point (the MouseDown uses the stored position, `0x0093aa70`).
2. **[U] Letterbox and offset of the movie inside `Minigame_Movie_Area`.** Proportional mapping is expected; the `pe1` calibration probe settles it.
3. **[U] Rollover feedback latency** (SWF → TCP → server tap → lab-mcp HTTP → lab). It should stay well under 150 ms per probe.
4. **[U] `cover_mc` after `openDoor`.** The rasterisation used frame 1, and the door animates away. The plan's blocked band ignores it; the pe1 probe confirms it.
5. **Dropped.** "Does the client close the window on `onEndMinigame`?" is answered **yes** [V] (§1.1).

### Owner decisions

1. May F2 (X inject) and F3 (GM force-win) be built? F3 is a new GM power and emits a Discord line on each use.
2. For the atlas: generate it from the user's client copy at lab start (the client is not in git), or commit derived geometry from a 2009 asset?
3. Fix the §7 library-suffix bug?
   - It makes boards visibly harder for human UAT: 5 more visible mouse-opaque playfield wires at difficulty 1.
   - The simulation shows the solver copes either way [S].
4. Tier convention: confirm that cursor placement through the virtual `GetCursorPos` plus CEGUI Lua inside an N1 flow stays N1, as `input.rs:1-22` already assumes.

### Out of scope, noted

- **Castle 708.** The `Castle_DHD` step is a Livewire repair (chain 1356, `castle_706_708_chains.sql:785`) followed by the separate DHD dial UI (`DHD.swf`, GateTravel, not a minigame). A DHD tool is its own packet.
- **Unwinnable games.** `CrystalGame` and `GoauldCrystals` fall to the placeholder, but their real SWFs never send `victory`. They cannot be won until they are ported (`crates/minigame/src/minigame/games/mod.rs:10-25`).
- **Placeholder SWFs** (Hack, Activate, Analyze, Bypass, Converse) [V]:
  - Stage 640x480, with `win_btn` at (470.45, 388.95), which sends `victory`.
  - A single rollover-free click wins them.
  - Target the button's centre; its extent is unparsed (DefineButton), so hover-verify is not available because it sends no rollover.

## 7. Side finding: wire library suffix (parity bug)

- **What the SWF exports.** `pGray1..4`, `pRust1..4`, `mYellowStripe1..4`, `gGreenBlack1..4`, `oRed1..4` and so on. There are **no bare `pGray` or `mYellowStripe` symbols** [V].
- **What the original did.** The AS1 extension embedded in the SWF (`setupWires`) builds the library name as `base + randRange(1,4)` for playfield, goal, moving and obstacle wires [V].
- **What Rust does.**
  - `setup.rs:239` and `:306` send bare `pGray` / `mYellowStripe`, so the client's `attachMovie` fails and those wires never exist.
  - `setup.rs:260` and `:326` use `random_range(1..4)`, which gives 1-3, so variant 4 never appears.
- **The Python reference is the source of the bug.** `deprecated/python/base/minigame/Livewire.py:531-560` has the same code, so the Rust is a faithful port of a broken reference.
- **Solver impact.** The solver treats unattachable libraries as absent (§2.3, `unattached`).
