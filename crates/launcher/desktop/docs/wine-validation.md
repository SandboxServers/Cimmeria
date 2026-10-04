# Wine runtime and helper validation

> **Type:** Reference
> **Audience:** Native launcher contributors and supervised testers
> **Last updated:** 2026-10-04
> **Companions:** [Desktop launcher](../README.md), [runtime provisioning evidence](../../../../docs/analysis/playtests/2026-10-03-macos-wine/runtime-provisioning.md), [implementation ledger](../../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md)

Paths in this reference are relative to `crates/launcher/desktop/` unless stated
otherwise. Shell commands run from the repository root. Mac content installation
is conditional on verified packaged-helper binding; it is not a release or
game-readiness claim.
Historical packet results live in the implementation ledger.

## Archive worker boundary

`engine/src/archive_worker/` defines a one-request extraction contract and
portable stdio mechanics. `cimmeria-archive-worker` is operational only when
built natively on Windows; non-Windows entry points exit with an error. The
Mac shell does not yet invoke it; the retained native Wine worker is described
in the desktop README. The experimental Wine
adapter and its validation boundaries are described below.

Input is NDJSON, bounded to 8,192 bytes per frame including the newline. An
extraction request carries `schema_version: 1`, `operation_id` (UUID), absolute
`archive` and `destination` paths, and `sha256`. The native parent supplies the
hash from authenticated content and owns the archive's directory. The helper
verifies the hash before creating a destination that must not already exist. It
never overlays an existing installation. On Windows, the verification handle
denies write/delete sharing and stays open while the extractor reopens the path.
This guards Windows file mutation/replacement; enforcement against native-host
writes under Wine remains unverified. It is not an adversarial filesystem
sandbox.

Stdin remains open as the ownership channel. A control frame with the same
schema/operation ID and `cancel: true` requests cooperative cancellation. EOF or
malformed controls also cancel; valid frames for a different ID are ignored.
Cancellation checkpoints can be coarse (between ZIP entries, patchset operations
or archive stages). Completion can win a late cancellation; no immediate abort
or rollback is promised. Partial output stays for parent reconciliation.

Progress is latest-value only, emitted at most every 100 ms into a one-slot
stdout queue. Replies carry schema/version, operation ID and either progress
counts or a `finished` error code; no filenames or raw errors are emitted. After
the blocking extractor returns, terminal enqueue/flush has a two-second budget.
Lost/blocked output yields a nonzero exit; it does not prove no files were
written. The executable exits after the result, including when a detached stdio
thread remains blocked. The parent must drain stdout, keep stdin open, impose
startup/operation deadlines, own staging and reconcile uncertain exits.

The helper uses the existing Windows FDI chain implementation. A Windows process
test checks real stdio, Unicode/space paths, operation identity, exactly one
terminal result and hash failure before output. A Windows sharing test guards
write/rename denial. Portable tests exercise the actual control reader and
injected blocked/broken writers. They do not establish Wine or real-cabinet
compatibility. Native Windows CI retains a debug helper artifact for seven days
for supervised validation; this is not a published release.

## Native helper supervision

`engine/src/helper_supervisor/` starts a native-selected executable with
explicit arguments, working directory and environment; inherited environment
variables are cleared. No deserialized or webview command accepts this
configuration. The caller must persist operation admission first. After
spawning, `record_host` must durably record the host PID before request
dispatch, or refuse dispatch. The owned wrapper supplies this journal callback,
as described under extraction identity below.

Protocol frames are limited to 8 KiB and event/progress channels retain one
observation. Default deadlines are five seconds for request writes, thirty
minutes for the operation, thirty seconds for cooperative cancellation and three
seconds for terminal/exit completion. Cancellation writes share the remaining
operation/cancellation deadline; cleanup cannot grant another cancellation
budget. Synchronous spawning and the persistence callback are not time-bounded.

