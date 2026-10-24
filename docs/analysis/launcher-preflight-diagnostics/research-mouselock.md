# Research: does the game lock the mouse to its window? (PD-38)

Date: 2026-10-24. Packet: [PD-38](work-packets-mac-extras.md#pd-38-what-the-games-mouse-lock-does-re-d4-4-wave-m-ready-now). Context: [research-mac-extras.md § 4](research-mac-extras.md#4-cursor-lock), owner decisions D4 and D5 in [OWNER-DECISIONS.md](OWNER-DECISIONS.md).

## Verdict: `none`

The shipped game never confines the cursor, in windowed mode or in fullscreen. No ini key, no `SavedSystemOptions.xml` key, no console command and no launch argument turns confinement on. The client does contain UE3's own lock (`FWindowsViewport::LockMouseToWindow`), but its only callers are gated on `GIsEditor`, and `GIsEditor` is 0 in the game. The strings `MouseLock` and `ToolTip_MouseLock` belong to the Unreal editor, and the setting they name has nothing to do with the cursor.

The launcher therefore has nothing to write before Play. A cursor lock needs the PD-39 hook, which D5 pre-approved for this case. A sketch is in [§ 6](#6-hook-site-for-pd-39).

Confidence: high for the static facts below; every one is cited to an address. One input to the verdict is an inference: that `0x01ead7ac` is `GIsEditor` ([§ 2](#2-the-flag-that-gates-the-lock-is-giseditor)). That evidence is strong, but UAT-PD12 should still confirm on a Mac that the cursor leaves the window in windowed play.

## Method

- Binary: `SGW.exe` from a local game folder, 31,228,448 bytes, image base `0x00400000`, i386. This host has no Ghidra, so the analysis used `pefile` plus `capstone` (linear sweep, IAT cross-references, RTTI vtable recovery and jump-table decoding). All addresses are virtual addresses at the preferred base.
- Win32 and DirectInput constants were checked against the headers: `TME_LEAVE = 0x2` per [Microsoft Learn, TRACKMOUSEEVENT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-trackmouseevent). `DISCL_EXCLUSIVE = 1`, `DISCL_NONEXCLUSIVE = 2` and `DISCL_FOREGROUND = 4` per Wine's [`include/dinput.h`](https://gitlab.winehq.org/kirill.zhumarin/wine/-/blob/wine-1.1.33/include/dinput.h).

## 1. Every call to ClipCursor, SetCapture and ReleaseCapture

IAT slots: `ClipCursor` `0x017efe64`, `GetClipCursor` `0x017efe20`, `SetCapture` `0x017efd78`, `ReleaseCapture` `0x017efe68`, `ShowCursor` `0x017efe6c`, `SetCursorPos` `0x017efda0`, `TrackMouseEvent` `0x017efda4`.

| Call site | Function | What it does | Gate |
|---|---|---|---|
| `ClipCursor` `0x004ccbaf`, `ReleaseCapture` `0x004ccba8` | crash and `appError` path | `ShowCursor(TRUE)` until the count is at least 0, `ReleaseCapture`, `ClipCursor(NULL)` | runs on a fatal error; it only releases |
| `ClipCursor` `0x012313ae`, `SetCapture` `0x012313a6` | viewport cleanup `0x012313a0` | `SetCapture(NULL)`, `ClipCursor(NULL)`, then the `ShowCursor(TRUE)` loop | teardown; it only releases |
| `ClipCursor` `0x01231b5e`, `SetCapture` `0x01231b57` | viewport destroy path | `SetCapture(NULL)`, `ClipCursor(NULL)` | teardown; it only releases |
| `ClipCursor` `0x01233e80`, `0x01233ea7`, `0x01233f4b` | `FWindowsViewport::LockMouseToWindow(UBOOL)` `0x01233e30` | Lock: `GetClipCursor(this+0x4bc)` saves the old clip, then `GetClientRect(hwnd)`, `MapWindowPoints`, `ClipCursor(&client)`, `this+0x4d4 = 1`. Unlock: `ClipCursor(&saved)`, `0x4d4 = 0`. Otherwise it re-syncs the rect if the window moved | **the only rectangle clip in the binary.** Called only from `CaptureMouse`, behind `GIsEditor` ([§ 2](#2-the-flag-that-gates-the-lock-is-giseditor)) |
| `SetCapture` `0x01233cfe`, `ReleaseCapture` `0x01233e06` | `FWindowsViewport::CaptureMouse(UBOOL)` `0x01233c60` | editor branch: hide the cursor, save its position, `SetCapture(hwnd)`, `LockMouseToWindow(TRUE)`; on release, `LockMouseToWindow(FALSE)` (unless fullscreen and focused), `SetCursorPos(saved)`, `ReleaseCapture`, show the cursor | `GIsEditor` at `0x01233cd6` and `0x01233dc9` |

`LockMouseToWindow` is vtable slot `+0x1c` of the `FWindowsViewport` vtable at `0x01ac7b44` (Complete Object Locator at `0x01ac7b40`), and nothing references it apart from that vtable entry, `0x01ac7b60`. The virtual-call sweep (for viewport-shaped objects only) found no caller outside `CaptureMouse`. An editor-only caller elsewhere would not change the verdict. The three calls inside `CaptureMouse` are: `0x01233cb5` (re-sync, gated on `GIsEditor` at `0x01233c9e`), `0x01233d0d` (lock, editor branch) and `0x01233df0` (unlock, editor branch).

### CaptureMouse in the game

`CaptureMouse(bCapture)` at `0x01233c60` runs as follows:

1. It picks `GetForegroundWindow()` if the viewport is fullscreen (vtable `+0x40`), and otherwise `GetFocus()`. It wants a capture only if `bCapture` is set and that window is the viewport's `hwnd` (`this+0x88`).
2. `0x01233c9e`: `if (GIsEditor) LockMouseToWindow(this+0x4d4)`. The game skips this.
3. Capture (`this+0x4cc` goes from 0 to 1). `0x01233cd6`: `if (GIsEditor) { hide cursor; GetCursorPos; SetCapture; LockMouseToWindow(TRUE) }`. Otherwise, at `0x01233d11`, it calls `TrackMouseEvent({cbSize=0x10, dwFlags=TME_LEAVE, hwnd})`. The game takes this branch, which asks for a `WM_MOUSELEAVE` notification and does not confine anything.
4. In both branches, the DirectInput mouse device (`0x01f053d4`) then does `Unacquire`, `SetCooperativeLevel(GetForegroundWindow(), 6)`, `Acquire`, and drains buffered data with `GetDeviceData`. `6` is `DISCL_NONEXCLUSIVE | DISCL_FOREGROUND`. A non-exclusive device neither hides nor confines the system cursor.
5. Release (`this+0x4cc` goes from 1 to 0). `0x01233dc9`: `if (GIsEditor) { ... ReleaseCapture ... }`. The game only clears the flag.

So in the game, a mouse "capture" means a `WM_MOUSELEAVE` request plus a non-exclusive DirectInput re-acquire.

### Who calls CaptureMouse in the game

These callers matter only to show that nothing else reaches the lock:

- `USGWViewportClient` / `UGameViewportClient::ReceivedFocus` `0x005d24e0`. It calls `Viewport->CaptureMouse(!(GWorld->IsPaused() || bDisplayingUIMouseCursor))`. `0x005e2260` is `UWorld::IsPaused` (it compares the `WorldInfo` pauser against `PauseDelay`). `bDisplayingUIMouseCursor` is `UGameViewportClient+0xcc` bit 0, and `UGameUISceneClient` writes it at `0x006771d0` and `0x00677250`.
- `LostFocus` `0x005d24c0` calls `CaptureMouse(FALSE)`.
- `0x005d2390` is the same rule on the `UObject` side. It is called from the engine at `0x008f79e0`, which `UGameEngine` reaches when a map finishes loading, via `GEngine+0x2dc` (the game viewport).
- Viewport `WndProc`:
  - `WM_SIZE` `0x01235ba5` captures again, or on `SIZE_MINIMIZED` calls `CaptureMouse(0)`.
  - `WM_DISPLAYCHANGE` `0x01235a4e` captures again.
  - `WM_ENTERSIZEMOVE` `0x01236765` calls `CaptureMouse(0)`.
  - Alt+Enter `0x01235d89` toggles fullscreen, then calls `CaptureMouse(TRUE)`.

All of these end in the `TrackMouseEvent` branch in the game.

## 2. The flag that gates the lock is GIsEditor

`0x01ead7ac` is a zero-initialised global (`.bss`). The binary writes it in exactly three places:

- `0x00418afc` writes 0. This is in `FEngineLoop::PreInit`'s normal start-up path, which writes `0x01ead7bc = 1`, `0x01ead7c0 = 0`, `0x01ead7ac = 0` and `0x01ead7b0 = 0`, then parses `DUMPMOVIE`, `BENCHMARK`, `NOINI` and `ONETHREAD` (`0x00418ae5` to `0x00418c06`).
- `0x00b8ef8b` and `0x00b8f0e4` are inside a shader-cache commandlet. It parses `NOCLIENT`, `NOSERVER` and `NOEDITOR` (`0x00b8ee8b`, `0x00b8eec8`, `0x00b8ef09`), then stores the three results, in that order, to `0x01ead7bc`, `0x01ead7c0` and `0x01ead7ac` (`0x00b8ef7f` to `0x00b8ef8b`). Later it sets all three to 1 (`0x00b8f0d8` to `0x00b8f0e4`).

The `NOCLIENT`/`NOSERVER`/`NOEDITOR` order maps the three globals to `GIsClient` (`0x01ead7bc`), `GIsServer` (`0x01ead7c0`) and `GIsEditor` (`0x01ead7ac`). The normal start-up path makes the client a client (`GIsClient = 1`) and not an editor (`GIsEditor = 0`), and no instruction moves an immediate, or loads the address, of `0x01ead7ac` anywhere else. In the game, then, `GIsEditor` stays 0, and `CaptureMouse` never reaches `SetCapture` or `LockMouseToWindow`.

This mapping agrees with the repo's existing RE: [address-map.md](../../reverse-engineering/address-map.md) lists `0x01ead7ac` as `GIsEditor`, `0x01ead7bc` as `GIsClient`, `0x01ead7c0` as `GIsServer` and `0x01eb0830` as `GIsGame`. [findings/atrea-editor.md](../../reverse-engineering/findings/atrea-editor.md) records that the Atrea editor patch sets `GIsEditor=1` at this address to turn SGW.exe into UnrealEd. Only in that editor mode would the lock run.

The behaviour matches the design: the editor captures the cursor during a viewport drag, and the game relies on DirectInput.

## 3. What `MouseLock` and `ToolTip_MouseLock` are

They come from the Unreal editor and do not lock the cursor:

| String | Address | Used at | Meaning |
|---|---|---|---|
| W`MouseLock` | `0x01991104`, `0x019911b4` | `FEditorModeTools::LoadConfig` `0x00be5bb0` and `SaveConfig` `0x00be5c30`, with section W`FEditorModeTools` and the ini global `0x01eae808` (`GEditorIni`) | the editor's "lock actor manipulation" flag |
| W`MouseLock` | `0x01a97330` | `0x01137c66` | a wxWidgets editor toolbar bitmap name |
| A`ToolTip_MouseLock` | `0x01a97864` | `0x01138fe4`, localised from the `UnrealEd` package | tooltip of that toolbar button |

Where the value lives:

- `Engine/Config/BaseEditor.ini:533`, `[FEditorModeTools] MouseLock=False`.
- `Engine/Localization/INT/UnrealEd.int:2220` reads `ToolTip_MouseLock=Prevent the mouse from being able to move/rotate/scale actors.  Only selection will be allowed.`

The game never reads `GEditorIni`, and the setting is not cursor confinement.

A search of every ASCII and UTF-16 string in the binary for `mouselock`, `lockmouse`, `capturemouse`, `mousecapture`, `clipcursor`, `confine`, `cursorlock` and `lockcursor` finds only:

- the strings above
- the import names `ClipCursor` and `GetClipCursor`
- the wxWidgets symbols `wxWindow::DoCaptureMouse` and `wxWindowBase::CaptureMouse`, which are editor UI

None of them is a console command, launch argument or options key.

The player options have no such key either. The live `SavedSystemOptions.xml` camera group holds only `mouseLookSensitivity`, `camZoomSpeed`, `maxZoomDistance`, `invertMouseY` and `invertMouseX`. The other groups are `audio`, `video` (which holds `windowedMode`), `ui` and `internal` (which holds `devWindowedMode`). The stock definitions in `SGWGame/Content/XML/SystemOptions.xml` define no mouse-lock or cursor option either; the only mouse options are the camera ones above.

## 4. SGW's mouse-look does not clip either

SGW's own mouse-look is the right-drag camera, `Event_Action_MouseLook`. Its handler is `SGWUIManager` `0x0056c620`, subscribed at `0x0056c87d` and `0x0056d869`. It works as follows:

- It keeps a reference count at `UIMgr+0x4a` and an active flag at `+0x49`.
- It calls `Viewport->ShowCursor(refcount == 0)` (vtable `+0x38`), which hides the cursor during the drag.
- On start it saves the position with `GetMousePos` (`+0x30`). On release it puts the cursor back with `SetMouse(x, y)` (`+0x34`, `ClientToScreen` plus `SetCursorPos`) and moves CEGUI's cursor with `injectMousePosition` (`0x011aeaa0`).
- It never calls `CaptureMouse`, `ClipCursor` or `SetCapture`.

While the cursor is hidden during a drag, the system cursor can therefore leave a windowed game, even though the release puts it back where the drag started. This is the likely source of the owner's complaint.

## 5. The answer to the packet's question

The game does not confine the cursor in windowed mode, either on its own or behind a switch the launcher could write. Fullscreen gets no `ClipCursor` either. Exclusive fullscreen keeps the cursor on the game's display only because the display belongs to the game, not because of a clip.

Verdict: **`none`**.

## 6. Hook site for PD-39

This is a sketch for the PD-39 re-spec, not a design. The DLL is the crate `crates/client-patches` (built as `cimmeria_client_patches.dll`). The PD-39 text says `tools/client-patches/`, but that folder holds only data-patch generators.

**Recommended: poll from the existing `FEngineLoop::Tick` detour.** The DLL already hooks `FENGINE_LOOP_TICK = 0x00416ec0` (`crates/client-patches/src/addresses.rs`). This needs no new fingerprinted site and no `WndProc` subclass. It runs on the game thread, which owns the window. Once per tick:

1. Find the viewport `HWND`: `GEngine` (`0x01ee1254`), then `+0x2dc` (`GameViewport`), then `+0x44` (`FWindowsViewport*`), then `+0x88` (`HWND`). Null-check every hop. The `+0x2dc` and `+0x44` offsets come from `0x008f79d6` and `0x005d2393`; confirm them on the live client in PD-39 before relying on them.
2. Clip only if all of these hold:
   - the setting is on
   - `GetForegroundWindow() == hwnd`
   - `!IsIconic(hwnd)`
   - the viewport is windowed, read from `FWindowsViewport` vtable `+0x40` (`IsFullscreen`) or the window style
   - the window is not in a size or move loop (track `WM_ENTERSIZEMOVE` and `WM_EXITSIZEMOVE`, or skip while `GetCapture()` is a non-client drag)
3. To clip: `GetClientRect`, then `MapWindowPoints(hwnd, NULL, ...)`, then `ClipCursor(&rect)`. Re-apply only when the rect or the state changes, and keep a `we_clipped` flag.
4. To release: when any condition fails and `we_clipped` is set, call `ClipCursor(NULL)` once and clear the flag. This covers deactivation, Cmd+Tab and minimise. Also release from the DLL's detach or exit path. The engine's own teardown (`0x012313ae`) and crash path (`0x004ccbaf`) call `ClipCursor(NULL)` anyway.

**Alternative: subclass the `WndProc`** with `SetWindowLongW(hwnd, GWL_WNDPROC, ...)`. The engine registers its class at `0x01232b88` (`RegisterClassExW`) and creates the window at `0x012343b6` (`CreateWindowExW`).

- Clip on `WM_ACTIVATE` active, `WM_EXITSIZEMOVE`, `WM_SIZE` (not minimised) and `WM_DISPLAYCHANGE`.
- Release on `WM_ACTIVATE` inactive, `WM_KILLFOCUS`, `SIZE_MINIMIZED`, `WM_ENTERSIZEMOVE` and `WM_DESTROY`.
- This reacts faster than polling, but it needs the `HWND` before the first activation, and it must chain to whatever `WndProc` is installed, including the engine's `SetWindowLongW` at `0x01234531`.

**Do not hook `LockMouseToWindow` (`0x01233e30`) or `CaptureMouse` (`0x01233c60`).**

- Forcing the editor branch would also hide the cursor and `SetCapture` it. That breaks CEGUI's visible cursor and moves the cursor back to the saved position on every release.
- `LockMouseToWindow` has no game callers to piggyback on.

**Behaviour notes:**

- On Windows, `ClipCursor` applies to the whole desktop, so the release path is mandatory.
- Under winemac.drv the clip applies only to the calling process, and the driver releases it when the game stops being frontmost ([research-mac-extras.md § 4](research-mac-extras.md#4-cursor-lock)).
- The clip does not stop mouse-look working: DirectInput is non-exclusive and reads relative motion.
- Wine and Windows both re-validate the clip when the display changes, so re-applying it on a rect change is enough.

## Open items

- UAT-PD12 should confirm, on a Mac, that the cursor leaves the windowed game today. That is the observable proof of `none`.
- PD-39 should check the `GEngine+0x2dc` and `GameViewport+0x44` offsets on the live client, for example by logging the derived `HWND` against `GetForegroundWindow()` once in the world.
