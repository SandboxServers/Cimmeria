# Launch contract: display settings

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-26
> **Companions:** [Native game launch contract](launch.md), [Linux (Proton)](launch-linux.md#display-on-linux)

What the launcher writes into the client's display profile before each Play, window sizes, the frame cap and the monitor choice. The shared admission and lifecycle rules are in [launch.md](launch.md).

## Display settings before each Play

The launcher owns the game's display mode (owner D3, D-PD16). Before each Play,
after the login servers are resolved and before the launch helper command is built,
it writes the saved display choice (`DisplayOptions::effective()`: always a
window on a Mac, the size clamped into range; windowed 1024x768 when nothing
was saved) into the client profile, `<Documents>/My Games/Firesky/SGWGame/`.
On Windows that is the player's Documents folder; on a Mac it is the one user
folder other than `Public` under the Wine prefix's `drive_c/users`, and it must
stay inside the prefix. On Linux it is `drive_c/users/steamuser/Documents/My Games/Firesky/SGWGame/`
under the private Proton prefix, always a window, and the first Play creates
`SavedSystemOptions.xml` from a seed ([Display on Linux](launch-linux.md#display-on-linux)).

| Mode | `SavedSystemOptions.xml` | `Config/SGWEngine.ini` `[WinDrv.WindowsClient]` | Helper arguments (PD-25) |
|---|---|---|---|
| `windowed` | `windowedMode` `valueBool="true"` | `StartupFullscreen=False`, `StartupResolutionX=<width>`, `StartupResolutionY=<height>` | `-windowed`, `ResX=<width>`, `ResY=<height>` |
| `fullscreen` (Windows only) | `windowedMode` `valueBool="false"`; `resolution` untouched | `StartupFullscreen=True`; size untouched | none |

A display change made in the game's own options lasts one session: the next
Play writes the launcher's choice again. The step never creates either file or
adds a missing key, never touches another ini section (`[IniVersion]` included)
or the chain files under the game folder, keeps every other byte and the line
endings, skips a file that is not UTF-8, writes atomically (temp file in the
same folder, then rename) only when the content changes, and refuses a
read-only file. It is best effort: it never fails the launch.

It logs one `display_settings` event on `launcher.launch` with `operation`,
`mode` and, per file, `saved_options` and `engine_ini` (`written`, `unchanged`,
`missing`, `key_missing`, `unreadable` or `failed`; `saved_options` also reports
`seeded` on Linux, when the first Play created the file). It is info when both files
were `written` or `unchanged`, and warn with
`failure_code = "display_settings_not_applied"` otherwise. No field carries a path.

The launch helper request carries the table's helper arguments in `args`
(`Request::args`, both the native and the Wine request). An empty list is left
out of the JSON, so a fullscreen request is byte for byte the one before PD-25,
and SGW.exe's command line is the bare exe as before. The helper accepts only
the three display arguments: `-windowed`, `ResX=<n>` and `ResY=<n>` with 3-4
ASCII digits each, at most three entries and none repeated
(`game_launch::valid_args`). Any other list is refused before the game starts,
the same way as an invalid exe path: `not_started` at stage `request`.

### Window sizes that fit a display

Every snapshot lists the active displays (`displays`, engine `display_targets`:
CoreGraphics on a Mac, `EnumDisplayMonitors` on Windows; empty when the list
fails) and, per display, the sizes that fit it (`display_sizes`). Positions and
sizes are in the units the game sees: points on a Mac (Wine's `RetinaMode` is
off), and on Windows the monitor's pixels divided by its effective scale
(`GetDpiForMonitor`), rounded to the nearest unit, because SGW.exe declares no
DPI awareness and Windows scales its window. A 2560x1440 monitor at 150% is
1707x960 and is offered sizes up to 1280x800. The
size rule: a size from 1024x768, 1280x720, 1280x800, 1440x900, 1600x900,
1680x1050, 1920x1080 and 2560x1440 fits when its width is at most the display's
and its height plus an 80-point frame margin (menu bar or taskbar, and the title
bar) is at most the display's. 1024x768 always stays on the list. A display
setup is keyed by FNV-1a 64 over the sorted display rectangles, never by name. An empty list (a failure, or a Mac whose displays are
asleep) logs one WARN `display_list_empty` on `launcher.settings`, logged again
only after a non-empty list.

### The frame cap

On a Mac, `preferences.display.frame_cap` sets DXVK's limiter,
`DXVK_FRAME_RATE` (the client has no vsync setting and nothing else bounds its
render loop): `fps30` (the default, stored as no key, and what an older
preferences file reads as), `fps60`, or `display`. `display` follows the
refresh rate of the display the current setup remembers, else the main
display. That rate is read from one display list at Play and clamped into
30..=240; an unknown rate or an empty list gives 60. Every snapshot carries
`frame_caps`, the caps this platform offers: all three on a Mac; `fps30` and
`fps60` on Linux; and none on Windows, which has no limiter without a client
patch and ignores the saved value.

On Linux the limiter is the same `DXVK_FRAME_RATE` variable, set by the
launcher in the game's environment for each Play (DXVK's own d3d9 reads it; Proton
does not set it). Settings offers `fps30` (the default) and `fps60`, and Play
honours the saved choice. The change applies the next time you press Play.
`display` is not offered on Linux yet: it follows the refresh rate of a display
list, and Linux does not read one yet.

### The monitor per display setup

`save_display_options` takes an optional `monitor`: the `x, y, width, height`
of one of the snapshot's `displays`. Each save calls the display list once and
computes the setup key and the matching display from that one list. A monitor
that matches a listed display is stored in `preferences.display_placements`
under the current setup key, replacing that key's entry; an absent or `null`
monitor removes the entry, which means the main display. The list keeps the 8
newest setups and drops the oldest. When the display list is empty (every failed
enumeration and every sleeping Mac share one key), or the monitor matches no
listed display, the save stores, replaces and removes nothing. Every snapshot
carries `display_monitor`: the `index` of the display whose rectangle equals the
entry stored for the current setup key, or `null` (no entry, no matching
display, or no displays) for the main display. Rectangles are compared exactly
as `display_targets` reports them, the same tuple the key hashes, so on Windows
in DPI-scaled units. The snapshot's `preferences` leaves `display_placements`
out, and an empty list is stored as no key.

The save logs `display_options_saved` on `launcher.settings` (INFO) with `mode`,
`width`, `height`, `frame_cap` (`fps30`, `fps60` or `display`) and
`monitor_remembered`: true when this save stored a monitor for the current
setup, false when it removed the entry or stored nothing. No field carries a
rectangle or a setup key.

### Opening the game on the remembered monitor

At Play, when a placement is saved, the worker lists the displays once and asks
for the display the current setup remembers (as `display_monitor` does). Only
when that display is not the main display does the helper request carry
`place_on`: `[x, y, width, height]` in the units the game sees (PD-31). The
main display, an unknown setup or no displays leave the field out, which is the
request from before placement; `schema_version` stays 1. The engine sends it only
in windowed mode. With Windows exclusive fullscreen the field is left out,
because the D3D9 device owns its adapter, and moving that window could only
lose the device or minimize the game.

After `process_started`, and only with `place_on`, the helper starts a detached
thread that never delays its wait for the game's exit. Every 250 ms for up to
60 s it looks for a visible, unowned top-level window of the game's process.
It then lists the monitors and converts each `rcMonitor` to game units with the
same rule as `display_targets` (`GetDpiForMonitor`, `MDT_EFFECTIVE_DPI`; under
Wine the DPI reads 96, so nothing changes). The monitor with exactly the
`place_on` rectangle is chosen, else the first with the same size. When the
window is already on that monitor, nothing moves. Otherwise the window is
centred in the monitor's work area with `SetWindowPos` (no resize, no z-order
change, no activation; a window larger than the work area goes to its top-left)
and checked again 3 s later, moved once more if it went back.

The helper reports `window_placed` once, with `outcome` one of `placed`,
`already_there`, `no_window` (no window within 60 s) or `no_monitor` (no
monitor matches). The supervisor accepts one such report between
`process_started` and `process_exited`, logs `display_placement` on
`launcher.launch` (INFO, `operation`, `outcome`) and otherwise ignores it; it
never changes the launch outcome. No field carries a rectangle.
