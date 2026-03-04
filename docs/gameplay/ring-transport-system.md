# Ring Transport System

Complete analysis of the Ring Transport / Asgard Teleporter system as implemented in the original Stargate Worlds game. Covers server logic, client behavior, database schema, entity definitions, visual effects, and mission integration.

## Overview

The Ring Transport system teleports players between predefined platform locations within a world (and across worlds). It supports multi-player transport — all players standing on the ring platform are transported together.

The visual presentation varies by map theme:
- **Goa'uld/Ancient maps** (Castle CellBlock, Harset, Lucia, Menfa Dark) use `GLB-RingTransporterBase` prefab — traditional ring platform animation
- **Asgard maps** (Omega Site) use `ASG-TeleporterBase00` / `ASG-TeleportBase00` prefab — beam-up/beam-down effect

The server code is identical for both — only the Kismet matinee differs.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│ CLIENT                                                              │
│                                                                     │
│  Player interacts with beam console (entity tag)                    │
│         │                                                           │
│         ▼                                                           │
│  Events.RingTransporterList fires                                   │
│         │                                                           │
│         ▼                                                           │
│  RingTransporterMapMode opens world map with destination icons      │
│         │                                                           │
│         ▼                                                           │
│  Player clicks destination → setRingTransporterDestination(src,dst) │
│         │                                                           │
│         ▼                                                           │
│  Kismet matinee plays (ring animation / beam effect)                │
│  Player hidden → teleported → new map loads → player revealed       │
└─────────────────────────────────────────────────────────────────────┘
         │                          ▲
         │ RPC calls                │ RPC calls
         ▼                          │
┌─────────────────────────────────────────────────────────────────────┐
│ SERVER                                                              │
│                                                                     │
│  Space script → RingTransporter.interact(player)                    │
│         │                                                           │
│         ▼                                                           │
│  Send onRingTransporterList(source, destinations) to client         │
│         │                                                           │
│         ▼                                                           │
│  Receive setRingTransporterDestination → validate → state machine   │
│         │                                                           │
│         ▼                                                           │
│  8-state FSM: lock → hide → teleport → load → reveal → unlock      │
│         │                                                           │
│         ▼                                                           │
│  Fire teleport::in / teleport::out events (for mission scripts)     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Server-Side Implementation

### State Machine (`python/cell/RingTransporter.py`)

The `RingTransporter` class implements an 8-state finite state machine:

| State | Value | Description |
|---|---|---|
| `STATE_IDLE` | 0 | No active transport |
| `STATE_SEND_WAIT` | 1 | Destination selected, waiting for players in region |
| `STATE_SEND_WARMUP` | 2 | Playing teleport-out sequence, timers running |
| `STATE_REMOTE_LOAD_WAIT` | 3 | Players teleported, waiting for map loads |
| `STATE_REMOTE_WARMUP` | 4 | Playing teleport-in sequence at destination |
| `STATE_COOLDOWN` | 5 | Making players visible, unlocking movement |
| `STATE_RECV_WAIT` | 6 | Destination ring: waiting for send ring to start |
| `STATE_RECV_WARMUP` | 7 | Destination ring: send started, playing its sequence |

#### State Transitions

```
SENDING RING:
  IDLE ──selectDestination()──▶ SEND_WAIT ──players present──▶ SEND_WARMUP
                                                                    │
    ┌──────────────────────────────────────────────────────────────┘
    │  +3.5s: hide players (setVisible=false)
    │  +4.0s: teleport players (teleportTo)
    ▼
  REMOTE_LOAD_WAIT ──all loaded──▶ REMOTE_WARMUP ──+3.0s──▶ COOLDOWN ──+2.5s──▶ IDLE

RECEIVING RING:
  IDLE ──remoteWait()──▶ RECV_WAIT ──remoteSend()──▶ RECV_WARMUP ──remoteTransport()──▶ REMOTE_LOAD_WAIT
                                                                                            │
                                          (then same as sending ring from REMOTE_LOAD_WAIT) │
```

#### Timing

