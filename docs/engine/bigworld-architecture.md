# BigWorld Architecture

> **Last updated**: 2026-03-08
> **RE Status**: Well understood from BigWorld reference source + Cimmeria implementation
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/`, BigWorld-Engine-1.9.1 (client match), `src/`, `docs/how-sgw-works.md`

---

## Overview

BigWorld Technology is an Australian MMO middleware platform that provides the networking, entity management, and server architecture for Stargate Worlds. SGW uses BigWorld ≥1.8.1 (confirmed via deprecation string `"The use of BW_RES_PATH environment variable is deprecated post 1.8.1"` in SGW.exe). Client-side Mercury networking matches BigWorld 1.9.1 source 1:1 (77+ debug strings verified). Server-side reference from BW 2.0.1 is also used. CME made extensive modifications to the application layer but left the Mercury networking core unmodified.

This document describes the BigWorld architectural concepts as they apply to SGW and Cimmeria.

## Core Concept: The Distributed Entity

The fundamental concept in BigWorld is the **distributed entity** -- a single game object (player, NPC, item) that has synchronized instances across multiple processes:

```
                    Entity "SGWPlayer"
                    /        |        \
               Base         Cell       Client
           (BaseApp)     (CellApp)   (SGW.exe)
              |              |            |
        Persistent       Spatial      Rendered
          State         Simulation    Representation
```

### Entity Split

| Component | Lives On | Responsible For |
|-----------|----------|-----------------|
| **Base** | BaseApp | Persistent state, database I/O, client connection proxy |
| **Cell** | CellApp | Spatial simulation, movement, combat, AI, AoI |
| **Client** | SGW.exe | Rendering, input, UI, prediction |

Each entity type defines which components it has via the `.def` file and which Python scripts exist:

```xml
<!-- entities/defs/SGWPlayer.def -->
<root>
    <Parent>SGWBeing</Parent>
    <Implements>
        <Interface>Communicator</Interface>
        <Interface>SGWInventoryManager</Interface>
        <!-- ... 9 more interfaces -->
    </Implements>
    <Properties>
        <!-- Properties with Flags determine where they exist -->
    </Properties>
</root>
```

## Server Architecture

### Standard BigWorld Components

| Component | Description | SGW/Cimmeria |
|-----------|-------------|--------------|
| **LoginApp** | Handles initial Mercury-based login | Replaced by AuthenticationServer (SOAP) |
| **BaseAppMgr** | Assigns players to BaseApps | Simplified; single BaseApp |
| **BaseApp** | Manages base entities, client proxies | Implemented in `src/baseapp/` |
| **CellApp** | Spatial simulation, movement, combat | Implemented in `src/cellapp/` |
| **CellAppMgr** | Manages cell space distribution | Not implemented; single CellApp |
| **DBMgr** | Database operations (MySQL in BW) | Replaced by direct PostgreSQL via SOCI |
| **Reviver** | Watchdog for crashed processes | Not implemented |
| **MessageLogger** | Central log aggregation | Replaced by file-based logging |

### Cimmeria Simplifications

SGW was designed for a single-server deployment. Cimmeria reflects this:

- **Single BaseApp**: No BaseAppMgr needed; one process handles all players
- **Single CellApp**: No CellAppMgr needed; one process runs all spaces
- **SOAP Auth**: CME replaced BigWorld's LoginApp with HTTP/SOAP authentication
- **PostgreSQL**: CME replaced BigWorld's MySQL/XML DBMgr with PostgreSQL + SOCI
- **No Reviver**: Development environment; process restarts are manual

## Mercury Networking Layer

Mercury is BigWorld's custom reliable UDP protocol. See [Mercury Wire Format](../protocol/mercury-wire-format.md) for details.

Key characteristics:
- UDP-based with selective reliability
- Channel abstraction for persistent connections
- Bundle-based message grouping
- Packet fragmentation for large messages
- Encryption (AES-256 in SGW, Blowfish in standard BW)

### Communication Paths

```
Client <-- Mercury/UDP (encrypted) --> BaseApp
                                        |
                                   Mercury/TCP
                                   (internal)
                                        |
                                      CellApp

