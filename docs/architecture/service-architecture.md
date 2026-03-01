# Cimmeria Service Architecture

> **Last updated**: 2026-03-01
> **RE Status**: Fully documented from source code
> **Sources**: `src/authentication/`, `src/baseapp/`, `src/cellapp/`, `config/`, `src/mercury/`

---

## Overview

Cimmeria implements three server processes that together emulate the Stargate Worlds server cluster. Each process has a distinct role and communicates with the others via internal Mercury protocol.

```
                           Internet
                              |
                    +-------------------+
                    | Authentication    |
                    | Server            |
                    | (HTTP/SOAP :8081) |
                    +-------------------+
                              |
                    Mercury TCP (:13001)
                              |
                    +-------------------+           Mercury TCP
                    | BaseApp           |<--------->+-----------+
                    | (UDP :32832)      |  (:13002) | CellApp   |
                    | (TCP :13002)      |           |           |
                    +-------------------+           +-----------+
                          |       |
                     Mercury UDP  |
                     (encrypted)  |
                          |       |
                    +--------+ +--------+
                    |Client 1| |Client 2|
                    +--------+ +--------+
```

## Authentication Server

### Role

Handles player login and shard selection via SOAP/HTTP. Does not participate in gameplay.

### Source Files

| File | Description |
|------|-------------|
| `src/authentication/service_main.hpp/cpp` | Service entry point and lifecycle |
| `src/authentication/frontend_connection.hpp/cpp` | HTTP client connection handler |
| `src/authentication/logon_connection.hpp/cpp` | Manages individual login sessions |
| `src/authentication/logon_queue.hpp/cpp` | Queue for pending login requests |
| `src/authentication/shard_client.hpp/cpp` | Connection to BaseApp for shard management |
| `src/authentication/launcher.cpp` | Process entry point |

### Configuration

