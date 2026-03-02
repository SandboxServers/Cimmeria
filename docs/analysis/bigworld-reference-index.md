# BigWorld Reference Source Index

> **Last updated**: 2026-03-01
> **RE Status**: Index of available reference source for reverse engineering
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/lib/`

---

## Overview

The Cimmeria project includes a copy of BigWorld Technology Engine 2.0.1 source code at `external/engines/BigWorld-Engine-2.0.1/`. This is the reference implementation of the engine that Cheyenne Mountain Entertainment licensed and modified for Stargate Worlds. The source is invaluable for reverse engineering SGW.exe and understanding the protocol.

This document indexes the BigWorld library source by topic area, noting which files have been used for Cimmeria development and which are relevant for ongoing RE work.

**Important**: The BigWorld source is in the `external/` directory which is excluded from git via `.gitignore`. It must be obtained separately and placed in the correct path.

## Library Directory Inventory

The BigWorld 2.0.1 `src/lib/` directory contains 47 subdirectories:

| Directory | Category | Relevance | Description |
|-----------|----------|-----------|-------------|
| `network/` | Networking | CRITICAL | Mercury protocol, channels, packets, bundles |
| `connection/` | Networking | HIGH | Client connection management, login |
| `server/` | Server | HIGH | Server infrastructure, watcher system |
| `entitydef/` | Entity | CRITICAL | Entity definitions, data descriptions |
| `pyscript/` | Scripting | HIGH | Python scripting integration |
| `python/` | Scripting | MEDIUM | Python C extensions |
| `cstdmf/` | Core | HIGH | Core types, memory, debug, watcher base |
| `math/` | Core | MEDIUM | Math library (vectors, matrices) |
| `resmgr/` | Resources | MEDIUM | Resource manager, file system |
| `chunk/` | World | MEDIUM | Chunk-based world streaming |
| `waypoint/` | Navigation | LOW | Waypoint navigation (pre-Recast) |
| `waypoint_generator/` | Navigation | LOW | Waypoint mesh generation |
| `physics2/` | Physics | LOW | Physics system |
| `model/` | Rendering | NONE | Model loading (client-only) |
| `moo/` | Rendering | NONE | Moo rendering engine |
| `romp/` | Rendering | NONE | Rendering pipeline |
| `terrain/` | Rendering | NONE | Terrain rendering |
| `particle/` | Rendering | NONE | Particle effects |
| `duplo/` | Rendering | NONE | Entity visual attachments |
| `camera/` | Rendering | NONE | Camera system |
| `speedtree/` | Rendering | NONE | SpeedTree integration |
| `speedtreert/` | Rendering | NONE | SpeedTree runtime |
| `post_processing/` | Rendering | NONE | Post-processing effects |
| `scaleform/` | UI | NONE | Scaleform GFx integration |
| `guimanager/` | UI | NONE | GUI management |
| `guitabs/` | UI | NONE | GUI tab system |
| `ual/` | UI | NONE | Asset browser UI |
| `web_render/` | UI | NONE | Web rendering (Chromium) |
| `input/` | Input | NONE | Input handling |
| `controls/` | Input | NONE | Control binding |
| `gizmo/` | Tools | NONE | Editor gizmos |
| `appmgr/` | Core | LOW | Application manager |
| `dbmgr_lib/` | Database | MEDIUM | Database manager library |
| `dbmgr_mysql/` | Database | LOW | MySQL-specific DB code |
| `dbmgr_xml/` | Database | LOW | XML-based DB fallback |
| `fmod/` | Audio | NONE | FMOD sound integration |
| `fmodsound/` | Audio | NONE | FMOD sound wrappers |
| `sound/` | Audio | NONE | Sound system |
| `xactsnd/` | Audio | NONE | XACT sound integration |
| `emptyvoip/` | Audio | NONE | VoIP stub |
| `open_automate/` | Testing | NONE | Automation framework |
| `unit_test_lib/` | Testing | NONE | Unit test utilities |
| `job_system/` | Core | LOW | Job/task scheduling |
| `sqlite/` | Database | NONE | SQLite integration |
| `png/` | Utility | NONE | PNG image handling |
| `zip/` | Utility | LOW | ZIP file handling |
| `third_party/` | Vendor | LOW | Third-party code |
| `ashes/` | Unknown | NONE | Unknown subsystem |

## Critical Reference Files

### Network Layer (`network/`)

These files define the Mercury protocol that Cimmeria reimplements.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `packet.hpp` | YES | Packet flags, MAX_SIZE, header/footer format |
| `bundle.hpp` | YES | Bundle building, reliability levels (RELIABLE_NO through RELIABLE_CRITICAL) |
| `channel.hpp` | YES | Channel traits (INTERNAL/EXTERNAL), send/receive windows, ACK handling |
| `interface_element.hpp` | YES | Message types (FIXED/VARIABLE/INVALID), REPLY_MESSAGE_IDENTIFIER=0xFF |
| `misc.hpp` | YES | SeqNum, MessageID, ChannelID, ReplyID, Reason enum |
| `basictypes.hpp` | YES | EntityID, SpaceID, EntityTypeID, DatabaseID, SessionKey, GameTime, Address |
| `network_interface.hpp` | Reference | NetworkInterface (Cimmeria equivalent: Nub) |
| `nub.hpp` | Reference | Legacy Nub class |
| `watcher_nub.hpp` | Reference | Watcher network endpoint |
| `watcher_connection.hpp` | Reference | Watcher client connection handler |
| `watcher_packet_handler.hpp` | Reference | Watcher packet processing |
| `watcher_glue.hpp` | Reference | Watcher-to-network bridge |
| `encryption_filter.hpp` | Reference | Blowfish encryption (CME replaced with AES-256) |
| `reliable_order.hpp` | Reference | Reliable message ordering |
| `condemned_channels.hpp` | Reference | Channel teardown |
| `irregular_channels.hpp` | Reference | Non-standard channel handling |

### Connection Layer (`connection/`)

Client connection management used for understanding the login handshake.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `common_client_interface.hpp` | YES | 32 avatarUpdate variants, tickSync, relativePosition, setVehicle |
| `server_connection.hpp` | Reference | Client-side server connection management |
| `login_interface.hpp` | Reference | Login protocol (CME replaced with SOAP) |
| `smart_server_connection.hpp` | Reference | Auto-reconnecting connection |

### Entity Definition Layer (`entitydef/`)

Entity type system used for property/method definition format.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `data_description.hpp` | YES | EntityDataFlags (DATA_GHOSTED=0x01 through DATA_ID=0x80) |
| `entity_description.hpp` | YES | DataDomain (BASE_DATA, CLIENT_DATA, CELL_DATA), method collections |
| `volatile_info.hpp` | YES | VolatileInfo: position/yaw/pitch/roll priority levels |
| `property_change.hpp` | YES | PropertyChange path system (top-level, slice, nested) |
| `entity_description_map.hpp` | Reference | Registry of all entity types |
| `method_description.hpp` | Reference | Individual method metadata |
| `data_type.hpp` | Reference | Data type descriptors |
| `data_types.hpp` | Reference | Built-in data type registry |
| `member_description.hpp` | Reference | Base class for data/method descriptions |
| `data_instances.hpp` | Reference | Default value system |
| `data_lod.hpp` | Reference | Level-of-detail data descriptions |

### Python Scripting (`pyscript/`)

Python embedding used for entity scripting.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `script.hpp` | Reference | Python interpreter initialization |
| `py_script_object.hpp` | Reference | Python object wrapping |
| `py_output_writer.hpp` | Reference | Python stdout/stderr redirection |
| `personality.hpp` | Reference | Module-level personality scripts |

### Server Infrastructure (`server/`)

Server-side utilities and monitoring.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `watcher_protocol.hpp` | Reference | Watcher binary protocol v2 |
| `watcher_forwarding.hpp` | Reference | Multi-process watcher query forwarding |
| `watcher_forwarding_collector.hpp` | Reference | Response aggregation |
| `server_app.hpp` | Reference | Base server application class |
| `server_app_option.hpp` | Reference | Command-line option parsing |
| `bwconfig.hpp` | Reference | Configuration file loading |
| `reviver_subject.hpp` | Reference | Process monitoring and restart |

### Core Utilities (`cstdmf/`)

Foundational types and debugging.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `watcher.hpp` | Reference | Watcher base classes (DataWatcher, FunctionWatcher, DirectoryWatcher) |
| `profile.hpp` | Reference | Performance profiling macros |
| `debug.hpp` | Reference | Debug output, assertions |
| `md5.hpp` | Reference | MD5 hashing |
| `binary_stream.hpp` | Reference | Binary serialization helpers |
| `smartpointer.hpp` | Reference | Reference-counted smart pointers |
| `timestamp.hpp` | Reference | High-resolution timestamps |
| `memory_tracker.hpp` | Reference | Memory allocation tracking |

### Resource Management (`resmgr/`)

File system and resource loading.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `datasection.hpp` | Reference | XML/binary data section interface |
| `xml_section.hpp` | Reference | XML data section implementation |
| `bwresource.hpp` | Reference | Resource path management |
| `zip_section.hpp` | Reference | ZIP-based resource loading |

### Database Layer (`dbmgr_lib/`)

Database abstraction (Cimmeria uses SOCI instead).

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `entity_auto_loader.hpp` | Reference | Auto-load entities from DB on startup |
| `db_entitydefs.hpp` | Reference | Entity schema in database |
| `write_entity_handler.hpp` | Reference | Entity persistence write handling |

### World/Chunk System (`chunk/`)

Spatial world management.

| File | Used in Cimmeria | Key Content |
|------|-----------------|-------------|
| `chunk_space.hpp` | Reference | Space-chunk relationship |
| `chunk_item.hpp` | Reference | Items within chunks |
| `chunk_manager.hpp` | Reference | Chunk loading/unloading manager |
| `geometry_mapping.hpp` | Reference | World geometry file mapping |

## Usage Patterns

### Files Directly Informing Cimmeria Implementation

These BigWorld files were studied to build Cimmeria's reimplementation:

```
network/packet.hpp          -> src/mercury/packet.hpp
network/bundle.hpp          -> src/mercury/bundle.hpp
network/channel.hpp         -> src/mercury/channel.hpp
network/misc.hpp            -> src/mercury/message.hpp
network/basictypes.hpp      -> src/mercury/message.hpp (types)
entitydef/data_description.hpp -> src/entity/ (property flags)
connection/common_client_interface.hpp -> src/baseapp/mercury/sgw/messages.hpp
```

### Files Useful for Ongoing RE

When reverse engineering a specific SGW.exe subsystem, consult:

| RE Target | BigWorld Reference Files |
|-----------|------------------------|
| Login handshake | `connection/login_interface.hpp`, `connection/server_connection.hpp` |
| Entity creation | `entitydef/entity_description.hpp`, `entitydef/data_description.hpp` |
| Property sync | `entitydef/property_change.hpp`, `entitydef/data_description.hpp` |
| Position updates | `connection/common_client_interface.hpp` |
| Channel management | `network/channel.hpp`, `network/condemned_channels.hpp` |
| Encryption | `network/encryption_filter.hpp` |
| Resource loading | `resmgr/datasection.hpp`, `resmgr/zip_section.hpp` |
| Watcher monitoring | `cstdmf/watcher.hpp`, `server/watcher_protocol.hpp` |
| Entity persistence | `dbmgr_lib/entity_auto_loader.hpp` |

### CME Divergences from BigWorld

When using BigWorld source as reference, keep in mind that CME modified several subsystems:

| Subsystem | BigWorld 2.0.1 | CME/SGW Modification |
|-----------|---------------|---------------------|
| Encryption | Blowfish (`encryption_filter.hpp`) | AES-256-CBC + HMAC-MD5 |
| Authentication | Mercury LoginApp | HTTP/SOAP endpoints |
| Session key | 4-byte `SessionKey` (uint32) | 64-char hex string (256-bit) |
| Packet flags | uint16 (11 defined flags) | uint8 (8 flags) |
| Space loading | `spaceData` messages only | Added `spaceViewportInfo` |
| UI framework | None (BigWorld is server-only) | CEGUI + Scaleform/GFx |
| Event system | None | CME::EventSignal (954 events) |
| Data pipeline | Binary DataSections | XML cooked data in PAK (ZIP) files |
| Database | MySQL (dbmgr_mysql) | PostgreSQL via SOCI |

## BigWorld Server Components (Reference)

BigWorld's full server cluster includes more processes than Cimmeria implements:

| BigWorld Process | Cimmeria Equivalent | Notes |
|-----------------|--------------------|----|
| LoginApp | AuthenticationServer | CME replaced with SOAP/HTTP |
| BaseApp | BaseApp | Core implementation |
| CellApp | CellApp | Core implementation |
| CellAppMgr | -- | Not needed (single CellApp) |
| BaseAppMgr | -- | Not needed (single BaseApp) |
| DBMgr | -- | Database access integrated into services |
| Reviver | -- | Process monitoring not implemented |
| BWMachined | -- | Machine management not implemented |
| MessageLogger | -- | Replaced with file-based logging |

---

## Server Application Directory Index

### `src/server/baseapp/` -- Base Entity Application (55 files)

The BaseApp manages persistent base entities, proxy connections for clients, and inter-service communication with CellApp and BaseAppMgr.

| File | Purpose |
|------|---------|
| `main.cpp` | Entry point, bootstraps BaseApp |
| `baseapp.hpp` / `.ipp` | Core BaseApp class -- event loop, channel management, entity hosting |
| `baseapp_config.hpp` | Configuration values for BaseApp |
| `baseapp_int_interface.hpp` / `.cpp` | **Internal Mercury interface** -- defines all inter-server messages |
| `external_interfaces.cpp` | Registers external (client-facing) interface |
| `base.hpp` | Base entity class -- persistence, cell creation, method dispatch |
| `bases.hpp` | Collection manager for all base entities |
| `proxy.hpp` | Proxy entity -- extends Base with client connection management |
| `entity_type.hpp` | Entity type descriptor for base entities |
| `message_handlers.hpp` | Message handler implementations for incoming Mercury messages |
| `mailbox.hpp` | Mailbox for routing messages to base entities |
| `client_entity_mailbox.hpp` | Mailbox to forward messages to the client's entities |
| `login_handler.hpp` | Handles client login and proxy creation |
| `pending_logins.hpp` | Queue of clients waiting to complete login |
| `entity_creator.hpp` | Creates base entities from DB or from cell requests |
| `entity_channel_finder.hpp` | Locates channels by entity ID |
| `create_cell_entity_handler.hpp` | Handler for creating the cell-side entity |
| `load_entity_handler.hpp` | Handler for loading entity from DB |
| `loading_thread.hpp` | Background thread for DB entity loading |
| `write_to_db_reply.hpp` | Handler for DB write completion |
| `backup_sender.hpp` | Sends entity backups to backup BaseApp |
| `backed_up_base_app.hpp` / `backed_up_base_apps.hpp` | Tracks backed-up state from other BaseApps |
| `offloaded_backups.hpp` | Manages backup offloading |
| `base_message_forwarder.hpp` | Forwards messages between base entities |
| `baseappmgr_gateway.hpp` | Communication with BaseAppMgr |
| `controlled_shutdown_handler.hpp` | Graceful shutdown orchestration |
| `dead_cell_apps.hpp` | Tracks dead CellApps for recovery |
| `data_downloads.hpp` | Streams data resources to clients |
| `download_streamer.hpp` / `download_streamer_config.hpp` | Throttled resource download |
| `global_bases.hpp` | Global named base entity registry |
| `shared_data_manager.hpp` | Shared data distribution across processes |
| `ping_manager.hpp` | Client connection liveness checks |
| `rate_limit_config.hpp` / `rate_limit_message_filter.hpp` | Client message rate limiting |
| `proxy_buffered_message.hpp` | Buffers messages for disconnected proxies |
| `proxy_rate_limit_callback.hpp` | Rate limit callback for proxy messages |
| `id_config.hpp` | Entity ID range configuration |
| `secondary_db_config.hpp` | Secondary database configuration |
| `sqlite_database.hpp` | Local SQLite for secondary DB |
| `bwtracer.hpp` | Debug tracing |
| `worker_thread.hpp` | Background worker thread |
| `script_bigworld.hpp` | Python `BigWorld` module for base scripts |
| `py_bases.hpp` | Python wrapper for bases collection |
| `py_cell_data.hpp` | Python wrapper for cell data access |

### `src/server/cellapp/` -- Cell Spatial Simulation (100+ files)

The CellApp manages spatial simulation: entity movement, AoI (Area of Interest), ghosting between cells, and game tick processing.

| File | Purpose |
|------|---------|
| `main.cpp` | Entry point, bootstraps CellApp |
| `cellapp.hpp` / `.ipp` | Core CellApp class -- event loop, cell management |
| `cellapp_config.hpp` | Configuration values |
| `cellapp_interface.hpp` / `.cpp` | **CellApp Mercury interface** -- all inter-server messages |
| `external_interfaces.cpp` | External interface registration |
| `entity.hpp` / `.ipp` | Cell entity -- position, AoI, properties, script |
| `real_entity.hpp` / `.ipp` | Real (authoritative) entity instance |
| `entity_type.hpp` / `.ipp` | Entity type descriptor for cell entities |
| `entity_cache.hpp` / `.ipp` | Entity lookup cache |
| `entity_extra.hpp` | Extended entity data (controllers, navigation) |
| `entity_navigate.hpp` / `.cpp` | Python-exposed navigation API |
| `entity_message_handler.hpp` | Routes messages to entities by ID prefix |
| `entity_population.hpp` / `.ipp` | Global entity registry |
| `entity_range_list_node.hpp` | Range-list node for AoI queries |
| `entity_vision.hpp` | Vision/AoI subsystem |
| `message_handlers.hpp` | Message handler implementations |
| `cell.hpp` / `.ipp` | A cell region within a space |
| `cells.hpp` | Collection of all cells |
| `cell_info.hpp` | Cell metadata |
| `cell_range_list.hpp` / `.ipp` | Spatial range list for entity queries |
| `cell_chunk.hpp` | Links cells to chunks |
| `cell_app_channel.hpp` / `cell_app_channels.hpp` | Inter-CellApp communication |
| `cell_viewer_connection.hpp` / `cell_viewer_server.hpp` | Debug cell visualization |
| `space.hpp` / `.ipp` | Game space containing cells |
| `space_branch.hpp` / `space_node.hpp` | BSP-like space subdivision |
| `spaces.hpp` | Collection of all spaces |
| `server_geometry_mapping.hpp` / `server_geometry_mappings.hpp` | Geometry file -> space mapping |
| `witness.hpp` / `.ipp` | Witness (player's AoI observer) |
| `mailbox.hpp` | Cell entity mailbox |
| `real_caller.hpp` | Calls methods on real entity |
| `controller.hpp` / `controllers.hpp` | Controller base class (movement, timer, etc.) |
| `history_event.hpp` / `.ipp` | Property change history for ghosting |
| `aoi_update_schemes.hpp` | AoI update bandwidth schemes |
| `loading_column.hpp` / `loading_edge.hpp` | Cell boundary loading |
| `updatable.hpp` / `updatables.hpp` | Per-tick update interface |
| `profile.hpp` / `.ipp` | Cell-specific profiling |
| `noise_config.hpp` | Noise parameters |
| `throttle_config.hpp` / `emergency_throttle.hpp` | Overload protection |
| `buffered_entity_messages.hpp` | Message buffering for entities not yet created |
| `buffered_ghost_message*.hpp` | Ghost message buffering/queuing |
| `cellapp_death_listener.hpp` | Handles other CellApp deaths |
| `cellappmgr_gateway.hpp` | Communication with CellAppMgr |
| `create_entity_near_entity_handler.hpp` | Handler for spatial entity creation |
| **Controllers (movement/AI):** | |
| `move_controller.hpp/cpp` | Moves entity toward a target point |
| `navigation_controller.hpp/cpp` | Pathfinding-based movement |
| `accelerate_to_point_controller.hpp/cpp` | Acceleration-based movement to point |
| `accelerate_to_entity_controller.hpp/cpp` | Acceleration-based movement to entity |
| `accelerate_along_path_controller.hpp/cpp` | Acceleration along a path |
| `turn_controller.hpp/cpp` | Rotation controller |
| `face_entity_controller.hpp/cpp` | Face toward another entity |
| `timer_controller.hpp/cpp` | Timed callback controller |
| `proximity_controller.hpp/cpp` | Trigger when entities enter range |
| `visibility_controller.hpp/cpp` | Controls entity visibility |
| `vision_controller.hpp/cpp` | Line-of-sight vision |
| `scan_vision_controller.hpp/cpp` | Scanning vision cone |
| `portal_config_controller.hpp/cpp` | Chunk portal configuration |
| `passenger_controller.hpp/cpp` | Vehicle passenger |
| `passenger_extra.hpp` / `passengers.hpp` | Passenger management |
| `py_client.hpp` | Python wrapper for client connection |
| `py_entities.hpp` | Python wrapper for entities collection |

### `src/server/loginapp/` -- Login Application (14 files)

The LoginApp authenticates clients and directs them to a BaseApp.

| File | Purpose |
|------|---------|
| `main.cpp` | Entry point |
| `loginapp.hpp` / `.cpp` | Core LoginApp -- accepts login requests, validates credentials |
| `loginapp_config.hpp` / `.cpp` | Configuration (max connections, rate limits) |
| `login_int_interface.hpp` / `.cpp` | Internal Mercury interface (just `controlledShutDown`) |
| `message_handlers.hpp` / `.cpp` | Login and probe message handlers |
| `database_reply_handler.hpp` / `.cpp` | Handles DB query replies for login |
| `nat_config.hpp` / `.cpp` | NAT traversal configuration |
| `status_check_watcher.hpp` / `.cpp` | Health check watcher |

### `src/server/baseappmgr/` -- BaseApp Manager (14 files)

Manages the pool of BaseApps: load balancing, entity creation routing, backup hash management.

| File | Purpose |
|------|---------|
| `main.cpp` | Entry point |
| `baseappmgr.hpp` / `.ipp` / `.cpp` | Core manager -- tracks BaseApps, routes entity creation |
| `baseappmgr_config.hpp` / `.cpp` | Configuration |
| `baseappmgr_interface.hpp` / `.cpp` | Mercury interface (createEntity, add, del, informOfLoad, etc.) |
| `baseapp.hpp` / `.cpp` | Per-BaseApp tracking data (load, entity count) |
| `message_handlers.cpp` | Message handler implementations |
| `reply_handlers.hpp` / `.cpp` | Reply handler implementations |
| `watcher_forwarding_baseapp.hpp` / `.cpp` | Watcher query forwarding to BaseApps |

### `src/server/cellappmgr/` -- CellApp Manager (2 files)

Minimal interface definition for CellApp management.

| File | Purpose |
|------|---------|
| `cellappmgr_interface.hpp` / `.cpp` | Mercury interface (createEntity, addApp, delApp, etc.) |

### `src/server/dbmgr/` -- Database Manager (38 files)

Manages all database operations: entity persistence, login validation, ID allocation.

| File | Purpose |
|------|---------|
| `main.cpp` | Entry point |
| `database.hpp` / `.ipp` / `.cpp` | Core Database manager |
| `dbmgr_config.hpp` / `.cpp` | Configuration |
| `db_interface.hpp` / `.cpp` | Mercury interface (logOn, loadEntity, deleteEntity, writeEntity, etc.) |
| `entity_auto_loader.hpp` / `.cpp` | Auto-loads entities from DB on server start |
| `login_handler.hpp` / `.cpp` | Processes login requests from LoginApp |
| `relogon_attempt_handler.hpp` / `.cpp` | Handles re-login attempts |
| `load_entity_handler.hpp` / `.cpp` | Entity loading from DB |
| `get_entity_handler.hpp` / `.cpp` | Entity retrieval |
| `write_entity_handler.hpp` / `.cpp` | Entity persistence writes |
| `delete_entity_handler.hpp` / `.cpp` | Entity deletion from DB |
| `look_up_dbid_handler.hpp` / `.cpp` | DB ID lookup |
| `look_up_entity_handler.hpp` / `.cpp` | Entity lookup by name |
| `login_app_check_status_reply_handler.hpp` / `.cpp` | Status check response handling |
| `consolidator.hpp` / `.cpp` | Database consolidation |
| `custom.hpp` / `.cpp` | Custom database extensions |
| `custom_billing_system.hpp` / `.cpp` | Custom billing integration |
| `py_billing_system.hpp` / `.cpp` | Python billing system API |
| `message_handlers.cpp` | Message handler implementations |
| `signal_set.hpp` | Signal set for DB operations |

### `src/server/reviver/` -- Process Reviver (8 files)

Monitors server processes and restarts them on failure.

| File | Purpose |
|------|---------|
| `main.cpp` | Entry point |
| `reviver.hpp` / `.cpp` | Core reviver -- pings processes, restarts on timeout |
| `reviver_config.hpp` / `.cpp` | Configuration |
| `reviver_interface.hpp` | Mercury interface for reviver |
| `component_reviver.hpp` / `.cpp` | Per-component monitoring |

### `src/server/tools/` -- Server Tools

| Tool | Purpose |
|------|---------|
| `bots/` | Load testing bot framework (30 files) -- simulates client connections |
| `bwmachined/` | Machine daemon (12 files) -- process management, interface discovery |
| `message_logger/` | Centralized logging (receives forwarded log messages) |
| `clear_auto_load/` | Clears auto-load entity list from DB |
| `consolidate_dbs/` | Consolidates secondary SQLite DBs into primary |
| `snapshot_helper/` | DB snapshot tool |
| `sync_db/` | DB schema synchronization |

### `src/server/web/` -- Web Integration

| Directory | Purpose |
|-----------|---------|
| `php/` | PHP-to-Python bridge for web-based entity control |
| `python/` | Python web integration (blocking DB logon handler, remote methods) |

---

## RTTI Class Name Cross-Reference (sgw.exe vs BigWorld)

Ghidra string analysis of sgw.exe reveals MSVC RTTI type descriptors (`.?AV...@@` patterns) and debug strings that map directly to BigWorld Mercury namespace classes. The following table shows confirmed BigWorld class presence in the SGW binary.

### Mercury Namespace Classes in sgw.exe

| RTTI String in sgw.exe | BW Source Class | SGW Address | Notes |
|------------------------|----------------|-------------|-------|
| `.?AVBundle@Mercury@@` | `Mercury::Bundle` | `0x01e919e8` | Message bundling -- core protocol |
| `.?AVChannel@Mercury@@` | `Mercury::Channel` | `0x01e9188c` | Network channel management |
| `.?AVChannelInternal@Mercury@@` | `Mercury::ChannelInternal` | `0x01e91e10` | Internal channel variant |
| `.?AVBaseNub@Mercury@@` | `Mercury::BaseNub` | `0x01e919ac` | Base network endpoint (renamed from NetworkInterface) |
| `.?AVNub@Mercury@@` | `Mercury::Nub` | `0x01e91b88` | Full network endpoint |
| `.?AVPacket@Mercury@@` | `Mercury::Packet` | `0x01e91dc8` | Raw packet |
| `.?AVPacketFilter@Mercury@@` | `Mercury::PacketFilter` | `0x01e93b2c` | Packet encryption/compression filter |
| `.?AVBundlePrimer@Mercury@@` | `Mercury::BundlePrimer` | `0x01e51f7c` | Pre-populates bundles with headers |
| `.?AVInputMessageHandler@Mercury@@` | `Mercury::InputMessageHandler` | `0x01e51fa0` | Handles incoming messages |
| `.?AVReplyMessageHandler@Mercury@@` | `Mercury::ReplyMessageHandler` | `0x01e51f24` | Handles reply messages |
| `.?AVTimerExpiryHandler@Mercury@@` | `Mercury::TimerExpiryHandler` | `0x01e51f50` | Timer callback handler |
| `.?AVNubException@Mercury@@` | `Mercury::NubException` | `0x01e53394` | Network exception |
| `.?AVProcessMessageHandler@BaseNub@Mercury@@` | `Mercury::BaseNub::ProcessMessageHandler` | `0x01e91900` | Process-level message handler |
| `.?AVQueryInterfaceHandler@BaseNub@Mercury@@` | `Mercury::BaseNub::QueryInterfaceHandler` | `0x01e91934` | Interface discovery handler |
| `.?AUConnection@Nub@Mercury@@` | `Mercury::Nub::Connection` | `0x01e91b60` | Per-address connection state |
| `.?AVReplyHandlerElement@Nub@Mercury@@` | `Mercury::Nub::ReplyHandlerElement` | `0x01e91a84` | Pending reply tracker |

### CME Client Message Classes in sgw.exe

| RTTI String in sgw.exe | Purpose | Address |
|------------------------|---------|---------|
| `.?AVClientMessage@Mercury@@` | Base client message class | `0x01e91e38` |
| `.?AVClientNetMessage@Mercury@@` | Network-layer client message | `0x01e91e8c` |
| `.?AVClientIncomingMessage@Mercury@@` | Incoming message from server | `0x01e91ee0` |
| `.?AVClientOutgoingMessage@Mercury@@` | Outgoing message to server | `0x01e91eb4` |
| `.?AVClientExceptionMessage@Mercury@@` | Network error message | `0x01e91e5c` |
| `.?AVClientChannelRegMessage@Mercury@@` | Channel registration message | `0x01e91f0c` |
| `.?AVClientInactivityDetectMessage@Mercury@@` | Inactivity timeout message | `0x01e91f3c` |
| `.?AVClientResetMessage@Mercury@@` | Channel reset message | `0x01e91f9c` |
| `.?AVClientChannelRequestStatsMessage@Mercury@@` | Channel stats request | `0x01e91f9c` |
| `.?AVClientChannelStatMessage@Mercury@@` | Channel statistics | `0x01e91fd4` |

### CME Game Classes in sgw.exe

| RTTI/Debug String | Source File (from debug paths) | Purpose |
|-------------------|-------------------------------|---------|
| `ServerConnection` (`0x01e533b8`) | `..\..\Server\bigworld\src\common\servconn.cpp` | Client-side server connection |
| `EntityManager` (`0x01df2704`) | `..\..\Server\bigworld\src\client\entity_manager.cpp` | Client entity management |
| `GameEntityManager` (`0x01df26e4`) | `.\Src\GameEntityManager.cpp` | CME game entity manager (extends EntityManager) |
| `ABigWorldEntity` (`0x01dc873c`) | `.\Src\BigWorldEntity.cpp` | Unreal-BigWorld entity bridge (Actor class) |
| `UBigWorldInfo` (`0x01dcd6c4`) | -- | Unreal BigWorld info actor |
| `UGameEntityManagerGCHook` (`0x01df11d4`) | `.\Src\GameEntityManager.cpp` | GC integration for entity manager |
| `SGWSpawnableEntity` | -- | CME spawnable entity base class |

### Key Debug Path Strings

These debug assertion paths confirm SGW.exe source structure relative to BigWorld:

```
..\..\..\..\Server\bigworld\src\client\entity_manager.cpp
..\..\..\..\Server\bigworld\src\common\servconn.cpp
.\Src\BigWorldEntity.cpp
.\Src\GameEntityManager.cpp
c:\BUILD\QA\SGW\Working\Development\Src\SGWGame\Inc\GameEntityFactory.h
```

This confirms CME maintained BigWorld in a `Server\bigworld\` subdirectory relative to their game code, and wrote SGW-specific extensions in `Src\SGWGame\`.

---

## Cimmeria Source BigWorld Header References

Cimmeria does **not** directly include BigWorld header files. Instead, it reimplements the Mercury protocol layer from scratch based on studying the BigWorld source. All `#include` directives in Cimmeria source reference Cimmeria's own headers.

