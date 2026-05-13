# World Entry Pipeline — RE Findings

**Confidence**: HIGH (core wire formats), MEDIUM (event chain internals)
**Date**: 2026-05-13
**Sources**: Ghidra decompilation of SGW.exe (x86 PE, image base 0x00400000), string
cross-references, RTTI recovery, `docs/protocol/world-entry-phases.md`,
`docs/protocol/login-handshake.md`, `docs/reverse-engineering/findings/entity-creation-wire-formats.md`

---

## Overview

World entry is the sequence of messages and internal events that carry the client from
a successful BaseApp login through terrain load to a fully-playable in-world state.
The pipeline has eight phases, driven jointly by the BigWorld networking layer and the
CME (Cheyenne Mountain Entertainment) EventSignal bus layered on top. The binary
provides strong evidence for the wire formats of all message-type boundaries; the
internal event chain (CME signals between subsystems) is partially recovered through
RTTI clusters and subscriber registration shims.

---

## Phase 1 — TCP / HTTP Authentication Handshake

**Client → auth server (port 8081): SOAP `SGWLoginRequest`**

Fields confirmed by string xrefs in `SGW.exe`:
- `SKU` — product identifier
- `AccountName` — login name
- `Password` — SHA-1 hash of password (confirmed by `login-handshake.md` and SOAP handler strings)
- `ProtocolDigest` — version negotiation token

**Server → client: SOAP `SGWLoginResponse`**

Returns a session ticket and shard list. The client stores the ticket for use in Phase 3.

**Evidence**: String `"SGWLoginRequest"` at `.exe` data section; SOAP handler xrefs in
Python `python/auth/SGWLogin.py` (source-side confirmation). Binary strings confirm
SHA-1 password hashing path.

---

## Phase 2 — Shard Select + BaseApp Connection

**Client → shard-select server: `SGWServerSelectRequest`**

CME signal `Event_NetIn_AccountLoginSuccess` is received after Phase 1 and dispatched
to `GameProxyPlayer` (subscriber RTTI: `CME_EventSignal_ZV5_..._AccountLoginSuccess...__vfunc_3`
@ `0x00df6d00`). Registration function: `register_NetIn_AccountLoginSuccess` @ `0x00de04c0`.

**Client → BaseApp (UDP, Mercury): `baseAppLogin`**

Wire format (25 bytes, from `login-handshake.md`):
```
[accountId: u32 LE]       4 bytes
[ticketLength: u8]         1 byte (always 20)
[ticket: char[20]]        20 bytes
```

Encryption: AES-256-CBC with HMAC-MD5 per login-handshake.md.

Post-login synchronization (from `login-handshake.md`):
- `updateFrequency` → server's preferred update rate
- `tickSync` → client-server clock alignment
- `gameTime` → initial game time

**CME signal**: `Event_NetIn_ServerSelectSuccess` received after shard selection.
Registration: `register_NetIn_ServerSelectSuccess` @ `0x00ddfd00`.

---

## Phase 3 — CREATE_BASE_PLAYER

**Trigger**: Client sends `playCharacter` (base method index 4, msg_id `0xC4`).
CME signal: `Event_NetOut_PlayCharacter` (strings at `0x019bf4f8`, `0x019c2670`).

**Server → client: `CREATE_BASE_PLAYER` (msg_id `0x05`, WORD_LENGTH)**

```
[msg_id:    0x05]        1 byte
[word_len:  u16 LE = 6]  2 bytes (payload size)
[entityId:  u32 LE]      4 bytes — player entity ID
[classId:   u8 = 0x02]   1 byte  — SGWPlayer (entities.xml position 2)
[propCount: u8 = 0]      1 byte  — no initial properties
```

Handler: `ServerConnection_CreateBasePlayer` @ `0x00dddca0`

From decompile:
1. Reads 4-byte entityId from data stream.
2. Reads 2-byte typeId (classId) from data stream.
3. Invokes entity creation delegate: instantiates the SGWPlayer base-proxy object.
4. Stores entity reference in the ServerConnection entity map.

**Server → client: `onClientMapLoad` (entity method index 117)**

Extended encoding (index >= 61):
```
[msg_id:     0xBD]        1 byte
[word_len:   u16 LE]      2 bytes
[entityId:   u32 LE]      4 bytes — player entity ID
[sub_index:  u8 = 56]     1 byte  — (117 - 61)
[areaName:   WSTRING]     variable — logical area identifier
[mapPath:    WSTRING]     variable — terrain package name (e.g. "Castle_CellBlock")
[WorldID:    i32 LE]      4 bytes — world identifier (matches `worlds` table)
[Location:   3x f32 LE]  12 bytes — spawn position
[Direction:  f32 LE]      4 bytes — spawn heading/yaw
```

Handler: `GameProxyPlayer_HandleOnClientMapLoad` @ `0x00df27f0`