`config/AuthenticationService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `logon_service_port` | 8081 | HTTP port for client SOAP requests |
| `base_service_port` | 13001 | Mercury TCP port for BaseApp communication |
| `db_connection_string` | (see config) | PostgreSQL connection |
| `protocol_digest` | MD5 hash | Entity definition checksum |
| `log_level` | DEBUG1 | Logging verbosity |

### Endpoints

| Path | Method | Description |
|------|--------|-------------|
| `/SGWLogin/UserAuth` | POST (SOAP) | Authenticate username/password |
| `/SGWLogin/ServerSelection` | POST (SOAP) | Select shard, get BaseApp address |

### Internal Protocol

The Auth Server communicates with the BaseApp via the unified protocol (`src/mercury/unified_protocol.hpp`):

| Direction | Message | Description |
|-----------|---------|-------------|
| Base->Auth | `FES_REGISTER_SHARD (0x10)` | Register as available shard |
| Auth->Base | `FES_REGISTER_SHARD_ACK (0x11)` | Confirm registration |
| Base->Auth | `FES_UPDATE_SHARD_STATUS (0x12)` | Population/status update |
| Auth->Base | `FES_LOGON_NOTIFICATION (0x13)` | Player authenticated |
| Auth->Base | `FES_LOGOFF_NOTIFICATION (0x14)` | Player disconnected |
| Auth->Base | `FES_KICK_PLAYER (0x15)` | Force disconnect |
| Base->Auth | `FES_KICK_PLAYER_ACK (0x16)` | Kick confirmed |
| Auth->Base | `FES_REQUEST_LOGON (0x17)` | Can this player log in? |
| Base->Auth | `FES_LOGON_ACK (0x18)` | Yes, login approved |
| Base->Auth | `FES_LOGON_NAK (0x19)` | No, login rejected |
| Both | `GENERIC_KEEPALIVE (0xFF)` | Connection keepalive |

## BaseApp

### Role

The core server process. Manages player sessions, acts as proxy between clients and the CellApp, handles persistence, and runs base entity Python scripts.

### Source Files

| File | Description |
|------|-------------|
| `src/baseapp/base_service.hpp/cpp` | Service lifecycle and configuration |
| `src/baseapp/launcher.cpp` | Process entry point |
| `src/baseapp/cell_manager.hpp/cpp` | CellApp connection management |
| `src/baseapp/entity/base_entity.hpp/cpp` | Base entity implementation |
| `src/baseapp/entity/base_entity_manager.hpp/cpp` | Entity creation/destruction |
| `src/baseapp/entity/cached_entity.hpp/cpp` | Entity cache stamp system |
| `src/baseapp/entity/ticker.hpp/cpp` | Game tick management |
| `src/baseapp/entity/base_py_util.hpp/cpp` | Python utility functions |
| `src/baseapp/mercury/sgw/client_handler.hpp/cpp` | Client Mercury message handling |
| `src/baseapp/mercury/sgw/connect_handler.hpp/cpp` | Client connection setup |
| `src/baseapp/mercury/sgw/messages.hpp/cpp` | Client message format tables |
| `src/baseapp/mercury/sgw/resource.hpp/cpp` | Cooked data serving |
| `src/baseapp/mercury/cell_handler.hpp/cpp` | CellApp message handling |
| `src/baseapp/minigame.hpp/cpp` | Minigame server |
| `src/baseapp/minigame_connection.hpp/cpp` | Minigame TCP connections |

### Configuration

`config/BaseService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `shard_name` | "Shard" | Display name for this server |
| `shard_id` | 1 | Unique shard identifier |
| `shard_port` | 32832 | UDP port for client connections |
| `shard_address` | 0.0.0.0 | Bind address |
| `shard_external_address` | 127.0.0.1 | Address given to clients |
| `service_port` | 13002 | Internal service TCP port |
| `auth_server_address` | 127.0.0.1 | Auth Server address |
| `auth_server_port` | 13001 | Auth Server port |
| `client_inactivity_timeout` | 300000 ms | Client disconnect timeout |
| `tick_rate` | 100 ms | Server tick interval |
| `nub_tickrate` | 25 ms | Network flush interval |
| `grid_chunk_size` | 50 | World grid chunk size |
| `grid_vision_distance` | 3 | AoI range in chunks |
| `grid_hysteresis` | 1 | AoI hysteresis in chunks |
| `cache_on_client` | true | Cache entities on client leave |
| `unreliable_tick_sync` | false | Send ticks unreliably |
| `unreliable_movement_update` | false | Send movement unreliably |
| `console_port` | 8989 | Python console port |
| `developer_mode` | false | Relaxed auth, elevated logging |

### Responsibilities

1. **Client proxy**: Maintains Mercury channels to connected clients
2. **Entity management**: Creates/destroys base entities, manages persistence
3. **Message routing**: Forwards client messages to CellApp and vice versa
4. **Cooked data**: Generates and serves PAK file data to clients
5. **Tick sync**: Sends periodic time synchronization to clients
6. **Auth bridge**: Registers with Auth Server, handles login/logoff
7. **AoI relay**: Receives AoI events from CellApp, sends entity create/destroy to clients
8. **Minigame host**: Runs TCP minigame server on configurable port

## CellApp

### Role

Runs the spatial simulation. Manages spaces, entity positions, movement, combat, AI, and Python cell scripts.

### Source Files

| File | Description |
|------|-------------|
| `src/cellapp/cell_service.hpp/cpp` | Service lifecycle and CellApp startup |
| `src/cellapp/cell_launcher.cpp` | Process entry point |
| `src/cellapp/space.hpp/cpp` | Space and SpaceManager implementation |
| `src/cellapp/base_client.hpp/cpp` | Connection to BaseApp |
| `src/cellapp/entity/cell_entity.hpp/cpp` | Cell entity implementation |
| `src/cellapp/entity/entity_manager.hpp/cpp` | Entity creation/destruction |
| `src/cellapp/entity/movement.hpp/cpp` | Movement controllers |
| `src/cellapp/entity/navigation.hpp/cpp` | Navmesh pathfinding |
| `src/cellapp/entity/cell_py_util.hpp/cpp` | Python utility functions |

### Configuration

