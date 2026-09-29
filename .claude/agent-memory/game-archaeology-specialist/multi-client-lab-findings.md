---
name: multi-client-lab-findings
description: Two SGW.exe on one machine (2026-09-29) - no client single-instance guard, SGW.lock is Ghidra's, focus-throttle address, named-instance lab fix, and Ghidra-MCP workflow tips that saved time
metadata:
  type: project
---

Full write-up: `docs/reverse-engineering/findings/multi-client-lab.md`. Live two-client runs were NOT done (coordinator paused the machine); client-side claims are Ghidra-only until the LT-1..LT-7 plan runs.

- **No single-instance check** in SGW.exe: every CreateMutex/Semaphore/Event site in decompiled functions has a NULL name. `binaries\SGW.lock`/`SGW.lock~`/`SGW.gpr`/`SGW.rep` are the **Ghidra project's** files, not the client's. Do not chase them.
- **Focus throttle**: `FEngineLoop::Tick` `0x00417100`, `GetForegroundWindow` pid compare at `0x004171a7..0x004171fe`; background = BELOW_NORMAL + `Sleep(5 ms)` per tick (`0x00491320`, float `0x017f9004`, double `0x01805f30` = 1000.0). Our virtual focus (IAT `GetForegroundWindow` swap) defeats it per process.
- **Hypothesis, unconfirmed**: named file mapping `"ZipArchive Mapping File"` (`0x0139ede0`, caller `0x0139f460`) is machine-wide, so two clients compacting a cache `.pak` at once could map the wrong file.
- Named lab instances (`CIMMERIA_LAB_INSTANCE`, `CIMMERIA_LAB_SESSION_FILE` env, passed through `start32::run_with_env`; CreateProcessW gets a NULL environment so the game inherits it).

Workflow tips: Ghidra MCP `list_imports` is truncated and huge; parse the PE with `pefile` (import IAT addresses), then `get_xrefs_to <IAT addr>`. `sgw-testhost` (tools/testhost/stage.sh) runs the lab DLLs headless with no SGW.exe, good for anything that only needs DLL boot behaviour. Files under crates/ and docs/guides are CRLF: patch with a CRLF-preserving script, not the Edit tool.
