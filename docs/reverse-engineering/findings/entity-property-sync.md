# Entity Property Sync Protocol — RE Findings

> **Date**: 2026-03-01
> **Confidence**: HIGH (decompiled code + BW 2.0.1 source + .def files)
> **Sources**: Ghidra decompilation of `sgw.exe`, BigWorld 2.0.1 engine source, `.def` files, `alias.xml`

---

## Overview

This document captures the reverse-engineered details of how BigWorld/Cimmeria assigns property and method IDs, streams entity data during creation, and propagates property changes. Getting this wrong causes client desync or crashes — property IDs must match exactly between server and client.

---

## 1. EntityDescription Parse Order

### Finding

Entity descriptions are parsed in a specific order that determines property and method ID assignment. The parse order was confirmed by decompiling `EntityDescription__unknown_01593600` at `0x01593600`.

**Parse order**:

```
1. Parent entity (recursive — parent's parent first, etc.)
2. Implements interfaces (in XML declaration order)
3. Own Properties
4. Own ClientMethods  (via FUN_01593420)
5. Own CellMethods    (via FUN_015934c0)
6. Own BaseMethods    (via FUN_01593560)
```

### Evidence

From `EntityDescription__unknown_01593600` (0x01593600), the call sequence is:

```c
// 1. Parse parent entity recursively
if (hasParent) {
    EntityDescription_parse(parentName);  // recurses
}

// 2. Parse each Implements interface in order
for (each <Interface> in <Implements>) {
    parseInterface(interfaceName);
}

// 3. Parse properties
EntityDescription_parseProperties(propertiesSection);

// 4. Parse client methods
FUN_01593420(clientMethodsSection);  // ClientMethods

// 5. Parse cell methods
FUN_015934c0(cellMethodsSection);    // CellMethods

// 6. Parse base methods
FUN_01593560(baseMethodsSection);    // BaseMethods
```

From `EntityDescription_parse` (0x01593cd0):
- Opens `entities/defs/<name>.def` (or `entities/defs/interfaces/<name>.def`)
- Reads `<Parent>` element → recursively parses parent first
- Reads `<ClientName>` and `<ServerOnly>` flags
- Delegates to the function above for the actual parsing

### SGWPlayer Parse Chain

For `SGWPlayer.def`, the full resolution order is:

```
SGWPlayer
├── Parent: SGWBeing (interface file)
│   ├── (no parent)
│   ├── Implements: (none listed in SGWBeing.def)
│   ├── SGWBeing Properties
│   ├── SGWBeing ClientMethods (8 methods)
│   ├── SGWBeing CellMethods (16 methods)
│   └── SGWBeing BaseMethods (3 methods)
├── Implements (in order):
│   ├── Communicator
│   ├── OrganizationMember
│   ├── MinigamePlayer
│   ├── GateTravel
│   ├── SGWInventoryManager (7 ClientMethods, 13 CellMethods)
│   ├── SGWMailManager
│   ├── Missionary
│   ├── SGWPoller
│   ├── ContactListManager
│   ├── SGWBlackMarketManager
│   └── ClientCache
├── SGWPlayer Properties
├── SGWPlayer ClientMethods (61 methods)
├── SGWPlayer CellMethods (90 methods, 43 Exposed)
└── SGWPlayer BaseMethods (25 methods, 8 Exposed)
```

**Critical**: SGWCombatant is NOT in SGWPlayer's Implements list — it must be included via SGWBeing or another interface in the chain. This affects method ID assignment for combat methods like `onStatUpdate`, `setCrouched`, etc.

---

## 2. Property ID Assignment

### Finding

Properties are assigned sequential IDs as they are parsed. Non-editor-only properties get an index in the main property table. Properties with client-visible flags additionally get an index in the client property list.

### Evidence

From `EntityDescription_parseProperties` (0x015924a0):