`config/CellService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `cell_id` | 1 | Unique CellApp identifier |
| `baseapp_address` | 127.0.0.1 | BaseApp address |
| `baseapp_port` | 13002 | BaseApp service port |
| `idle_update_ticks` | 10 | Ticks between idle updates |
| `cache_entity_messages` | true | Cache static NPC messages |
| `cache_player_messages` | false | Cache static player messages |
| `console_port` | 8990 | Python console port |
| `developer_mode` | false | Developer mode |

### Base-Cell Protocol

The CellApp and BaseApp communicate using the protocol defined in `src/mercury/base_cell_messages.hpp`. Protocol version: 391.

See [Mercury Wire Format](../protocol/mercury-wire-format.md) for the full message list.

## Shared Components

### UnifiedKernel

Static library linked by all three servers:

| Component | Files | Description |
|-----------|-------|-------------|
| Mercury | `src/mercury/` | UDP/TCP networking |
| Entity | `src/entity/` | Entity definitions, Python integration |
| Common | `src/common/` | Database, console, service base class |
| XML | `src/xml/` | TinyXML2 parser |
| Logging | `src/log/` | Logger implementation |
| Util | `src/util/` | Crash dump, singletons |

### Service Base Class

All three servers inherit from a common `Service` base class (`src/common/service.hpp`) that provides:

- Configuration file loading
- Database connection management
- Timer management
- Logging setup
- Graceful shutdown

### Database

PostgreSQL accessed via SOCI 3.2.1 (`src/common/database.hpp`):

- Connection string configurable per service
- Used for account data, character data, game resources
- Schema in `db/sgw.sql` (accounts/characters) and `db/resources.sql` (game data)

### Python Console

Both BaseApp and CellApp offer a Python REPL console for remote administration:

| Service | Default Port | Description |
|---------|-------------|-------------|
| BaseApp | 8989 | Base entity administration |
| CellApp | 8990 | Cell entity administration |

Access requires a password set in the config file (`py_console_password`). The console is disabled if no password is configured.

## Startup Sequence

### 1. CellApp Starts

```
1. Load config/CellService.config
2. Connect to PostgreSQL database
3. Load entity definitions (entities/)
4. Initialize Python interpreter
5. Load space definitions (entities/spaces.xml, entities/cell_spaces.xml)
6. Create initial spaces with navmesh data
7. Connect to BaseApp (baseapp_address:baseapp_port)
8. Send CELL_BASE_AUTHENTICATE handshake
9. Receive BASE_CELL_AUTHENTICATE_ACK
10. Send CELL_BASE_SPACE_DATA for each created space
11. Start game tick timer
12. Start Python console (if password configured)
```

### 2. BaseApp Starts

```
1. Load config/BaseService.config
2. Connect to PostgreSQL database
3. Load entity definitions (entities/)
4. Initialize Python interpreter
5. Initialize cooked data pipeline
6. Accept CellApp connection on service_port
7. Connect to Auth Server (auth_server_address:auth_server_port)
8. Send FES_REGISTER_SHARD
9. Receive FES_REGISTER_SHARD_ACK
10. Start UDP listener on shard_port for clients
11. Start minigame TCP server
12. Start game tick timer
13. Start Python console (if password configured)
```

### 3. Auth Server Starts

```
1. Load config/AuthenticationService.config
2. Connect to PostgreSQL database
3. Start HTTP listener on logon_service_port
4. Accept BaseApp connection on base_service_port
5. Ready for client logins
```

## Related Documents

- [BigWorld Architecture](../engine/bigworld-architecture.md) -- BigWorld reference architecture
- [Login Handshake](../protocol/login-handshake.md) -- Connection establishment
- [Mercury Wire Format](../protocol/mercury-wire-format.md) -- Protocol details
- [Space Management](../engine/space-management.md) -- CellApp world management
- [Connection Flow](../connection-flow.md) -- End-to-end player connection

## TODO

- [ ] Document the minigame server protocol (TCP format)
- [ ] Document the entity backup/restore system between Base and Cell
- [ ] Document the developer mode behaviors in detail
- [ ] Map all Python console commands available on each service
- [ ] Document graceful shutdown sequence and entity persistence
