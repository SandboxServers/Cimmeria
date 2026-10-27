# Launch contract: Linux (Proton)

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-26
> **Companions:** [Native game launch contract](launch.md), [display settings](launch-display.md), [Linux install and release notes](release.md), [campaign ledger](../../../../docs/analysis/launcher-linux-play-proton/README.md)

How the Linux AppImage starts the game through an installed Proton. The shared admission, lifecycle and resource rules are in [launch.md](launch.md).

## Linux (Proton)

On Linux, Play runs the Windows game through a Proton build the player already
has. The launcher downloads no Proton and no Wine. The pieces below run in this
order on every Play. Steps 3 and 6 do nothing once they have succeeded; step 4 then only hashes the two DLLs.
Source: `engine/src/storage/launch/proton.rs` and `engine/src/linux_proton/`.

| Step | What happens | Failure |
|---|---|---|
| 1. Find the Proton | The saved choice is searched for again; a build that is gone, or whose `version` file changed, stops Play. | `proton_missing` |
| 2. Claim the prefix | The private prefix under the state root is locked for the whole Play. A second launcher gets `busy`. | `busy` |
| 3. Set up the prefix | `proton run wineboot.exe`, only when the stamp is missing or names another Proton build. | `proton_setup_failed` |
| 4. Native DirectX DLLs | `d3dx9_40.dll` and `xinput1_3.dll` are checked on every Play and extracted when missing or wrong. | `proton_setup_failed` |
| 5. Client preparation | Login servers, stock names, ASLR and containment, the same as every platform. | `launch_prepare_failed` |
| 6. PhysX | The prerequisite worker installs PhysX 7.11.13 once per prefix. | `proton_setup_failed` |
| 7. Display | Windowed only; the first Play seeds the saved options. | never fails Play |
| 8. Run | `proton runinprefix` starts the launch worker, which starts `SGW.exe`. | the worker's own codes |

### Finding and ranking Proton

Discovery runs live on every call (`launch::resources::found`); nothing is
cached. It reads Steam's client folder (`~/.steam/root`, then
`~/.local/share/Steam`, then `~/.steam/steam`), every library listed in
`steamapps/libraryfolders.vdf`, the client's `compatibilitytools.d` and the
system folder `/usr/share/steam/compatibilitytools.d`. A folder counts only
when it holds `proton`, `toolmanifest.vdf` and `version`.

Each build gets an id, `<kind>:<directory name>`, and a kind that sets its rank:

| Rank | Kind | Directory name |
|---|---|---|
| 1 | `ge` | starts with `GE-Proton` |
| 2 | `experimental` | starts with `Proton - Experimental` |
| 3 | `valve` | `Proton <digit>...` under `steamapps/common` |
| 4 | `other` | anything else that is complete |

Within a kind the newest comes first. The saved preference
(`proton_command` `select`, id only) wins when that id is still found; otherwise
the top-ranked build runs. The Settings display panel shows a Proton picker
when at least one build is found, and the frontend sends an id, never a path.
With none found, Play stays off and the status lists `proton` among the
missing pieces. The Play copy tells the player to install Proton Experimental
from Steam, or GE-Proton with ProtonUp-Qt, then reopen the launcher.

### Invocation

Every call is bare: `<root>/proton`, never the Steam Linux Runtime container's
`_v2-entry-point`. The toolmanifest's runtime is logged, not used.

| Purpose | Command |
|---|---|
| Prefix setup | `<root>/proton run wineboot.exe` (180 second limit) |
| Prerequisite worker and game | `<root>/proton runinprefix <Z:\ path of the worker>` |

