# Space Management

> **Last updated**: 2026-03-01
> **RE Status**: Well documented from Cimmeria source code
> **Sources**: `src/cellapp/space.hpp`, `src/entity/world_grid.hpp`, `entities/cell_spaces.xml`, `entities/spaces.xml`, `config/BaseService.config`, `config/CellService.config`

---

## Overview

Spaces are the fundamental world containers in BigWorld. Each space represents a self-contained game world (a planet surface, space station, instance, etc.). The CellApp manages all spaces and the entities within them. The WorldGrid system provides efficient Area of Interest (AoI) calculations.

## Space Definitions

### cell_spaces.xml

Defines which spaces the CellApp creates at startup:

```xml
<Spaces>
    <Space WorldName="SandBox" />
    <Space WorldName="Castle" />
    <Space WorldName="Agnos" />
    <Space WorldName="Lucia" />
    <Space WorldName="Omega_Site" />
    <Space WorldName="Tollana" />
    <Space WorldName="Agnos_Library" />
    <Space WorldName="Sewer_Falls" />
    <Space WorldName="Harset" />
    <Space WorldName="Harset_CmdCenter" />
    <Space WorldName="Dakara_E1" />
    <Space WorldName="Ihpet_Crater_Dark" />
    <Space WorldName="Ihpet_Crater_Light" />
    <Space WorldName="Menfa_Dark" />
    <Space WorldName="Menfa_Light" />
    <Space WorldName="Beta_Site_Evo_1" />
</Spaces>
```

Total: 16 persistent spaces loaded at startup.

### spaces.xml

Defines world metadata for each space (extents, instancing rules):

```xml
<Space WorldName="Agnos">
    <Instanced>false</Instanced>
    <Extents minX="-500" minY="-500" maxX="500" maxY="500" />
</Space>
```

### WorldData Structure

From `src/cellapp/space.hpp`:

```cpp
struct WorldData {
    std::string worldName;     // Space name
    bool instanced;            // Multiple instances allowed?
    struct {
        int minX, minY;        // World extents (meters)
        int maxX, maxY;
    } extents;
};
```

## Space Class

The `Space` class (`src/cellapp/space.hpp`) manages a single game world:

### State

| Field | Type | Description |
|-------|------|-------------|
| `spaceId_` | uint32 | Unique space identifier |
| `worldName_` | string | References WorldData |
| `creatorId_` | uint32 | Entity that created this space (0 = system) |
| `entities_` | map<uint32, CellEntity> | All entities in the space |
| `dbEntities_` | multimap<uint32, CellEntity> | Entities indexed by database ID |
| `players_` | map<uint32, CellEntity> | Player entities (subset of entities_) |
| `navMesh_` | NavigationMesh* | Recast/Detour navigation data |

### Operations

- `addEntity()` -- Add an entity to the space
- `removeEntity()` -- Remove an entity
- `promoteToPlayer()` -- Move entity to player set (enables AoI witnessing)
- `demoteToEntity()` -- Remove from player set
- `findEntitiesByDbid()` -- Lookup by database ID
- `isValidPosition()` -- Check if position is within world extents
- `findPath()` / `findPathLength()` -- Navmesh pathfinding
- `tick()` -- Update all entities in the space

## SpaceManager

The `SpaceManager` singleton manages all spaces:

### Lifecycle

```
initialize() -> loadSpaces() -> create() ... destroy() -> ~SpaceManager()
```

### Space ID Allocation

Each CellApp has a local ID space. The SpaceManager converts between local and global IDs:

```cpp
bool isLocalId(uint32_t spaceId) const;
uint32_t toGlobalId(uint32_t spaceId) const;
```

### Instanced vs Shared Spaces

- **Shared spaces** (instanced=false): One instance for all players. Listed in `worldSpaces_` map. Created at startup.
- **Instanced spaces** (instanced=true): Created on demand per request. Multiple copies can exist simultaneously. Destroyed when empty.

### Events

```cpp
enum Event {
    Created,   // New space created
    Deleted    // Space destroyed
};
```

The BaseApp subscribes to space events to track which spaces are available for player placement.

### Game Timer

SpaceManager maintains a game timer for entity events:

```cpp
TimerMgr::TimerId addEntityTimer(double completeTime, PyObject * callback);
void cancelEntityTimer(TimerMgr::TimerId timerId);
```

## World Grid System

The WorldGrid (`src/entity/world_grid.hpp`) is a template-based spatial partitioning system that provides efficient AoI calculations.

### Grid Parameters

