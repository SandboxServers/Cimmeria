# Native game launch contract

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-26
> **Companions:** [Desktop workspace](../README.md), [prerequisites](prerequisites.md), [runtime evidence](../../../../docs/analysis/playtests/2026-10-03-macos-wine/runtime-provisioning.md)

## Where the rest of the contract lives

| Page | Covers |
|---|---|
| [launch-mac.md](launch-mac.md) | macOS graphics and prerequisite boundaries, per-launch Wine and DXVK logs, the optional application identity |
| [launch-linux.md](launch-linux.md) | Linux (Proton): discovery, prefix, invocation, native DirectX, PhysX, display, environment, failure codes |
| [launch-display.md](launch-display.md) | Display settings before each Play, window sizes, the frame cap, the monitor per display setup |
| [launch-telemetry.md](launch-telemetry.md) | Opt-in game telemetry |

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
Every not-started launch the launcher detects itself records its failure code too: `verify_failed` is
`game_files_invalid`, `prepare_failed` is `launch_prepare_failed`,
`login_servers_refused` is `login_servers_invalid`, and a game process that could not
be created is `spawn_not_found`, `spawn_permission_denied`, `spawn_failed`, or on
macOS `rosetta_missing` when the OS reports a bad CPU type (EBADARCH). The launch
status returns it as `failure` (`null` when none is known), and the `launch_not_started`
event carries it as `failure_code`; a spawn failure also carries `os_error`, the raw OS
error number (for example 86 for a missing Rosetta). A launch the launch helper reports
as not started keeps its `reason` (from the helper's stage) and `win32`, and records a
failure code too: a refused request is `launch_resources_missing`; a failed process
creation is `spawn_not_found` (Win32 2 or 3), `spawn_permission_denied` (5, 225 or 226,
for example security software), `game_files_invalid` (193, a damaged SGW.exe) or
`spawn_failed` (anything else, or a helper too old to name its stage).
A Play refused before any operation exists logs `launch_refused` (warn) with no
operation number and `reason` `resources_unavailable` (no bundled Play resources) or
`resources_unresolved` (the resources could not be resolved for this copy).

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

Linux with Proton, recorded separately (see [Linux (Proton)](launch-linux.md#linux-proton)):

9. With no Proton installed, confirm Play is off and the copy names Proton
   Experimental and GE-Proton. Install one, reopen the launcher, and confirm
   Play is on and the Settings picker lists it.
10. First Play in a clean state root: confirm the prefix setup and the PhysX
    install finish, `proton.log` and `physx.log` exist under `logs/game/<op>/`,
    and `display_settings` reports `saved_options="seeded"` with the game in a
    1024x768 window.
11. Second Play: confirm setup and PhysX are skipped, the DirectX DLLs report
    `present`, and the login screen renders.
12. Remove the selected Proton, press the stale Play button, and record the
    refusal (today `corrupt_state`; FU-13 tracks `proton_missing`).

## Desktop Play control and resource binding

The desktop shell registers `launch_command` with only `inspect` and `play`.
Play accepts schema version, operation UUID, inspected operation revision and
saved installation UUID. Native code resolves all resources from its bundle;
the frontend cannot supply paths, executable names, environment or hashes.

| Compile-time SHA-256 variable | Fixed bundle resource |
|---|---|
| `CIMMERIA_LAUNCH_HELPER_SHA256` | `windows/cimmeria-launch-worker.exe` |
| `CIMMERIA_CLIENT_PATCHES_SHA256` | `windows/cimmeria_client_patches.dll` |
| `CIMMERIA_CLIENT_TELEMETRY_SHA256` | `windows/cimmeria_client_telemetry.dll` (optional, [opt-in](launch-telemetry.md#opt-in-game-telemetry)) |
| `CIMMERIA_D3D9_SHA256` | `graphics/d3d9.dll` (Mac required) |
| `CIMMERIA_ROSETTA_X87_SHA256` | `graphics/rosettax87` (optional pair) |
| `CIMMERIA_ROSETTA_X87_LIBRARY_SHA256` | `graphics/libRuntimeRosettax87` (optional pair) |

The launch helper is required for every installation. The patch artifact is
required only by the installations that inject it (next section); it is never
silently dropped to enable Play.
On Linux the same variables bind `windows/cimmeria-launch-worker.exe`,
`windows/cimmeria_client_patches.dll` and `windows/cimmeria_client_telemetry.dll`;
the graphics rows do not apply, and the Proton build is discovered at run
time rather than pinned ([Linux (Proton)](launch-linux.md#linux-proton)).
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