```c
// For each property in the <Properties> section:
DataDescription_parse_2(propertyDesc, xmlSection);

if ((flags & 0x06) == 0) {
    // Not client-visible
    if ((flags & 0x01) != 0) {
        // CELL_PUBLIC (DATA_GHOSTED) — validate type restrictions
        // PYTHON, USER_TYPE, CLASS types cannot be propagated
        // ARRAY/TUPLE/FIXED_DICT with complex subtypes warned
    }
}

if ((flags >> 6 & 1) == 0) {  // Not EDITOR_ONLY (bit 6 = 0x40)
    propertyIndex = propertyTable.size();  // sequential assignment
    propertyMap[propertyName] = propertyIndex;

    if ((flags & 0x06) != 0) {  // Client-visible (OWN_CLIENT or OTHER_CLIENT)
        clientPropertyList.push_back(propertyIndex);
    }

    // Add to main property array
    propertyArray.append(propertyDesc);
}
```

### Property Flag Parsing

From `DataDescription_parse_2` (0x015974a0):

```c
// Parse property type
DataType_buildDataType(typeSection);  // stored at this+0x1c

// Parse flags
DataDescription_parse_1(flagsSection, &this->flags);  // stored at this+0x20

// Parse persistence
if (readBool("Persistent")) {
    this->flags |= 0x20;  // DATA_PERSISTENT
}

// Parse identifier
if (readBool("Identifier")) {
    this->flags |= 0x80;  // DATA_ID
}

// Parse default value
DataType::createFromSection("Default");  // stored at this+0x24

// Parse database length
readInt("DatabaseLength");  // stored at this+0x3c
```

### Property Flags Summary

| Bit | Hex | Flag | .def Name |
|-----|-----|------|-----------|
| 0 | 0x01 | DATA_GHOSTED | CELL_PUBLIC |
| 1 | 0x02 | DATA_OTHER_CLIENT | OTHER_CLIENTS |
| 2 | 0x04 | DATA_OWN_CLIENT | OWN_CLIENT |
| 3 | 0x08 | DATA_BASE | BASE |
| 4 | 0x10 | DATA_CLIENT_ONLY | CLIENT_ONLY |
| 5 | 0x20 | DATA_PERSISTENT | `<Persistent>true</Persistent>` |
| 6 | 0x40 | DATA_EDITOR_ONLY | EDITOR_ONLY |
| 7 | 0x80 | DATA_ID | `<Identifier>true</Identifier>` |

Client-visible mask: `flags & 0x06` (bits 1+2 = DATA_OTHER_CLIENT | DATA_OWN_CLIENT)

---

## 3. Method ID Assignment

### Finding

Method IDs are assigned sequentially within each method category (ClientMethods, CellMethods, BaseMethods), following the same parse order as properties: parent first, then interfaces in order, then own methods.

### Evidence

From `MethodDescription_parse` (0x01594f60):

```c
// For each method in <ClientMethods>, <CellMethods>, or <BaseMethods>:
// Parse <Arg> children
for (each <Arg> child) {
    DataType *argType = DataType_buildDataType(argTypeSection);
    argTypes.push_back(argType);  // stored in vector at this+0x20

    // Parse <ArgName> if present
    if (hasArgName) {
        argNames.push_back(argNameStr);
    }
}

// Check for <Exposed/> flag
if (hasExposedTag) {
    this->exposed = true;
}
```

Methods are added to their category's method list in parse order, getting sequential IDs starting from 0.

### Wire Format for Method Calls

From the universal RPC dispatcher at `0x00c6fc40`:

```
Client → Server (cell method):   [1 byte: methodID | 0x80] [serialized args]
Client → Server (base method):   [1 byte: methodID | 0xC0] [serialized args]
Server → Client (client method): [method ID] [serialized args]
```

**Important**: The `methodID` in the header byte is the **sequential index** within that method category, NOT a global ID. Cell method 0 is the first cell method parsed (from the topmost parent/interface).

---

## 4. Entity Creation Messages

### createBasePlayer

