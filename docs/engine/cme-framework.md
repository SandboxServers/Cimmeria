# CME Framework Layer

> **Last updated**: 2026-03-01
> **RE Status**: Partially documented from Ghidra analysis + codebase references
> **Sources**: Ghidra string analysis (STATUS.md), `docs/how-sgw-works.md`, `data/scripts/`

---

## Overview

Cheyenne Mountain Entertainment (CME) built a custom framework layer on top of BigWorld Technology and Unreal Engine 3. This layer bridges the two engines and adds game-specific systems not present in either middleware platform.

The CME framework is primarily client-side code in `sgw.exe`, but several systems have server-side implications.

## EventSignal Framework

The EventSignal system is CME's central event bus. It connects all subsystems using a publish-subscribe pattern.

### Architecture

```
[EventSignal Bus]
    |
    +-- Event_NetOut_*   (Client -> Server messages)
    +-- Event_NetIn_*    (Server -> Client messages)
    +-- Event_UI_*       (UI events)
    +-- Event_Game_*     (Gameplay events)
    +-- Event_System_*   (System events)
```

### Scale

| Category | Count | Description |
|----------|-------|-------------|
| Total event types | 954 | Unique event signal strings in binary |
| Event_NetOut | 253 | Client-to-server messages |
| Event_NetIn | 167 | Server-to-client messages |
| Other events | ~534 | UI, game, system events |

### Event_NetOut / Event_NetIn

These events bridge the CME EventSignal system to BigWorld's Mercury protocol:

- `Event_NetOut_*` -- When the client UI triggers an action, an EventSignal is fired. A listener serializes the data and sends it as a Mercury message to the server.
- `Event_NetIn_*` -- When the server sends a Mercury message, a handler deserializes it and fires the corresponding EventSignal for the client to process.

See [Event-Net Mapping](../analysis/event-net-mapping.md) for the cross-reference between event names and .def methods.

## CME::PropertyNode

The PropertyNode system provides a hierarchical property tree for the client. It extends BigWorld's flat entity property model with a tree structure used for UI binding.

### Known Usage

- Character stats display
- Inventory item properties
- Mission objective tracking
- Organization roster data

### Server Implications

The server must populate PropertyNode-compatible data structures when sending entity property updates. The exact serialization format is part of the client protocol.

TODO: Reverse engineer the PropertyNode binary format from Ghidra.

## CME::SoapLibrary

CME replaced BigWorld's LoginApp with a SOAP/HTTP authentication system. The `SoapLibrary` class handles:

- XML parsing for SOAP requests/responses
- HTTP client/server implementation
- Session token management
- Shard list serialization

The server-side equivalent is the `AuthenticationServer` (`src/authentication/`), which serves SOAP endpoints for `/SGWLogin/UserAuth` and `/SGWLogin/ServerSelection`.

## SpaceViewport System

A CME extension to BigWorld's space system. BigWorld normally sends space data via `spaceData` messages. CME added a `spaceViewportInfo` message that must be sent between `createBasePlayer` and `createCellPlayer` during login.

### Purpose

The SpaceViewport tells the client:
- Which map/zone to load
- Initial viewport parameters
- Zone-specific rendering settings

This was likely added to support Stargate zone transitions, where players move between distinct worlds (planets, space stations, etc.) that each have different environmental settings.

### Server Implementation

The BaseApp sends `spaceViewportInfo` during the player creation sequence. See [Login Handshake](../protocol/login-handshake.md) Step 4.

## CookedData System

CME built a data pipeline that converts database content into XML "cooked" data files served to the client. See [Cooked Data Pipeline](cooked-data-pipeline.md) for details.

Key classes (from Ghidra):
- `CookedElementBase` -- Base class for cooked data elements
- `CookedDataCache` -- Client-side cache for received data
- `CookedDataManager` -- Manages loading and versioning

## Atrea Script Editor

CME built a visual scripting tool called the "Atrea Script Editor" for creating mission and effect scripts. This is part of the ServerEd Qt tool.

### Pipeline

```
.script files (data/scripts/)    -- Visual node graphs (XML source)
        |
        | compiled by scriptcompiler.cpp
        v
.py files (python/cell/)         -- Auto-generated Python
```

The `.script` files define node-graph logic with:
- Trigger nodes (mission start, effect applied, etc.)
- Action nodes (give item, deal damage, play animation)
- Condition nodes (has item, is level, etc.)
- Flow control (branches, loops, delays)

### Script Categories

| Directory | Count | Purpose |
|-----------|-------|---------|
| `data/scripts/effects/` | ~500+ | Combat effect scripts |
| `data/scripts/missions/` | ~100+ | Mission logic scripts |
| `data/scripts/space/` | ~10 | Space/zone event scripts |

## Dual UI System

SGW ships with two UI rendering systems, likely reflecting a mid-development transition:

| System | Technology | Classes | Usage |
|--------|-----------|---------|-------|
| CEGUI | Open-source C++ UI + Lua | 438 classes | Original UI framework |
| Scaleform/GFx | Flash/ActionScript | 271 classes | Newer animated UI |

### Server Implications

The server doesn't need to know which UI system the client uses. Both systems receive the same EventSignal events and BigWorld entity data.

## CME Network Extensions

CME modified BigWorld's networking in several ways:

| Extension | Standard BigWorld | CME/SGW |
|-----------|------------------|---------|
| Encryption | Blowfish | AES-256-CBC + HMAC-MD5 |
| Authentication | Mercury LoginApp | HTTP/SOAP |
| Session key | 4-byte SessionKey | 64-char hex (256-bit) |
| SpaceViewport | Not present | Custom message for zone loading |
| Minigame server | Not present | Separate TCP server for minigames |

## Known CME String Prefixes in Binary

From Ghidra analysis, the following CME-specific string prefixes have been identified:

| Prefix | Count | Description |
|--------|-------|-------------|
| `CME::` | ~50+ | CME namespace classes |
| `Atrea` | ~20+ | Atrea-branded components |
| `SGW` | ~100+ | Game-specific classes |
| `Event_Net` | 420 | Network event signals |
| `Event_UI_` | ~200+ | UI event signals |

## Related Documents

- [How SGW Works](../how-sgw-works.md) -- Technology stack overview
- [Cooked Data Pipeline](cooked-data-pipeline.md) -- Data baking system
- [BigWorld Architecture](bigworld-architecture.md) -- Base engine architecture
- [Event-Net Mapping](../analysis/event-net-mapping.md) -- Event to .def method mapping

## TODO

- [ ] Reverse engineer CME::PropertyNode serialization format
- [ ] Catalog all CME:: class names from RTTI in Ghidra
- [ ] Document the Atrea Script Editor node types and connections
- [ ] Identify all SpaceViewport parameters from Ghidra
- [ ] Document the minigame server protocol (TCP-based)
- [ ] Map the ~534 non-network EventSignal types
- [ ] Determine if CME modified BigWorld's entity serialization format
