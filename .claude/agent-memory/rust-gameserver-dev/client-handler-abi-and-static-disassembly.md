---
name: client-handler-abi-and-static-disassembly
description: CME event handlers are thiscall(this, event, subject) ret 8 even where a finding says one arg; verify ret N by disassembling the local QA SGW.exe with capstone before writing any detour; the event-bag getters take a hand-built MSVC std::string
metadata:
  type: project
---

**Every CME subscriber member callback pops two stack args** (`thiscall(this,
event*, subject*)`, `ret 8`): `0x00e09160` EffectSet, `0x00ea6af0`
CooldownManager, `0x00d05790` SequenceManager::onSequence, the GameBeing stat
handlers `0x00e01f40`/`0x00e02060`, `0x00e01c90`. The AB-C0 finding wrote three
of them as `thiscall(this, event*)` (corrected 2026-10-04, PR after #1178). The
same class of error (missing stack arg) was found in the 2026-09-27 anchor
audit too.

**Why:** a detour with the wrong arg count leaves 4 bytes on the stack per
call and corrupts the caller. Findings are written from decompiles, which drop
unused trailing params.

**How to apply:** before writing any inline detour, disassemble the function
to its `ret N` yourself. No Ghidra needed: the QA client copy sits next to the
main checkout (`../SGW/Stargate Worlds-QA/Working/binaries/SGW.exe`, ASLR off,
image base 0x400000). Map VA→file offset through the PE section table and run
capstone (`CS_MODE_32`). On this workstation the default 32-bit Python's
capstone DLL fails to load; `py -3.13` (64-bit) works. Do not name the script
`dis.py` — it shadows the stdlib module capstone imports. The same file gives
the fingerprint bytes and RTTI names (vtable[-1] → COL → `+0xc` type
descriptor → name at `+8`); that is how `0x00e0a810` turned out to build
`Event_NetOut_elementDataRequest`, not a bar removal.

**Event-bag getters** `GetInt 0x00e3cba0` / `GetFloat 0x00e3cc20` /
`GetByte 0x00d434d0`: `bool thiscall(event, const std::string*, T*)`, `ret 8`.
Callable from a handler detour on the main thread with a hand-built MSVC 2008
string (`{alloc u32, buf[16], size, cap}`, 0x1c bytes; a name of 16+ chars puts
a pointer to a static NUL-terminated copy in `buf[0..4]` and `cap >= 16`).
Implementation: `crates/client-telemetry/src/hooks/ability_trace/event_bag.rs`.
INT8 fields (`Type`, `ViewType`, `PrimaryTarget`) need `GetByte`; the handler's
own call sites tell you which getter each field uses.

Related: [[injected-dll-unwind-and-lua-error-rules]], [[cme-registry-is-a-factory-not-subscribe]].
