# World Entry Phases

> **Last updated**: 2026-03-08
> **RE Status**: Verified via Ghidra, pcap captures, C++ source, and Python scripts
> **Sources**: `src/baseapp/mercury/sgw/client_handler.cpp`, `python/cell/SGWPlayer.py`, `crates/services/src/base/world_entry.rs`, `crates/services/src/mercury/world_data.rs`

---

## Overview

World entry is the process of transitioning a player from the character select screen into the game world. It occurs after Phase 4 (character list / tick sync) and involves multiple message exchanges between client and server.

The process is split into two sub-phases (5b-A and 5b-B) with a client-side terrain load in between, followed by a large fragmented `mapLoaded` bundle containing all entity state initialization.

## Phase 5a: RESET_ENTITIES

**Trigger**: Client sends `playCharacter` (msg_id `0xC4` — Account base method index 4).

**Server sends**:
```
RESET_ENTITIES (0x04) — CONSTANT_LENGTH = 1
  [keepBase: u8 = 0]
```

This tells the client to tear down all existing entity state (from a previous session or character). The client processes this, then sends `onClientReady` (msg_id `0x01`) to signal it's ready for the new entity.

**Server stores**: Pending world entry data (player ID, position, world name) for use in Phase 5b.

## Phase 5b-A: CREATE_BASE_PLAYER + onClientMapLoad

**Trigger**: Client sends `onClientReady` after processing RESET_ENTITIES.

**Server sends** (single Mercury packet):

### Message 1: CREATE_BASE_PLAYER (0x05)
```
WORD_LENGTH format:
  [msg_id: 0x05]
  [word_len: u16 LE = 6]
  [entityId: u32 LE]        — player's entity ID
  [classId: u8 = 0x02]      — SGWPlayer class ID
  [propCount: u8 = 0]       — no initial properties
```

Creates the base entity on the client. The `classId` determines which entity class the client instantiates (0x02 = SGWPlayer per `entities.xml` ordering).

### Message 2: onClientMapLoad (method index 117)
```
Extended encoding (index 117 >= 61):
  [msg_id: 0xBD]
  [word_len: u16 LE]
  [entityId: u32 LE]
  [sub_index: u8 = 56]      — (117 - 61)
  [clientMap: WSTRING]       — terrain name, e.g., "Castle_CellBlock"
  [worldId: i32 LE]          — world identifier from worlds table
```

Tells the client which terrain to load. The client begins an async terrain load, then — once loading is complete — sends a base method call (typically `versionInfoRequest` or `elementDataRequest`) which triggers Phase 5b-B.

### Client Map Mapping

| World Name | Client Map | World ID |
|-----------|------------|----------|
| `Castle_CellBlock` | `Castle_CellBlock` | 12 |
| `SGC_W1` | `SGC_W1` | 58 |
| `P2A-018_W1` | `P2A-018_W1` | 3 |
| `Agnos_W1` | `Agnos_W1` | 6 |

Source of truth: `db/resources/Worlds/Seed/worlds.sql`

## Phase 5b-B: VIEWPORT + CELL + POSITION

**Trigger**: Client sends any cell/base method call after Phase 5b-A (terrain loaded).

**Server sends** (single Mercury packet, three messages):

### Message 1: SPACE_VIEWPORT_INFO (0x08)
```
CONSTANT_LENGTH = 13:
  [entityId: u32 LE]         — player entity ID
  [entityId: u32 LE]         — same entity ID (repeated)
  [spaceId: u32 LE]          — space identifier
  [viewportId: u8 = 0]       — always 0
```

### Message 2: CREATE_CELL_PLAYER (0x06)
```
WORD_LENGTH format, payload = 32 bytes:
  [msg_id: 0x06]
  [word_len: u16 LE = 32]
  [spaceId: u32 LE]          — space identifier (matches VIEWPORT)
  [vehicleId: u32 LE = 0]    — always 0 (no vehicle)
  [posX: f32 LE]             — X position
  [posY: f32 LE]             — Y position (vertical)
  [posZ: f32 LE]             — Z position
  [rotX: f32 LE]             — rotation X (pitch)
  [rotZ: f32 LE]             — rotation Z (yaw) ← NOTE: Y/Z SWAPPED
  [rotY: f32 LE]             — rotation Y (roll) ← NOTE: Y/Z SWAPPED
```

