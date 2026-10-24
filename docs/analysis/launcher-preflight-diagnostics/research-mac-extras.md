# Research: Mac display extras under winemac.drv (owner D4)

Date: 2026-10-24. Scope: whether the four D4 extras ([OWNER-DECISIONS.md § D4](OWNER-DECISIONS.md)) can be built under winemac.drv and the pinned DXVK v1.10.3-async, and how. It focuses on extras (2) (Retina-aware sizes and the monitor picker) and (4) (cursor lock and logs), which the owner asked to have checked. The packets are in [work-packets-mac-extras.md](work-packets-mac-extras.md).

Sources: Wine source at `master` (wine-mirror on GitHub), WineHQ Bugzilla, the wine(1) man page, Microsoft Learn, and the client binary and data in a local game folder. "Pinned Wine" means the WoWSilicon runtime r17 (Wine 11.13) that the launcher downloads.

## Verdicts

| Extra | Verdict | Confidence | Packet |
|---|---|---|---|
| (1) Borderless | Feasible in principle; needs UAT-PD8 (unchanged from research-display-wine.md § 1) | medium-high | PD-27 |
| (1) Safe-mode relaunch | Feasible; builds on the PD-29/PD-30 hidden-window detection | high | PD-36 |
| (2) Retina-aware size list | Feasible host-side (sizes in points from CoreGraphics) | high | PD-31, PD-34 |
| (2) `RetinaMode` (HiDPI rendering) | Technically feasible, but prefix-wide, a fixed 2x, and harmful on mixed-DPI setups. **Flagged: experimental, opt-in, after UAT-PD10 and an owner call (D-PD23)** | medium | PD-40 |
| (2) Monitor picker, remembered per display setup | Feasible through the launch helper (a Win32 `SetWindowPos` inside the same wineserver). Needs a Mac probe (UAT-PD11) | medium | PD-32, PD-33, PD-34 |
| (3) Frame cap 30 / 60 / display rate | Feasible on macOS (`DXVK_FRAME_RATE`). **Not available on Windows** without a client patch: native d3d9 has no limiter knob and the client has no vsync option | high (Mac) | PD-35 |
| (4) Cursor lock | **Not feasible from the launcher or the helper.** Feasible only if the game itself calls `ClipCursor` in windowed mode, or through a hook in the already-injected client patch DLL (a client patch, so a maintainer decision, D-PD22). An RE packet runs first | medium | PD-38, PD-39 |
| (4) Per-launch logs and "Open logs" | Feasible; uses existing pieces (`tauri-plugin-opener`, PD-28's folder) | high | PD-28, PD-37 |

## 1. Retina: sizes in points, and `RetinaMode`

**How winemac.drv reports sizes.** With `RetinaMode` off (the default), Windows programs see the Mac's logical size in points. A 1710x1107-point built-in panel (3420x2214 pixels) is a 1710x1107 desktop to the game. That matches observation-windowed.md, where the 1024x768 client area drew at 1024x768 points.

**Size list (feasible, high confidence).** The launcher can list the displays itself through CoreGraphics (`CGGetActiveDisplayList`, `CGDisplayBounds` for points, `CGDisplayModeGetPixelWidth` for pixels, `CGDisplayModeGetRefreshRate`). It then offers only the window sizes that fit the chosen display's bounds in points, less the menu bar and the title bar. No Wine setting is needed. "Match display" (Borderless) is the display's own size in points. A window larger than the display is what makes macOS clip or move it, so the filter is also a safety rule.

**`RetinaMode` (technically feasible; flagged).**

- `setup_options()` in `macdrv_main.c` reads `RetinaMode` from the global `HKCU\Software\Wine\Mac Driver` key only, never the per-app key. The source comment is "Don't use appkey. The DPI and monitor sizes should be consistent for all processes in the prefix." ([macdrv_main.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/macdrv_main.c)). So the toggle is prefix-wide.
- With it on, `display.c` doubles the width and height it reports for the original display mode, so the game sees pixels, not points ([display.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/display.c)).
- `cocoa_app.m` scales mouse motion by a fixed `retina_on ? 2 : 1`, not per display ([cocoa_app.m](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_app.m)).
- Bug [53081](https://bugs.winehq.org/show_bug.cgi?id=53081) ("Retina Mode broken since 7f7f9fa…") was reported against Wine 7.9 and fixed in 7.21 by commit `223ac6704435a09423030f4865b1a92a87e6ea49`. The pinned Wine 11.13 is later.
- Bug [42973](https://bugs.winehq.org/show_bug.cgi?id=42973) is still UNCONFIRMED. In it, Ken Thomases (the winemac.drv author) says: "Retina mode is keyed off of the original resolution as it was at program launch".
- A WineHQ search summary also advises raising the DPI (`LogPixels` 192) at the same time, and warns that many programs handle that badly. CrossOver's "High Resolution Mode" sets the same key. This came from a search summary, not a page I opened.

What that means for this game:

- A 1024x768 window becomes 512x384 points on a Retina panel unless the game picks a larger size. So the size list has to switch to pixels when the mode is on.
- GPU load quadruples, on MoltenVK.
- On a mixed setup (a 1x ultrawide plus a Retina panel, like the owner's), the fixed 2x is wrong on the 1x display.
- It changes every process in the shared game prefix, so the launcher has to own the value (write `y` or `n` before every Play) the same way it owns the display keys.

Recommendation: keep it off by default. Offer it only as an opt-in "Sharper text on Retina displays (experimental)" switch, shown only when the chosen display's backing scale is 2. Build it only after UAT-PD10 shows the client renders correctly with it, and the owner agrees (D-PD23). This is flagged, not dropped.

## 2. Monitor picker

- winemac.drv sizes display modes from `CGMainDisplayID()` and carries a `/* TODO: support displays other than the main one */` comment ([display.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/display.c)). Mode changes only ever happen on the main display; D-PD15 forbids mode changes on a Mac anyway.
- Every display is exposed to Win32 as a monitor in virtual-screen coordinates, so `EnumDisplayMonitors` and `GetMonitorInfoW` work inside the prefix. A window moved there with `SetWindowPos` is moved by Cocoa onto that display.
- The game decides its own window position, and there is no ini key or argument for it (research-display-client.md lists none). So the window has to be moved after it appears.
- The **launch helper** is the right place to do that. It is a Win32 process in the same prefix and wineserver, and it stays alive until the game exits: it creates the game suspended, injects the DLLs, resumes it, then `running.wait()` (`RP/game_launch/windows.rs`). After `ProcessStarted` it can find the game's top-level window by process id and centre it on the chosen monitor's work area. The same code works on native Windows, so one implementation serves both platforms.
- To remember the choice per display setup, key it by a fingerprint of the monitor set: the count plus each monitor's size and position, sorted. No monitor names are used, so the key is not personal data. A setup that hasn't been seen before uses the main display.
- Risk: the game may move or resize its own window after it has been placed, for example when it applies `SavedSystemOptions.xml`. The helper therefore places the window again once, when it first becomes visible after a resize. UAT-PD11 confirms this on a Mac with two displays and on Windows with two monitors.

## 3. Frame cap

- Today `wine.rs:63` sets `const FRAME_RATE_LIMIT: &str = "30";`, which is passed as `DXVK_FRAME_RATE` at `:87`. DXVK honours `DXVK_FRAME_RATE` as a present-rate limit (research-display-wine.md § 3). Replacing the constant with a preference value (`30`, `60`, or the display rate) is a small, Mac-only change.
- The display rate comes from `CGDisplayModeGetRefreshRate` for the display the game opens on (PD-31). That call returns 0 on some displays, so 0 or an unknown rate falls back to 60.
- Windows runs the game on native d3d9 (the DXVK DLL is Mac-only), which has no limiter knob, and the client has no vsync option (F21). A cap on Windows would need a client patch or a shipped limiter, both of which are out of scope. The setting is therefore hidden on Windows. This is flagged in D-PD20, not dropped.

## 4. Cursor lock

**What winemac.drv does.**

- `macdrv_ClipCursor` in `mouse.c` turns the rectangle into a `CGRect`, or `CGRectInfinite` on reset. A FIXME there notes that the clip applies only to the calling process ([mouse.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/mouse.c)).
- `cocoa_cursorclipping.m` uses a confinement handler by default on macOS 10.13 and later, through the undocumented `setMouseConfinementRect:`. That handler confines the cursor only while one of the **calling process's** Wine windows is frontmost and key, so Cmd+Tab releases it with no extra code ([cocoa_cursorclipping.m](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_cursorclipping.m)). It needs no Accessibility permission. The older event-tap fallback does need it on Catalina and later.

**Consequence.** The clip has to come from the game process itself:

- The launcher can't do it: it is not a Wine process.
- The helper can't do it: it has no window, and its clip would apply only to itself.
- On Windows, `ClipCursor` is a desktop-wide shared resource that the owner must release when it loses focus. Wine's behaviour above makes the release automatic on a Mac; Windows has no equivalent.

**What the client has.**

- SGW.exe imports `ClipCursor`, `GetClipCursor`, `SetCapture`, `ReleaseCapture`, `SetCursorPos` and `ShowCursor`.
- It contains the strings `ToolTip_MouseLock` and a UTF-16 `MouseLock`.
- The stock `SystemOptions.xml` has no mouse-lock option. So the game has some mouse-lock feature, probably a key binding or the mouse-look capture, but when it clips the cursor (fullscreen only, or windowed too) is unknown.

**Paths, in order.**

1. **RE first (PD-38, `game-archaeology-specialist`).** Find what `MouseLock` controls and every call site of `ClipCursor`. If the game clips in windowed mode, or behind an option, the launcher can enable that option in the profile like the other display keys, with no client patch. UAT-PD12 checks the result on a Mac.
2. **Otherwise, a hook (PD-39).** The already-injected `cimmeria_client_patches.dll` would clip the cursor to the game window's client area while the window is active and in windowed mode, and release it on `WM_ACTIVATE` inactive and on exit. This is a client patch, so it needs a maintainer decision (D-PD22).

## 5. Logs

- winemac.drv's display code logs to the `display` debug channel (`WINE_DEFAULT_DEBUG_CHANNEL(display)` in [display.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/display.c)). Window code uses `macdrv` and cursor code uses `cursor`.
- In `WINEDEBUG`, an entry is an optional class, then `+` or `-`, then a channel. With no class, every class of that channel is on ([wine(1)](https://man.archlinux.org/man/extra/wine/wine.1.en); example `warn+dll,+heap`).
- So D4's "`WINEDEBUG=+display` or a minimal set" becomes `err+all,fixme-all,+display` (PD-28).
- The "Open logs" button reuses `tauri_plugin_opener::reveal_item_in_dir`, which `show_install_directory` already calls (`S/main.rs:135-146`). No new dependency is needed.

## 6. Windows default (for D3)

D3 asks for windowed 1024x768 "on every platform where that's safe". Nothing found makes it unsafe on Windows:

- The lab starts every Windows client with `-windowed ResX=1280 ResY=720` (`crates/lab/src/supervisor/process.rs:54-79`).
- The chain ini's own default is windowed 1024x768 (`GameplayEngine.ini:351-354`).

So Windows defaults to windowed 1024x768 too (D-PD14, revised). The cost is that existing Windows players who chose fullscreen in the game get a window at their first Play after the update. The settings note says so, and Fullscreen stays one choice away (D-PD21).

`resolution` in `SavedSystemOptions.xml` is a D3D mode index that only fullscreen uses (research-display-client.md § 3). D3 lists it among the keys the launcher writes. The launcher writes the windowed size through the ini keys, which is what the client uses in windowed mode. It leaves `resolution` unchanged until an RE pass confirms how the client orders its mode list (D-PD21).
