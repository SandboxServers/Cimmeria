---
name: client-interact-firer-by-id
description: Client right-click interact is pick -> resolver -> firer FUN_00def4b0 (stdcall, entity id); lab client_interact calls the firer directly (N3 native_call)
metadata:
  type: reference
---

Stock RMB: `FUN_00e84b20` (mouse handler) -> resolver `FUN_00e84860` (pixel -> entity id) -> firer `FUN_00def4b0`. The firer is `__stdcall(uint32 entity_id)`, returns void, builds `Event_NetOut_Interact` with property "OverrideTarget". Ghidra shows the parameter as `char` (decompiler narrowing); the stock caller pushes the full word. Verified from the SGW.exe bytes: epilogue `ret 4`, callers do not clean the stack, so it is not cdecl.

`client_interact {entity_id}` (crates/lab `supervisor/world/interact.rs`) calls it through bridge `call_native` (`conv: stdcall`, rebased by `module_info.slide`). Tier N3 `native_call`, never a native pass. Evidence is inconclusive on purpose (server reaction graded from the log/tap), denied beyond 5 m.

**Why:** hover search cannot hit some props/NPC pixels (Kaerlia 100205, Ambernol vial, Ring Transport Controls).
**How to apply:** the live wire result was NOT verified when this landed; the stock checks in front of the firer are not made by the tool: the map-check-dep early-out (`DAT_01eb082c`), the allowed-target filter `FUN_00e85e80` and the seen-flag gate `FUN_00e68570` (`+0x30`/`+0x31`). Confirm with a packet tap before relying on it.
