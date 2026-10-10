---
type: reference
audience: Contributors to the lab input bridge (crates/client-telemetry) and the Livewire autosolve work; minigame-systems-advisor
last_updated: 2026-10-10
companion_docs:
  - ../../analysis/cellblock-autoplay/livewire-autosolve.md
  - client-engine-sinks-and-seams.md
---

# CEGUI Mouse Input Feed (SGW.exe) - RE Findings

**Date**: 2026-10-10
**Analyst**: Game Archaeology Specialist
**Method**: headless Ghidra (`tools/re/ghidra-headless/Probe.java`), static decompile only. No live debugger run.
**Confidence**: [V] = read in a decompile, [U] = inferred.

---

## Summary

CEGUI is fed from UE3's `FWindowsViewport`, through the `SGWUIManager` singleton. The mouse-move feed is
triggered by DirectInput mouse X/Y delta records in the viewport's per-tick input pump, not by `WM_MOUSEMOVE` and
not by a free-running `GetCursorPos` poll. `GetCursorPos` is only the position source once the pump has fired.
The feed calls the absolute `System::injectMousePosition`, never the delta `injectMouseMove`.

This explains the lab observations: `MouseCursor:setPosition` sticks (nothing re-injects), and a posted
`WM_MOUSEMOVE` never reaches CEGUI. It also corrects `livewire-autosolve.md` section 1.2, which says "the game
feeds CEGUI from `GetCursorPos`".

---

## 1. CEGUI::System injectors

| Function | Address | Signature | Evidence |
| --- | --- | --- | --- |
| `System::getSingleton` | `0x011aeaf0` | `__cdecl System*()`; global `DAT_01f010c8` (`ms_Singleton`, asserts if null) | [V] decompile |
| `System::injectMouseMove` | `0x011ae5b0` | `__thiscall char (System*, float dx, float dy)`, `ret 8`, returns "handled" | [V] |
| `System::injectMousePosition` | `0x011aeaa0` | `__thiscall void (System*, float x, float y)`, `ret 8` | [V] |
| `System::injectMouseWheelChange` | `0x011ae9a0` | `__thiscall void (System*, float z)` | [V] |
| `System::injectMouseButtonDown` | `0x011afe40` | `__thiscall char (System*, int MouseButton)`, `ret 4` | [V] |
| `System::injectMouseButtonUp` | `0x011b0040` | `__thiscall char (System*, int MouseButton)`, `ret 4` | [V] |
| `System::injectKeyDown / KeyUp / Char` | `0x011ae710` / `0x011ae7f0` / `0x011ae8d0` | `KeyEventArgs` dispatch (not mouse) | [V] |
| `MouseCursor::getSingleton` | `0x011c1e10` | global `DAT_01f0124c`; visible flag at `+0x28`, position `+0x1c/+0x20` | [V] |
| `MouseCursor::setPosition` / `offsetPosition` | `0x011c1e50` / `0x011c1e70` | both end in clamp `0x011c1ac0` | [V] |

Names are mine. SGW.exe carries no injector symbols, and no `injectMouse*` strings exist, so the identity rests on
the bodies: `MouseEventArgs` vftable, `MouseCursor` singleton use, window hit-test `0x011ade50`, then virtual
dispatch to `Window::onMouseMove` and its siblings.

- `injectMousePosition` is `MouseCursor::setPosition(x, y)` followed by `injectMouseMove(0.0, 0.0)` [V]. There is no
  zero-delta early-out, so a `MouseMove` event with the new position still reaches the target window. CEGUI's
  `DragContainer` threshold logic is position-based, so a position-only move is enough [U, CEGUI 0.5/0.6 source
  knowledge].
- `injectMouseMove` multiplies the delta by the scaling factor at `System+0x9c`, offsets the cursor, hit-tests,
  and fires enter/leave then move [V].
- `injectMouseButtonDown/Up` read the CEGUI cursor position (`MouseCursor+0x1c/+0x20`). They take no coordinates
  [V]. That is why posted `WM_LBUTTONDOWN/UP` act at the CEGUI cursor, whatever the OS cursor says.

---

## 2. Who feeds them

`SGWUIManager` singleton: getter `0x0056f980` (global `g_SGWUIManager_ptr`, lazy-created, size `0xd4`). Fields used:
`+0x0c` = `CEGUI::System*`, `+0x38` = the `FWindowsViewport*`, `+0x49` = mouse-look active, `+0x4a` = mouse-look
counter, `+0x50/+0x54` = last polled (x, y).

