# Address Map — Key Locations in SGW.exe

> **Last updated**: 2026-03-01
> **Binary**: SGW.exe (32-bit x86 PE, MSVC)
> **Image base**: TODO — record after loading in Ghidra

---

## Overview

Key virtual addresses, vtables, global variables, and important functions discovered during reverse engineering. All addresses are virtual addresses as loaded by Ghidra.

## Important Globals

| Address | Name | Type | Notes |
|---------|------|------|-------|
| `0x01ef244c` | `g_EntityManager` | `EntityManager*` | Singleton — set in `BW_client_entity_manager` constructor |
| TODO | `g_ConnectionModel` | `ServerConnection*` | Main server connection object |
| TODO | `g_App` | `App*` | Application singleton |
| TODO | `g_ScriptManager` | `ScriptManager*` | Python script engine |

## Key Vtables

Vtables identified via RTTI (script 01) or manual analysis.

| Address | Class | vfunc Count | Notes |
|---------|-------|-------------|-------|
| See `BW_client_entity_manager` | `EntityManager` | — | Dual vtable: ServerMessageHandler + FCallbackEventDevice |
| TODO | `ServerConnection` | — | Mercury connection to server |
| TODO | `Entity` | — | Base entity class (BigWorld) |
| TODO | `CME::EventSignal` | — | CME event dispatch (client-side UI bus only) |

## Core Architecture Functions — Phase 2 Decompiled

### Universal RPC Dispatcher

| Address | Function | Notes |
|---------|----------|-------|
| `0x00c6fc40` | Universal RPC dispatcher | ALL outgoing entity method calls route here |
| `0x00dd6a60` | `ServerConnection_startEntityMessage` | Writes cell method header: `methodID \| 0x80` |
| `0x00dd6980` | `ServerConnection_startProxyMessage` | Writes base method header: `methodID \| 0xC0` |

### Entity Creation

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dddca0` | `ServerConnection_createBasePlayer` | 4B entityID + 2B typeID + property stream |
| `0x00dda2e0` | `ServerConnection_createCellPlayer` | 4B skip + 4B spaceID + 12B Vec3 pos + property stream |

### Entity Manager (entity_manager.cpp)

| Address | Function | Notes |
|---------|----------|-------|
| `0x00dd3330` | `BW_client_entity_manager` | Constructor — initializes EntityManager singleton |
| `0x00dd0d00` | `BW_client_entity_manager_1` area | Entity method dispatch |
| `0x00dd2900` | `BW_client_entity_manager_5` | Entity leave AoI — decrements refcount, cleanup |
| `0x00dd27f0` | `BW_client_entity_manager_4` area | Entity enter AoI — increments refcount, enterWorld |
| `0x00dd1b10` | `BW_client_entity_manager_6` | Entity position/movement update |

### EntityDescription Parsing

| Address | Function | Notes |
|---------|----------|-------|
| `0x01593cd0` | `EntityDescription_parse` | Opens .def file, handles Parent recursion |
| `0x01593600` | `EntityDescription__unknown_01593600` | Parse dispatch: Implements→Properties→Methods |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment (sequential, excludes EDITOR_ONLY) |
| `0x01594f60` | `MethodDescription_parse` | Method signature parsing (Args, ArgNames, Exposed) |
| `0x015974a0` | `DataDescription_parse_2` | Property type + flags + default value parsing |
| `0x015959c0` | `DataDescription_parse_1` | Property flag string → bitmask conversion |
| `0x01593420` | FUN_01593420 | ClientMethods parsing (called from parse dispatch) |
| `0x015934c0` | FUN_015934c0 | CellMethods parsing (called from parse dispatch) |
| `0x01593560` | FUN_01593560 | BaseMethods parsing (called from parse dispatch) |

### Property Change

| Address | Function | Notes |
|---------|----------|-------|
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Writes property change to stream (4B header + values) |

### Event Signal Registration (CME — client-side only)

| Address | Event Name | Notes |
|---------|------------|-------|
| `0x00cb7d90` | `register_NetOut_UseAbility` | Returns string "Event_NetOut_UseAbility" |
| `0x00d771e0` | `register_NetIn_onEffectResults` | Returns string "Event_NetIn_onEffectResults" |
| `0x00d7f520` | `register_NetIn_TimerUpdate` | Returns string "Event_NetIn_TimerUpdate" |
| `0x00d86620` | `register_NetIn_onStatUpdate` | Returns string "Event_NetIn_onStatUpdate" |
| `0x00d86c10` | `register_NetIn_onStatBaseUpdate` | Returns string "Event_NetIn_onStatBaseUpdate" |
| `0x00d7d300` | `register_NetIn_onContainerInfo` | Returns string "Event_NetIn_onContainerInfo" |

**Note**: Event registration functions simply return a name string. They do NOT contain serialization logic. The actual network serialization is handled by the universal RPC dispatcher at `0x00c6fc40`. See `docs/reverse-engineering/findings/combat-wire-formats.md` for details.

## Mercury Protocol Functions

| Address | Function | Notes |
|---------|----------|-------|
| TODO | `Mercury::Channel::send` | Outgoing message dispatch |
| TODO | `Mercury::Channel::processMessage` | Incoming message processing |
| TODO | `Mercury::Bundle::startMessage` | Begin constructing a message |
| TODO | `Mercury::Bundle::addBlob` | Add raw data to message |

## Entity Property Functions

| Address | Function | Notes |
|---------|----------|-------|
| `0x015652d0` | `FNetworkPropertyChange__vfunc_0` | Property change serialization |
| `0x015924a0` | `EntityDescription_parseProperties` | Property ID assignment |
| `0x015974a0` | `DataDescription_parse_2` | Property flag/type parsing |
| TODO | `Entity::readCellData` | Deserialize cell entity data |
| TODO | `Entity::readBaseData` | Deserialize base entity data |

## UE3 Integration Points

| Address | Function | Notes |
|---------|----------|-------|
| TODO | `UGameEngine::Init` | Engine initialization |
| TODO | `AWorldInfo::execGetTimeOfDay` | Time of day native |
| TODO | Various `exec*` stubs | UE3 UnrealScript native functions |

## String Table Locations

| Address Range | Content | Count |
|---------------|---------|-------|
| TODO | Event_NetOut_* strings | 479 |
| TODO | Event_NetIn_* strings | 496 |
| TODO | RTTI type descriptors | ~9,700 |
| TODO | BigWorld source paths | ~200+ |
| TODO | CME:: debug strings | ~100+ |

---

## How to Update

When you identify a key address in Ghidra:
1. Replace the relevant TODO with the actual address (e.g., `0x14001a2b0`)
2. Update the "Decompiled?" column if you've analyzed the function
3. Add new rows for important discoveries
4. Cross-reference with findings in `docs/gameplay/` and `docs/protocol/` docs

## Naming Conventions

- Ghidra function names: `ClassName_methodName` (underscore separator)
- Vtable entries: `ClassName__vfunc_N` (double underscore + index)
- Inferred names: `ClassName__unknown_HEXADDR` (script 10)
- Event handlers: `EventHandler_NetOut_EventName` or `EventHandler_NetIn_EventName`