### Cimmeria's Mercury Reimplementation (modeled on BigWorld)

Cimmeria's `src/mercury/` directory contains a clean-room reimplementation of the BigWorld Mercury protocol:

| Cimmeria Header | BigWorld Equivalent | Concept Mapped |
|-----------------|-------------------|----------------|
| `src/mercury/packet.hpp` | `network/packet.hpp` | Packet flags, structure, MAX_SIZE |
| `src/mercury/bundle.hpp` | `network/bundle.hpp` | Message bundling, reliability |
| `src/mercury/channel.hpp` | `network/channel.hpp` | Channel state, ACK tracking |
| `src/mercury/message.hpp` | `network/misc.hpp` + `basictypes.hpp` | MessageID, SeqNum, EntityID, types |
| `src/mercury/stream.hpp` | `cstdmf/binary_stream.hpp` | Binary serialization |
| `src/mercury/memory_stream.hpp` | -- | Memory-backed stream |
| `src/mercury/nub.hpp` | `network/nub.hpp` / `network_interface.hpp` | Network endpoint |
| `src/mercury/encryption_filter.hpp` | `network/encryption_filter.hpp` | Packet encryption (AES, not Blowfish) |
| `src/mercury/transparent_filter.hpp` | -- | No-op filter |
| `src/mercury/condemned_channels.hpp` | `network/condemned_channels.hpp` | Channel teardown |
| `src/mercury/timer.hpp` | (internal) | Timer facility |
| `src/mercury/base_cell_messages.hpp` | (multiple interfaces) | Base-Cell message definitions |
| `src/mercury/message_handler.hpp` | `network/interface_macros.hpp` | Message dispatch |
| `src/mercury/sgw/entity_message_handler.hpp` | (baseapp message_handlers) | SGW entity message routing |
| `src/mercury/tcp_server.hpp` | -- | TCP server (CME addition) |
| `src/mercury/unified_connection.hpp` | -- | Unified connection (CME TCP transport) |
| `src/mercury/unified_protocol.hpp` | -- | Unified protocol (CME addition) |

