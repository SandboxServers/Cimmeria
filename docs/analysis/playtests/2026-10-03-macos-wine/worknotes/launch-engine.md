# Native launch engine handoff

> **Type:** Reference (bounded implementation worknote)
> **Audience:** Launcher coordinator and reviewers
> **Last updated:** 2026-10-04
> **Companions:** [Launch contract](../../../../../crates/launcher/desktop/docs/launch.md), [delegation plan](../launcher-delegation-plan.md)

## Revision and ownership

Base: `0b10d869c869793ab506dbf9215ddb91714a244b`.
Owned implementation: `bfdffa7d320e0dc6f4baa76c25d02840a6e3d647`.
Reserved integration/head: `97558fd9311c4da79ef61cc3fdfd1239b7147e45`.
This worknote follows those commits without changing executable behavior.
Apply the chain once, in that order. No push, PR, merge, deployment or release
was performed by this worker. Shell/frontend ownership remains untouched.

Owned files are the new `engine/src/storage/launch/` modules/tests,
`runtime-probe/src/game_launch/`, its `cimmeria-launch-worker` binary,
`desktop/docs/launch.md` and the unique launcher-launch-engine memory folder.
All crate paths in this paragraph are under `crates/launcher/desktop/`.

The separate reserved-file integration commit contains exactly:

- `crates/launcher/desktop/Cargo.lock`
- `crates/launcher/desktop/runtime-probe/Cargo.toml`
- `crates/launcher/desktop/runtime-probe/src/lib.rs`
- `crates/launcher/desktop/engine/src/lib.rs`
- `crates/launcher/desktop/engine/src/storage/mod.rs`
- `crates/launcher/desktop/engine/src/mac_wine/mod.rs`
- `crates/launcher/desktop/engine/src/mac_wine/paths.rs`
- `crates/launcher/desktop/engine/src/mac_wine/prerequisites/mod.rs`
- `crates/launcher/desktop/engine/src/mac_wine/prerequisites/prefix.rs`

The four Wine visibility changes expose existing checked prefix resources,
path conversion and environment construction inside the crate. They do not
change extraction/prerequisite behavior. No existing Windows-launcher or
client-launch source was changed.

## Delivered behavior and shell contract

The existing operation journal owns Launch. A durable plan binds the installation,
current successful prerequisite generation and independently pinned native
resources. Duplicate admission never dispatches twice; duplicate dispatch fails.
Preparation retains the relevant owner/cache locks, validates mutation paths,
and reuses stock-case/login-server/ASLR preparation outside the UI thread.

The new x86 worker retains the original Windows handle through resume and exit,
using shared client-launch primitives. A bounded native supervisor reports
separate host/guest identifiers, actual guest start/exit, early exit and unknown
outcomes. It persists observations and retains the task if its view is dropped.
An interrupted launcher reopens as unknown, never stale-live or auto-replayed.

Integrate with these calls:

1. Resolve `launch::Artifact::open` from native bundle paths and build-pinned
   SHA-256 values. Construct `Resources { helper, client_patches, graphics }`.
2. Under the existing state mutex call `admit_launch(id, operation_revision,
   installation_id, resources)`; dispatch only when `Admission.dispatch` is true.
3. Call `launch::dispatch(state.clone(), id)` in the application Tokio runtime.
   Retain its `Worker`; expose only safe `Observation`, operation snapshot and
   flat mapped errors over IPC. Do not serialize `Plan` or resource paths to IPC.
4. Observe `Worker.observation` and `DesktopState::launch_observation()`. Keep the
   operation busy through game exit. Zero exit code means normal observed exit,
   not login. `ProcessStarted` never means world-ready.
5. Pre-spawn cancellation uses `Worker::request_cancel`. Running cancellation is
   rejected. Unknown remains reconciliation-required; no automatic fallback,
   replay, PID kill or launch-recovery API is supplied by this packet.

Login servers come from the saved installation intent. Patch selection comes
from native `Resources.client_patches`; None explicitly omits patches. No game
telemetry session/DLL is implicitly created from launcher-summary consent.
Legacy import must deliberately validate/map configuration; it must not invent
signed installation evidence. Documentation includes the exact human game UAT
checklist and required artifact/native build commands.

## Validation

All commands ran from the assigned worktree. Compiling Cargo commands used the
build lane and the pinned toolchain; no Windows artifact was built on Mac.

```bash
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine -p cimmeria-runtime-probe --lib
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine launch --lib
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine -p cimmeria-runtime-probe --all-targets -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
git diff --check
```

The combined library run passed 318 tests (engine 302, probe 16), with 15
pre-existing opt-in engine tests ignored. After adding the final observer-drop
regression, the focused launch run passed all nine tests; the full suite was not
rerun for that test-only addition. Final strict all-target clippy, formatting and
diff whitespace checks passed. One initial compile failed because a test tried
to clone the non-Clone command structure; the test now uses distinct fixtures.

The process tests launch real Python hosts and exercise bounded pipes, host death,
wrong guest identity, early exit, unknown injection outcomes and oversized output.
They do not execute Windows injection or Wine. Admission/persistence tests use
real temporary native state on disk and fixture prerequisite evidence. The final
observer-drop test proves retained task completion and durable known failure.
No frontend changed: JS REPL/visual UAT belongs to integration and was not run here.

## Remaining gates and next action

The coordinator owns shell registration, restricted Play IPC/Effect controls,
README/docs indexes and the ledger. The new worker still needs a Windows-native
x86 build, pinned staging and packaging binding; current existing archive and
prerequisite workers cannot substitute for it. Real Windows helper/injection UAT,
managed Wine SGW launch, D3D9 rendering, login, world entry and Black Market UAT
were not run. No visible game, WireGuard or live service was touched.

Mac requires a separately pinned native D3D9 overlay beside SGW. Optional x87
acceleration requires its pinned adjacent companion; absent acceleration means
stock Rosetta. Existing mismatched D3D9 files are refused. Exact artifact
provenance/notices and distribution approval remain packaging gates, separate
from successful prerequisite loading. Follow `desktop/docs/launch.md` for
resource staging and the complete human launch/login/world checklist.
