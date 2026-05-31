---
name: ghidra-trainer-addresses
description: Ghidra RE addresses for trainer-related functions in SGW.exe — treat as hypotheses, re-verify in Ghidra before pinning
metadata:
  type: project
---

Source files: `docs/reverse-engineering/decompiled/01_sgw_game_classes.c`, `13_other_game.c`, `14_standalone_named.c`.

| Symbol | Address | Notes |
|--------|---------|-------|
| `register_NetIn_onTrainerOpen` | returns string `"Event_NetIn_onTrainerOpen"` | In `14_standalone_named.c:288531` |
| `CME_EventSignal_VEvent_NetIn_onTrainerOpen___TypedEmitInfo__vfunc_0` | body at `FUN_00d80030`, emitter at `0x00d80090` | TypedEmitInfo destructor stub; body is the actual TypedEmitInfo |
| `SGWNetworkManager_VEvent_NetOut_TrainAbility___EventHandler__vfunc_0` | calls `FUN_00d5a810` | Network send handler for TrainAbility outbound |
| `Event_SlashCmd_TrainAbility` vtable | `0x018441f8` | GM slash-command variant, not the live wire path |
| `SGWScriptedWindow_X_UEvent_UI_TrainerOpen___GameEventHandler__vfunc_0` | calls `FUN_00cdfba0` | Client-side trainer window open handler |
| `SGWScriptedWindow_X_UEvent_UI_TrainerUpdate___GameEventHandler__vfunc_0` | calls `FUN_00cdfa30` | Client-side trainer window update (resend list path) |
| `register_NetIn_AbilityTreeInfo` | returns `"Event_NetIn_AbilityTreeInfo"` | Separate from onTrainerOpen; maps to `onAbilityTreeInfo` alias |

## Key finding
`Event_NetIn_AbilityTreeInfo` is a distinct server→client event registered adjacent to `onTrainerOpen` in the init sequence. It maps to `onAbilityTreeInfo` (confirmed `14_standalone_named.c:296784`). It is NOT part of the trainer-open payload — it is for the ability-tree UI panel. Do not conflate.

## Decompiler bodies not yet captured
- `FUN_00d80030` — onTrainerOpen TypedEmitInfo body (actual deserializer)
- `FUN_00d5a810` — TrainAbility network serializer body

These are not needed for the current implementation since `.def` is authoritative. Capture only if client trainer UI fails to populate (wire format mismatch hypothesis).
