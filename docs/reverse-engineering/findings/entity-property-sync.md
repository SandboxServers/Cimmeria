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

## 9. Vector Helper Infrastructure (W-entity-desc-A findings, 2026-05-13)

The EntityDescription parse chain relies on a cluster of vector and map helpers in `[0x0158e060, 0x0158ea60]`. These were recovered in Session 5 W-entity-desc-A and are listed below for cross-reference. All addresses confirmed via Ghidra decompilation.

### DataDescriptionParseVec Helpers (parse-time 0x110-byte form)

| Address | Name | Notes |
|---------|------|-------|
| `0x0158e060` | `DataDescriptionParseVec_GetSize` | `(end-begin)/0x110`; fields at `this+4`/`+8` |
| `0x0158e0a0` | `DataDescriptionParseVec_AllocN` | `scalable_malloc(n*0x110)` with overflow guard |
| `0x0158e180` | `DataDescriptionParseVec_GetSizeAlt` | Same formula; fields at `this+0x10`/`+0x14` |
| `0x0158e1a0` | `DataDescriptionParseVec_GetAt` | Bounds-checked `begin + idx*0x110` |
| `0x0158e310` | `DataDescriptionParseVec_ForEachFindMax` | Functor-per-element, tracks max return value |

### MethodDescriptionVec Helpers (0x50-byte form)

| Address | Name | Notes |
|---------|------|-------|
| `0x0158e080` | `MethodDescriptionVec_GetSize` | `(end-begin)/0x50`; fields at `this+4`/`+8` |
| `0x0158e110` | `MethodDescriptionVec_AllocN` | `scalable_malloc(n*0x50)` with overflow guard |
| `0x0158e460` | `MethodDescriptionVec_ReserveN` | Init sub-vector with N capacity; max 0x3333333 |
| `0x0158e280` | `MethodDescriptionVec_CopyRangeToOffset` | Copy range to `src+offset` using CopyAssign |
| `0x0158e590` | `MethodDescriptionVec_CopyRangeToOffsetThunk` | One-liner thunk for above |

### DataDescriptionVec Helpers (runtime 0x40-byte form)

| Address | Name | Notes |
|---------|------|-------|
| `0x0158e4b0` | `DataDescriptionVec_ReserveN` | Init sub-vector with N capacity (0x40 each); max 0x3ffffff |

### SEH Copy-Construct Wrappers

| Address | Name | Notes |
|---------|------|-------|
| `0x0158e1e0` | `MethodDescription_CopyCtorSEH` | Guards `MethodDescription_CopyCtor`; skips if dst==null |
| `0x0158e230` | `DataDescription_PartialInitSEH` | Guards `DataDescription_PartialInit` (0x40-byte form); skips if dst==null |
| `0x0158e500` | `DataDescriptionVec_UninitCopyRange` | SEH-wrapped range copy, 0x40-byte stride |
| `0x0158e5c0` | `MethodDescriptionVec_UninitCopyRange` | SEH-wrapped range copy, 0x50-byte stride |
| `0x0158ea00` | `MethodDescriptionVec_UninitCopyRangeThunk` | 5-arg thunk (drops params 1, 3) |
| `0x0158ea30` | `DataDescriptionVec_UninitCopyRangeThunk` | 5-arg thunk (drops params 1, 3) |
| `0x0158ea60` | `DataDescriptionVec_UninitCopyRangeThunk2` | 3-arg direct thunk |

### EntityDescription Method ID Map (std::map<uint32, uint16>)

| Address | Name | Notes |
|---------|------|-------|
| `0x0158e650` | `EntityDescriptionMap_LowerBound` | MSVC xtree lower_bound; sentinel at node+0x15 |
| `0x0158e840` | `EntityDescriptionMap_InsertOrFind` | MSVC xtree insert-or-find |
| `0x0158e710` | `EntityDescription_FindMethodIdByName` | **KEY**: returns `uint16` at node+0x10; 0xffff if not found |
| `0x0158e780` | `EntityDescription_FindAndWritePropertyByName` | Name-scans parse-time DataDescVec, calls `EntityDescription_WriteClientData` on match |

### Critical Finding: Method ID Lookup is Directly Wired to RPC Dispatch

