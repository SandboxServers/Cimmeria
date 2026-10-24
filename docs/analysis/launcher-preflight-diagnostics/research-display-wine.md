# Research: display mode under Wine on macOS (Wine, winemac.drv, DXVK, launcher integration)

Date: 2026-10-24. Author: research agent (read-only; no prefix or code edits).
Feeds: [OWNER-DECISIONS.md § D2](OWNER-DECISIONS.md). Companion: `research-display-client.md` (the client's own knobs) and [observation-single-display.md](observation-single-display.md).

## Summary

The shield-band symptom is fully explained by source code, and it is **not** caused by the Wine registry. It's the D3D9 layer doing an exclusive-fullscreen display-mode change:

1. The client's saved options say fullscreen. `SavedSystemOptions.xml` in the game prefix has `windowedMode` = `false` (and `resolution` = index 0). `StartupFullscreen=False` in `SGWEngine.ini` doesn't override that. The lab already found this on Windows ([crates/lab/src/supervisor/process.rs](../../../crates/lab/src/supervisor/process.rs) `window_args` doc comment).
2. The game creates its D3D9 device with `Windowed = FALSE`. The DXVK `d3d9.dll` we ship then runs `EnterFullscreenMode`: it changes the monitor's display mode, strips the window frame, and calls `SetWindowPos(HWND_TOPMOST, …)` to cover the monitor.
3. winemac.drv turns the mode change into `CGCaptureAllDisplays()` (`setMode:forDisplay:`). Once displays are captured, it gives an active window level `CGShieldingWindowLevel() + 1`, plus 1 more for a floating (`WS_EX_TOPMOST`) window. That's **2147483628 + 2 = 2147483630**, the exact layer observed in both runs.
4. The single-display run confirms it: the built-in panel switched to a 1280x800 mode, with a shield window at exactly `CGShieldingWindowLevel` ([observation-single-display.md](observation-single-display.md)). On the dual-display Mac, `CGCaptureAllDisplays` shields **both** displays. Why the window was `onscreen=0` there isn't established (see Open questions).

**Recommendation.** On macOS, never let the game change the display mode. Set `DXVK_FORCE_WINDOWED=1` in the spawn environment on every launch. The pinned DLL honours it (verified by its strings and by the DXVK v1.10.3 source). Then express each mode through the window size and the client's own options:

| Mode | macOS strategy | Windows | Confidence |
|---|---|---|---|
| Windowed | `DXVK_FORCE_WINDOWED=1` + client windowed at the chosen WxH (client knob; see the companion doc) | Client knob only (native D3D9) | High |
| Borderless (fullscreen-window) | `DXVK_FORCE_WINDOWED=1` + client resolution = the target display's size **in points** (with `RetinaMode` off, the Wine desktop is in points). winemac.drv marks a frameless window that covers a screen as fullscreen: level `NSStatusWindowLevel + 1`, no capture. | Client "fullscreen"/borderless knob | Medium-high (mechanism verified in source; not run on this Mac yet) |
| Exclusive fullscreen | **Don't offer on macOS.** Map it to Borderless, or show it disabled with a note. It's the mode that failed. If it's ever offered, it needs explicit release/cleanup, because a hung game leaves every display captured. | Native, unchanged | High (that it's unsafe) |
| Resolution | Launch-time WxH through the client (`-windowed ResX= ResY=` args or saved options; see the companion doc). Under forced-windowed it only sizes the window. It never changes the Mac's display mode. | Same args, native | Medium (needs the args to reach SGW.exe; see § 4) |
| Wine virtual desktop | **Not viable on macOS**: winemac.drv has no virtual-desktop implementation | n/a | Medium-high |

Leave `HKCU\Software\Wine\Mac Driver` at its defaults (the prefix has no such key today). Optionally write `CaptureDisplaysForFullscreen="n"` explicitly as a pin. It's already the default, and it isn't the cause.

## 1. winemac.drv registry options (`HKCU\Software\Wine\Mac Driver`)

Source: `dlls/winemac.drv/macdrv_main.c`, `setup_options()` ([wine-mirror master](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/macdrv_main.c)). gitlab.winehq.org blocked the automated fetch (Anubis), so the GitHub mirror was used.