### Cimmeria Application Headers Referencing Mercury

| Cimmeria Source File | Mercury Headers Used |
|---------------------|---------------------|
| `src/baseapp/base_service.hpp` | `tcp_server.hpp`, `nub.hpp` |
| `src/baseapp/mercury/sgw/messages.hpp` | `message.hpp` |
| `src/baseapp/mercury/sgw/connect_handler.cpp` | `channel.hpp`, `nub.hpp`, `bundle.hpp`, `encryption_filter.hpp` |
| `src/baseapp/mercury/sgw/client_handler.hpp` | `sgw/entity_message_handler.hpp` |
| `src/baseapp/mercury/cell_handler.hpp` | `unified_connection.hpp`, `tcp_server.hpp` |
| `src/cellapp/base_client.hpp` | `unified_connection.hpp`, `sgw/entity_message_handler.hpp` |
| `src/cellapp/base_client.cpp` | `base_cell_messages.hpp`, `transparent_filter.hpp`, `nub.hpp` |
| `src/cellapp/entity/entity_manager.hpp` | `nub.hpp`, `base_cell_messages.hpp` |
| `src/entity/entity.hpp` | `nub.hpp` |
| `src/entity/mailbox.hpp` | `nub.hpp` |
| `src/entity/method.hpp` | `bundle.hpp` |
| `src/entity/python.hpp` | `stream.hpp` |
| `src/entity/py_client.hpp` | `unified_connection.hpp`, `tcp_server.hpp` |
| `src/authentication/shard_client.hpp` | `unified_connection.hpp` |
| `src/authentication/service_main.hpp` | `tcp_server.hpp` |
| `src/common/service.hpp` | `timer.hpp` |