**Important**: Rotation is sent in `X, Z, Y` order (Y and Z swapped from the typical X, Y, Z convention). The client reads these values positionally at fixed offsets.

### Message 3: FORCED_POSITION (0x31)
```
CONSTANT_LENGTH = 49:
  [entityId: u32 LE]         — player entity ID
  [spaceId: u32 LE]          — space identifier
  [vehicleId: u32 LE = 0]    — always 0
  [posX: f32 LE]             — X position
  [posY: f32 LE]             — Y position
  [posZ: f32 LE]             — Z position
  [velX: f32 LE = 0.0]       — velocity X (zero at spawn)
  [velY: f32 LE = 0.0]       — velocity Y
  [velZ: f32 LE = 0.0]       — velocity Z
  [rotX: f32 LE]             — rotation X
  [rotZ: f32 LE]             — rotation Z ← Y/Z SWAPPED
  [rotY: f32 LE]             — rotation Y ← Y/Z SWAPPED
  [flags: u8 = 0]            — position flags
```

## Fragmented mapLoaded Bundle

**Sent immediately after** the Phase 5b-B packet.

The `mapLoaded` bundle contains 27+ entity method calls packed into a single Mercury bundle that exceeds the maximum packet size (1300 bytes per fragment). The bundle is split into fragments using Mercury's fragmentation protocol.

### Fragment Structure

Each fragment packet:
```
[flags | FLAG_FRAGMENTED: u8]
[body_chunk: variable]
[frag_begin: u32 LE]        — sequence ID of first fragment
[frag_end: u32 LE]          — sequence ID of last fragment
[seq_id: u32 LE]            — this fragment's sequence ID
[acks: variable]            — acknowledgements
```

