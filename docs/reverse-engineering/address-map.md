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
| TODO | `g_EntityManager` | `EntityManager*` | Singleton — access point for all entities |
| TODO | `g_ConnectionModel` | `ServerConnection*` | Main server connection object |
| TODO | `g_App` | `App*` | Application singleton |
| TODO | `g_ScriptManager` | `ScriptManager*` | Python script engine |

## Key Vtables

Vtables identified via RTTI (script 01) or manual analysis.

| Address | Class | vfunc Count | Notes |
|---------|-------|-------------|-------|
| TODO | `EntityManager` | — | Entity lifecycle management |
| TODO | `ServerConnection` | — | Mercury connection to server |
| TODO | `Entity` | — | Base entity class (BigWorld) |
| TODO | `CME::EventSignal` | — | CME event dispatch |

## Event Handler Functions

Key Event_NetOut/NetIn handler addresses (script 04 output).

### NetOut Handlers (Client → Server)

| Address | Event Name | .def Method | Decompiled? |
|---------|------------|-------------|-------------|
| TODO | `Event_NetOut_UseAbility` | SGWCombatant.useAbility | NO |
| TODO | `Event_NetOut_MoveItem` | SGWInventoryManager.moveItem | NO |
| TODO | `Event_NetOut_MissionAssign` | Missionary.missionAssign | NO |
| TODO | `Event_NetOut_onDialGate` | GateTravel.onDialGate | NO |
| TODO | `Event_NetOut_CreateCharacter` | Account.createCharacter | NO |

### NetIn Handlers (Server → Client)

| Address | Event Name | .def Method | Decompiled? |
|---------|------------|-------------|-------------|
| TODO | `Event_NetIn_onEffectResults` | SGWCombatant.onEffectResults | NO |
| TODO | `Event_NetIn_onContainerInfo` | SGWInventoryManager.onContainerInfo | NO |
| TODO | `Event_NetIn_onMissionUpdate` | Missionary.onMissionUpdate | NO |
| TODO | `Event_NetIn_onStatUpdate` | SGWCombatant.onStatUpdate | NO |
| TODO | `Event_NetIn_CharacterList` | Account.characterList | NO |

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
| TODO | `Entity::setProperty` | Property update dispatch |
| TODO | `Entity::readCellData` | Deserialize cell entity data |
| TODO | `Entity::readBaseData` | Deserialize base entity data |
| TODO | `Entity::handlePropertyUpdate` | Process property change from server |

## UE3 Integration Points

| Address | Function | Notes |
|---------|----------|-------|
| TODO | `UGameEngine::Init` | Engine initialization |
| TODO | `AWorldInfo::execGetTimeOfDay` | Time of day native |
| TODO | Various `exec*` stubs | UE3 UnrealScript native functions |

## String Table Locations

| Address Range | Content | Count |
|---------------|---------|-------|
| TODO | Event_NetOut_* strings | 253 |
| TODO | Event_NetIn_* strings | 167 |
| TODO | RTTI type descriptors | ~2000+ |
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