---

## BigWorld Protocol-Affecting #define Macros

### Packet Structure Macros (`network/`)

| Macro | File | Value/Effect |
|-------|------|-------------|
| `PACKET_MAX_SIZE` | `packet.hpp` | `1472` -- Maximum UDP payload (MTU - UDP_OVERHEAD) |
| `FLAG_HAS_REQUESTS` | `packet.hpp` | `0x0001` -- Packet contains request messages |
| `FLAG_HAS_PIGGYBACKS` | `packet.hpp` | `0x0002` -- Packet carries piggybacked sub-packets |
| `FLAG_HAS_ACKS` | `packet.hpp` | `0x0004` -- Packet carries ACK footers |
| `FLAG_ON_CHANNEL` | `packet.hpp` | `0x0008` -- Packet belongs to a channel |
| `FLAG_IS_RELIABLE` | `packet.hpp` | `0x0010` -- Packet requires acknowledgement |
| `FLAG_IS_FRAGMENT` | `packet.hpp` | `0x0020` -- Packet is part of a fragmented bundle |
| `FLAG_HAS_SEQUENCE_NUMBER` | `packet.hpp` | `0x0040` -- Packet has a sequence number footer |
| `FLAG_INDEXED_CHANNEL` | `packet.hpp` | `0x0080` -- Channel identified by index, not address |
| `FLAG_HAS_CHECKSUM` | `packet.hpp` | `0x0100` -- Packet has a CRC32 checksum footer |
| `FLAG_CREATE_CHANNEL` | `packet.hpp` | `0x0200` -- Create anonymous channel on receipt |
| `FLAG_HAS_CUMULATIVE_ACK` | `packet.hpp` | `0x0400` -- Packet carries cumulative ACK |
| `KNOWN_FLAGS` | `packet.hpp` | `0x07FF` -- Mask of all valid flags (11 bits) |

