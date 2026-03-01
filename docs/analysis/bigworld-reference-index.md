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

## Related Documents

- [BigWorld Architecture](../engine/bigworld-architecture.md) -- Architecture overview
- [Mercury Wire Format](../protocol/mercury-wire-format.md) -- Protocol reimplementation details
- [Entity Property Sync](../protocol/entity-property-sync.md) -- Property distribution flags
- [CME Framework](../engine/cme-framework.md) -- CME modifications to BigWorld
- [Service Architecture](../architecture/service-architecture.md) -- Cimmeria service layout

## TODO

- [ ] Index the `src/server/` app directories (baseapp, cellapp, loginapp source)
- [ ] Cross-reference BigWorld RTTI class names with Ghidra findings in sgw.exe
- [ ] Document which BigWorld header files are included by Cimmeria source
- [ ] Catalog BigWorld #define macros that affect protocol behavior
- [ ] Identify BigWorld version-specific protocol features (2.0.1 vs other versions)
- [ ] Document the BigWorld `src/common/` directory (shared server code)
- [ ] List all BigWorld message handlers for comparison with SGW handler addresses
- [ ] Index the BigWorld Python API bindings for entity scripting reference