**Source**: `ServerConnection_createBasePlayer` at `0x00dddca0`

**Wire format**:

```
createBasePlayer message:
+----------+----------+-------------------+
| EntityID | TypeID   | Property Stream   |
| 4 bytes  | 2 bytes  | variable          |
+----------+----------+-------------------+
```

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | EntityID | Unique entity ID for this session |
| 4 | 2 | uint16 | TypeID | Entity type index in `entities.xml` |
| 6 | var | bytes | PropertyStream | Properties matching `CLIENT_DATA \| BASE_DATA` |

**Evidence** (decompiled):
```c
void ServerConnection_createBasePlayer(void *this, int *stream) {
    // Read 4-byte entity ID
    uint32 *entityId = stream->read(4);
    this->playerEntityId = *entityId;  // stored at this+0x16c

    // Read 2-byte entity type ID
    uint16 *typeId = stream->read(2);

    // Invoke entity manager callback with remaining stream
    if (this->entityManagerCallback != NULL) {
        this->entityManagerCallback(this->playerEntityId, *typeId, stream);
    }

    // Check for buffered createCellPlayer
    if (bufferedCellPlayerMsg.remainingLength() > 0) {
        ServerConnection_createCellPlayer(this, bufferedCellPlayerMsg);
    }
}
```

### createCellPlayer

**Source**: `ServerConnection_createCellPlayer` at `0x00dda2e0`

**Wire format**:

```
createCellPlayer message:
+------+---------+----------+-------------------+
| Skip | SpaceID | Position | Property Stream   |
| 4B   | 4B      | 12B      | variable          |
+------+---------+----------+-------------------+
```

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | (skip) | Consumed but not used directly |
| 4 | 4 | uint32 | SpaceID | BigWorld space ID |
| 8 | 12 | Vec3 | Position | x, y, z as float32 |
| 20 | var | bytes | PropertyStream | Properties matching `CLIENT_DATA \| CELL_DATA` |

**Evidence** (decompiled):
```c
void ServerConnection_createCellPlayer(void *this, int *stream) {
    if (this->playerEntityId == 0) {
        // No base player yet — buffer this message
        bufferMessage(stream);
        return;
    }

    stream->read(4);              // skip 4 bytes
    int spaceId = *stream->read(4);     // read space ID
    Vec3 *pos = stream->read(12);       // read position (3 floats)

    // Additional stream processing
    FUN_015846a0(stream);  // likely reads remaining property data

    // Invoke entity manager callback
    if (this->entityManagerCallback != NULL) {
        this->entityManagerCallback(
            this->playerEntityId, spaceId, 0,
            pos, ...  // position, direction, stream
        );
    }
}
```

**Ordering guarantee**: `createBasePlayer` always comes first. If `createCellPlayer` arrives first, it is buffered and replayed after `createBasePlayer` completes.

### Property Stream Format

During entity creation, properties are serialized in their assigned order (by property ID). The stream format depends on the data domain filter:

**For createBasePlayer** (`CLIENT_DATA | BASE_DATA`):
- Properties with `DATA_OWN_CLIENT` (0x04) OR `DATA_BASE` (0x08) flags
- Each property serialized using its `DataType::addToStream`
- No property ID prefix — properties are in implicit sequential order

**For createCellPlayer** (`CLIENT_DATA | CELL_DATA`):
- Properties with `DATA_OWN_CLIENT` (0x04) OR `DATA_GHOSTED` (0x01) flags
- Same sequential serialization

---

## 5. Entity Enter/Leave AoI

### Finding

When an entity enters a player's Area of Interest, the server sends entity data to the client. When it leaves, the client cleans up.

**Enter AoI** (entity_manager.cpp ref at `0x00dd27f0`):
- Increments reference count on entity
- If entity has a Python object (`getEnterCount() > 0`), calls `EntityManager_enterWorld`
- Asserts: `vehicleID == 0` for client-only entities
- Sends entity properties matching `ONLY_OTHER_CLIENT_DATA`

