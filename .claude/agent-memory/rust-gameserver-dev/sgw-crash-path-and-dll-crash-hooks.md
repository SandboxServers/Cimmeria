---
name: sgw-crash-path-and-dll-crash-hooks
description: SGW.exe crashes are caught by UE3 __except frames, not a top-level filter; the MiniDumpWriteDump IAT slot is the choke point; offline PE/capstone recipe when Ghidra is down
metadata:
  type: reference
---

Read from the QA SGW.exe on 2026-09-28 (static, not yet seen live):

- UE3 `WinMain` runs `GuardedMain` (0x00416010) under `__try/__except(CreateMiniDump 0x0041ddb0)`; four
  more engine thread bodies do too. A top-level unhandled-exception filter never sees those faults.
- The only `MiniDumpWriteDump` caller is CME's writer 0x00a55a70 via thunk 0x012f5906 (`jmp [0x017F0058]`),
  dump type 0 or 2 (full memory). Hooking that IAT slot sees the game's own crashes; the Phase 6 DLL
  crash module does exactly that (`crates/client-telemetry/src/crash/`).
- SGW's own `SetUnhandledExceptionFilter` calls are CRT only (start-up C++ filter, `__report_gsfailure`).
- Normal quit = CRT `exit` through SGW's IAT (0x017EF9A8, MSVCR80); forced quit = `ExitProcess` (0x017EF238).
- IAT slot VAs come from pefile `imp.address`; the old docs listed hint/name RVAs (odd addresses) instead.

Offline recipe when no Ghidra instance runs: `py -3.13` (64-bit Store Python; the 32-bit default
python cannot load capstone's DLL) + `pefile` + `capstone`, scan `.text` for `FF 15/FF 25 <slot>` and
`E8 rel32` xrefs. Don't name the script `dis.py` (shadows stdlib `dis`, capstone import breaks).

Test traps met on the way: `bridge::dynamic_hooks` install test "patches for real" on i686, so any
hard-coded address may be mapped in a given test binary's layout (0x00abc100 started failing once the crate grew; base passed, the null-region address passes);
use the never-mapped first 64 KiB. An uploader that re-blocks for the full cadence with a non-empty
batch misses a flush request that lands after the event woke it: cap the wait (`URGENT_RETRY`).
Host (x86_64) clippy on the crate fails on an unused `JMP_REL32_LEN`; CI only lints i686.

Related: [[telemetry-anchor-audit-and-hookgate]], [[offline-client-event-trace-and-udp-port-trap]]
