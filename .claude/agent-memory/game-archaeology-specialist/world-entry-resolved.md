---
name: world-entry-resolved
description: World-entry open questions Q1-Q4 resolved: ENABLE_ENTITIES=8 bytes (corrected), PostLoad handler=0x00de8430, World_Loaded emitter=0x005541a0, ClientReady wire-send=0x00d43dc0.
metadata:
  type: project
---

# World Entry — Resolved Open Questions

> [!WARNING] Confidence: STALE — Q1 contradicted by V5 W-enable-entities finding
>
> Triaged 2026-05-13 (Phase −0.5 step 4). **Q1 is WRONG.** This file's Q1 says "ENABLE_ENTITIES = 1 byte (stock BigWorld keepBase u8)". V5 W-enable-entities (recorded in `docs/reverse-engineering/findings/world-entry-pipeline.md` §"RESET_ENTITIES + ENABLE_ENTITIES Exchange" / "CONFIRMED (W-enable-entities, 2026-05-13)") confirms **8 bytes** — a SGW-custom `uint64` dummy payload. Disassembly of the static initializer at `0x017bade0–0x017bae07` shows `PUSH 0x8` at `0x017bade9` is the size argument; the `MOV DWORD PTR [EAX], 0x1` at `0x017badf7` that this file misread is a reliability flag, not the size field. Cross-validated against `deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp` line 83: `{Message::CONSTANT_LENGTH, 8, "ENABLE_ENTITIES", true}`.
>
> Q2 / Q3 / Q4 are still correct as far as we know but should also be re-verified during chapter authoring against any new V5 findings before promoting.
>
> PROMOTION TARGET (after Q1 correction): spec.world.world-entry §"RESET_ENTITIES + ENABLE_ENTITIES" + §"Event_Level_PostLoad + Event_World_Loaded + Event_ClientReady handlers"

## World Entry Pipeline — Resolved Open Questions (W-misc-gaps, 2026-05-13)

### Q1: ENABLE_ENTITIES payload size

**Answer: 8 bytes** (SGW-custom `uint64` dummy payload).
- ~~Pre-V5 claim: 1 byte (stock BigWorld `keepBase` u8) — **wrong**.~~
- Confirmed via V5 W-enable-entities (2026-05-13). Static initializer at `0x017bade0–0x017bae07`: `PUSH 0x8` at `0x017bade9` is the size argument; the `MOV DWORD PTR [EAX], 0x1` at `0x017badf7` is a reliability flag, not the size field. The pre-V5 read of `0x017bae02` misidentified that flag as `size`.
- Cross-validated against `deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp` line 83: `{Message::CONSTANT_LENGTH, 8, "ENABLE_ENTITIES", true}`.
- See `docs/reverse-engineering/findings/world-entry-pipeline.md` §"RESET_ENTITIES + ENABLE_ENTITIES Exchange" / "CONFIRMED (W-enable-entities, 2026-05-13)" for the full reconciliation.

### Q2: GameProxyPlayer Event_Level_PostLoad handler

**Handler: `GameProxyPlayer_HandleEvent_Level_PostLoad` (0x00de8660)** → wrapper → `FUN_00de8430` (0x00de8430).
- Registered in `FUN_00df4270` (main callback registration, 35 subscriptions).
- Alternate handler `LAB_00de9e60` registered only on account-disconnect path (`FUN_00def710`).
- Body: reads UE3 PlayerController from `GEngine+0x2D0/+0x40`, sets `*(pController+0x5A)=2` (input mode),
  copies transform (+0xDC/+0xE4/+0xE8/+0xF0) to vehicle if any. Guarded by `DAT_01eb082c` (editor flag).

### Q3: Event_World_Loaded emitter

**Emitter: `FUN_005541a0` (0x005541a0)**.
- Called from `FUN_007100d0` (0x007100d0) — reads global WorldInfo* from `DAT_01ee2684`.
- `FUN_007100d0` has NO static callers — invoked via UE3 streaming completion callback table.
- Fires after: (a) no sub-level has pending-stream bit set, (b) all entities at level have `+0x164==0`.
- Guard: `DAT_01ee2b6c` — prevents double-emit.
- Distinct from `Event_Level_PostLoad` which fires per-level from `EntityManager_PostLoadMap` (0x00dd0b00).

### Q4: SGWNetworkManager ClientReady handler wire-send

**Handler: `SGWNetworkManager_EventHandler_ClientReady_invoke` (0x00d43dc0)**.
- Created by `FUN_00d57030` (0x00d57030).
- Body: reads `pMethodDesc` from `this+4`, `pArgData` from `this+8`,
  calls `EnsureEntityRpcRegistryAllocated()` + `RouteOutgoingEntityRpc(param_2, pMethodDesc, pArgData)`.
- `pMethodDesc` = enableEntities method descriptor; `pArgData` = 1-byte keepBase arg.

### Q5/Q6: Still open

- Q5 (resetEntities CONSTANT_LENGTH): not investigated.
- Q6 (setupStargateInfo method 65): "setupStargateInfo" string NOT in binary — server-side name only.
  `DBGateInfo` CookedData RTTI at `0x004288e0` may be data carrier. Requires tracing Extended method dispatch.