**Leave AoI** (entity_manager.cpp ref at `0x00dd2900` area):
- Decrements reference count
- If count reaches 0, removes entity from world
- Checks `SGW::CEF_Remote` flag before cleanup

### Entity Position Update

From `BW_client_entity_manager_6` (entity movement handler):
- Source path: `entity_manager.cpp`
- Receives: entityID, position (Vec3), direction (yaw, pitch, roll), spaceID, vehicleID
- Checks `LOGENTITYMOVE` debug flag for logging
- Applies NaN filtering on position components (replaces NaN with current position)
- Calls the entity's position update method

---

## 6. Property Change Messages

### Finding

Property changes at runtime use a compact encoding. From `FNetworkPropertyChange__vfunc_0` at `0x015652d0`:

```c
void FNetworkPropertyChange::apply(int *stream) {
    // Write 4 bytes from this+0x2c (property change header)
    stream->write(this + 0x2c, 4);

    // Chain 3 property value writes
    writePropertyValue(stream, this + 0x08);  // field 1
    writePropertyValue(stream, this + 0x14);  // field 2
    writePropertyValue(stream, this + 0x20);  // field 3
}
```

### Property Change Wire Format

From BigWorld 2.0.1 source (`property_change.hpp`):

```
For property IDs 0-59:
  [1 byte: propertyID]  [value bytes]

For property IDs 60+:
  [1 byte: 0x3C + (id-60)/256]  [1 byte: (id-60)%256]  [value bytes]
```

| Property ID Range | Header Size | Encoding |
|-------------------|-------------|----------|
| 0-59 | 1 byte | Direct: `propertyID` |
| 60-315 | 2 bytes | Extended: `0x3C`, `propertyID - 60` |
| 316+ | 2 bytes | Extended: `0x3D`, `propertyID - 316` |

**Change type** (1 byte following header):

| Value | Type | Description |
|-------|------|-------------|
| 0 | `PROPERTY_CHANGE_TYPE_SINGLE` | Entire value replaced |
| 1 | `PROPERTY_CHANGE_TYPE_SLICE` | Array element changed |

---

## 7. Filtered Property Lists

### Finding (from EntityDescription_parseProperties)

The property parser builds multiple filtered lists for different use cases:

1. **All properties** (`this+0x5c`): Every non-editor-only property, in parse order
2. **Client properties** (`this+0x6c`): Properties with `flags & 0x06 != 0` (OWN_CLIENT or OTHER_CLIENT)
3. **Other-client properties**: Properties with `DATA_OTHER_CLIENT` (0x02) flag — sent to nearby players
4. **Property name→index map** (`this+0x7c`): Red-black tree mapping name strings to indices

### Type Restrictions on Propagated Properties

From the decompiled validation in `EntityDescription_parseProperties`:

| Type | Can be CELL_PUBLIC? | Can be OTHER_CLIENTS? | Notes |
|------|--------------------|-----------------------|-------|
| INT8/16/32, UINT8/16/32 | Yes | Yes | — |
| FLOAT/FLOAT32/64 | Yes | Yes | — |
| STRING/WSTRING | Yes | Yes | — |
| VECTOR3 | Yes | Yes | — |
| FIXED_DICT | Yes | Yes | Subtypes must be simple |
| ARRAY/TUPLE | Yes | Yes | Element type must be simple |
| PYTHON | Warning | Warning | "PYTHON properties should not be propagated" |
| USER_TYPE | Warning | Warning | "USER_TYPE properties should not be propagated" |
| CLASS | Warning | Warning | "CLASS properties should not be propagated" |

"Simple" means: not PYTHON, not USER_TYPE, not CLASS. FIXED_DICT members and ARRAY elements are recursively checked.

### Excluded Properties (client-side filtering)

From the decompiled code, the client maintains a separate exclusion list for properties that should NOT be sent to certain clients. The following property names are explicitly excluded from the client's processing in `EntityDescription_parseProperties`:

