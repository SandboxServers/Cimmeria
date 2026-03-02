# Spawn System

## Status: Known/Missing (KM)

The spawn system is fully defined in entity `.def` files and partially defined in the
PostgreSQL schema, but has **zero lines of implementation** in Python. Both
`SGWSpawnRegion` and `SGWSpawnSet` are empty stubs that call `super().__init__()` and
nothing else. Twenty base methods are declared across the two entities; none are
implemented. The design intent is reconstructed here from property names, method
signatures, DB schema, and standard MMO conventions.

---

## Overview

The spawn system uses two server-only entities that cooperate through BigWorld base
mailboxes. Neither entity has a cell presence, client methods, or visible world state.
All communication between them is base-to-base RPC.

```
SpawnRegion (area manager, BASE only)
  |-- SpawnSet 1 (mob group, BASE only)
  |     |-- SGWMob A  (cell entity, has spawner mailbox back to set)
  |     |-- SGWMob B
  |-- SpawnSet 2 (mob group, BASE only)
        |-- SGWMob C
```

**SGWSpawnRegion** is the top-level authority. It owns a list of SpawnSets, controls
how many may be active simultaneously, manages respawn timers, and receives
notifications from players entering or leaving the region. It also collects mission
progress data and time-of-day events.

**SGWSpawnSet** manages a specific group of spawn points and the mobs that fill them.
It selects which mob type to spawn using a weighted spawn table, tracks live mob entity
IDs, and reports its current population back to the region.

Both entities are `ServerOnly` and inherit from `SGWEntity`. `SGWSpawnSet` also
implements the `GroupAuthority` interface.

---

## Database Schema

### `resources.spawnlist`

The primary spawn data table. Each row places one entity instance in the world.

| Column | Type | Notes |
|--------|------|-------|
| `spawn_id` | integer PK | Auto-increment |
| `x`, `y`, `z` | real | World position |
| `heading` | real | Orientation (radians) |
| `world_id` | integer | Target space/world |
| `template_id` | integer | FK to `entity_templates` |
| `tag` | varchar(100) | Unique name for mission references |
| `set_name` | varchar(100) | Optional spawn set membership |

Existing data shows named tags (`MessHall_Guard1`, `Prisoner_329`, etc.) used by
missions to reference specific entity instances.

### `resources.entity_templates`

Defines the mob archetype used when creating an entity from a spawn point.

| Column | Notes |
|--------|-------|
| `template_id` | PK |
| `template_name` | Human-readable name |
| `class` | Python entity class name |
| `level` | Base level |
| `alignment`, `faction` | Allegiance values |
| `name_id` | Localisation string ID |
| `ability_set_id` | FK to ability sets |
| `loot_table_id` | FK to `loot_tables` |
| `patrol_path_id` | FK to patrol path definition |
| `patrol_point_delay` | Seconds to wait at each patrol waypoint |
| `body_set`, `static_mesh` | Visual representation |

### `resources.spawn_sets`

DB-level spawn set records (separate from the BigWorld entity).

| Column | Notes |
|--------|-------|
| `set_id` | PK |
| `name` | Set name |
| `type` | Set type string |
| `world_id` | Owning world |
| `height` | Vertical offset |

### `resources.spawn_points`

Individual spawn point positions, referenced by spawn set.

| Column | Notes |
|--------|-------|
| `point_id` | PK |
| `x`, `y`, `z` | World position |
| `orientation` | Heading (radians) |
| `spawn_set_name` | Name of owning set |
| `spawn_table_name` | Which spawn table applies at this point |

**Note:** No `spawn_tables` table exists in the current schema. The `spawnTableIDs`
property on `SGWSpawnSet` stores `(SpawnTableID, Weight)` tuples in a Python list,
suggesting spawn table data may be loaded from a separate source or the table needs to
be added.

---

## SGWSpawnRegion

**Source files:**
- `python/base/SGWSpawnRegion.py` (stub)
- `python/cell/SGWSpawnRegion.py` (stub)
- `entities/defs/SGWSpawnRegion.def`

### Properties