### Interface Definition Macros (`network/common_interface_macros.hpp`)

These macros define how Mercury message handlers are registered:

| Macro | Effect |
|-------|--------|
| `BW_BEGIN_STRUCT_MSG(TYPE, NAME)` | Fixed-size struct message, dispatched to `TYPE::NAME` |
| `BW_BEGIN_STRUCT_MSG_EX(TYPE, NAME)` | Same with extended handler (includes source address) |
| `BW_STREAM_MSG(TYPE, NAME)` | Variable-length stream message (2-byte length prefix) |
| `BW_STREAM_MSG_EX(TYPE, NAME)` | Variable-length with extended handler |
| `BW_BIG_STREAM_MSG(TYPE, NAME)` | Variable-length with 4-byte length prefix |
| `BW_BIG_STREAM_MSG_EX(TYPE, NAME)` | Same with extended handler |
| `BW_ANONYMOUS_CHANNEL_CLIENT_MSG(IF)` | Anonymous channel message handler |
| `DEFINE_INTERFACE_HERE` | Triggers interface table generation on second include pass |
| `DEFINE_SERVER_HERE` | Triggers server-side handler binding on second include pass |

### Client Interface Macros (`connection/client_interface.hpp`)

| Macro | Effect |
|-------|--------|
| `MF_BEGIN_CLIENT_MSG(NAME)` | Fixed-size client message dispatched to `ServerConnection::NAME` |
| `MF_VARLEN_CLIENT_MSG(NAME)` | Variable-length client message (2-byte length) |
| `MF_VARLEN_WITH_ADDR_CLIENT_MSG(NAME)` | Variable-length with sender address |

### BaseApp External Interface Macros (`connection/common_baseapp_interface.hpp`)

| Macro | Effect |
|-------|--------|
| `MF_BEGIN_PROXY_MSG(NAME)` | Fixed-size proxy message |
| `MF_BEGIN_BLOCKABLE_PROXY_MSG(NAME)` | Blockable proxy message (can be rate-limited) |
| `MF_BEGIN_UNBLOCKABLE_PROXY_MSG(NAME)` | Unblockable proxy message (always processed) |
| `MF_VARLEN_PROXY_MSG(NAME)` | Variable-length proxy message |
| `MF_VARLEN_BLOCKABLE_PROXY_MSG(NAME)` | Variable-length blockable proxy message |

### AvatarUpdate Macros (`connection/common_client_interface.hpp`)

These macros generate the 32 avatarUpdate message variants:

| Macro | Effect |
|-------|--------|
| `AVUPMSG(ID, POS, DIR)` | Generates an avatarUpdate variant |
| `AVUPID_NoAlias` / `AVUPID_Alias` | Entity identification: full EntityID or 1-byte alias |
| `AVUPPOS_FullPos` / `OnChunk` / `OnGround` / `NoPos` | Position encoding variants |
| `AVUPDIR_YawPitchRoll` / `YawPitch` / `Yaw` / `NoDir` | Direction encoding variants |

### Entity Data Flags (`entitydef/data_description.hpp`)

| Macro/Constant | Value | Effect |
|---------------|-------|--------|
| `DATA_GHOSTED` | `0x01` | Property replicated to ghost entities |
| `DATA_OTHER_CLIENT` | `0x02` | Property sent to other clients' views |
| `DATA_OWN_CLIENT` | `0x04` | Property sent to owning client |
| `DATA_BASE` | `0x08` | Property sent to base entity |
| `DATA_CLIENT_ONLY` | `0x10` | Static client-side only data |
| `DATA_PERSISTENT` | `0x20` | Property persisted to database |
| `DATA_EDITOR_ONLY` | `0x40` | Only used by editor tool |
| `DATA_ID` | `0x80` | Indexed column in database |
| `DATA_DISTRIBUTION_FLAGS` | (combined) | Mask of distribution-relevant flags |
| `DEFAULT_DATABASE_LENGTH` | `65535` | Default max DB column length |

### Configuration/Build Macros

| Macro | File | Effect |
|-------|------|--------|
| `ENABLE_WATCHERS` | `cstdmf/config.hpp` | Enables watcher monitoring system (always on except consumer builds) |
| `MF_SERVER` | (build config) | Defined for server builds; enables server-only code paths |
| `INTERFACE_NAME` | (per-interface) | Set before including common macros to name the current interface |
| `MESSAGE_LOGGER_VERSION` | `network/logger_message_forwarder.hpp` | `7` -- Message logger protocol version |

---

## BigWorld Version-Specific Protocol Features (2.0.1)

### Version Constants

BigWorld 2.0.1 defines its version in `src/lib/cstdmf/bwversion.hpp`:

```cpp
#define BW_VERSION_MAJOR 2
#define BW_VERSION_MINOR 0
#define BW_VERSION_PATCH 0
```

### Login Protocol Version

The protocol version is tracked in `src/lib/connection/login_interface.hpp` as `LOGIN_VERSION = 56`. The file contains a detailed changelog of all 56 protocol versions:

| Version | Change |
|---------|--------|
| 11 | Pitch and roll sent in createEntity |
| 12 | Added voiceData to client interface |
| 13 | Entity to update sent in avatarUpdate |
| 14 | EntityTypeID changed to uint16 |
| 15 | Implemented spaces, viewports, space data |
| 16 | setGameTime has only game time (renamed from setTime) |
| 17 | Implemented vehicles; split enterAoI into 3 variants |
| 18 | Upstream avatarUpdate supports vehicles; removed requestBandwidth |
| 19 | Cell fault tolerance |
| 20 | Resource versioning and update messages |
| 21 | Added changeProxy to client interface |
| 22 | Base app fault tolerance |
| 23 | Co-ordinated live resource updates |
| 24 | Session key authentication |
| 25 | Player entity data from createPlayer instead of login reply |
| 26 | Separate createBasePlayer and createCellPlayer messages |
| 27 | Explicit pose corrections; control toggle; removed cell IDs |
| 28 | Replaced LogOnReplyStatus with LogOnStatus |
| 29 | Client type indices collapsed; signed/unsigned data MD5s differ |
| 30 | Changes to once-off reliable data handling |
| 31 | Changed packed float y-value format |
| 32 | Added baseAppLogin message to fix NAT issues |
| 33 | Configuration option for ordering client-server channel |
| 34 | Login uses once-off reliability to LoginApp |
| 35 | Added setGameTime message to BaseApp for DB restore |
| 36 | Reverted to 1472 MTU; added disconnectClient and loggedOff |
| 37 | Piggybacking for ordered channels |
| 38 | Xbox 360 (big-endian) support |
| 39 | Login no longer uses once-off reliability |
| 40 | LOGIN_VERSION is now 4 bytes |
| 41 | Piggyback length changed to ones complement |
| 42 | Added support for fully encrypted sessions |
| 43 | Added FLAG_HAS_CHECKSUM; packet headers now 2 bytes |
| 44 | Removed RelPosRef; removed updater and viewport code |
| 45 | All logins RSA encrypted; Blowfish encryption optional |
| 46 | Blowfish encryption mandatory |
| 47 | FLAG_FIRST_PACKET invalid on external nubs/channels |
| 48 | Public keys no longer fetchable from server |
| 49 | Blowfish encryption XOR stage prevents replay attacks |
| 50 | Roll expressed with 2pi radians |
| 51 | Preventing replay attacks with unreliable packets |
| 52 | Added cumulative ACKs |
| 53 | Login replies no longer once-off reliable; client no longer accepts once-off reliable |
| 54 | LogOnParams no longer streams nonce twice |
| 55 | Optimised slice changes to arrays |
| 56 | createEntity can now be compressed |

### Version-Conditional Code

BigWorld 2.0.1 has minimal `#if`-guarded version code. The main version-conditional logic is in the message logger:

```cpp
// network/logger_message_forwarder.hpp
#define MESSAGE_LOGGER_VERSION 7

// network/forwarding_string_handler.cpp
#if MESSAGE_LOGGER_VERSION < 7
    // Legacy 32-bit format handlers
#endif
```

The `LogOnStatus` enum includes `LOGIN_BAD_PROTOCOL_VERSION` for rejecting clients with mismatched protocol versions.