| Step | Address | What it does | Evidence |
| --- | --- | --- | --- |
| Pump | `0x0056b080` | `__fastcall (SGWUIManager*)`. If `+0x49 == 0` and `+0x38 != 0`: calls `viewport->vfunc +0x30` to read the position into `+0x50/+0x54`, then `System::injectMousePosition(x, y)`. Returns the handled flag. If mouse-look is active it does nothing. | [V] |
| Wrapper | `0x00581a00` | `__thiscall (InputMgr*, a, b)`, singleton `0x005822c0` (`DAT_01ee2b1c`). Stores args at `+0x0c/+0x10`, calls the pump, and only if CEGUI did not handle it emits CME `Event_Input_MouseMove` through `FUN_00a372f0`. | [V] |
| DirectInput source | `0x01234770` | `FWindowsViewport` vtable slot at `0x01ac7b88`. Per tick: unacquire, `SetCooperativeLevel(GetForegroundWindow(), 6)`, acquire, `Poll`, `GetDeviceData` into 20-byte records on `DAT_01f053d4`. Record offset 0 (DIMOFS_X) and 4 (DIMOFS_Y) are accumulated, then each is sent to the viewport client's `InputAxis` (`vfunc +0x18`) and then to the wrapper `0x00581a00`. | [V] |
| DirectInput gate | `0x01234770`, top | Runs only if `this+0x34` (viewport client) is non-null and (`this+0x4cc != 0` (mouse captured, set in `0x01233c60`) or `client->vfunc +0x40() != 0`). | [V] gate shape; [U] meaning of `vfunc +0x40` |
| Focus re-sync | `WndProc` case `WM_SETFOCUS (0x7)` in `0x012357d0` | Calls `0x00581a00(0, 0)`, so the CEGUI cursor snaps to the OS cursor when the window gains focus. | [V] |
| Mouse-look exit | `0x0056c620` | Event handler (registered in `0x0056d810` for the MouseLook press/release events). On release it calls `viewport->vfunc +0x34` (`SetCursorPos`) with the saved position and then `injectMousePosition`. | [V] |
| Buttons | `WndProc` `0x012357d0` cases `0x201/0x204/0x207/0x20b` then `0x00581c30` then `0x0056b130` then `0x011afe40` | Down. Up goes through `0x00581f80`, `0x0056b1b0`, `0x011b0040`. The WndProc case also calls `SetFocus` and the viewport client's `InputKey` (`vfunc +0x10`). | [V] |
| Wheel | `WM_MOUSEWHEEL (0x20a)` then `0x00581fb0` then `0x0056b320` then `0x011ae9a0` | | [V] |

`WM_MOUSEMOVE (0x200)` in `0x012357d0` does this and nothing else: if `this+0x4d0 == 0`, call the viewport client's
`vfunc +0x28` (MouseMove) and set `this+0x4b4 = 1`. It never calls `0x00581a00`, `0x0056b080` or any `System`
injector [V]. Whether the client's `vfunc +0x28` reaches CEGUI was not traced [U: unlikely, since the CEGUI
injectors have only the callers listed here; see the cross-reference list in section 5].

### 2.1 What the position source reads

`FWindowsViewport` vtable `0x01ac7b44` + offset: `+0x30` = `0x01233fd0` = `GetCursorPos` then
`ScreenToClient(hwnd = this+0x88)`, writes (x, y) to the out pointer [V]. `+0x34` = `0x01234010` =
`ClientToScreen` then `SetCursorPos` [V]. `+0x38` = `0x01234050` = `ShowCursor` loop [V]. Siblings
`0x01233f70` / `0x01233fa0` return client X / Y the same way [V].

So the pump uses the **hooked `GetCursorPos`**, converted with the real `ScreenToClient` against the game window.
CEGUI coordinates are therefore client pixels.

### 2.2 Does it compare against the CEGUI cursor?

No. The pump has no delta test and no comparison against the last position or the CEGUI cursor [V]. The only
gates are mouse-look (`+0x49`), the viewport pointer (`+0x38`), and the DirectInput trigger above. `injectMousePosition`
sets the position unconditionally.

---

## 3. user32 `GetCursorPos` import callers

IAT slot `0x017efd70` (name hint at `0x01d6bd2c`). Six call sites [V]:

| Call site | Function | Purpose |
| --- | --- | --- |
| `0x0103a1f0` | `0x0103a100` | wxWidgets editor popup menu (`ScreenToClient`, `PopupMenu`) |
| `0x0103a09b` | `0x01039f10` | wxWidgets editor popup menu |
| `0x01233cf1` | `0x01233c60` | `FWindowsViewport` capture-mouse: saves the OS cursor in `+0x4b4` when starting mouse capture |
| `0x01233f7b` | `0x01233f70` | viewport `GetMouseX` (vtable `0x01ac7b6c`) |
| `0x01233fab` | `0x01233fa0` | viewport `GetMouseY` (vtable `0x01ac7b70`) |
| `0x01233fdb` | `0x01233fd0` | viewport `GetMousePos(out)` (vtable `0x01ac7b74`), the CEGUI feed's source |

