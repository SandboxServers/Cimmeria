# AoI Entity Introduction Protocol

## Overview

When a ghost (NPC/object) entity enters a player's Area of Interest, the client
needs THREE phases to fully register the entity:

## Phase 1: Entity Creation + Position (BaseApp -> Client)
Triggered by `CachedEntity::onEntityVisible()` in `cached_entity.cpp:173`.

```
CREATE_ENTITY (0x09)     -- entityId, classId, idAlias=0xFF, 0x00, 0x00
UPDATE_AVATAR (0x10)     -- entityId, position, packedVelocity, flags, yaw, pitch, roll
```

Code: `client_handler.cpp:492-504` (createEntity + moveEntity)

## Phase 2: Property Cache Deltas (BaseApp -> Client)
Triggered by `CachedEntity::sendDeltas()` in `cached_entity.cpp:204`.

These are cached entity method calls stored on the BaseApp per property set.
Each is an entity method call message (0x80+N). The specific methods depend
on the entity type, driven by `createOnClient()` Python cascade.

## Phase 3: CellApp createOnClient() (CellApp -> BaseApp -> Client)
Triggered by `sendRequestEntityUpdate()` in `cached_entity.cpp:232`.
The CellApp receives REQ_ENTITY_UPDATE, calls entity.createOnClient(mailbox),
which triggers Python to send entity method calls.

### SGWMob createOnClient() call chain:
1. `SGWSpawnableEntity.createOnClient(mailbox)`:
   - `onEntityProperty(GENERICPROPERTY_DatabaseId, speakerId)` (if template)
   - `onKismetEventSetUpdate(kismetEventSetId)` (if non-zero)
   - `onStaticMeshNameUpdate(staticMeshName, bodySet)` (if set)
   - `InteractionType(interactionType)`
   - `onBeingNameIDUpdate(beingNameId)` (if non-zero)
   - `onEntityFlags(entityFlags)`
   - **`onVisible(1)`** <-- CRITICAL: makes entity visible
2. `SGWBeing.createOnClient(mailbox)`:
   - `onLevelUpdate(level)`
   - `onTargetUpdate(targetId)`
   - `onBeingNameUpdate(beingName)` (if not empty)
   - `onAlignmentUpdate(alignment)`
   - `onFactionUpdate(faction)`
   - `onStateFieldUpdate(stateField)`
   - `onMeleeRangeUpdate(meleeRange)` (if non-zero)
   - `onArchetypeUpdate(archetype)` (if not ARCHETYPE_Any)
   - `sendStats(mailbox, False, False)`
3. `SGWMob.createOnClient(mailbox)`:
   - `onAggressionOverrideUpdate(aggressionOverride)` (if set)
   - `onEntityProperty(GENERICPROPERTY_MobAggression, aggressionOverride)` (if set)
   - `onEntityProperty(GENERICPROPERTY_AmmoTypeId, ammoTypeId)`

## enterAoI() (Re-entering entity already cached on client)
When a previously-seen entity re-enters AoI, the C++ does NOT re-create it.
Instead it sends `onVisible(true)` via `client_handler.cpp:507-514`:
```cpp
bundle.beginEntityMessage(0x08, entityId);  // method index 8 = onVisible
bundle << (uint8_t)0x01;                     // visible = true
```
Wire: msg_id=0x88 (0x80|0x08), entity_id, 0x01

## Client Method Indices for SGWSpawnableEntity (0-based)
These need verification but from the .def ordering:
0: onStaticMeshNameUpdate
1: onSequence
2: onEntityMove
3: InteractionType
4: onEntityFlags
5: getInteractions
6: toggleInteractionDebugging
7: onEntityProperty
8: **onVisible**
9: onKismetEventSetUpdate
10: onEntityTint
11: onBeingNameIDUpdate

## Viewport Association
- CREATE_ENTITY does NOT include a space ID
- The client associates the entity with the player's viewport/space
- "Viewport for entity X is unknown" = entity not properly associated
- The viewport was established by SPACE_VIEWPORT_INFO (0x08) during world entry
- All ghost entities on the same channel share the player's viewport

## Rust Rewrite Gap
The Rust `EnteredAoI` handler at `world_entry.rs:497` only sends Phase 1.
It needs to also send the createOnClient() property cascade (Phases 2/3).
Without at minimum `onVisible(1)`, entities are created but invisible.