Fragment body size: 1300 bytes (conservative, under Mercury's `MAX_BODY_LENGTH = 1411`).

The client reassembles ALL fragments before processing the bundle atomically.

### mapLoaded Bundle Contents (in order)

| # | Method | Index | Encoding | Description |
|---|--------|-------|----------|-------------|
| 1 | `onPlayerDataLoaded` | 115 | Extended (0xBD, sub=54) | Signals player data ready |
| 2 | `setupWorldParameters` | 122 | Extended (0xBD, sub=61) | World name, ID, instance, max players |
| 3 | `clearClientHintedGenericRegions` | 124 | Extended (0xBD, sub=63) | Clear region hints |
| 4 | `onResetMapInfo` | 126 | Extended (0xBD, sub=65) | Reset map state |
| 5 | `onStatUpdate` | 20 | Direct (0x94) | 70+ current stat values |
| 6 | `onStatBaseUpdate` | 21 | Direct (0x95) | 70+ base stat values |
| 7 | `onArchetypeUpdate` | 23 | Direct (0x97) | Archetype identifier |
| 8 | `onLevelUpdate` | 15 | Direct (0x8F) | Character level |
| 9 | `onAlignmentUpdate` | 24 | Direct (0x98) | Faction alignment |
| 10 | `onFactionUpdate` | 25 | Direct (0x99) | Faction ID |
| 11 | `onBeingNameUpdate` | 17 | Direct (0x91) | Character name |
| 12 | `onExtraNameUpdate` | 130 | Extended (0xBD, sub=69) | Extra name/title |
| 13 | `onExpUpdate` | 131 | Extended (0xBD, sub=70) | Current XP |
| 14 | `onMaxExpUpdate` | 132 | Extended (0xBD, sub=71) | Max XP for level |
| 15 | `onStateFieldUpdate` | 19 | Direct (0x93) | State flags |
| 16 | `onTargetUpdate` | 16 | Direct (0x90) | Current target entity |
| 17 | **`BeingAppearance`** | **26** | **Direct (0x9A)** | **Bodyset + component list** |
| 18 | **`onEntityTint`** | **10** | **Direct (0x8A)** | **Primary/secondary/skin tint** |
| 19 | `onEntityProperty` | 7 | Direct (0x87) | Entity property flags |
| 20 | `onKismetEventSetUpdate` | 9 | Direct (0x89) | Kismet event state |
| 21 | `onTimeOfDay` | 102 | Extended (0xBD, sub=41) | Time of day value |
| 22+ | `onBagInfo` × N | 69 | Extended (0xBD, sub=8) | Inventory bag structure |
| 23+ | `onUpdateItem` × N | 72 | Extended (0xBD, sub=11) | Inventory items |
| 24 | `onCashChanged` | 75 | Extended (0xBD, sub=14) | Currency amount |
| 25 | `onKnownAbilitiesUpdate` | 101 | Extended (0xBD, sub=40) | Known ability list |
| 26 | `onAbilityTreeInfo` | 141 | Extended (0xBD, sub=80) | Ability tree data |
| 27 | `onUpdateKnownCrafts` | 139 | Extended (0xBD, sub=78) | Crafting recipes |
| 28+ | Mission updates × N | 80-84 | Extended | Per active mission |
| 29 | `setupStargateInfo` | 65 | Extended (0xBD, sub=4) | Stargate data |

### Entity Method Encoding Reference

- **Direct** (indices 0-60): `msg_id = index | 0x80` → range `0x80-0xBC`
- **Extended** (indices 61+): `msg_id = 0xBD`, `sub_index = index - 61`
- All entity methods use WORD_LENGTH (u16 prefix after msg_id)

## C++ vs Rust Timing

### C++ Reference Server

```
BaseApp                         CellApp                     Client
   │                               │                          │
   ├──VIEWPORT+CELL+POSITION──────────────────────────────────►│
   │                               │                          │
   ├──createEntity(player)────────►│                          │
   │                               │                          │
   │                    ┌──────────┤  (entity construction)   │
   │                    │  async   │                          │
   │                    └──────────┤                          │
   │                               │                          │
   │◄──mapLoaded(data)─────────────┤                          │
   │                               │                          │
   ├──[fragmented mapLoaded]──────────────────────────────────►│
   │                               │                          │
```

The inter-service round-trip between VIEWPORT+CELL+POSITION and mapLoaded provides natural delay for the client to process CREATE_CELL_PLAYER and mark the entity as "ready."

### Rust Server

```
RustServer                                               Client
   │                                                        │
   ├──VIEWPORT+CELL+POSITION───────────────────────────────►│
   ├──[fragmented mapLoaded]───────────────────────────────►│  (immediate)
   │                                                        │
```

Both are built and sent in the same function call. If the client processes the mapLoaded fragments before finishing CREATE_CELL_PLAYER processing, `BeingAppearance` may be silently dropped (see [Client Visual System](../engine/client-visual-system.md) — entity "not ready" path).

## Rotation Order Convention

| Context | Wire Order | Notes |
|---------|-----------|-------|
| CREATE_CELL_PLAYER | X, Z, Y | Y and Z swapped |
| FORCED_POSITION (world entry) | X, Z, Y | Same swap |
| FORCED_POSITION (standalone) | Caller-provided | Typically X, Z, Y by convention |
| Client reads | Positional | Always `[rot1][rot2][rot3]` at fixed offsets |

The C++ source confirms: `rotX << rotZ << rotY` in `createCellPlayer()` (file `client_handler.cpp:384-414`).

## Related Documents

- [Login Handshake](login-handshake.md) — Phases 1-3 (SOAP + Mercury connect)
- [Mercury Wire Format](mercury-wire-format.md) — Packet structure and message encoding
- [Client Visual System](../engine/client-visual-system.md) — How the client handles BeingAppearance
- [Character Visual Components](../engine/character-visual-components.md) — Visual data sources
- [Position Updates](position-updates.md) — Movement update format (post-entry)
