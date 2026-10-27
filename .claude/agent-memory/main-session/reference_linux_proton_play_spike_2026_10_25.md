---
name: reference_linux_proton_play_spike_2026_10_25
description: PP-S1 on the Steam Deck - SGW under Proton needs native d3dx9_40 and the windowed client profile; bare runinprefix stdio works, container mode does not
metadata:
  type: reference
---

# Linux Proton Play spike (PP-S1) — 2026-10-25

Source: [worknotes/PP-S1.md](../../../docs/analysis/launcher-linux-play-proton/worknotes/PP-S1.md). Steam Deck, Desktop Mode, GE-Proton11-11. Owner at the screen.

- **Wine's builtin `d3dx9_40` crashes SGW.exe** after the login map starts: `ACCESS_VIOLATION` at `SGW.exe!0x00EC6F9D`, error history `ShaderRef != NULL && *ShaderRef != NULL` (`ShaderManager.h(759)`). Same on Proton Experimental.
  - Fix: the native `d3dx9_40.dll` from the game's `.cimmeria-prerequisites/DX9.0c/Nov2008_d3dx9_40_x86.cab`, with `WINEDLLOVERRIDES="d3dx9_40,xinput1_3=n"`. Native `xinput1_3` (`APR2007_xinput_x86.cab`) went in too; whether it is needed is unsplit.
  - `DXSETUP.exe /silent` exits 247 under Proton and installs nothing. Wine's `expand.exe` cannot read the cab either; host `7z x` can.
  - Compare [the Windows ARM report](reference_windows_game_prerequisites_2026_10_23.md), where DirectX was also the fix.
- **A black window that needs a taskbar touch** comes from the game's own first-run profile: `Documents/My Games/Firesky/SGWGame/SavedSystemOptions.xml` `windowedMode=false`. The launcher's existing display-settings write (`launch-display.md` § Display settings before each Play) fixes it.
- **Stdio:** bare `proton runinprefix <worker>` passes stdin/stdout. The Steam Linux Runtime `_v2-entry-point --verb=waitforexitandrun` gives exit 1 and no stdout (SLR_4 and sniper).
- **PhysX:** without it, the game shows "Failed to initialize physics sub system". The prerequisite worker installs the MSI but returns `failed/probe`.
- **Crash dumps** go to the prefix's `Documents/My Games/Firesky/SGWGame/CrashDumps/`, not the game folder.
- **Login and world entry** on `play.cimmeria.app:8081` passed. World entry took about 2-3 minutes (`No player` from `Minimap.lua` meanwhile); no Windows baseline was taken.