The three wx sites are editor-only. Gameplay uses the last four, all inside `FWindowsViewport`.

---

## 4. Recommendation for the injected DLL

Ranked:

1. **Call `System::injectMousePosition` directly on the game main thread (option a).** Signature:
   `void __thiscall (System* ecx, float x, float y)`, address `0x011aeaa0`, `ret 8`. Get `System*` from
   `*(System**)0x01f010c8` (or call `0x011aeaf0`; it asserts on null). Pass client-pixel coordinates. It sets the
   cursor and fires a real `MouseMove` through the normal hit-test, which is exactly what the game's own pump does.
   For a drag: `injectMousePosition(start)`; `injectMouseButtonDown(0)` (`0x011afe40`, `__thiscall`, one int, left
   button is CEGUI `LeftButton == 0` [U]; verify with `0x00941200`, the mapper used by `0x0056b130`); several
   `injectMousePosition` steps that exceed the drag threshold; `injectMouseButtonUp(0)` (`0x011b0040`). Posted
   `WM_LBUTTON*` also works for the button edges, since they use the CEGUI cursor.
2. **Native pump alternative (option b').** Call `0x0056b080` with `ecx = SGWUIManager*` (from `g_SGWUIManager_ptr`,
   or the getter `0x0056f980`). It reads the hooked `GetCursorPos` through the viewport and injects it, so a lab
   that already sets the virtual cursor needs one call per step. It is a no-op while mouse-look is active
   (`+0x49 != 0`) and when `+0x38 == 0`. Avoid `0x00581a00`: on a miss it also emits `Event_Input_MouseMove` into the
   CME bus, which the player controller consumes.
3. **Not viable: changing only the virtual `GetCursorPos` (option b)** or **posting `WM_MOUSEMOVE` (option c).**
   Nothing polls the position on a timer, and the `WM_MOUSEMOVE` handler does not touch CEGUI. The only trigger left
   is DirectInput delta records, which the bridge's cursor virtualisation does not generate.

Risks. All three entry points take floats on the stack, so a hand-built call must push `y` then `x` and rely on the
callee's `ret 8`; a wrong calling convention corrupts the stack silently. Run on the main thread only: CEGUI is not
thread-safe and `FUN_011ade50` walks live window trees.

---

## 5. Open questions

- Meaning of the viewport client's `vfunc +0x40` and `FWindowsViewport+0x4d0` (gates for the DI pump and
  `WM_MOUSEMOVE`). Both are read in the cited decompiles; neither was resolved to a name.
- Whether `USGWViewportClient` (`0x00e83740` is its slot 0) overrides MouseMove (`vfunc +0x28`). Its vtable was not
  located; the `0x019e258c` table the destructor appears in decodes to engine stubs.
- CEGUI `MouseButton` numbering at `0x00941200` / `0x00941250` (not decompiled).
- Not run live: no x64dbg session. A breakpoint on `0x011aeaa0` while moving the real mouse would confirm the
  DirectInput trigger and the call rate.

## 6. Cross-reference targets

- `docs/analysis/cellblock-autoplay/livewire-autosolve.md` section 1.2: replace "the game feeds CEGUI from
  `GetCursorPos`" with this finding's trigger/source split.
- `crates/client-telemetry/src/bridge/input/focus.rs:8-9` comment ("the UI cursor follows the OS cursor") is true only
  after a DirectInput-triggered pump.

---

## 7. Follow-up: DragContainer drop resolution (2026-10-10)

Live result that prompted this: native `injectMousePosition` + `injectMouseButtonDown/Up` starts a real inventory drag
(`getDragInfo` returns 3, the container follows the cursor), but no `DragDropItemEnters` or `Dropped` ever fires.

### 7.1 DragContainer vtable and fields

`DragContainer` vtable `0x01aaedc4`. Window event slots: enters `+0x90`, leaves `+0x94`, move `+0x98`, wheel `+0x9c`,
down `+0xa0`, up `+0xa4` [V, matches the call offsets in `injectMouseMove/ButtonDown`]. Window drag-drop notifiers:
`+0xc0` Enters, `+0xc4` Leaves, `+0xc8` Dropped [V]. Fields: `+0xd8` enabled, `+0xd9` visible, `+0xfc`
DragDropTarget flag (`isDragDropTarget`, `0x011a3360`), `+0x23c` left-button-down, `+0x23d` dragging enabled, `+0x23e`
dragging, `+0x260` drag threshold, `+0x270` d_dropTarget, `+0x274` drag cursor image.

