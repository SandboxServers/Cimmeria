---
name: injected-dll-unwind-and-lua-error-rules
description: Detours of SGW.exe functions with a C++ EH prologue use extern "thiscall-unwind" (and lua51 detours C-unwind: it throws C++ exceptions); client Lua errors outside pcall exit, never unwind; MinHook chains two DLLs on one target
metadata:
  type: reference
---

Rules settled while writing `crates/client-patches` (2026-09-27). They apply
to any Rust detour injected into `SGW.exe`.

- **A prologue of `6A FF 68 <handler>` or `64 A1 00 00 00 00` means an MSVC
  C++ exception frame.** The function or its callees may throw. For
  example, the dispatcher's caller `0x00dd2b80` throws `bad_alloc` via
  `_CxxThrowException`, and UE3's `FEngineLoop::Tick` reports fatal errors
  by throwing. Give its detour and its trampoline type
  `extern "thiscall-unwind"` (stable on 1.98). A plain `extern "thiscall"`
  Rust function aborts the process when a foreign exception unwinds
  through it. With `-unwind`, drop guards in the detour run and the
  exception reaches the game's own handler.
- **Do not wrap a call that can throw a C++ exception in `catch_unwind`.**
  Catching a foreign exception there either aborts or returns `Err`, and
  which one is unspecified. Wrap only pure-Rust work.
- **A client Lua error takes one of two paths.**
  - *No handler active* (`L->errorJmp == NULL`, the case at the top of
    `Tick`): `luaD_throw` calls the panic function and then `exit()`.
    Nothing unwinds, so a Lua API import called only there can be plain
    `extern "C"`. Only allocation failures and `__gc` errors can raise
    there anyway. Look up globals with `lua_rawget`, so no metamethod runs
    unprotected.
  - *Under `lua_pcall`*: `lua51.dll` raises the error as a C++ exception
    (it imports `_CxxThrowException`) and it unwinds to the nearest
    `lua_pcall`. A `lua_call` made inside a pcall'd chunk is on that path,
    so anything that can sit between the throw and the pcall, such as the
    telemetry DLL's `lua_call` IAT detour, must use `extern "C-unwind"`;
    a plain `extern "C"` frame there aborts the process. When in doubt,
    use `-unwind`: it costs nothing.
- **Two DLLs can MinHook the same target.** MinHook's trampoline builder
  relocates an existing `E9 rel32` at the target, so the second hook chains
  onto the first. It overwrites exactly 5 bytes, so the prologue after
  offset 5 is intact and a fingerprint can check it. The two MinHook copies
  do not coordinate. Re-read the prologue between `MH_CreateHook` and
  `MH_EnableHook`, and never unhook while the other DLL is loaded: unhooking
  restores the saved original bytes over the chained hook.
- **Confirmed layouts (Ghidra, and the engine's own sender at `0x00c6fc40`).**
  - The `MethodDescription` name is an MSVC `std::string`: buffer at `+4`,
    size `+0x14`, capacity `+0x18`, inline when the capacity is below 16.
    `MethodDescription` elements are `0x50` bytes.
  - Stream vtable: `+4` is `retrieve(n)`, `+8` is `remainingLength()`.
  - `0x00dd2b80` copies `remainingLength()` bytes into an entity's message
    queue, which is strong evidence the stream holds one message.

Related: [[i686-test-exe-uac-installer-detection]].