### CME/SGW Version Differences

SGW.exe was built on BigWorld ~1.8-1.9 era (pre-2.0), evidenced by:
- Packet flags are `uint8` (8 bits) in SGW vs `uint16` (11 flags) in BW 2.0.1
- SGW uses Nub class directly (BW 2.0 renamed to NetworkInterface)
- SGW strings reference `servconn.cpp` (older name; BW 2.0 has `server_connection.cpp`)
- SGW added `spaceViewportInfo` message not present in BW 2.0.1
- SGW has CME-specific `Client*Message` RTTI classes not in BW source

---

## BigWorld `src/common/` Directory (Shared Server Code)

The `src/common/` directory contains 15 files shared between client and server builds:

| File | Purpose | Relevance |
|------|---------|-----------|
| `simple_client_entity.hpp` / `.cpp` | Dispatches property and method events to Python entity objects. Key functions: `propertyEvent()` applies a property update from binary stream; `methodEvent()` calls a method from binary stream. | HIGH -- defines how entity updates are decoded |
| `space_data_types.hpp` | Constants and structures for space data system. Defines `SPACE_DATA_TOD_KEY` (0), `SPACE_DATA_MAPPING_KEY_CLIENT_SERVER` (1), `SPACE_DATA_MAPPING_KEY_CLIENT_ONLY` (2), `SpaceData_ToDData` (time-of-day), `SpaceData_MappingData` (geometry transform + path). User keys range 256-32511; cell-only keys start at 16384. | HIGH -- defines space data key semantics |
| `db_interface_utils.hpp` / `.cpp` | Utility functions for raw database command execution via Mercury channel or NetworkInterface. Used by both BaseApp and CellApp to send SQL commands to DBMgr. | MEDIUM |
| `doc_watcher.hpp` / `.cpp` | Documentation-enhanced watcher system. Wraps `addWatcher()` calls with `WatcherDoc::setWatcherDoc()` to associate documentation strings with watcher paths. Defines `BW_INIT_WATCHER_DOC` and overloaded `MF_WATCH`/`MF_WATCH_REF` macros. | LOW |
| `py_chunk.cpp` | Python bindings for chunk system -- finding chunks at coordinates, checking loaded state. | LOW |
| `py_network.hpp` / `.cpp` | Python bindings for network operations -- exposes watcher functions to Python scripts. | LOW |
| `py_physics2.hpp` / `.cpp` | Python bindings for physics2 library -- collision queries. | LOW |
| `py_server.hpp` / `.cpp` | Python bindings for server library (stub -- header only, empty). | NONE |
| `pch.hpp` | Precompiled header for common directory. | NONE |

---

## BigWorld Message Handler Catalog

### BaseAppIntInterface (Internal) -- `src/server/baseapp/baseapp_int_interface.hpp`

Messages between BaseApp and other server processes:

| Message | Type | Handler | Purpose |
|---------|------|---------|---------|
| `createBaseWithCellData` | Stream (EX) | `BaseApp` | Create base entity with cell data |
| `createBaseFromDB` | Stream (EX) | `BaseApp` | Create base entity from database |
| `logOnAttempt` | Stream (EX) | `BaseApp` | Client login attempt notification |
| `addGlobalBase` | Stream | `BaseApp` | Register a global base entity |
| `delGlobalBase` | Stream | `BaseApp` | Deregister a global base entity |
| `handleCellAppMgrBirth` | Struct | `BaseApp` | CellAppMgr started notification |
| `handleBaseAppMgrBirth` | Struct | `BaseApp` | BaseAppMgr started notification |
| `handleCellAppDeath` | Stream | `BaseApp` | CellApp died notification |
| `startup` | Struct | `BaseApp` | Server startup signal (bootstrap, autoload flags) |
| `shutDown` | Struct | `BaseApp` | Shutdown signal |
| `controlledShutDown` | Stream (EX) | `BaseApp` | Graceful shutdown |
| `startOffloading` | Stream | `BaseApp` | Begin entity offloading |
| `setCreateBaseInfo` | Stream | `BaseApp` | Set entity creation parameters |
| `startBaseEntitiesBackup` | Struct | `BaseApp` | Begin backing up entities to another BaseApp |
| `stopBaseEntitiesBackup` | Struct | `BaseApp` | Stop backup |
| `backupBaseEntity` | Big Stream (EX) | `BaseApp` | Individual entity backup data |
| `ackOffloadBackup` | Stream | `BaseApp` | Acknowledge offload backup |
| `stopBaseEntityBackup` | Struct (EX) | `BaseApp` | Stop backing up specific entity |
| `handleBaseAppBirth` | Stream (EX) | `BaseApp` | Another BaseApp started |
| `handleBaseAppDeath` | Stream (EX) | `BaseApp` | Another BaseApp died |
| `setBackupBaseApps` | Stream (EX) | `BaseApp` | Set backup assignment |
| `setSharedData` | Stream | `BaseApp` | Set shared data value |
| `delSharedData` | Stream | `BaseApp` | Delete shared data value |
| `setClient` | Struct | `BaseApp` | Set current client context (EntityID) |
| `currentCell` | Struct (EX) | `Base` | Set cell that owns this base |
| `teleportOther` | Struct (EX) | `Base` | Teleport to another cell |
| `emergencySetCurrentCell` | Stream (EX) | `BaseApp` | Emergency cell reassignment |
| `forwardedBaseMessage` | Stream | `BaseApp` | Forwarded base entity message |
| `sendToClient` | Proxy | -- | Forward data to client |
| `createCellPlayer` | VarLen Proxy | -- | Create player's cell entity |
| `spaceData` | VarLen Proxy | -- | Space data update |
| `enterAoI` | Proxy | -- | Entity entered AoI (id + alias) |
| `enterAoIOnVehicle` | Proxy | -- | Entity entered AoI on vehicle |
| `leaveAoI` | VarLen Proxy | -- | Entity left AoI |
| `createEntity` | VarLen Proxy | -- | Create entity in client |
| `updateEntity` | VarLen Proxy | -- | Update entity properties |
| *(32 avatarUpdate variants)* | Proxy | -- | Position/direction updates |
| `detailedPosition` | Proxy | -- | High-precision position |
| `forcedPosition` | Proxy | -- | Server-corrected position |
| `modWard` | Proxy | -- | Add/remove ward control |
| `callClientMethod` | VarLen Proxy | -- | Call method on client entity |
| `acceptClient` | VarLen Proxy | -- | Accept client connection |
| `backupCellEntity` | Stream | `Base` | Backup cell entity state |
| `writeToDB` | Stream | `Base` | Write entity to database |
| `cellEntityLost` | Stream (EX) | `Base` | Cell entity was lost |
| `startKeepAlive` | Struct (EX) | `Base` | Keep entity alive after client disconnect |
| `callBaseMethod` | Stream (EX) | `Base` | Call base entity method |
| `callCellMethod` | Stream | `Base` | Call cell entity method |
| `getCellAddr` | Stream (EX) | `Base` | Query cell address |
| `entityMessage` | Variable | -- | Generic entity message (IDs 128-254) |
| `callWatcher` | Stream (EX) | `BaseApp` | Watcher query forwarding |

### BaseAppExtInterface (External/Client) -- `src/lib/connection/baseapp_ext_interface.hpp`

Messages from client to BaseApp:

| Message | Type | Purpose |
|---------|------|---------|
| `baseAppLogin` | Stream (EX) | Initial login to BaseApp |
| `authenticate` | Struct (EX) | Session key authentication |
| `avatarUpdateImplicit` | Blockable | Position update (implicit space) |
| `avatarUpdateExplicit` | Blockable | Position update (explicit space+vehicle) |
| `avatarUpdateWardImplicit` | Blockable | Ward position update (implicit) |
| `avatarUpdateWardExplicit` | Blockable | Ward position update (explicit) |
| `ackPhysicsCorrection` | Blockable | Acknowledge server position correction |
| `ackWardPhysicsCorrection` | Blockable | Acknowledge ward position correction |
| `switchInterface` | Fixed (0 bytes) | Switch to cell interface (may be unused) |
| `requestEntityUpdate` | VarLen Blockable | Request entity data updates |
| `enableEntities` | Blockable | Enable entity processing |
| `restoreClientAck` | Unblockable | Acknowledge client restore |
| `disconnectClient` | Unblockable | Client voluntary disconnect |
| `entityMessage` | Variable | Generic entity message (IDs 128-254) |

### ClientInterface (Server to Client) -- `src/lib/connection/client_interface.hpp`

Messages from server to client:

| Message | Type | Purpose |
|---------|------|---------|
| `authenticate` | Struct | Session key verification |
| `bandwidthNotification` | Struct | Bandwidth allocation (bps) |
| `updateFrequencyNotification` | Struct | Update rate (hertz) |
| `setGameTime` | Struct | Synchronize game time |
| `resetEntities` | Struct | Reset all entities (keepPlayerOnBase flag) |
| `createBasePlayer` | VarLen | Create base player entity |
| `createCellPlayer` | VarLen | Create cell player entity |
| `spaceData` | VarLen | Space data (spaceID, entryID, key, value) |
| `createEntity` | VarLen | Create entity in client AoI |
| `updateEntity` | VarLen | Update entity properties |
| `enterAoI` | Struct | Entity entered area of interest |
| `enterAoIOnVehicle` | Struct | Entity entered AoI on a vehicle |
| `leaveAoI` | VarLen | Entity left area of interest |
| *(32 avatarUpdate variants)* | Struct | Position/direction updates |
| `detailedPosition` | Struct | High-precision entity position |
| `forcedPosition` | Struct | Server position correction |
| `controlEntity` | Struct | Toggle entity control |
| `voiceData` | VarLen+Addr | Voice chat data |
| `restoreClient` | VarLen | Restore client after reconnect |
| `switchBaseApp` | VarLen | Redirect to different BaseApp |
| `resourceHeader` | VarLen | Resource download header |
| `resourceFragment` | VarLen | Resource download data fragment |
| `loggedOff` | Struct | Server disconnect notification (reason) |
| `entityMessage` | Variable | Generic entity message (IDs 128-254) |

### CellAppInterface -- `src/server/cellapp/cellapp_interface.hpp`

Messages to the CellApp:

| Message | Type | Handler | Purpose |
|---------|------|---------|---------|
| `addCell` | Stream (EX) | `CellApp` | Add a new cell |
| `startup` | Struct | `CellApp` | Startup with BaseApp address |
| `setGameTime` | Struct | `CellApp` | Set game time |
| `handleCellAppMgrBirth` | Struct | `CellApp` | CellAppMgr birth notification |
| `handleCellAppDeath` | Struct | `CellApp` | CellApp death notification |
| `handleBaseAppDeath` | Stream | `CellApp` | BaseApp death notification |
| `shutDown` | Struct | `CellApp` | Shutdown |
| `controlledShutDown` | Struct | `CellApp` | Graceful shutdown (stage + time) |
| `sendEntityPositions` | Variable | -- | Bulk entity position send |
| `setSharedData` / `delSharedData` | Stream | `CellApp` | Shared data management |
| `setBaseApp` | Struct | `CellApp` | Set BaseApp address |
| `onloadTeleportedEntity` | Stream (EX) | `CellApp` | Receive teleported entity |
| `cellAppMgrInfo` | Struct | `CellApp` | Load info from CellAppMgr |
| `createGhost` | Stream (EX) | `Space` | Create ghost entity |
| `spaceData` | Stream | `Space` | Space data update |
| `allSpaceData` | Stream | `Space` | Full space data dump |
| `updateGeometry` | Stream | `Space` | Geometry update |
| `spaceGeometryLoaded` | Stream | `Space` | Geometry loaded notification |
| `shutDownSpace` | Stream | `Space` | Shut down a space |
| `createEntity` | Stream (EX) | `Cell` | Create entity in cell |
| `createEntityNearEntity` | Variable | `Cell` | Create entity near existing entity |
| `shouldOffload` | Stream | `Cell` | Offload check |
| `retireCell` / `removeCell` | Stream | `Cell` | Cell retirement |
| `notifyOfCellRemoval` | Stream | `Cell` | Cell removal notification |
| `ackCellRemoval` | Stream (EX) | `Cell` | Acknowledge cell removal |
| `avatarUpdateImplicit` | Entity (REAL) | `Entity` | Position update (implicit) |
| `avatarUpdateExplicit` | Entity (REAL) | `Entity` | Position update (explicit) |
| `ackPhysicsCorrection` | Entity (REAL) | `Entity` | Physics correction ACK |
| `enableWitness` | Entity Request (REAL) | `Entity` | Enable witness (AoI observer) |
| `witnessCapacity` | Entity (WITNESS) | `Entity` | Set witness bandwidth |
| `requestEntityUpdate` | Entity (WITNESS) | `Entity` | Request entity update |
| `witnessed` | Entity (REAL) | `Entity` | Notify entity it is witnessed |
| `writeToDBRequest` | Entity Request (REAL) | `Entity` | Write entity to DB |
| `destroyEntity` | Entity (REAL) | `Entity` | Destroy entity |
| `onload` | Entity Raw (GHOST) | `Entity` | Load ghost entity data |
| `ghostAvatarUpdate` | Entity (GHOST) | `Entity` | Ghost position update |
| `ghostHistoryEvent` | Entity (GHOST) | `Entity` | Ghost property change history |
| `ghostSetReal` | Entity (GHOST) | `Entity` | Set real entity owner |
| `ghostSetNextReal` | Entity (GHOST) | `Entity` | Set next real entity address |
| `delGhost` | Entity (GHOST) | `Entity` | Delete ghost entity |
| `ghostVolatileInfo` | Entity (GHOST) | `Entity` | Ghost volatile info update |
| `ghostControllerCreate/Delete/Update` | Entity (GHOST) | `Entity` | Ghost controller management |
| `ghostedDataUpdate` | Entity (GHOST) | `Entity` | Ghost property data update |
| `checkGhostWitnessed` | Entity (GHOST) | `Entity` | Check if ghost is witnessed |
| `aoiUpdateSchemeChange` | Entity (GHOST) | `Entity` | AoI update scheme change |
| `runScriptMethod` | Entity (REAL) | `Entity` | Run cell script method |
| `callBaseMethod` | Entity (REAL) | `Entity` | Call base method via cell |
| `callClientMethod` | Entity (REAL) | `Entity` | Call client method via cell |
| `delControlledBy` | Entity (REAL) | `Entity` | Remove controller |
| `forwardedBaseEntityPacket` | Entity (REAL) | `Entity` | Forwarded base message |
| `onBaseOffloaded` | Entity Raw (REAL) | `Entity` | Base entity was offloaded |
| `teleport` | Entity (REAL) | `Entity` | Teleport entity |
| `runExposedMethod` | Variable | -- | Generic exposed method (IDs 128-254) |
| `callWatcher` | Stream (EX) | `CellApp` | Watcher query forwarding |

### LoginInterface -- `src/lib/connection/login_interface.hpp`

| Message | Type | Purpose |
|---------|------|---------|
| `login` | Variable (2) | Login request (version, encrypted flag, LogOnParams) |
| `probe` | Fixed (0) | Server probe (returns host/owner/users info) |

### LoginIntInterface (Internal) -- `src/server/loginapp/login_int_interface.hpp`

| Message | Type | Purpose |
|---------|------|---------|
| `controlledShutDown` | Empty | Shutdown command |

### DBInterface -- `src/server/dbmgr/db_interface.hpp`

| Message | Type | Handler | Purpose |
|---------|------|---------|---------|
| `handleBaseAppMgrBirth` | Struct | `Database` | BaseAppMgr birth notification |
| `shutDown` | Struct | `Database` | Shutdown |
| `controlledShutDown` | Struct | `Database` | Graceful shutdown |
| `cellAppOverloadStatus` | Struct | `Database` | CellApp load notification |
| `logOn` | Stream (EX) | `Database` | Player login validation |
| `loadEntity` | Stream (EX) | `Database` | Load entity from DB |
| `deleteEntity` | Struct (EX) | `Database` | Delete entity from DB |
| `deleteEntityByName` | Stream (EX) | `Database` | Delete entity by name |
| `lookupEntity` | Struct (EX) | `Database` | Look up entity by DBID |
| `lookupEntityByName` | Stream (EX) | `Database` | Look up entity by name |
| `lookupDBIDByName` | Stream (EX) | `Database` | Look up DBID by entity name |
| `putIDs` | Stream (EX) | `Database` | Return unused entity IDs |
| `getIDs` | Stream (EX) | `Database` | Request new entity IDs |
| `writeGameTime` | Struct | `Database` | Persist game time |
| `handleDatabaseBirth` | Struct | `Database` | Database ready notification |
| `handleBaseAppDeath` | Stream (EX) | `Database` | BaseApp death notification |
| `checkStatus` | Stream (EX) | `Database` | Status check request |
| `secondaryDBRegistration` | Stream (EX) | `Database` | Register secondary DB |
| `updateSecondaryDBs` | Stream (EX) | `Database` | Update secondary DB list |
| `getSecondaryDBDetails` | Stream (EX) | `Database` | Query secondary DB details |

### BaseAppMgrInterface -- `src/server/baseappmgr/baseappmgr_interface.hpp`

| Message | Type | Purpose |
|---------|------|---------|
| `createEntity` | Stream (EX) | Route entity creation to best BaseApp |
| `add` | Struct (EX) | Register a new BaseApp |
| `recoverBaseApp` | Stream (EX) | Recover a BaseApp from backup |
| `del` | Struct (EX) | Deregister a BaseApp |
| `informOfLoad` | Struct (EX) | Report load (load, numBases, numProxies) |
| `shutDown` | Struct | Shutdown |
| `controlledShutDown` | Struct | Graceful shutdown |
| `handleBaseAppDeath` | Struct | BaseApp died |
| `handleCellAppMgrBirth` | Struct | CellAppMgr birth |
| `handleBaseAppMgrBirth` | Struct | BaseAppMgr birth |
| `handleCellAppDeath` | Stream (EX) | CellApp died |
| `registerBaseGlobally` | Stream (EX) | Register global base |
| `deregisterBaseGlobally` | Stream (EX) | Deregister global base |
| `requestHasStarted` | Stream (EX) | Check if server has started |
| `initData` | Stream (EX) | Initialize game time etc. |
| `startup` | Stream (EX) | Server ready to start |
| `checkStatus` | Stream (EX) | Status check |
| `spaceDataRestore` | Stream (EX) | Restore space data |
| `setSharedData` / `delSharedData` | Stream | Shared data management |
| `useNewBackupHash` | Stream (EX) | Update backup hash |
| `informOfArchiveComplete` | Stream (EX) | Archive finished |
| `retireApp` | Stream (EX) | Retire a BaseApp |
| `requestBackupHashChain` | Stream (EX) | Request backup hash chain |

