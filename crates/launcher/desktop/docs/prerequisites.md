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
strict clippy passed (`20261004-063517-87120`); neither compiles or executes the
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
It never installs prerequisites or runs the game. It is not the production
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
game. PhysX SDK initialization, graphics and login remain unverified; the report
kept both readiness-related flags false. That successful run used report schema 1 and did not attempt SDK initialization.
The current strict schema-2 decoder requires a new helper; native Windows
validation of the SDK change remains pending.

The [experimental PhysX SDK probe](physx-probe.md) records the original loader ABI,
implemented hash-pinned lifecycle and outstanding supervision gates. Null default
allocator/output pointers remain experimental; no production integration or new
frontend behavior is implemented.
