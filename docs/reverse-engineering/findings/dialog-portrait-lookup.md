# RE Finding: Dialog Portrait and Speaker-Name Lookup

**Confidence**: HIGH (portrait entity path), MEDIUM (speaker-name fallback trigger — Lua script not recovered)
**Date**: 2026-05-25
**Phase**: V5 investigation (targeted, single-session)
**Sources**:

- `FUN_00d26850` — DialogController constructor (CME subscriber registration)
- `FUN_00d25310` — `Event_NetIn_DialogDisplay` handler
- `FUN_00d24f10` — core dialog-show function
- `FUN_00d22c90` — entity slot-pin function
- `FUN_00c67bd0` — GameEntityManager slot-17 mapping + `Event_UI_UnitMappingChanged` emitter
- `FUN_015e4d10` — `CookedData_DialogScreenType` PAK parser
- `FUN_00e6a430` — PortraitManager entity lookup
- `FUN_00ac58d0` / `FUN_00ae2b20` — `createCharacterPortrait` Lua binding + implementation
- `0x00ad4ae5` — Lua method table entry for `createCharacterPortrait`
- `0x00cc39b7`/`0x00cc39c6` — `Event_UI_UnitMappingChanged` Lua binding in `FUN_00cc33f0`
- `docs/reverse-engineering/decompiled/14_standalone_named.c` line ~2797 — CookedDataDialogs.pak load
- `docs/reverse-engineering/decompiled/09_game_ui_visuals.c` — PortraitManager body
- `docs/reverse-engineering/address-map.md` — prior finding: `GENERICPROPERTY_DatabaseId=9`

---

## Questions Investigated

This finding answers four questions raised before the server-side fix for the dialog portrait/name bugs:

- **A** — Dialog UI render path: what C++ functions execute between dialog-screen open and portrait/name population?
- **B** — Speaker → portrait actor: does the client search AoI by `GENERICPROPERTY_DatabaseId`? Or by entity ID? Or via a dedicated wire method?
- **C** — Speaker → name string: how does the name fall back to the player in the Col Marsh case?
- **D** — Required server-side wire data: is `onEntityProperty(GENERICPROPERTY_DatabaseId=9)` sufficient, or is another method required?

---

## Architecture: Two Independent Tracks

The dialog window resolves portrait and name through two independent lookup pipelines that share no runtime state at display time.

---

## Track 1 — Portrait Actor Lookup

### Summary

The portrait is populated by a Lua callback that fires when `Event_UI_UnitMappingChanged` delivers a non-zero entity to GameEntityManager slot 17 ("DialogSpeaker"). The entity is identified by its wire `EntityId` — the same numeric ID present in the `Event_NetIn_DialogDisplay` message — via `LookupEntityListenerEntry`. The `GENERICPROPERTY_DatabaseId` field is NOT searched at portrait time.

### Call Chain (all confirmed)

```
Event_NetIn_DialogDisplay (wire method 105)
  └─ FUN_00d25310            [DialogController handler @ 0x00d25310]
       match: param_1[1] == dialog->entityId
       └─ FUN_00d25200       [activate dialog]
            └─ FUN_00d24f10  [core dialog-show @ 0x00d24f10]
                 ├─ FUN_00d22c90(dialog, 0x1b58)  [pin player-side slot]
                 ├─ FUN_00d22c90(dialog, 0x1bbc)  [pin NPC-side slot]
                 │    └─ LookupEntityListenerEntry(GameEntityManager, npc_entity_id, ...)
                 │         → returns listener entry if entity is in AoI listener table
                 │         → calls FUN_00d06ca0 → FUN_00d067e0 to pin entity
                 └─ FUN_00d27b80(pvVar1, 0, puVar2, 1)
                      → emits Event_UI_DialogDisplay (carries DialogID only)

GameEntityManager slot registration (concurrent path):
FUN_00c67bd0(GameEntityManager, 0x11, entity_id)   [slot 17 = "DialogSpeaker"]
  └─ emits Event_UI_UnitMappingChanged
       └─ SGWScriptedWindow Lua dispatch (FUN_00cc33f0 @ 0x00cc39b7/0x00cc39c6)
            └─ Lua: createCharacterPortrait(portraitImagesetName, entity_id, ...)
                 └─ FUN_00ac58d0 → FUN_00ae2b20
                      └─ ImagesetManager::getImageset(portraitName)
                      └─ FUN_00e6b890(PortraitManager, entity_id, ...)
                           └─ FUN_00e6a430: LookupEntityListenerEntry(GameEntityManager, entity_id, ...)
```