**AUDIT FINDING** (discrepancy vs `world-entry-phases.md`): The existing doc claims fields
`clientMap (WSTRING)` and `worldId (i32)`. Binary assert strings at `0x019d26c8` (`areaName`),
`0x019d26d0` (`mapPath`), and `0x019d2684` (`WorldID`) show the actual field names. Two
additional fields — `Location` (3x f32) and `Direction` (f32) — are present in the binary
but not documented. The field names `clientMap`/`worldId` are Cimmeria inventions and should
be corrected in `world-entry-phases.md`.

---

## Phase 4 — Terrain Load (Client-Side Async)

After receiving `onClientMapLoad`, the client triggers a UE3 level load for `mapPath`.
This is an asynchronous operation; the client enters a loading screen.

**UE3 completion callback**: When terrain finishes loading, UE3's `FCallbackEventDevice`
fires callback index `0x32` (`CALLBACK_PostLoadMap`). The registered handler is:

`EntityManager_PostLoadMap` @ `0x00dd0b00`

From decompile:
1. Asserts `in_stack == 0x32` (CALLBACK_PostLoadMap).
2. Optionally fires voice device callback `0x36` if voice handler is set.
3. Gets CME EventSignal system via `thunk_FUN_0054c900()`.
4. Calls `FUN_00a372f0` to fire `Event_Level_PostLoad` to all CME subscribers.

**CME subscribers for `Event_Level_PostLoad`**: `GameProxyPlayer` (RTTI accessor
`0x00df6e80`) and `GameAppearanceManager` (RTTI accessor `0x00e9a480`).

**Open question**: The `GameProxyPlayer` handler body for `Event_Level_PostLoad` is stored
at runtime in `CmeMemberCallback.pMethodPtr` (field +0x08 of the MemberCallback instance).
Static analysis cannot read this — the handler body address is not statically resolvable from
the vtable alone. The handler is expected to emit `Event_NetOut_ClientReady` or send
`ENABLE_ENTITIES` directly.

---

## Phase 5 — RESET_ENTITIES + ENABLE_ENTITIES Exchange

**Note on terminology**: The BigWorld codebase calls this `enableEntities`. The client
sends this as a base-channel method after resetting entity state. The signal name string
`"onClientReady"` at `0x019c2828` is the CME bus name for this event; the wire method
is registered as `"enableEntities"` at `0x019d092c`.

### RESET_ENTITIES (server → client, msg_id 0x04)

Wire format (CONSTANT_LENGTH = 1):
```
[keepBase: u8 = 0]   1 byte
```

Message handler: registered in the BigWorld client message table at address `0x017bb210`
(reference to `"resetEntities"` string at `0x019d09f8`). The handler calls
`PurgeAndRebuildEntityStateLists` @ `0x00dda0e0`.

`PurgeAndRebuildEntityStateLists` from decompile:
1. Asserts `*(this+0x316) != 0` — entities must be enabled before reset.
2. `ServerConnection_Send` — flush outgoing bundle.
3. Resets 4 linked-list sentinels at offsets `+0xF88`, `+0xF94`, `+0xFA0`, `+0xFB0`.
4. Clears pending-update flag at `+0xFA8`.
5. Copies template at `+0xFE8` into `+0xFEC` and `+0xFF4`.
6. If `*pSpaceData == 0` (new space): zeros entity-ID field at `+0x16C`; compacts
   pending-message vector at `+0xFD0/+0xFD4` via `memmove_s`.
7. `ServerConnection_Send` — flush again.
8. Clears `*(this+0x316)` — `bEntitiesEnabled = 0`.
9. Calls `BroadcastEntityActivation(this)` — sends ENABLE_ENTITIES.
10. If handler registered at `*(this+0x168)`: calls `handler->vtable[15](spaceData)`.

### ENABLE_ENTITIES (client → server, base method index 1)

`BroadcastEntityActivation` @ `0x00dd9280` from decompile:
1. `ServerConnection_AssertChannel` — assert channel open.
2. `FUN_0157ad80(this, DAT_01ef2500)` — start message using ENABLE_ENTITIES descriptor.
3. Reserves `DAT_01ef2500->size` bytes in the bundle.
4. Logs `"ServerConnection::enableEntities: Enabling entities %d"` (@ `0x019cf548`).
5. `ServerConnection_Send` — flush to BaseApp channel.
6. Sets `*(pServerConn+0x316) = 1` — `bEntitiesEnabled` flag.

**CONFIRMED (W-enable-entities, 2026-05-13)**: ENABLE_ENTITIES carries **8 bytes** — a
SGW-custom `uint64` dummy payload. The descriptor at `DAT_01ef2500` is initialized by the
static initializer block at `0x017bade0`–`0x017bae07`. Disassembly of that block shows
`PUSH 0x8` at `0x017bade9` as the size argument to the InterfaceElement constructor
(`0x015785c0`). Cross-validated against the `resetEntities` initializer at `0x017bb200`–
`0x017bb225`, which pushes `0x1` for its 1-byte `keepBase` payload — confirming the push
position IS the size field. The SGW C++ source at
`deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp` line 83 independently confirms:
`{Message::CONSTANT_LENGTH, 8, "ENABLE_ENTITIES", true}`.