AuthServer <-- Mercury/TCP --> BaseApp
```

### Mercury::Nub Threading Model (from SGW.exe RE)

Mercury::Nub (ctor at `0x015841d0`) creates a background "NetworkThread for ExternalNub" that handles raw UDP I/O independently of the game thread:

```
NetworkThread                           Game Thread (UE3)
-----------                             ----------------
Nub::processPendingEvents()             UGameEngine::Tick()
  recvfrom() loop                         TickDispatch()
  parse packet headers/footers             pop from concurrent_queue
  wrap as ClientMessage                    dispatch through interface handlers
  push to tbb::concurrent_queue           try/catch NubException
```

Two `tbb::concurrent_queue<RefCountedObj<ClientMessage>>` at Nub offsets +0x138 and +0x150 bridge the threads.

Socket errors (WSAETIMEDOUT, WSAECONNRESET, WSAECONNREFUSED) are also delivered as NubException objects through the queue. The game thread catches these in `ServerConnection::processInput` and handles:
- `REASON_CORRUPTED (-4)` → drop packet, continue
- `REASON_DISCONNECTED (-2)` / `REASON_TIMEOUT (-7)` → disconnect
- Other → log and continue

### SOAP Login Flow (from SGW.exe RE)

CME replaced BigWorld's standard LoginApp with HTTP/SOAP authentication using curl/gSOAP:

```
1. UE3 creates UNetPendingLevel → UBWNetDriver → UBWConnection
2. ServerConnection::logOnBegin:
   └── curl POST to /SGWLogin/UserAuth (async via curl_multi)
       └── SGWLoginRequest (gSOAP, namespace: sgwlogin)
3. SGWLoginResponse → SGWLoginSuccess contains:
   └── BaseAppAddress, ServerName, Load, Ticket, ShardList
4. BaseAppLoginHandler → Mercury::Channel to BaseApp
   └── Sends "baseAppLogin" message
5. BaseApp replies → channel transferred to ServerConnection::pChannel_
6. ServerConnection::logOn → sends "authenticate" message
7. ServerConnection::enableEntities → entity streaming begins
```

SOAP namespace: `http://www.stargateworlds.com/xml/sgwlogin`
Auth endpoint: `/SGWLogin/UserAuth`
Server select: `/SGWLogin/ServerSelection`

## Entity System

### Entity Type Registry

All entity types are defined in `entities/entities.xml` and parsed at startup. Each type has:

- A `.def` file in `entities/defs/` defining properties and methods
- Optional parent type (inheritance)
- Optional interface implementations
- Python scripts in `python/base/` and `python/cell/`

### Entity Type Hierarchy

```
Account (standalone -- no cell component)
SGWEntity (base entity type)
+-- SGWSpawnableEntity
|   +-- SGWBeing (has SGWBeing interface)
|   |   +-- SGWMob (NPCs and monsters)
|   |   |   +-- SGWPet
|   |   +-- SGWPlayer (11 interfaces)
|   |       +-- SGWGmPlayer
|   +-- SGWDuelMarker
+-- SGWBlackMarket
+-- SGWCoverSet
+-- SGWEscrow
+-- SGWSpaceCreator
+-- SGWSpawnRegion
+-- SGWSpawnSet
+-- SGWPlayerRespawner
+-- SGWChannelManager
+-- SGWPlayerGroupAuthority
```

### Entity IDs

From `src/mercury/base_cell_messages.hpp`:

| Range | Owner | Description |
|-------|-------|-------------|
| `0x00000001 - 0x0FFFFFFF` | BaseApp | Base entity IDs |
| `0x10000000 + (cellId << 21)` | CellApp | Cell-local entity IDs |
| `> 0x40000000` | Client | Client-only entity IDs |

### Entity Methods (RPC)

Methods defined in `.def` files become remotely callable:

- **Client methods**: Server calls methods on the client (server-to-client RPC)
- **Base methods**: Cell or client calls methods on the base entity
- **Cell methods**: Base or client calls methods on the cell entity
- **Exposed methods**: Cell methods callable from the client (client-to-server RPC)

## Space and Cell Management

### Spaces

A space is a self-contained game world. Each space has:
- A unique Space ID
- A world name (references map data)
- Entity instances within it

SGW defines 16 spaces in `entities/cell_spaces.xml`:
- SandBox, Castle, Agnos, Lucia, Omega_Site, Tollana, Agnos_Library, Sewer_Falls, Harset, Harset_CmdCenter, Dakara_E1, Ihpet_Crater_Dark, Ihpet_Crater_Light, Menfa_Dark, Menfa_Light, Beta_Site_Evo_1

### Cells

In standard BigWorld, a large space can be split across multiple CellApps using cell boundaries. Each cell manages a rectangular region of the space. Entities near cell boundaries are "ghosted" to adjacent cells.

SGW does not use multi-cell spaces. Each space runs entirely on the single CellApp.

### Area of Interest (AoI)

The AoI system determines which entities are visible to each player:

1. The world is divided into a grid of chunks (`grid_chunk_size = 50` meters)
2. Each player can see entities within `grid_vision_distance = 3` chunks
3. Hysteresis (`grid_hysteresis = 1` chunk) prevents flicker at boundaries
4. When an entity enters a player's AoI, a `createEntity` message is sent
5. When it leaves, a `deleteEntity` or cache-hide message is sent

See [Space Management](space-management.md) for implementation details.

## Python Scripting

Game logic is written in Python 3.4 embedded via Boost.Python:

```
python/
  base/         -- Base entity scripts (persistent logic)
    Account.py
    SGWPlayer.py
    ...
  cell/         -- Cell entity scripts (spatial logic)
    SGWPlayer.py
    SGWMob.py
    ...
    missions/   -- Auto-generated mission scripts
```

Each entity type has corresponding Python classes that implement the methods declared in the `.def` file. The C++ engine calls into Python for game logic, and Python can call back into C++ for engine operations.

## Configuration System

BigWorld uses XML configuration files. Cimmeria follows this pattern:

| File | Purpose |
|------|---------|
| `config/AuthenticationService.config` | Auth server settings |
| `config/BaseService.config` | BaseApp settings |
| `config/CellService.config` | CellApp settings |
| `entities/entities.xml` | Entity type registry |
| `entities/cell_spaces.xml` | Initial space list |
| `entities/spaces.xml` | Space world definitions |

Each config file supports a `.local` override for environment-specific settings.

## Related Documents

- [Mercury Wire Format](../protocol/mercury-wire-format.md) -- Networking protocol
- [Space Management](space-management.md) -- World grid and AoI details
- [CME Framework](cme-framework.md) -- SGW-specific extensions
- [Service Architecture](../architecture/service-architecture.md) -- Cimmeria's implementation
- [How SGW Works](../how-sgw-works.md) -- High-level overview

## Entity Serialization Format for Database Persistence

> **Source**: `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/entity_description.cpp`, `src/server/dbmgr/database.cpp`

BigWorld serializes entity properties to a binary stream for database persistence using a **4-pass system**. The same streaming infrastructure is used for network transfer and DB writes, with the `ONLY_PERSISTENT_DATA` flag filtering out non-persistent properties during DB operations.

### The 4-Pass Serialization System

`EntityDescription::addToStream()` iterates over all properties in 4 passes, each targeting a different data domain:

| Pass | Data Category | BASE flag | CLIENT flag | Description |
|------|--------------|-----------|-------------|-------------|
| 0 | Base-only | true | false | Properties with `BASE` flag but not `OWN_CLIENT` |
| 1 | Base + Client | true | true | Properties with `BASE_AND_CLIENT` flag |
| 2 | Cell + Client | false | true | Properties with `ALL_CLIENTS`, `OWN_CLIENT`, `OTHER_CLIENTS` |
| 3 | Cell-only | false | false | Properties with `CELL_PRIVATE` or `CELL_PUBLIC` (no client flag) |