- `publicReservationData`
- `publicMissionData`
- `completedMissions`
- `aggressionOverrides`
- `effectMonikers`

These are added to a separate filter set (`auStack_c4`) and excluded from the client property table, even though they may have propagation flags.

---

## 8. Data Domains for Property Streaming

From BigWorld `entity_description.hpp` and confirmed in the decompiled entity creation handlers:

| Domain | Value | Description | Used In |
|--------|-------|-------------|---------|
| `BASE_DATA` | 0x01 | Properties for the base entity | createBasePlayer |
| `CLIENT_DATA` | 0x02 | Properties for the client | Both create messages |
| `CELL_DATA` | 0x04 | Properties for the cell entity | createCellPlayer |
| `EXACT_MATCH` | 0x08 | Flags must match exactly | Selective streaming |
| `ONLY_OTHER_CLIENT_DATA` | 0x10 | Only OTHER_CLIENT props | AoI enter for other players |
| `ONLY_PERSISTENT_DATA` | 0x20 | Only persistent props | Database save/load |

---

## Cross-Validation Summary

| Finding | Ghidra | BW Source | .def Files | Confidence |
|---------|--------|-----------|------------|------------|
| Parse order (Parent→Impl→Own) | Y (0x01593600) | Y | Y | HIGH |
| Property sequential ID assignment | Y (0x015924a0) | Y | Y | HIGH |
| Method sequential ID assignment | Y (0x01594f60) | Y | Y | HIGH |
| createBasePlayer format | Y (0x00dddca0) | Y | — | HIGH |
| createCellPlayer format | Y (0x00dda2e0) | Y | — | HIGH |
| Property change encoding | Y (0x015652d0) | Y | — | HIGH |
| Client property exclusions | Y (0x015924a0) | N/A | — | MEDIUM |
| Property flag values | Y (0x015974a0) | Y | Y | HIGH |

---

## Implementation Impact

### Critical: Property ID Assignment Must Match

The server MUST assign property IDs in the exact same order as the client parser:

1. Parse `<Parent>` entity recursively (parent's parent first)
2. Parse each `<Interface>` in `<Implements>` section, in declaration order
3. Parse `<Properties>` section
4. Skip `EDITOR_ONLY` properties (flag 0x40)
5. Assign sequential IDs starting from the parent's last ID + 1

If the server assigns different IDs, property updates will modify the wrong properties on the client, causing silent data corruption or crashes.

### Critical: Method ID Assignment Must Match

Same principle applies to method IDs. Each method category (Client, Cell, Base) has its own sequential numbering:

- Cell method 0 is the first cell method from the topmost ancestor
- Cell method N is offset by all parent/interface cell methods

For example, if SGWBeing defines 16 CellMethods and SGWCombatant defines 14 CellMethods, then SGWPlayer's first own CellMethod would be at index `16 + 14 + (other interface methods)`.

### createBasePlayer Before createCellPlayer

The server must send `createBasePlayer` before `createCellPlayer`. The client buffers any `createCellPlayer` received before `createBasePlayer` and replays it after. However, relying on this buffering is not recommended — always send in the correct order.

### Property Exclusion List

Five property names are excluded from client-side processing even if they have propagation flags:
- `publicReservationData`, `publicMissionData`, `completedMissions`, `aggressionOverrides`, `effectMonikers`

The server should NOT attempt to send these as property updates to the client.

### NaN Position Handling

The client filters NaN values in entity position updates, replacing each NaN component with the entity's current position. The server should never send NaN positions, but the client is resilient to them.

---

## Related Documents

- [Combat Wire Formats](combat-wire-formats.md) — Method call serialization
- [Inventory Wire Formats](inventory-wire-formats.md) — Inventory message formats
- [Entity Property Sync Protocol](../../protocol/entity-property-sync.md) — Higher-level protocol doc
- [Entity Type Catalog](../../engine/entity-type-catalog.md) — All entity definitions
