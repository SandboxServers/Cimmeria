# SGW Crash-Dump Pipeline

What `SGW.exe` does when it crashes, where the diagnostic artifacts land, how to trigger a crash deliberately for pipeline validation, and how to turn the resulting minidump into a Ghidra address you can decompile.

The crash machinery is **in-process** — no separate `CrashReport.exe` ships with the client. Everything from exception filter to dump-write to (attempted) upload happens inside `SGW.exe` itself, using the shipped `dbghelp.dll`. This doc is general-purpose: any SGW client crash flows through the same pipeline, whether it's a vanilla render-thread fault or a structurally-invalid asset package that the UE3 loader rejects.

## See also

- [engine/ue3-package-format.md](../engine/ue3-package-format.md) — the SGW UE3 package binary layout. If you are writing packages the client has to load, a failed load lands here as a minidump, and the faulting RVA usually names the loader function that rejected the structure.
- [client-tools.md](../client-tools.md) — broader client-side tooling context.
- [reverse-engineering/](../reverse-engineering/) — Ghidra-side catalog the RVAs from this pipeline feed into.

## Architecture

Discovered via Ghidra of `SGW.exe`:

| Field | Value |
|---|---|
| Dump directory | `binaries/CrashDumps/SGW_<YYYY-MM-DD>_<HH-MM-SS>_<ComputerName>_<UserName>_<Build>/` |
| Dump file | `minidump.dmp` — standard Windows minidump via `MiniDumpWriteDump` from shipped `dbghelp.dll` |
| Sidecar | XML manifest matching schema `crash:CrashReport` / `crashapp:CrashFileList` / `crashapp:CrashFileEntry` (likely zipped with the dump) |
| Auto-upload | On next launch tries to send to `\\skaro\crashDump\GameDumps` — dead Cheyenne Mountain internal UNC, won't reach anywhere |
| Symbols | No PDBs shipped. Stripped retail binary. Map RVAs through Ghidra (project `SGW`, TCP `:8100`) |
| Crash-handler function | `FUN_00415e00` builds the dump path string |
| Exception filter | `WinMain` runs `GuardedMain` (`0x00416010`) under `__try/__except`; the filter calls `CreateMiniDump` (`0x0041ddb0`). Four more engine thread bodies call it too. A fault on those threads is handled there and never reaches a top-level filter |
| Dump writer | CME's writer at `0x00a55a70` is the only caller of `MiniDumpWriteDump` (thunk `0x012f5906`, IAT slot `0x017F0058`). Dump type 0 (normal) or 2 (full memory), with the live exception pointers |
| Exec console handler | `FUN_0048bd40` — the UE3 `Exec()` command dispatcher |

The auto-upload step is the only piece that matters to nobody anymore — the upload target is a UNC path inside CME's old Cheyenne Mountain network and silently fails. The dump itself still gets written locally on every crash, which is all the reading pipeline below cares about.

## What the telemetry DLL adds

When the launcher injects the telemetry DLL (telemetry opted in), a crash
also produces, independently of the game's own dump above:

- `client.crash` in SigNoz within a few seconds, with the fault as
  `module+offset` (`SGW.exe+0x00016ec5`), the exception code and, for an
  access violation, the address touched. The offset is the RVA to open
  in Ghidra; add `0x00400000` for the VA (ASLR is off).
- `Binaries/sessions/crash-<session_id>-<ts_ms>.dmp`: a `MiniDumpNormal`
  dump with thread info and unloaded modules, a few MB, next to a `.jsonl`
  sidecar with the same fields as `client.crash`. The launcher's
  end-of-session bundle ships the `sessions/` folder; the server logs the
  dump's arrival as `launcher.bundle.crash_dump` but does not keep it.
- `client.exit` when the process leaves, with `after_crash` set after a
  crash.

The DLL does not change how the game handles the crash: it records it
from inside the game's `MiniDumpWriteDump` call and then lets that call
run, and its top-level filter hands every exception to the filter the
game set. Details: [client-telemetry.md § Crash and exit
capture](../architecture/client-telemetry.md#crash-and-exit-capture-phase-6).
Where to find the events: [telemetry.md § Crashes and
exits](../operations/telemetry.md#crashes-and-exits).

## How to trigger a deliberate crash

For validating the pipeline end-to-end before relying on it for diagnosis of an actual crash. In-game, press tilde (`~`) to open the UE3 console, then any of:

| Command | Effect |
|---|---|
| `DEBUG CRASH GPF` | Writes `0x7b` to `0x00000000`. Logs `"Crashing with voluntary GPF"` first. Instant GPF. |
| `DEBUG CRASH ASSERT` | Calls `FUN_00486000("0", ".\\Src\\UnMisc.cpp", 0x101a)` — UE3 `appFailAssert`. |
| `/forceclientcrash` | SGW slash command `Event_SlashCmd_ForceClientCrash` (chat input). |
| `/forcerenderthreadcrash` | SGW slash command `Event_SlashCmd_ForceRenderThreadCrash` (render-thread variant). |

All of these write a real `.dmp` under `CrashDumps/`. Use one as the **known-good test case** for the reading pipeline before relying on it for a real-crash diagnosis (e.g. spliced-map load failure).

## How to read a crash dump

> **⚠ `tools/sgw_read_crash.py` is not in the repository.** As of
> 2026-07-25 no such file exists under [`tools/`](../../tools/), and unlike
> the UE3 splicer scripts it left no `.pyc` remnant either. The commands
> in this section will fail. The description below is retained as a spec
> for rebuilding the reader; until then, read dumps with WinDbg or a
> direct `minidump` script.

The reader is `tools/sgw_read_crash.py` — parses a Windows minidump (requires `pip install minidump`), locates `SGW.exe`'s base address, and prints the exception record + crashed thread's instruction pointer as an `SGW.exe` RVA ready to plug into Ghidra. `--latest <dir>` picks the newest `SGW_*` subdir under the crash root.

```text
python tools/sgw_read_crash.py --latest \
  "C:/Users/Steve/source/projects/SGW/Stargate Worlds-QA/Working/binaries/CrashDumps"
```

The tool prints:

- System info (architecture, OS build).
- Modules (with `SGW.exe` base address called out).
- Exception code (e.g. `EXCEPTION_ACCESS_VIOLATION`), faulting address, and the `SGW.exe` RVA.
- Crashed thread's `EIP`/`RIP`, `ESP`/`RSP`, `EBP`/`RBP`.

The RVA goes straight into Ghidra:

```text
mcp__ghidra__get_function_by_address  -> function name + entry point
mcp__ghidra__decompile_function       -> pseudocode for the faulting function
```

## Status

| Task | State |
|---|---|
| Crash-dump reader tool | **Missing** — `tools/sgw_read_crash.py` is not in the repo (see the warning above). Previously recorded as "landed"; the file is absent as of 2026-07-25. |
| Deliberate-crash console commands documented | Done — four commands verified, see table above |
| First real-crash walkthrough | Pending — awaiting a crash worth analyzing end-to-end |
| Telemetry DLL crash capture (`client.crash`, sessions-dir dump) | Implemented 2026-09-28; not yet exercised by a real crash. The `DEBUG CRASH GPF` command above is the test case |