| Function | Address | Role |
| --- | --- | --- |
| onMouseButtonDown | `0x011c0a90` | acts only if `args.button (+0x1c) == 0` (LeftButton == 0 [V]); captures input |
| onMouseMove | `0x011c0bc0` | if dragging (`+0x23e`): `doDragging`. Else if dragging enabled and the threshold test `0x011c0860` passes: `onDragStarted` (vfunc `+0x110`) and **no** `doDragging` on that same move |
| doDragging | `0x011c0980` | moves the container, then calls `onDragPositionChanged` (vfunc `+0x118`) |
| onDragPositionChanged | `0x011c0e30` | the drop-target logic, below |
| onDragDropTargetChanged | `0x011c1030` (vfunc `+0x12c`) | Leaves on the old target, then sets `d_dropTarget`, walks up parents until `isDragDropTarget` (`+0xfc`) is true, then Enters |
| onMouseButtonUp | `0x011c0b20` | acts only if `args.button == 0` and `+0x23e`: `onDragEnded` (vfunc `+0x114`), then releases capture |
| onDragEnded | `0x011c0dd0` | fires DragEnded, then Dropped on `d_dropTarget` (`0x011a2930`) **only if non-null** |

`onDragPositionChanged` (disassembly [V]): fires `DragPositionChanged`; `root = System+0x2c` (the GUI sheet; null means
nothing happens); saves `this+0xd8`, **sets it to 0** (the "enabled hack"), calls
`root->getTargetChildAtPosition(MouseCursor pos +0x1c/+0x20)` (`0x011a54a0`), restores `+0xd8`; falls back to `root`
when null; if the result differs from `d_dropTarget` it fires `onDragDropTargetChanged`.

`getTargetChildAtPosition` (`0x011a54a0`): children in reverse (top first); needs the child visible and effectively
visible; recurses, then `child->isHit(pos)` (vfunc `+0xc`, `0x011a1b50`). `isHit` fails if the window or an ancestor is
disabled, or the clipped pixel rect is empty or misses the point. **It has no MousePassThrough test** [V], so
`MousePassThroughEnabled` on the dragged item is not what blocks the drop.

### 7.2 Why no Enters: ranked suspects

1. [U, most likely] **Too few moves.** The move that crosses the threshold only starts the drag; `doDragging`, and so
   target selection, runs on the next move. A single jump from source to target, or one move after the threshold
   crossing, leaves `d_dropTarget` null. `onDragEnded` then drops nothing. Neither can `Enters` fire.
2. [U] **Target not flagged DragDropTarget.** Enters goes to the nearest ancestor of the hit window with `+0xfc` set,
   not to the hit window. If Slot3 and AllSlotFrame have `DragDropTarget=False` in the layout, another window gets it.
   Subscribe to the dragged container's `DragDropTargetChanged` to see which window was chosen.
3. [U] `System+0x2c` (GUI sheet) null or not the ancestor of the inventory windows.
4. A non-left button index in Up. Left must be exactly 0 [V].

### 7.3 Game button path beyond the injector

`WM_*BUTTONDOWN` in `0x012357d0`: constructs a scope object (`0x01234130`), `SetFocus(hwnd)`, viewport client
`InputKey` (`vfunc +0x10`), then `0x00581c30` -> `0x0056b130`. `0x0056b130` checks `SGWUIManager+0x38 != 0`, maps the
button with `0x00941200`, calls `injectMouseButtonDown`. If CEGUI did not handle it, `0x00581c30` falls through to the
CME key-press path `0x00581a70`. `WM_*BUTTONUP`: `0x00581f80` -> `0x0056b1b0` -> `injectMouseButtonUp`, then
`0x00581dc0` (CME release). No capture call, no `setMouseCursor`, and no drag-manager state are touched [V]. Capture is
taken inside `DragContainer::onMouseButtonDown`.

### 7.4 SGW-side drag manager

It is plain state on the `SGWUIManager` singleton, with no per-tick drop computation [V]. `dragItem(a,b,c)`
(`0x00ad8c90`) stores type 3 at `+0x74`, the item id at `+0x78` (looked up in the player's container table), quantity at
`+0x7c`. `getDragInfo` (`0x00adb280`) returns the type and a table keyed by that type (3: container, slot, quantity;
2: action; 5: slot, quantity; 6: mailId; 7-9: unitId, id; 10: value1, value2). `dragStarted(window)` (`0x0056aac0`)
reads the viewport mouse position and, if the window is a `DragContainer`, calls its vfuncs `+0xa0` and `+0x110`,
which is a synthetic ButtonDown plus `onDragStarted`. `dragAborted` (`0x0056ad50`) is a thin wrapper. Drop resolution
therefore lives in CEGUI events and the layout's Lua handlers. The code that clears `+0x74` was not found.

### 7.5 Recommended native sequence

1. `injectMousePosition(source centre)`; wait a frame (hover).
2. `injectMouseButtonDown(0)`.
3. `injectMousePosition` in several steps, one per frame: the first beyond `DragThreshold` (read `container+0x260`)
   starts the drag; then at least two more moves, the last ones over the target.
4. Read `*(Window**)(container+0x270)`. Non-null means a drop will resolve.
5. `injectMouseButtonUp(0)` while still over the target.

Open: the clearing site for `SGWUIManager+0x74`; the layout's `DragDropTarget` values; `0x011f3460` (screen to window
conversion) not decompiled.

