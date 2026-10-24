# Native game launch contract

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-23
> **Companions:** [Desktop workspace](../README.md), [prerequisites](prerequisites.md), [runtime evidence](../../../../docs/analysis/playtests/2026-10-03-macos-wine/runtime-provisioning.md)

## Admission and integration

The engine exposes `launch::{Artifact, Resources, Graphics, dispatch, Worker,
Observation}` and `DesktopState::{admit_launch, launch_plan,
launch_observation}`. Admission takes a new operation UUID, inspected operation
revision, saved installation UUID and native-resolved resources. JavaScript must
send only identity/revision; it must never supply executable paths, hashes,
DLLs or environment. An identical current UUID returns `dispatch: false`.
A different attempt while Launch owns the operation journal is busy. Preserve
the returned worker in the application host; disposal of a view does not cancel
its task. Observe its watch receiver and the durable native snapshot.

`Artifact::open(path, expected_sha256_hex)` requires an absolute canonical
ordinary file matching an independently trusted build digest. `Resources.helper`
is the x86 lifecycle worker. `client_patches: Some(artifact)` injects the approved
patch DLL before the first game thread runs; `None` explicitly starts without
that functionality. No telemetry DLL or session is created. Launcher summary
consent never changes game telemetry. Imports do not manufacture signed content
evidence or installation ownership. A fresh installation uses its saved login
servers and the bundle's patch artifact. A verified adopted copy uses the patch
setting and ordered login servers the user reviewed; see
[Patch policy per installation](#patch-policy-per-installation).

Admission verifies installed identity/content, resource hashes and platform
support. Mac additionally requires successful current prerequisite evidence and
an explicit graphics resource. Dispatch retains installation, prefix and runtime
cache locks, repeats resource checks, then reuses existing `client_setup::prepare`
(stock filename restoration, login server Lua and ASLR). The native task runs
that preparation off the UI thread. The engine makes no downloads during Play.

## Embedded Windows Play resources

A packaged Windows launcher embeds the x86 launch worker, client-patches DLL
and player telemetry DLL. The build rejects missing or mismatched staged bytes
against independent SHA-256 pins. Before binding launch resources, startup
verifies or repairs their ordinary files under
`<app-local-data>/payloads/<content-digest>/windows/`. This app-local cache gives
Windows process creation and DLL loading the real file paths they require;
players do not manage loose sidecars. Embedded builds never fall back to nearby
`windows/` files. Older loose files are left alone.

Publication and cleanup are serialized. Each running launcher holds a shared
lease for its payload version; cleanup skips active versions and preserves
unknown entries. Missing or corrupt regular files are repaired atomically;
unsafe paths, links and reparse points are refused. A setup failure produces a
native error and closed-code startup event rather than running unverified bytes.
Native resource hash checks still run when Play binds and dispatches resources.

macOS keeps helpers, DLLs and graphics in `Cimmeria Launcher.app` resources;
it does not use this Windows extraction cache. The game, WebView2 prerequisite
on Windows and downloaded Wine runtime on Mac remain separate. See the
[release stages](release.md#what-the-windows-job-builds) for clean-folder
`--verify-embedded-payload` verification without opening the UI or game.

## Setup visibility observation

Native `LaunchStatus` includes `setup_complete: bool` separately from its
optional `installation_id`. The latter is an admission identity and may be
withheld while a game launch or launcher update owns the operation boundary;
its absence alone does not mean installation setup is incomplete.

`setup_complete` requires verified installed content, available launch resources,
a prepared Wine runtime when the platform requires one, no required state reopen
and either idle state or a known launch that does not need reconciliation.
The frontend uses this observation only to hide completed-setup prompts.
It changes neither Play admission nor launcher-update gating and adds no Plan
field. Settings has a separate **Check Play status** action, with immediate
feedback and an adjacent inspection result, even when the main-page recheck is
hidden. Actual launch admission still performs the checks above.

## Helper and lifecycle

Build the helper **natively on Windows** through the lane:

```bash
bash tools/build-lane/lane.sh cargo build --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-runtime-probe --bin cimmeria-launch-worker \
  --target i686-pc-windows-msvc --release
```

Stage `cimmeria-launch-worker.exe` as a native-resolved bundle resource and pin its
SHA-256 independently in the shell build, just as for the prerequisite helper.
The `launcher-runtime-probe` Windows CI job builds this helper and the patch DLL,
logs their SHA-256 identities, and retains them in its `windows-runtime-probe`
artifact. Record the source revision and successful native build when staging;
the artifact is development validation, not a signed release package.
The new helper is not interchangeable with `sgw-start32.exe`: it accepts bounded
schema-1 JSON on stdin, emits bounded identity-bound JSON lines on stdout, and
stays alive until the game exits. It reuses `cimmeria-client-launch`'s existing
suspended-process creation, ordered injection and `resume_running` primitives.
The retained original Windows process handle avoids an OpenProcess race when
SGW exits immediately. No legacy launcher source behavior changes.

The helper starts the game under plain `C:\...` paths. A native launcher sends
canonical paths, which on Windows carry the verbatim `\\?\` prefix. Windows does
not resolve `..` under a verbatim working directory, and `SGW.exe` finds its
config as `..\SGWGame\Config\`, so with the prefix it quits at startup with
"Failed to find default engine .ini file". The helper strips the prefix from the
exe and the working directory with `cimmeria_client_launch::launch::strip_verbatim`,
the function the Windows launcher already uses. A Wine guest path is already
plain. Changing the helper changes its SHA-256, so restage it and rebuild the
shell with the new `CIMMERIA_LAUNCH_HELPER_SHA256`.

### Development resource staging

Native Windows [run 37214723035](https://github.com/SandboxServers/Cimmeria/actions/runs/37214723035)
at `d73eea0ed96e9dfbc91553101a3923300ab9e87c` passed the x86 helper's clippy/tests/build
and built the patch DLL. Its logged SHA-256 values were checked while staging:

| Resource | Kind for `tools/stage-helper.py` | SHA-256 |
|---|---|---|
| `cimmeria-launch-worker.exe` | `launch` | `ef3b74030f5d9e38c9ba5405897cb56f9a16bb1d3afe1613687c1845706b0c65` |
| `cimmeria_client_patches.dll` | `client-patches` | `92cd8dc484781f9c04a8c1e816e10399bb74d9d1dc6399e4c09a710a2dcf289d` |

Run the staging tool with the downloaded artifact path, `--kind`, `--sha256`
from the trusted native build, and `--revision` with that full source SHA. It
validates hash, PE32/x86 architecture and executable-versus-DLL role before
replacement, then prints the shell's compile-time identity variable. The
provenance receipt beside a resource is not a trust source. Three staging tests
passed, including refusal to replace an existing resource on a wrong hash or
wrong executable/DLL role. No actual Wine injection or rendering is proved by
these builds or staging checks. Later code changes require new native artifacts.

Observations distinguish:

- `preparing`: retained worker admitted; no host observed.
- `host_started`: native helper/Wine PID observed; no guest-start proof yet.
- `process_started`: helper confirms resume and reports a **Windows guest PID**.
- `process_exited`: matching guest handle signals exit, plus code and `early`
  (under ten seconds from receipt of `process_started`); helper also exits cleanly.
- `not_started`: native preparation/spawn failed, or helper confirms no game spawn.
- `cancelled`: cancellation was accepted before dispatch entered Running.
- `unknown`: lost/malformed observation, mismatched guest, injection/resume
  uncertainty, timeout, panic or persistence failure; requires reconciliation.

`process_started` means neither login nor renderer readiness. The operation stays
Running for the whole observed game lifetime. A zero exit code yields Succeeded;
other exit codes yield Failed. Both can be early exits; neither proves gameplay.
Host and guest PIDs are separate namespaces, even if their numbers happen to
match. Never signal a host PID using the guest ID.

Cancellation after dispatch enters Running is rejected. Use the game's own quit
flow. Closing the launcher does not promise to stop the game; reopening changes
an interrupted Launch to reconciliation-required and reports `unknown` rather
than replaying it or showing a stale PID as live. This packet deliberately has no
automatic launch recovery, PID kill or fallback launch after uncertainty. The
existing injection termination primitive is best effort; it cannot authorize a
second launch. A startup reply has a 60-second bound; an observed game has no
lifetime deadline. Observation loss leaves the operation gated.

A Play that fails before the game starts ends `failed` with a logged reason. When
dispatch fails while the operation is still Starting and no launch observation was
recorded yet, the operation first records a not-started launch result
(`not_started`) carrying `failure`, the failure code (for example `local_io`),
then goes to Failed, and `launch_not_started` (warn) carries the error's code as
`reason` (for example `io`) and the failure code as `failure_code`. An unconfirmed save
(`persistence_uncertain`), an operation that already left Starting, a launch that
already recorded an observation, or launch state that cannot be read
(`corrupt_state`) goes to reconciliation-required instead.
Every other not-started launch records its failure code too: `verify_failed` is
`game_files_invalid`, `prepare_failed` is `launch_prepare_failed`,
`login_servers_refused` is `login_servers_invalid`, and a game process that could not
be created is `spawn_not_found`, `spawn_permission_denied`, `spawn_failed`, or on
macOS `rosetta_missing` when the OS reports a bad CPU type (EBADARCH). The launch
status returns it as `failure` (`null` when none is known), and the `launch_not_started`
event carries it as `failure_code`; a spawn failure also carries `os_error`, the raw OS
error number (for example 86 for a missing Rosetta). A launch the helper reports as not
started (`reason` from its stage, with `win32`) has no failure code yet.
A Play refused before any operation exists logs `launch_refused` (warn) with no
operation number and `reason` `resources_unavailable` (no bundled Play resources) or
`resources_unresolved` (the resources could not be resolved for this copy).

## Mac graphics and prerequisite boundaries

Launch reopens the exact successful generation under `game-prefixes`, never the
headless extraction prefix. It verifies the cached managed runtime while holding
its cache lock. Wine's game environment enables its graphics driver and sets
`d3d9=n`; the pinned `Graphics.d3d9` is staged beside the actual `SGW.exe`.
An existing matching DLL is reused. An existing different DLL is refused rather
than silently overwritten. `CX_FWD_COMPAT_GL_CTX=1` preserves the experimentally
established Mac OpenGL compatibility workaround. Game preparation also sets
`VK_DRIVER_FILES` to `lib/vulkan/icd.d/MoltenVK_icd.json` inside that verified
runtime, so D9VK discovers the bundled MoltenVK surface implementation. A missing,
non-file or redirected descriptor fails preparation before any helper spawn.
This selection is native-owned; the webview cannot supply a driver path.
`DXVK_FORCE_WINDOWED=1` makes the DXVK `d3d9.dll` present windowed whatever the
client asks for: exclusive fullscreen would change the display mode, which
winemac.drv turns into a capture of every display (black screen, Cmd+Tab
freeze).
Extraction and prerequisite operations retain their headless environment.

`Graphics.rosetta_x87` optionally binds an independently pinned accelerator and
its adjacent companion library; it sets `ROSETTA_X87_PATH`. `None` uses stock
Rosetta without an extra accelerator. Supplying resources is not a license,
redistribution or performance claim. The managed Wine archive alone contains
neither the chosen D3D9 overlay nor the optional x87 accelerator. The integrator
must stage exact artifacts, record provenance and notices and validate them with
the real client. Successful PhysX/module prerequisites do not demonstrate device
creation, DLL patch hooks, login or world entry.

### Per-launch Wine and DXVK logs

Each Mac Play writes its own logs to `logs/game/<operation>/` under the launcher
state root (PD-28): `wine.log` is the Wine helper's stderr, at most 4 MiB
(`STDERR_CAP`; the rest is read and dropped, so Wine never blocks on a full
pipe), and DXVK writes its `*_d3d9.log` there through `DXVK_LOG_PATH` (the
folder's `Z:\` guest path) with `DXVK_LOG_LEVEL=info`. With logs on,
`WINEDEBUG` is `err+all,fixme-all,+display`: Wine's errors plus winemac.drv's
display code. Creating the folder prunes `logs/game/` to 5 operation folders:
the new one and the 4 newest others by folder mtime. If the folder can't be
created, Play starts without logs and with `WINEDEBUG=-all`, as before.

These logs contain local paths, so they stay on the computer: the debug log
upload reads only the `launcher.*.log` files in the app-data `logs/` folder (a
sibling of `state/`, not this one) and the game's own logs, never
`state/logs/game/`. Windows Play is unchanged; its helper's stderr is discarded.

## Optional Mac application identity for the game window

macOS automation that selects an application by name, bundle path or bundle
identifier cannot select a stock Wine game. The process that owns the game
window registers with Launch Services as `wine`, with no bundle identifier and
no bundle. A separate wrapper application does not change that: windows and
their accessibility tree belong to the process that created them, and a Wine
guest process is never the wrapper.

A Wine process takes its identity from the directory its loader really runs
from. The pinned runtime's loader sets `wineloader` to `wine` beside the
`ntdll.so` it loaded, after `realpath`
([`init_paths`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/ntdll/unix/loader.c#L387-L404)),
and every child process, including `SGW.exe`, executes that path
([`preloader_exec`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/ntdll/unix/loader.c#L443-L466)).
macOS gives a process a main bundle when that path is
`<name>.app/Contents/MacOS/`. Links do not count, because the loader resolves
them first.

Setting `CIMMERIA_WINE_APP_IDENTITY=1` in the launcher's own environment makes
Play stage `wine-app-identity/Stargate Worlds.app` under the launcher state root
and start the launch worker with the loader inside it. The webview cannot set
this. Without the variable Play starts `bin/wine` from the runtime exactly as
before and writes nothing.

| Bundle entry | Content |
|---|---|
| `Contents/Info.plist` | `CFBundleIdentifier` `app.cimmeria.stargate-worlds`, `CFBundleName` `Stargate Worlds`, `CFBundleExecutable` `wine`, `LSUIElement` true |
| `Contents/MacOS/<file>` | A copy of every regular file in the runtime's `lib/wine/x86_64-unix/`, including the loader and `ntdll.so`, each checked against the bytes it was copied from |
| `Contents/MacOS/libvulkan.1.dylib` | Link to the same runtime file the runtime's own relative link names |
| `Contents/MacOS/x86_64-unix` | Link to `.`, where Wine looks for a builtin's unix library |
| `Contents/MacOS/x86_64-windows`, `i386-windows` | Links to the runtime's PE directories |
| `share` | Link to the runtime's `share`, where Wine looks for its data |

Staging runs after the runtime tree digest is verified and while its cache lock
is held. The bundle is rebuilt on every Play in a staging directory and swapped
in; an old bundle is removed without following links, and a bundle path or
state directory that is a link is refused. The bundle is then registered with
`lsregister -f` so lookups by name or identifier can find it. If anything
fails before the worker starts, Play uses the stock loader and prints the
reason to the launcher's standard error. The environment, the 30 FPS
`DXVK_FRAME_RATE` limit, the helper protocol and host supervision are the same
on both paths: the host PID is still the real Wine process.

`LSUIElement` matters. Every Wine process of the session shares the bundle.
Wine's Mac driver promotes a process to a regular application only when it
shows a window
([`cocoa_app.m`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/winemac.drv/cocoa_app.m#L312));
without the key the windowless desktop host `explorer.exe` also becomes a
foreground application with the same identifier.

What was observed on macOS 26.6.1 with the pinned runtime, never with the game:

| Case | Window owner as Launch Services sees it |
|---|---|
| Stock loader, Wine Notepad | name `wine`, no bundle identifier, no bundle |
| Foreground wrapper application starting a separate window-owning process (native fixture) | wrapper has the identifier and zero windows; the child owns the window and has no identifier |
| Staged bundle loader, 32-bit `cmd` starting 32-bit Notepad | Notepad process is `Stargate Worlds`, `app.cimmeria.stargate-worlds`, type Foreground, one accessibility window; `explorer.exe` has the same identifier, type UIElement, no accessibility window |
| `lsregister -f` on a fixture bundle under the user Library | `NSWorkspace` resolves the identifier to the bundle path; the same bundle under `/tmp` does not resolve |

The last case is the ignored engine test
`real_wine_window_owner_carries_the_bundle_identity`. It needs
`CIMMERIA_WINE_RUNTIME_CLONE` set to a copy of the managed runtime directory,
uses a throwaway prefix, shows a Notepad window and stops only that prefix's
wineserver:

```bash
CIMMERIA_WINE_RUNTIME_CLONE=<copy of the runtime directory> \
  bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine \
  --lib app_identity -- --ignored
```

Still unproved, and the reason this is opt-in:

- The game itself under the staged loader: D9VK and MoltenVK rendering, patch
  injection, login. A failure after the worker starts has no fallback.
- The optional x87 accelerator. Wine executes `ROSETTA_X87_PATH` in place of the
  loader for 32-bit processes; whether the process that ends up owning the
  window still runs from the bundle has not been observed.
- Whether a given automation tool binds the window owner. Two running
  processes share the identifier and the windowless one registers first. A tool
  that takes the first match instead of the regular application gets no window.

To roll back, start the launcher without the variable. The staged bundle is
inert; delete `wine-app-identity/` under the state root and run
`lsregister -u` on the bundle path to drop the registration. Opening the bundle
directly runs the loader with no arguments, which prints its usage and exits
without creating a prefix.

## Validation and human launch checklist

Portable tests cover operation identity/revision, duplicate admission/dispatch,
pre-spawn cancellation, disk reopen and lost observations, hash replacement,
Mac prerequisite/graphics admission, and real host-process pipes reporting guest
start/early exit, host death, mismatched guest and malformed/oversized output.
The native preparation regression uses an inert PE32 fixture, reads installation
identity through its exclusively locked owner handle, runs real client setup and
checks that a competing owner remains excluded until the guard is dropped. On
Windows it exercises the dispatch preparation entry point; on other hosts it
exercises the same native preparation implementation without enabling dispatch.
The locked-handle read regression requires a native Windows run because Unix
locks do not forbid reads through another handle.
Mac-only environment regressions exercise the production game environment builder:
exact bundled descriptor selection, interactive overrides, headless policy
isolation, and refusal of missing, directory, symlink or redirected-ancestor
descriptors before spawn. Removing the driver selection makes the selection
regression fail. These inert filesystem fixtures do not execute Wine or validate
runtime archive contents, device creation, rendering, login or world entry.

Those host fixtures are Python protocol actors, not Wine or SGW. Windows x86
helper compilation/injection, managed Wine launch and rendering require separate
native validation. No frontend code changes in this packet; JS REPL and visual
UAT belong to the shell integration.

After packaging and wiring Play, record each result separately:

1. Verify the bundle contains the pinned x86 helper and chosen patch/graphics
   resources; prove a missing or replaced artifact leaves Play unavailable.
2. Complete installation and prerequisites, press Play once, then immediately
   press it again. Verify exactly one helper/game and visible starting feedback.
3. Confirm the real SGW window renders the login screen; record D3D9/x87 choice
   and distinguish any prerequisite, injection or device-creation failure.
4. Sign in with the tester's account, select/create a character and enter a world.
   Record observed login/world results independently of `process_started`.
5. Exercise movement and a patched client feature (for example Black Market)
   with the approved patch DLL. Record failures and skipped tests explicitly.
6. Quit the game normally, verify matching guest/host exit and Play re-enables.
   Repeat with an early failure and confirm its honest exit feedback.
7. Close/reopen the launcher while the game runs. Verify no duplicate launch,
   stale-liveness claim, Repair or uninstall while reconciliation is required.
8. Verify summary opt-in/out and imported settings do not implicitly enable game
   telemetry. Run the integration's JS logic and visual/keyboard UAT separately.

## Desktop Play control and resource binding

The desktop shell registers `launch_command` with only `inspect` and `play`.
Play accepts schema version, operation UUID, inspected operation revision and
saved installation UUID. Native code resolves all resources from its bundle;
the frontend cannot supply paths, executable names, environment or hashes.

| Compile-time SHA-256 variable | Fixed bundle resource |
|---|---|
| `CIMMERIA_LAUNCH_HELPER_SHA256` | `windows/cimmeria-launch-worker.exe` |
| `CIMMERIA_CLIENT_PATCHES_SHA256` | `windows/cimmeria_client_patches.dll` |
| `CIMMERIA_CLIENT_TELEMETRY_SHA256` | `windows/cimmeria_client_telemetry.dll` (optional, [opt-in](#opt-in-game-telemetry)) |
| `CIMMERIA_D3D9_SHA256` | `graphics/d3d9.dll` (Mac required) |
| `CIMMERIA_ROSETTA_X87_SHA256` | `graphics/rosettax87` (optional pair) |
| `CIMMERIA_ROSETTA_X87_LIBRARY_SHA256` | `graphics/libRuntimeRosettax87` (optional pair) |

The launch helper is required for every installation. The patch artifact is
required only by the installations that inject it (next section); it is never
silently dropped to enable Play.
The x87 pair must either have both pinned artifacts or be entirely absent; absent
acceleration uses stock Rosetta. Resource hashes are rechecked during inspection,
admission and dispatch. Missing/mismatched resources produce an unavailable Play
control; they do not manufacture installation or graphics readiness.

The application-scoped Effect workflow publishes immediate first-click feedback,
serializes commands and polls native state. A transport timeout never replays Play.
Inspection observes the retained native worker even after a tab change or lost
reply. Running keeps Play, directory changes, Repair and uninstall gated until
observed exit; restart with an unfinished attempt reports unknown and stays gated.
The display distinguishes preparation, process start, early/normal exit and
unknown. No message claims login or world entry from process start or zero exit.
Launcher summary consent remains independent and is not changed by Play.

## Display settings before each Play

The launcher owns the game's display mode (owner D3, D-PD16). Before each Play,
after the login servers are resolved and before the launch helper command is built,
it writes the saved display choice (`DisplayOptions::effective()`: always a
window on a Mac, the size clamped into range; windowed 1024x768 when nothing
was saved) into the client profile, `<Documents>/My Games/Firesky/SGWGame/`.
On Windows that is the player's Documents folder; on a Mac it is the one user
folder other than `Public` under the Wine prefix's `drive_c/users`, and it must
stay inside the prefix.

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
`missing`, `key_missing`, `unreadable` or `failed`). It is info when both files
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
`width`, `height` and `monitor_remembered`: true when this save stored a monitor
for the current setup, false when it removed the entry or stored nothing. No
field carries a rectangle or a setup key.

## Patch policy per installation

Native code decides whether Play injects client patches. The renderer cannot.

| Installation | Patch artifact | Play |
|---|---|---|
| Fresh or directly adopted, preference on | Verified | Injects patches |
| Fresh or directly adopted, preference on | Absent or replaced | Unavailable |
| Fresh or directly adopted, preference off | Any | Starts without patches |
| Legacy-bound adopted, patches reviewed on | Verified | Injects patches |
| Legacy-bound adopted, patches reviewed on | Absent or replaced | Unavailable |
| Legacy-bound adopted, patches reviewed off | Any | Starts without patches |

`DesktopState::resolve_play_resources` applies this table to the bundle, and
`admit_launch` refuses resources that disagree with it, so a host that skips
resolution cannot inject or drop the patch.

A non-null `adopted_login_servers` identifies the legacy imported/reviewed
binding. A direct-folder adoption follows the saved patch preference instead.
A legacy-bound adopted copy's imported settings are accepted only while every independent
record of the review agrees:

- the installed index's adoption provenance;
- the Published adoption record `adoption-<work>.json`;
- the admission checkpoint `adoption-plan-<work>.json`, which is why that file
  must outlive publication;
- the Adopt journal digest, while Adopt is still the latest operation;
- the retained `legacy-import.json`, re-derived from the exact legacy JSON.

When they disagree, Play and prerequisite setup are not offered and both
commands are refused. Inspect still answers, the copy can still be uninstalled,
and the bundle is not reported as missing resources. The signed launcher minimum
belongs to the owner's release and is checked first, so it is reported whether or
not the imported settings verify.

The Play view has no separate message for refused imported settings yet. It
shows the general "finish installation and compatibility checks" line.

## Opt-in game telemetry

The launch helper injects `cimmeria-client-telemetry` after the client patches
when the player has opted in. It is off by default. The DLL observes the
running game and uploads events and engine logs to the server; what it
captures is described in [client-telemetry.md](../../../../docs/architecture/client-telemetry.md).

**The choice is its own record.** `game-telemetry.json` in the state directory
holds `opted_in` and, from the first opt-in, an install identity. The
`game_telemetry_command` (`inspect`, `set`) reads and saves it. Launcher-summary
consent (`preferences.json`) is a different choice and never turns it on, and
saving one does not touch the other. Opting out keeps the identity, so a later
opt-in is the same install to the server.

**The DLL is an optional bundle resource.** A build that pins no
`CIMMERIA_CLIENT_TELEMETRY_SHA256`, or whose DLL no longer matches, cannot offer
game telemetry: `set` with `opted_in: true` fails with `platform_unavailable`
and saves nothing. Play is unaffected. A `lab-bridge` build is refused three
times: by `tools/stage-helper.py --kind client-telemetry`, when the shell
resolves its bundle, and at admission. It is built natively by the
`launcher runtime probe` workflow, which logs its SHA-256 next to the patch
DLL's.

**What Play does when it is on:**

1. `resolve_play_resources` keeps the DLL in the launch resources. Admission
   refuses it for a player who is not opted in, even from a host that skipped
   resolution.
2. The worker asks the installation's first login server for a session:
   `POST <login server>/api/auth/dev-session`, with the identity and the tags
   `desktop-launcher`, the host OS and `wine` or `native`. The wait is bounded
   to 8 seconds. Addresses follow the Windows launcher's rule
   (`telemetry/endpoint.rs`, compiled into the engine): https anywhere, plain
   http only to this machine or to a login server's own host and port. The rule
   is applied to the upload address the server returns as well.
3. The session marker is written to `Working/Binaries/sessions/current-session.json`,
   the file the DLL reads at boot. It is the same shape the Windows launcher
   writes (`telemetry/session.rs`, compiled into the engine).
4. The DLL is added to the helper's request after the patch DLL. The helper
   already accepts two DLLs; it is unchanged.

No session, a refused address or an unwritable marker means the DLL is left
out and the game starts as it would with telemetry off. `game-telemetry-status.json`
records which of those happened for the last launch (`attached`,
`session_unavailable`, `endpoint_refused`, `session_not_written`), and the
main-page Game diagnostics status shows it. `attached` means the DLL was on the injection list with a marker on
disk. It does not claim the DLL loaded or that an upload arrived; the DLL's own
`cimmeria-client-telemetry.log` beside `SGW.exe` and the server are the evidence
for those.

**The launcher's half of the session** (`game_telemetry/session_loop/`). With a
session minted, the launcher also uploads, as the Windows launcher's runner
does, through its own queue (`telemetry-queue.jsonl` in the state directory):

- Before the helper spawns the game, every tailed file already on disk is taken
  in at its end, so an earlier session's lines never go again. A file created
  later is read from its first line. An older file that only now matches (a log
  rotated by rename) starts at its end, and a renamed tracked file keeps its
  offset. A tick reads at most 1 MiB of each file and carries on next tick.
- On the server's flush cadence it sends each new complete line of
  `Binaries/SGWDebugLog*` (as `debug_log`) and every `.log` and `.txt` under
  `Binaries/sessions` at any depth (as `client_log`; the files the bundle takes,
  case ignored), with the Windows launcher's fields, then flushes.
- One session at a time: the queue is shared, so a new Play waits for the
  previous session's bundle and last flush before it queues anything (at most
  35 seconds, after which the previous session is stopped and its unsent
  events go with the new one). Without the wait, the old session's last flush
  would send the new session's first events under the old session id.
- The supervisor's exit observation ends it: a `session_meta` event of kind
  `game_exited` with `exit_code`, `pid` (the game's), `host_pid` (the helper's)
  and `early`, then the end-of-session bundle, then a last flush bounded to 30
  seconds. It runs after the launch result is recorded and never delays it. An
  ending the supervisor did not observe sends no `game_exited`.
- Closing the launcher while the game runs ends the session there. What was
  queued stays on disk and goes with the next session's first flush. Offsets
  are not saved: a restarted launcher does not resume the session, and the next
  Play's baseline means nothing is sent twice.
- Opting out while the game runs stops it at once: the transport checks the
  choice before and after each queue write and send, nothing more is sent, no
  bundle goes, and no queue is left behind. A chunk already being posted at
  that moment completes.

A plan that does not carry the DLL serializes without a `client_telemetry` key,
so plans written before this field existed keep their digests.

**Capture switches.** The game's environment is built from scratch, so the
DLL's developer switches are copied on purpose, and only when telemetry is
attached: `CIMMERIA_CLIENT_CAPTURE`, `CIMMERIA_CLIENT_HOOKS_ENABLE` and
`CIMMERIA_CLIENT_HOOKS_DISABLE`, each a short comma-separated list. Set them in
the launcher's own environment, for example on macOS:

```bash
open --env CIMMERIA_CLIENT_CAPTURE=unfilter,firehose "<launcher bundle>"
```

**Launcher events.** For an opted-in player the launcher also sends the
Windows launcher's own events, built by the same core code:
`client.launcher.install_result` once per Install, Repair or game Update run,
and the client-patches DLL's `client.patches.boot` and `client.patches.counts`
for each Play. The boot event's `injection` follows the policy Play used, so
an adopted copy whose reviewed choice is patches off reports `opted_out`.
When Play's own telemetry session runs, the client-patches events go through
it: the boot verdict on the first tick after the DLL logs it, the counts at
most once a minute while they move, and whatever is still owed (the verdict
if none was logged, the final counts) after the last tail tick and before
`game_exited`. Otherwise, and always for the install result, they go to the
disk queue and ship with the next session's first flush. Either way they are
reported only for a game that ran.

**Adoption steps.** The shell's subscriber (`shell/src/telemetry.rs`) writes a
rolling `logs/launcher.<date>.log`, 7 kept, with the home folder written as
`~`. The `adoption` forward layer hands each step of **Use an existing game
folder** to a bounded channel that drops when full, and
`launcher_session::LauncherSender` sends it as a `client.launcher.adoption`
event in a `launcher` session. That session is minted on the first step, lasts
15 minutes and is re-minted when it runs out; events are posted at once and
dropped silently when offline. The six steps are C10 of
`docs/analysis/launcher-adopt-ux/work-packets-wave2.md`.
Nothing leaves without the game-diagnostics opt-in, in a build with the module.
`client.launcher.install_result` is unchanged and still waits for Play.

**Other launcher events.** The shell writes its own events as tracing events, one
target per subsystem: `launcher.startup`, `launcher.crash`, `launcher.update`,
`launcher.game_update`, `launcher.install`, `launcher.repair`, `launcher.launch`,
`launcher.settings`, `launcher.ui` and `launcher.telemetry` are emitted;
`launcher.login` has no emitter yet. Each event has a snake_case `event` name:
`<op>_started` and `<op>_finished` are info, `<op>_failed` is warn. A refusal
(`launch_refused`) and `launch_not_started` are warn too. Wire rows use the same name with
a `client.` prefix (`client.launcher.install`). The full subsystem table is in
[observability.md](../../../../docs/architecture/observability.md). `launcher.launch` carries `launch_refused`, `launch_started`, `launch_not_started`,
`launch_failed`, `game_started`, `game_exited`, `launch_finished` and
`display_settings` ([Display settings before each Play](#display-settings-before-each-play)), each with the
operation number (except a `launch_refused` before any operation, see above); refusals, failures and not-started launches add a closed `reason`.
`launch_started` also carries `adoption_check` (`light`, `full`, or `none` for an
install not adopted from a folder) and `patches_incomplete` (how many patches the
copy's `launcher-installed.json` lists in `incomplete_patches`). When the installed
copy cannot be read for those tags, the launcher logs a WARN `launch_tags_unavailable`
(the event name only, no error text) and the launch goes on untagged (`none`, 0).
Two events on `launcher.telemetry` are named here: `pipeline_recovered`, one row when the
upload pipeline comes back after failures, and `game_session_mint_failed`, when
a game session could not be minted and Play went on with no game diagnostics.
Their fields are not listed in this file.

**Not covered:**

- An install result is queued only by the journal commit that ends the run.
  A run ended any other way sends none: one finished by reconciliation after
  a restart, one whose launcher stopped between its terminal commit and the
  queue append, and a Repair or Update preparation that finished after its
  owner went away (left for reconciliation). Nothing
  records which runs were reported, so reporting them later could not be
  made exactly-once.
- An adopted copy still launches without game telemetry. Its imported choice
  is reported as unavailable, as before.
- The session token is minted once per Play and is not refreshed. A session
  that outlives the token stops uploading.
- Lines the game writes after the launcher closed are not streamed, and that
  session's bundle is not sent (see below).

## Headless integration checks

Run shell tests through the build lane, then use the emitted shell test binary for
`LAUNCH_UAT_BINARY` when running `npm run uat:launch` from
`crates/launcher/desktop/frontend`. The ignored `launch_uat_bridge` fixture uses
real native admission, operation journals and filesystem persistence with inert
installed content and prerequisite evidence. Its explicit lifecycle observations
stand in for Windows/Wine execution. JavaScript exercises the real Effect decoder,
workflow and DOM controls through that fixture, including duplicate clicks,
early exit, lost reply, reopened unknown state and unchanged consent.

These checks do not validate real injection, Wine/Windows execution, D3D9/x87,
login, world entry or packaged visual/keyboard behavior. Keep the human checklist
above as a separate gate.

## Local lab development tools

The runtime-probe workflow separately builds Windows-native `cimmeria-lab`,
`sgw-start32` and the lab-bridge telemetry DLL for supervised game UAT. They are
retained as `windows-local-lab-tools` and never enter the player app bundle.
Follow the [lab rulebook](../../../../docs/guides/live-research-lab.md). A local
UAT session must use a fresh loopback bridge token and an empty upload endpoint;
no live mint, telemetry upload or server-side lab access is part of this campaign.
Native Windows compilation does not establish Wine bridge compatibility.

### Minimum-version status

Idle Play inspection verifies the retained signed release and reports
`launcher_update_required`. When it blocks, no installation capability is
returned and repeated status polls keep Play disabled with update guidance.
This read does not change consent or the operation journal. Existing native
admission checks remain authoritative. Development-build exemptions retain the
legacy policy; this status does not imply updater download or replacement.

## Background observation and the Play button

A status read preserves the last confirmed Play capability while it is pending;
it does not show the starting state or dim the button. Clicking Play during a
read queues one intent through the Effect semaphore, refreshes native state, then
admits it only if still eligible. Duplicate clicks are suppressed immediately.
Changed native capability or a failed read still disables Play. A failed read
stops automatic polling until explicit recheck succeeds, so a macOS folder-access
prompt cannot accumulate an unbounded sequence of native reads.

The held-read regression covers stable enabled/text/busy state and exactly one
click admitted after the read. Native-persistence Play UAT additionally covers
lost replies, reopening and minimum-launcher rejection. This does not establish
actual game startup, login or world entry.

The same steady-state rule applies to **Recheck Play status**: background reads
do not toggle its disabled style. Read deduplication remains internal; launching
a game still disables both controls until the native result is observed.

## Applied content identity

Native logs emit `launcher.install` / `applied_content_identity` after
successful content publication (`phase=applied`) and successful Play
admission (`phase=play_admitted`). Interrupted Install, Update and Repair
recovery also emit the applied identity after successful terminal journal
reconciliation; partial or refused recovery emits none. `manifest_sha256` hashes the retained,
reverified installed manifest; it does not describe a newly fetched offer.
`overlay_ids` lists fully applied signed-manifest patches rooted at
`sgw_game`, checked against the live `launcher-installed.json` ledger via
`has_applied_patch` and its `@sgw_game` keys. `incomplete_overlay_ids` lists
partially applied overlays separately. Failed or staged-only content emits
no applied identity. No file paths or secrets are emitted. Play admission
is not proof the game process loaded an overlay. Identity logging is best
effort and cannot fail an already committed installation.