| Phase | Delay | What Happens |
|---|---|---|
| Send warmup | +3.5s | All sending players become invisible |
| Send complete | +4.0s | `teleportTo()` — actual position change / map load |
| *(map load)* | variable | Client loads new world if cross-world transport |
| Arrival warmup | +3.0s | All players become visible at destination |
| Cooldown | +2.5s | Movement unlocked, `teleport::in` event fires |

**Total minimum time:** ~9.5s + map load time

#### Key Methods

**`interact(player)`** — Called when player activates a ring console. Sets `player.ringSourceId`, builds `RegionInfo` dicts for source and all valid destinations, sends `player.client.onRingTransporterList(source, destinations)`.

**`selectDestination(player, regionId)`** — Validates destination is in `destinationIds`. Resolves the remote ring: same-world via `space.transporters.get()`, cross-world via `rgnInfo.instance` (direct Python object reference). Transitions to `SEND_WAIT`, notifies remote ring via `remoteWait()`.

**`__beginTransport()`** — Snapshots all players in region, plays `Region_Teleport_Out` Kismet sequence (event 8000), calls `player.onTeleportOut()` and locks movement via `BSF_MovementLock` state flag.

**`__doTransport()`** — Sets `player.destinationRingId` and calls `player.teleportTo(destPos, worldName=destWorldName)` for each player.

**`playerLoaded(player)`** — Called from `SGWPlayer.onClientReady()` when a teleported player finishes loading. When all expected players have loaded, triggers `__allPlayersLoaded()` which plays `Region_Teleport_In` (event 8001).

### RingTransporterManager

Each `Space` owns a `RingTransporterManager`:
```python
# python/cell/Space.py
self.transporters = RingTransporterManager()
self.transporters.load(self, worldId)
```

The manager loads all `ring_transport_regions` rows for the world and creates `RingTransporter` instances. Access by region ID via `space.transporters.get(id)`.

### Player Methods (`python/cell/SGWPlayer.py`)

**Properties:**
- `ringSourceId` — Set when interacting with a ring console, cleared on destination selection
- `destinationRingId` — Set to region ID during transport, cleared on arrival (persisted in player backups)

**RPCs:**
- `setRingTransporterDestination(regionId, destinationId)` — Exposed BaseMethod (client→server). Validates source matches `ringSourceId`, delegates to `region.selectDestination()`.
- `onRingTransporterList(regionInfo, regionList)` — ClientMethod (server→client). Sends destination choices.

**Teleport:**
- `teleportTo(position, yaw, worldName)` — Generic teleport. Sends `onClientMapLoad` for loading screen, calls `moveTo()`.
- `onClientReady()` — After map load, checks `destinationRingId` and notifies the destination ring via `transporter.playerLoaded(self)`.
- `onTeleportIn(regionId)` — Fires `teleport::in` event (consumed by mission scripts).
- `onTeleportOut(regionId, destinationId)` — Fires `teleport::out` event.

### Cross-World Transport

The Omega Site ↔ Omega Site Command Center link (regions 14 ↔ 17) demonstrates cross-world transport. Resolution works via direct Python object references:

```python
rgnInfo = DefMgr.require('ring_transporter_region', regionId)
if rgnInfo.world.world == self.worldName:
    self.remoteRegion = player.space.transporters.get(regionId)  # same world
else:
    self.remoteRegion = rgnInfo.instance  # cross-world: direct object reference
```

`rgnInfo.instance` is set during `RingTransporter.__init__()`. This requires both spaces to be loaded in the same server process.

---

## Client-Side Implementation

### Event Flow (from Ghidra)

The client uses a 4-event chain through the CME (Cheyenne Mountain Entertainment) event system:

| Event Class | Direction | Purpose |
|---|---|---|
| `Event_NetIn_onRingTransporterList` | Network → Game | Deserializes server RPC |
| `Event_UI_onRingTransporterList` | Game → UI | Forwarded to `SGWScriptedWindow` |
| `Event_SlashCmd_SetRingTransporterDestination` | UI → Network | From Lua or `/gmsetringdestination` |
| `Event_NetOut_SetRingTransporterDestination` | Network → Server | Serialized to wire |

**Key binary addresses:**