W-misc-gaps Session 5 (2026-05-13) incorrectly concluded 1 byte. That session misread the
initializer: the `MOV DWORD PTR [EAX], 0x1` at `0x017badf7` writes a reliability flag into
the stack-allocated struct passed by pointer to the constructor — it is NOT the size field.
The size `0x8` is the distinct `PUSH 0x8` three instructions earlier.

---

## Phase 6 — SPACE_VIEWPORT_INFO + CREATE_CELL_PLAYER + FORCED_POSITION

After the server receives ENABLE_ENTITIES, it sends three messages in one Mercury packet.

### SPACE_VIEWPORT_INFO (msg_id 0x08, CONSTANT_LENGTH = 13)

```
[entityId:   u32 LE]   — player entity ID
[entityId2:  u32 LE]   — same entity ID (repeated)
[spaceId:    u32 LE]   — space identifier
[viewportId: u8 = 0]   — always 0
```

Handler: `ServerConnection_SpaceViewportInfo` @ `0x00dda6c0`
(documented in `entity-creation-wire-formats.md`)

### CREATE_CELL_PLAYER (msg_id 0x06, WORD_LENGTH = 32 bytes)

```
[msg_id:     0x06]       1 byte
[word_len:   u16 LE]     2 bytes
[spaceId:    u32 LE]     4 bytes
[vehicleId:  u32 LE = 0] 4 bytes — always 0 at world entry
[posX:       f32 LE]     4 bytes
[posY:       f32 LE]     4 bytes — vertical
[posZ:       f32 LE]     4 bytes
[rotX:       f32 LE]     4 bytes — pitch
[rotZ:       f32 LE]     4 bytes — yaw    *** Y/Z SWAPPED ***
[rotY:       f32 LE]     4 bytes — roll   *** Y/Z SWAPPED ***
```

Handler: `ServerConnection_CreateCellPlayer` @ `0x00dda2e0`

Rotation is read via `FUN_015846a0` which applies the X, Z, Y ordering internally.
Confirmed by C++ source comment in `client_handler.cpp`: `rotX << rotZ << rotY`.

### FORCED_POSITION (msg_id 0x31, CONSTANT_LENGTH = 49)

```
[entityId:  u32 LE]      4 bytes
[spaceId:   u32 LE]      4 bytes
[vehicleId: u32 LE = 0]  4 bytes
[posX:      f32 LE]      4 bytes
[posY:      f32 LE]      4 bytes
[posZ:      f32 LE]      4 bytes
[velX:      f32 LE = 0]  4 bytes
[velY:      f32 LE = 0]  4 bytes
[velZ:      f32 LE = 0]  4 bytes
[rotX:      f32 LE]      4 bytes
[rotZ:      f32 LE]      4 bytes — *** Y/Z SWAPPED ***
[rotY:      f32 LE]      4 bytes — *** Y/Z SWAPPED ***
[flags:     u8 = 0]      1 byte
```

Handler: `ServerConnection_ForcedPosition` @ `0x00dd9ee0`
(documented in `entity-creation-wire-formats.md`)

---

## Phase 7 — mapLoaded Bundle (Server → Client, Fragmented)

Immediately after Phase 6, the server sends a fragmented Mercury bundle containing 27+
entity method calls. This bundle initializes all player state visible to the client.

**Note on "mapLoaded" terminology**: The string `"mapLoaded"` does not appear in the
`SGW.exe` binary. The term is a BigWorld C++ server-side concept (CellApp sends `mapLoaded`
to BaseApp, which builds the bundle). From the client's perspective, this is simply a large
fragmented entity-method bundle received after `CREATE_CELL_PLAYER`.

### Fragment Structure

Each fragment (from Mercury protocol — `mercury-protocol-internals.md`):
```
[flags | FLAG_FRAGMENTED: u8]
[body_chunk: variable, max 1300 bytes]
[frag_begin: u32 LE]   — sequence ID of first fragment
[frag_end:   u32 LE]   — sequence ID of last fragment
[seq_id:     u32 LE]   — this fragment's sequence ID
[acks: variable]
```

The client reassembles all fragments before processing.

### Bundle Contents (in order, 29 entries)