Success requires a matching terminal event, successful process exit and stdout
EOF. Silence, malformed/duplicate events, identity mismatch, contradictory exit
and deadlines require reconciliation. These outcomes describe extraction, not
installation or game readiness. Cleanup targets only the owned child; stopping a
Wine host does not establish that its guest processes have exited. The native
operation scope must outlive the webview and retain recovery state on
uncertainty.

A custom real-stdio harness exercises thirteen scenarios, including failed
ownership before dispatch, cancellation, an unresponsive child, progress
flooding, supervisor abort and durable checkpoints. The abort case verifies
direct-child OS-lock release. A separate blocked-pipe regression checks
cancellation-write deadlines. Windows-only integration connects the supervisor
to the actual archive worker. Test-harness success is not Wine guest-death proof.

## Managed Mac runtime cache

The macOS-only `engine/src/mac_runtime/` prepares a native-selected, OS-locked
cache for the pinned Wine runtime. Its fixed HTTPS download uses a ten-second
connect timeout, a 300-second request timeout and at most five redirects.
Streamed size checks and exact archive size/SHA-256 validation precede
extraction. A temporary archive and staging directory isolate unpublished work.

Extraction runs the fixed `/usr/bin/tar` command with an empty environment on a
blocking task. Before publication, a pinned canonical tree digest checks every
relative path, entry type, executable permission bits and file/link data. The
expected digest was derived from the authenticated archive. Reuse repeats the
full tree check; a damaged existing cache fails closed and is preserved.
Publication renames the verified tree and syncs the cache root. These checks do
not prove power-loss durability or runtime compatibility.

Download waits observe cancellation. Tar extraction is not immediately
interruptible: cancellation is checked before publication. The blocking task
owns the lock, archive and staging directory, so dropping an observer cannot
remove staging while tar is writing. The experimental adapter and retained native
worker use this cache; shell resource binding and game prerequisites remain open.

The pinned-archive extraction/full-tree smoke passed when explicitly invoked
with `CIMMERIA_RUNTIME_ARCHIVE`; its normal suite entry remains ignored without
that asset. It did not execute Wine. Ordinary tests cover tree changes, cache
locking, invalid archives, cancellation, damaged-cache retention and transport
bounds/stalls. Licensing/distribution and real game gates remain open.

## Extraction identity and helper journal

Native admission now takes `AdmissionRequest`, including immutable extraction
backend identity. `Native` remains the default and is omitted when serializing
intent, preserving existing schema-1 intent digests. `Wine` binds runtime and
helper SHA-256 values into the intent. Reusing an operation ID with a different
backend conflicts. Native dispatch, resume and recovery reject Wine intents,
even when no output exists; restart never silently substitutes native
extraction.

`storage/helper_journal/` persists launch intent, host-started and finished
checkpoints with operation ID, attempt ID and intent digest. Host PID is
retained independently of the phase, including an uncertain finish. It is
diagnostic identity evidence, not authority to kill a reused PID or proof of
Wine guest termination. An existing helper record blocks automatic redispatch.

`helper_supervisor::run_owned` verifies the request's seed hash against cached
signed evidence, persists launch intent before spawning, records the host
through the supervisor callback before request delivery, and persists the
observed result before returning it. Callers must map journal errors to
reconciliation. They must still validate runtime/helper artifacts and
host-to-guest paths and keep this future in native operation scope. The
retained Wine worker uses this wrapper; production shell resource binding and
restart/prefix recovery remain unconnected.

The real-stdio owned-wrapper fixture checks that the child sees `HostStarted`
before extraction input and that completion is persisted. Journal tests preserve
PID evidence across uncertain finish. These fixtures establish checkpoint
ordering, not Wine execution or game readiness.

## Experimental headless Wine seed adapter

