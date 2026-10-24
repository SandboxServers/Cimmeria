# Launcher preflight diagnostics: Mac display extras (owner D4; PD-31 to PD-40)

Ledger: [README.md](README.md). The rules R1-R10, the checks table (D1, D2e, D2s, D2p, D3e, D3s, D3p, F1, MG, WG, RG, WR) and C-PD1 are in [work-packets.md](work-packets.md); **every packet here follows them.** The display contracts C-PD6 and C-PD7 are in [work-packets-display.md](work-packets-display.md). Owner ruling: [OWNER-DECISIONS.md § D4](OWNER-DECISIONS.md). Feasibility and sources: [research-mac-extras.md](research-mac-extras.md).

Paths: `E` = `crates/launcher/desktop/engine/src/`, `S` = `crates/launcher/desktop/shell/src/`, `FE` = `crates/launcher/desktop/frontend/`, `RP` = `crates/launcher/desktop/runtime-probe/src/`, `LD` = `crates/launcher/desktop/docs/`. The anchors were taken at `main` `2019922fe`, and wave D moves some of them. Re-grep every anchor, and return `blocked` when one no longer holds.

## D4 coverage

| D4 extra | Packets | Verdict (research-mac-extras.md) |
|---|---|---|
| (1) Borderless | PD-27 (work-packets-display.md), sized from PD-31 | after UAT-PD8 |
| (1) Safe-mode relaunch | PD-36 (wave 4, after PD-29/PD-30) | feasible |
| (2) Retina-aware size list | PD-31, PD-34 | feasible |
| (2) `RetinaMode` HiDPI | PD-40 | **flagged:** experimental; UAT-PD10 and D-PD23 first |
| (2) Monitor picker per display setup | PD-31, PD-32, PD-33, PD-34 | feasible; UAT-PD11 |
| (3) Frame cap 30 / 60 / display | PD-35 | macOS only; **flagged:** not possible on Windows without a client patch (D-PD20) |
| (4) Cursor lock | PD-38 (RE), then PD-39 | **flagged:** only from inside the game; D-PD22 decides whether a client patch is allowed if the game has no switch for it |
| (4) Per-launch logs and "Open logs" | PD-28 (now `+display`), PD-37 | feasible |

## Dispatch

| Wave | Packets and order | Ships as |
|---|---|---|
| M | PD-38 can start now: it is RE only, needs no worktree build, and runs on any host. When wave D has merged: PD-28 and PD-31 in parallel. Then PD-32 (after PD-31). Then PD-35 (after PD-28 and PD-32) and PD-33 (after PD-32 and PD-25) in parallel. Then PD-34 (after PD-33 and PD-35). Runs at the same time as waves 2 and 3. | Integration branch `launcher-preflight-diagnostics/mac-extras` (worktree `pd-wm`) from `origin/main`, merged per packet with `tools/build-lane/merge-noff.sh`. One PR (PM). |
| 4 | PD-36 (after PD-29, PD-30 and PD-24) and PD-37 (after PD-28), alongside PD-29 and PD-30. | One PR per packet. |
| 5 | PD-39 (after PD-38, and after D-PD22 if it needs a hook) and PD-40 (after UAT-PD10 and D-PD23). | One PR per packet. |

Shared files inside wave M: `E/storage/display_options.rs` (PD-32, then PD-35) and `E/storage/launch/wine.rs` (PD-28, then PD-35). Hence the order above.

## Contract

### C-PD8: displays, placement and frame cap (PD-31, PD-32, PD-35)