| # | Method | Index | Encoding | Handler / Note |
|---|--------|-------|----------|----------------|
| 1 | `onPlayerDataLoaded` | 115 | Extended (0xBD sub=54) | Player data ready signal |
| 2 | `setupWorldParameters` | 122 | Extended (0xBD sub=61) | Physics constants; `GameWorldConstants_HandleSetupWorldParameters` @ `0x00c71a20` |
| 3 | `clearClientHintedGenericRegions` | 124 | Extended (0xBD sub=63) | Clear region hints |
| 4 | `onResetMapInfo` | 126 | Extended (0xBD sub=65) | Reset map state |
| 5 | `onStatUpdate` | 20 | Direct (0x94) | 70+ current stat values |
| 6 | `onStatBaseUpdate` | 21 | Direct (0x95) | 70+ base stat values |
| 7 | `onArchetypeUpdate` | 23 | Direct (0x97) | Archetype identifier |
| 8 | `onLevelUpdate` | 15 | Direct (0x8F) | Character level |
| 9 | `onAlignmentUpdate` | 24 | Direct (0x98) | Faction alignment |
| 10 | `onFactionUpdate` | 25 | Direct (0x99) | Faction ID |
| 11 | `onBeingNameUpdate` | 17 | Direct (0x91) | Character name |
| 12 | `onExtraNameUpdate` | 130 | Extended (0xBD sub=69) | Title/extra name |
| 13 | `onExpUpdate` | 131 | Extended (0xBD sub=70) | Current XP |
| 14 | `onMaxExpUpdate` | 132 | Extended (0xBD sub=71) | Max XP for level |
| 15 | `onStateFieldUpdate` | 19 | Direct (0x93) | BSF_* state flags |
| 16 | `onTargetUpdate` | 16 | Direct (0x90) | Current target entity |
| 17 | `BeingAppearance` | 26 | Direct (0x9A) | Bodyset + component list |
| 18 | `onEntityTint` | 10 | Direct (0x8A) | Primary/secondary/skin tint |
| 19 | `onEntityProperty` | 7 | Direct (0x87) | Entity property flags |
| 20 | `onKismetEventSetUpdate` | 9 | Direct (0x89) | Kismet event state |
| 21 | `onTimeOfDay` | 102 | Extended (0xBD sub=41) | Time of day |
| 22+ | `onBagInfo` × N | 69 | Extended (0xBD sub=8) | Inventory bags |
| 23+ | `onUpdateItem` × N | 72 | Extended (0xBD sub=11) | Inventory items |
| 24 | `onCashChanged` | 75 | Extended (0xBD sub=14) | Currency |
| 25 | `onKnownAbilitiesUpdate` | 101 | Extended (0xBD sub=40) | Ability list |
| 26 | `onAbilityTreeInfo` | 141 | Extended (0xBD sub=80) | Ability tree |
| 27 | `onUpdateKnownCrafts` | 139 | Extended (0xBD sub=78) | Crafting recipes |
| 28+ | Mission updates × N | 80–84 | Extended | Per active mission |
| 29 | `setupStargateInfo` | 65 | Extended (0xBD sub=4) | Stargate data |

**Encoding reference**:
- Direct (index 0–60): `msg_id = index | 0x80` → range `0x80–0xBC`
- Extended (index 61+): `msg_id = 0xBD`, `sub_index = index - 61`

`setupWorldParameters` handler `GameWorldConstants_HandleSetupWorldParameters` @ `0x00c71a20`
loads `BW_TO_UE3_SCALE = 100.0f` into `DAT_018cad90` (`0x42C80000` in IEEE 754), plus
`maxWalkSpeed`, `gravity`, and `jumpZ` constants.

---

## Phase 8 — ClientReady → Entity AoI Bootstrap

After the client processes the mapLoaded bundle, it has full player state. The CME signal
`Event_NetOut_ClientReady` (signal string `"onClientReady"` @ `0x019c2828`) is registered
via `register_NetOut_ClientReady` @ `0x00d93d80`. The `SGWNetworkManager` subscribes:
- RTTI accessor: `CME_EventSignal_ZV6_..._ClientReady...__vfunc_3` @ `0x00d45be0`
- vtable ref: `0x019c5f2c`
- Handler dtor: `SGWNetworkManager_VEvent_NetOut_ClientReady___EventHandler__vfunc_0` @ `0x00d67b90`

The `GameProxyPlayer` `Event_World_Loaded` subscriber RTTI: `0x00df7b80` (vtable at
`0x019d5d94`). `Event_Level_PostLoad` subscriber RTTI: `0x00df6e80` (vtable at
`0x019d5abc`).

`Event_World_Loaded` also has a `Minimap` subscriber (RTTI: `0x00e2af30`).

After ClientReady is acknowledged server-side, the server begins streaming AoI (Area of
Interest) entity updates. The `Event_Player_PawnCreated` signal triggers
`GameProxyPlayer_HandlePlayerPawnCreated` @ `0x00de8670`, which sets up:
- Target indicator actor (`*(GameProxyPlayer+0x10)`)
- Ground target actor (`*(GameProxyPlayer+0x14)`)
- Player controller binding (`*(GameProxyPlayer+0x0C)`)
- Physics/position data sync to the UE3 pawn

