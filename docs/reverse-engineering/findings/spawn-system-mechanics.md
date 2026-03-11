# Spawn System Mechanics — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation
> **Confidence**: HIGH — decompiled class hierarchy, serialization, editor tools, RTTI

---

## Key Finding: Spawning Is Server-Driven

The client binary contains only **editor placement tools and visualization code**. No spawn logic (timers, counts, conditions, respawn delays) exists in the client. No strings for `RespawnTime`, `RespawnDelay`, `SpawnRate`, `SpawnCount`, `MaxSpawns`, `SpawnCondition`, `SpawnFilter`, `MinLevel`, `MaxLevel`, `SpawnWeight`, or `SpawnChance` were found.

**The server must implement all spawn behavior logic.**

### Name Correction

The Python server stubs are named `SGWSpawnRegion` and `SGWSpawnSet` — the actual UE3 class names are:
- **`ASGWSpecSpawnSet`** — region containing spawn points (maps to `SGWSpawnSet`)
- **`ASGWSpecSpawnPoint`** — individual spawn location (maps to `SGWSpawnRegion` points)

---

## Class Hierarchy

### Spawn Set (Region-based)
```
AActor
  → ASGWRegion           (0x1B4 bytes)
    → ASGWSpecRegion     (0x1D0 bytes)
      → ASGWSpecSpawnSet (0x1D4 bytes)
```

### Spawn Point (Actor-based)
```
AActor
  → ASGWSpecActor        (0x1D0 bytes)
    → ASGWSpecSpawnPoint (0x1FC bytes)
```

### Sibling Region Types (all in SGWSpecRegion.cpp)

| Class | Size | Purpose |
|-------|------|---------|
| SGWSpecCircleRegion | 0x1D0 | Circular area |
| SGWSpecBoxRegion | 0x1D0 | Rectangular area |
| SGWSpecWayPoints | 0x1D0 | Path/waypoint chain |
| SGWSpecSpawnSet | 0x1D4 | Spawn region with child points |
| SGWSpecMeshRegion | 0x1C0 | Mesh-defined area |

---

## Spawn Point Data Structure (UNRPC serialization)

### `unrpc:SpawnPointCreateData` (type 0x0d, 0x20 bytes)

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x04 | spawnSetName | string | Parent spawn set name |
| +0x08 | spawnTableName | string | Mob/loot table reference |
| +0x0c | x | float | World position X |
| +0x10 | y | float | World position Y |
| +0x14 | z | float | World position Z |
| +0x18 | orientation | float | Facing direction (radians) |

### `unrpc:SpawnPoint` (type 0x0c, 0x2c bytes)

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x04 | id | int64 | Unique spawn point ID |
| +0x0c | spawnSetName | string | Parent spawn set name |
| +0x10 | spawnTableName | string | Mob/loot table reference |
| +0x14 | x | float | World position X |
| +0x18 | y | float | World position Y |
| +0x1c | z | float | World position Z |
| +0x20 | orientation | float | Facing direction (radians) |
| +0x24 | checkedOut | bool | Locked for editing |

### `unrpc:SpawnPointNameID` (type 0x0b, 0x14 bytes)

| Offset | Field | Type |
|--------|-------|------|
| +0x04 | id | int64 |
| +0x0c | spawnSetName | string |

### `unrpc:SpawnPointID` (type 0x0a)

| Offset | Field | Type |
|--------|-------|------|
| +0x04 | id | int64 |

Serializer: `0x015ac2c0`, Deserializer: `0x015bf320` / `0x015bf540`

---

## Spawn Set Loading (Level Init)

Function at `0x00bfc930` processes spawn data during level initialization:

1. Iterates level data for region types: "BoundingBox", "Cylinder", "Points", "SpawnSet"
2. For "SpawnSet" entries:
   - Creates `ASGWSpecSpawnSet` actor via `ULevel::SpawnActor`
   - Computes **centroid** of all spawn points (average of all positions)
   - Sets SpawnSet actor position to centroid
   - Adjusts all point offsets relative to centroid
   - For each point, creates `ASGWSpecSpawnPoint` child actor
   - Names each point `"SpawnPoint_%i"`
   - Links points to parent set