| Function | Address | Purpose |
|---|---|---|
| `register_NetIn_onRingTransporterList` | `0x00d89020` | Registers incoming event |
| `register_NetOut_SetRingTransporterDestination` | `0x00ae9dd0` | Registers outgoing event |
| Lua binding `setRingTransporterDestination` | `0x00aa2180` | 3-arg Lua function |
| Outbound event builder | `0x00aeab70` | Builds `Event_NetOut` with regionId + destId |
| Slash command handler | `0x00c8a830` | In `SGWTextCommandManager.cpp` line 3125 |
| `SGWNetworkManager` handler | `0x00d68c30` | Serializes to wire |
| `SGWScriptedWindow` UI handler | `0x00ce8140` | Opens destination picker |

### Destination Picker UI (`RingTransporterMapMode.lua`)

**File:** `SGWGame/Content/UI/Core/WorldMap/RingTransporterMapMode.lua`

The ring transport UI reuses the world map in a special mode:

```lua
RingTransporterWorldMapMode = {}
RingTransporterWorldMapMode.bAllowWorldChange = false
```

1. Subscribes to `Events.RingTransporterList`
2. When `onRingTransporterList(window, sourceRegion, destinationList)` fires, stores source and destinations
3. Opens world map with destination icons rendered at map coordinates
4. Each destination shows a `set:MinimapIcons image:Transporter` icon (17x18 pixels) with name label in `Verdana-10`
5. On click, calls `setRingTransporterDestination(sourceRegionId, destRegionId)` and closes the map

### Kismet Sequences (UE3 Visual Effects)

`USeqEvent_RegionTeleport` at `0x0069fc40` handles two event types:

| Event ID | Enum | Trigger |
|---|---|---|
| 8000 | `Region_Teleport_Out` | Ring activation / beam-up animation |
| 8001 | `Region_Teleport_In` | Ring arrival / beam-down animation |

The filtering logic (at `0x006a09e0`):
```cpp
if (this->sequenceEventType == this->teleportDirection + 8000)
    USequenceEvent::fire();
```

Sequences are per-ring-platform matinees baked into the client level packages. Only played for the first player to avoid animation conflicts (noted as FIXME in server code).

### Slash Command

`/gmsetringdestination [regionId] [destinationId]` — Programmer-only debug command (`Access="p"`) defined in `InternalSlashCommands.xml`.

### World Map POI Data

Omega Site (world 18) has 5 transporter POIs defined in `WorldMapPOIData.lua` using `set:MinimapIcons image:Transporter` at coordinates (7,12), (7,-15), (-5,-40), (-42,-18), (39,-24).

---

## Entity Definitions

### SGWPlayer.def

**Properties:**
```xml
<currentRingTransporterDestination>   PYTHON   CELL_PRIVATE   Default: None  </currentRingTransporterDestination>
<pendingRingTeleport>                 INT8     CELL_PRIVATE   Default: 0     </pendingRingTeleport>
```

> **Note:** These entity def properties are defined but the Python implementation uses Python-level instance attributes (`ringSourceId`, `destinationRingId`) instead. Likely vestigial from an earlier C++ implementation.

**BaseMethods (client→server):**
```xml
<setRingTransporterDestination>  <!-- Exposed -->
    <Arg> INT32  aRegionId      </Arg>
    <Arg> INT32  aDestinationId </Arg>
</setRingTransporterDestination>

<onSquadMemberRingTransport>  <!-- NOT exposed, server-only -->
    <Arg> INT32 </Arg>  <!-- source region -->
    <Arg> INT32 </Arg>  <!-- destination region -->
</onSquadMemberRingTransport>

<onSquadMemberRingTransportFinished>  <!-- NOT exposed, server-only -->
    <Arg> INT32 </Arg>  <!-- destination region -->
</onSquadMemberRingTransportFinished>
```

**ClientMethods (server→client):**
```xml
<onRingTransporterList>
    <Arg> RegionInfo                   aRegion     </Arg>
    <Arg> ARRAY <of> RegionInfo </of>  aRegionList </Arg>
</onRingTransporterList>
```

### RegionInfo Type (`entities/defs/alias.xml`)

