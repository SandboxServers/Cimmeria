# Player controller mouse and movement input model

Status: static RE (headless Ghidra, `SGW.exe`, read-only), 2026-10-22. Not yet confirmed live. Rows marked *inferred* map a handler to an action by subscription order and body shape, not by a resolved action name.

Why this exists: the owner moves mostly with the mouse (LMB drag orbits, RMB drag turns the character, both buttons walk toward the camera). The 2026-10-22 Cellblock run captured none of that beyond throttled `MouseClick` (252) and `MouseLook` (370) CME event names. This page is the hook map for the lab-nav-telemetry campaign ([docs/analysis/lab-nav-telemetry/](../../analysis/lab-nav-telemetry/README.md)).

Related: [cegui-mouse-input-feed.md](cegui-mouse-input-feed.md) (camera offsets proven live, section 10).

## 1. The three mouse behaviours

| What the player does | Code path |
|---|---|
| LMB drag orbits the camera | LMB handler `0x00e85710` → shared button helper `0x00e85540` subscribes MouseMove `0x00e83f40` → camera AddYaw `0x00e7e600` / AddPitch `0x00e7e590` |
| RMB drag turns the character | RMB handler `0x00e85860` sets controller `+0x498` bit 0 → pawn rotation override `0x00e7b240` copies the camera rotator (`+0x368`/`+0x36c`/`+0x370`) onto the pawn while `pawn+0x16c > 0`; MouseMove orbits as for LMB |
| Both buttons walk toward the camera | Either handler sets `+0x498` bit 3 when `+0x49c` has both bits → PlayerTick `0x00e84d70` adds 1.0 to `aForward` every tick while `+0x498 & 0xc`; the pawn yaw equals the camera yaw, so it walks where the camera looks |
| Short LMB click (drag < 5 counts both axes) | `0x00e84ac0`: select the entity under the cursor (`0x00def320`, or clear with 0) |
| Short RMB click (drag < 5 counts) | `0x00e84b20`: interact/use the entity under the cursor (`0x00def4b0`) unless it is the player's own id. This is why an RMB click turns the character toward the NPC first. |

## 2. `ASGWController_Player`

Vtable `0x019e2b2c`. Constructor `0x00e85680` clears `+0x498` bits 0, 2 and 3 (`& 0xfffffff2`) and the low bits of `+0x49c`. Slot 88 (`0x019e2c8c`) is PlayerTick, overridden by `0x00e84d70`.

| Offset | Meaning |
|---|---|
| `+0x2b0` | camera (`ASGWCamera_Player`, vtable `0x019e1134`) |
| `+0x2b8` | bit 1 set by the jump handler |
| `+0x3f4` | `PlayerInput*` |
| `+0x498` | mode bits: 0 RMB mouselook (pawn faces camera), 1 jump held, 2 autorun, 3 mouse-walk (both buttons) |
| `+0x49c` | button bits: 0 LMB held, 1 RMB held |
| `+0x4a0` / `+0x4a4` | drag accumulators, mouse counts, reset on each press |
| `+0x4a8` / `+0x4b4` | ground-target point (cursor ray, `0x00e84b70`) |
| `+0x4c0` | hover entity id |

### Action subscriptions

`0x00e85c00` subscribes, `0x00e85d30` unsubscribes (then drops MouseMove `0x00e83f40`). Order and meaning:

| # | Handler | Meaning |
|---|---|---|
| 1 | `0x00e85710` | LMB (MouseClick) |
| 2 | `0x00e83e10` | forward: clears autorun (bit 2), +1 `aForward` unless bit 3 |
| 3 | `0x00e83e50` | back: clears autorun, -1 `aForward` |
| 4, 5 | `0x00e83e80`, `0x00e83eb0` | strafe ±1 (`PlayerInput+0xc0`) *inferred left/right* |
| 6, 7 | `0x00e84e60`, `0x00e84f90` | turn: `20000 * dt` on `PlayerInput+0xa4`, 0 when `pawn+0x16c == 0` *inferred left/right* |
| 8 | `0x00e850c0` | jump/up (`PlayerInput+0xc4`), sets `+0x2b8 \|= 2`, `+0x498` bit 1 = held |
| 9 | `0x00e853d0` | crouch toggle *inferred* |
| 10 | `0x00e85860` | RMB (MouseLook) |
| 11, 12 | `0x00e83ee0`, `0x00e83f10` | camera vtable `+0x2f8` / `+0x2f4`: zoom in/out *inferred direction* |
| 13 | `0x00e83a10` | up impulse *inferred* |
| 14 | `0x00e85a90` | ToggleWalk (`aMovementType` event) |
| 15 | `0x00e83a50` | ToggleAutorun: `+0x498 ^= 4` |