### Editor Visualization (SceneManager at `0x010cbc90`)

1. Gets current level name
2. Queries SpecSuite service for all SpawnSet-type specs
3. For each SpawnSet, iterates child spawn points
4. Reads position (scaled by Unreal units factor at `0x0181418c`)
5. Reads orientation (converted to radians via `0x01883bb8`)
6. Spawns `ASGWSpecSpawnPoint` actors for editor display

---

## SOAP RPC Operations (SpecSuite Service)

Endpoint: `http://localhost:8088/soap/SpecSuite/UnRPC.asmx`

| Operation | Description |
|-----------|-------------|
| `SpawnPointGet` | Retrieve spawn point(s) |
| `SpawnPointCreate` | Create new spawn point |
| `SpawnPointSave` | Save/update spawn point |
| `SpawnPointDelete` | Delete spawn point |
| `SpawnPointCheckout` | Lock for collaborative editing |

Type registry at `0x015c51f0`.

---

## Editor Actor Factories

| Factory Class | Address | Creates |
|---------------|---------|---------|
| `USGWActorFactorySpawnSet` | `0x008cf4c0` | SpawnSet regions |
| `USGWActorFactorySpawnPoint` | `0x008ce5e0` | SpawnPoint actors |
| `USGWActorFactoryRegion` | `0x008ce6a0` | Generic regions |
| `USGWActorFactoryCircleRegion` | `0x008ce760` | Circle regions |
| `USGWActorFactoryBoxRegion` | — | Box regions |
| `USGWActorFactoryWaypoint` | — | Waypoints |

---

## Respawner System (Separate from Spawn Points)

The respawner is a distinct system — likely an inventory item or interactable:

| Property | Description |
|----------|-------------|
| `respawnerID` | Unique respawner ID |
| `respawnerName` | Display name |
| `RespawnerMobId` | Mob template to respawn |
| `TimeToAid` | Timer for respawn/aid |

**Network events:**
- `Event_NetOut_Respawn` — client requests respawn
- `Event_NetOut_GiveRespawner` — GM gives respawner item
- `/respawn` and `/gmGiveRespawner` slash commands

Handler at `0x00cc2eb0` iterates property tree extracting respawnerID/Name pairs.

---

## Spawn Point CSV Export

Function at `0x00ff6420` provides editor CSV export with header:
```
SpawnPointXPosition,SpawnPointYPosition,SpawnPointZPosition,OrientationRadians,
```

SpawnPointSets can also be saved/loaded as UE3 prefabs (`SPAWNPOINTSET` string at `0x0199469c`).

---

## Key Global Variables

| Address | Description |
|---------|-------------|
| `0x01ee4a7c` | Cached UClass* for ASGWSpecSpawnPoint |
| `0x01ee4a78` | Cached UClass* for ASGWSpecActor |
| `0x01ee4a68` | Cached UClass* for SGWSpecBoxRegion |
| `0x01ee4a64` | Cached UClass* for SGWSpecCircleRegion |
| `0x01ee4a60` | Cached UClass* for SGWSpecRegion |
| `0x01ee4a5c` | Cached UClass* for SGWRegion |
| `0x0181418c` | Unreal unit conversion scale factor |
| `0x01883bb8` | Radian conversion factor |

---

## Implications for Cimmeria

1. **All spawn behavior must be server-implemented.** The `spawnTableName` field on each SpawnPoint references what should spawn — the server needs a table mapping these names to mob templates, counts, and timers.

2. **SpawnSet is a container of SpawnPoints.** Each SpawnPoint has position, orientation, and a `spawnTableName`. The server should iterate points in a set and spawn mobs at those locations.

3. **No client-side spawn conditions exist.** Level ranges, time-of-day gating, and spawn caps are purely server logic to implement.

4. **The respawner system is separate** — it's for player-activated mob respawning (likely an item), not the automatic world population system.

5. **Spawn data comes from SpecSuite SOAP service** during editor sessions, and from cooked level data during gameplay. The server needs to read the cooked level data to know where spawn points are.