```xml
<RegionInfo>FIXED_DICT
    <Properties>
        <RegionId>    <Type> INT32   </Type> </RegionId>
        <DisplayName> <Type> INT32   </Type> </DisplayName>
        <WorldId>     <Type> INT32   </Type> </WorldId>
        <Position>    <Type> VECTOR3 </Type> </Position>
    </Properties>
</RegionInfo>
```

Wire format: `RegionId(i32) + DisplayName(i32) + WorldId(i32) + Position(3x f32)` = 24 bytes per region.

### Relevant Enumerations (`enumerations.xml`)

| Enum | Name | Value | Purpose |
|---|---|---|---|
| `EInteractionType` | `RingTransporter` | 7 | Interaction type for ring consoles |
| `EInteractionNotificationType` | `INT_RingNetwork` | 32 | Bitmask flag for ring interactables |
| `ESequenceEventType` | `Region_Teleport_Out` | 8000 | Kismet event: send animation |
| `ESequenceEventType` | `Region_Teleport_In` | 8001 | Kismet event: receive animation |
| `ERegionEvents` | `REGION_EVENT_TeleportOut` | 3 | Region event type |
| `ERegionEvents` | `REGION_EVENT_TeleportIn` | 4 | Region event type |
| `EDUEL_DEFEAT` | `EDUEL_DEFEAT_Teleport` | 5 | Duels auto-forfeit on teleport |

---

## Database Schema

### Table: `resources.ring_transport_regions`

```sql
CREATE TABLE ring_transport_regions (
    region_id               integer NOT NULL,           -- PK, auto-increment
    world_id                integer NOT NULL,           -- FK → worlds
    x                       real NOT NULL,              -- World position X
    y                       real NOT NULL,              -- World position Y
    z                       real NOT NULL,              -- World position Z
    tag                     character varying(100),     -- Human-readable name
    height                  real NOT NULL,              -- Trigger volume height
    radius                  real NOT NULL,              -- Trigger volume radius
    event_set_id            integer NOT NULL,           -- FK → event_sets (Kismet sequences)
    display_name_id         integer NOT NULL,           -- FK → texts (all use 7508)
    destination_region_ids  integer[] DEFAULT '{}',     -- Valid destination IDs
    point_set_id            integer NOT NULL            -- FK → point_sets (spatial trigger)
);
```

### Related Tables

- **`resources.event_sets`** — Each ring has an event_set_id linking to Kismet sequences
- **`resources.event_sets_sequences`** — Links event_set_id → sequence_id (2 per ring: out + in)
- **`resources.sequences`** — Kismet script paths (e.g., `MapName-Chunk.Main_Sequence.Prefabs.GLB-RingTransporterBase_TC00_Pf0_Seq`)
- **`resources.point_sets` + `resources.point_set_points`** — Spatial trigger boxes for player detection

### All 27 Ring Transport Regions (Seed Data)

#### Castle CellBlock (world 12) — 3 regions, linear chain
| ID | Tag | Destinations | Notes |
|---|---|---|---|
| 1 | CellblockRing1 | {2} | Mission-gated (HackTheRings minigame) |
| 2 | CellblockRing2 | {3} | Mission-gated (EscapeTheCellblock) |
| 3 | CellblockRing3 | {} | Dead end — receive only |

#### Harset (world 57) — 5 regions, fully connected mesh
| ID | Tag | Destinations |
|---|---|---|
| 4 | HarsetRingLeftBottom | {5,6,7,8} |
| 5 | HarsetRingRightBottom | {4,6,7,8} |
| 6 | HarsetRingLeft | {4,5,7,8} |
| 7 | HarsetRingLeftTop | {4,5,6,8} |
| 8 | HarsetinRingRight | {4,5,6,7} |

#### Omega Site (world 18) — 5 regions, hub + cross-world link
| ID | Tag | Destinations | Notes |
|---|---|---|---|
| 10 | OmegaSiteCenterRegion | {11,12,16} | Hub |
| 11 | OmegaSiteBottomRegion | {10,12,16} | |
| 12 | OmegaSiteLeftRegion | {10,11,16} | |
| 14 | OmegaSiteToCmdCenterRegion | {17} | **Cross-world** to CmdCenter |
| 16 | OmegaSiteRightRegion | {10,11,12} | |

