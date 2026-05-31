---
name: wire-format-trainer
description: onTrainerOpen (method 113) and trainAbility (cell method 77) wire layouts — confirmed from SGWPlayer.def and alias.xml
metadata:
  type: project
---

## onTrainerOpen (server → client, client method index 113)

Source: `entities/defs/SGWPlayer.def:1194-1198`, `entities/defs/alias.xml:417-422`, `docs/protocol/client-method-dispatch-table.md:255`.

```
INT32   TrainerID            // NPC entity id, LE
UINT32  count                // ARRAY element count, LE
[N × (INT32 abilityID, UINT8 trainable)]  // 5 bytes each
INT32   CostToRespec         // Naquadah respec cost (placeholder 1000), LE
```

`trainable`: 0 = cannot train (level too low, missing prereq, or already known), 1 = can train now.

Total frame: `4 + 4 + N×5 + 4` bytes.

Rust implementation: `crates/services/src/cell/cell_methods/player/trainer_interaction.rs:166-173`.

## trainAbility (client → server, cell method index 77)

Source: `entities/defs/SGWPlayer.def:635-638`, `docs/protocol/cell-method-dispatch-table.md:283`.

```
INT32   abilityId   // 4 bytes, LE
```

## "Ability learned" confirmation

No separate message. Path is: `BaseToCellMsg::AbilityGranted` → `send_known_abilities_update` → `onKnownAbilitiesUpdate` (method 101) with full ability array refresh.

## Ghidra event names (RE findings)

- `Event_NetIn_onTrainerOpen` — BigWorld event name for method 113 inbound (confirmed `14_standalone_named.c:288534`)
- `Event_NetOut_TrainAbility` — BigWorld event name for cell method 77 outbound (client→server) (confirmed `01_sgw_game_classes.c:5827`)
- `Event_NetIn_AbilityTreeInfo` — **separate** event, NOT part of trainer-open flow; used for ability-tree UI panel
