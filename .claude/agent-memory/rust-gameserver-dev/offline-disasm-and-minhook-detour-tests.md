---
name: offline-disasm-and-minhook-detour-tests
description: Disassembling SGW.exe without Ghidra (pefile + capstone under the 64-bit Store Python), the shared-scratchpad trap, and the MinHook patch/call/unpatch test pattern for client-telemetry detours
metadata:
  type: reference
---

Learned 2026-10-04 implementing AB-C1/AB-C2 (`client.ability.*`).

**Disassembling the QA client offline.** The client image is at
`..\SGW\Stargate Worlds-QA\Working\binaries\SGW.exe` (ASLR off, base
0x400000). `pefile` + `capstone` work under `py -V:3.13` (the 64-bit Store
Python); the default 32-bit Python's capstone DLL fails to load (WinError
193). Run it from the PowerShell tool: the same `py -V:3.13` call from Bash
in a worktree sometimes reports "No suitable Python runtime". Map VA with
`pe.get_memory_mapped_image()[va - base:]`. A raw E8/E9 scan of `.text` plus
an absolute-pointer scan gives callers and vtable slots in seconds; good
enough to verify a finding's prologue bytes, `ret N` and branch addresses
before hooking. Do not name a script `dis.py` (shadows stdlib `dis`, capstone
import breaks).

**The session scratchpad can be shared with other sessions.** A file I
created there was replaced by another agent's same-named script mid-task.
Work in a unique subdirectory of the scratchpad.

**MinHook patch/call/unpatch tests.** For an inline detour, install it with
`minhook_sys::MH_CreateHook` + `MH_EnableHook` over a `#[inline(never)]`
stand-in of the same ABI (`thiscall-unwind`, `C-unwind`), publish the
trampoline into the detour's `OnceLock`, call the stand-in through a
`black_box` fn pointer, then `MH_DisableHook` + `MH_RemoveHook` and call
again to show the detour is gone. Wire stand-ins like the game's chain (A
calls hooked B calls hooked C) to drive thread-local state machines in real
order. A Rust panic in a stand-in models a Lua error (C++ throw) and unwinds
fine through the MinHook trampoline on i686, running the detour's `Drop`
guards. Each trampoline `OnceLock` can be set once per process, so give each
detour exactly one test (cargo test shares a process; nextest does not).
Clippy wants `f as *const () as usize`, and `0x1 as *mut c_void` trips
`manual_dangling_ptr`.

**Game code called from a detour runs outside `catch_unwind`.** A foreign
(C++) exception reaching `catch_unwind` aborts; call the game's readers
(e.g. the event-bag `GetInt` `0x00e3cba0`) between `guarded` blocks, never
inside one.

Related: [[offline-client-event-trace-and-udp-port-trap]], [[injected-dll-unwind-and-lua-error-rules]].