How it reads options. Each option is looked up first in `HKCU\Software\Wine\AppDefaults\<exe>\Mac Driver`, then in `HKCU\Software\Wine\Mac Driver`. The exe name is the basename of the image path, so for us `AppDefaults\SGW.exe\Mac Driver`. Booleans are true if the value's first character is `y`, `Y`, `t`, `T` or `1`. They're read once when the driver loads, so they must be set before SGW.exe starts.

| Value | Default | Semantics (from source) | Relevance |
|---|---|---|---|
| `CaptureDisplaysForFullscreen` | `false` | When true, `updateFullscreenWindows` calls `CGCaptureAllDisplays()` whenever any Wine window is "fullscreen" (its content rect covers a whole screen) **without** a mode change. When false, capture happens **only** through a real display-mode change. | Not our trigger: our prefix has no `Mac Driver` key, so it's false. Keep it false. Borderless relies on it being false. |
| `WindowsFloatWhenInactive` | `nonfullscreen` | `none`, `all`, or anything else (= nonfullscreen). Decides whether `WS_EX_TOPMOST` ("floating") windows stay at `NSFloatingWindowLevel` while the app is inactive. Default: they do, except fullscreen ones. | Optional: `none` stops a topmost SGW window sitting over other apps after Cmd-Tab. Low priority. |
| `AllowImmovableWindows` | `true` | Lets Wine make windows the Win32 app treats as unmovable (for example a `WS_POPUP` with no caption) immovable in Cocoa too. | Borderless: leave it true so macOS can't drag the borderless window off the screen origin. |
| `RetinaMode` | `false` | Read from the global key only (never AppDefaults), because DPI and monitor sizes must stay consistent across the prefix. When false, Wine reports displays in points (for example 1440x900 on a 2880x1800 panel) and macOS upscales. | Leave it false. The borderless WxH must then be the point size. Turning it on changes every mode Wine reports and the game's UI scale. That's out of scope. |
| `UseConfinementCursorClipping`, `CursorClippingLocksWindows` | `true`, `true` | `ClipCursor` handling. | Not display mode. |
| `AllowVerticalSync`, `AllowSetGamma`, `AllowSoftwareRendering`, `OpenGLSurfaceMode`, `EnableAppNap`, `UsePreciseScrolling`, `Left/RightOptionIsAlt`, `Left/RightCommandIsCtrl` | see source | GL, gamma, App Nap, input mapping. | Not display mode. `AllowSetGamma` only matters for the GL path. |

**Does capture explain the shield layer? Yes.** From `cocoa_window.m` `minimumLevelForActive:` ([source](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_window.m)):

- `captured = (fullscreen || [self screen]) && [[WineApplicationController sharedController] areDisplaysCaptured];`
- if captured: `level = CGShieldingWindowLevel() + 1; /* Need +1 or we don't get mouse moves */`, else if fullscreen: `level = NSStatusWindowLevel + 1`; then `if (self.floating) level++;`

From `cocoa_app.m` ([source](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_app.m)):

- `areDisplaysCaptured` returns `[originalDisplayModes count] > 0 || displaysCapturedForFullscreen`.
- `setMode:forDisplay:` returns early if the requested mode equals the current one ("Already there!"). Otherwise it calls `CGCaptureAllDisplays()` (only while the app is active; when inactive, the mode is stored as "latent" and applied on activation), then `CGDisplaySetDisplayMode`, and records the original mode.
- The original mode is restored and the displays released when the game sets the original mode back, on `handleCommandTab`, or when the process dies.