### Shared button helper `0x00e85540 (this, event)`

Reads the event's released flag. Sets camera `+0x348` bit 1 to "released" (the camera chases the pawn yaw while moving only when no button is held). On press it zeroes `+0x4a0`/`+0x4a4` and subscribes MouseMove through `0x00e87710`; on release it unsubscribes through `0x00e876a0`. Returns true on press.

### MouseMove `0x00e83f40 (this, int* d)`

`+0x4a0 += d[0]`, `+0x4a4 += d[1]`, then camera vtable `+0x300` (AddYaw, slot `0x019e1434`) with `(float)d[0]` and `+0x2fc` (AddPitch, slot `0x019e1430`) with `(float)d[1]`.

### PlayerTick `0x00e84d70`

1. Ground-target mode set → `0x00e84b70` (cursor ray to ground, writes `+0x4a8`/`+0x4b4`).
2. CEGUI not owning the mouse (`0x0056b890 == 0`) → hover pick `0x00e84860` (entity trace, APawn fallback) → `0x00e83cb0(this, id)` hover highlight; `+0x4c0` holds the id.
3. `+0x498 & 0xc` → `aForward += 1`, then `0x00e83d60` (backward-walk flag when `aForward < 0`).

`0x00e83fa0` deprojects the cursor to a world ray, optionally offset by the accumulated drag.

## 3. Camera `ASGWCamera_Player`

Offsets as in [layout.rs](../../../crates/client-telemetry/src/hooks/camera/layout.rs): flags `+0x348`, gain `+0x358`, target distance `+0x360`, smoothed distance `+0x364`, pitch `+0x368`, yaw `+0x36c`, roll `+0x370`.

- AddYaw `0x00e7e600`: `yaw += gain * dx * (flags & 4 ? DAT_01814190 : 1)`, wrapped to ±0x8000. AddPitch `0x00e7e590` is the same shape on pitch.
- Update `0x00e7eae0` (vtable data `0x019e1264`): smooths `+0x364` toward `+0x360`; when the pawn speed is above epsilon and flags bit 1 is set, chases yaw toward the pawn rotation (`pawn+0xec`); a collision trace pulls the camera in.

## 4. Below the controller

| Layer | Address | Notes |
|---|---|---|
| DirectInput pump | `0x01234770` | device `*(0x01f053d4)`: Acquire (vt `+0x1c`), SetCooperativeLevel (`+0x34`), Poll (`+0x64`); loops GetDeviceData (vt `+0x28`, index 10) one 0x14-byte record at a time. `dwOfs` 0 = X, 4 = Y (negated), 8 = wheel (<0 `0x005820f0`, >0 `0x00581fb0`). Emits InputAxis with dt (`+0x4d8`/`+0x4dc` last timestamps) and `0x00581a00(InputMgr 0x005822c0, dx, 0)` / `(0, dy)` per record |
| WndProc | `0x012357d0` | button down/up, client x,y, wheel, focus |
| Keyboard | `0x00581c30` → `0x00581a70` | CEGUI first (`0x0056b130`), then MapVirtualKeyEx/ToUnicodeEx text injection, modifiers at `+0x14`, CME key event. **Never log keys or text.** |
| Mouse button to CEGUI | `0x00581f80` | CEGUI-handled vs world |
| UI MouseLook | `0x0056c620` (SGWUIManager) | `+0x4a` counter ±1, `+0x49` active; hides/shows cursor (vt `+0x38`), saves/restores cursor at `+0x50`/`+0x54`, calls CEGUI `0x011aeaa0` |

`PlayerInput` (`*(PC+0x3f4)`): `+0xa4` turn, `+0xa8` forward, `+0xc0` strafe, `+0xc4` up.

## 5. Hook plan (summary)

All aggregated (per ~100 ms window or per transition), no per-frame rows, no keys or text. Detail and packets: [lab-nav-telemetry work-packets](../../analysis/lab-nav-telemetry/work-packets.md).