| Property | Type | Default | Flag | Purpose |
|----------|------|---------|------|---------|
| `SpawnRegionID` | UINT32 | — | BASE | DB identity |
| `WorldID` | INT32 | — | BASE | Space this region belongs to |
| `MySpawnSetIDs` | ARRAY\<INT32\> | — | BASE | DB IDs of owned SpawnSets |
| `MySpawnSetEntityIDs` | ARRAY\<INT32\> | — | BASE | BigWorld entity IDs of created SpawnSets |
| `RegionSpawnTimer` | INT32 | 10 | BASE | Spawn tick interval (seconds) |
| `Activated` | INT8 | 1 | BASE | Whether region is active |
| `MaxActiveSets` | INT32 | 0 | BASE | Max concurrent active SpawnSets |
| `MinRespawnSeconds` | INT32 | 0 | BASE | Minimum respawn delay |
| `MaxRespawnSeconds` | INT32 | 0 | BASE | Maximum respawn delay |
| `SpawnSetMinCooldown` | INT32 | 0 | BASE | Min cooldown before set reactivation |
| `SpawnSetMaxCooldown` | INT32 | 0 | BASE | Max cooldown before set reactivation |
| `RespawnFlag` | UINT8 | 1 | BASE | Respawning enabled flag |
| `EventSetId` | INT32 | 0 | BASE | Kismet event set ID |
| `entitiesInRegion` | PYTHON | {} | BASE | Players currently in region |
| `missionData` | PYTHON | {} | BASE | `{missionId: {step: count}}` |
| `CurrentPopulation` | PYTHON | {} | BASE | Population map across all sets |
| `LastReportedPopulation` | UINT32 | 0 | BASE | Previous population total |
| `mobDeaths` | UINT32 | 0 | BASE | Cumulative mob death count |
| `timerReduction` | FLOAT | 0 | BASE | Respawn timer reduction factor |
| `respawnTimers` | PYTHON | [] | BASE | Active respawn timer handles |
| `populationReportTimer` | INT32 | 0 | BASE | Timer handle for population reports |
| `missionEventTimer` | CONTROLLER_ID | 0 | BASE | Timer for mission event polling |
| `spaceCreatorBaseMB` | MAILBOX | — | BASE | Mailbox to the space creator |
| `lastTimeCheck` | FLOAT | 0 | CELL_PRIVATE | Last time-of-day check timestamp |
| `lastWorldTime` | FLOAT | 0 | CELL_PRIVATE | Last recorded world time |
| `detectionRadius` | FLOAT | 0 | CELL_PRIVATE | Radius for player detection |
| `detectionController` | CONTROLLER_ID | 0 | CELL_PRIVATE | BigWorld proximity controller ID |

### Base Methods

All 14 methods are declared but unimplemented.

| Method | Arguments | Description |
|--------|-----------|-------------|
| `RegisterSpawnSetBase` | entityId: INT32 | SpawnSet calls this after creation |
| `ActivateMe` | INT32 | Activate the region |
| `DeactivateMe` | INT32 | Deactivate the region |
| `deactivateSet` | spawnSetId: INT32, despawnMobs: UINT8 | Deactivate one set, optionally despawning its mobs |
| `activateSet` | spawnSetId: INT32 | Activate one specific set |
| `StartRespawnTimer` | (none) | Begin countdown for next respawn cycle |
| `onTimeOfDayTick` | INT32 | Receive time-of-day update |
| `onSpawnSetActivate` | entityId: INT32 | Callback when a set activates |
| `onSpawnSetDeactivate` | entityId: INT32 | Callback when a set deactivates |
| `onEntityEnter` | entityId: INT32, missionData: ARRAY\<PYTHON\> | Player entered region |
| `onEntityExit` | entityId: INT32 | Player left region |
| `entityMissionStepUpdate` | entityId: INT32, missionId: INT32, stepCount: INT32 | Mission step progress from a player |
| `entityMissionComplete` | entityId: INT32, missionId: INT32 | Player completed a mission in region |
| `reportPopulation` | spawnSetId: INT32, currentPop: INT32 | SpawnSet reports its live count |
| `onMobDeath` | (none) | A mob in this region died |

---

## SGWSpawnSet

**Source files:**
- `python/base/SGWSpawnSet.py` (stub)
- `python/cell/SGWSpawnSet.py` (stub)
- `entities/defs/SGWSpawnSet.def`

### Properties