So the observed layer 2147483630 = shield 2147483628 + 1 (captured) + 1 (floating, from DXVK's `HWND_TOPMOST`). With `CaptureDisplaysForFullscreen` false, the only way to get there is a display-mode change, and that comes from the D3D9 fullscreen path (§ 3). A window that covers a screen **without** a mode change sits at `NSStatusWindowLevel + 1` (+1 if topmost) instead: above the menu bar and Dock, with no shield. That's the behaviour we want for Borderless.

`fullscreen` is computed in `updateFullscreen`: `!(styleMask & NSWindowStyleMaskFullScreen) && screen_covered_by_rect(contentRect, [NSScreen screens])`. A window is fullscreen when its content rect contains a whole screen frame.

## 2. Wine virtual desktop (`explorer /desktop=…`, `HKCU\Software\Wine\Explorer`)

**Not viable with winemac.drv.**

- Gcenx (macOS Wine packager), WineHQ forum, 2021: "Virtual desktop only works with X11 display driver not the macOS default winemac.drv" ([forum](https://forum.winehq.org/viewtopic.php?p=133982)).
- Bug 34374 (Wine 1.7.0 era) reported that the Mac driver ignores "Emulate a virtual desktop", while X11 on the Mac honours it ([via search summary of wine-bugs](https://list.winehq.org/hyperkitty/list/wine-bugs@list.winehq.org/)).
- Current source agrees: `dlls/winemac.drv/window.c` has no virtual-desktop code path. The desktop window gets no Cocoa window, and only top-level children of the desktop do ([source](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/window.c)). I found no release note adding it since, so this is inferred from current master and not tested on the pinned Wine 11.13.
- The only route would be winex11.drv plus XQuartz. That's a heavy dependency, slower, and the launcher explicitly runs winemac. Rejected.

How it works where it does work (Linux/X11): `wine explorer /desktop=Name,WxH program.exe`, or the registry pair `HKCU\Software\Wine\Explorer` `"Desktop"="Name"` + `HKCU\Software\Wine\Explorer\Desktops` `"Name"="WxH"`. The trade-offs on X11: the game's mode changes are contained inside the desktop window (that's the appeal), you get one extra window frame, and input focus quirks. None of this applies on macOS.

## 3. DXVK options (the pinned `d3d9.dll`)

**Correction to repo docs.** [crates/launcher/desktop/shell/resources/graphics/README.md](../../../crates/launcher/desktop/shell/resources/graphics/README.md) calls the binary "D9VK". The pinned DLL (SHA-256 `033cef34…045e`, checked on `~/Documents/cimmeria-test/game/Working/binaries/d3d9.dll`) self-identifies as **`DXVK v1.10.3-20230507-async (mac…)`**, the DXVK-macOS async fork. Its strings include `DXVK_FORCE_WINDOWED`, `DXVK_FRAME_RATE`, `DXVK_CONFIG_FILE`, `DXVK_LOG_LEVEL`, `DXVK_LOG_PATH`, `DXVK_HUD`, `DXVK_STATE_CACHE*` and these `d3d9.*` keys: `deferSurfaceCreation`, `enableDialogMode`, `enumerateByDisplays`, `deviceLossOnFocusLoss`, `dpiAware`, `forceAspectRatio`, `presentInterval`, `maxFrameRate`, `maxFrameLatency`, `numBackBuffers`, `tearFree`, `noExplicitFrontBuffer`, and others. Newer upstream keys (for example `d3d9.modeCountCompatibility`, `d3d9.forceRefreshRate`, `dxvk.allowFse`) are **absent**, so upstream `dxvk.conf` documentation must be checked against this binary.

What fullscreen does in DXVK 1.10.3 `D3D9SwapChainEx` ([v1.10.3 d3d9_swapchain.cpp](https://github.com/doitsujin/dxvk/blob/v1.10.3/src/d3d9/d3d9_swapchain.cpp)):

- `EnterFullscreenMode` saves the window rect, calls `ChangeDisplayMode` (`SetMonitorDisplayMode`, which is the mode change winemac turns into a capture), clears `WS_OVERLAPPEDWINDOW`, then `SetWindowPos(m_window, HWND_TOPMOST, <monitor rect>, SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOACTIVATE)`.
- `NormalizePresentParameters`: `if (env::getEnvVar("DXVK_FORCE_WINDOWED") == "1") pPresentParams->Windowed = TRUE;`. `Reset` branches on `Windowed`, so with the variable set, `EnterFullscreenMode` is never called: no mode change, no topmost, no capture. It applies at `CreateDevice` and at every `Reset`, so an in-game toggle can't re-enter exclusive mode either.

Caveat: forcing windowed doesn't resize or restyle the game's window. It keeps whatever size and style the client created, probably a frameless `WS_POPUP` at its configured resolution when the client thinks it's fullscreen. So the window size has to come from the client resolution (companion doc). That's exactly how Borderless works: set the resolution to the display's point size.

Other relevant keys (set with `DXVK_CONFIG="key = value; …"` or `DXVK_CONFIG_FILE=<guest path>`, per the [DXVK README](https://github.com/doitsujin/dxvk/blob/master/README.md)):

| Key | Use |
|---|---|
| `d3d9.deferSurfaceCreation = True` | DXVK's documented fix for "window stays black" in some games ([dxvk.conf](https://github.com/doitsujin/dxvk/blob/master/dxvk.conf)). Keep it as a fallback only if a forced-windowed launch is still black. |
| `d3d9.deviceLossOnFocusLoss` | Leave it false. The client's own reset-on-focus path ([multi-client-lab.md](../../reverse-engineering/findings/multi-client-lab.md)) already handles focus. |
| `d3d9.maxFrameRate` | Equivalent to the existing `DXVK_FRAME_RATE=30` env var. No change. |
| `d3d9.enumerateByDisplays` | Affects adapter enumeration on multi-monitor setups. Leave it at the default. A candidate knob if the dual-display run still misbehaves. |

I found no DXVK-on-MoltenVK issue specific to D3D9 exclusive fullscreen on Mac. The general DXVK-macOS guidance is that upstream DXVK needs Mac patches, which is why the fork exists ([Gcenx/wine-on-mac](https://github.com/Gcenx/wine-on-mac)). Our symptom is explained without a MoltenVK bug.

## 4. How the launcher can apply it

Current state (read from the code):

- Spawn env: `crates/launcher/desktop/engine/src/storage/launch/wine.rs` `game_environment()`. It builds on `mac_wine::environment()` (`crates/launcher/desktop/engine/src/mac_wine/mod.rs:153`, which sets `WINEPREFIX`, `WINESERVER`, `DYLD_LIBRARY_PATH`, `HOME`, `PATH`, `WINEDEBUG=-all`), then overrides `WINEDLLOVERRIDES=d3d9=n;…` (which re-enables winemac.drv, disabled in the headless base env) and adds `CX_FWD_COMPAT_GL_CTX`, `DXVK_FRAME_RATE=30`, `ROSETTA_X87_PATH`, `VK_DRIVER_FILES`.
- Game command line: **none.** The helper `Request` (`crates/launcher/desktop/runtime-probe/src/game_launch/mod.rs`) has `exe`, `directory` and `dlls` only, with no args field. The lab's `-windowed ResX= ResY=` (`crates/lab/src/supervisor/process.rs:58-79`) therefore has no launcher equivalent yet. Adding `args: Vec<String>` means a `schema_version` bump and a helper change in `game_launch/windows.rs` (the `CreateProcess` command line).
- Registry writes: production writes **none**. The only `wine reg add` is the PhysX diagnostic in `crates/launcher/desktop/engine/src/mac_wine/probe_smoke.rs:157-191` (env-cleared `wine reg add … /f /reg:32`, a 30 s bounded wait, then `stop_prefix`). That's the pattern to copy if a registry pin is ever wanted.
- Prefixes: the game runs in `state/game-prefixes/<install>/<op>/bottle`, separate from the `state/wine-prefixes/<install>` used for extraction. Neither has a `Mac Driver` key, so winemac runs on defaults.

Where a per-launch display mode plugs in:

1. **Setting**: add `display_mode` (`windowed`/`borderless`/`fullscreen`) and `resolution` to launcher preferences, beside `login_servers`. Default to Windowed on macOS (D2).
2. **Env (macOS, every mode)**: in `game_environment()`, always insert `DXVK_FORCE_WINDOWED=1`. It's a one-line, unconditional safety rail: it can't produce the capture even if the client's saved options say fullscreen. A unit test in `wine_tests.rs` can assert it's present. Revert it and the test fails.
3. **Window size/mode (both platforms)**: apply through the client in `preparation::prepare()` / `client_setup::prepare()`, which already rewrites `LoginInternal.lua` on every launch. Either rewrite `windowedMode`/`resolution` in `SavedSystemOptions.xml`, or pass `-windowed ResX= ResY=` once `Request` carries args. Which knobs the client honours (and whether it rewrites them on exit) is the companion doc's question. Because this runs on every launch, the client rewriting the file on exit doesn't matter.
4. **Borderless size on macOS**: resolve the target display's point size host-side (`NSScreen.frame` or `CGDisplayBounds`, both in points), not from Wine. Use the main display unless the user picks one. Pass it as ResX/ResY.
5. **Registry (optional)**: if `CaptureDisplaysForFullscreen=n` or `WindowsFloatWhenInactive=none` is wanted, write it once at game-prefix creation, under `AppDefaults\SGW.exe\Mac Driver` so it's scoped to the game, using the `probe_smoke` `wine reg add` pattern. That's not needed for the fix.
6. **Windows**: no env change (native d3d9; `DXVK_FORCE_WINDOWED` is only read by DXVK). Only step 3 applies.

## 5. Logging recommendation (capture evidence next time)

Today `WINEDEBUG=-all` and the game's stderr isn't captured (D2). For a diagnostic launch (opt-in, or the run after a "no visible window" failure):

- `WINEDEBUG=err+all,warn+display,+display,warn+macdrv,fixme-all`. `display` is winemac's mode-setting channel (`display.c`: `WINE_DEFAULT_DEBUG_CHANNEL(display)`). It logs `macdrv_ChangeDisplaySettings` calls, "looking for WxHxbpp @Hz" and "Requested display settings match mode N", which proves or rules out a mode change directly. Add `+macdrv` (window/event channel) only for short runs, because it's very chatty. Redirect wine's stderr to `<op log dir>/wine.log` with a size cap.
- `DXVK_LOG_LEVEL=info` (default `info`; `debug` only on request) and `DXVK_LOG_PATH=<guest path of the per-op log dir>`. Convert the path with `mac_wine::paths::guest`, because DXVK opens it through Win32. That gives `SGW_d3d9.log` with adapter, swapchain and present-mode lines, and lets you confirm `DXVK v1.10.3…` and the windowed swapchain.
- Optional `DXVK_HUD=devinfo,fps` for a visual "is it rendering" check.
- Host side, at launch+N s: snapshot `CGWindowListCopyWindowInfo` for the SGW windows (layer, bounds, onscreen) and `CGDisplayCopyDisplayMode` per display. That's what the single-display observation did by hand. A window layer >= `CGShieldingWindowLevel()`, or a display mode differing from the pre-launch mode, is an automatic "display captured" failure signal for the requirement-1/3 "no visible window" check.

## Open questions

- Why `onscreen=0` on the dual-display run. `CGCaptureAllDisplays` captures both displays. The window's monitor rect comes from DXVK's `GetDefaultMonitor()` / `MonitorFromWindow`. A placement on the non-main display, or a latent mode applied on activation, are both plausible. It becomes moot once the mode change is suppressed. To close it, rerun with the ultrawide and `WINEDEBUG=+display` without the fix.
- Whether SGW's "fullscreen" with forced-windowed gives a frameless popup at ResX/ResY (assumed above). Confirm on the first Borderless UAT.
- Whether macOS Spaces or "Displays have separate Spaces" changes winemac's fullscreen detection on the 3840x1080 display. Untested.

## Sources

- winemac.drv source, wine-mirror master: [macdrv_main.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/macdrv_main.c), [cocoa_app.m](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_app.m), [cocoa_window.m](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/cocoa_window.m), [window.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/window.c), [display.c](https://github.com/wine-mirror/wine/blob/master/dlls/winemac.drv/display.c). Read from master, not the pinned 11.13 tag. These code paths are long-standing, but they weren't diffed against 11.13.
- DXVK v1.10.3: [d3d9_swapchain.cpp](https://github.com/doitsujin/dxvk/blob/v1.10.3/src/d3d9/d3d9_swapchain.cpp). DXVK [README](https://github.com/doitsujin/dxvk/blob/master/README.md) (env vars), [dxvk.conf](https://github.com/doitsujin/dxvk/blob/master/dxvk.conf) (keys; upstream master, so check against the binary's strings).
- WineHQ forum, virtual desktop on macOS: [Gcenx reply](https://forum.winehq.org/viewtopic.php?p=133982).
- Local evidence (read-only): the pinned `d3d9.dll` strings and SHA-256; the game prefix `SavedSystemOptions.xml` (`windowedMode=false`); `SGWEngine.ini` (`StartupFullscreen=False`); the `user.reg` of both prefixes (no `Mac Driver` key); runtime `bin/wine --version` = `wine-11.13`.
