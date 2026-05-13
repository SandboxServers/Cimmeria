---
name: world-entry-resolved
description: World-entry open questions Q1-Q4 resolved: ENABLE_ENTITIES=1 byte, PostLoad handler=0x00de8430, World_Loaded emitter=0x005541a0, ClientReady wire-send=0x00d43dc0.
metadata:
  type: project
---

## World Entry Pipeline — Resolved Open Questions (W-misc-gaps, 2026-05-13)

### Q1: ENABLE_ENTITIES payload size
**Answer: 1 byte** (stock BigWorld `keepBase` u8).
- Init site: `0x017bae02` — `MOV DWORD PTR [DAT_01ef2500->size], 1`
- The 8-byte SGW-custom claim was wrong. Correct `docs/protocol/world-entry-phases.md` if encountered.

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