#### Omega Site Command Center (world 80) — 1 region
| ID | Tag | Destinations | Notes |
|---|---|---|---|
| 17 | OmegaSiteCmdCenterRegion | {14} | **Cross-world** back to Omega Site |

#### Lucia (world 15) — 12 regions, near-fully connected
| ID | Tag | Destinations |
|---|---|---|
| 18 | Lucia_Ring_ffff0000 | {19,20,21,22,24,25,26,27,28} |
| 19 | Lucia_Ring_fffe0000 | {18,20,21,22,24,25,26,27,28} |
| 20 | Lucia_Ring_fffc0000 | {18,19,21,22,24,25,26,27,28} |
| 21 | Lucia_Ring_fffeffff | {18,19,20,22,24,25,26,27,28} |
| 22 | Lucia_Ring_fffe0002 | {18,19,20,21,24,25,26,27,28} |
| 24 | Lucia_Ring_fffefff8 | {18,19,20,21,22,25,26,27,28} |
| 25 | Lucia_Ring_fff8fffe | {18,19,20,21,22,24,26,27,28} |
| 26 | Lucia_Ring_fff90006 | {18,19,20,21,22,24,25,27,28} |
| 27 | Lucia_Ring_0002000b | {18,19,20,21,22,24,25,26,28} |
| 28 | Lucia_Ring_00070001 | {18,19,20,21,22,24,25,26,27,29} |
| 29 | Lucia_Ring_00080001 | {28} | One-way to ring 28 |

#### Menfa Dark (world 77) — 3 regions
| ID | Tag | Destinations | Notes |
|---|---|---|---|
| 30 | MenfaDark_Ring_00040001 | {31} | Paired |
| 31 | MenfaDark_Ring_00040000 | {30} | Paired |
| 32 | MenfaDark_Ring_ffff0003 | {} | Dead end — receive only |

> **Note:** Region IDs 9, 13, 15, 23 are unused gaps in the sequence (max=32).
> All regions use `display_name_id = 7508` (shared "Ring Transporter" text).

---

## Space Scripts (Level Bindings)

Space scripts bind entity interaction tags to ring transport regions. All are auto-generated by the Atrea Script Editor from `.script` node graph XML files.

### Omega Site (`python/cell/spaces/Omega_Site.py`)

| Interaction Tag | Region ID |
|---|---|
| `OmegaSiteBeamCenter` | 10 |
| `OmegaSiteBeamBottom` | 11 |
| `OmegaSiteBeamLeft` | 12 |
| `OmegaSiteBeamToCmdCenter` | 14 |
| `OmegaSiteBeamRight` | 16 |

Each subscribes to `entity.interact.tag::{TagName}` and calls `space.transporters.get(id).interact(player)`.

### Omega Site CmdCenter (`python/cell/spaces/Omega_Site_CmdCenter.py`)

| Interaction Tag | Region ID |
|---|---|
| `OmegaSiteCmdCenterBeam` | 17 |

### Harset (`python/cell/spaces/Harset.py`)

| Interaction Tag | Region ID |
|---|---|
| `HarsetRingLeftBottom` | 4 |
| `HarsetRingRightBottom` | 5 |
| `HarsetRingLeft` | 6 |
| `HarsetRingLeftTop` | 7 |
| `HarsetRingRight` | 8 |

Also has a non-ring zone transition: `Harset.CommandCenterTransition` triggers `moveTo(0, 0.355, -20, worldName='Harset_CmdCenter')`.

### Lucia (`python/cell/spaces/Lucia.py`)

10 ring interactions mapping tags to regions 18-29.

### Menfa Dark (`python/cell/spaces/Menfa_Dark.py`)

3 ring interactions: regions 30, 31, 32.

### Castle CellBlock (`python/cell/spaces/Castle_CellBlock.py`)

Ring interactions are handled by **mission scripts**, not the level script. The level script only subscribes to `teleport::in` at region 2 for mission 640 completion.

---

## Mission Integration

### HackTheRings (Mission 640) — Castle CellBlock

**File:** `python/cell/missions/Castle_CellBlock/HackTheRings.py`