| Property | Type | Default | Flag | Purpose |
|----------|------|---------|------|---------|
| `SpawnSetID` | UINT32 | 0 | BASE | DB identity |
| `mySpawnRegionID` | INT32 | 0 | BASE | Owning region DB ID |
| `MySpawnPoints` | PYTHON | [] | BASE | List of spawn point definitions |
| `SpawnPointLastUsed` | ARRAY\<FLOAT\> | — | BASE | Per-point last-used timestamp |
| `spawnPointReservation` | PYTHON | {} | BASE | Reserved points (in-progress spawns) |
| `bRandomizeSpawnPoints` | INT8 | 0 | BASE | Random vs sequential point selection |
| `MyMobBaseIDList` | PYTHON | [] | BASE | Entity IDs of currently live mobs |
| `deadMOBs` | ARRAY\<INT32\> | — | BASE | Entity IDs of recently dead mobs |
| `CurrentPopulation` | UINT32 | 0 | BASE | Live mob count |
| `LastReportedPopulation` | UINT32 | 0 | BASE | Population at last report |
| `Activated` | INT8 | 1 | BASE | Whether set is currently spawning |
| `bLinked` | INT8 | 0 | BASE | Linked to another set (dependency) |
| `spawnTableIDs` | PYTHON | [] | BASE | List of `(SpawnTableID, Weight)` tuples |
| `currentSpawnTableID` | INT32 | 0 | BASE | Currently selected spawn table |
| `minMOBLevel` | INT32 | 0 | BASE | Minimum mob level override |
| `maxMOBLevel` | INT32 | 0 | BASE | Maximum mob level override |
| `cooldownSeconds` | INT32 | 0 | BASE | Current cooldown duration |
| `minCooldownSeconds` | INT32 | 0 | BASE | Minimum cooldown range |
| `maxCooldownSeconds` | INT32 | 0 | BASE | Maximum cooldown range |
| `lastCooldownStart` | INT32 | 0 | BASE | Timestamp when cooldown started |
| `cooldownTimer` | INT32 | 0 | BASE | Active cooldown timer handle |
| `populationReportTimer` | INT32 | 0 | BASE | Timer handle for periodic reports |
| `spaceCreatorBaseMB` | MAILBOX | — | BASE | Mailbox to the space creator |

### Base Methods

All 6 methods are declared but unimplemented.

| Method | Arguments | Description |
|--------|-----------|-------------|
| `RegisterMobBase` | entityId: INT32, mobId: INT32 | Mob calls after it finishes spawning |
| `MobFailedToSpawn` | entityId: INT32 | Mob creation failed, release the point |
| `BeingDeath` | entityId: INT32 | Mob died, remove from live list |
| `BeingDespawn` | entityId: INT32 | Mob was manually despawned |
| `ActivateMe` | (none) | Begin spawning mobs at all points |
| `DeactivateMe` | despawnMobs: UINT8 | Stop spawning; optionally destroy existing mobs |

---

## SGWMob Spawn Fields

From `entities/defs/SGWMob.def`, each mob carries two spawn-system properties:

| Property | Type | Flag | Purpose |
|----------|------|------|---------|
| `spawner` | MAILBOX | CELL_PRIVATE | Base mailbox back to the owning SpawnSet |
| `mobGroup` | PYTHON | CELL_PRIVATE | Group membership data |

The `spawner` mailbox is set at creation time and is how a mob calls `BeingDeath` or
`BeingDespawn` on its set without needing to know which set it belongs to by ID.

---

## Reconstructed Lifecycle

### Region Startup

1. Region entity loads from DB. `MySpawnSetIDs` contains the DB IDs of its sets.
2. Region creates one `SGWSpawnSet` entity per ID, storing resulting entity IDs in
   `MySpawnSetEntityIDs`.
3. Each SpawnSet, once created, calls `RegisterSpawnSetBase(entityId)` on the region's
   base mailbox to confirm it is ready.
4. Region activates up to `MaxActiveSets` sets by calling `ActivateMe()` on each
   selected set's base mailbox.

### SpawnSet Activation

1. Set receives `ActivateMe()`.
2. Set selects a spawn table from `spawnTableIDs` using weighted random selection.
   The selected `SpawnTableID` is stored in `currentSpawnTableID`.
3. Set iterates `MySpawnPoints`. If `bRandomizeSpawnPoints` is 1, the order is
   shuffled; otherwise points are used sequentially.
4. For each point, a mob entity is created using the template specified by the spawn
   table. The mob's `spawner` mailbox is set to the SpawnSet's base mailbox.
5. Each successfully created mob calls `RegisterMobBase(entityId, mobId)` back to the
   set. The set appends the entity ID to `MyMobBaseIDList` and increments
   `CurrentPopulation`.
6. If a mob fails to create (navmesh placement error, resource missing), it calls
   `MobFailedToSpawn(entityId)` to release the reserved spawn point.