---

## Signal Bus Summary

| Phase | CME Signal | Direction | Evidence |
|-------|-----------|-----------|----------|
| 2 | `Event_NetIn_AccountLoginSuccess` | Server→client | RTTI @ `0x00df6d00`, reg @ `0x00de04c0` |
| 2 | `Event_NetIn_ServerSelectSuccess` | Server→client | RTTI @ `0x00df6e00`, reg @ `0x00ddfd00` |
| 3 | `Event_NetOut_PlayCharacter` | Client→server | Strings @ `0x019bf4f8`, `0x019cb1fc` |
| 4 | `Event_Level_PostLoad` | Internal | Emitted by `EntityManager_PostLoadMap` @ `0x00dd0b00` |
| 4 | `Event_World_Loaded` | Internal | RTTI cluster @ `0x00df7b80`, `0x00e2af30` |
| 8 | `Event_NetOut_ClientReady` | Client→server | RTTI @ `0x00d45be0`, reg @ `0x00d93d80` |
| 8 | `Event_Player_PawnCreated` | Internal | Handler @ `0x00de8670` |

---

## Key Constants and Globals

| Symbol | Address | Value | Note |
|--------|---------|-------|------|
| `GEngine` | `DAT_01ee1254` | runtime | UE3 GEngine singleton |
| `g_EntityManager` | `DAT_01ef244c` | runtime | BigWorld EntityManager |
| `BW_TO_UE3_SCALE` | `DAT_018cad90` | `0x42C80000` (100.0f) | BigWorld unit → UE3 cm |
| `bEntitiesEnabled` | `ServerConnection+0x316` | u8 | Set by BroadcastEntityActivation |
| `ENABLE_ENTITIES descriptor` | `DAT_01ef2500` | runtime | Size field determines payload bytes |
| `mControlledEntity` | `GameProxyPlayer+0x08` | pointer | Active entity |
| `mPlayerController` | `GameProxyPlayer+0x0C` | pointer | UE3 player controller |
| `mTargetIndicator` | `GameProxyPlayer+0x10` | pointer | Target indicator actor |
| `mGroundTarget` | `GameProxyPlayer+0x14` | pointer | Ground target actor |

---

## Timing Divergence: C++ vs Rust

The C++ reference server uses an inter-service round-trip between Phase 6 and the
mapLoaded bundle — the CellApp constructs the bundle and sends it back to the BaseApp,
which forwards it to the client. This round-trip gives the client time to finish processing
`CREATE_CELL_PLAYER` and mark the entity as "ready" before `BeingAppearance` arrives.

The Rust server builds and sends both in the same call, so the mapLoaded bundle may arrive
at the client before `CREATE_CELL_PLAYER` processing is complete. If the entity is not yet
marked "ready" when `BeingAppearance` (method index 26) is processed, the visual system
silently drops the appearance data (see `client-visual-system.md`). A small artificial delay
or a proper "entity ready" gate is needed in the Rust implementation.

---

## Audit Findings vs `world-entry-phases.md`

| Claim | Verdict | Evidence |
|-------|---------|----------|
| `onClientMapLoad` fields: `clientMap (WSTRING)` + `worldId (i32)` | **WRONG NAMES** | Assert strings show `areaName`, `mapPath`, `WorldID`, `Location`, `Direction` (0x019d26c8, 0x019d26d0, 0x019d2684) |
| RESET_ENTITIES → `onClientReady` (msg_id 0x01) sequence | CONSISTENT | `PurgeAndRebuildEntityStateLists` → `BroadcastEntityActivation` sends ENABLE_ENTITIES (base method 1) |
| ENABLE_ENTITIES: 8-byte SGW custom payload | UNVERIFIED | `BroadcastEntityActivation` reads `DAT_01ef2500->size`; cannot read at static analysis time |
| CREATE_CELL_PLAYER rotation: X, Z, Y (Y/Z swapped) | CONFIRMED | `FUN_015846a0` rotation reader; `client_handler.cpp` pattern |
| `classId 0x02 = SGWPlayer` | CONSISTENT | `ServerConnection_CreateBasePlayer` reads `typeId u16`; class 2 = SGWPlayer per entities.xml |
| FORCED_POSITION CONSTANT_LENGTH = 49 | CONSISTENT | `ServerConnection_ForcedPosition` @ `0x00dd9ee0` (entity-creation-wire-formats.md) |

---

## Open Questions