The macOS-only `engine/src/mac_wine/` implements the seed-extractor interface
using the pinned runtime and a native-selected Windows helper. Preparation
checks intent-bound runtime/helper hashes, probes Rosetta and creates a new
private operation prefix with a locked ownership marker. Existing prefixes are
not silently reused. The prefix creates `drive_c` and maps `dosdevices/c:` to
`../drive_c`; native paths map through its explicit `Z:` drive. Invalid Windows
path spellings are rejected.

The command uses an explicit environment that disables graphics drivers and
unneeded bootstrap components. This is a headless extraction experiment, not a
game-launch environment. Extraction requires the operation's cache/staging paths
and uses durable helper supervision. Normal completion attempts prefix-scoped
`wineserver -k` followed by `-w`; cleanup failure remains uncertain. This does
not establish cleanup after every possible interruption or guest lifecycle
recovery.

The initial ZIP smoke timed out with an uncertain result. A minimal diagnostic
identified a missing C-drive/system32 path; adding the explicit C-drive
directory and mapping produced a successful helper terminal and extracted
fixture. The original managed-runtime Windows-CI-helper ZIP smoke then passed in
19.574 seconds. The C-drive regression was observed failing before the fix. Six
ordinary adapter tests include concurrent ownership coverage. The latest ordinary run passed 201 library tests with six ignored entries;
strict all-target engine clippy passed. External-asset smokes run separately. Process inspection found no remaining test
Wineboot/wineserver processes. No frontend changed, so frontend UAT was not
rerun.

The adapter is connected to the retained native `dispatch_wine` worker, but not
to shell resource selection through the verified binding described below. The smoke proves fixture ZIP extraction only.
The original client RAR/chained-cabinet smoke subsequently passed, as recorded
below. Prerequisites, game launch and distribution clearance remain unproven.

## Supervised real-archive smoke

`mac_wine/smoke.rs` contains ignored integration tests. The real-client test
requires an existing archive, the signed release manifest plus adjacent
`manifest.json.sig`, and a Windows-native helper executable. Supply the matching
`LAUNCHER_MANIFEST_PUBKEY_HEX` when compiling; the development fixture key does
not authenticate a production manifest. Run from the repository root on macOS:

```bash
# Set these variables to the supervised test inputs before invoking the lane:
# SGW_CLIENT_RAR: absolute path to the original client RAR
# CIMMERIA_SMOKE_MANIFEST: absolute path to manifest.json
# CIMMERIA_WINE_HELPER: absolute path to the Windows-native helper executable
# LAUNCHER_MANIFEST_PUBKEY_HEX: manifest verification key used at compile time
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine --lib \
  mac_wine::smoke::original_client_rar_under_managed_wine \
  -- --exact --ignored --nocapture
```

The test authenticates the manifest and verifies source size/hash before copying
input into its owned temporary cache. It runs extraction under the pinned Wine
runtime and checks an `MZ` executable, an `SGWGame` directory, removal of temporary
cabinet expansion output and a completed helper journal. The private temporary
tree is removed after the test. The recorded passing run is detailed below.
This test does not establish patch application, game startup or Play readiness.


## Real client archive evidence — 2026-10-04

`original_client_rar_under_managed_wine` passed in 299.64 seconds (301.146 seconds
for the lane invocation), using the Windows debug helper from `17b949f4c`.
The source was 4,135,724,034 bytes with SHA-256
`7ba97ed2cb94f86edaba17a513824ae08d0f19920583d2a3da242faf1e034f07`.
The signed release manifest authenticated under public key
`7d78f576e86c2a3993a35080538b122cf0d40bf7010a39e96483e7feac7d004e`.

The run reached 5,983/5,983 cabinet-extraction progress, found the expected
`MZ` SGW executable and `SGWGame` directory, confirmed `.tmp-unpack` was absent,
and persisted a completed helper result. Its private temporary tree was removed
afterward. Final inspection found no remaining Wine/helper processes after the
successful smoke and cancellation of a redundant diagnostic. This establishes
original RAR/chained-cabinet extraction through the
managed Wine/helper path. It does not establish patch application, prerequisites,
game launch, login or gameplay. The debug-helper duration is not a release
performance benchmark. Packaged-helper binding is a separate gate described below.


