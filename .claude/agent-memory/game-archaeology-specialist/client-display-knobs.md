---
name: client-display-knobs
description: SGW.exe display mode knobs. The ini [WinDrv.WindowsClient], the WINDOWED/ResX=/ResY= switches, and SavedSystemOptions windowedMode/resolution via the shared handler 0xd3faa0 (which wins). No borderless, no vsync.
metadata:
  type: project
---

The SGW client has two display layers. **SavedSystemOptions wins once the game is running.** Static recovery, 2026-10-24; full report in `docs/analysis/launcher-preflight-diagnostics/research-display-client.md`.

- **Startup frame**
  - Function: `UGameEngine::Init` at `0x008f5d3a`.
  - Inputs: the ini `[WinDrv.WindowsClient] StartupFullscreen/StartupResolutionX/Y` (UClient `+0x50/+0x54/+0x58`) and the command-line `WINDOWED`, `ResX=`, `ResY=` (UTF-16 strings at `0x018f12ec..0x018f1304`).
  - There is no `FULLSCREEN` switch and no `[SystemSettings]` section.
  - `CreateViewportFrame` reads `+0x5c/+0x60/+0x64`; the writer of those fields is untraced.
- **Options layer**
  - `Event_Option_WindowedMode`, `Event_Option_Resolution` and `Event_Option_DevWindowedMode` all thunk into `0x00d3faa0`.
  - The handler calls `FViewport::Resize`. Windowed size = ini Startup X/Y. Fullscreen size = the D3D mode list entry at the `resolution` index (enumerated by `0x00ec3f20`).
- **Persistence**
  - The client rewrites `SavedSystemOptions.xml` on exit and on crash.
  - It never rewrites `SGWEngine.ini` on exit.
  - Stock `windowedMode` defaults to false, so a fresh profile goes fullscreen even when the ini says False.

**Why:** the D2 black-screen and display-takeover diagnosis looked only at the ini. The real cause was the saved `windowedMode=false`.

**How to apply:** any launcher or lab display control must write `SavedSystemOptions.xml` `windowedMode` before every launch; the ini and args are secondary. See [[multi-client-lab-findings]].
