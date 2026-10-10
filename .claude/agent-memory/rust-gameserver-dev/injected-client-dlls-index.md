---
name: injected-client-dlls-index
description: Sub-index of injected client DLL memories (detours, client ABI, lab bridge, telemetry anchors, injector) moved out of MEMORY.md to keep it small
metadata:
  type: reference
---

# Injected client DLLs

- [offline-disasm-and-minhook-detour-tests.md](offline-disasm-and-minhook-detour-tests.md) — capstone under `py -V:3.13` reads SGW.exe offline; MinHook stand-in tests for detours; game calls outside catch_unwind.
- [injected-dll-unwind-and-lua-error-rules.md](injected-dll-unwind-and-lua-error-rules.md) — `thiscall-unwind` detours for C++-EH prologues.
- [entity-method-stream-is-memory-ostream.md](entity-method-stream-is-memory-ostream.md) — onEntityMethod's live stream is a queued MemoryOStream subobject (cursor +0x14, end +0xc), not MemoryIStream.
- [client-handler-abi-and-static-disassembly.md](client-handler-abi-and-static-disassembly.md) — CME handlers are `ret 8` (event, subject); verify `ret N` with capstone on the local QA exe; event-bag getters.
- [lab-probe-traffic-starves-watchdog.md](lab-probe-traffic-starves-watchdog.md) — per-field mem_reads starved the lab heartbeat and killed a healthy client; keep bridge reads coarse.
- [lab-watchdog-load-grace-main-thread-cpu.md](lab-watchdog-load-grace-main-thread-cpu.md) — bridge calls are all main-thread; a world load is seen only as main-thread CPU from outside; 120 s grace.
- [lab-event-store-and-ui-lua-hooks.md](lab-event-store-and-ui-lua-hooks.md) — events_read drains; read via the supervisor store; one UI Lua subscription per window per event.
- [lab-ui-reader-lua-traps.md](lab-ui-reader-lua-traps.md) — stock UI Lua facts behind the UI readers (right-click use, Ctrl-drag split, one chat capture via the events store; lupa offline check.
- [client-patch-send-natives-traps.md](client-patch-send-natives-traps.md) — startEntityMessage sends even offline; microseh masks the ABI; no cpcall around C-function args.
- [telemetry-anchor-audit-and-hookgate.md](telemetry-anchor-audit-and-hookgate.md) — telemetry anchors never ran and 5 were wrong (IAT hint/name RVAs, COL-shifted vtable.
- [cme-registry-is-a-factory-not-subscribe.md](cme-registry-is-a-factory-not-subscribe.md) — 0x00a5c0f0/0x00a5c150 are the CME event-factory map (create/count by std::string); the CME subscribe never worked.
- [dll-boot-testhost-traps.md](dll-boot-testhost-traps.md) — sgw-testhost harness: console children hold a piped stdout (start32::run blocks), restage after DLL edits, derive site counts.
- [cargo-artifact-hardlink-cp-trap.md](cargo-artifact-hardlink-cp-trap.md) — `cp` over a built DLL writes through the hardlink into deps/; `rm` first, `touch` a source to recover.
- [injector-bitness-and-start32-helper.md](injector-bitness-and-start32-helper.md) — x64 launcher injects via the i686 sgw-start32 helper; the WOW64 resolver fails on suspended targets.
- [client-unit-slots-and-actor-pose.md](client-unit-slots-and-actor-pose.md) — Lua units are slots (map at mgr+0x130), pin private slots 7700+; actor pose at +0xDC; worldToPixel only in PreRender.