`EntityDescription_FindMethodIdByName` (`0x0158e710`) is called from:
- `ProcessEntityMethodEmission` (`0x00c6f8f0`)
- `RouteOutgoingEntityRpc` (`0x00c6fc40`) — the universal RPC dispatcher

This confirms the method ID encoding in the wire format maps directly through the EntityDescription method ID map. The `uint16` stored at RB-tree node+0x10 is the encoded wire method ID. Return value 0xffff is the "not found" sentinel (same as BigW `0xffff` exposed-method sentinel from `MethodDescriptionVec_AtBounded`).

### Open Question: DataDescription Dual Name Fields

`EntityDescription_FindAndWritePropertyByName` (`0x0158e780`) compares two StdStringMSVC fields within the same 0x110-byte parse-time DataDescription:
- Field 1: element+0x24 (length at +0x34, capacity at +0x38)
- Field 2: element+0x40 (length at +0x50, capacity at +0x54)

Both are compared against the search name. This implies the parse-time DataDescription stores two distinct name strings. Based on `DataDescription_Constructor` (`0x01591fb0`) which initializes three StdStringMSVC at +0x04, +0x24, and +0x40, the likely layout is:

| Offset | Field |
|--------|-------|
| +0x04 | Internal/server name (StdStringMSVC) |
| +0x24 | Client-visible name (StdStringMSVC) |
| +0x40 | Alias or qualified name (StdStringMSVC) |

This is **hypothesis pending verification** — a cross-reference to `DataDescription_parse_2` (`0x015974a0`) and the BigWorld 2.0.1 `DataDescription` source would confirm which field is set from which XML element.

---

---

## 10. DataType Two-Registry System (W-entity-desc-B findings, 2026-05-13)

The BigWorld entity description parser uses two separate `std::map<string, DataType*>` registries with distinct roles. Both reside in the SGW.exe `.data` segment; neither is documented in the BW 2.0.1 public source.

### Registry Addresses

| Symbol | Address | Populated By | Queried By |
|--------|---------|-------------|------------|
| `g_mapDataTypeRegistry` | `DAT_01f126b8` | `DataType_RegisterBuiltins` (`0x01596c40`) | `DataType_BuildFromSection` (`0x01597150`) |
| `g_pMetaDataTypeRegistry` | `DAT_01f126b4` | `DataType_Register` (`0x01597ce0`) | `DataType_LookupByName` (`0x01595f00`) |

### Primary Registry: g_mapDataTypeRegistry

`DataType_RegisterBuiltins` (`0x01596c40`) reads `entities/defs/alias.xml`. For each XML child element it:
1. Calls `DataType_BuildFromSection` recursively to instantiate the DataType.
2. Inserts the element's tag name → `DataType*` into `g_mapDataTypeRegistry` via `StdMap_DataType_EmplaceOrFind` (MSVC xtree insert).

**Role**: This is the BUILD path. When parsing a `.def` file and encountering a `<Type>` tag, `DataType_BuildFromSection` looks up the tag string here to find the matching factory.

### Secondary Registry: g_pMetaDataTypeRegistry

`DataType_Register` (`0x01597ce0`) is called by all 17 `SimpleMetaDataType<T>::Constructor` functions during static initialization. It:
1. Lazy-allocates `g_pMetaDataTypeRegistry` via `scalable_malloc(0xc)` + `FUN_00460320` (map constructor).
2. Calls `vtable[1](this)` on the MetaDataType to get its name string.
3. Duplicate-checks via `FUN_0158ea90` (std::map::find).
4. Logs `"MetaDataType::addType: %s has already been registered."` on duplicate.
5. Inserts name → `SimpleMetaDataType<T>*` via `FUN_00476590` (std::map::operator[]).

**Role**: This is the LOOKUP path. `DataType_LookupByName` (`0x01595f00`) reads `g_pMetaDataTypeRegistry` to find a DataType instance by its C++ type name.

### Key Distinction: Different Key Spaces

