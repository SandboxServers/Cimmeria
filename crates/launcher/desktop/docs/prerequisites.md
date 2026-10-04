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
prefix-generation UUIDs to absolute game/package/fresh-scratch paths. It validates
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

Fourteen portable tests passed (`20261004-073512-15727`) before one additional
assertion; final tests/clippy, native Windows CI and real Wine worker execution
remain pending. Earlier EXE/msiexec experiments do not validate this new native
MSI API path. Production admission, journaling, supervision and cancellation are
still unfinished; no frontend behavior changed.

## Planned production integration boundary

This coordination is not implemented. A separate `PrepareRuntime` operation must
follow content installation, with an exclusive per-install game-prefix generation
distinct from the extraction prefix. Bind immutable installation, runtime, helper
and prerequisite-package identities before mutation; selected preferences cannot
retarget an admitted attempt.

The new worker supplies the install/probe sequence, but durable coordination is
not implemented. Persist
launch intent, then host identity before sending the mutating request, followed by
installer result, SDK result and durable completion. A successful installer exit
alone is insufficient. Uncertain requests/results require explicit reconciliation:
verify ownership and identity, stop/wait for that exact prefix, then inspect the
recorded outcome. Host exit is never proof that Wine guests have stopped.

Supervision still needs bounded input/output, deadlines, cancellation policy and
crash/reopen tests spanning each durable transition. Preserve consent/preferences
and keep diagnostic results distinct from launch permission. Verified PhysX alone
cannot enable Play while graphics and the remaining launch gates are pending.
