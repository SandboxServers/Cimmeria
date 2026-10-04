# Native game launch contract

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-04
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
Extraction and prerequisite operations retain their headless environment.

`Graphics.rosetta_x87` optionally binds an independently pinned accelerator and
its adjacent companion library; it sets `ROSETTA_X87_PATH`. `None` uses stock
Rosetta without an extra accelerator. Supplying resources is not a license,
redistribution or performance claim. The managed Wine archive alone contains
neither the chosen D3D9 overlay nor the optional x87 accelerator. The integrator
must stage exact artifacts, record provenance and notices and validate them with
the real client. Successful PhysX/module prerequisites do not demonstrate device
creation, DLL patch hooks, login or world entry.

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

`LSUIElement` matters. Every Wine process started from the bundle shares it.
Wine's Mac driver promotes a process to a regular application only when it
shows a window
([`cocoa_app.m`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/winemac.drv/cocoa_app.m#L312));
without the key a windowless Wine process from the bundle also becomes a
foreground application with the same identifier.

### The desktop host stays on the stock loader

Every Wine desktop has one windowless `explorer.exe /desktop` process. Wine
starts it from the loader of the first process that asks for the desktop window
([`get_desktop_window`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/win32u/winstation.c#L790-L898)).
When that process is the game, the desktop host runs from the bundle and is a
second running application with the game's identifier. It also registers first.
The first game run with the opt-in showed exactly that, and a tool that asked
for the application by identifier or by bundle path timed out.

So before the launch worker starts, Play starts a keeper from the stock loader
(`engine/src/mac_wine/desktop_host.rs`), with the game's environment:

```text
<runtime>/bin/wine C:\windows\system32\cmd.exe /d /c
  "C:\windows\system32\rundll32.exe && echo CIMMERIA-DESKTOP-READY && pause"
```

- `rundll32` with no arguments creates its hidden owner window and exits.
  Creating a window makes Wine start the desktop host and wait for it. The
  desktop host therefore runs from the stock loader: name `wine`, no bundle
  identifier.
- `cmd /c` waits for `rundll32`, prints the ready line only if it succeeded,
  and then blocks in `pause`. Play waits up to 60 seconds for that line. The
  wait is part of preparation, so the operation is still Starting, and a
  cancellation ends the wait at once.
- The keeper holds the desktop open until the launch worker's observation
  ends. Wine closes a desktop one second after its last user leaves
  ([`remove_desktop_user`](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/server/winstation.c#L446-L456)),
  so nothing ever stops the desktop host. A running game keeps it; a game that
  never started does not.
- The keeper ends when its input closes. That is how Play releases it, and it
  also happens when the launcher exits or dies, so no Wine process is orphaned.
  A keeper that has not ended ten seconds after release is killed. It is a
  windowless `cmd`, never the game.
- If the keeper cannot start, ends early or does not report in time, Play waits
  for it to end and uses the stock loader, as if the opt-in were off. The staged
  loader is never used without a stock desktop host: two applications with one
  identifier are worse than none.

The keeper does not change which process the launcher supervises, the helper
protocol, the environment or the 30 FPS limit. Session start-up that used to
happen inside the launch worker's 60-second first reply now happens under the
keeper's own 60-second bound, before the worker starts.

What was observed on macOS 26.6.1 with the pinned runtime, never with the game:

| Case | Window owner as Launch Services sees it |
|---|---|
| Stock loader, Wine Notepad | name `wine`, no bundle identifier, no bundle |
| Foreground wrapper application starting a separate window-owning process (native fixture) | wrapper has the identifier and zero windows; the child owns the window and has no identifier |
| Staged bundle loader alone, 32-bit `cmd` starting 32-bit Notepad | Two running applications have the bundle identifier: `explorer.exe /desktop`, type UIElement, registered first, and Notepad, type Foreground, which owns the window |
| Stock keeper first, then the same staged launch | One running application has the bundle identifier: Notepad, type Foreground, one titled window and one accessibility window. `explorer.exe /desktop` is `wine`, no identifier, type BackgroundOnly |
| Same, keeper released while Notepad runs | The keeper ends; the desktop host and Notepad stay |
| Same, Notepad then closed | The desktop host ends within two seconds and `wineserver` within five; nothing is left |
| Keeper alone, then released | The same teardown; no application of type Foreground appeared at any point |
| `lsregister -f` on a fixture bundle under the user Library | `NSWorkspace` resolves the identifier to the bundle path; the same bundle under `/tmp` does not resolve |
| The game with the opt-in, before the keeper existed (2026-10-04) | `explorer.exe` UIElement and `SGW.exe` Foreground, both `app.cimmeria.stargate-worlds`; no x87 accelerator was staged |

The stock-keeper rows are the ignored engine test
`real_wine_window_owner_alone_carries_the_bundle_identity`. It uses the
production staging and keeper code with a bundle labelled as a fixture
(`app.cimmeria.fixture.wine-identity`), never the game's identifier. It needs
`CIMMERIA_WINE_RUNTIME_CLONE` set to a copy of the managed runtime directory,
uses a throwaway prefix and shows a Notepad window. It asserts that exactly one
running application has the identifier, that it is the 32-bit Notepad started
by a 32-bit parent, that it owns a window, that the desktop host has no
identifier, and that the Wine session ends by itself once Notepad closes and
the keeper is released. Without the `rundll32` step it fails with two
applications. It initialises the prefix first, because creating a prefix
starts the desktop host from the first loader whatever the keeper does.

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
- Whether the automation tool that timed out now binds the game. With the
  keeper only one running application has the identifier, but that was observed
  with Notepad, not with the game or the tool. Separately, while the active
  Space was another application's full-screen Space, the accessibility window
  lists of both the game and a fixture Notepad were empty, and the Notepad's
  list had one window once its Space was active. That may be a second cause.
- The desktop host and the game running from different paths of the same
  loader bytes, with the game itself: display mode changes, clipboard and input.
  Notepad showed no difference.

To roll back, start the launcher without the variable. No keeper is started
then. The staged bundle is inert; delete `wine-app-identity/` under the state root and run
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

## Patch policy per installation

Native code decides whether Play injects client patches. The renderer cannot.

| Installation | Patch artifact | Play |
|---|---|---|
| Fresh install | Verified | Injects patches |
| Fresh install | Absent or replaced | Unavailable |
| Adopted, patches reviewed on | Verified | Injects patches |
| Adopted, patches reviewed on | Absent or replaced | Unavailable |
| Adopted, patches reviewed off | Any | Starts without patches |

`DesktopState::resolve_play_resources` applies this table to the bundle, and
`admit_launch` refuses resources that disagree with it, so a host that skips
resolution cannot inject or drop the patch.

An adopted copy's imported settings are accepted only while every independent
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
