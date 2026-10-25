# PP-S1 findings: Proton Play spike on the Deck

> **Evidence not committed.** The photos (`PP-S1-*.jpg`) and the crash dumps (`crashdumps/`, each zip holds a minidump and the machine's `hwprofile.xml`) are 15 MB of binaries kept off git. They are on the Deck in the main checkout's `.claude/reports/launcher-linux-play-proton/` (gitignored). Logs and prefixes are in `~/pp-spike/`. Where the text says "beside this report" or names a photo, it means that folder.

**Result: done. Steps 1-6 pass on GE-Proton11-11 (bare mode), with two changes the launcher must make.** The second pass ran 09:03-09:14.

**The two changes:**
1. **Native `d3dx9_40.dll`** from the game's own `.cimmeria-prerequisites/DX9.0c/Nov2008_d3dx9_40_x86.cab`, set to native with `WINEDLLOVERRIDES="d3dx9_40,xinput1_3=n"`. The native `xinput1_3` from `APR2007_xinput_x86.cab` went in with it; whether that one is needed was not split out. With Wine's builtin `d3dx9_40` the game crashes on its shader assertion. `DXSETUP.exe` itself fails under Proton, so the DLLs have to be extracted from the cabs.
2. **The windowed client profile** that the launcher already writes on Windows and macOS (`launch.md` § Display settings before each Play): `SavedSystemOptions.xml` `windowedMode` `valueBool="true"`, plus `SGWEngine.ini` `[WinDrv.WindowsClient]` `StartupFullscreen=False`, `StartupResolutionX=1280`, `StartupResolutionY=720`. Without it, the game's own default `windowedMode=false` leaves the window black until a taskbar touch.

**Still open:**
- Step 7's prerequisite worker returns `failed/probe`.
- Container mode fails step 2.
- Proton 9 and Proton 11 were not run past step 2.
- World entry took about 2-3 minutes.

See [Step 3b](#step-3b-packaged-directx-tested-on-ge11-pass), [Steps 4-6](#step-4-patches-dll-tested-on-ge11dx-pass) and the rulings.

**First-pass result (08:08-08:52, before the two changes): blocked.** Stdio works in bare mode on every Proton, but no Proton reaches the login screen (step 3), so step 4 cannot pass. Per the packet: "Stop and report if … no Proton passes step 4". With PhysX in the prefix, the game crashes the same way on GE-Proton11-11 and Proton Experimental: `ACCESS_VIOLATION` at `SGW.exe!0x00EC6F9D` after the assertion `ShaderRef != NULL && *ShaderRef != NULL` (`ShaderManager.h(759)`). Container mode (D-PP3) fails step 2 on every Proton.

Run 2026-10-25, 08:08-08:52 local (UTC-5), Steam Deck, Desktop Mode (KDE Plasma, Wayland session, Xwayland `DISPLAY=:0`), owner at the Deck. Every row is marked **tested** or **untested**.

## Preconditions

- `git log --oneline 0a25ef5d4..origin/main -- crates/launcher tools/launcher-release` → empty output, exit 0 (`origin/main` = `85cc212f3`). **Nothing landed in the launch code since the design.** Owner told.
- The campaign docs `docs/analysis/launcher-linux-play-proton/` are **not in the worktree or on any branch**. They exist only as untracked files in the main checkout (`git -C ~/Projects/Cimmeria status` → `?? docs/analysis/launcher-linux-play-proton/`). Read from there.
- The packet's output is a committed `worknotes/PP-S1.md`. Per the owner's brief (no repo source changes) this report is in `.claude/reports/` instead, and nothing is committed.
- Deck changes: only `~/pp-spike/` (bin, compat-*, logs, scratch). No packages, no Steam settings, no downloads beyond the named `gh run download`.

## Inputs

### Windows pieces (tested)

`gh run download 40142858139 -n windows-runtime-probe -D ~/pp-spike/bin` → exit 0. Run 40142858139 is the newest green `launcher runtime probe` on `main` (head `5f3a86c21`, an ancestor of `origin/main`). All four are i686 **debug** builds, copied flat into `~/pp-spike/bin/`.

| File | SHA-256 |
|---|---|
| cimmeria-launch-worker.exe | `66776b87e9f2d30514f9cc150779336300a45ca00358ba0b3017c1b0b29cf84c` |
| cimmeria-prerequisite-worker.exe | `af9cdecfb8796d0da0713d89c69969b5099d0e4d1caf5764235b83fc5ff889f1` |
| cimmeria_client_patches.dll | `52376c24ccae441a1fc182f536af61831929f1cb53f44b08cb3831ff4d2d252f` |
| cimmeria_client_telemetry.dll | `ff889a79ccbbd98c4914a8cf64a3f120412c454b29131f588bfc20cf18b7604d` |

**Deviation:** the packet's `client-telemetry-build` artifact does not exist. Its workflow (file `client-telemetry-build.yml`, name `client-telemetry`, push to `main` only) has no upload step. `gh api "repos/{owner}/{repo}/actions/artifacts?name=client-telemetry-build"` → `total_count 0`, and the three newest green `client-telemetry` runs (39961995814, 39153483556, 38976131897) have no artifacts. The telemetry DLL used is the one inside `windows-runtime-probe`: `strings | grep -ciE 'lab[-_ ]?bridge'` → `0`, read as the non-lab-bridge build.

### Game copy (tested)

- At start there was no usable install. The default state root `~/.local/share/app.cimmeria.launcher.desktop` holds `state/install-result.json` → `"outcome":"install_failed"` (op `6dae159d…`, Oct 24), and only a stage folder with a 4135724034-byte `.tmp-seed-…download`.
- The owner installed from `client.rar` with the launcher during the spike. The running launcher was a **debug build** (`/home/deck/Projects/Cimmeria/target/desktop/debug/cimmeria-launcher-desktop`), not the AppImage, with `XDG_DATA_HOME=/home/deck/cimmeria-launcher-test-20261025/xdg-data`. Result: `state/install-result.json` → `"outcome":"content_prepared"` (op `c776d3d8-9e59-4505-b308-2f5b483901ba`).
- Game: `~/cimmeria-launcher-test-20261025/xdg-data/app.cimmeria.launcher.desktop/Stargate Worlds/game/Working/binaries/SGW.exe`, on `/dev/nvme0n1p8` (internal, **not the SD card**). The binaries folder is spelled `Working/binaries` (lower case, F-10). PhysX package: `…/game/.cimmeria-prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe`.

### Protons (tested)

| Tag | Root | `version` | `require_tool_appid` → R |
|---|---|---|---|
| ge11 | `~/.local/share/Steam/compatibilitytools.d/GE-Proton11-11-x86_64` | `1792432585 GE-Proton11-11` | 4183110 → `SteamLinuxRuntime_4/_v2-entry-point` |
| exp | `~/.local/share/Steam/steamapps/common/Proton - Experimental` | `1791989105 experimental-11.0-20261014-x86_64` | 4183110 → SLR_4 |
| p9 | `~/.local/share/Steam/steamapps/common/Proton 9.0 (Beta)` | `1749140930 proton-9.0-4f` | 1628350 → `SteamLinuxRuntime_sniper/_v2-entry-point` |
| p11 (owner-added) | `/run/media/mmcblk0p1/steamapps/common/Proton 11.0` | `1791559474 proton-11.0-2e-x86_64` | 4183110 → SLR_4 |

**F-04 correction:**
- The SD-card library also holds `Proton 10.0` (`1785139158 proton-10.0-4b`, needs 1628350 = sniper) and `Proton 11.0`.
- The GE directory is `GE-Proton11-11-x86_64`, not `GE-Proton11-11`.
- The owner asked to add Proton 11.0 with steps 1-6.

For every run: `STEAM_COMPAT_CLIENT_INSTALL_PATH=~/.local/share/Steam` and `STEAM_COMPAT_DATA_PATH=~/pp-spike/compat-<tag>`. Logs are in `~/pp-spike/logs/<tag>-s<N>[x][-mode].{out,err,run}`. Helper scripts (scratchpad, not committed):
- `s1.sh`: `timeout 120 "$P/proton" run wineboot.exe`
- `s2.sh`: the step-2 pipe, bare then `"$R" --verb=waitforexitandrun -- …`
- `s3.sh`: builds the request with `jq` and pipes it to `"$P/proton" runinprefix 'Z:\home\deck\pp-spike\bin\cimmeria-launch-worker.exe'`, timestamping each stdout line
- `s7.sh`: the prerequisite worker via `runinprefix`

## Step 1: prefix setup (bare) — tested, **pass ×4**

| Proton | Exit | Secs | `pfx/system.reg` | syswow64 `d3d9.dll`, `strings \| grep -ci dxvk` | Result |
|---|---|---|---|---|---|
| ge11 | 0 | 4 | 3916493 B | 694 | **pass** |
| exp | 0 | 4 | 3880249 B | 689 | **pass** |
| p9 | 0 | 4 | 3597533 B | 353 | **pass** |
| p11 | 0 | 7 | 3874912 B | 641 | **pass** |

Log lines:
- ge11: `Proton: Upgrading prefix from None to GE-Proton11-11 (/home/deck/pp-spike/compat-ge11/)`
- exp and p11: `Proton: Upgrading prefix from None to 11.0-100`
- p11 also: `Proton: Error while copying to ".../system32/amdxcffx64.dll": No such file or directory` (exit still 0)

The `cmd.exe /c exit` fallback was not needed.

## Step 2: stdio — tested, **bare pass ×4, container fail ×4**

Request (verbatim from the packet): `{"schema_version":1,"operation_id":"6f1c0a52-0000-4000-8000-000000000001","exe":"Z:\\nope\\SGW.exe","directory":"Z:\\tmp","dlls":[]}`

| Proton | Bare `"$P/proton" runinprefix '<worker>'` | Container `"$R" --verb=waitforexitandrun -- "$P/proton" runinprefix '<worker>'` |
|---|---|---|
| ge11 | **pass**: exit 0, 1 line | **fail**: exit 1, 0 stdout lines. Repeated once after `wineserver` had exited: exit 1, 0 lines. |
| exp | **pass**: exit 0, 1 line | **fail**: exit 1, 0 lines |
| p9 | **pass**: exit 0, 1 line | **fail**: exit 1, 0 lines (sniper) |
| p11 | **pass**: exit 0, 1 line | **fail**: exit 1, 0 lines |

- **Bare stdout** (all four): `{"schema_version":1,"operation_id":"6f1c0a52-0000-4000-8000-000000000001","observation":{"event":"not_started","stage":"request"}}`
- **Container stderr** (SLR_4, no error line): `i386-linux-gnu-capsule-capture-libs: warning: Dependencies of /usr/lib32/libMangoHud.so not found, ignoring: Missing dependencies: Could not find "libxkbcommon.so.0" …`, then `ntsync: up and running.` On p9 (sniper), stderr has only `wine: using kernel write watches, use_kernel_writewatch 1.`
- **Why `stage:"request"`:** this is the worker's request-validation exit (`runtime-probe/src/game_launch/windows.rs`, the `exe.parent() != directory` check). The packet's request has `exe` `Z:\nope\SGW.exe` and `directory` `Z:\tmp`. The pass condition (one `not_started` line) is still met. The step-3 requests use the exe's own folder as `directory`.

## Step 3: game, no DLLs — tested on ge11 and exp, **fail** (no login screen)

Request: `{"schema_version":1,"operation_id":"6f1c0a52-0000-4000-8000-000000000003","exe":"Z:\\home\\deck\\cimmeria-launcher-test-20261025\\xdg-data\\app.cimmeria.launcher.desktop\\Stargate Worlds\\game\\Working\\binaries\\SGW.exe","directory":"Z:\\…\\Working\\binaries","dlls":[],"args":["-windowed","ResX=1280","ResY=720"]}`. Bare mode only: container mode failed step 2.

| Run | Prefix state | `process_started` | `process_exited` | Host exit | `wineserver` | On screen (owner) |
|---|---|---|---|---|---|---|
| ge11 #1 (s3) | no PhysX | 08:35:16.846 pid 264 | 08:36:22.478 code 0 | 08:36:22.531, exit 0 (53 ms) | gone after 2 s | Dialog "Failed to initialize physics sub system. Perhaps try reinstalling PhysX drivers. (NxCreatePhysicsSDK(...) returned NULL)". The owner clicked OK. Photo `PP-S1-ge11-s3-physx-dialog.jpg`. |
| ge11 #2 (s3b) | PhysX MSI installed (step 7) | 08:37:15.135 pid 264 | 08:39:16.000 code 1 | 08:39:16.054, exit 0 | gone after 2 s | Black window titled "Stargate Worlds" (photo `PP-S1-ge11-s3b-black-window.jpg`). Music starts when the owner touches the taskbar icon, then "Game Has crashed! Sorry. Dump has been created …" (photo `PP-S1-ge11-s3b-crash-dialog.jpg`). |
| exp #1 (exp-s3) | PhysX MSI installed (step 7) | 08:40:05.498 pid 240 | 08:40:35.699 code 1 | 08:40:35.741, exit 0 | gone after 2 s | The owner reported the PhysX dialog, but the prefix has a crash dump at 13:40:30 UTC with the same shader crash. The owner was also handling the GE11 run, so the visual report is less certain. |
| ge11 #3 (s3c) | PhysX | 08:44:53.095 pid 264 | 08:46:10.876 code 1 | exit 0 | gone after 2 s | The black window stays black with no music. Touching the taskbar icon minimises it, the music starts, and "Game Has crashed!" appears **twice** (dismissed twice). Photos `PP-S1-ge11-s3c-taskbar.jpg` and `PP-S1-ge11-s3c-crash-dialog.jpg`. |
| ge11 #4 (s3d) | PhysX, plus `DXVK_FORCE_WINDOWED=1` | 08:50:37.952 pid 264 | 08:52:13.378 code 1 | 08:52:13.435, exit 0 | gone after 2 s | Same: black with no music until the taskbar icon is touched, then the same crash. Photo `PP-S1-ge11-s3d-forcewindowed-black.jpg`. The variable was confirmed in `/proc/<SGW pid>/environ`. |

**Window and process evidence (ge11 #3), sampled at 3, 10 and 20 s:**
- One `SGW.exe` process (`pgrep -fc 'Working\\binaries\\SGW.exe'` → 1), plus Wine's own `services.exe`, `winedevice.exe` ×2, `plugplay.exe`, `svchost.exe`, `lsass.exe`, `explorer.exe /desktop`, `rpcss.exe`, `tabtip.exe` and `xalia.exe`.
- X windows of the `SGW.exe` pid (`xdotool search --pid`):
  - `Stargate Worlds`: `IsViewable`, `-geometry 1280x800+0+37`, `WM_CLASS "steam_proton"`, `_NET_WM_WINDOW_TYPE_NORMAL`, `_NET_WM_STATE_FOCUSED`, and the active window
  - `Default IME`: `IsUnMapped` 1x1
  - a nameless window: `IsUnMapped` 1x1
- The taskbar shows one entry. **One game window, not two.**
- The window was not visible at t+3 s and was visible at t+10 s.

**Lifecycle (tested, pass on every run):** `process_started`, then `process_exited`; the host process exits within 60 ms of `process_exited`; `wineserver` is gone within 2 s. Nothing lingered after any run (`pgrep` for `SGW.exe|launch-worker|prerequisite-worker|wineserver` → none at 08:42).

**Display arguments:** `-windowed` gives a decorated window, but `ResX=1280 ResY=720` did not give a 1280x720 window: X reports `1280x800+0+37` with and without `DXVK_FORCE_WINDOWED=1`. Who sizes it (the game or KWin) was not determined.

### Crash dumps (tested)

The game writes dumps into the prefix, at `pfx/drive_c/users/steamuser/Documents/My Games/Firesky/SGWGame/CrashDumps/`. The game folder's `Working/binaries/CrashDumps/` holds only `DO_NOT_DELETE.TXT`. Proton stderr has no exception record (`grep -ciE 'exception|page fault|backtrace'` → 0), because the game's own handler catches the crash. Copies: `crashdumps/` beside this report (`*.crash.zip`, each with `minidump.dmp`, `report.txt`, `Interface.log`, `hwprofile.xml` and `system_options.xml`; `report.txt` extracted).

| Dump (UTC) | Run | `report.txt` |
|---|---|---|
| `13.38.55` | ge11 #2 | `Exception: ACCESS_VIOLATION (0xC0000005)` / `Address: SGW.exe!0x00EC6F9D` / `Error history: ShaderRef != NULL && *ShaderRef != NULL  c:\build\qa\sgw\working\development\src\engine\inc\ShaderManager.h(759)` |
| `13.40.30` | exp #1 | identical exception, address and error history |
| `13.45.54` | ge11 #3 | identical |
| `13.52.00` | ge11 #4 (`DXVK_FORCE_WINDOWED=1`) | identical |

The call stack in every dump is `SGW.exe!0x00EC6F9D` ← `<unknown>!0xFE9C2E00`/`0xFEABEE00` ← `SGW.exe!0x0081C2E0` ← `0xCCCCCCCC` frames. Not diagnosed further (owner brief: no diagnosis past the packet).

**Steps 3-7 on p9 and p11: untested.** The owner stopped the p9 PhysX run before it wrote anything (`p9-s7.out` empty, no `AGEIA Technologies` folder in `compat-p9`), then chose to stop after the `DXVK_FORCE_WINDOWED` retry.

## Step 3b: packaged DirectX — tested on ge11, **pass**

Off-packet; the owner asked for it. Prefix `~/pp-spike/compat-ge11-dx` is a `cp -a` of `compat-ge11` (which already has PhysX), so the ge11 prefix itself is unchanged.

1. **`DXSETUP.exe /silent` fails** (tested, three tries):
   - `proton run 'Z:\…\DX9.0c\DXSETUP.exe' /silent` → exit 247 in 3 s.
   - `proton run DXSETUP.exe /silent` from the DX folder → exit 1 with `err:steam:run_process Failed to create process L"DXSETUP.exe /silent": 2`, because the steam shim does not resolve relative paths.
   - `proton runinprefix 'Z:\…\DXSETUP.exe' /silent` with `WINEDEBUG=err,+loaddll` → exit 247. It loads `dsetup.dll`, `DSETUP32.DLL` and `C:\users\steamuser\AppData\Local\Temp\DX7c9f.tmp\dxupdate.dll` (native), logs `err:mscoree:LoadLibraryShim error reading registry key for installroot`, and exits.
   - No `DirectX.log` or `DXError.log` was written, and `syswow64/d3dx9_40.dll` is unchanged (Wine's, 08:08).
   - Wine's `expand.exe` on the cab: `cabinet extraction failed`, exit 1.
2. **Manual extraction** with the host's `7z x` (exit 0) from the packaged cabs:

   | Cab | DLL | SHA-256 |
   |---|---|---|
   | `Nov2008_d3dx9_40_x86.cab` | `d3dx9_40.dll`, 4379984 B, PE32 i386, Oct 10 2008 | `16fd909aeb68d0d1aca8529dc7f78880b97d6649d70ce8d03a2c858bc28e216b` |
   | `APR2007_xinput_x86.cab` | `xinput1_3.dll`, 81768 B | `8d540d484ea41e374fd0107d55d253f87ded4ce780d515d8fd59bbe8c98970a7` |

   Both were copied into `compat-ge11-dx/pfx/drive_c/windows/syswow64/`. The game imports exactly `d3d9.dll`, `d3dx9_40.dll` and `XINPUT1_3.dll` (`strings` on `SGW.exe`).
3. **Run** `ge11dx-s3`: `s3.sh` with no DLLs and args `-windowed ResX=1280 ResY=720`, env `WINEDLLOVERRIDES="d3dx9_40,xinput1_3=n" WINEDEBUG=+loaddll`.
   - `08:58:59.010 {"…","observation":{"event":"process_started","guest_pid":264}}`
   - Load trace: `d3dx9_40.dll … native`, `XINPUT1_3.dll … native`, `d3d9.dll … native` (DXVK), `AGEIA Technologies\v2.6.3\PhysXCore.dll … native`, and the game's `PhysXLoader.dll` and `NxCooking.dll`. (`d3dx9_31` builtin was also loaded.)
   - **The login screen renders**: account name and password fields, LOGIN, and the SGW website, Manage Account, Forums and Options buttons, on the Stargate scene. Photo `PP-S1-ge11dx-s3-login.jpg`.
   - The owner still had to touch the taskbar icon (twice) before the game started drawing.
   - Window: `Stargate Worlds`, `IsViewable`, `-geometry 1280x785+0--22`, plus the same two unmapped 1x1 helpers.
   - The owner quit: `09:02:39.559 {"…","observation":{"event":"process_exited","guest_pid":264,"code":0}}`, host `exit=0` at 09:02:39.606 (47 ms later), `wineserver` gone after 2 s, nothing lingering.
   - No new crash dump.
   - **Pass**: `process_started`, the login screen renders, the quit gives `process_exited`, and the host exits within 10 s.

### Client profile fix for the black window (tested)

- The game's first run wrote `Documents/My Games/Firesky/SGWGame/SavedSystemOptions.xml` with `windowedMode` `valueBool="false"`, and `Config/SGWEngine.ini` `[WinDrv.WindowsClient]` with `StartupFullscreen=False`, `StartupResolutionX=1024`, `StartupResolutionY=768`. The launcher never ran Play on this Deck, so its display step (`launch.md:397-425`) never rewrote them.
- The originals are backed up at `~/pp-spike/SGWEngine.ini.orig` and `~/pp-spike/SavedSystemOptions.xml.orig`. Edited with `sed`, CRLF kept, as the launcher's windowed row does it:
  ```
  SGWEngine.ini           -StartupResolutionX=1024  +StartupResolutionX=1280
                          -StartupResolutionY=768   +StartupResolutionY=720
  SavedSystemOptions.xml  windowedMode valueBool="false" -> "true"
  ```
- **Effect (steps 4 and 5, owner):** "Game launched fine and didn't require a touch", and the same on the next run. At t+12 s the window is `Stargate Worlds`, `IsViewable`, `-geometry 1280x688+0+37`, `_NET_WM_STATE_FOCUSED`, `WM_STATE Normal`, and the active window.
  - Before the change it was 1280x800 or 1280x785. 688 = 800 - 37 (title bar) - 75 (taskbar), so the 720-high window looks clamped to the work area. Not tested further.

## Step 4: patches DLL — tested on ge11-dx, **pass**

Request: as in step 3, with `"dlls":["Z:\\home\\deck\\pp-spike\\bin\\cimmeria_client_patches.dll"]`. Same prefix, overrides and client profile as above. Log `~/pp-spike/logs/ge11dx-s4.{out,err}`.

- `09:07:32.546 … "event":"process_started","guest_pid":264`
- The load trace shows `Z:\home\deck\pp-spike\bin\cimmeria_client_patches.dll at 790A0000: native`.
- The login screen renders with no touch (owner).
- `Working/binaries/cimmeria-client-patches.log`, where `client-patches.md:195` says the DLL logs (a file next to `SGW.exe`), is saved as `~/pp-spike/logs/ge11dx-s4-patches.log`:
  ```
  [cimmeria-client-patches +0ms] attached, version 0.1.0, host Z:\…\Working\binaries\SGW.exe
  [cimmeria-client-patches +0ms] strstream: msvcp80.dll loses a write before a read (wrote 4, read 0); underflow shim installed in vtable slot 4, the round trip now reads 4
  [cimmeria-client-patches +0ms] lua51.dll exports resolved
  [cimmeria-client-patches +1ms] install lock taken
  … 8 sites, each ": stock" …
  [cimmeria-client-patches +11ms] hooked FEngineLoop::Tick
  [cimmeria-client-patches +20ms] hooked Client_NetIn_EntityMethodDispatch
  [cimmeria-client-patches +29ms] hooked EntityDescription_GetExposedClientMethodByIndex
  [cimmeria-client-patches +29ms] Black Market installed: received calls go to the Lua table CimmeriaBM, …
  [cimmeria-client-patches +9371ms] registered CimmeriaBMNative (version 0.1.0) in lua_State 0xe3b36980 (#1)
  ```
- Quit: `09:08:16.613 … "event":"process_exited","guest_pid":264,"code":0`, host `exit=0` at 09:08:16.656, `wineserver` gone after 2 s.

## Step 5: both DLLs — tested on ge11-dx, **pass**

Request `"dlls":["Z:\\…\\cimmeria_client_patches.dll","Z:\\…\\cimmeria_client_telemetry.dll"]` (patches first). Log `~/pp-spike/logs/ge11dx-s5.{out,err}`, plus `-patches.log` and `-telemetry.log`.

- `09:08:33.101 … "event":"process_started","guest_pid":264`
- The login screen renders with no touch (owner: "Launched fine").
- **`Working/binaries/cimmeria-client-telemetry.log` appears** (634 B, 09:08):
  ```
  [cimmeria-client-telemetry +0ms] attached, version 0.1.0, host Z:\…\SGW.exe, dll Z:\home\deck\pp-spike\bin\cimmeria_client_telemetry.dll
  [cimmeria-client-telemetry +0ms] cimmeria-client-telemetry build flavour: player
  [cimmeria-client-telemetry +0ms] no telemetry session (could not read current-session.json at Z:\…\Working\binaries\sessions\current-session.json: Path not found …
  ```
  "No telemetry session" is expected: the launcher did not run Play, so no session file was written.
- With both DLLs loaded, the patches DLL still logs `install lock taken` and `Black Market installed` (+43 ms).
- Quit (after step 6): `09:13:34.633 … "event":"process_exited","guest_pid":264,"code":0`, host `exit=0` at 09:13:34.676 (43 ms), `wineserver` gone after 2 s, nothing lingering, no new crash dump.

## Step 6: login and world entry — tested on ge11-dx (in the step 5 session), **pass, with a slow world load**

- **Server:** `Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua` (written by the launcher at install: "this file is rewritten before every launch") has `LoginMod.servers["Cimmeria"] = "http://play.cimmeria.app:8081"`. The owner used their own account; no credentials were recorded.
- **Login and character select:** owner: "Logged in". Photo `PP-S1-ge11dx-s6-loading.jpg` shows the Stargate Worlds loading screen at 09:09.
- **World entry:**
  - At 09:10 the HUD, minimap, mission tracker ("In Service to a Lesser", "Shiny Things", "Recruit Morrigan") and chat were drawn over a dark, partly loaded scene. Photo `PP-S1-ge11dx-s6-in-world.jpg`. Owner: "I am stuck loading in though".
  - While it was stuck:
    - `Interface.log` had 12× `(Error) CEGUI::ScriptException in file .\Src\MapManager.cpp(296) : No player` from `Minimap.lua:117`, starting 09:09:45.
    - `SGW.exe` was at 185% CPU (mostly `RenderingThread`), 1.36 GB RSS, 0 MB disk reads in 10 s, `rchar` about 85 KB/s.
    - One UDP socket was open.
  - **Owner: "It did eventually finish loading in."** At 09:12 the character stands beside the NPC "Machra", the chat shows the welcome and "You have joined channel" lines, and the mission list and minimap are filled. Photo `PP-S1-ge11dx-s6-in-world-loaded.jpg`.
  - The patches DLL logged `registered CimmeriaBMNative … in lua_State 0xbcafe980 (#2)` at +189381 ms (≈09:11:42), the in-world UI's Lua state.
  - **World entry took about 2-3 minutes** after the loading screen. Not diagnosed: no Windows comparison on the same server was run, and the owner did not say whether Windows is this slow.

## Step 7: PhysX — tested on ge11 and exp, **fail** (result is not `probed`)

- **Without PhysX** (fresh ge11 prefix, step 3 run #1): the game process starts but the game stops at the PhysX dialog quoted above. **PhysX is needed.**
- **Prerequisite worker** via `"$P/proton" runinprefix 'Z:\home\deck\pp-spike\bin\cimmeria-prerequisite-worker.exe'`, with the request `{"schema_version":1,"operation_id":"6f1c0a52-0000-4000-8000-000000000007","prefix_generation":"6f1c0a52-0000-4000-8000-0000000000a7","game_binaries":"Z:\\…\\game\\Working\\binaries","package":"Z:\\…\\game\\.cimmeria-prerequisites\\PhysX\\PhysX_7.11.13_SystemSoftware.exe","scratch":"Z:\\home\\deck\\pp-spike\\scratch\\<tag>-<epoch>"}` (fields from `D/runtime-probe/src/prerequisite/mod.rs`; the scratch folder must not exist beforehand, per `windows.rs:61`):

| Proton | Exit | Secs | stdout | Result |
|---|---|---|---|---|
| ge11 | 0 | 8 | `{"schema_version":1,"operation_id":"6f1c0a52-0000-4000-8000-000000000007","prefix_generation":"6f1c0a52-0000-4000-8000-0000000000a7","result":{"state":"failed","reason":"probe"}}` | **fail** |
| exp | 0 | 8 | identical `{"state":"failed","reason":"probe"}` | **fail** |
| p9 | n/a | n/a | killed before output | **untested** |

- `failed/probe` (not `installer_failed`) means the MSI returned 0 and the functional probe that follows failed (`after_install`, `prerequisite/mod.rs:94`).
- The MSI did install files: `compat-ge11/pfx/drive_c/Program Files (x86)/AGEIA Technologies/v2.3.1 … v2.7.4/PhysXCore.dll`, plus `AGEIA PhysX Properties` shortcuts.
- After this install, the game gets past the PhysX dialog on ge11 (runs #2-#4 crash later, in the shader code).

## Step 8: SD card — **untested**

The game is on internal storage, and container mode already failed step 2, so the container-mode rerun was not reached.

## Step 9: Game Mode (optional) — **untested**

## Rulings the worknote must state

| Ruling | Evidence | State |
|---|---|---|
| D-PP3 container or bare | Container: step 2 exit 1, no stdout, on SLR_4 (ge11, exp, p11) and sniper (p9). Bare: step 2 passes on all four. | **Bare.** The container fails step 2, which is the packet's own "the ruling is bare" condition. |
| D-PP2 setup target | `proton run wineboot.exe`: exit 0 in 4-7 s on all four, `system.reg` present, DXVK `d3d9.dll` in syswow64. | **`wineboot.exe`** (tested). |
| D-PP4 default Proton order | Steps 1-2 are identical across ge11, exp, p9 and p11. Steps 3-6 pass on ge11 with the two changes. Without them, ge11 and exp crash identically. exp, p9 and p11 were not run with the changes. | **GE-Proton11-11 is the only tested working Proton.** No evidence ranks the others. |
| D-PP7 is PhysX needed | Without PhysX: the "Failed to initialize physics sub system … NxCreatePhysicsSDK(...) returned NULL" dialog. With the MSI installed: no PhysX dialog on ge11. | **Needed** (tested). The worker's result is `failed/probe`, not `probed`, so the packet's step-7 pass condition fails. |
| Extra variable the game needed | `DXVK_FORCE_WINDOWED=1` (the macOS launch's variable, `launch.md:197`) changed nothing: same black window and the same crash. The **native `d3dx9_40.dll` from the packaged DX9.0c cab** plus `WINEDLLOVERRIDES="d3dx9_40,xinput1_3=n"` reaches the login screen (step 3b). | **Native `d3dx9_40` is needed** (ge11, tested). `DXSETUP.exe` itself fails under Proton (exit 247). Whether `xinput1_3` native is needed was not split out. The window still needs a focus change before it draws. |
| Display arguments work | Args alone (with the game's default profile `windowedMode=false`, 1024x768): a decorated window of 1280x800 or 1280x785 that stays black until a taskbar touch. Args plus the launcher's windowed profile (`windowedMode=true`, `StartupResolution` 1280x720): `1280x688+0+37`, which draws without a touch. | **They work together with the profile write, not alone.** Linux Play must run the existing display-settings step (`launch.md:397`) on the Proton prefix's `steamuser` Documents, as macOS does for its prefix. On the Deck's 1280x800 screen the 720-high window is clamped to the work area (688). |
| Login and world entry | GE11-dx, both DLLs, `play.cimmeria.app:8081`: login, character select and world entry all pass. World entry took about 2-3 minutes, with `No player` from the minimap Lua until the player arrived. | **Pass** (tested). Load time not diagnosed. |

## Stop condition

Packet: "Stop and report if no mode passes step 2, or no Proton passes step 4." Bare mode passes step 2, and **GE-Proton11-11 passes step 4** once it has the native `d3dx9_40` and the windowed client profile. The stop condition does not fire. (In the first pass, before the DirectX test, it did fire; that history is kept above.)

Open items for the architect, recorded but not investigated:
- **How Linux Play gets the native `d3dx9_40`.** `DXSETUP.exe` exits 247 under Proton, so it means extracting from the packaged cab plus a `WINEDLLOVERRIDES` entry (or a native-DLL install into the prefix's `syswow64`). Whether `xinput1_3` native is also needed is unsplit. The shader crash at `SGW.exe!0x00EC6F9D` (dumps attached) is what Wine's builtin `d3dx9_40` produces.
- **The windowed profile write** (`launch.md:397`) must run for the Proton prefix. Without it the window stays black, with no audio, until a focus change.
- Steps 3-6 on exp, p9 and p11 with the two changes (the first-pass exp crash matches ge11's).
- World entry took about 2-3 minutes; no Windows baseline was taken.
- The prerequisite worker's `failed/probe` result.
- The container stdio failure.
- The missing `client-telemetry-build` artifact.
- The F-04 inventory correction.
- The spike used a debug launcher build with a test `XDG_DATA_HOME`, not the AppImage.
