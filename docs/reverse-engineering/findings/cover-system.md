# Cover System — Client Binary Analysis

> **Last updated**: 2026-05-13
> **Source**: SGW.exe Ghidra decompilation, V5 Documentation Campaign session 4 (W-cover)
> **Confidence**: HIGH — decompiled constructors, destructors, event handlers, loader, spatial-tree query with debug strings and source paths
> **Issue**: [#209](https://github.com/SandboxServers/Cimmeria/issues/209) — NPC AI ignores cover system

---

## Summary

The cover system is a fully server-driven spatial AI feature. The client holds 1,332 `SGWCoverSet` entity chunks in `entities/defs/` (confirmed by issue #209), each referencing UE3 `ACoverLink` actors that embed cover slot metadata. NPCs select cover via server-side logic and announce movement to cover using `aMovementType = 0` (CoverAdvance). The client renders NPC-in-cover posture via `USGWAnim_BlendByCover` and exposes a `CoverInfo` client object that tracks nearby cover node quality for the locally-controlled player. A `SGWCoverSet` server entity owns the spatial reservation state for all cover nodes in a chunk.

---

## Cover Entity Types

### Server-Side Entity: `SGWCoverSet`

Defined in `entities/defs/SGWCoverSet.def`. **ServerOnly** — never streamed to the client as an entity object.

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `chunkName` | STRING | CELL_PRIVATE | Name of the UE3 chunk this cover set belongs to |
| `chunkID` | UINT32 | CELL_PUBLIC | Numeric chunk identifier |
| `proximityID` | CONTROLLER_ID | CELL_PRIVATE | BigWorld proximity controller watching for entities in range |
| `reservedCoverNodes` | PYTHON dict | CELL_PRIVATE | `{nodeID -> [(slot, entityID)]}` — authoritative reservation map |
| `publicReservationData` | ARRAY of `PublicCoverNodeReservationData` | CELL_PUBLIC | Broadcast reservation state: `[{nodeID, slotID, entityID}]` |
| `proximityRetries` | INT8 | CELL_PRIVATE | Default 10 — retry count for proximity controller setup |

**Cell Methods:**

```
reserveCoverSlot(entityID: INT32, nodeID: INT32, slotID: INT8)
releaseCoverSlot(entityID: INT32, nodeID: INT32, slotID: INT8)
```

`reserveCoverSlot` automatically releases any slot already held by `entityID` before granting the new one (confirmed by .def comment: "this will automatically release any slots already reserved by entity").

`publicReservationData` is the client-visible projection of the reservation state. When the server updates `reservedCoverNodes`, it must also update `publicReservationData` so nearby clients know which slots are occupied.

**Evidence**: `entities/defs/SGWCoverSet.def` (line 35 comment); `EntityDescription_ParseProperties` at `0x015924a0` references `publicReservationData` string at `0x01b1a5f4`.

---

### UE3 Client-Side Classes (placed in levels, not BigWorld entities)

These are Unreal Engine 3 AActor subclasses placed in level geometry, not BigWorld network entities.

| Class | RTTI | Source | Purpose |
|-------|------|--------|---------|
| `ACoverLink` | `0x01dc6c7c` | `CoverLink.cpp` | Individual cover node with 1+ slots. Extends `ANavigationPoint`. |
| `CoverGroup` | `0x01dc6c60` | — | Named group of `ACoverLink`s that can be toggled on/off as a unit |
| `CoverSlotMarker` | `0x01dc6c40` | — | Marks a single cover position within an `ACoverLink` |
| `ASGWSpecCoverNode` | `0x01dd5014` | `SGWSpecCoverNode.cpp` | SGW-specific cover node editor actor |
| `USGWCoverNodeComponent` | `0x01dd5dfc` | `SGWCoverNodeComponent.cpp` | UE3 component managing the per-chunk cover node prefab set |
| `USGWAnim_BlendByCover` | `0x019e557c` | `SGWAnim_BlendByCover.cpp` | Animation blend node: switches NPC pose between stand/crouch/prone based on cover state |
| `UCoverSlipReachSpec` | `0x01dc8564` | — | Navigation reach spec for lateral cover-slip movement |

**ACoverLink slot layout** (stride 0x9c per slot, base at `this+0x28c`, count at `this+0x290`):

- `+0x20`: DefinedPaths array pointer, `+0x24`: DefinedPaths count, stride 0x38
- `+0x44`: FireLinks array, `+0x48`: count, stride 0x1c
- `+0x50`: ClaimedBy entity (0 = unclaimed)
- `+0x54..+0x60`: FireLink target actor references (4 pointers)
- `+0x6c`: FireLinks2 array, `+0x70`: count, stride 0x1c
- `+0x84`: ExposedFireLinks array, `+0x88`: count, stride 0x1c
- `+0x9a`: flags bitmask (bit 0-2: slot type)

**Evidence**: `ACoverLink__vfunc_183` at `0x00704be0` (slot enumeration); `ACoverLink__vfunc_205` at `0x00700800` (HasFireLinkTo).

---

### Server-Side Spatial Index: CoverSpace

The server (BigWorld Python cell) exposes a `CoverSpace` spatial tree for cover node lookup. The tree is per-chunk and queried via:

```python
getCoverCount(chunkID)      # number of nodes in chunk
getCoverNode(chunkID, idx)  # returns node handle
getCoverAngles(chunkID, idx) # returns [defAngle, offAngle]
getUnitCoverHeight(chunkID, idx) # returns CoverHeight enum value
updateCover(chunkID, param) # rebuild tree for chunk
```

Debug string at `0x01b27700` shows the tree stats format:
```
CoverSpace TreeStats: Chunk (%u) Time (%d sec %d ms) Nodes (%u) K (%d) MaxDepth (%u) MaxCoverNodes (%u) Branches (%u) Leaves (%u) Overlap (%3.2f)
```

These are Python-exposed C extension functions. The client side counterpart is `CoverInfo::UpdateCover` at `0x00e71710` which queries `FUN_01608820` (CoverSpace factory) → `FUN_0160c3b0` (get chunk tree) → `FUN_01609760` (query candidates at position).

**Evidence**: Lua binding `FUN_00aa0d10` at `0x00aa0d10`; Python error strings at `0x0193fa88`–`0x0193fbb8`; Python function name table at `0x01955658`–`0x019556d8`.

---

## Cover Discovery Algorithm

### How NPCs Find Candidate Cover Nodes

The server is entirely responsible for NPC cover selection. The client only receives the result via `aMovementType = 0` + `aPath`.

**Server-side algorithm (inferred from Python function names and CoverSpace tree structure):**

1. NPC enters combat and AI state machine evaluates cover (behavior event triggers).
2. Server calls `getCoverCount(chunkID)` to enumerate available nodes near the NPC's chunk.
3. For each node, server checks:
   - `reservedCoverNodes[nodeID]` — is any slot already claimed?
   - `getCoverAngles(chunkID, idx)` — are the defensive/offensive angles favorable relative to the enemy?
   - Distance from NPC to node (filtered by `aDistanceWeight`).
   - `getUnitCoverHeight(chunkID, idx)` — is the cover tall enough for the NPC model?
4. Server scores nodes by weighted sum: `aDefCoverWeight × defAngle + aOffCoverWeight × offAngle + aCoverWeight × cover + aMoveWeight × moveScore + aCrossPathWeight × crossPathScore + aDistanceWeight × distScore`.
5. Best scoring unoccupied node + slot is selected.
6. Server calls `SGWCoverSet.reserveCoverSlot(npcEntityID, nodeID, slotID)` to claim it.
7. Server updates `publicReservationData` so clients know the slot is occupied.
8. Server sends `aMovementType=0` + `aPath=[waypoints to node]` + `aEntityId` to the client via the existing movement protocol.

**Evidence**: Cover weight fields confirmed in `SGWTextCommandMgr_OnChangeCoverWeight` at `0x00c87430`; weight field names: `aDistanceWeight`, `aDefCoverWeight`, `aOffCoverWeight`, `aMoveWeight`, `aCrossPathWeight`, `aCoverWeight` set via `FUN_00cb1d40` (SetField wrapper).

---

## Cover Selection Criteria

### Disqualification Conditions (confirmed from binary)

1. **Already occupied**: `reservedCoverNodes[nodeID]` has any entry for this slot — enforced by `SGWCoverSet.reserveCoverSlot` which reads the dict.
2. **Out of chunk**: Cover nodes are per-chunk; NPC must be within `proximityID` range for the `SGWCoverSet` to fire proximity events.
3. **Wrong height**: `getUnitCoverHeight` returns 0–3 (4 enum values, switch at `0x00904d80` maps to float constants at `DAT_018f41d4`/`d0`/`cc`/`c8`). Server can filter by minimum height.
4. **Line of sight** (strong inference): `ACoverLink__vfunc_205` / `HasFireLinkTo` at `0x00700800` provides LOS checks between slots. The cover quality system (`aDefCoverWeight`, `aOffCoverWeight`) encodes LOS quality — higher weight = better cover from that angle.
5. **CoverGroup disabled**: `ACoverGroup::ToggleGroup` / `EnableGroup` / `DisableGroup` UScript methods (at `0x0181f04c`–`0x0181f0c8`) can disable entire groups of cover nodes. Server should respect the enabled state.

**Evidence**: `ACoverLink__vfunc_205` at `0x00700800`; `intACoverGroupexecToggleGroup` string at `0x0181f04c`; `reserveCoverSlot` .def comment.

---

## Cover Occupancy and Claim Mechanism

### Wire Protocol (Server → Client)

The client has no direct cover-claim message. The reservation state is surfaced through two channels:

1. **`publicReservationData` property update** — `SGWCoverSet` is `CELL_PUBLIC`, so BigWorld streams `publicReservationData` updates to nearby clients when the array changes. Each element is a fixed-dict `{nodeID: INT32, slotID: INT8, entityID: INT32}`. Client receives this as a standard BigWorld property update on the `SGWCoverSet` entity. **However, because `SGWCoverSet` is `ServerOnly`, this property update goes to other server-side entities, not to game clients directly.** The `CELL_PUBLIC` flag means other cell entities (not game clients) can read it.

2. **Movement type 0 (CoverAdvance)** — confirmed at `0x019d2ca4`: `"Entity: %d is moving to cover"`. When the NPC moves to cover, the client gets the movement update which visually positions the NPC at the cover node. The `USGWAnim_BlendByCover` node then blends the animation based on cover height.

**Key finding**: `SGWCoverSet` is `ServerOnly` — game clients never receive the entity or its properties directly. The client visualizes NPC-in-cover state purely from movement type 0 + the NPC's final position at the cover node.

### CoverInfo Client Object (player-side only)

`CoverInfo` (dtor at `0x00e73280`, impl at `0x00e71c30`) is a client-side helper object attached to the local player entity. It:
- Subscribes to `Event_Entity_ProxyPlayerCellCreated` (slot `0x30` on `FCallbackEventDevice`) and `Event_Entity_Destroyed` (slot `0x2f`).
- Subscribes to `Event_Player_EntityControl` via a MemberCallback.
- Maintains a position cache and a `CoverSpace` query result.
- On `updateCover()` call: if the player has moved beyond threshold `DAT_018fde24`, queries the server-side CoverSpace at the player's position and returns a sorted list of cover candidates.

This is a **player-facing cover UI aid**, not NPC AI. It powers the cover quality HUD (`CoverQRModifier` property at `0x01956eac` — displayed when the player is in cover for QR/combat modifier display).

**Evidence**: `CoverInfo_Dtor` at `0x00e71c30` (FCallbackEventDevice slot unsubscription); `CoverInfo_UpdateCover` at `0x00e71710` (position delta check, CoverSpace query).

---

## Cover Weight Events (Client → Server)

Three `Event_NetOut_*` signals exist for GM/dev tools to tune cover weights at runtime:

### `Event_NetOut_ChangeCoverWeight` (confirmed handler: `0x00c87430`)

Fired by `/changecoverweight` slash command. Wire payload (0x0C byte NetworkEvent, Pattern B ctor):

| Field | Type | Source arg |
|-------|------|-----------|
| `aDistanceWeight` | FLOAT | `distance` |
| `aDefCoverWeight` | FLOAT | `defCover` |
| `aOffCoverWeight` | FLOAT | `offCover` |
| `aMoveWeight` | FLOAT | `move` |
| `aCrossPathWeight` | FLOAT | `crossPath` |
| `aCoverWeight` | FLOAT | `cover` |

**Source**: `SGWTextCommandManager.cpp` lines `0xB15`–`0xB1A` (confirmed by assert strings).

### `Event_NetOut_ChangeCoverStanceWeight` (confirmed handler: `0x00c87d00`)

Fired by `/changecoverstanceweight`. Same 6 float fields plus:

| Field | Type | Source arg |
|-------|------|-----------|
| `aStanceName` | WSTRING | `stance` |

**Source**: `SGWTextCommandManager.cpp` lines `0xB32`–`0xB38`.

### `Event_NetOut_RegenerateCoverLinks` (registered at `0x00cbc8f0`)

Fires a server-side rebuild of the cover link graph. No payload fields beyond the NetworkEvent base. Used after geometry edits.

**Evidence**: All three registered in `RegisterBulkNetOutSignals` at `0x00db3390`; event registration strings at `0x019b3ff4`, `0x019b402c`, `0x019b4060`.

---

## Client Visualisation

### Animation: `USGWAnim_BlendByCover`

Source: `SGWAnim_BlendByCover.cpp` (string at `0x019e55a8`). This UE3 animation blend node selects an NPC pose based on the cover height enum (0=stand, 1=crouch, 2=prone, 3=wall-hug inferred from 4-case enum).

**How it fires**: When a NPC reaches the cover position (movement type 0 completes), the NPC's skeletal mesh animation tree evaluates `USGWAnim_BlendByCover` which reads the cover height from the actor's cover state. No explicit server message is needed — the position at the cover node is sufficient.

### HUD: CoverQRModifier

Property `CoverQRModifier` (string at `0x01956eac`, Lua type registered in `FUN_00ab1860`). Displayed when the player is in cover. This is a combat Quick-Response modifier — being in cover improves the player's QR value. The property is set server-side and received via standard property update.

### Debug Commands

| Slash Command | Event | Purpose |
|--------------|-------|---------|
| `/showcover` | `Event_SlashCmd_ShowCover` | Toggle cover node debug rendering |
| `/regeneratecoverlinks` | `Event_SlashCmd_RegenerateCoverLinks` | Rebuild cover link graph |
| `/changecoverweight` | `Event_SlashCmd_ChangeCoverWeight` | Tune cover weight parameters |
| `/changecoverstanceweight` | `Event_SlashCmd_ChangeCoverStanceWeight` | Tune per-stance weights |

**Evidence**: Event strings at `0x018424d0`–`0x018426a8`; editor mode `"MODE COVEREDIT"` at `0x01978a8c` / `0x01a3e1c4`.

---

## Cover Node Data Layout

Cover nodes are loaded from `covernodes_local.pak` (string at `0x017f9bb8`) as a `MyCoverNodeArchive` (string at `0x017f9b80`). The XML format uses element names `coverSet`, `coverNodes`, `coverNode` with property fields `coverheight`, `coverquality`, `coverwidth`. These map to the SGW-specific `SGWCoverSet` Python entity.

**CoverNodePrefabData struct** (0x18 bytes, confirmed by loop stride in `USGWCoverNodeComponent_SpawnCoverNode` at `0x00904d80`):

```
+0x00  float  position.x  (BigWorld meters)
+0x04  float  position.y
+0x08  float  position.z
+0x0C  float  orientation  (radians, converted to UE3 rotation units by * DAT_0181998c * _DAT_018199e0)
+0x10  u8     coverHeight  (0=Low/Stand, 1=Mid/Crouch, 2=High/Prone, 3=Wall — maps to float at DAT_018f41d4/d0/cc/c8)
+0x14  u8     quality      (cover quality score)
+0x15  u8     width        (cover width)
+0x16  u8     flags        (bit 0 = hasCoverAbove, bit 1 = hasLeanLeft/Right)
```

**Evidence**: `CoverNodeXmlLoader` at `0x010556a0`; `USGWCoverNodeComponent_SpawnCoverNode` at `0x00904d80`; string `"SGW_Cover.CoverNode"` at `0x018f3f1c` (static mesh reference).

---

## Recommended Rust Implementation Approach for Issue #209

### Minimal Viable Cover (MVC) — what the client needs to see NPCs use cover

The client only needs two things to show NPCs using cover correctly:

1. **Movement type 0 (CoverAdvance)** must be sent when an NPC moves to a cover position. This is already partially implemented (the movement type enum is wired per `npc-ai-state-machine.md`). The missing piece is the server actually *selecting* a cover node and pathing to it.

2. **Final position at a real cover node** — the NPC must arrive at a valid `ACoverLink` slot position so `USGWAnim_BlendByCover` picks the correct pose. The position is in BW meters (divide by 100 for UE3 cm).

### Implementation Plan

#### Step 1 — Load Cover Node Data (no wire changes)

The `SGWCoverSet` entities exist in `entities/defs/`. The server must:
- Load `covernodes_local.pak` per chunk (or equivalent database representation) during cell entity creation.
- Build an in-memory spatial index (a k-d tree or simple distance-sorted list suffices initially) of `(nodeID, position, coverHeight, slots[])`.

#### Step 2 — SGWCoverSet Python Entity (new)

Implement `SGWCoverSet` cell entity in Python (or Rust cell entity if the architecture supports it) with:
- `reserveCoverSlot(entityID, nodeID, slotID)` — mark slot as occupied, update `publicReservationData`.
- `releaseCoverSlot(entityID, nodeID, slotID)` — clear slot.
- Proximity controller to fire events when entities enter/leave the cover set's area.

#### Step 3 — NPC AI Cover Selection (Rust cell logic)

When an NPC enters combat state:
1. Query the nearest `SGWCoverSet` for the NPC's chunk.
2. Call (conceptually) `getCoverCount` and `getCoverNode` to get candidates.
3. Filter: skip occupied nodes (`reservedCoverNodes` check), skip nodes facing wrong direction relative to enemy, skip nodes too far away.
4. Score remaining nodes: apply `aDefCoverWeight`, `aMoveWeight`, `aDistanceWeight` defaults.
5. Select best node and slot.
6. Call `reserveCoverSlot` on the owning `SGWCoverSet`.
7. Path the NPC to the node position.
8. Send `aMovementType=0` + `aPath=[waypoints]` to the client.

#### Step 4 — Cover Release

When NPC dies, leaves combat, or is displaced:
- Call `releaseCoverSlot`.
- Update `publicReservationData`.
- Send new movement type (e.g., type 2 leash-back).

#### Step 5 — CoverQRModifier (player-facing)

When the player reaches a cover node, send `CoverQRModifier` property update. This is a separate enhancement, not needed for NPC cover to work visually.

### Default Cover Weights

From `Event_NetOut_ChangeCoverWeight` field mapping:
- Start with: `aDefCoverWeight=1.0, aOffCoverWeight=0.5, aCoverWeight=1.0, aMoveWeight=0.3, aCrossPathWeight=0.2, aDistanceWeight=0.5`
- Tune once NPCs are moving to cover and the visual behavior is observable.

### Not Required for #209

- `CoverInfo` client object (player HUD aid) — server-side NPC AI works without it.
- `Event_NetOut_RegenerateCoverLinks` handler — needed only for editors.
- `Event_NetOut_ChangeCoverWeight` server handler — needed only for live tuning.
- `USGWAnim_BlendByCover` — this is pure client-side UE3, no server work needed.

---

## Open Questions

1. **`PublicCoverNodeReservationData` type definition** — the fixed-dict schema is referenced in `SGWCoverSet.def` but the concrete type definition is not in any `.def` or `.py` file in the repo. The server-side type must be defined somewhere (possibly in a `aliases.xml` or `user_data_object_defs/`). **Evidence needed**: locate the `PublicCoverNodeReservationData` type in the BigWorld entity definition files.

2. **Cover node chunk boundary behavior** — what happens when an NPC crosses a chunk boundary mid-cover? Does the `SGWCoverSet` proximity controller fire and does the NPC need to release the slot from the old chunk's set and claim from the new? **Evidence needed**: trace `proximityID` controller firing logic.

3. **CoverHeight enum exact values** — the 4 float constants at `DAT_018f41d4`/`d0`/`cc`/`c8` are referenced in `USGWCoverNodeComponent_SpawnCoverNode` but their actual float values were not read in this session. **Evidence needed**: `mcp__ghidra__read_memory` at those 4 addresses to get actual height values.

4. **`aMovementType=0` path format** — the `aPath` field for CoverAdvance: does it contain the full nav path (multiple waypoints) or just the destination? The existing movement handler at `0x00deb660` (timed out) would confirm. **Evidence needed**: retry decompile with longer timeout or read raw disassembly.

5. **`SGWAnim_BlendByCover` cover state trigger** — how does the animation node know the NPC is "in cover" vs "moving to cover"? Is there a `bStateField` bit, or does it read the actor's current cover link reference? **Evidence needed**: decompile `USGWAnim_BlendByCover` methods beyond the scalar destructor stub.

6. **1,332 Atrea cover nodes** — are these the full set of cover nodes or a subset? Is there a separate Atrea vs. SGW cover node format? **Evidence needed**: examine the content of `covernodes_local.pak` or equivalent DB entries.

---

## Cross-Reference Targets

- `docs/reverse-engineering/findings/npc-ai-state-machine.md` — update "Cover System" section with `CoverInfo` client object details and `publicReservationData` CELL_PUBLIC note.
- `docs/reverse-engineering/address-map.md` — add "Cover system" subsection (see companion update below).
- `entities/defs/SGWCoverSet.def` — no change needed; correctly describes the entity.
- `docs/content/mission-chains.md` — no cover-specific missions identified in this session.
- `docs/protocol/` — no new client→server messages; cover claim uses existing BigWorld property update protocol via `publicReservationData`. The 3 `Event_NetOut_Cover*` events should be catalogued if not already present.