### Key Evidence

**`FUN_00d25310` entity-ID match** (decompiled, confirms wire EntityId is the lookup key):

```c
// param_1[1] = wire EntityId from Event_NetIn_DialogDisplay
// *(int *)(*(*(dialog+0x10)+8) + 0x14) = dialog->entityId stored at creation time
if (*(int *)(param_1 + 4) == *(int *)(*(*(dialog+0x10)+8) + 0x14)) {
    FUN_00d271c0(*(void**)(dialog+0x10), dialog_ptr);
    FUN_00d25200(this, dialog_ptr);
}
```

**`FUN_00d22c90` AoI lookup** (decompiled, confirms `LookupEntityListenerEntry` uses entity ID, not DatabaseId):

```c
uVar7 = LookupEntityListenerEntry(GameEntityManager, pvIgnored, uVar7, unaff_SI);
if ((uVar7 != 0) && (piVar2 = *(int **)(uVar7 + 8), piVar2 != NULL)) {
    FUN_00d06ca0(ppuVar4, puVar5, puVar6, piVar2);  // pin entity
    FUN_00d01c50(local_18, uVar7, '\x01');
    FUN_00d01bd0(local_18, (int)param_1, 0);
    FUN_00d00ac0((int)local_18);
}
```

**`FUN_00c67bd0` slot-17 emit** (decompiled, confirms UnitMappingChanged carries entity to Lua):

```c
FUN_00a372f0(this_00, 0, &iStack_3c,
             &CME::EventSignal::NoSubject::RTTI_Type_Descriptor,
             (type_info *)&Event_UI_UnitMappingChanged::RTTI_Type_Descriptor);
FUN_00e6b6c0(*(void **)(iVar4 + 0x58), param_1);  // notifies PortraitManager subsidiary
```

**`FUN_00e6a430` PortraitManager entity lookup** (decompiled, confirms same `LookupEntityListenerEntry` pattern — no DatabaseId search):

```c
uVar13 = LookupEntityListenerEntry(GameEntityManager, param_2, uVar13, unaff_DI);
if ((uVar13 == 0) || (this_00 = *(int **)(uVar13 + 8), this_00 == NULL)) return NULL;
```

**`createCharacterPortrait` Lua method table entry** (`0x00ad4ae5`):

```asm
00ad4ae5: PUSH 0xac58d0     ; createCharacterPortrait C++ function
00ad4aea: PUSH 0x1954fe0    ; "createCharacterPortrait"
00ad4aef: PUSH ESI          ; lua state
00ad4af0: CALL 0x00403ec0   ; register Lua method
```

### Answer to Question B

**The portrait lookup does NOT search by `GENERICPROPERTY_DatabaseId`.** It searches by the wire `EntityId` from `Event_NetIn_DialogDisplay`. If the entity is in the client's AoI listener table under that entity ID, the portrait renders. If not, slot 17 is never populated and the portrait remains blank.

### Answer to Question D

`onEntityProperty(GENERICPROPERTY_DatabaseId=9, value=speaker_id)` is NOT sufficient to populate the portrait. The required sequence is:

1. The NPC entity must be visible in the client's AoI (i.e., it must have gone through the entity-creation pipeline and be in `LookupEntityListenerEntry`'s table).
2. `Event_NetIn_DialogDisplay` must carry the same wire `EntityId` as the AoI-registered entity.

If (1) holds and (2) delivers the right `EntityId`, the slot-17 pin fires automatically and the portrait renders. No additional wire method is needed. The `GENERICPROPERTY_DatabaseId` broadcast is orthogonal to portrait display — it populates an index used by other subsystems, not this one.

---

## Track 2 — Speaker Name String

### Summary

The speaker name is resolved by the Lua dialog script from the CookedData `DialogScreenType` record, not from a wire event. Each screen's `SpeakerID` is stored at offset `+0x1C` in the parsed binary PAK record (`piVar2[7]`). Lua looks up the name string for that `SpeakerID` from the `speakers` table in CookedData. When the name is empty or the `SpeakerID` maps to no record, Lua falls back to the player's own name.

