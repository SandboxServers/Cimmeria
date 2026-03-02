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

## Event Stamps vs Simple Overwrite: Property Update Ordering

> **Source**: `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/entity_description.cpp` (lines 353-374), `data_description.hpp`, `property_event_stamps.hpp`

### Rule: Only OTHER_CLIENT Properties Get Event Stamps

Event stamps are **exclusively** assigned to properties with the `DATA_OTHER_CLIENT` flag (i.e., properties visible to other players). All other properties use **simple overwrite** with no ordering guarantee.

From `entity_description.cpp`, inside `parseProperties()`:

```cpp
#ifdef MF_SERVER
if (dataDescription.isOtherClientData())
{
    // ... detail level assignment ...
    dataDescription.eventStampIndex( numEventStampedProperties_ );
    numEventStampedProperties_++;
}
#endif
```

The `eventStampIndex` is a sequential integer assigned during entity type parsing. Properties without `DATA_OTHER_CLIENT` have `eventStampIndex_ = -1` (default), meaning they are not stamped.

### Property Categories and Their Update Mechanism

| Property Flag | Event Stamped? | Update Mechanism |
|---------------|---------------|------------------|
| `CELL_PRIVATE` | No | Cell-local only; never sent anywhere |
| `CELL_PUBLIC` (no `OTHER_CLIENT`) | No | Ghosted to other CellApps via simple overwrite |
| `OTHER_CLIENTS` | **Yes** | Sent to other clients with event ordering |
| `OWN_CLIENT` | No | Sent to own client via simple overwrite |
| `ALL_CLIENTS` | **Yes** | Has `OTHER_CLIENT` bit, so event-stamped for other clients |
| `BASE` | No | Base-only; never event-stamped |
| `BASE_AND_CLIENT` | No | Sent to own client; no `OTHER_CLIENT` bit |

### How Event Stamps Work

The `PropertyEventStamps` class (from `property_event_stamps.hpp`) maintains a vector of `EventNumber` values, one per event-stamped property. The type is `int32`, starting at `INITIAL_EVENT_NUMBER = 1`.

```cpp
class PropertyEventStamps {
    void init(const EntityDescription & entityDescription);
    void init(const EntityDescription & entityDescription, EventNumber lastEventNumber);
    void set(const DataDescription & dataDescription, EventNumber eventNumber);
    EventNumber get(const DataDescription & dataDescription) const;
    void addToStream(BinaryOStream & stream) const;
    void removeFromStream(BinaryIStream & stream);
private:
    std::vector<EventNumber> eventStamps_;  // Indexed by eventStampIndex
};
```

When an `OTHER_CLIENT` property changes on the CellApp:

1. The entity's `PropertyEventStamps` records the current `EventNumber` for that property
2. The property change is sent to other clients via `ghostHistoryEvent` messages (in multi-cell BW) or via the AoI update system
3. Receiving clients can use the `EventNumber` to order property changes correctly, even if they arrive out of order

### Why Only OTHER_CLIENT?

- **OWN_CLIENT** properties go to a single client over a reliable ordered channel -- ordering is guaranteed by Mercury
- **CELL_PUBLIC** ghost updates go over internal server channels -- also ordered
- **OTHER_CLIENT** properties fan out to multiple clients through the AoI system, where property changes from different sources (position updates, combat, etc.) may interleave -- event stamps provide a total ordering

### SGW Implications

In SGW's .def files:

