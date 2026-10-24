# Observation: windowedMode=true run (macOS, dual display)

Date: 2026-10-24, 12:20-12:32 CDT. Read-only observer: nothing was clicked, killed or edited.

Setup: SGW.exe under Wine, DXVK v1.10.3-async on MoltenVK, Cimmeria Launcher (operation 3).
`SavedSystemOptions.xml` had `windowedMode=true` (set at 12:17:43; `devWindowedMode=true` is also present).
`SGWEngine.ini` had StartupFullscreen=False at 1024x768.
Tooling: `/tmp/sgwobs/poll2.sh` (5 s poll), `winlist` (CGWindowList filtered to wine-owned windows), and `disp` (CGGetOnlineDisplayList plus the current mode).
The raw timeline is in `/tmp/sgwobs/timeline2.txt`.

## Result

- **Windowed, not captured.** For the whole run the SGW main window (id 37108) stayed at layer 0 with bounds 472,44 1024x800 (1024x768 client area plus a title bar).
  No window at shield level 2147483628 appeared in any poll.
- **No display mode change.** Two displays were online for the whole run, and neither changed mode:
  - display 2 (main): 3840x1080 ultrawide
  - display 1: 1710x1107 points (3420x2214 px) built-in
- **Cmd+Tab survived, twice.** The window went off-screen and came back twice with no log errors, and the process stayed alive.
- **Clean exit.** The launcher logged `game_exited exit_code=0 early=false duration_ms=713848`.
  No new file appeared in `CrashDumps/` (the only dump there is from the 12:10 run) or in `~/Library/Logs/DiagnosticReports/`.
- **windowedMode stayed `true`** throughout and after exit. The client did **not** rewrite `SavedSystemOptions.xml` on this exit: its mtime is still 12:17:43.
  The previous observer saw a rewrite on exit, so that rewrite may depend on the exit path or on the options UI.

## Timeline

| Time | Event |
|---|---|
| 12:19:56 | SGW.exe (pid 22549) starts. The launcher logs `game_started startup_ms=3097`. |
| 12:20:50 | First snapshot: window 37108 at layer 0, 472,44 1024x800, on-screen. Displays as listed above. The shield level reads 2147483628, but no window uses it. |
| 12:21:13 | The poll loop starts. CPU about 205%, RSS about 785 MB. `windowedMode=true`. |
| 12:21:14-12:21:33 | Interface.log: CEGUI imagesets and layouts load. Two benign `CEGUI::UnknownObjectException ... 'BackgroundEnabled '` lines. The patches log shows `registered CimmeriaBMNative ... lua_State #2` at +95 s, which is consistent with the UI loading for world entry. |
| 12:21:39 | RSS peaks at about 1.19 GB, then settles at 860-870 MB. CPU stays at 170-220%. |
| 12:27:20 | **Cmd+Tab away:** window 37108 becomes `onscreen=0`. Layer, bounds and display modes do not change. RSS drops to about 800 MB. |
| 12:27:45 | **Back:** window 37108 is `onscreen=1` again at the same layer and bounds. |
| 12:28:42 | Second switch away: `onscreen=0`. |
| 12:29:19 | Second return: `onscreen=1`. No log output at either transition. |
| 12:29-12:31:48 | Steady state: CPU about 190-240%, RSS about 812 MB. |
| 12:31:52 | The launcher logs `game_exited exit_code=0` and `game_session_finished count=7 bundle_sent=true`. |
| 12:31:54 | The poll confirms SGW.exe is gone. `windowedMode=true` (file mtime 12:17:43). |

## Logs checked

- `SGWGame/Logs/Interface.log`: last write 12:21:33. No fatal, "General protection fault", "Rendering thread", or device reset/lost lines.
- `SGWGame/Logs/Launch.log` and `Appearance.log`: 0 bytes.
- `binaries/cimmeria-client-telemetry.log`: only hook-install lines at startup (last write 12:19:58). No runtime events were written locally.
- `binaries/SGWDebugLog.log`: one line at 12:20:10, nothing after.
- `binaries/cimmeria-client-patches.log`: Black Market and strstream lines only.

## Comparison with the previous runs

The fullscreen runs changed the display mode to 1280x800, put the window at layer 2147483630 with an on-screen shield window at 2147483628, and Cmd+Tab ended in `Rendering thread exception: General protection fault`.
With `windowedMode=true` there was no capture, no mode switch and no shield window, and two Cmd+Tab round trips ran without errors.
That points to the exclusive-fullscreen capture and release path as the trigger for the GPF.

Caveat: this run had two displays (the ultrawide was main), while the crashing runs had a single display.
The display count was not isolated, so the evidence is consistent with the fullscreen path, not proof of it.
