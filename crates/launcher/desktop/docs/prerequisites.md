# Prerequisite evidence and runtime probe

Prerequisite retention is documented in [Wine validation](wine-validation.md#inert-prerequisite-retention).
Retained installers are inert data; their presence does not mean installation or
readiness. The `runtime-probe` crate belongs to the standalone desktop workspace.
Its executable operates only in native Windows x86 builds; other targets exit
unsupported. It is not yet integrated with the managed shell or run against the
real game.

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

The JSON report uses fixed component IDs with `loaded`, `unavailable` plus a
Win32 error, or `context_unavailable`; it includes no paths. Both
`physx_engine_checked` and `game_started` are always false. Loading a DLL can run
its entry point and load dependencies, as described by Microsoft's
[LoadLibrary documentation](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw).
A successful load does not initialize the PhysX SDK, create a graphics device or
establish readiness to launch SGW.

## Validation and integration gates

Portable request/report checks:

```bash
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-runtime-probe
```

Three portable tests passed locally (`20261004-063638-87802`), and native Mac
strict clippy passed (`20261004-063517-87120`); neither compiles or executes the
Windows-only implementation. The native Windows i686 CI job builds/tests,
self-tests successful `kernel32` loading and a missing DLL, and uploads the helper.
A Windows-only fixture exercises search configuration and invalid-manifest
reporting through the actual probe. Native Windows results remain pending.
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