The caller specifies which passes to execute via `dataDomains` bitmask flags:

```cpp
enum DataDomain {
    BASE_DATA   = 0x1,   // Include passes 0, 1
    CLIENT_DATA = 0x2,   // Include passes 1, 2
    CELL_DATA   = 0x4,   // Include passes 2, 3
    EXACT_MATCH = 0x8,   // Require all flags to match (not just any)
    ONLY_OTHER_CLIENT_DATA = 0x10,  // Filter: only OTHER_CLIENT properties
    ONLY_PERSISTENT_DATA   = 0x20   // Filter: only Persistent=true properties
};
```

### Pass Selection Logic

`shouldSkipPass(pass, dataDomains)` decides whether to execute a pass. Each pass maps to a required set of domain flags:

```
Pass 0 requires: BASE_DATA
Pass 1 requires: BASE_DATA | CLIENT_DATA
Pass 2 requires: CELL_DATA | CLIENT_DATA
Pass 3 requires: CELL_DATA
```

Without `EXACT_MATCH`, a pass runs if **any** of its required flags are set in `dataDomains`. With `EXACT_MATCH`, **all** flags must match exactly.

### Property Filtering Within a Pass

`shouldConsiderData(pass, pDD, dataDomains)` filters individual properties within an active pass:

1. The property's `isBaseData()` and `isClientServerData()` must match the pass's expected pattern
2. If `ONLY_OTHER_CLIENT_DATA` is set, the property must have `DATA_OTHER_CLIENT`
3. If `ONLY_PERSISTENT_DATA` is set, the property must have `DATA_PERSISTENT` (i.e., `<Persistent>true</Persistent>` in the .def)

### Database Write Path

When the BaseApp writes an entity to the database, it uses:

```cpp
// In database.cpp (defaultEntityToStrm):
desc.addSectionToStream(pSection, strm,
    EntityDescription::BASE_DATA | EntityDescription::CELL_DATA |
    EntityDescription::ONLY_PERSISTENT_DATA);
```

This executes all 4 passes but filters to only persistent properties. The resulting binary stream contains properties serialized in this fixed order:

1. **Pass 0**: Base-only persistent properties (e.g., `account` mailbox if persistent)
2. **Pass 1**: Base+Client persistent properties (e.g., character name, appearance)
3. **Pass 2**: Cell+Client persistent properties (e.g., player name, abilities)
4. **Pass 3**: Cell-only persistent properties (e.g., position, experience)

Each property is serialized by its `DataType::addToStream()` method (type-specific: INT32 as 4 bytes, STRING as length-prefixed, ARRAY as count + elements, etc.).

### BigWorld vs Cimmeria Database Layer

| Aspect | BigWorld (DBMgr) | Cimmeria |
|--------|-----------------|----------|
| Database | MySQL via `dbmgr_mysql` module | PostgreSQL via SOCI 3.2.1 |
| Process | Separate `DBMgr` process | Direct DB calls from BaseApp |
| Communication | Mercury messages between BaseApp and DBMgr | In-process function calls |
| Entity storage | One table per entity type (auto-generated schema) | PostgreSQL tables (`sgw.sql`) |
| Property format | Binary stream (`BinaryOStream`) decomposed into SQL columns | Binary stream or SQL columns via SOCI |
| Write trigger | `Database::writeEntity()` Mercury message | BaseApp persistence tick |

BigWorld's `DBMgr` receives entity data as a binary stream over Mercury (`writeEntity` message with flags byte, entity type ID, and database ID), then the `IDatabase` implementation (MySQL or XML) unpacks the stream into table rows. Cimmeria bypasses this with direct PostgreSQL writes from the BaseApp process.

## Mailbox System for Cross-Process Entity Communication

> **Source**: `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/mailbox_base.hpp`, `src/lib/network/basictypes.hpp`, `src/server/cellapp/mailbox.hpp`, `src/server/baseapp/mailbox.hpp`

