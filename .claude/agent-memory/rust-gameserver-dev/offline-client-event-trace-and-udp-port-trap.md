---
name: offline-client-event-trace-and-udp-port-trap
description: Without Ghidra, identify which CME event an SGW.exe function raises from raw bytes plus MSVC RTTI; client Lua is the fastest receiver trace; wireclient Orchestrator starts fail with WSAEACCES when a UDP port comes from a TCP-ephemeral bind
metadata:
  type: reference
---

**Tracing a client receiver with no Ghidra instance (SS-D2, 2026-09-27).**

- Client Lua first: the client's `Working/SGWGame/Content/UI/Core/**/*.lua` (93 files). `Property.X` + `Events.PropertyUpdated` is the `onEntityProperty` generic-property table (TrainingPoints, AccessLevel, PVPFlag...). `getUnitProperty(unit, Property.X)` reads it. This settled the PvP-flag vehicle (`onEntityProperty(4, v)`, not the `pvpFlag` CELL_PUBLIC property).
- Bytes: a 30-line Python PE reader (section table → file offset) is enough; the system Python is 32-bit 3.13 and `capstone` has no wheel, so read raw bytes and decode `E8 rel32` calls by hand.
- Which CME event a native handler raises: the queued event node constructor writes the event's `type_info` pointer at `node+8` and a `TypedEmitInfo<Event_X>` vtable. `type_info+8` is the mangled name (`.?AUEvent_UI_DuelTimerStart@@`); for a vtable, `dword[vt-4]` is the complete-object locator, `dword[col+12]` the type descriptor, `+8` the name. Proved `onTimerUpdate` type 14 → `Event_UI_DuelTimerStart` (`0x00dec9e0` → `0x00dfdcb0` → `0x00df57c0`).

**wireclient harness trap:** `ephemeral_port()` binds TCP `:0`, but the BaseApp port is UDP. On this host Windows hands out TCP-ephemeral ports inside a Hyper-V UDP exclusion range (`netsh interface ipv4 show excludedportrange protocol=udp`) for long stretches, so every `Orchestrator` start fails with os error 10013 and it looks like a sandbox problem (disabling the sandbox changes nothing). `support::ephemeral_udp_port()` fixes the base port.

See [[stat-with-no-consumer-trap]] for the server-side mirror: a property existing in a def says nothing about who reads it.