## 8. Follow-up: d_dropTarget stays 0 after a live injected drag (2026-10-10)

Live memory diff: container vtable `0x01AAEDC4`, `+0x260 = 8.0`, `+0x270` is 0 throughout, and the
`DragDropTargetChanged` Lua handler fires 7 times mid-drag and once at release, with a nil first argument.

- **The sheet is non-null [V by inference].** In `onDragPositionChanged` (`0x011c0e30`) the whole target block is
  skipped when `System+0x2c` is 0. The `DragDropTargetChanged` event fired, and it is fired first thing inside
  `onDragDropTargetChanged` (`0x011c1030`, vfunc `+0x12c`), which is reachable only past that test. The target passed in
  is never 0 either (null falls back to the sheet).
- **Why `+0x270` stays 0 [V code, U cause].** `0x011c1030` stores `args.window` (`args+8`) into `+0x270`, then loops
  `while (+0x270 && !*(byte*)(+0x270 + 0xfc)) +0x270 = +0x270->parent (+0x48)`. If no window from the hit window up to
  the sheet has the DragDropTarget flag (`+0xfc`, `isDragDropTarget` at `0x011a3360`), the loop ends at 0 and nothing
  receives Enters. Because `0 != hitWindow` on every move, the event re-fires each move, which matches 7 firings.
  Dropped needs a non-null `+0x270` at `onDragEnded`, so none fires. The Window constructor default for `+0xfc` was not
  located (stock CEGUI defaults to true), so the layout may be clearing it.
- **Other writers of `+0x270`:** the constructor (`0x011c10f0`, also sets `+0x274 = -1`), `onDragDropTargetChanged`,
  and `FUN_011c0c90`, which is `onCaptureLost` (vtable `+0x68`, `0x01aaee2c`). `onCaptureLost` ends the drag
  (`+0x23e = 0`), restores the original area and alpha, and zeroes `+0x270`. It runs from `releaseInput` right after
  `onDragEnded`, so it explains the post-release 0. The extra firing at release is not explained; `onCaptureLost` does
  not fire the event.
- **Nil Lua argument [U].** The event's args are built as a `WindowEventArgs`-style struct (vftable `0x01aa8d4c`) with
  the target in `+8`. A nil first argument is most likely the binding reading a `DragDropEventArgs` field (`dragDropItem`,
  `+0xc`) that this struct does not carry. It does not show that the target was null.
- **What to read live.** `sheet = *(u32*)(System+0x2c)`. Walk `w = InventoryWin; while (*(u32*)(w+0x48)) w = *(u32*)(w+0x48)`
  and compare the top with `sheet`. For Slot3, its parents and the sheet, read the byte at `+0xfc`. Expect all 0.
- **Fix.** Set DragDropTarget true on the intended target (Lua `setProperty("DragDropTarget","True")`, or write the
  byte at `window+0xfc`). The next move gives Enters and Up gives Dropped. Direct fallback after the normal Up:
  `notifyDragDropItemDropped` = `0x011a2930`, `__thiscall (target*, draggedContainer*)`, `ret 4`, which fires the
  target's Dropped event. Swapping the sheet is unnecessary.
- **Which container is dragging [V live].** In the same live drag the container's flag word at `+0x23c` read
  `0x00000001` after the press and `0x00010101` once dragging (the `+0x23e` dragging byte set), and its alpha at
  `+0x070` went from 1.0 to 0.5. `getDragInfo` is global SGW drag state and would also report another item's drag, so
  the lab gates the explicit drop on the source container's own `+0x23e` byte.

## 9. Follow-up: camera yaw/pitch and zoom through DirectInput (2026-10-10)

Sources: the pump `0x01234770` and `FWindowsViewport::WndProc` `0x012357d0` (decompiles already cited above),
`UWindowsClient::Init` `0x012329e0` (`WinClient.cpp`), capture function `0x01233c60`, and
`Content/XML/BindableActions.xml` in the client tree.

### 9.1 The live mouse device [V]

