# cimmeria-client-launch

Shared client launch primitives: start `SGW.exe` suspended, inject DLLs
into it, resume it, and patch its `.rdata` server hostname. Used by
`sgw-launcher` and `cimmeria-lab` so both drive the same code (see
issue #685).

| Module | What it holds |
|---|---|
| `launch` | `SGW.exe` and Atera launches, `checked_sgw_exe`, `launch_sgw_injected` (same-bitness injection) |
| `inject` | `inject_dll`: `VirtualAllocEx` → `WriteProcessMemory` → `CreateRemoteThread(LoadLibraryW)`, with the `BitnessMismatch` guard |
| `process` | `SuspendedProcess`, `RunningProcess` (wait on a game by handle or pid), `OpenedProcess`, command-line quoting |
| `start32` | The `sgw-start32` helper's contract, shared by the helper (crate [`cimmeria-start32`](../start32/)) and its callers |
| `patch_rdata` | The `SGW.exe` hostname patch |

## Why a 32-bit helper

`inject_dll` hands the remote thread the injector's own `LoadLibraryW`,
which only exists in a process of the same bitness. `SGW.exe` is 32-bit
and the launcher is 64-bit, and a 64-bit process cannot reach the
target's 32-bit `LoadLibraryW` either:

- a process created `CREATE_SUSPENDED` has no 32-bit kernel32 mapped
  yet (measured: `EnumProcessModulesEx(LIST_MODULES_32BIT)` returns no
  modules until it runs);
- a thread a 64-bit process creates there starts in 64-bit mode, so it
  cannot run 32-bit `LoadLibraryW` code even at the right address.

So a 64-bit caller runs `sgw-start32.exe`, an i686 binary in its own
crate, [`cimmeria-start32`](../start32/), which links only this library
(thiserror and windows-sys, never the launcher's dependencies). It does
the suspended launch and the injection at the target's bitness. A direct injection across bitness is refused with
`InjectError::BitnessMismatch` rather than failing as a bare
`RemoteLoadFailed`.

## The `sgw-start32` contract

Build it for i686. Its `build.rs` embeds an `asInvoker` manifest, so it
starts without elevation (an unmanifested 32-bit exe whose name looks
like an installer trips UAC's installer detection with os error 740;
the name also avoids those words), and a version resource (product,
company, description), so antivirus software and Windows can identify
it:

```sh
bash tools/build-lane/lane.sh cargo build -p cimmeria-start32 \
  --target i686-pc-windows-msvc
```

### Command line

```text
sgw-start32 spawn <exe> [--cwd <dir>] [--dll <path>]... [-- <arg>...]
sgw-start32 pid <pid> [--dll <path>]...
```

- `spawn` starts `<exe>` suspended, in `--cwd` (default: the exe's own
  directory), injects each `--dll` in the order given, then resumes it.
  Arguments after `--` are passed to `<exe>`.
- `pid` injects each `--dll` into a process that is already running.
- At least one `--dll` is required. The order is the injection order:
  the launcher passes `cimmeria-client-patches.dll` before the telemetry
  DLL.

### Output and exit code

Exactly one line on stdout:

| Result | stdout | Exit |
|---|---|---|
| Started | `ok pid=<pid>` | 0 |
| Failed | `error kind=<kind> detail=<one-line text>` | 1 |
| Usage error | `error kind=usage detail=<text>` | 2 |

`<kind>` is one of `usage`, `not_found` (the exe or a DLL is missing),
`spawn`, `open_process`, `bitness_mismatch`, `remote_load_failed` (the
remote `LoadLibraryW` returned NULL: not a loadable DLL, or its
`DllMain` returned FALSE), `inject`, `resume`, `unsupported` (not
Windows). A `spawn` whose injection fails terminates the suspended
process before it answers, so the caller can start the program plainly.
The helper never waits for the target: it exits as soon as the target
is resumed.

### Calling it from Rust

Use `start32` rather than formatting the command line by hand, so the
caller and the helper cannot drift:

```rust
use cimmeria_client_launch::inject::RunningProcess;
use cimmeria_client_launch::start32::{self, Request, Target};

let pid = start32::run(&helper_path, &Request {
    target: Target::Spawn { exe, cwd: Some(install_dir), args: vec![] },
    dlls: vec![patches_dll, telemetry_dll],
})?; // HelperError::{Run, Failed { kind, detail }, Garbled { .. }}
let game = RunningProcess::open(pid)?; // SYNCHRONIZE: wait on the exit
let exit_code = game.wait()?;
```

`sgw-launcher` embeds the helper and keeps it at one stable path,
`<launcher dir>/sgw-start32.exe`, never `%TEMP%`
(`crates/launcher/src/start32_helper.rs`), so an antivirus exclusion for
the launcher's folder survives updates. The telemetry launch and
`cimmeria-lab` can call the same helper with their own DLL lists; a
caller that ships separately should likewise keep it beside itself under
that name.

## Tests

The portable logic (argument parsing, the stdout contract, command-line
quoting, the bitness decision) is unit-tested on every host. The
helper tests run the real i686 helper from an x64 test build and inject
a real 32-bit DLL (`SysWOW64\version.dll`) into a real 32-bit program
(a copy of `SysWOW64\PING.EXE`): it loads, the program runs with its
arguments and exits 0; a non-PE "DLL" comes back `remote_load_failed`;
a missing one `not_found`; and the helper starts unelevated and carries
its manifest. Point `CIMMERIA_TEST_START32` at the built helper:

```sh
CIMMERIA_TEST_START32='<target dir>\i686-pc-windows-msvc\debug\sgw-start32.exe' \
  bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-client-launch
```

Unset, those tests skip locally; with `CI` set they fail, so they cannot
pass vacuously in [launcher-build.yml](../../.github/workflows/launcher-build.yml),
which builds the helper and sets the variable.