A **mailbox** is BigWorld's abstraction for sending messages to a remote entity component. When Python scripts call methods on another entity's base, cell, or client, the call goes through a mailbox that serializes the method call and routes it over Mercury to the correct process.

### EntityMailBoxRef (Wire Format)

The compact wire representation of a mailbox is `EntityMailBoxRef` (12 bytes):

```
EntityMailBoxRef:
+------------------+------------------+
|    EntityID      |  Mercury::Address |
|    (4 bytes)     |  ip(4) port(2)   |
|                  |  salt(2)         |
+------------------+------------------+
                         |
               salt bits encode:
               [15:13] = Component (3 bits)
               [12:0]  = EntityTypeID (13 bits)
```

### Component Types

The `Component` enum encodes the target and routing path:

| Component | Value | Description |
|-----------|-------|-------------|
| `CELL` | 0 | Direct to cell entity on a CellApp |
| `BASE` | 1 | Direct to base entity on a BaseApp |
| `CLIENT` | 2 | Direct to client via its BaseApp proxy |
| `BASE_VIA_CELL` | 3 | To base entity, routed through CellApp |
| `CLIENT_VIA_CELL` | 4 | To client, routed through CellApp then BaseApp |
| `CELL_VIA_BASE` | 5 | To cell entity, routed through BaseApp |
| `CLIENT_VIA_BASE` | 6 | To client, routed through BaseApp |

The "VIA" variants enable indirect routing when the sender does not have a direct channel to the destination. For example, a CellApp entity that wants to call a method on another entity's base (on a different BaseApp) would use `BASE_VIA_CELL` to route through the local CellApp's channel.

### Python Mailbox Class Hierarchy

```
PyEntityMailBox (mailbox_base.hpp)
    Abstract base: findMethod(), getStream(), sendStream()
    |
    +-- ServerEntityMailBox (cellapp/mailbox.hpp)
    |       CellApp-side mailbox base class
    |       |
    |       +-- CellEntityMailBox
    |       |       Target: cell component on (possibly remote) CellApp
    |       |       Has base() and client() proxy getters
    |       |
    |       +-- BaseEntityMailBox
    |               Target: base component on a BaseApp
    |
    +-- (BaseApp variants in baseapp/mailbox.hpp)
            |
            +-- CommonCellEntityMailBox
            |       Target: cell entity from the BaseApp side
            |
            +-- CommonBaseEntityMailBox
                    Target: base entity from the BaseApp side
```

### Method Call Flow (getStream / sendStream)

When Python calls a method on a mailbox, the following sequence occurs:

1. **Attribute lookup**: `mailbox.methodName(args)` triggers `pyGetAttribute("methodName")`
2. **Method resolution**: `findMethod("methodName")` searches the entity type's method descriptions for the target component (base/cell/client)
3. **Stream creation**: `getStream(methodDesc)` opens a Mercury bundle to the target address, writes the method header (message ID, entity ID)
4. **Argument serialization**: Python arguments are serialized to the stream using each parameter's `DataType`
5. **Send**: `sendStream()` finalizes and dispatches the Mercury bundle

### Mailbox Pickling (Persistence)

Mailboxes can be pickled to `EntityMailBoxRef` for storage or transfer:

- `reduceToRef(PyObject*)` extracts the compact 12-byte representation
- `constructFromRef(EntityMailBoxRef&)` reconstructs a Python mailbox from the ref
- Component-specific factory functions are registered via `registerMailBoxComponentFactory()`

### Cimmeria Simplifications

Since Cimmeria uses a single BaseApp and single CellApp:

- The `addr` field in `EntityMailBoxRef` always points to the same processes
- The "VIA" routing variants are unnecessary (no multi-hop routing)
- Mailbox resolution is simplified since all entities are on known, fixed addresses

## BigWorld Version Differences: 1.9.x vs 2.0.x

> **Source**: `external/engines/BigWorld-Engine-2.0.1/src/lib/cstdmf/bwversion.hpp`
>
> **Note**: The reference source directory is named `BigWorld-Engine-2.0.1` but `bwversion.hpp` declares version `2.0.0` (`BW_VERSION_MAJOR=2, BW_VERSION_MINOR=0, BW_VERSION_PATCH=0`). Copyright states 1999-2010. No 1.9.1 reference source is available for direct comparison.