1. ~~**ENABLE_ENTITIES payload size**~~: **CLOSED** (W-enable-entities, 2026-05-13, correcting W-misc-gaps).
   ENABLE_ENTITIES carries **8 bytes** (SGW-custom `uint64` dummy). The static initializer
   at `0x017bade0`–`0x017bae07` pushes `0x8` as the size argument (`0x017bade9: PUSH 0x8`)
   to the InterfaceElement constructor `0x015785c0`. The `resetEntities` initializer at
   `0x017bb200`–`0x017bb225` pushes `0x1` (its known 1-byte size) at the same stack position,
   confirming the push semantics. Corroborated by `deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp:83`:
   `{Message::CONSTANT_LENGTH, 8, "ENABLE_ENTITIES", true}`.

   W-misc-gaps misread the `MOV DWORD PTR [EAX], 0x1` at `0x017badf7` as writing the size.
   That instruction writes a reliability/type flag into the stack struct passed by pointer —
   a distinct field from the size push. The `docs/world-entry-phases.md` 8-byte claim is
   **CORRECT** and should be restored. The bigworld-engine-advisor memory claim (8 bytes, SGW;
   1 byte, stock BW) is confirmed accurate.

2. ~~**GameProxyPlayer `Event_Level_PostLoad` handler body**~~: **CLOSED** (W-misc-gaps, 2026-05-13).
   The primary handler is `GameProxyPlayer_HandleEvent_Level_PostLoad` (`0x00de8660`) — a
   one-instruction wrapper over `FUN_00de8430` (0x00de8430). Registered via `FUN_00df4270`
   (the main GameProxyPlayer callback registration). A second conditional subscription uses
   handler `LAB_00de9e60` (only on account disconnect path, `FUN_00def710`).

   **`FUN_00de8430` body**:
   - If `param_1+0xC` (mPlayerController) already non-null: no-op.
   - Otherwise: reads UE3 PlayerController from global engine singleton `DAT_01ee1254+0x2D0/+0x40`.
   - Sets `*(pPlayerController+0x5A) = 2` — input mode (full game input).
   - If `param_1+8` (vehicle/mount entity) exists: copies 24 bytes of transform data
     (offsets `+0xDC`, `+0xE4`, `+0xE8`, `+0xF0`) from PlayerController to vehicle.
   - If `param_1+4` (another entity) exists: same transform copy.
   - Flag `DAT_01eb082c`: if set, skips transform copy (editor/replay path).

3. ~~**`Event_World_Loaded` emitter**~~: **CLOSED** (W-misc-gaps, 2026-05-13).
   Emitter is `FUN_005541a0` (0x005541a0), called from `FUN_007100d0` (0x007100d0).
   `FUN_007100d0` reads the global `WorldInfo*` from `DAT_01ee2684` and passes it to the
   emitter. It has no static callers — it is invoked via a UE3 streaming completion callback
   (likely `UEngine::NotifyLevelStreamingStateChanged` or equivalent callback table).

   **`FUN_005541a0` conditions for emit**:
   - `DAT_01ee2b6c == 0` — "world-loaded already fired" guard (returns 0 if set).
   - All sub-levels in `param_1+0x50` → `+0x3C` → `+0x264` (level count) have no bit-2
     set (pending stream flag).
   - All entities at `param_1+0x44` (entity list, `+0x48` entries) have `+0x164 == 0`.
   - On success: allocates 12-byte `Event_World_Loaded` struct, emits via CME.

   This is distinct from `Event_Level_PostLoad` (fired per individual level by
   `EntityManager_PostLoadMap` at `0x00dd0b00`). `Event_World_Loaded` fires once when the
   entire streaming world is settled.

4. ~~**SGWNetworkManager `ClientReady` handler body**~~: **CLOSED** (W-misc-gaps, 2026-05-13).
   Handler is `SGWNetworkManager_EventHandler_ClientReady_invoke` (`0x00d43dc0`), created
   by `FUN_00d57030` (0x00d57030). Registration chain:
   `FUN_00d57030` → `FUN_00d4d540` → MemberCallback ctor `FUN_00d45b70` → subscribe.

   **Handler body** (3 instructions):
   ```c
   pArgData = *(void**)((int)this + 8);      // stored at construction time
   pMethodDesc = *(void**)((int)this + 4);   // stored at construction time
   EnsureEntityRpcRegistryAllocated();
   RouteOutgoingEntityRpc(param_2, pMethodDesc, pArgData);
   ```
   Routes the ClientReady event as an outgoing Mercury entity RPC. `pMethodDesc` = the
   `enableEntities` method descriptor; `pArgData` = serialized args (the 1-byte keepBase
   field). `RouteOutgoingEntityRpc` serializes to the Mercury bundle and flushes.

5. **`resetEntities` CONSTANT_LENGTH**: Does the client parse a 1-byte `keepBase` field?
   `PurgeAndRebuildEntityStateLists` does not reveal stream layout. The `pSpaceData`
   parameter determines branch behavior but its origin is unclear.