Host paths reach the worker as `Z:\` paths (`linux_proton::paths::guest`). The
launcher passes the worker request over the same pipe protocol the other
platforms use.

### The private prefix and its stamp

One prefix lives under each state root, at `<state>/proton/compatdata`
(`STEAM_COMPAT_DATA_PATH`; the Wine prefix is its `pfx` folder). The launcher
never touches a prefix Steam owns. `cimmeria-setup.json` in `compatdata` records
`schema_version` (1), `proton_version` (the `version` file text of the build that
last set the prefix up) and `physx` (`not_installed`, `installed` or `failed`).
The setup runs again when the stamp is missing, unreadable or names another
`proton_version`; the PhysX state survives a re-run. The setup counts as good
only when the script exits 0 and the prefix has its `system.reg`.

### Native DirectX DLLs

The game needs its own `d3dx9_40.dll` and `xinput1_3.dll`; Wine's copies do not
work. Play extracts them from the cabinets the install already holds, with no
download and no Wine:

| DLL | Cabinet (under `.cimmeria-prerequisites/DX9.0c`) | Size | SHA-256 |
|---|---|---|---|
| `d3dx9_40.dll` | `Nov2008_d3dx9_40_x86.cab` | 4,379,984 bytes | `16fd909a...e216b` |
| `xinput1_3.dll` | `APR2007_xinput_x86.cab` | 81,768 bytes | `8d540d48...970a7` |

The pins are in `linux_proton::directx::DLLS`. On every Play each DLL in the
prefix's `drive_c/windows/syswow64` is hashed; a missing, resized, linked or
wrong one is extracted from its cabinet (`extract_cab_member`, one LZX member),
checked against the size and hash, written to a temporary file and renamed
over the target. Bytes that miss the pin never reach the prefix. The check runs
every Play because a Proton prefix upgrade may rewrite `syswow64`. The game
process gets `WINEDLLOVERRIDES=d3dx9_40=n;xinput1_3=n;winemenubuilder.exe=d;mscoree=d;mshtml=d`.
A failure logs `proton_directx_failed` (no DLL name yet) and Play stops with
`proton_setup_failed`.

### PhysX

The first Play in a prefix installs PhysX 7.11.13 through the prerequisite
worker (`cimmeria-prerequisite-worker.exe`, the launch worker's sibling in
`windows/`), run with `proton runinprefix` and a 180 second limit. Only a
`probed` result whose SDK reports `InitializedAndReleased` stamps the prefix
`installed`. An installer failure, a failed probe or a worker fault stamps
`failed`, logs one `proton_physx` row with a closed `result` and `reason`, and
stops Play with `proton_setup_failed`; the next Play tries again. The scratch
folder is `<state>/proton/scratch/<operation>`; the host creates only the parent.
Nothing there is cleaned up yet.

### Display on Linux

Linux Play is windowed only. `DisplayOptions::effective()` still applies, so a
saved fullscreen choice runs as a window. The client profile lives in the
prefix, at `pfx/drive_c/users/steamuser/Documents/My Games/Firesky/SGWGame/`.
Unlike Windows and Mac, the first Play creates `SavedSystemOptions.xml` from a
441-byte seed (refused when any folder from the profile up to the prefix is a
symlink, and never over a file that appeared meanwhile). The `display_settings`
event then reports `saved_options="seeded"` and that Play runs at 1024x768
(FU-10: the seeded options carry no saved size). Later Plays edit the file the
way [Display settings](launch-display.md#display-settings-before-each-play) describes.

### Environment

The child starts from an empty environment (C-4). These variables are copied
from the launcher when set: `HOME`, `USER`, `LOGNAME`, `PATH`, `LANG`,
`LC_ALL`, `DISPLAY`, `WAYLAND_DISPLAY`, `XAUTHORITY`, `XDG_RUNTIME_DIR`,
`XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP`, `XDG_DATA_DIRS`,
`DBUS_SESSION_BUS_ADDRESS`, `PULSE_SERVER`, `PIPEWIRE_RUNTIME_DIR`, `SteamDeck`
and `GAMESCOPE_WAYLAND_DISPLAY`. The launcher adds `STEAM_COMPAT_DATA_PATH`,
`STEAM_COMPAT_CLIENT_INSTALL_PATH` and `WINEDLLOVERRIDES`. The game gets
`DXVK_FRAME_RATE` (the frame cap: `30` for 30 fps, the default, or `60` for 60 fps; § The frame cap), and, when the per-launch log folder exists,
`DXVK_LOG_LEVEL=info` and `DXVK_LOG_PATH`. With game telemetry on, the telemetry
pass-through variables are added. `PROTON_LOG`, `SteamAppId` and `LD_PRELOAD`
are never passed.

### Logs and events

Each Play writes `proton.log` (setup and launch stderr), `physx.log` and the
DXVK logs under `logs/game/<operation>/`. The `launcher.proton` target, which
the launcher does not forward (it is not in `launcher_targets.txt`), logs
`proton_physx`, `proton_directx` (per DLL, with the DLL name and `present` or `installed`)
and `proton_directx_failed`. The Settings picker logs `proton_refused` (reason
`unknown_proton` or a save failure) when a choice is refused. No field carries
a path.

### Failure codes and tags

| Code | When | Copy |
|---|---|---|
| `proton_missing` | No Proton found, or the selected build is gone or changed. | "Proton isn't installed." Install Proton Experimental from Steam, or GE-Proton with ProtonUp-Qt, then press Play again. |
| `proton_setup_failed` | Prefix setup, the DirectX step or PhysX failed. | "Proton couldn't set up the game." Press Play to try again, then upload logs from Settings. |

A session with game telemetry on carries the tags `desktop-launcher`, `linux`
and `proton`.

### What Linux still lacks

Repair, Update and the launcher's self-update answer `platform_unavailable` on
Linux (D-PP18). Game Mode and Gamescope handling are a follow-up.

### Embedded Windows pieces

The AppImage embeds the launch worker, the prerequisite worker, the patches DLL
and the telemetry DLL, each pinned by SHA-256 as on Windows. They are written
to the app-local payload cache and checked again when Play binds resources.