The game has exactly one DI mouse: `DAT_01f053d4`, created in `UWindowsClient::Init` (`0x012329e0`, `WinClient.cpp`
lines 0x131-0x13d) from the `IDirectInput8` in `DAT_01f053d8` (version 0x800) with `GUID_SysMouse`. It uses
`c_dfDIMouse`, `DIPROP_BUFFERSIZE = 0x3ff` (buffered), and `DIPROP_AXISMODE = 1` (relative). The keyboard device is
created but never read. The bridge should identify the live device by pointer: in its `GetDeviceData` hook compare
`this == *(void**)0x01f053d4`. COM vtables are shared across device instances, so a vtable hook alone cannot tell the
game's mouse from the other 3 mouse devices (other libraries).

### 9.2 What the pump does with a buffered record [V]

`GetDeviceData` (vtable `+0x28`), element size `0x14` (`DIDEVICEOBJECTDATA`), flags 0 (consume), not
`GetDeviceState`. The pump calls `Unacquire`, `SetCooperativeLevel(GetForegroundWindow(), 6)` (non-exclusive,
foreground), `Acquire`, `Poll` (`+0x64`) and requires `Acquire` to succeed. Records are read with a requested count of 1
per call, and the loop ends when the returned count is 0. A hook must honour `*pdwInOut` and set it to the number
delivered. Per record by `dwOfs`:

| dwOfs | Meaning | Action |
| --- | --- | --- |
| 0 | X | queue (dwTimeStamp, dwData) |
| 4 | Y | queue (dwTimeStamp, dwData) |
| 8 | wheel | dwData < 0 down, > 0 up (see 9.4) |
| others | buttons | ignored (buttons come only from window messages) |

After the loop, per queued X sample: `client->InputAxis` (client vtable `+0x18`; client = `*(viewport+0x38)`) with
`(viewport, 0, FName MouseX = DAT_01ee1fe8/1fec, (float)dwData, dt)`, where `dt = (dwTimeStamp - viewport+0x4d8) * DAT_017fff2c`
(0.001 [U]); then `0x00581a00(InputMgr, dx, 0)`. Y is the same with `MouseY = DAT_01ee1ff0/1ff4` at `+0x4dc` and
`0x00581a00(InputMgr, 0, dy)`. `dwTimeStamp` therefore needs to be monotonic milliseconds; an all-zero stamp gives
`dt = 0`. Raw counts are passed unscaled; sensitivity is applied downstream (`mouseLookSensitivity` in
`SystemOptions.xml`).

`0x00581a00` (InputMgr singleton getter `0x005822c0`, object `0x01ee2b1c`) pumps CEGUI (`0x0056b080`) and, when CEGUI
did not handle it (always the case while mouse-look is active), emits CME `Event_Input_MouseMove{dx, dy}` through
`FUN_00a372f0`. `ASGWController_Player` subscribes to that event (RTTI `MemberCallback<ASGWController_Player,
Event_Input_MouseMove>`) [V]; its handler address was not located, so whether the camera turn is driven by that event,
by `InputAxis` into `UPlayerInput`, or both is [U].

### 9.3 Gating

- DI is read only if the viewport client (`viewport+0x38`) is non-null and (`viewport+0x4cc != 0`, the captured flag set
  by `0x01233c60`, or `client->vfunc +0x40() != 0`) [V]. The meaning of `vfunc +0x40` is unresolved (a
  "wants polling mouse movement" test in stock UE3 [U]).
- The capture function `0x01233c60` accepts only if `GetFocus()` (windowed) or `GetForegroundWindow()` (fullscreen)
  equals `viewport+0x88` (hwnd). Virtual focus satisfies this [V]. On acquiring capture it runs a flush loop of
  `GetDeviceData` calls that drains the buffer. Inject records only after the capture flag reads 1, or they are eaten.