- Properties flagged `OTHER_CLIENTS` or `ALL_CLIENTS` get event stamps: e.g., `knownAbilities`, `playerState`, `combatState`
- Properties flagged `CELL_PUBLIC` (without `OTHER_CLIENT`) do NOT get event stamps: e.g., `playerName` (CELL_PUBLIC only in SGW's defs)
- Properties flagged `OWN_CLIENT` or `BASE_AND_CLIENT` never get event stamps

The count of event-stamped properties per entity type is stored in `EntityDescription::numEventStampedProperties_` and used to size the `PropertyEventStamps` vector at entity creation.

## Incremental Update Flag Behavior in the Cache Stamp System

> **Source**: `src/baseapp/entity/cached_entity.hpp`, `src/baseapp/entity/cached_entity.cpp`, `src/mercury/base_cell_messages.hpp`

The cache stamp system is Cimmeria's extension to BigWorld for efficient entity data distribution. It tracks versioned property sets per entity, allowing the BaseApp to send only the changes a client has not yet seen.

### Architecture Overview

```
CellApp                    BaseApp                         Client
   |                          |                              |
   |-- UPDATE_CACHE_STAMP --> |                              |
   |   (entity, propset,     |                              |
   |    messages[])           |                              |
   |                      CachedEntity                       |
   |                      stores versioned                   |
   |                      PropertySets                       |
   |                          |                              |
   |                          |-- sendDeltas() ----------> |
   |                          |   (only messages the        |
   |                          |    client hasn't seen)      |
```

### Data Structures

**PropertySet** -- One per cacheable property group (up to `MaxPropertySets = 2`):

```
PropertySet:
  firstStampId: uint32    -- Version number of the first stamp in the current window
  stamps: vector<CacheStamp>  -- Each stamp = one UPDATE_CACHE_STAMP transaction
  messages: vector<CacheMessage>  -- All messages across all stamps
```

The version number for stamp `i` is `firstStampId + i`. When invalidated, `firstStampId` advances past all current stamps and both vectors are cleared.

**CacheStamp** -- One per `beginCacheStamp()`/`endCacheStamp()` transaction:

```
CacheStamp:
  messages: vector<unsigned int>  -- Indices into PropertySet::messages
```

**CacheMessage** -- An individual entity RPC message within a stamp:

```
CacheMessage:
  messageId: uint8       -- RPC method ID
  flags: uint8           -- Distribution flags (INCREMENTAL_UPDATABLE)
  minVersion: uint32     -- First version that includes this message
  maxVersion: uint32     -- Last version where this message is still valid (0xFFFFFFFF = no limit)
  argsLength: uint16     -- Length of serialized arguments
  args: uint8*           -- Serialized method arguments
```

**KnownVersion** -- Tracks what a witness (player) has seen of a particular entity:

```
KnownVersion:
  generationId: uint32   -- Entity ID reuse counter
  versions[MaxPropertySets]: uint32  -- Last seen version per property set
```

### The INCREMENTAL_UPDATABLE Flag

From `base_cell_messages.hpp`, the `INCREMENTAL_UPDATABLE` flag is set on cache messages within the `UPDATE_CACHE_STAMP` protocol message:

```
CELL_BASE_UPDATE_CACHE_STAMP (0x11):
  uint32    Entity ID
  uint32    PropSet ID     (0 to MaxPropertySets-1)
  uint8     Invalidate     (clear existing cache?)
  Message[] Messages:
      uint8  Message ID
      uint8  Flags          (INCREMENTAL_UPDATABLE = 0x01)
      uint16 Length
      byte[] Args
```

When the CellApp sends an `UPDATE_CACHE_STAMP`, the BaseApp handler (`onUpdateCacheStamp()`) processes it as follows:

1. If `Invalidate` is set, `invalidateCacheStamps(propertySetId)` clears all existing stamps and messages for that property set, advancing `firstStampId` past the old window
2. `beginCacheStamp(propertySetId)` opens a new stamp (appends to the stamps vector)
3. For each message: `addCacheMessage()` creates a `CacheMessage` with `minVersion = lastVersion()` and `maxVersion = 0xFFFFFFFF` (unbounded)
4. `endCacheStamp()` finalizes the stamp and propagates the new messages to all current witnesses via `visitWitnesses()`

### Delta Computation (sendDeltas)

When a witness needs updates (e.g., an entity becomes visible), `sendDeltas()` compares the witness's `KnownVersion` against the entity's current `PropertySet` state:

```
sendDeltas(witness, propertySetId, refVersion):
  1. If refVersion > lastVersion: WARN and reset to firstStampId - 1
  2. If refVersion < firstStampId - 1: stamps were invalidated, reset
  3. Starting from refIndex = refVersion + 1 - firstStampId:
     For each stamp from refIndex to end:
       For each message in that stamp:
         If msg.minVersion == current_stamp_version
            AND msg.maxVersion >= lastVersion:
           Send the message to the witness
```

The `minVersion`/`maxVersion` pair implements **message supersession**: when a property is updated multiple times, older messages can be superseded by setting their `maxVersion` to a value less than `lastVersion`. Currently, Cimmeria sets `maxVersion = 0xFFFFFFFF` for all messages (no supersession), meaning all historical messages within the stamp window are sent to late-joining witnesses.

### Generation ID (Entity ID Reuse)

When an entity ID is reused (entity destroyed and recreated), `CachedEntityManager::addEntity()` increments the `generationId`. During `sendDeltas()`, if the witness's `KnownVersion.generationId` does not match the entity's current `generationId_`, all property sets are sent from version 0 (full update), ensuring stale cache data from a previous entity occupying the same ID is not used.

### Entity Visibility Integration

The cache system integrates with the AoI world grid:

| Event | Cache Behavior |
|-------|---------------|
| Entity becomes visible (`onEntityVisible`) | `sendDeltas()` with stored `KnownVersion` (or default 0 for new entities) |
| Entity becomes invisible (`onEntityInvisible`) | If caching enabled: save current versions to `knownEntities_` map. If destroyed: erase from map |
| Entity re-enters AoI | `sendDeltas()` uses saved `KnownVersion`, sending only changes since last seen |
| Entity destroyed | `leaveAoI(entityId, clearCache=true)` tells client to discard cached data |

### CacheOnClient Configuration

The `cache_on_client` config parameter (read from `BaseService.config`) controls whether the BaseApp uses the caching optimization:

- **Enabled** (default): Witnesses track `KnownVersion` per entity. On re-enter, only delta messages are sent. Client cache is preserved on `leaveAoI`.
- **Disabled**: Every visibility event sends a full `createEntity` + all cache messages. No `knownEntities_` map is allocated. `leaveAoI` always clears client cache.

### Cell Entity Flags

From `base_cell_messages.hpp`, the CellApp sends flags with each entity describing its cache properties:

| Flag | Value | Description |
|------|-------|-------------|
| `ENTITY_HAS_DYNAMIC` | 0x01 | Entity has dynamic (frequently changing) properties |
| `ENTITY_NOT_CACHED` | 0x02 | Entity data is not locally cached on BaseApp |

When an entity becomes visible, the BaseApp checks these flags to determine what to request from the CellApp:

```cpp
if (ref->flags() & Mercury::ENTITY_HAS_DYNAMIC)
    requested |= Mercury::UPDATE_DYNAMIC;
if (ref->flags() & Mercury::ENTITY_NOT_CACHED)
    requested |= Mercury::UPDATE_STATIC | Mercury::UPDATE_UNCACHED;
```

This ensures the BaseApp only requests data it does not already have cached.

## TODO

- [x] ~~Reverse engineer the exact property streaming order~~ → Documented in `findings/entity-property-sync.md`: sequential in parse order (Parent->Implements->Own)
- [x] ~~Determine if SGW uses LoD levels~~ → NO. All `<LoDLevels>` tags empty. See `engine/entity-lod-system.md`
- [x] ~~Map which properties use event stamps vs which use simple overwrite~~ → Only `OTHER_CLIENT` properties (those with `DATA_OTHER_CLIENT` flag) receive event stamps via `eventStampIndex`. All other properties use simple overwrite. See "Event Stamps vs Simple Overwrite" section above.
- [x] ~~Document the incremental update flag behavior in the cache stamp system~~ → Cimmeria's `CachedEntity` uses `PropertySet` versioning with `CacheMessage` min/max version pairs. `sendDeltas()` compares witness `KnownVersion` against entity state to send only unseen messages. `generationId` handles entity ID reuse. See "Incremental Update Flag Behavior" section above.
- [x] ~~Verify property ID encoding for >60 properties~~ → IDs 0-59 = 1-byte, 60+ = 2-byte extended. See `findings/entity-property-sync.md`
- [x] ~~Determine exact format of createBasePlayer/createCellPlayer~~ → Fully documented in `findings/entity-property-sync.md`
