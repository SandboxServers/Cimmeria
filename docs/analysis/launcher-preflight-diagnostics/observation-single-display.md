# Observation: SGW launch on a single display (built-in Retina only)

Date: 2026-10-24, 12:07:00 to 12:08:18 CDT. Read-only observation; nothing was killed, clicked or edited.
Host: macOS (Darwin 27.0.0), Apple M5. The ultrawide (3840x1080) was unplugged; only the built-in display was connected.
Question: did the dual-display setup cause last run's invisible window (`onscreen=0` at layer 2147483630, inside the `CGShieldingWindowLevel` 2147483628 band)?

## Method

- Processes: `ps -axo pid,etime,%cpu,rss,command | grep -i -E 'SGW.exe|wineserver|cimmeria-launch-worker'`.
- Windows: a compiled Swift helper calling `CGWindowListCopyWindowInfo(.optionAll)`, filtered to owners named `wine`/SGW. (`python3` has no `Quartz` module on this Mac.)
- Displays: `system_profiler SPDisplaysDataType` at start and on each layer change; at the end, `CGGetOnlineDisplayList` + `CGDisplayCopyDisplayMode` + `CGShieldingWindowLevel()`.
- Logs: byte size and new lines of `Interface.log` in the game prefix (`.../bottle/drive_c/users/derek/Documents/My Games/Firesky/SGWGame/Logs/Interface.log`) and `logs/launcher.2026-10-24.log`.
- Crash reports: diff of `~/Library/Logs/DiagnosticReports/` each poll.
- Loop: 5 s polls, at most 72 polls (6 min). Stop when SGW.exe exits, or when a non-500x500 SGW window has reported `onscreen=1` for 30 s.

The game was already running (etime 00:15) at the first check, so there was no wait phase.

## Display at start

`system_profiler`: one display, "Color LCD", Built-in Liquid Retina, 2880 x 1864 Retina, Main Display: Yes. No external display.

## Timeline

| Time | SGW etime | SGW %CPU | RSS (KB) | Main window id 36971 | Other SGW windows | Interface.log |
|---|---|---|---|---|---|---|
| 12:07:00 (pre-check) | 00:15 | 157.4 | 628316 | layer 21, 0,34 1280x800, onscreen=1 | 36970/36968 500x500, layer 0, onscreen=0 | |
| 12:07:33 poll 1 | 00:37 | 145.9 | 645572 | layer 21, onscreen=1 | same | 117440 B; last lines: `EULA.layout` loaded 12:07:07, then `BankGothic_10pt` imageset 12:07:30 |
| 12:07:38 poll 2 | 00:42 | 143.9 | 680352 | **layer 2147483630**, 0,34 1280x800, **onscreen=1** | 36977 layer 2147483629 0,0 1280x800 onscreen=0; **36976 layer 2147483628 (shield) 0,0 1280x800 onscreen=1** | unchanged |
| 12:07:43 poll 3 | 00:47 | 148.4 | 693904 | 2147483630, onscreen=1 | same | unchanged |
| 12:07:48 poll 4 | 00:52 | 137.8 | 668424 | 2147483630, onscreen=1 | same | unchanged |
| 12:07:54 poll 5 | 00:58 | 110.9 | 672452 | 2147483630, onscreen=1 | same | +115 B: `12:07:50 Attempting to create Imageset 'BankGothic_12pt_auto_glyph_images_'` |
| 12:07:59 poll 6 | 01:03 | 169.9 | 715132 | 2147483630, onscreen=1 | same | unchanged (117555) |
| 12:08:04 poll 7 | 01:08 | 210.3 | 719960 | 2147483630, onscreen=1 | same | unchanged; **STOP: on-screen >= 30 s** |
| 12:08:18 (post-check) | 01:22 | 207.3 | 723500 | 2147483630, onscreen=1 | adds 4 windows 1280x30 layer 0 onscreen=0 | unchanged |

wineserver (pid 17178) stayed at about 2 to 4% CPU; cimmeria-launch-worker.exe (pid 17223) at 0.0%.

At the layer change (poll 2) `system_profiler` still reported 2880 x 1864 Retina. The post-check display query showed the mode had switched:

```text
display 1 main=1 bounds=(0.0, 0.0, 1280.0, 800.0) mode=1280x800 px=2560x1600
shieldLevel=2147483628
```

So the game captured the built-in display and set a 1280x800 mode (2560x1600 backing). Its window sits at shield+2, above the shield window (id 36976, exactly at `CGShieldingWindowLevel`).

- Launcher log: no new lines during the observation (stayed at 19981 bytes).
- Crash reports: none new. The newest file is `Helium Helper-2026-10-24-105202.ips`, which is unrelated.
- SGW.exe did not exit.

## Findings

1. **The window is on-screen this time.** It still goes into the captured-display band (layer 2147483630, the same layer as last run), but `onscreen=1` held at that layer for 26 s, and for 45 s counting the earlier layer-21 period. The shield window is on-screen too, and the display mode switched to 1280x800. That is a normal fullscreen-exclusive capture. Last run had `onscreen=0` at the same layer.
2. **Interface.log is past `EULA.layout`.** `EULA.layout` finished loading at 12:07:07, the last GUI layout loaded. After it came only font imageset creation (`BankGothic_10pt` at 12:07:30, `BankGothic_12pt` at 12:07:50), then silence. No new layout loaded and no errors in the window.
3. **CPU** ran about 110 to 210% (more than one core) and was rising at the end (146, 144, 148, 138, 111, 170, 210, 207). RSS grew from 628 MB to 724 MB. That fits a render loop that is running, not a hang.
4. **Caveat:** CGWindowList `onscreen=1` means the window server is compositing it. It does not prove the pixels show game content. Only the user can confirm whether the EULA or login screen is actually visible.

## Verdict on the dual-display theory

Supported, not proven. The single variable changed was unplugging the ultrawide. The fullscreen capture then resolved to the built-in display, and the window came on-screen at the same shield-band layer where it was `onscreen=0` before. That fits the theory: with two displays, the capture or shield landed on one display while the window was placed on, or bounded to, the other. To confirm, the user should say whether the screen visibly showed the game. A controlled rerun with the ultrawide plugged back in that reproduces `onscreen=0` would close it. Whether the window rendered on the 3840x1080 display during that run is not recorded here.