### CellAppMgrInterface -- `src/server/cellappmgr/cellappmgr_interface.hpp`

| Message | Type | Purpose |
|---------|------|---------|
| `createEntity` | Stream (EX) | Create entity in best cell |
| `createEntityInNewSpace` | Stream (EX) | Create entity in new space |
| `prepareForRestoreFromDB` | Stream (EX) | Prepare for DB restore |
| `startup` | Stream (EX) | Server startup |
| `shutDown` | Struct | Shutdown |
| `controlledShutDown` | Struct | Graceful shutdown |
| `shouldOffload` | Struct | Enable/disable offloading |
| `addApp` | Stream (EX) | Register a CellApp |
| `recoverCellApp` | Stream | Recover CellApp |
| `delApp` | Struct | Deregister CellApp |
| `setBaseApp` | Struct | Set BaseApp address |
| `handleCellAppMgrBirth` | Struct | CellAppMgr birth |
| `handleBaseAppMgrBirth` | Struct | BaseAppMgr birth |
| `handleCellAppDeath` | Struct (EX) | CellApp died |
| `handleBaseAppDeath` | Stream | BaseApp died |
| `ackCellAppDeath` | Struct (EX) | Acknowledge CellApp death |
| `gameTimeReading` | Struct (EX) | Game time contribution |
| `updateSpaceData` | Stream (EX) | Update space data |
| `shutDownSpace` | Struct | Shut down a space |
| `ackBaseAppsShutDown` | Struct | Acknowledge BaseApp shutdown |
| `checkStatus` | Stream (EX) | Status check |
| `informOfLoad` | Struct | CellApp load report |
| `updateBounds` | Stream | Cell boundary update |
| `retireApp` | Struct | Retire a CellApp |
| `ackCellAppShutDown` | Struct | Acknowledge CellApp shutdown |
| `setSharedData` / `delSharedData` | Stream | Shared data management |

---

## BigWorld Python API Bindings Index

BigWorld uses a custom Python binding framework built on CPython C API (not Boost.Python). The framework is defined in `src/lib/pyscript/pyobject_plus.hpp` with macros:

- `PY_TYPEOBJECT(TYPE)` -- Define Python type object
- `PY_BEGIN_METHODS(TYPE)` / `PY_END_METHODS()` -- Method table
- `PY_BEGIN_ATTRIBUTES(TYPE)` / `PY_END_ATTRIBUTES()` -- Attribute table
- `PY_METHOD(NAME)` -- Register a method
- `PY_RW_ATTRIBUTE_DECLARE(MEMBER, NAME)` -- Read-write attribute
- `PY_RO_ATTRIBUTE_DECLARE(MEMBER, NAME)` -- Read-only attribute
- `PY_AUTO_MODULE_FUNCTION(RET, NAME, ARGS, MODULE)` -- Module-level function

### Server-Side Python Types

| Python Type | Source File | Purpose |
|-------------|-----------|---------|
| **BaseApp** | | |
| `BigWorld` module | `baseapp/script_bigworld.hpp` | Top-level `BigWorld` module for base scripts |
| `PyBases` | `baseapp/py_bases.hpp` | `BigWorld.entities` collection |
| `PyCellData` | `baseapp/py_cell_data.hpp` | Access to cell entity data from base |
| **CellApp** | | |
| `EntityNavigate` | `cellapp/entity_navigate.cpp` | Entity navigation API (moveToPoint, navigateStep, canNavigateTo, etc.) |
| `PyClient` | `cellapp/py_client.hpp` | Client connection from cell entity |
| `PyEntities` | `cellapp/py_entities.hpp` | Cell entities collection |
| **DBMgr** | | |
| `PyBillingResponse` | `dbmgr/py_billing_system.cpp` | Billing system response object |

### Client-Side Python Types

| Python Type | Source File | Purpose |
|-------------|-----------|---------|
| `PyEntities` | `client/py_entities.cpp` | Client entities collection |
| `PyServer` | `client/py_server.cpp` | Server method caller from client |
| `PyChunkModel` | `client/py_chunk_model.cpp` | Chunk model access |
| `PySceneRenderer` | `client/py_scene_renderer.cpp` | Scene rendering control |

### Common Python Bindings (Shared)

| Python Type | Source File | Purpose |
|-------------|-----------|---------|
| `PyServer` (common) | `common/py_server.cpp` | Base server method caller |
| `PyNetwork` | `common/py_network.cpp` | Network/watcher functions |
| `PyPhysics2` | `common/py_physics2.cpp` | Physics collision queries |
| `PyChunk` | `common/py_chunk.cpp` | Chunk system access |
| `SimpleClientEntity` | `common/simple_client_entity.cpp` | Property/method event dispatch |

### Scripting Infrastructure (`pyscript/`)

| File | Purpose |
|------|---------|
| `script.hpp` | Python interpreter init, module creation, `PY_AUTO_MODULE_FUNCTION` |
| `pyobject_plus.hpp` | Core framework: `PyObjectPlus` base class, all PY_* macros |
| `py_callback.cpp` | Timer-based Python callbacks |
| `py_data_section.cpp` | DataSection access from Python |
| `py_import_paths.cpp` | Python import path management |
| `py_output_writer.cpp` | Redirect Python stdout/stderr to BigWorld logging |
| `py_to_stl.cpp` | Python-to-STL container conversion |
| `py_traceback.cpp` | Python traceback formatting |
| `resource_table.cpp` | Resource table Python type (get, link, unlink, at, sub, etc.) |

### Server Tools Python Types

| Python Type | Source File | Purpose |
|-------------|-----------|---------|
| `ClientApp` | `tools/bots/client_app.cpp` | Bot client application |
| `Entity` | `tools/bots/entity.cpp` | Bot entity |
| `PyBots` | `tools/bots/py_bots.cpp` | Bots collection |
| `PyEntities` | `tools/bots/py_entities.cpp` | Bot entities collection |
| `PyServer` | `tools/bots/py_server.cpp` | Bot server method caller |
| `ServerCaller` | `tools/bots/py_server.cpp` | Individual method caller |
| `PyBWLog` | `tools/message_logger/py_bwlog.cpp` | Log reader API |
| `PyQuery` | `tools/message_logger/py_query.cpp` | Log query (iterable) |
| `PyQueryResult` | `tools/message_logger/py_query_result.cpp` | Log query result |
| `PyUserLog` | `tools/message_logger/py_user_log.cpp` | Per-user log access |

---

## Related Documents

- [BigWorld Architecture](../engine/bigworld-architecture.md) -- Architecture overview
- [Mercury Wire Format](../protocol/mercury-wire-format.md) -- Protocol reimplementation details
- [Entity Property Sync](../protocol/entity-property-sync.md) -- Property distribution flags
- [CME Framework](../engine/cme-framework.md) -- CME modifications to BigWorld
- [Service Architecture](../architecture/service-architecture.md) -- Cimmeria service layout

## TODO

- [x] Index the `src/server/` app directories (baseapp, cellapp, loginapp source) -- Documented all 9 server directories with file listings and purpose descriptions
- [x] Cross-reference BigWorld RTTI class names with Ghidra findings in sgw.exe -- Found 16+ Mercury namespace RTTI classes, 10 CME Client*Message classes, and key game classes (ServerConnection, EntityManager, GameEntityManager, ABigWorldEntity)
- [x] Document which BigWorld header files are included by Cimmeria source -- Cimmeria does not include BW headers directly; mapped 17 Cimmeria mercury/ headers to their BW equivalents and listed all application-level mercury includes
- [x] Catalog BigWorld #define macros that affect protocol behavior -- Documented packet flags (11), interface macros, avatarUpdate generation macros, entity data flags, and configuration macros
- [x] Identify BigWorld version-specific protocol features (2.0.1 vs other versions) -- Documented BW_VERSION 2.0.0, LOGIN_VERSION 56 with full 56-version changelog, MESSAGE_LOGGER_VERSION 7, and CME/SGW version differences
- [x] Document the BigWorld `src/common/` directory (shared server code) -- Indexed all 15 files with purposes and relevance ratings
- [x] List all BigWorld message handlers for comparison with SGW handler addresses -- Cataloged all messages across 7 interfaces: BaseAppInt (40+), BaseAppExt (14), Client (24+), CellApp (50+), Login (2), LoginInt (1), DB (20), BaseAppMgr (23), CellAppMgr (24)
- [x] Index the BigWorld Python API bindings for entity scripting reference -- Indexed 65+ py_*.cpp files, documented binding framework macros, and cataloged all server/client/common/tools Python types