### Key Evidence

**`FUN_015e4d10` CookedData PAK parser** (decompiled, confirms `SpeakerID` is read from PAK, not wire):

```c
pcVar3 = FUN_00a3c1e0((int)param_1, (byte *)"ScreenID", 1);
iVar1 = FUN_00a3d050((int)param_1, pcVar3, puVar8);  // → piVar2[5]

pbVar4 = FUN_00a3c1e0((int)param_1, &DAT_01b23c98, 1);
iVar1 = FUN_00a3d1e0((int)param_1, pbVar4, piVar9);  // → piVar2[6] (string field)

pcVar3 = FUN_00a3c1e0((int)param_1, (byte *)"SpeakerID", 1);
iVar1 = FUN_00a3d050((int)param_1, pcVar3, puVar8);  // → piVar2[7] = offset +0x1C
```

The `"SpeakerID"` field name (literal string at the call site) and the integer read into `piVar2[7]` are unambiguous. This is a PAK-time parse, not a wire-time receive.

**`Event_UI_DialogSpeakerChanged` and `Event_UI_DialogDisplay` both deliver data to Lua** via SGWScriptedWindow handlers (confirmed from `docs/reverse-engineering/decompiled/01_sgw_game_classes.c` lines 4135–4161). The decompiled stubs call `FUN_00cd9580` (DialogDisplay) and `FUN_00ccca70` (4-arg Lua dispatcher). The CME event carries `DialogID` only — no speaker name, no `SpeakerID`. Lua must read the screen record from the CookedData cache using that `DialogID`.

### Answer to Question C

The player name appears in the Col Marsh case because `SpeakerID=256` has an empty (`''`) name in the `speakers` CookedData table. When Lua resolves an empty string from the CookedData lookup, it substitutes the current player's display name. This is the built-in fallback for zero or missing speaker names.

Note: `entity_templates.speaker_id=941` is the server-side column used to populate `GENERICPROPERTY_DatabaseId` on the entity. It does NOT feed into CookedData at runtime — CookedData is baked at game-ship time. The relevant fix is in the `dialog_screens.speaker_id` column (which maps to the PAK `SpeakerID` field), not in `entity_templates.speaker_id`.

---

## Symptom Diagnoses

### Prisoner 329 (dialog_id 2300) — blank portrait, correct name

- Name correct: `SpeakerID` in the `dialog_screens` PAK record resolves to "Prisoner 329" in CookedData. This track is working.
- Portrait blank: the `EntityId` in the server's `onDialogDisplay` wire event does not match the entity ID registered for Prisoner 329 in the client's AoI listener table. The target-selector widget sees the entity (it uses the AoI grid/zone system), but `LookupEntityListenerEntry` does not find it under the entity ID the server sent. Result: `FUN_00d22c90` returns early (`uVar7 == 0`), slot 17 is never set, portrait never renders.

### Future Col Marsh (dialog_id 4001) — player name on all screens, blank portrait

- Name wrong: `SpeakerID=256` in `dialog_screens` has an empty string in the CookedData `speakers` table. Lua fallback substitutes the player name. The PAK entry must be corrected to point to the right speaker record, or the speakers table entry for 256 must be populated.
- Portrait blank: same entity-ID mismatch as Prisoner 329. Additionally, Future Col Marsh may not be spawned as an AoI entity at the dialog trigger location — Marsh's dialog fires before or during a cutscene, not while standing next to a visible NPC.

---

## Answer to Question A — Full Render Path