6. **`setupStargateInfo` handler** (method index 65): The string "setupStargateInfo" does NOT
   appear anywhere in SGW.exe — it is a server-side method name only. The client handles it
   as a numeric Mercury method index (65 / Extended `0xBD sub=4`). The client-side handler
   address was not located in this session. Candidate subsystems: a `StargateManager` class
   (no RTTI found with that name) or `GameWorldConstants`. The `DBGateInfo` CookedData type
   (RTTI accessor `0x004288e0`) may be the data carrier. Requires tracing the Extended method
   dispatch table from the Mercury layer.

---

## Address Map Additions

```
# World Entry — function addresses
0x00dd9280   BroadcastEntityActivation            sends ENABLE_ENTITIES; sets bEntitiesEnabled @ +0x316
0x00dda0e0   PurgeAndRebuildEntityStateLists      resetEntities handler; resets entity lists; calls BroadcastEntityActivation
0x00dd0b00   EntityManager_PostLoadMap            UE3 CALLBACK_PostLoadMap (0x32) → fires Event_Level_PostLoad
0x00df27f0   GameProxyPlayer_HandleOnClientMapLoad handler for onClientMapLoad (method 117); actual fields: areaName, mapPath, WorldID, Location, Direction
0x00c71a20   GameWorldConstants_HandleSetupWorldParameters  setupWorldParameters handler (method 122); loads BW_TO_UE3_SCALE=100.0f
0x00dddca0   ServerConnection_CreateBasePlayer    CREATE_BASE_PLAYER (0x05); reads entityId u32 + typeId u16
0x00dda2e0   ServerConnection_CreateCellPlayer    CREATE_CELL_PLAYER (0x06); 32-byte payload; Y/Z rotation swap
0x00dda6c0   ServerConnection_SpaceViewportInfo   SPACE_VIEWPORT_INFO (0x08); CONSTANT_LENGTH=13
0x00dd9ee0   ServerConnection_ForcedPosition      FORCED_POSITION (0x31); CONSTANT_LENGTH=49
0x00de04c0   register_NetIn_AccountLoginSuccess   Pattern A subscription for AccountLoginSuccess
0x00ddfd00   register_NetIn_ServerSelectSuccess   Pattern A subscription for ServerSelectSuccess
0x00dde8d0   register_NetIn_LoginFailure          Pattern A subscription for LoginFailure
0x00d93d80   register_NetOut_ClientReady          registration shim for Event_NetOut_ClientReady
0x00d45be0   MemberCallbackRtti_ClientReady__SGWNetworkManager  RTTI accessor for SGWNetworkManager+ClientReady
0x00c6ed70   VoiceHandlerGated_ActivateEntitiesAndSetServerConnection  secondary activation path (voice-handler gated)
0x00de8670   GameProxyPlayer_HandlePlayerPawnCreated  Event_Player_PawnCreated handler; sets up HUD actors
0x00449b20   Event_NetOut_versionInfoRequest_vfunc_3  versionInfoRequest signal emitter
0x00df7b80   CME RTTI: GameProxyPlayer + Event_World_Loaded subscriber
0x00df6e80   CME RTTI: GameProxyPlayer + Event_Level_PostLoad subscriber
0x00e9a480   CME RTTI: GameAppearanceManager + Event_Level_PostLoad subscriber
0x00e2af30   CME RTTI: Minimap + Event_World_Loaded subscriber

# World Entry — addresses added by W-misc-gaps (2026-05-13)
0x00de8660   GameProxyPlayer_HandleEvent_Level_PostLoad  primary PostLoad handler (wrapper)
0x00de8430   FUN_00de8430                PlayerController assignment + transform copy (PostLoad body)
0x00de9e60   LAB_00de9e60               alternate PostLoad handler (account-disconnect code path)
0x00def710   FUN_00def710               account disconnect fn; conditionally registers PostLoad
0x00df4270   FUN_00df4270               main GameProxyPlayer callback registration fn
0x005541a0   FUN_005541a0               Event_World_Loaded emitter (fires after all sub-levels ready)
0x007100d0   FUN_007100d0               Event_World_Loaded trigger thunk (no static callers)
0x00d43dc0   SGWNetworkManager_EventHandler_ClientReady_invoke  ClientReady wire-send handler
0x00d57030   FUN_00d57030               SGWNetworkManager::EventHandler<ClientReady> ctor
0x00d45b70   FUN_00d45b70               MemberCallback<ClientReady> ctor
0x00d4d540   FUN_00d4d540               MemberCallback<ClientReady> subscribe wrapper

# World Entry — data addresses
DAT_01ef2500  ENABLE_ENTITIES message descriptor (CONSTANT_LENGTH = 8, SGW-custom uint64 dummy)
DAT_01ee2b6c  "world loaded guard" flag — set after Event_World_Loaded first fires
DAT_01ee2684  global WorldInfo* pointer — read by Event_World_Loaded trigger thunk
DAT_018cad90  BW_TO_UE3_SCALE = 0x42C80000 = 100.0f
DAT_01eb082c  editor/replay mode flag — if set, PostLoad skips transform copy
0x017bade9    ENABLE_ENTITIES descriptor init site — `PUSH 0x8` (size arg to InterfaceElement ctor 0x015785c0)
0x017bae02    ENABLE_ENTITIES descriptor ptr stored — `MOV [0x01ef2500], EAX`
0x019c2828    "onClientReady" signal name string
0x019cf548    "ServerConnection::enableEntities: Enabling entities %d\n" debug log
0x019d09f8    "resetEntities" method name string
0x019d26c8    "areaName" assert string (onClientMapLoad field name)
0x019d26d0    "mapPath" assert string (onClientMapLoad field name)
0x019d2684    "WorldID" assert string (onClientMapLoad field name)
0x017bb210    resetEntities message table registration site
0x017bb1c7    PurgeAndRebuildEntityStateLists function pointer write site
```

