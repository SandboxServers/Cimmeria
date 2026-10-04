# Prerequisite evidence and runtime probe

Prerequisite retention is documented in [Wine validation](wine-validation.md#inert-prerequisite-retention).
Retained installers are inert data; their presence does not mean installation or
readiness. The `runtime-probe` crate belongs to the standalone desktop workspace.
Its executable operates only in native Windows x86 builds; other targets exit
unsupported. It is not yet integrated with the managed shell. The original-client module-only
smoke below predates the experimental SDK call.

## Request and report

The one-shot helper reads stdin to EOF, accepting at most 8 KiB of JSON:

```json
{"schema_version":1,"game_binaries":"C:\\Games\\SGW\\Working\\Binaries"}
```

Unknown fields, unsupported schemas, relative paths and parent traversal are
rejected. Native code canonicalizes the directory and requires `SGW.exe`. It
activates resource 1 of that executable's manifest, then tries fixed modules:
VC80 CRT/CPP (`msvcr80.dll`, `msvcp80.dll`), `d3dx9_40.dll`, `xinput1_3.dll` and
the absolute game-directory `PhysXLoader.dll`. Search is restricted to configured
game/system directories and loader error dialogs are suppressed. Failed activation
reports VC80 checks as `context_unavailable`; independent module checks continue.

The host decoder bounds reports to 8 KiB and rejects unknown fields, unsupported
report schemas other than 2, wrong architecture, duplicate/missing/reordered
components, impossible context results and any claim that the game ran. SDK
creation outcomes also require a loaded PhysX module. Request schema remains 1. Empty state variants use explicit
empty structs so Serde rejects extra fields; a regression test failed with unit
variants and passes with this representation. The Wine smoke uses this decoder.

The JSON report uses fixed component IDs with `loaded`, `unavailable` plus a
Win32 error, or `context_unavailable`; it includes no paths. Schema 2 replaces the old `physx_engine_checked` boolean
with tagged `physx_sdk` evidence; `game_started` remains false. Loading a DLL can run
its entry point and load dependencies, as described by Microsoft's
[LoadLibrary documentation](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw).
A successful module load does not establish SDK initialization. The separate
experimental creation/release check is described in [PhysX ABI evidence](physx-probe.md);
neither check establishes graphics or readiness to launch SGW.

## Validation and integration gates

Portable request/report checks:

```bash
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-runtime-probe
```

Eight portable tests passed locally (`20261004-070649-1979`), and native Mac
strict engine/probe clippy passed (`20261004-071451-5792`); neither compiles or executes the
Windows-only implementation. The standalone `.github/workflows/launcher-runtime-probe.yml` native Windows i686
job builds/tests,
self-tests successful `kernel32` loading and a missing DLL, and uploads the helper.
A Windows-only fixture exercises search configuration and invalid-manifest
reporting through the actual probe. Native Windows x86 clippy/tests/self-test passed at `14e1dfa6b` in run
`37199961628`; later source changes still require their own Windows result.
Before integration, a supervisor must enforce a deadline, bind the helper hash
and validate ownership of the game root. No new frontend behavior or visual UAT
is included.

## Original client evidence

Read-only inspection of the original signed-release seed on 2026-10-04 found
`Working/binaries/SGW.exe` as PE32 i386. Its unmodified SHA256 is
`b25adf3880256c6a6bab31594c0879c411ec0005ea4b7260aef991eaf8947e31`.
Resource `MANIFEST/1` exists and requests x86 `Microsoft.VC80.CRT` versions
`8.0.50727.762` and `8.0.50608.0`, plus Common Controls 6. The probe therefore
uses the actual resource instead of constructing a single-version CRT manifest.
Assembly policy resolution still requires native Windows/Wine execution.

The executable imports `MSVCR80.dll`, `MSVCP80.dll`, `d3dx9_40.dll`,
`XINPUT1_3.dll`, `NxCooking.dll` and `PhysXLoader.dll`, including
`NxCreatePhysicsSDK`. The original `PhysXLoader.dll` SHA256 is
`863e3ec87198bf1a5d5638a20695529dacc9460b0939f2579fe7a7faad2af924`.
Its strings reference `PhysXCore.dll`, `PhysXCooking.dll`, an AGEIA registry
path and `enableLocalPhysXCore`. Those strings are diagnostic leads, not proof
of successful registry lookup or SDK initialization. The current probe does
not cover every game import, including `NxCooking.dll`.

Reproduce without executing the client: use `7zz` to extract all four
`Data/DATA*.CAB` volumes from the authenticated seed, then extract the two
binaries from `DATA1.CAB` with the other volumes alongside it. Inspect the
manifest with `7zz x -so SGW.exe .rsrc/1033/MANIFEST/1` and imports with
`objdump -p SGW.exe`. Hash the original bytes before client setup changes the
ASLR flag. Keep proprietary binary artifacts outside the repository.

## Private Wine probe smoke

The ignored engine test `mac_wine::probe_smoke::original_client_module_probe_in_private_wine_prefix`
uses the pinned, tree-verified Wine runtime in a disposable prefix. Supply
`CIMMERIA_RUNTIME_PROBE` and `CIMMERIA_RUNTIME_PROBE_SHA256` from the same native
Windows CI artifact, and `SGW_PROBE_BINARIES` pointing to the original unmodified
client binaries. The test verifies the probe hash and both client-file hashes
before Wine starts, copies only the two client binaries into its private tree,
and disables window drivers. It bounds execution to 120 seconds and stdout to
8 KiB, then stops/waits for the private prefix before evaluating the report.
Without opt-in provisioning variables it installs no prerequisites; it never
runs the game. The vendor MSI experiment below enables private-prefix provisioning.
It is not the production
supervisor and does not validate cancellation or launcher-restart recovery.

```bash
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine --lib \
  mac_wine::probe_smoke::original_client_module_probe_in_private_wine_prefix \
  -- --exact --ignored --nocapture
```

The harness passes native Mac strict clippy (lane `20261004-064521-91420`).
Execution passed in 24.316 seconds (lane `20261004-065843-97656`) using the native
Windows artifact from `14e1dfa6b`, run `37199961628`, artifact `11302821588`.
Executable SHA256:
`06518c33a0ae3b3f014511ee8e839eefc54d10a6ef111e5021e2fc0f6a0da255`.
The clean prefix loaded the SGW activation context and all five checked modules
without vendor installers. Therefore module loadability alone cannot establish
that those installers are required or that Wine's implementations satisfy the
game. That historical schema-1 run kept both flags false and did not attempt SDK
initialization. The schema-2 Windows and SDK before/after results are below.

The [experimental PhysX SDK probe](physx-probe.md) records the original loader ABI,
implemented hash-pinned lifecycle and outstanding supervision gates. Null default
allocator/output pointers are verified for the original core; no production
integration or new frontend behavior is implemented.

## SDK before/after check

The schema-2 probe from native Windows run `37201203156`, commit `ebeaaaa47`,
artifact `11302724959`, passed the private Wine baseline in 24.278 seconds
(lane `20261004-071305-4880`). All modules loaded, but SDK creation returned null
with numeric error `1`. This is stronger evidence than DLL presence alone.

Set optional `SGW_PHYSX_CORE` to the inertly extracted, hash-verified original
2.6.3 core described in [ABI evidence](physx-probe.md). The same smoke then copies
that core into its private tree, adds its path to the private prefix's 32-bit AGEIA
registry view through a bounded `wine reg` invocation, and reruns the probe.
The before/after test passed in 23.074 seconds (lane `20261004-071507-6004`):
`create_failed` with error `1`, then `initialized_and_released`. Both runs kept
`game_started` false; prefix stop/wait preceded assertions.

This optional registration is a diagnostic fixture, not production provisioning
or vendor-installer validation. No other installation, game process, graphics,
login or gameplay was exercised. Probe executable SHA256:
`3ed60ee8fba3a6b02bf860b559e5ca55f4c5b99836d88126b3f86865ebac3ebc`.

## Original vendor MSI experiment

The schema-2 private-prefix smoke passed in 29.457 seconds (lane
`20261004-072220-9439`). After changing stderr capture to drain continuously
while retaining at most 16 KiB, the rerun passed in 30.806 seconds
(`20261004-072721-11858`); strict engine clippy also passed
(`20261004-072635-11228`). The clean prefix loaded all five modules but SDK creation
returned numeric error `1`. After the original vendor MSI exited zero, the harness
stopped the private prefix, restarted the probe and observed
`initialized_and_released`. No window drivers were enabled and SGW was not started.

To repeat, use the private smoke command above with the schema-2 helper/hash from
[its native CI artifact](physx-probe.md#implemented-sdk-call-and-remaining-supervision-gates),
`SGW_PROBE_BINARIES`, and `SGW_PHYSX_INSTALLER` pointing to the original retained
PhysX 7.11.13 EXE. Set `SGW_PHYSX_INSTALLER_MODE=msi` (the default) and leave
`SGW_PHYSX_CORE` unset; manual-core and vendor-installer modes are mutually exclusive.
The harness calls `prerequisites::physx_msi`, re-exported by the engine from the
small `runtime-probe` library through a normal dependency. It verifies
the exact 39,242,016-byte EXE and SHA-256
`920d5e09e6ba0a92342271c18c67472461813424d70b5c0b981b6f13b129fbf6`,
then carves its embedded MSI at byte offset 35,463, length 38,811,648 bytes.
Those boundaries were independently measured with 7-Zip. Shared extraction also
checks the compound-file signature and MSI SHA-256
`3f122f4be03c6ae42652d28cc5ed48669e0348c8d3650f389954014992a06c8b`.
Callers own bounded source reads, file safety and destination creation; extracting
bytes grants neither execution permission nor license acceptance. Two portable
negative tests cover wrong container lengths and a forged compound header. The
real authenticated smoke exercises the shared function and rejects a same-size
wrapper mutation before installing the restored original bytes. The targeted
prerequisite suite passed six tests (including the two new package guards) in
lane `20261004-072927-13082`; the refactored real Wine test passed in 29.367 seconds
(`20261004-072950-13307`), and strict engine clippy passed
(`20261004-073026-13812`). No frontend state or persistence behavior changed in
this packet; it therefore adds no JS logic or visual UAT coverage.

In the disposable prefix it invokes Wine with
`msiexec /i <MSI> /qn /norestart REBOOT=ReallySuppress /l*v <log>`, bounds the
installer to 180 seconds, and stops/waits for the prefix before the next probe.
The alternative EXE `/s` experiment exited `1` with window-driver errors on stderr;
that observation does not establish the sole failure cause. This validates a
specific vendor-MSI/SDK lifecycle, not the production supervisor, cancellation or
restart recovery, graphics, game launch or readiness.

## Native prerequisite worker

`cimmeria-prerequisite-worker` now implements a one-shot Windows x86 install/probe
sequence. Schema-1 requests are bounded to 8 KiB and bind non-nil operation and
prefix-generation UUIDs to absolute Windows guest game/package/fresh-scratch
paths. Validation uses Windows drive-path syntax on both hosts; macOS native
`Path::is_absolute` cannot validate a Wine request. Device/UNC paths, traversal,
alternate streams and NULs are refused. It validates
the exact package before creating scratch, writes the authenticated MSI and holds
its deny-write/delete sharing handle through installation. Existing scratch is
refused; failed attempts retain evidence rather than overlaying another attempt.

The worker selects `INSTALLUILEVEL_NONE` with
[MsiSetInternalUI](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msisetinternalui),
then calls [MsiInstallProductW](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msiinstallproductw)
with `REBOOT=ReallySuppress`. Only installer code `0` proceeds to the existing
probe. Every nonzero status, including `3010`, remains an installer failure.
A `probed` result means evidence was collected: its nested SDK result may still
be failure, and it never implies readiness.

Results are bounded to 16 KiB. The shared decoder enforces schema and both UUIDs,
validates the nested schema-2 probe report, and refuses `not_checked` SDK evidence
or an installer-failure result carrying code zero. The native CI artifact now
contains both probe and prerequisite-worker executables. To exercise the worker
inside the vendor fixture above, additionally set `SGW_PREREQUISITE_WORKER` and
`SGW_PREREQUISITE_WORKER_SHA256` to that artifact and its verified digest.

Fifteen portable tests pass (`20261004-074214-19556`), including the Windows-path
regression. Strict engine/probe clippy passes (`20261004-074248-20069`). Native
Windows x86 CI passed at `dc69f3e93`, run `37202769972`, artifact `11303288407`.
That artifact predates the host-independent path-validation correction; final
revision Windows validation remains required. Its executable hashes are:

- Prerequisite worker: `94edc387f16263c49019e118f45d41b9e23741cdfe88d6189845e5687bc78c24`.
- Runtime probe: `0e41b5cb34351c8f59b8e88d8955f7b1b852d5871df1285a113574eabd191b3a`.

The real worker smoke passed in 28.252 seconds (`20261004-074227-19820`) using
those Windows-built artifacts and the corrected Mac supervisor. The clean-prefix
SDK failed with code 1; the worker installed the original MSI through the native
API and reported initialized-and-released. After prefix stop/wait and restart,
an independent probe also initialized/released it. All runs kept `game_started`
false. This proves the API path in the fixture, not durable launcher integration.

## Host transport supervision

`engine::prerequisites::supervisor::run` bounds requests/results, requires a host
identity callback before dispatch, and observes both strict result decoding and
successful process exit. `Observed` can contain installer or SDK failure; it is
not a success/ready state. Nonzero process exit, malformed/duplicate output,
wrong identities, deadlines and cancellation after dispatch require reconciliation.
Only the owned host child is stopped; caller-owned prefix shutdown remains required.

The subprocess harness passes (`20261004-074153-19316`), exercising those cases,
pre-dispatch cancellation, refused host recording and recording before request
consumption. It uses real subprocesses with inert fixture results, not MSI or game
execution. The real Wine smoke now exercises this supervisor too. Durable native
storage and native Mac prefix coordination are described below; shell admission
and resource packaging remain unconnected. The prior worker source `10d67a2b9` passed native Windows CI
`37203128166`; this does not validate the newer storage/UI packet.

## Durable runtime-setup state and integration boundary

Native `runtime_setup` storage now admits a separate `PrepareRuntime` operation
from reverified installed identity/content, never the selected preferences folder.
It binds installation identity, runtime/helper hashes, fixed package/probe policy
and a fresh prefix-generation UUID. The plan derives a per-install game-prefix
path distinct from extraction prefixes; admission writes state only and creates
no prefix. Identical-ID admission never dispatches again; changed identities fail.

Dispatch evidence advances `LaunchIntent` → `HostStarted` → `Observed` →
`Quiescent`, bound to the plan digest. Launch intent precedes spawn; host identity
must be durable before the mutating request. A decoded result with matching IDs
is only observed evidence. `finish_runtime_after_stop` is a native caller contract:
the coordinator must actually verify ownership and stop/wait for the prefix.
Storage does not inspect or prove process termination, and this method must not
be exposed directly to IPC. Host exit is never proof of Wine guest termination.

Quiescent evidence is persisted before terminal commit. Complete module/context
checks plus SDK creation/release can record prerequisite success; other observed
outcomes fail. Interrupted nonterminal attempts reopen behind reconciliation,
including a crash between Quiescent persistence and terminal commit. Neither
reopen nor an observed successful report permits automatic replay or inferred
success. Preferences and diagnostics consent remain unchanged.

Native Mac dispatch and constrained explicit reconciliation now exist as described
below. Shell admission/resource packaging remain unwired; there is no new UI
admission button. The Effect/view decodes compatibility-operation status for
inspection, distinguishes interruption from completion, and keeps Play disabled:
PhysX verification cannot bypass graphics or remaining launch gates.

Eight native persistence tests passed (`20261004-075124-23735`), covering identity,
no replay across reopen, tampering, ownership/content rechecks and persistence
faults, including the Quiescent/terminal boundary. Thirty-one frontend tests,
type checking and JS logic UAT through the actual Effect/view passed; IPC was
fixture-backed, not native persistence or visual validation. The full engine
library suite passed 247 tests with 11 explicit ignored smokes
(`20261004-075254-24651`); strict engine/shell clippy passed
(`20261004-075146-24072`) and the frontend build passed. Earlier worker CI/smokes
do not validate this new state integration on Windows.

### Persistent prerequisite evidence

Admission writes `runtime-selection.json` after the immutable plan and before
beginning the operation. A new attempt supersedes earlier success even if
admission or dispatch then fails; lookup never falls back to an older prefix.
`prepared_runtime()` validates the selected plan/digest, strict historical record
and nested report. It requires Quiescent evidence with successful activation,
module loading and SDK creation/release, matching installed identity and the
existing content checks (not a complete corruption scan). Active/recovery work
hides evidence; corrupt records error and uncertain storage requires reopening.
Preferences do not transfer evidence, and another installation cannot inherit it.
This is historical prerequisite evidence, not current prefix/process/graphics
validation or permission to launch. Launch must independently lock and check
those resources. Twelve persistence tests passed (`20261004-083537-39624`);
the enhanced headless smoke passed in 28.060s (`20261004-083607-39880`), covering
lookup after reopen/reconciliation and invalidation after confirmed uninstall.
The full engine suite passed 255 tests with 12 opt-in tests ignored before the
additional installation-identity test. No frontend behavior changed here.


## Retained Mac coordinator

`mac_wine::prerequisites::dispatch` validates the planned helper hash and pinned
runtime, then retains installed-root, exact generation-prefix and verified cached
runtime locks. It creates a fresh headless game prefix distinct from extraction
prefixes; existing generations are never adopted. Dropping the observer does not
stop the retained worker. Its host callback persists identity before sending the
request; observation is stored before verified-prefix stop/wait and durable finish.
Unknown outcomes remain gated even when stop/wait succeeds. Resource failures or
proven pre-spawn cancellation may terminate without asserting SDK evidence.

Explicit `reconcile` accepts only Observed/Quiescent evidence with the recorded
host absent, checks operation ID/revision again around resource acquisition, and
revalidates exact prefix ownership and cached runtime under locks. Host absence
uses signal zero, not PID termination; live/reused PIDs are refused. It stops/waits
for the verified prefix before finishing. LaunchIntent/HostStarted or unknown
results remain gated; this is not general crash/descendant recovery.

With `SGW_PREREQUISITE_WORKER`, its matching `SGW_PREREQUISITE_WORKER_SHA256`,
`SGW_PROBE_BINARIES` and `SGW_PHYSX_INSTALLER` set to the authenticated artifacts
above, run the opt-in coordinator smoke without a production manifest-key override
(the installed-content fixture is development-signed):

```bash
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine --lib \
  mac_wine::prerequisites::tests::retained_coordinator_installs_checks_and_persists_owned_runtime \
  -- --exact --ignored --nocapture
```

The real coordinator smoke passed in 26.297 seconds (`20261004-075925-27636`),
using the original two SGW files/vendor MSI with a development-signed installed
fixture. It dropped the observer, refused duplicate dispatch and preserved the
installer/SDK result across reopen with consent false. This was not a full-seed
installation or game UAT. The enhanced rerun passed in 27.997 seconds
(`20261004-080238-28655`): it restored a synthetic preterminal operation snapshot
while retaining the real Quiescent record, reopened behind reconciliation,
refused a stale revision and explicitly stopped/reconciled the owned prefix.
It did not rerun the installer. This simulates a specific durable-write gap;
it does not validate actual power loss or an unobserved helper crash.
Three local tests passed (`20261004-080135-28262`), as did strict clippy
(`20261004-080139-28218`). No UI admission, packaged prerequisite resource or
visual validation is claimed; prerequisite success still cannot enable Play.


## Prerequisite helper resource staging

The shared staging tool now accepts `--kind prerequisite` for the native Windows
x86 helper. It requires the full trusted build revision and an independently
supplied SHA-256, checks PE32/i386 headers, and writes
`shell/resources/windows/cimmeria-prerequisite-worker.exe` plus ignored
`prerequisite-helper-build.json`. The default `archive` mode still requires
AMD64/PE32+ and uses separate filenames. Both executables are public bundle
resources with mode `0644`; neither receipt authorizes execution.

From the repository root, after downloading the native Windows CI artifact:

```bash
python3 crates/launcher/desktop/tools/stage-helper.py "$SGW_PREREQUISITE_WORKER" \
  --kind prerequisite --sha256 "$SGW_PREREQUISITE_WORKER_SHA256" \
  --revision 10d67a2b9ed146357779e633725441675ea5f7b7
python3 -m unittest discover -s crates/launcher/desktop/tools -p test_stage_helper.py
```

Windows run `37203128166` succeeded for that revision. Its downloaded worker,
SHA-256 `215e40924ce44d194af63c32da8de3347cfb983440cf554faaf0aec98b694815`,
passed staging. Two Python tests cover independent resources/provenance and
wrong hash, revision and architecture rejection. Two native resource tests
passed (`20261004-081125-30919`), including rejection after replacement and
refusal to use the archive identity or receipt as the prerequisite trust source.
The engine's distinct `PrerequisiteResource` exposes path, identity and
reverification without an archive-backend conversion.

The Mac shell now consumes `CIMMERIA_PREREQUISITE_HELPER_SHA256` at compile time
and resolves only `windows/cimmeria-prerequisite-worker.exe` from native bundle
resources. It verifies the resource at resolution and again before admission.
Supply that same digest when invoking the lane build. Staging/compilation alone
does not connect setup to the Install button or establish self-contained startup.

## Native shell setup command

`install_command` accepts `prepare_runtime` with schema version 1, a fresh
operation ID, the observed operation revision and the installed content ID.
There are no webview-selected paths, executables, hashes, URLs, prefix generations
or success reports. This command performs no release fetch. Missing/replaced
resources fail before state initialization; a selected Settings directory alone
cannot authorize setup. Windows currently returns `platform_unavailable` for
this Mac compatibility operation; it does not run Wine or a Mac recipe.

Admission uses the durable installed identity and its native-selected runtime
hash. The retained coordinator receives the independently pinned prerequisite
helper. Duplicate identical operation IDs do not redispatch. Dispatch failure
marks the admitted operation uncertain immediately. The existing `cancel`
command routes by operation ID to the retained prerequisite worker, which
persists cancellation before signaling it. Setup evidence stays native; the
webview only observes operation state. Observed-result reconciliation is now exposed explicitly; successful prerequisites cannot enable Play.
The Effect sequencing contract is described below.

Native shell tests cover missing/replaced resources, forged command fields,
selected-folder refusal, consent preservation, retained dispatch, duplicate
requests, cancellation routing and durable terminal observation. The dispatch
fixture uses signed inert content and a missing runtime cache, so it proves
ownership/failure handling without executing Wine or authenticating game files.
The opt-in resource-binding test resolves the staged Windows CI worker with the
compile-time digest and proves no state or worker is created by resolution.

The shell suite passed 21 tests with two opt-in tests ignored
(`20261004-081707-32965`). The separate compiled resource-binding test passed
(`20261004-081747-33227`) with the staged x86 artifact:

```bash
export CIMMERIA_PREREQUISITE_HELPER_SHA256=215e40924ce44d194af63c32da8de3347cfb983440cf554faaf0aec98b694815
export CIMMERIA_TEST_RESOURCE_DIR="$PWD/crates/launcher/desktop/shell/resources"
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-desktop \
  host::runtime_setup::tests::compiled_prerequisite_resource_resolves_without_starting_work \
  -- --exact --ignored
```

No frontend code changed in this native-boundary packet; no new JS or visual UAT
is claimed. The Effect/UI integration requires its own logic and interaction
verification. No application window, game or live telemetry endpoint was opened.


## Effect and main Install flow

The native status includes `runtime_setup`, either null or an eligible installed
content ID. It is exposed only for completed Mac content or a known terminal
failed/cancelled prerequisite attempt with a verified helper. Active/uncertain
work and successful prerequisite attempts never advertise a new setup attempt.
Admission still revalidates identity/content/resources; a capability snapshot
is not permission to bypass native checks.

An Install click authorizes the content → prerequisite sequence while that
frontend scope is alive. Immediate completion and completion found by polling
both advance exactly once through Effect `prepareRuntime`, with a fresh work ID
and an inspected predecessor/revision. Cancellation clears this continuation,
even if content then completes successfully. Reopening, observer failure or a
lost mutation reply does not replay setup: the main button offers “Continue
installation” when native state permits it. A replaced predecessor is refused
before dispatch. Setup cancellation routes to its own work ID; consent is not
changed. A cancelled/failed known-terminal setup can be explicitly retried with
a new generation. Unknown recovery remains gated.

Validation: 36 frontend tests, TypeScript checking, production frontend build,
and `npm run uat:install` passed. The sequential JS logic UAT mounts the actual
Effect/view code against controlled native replies, exercises automatic setup,
explicit continuation after reopen, visible cancellation and unchanged consent.
Tests additionally cover delayed completion, lost replies, predecessor changes
and cancellation winning the continuation decision. Native target projection
and admission tests passed (`20261004-082325-35183`); the full shell suite passed
21 tests with two opt-in checks ignored (`20261004-082215-34649`). These passes do
not exercise the actual Tauri window, Wine installation via UI, disk persistence
through JS, graphics, login or gameplay. Those remain separate UAT gates.


## Explicit observed-result recovery

The shell exposes recovery only when the current runtime operation requires
reconciliation, its strict saved result is Observed/Quiescent, and the recorded
host PID is absent. This availability is advisory: execution repeats identity,
revision, resource ownership and absence checks before stopping/waiting for the
exact prefix and committing its result. A live/reused PID, missing plan,
unobserved helper or uncertain result never authorizes recovery. No arbitrary
PID is terminated and no prerequisite installer is replayed.

The main button remains gated during recovery. “Recover compatibility setup”
uses the existing Effect reconcile command with the current ID/revision. Tauri
routes runtime operations to the retained asynchronous coordinator, leaving
content recovery with its existing implementation. Losing the invoke reply or
window does not cancel the retained native reconciliation. Completed recovery
still displays prerequisite status, never Play readiness. Unknown-outcome
recovery needs further work; it is not inferred from host absence alone.

Four engine recovery/ownership tests passed (`20261004-082727-36777`), and 22
shell tests passed with two opt-in checks ignored (`20261004-082824-37096`). The
headless original-file coordinator smoke passed in 27.470 seconds
(`20261004-082841-37275`) using the Windows `10d67a2b9` artifact. It verified
availability after the synthetic interrupted-commit reopen, explicit recovery
without installer replay, and unavailable recovery after terminal success.
This remains a synthetic durable-write boundary, not actual power-loss proof.
37 frontend tests, the frontend build and JS logic UAT passed; UAT verified the
explicit action, operation/revision binding, no setup replay and unchanged
consent through controlled IPC. No native visual or game UAT is claimed.