The retained-worker fixture additionally passed Wine seed extraction followed by
a native ZIP patch, client preparation and receipt publication after observer
disposal (22.596 seconds). Both strengthened ignored checks passed in a 19.090-second run, including
duplicate rejection and cancellation before cache/prefix/helper/network work.
Strict engine clippy and thirteen shell tests passed; final engine suite passed
204 tests with eight ignored entries. This is distinct from the original-RAR
extraction smoke:
no original-client patch/prerequisite/gameplay result is implied.

The adapter checks cancellation before helper dispatch and preserves a supervisor
`NotStarted(Cancelled)` outcome as cancellation. Seven adapter tests passed; the
pre-cancel fixture proves no helper journal, output or attempt to execute its
nonexistent helper. That early exit also avoids starting prefix cleanup commands.


## Packaged helper staging and Mac build

`tools/stage-helper.py` requires a Windows artifact, an independently supplied
64-digit expected SHA-256 and its full 40-digit source commit. It checks bounded
regular-file input, hash and AMD64 PE32+ headers before replacing the ignored
resource. `helper-build.json` records provenance; it is not a trust source.
The shell embeds the expected hash at compile time and resolves a fixed resource
path, verifying again before release fetching/admission.

For the tested debug helper, Windows CI run `37189445603` built commit
`17b949f4c8a5e37f9840177739ec8f6f4e85a9e4`; artifact SHA-256 is
`5b51149c6a6c0a5f4403b3344433015f2db140bb45fe14e4425c0437b1605406`.
From the repository root, set `CIMMERIA_WINE_HELPER` to the downloaded executable
and run on macOS (the Tauri CLI must already be installed):

```bash
export CIMMERIA_WINDOWS_HELPER_SHA256=5b51149c6a6c0a5f4403b3344433015f2db140bb45fe14e4425c0437b1605406
export LAUNCHER_MANIFEST_PUBKEY_HEX=7d78f576e86c2a3993a35080538b122cf0d40bf7010a39e96483e7feac7d004e
python3 crates/launcher/desktop/tools/stage-helper.py "$CIMMERIA_WINE_HELPER" \
  --sha256 "$CIMMERIA_WINDOWS_HELPER_SHA256" \
  --revision 17b949f4c8a5e37f9840177739ec8f6f4e85a9e4
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run build --prefix crates/launcher/desktop/frontend
bash tools/build-lane/lane.sh bash crates/launcher/desktop/bundle-macos-dev.sh
```

The output is under `target/desktop/debug/bundle/macos/`. Both identity variables
are build inputs, not runtime configuration. Ordinary development-key fixture
tests should run without the release-manifest key override. For a different
helper revision, obtain its expected digest from that trusted build and update
both staging and compilation; do not derive trust from the staged receipt.

The development `.app` was built and its embedded helper matched the expected
hash. The app was not opened. A staged-resource resolver/Wine-admission/cancel
fixture passed in 1.961 seconds, but does not prove actual Tauri-window routing
or self-contained startup. Mac content installation is supported only when this
binding verifies. Wine recovery/resume and game prerequisites/launch remain open.

Final validation passed 205 engine tests (eight ignored), 15 shell tests (one
ignored), strict engine/shell clippy and final development bundling. The resource
smoke passed separately. The packaged helper at
`Contents/Resources/windows/cimmeria-archive-worker.exe` matched the expected hash.
Staging now sets the public helper resource to mode `0644` for multi-user reads;
its regression test passed. The final `.app` resource's SHA256 and mode `0644`
were verified. The resource-resolver admission/cancellation smoke also passed
against that actual app resource directory in 1.872 seconds, without opening a
window. This does not establish final startup or gameplay readiness.
The app was not opened.