---

## ENABLE_ENTITIES Payload Reconciliation

> **Investigated by**: W-enable-entities, 2026-05-13
> **Verdict**: **8 bytes** (SGW-custom `uint64` dummy). Binary is definitive.

### The Contradiction

Two sources disagreed:

| Source | Claim |
|--------|-------|
| `bigworld-engine-advisor` persistent memory | SGW = 8 bytes (`uint64` dummy); stock BW = 1 byte (`uint8` dummy) |
| W-misc-gaps Session 5 (2026-05-13) | 1 byte (stock BW), "8-byte claim is incorrect" |

### Binary Evidence

The ENABLE_ENTITIES message descriptor stored at `DAT_01ef2500` is populated by a static
initializer block. Disassembly of that block via `mcp__ghidra__get_assembly_context` at
`0x017bae02` (the store instruction) reveals the construction sequence:

```asm
017bade0: PUSH -0x1           ; SEH cookie
017bade2: PUSH 0x1            ; reliability flag
017bade4: PUSH ECX            ; interface table ptr
017bade5: MOV EAX, ESP        ; EAX = &stack_struct
017bade7: PUSH 0x0            ; param: flags
017bade9: PUSH 0x8            ; ← SIZE = 8 bytes   <-- THE KEY PUSH
017badeb: PUSH 0x0            ; param: type
017baded: PUSH 0x19d092c      ; ← "enableEntities" string
017badf2: MOV ECX, 0x1ef24cc  ; interface registry
017badf7: MOV DWORD PTR [EAX], 0x1  ; sets reliability byte in stack struct
017badfd: CALL 0x015785c0     ; InterfaceElement ctor/registrar
017bae02: MOV [0x01ef2500], EAX ; store descriptor ptr
```

The string at `0x019d092c` reads `"enableEntities"` — confirmed by memory inspection.

**Calibration against `resetEntities`** (known 1-byte payload): the `resetEntities`
initializer at `0x017bb200`–`0x017bb225` uses the identical push pattern with `PUSH 0x1`
at `0x017bb20c` (the size position). Since `resetEntities` is documented and confirmed as
1-byte `keepBase`, the push at that stack position IS the size field. Therefore `PUSH 0x8`
for `enableEntities` means `CONSTANT_LENGTH = 8`.

### Where W-misc-gaps Went Wrong

W-misc-gaps reported address `0x017bae02` as `"MOV DWORD PTR [struct], 1"` and concluded
the size field was 1. That address is the store-result instruction (`MOV [0x01ef2500], EAX`)
— it stores the constructed descriptor pointer, not a size value. The `MOV DWORD PTR [EAX], 0x1`
at `0x017badf7` writes a reliability flag into the stack-allocated argument struct, not the
message size. W-misc-gaps did not retrieve the full disassembly context and reported only
the one instruction visible in cross-reference output.

### C++ Source Corroboration

`deprecated/cpp/src/baseapp/mercury/sgw/messages.cpp`, line 83:
```cpp
{Message::CONSTANT_LENGTH, 8, "ENABLE_ENTITIES", true},
```

This SGW-custom message table defines `CONSTANT_LENGTH = 8`. The comment above it
(`uint64 Dummy`) names the payload field.

Stock BigWorld (`external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp`):
```cpp
MF_BEGIN_BLOCKABLE_PROXY_MSG( enableEntities )
    uint8   dummy;
END_STRUCT_MESSAGE();
```

Stock BW = 1 byte (`uint8`). SGW = 8 bytes (`uint64`). The difference is a deliberate
SGW customization, not an error in the bigworld-engine-advisor memory.

### Definitive Answer

| Direction | Message | Size | Field |
|-----------|---------|------|-------|
| Client → server | `enableEntities` (base method index 1) | **8 bytes** | `uint64 dummy = 0` |

The `bigworld-engine-advisor` memory claim is confirmed correct. W-misc-gaps reached a wrong
conclusion through incomplete disassembly context. The `world-entry-pipeline.md` Open Question
1 CLOSED entry (written by W-misc-gaps) has been corrected in place above.
