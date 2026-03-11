# Viewport System Architecture (Client-Side)

## Data Structures on ServerConnection

- `this+0xf84`: **Viewport table** (`std::map<u8, ViewportEntry>`) - viewportId -> {entityId1, entityId2, spaceRef, isNew}
- `this+0xf90`: **Entity-to-viewport chain** (`std::map<int, int>`) - entityId -> viewportByte or vehicleEntityId
- `this+0xf9c`: **Space refcount** (`std::map<int, int>`) - spaceId -> refcount
- `this+0xfa8`: **Current viewport byte** (u8) - set by spaceViewportInfo handler
- `this+0xf7c`: **Pending viewport byte** (char, -1=none) - for vehicle/entity association
- `this+0xf80`: **Pending vehicle ID** (int, -1=none) - for vehicle chain setup
- `this+0x16c`: **Player entity ID** (u32) - set by createBasePlayer

## How spaceViewportInfo (0x08) Sets Up Viewports

Handler at `0x00dda6c0`:
1. Creates/updates entry in viewport table (this+0xf84) indexed by viewportId byte
2. Increments space refcount in this+0xf9c
3. If entityId2 == playerEntityId (this+0x16c):
   - Writes entityId -> viewportByte mapping in this+0xf90
   - Stores viewportByte at this+0xfa8 (current viewport)

## How Ghost Entities Get Viewport Association

CRITICAL FINDING: Ghost entities get their viewport association from the
`avatarUpdate` (UPDATE_AVATAR 0x10-0x2F) message handler, NOT from CREATE_ENTITY.

Each avatarUpdate handler (e.g., FUN_00ddaae0 for 0x10):
1. Reads current viewport byte from this+0xfa8
2. Inserts entityId -> (0xFFFFFF00 | viewportByte) into this+0xf90 map
3. Calls FUN_00dd9d20 which calls svidFollow to verify the chain resolves

The default value for new map entries (in FUN_00de3aa0) is 0xFFFFFF00 (viewport byte 0).

## svidFollow Algorithm (at 0x00dd82d0)

Traverses entity-to-viewport chain:
- Look up entityId in this+0xf90
- If value < 1 (signed): return low byte as viewport byte (SUCCESS)
- If value >= 1: treat as vehicle entityId, follow chain
- If chain exhausts without finding viewport byte: ERROR
  "Viewport for entity %d (immed vehicle %d, top vehicle %d) is unknown"

## Callers of svidFollow

Only TWO callers:
1. `ServerConnection_addMove` (0x00dd9330) - CLIENT sending position updates
2. `FUN_00dd9d20` (0x00dd9d20) - called from ALL 32 avatarUpdate RECEIVE handlers

## Key Insight: First UPDATE_AVATAR Creates the Association

The first UPDATE_AVATAR for a ghost entity CREATES the viewport map entry.
CREATE_ENTITY (0x09) does NOT create it. This means the entity exists in the
EntityManager but has no viewport association until its first position update.

Entity method calls (0x80+N) do NOT go through svidFollow -- they are dispatched
by entityId directly on the EntityManager. So entity methods work fine without
viewport association.

## When svidFollow Fails

The error occurs when:
1. An avatarUpdate arrives for an entity, creating/refreshing the map entry
2. FUN_00dd9d20 is called, which calls svidFollow
3. svidFollow finds the entry but can't resolve it to a viewport byte

This can happen if:
- The viewport byte at this+0xfa8 is invalid (not set by spaceViewportInfo)
- The map was corrupted by RESET_ENTITIES clearing viewport state
- Race condition: viewport not yet established when first AoI entity arrives
