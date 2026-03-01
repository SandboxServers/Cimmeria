# Entity Property Synchronization Protocol

> **Last updated**: 2026-03-01
> **RE Status**: Partially documented from BigWorld 2.0.1 entitydef source + Cimmeria .def files
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/`, `entities/defs/`, `src/mercury/base_cell_messages.hpp`

---

## Overview

BigWorld entities have properties that are automatically synchronized between server components and the client. Each property has flags that control where it exists and who receives updates. The synchronization protocol uses Mercury messages to propagate property changes.

## Property Flags

Defined in `data_description.hpp` (`EntityDataFlags`):

| Flag | Value | .def Name | Description |
|------|-------|-----------|-------------|
| `DATA_GHOSTED` | 0x01 | `CELL_PUBLIC` | Synced to ghost entities (other CellApps) |
| `DATA_OTHER_CLIENT` | 0x02 | `OTHER_CLIENTS` | Sent to other players who can see this entity |
| `DATA_OWN_CLIENT` | 0x04 | `OWN_CLIENT` | Sent only to the owning client |
| `DATA_BASE` | 0x08 | `BASE` | Exists on the base entity |
| `DATA_CLIENT_ONLY` | 0x10 | `CLIENT_ONLY` | Static client-side data only |
| `DATA_PERSISTENT` | 0x20 | `Persistent` | Saved to the database |
| `DATA_EDITOR_ONLY` | 0x40 | `EDITOR_ONLY` | Editor-only data |
| `DATA_ID` | 0x80 | `Identifier` | Indexed database column |

### Composite Flags in .def Files

The .def files use composite flag names:

| .def Flag | Equivalent Bits | Where it Exists |
|-----------|----------------|-----------------|
| `CELL_PRIVATE` | (none of the above) | Cell only, not synced anywhere |
| `CELL_PUBLIC` | `DATA_GHOSTED` | Cell + ghosts on other CellApps |
| `OTHER_CLIENTS` | `DATA_GHOSTED + DATA_OTHER_CLIENT` | Cell + other clients |
| `OWN_CLIENT` | `DATA_OWN_CLIENT` | Cell + own client |
| `ALL_CLIENTS` | `DATA_GHOSTED + DATA_OTHER_CLIENT + DATA_OWN_CLIENT` | Cell + all clients |
| `BASE` | `DATA_BASE` | Base entity only |
| `BASE_AND_CLIENT` | `DATA_BASE + DATA_OWN_CLIENT` | Base + own client |

## Property Distribution in SGW Entity Defs

### SGWPlayer Example Properties

From `entities/defs/SGWPlayer.def`:

```xml
<!-- Visible to all clients (OTHER_CLIENTS equivalent via CELL_PUBLIC) -->
<playerName>
    <Type>WSTRING</Type>
    <Flags>CELL_PUBLIC</Flags>
    <Persistent>true</Persistent>
</playerName>

<!-- Known abilities visible to other players -->
<knownAbilities>
    <Type>ARRAY <of>INT32</of></Type>
    <Flags>CELL_PUBLIC</Flags>
</knownAbilities>

<!-- Private to the cell, not synced -->
<experience>
    <Type>INT32</Type>
    <Flags>CELL_PRIVATE</Flags>
</experience>

<!-- Base-only, references another entity -->
<account>
    <Type>MAILBOX</Type>
    <Flags>BASE</Flags>
</account>
```

## Synchronization Flow

### Property Change on CellApp

When a cell entity property changes:

1. Python script sets `self.propertyName = newValue`
2. The entity system detects the change and determines distribution flags
3. Based on flags, the change is sent to:
   - **Own client**: Via the Base->Client channel
   - **Other clients**: Via AoI witnesses through the Base->Client channels
   - **Ghost entities**: To other CellApps (not used in single-CellApp SGW)
   - **Base entity**: If `DATA_BASE` flag is set

### Message Format: Property Update

Property changes are sent to the client as entity method calls or property update messages. In BigWorld, property changes use a path-based system:

```
PropertyChange message:
+---------------+----------------+------------------+
| Property Path | Change Type    | New Value        |
| (variable)    | (1 byte)       | (variable)       |
+---------------+----------------+------------------+
```

From `property_change.hpp`:

| Change Type | Value | Description |
|-------------|-------|-------------|
| `PROPERTY_CHANGE_TYPE_SINGLE` | 0 | Single value changed |
| `PROPERTY_CHANGE_TYPE_SLICE` | 1 | Array slice changed |

Property IDs 0-60 use a compact single-byte encoding. IDs 61+ use extended encoding.

### Cimmeria Cache Stamp System

Cimmeria extends BigWorld's property sync with a cache stamp system for efficiency. From `base_cell_messages.hpp`:

```
CELL_BASE_UPDATE_CACHE_STAMP (0x11):
  uint32    Entity ID
  uint32    PropSet ID     (0 to MaxPropertySets-1)
  uint8     Invalidate     (clear existing cache?)
  Message[] Messages:
      uint8  Message ID
      uint8  Flags          (INCREMENTAL_UPDATABLE)
      uint16 Length
      byte[] Args