From `config/BaseService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `grid_chunk_size` | 50 meters | Size of each grid cell |
| `grid_vision_distance` | 3 chunks | How far entities can see (150m default) |
| `grid_hysteresis` | 1 chunk | Extra distance before leaving AoI (50m) |

### Grid Structure

```
World extents: minX=-500, minY=-500, maxX=500, maxY=500
Chunk size: 50m x 50m
Grid dimensions: 20 x 20 = 400 chunks
Vision distance: 3 chunks = 150m
Hysteresis: 1 chunk = 50m
Effective AoI radius: 150m (enter) / 200m (leave)
```

### Chunk Contents

Each chunk stores:
- `objects`: All entities in this chunk
- `witnesses`: Entities with visibility controllers (players)

### AoI Algorithm

#### Entity Enters AoI

1. Entity added to grid or moves to new chunk
2. For each chunk within `visionDistance` of the entity's chunk:
   - Notify all witnesses in that chunk: `onEntityVisible(entity)`
3. If entity is a witness, scan same range for all objects

#### Entity Leaves AoI

1. Entity moves to new chunk
2. Calculate which chunks are no longer within `visionDistance`
3. For affected chunks:
   - Without hysteresis: Immediately call `onEntityInvisible(entity)`
   - With hysteresis: Add to `visionExceptions_` list (still visible, but out of normal range)
4. When entity moves beyond `visionDistance + hysteresis`:
   - Call `onEntityInvisible(entity)` and remove from exceptions

#### Hysteresis

Hysteresis prevents rapid enter/leave cycling when an entity is near the AoI boundary:

```
                    +------ hysteresis zone ------+
                    |                              |
  [invisible] <--- | --- [visible] --- | --- [visible but tracked] --- | ---> [invisible]
                    |                  |                                |
              visionDistance    visionDistance              visionDistance + hysteresis
```

- An entity **enters** AoI when it comes within `visionDistance` chunks
- An entity **leaves** AoI when it moves beyond `visionDistance + hysteresis` chunks
- While in the hysteresis zone, the entity is tracked as a "vision exception"

### Large Movement Optimization

If an entity crosses more than 1 chunk in a single tick (teleport, fast movement), the grid resets its visibility data rather than incrementally updating:

```cpp
if (abs(oldPos.x - newPos.x) > 1 || abs(oldPos.y - newPos.y) > 1) {
    // Full reset: remove all vision data and rebuild
}
```

This is logged as a performance warning.

### WorldGridMember

Entities that participate in the grid system must inherit from `WorldGridMember<T>`:

```cpp
template <typename _T>
class WorldGridMember {
protected:
    std::set<boost::weak_ptr<_T>> witnesses_;         // Entities seeing us beyond normal range
    std::set<boost::weak_ptr<_T>> visionExceptions_;  // Entities we see beyond normal range
    WorldGrid<_T> * grid_;                            // Grid we belong to
};
```

## Base-Cell Space Communication

The CellApp informs the BaseApp about space changes:

| Message | Description |
|---------|-------------|
| `CELL_BASE_SPACE_DATA (0x03)` | Space created/destroyed notification |
| `CELL_BASE_ENTER_AOI (0x07)` | Entity entered a witness's AoI |
| `CELL_BASE_LEAVE_AOI (0x08)` | Entity left a witness's AoI |
| `CELL_BASE_MOVE (0x09)` | Entity moved within AoI |
| `CELL_BASE_SWITCH_SPACE (0x0C)` | Entity moving to different space |

The BaseApp translates these into client-facing messages.

## Space Transitions (Stargate Travel)

When a player uses a Stargate, the flow is:

1. Client sends gate travel request
2. Cell Python script handles the gate address
3. CellApp sends `CELL_BASE_SWITCH_SPACE` with destination world name
4. BaseApp removes cell entity from old space
5. BaseApp requests new cell entity in destination space
6. Client receives new `spaceViewportInfo` + `createCellPlayer`

For instanced spaces, a new Space is created if one doesn't already exist.

## Related Documents

- [BigWorld Architecture](bigworld-architecture.md) -- Entity and space model
- [Position Updates](../protocol/position-updates.md) -- Movement within spaces
- [Service Architecture](../architecture/service-architecture.md) -- Server configuration
- [Entity Property Sync](../protocol/entity-property-sync.md) -- AoI-driven property updates

## TODO

- [ ] Document the NavigationMesh integration per space (Recast/Detour)
- [ ] Document the space data format sent to clients (space geometry keys)
- [ ] Verify world extents for all 16 spaces from spaces.xml
- [ ] Profile the WorldGrid algorithm with realistic entity counts
- [ ] Document the instanced space creation and cleanup lifecycle
- [ ] Investigate BigWorld's cell boundary system for future multi-CellApp support
