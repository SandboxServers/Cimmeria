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
consent never changes game telemetry. Imported legacy configuration must be
validated and deliberately mapped before admission; imports do not manufacture
signed content evidence or installation ownership. This packet uses the saved
installation's login servers and native bundle patch selection.

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
established Mac OpenGL compatibility workaround.

`Graphics.rosetta_x87` optionally binds an independently pinned accelerator and
its adjacent companion library; it sets `ROSETTA_X87_PATH`. `None` uses stock
Rosetta without an extra accelerator. Supplying resources is not a license,
redistribution or performance claim. The managed Wine archive alone contains
neither the chosen D3D9 overlay nor the optional x87 accelerator. The integrator
must stage exact artifacts, record provenance and notices and validate them with
the real client. Successful PhysX/module prerequisites do not demonstrate device
creation, DLL patch hooks, login or world entry.

## Validation and human launch checklist

Portable tests cover operation identity/revision, duplicate admission/dispatch,
pre-spawn cancellation, disk reopen and lost observations, hash replacement,
Mac prerequisite/graphics admission, and real host-process pipes reporting guest
start/early exit, host death, mismatched guest and malformed/oversized output.
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
