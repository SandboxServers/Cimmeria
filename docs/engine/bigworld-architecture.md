# BigWorld Architecture

> **Last updated**: 2026-03-01
> **RE Status**: Well understood from BigWorld 2.0.1 reference source + Cimmeria implementation
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/`, `src/`, `docs/how-sgw-works.md`

---

## Overview

BigWorld Technology is an Australian MMO middleware platform that provides the networking, entity management, and server architecture for Stargate Worlds. SGW uses BigWorld ~1.9.x (between versions 1.9.1 and 2.0.1) with extensive modifications by Cheyenne Mountain Entertainment (CME).

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

## TODO

- [ ] Document the entity serialization format for database persistence
- [ ] Document the ghost entity system (DATA_GHOSTED) for multi-cell understanding
- [ ] Document the mailbox system for cross-process entity communication
- [ ] Map BigWorld version differences between 1.9.1 and 2.0.1 relevant to SGW
- [ ] Document the entity creation/destruction lifecycle in detail