1. GetDeviceData post-call aggregate (dx, dy, wheel, records). The existing DInput hook (`crates/client-telemetry/src/bridge/input/dinput.rs`) is behind `#[cfg(feature = "lab-bridge")]`; the player DLL needs a non-lab read-only hook.
2. Inline hooks on `0x00e85710` / `0x00e85860`: press/release, drag totals on release, click class (select via `0x00e84ac0`, interact via `0x00e84b20`, with the picked id).
3. `+0x498` mode transitions (mouselook, jump, autorun, mouse-walk), sampled.
4. Camera AddYaw/AddPitch/zoom aggregates.
5. A ~4 Hz pose sample while anything changes: camera yaw/pitch/distance, pawn location and rotator, `PlayerInput` forward/strafe/turn, hover id.
6. Hover changes via `0x00e83cb0`.
7. CEGUI-handled vs world button routing (`0x00581f80`).

## 6. Poll model (LN-15)

Static RE, headless Ghidra (`tools/re/ghidra-headless/Probe.java`, read-only), 2026-10-22. Marks: **static** = read from the decompile or disassembly; **inferred** = standard UE3/CEGUI 0.6 shape, not traced instruction by instruction. Nothing here is confirmed live; LN-13 does that.

### 6.1 `0x0056b890` "CEGUI owns the mouse" is a hit test, not a field

**static.** The only caller is PlayerTick (`0x00e84dab`). `this` is the `SGWUIManager` singleton (getter `0x0056f980`, global `g_SGWUIManager_ptr` at VA `0x01ee2a58`). The function:

1. Returns false if `UIMgr+0xc` (the CEGUI `System*`) or `UIMgr+0x38` (cursor provider) is null.
2. Calls cursor provider vtable `+0x30` for the integer cursor position, converts to float.
3. Calls `Window::getChildAtPosition` `0x011a55b0` on the active sheet `System+0x2c`.
4. Returns true when the hit window is non-null, is not the sheet itself, and its name (wide string at `window+0x1e8`) compared with `L"BackgroundWin"` (`0x0183f8c8`, compare `0x00570ff0`) is non-zero.

So there is no stored flag to poll: the answer is recomputed by a recursive child walk every tick.

**Nearest pollable chain (static, with one caveat below):** CEGUI `System::injectMouseMove` `0x011ae5b0` stores the window under the mouse at `System+0x28` (`d_wndWithMouse`) after calling `getTargetWindow` `0x011ade50`.

| Step | Read | Type |
|---|---|---|
| 1 | `*(u32*)0x01ee2a58` → `UIMgr` | pointer, may be 0 before UI init |
| 2 | `*(u32*)(UIMgr+0xc)` → `System` | pointer |
| 3 | `*(u32*)(System+0x28)` → `wndWithMouse` | `CEGUI::Window*`, may be 0 |
| 4 | `*(u32*)(System+0x2c)` → active sheet | `CEGUI::Window*` |
| 5 | name at `wndWithMouse+0x1e8` | MSVC `std::wstring` (length u32 at `+0x14` of the string, so `window+0x1fc`) *inferred layout* |

"CEGUI owns the mouse" ≈ `wndWithMouse != 0 && wndWithMouse != sheet && name != "BackgroundWin"`. LN-10 can resolve the BackgroundWin window pointer once by name and compare pointers after that.

Caveat (static): this is not the same predicate as `0x0056b890`. `getTargetWindow` returns the capture window (`DAT_01f00278`, CEGUI `Window::d_captureWindow`) when one is set, falls back to the sheet when nothing is hit, and is clamped to the modal target `System+0x30`. `d_wndWithMouse` is updated only on mouse-move injection, so it can be stale while the cursor is still or hidden (UI MouseLook hides it, `0x0056c620`). A read-only post-call capture of AL at `0x0056b890` would be exact; that is a hook, not a poll (D-LN5).

Also seen: CEGUI `System::injectMouseButtonUp` is `0x011b0040` (`System+0x50` sys-key bits, `+0x74` click trackers, parent link `window+0x48`). `0x00581f80` sends every button to CEGUI (`0x0056b1b0`) and then to the world handler `0x00581dc0`.

### 6.2 Drag counters on release

**static.** Helper `0x00e85540`: the press branch (`released == 0`) zeroes `+0x4a0`/`+0x4a4` and subscribes MouseMove; the release branch only unsubscribes (`0x00e876a0`) and never writes them. It returns true on press. LMB `0x00e85710` and RMB `0x00e85860` test `|+0x4a0| < 5 && |+0x4a4| < 5` after the helper returns false, so the release tick reads the final drag. `0x00e84ac0`/`0x00e84b20` themselves never read the counters. D-LN5 holds.

