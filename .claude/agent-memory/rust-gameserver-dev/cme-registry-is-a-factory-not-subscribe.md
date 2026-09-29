---
name: cme-registry-is-a-factory-not-subscribe
description: SGW.exe 0x00a5c0f0/0x00a5c150 are the CME event-factory registry (create / count by std::string), not LookupByName/Subscribe; the DLL's CME subscriber install never worked; how the replacement hook and string reads work
metadata:
  type: reference
---

Found 2026-09-28 (lab-automation client seams, PR on branch worktree-agent-a68b33e5fe642befb).

- `0x0155f790` returns `0x01f11fc4`, a function-local static `std::map<std::string, factory>`, filled by `CMERegistry__RegisterAllEventEmitHandlers` (`0x005c75d0`) keyed by class name (`Event_Action_MouseClick`...).
- `0x00a5c0f0` = create: `thiscall(registry, const std::string&) -> event*`, `ret 4`. `Client_NetIn_EntityMethodDispatch` calls it for every routed inbound method, so hooking it names every accepted NetIn method (`client.cme.event`).
- `0x00a5c150` = `count(name)`; subscribes nothing. The old install passed a C string: never worked. Removed.
- Real subscribe candidates: `FUN_00a37790` / `FUN_00a374a0` (called with a new `MemberCallback`, ctor e.g. `0x00d34cb0`). The invoker `0x00e04570` pushes TWO stack args; a handler must `ret 8`.
- MSVC 2008 strings on i686: 28 bytes, buf/ptr at +4, size +0x14, cap +0x18; heap when cap >= 16 (narrow) or >= 8 (wide). CEGUI `String` in this client is `std::wstring`. Reader: `crates/client-telemetry/src/msvc_string.rs`.

**How to verify offline:** 64-bit Python + capstone works; call it by full path (`...\WindowsApps\PythonSoftwareFoundation.Python.3.13_...\python.exe`), `py -V:3.13` fails from some cwd. Headless Ghidra fails with `LockException` when another session holds the project; don't delete the lock then.

**Test trap:** the two `hooks::primitives` inline-hook tests patched the same fn concurrently (1-in-6 AV on i686); now serialized with a mutex.

Related: [[telemetry-anchor-audit-and-hookgate]].
