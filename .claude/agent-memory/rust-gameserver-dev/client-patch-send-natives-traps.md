---
name: client-patch-send-natives-traps
description: BM-04 send natives in cimmeria-client-patches - startEntityMessage does not refuse offline, microseh catches through non-unwind ABIs (so no ABI revert proof), Lua C-function args vs lua_cpcall, disassembling SGW.exe without Ghidra
metadata:
  type: project
---

Learned building the Black Market send natives (BM-04, 2026-09-27).

- `ServerConnection::startEntityMessage` (`0x00dd6a60`) checks `[conn+0x30c]`, logs an error when offline, and **starts the message anyway**. The caller must check the flag itself, as the engine's own sender `FUN_00c6fc40` does.
- `microseh::try_seh` catches an access violation and an `0xE06D7363` (MSVC throw) raised through a fake `extern "thiscall"` callee just as it does through `"thiscall-unwind"`. So a test that raises through the guard is **not** a revert proof for the #915 ABI rule. Keep `-unwind` by rule, and say in the comment that the guard is what stops the exception.
- A Lua C function reads its args at stack indices 1..n. You cannot wrap the arg reading in `lua_cpcall`, because the protected body gets a new frame and cannot see the caller's args. So read args unprotected (a Lua OOM then unwinds to the script's pcall, which is fine), put `catch_unwind` only around the non-Lua part, and guard the rest with `AbortOnPanic`. A Lua error hitting `catch_unwind` may abort the process.
- Without Ghidra, `python -m pip install --user --only-binary=:all: iced-x86` works on Python 3.13 (capstone has no wheel there and fails to build). Map VA to file offset from the PE section table. That was enough to pin the send side's fingerprint bytes and to read the itemDef lookup chain (see the evidence doc §4).
- In `FakeLua`, `called_with(args)` simulates a native's view: args are the whole stack and the heap may be touched (protected depth 1). `type_at` past the top returns `LUA_TNONE`, as the real API does for a missing argument.

Related: [[injected-dll-unwind-and-lua-error-rules]], [[i686-test-exe-uac-installer-detection]].