RMB detail: `+0x498` bit 0 is set to the helper's return (press = 1) and the click branch runs only when bit 0 is 0, that is on release.

### 6.3 Click targets come from a fresh trace

**static.** Both `0x00e84ac0` (select, `0x00def320`) and `0x00e84b20` (interact, `0x00def4b0`) call the pick `0x00e84860` again: cursor deproject `0x00e83fa0`, world trace `0x008721b0`, APawn fallback `0x00875970`. Neither reads the hover id `+0x4c0`. Both skip when `g_dwMapCheckDepEnabled != 0 && DAT_01eb082c == 0`. `0x00e84b20` also skips the player's own id (`*(PC+0x1b0)+0x1b4`). A telemetry row that wants the picked id must take it from those calls, not from `+0x4c0`.

### 6.4 PlayerTick and the engine tick

**static:** PlayerTick `0x00e84d70` has one reference, vtable slot `0x019e2c8c` (no direct caller). `FEngineLoop::Tick` `0x00416ec0` calls the engine vtable `+0x10c` with `GDeltaTime` (`DAT_01daea88`), then pumps Windows messages (`PeekMessage`/`DispatchMessage`, so WndProc `0x012357d0`) after that. **inferred:** engine `+0x10c` is `UGameEngine::Tick` → world tick → actor ticks → PlayerTick, inside `FEngineLoop::Tick`. The DirectInput pump `0x01234770` is a vtable method (data ref `0x01ac7b88`), reached from viewport input processing during the engine tick (*inferred*). A poll at the top of the existing `FEngineLoop::Tick` detour therefore sees the previous frame's controller state.

Annotation note: the Ghidra global `g_pFlashExternalWindowModule` used here is `GEngine` (`+0x2d0` GamePlayers, `[0]+0x40` actor, `+0x2b0` camera in `0x00e84860`). Misnamed, see [annotation-script-shift-bugs.md](annotation-script-shift-bugs.md).

### 6.5 PlayerInput axes

**static, floats:** `+0xa8` forward (`+= g_flOnePointZero` in `0x00e83e10` and PlayerTick), `+0xc0` strafe (`-= 1.0` in `0x00e83e80`), `+0xa4` turn (`-= (float)20000 * time` in `0x00e84e60`), `+0xc4` up (`+= time * DAT_0181418c` in `0x00e850c0`). Every handler accumulates (`+=`/`-=`), never stores. **inferred:** they are UE3 `input float` axes zeroed each frame by `UPlayerInput` before bindings run; nothing in the controller clears them. A poll reads the last frame's summed value.

### 6.6 Writers of `+0x498`

**static,** instruction scan `0x498]` across the binary, controller range only (other hits belong to unrelated classes at `0x005d5520`, `0x00828240`, `0x009xxxxx`, `0x00bxxxxx`, `0x012xxxxx`). No byte writes to `+0x499..+0x49b` in the controller.

| Address | Function | Write |
|---|---|---|
| `0x00e83a73` | ToggleAutorun `0x00e83a50` | `^= 4`, skipped when `g_dwMapCheckDepEnabled != 0 && DAT_01eb082c == 0` |
| `0x00e83e10` | forward `0x00e83e10` | `&= ~4` |
| `0x00e83e56` | back `0x00e83e50` | `&= ~4` |
| `0x00e853a9` | jump `0x00e850c0` | bit 1 = held (`!released`) |
| `0x00e856d5` | constructor `0x00e85680` | `&= 0xfffffff2` (bits 0, 2, 3) |
| `0x00e8583e` | LMB `0x00e85710` | bit 3 = both buttons held |
| `0x00e85946` | RMB `0x00e85860` | bit 0 = press |
| `0x00e859a4` | RMB `0x00e85860` | bit 3 = both buttons held |

Readers: `0x00e7b256` (pawn rotation override, bit 0), `0x00e84b79` (ground target, bit 0), `0x00e84e27` (PlayerTick, `& 0xc`), `0x00e83e17` (forward, bit 3), `0x00e85260` (jump, bit 1), `0x00e8594c` (RMB, bit 0). The scan finds only `[reg + 0x498]` forms; a write through a computed base would be missed.
