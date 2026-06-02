# SGW Crash-Dump Pipeline

What `SGW.exe` does when it crashes, where the diagnostic artifacts land, how to trigger a crash deliberately for pipeline validation, and how to turn the resulting minidump into a Ghidra address you can decompile.

The crash machinery is **in-process** — no separate `CrashReport.exe` ships with the client. Everything from exception filter to dump-write to (attempted) upload happens inside `SGW.exe` itself, using the shipped `dbghelp.dll`. This doc is general-purpose: any SGW client crash flows through the same pipeline, whether it's a vanilla render-thread fault or the kind of structurally-invalid package that the [UE3 package splicer](ue3-package-splicer.md) might produce.

## See also

- [ue3-package-splicer.md](ue3-package-splicer.md) — the splicer is a primary consumer of this pipeline; spliced maps that fail to load surface as minidumps here.
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
| Exec console handler | `FUN_0048bd40` — the UE3 `Exec()` command dispatcher |

The auto-upload step is the only piece that matters to nobody anymore — the upload target is a UNC path inside CME's old Cheyenne Mountain network and silently fails. The dump itself still gets written locally on every crash, which is all the reading pipeline below cares about.

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

The reader is [`tools/sgw_read_crash.py`](../../tools/sgw_read_crash.py) — parses a Windows minidump (requires `pip install minidump`), locates `SGW.exe`'s base address, and prints the exception record + crashed thread's instruction pointer as an `SGW.exe` RVA ready to plug into Ghidra. `--latest <dir>` picks the newest `SGW_*` subdir under the crash root.

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
| Crash-dump reader tool | Done — `tools/sgw_read_crash.py` landed |
| Deliberate-crash console commands documented | Done — four commands verified, see table above |
| First real-crash walkthrough | Pending — awaiting a crash worth analyzing end-to-end |