`E/display_targets/mod.rs` (new, cross-platform types, `pub mod display_targets;` in `E/lib.rs`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DisplayInfo {
    pub index: u32, pub main: bool,
    pub x: i32, pub y: i32, pub width: u32, pub height: u32, // in the units the game sees (see PD-31)
    pub scale_percent: u32,   // 100, 200, 150 ...
    pub refresh_hz: u32,      // 0 = unknown
}
pub const SIZES: &[(u32, u32)] = &[(1024,768),(1280,720),(1280,800),(1440,900),(1600,900),(1680,1050),(1920,1080),(2560,1440)];
pub const FRAME_MARGIN: u32 = 80;                           // menu bar or taskbar plus the title bar
pub fn sizes_for(d: &DisplayInfo) -> Vec<(u32, u32)>;       // SIZES with w <= d.width and h + FRAME_MARGIN <= d.height; never empty (1024x768 stays)
pub fn setup_key(ds: &[DisplayInfo]) -> String;             // 16 lowercase hex: FNV-1a 64 over (x,y,width,height) sorted; no names
pub fn list() -> Vec<DisplayInfo>;                          // macOS CoreGraphics, Windows EnumDisplayMonitors; empty on failure
```

`E/storage/display_options.rs` additions (C-PD6 stays the base):

- `DisplayOptions` gains `#[serde(default)] frame_cap: FrameCap`. `enum FrameCap { Fps30, Fps60, Display }` (snake_case `fps30`, `fps60`, `display`) has the default `Fps30`, which is today's fixed value (R10).
- `Preferences` gains `#[serde(default)] display_placements: Vec<Placement>`, where `struct Placement { setup: String, x: i32, y: i32, width: u32, height: u32 }`. The list holds at most 8 entries, and saving a 9th drops the oldest.
- `SaveDisplayOptions` gains `#[serde(default)] monitor: Option<MonitorRect>` (`x, y, width, height`). When it is some, the engine stores it under the current `setup_key(list())`, replacing any entry for that key. When it is none, the entry is removed, which means "main display".
- `NativeSnapshot` gains:
  - `displays: Vec<DisplayInfo>`
  - `display_sizes: Vec<Vec<[u32; 2]>>`, one list per display, from `sizes_for`
  - `display_monitor: Option<u32>`, the index the current setup remembers
  - `frame_caps: Vec<FrameCap>`: `[fps30, fps60, display]` on macOS, empty elsewhere
- TS: the same fields, in the same commit (R8).

Event `display_options_saved` gains `frame_cap` and `monitor_remembered` (bool).

## PD-38: what the game's mouse lock does (RE; D4 (4); wave M, Ready now)

- **Host:** any. **Agent:** `game-archaeology-specialist`, with the `re-lookup` skill: docs first, then the Ghidra MCP or headless Ghidra. **Worktree:** none (it writes docs only, committed by the orchestrator's hands-on session). **Branch:** `launcher-preflight-diagnostics/pd-38-mouselock-re`.
- **Do:**
  1. In SGW.exe, find every call site of `ClipCursor`, `SetCapture` and `ReleaseCapture`. For each one, find what decides whether it runs: fullscreen only, a mouse-look drag, or an option.
  2. Find what the strings `MouseLock` (UTF-16) and `ToolTip_MouseLock` belong to: a key binding, an options entry, or a UI toggle. Find where its value is stored (which `.xml` or `.ini` file, and which key).
  3. Answer one question: does the game confine the cursor in **windowed** mode, on its own or behind a switch the launcher could write before Play, the way C-PD7 writes the display keys?
- **Output:** `docs/analysis/launcher-preflight-diagnostics/research-mouselock.md` (CRLF) with addresses and evidence, plus a fact file in `.claude/agent-memory/game-archaeology-specialist/`. Return `PD-38 done - <path>` with one of three verdicts: `switch` (name the key), `always` (the game already clips in windowed mode), or `none`.
- **Then:** the owner runs UAT-PD12 on a Mac. The orchestrator re-specs PD-39 from the verdict.

## PD-31: list displays and the sizes that fit (D4 (2); C-PD8)

- **Host:** mac. **Mac gate:** yes. **Windows gate:** WG (the `cfg(windows)` enumeration). **Agent:** packet-coder role (R1). **Worktree:** `pd-31`, from the `pd-wm` tip.
- **Files:** `E/display_targets/mod.rs` (new), `E/display_targets/mac.rs` (new), `E/display_targets/windows.rs` (new), `E/display_targets/tests.rs` (new), `E/lib.rs` (`mod` line), `E/commands.rs` (snapshot fields), `FE/src/contract.ts`, the contract test that decodes `NativeSnapshot`.
- **Do:**
  1. Write the pure part of `mod.rs` per C-PD8: the types, `SIZES`, `sizes_for` and `setup_key`. Write FNV-1a by hand; `DefaultHasher` is not stable across Rust releases.
  2. `mac.rs`: raw FFI to CoreGraphics (`#[link(name = "CoreGraphics", kind = "framework")]`, no new crate). Use:
     - `CGGetActiveDisplayList` to list the displays
     - `CGDisplayBounds` for position and size, in points
     - `CGMainDisplayID` for `main`
     - `CGDisplayCopyDisplayMode`, then `CGDisplayModeGetPixelWidth` divided by the width in points, for `scale_percent`
     - `CGDisplayModeGetRefreshRate`, rounded, for `refresh_hz`

     Release every mode you copy. Index the displays in list order.
  3. `windows.rs`: `EnumDisplayMonitors`, `GetMonitorInfoW` (`rcMonitor`, `MONITORINFOF_PRIMARY`), and `EnumDisplaySettingsW(ENUM_CURRENT_SETTINGS)` for `dmDisplayFrequency`. Get the scale from `GetDpiForMonitor` if `windows-sys` already enables that feature; otherwise use 100. Return `blocked` rather than add a feature.
     - **Units:** first check whether SGW.exe's manifest declares DPI awareness. Use the manifest text in the game folder or the RE docs, and say what you found in the report.
     - If SGW.exe is not DPI-aware, report the size divided by the scale, because that is what the game sees. Otherwise report pixels.
  4. `commands.rs`: fill in `displays` and `display_sizes`. Call `list()` once per snapshot; it is cheap.
- **Tests:**
  - `sizes_for` on 1710x1107 gives every size up to 1600x900, excluding 1680x1050 because 1050 + 80 > 1107.
  - `sizes_for` on 800x600 gives `[1024x768]`.
  - `setup_key` is the same for a list and its reverse, and different when one display moves.
  - A known vector pins the FNV value.
  - A `#[cfg(target_os = "macos")]` smoke test: `list()` is non-empty on the CI Mac and has exactly one `main`.
  - Revert proof: dropping the margin puts 1680x1050 back on the list.
- **Checks:** D1, D2e, D3e `display_targets`, D3s `contract`, F1, MG, WG.
- **Docs rows:** 57 (`LD/launch.md`: the size rule).
- **Commit:** `feat(launcher): list displays and the window sizes that fit them (PD-31)`

## PD-32: remember the monitor per display setup (D4 (2); C-PD8)

- **Host:** mac. **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-32`, from the `pd-wm` tip after PD-31.
- **Files:** `E/storage/display_options.rs`, `E/storage/display_options_tests.rs`, `E/storage/mod.rs` (only the field line in `Preferences`, `StoredPreferences` and the conversions), `E/commands.rs`, `E/storage/settings_events.rs`, `FE/src/contract.ts`, the contract test.
- **Do:**
  1. Add `Placement`, `MonitorRect` and the `monitor` field on `SaveDisplayOptions`, per C-PD8. Write `pub(crate) fn remember(list: &mut Vec<Placement>, setup: &str, monitor: Option<MonitorRect>)`, which is pure: replace, remove, or push and keep at most 8.
  2. Write `pub(crate) fn placement_for(placements: &[Placement], displays: &[DisplayInfo]) -> Option<DisplayInfo>`. It takes the entry for `setup_key(displays)`, then the display whose rectangle equals the stored one. No match gives `None`, which means the main display. Fill `display_monitor` in the snapshot from it.
  3. Add `monitor_remembered` to the event.
- **Tests:**
  - `remember` replaces the entry for the same key, removes it on `None`, and keeps the newest 8.
  - `placement_for` returns `None` for an unknown setup, and the matching display after a round trip through `save_display_options` and a reopen.
  - An old `preferences.json` reads with an empty list.
  - Revert proof: skipping the setup-key match returns a display for an unknown setup.
- **Checks:** D1, D2e, D3e `display_options`, D3s `contract`, F1, MG.
- **Docs rows:** 34 (event fields), 57.
- **Commit:** `feat(launcher): remember which monitor the game opens on for each display setup (PD-32)`

## PD-33: the launch helper places the game window on the chosen monitor (D4 (2))

- **Host:** mac for the engine half. The helper half is i686 Windows code, verified by **RG** and **WR**, with UAT-PD11 on both platforms. **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-33`, from the `pd-wm` tip after PD-32, with PD-25 merged.
- **Files:** `RP/game_launch/mod.rs`, `RP/game_launch/windows.rs`, `RP/game_launch/placement.rs` (new), `RP/game_launch/placement_tests.rs` (new), `E/storage/launch/worker.rs` and `E/storage/launch/wine.rs` (only the `Request` literal and the event arm), `E/storage/launch/events.rs` (one fn), `LD/launch.md`.
- **Do:**
  1. `Request` gains `#[serde(default, skip_serializing_if = "Option::is_none")] place_on: Option<[i32; 4]>` (x, y, width, height). `schema_version` stays 1, for the same reason PD-25 gives.
  2. `Event` gains `WindowPlaced { outcome: PlacementOutcome }`, where `PlacementOutcome` is one of `placed`, `already_there`, `no_window` or `no_monitor`. The engine logs it as `display_placement` (INFO; `launcher.launch`; `operation`, `outcome`) and otherwise ignores it.
  3. `placement.rs`, pure part: `choose(monitors: &[[i32; 4]], target: [i32; 4]) -> Option<usize>` takes an exact rectangle first, then the same size, else `None`. `centre(work: [i32; 4], window: [i32; 4]) -> (i32, i32)` keeps the window inside the work area.
  4. `windows.rs`, after `ProcessStarted` and only when `place_on` is some, in a thread that must never delay `running.wait()`:
     - Every 250 ms for up to 60 s, `EnumWindows` for a visible top-level window of the game's process id. Then `EnumDisplayMonitors` and `choose`.
     - If `MonitorFromWindow` already returns that monitor, report `already_there`.
     - Otherwise `SetWindowPos(.., SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE)` to the `centre` of the monitor's work area. 3 s later, place it once more if the window has moved back to another monitor.
     - Emit `WindowPlaced` once.
  5. The engine sends `place_on` from `placement_for` (PD-32), and only when the result is not the main display. The main display means no field, which is the same request as today (R10).
- **Tests:**
  - The `choose` and `centre` tables, including a window larger than the work area, which is pinned to the work area's top-left.
  - A request without `place_on` decodes, and one with `place_on: null` is not emitted.
  - The `WindowPlaced` event round-trips.
  - Engine: a placement for the main display sends no field.
  - Revert proof: dropping the size fallback fails the "moved monitor" row.
- **Checks:** D1, D2p and D3p `game_launch` (on the Mac only if the crate builds there; otherwise say so), RG, WR, D2e, D3e `launch`, MG.
- **Docs rows:** 57 (`LD/launch.md`), 34.
- **Commit:** `feat(launcher): open the game on the monitor the player chose (PD-33)`

## PD-35: a frame cap the player chooses (D4 (3); macOS only, D-PD20)

- **Host:** mac. **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-35`, from the `pd-wm` tip after PD-28 and PD-32.
- **Files:** `E/storage/display_options.rs`, `E/storage/display_options_tests.rs`, `E/storage/launch/wine.rs`, `E/storage/launch/wine_tests.rs`, `E/commands.rs` (`frame_caps`), `FE/src/contract.ts`, `docs/guides/macos.md`.
- **Do:**
  1. Add `FrameCap` and the `frame_cap` field per C-PD8. Write `pub(crate) fn frame_rate(cap: FrameCap, refresh_hz: u32) -> u32`, which is pure: `Fps30` gives 30, `Fps60` gives 60, and `Display` gives `refresh_hz` clamped into 30..=240, or 60 when `refresh_hz` is 0.
  2. `wine.rs`: replace `const FRAME_RATE_LIMIT` (`:63`) with the value from `frame_rate(display.frame_cap, <refresh of the placement display or the main display>)`, passed into `game_environment`. Keep the variable name `DXVK_FRAME_RATE`.
  3. On Windows, `frame_caps` is empty and the value is ignored.
- **Tests:**
  - The `frame_rate` table, including 0 Hz giving 60 and 360 Hz giving 240.
  - `game_environment` with `Fps60` has `DXVK_FRAME_RATE == "60"`.
  - An old preferences file gives 30, which is today's value.
  - Revert proof: a fixed 30 fails the `Fps60` row.
- **Checks:** D1, D2e, D3e `display_options`, D3e `wine`, F1, MG.
- **Docs rows:** 57 (`docs/guides/macos.md`, `LD/launch.md`).
- **Commit:** `feat(launcher): choose the Mac frame cap: 30, 60 or the display's rate (PD-35)`

## PD-34: sizes, monitor and frame cap in the gear (D4 (2), (3))

- **Host:** mac (frontend). **Agent:** packet-coder role. **Worktree:** `pd-34`, from the `pd-wm` tip after PD-33 and PD-35.
- **Files:** `FE/src/display-view.ts`, `FE/src/display-view.test.ts`, `FE/ui/index.html` (only inside `#display-panel`).
- **Do:**
  1. Size select: the list is `display_sizes[<chosen monitor or main>]` plus the saved size when it is not on the list, labelled `(doesn't fit this display)`. Changing the monitor refreshes the list.
  2. Monitor select `#display-monitor`, shown only when `displays.length > 1`. Label each entry `Main display` / `Display 2` ... with its size (`1710 × 1107`). Saving sends `monitor` as the chosen rectangle, or `null` for the main display.
  3. Frame cap select `#display-frame-cap`, shown only when `frame_caps` is non-empty. The labels are `30 fps (cooler, quieter)`, `60 fps` and `Match display (<n> Hz)`.
  4. Copy under the monitor select: `Remembered for this set of displays.`
- **Tests:**
  - Two displays render the monitor select, and choosing display 2 swaps the size list.
  - One display renders no monitor select.
  - A Windows snapshot (`frame_caps` empty) has no frame-cap select.
  - Save sends `monitor` and `frame_cap`, and the first press shows `Saving…`.
  - Revert proof: a fixed size list fails the swap test.
- **Checks:** F1.
- **Docs rows:** none (PD-18 or PD-21 document the setting).
- **Commit:** `feat(launcher): pick the window size, monitor and frame cap in Settings (PD-34)`

## PD-36: offer a safe relaunch when the window doesn't appear (D4 (1); wave 4)

- **Host:** mac. **Mac gate:** yes. **Windows gate:** WG, with UAT-PD13 on Windows. **Agent:** packet-coder role. **Worktree:** `pd-36`, from `origin/main` after PD-24, PD-29 and PD-30 have merged.
- **Brief input:** first grep `E/storage/launch/` and `S/host/launch/` for an existing path that stops the running game: `kill`, `terminate`, `TerminateProcess`, `wineserver -k`, or a cancel that ends the process. Say what you found.
- **Files:** `E/storage/launch/safe_mode.rs` (new), `E/storage/launch/safe_mode_tests.rs` (new), `E/storage/launch/mod.rs` (`mod` line), the launch request type in `S/host/launch/mod.rs` (one optional field), `E/storage/launch/worker.rs` (call lines; it must stay under 450), `E/storage/launch/events.rs` (one fn), `FE/src/launch-view.ts`, `FE/src/launch-view.test.ts`, `FE/src/launch-workflow.ts`, `LD/launch.md`.
- **Do:**
  1. The launch request gains `#[serde(default)] safe: bool`, with the TS change in the same commit.
  2. `safe_mode.rs`: `pub(crate) fn safe(display: DisplayOptions) -> DisplayOptions` returns `Windowed`, 1024x768, `Fps30`. Safe mode also sends no `place_on` (main display). Nothing is saved: the preference is unchanged.
  3. The worker applies `safe(..)` in place of the saved options when `safe` is true, and logs `launch_safe_mode` (INFO, `launcher.launch`, `operation`).
  4. `launch-view.ts`: when `status.failure == "game_window_hidden"`, add a button `#launch-safe` labelled `Relaunch in a safe window`.
     - If a stop path exists, the first press stops the game, then sends a launch with `safe: true`.
     - If none exists, show the button only once the game has exited, with the line `Quit the game first (Cmd+Q on a Mac, Alt+F4 on Windows), then relaunch.`
     - Either way, the first press shows `Relaunching…`.
- **Tests:**
  - `safe()` ignores a saved Borderless 2560x1440 at 60 fps.
  - A launch with `safe: true` writes 1024x768 windowed through PD-24's capture, and the preference file is unchanged.
  - FE: the button appears for `game_window_hidden` and not for `spawn_failed`.
  - Revert proof: applying the saved options makes the 1024x768 assertion fail.
- **Checks:** D1, D2e, D2s, D3e `safe_mode`, D3s `launch`, F1, MG, WG.
- **Docs rows:** 57 (`LD/launch.md`), 34.
- **Commit:** `feat(launcher): offer a safe windowed relaunch when the game window doesn't appear (PD-36)`

## PD-37: an "Open logs" button for the game's launch logs (D4 (4); wave 4)

- **Host:** mac. **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-37`, from `origin/main` after PD-28 has merged.
- **Files:** `E/storage/launch/game_logs.rs` (PD-28; add `latest`), `E/storage/launch/game_logs_tests.rs`, `S/host/game_logs.rs` (new), `S/host.rs` (`mod` line), `S/main.rs` (one `generate_handler!` line), `E/commands.rs` (snapshot flag), `FE/src/display-view.ts` or a new `FE/src/game-logs-view.ts` with its test, `FE/src/launch-view.ts` (one link), `FE/src/contract.ts`, `FE/ui/index.html`, `FE/package.json`.
- **Do:**
  1. `game_logs::latest(state_root) -> Option<PathBuf>`: the newest folder under `logs/game/`.
  2. `S/host/game_logs.rs`: `#[tauri::command] open_game_logs() -> Result<bool, JobError>`. It returns `Ok(false)` when there is no folder. Otherwise it calls `tauri_plugin_opener::reveal_item_in_dir(<folder>/wine.log or the folder)`, the same pattern as `show_install_directory` (`S/main.rs:135-146`). An opener error is `JobError::Io`.
  3. The snapshot gains `game_logs: bool`: true on macOS, false elsewhere, because PD-28 writes these logs only on a Mac.
  4. FE: a button `Open game logs` in the gear, under the display settings, shown only when `game_logs` is true. The first press shows `Opening…`. `Ok(false)` shows `No game logs yet. They appear after the next Play.` The `game_window_hidden` and `game_exited_nonzero` failure lines on a Mac get the same button.
- **Tests:**
  - `latest` picks the newest of three folders and returns `None` when there are none.
  - The command returns `false` with an empty state.
  - FE: the button is hidden when `game_logs` is false. One press calls `open_game_logs` once and shows `Opening…`.
  - Revert proof: sorting by name instead of mtime fails `latest`.
- **Checks:** D1, D2e, D2s, D3e `game_logs`, D3s `game_logs`, F1, MG.
- **Docs rows:** 57 (`docs/guides/macos.md` troubleshooting: "Settings → Open game logs").
- **Commit:** `feat(launcher): open the game's launch logs from Settings (PD-37)`

## PD-39: cursor lock in windowed mode (D4 (4); wave 5)

- **Status (2026-10-24):** PD-38 returned **`none`**, confirmed by two review passes (`research-mouselock.md`, `.claude/reports/launcher-preflight-diagnostics/PD-38-review-1.md`). D-PD22 is **resolved by owner D5**: the client-patch hook is approved. PD-39 is the `none` path, and it still depends on UAT-PD12 for live verification.
  - The hook lives in `cimmeria_client_patches.dll` (`crates/client-patches`). While the game window is active and the game is windowed, it clips the cursor to the window's client area. It releases the clip on `WM_ACTIVATE` inactive, on minimise and on exit. Built for Windows (i686), so verified by CI and UAT-PD12 on both platforms.
  - The window-handle chain (`GEngine` → `+0x2dc` → `+0x44` → `+0x88`) is medium confidence. Cross-check it against the second route in `research-display-client.md:139`, or use `GetForegroundWindow` filtered by the game's PID, before relying on it.
  - The `switch` and `always` branches no longer apply.
- **Settings:** an `Lock the mouse to the game window` toggle in the gear, default on, applied at the next Play.

## PD-40: experimental Retina rendering (D4 (2); wave 5)

- **Status:** **BlockedDecision D-PD23** and BlockedDependency on UAT-PD10. Expected shape (research-mac-extras.md § 1):
  - `DisplayOptions` gains `retina: bool` (macOS only, default false). It is offered only when the chosen display's `scale_percent` is 200.
  - Before every Play the launcher writes `HKCU\Software\Wine\Mac Driver` `RetinaMode` = `y` or `n` into the game prefix. It always writes, so the prefix never keeps a stale value. Use the same `wine reg add` path the launcher's Wine smoke probe uses; find it with a grep for `reg add` in `E/mac_wine/`. Add `LogPixels` only if UAT-PD10 says so.
  - When `retina` is on, the size list switches to pixels.
- **Host:** mac. MG plus a Mac with a Retina panel and a second 1x display for UAT-PD10.