```

This allows the BaseApp to cache static entity data (like appearance) and avoid re-sending it to clients who already have it. `MaxPropertySets = 2` limits the number of cacheable property groups per entity.

### Entity Update Request Flags

When a client needs entity data, the Base can request specific update types:

| Flag | Value | Description |
|------|-------|-------------|
| `UPDATE_STATIC` | 0x01 | Static properties (appearance, name) |
| `UPDATE_DYNAMIC` | 0x02 | Dynamic properties (health, state) |
| `UPDATE_UNCACHED` | 0x04 | Properties not yet cached on client |

## Entity Creation Property Streaming

When creating an entity, properties are streamed in a specific order based on domain:

### Data Domains (from `entity_description.hpp`)

| Domain | Value | Description |
|--------|-------|-------------|
| `BASE_DATA` | 0x1 | Properties for the base entity |
| `CLIENT_DATA` | 0x2 | Properties for the client |
| `CELL_DATA` | 0x4 | Properties for the cell entity |
| `EXACT_MATCH` | 0x8 | Must match flags exactly |
| `ONLY_OTHER_CLIENT_DATA` | 0x10 | Only OTHER_CLIENT properties |
| `ONLY_PERSISTENT_DATA` | 0x20 | Only persistent properties |

### CreateBasePlayer Property Stream

When creating the base player entity, the server streams:
1. Entity type ID
2. Entity ID
3. Properties matching `CLIENT_DATA | BASE_DATA`
4. Each property is serialized according to its `DataType`

### CreateCellPlayer Property Stream

When creating the cell player:
1. Space ID
2. Entity ID
3. Position (Vec3)
4. Direction (Vec3)
5. Properties matching `CLIENT_DATA | CELL_DATA`

## Level of Detail (LoD)

BigWorld supports up to 6 LoD levels (`MAX_DATA_LOD_LEVELS`). Properties can be assigned a detail level that controls at what distance they are sent to other clients. Higher detail levels are only sent to nearby clients.

SGW uses `CELL_PUBLIC` for most shared properties without explicit LoD level assignments, suggesting a flat LoD model.

## Event Stamps

Properties can be time-stamped with an `EventNumber` (int32, starting at 1). This allows the client to determine ordering of property changes. The `eventStampIndex` on each `DataDescription` tracks which properties participate in event stamping.

## Related Documents

- [Mercury Wire Format](mercury-wire-format.md) -- Underlying packet protocol
- [Position Updates](position-updates.md) -- Volatile position/direction syncing
- [Entity Type Catalog](../engine/entity-type-catalog.md) -- All entity definitions
- [BigWorld Architecture](../engine/bigworld-architecture.md) -- Entity split model

## TODO

- [x] ~~Reverse engineer the exact property streaming order~~ → Documented in `findings/entity-property-sync.md`: sequential in parse order (Parent→Implements→Own)
- [x] ~~Determine if SGW uses LoD levels~~ → NO. All `<LoDLevels>` tags empty. See `engine/entity-lod-system.md`
- [ ] Map which properties use event stamps vs which use simple overwrite
- [ ] Document the incremental update flag behavior in the cache stamp system
- [x] ~~Verify property ID encoding for >60 properties~~ → IDs 0-59 = 1-byte, 60+ = 2-byte extended. See `findings/entity-property-sync.md`
- [x] ~~Determine exact format of createBasePlayer/createCellPlayer~~ → Fully documented in `findings/entity-property-sync.md`