```
Wire arrives:  Event_NetIn_DialogDisplay (method 105)
               payload: EntityId, DialogID

FUN_00d25310:  matches EntityId → finds AvailableDialog → calls FUN_00d25200

FUN_00d24f10:  determines player/NPC slot (screen byte 0x28 == 3 ?)
               calls FUN_00d22c90(dialog, 0x1b58) — player slot
               calls FUN_00d22c90(dialog, 0x1bbc) — NPC slot
               both internally: LookupEntityListenerEntry(GameEntityManager, entityId, ...)
               if found: pins entity → fires into slot 17 via FUN_00c67bd0

FUN_00c67bd0:  GameEntityManager[slot 17] = entityId
               emits Event_UI_UnitMappingChanged

SGWScriptedWindow:  receives UnitMappingChanged
               dispatches to Lua dialog script

Lua:           reads slot-17 entity
               calls createCharacterPortrait(imageset, entity, ...)
               FUN_00ac58d0 → FUN_00ae2b20:
                 ImagesetManager::getImageset(imageset)
                 FUN_00e6b890(PortraitManager, entity, ...)
                   FUN_00e6a430: LookupEntityListenerEntry again
                   if found: render head mesh into portrait circle

FUN_00d27b80:  emits Event_UI_DialogDisplay (carries DialogID)

SGWScriptedWindow:  receives DialogDisplay
               dispatches to Lua dialog script

Lua:           uses DialogID to look up dialog from CookedData cache
               reads current screen's SpeakerID from piVar2[7]
               looks up name from CookedData speakers table
               if name empty or SpeakerID==0: substitutes player name
               sets name label text
```

---

## Implementation Impact (Server-Side)

### Fix 1 — Portrait (both NPCs)

The server must ensure `EntityId` in `onDialogDisplay` equals the entity ID the client received during entity creation for that NPC. The most robust path:

- Track the entity ID assigned during `CREATE_CELL` or equivalent for the dialog NPC.
- Pass that entity ID — not a database ID, not a template ID — as the `EntityId` field in the `Event_NetIn_DialogDisplay` wire event.

### Fix 2 — Col Marsh name

Either:
- Update the `speakers` table entry for `SpeakerID=256` to have name `"Future Col Marsh"` (or the correct localized string); or
- Update `dialog_screens` rows for `dialog_id=4001` to reference a `SpeakerID` that already has the correct name in CookedData.

The `entity_templates.speaker_id=941` field is irrelevant to name display. It is used only for the `GENERICPROPERTY_DatabaseId` AoI broadcast, not for CookedData name lookup.

### Fix 3 — Prisoner 329 entity visibility

If Prisoner 329 is in its cell and the dialog fires while the player is adjacent, the entity should already be in AoI. Verify whether the server is:
(a) sending the wrong `EntityId` in `onDialogDisplay` (most likely), or
(b) not including Prisoner 329 in the AoI update before dialog fires (race condition).

If (b), the server must ensure the NPC entity creation messages complete before the dialog display message is sent.

---

## Open Questions

1. **Constants `0x1b58` / `0x1bbc` (decimal 7000 / 7100)**: These are passed to `FUN_00d22c90` as the second argument — likely screen-type or dialog-slot enum values distinguishing player vs NPC sides. Exact semantics not confirmed; functional behavior confirmed by context.

2. **`Event_UI_DialogSpeakerChanged` emitter**: Not fully traced. Likely fired from screen-transition logic within `DialogController` when the active screen changes and the speaker entity differs. Does not affect the main portrait path (which is driven by `UnitMappingChanged`) but may affect per-screen name updates.

3. **Lua script source**: The dialog Lua script that reads `SpeakerID` and calls `createCharacterPortrait` was not recovered — it is in the game's Lua package, not the binary. The call chain above is inferred from the C++ bindings and CME event routing. The fallback-to-player-name behavior is inferred from symptom observation; the exact Lua condition (empty string? SpeakerID==0?) is not confirmed from source.

4. **Screen byte `0x28 == 3` condition**: The decompiled `FUN_00d24f10` selects player vs NPC slot based on a byte at offset `0x28` of the screen record. The exact enum values are not catalogued here.

---

## Summary Table

| Question | Answer | Confidence |
|----------|--------|------------|
| A — render path | Wire EntityId → AoI lookup → slot 17 → UnitMappingChanged → Lua createCharacterPortrait | HIGH |
| B — portrait lookup mechanism | Wire EntityId via LookupEntityListenerEntry; DatabaseId NOT used | HIGH |
| C — name fallback trigger | SpeakerID → CookedData speakers table; empty string → player name | HIGH (mechanism), MEDIUM (exact fallback condition) |
| D — required server wire data | EntityId in onDialogDisplay must match AoI entity ID; DatabaseId broadcast insufficient alone | HIGH |