- Mouse-look is the bindable action `MouseLook` (`Event_Action_MouseLook`, `defaultBind RMOUSE`, tooltip "Hold this to
  look around with the camera, click to interact", `releaseEvent=1`) [V]. The handler `0x0056c620` keeps a counter at
  `SGWUIManager+0x4a`: press +1, release -1 (by the event's "released" argument); `+0x49 = (count != 0)`. It also
  hides the cursor (`viewport vfunc +0x38`). So `+0x49` only gates the CEGUI feed and the cursor. A held RMB is
  required for the camera [U, strong: the action text, plus the click router `0x00e85860` on the same event].
- Chain for RMB: `WM_RBUTTONDOWN (0x204)` -> `0x00581c30(2)` -> (if CEGUI did not consume it) `0x00581a70` -> CME
  `Event_Input_KeyPress` -> `BindableActionManager` (RTTI subscriber [V]) -> `MouseLook`. A posted RBUTTONDOWN therefore
  works, and also calls the client's `InputKey`.

### 9.4 Wheel and zoom

`CameraZoomIn` / `CameraZoomOut` default to `MWHEELUP` / `MWHEELDOWN` [V]. Two sources, same effect:
- DI record `dwOfs == 8`, taken in the pump.
- `WM_MOUSEWHEEL (0x20a)` in `0x012357d0`, **only when `viewport+0x4d0 == 0`** [V]. The sign of `HIWORD(wParam)`
  selects the direction.

Both do `client->InputKey(viewport, 0, FName ScrollUp = DAT_01ee1ff8/1ffc or ScrollDown = DAT_01ee2000/2004,
IE_Pressed 0)`, then the same with `IE_Released 1, 1.0f, 0`, then `0x00581fb0(InputMgr, delta)` (up) or `0x005820f0`
(down). Those go to CEGUI's wheel (`0x0056b320` -> `0x011ae9a0`) and, if not handled, to the CME key path with key 300
(`0x00581a70`), which the bindable actions consume. `viewport+0x4d0` is written by the viewport vfunc at `0x01233f60`
(`this+0x4d0 = param`) [V, from the disassembly; the decompiler shows an empty body]. It appears to select "DirectInput
mouse mode", but its callers were not found [U].

### 9.5 Recommendation

Do not rely on the DI hook alone. In order:

1. **Mirror the pump's two calls natively on the main thread**, per sample, with RMB held (post `WM_RBUTTONDOWN`, wait
   a frame, release after): `client->InputAxis(viewport, 0, MouseX|MouseY FName, (float)counts, dt_seconds)`
   (vtable `+0x18`) and `0x00581a00(*InputMgr 0x005822c0, dx, dy)`. This skips capture, focus, `Acquire` and
   the buffer flush. Use `viewport = *(SGWUIManager+0x38)`, `SGWUIManager` from `0x0056f980`.
2. **Zoom:** post `WM_MOUSEWHEEL` if `*(u32*)(viewport+0x4d0) == 0`; otherwise call `0x00581fb0` / `0x005820f0` on
   `InputMgr` natively (up/down), preceded by the two `InputKey` calls for fidelity.
3. If the DI hook is kept: deliver records only after `viewport+0x4cc == 1`, one at a time, honour `*pdwInOut`, use a
   monotonic millisecond `dwTimeStamp`, `dwOfs` 0/4/8, signed `dwData`.

Open: why `events_delivered` is 0 (candidate causes: the gate in 9.3 being false while no capture is held, the capture
flush eating records, or the hook matching the wrong device); the `ASGWController_Player` mouse-move handler and its
gain; the meaning of `vfunc +0x40` and `+0x4d0`.

## 10. Follow-up: finding the local PlayerController and camera (2026-10-10)

Method: headless Ghidra on SGW.exe plus read-only `client_mem_read` on the live client (in world). [V] = static and live agree.

### 10.1 Why the old chain returns null

`g_pGLevel` (`0x01EE2684`, live value 0xE5712210) is the `UWorld` [V]. The lab chain `[[[[g_pGLevel+0x50]+0x3C]]+0x35C]` is exactly `0x0054d8d0`: `*(**(*(w+0x50)+0x3c)+0x35c)`. Level = `*(w+0x50)`, its Actors array data = `*(level+0x3c)`, count = `*(level+0x40)` (live 0x32). `Actors[0]` is the `AWorldInfo` (vtable `0x018a02b4`). The hops are right, but `WorldInfo+0x35c` is 0 live. The field is not the local PlayerController in this build. It is only read as a flag-bearing actor (`+0x1b0`, `+0x254`) by `PlayerTick` (`0x005e4350`), so it is empty in a normal game. Do not use `0x0054d8d0` as a PlayerController getter.

### 10.2 Reliable path to the PlayerController [V]

Scan the level's Actors array for the actor whose vtable is `0x019E2B2C` (`ASGWController_Player`, slot 0 = `0x00e83c10`). Live: `Actors` data 0xDCD77410, entry 36 = PC 0xE0FA6410 (the camera is the next entry, 37). Cost: one read of `count*4` bytes plus one 4-byte read per entry. Cache the PC and revalidate its vtable each tick.

### 10.3 Camera object [V]

`ASGWCamera_Player` (`SGWCamera_Player.cpp`, size 0x378, vtable `0x019E1134`) = `*(PC + 0x2b0)` (all `ASGWController_Player` input handlers do `FUN_00df5f20(*(this+0x2b0))`). Live: 0xDC68E410. Third-person camera fields:

| Offset | Type | Meaning | Live |
| --- | --- | --- | --- |
| +0x344 | f32 | max-distance scale | 0.75 |
| +0x348 | u32 | flags; bit 2 (0x4) inverts yaw, bit 3 (0x8) inverts pitch (multiplier -1.0f at `0x01814190`) | 3 |
| +0x34c, +0x350 | f32 | zoom step = 10.0 * 3.0 = 30 per notch | 10, 3 |
| +0x358 | f32 | look gain (mouse counts to rotator units) | 20.0 |
| +0x360 | f32 | camera distance (third-person zoom) | 250.0 |
| +0x368 | i32 | camera pitch offset. The pitch handler `0x00e7e590` does not clamp the stored value: repeated native pitch turns ran it to 1563 degrees and the view looked straight down (live, 2026-10-10). Whatever limits pitch to +-0x4000 acts later, on the view, so the lab clamps its own turns (+-0x3800). | 0 |
| +0x36c | i32 | camera yaw offset, wraps at +-0x8000 (65536 units per turn) | 0 |

Zoom limits: min 100.0 (`0x019e1000`), max `0.75 * 900.0 (0x018cb14c) + 100.0` = 775.0. The pawn's own Rotation is a different thing: `AActor+0xE8` (Pitch, Yaw, Roll as i32), location `+0xDC` (matches `memory.rs`). The camera yaw/pitch above are offsets relative to it.

### 10.4 Native zoom and look (preferred over key injection)

`ASGWController_Player` subscribes (`FUN_00e85c00`, `FUN_00e85540`) to `Event_Action_ZoomIn/Out` and `Event_Input_MouseMove`. The handlers are one-line thunks on the camera vtable (all `thiscall`, `ecx` = camera, callee pops nothing extra):

| Handler | Camera vtable slot | Effect | Signature |
| --- | --- | --- | --- |
| `0x00e83ee0` ZoomIn | +0x2f8 -> `0x00e7e560` | `dist -= 30` while `dist > 100` | `void(void)` |
| `0x00e83f10` ZoomOut | +0x2f4 -> `0x00e7e870` | `dist += 30` while `dist < 775` | `void(void)` |
| `0x00e83f40` MouseMove(dx,dy) | +0x300 -> `0x00e7e600` (yaw), +0x2fc -> `0x00e7e590` (pitch) | `yaw += gain*dx*inv`, `pitch += gain*dy*inv` | `void(float)` each |
| ExecYawAbsolute | +0x304 -> `0x00e7e680` | sets yaw from a float | `void(float)` |

So the lab can zoom by calling `0x00e7e560` / `0x00e7e870` with `ecx` = camera, or by writing `camera+0x360` (float, keep 100..775). It can turn by calling `0x00e7e600(ecx=camera, float dx)` and `0x00e7e590(ecx=camera, float dy)`, or by writing `+0x36c` / `+0x368`. The handler also accumulates raw counts into `PC+0x4a0` / `PC+0x4a4` (not needed to move the camera). Camera movement does not need RMB native-side: the RMB `MouseLook` gate only decides whether `Event_Input_MouseMove` reaches the handler (it is subscribed on `released==0`, unsubscribed on release).

### 10.5 `0x00581fb0` / `0x005820f0` and `InputAxis`

- `0x00581fb0(InputMgr, int delta)` (wheel up, key 300) and `0x005820f0(InputMgr, int delta)` (wheel down, key 0x12d): `thiscall`, `this` = the InputMgr singleton (`0x01ee2b1c`, from `0x005822c0`), one `int` stack arg (cast to float for CEGUI via `0x0056b320`), callee cleans (`ret 4`). Not zooming directly: they feed CEGUI first, then emit the key through CME. Native zoom is 10.4.
- Viewport client `InputAxis` (vtable `+0x18`): UE3 `UBOOL InputAxis(FViewport* vp, INT ControllerId, FName Key, FLOAT Delta, FLOAT DeltaTime, UBOOL bGamepad)`, `thiscall`, `FName` passed by value as two dwords (index, number). [U for the trailing bGamepad param; the pump passes 5 values per `9.2`.]
- FName values, read live from `0x01ee1fe8` (index, number): MouseX = {0x2a69, 0}, MouseY = {0x2a6a, 0}, ScrollUp = {0x2a67, 0} (`0x01ee1ff8`), ScrollDown = {0x2a66, 0} (`0x01ee2000`). The indices are per-run name-table slots; read them from those addresses instead of hardcoding.
- Caution: `V = *(SGWUIManager+0x38)` = 0xEE8B3204 is an interior (multiple-inheritance) pointer. `*(V+0x38)` live is the static address 0x01849C0C (an `FViewport` vtable-like), not a heap client object, so `viewport+0x38` as "client" in `9.2` is not confirmed for this pointer. Use the camera calls in 10.4 and avoid depending on `InputAxis`.

### 10.6 Open

- The real `GEngine` -> `GamePlayers[0]` -> `Actor` (PlayerController) path was not located; the Actors scan in 10.2 is the working recipe.
- The `+0x340` / `+0x354` / `+0x35c` / `+0x364` camera floats were read but their roles were not traced.