7. Set calls `reportPopulation(SpawnSetID, CurrentPopulation)` on the region.
8. Region calls `onSpawnSetActivate(entityId)` to record the set as active.

### Mob Death

1. When an `SGWMob` dies, it calls `BeingDeath(entityId)` on its `spawner` mailbox.
2. SpawnSet removes the entity ID from `MyMobBaseIDList`, appends to `deadMOBs`,
   decrements `CurrentPopulation`.
3. SpawnSet calls `reportPopulation(SpawnSetID, CurrentPopulation)` on the region.
4. Region increments `mobDeaths` and calls `onMobDeath()`.
5. If `RespawnFlag` is 1, region calls `StartRespawnTimer()`. The timer delay is
   selected randomly between `MinRespawnSeconds` and `MaxRespawnSeconds`, reduced by
   `timerReduction`. The timer handle is appended to `respawnTimers`.
6. On timer expiry, region reactivates either the depleted set (if its cooldown has
   elapsed) or another set from the cooldown-ready pool.

### Set Cooldown

When all mobs in a set die and respawning is deferred, the set enters cooldown:

1. A random duration is selected between `SpawnSetMinCooldown` and
   `SpawnSetMaxCooldown` (region) or `minCooldownSeconds` / `maxCooldownSeconds`
   (set).
2. `lastCooldownStart` is recorded and `cooldownTimer` is started.
3. When `MaxActiveSets` is greater than one, the region can activate a different set
   during the cooldown window, allowing mob variety to rotate across the region.

### Player Detection and Mission Integration

The `detectionRadius` and `detectionController` (CELL_PRIVATE) indicate the cell-side
spawn region uses a BigWorld proximity trigger (TrapController or similar) to detect
when players enter or leave the region volume.

When a player enters:
1. Cell entity fires `onEntityEnter(entityId, missionData)` on the base mailbox.
2. Region stores the entity in `entitiesInRegion` and merges the player's relevant
   mission data into `missionData`.

When a player exits:
1. Cell entity fires `onEntityExit(entityId)` on the base mailbox.
2. Region removes the entity from `entitiesInRegion`.

As a player completes mission objectives (killing mobs), the mission system calls
`entityMissionStepUpdate(entityId, missionId, stepCount)` and
`entityMissionComplete(entityId, missionId)`. The region forwards or aggregates these
into `missionData` for the kill count tracking visible in
`CONDITION_FEEDBACK_SpawnRegionActivated` and related condition feedback types
(defined in `resources.sql`).

### Time of Day

The `lastTimeCheck` and `lastWorldTime` cell properties, combined with the
`onTimeOfDayTick(INT32)` base method, suggest the spawn region supports day/night
mob switching. The cell-side would poll world time, detect a threshold crossing, and
notify the base via `onTimeOfDayTick`. The base could then swap which spawn tables are
active or toggle set activation accordingly.

---

## Spawn Table System

`spawnTableIDs` stores a list of `(SpawnTableID, Weight)` tuples directly on the
SpawnSet entity. Selection algorithm (inferred):

```python
import random

def selectSpawnTable(spawnTableIDs):
    total = sum(weight for _, weight in spawnTableIDs)
    roll = random.uniform(0, total)
    cumulative = 0
    for table_id, weight in spawnTableIDs:
        cumulative += weight
        if roll <= cumulative:
            return table_id
    return spawnTableIDs[-1][0]
```

The selected table controls which `entity_templates` entries are used at each spawn
point. This allows a single SpawnSet to sometimes spawn one mob type and sometimes
another, providing visible variety in the world.

**Missing:** No `spawn_tables` table exists in `resources.sql`. One will need to be
created that maps `SpawnTableID` to a set of `(template_id, weight)` rows, or the
spawn point itself may encode the template directly via `spawn_table_name` in
`resources.spawn_points`.

---

## Known Gaps

| Gap | Impact |
|-----|--------|
| No Python implementation in any file | System is entirely non-functional |
| No `spawn_tables` DB table | Cannot resolve `SpawnTableID` to templates |
| SpawnRegion/SpawnSet creation entry point unknown | Unclear what instantiates these entities at server start (space creator or BaseApp init) |
| `bLinked` semantics not specified | Linked-set dependencies are undefined |
| `timerReduction` source unknown | Nothing sets this field; possibly player population scaling |
| Cell-side region has no implementation | Detection controller is never registered |
| `missionEventTimer` purpose unclear | May be for periodic condition feedback checks |