Player interacts with `HackTheRings_Switch`:
1. If mission step 2120 is **Active**: Launch Livewire minigame. On victory → advance to step 2215.
2. If mission step 2120 is **Completed**: Open ring transport dialog for region 1 (`CellblockRing1`).

The ring is gated behind a puzzle minigame — you must hack it before you can use it.

### EscapeTheCellblock (Mission 680) — Castle CellBlock

**File:** `python/cell/missions/Castle_CellBlock/EscapeTheCellblock.py`

- Player interacts with `Preparation_RingSwitch` → If mission step 2344 active, opens ring transport for region 2.
- Subscribes to `teleport::in` at region 3 → Advances mission step 2345 (escaped the cellblock).

### Duel Forfeit

Teleporting during a duel auto-forfeits with reason `EDUEL_DEFEAT_Teleport = 5`.

---

## Effect System

An alternative movement-lock mechanism exists as an ability-based effect:

```sql
-- effect_id=979, ability_id=892
-- 'Prevent player from moving during transport'
-- 'Ring Transport - decrease movement speed'
-- pulse_count=1, pulse_duration=12, flags=4096 (HideIconOnClient)
```

This appears redundant with the `BSF_MovementLock` state flag used by the Python state machine. May be from an earlier implementation or an additional safety net.

---

## Editor Support (`entities/editor/Nodes.xml`)

Three visual scripting nodes for the Atrea Script Editor:

### Act_RingTransportDlg ("Ring Transport Dialog")
- **Category:** Player
- **Inputs:** Trigger, Player entity, Region Id (DB ref)
- **Outputs:** Successful, Failed
- **Code:** `space.transporters.get(regionId).interact(player)`

### Event_Teleport ("Teleport In")
- **Category:** Player
- **Event:** `teleport::in` filtered by region ID
- **Outputs:** Trigger, Player, Region Id

### Event_TeleportOut ("Teleport Out")
- **Category:** Player
- **Event:** `teleport::out` filtered by region ID
- **Outputs:** Trigger, Player, Src Region Id, Dst Region Id

---

## Known Limitations & Unimplemented Features

1. **Squad transport notifications** — `onSquadMemberRingTransport` and `onSquadMemberRingTransportFinished` are defined in `SGWPlayer.def` but never called. Squad members are NOT notified when a teammate uses a ring.

2. **Vestigial entity properties** — `currentRingTransporterDestination` (PYTHON) and `pendingRingTeleport` (INT8) are defined in `SGWPlayer.def` but unused by the Python implementation, which uses instance attributes instead.

3. **Single-player Kismet animation** — The ring matinee sequence only plays for the first player in the transport group. Two FIXMEs in the code note this limitation: playing per-player would "mess up the ring matinee."

4. **Dead-end rings** — Regions 3 and 32 have empty destination lists. They can receive but never initiate transport.

5. **Cross-world requires co-location** — Cross-world transport uses direct Python object references between `RingTransporter` instances, requiring both spaces to run in the same server process.

---

## Complete Data Flow

```
1. Player interacts with beam console
      entity.interact.tag::OmegaSiteBeamCenter
          │
2. Space script callback
      space.transporters.get(10).interact(player)
          │
3. Server sends destination list
      player.client.onRingTransporterList(sourceRegion, destinations)
          │
4. Client receives Event_NetIn_onRingTransporterList
      → forwards to Event_UI_onRingTransporterList
      → SGWScriptedWindow opens RingTransporterMapMode
      → World map shows destination icons (MinimapIcons:Transporter)
          │
5. Player clicks destination
      Lua calls setRingTransporterDestination(sourceId, destId)
      → Event_NetOut_SetRingTransporterDestination → wire
          │
6. Server receives setRingTransporterDestination RPC
      SGWPlayer.setRingTransporterDestination(regionId, destId)
      → RingTransporter.selectDestination(player, destId)
          │
7. State machine: IDLE → SEND_WAIT → SEND_WARMUP
      Play Region_Teleport_Out (Kismet event 8000)
      Lock movement (BSF_MovementLock)
      Notify remote ring: remoteWait() → remoteSend()
          │
8. +3.5s: Hide all players (setVisible=false)
          │
9. +4.0s: Teleport (player.teleportTo → moveTo)
      Set player.destinationRingId = destRegionId
      STATE → REMOTE_LOAD_WAIT
          │
10. Client loads new map (if cross-world)
      SGWPlayer.onClientReady()
      → space.transporters.get(destinationRingId).playerLoaded(self)
          │
11. All players loaded → REMOTE_WARMUP
      Play Region_Teleport_In (Kismet event 8001)
          │
12. +3.0s: Show all players (setVisible=true)
      STATE → COOLDOWN
          │
13. +2.5s: Unlock movement, fire events
      player.onTeleportIn(regionId) → 'teleport::in' event
      STATE → IDLE
```

