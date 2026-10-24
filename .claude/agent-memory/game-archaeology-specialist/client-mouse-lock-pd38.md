---
name: client-mouse-lock-pd38
description: SGW.exe never clips the cursor in game (windowed or fullscreen); UE3 LockMouseToWindow is GIsEditor-gated; MouseLock strings are editor-only; globals GIsClient/GIsServer/GIsEditor addresses
metadata:
  type: project
---

PD-38 (2026-10-24), static RE with pefile and capstone on macOS (no Ghidra). Full write-up: `docs/analysis/launcher-preflight-diagnostics/research-mouselock.md`.

- **Verdict `none`.** The game has no cursor-confinement switch of any kind: ini, SavedSystemOptions, console command or launch argument.
- **The only rectangle `ClipCursor` is `FWindowsViewport::LockMouseToWindow` at `0x01233e30`** (vtable `0x01ac7b44`, slot `+0x1c`). Its callers in `CaptureMouse` (`0x01233c60`) are all behind `GIsEditor`.
  - In the game, `CaptureMouse` does `TrackMouseEvent(TME_LEAVE)` and then a DirectInput re-acquire with `SetCooperativeLevel(fg, 6)` (`NONEXCLUSIVE|FOREGROUND`).
  - It never calls `SetCapture` or clips.
- **Engine mode globals**, mapped from the `NOCLIENT`/`NOSERVER`/`NOEDITOR` commandlet at `0x00b8ee8b` and the PreInit path at `0x00418ae5`:
  - `GIsClient` = `0x01ead7bc`
  - `GIsServer` = `0x01ead7c0`
  - `GIsEditor` = `0x01ead7ac`
  - This matches `docs/reverse-engineering/address-map.md`. Its `GIsGame` is `0x01eb0830`. Don't mistake the heavily referenced `0x01ead7ac` for `GIsGame`; that wrong guess cost time in PD-38.
- The W`MouseLock` and `ToolTip_MouseLock` strings belong to the editor's `[FEditorModeTools] MouseLock` in `GEditorIni` (`0x01eae808`), which locks actor manipulation, and to a wx toolbar button. They are not about the cursor.
- **Other addresses:**
  - `UWorld::IsPaused` `0x005e2260`
  - `GWorld` `0x01ee2684`
  - `UGameViewportClient::ReceivedFocus` `0x005d24e0`; `LostFocus` `0x005d24c0`
  - `bDisplayingUIMouseCursor` = GVC`+0xcc` bit 0, written by `UGameUISceneClient` functions `0x00677180` and `0x00677230` (write sites `0x006771d0` and `0x00677250`)
  - The viewport `HWND` chain is `GEngine` `0x01ee1254` → `+0x2dc` → `+0x44` → `+0x88`. Medium confidence; verify on the live client.
- **Method gotcha:** the DLL is `crates/client-patches`, not `tools/client-patches` (that folder holds data generators). For PD-39, polling from the existing `FEngineLoop::Tick` detour is the cheapest hook.

**Why:** PD-39 (the client-patch cursor lock) was pre-approved only if this verdict is `none`.

**How to apply:** don't re-investigate an in-game mouse-lock option. Use the globals above when reading UE3 engine gates. See also [[client-display-knobs]] and [[controller-mouse-input-model]].
