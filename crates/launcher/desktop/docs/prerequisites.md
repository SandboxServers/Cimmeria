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

Three portable tests passed locally (`20261004-063409-86560`); this does not compile
or execute the Windows-only implementation. Strict clippy is pending. The native
Windows i686 CI job is configured to build/test, self-test successful `kernel32`
loading and a missing DLL, and upload the helper. Its result remains pending.
Before integration, a supervisor must enforce a deadline, bind the helper hash
and validate ownership of the game root. No new frontend behavior or visual UAT
is included.