The two registries can have **different keys for the same underlying C++ type**. `alias.xml` creates aliases like `"INT8" → IntegerDataType<signed_char>`, while the MetaDataType registry stores `"INT8"` (the string returned by the SimpleMetaDataType's `getName()` vtable slot). In practice for the primitive types both maps use the same name strings, but the architecture permits divergence (e.g., if alias.xml uses `"INTEGER"` while the MetaDataType was registered as `"INT32"`).

### W4-B2 Ambiguity Resolution

W4-B2 documented both `g_pMetaDataTypeRegistry` and `g_mapDataTypeRegistryLookup` as separate globals, both at `DAT_01f126b4`. This was an error: there is only ONE object at that address. `DataType_Register` populates it; `DataType_LookupByName` reads it. The correct canonical name is `g_pMetaDataTypeRegistry`.

### DataType Subclass Hierarchy

17 concrete DataType subclasses are registered, each with a 4-function group (DtorBody, Constructor, GetTypeName_WriteStream, New):

| Type | Constructor | MD5 Type Encoding |
|------|-------------|-------------------|
| `IntegerDataType<unsigned char>` (UInt8) | `0x01599150` | 1 (1-byte uint) |
| `IntegerDataType<char>` (Int8) | `0x015995f0` | 1 (1-byte signed) |
| `IntegerDataType<unsigned short>` (UInt16) | `0x01599340` | 2 (2-byte uint) |
| `IntegerDataType<short>` (Int16) | `0x015997d0` | 2 (2-byte signed) |
| `IntegerDataType<long>` (Int32) | `0x015999b0` | 4 (4-byte int) |
| `LongIntegerDataType<unsigned long>` (UInt32) | `0x01599b90` | 4 (4-byte uint) |
| `LongIntegerDataType<__int64>` (Int64) | `0x01599d90` | 8 (8-byte signed) |
| `LongIntegerDataType<unsigned __int64>` (UInt64) | `0x01599f70` | 8 (8-byte uint) |
| `FloatDataType` | `0x0159a220` | `"Float"` (6 bytes literal) |
| `StringDataType` | `0x0159a3f0` | (inherits string path) |
| `WideStringDataType` | `0x0159a5e0` | (inherits wide path) |
| `PythonDataType` | `0x0159a790` | `"Python"` (7 bytes literal) |
| `VectorDataType<Vector2>` | `0x0159aa00` | `"Vector"` + 4-byte '2' marker |
| `VectorDataType<Vector3>` | `0x0159acf0` | `"Vector"` + 4-byte '3' marker |
| `VectorDataType<Vector4>` | `0x0159af80` | `"Vector"` + 4-byte '4' marker |
| `BlobDataType` | `0x0159b300` | `"Blob"` (5-byte literal at DAT_01b1ba80) |
| `MailBoxDataType` | `0x0159b510` | `"MailBox"` (8 bytes literal) |

SimpleMetaDataType<T> constructors: `0x0159db10`–`0x0159e510` (17 functions, sequential, each calls `DataType_Register`).

---

## 11. MD5 Type Signature Hashing (W-entity-desc-B findings, 2026-05-13)

The BigWorld entity description system uses MD5 to compute a type signature for each DataType. This signature is used for protocol versioning — the server and client must agree on the hash of each entity's property/method type layout.

### MD5 Infrastructure (confirmed via Ghidra decompilation)

| Address | Function | Notes |
|---------|----------|-------|
| `0x015a3d70` | `MD5_Init` | Sets bit_count=0, digest=[0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476] |
| `0x015a3da0` | `MD5_Update` | Thin wrapper → `MD5_Update_Block` |
| `0x015a3c00` | `MD5_Update_Block` | Core block processor; partial-block handling at byte-aligned offsets |
| `0x015a3cd0` | `MD5_Finalize` | Appends padding + 8-byte length, writes 16-byte digest |
| `0x015a3dc0` | `MD5_Finalize_Wrapper` | Thin wrapper → `MD5_Finalize` |
| `0x015a3de0` | `MD5_DigestToHexString` | 16-byte digest → 32-char uppercase hex; uses `"0123456789ABCDEF"` table at `DAT_01b1bd40` |

### Type Encoding into MD5 Stream

Each `DataType::GetTypeName_WriteStream` method feeds binary type data into the MD5 stream:

- **Integer types**: Write a 5-byte prefix + 1 byte for the type size (1=byte, 2=short, 4=int, 8=int64)
- **Float**: Writes literal string `"Float"` (6 bytes)
- **Python**: Writes literal string `"Python"` (7 bytes)
- **Vector2/3/4**: Writes `"Vector"` (7 bytes) + a 4-byte marker at DAT_01b1b990/b9e8/ba30
- **Blob**: Writes 5-byte literal at DAT_01b1ba80
- **MailBox**: Writes literal string `"MailBox"` (8 bytes)

The resulting MD5 hash is a compact protocol version fingerprint for the entity's type schema. Mismatch between client and server would indicate a schema divergence.

---

## 12. CME Property System (W-entity-desc-B findings, 2026-05-13)

The `CME::Detail::PropertyNode::Property<T>` hierarchy provides a secondary typed property container separate from the BigWorld stream-based protocol. This is used internally for the SGW CME (Custom Map Entity?) layer.

### TypeList Coverage

The `CME::BasicPropertyList<TypeList<14 types>>` covers: `uint8`, `int8`, `uint16`, `int16`, `uint32`, `int32`, `uint64`, `int64`, `float`, `wstring`, `Vector2`, `Vector3`, `Vector4`, `NullType` (terminator).

### Key Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x0159ba30` | `CMEProperty_UInt16_New` | `scalable_malloc(0x0c)`, calls UInt16 ctor with value |
| `0x0159bad0` | `CMEProperty_UInt64_New` | `scalable_malloc(0x10)` |
| `0x0159bb70` | `CMEProperty_Vector2_New` | `scalable_malloc(0x10)`, stores 8 bytes (2 floats) |
| `0x0159bc10` | `CMEProperty_Vector4_New` | `scalable_malloc(0x18)`, stores 16 bytes (4 floats) |
| `0x0159bcd0` | `CMEPropertyList_StreamToTree` | Iterates CME property list; writes each entry to property tree via vtable[7] |
| `0x0159bd70` | `CMEPropertyList_PrintToStream` | Prints `[val1, val2, ...]` format to wchar_t stream |
| `0x015a27f0` | `CMEBasicPropertyList_StreamToTree` | Full TypeList dispatch; calls `CMEPropertyList_StreamToTree` |
| `0x015a2880` | `CMEBasicPropertyList_PrintToStream` | Calls `CMEPropertyList_PrintToStream` |

### CME Property Object Sizes

| Type | Size | Layout |
|------|------|--------|
| `Property<uint8>` | 0x0c | `[vftable][+4=padding][+8=value_byte]` |
| `Property<uint16>` | 0x0c | `[vftable][+4=pad][+8=value_u16]` |
| `Property<uint64>` | 0x10 | `[vftable][+4=pad][+8..0xc=value_u64]` |
| `Property<Vector2>` | 0x10 | `[vftable][+4=pad][+8..0xc=x,y floats]` |
| `Property<Vector4>` | 0x18 | `[vftable][+4=pad][+8..0x14=xyzw floats]` |

The `CMEPropertyTree_Set*` functions at `015a0210` (UInt16), `015a0320` (UInt64), `015a0430` (Vector2), `015a0540` (Vector4) wrap these allocators with property-tree insert/update via `FUN_00438990` + `FUN_0043b710`.

---

## 13. Sub-Slot Client Method Encoding — Final Confirmation (W-entity-desc-B, 2026-05-13)

**Status: CONFIRMED** from W4-B1 evidence. No new binary evidence found in the `[0x01599000, 0x015c0000)` range changes this conclusion.

From W4-B1 (`worker-4b1.checkpoint.json`):

- `EntityDescription_AssignClientMethodIds` at `0x01590df0`: Iterates client methods. When `methodCount >= 0x3e` (62), switches to sub-slot encoding.
- `EntityDescription_DecodeClientMethodId` at `0x01590ee0`: Decodes multi-byte method IDs for methods at index 62+.
- Threshold `0x3e = 62` matches the BigWorld 2.0.1 `entity_method_descriptions.cpp::checkExposedForSubSlots()` boundary exactly.

**SGWPlayer has 157 client methods total** (sum across all parsed entities in its hierarchy). Sub-slot encoding applies to all methods at index 62 and above. This is handled transparently by the W4-B1 functions and does not require changes to the higher-level serialization logic documented in this file.

---

## Related Documents

- [Combat Wire Formats](combat-wire-formats.md) — Method call serialization
- [Inventory Wire Formats](inventory-wire-formats.md) — Inventory message formats
- [Entity Property Sync Protocol](../../protocol/entity-property-sync.md) — Higher-level protocol doc
- [Entity Type Catalog](../../engine/entity-type-catalog.md) — All entity definitions