### What We Know

SGW was developed against BigWorld **~1.9.x** (estimated between 1.9.1 and 2.0.1). The exact version is unknown, but the following evidence constrains it:

| Evidence | Implication |
|----------|-------------|
| SGW uses AES-256 encryption | BW standard uses Blowfish; CME replaced this |
| SGW uses SOAP authentication | BW standard uses LoginApp; CME replaced this |
| SGW uses PostgreSQL | BW standard uses MySQL; CME replaced this |
| `BaseCellProtocolVersion = 391` in Cimmeria | Protocol version must match between BW library and game |
| Entity .def format matches BW 2.0.1 schema | .def parsing is compatible with the 2.0.1 reference |
| No `CellAppMgr` in SGW | Single-server design; may predate BW 2.0 multi-cell improvements |

### Known Differences Between BW 1.9.x and 2.0.x (from 2.0.1 source comments)

The BW 2.0.1 source contains several indicators of evolution from 1.9.x:

1. **EntityID type migration**: `basictypes.hpp` notes `EntityID` "replaces the legacy ObjectID type," suggesting the rename happened around the 1.9/2.0 boundary.

2. **Network compression**: `entity_description.hpp` has `internalNetworkCompressionType_` and `externalNetworkCompressionType_` fields with `BWCompressionType` enum, which may be a 2.0 addition for bandwidth optimization.

3. **Data consolidation**: `database.hpp` has a `Consolidator` class and consolidation methods (`consolidateData()`, `startConsolidationProcess()`), a feature for merging secondary database files that may be new in 2.0.

4. **Secondary database support**: `secondaryDBRegistration()`, `updateSecondaryDBs()`, and related methods in `Database` class suggest a secondary/backup database system added in 2.0.

5. **Auto-load entities**: `UpdateAutoLoad` enum in `putEntity()` indicates a 2.0 feature for entities that auto-load on server startup.

6. **Volatile info refactoring**: `VolatileInfo` is a separate class (`volatile_info.hpp`) in 2.0.1, possibly refactored from inline fields in 1.9.

### What SGW Likely Does Not Use from 2.0

Given SGW's single-server architecture, these 2.0 features are likely unused:

- Multi-BaseApp/CellApp manager improvements
- Data consolidation (for multi-DB secondary backup)
- Advanced network compression (single-server has low internal bandwidth needs)
- The Reviver watchdog enhancements

### Limitations

Without access to BW 1.9.1 source or official changelogs, a complete diff is not possible. The version mapping is based on inference from code comments, feature presence in 2.0.1, and SGW's known feature set.

## TODO

- [x] ~~Document the entity serialization format for database persistence~~ → 4-pass system with `shouldConsiderData`/`shouldSkipPass`; `ONLY_PERSISTENT_DATA` filters non-persistent properties. See "Entity Serialization Format" section above.
- [x] ~~Document the ghost entity system~~ → Full deep-dive in `engine/space-management.md` (Phase 5: Ghost Entity System section)
- [x] ~~Document the mailbox system for cross-process entity communication~~ → `EntityMailBoxRef` wire format (12 bytes, Component enum in salt bits), `PyEntityMailBox` class hierarchy with `getStream`/`sendStream` pattern. See "Mailbox System" section above.
- [x] ~~Map BigWorld version differences between 1.9.1 and 2.0.1 relevant to SGW~~ → Limited: no 1.9.1 source available. BW 2.0.1 header declares version 2.0.0 (copyright 1999-2010). Key differences inferred from code comments: EntityID rename, network compression, data consolidation, secondary DB support. See "BigWorld Version Differences" section above.
- [x] ~~Document entity creation/destruction lifecycle~~ → createBasePlayer/createCellPlayer in `findings/entity-property-sync.md`; entity lifecycle in `findings/entity-types-wire-formats.md`