---

## File Index

| Category | File | Purpose |
|---|---|---|
| **Core Python** | | |
| | `python/cell/RingTransporter.py` | State machine + RingTransporterManager |
| | `python/common/defs/RingTransporterRegion.py` | DB data model |
| | `python/cell/SGWPlayer.py` | teleportTo, setRingTransporterDestination, onClientReady |
| | `python/cell/Space.py` | Creates RingTransporterManager per space |
| | `python/cell/GenericRegion.py` | Spatial trigger system for region enter/exit |
| | `python/common/Constants.py` | INTERACTION_RingTransport = 7457 |
| | `python/Atrea/enums.py` | Region_Teleport_Out/In, BSF_MovementLock |
| | `python/common/defs/EventSet.py` | Event set loading + getSequence() |
| **Space Scripts** | | |
| | `python/cell/spaces/Omega_Site.py` | 5 beam interactions (Asgard theme) |
| | `python/cell/spaces/Omega_Site_CmdCenter.py` | 1 beam interaction |
| | `python/cell/spaces/Harset.py` | 5 ring interactions |
| | `python/cell/spaces/Lucia.py` | 10 ring interactions |
| | `python/cell/spaces/Menfa_Dark.py` | 3 ring interactions |
| | `python/cell/spaces/Castle_CellBlock.py` | Mission-gated teleport::in handler |
| **Mission Scripts** | | |
| | `python/cell/missions/Castle_CellBlock/HackTheRings.py` | Ring 1 gated by Livewire minigame |
| | `python/cell/missions/Castle_CellBlock/EscapeTheCellblock.py` | Ring 2/3 gated by mission progress |
| **Entity Definitions** | | |
| | `entities/defs/SGWPlayer.def` | Properties + RPC methods |
| | `entities/defs/alias.xml` | RegionInfo FIXED_DICT type |
| | `entities/defs/enumerations.xml` | Interaction types, sequence events |
| | `entities/editor/Nodes.xml` | Visual script editor nodes |
| **Database** | | |
| | `db/resources/Worlds/Tables/ring_transport_regions.sql` | Table DDL |
| | `db/resources/Worlds/Seed/ring_transport_regions.sql` | 27 region seed records |
| | `db/resources/Events/Seed/event_sets.sql` | Event set names |
| | `db/resources/Events/Seed/event_sets_sequences.sql` | Event set → sequence links |
| | `db/resources/Events/Seed/sequences.sql` | Kismet script paths |
| | `db/resources/Effects/Seed/effects.sql` | Movement-lock effect 979 |
| **Client** | | |
| | `SGWGame/Content/UI/Core/WorldMap/RingTransporterMapMode.lua` | Destination picker UI |
| | `SGWGame/Content/UI/Core/WorldMap/WorldMapPOIData.lua` | Map POI icons |
| | `SGWGame/Content/UI/CEGUIData/imagesets/MinimapIcons.imageset` | Transporter icon (17x18) |
| | `Common/xml/slash_commands/InternalSlashCommands.xml` | `/gmsetringdestination` debug cmd |
| **Script Sources** | | |
| | `data/scripts/spaces/Omega_Site.script` | Node graph XML (generates .py) |
| | `data/scripts/spaces/Harset.script` | Node graph XML |
| | `data/scripts/spaces/Lucia.script` | Node graph XML |
| | `data/scripts/missions/Castle_CellBlock/HackTheRings.script` | Mission node graph |
| | `data/scripts/missions/Castle_CellBlock/EscapeTheCellblock.script` | Mission node graph |
