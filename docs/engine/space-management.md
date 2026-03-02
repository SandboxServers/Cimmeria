# Space Management

> **Last updated**: 2026-03-01
> **RE Status**: Well documented from Cimmeria source code + BigWorld 2.0.1 reference
> **Sources**: `src/cellapp/space.hpp`, `src/entity/world_grid.hpp`, `entities/cell_spaces.xml`, `entities/spaces.xml`, `config/BaseService.config`, `config/CellService.config`, BW `cellapp/space.hpp`, BW `cellapp/space_node.hpp`, BW `cellapp/space_branch.hpp`, BW `cellapp/cell.hpp`, BW `cellapp/cell_info.hpp`, BW `cellapp/cells.hpp`, BW `cellapp/entity.hpp`, BW `cellapp/entity_cache.hpp`, BW `cellapp/buffered_ghost_message.hpp`, BW `cellapp/buffered_ghost_messages.hpp`, BW `cellapp/buffered_ghost_messages_for_entity.hpp`, BW `cellapp/buffered_ghost_message_queue.hpp`, BW `cellapp/cellapp_config.hpp`, BW `lib/server/balance_config.hpp`

---

## Overview

Spaces are the fundamental world containers in BigWorld. Each space represents a self-contained game world (a planet surface, space station, instance, etc.). The CellApp manages all spaces and the entities within them. The WorldGrid system provides efficient Area of Interest (AoI) calculations.

## Space Definitions

### cell_spaces.xml

Defines which spaces the CellApp creates at startup:

From `entities/spaces.xml` (24 total — 16 shared + 8 instanced):

| Space | Instanced | Extents (X) | Extents (Y) | Size |
|-------|-----------|-------------|-------------|------|
| Agnos | no | -2400 to 2200 | -3200 to 2800 | 4600×6000 |
| Agnos_Library | no | -600 to 600 | -600 to 600 | 1200×1200 |
| Beta_Site_Evo_1 | no | -1600 to 2600 | -3000 to 3000 | 4200×6000 |
| Castle | no | 0 to 2400 | 0 to 2400 | 2400×2400 |
| Castle_CellBlock | **yes** | -800 to 800 | -800 to 800 | 1600×1600 |
| Dakara_E1 | no | -2000 to 2000 | -2000 to 2000 | 4000×4000 |
| Dakara_E1_StoryRm | **yes** | 0 to 200 | 0 to 200 | 200×200 |
| Harset | no | -1000 to 800 | -800 to 800 | 1800×1600 |
| Harset_CmdCenter | no | -400 to 400 | -400 to 400 | 800×800 |
| Harset_Market | **yes** | 0 to 0 | 0 to 0 | 0×0 |
| Harset_StorageRm | **yes** | -200 to 400 | -200 to 400 | 600×600 |
| Ihpet_Crater_Dark | no | -2600 to 200 | -600 to 1600 | 2800×2200 |
| Ihpet_Crater_Light | no | -2600 to 200 | -600 to 1600 | 2800×2200 |
| Lucia | no | -2800 to 2600 | -3000 to 3600 | 5400×6600 |
| Menfa_Dark | no | -1800 to 1200 | -1800 to 1800 | 3000×3600 |
| Menfa_Light | no | -1800 to 1200 | -1800 to 1800 | 3000×3600 |
| Omega_Site | no | -1000 to 1200 | -1200 to 1200 | 2200×2400 |
| Omega_Site_CmdCenter | **yes** | -400 to 400 | -600 to 600 | 800×1200 |
| SandBox | no | -600 to 1000 | -1000 to 800 | 1600×1800 |
| Sewer_Falls | no | -600 to 1000 | -1000 to 800 | 1600×1800 |
| SGC | **yes** | -400 to 400 | 0 to 800 | 800×800 |
| SGC_W1 | **yes** | -400 to 400 | -400 to 800 | 800×1200 |
| Tollana | no | -2400 to 1800 | -2400 to 2400 | 4200×4800 |
| Tollana_Curia | **yes** | 0 to 200 | 0 to 200 | 200×200 |

Total: 16 persistent (non-instanced) spaces loaded at startup. 8 additional instanced spaces created on demand (24 total defined in `spaces.xml`).

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

---

## BigWorld Reference: BSP Tree Space Partitioning

> The following sections document BigWorld 2.0.1's full space partitioning system from reference source code. Cimmeria currently uses a simpler single-CellApp model with WorldGrid (documented above). This reference material is included for future multi-CellApp support.

### SpaceNode Hierarchy

BigWorld partitions each Space into cells using a Binary Space Partition (BSP) tree. The tree is stored on each CellApp as `pCellInfoTree_` in the `Space` class. It is deserialized from a binary stream sent by the CellAppMgr when the space's cell layout changes.

The BSP tree has three node types in a class hierarchy:

```
SpaceNode (abstract base)
  |
  +-- SpaceBranch   (internal node -- splitting plane)
  |
  +-- CellInfo      (leaf node -- one cell)
```

**SpaceNode** (`cellapp/space_node.hpp`) is the abstract base class:

```cpp
class SpaceNode
{
public:
    virtual void deleteTree() = 0;
    virtual const CellInfo * pCellAt( float x, float z ) const = 0;
    virtual void visitRect( const BW::Rect & rect, CellInfoVisitor & visitor ) = 0;
    virtual void addToStream( BinaryOStream & stream ) const = 0;

protected:
    virtual ~SpaceNode() {};
};
```

The three virtual methods define the BSP contract:
- `pCellAt(x, z)` -- traverse the tree to find which cell owns a point
- `visitRect(rect, visitor)` -- find all cells that overlap a rectangle (used for ghost range queries)
- `addToStream()` -- serialize the tree for network transmission

**SpaceBranch** (`cellapp/space_branch.hpp`) represents an internal (splitting) node:

```cpp
class SpaceBranch : public SpaceNode
{
public:
    SpaceBranch( Space & space, const BW::Rect & rect,
            BinaryIStream & stream, bool isHorizontal );

    virtual const CellInfo * pCellAt( float x, float z ) const;
    virtual void visitRect( const BW::Rect & rect, CellInfoVisitor & visitor );

private:
    float       position_;      // Split position along the axis
    bool        isHorizontal_;  // true = split on Z axis, false = split on X axis
    SpaceNode * pLeft_;         // Subtree for coordinates < position_
    SpaceNode * pRight_;        // Subtree for coordinates >= position_
};
```

Each branch splits the space along either the X or Z axis at `position_`. The `isHorizontal_` flag determines the split axis (horizontal means the dividing line runs along X, splitting on Z). Traversal is O(log n) where n is the number of cells.

### Cell Boundaries

**CellInfo** (`cellapp/cell_info.hpp`) is the leaf node of the BSP tree, representing a single cell:

```cpp
class CellInfo : public SpaceNode, public ReferenceCount
{
public:
    CellInfo( SpaceID spaceID, const BW::Rect & rect,
            const Mercury::Address & addr, BinaryIStream & stream );

    const Mercury::Address & addr() const   { return addr_; }
    float getLoad() const                   { return load_; }
    const BW::Rect & rect() const           { return rect_; }
    void rect( const BW::Rect & rect )      { rect_ = rect; }

    bool contains( const Vector3 & pos ) const
        { return rect_.contains( pos.x, pos.z ); }

    bool shouldDelete() const;
    bool isDeletePending() const;
    bool hasBeenCreated() const;

private:
    SpaceID             spaceID_;       // Which space this cell belongs to
    Mercury::Address    addr_;          // CellApp address that owns this cell
    float               load_;          // Current load value for load balancing
    BW::Rect            rect_;          // Axis-aligned bounding rectangle
    bool                shouldDelete_;  // Marked for removal
    bool                isDeletePending_;
    bool                hasBeenCreated_;
};
```

Key points about CellInfo:
- Each cell is an axis-aligned rectangle (`BW::Rect`) in the XZ plane
- The `addr_` field identifies which CellApp process owns this cell -- different cells in the same Space can be on different CellApp processes
- The `load_` value is used by the CellAppMgr for load balancing decisions
- Cells can be dynamically created and removed (`shouldDelete_`, `isDeletePending_`)

The `Space` class maintains a map of all known cells by CellApp address:

```cpp
// In Space (private)
typedef std::map< Mercury::Address, SmartPointer< CellInfo > > CellInfos;
CellInfos cellInfos_;
SpaceNode * pCellInfoTree_;  // BSP root
```

The **Cells** collection (`cellapp/cells.hpp`) manages all cells owned by the local CellApp:

```cpp
class Cells
{
public:
    bool empty() const;
    size_t size() const;
    int numRealEntities() const;

    void handleCellAppDeath( const Mercury::Address & addr );
    void checkOffloads();           // Periodic offload check for all cells
    void backup( int index, int period );

    void add( Cell * pCell );
    void destroy( Cell * pCell );

    void addBoundsToStream( BinaryOStream & stream, int maxEntityOffload ) const;

private:
    typedef std::vector< Cell * > Container;
    Container container_;
};
```

A single CellApp process can own multiple cells across different spaces. The `Cells::checkOffloads()` method iterates all local cells and triggers entity offloading when entities cross cell boundaries.

### Entity Offloading

When a real entity moves across a cell boundary (from one CellInfo rect to another), it is "offloaded" from the source CellApp to the destination CellApp. The `Cell` class has the core offload method:

```cpp
class Cell
{
public:
    void offloadEntity( Entity * pEntity, CellAppChannel * pChannel,
            bool shouldSendPhysicsCorrection = false );

    void addRealEntity( Entity * pEntity, bool shouldSendNow );

    bool checkOffloadsAndGhosts();

    // Load balancing
    bool shouldOffload() const;
    void shouldOffload( bool shouldOffload );

    void retireCell( BinaryIStream & data );
    void removeCell( BinaryIStream & data );

private:
    Entities realEntities_;     // All real entities owned by this cell
    bool shouldOffload_;        // Flag set by CellAppMgr
    Space & space_;
    ConstCellInfoPtr pCellInfo_;
};
```

The `Entity` class supports offload/onload with a real-to-ghost conversion:

```cpp
class Entity : public PyInstancePlus
{
public:
    void offload( CellAppChannel * pChannel, bool isTeleport );
    void onload( const Mercury::Address & srcAddr,
        const Mercury::UnpackedMessageHeader & header,
        BinaryIStream & data );

    void createGhost( Mercury::Bundle & bundle );

    bool isReal() const;
    RealEntity * pReal() const;

    const Mercury::Address & realAddr() const;
    const Mercury::Address & nextRealAddr() const;

    NumTimesRealOffloadedType numTimesRealOffloaded() const;  // uint16

private:
    void convertRealToGhost( BinaryOStream * pStream = NULL,
            CellAppChannel * pChannel = NULL, bool isTeleport = false );
    void convertGhostToReal( BinaryIStream & data,
        const Mercury::Address * pBadHauntAddr = NULL );

    RealEntity *    pReal_;          // Non-null only if this is the real entity
    CellAppChannel * pRealChannel_;  // Channel to the CellApp owning the real
    Mercury::Address nextRealAddr_;  // Next real address during offload
    NumTimesRealOffloadedType numTimesRealOffloaded_;  // uint16, increments each offload
};
```

**Offload lifecycle:**

1. `Cell::checkOffloadsAndGhosts()` runs periodically (controlled by `checkOffloadsPeriod`)
2. For each real entity, if the entity's position is outside the cell's `CellInfo::rect()`, the entity needs offloading
3. The BSP tree is queried via `Space::pCellAt(x, z)` to find the destination `CellInfo`
4. The destination CellApp address comes from `CellInfo::addr()`
5. `Cell::offloadEntity()` serializes the entity's real state and sends it to the destination CellApp
6. On the source, `Entity::convertRealToGhost()` strips the `RealEntity` data and converts the entity to a ghost (the entity remains as a ghost on the source CellApp since it is near the boundary)
7. On the destination, `Entity::convertGhostToReal()` (or `Entity::onload()` if no ghost exists yet) promotes/creates the entity as real
8. `numTimesRealOffloaded_` is incremented to track the transition count

The `offloadHysteresis` config value (see CellApp Configuration below) prevents entities from ping-ponging back and forth across a cell boundary by requiring them to move a minimum distance past the boundary before triggering the offload.

---

## BigWorld Reference: Ghost Entity System

### Ghost Entity Lifecycle

In BigWorld, every entity exists as exactly one **real** entity and zero or more **ghosts**. The real entity is the authoritative copy, owned by the CellApp whose cell contains the entity's position. Ghost entities are read-only copies maintained on adjacent CellApps so that entities near cell boundaries can still see and interact with each other.

From the Entity class docstring in the source:

> An Entity may be "real" or "ghosted", where a "ghost" Entity is a copy of a "real" Entity that lives on an adjacent cell. For each entity there is one authoritative "real" Entity instance, and 0 or more "ghost" Entity instances which copy it.

**When a ghost is created:**

An entity gets a ghost on an adjacent CellApp when it is within `ghostDistance` of that cell's boundary. The `Space::createGhost()` method handles ghost creation:

```cpp
class Space : public TimerHandler
{
public:
    void createGhost( const Mercury::Address & srcAddr,
            const Mercury::UnpackedMessageHeader & header,
            BinaryIStream & data );

    void createGhost( const EntityID entityID,
                      BinaryIStream & data );
};
```

On the Entity side, ghosts are initialized with `initGhost()`:

```cpp
// Real entity creation
bool initReal( BinaryIStream & data, PyObject * pDict,
    bool isRestore, Mercury::ChannelVersion channelVersion,
    EntityPtr pNearbyEntity );

// Ghost entity creation
void initGhost( BinaryIStream & data );
```

**Real vs ghost duality:**

| Aspect | Real Entity | Ghost Entity |
|--------|-------------|--------------|
| `isReal()` | `true` | `false` |
| `pReal()` | Non-null `RealEntity*` | `NULL` |
| `pRealChannel_` | `NULL` (it is the real) | Channel to real's CellApp |
| Authority | Authoritative -- can modify state | Read-only copy |
| Python scripting | Full script access | Limited (forwards calls to real) |
| AoI management | Owns witness, manages AoI | Can be seen by local witnesses |
| Position updates | Receives from client/controllers | Receives via `ghostAvatarUpdate` |

**Ghost update messages** sent from real to ghost:

| Message | Purpose |
|---------|---------|
| `ghostAvatarUpdate` | Position/direction sync |
| `ghostHistoryEvent` | Property change events |
| `ghostedDataUpdate` | Bulk data synchronization |
| `ghostSetReal` | Transfer real ownership to this ghost |
| `ghostSetNextReal` | Notify ghost of upcoming real transfer |
| `delGhost` | Delete this ghost (entity moved out of range) |
| `ghostVolatileInfo` | Update volatile position reporting config |
| `ghostControllerCreate/Delete/Update` | Sync controller state |

**Ghost distance configuration** from `CellAppConfig`:

```cpp
static ServerAppOption< float > ghostDistance;       // How far past boundary ghosts extend
static ServerAppOption< int >   maxGhostsToDelete;   // Max ghosts to clean up per tick
static ServerAppOption< float > minGhostLifespan;    // Minimum time before ghost can be deleted
static ServerAppOption< int >   minGhostLifespanInTicks;  // Same, in tick units
static ServerAppOption< int >   ghostUpdateHertz;    // Frequency of ghost position updates
```

The `ghostDistance` defines how far beyond a cell boundary an entity's ghost is maintained. An entity within `ghostDistance` of a cell boundary will have a ghost created on the adjacent CellApp. When the entity moves further than `ghostDistance` from the boundary, the ghost is deleted (subject to `minGhostLifespan`).

### BufferedGhostMessage System

Ghost messages can arrive out of order when a real entity is offloaded between CellApps. Consider: CellApp A owns the real and is sending ghost updates to CellApp C. When the real offloads from A to B, CellApp B starts sending ghost updates to C. Due to network ordering, messages from B might arrive at C before the final messages from A.

BigWorld solves this with a **subsequence buffering** system. From the `BufferedGhostMessages` class comment:

> We consider the lifespan of a ghost as being split into subsequences of messages from each CellApp that the real visits. The Real entity tells ghosts of the next address before offloading. This allows the ghost to chain these subsequences together and play them in the correct order.

**BufferedGhostMessage** (`buffered_ghost_message.hpp`) -- abstract base for a single buffered message:

```cpp
class BufferedGhostMessage
{
public:
    BufferedGhostMessage( bool isSubsequenceStart, bool isSubsequenceEnd );

    virtual void play() = 0;
    virtual bool isCreateGhostMessage() const { return false; }

    bool isSubsequenceStart() const;
    bool isSubsequenceEnd() const;

private:
    bool isSubsequenceStart_;   // First message in a subsequence from one CellApp
    bool isSubsequenceEnd_;     // Last message in a subsequence (triggers next queue)
};
```

The `isSubsequenceStart_` / `isSubsequenceEnd_` flags allow the system to detect boundaries between message runs from different CellApps.

**BufferedGhostMessageQueue** (`buffered_ghost_message_queue.hpp`) -- a FIFO queue of messages from a single source CellApp:

```cpp
class BufferedGhostMessageQueue : public ReferenceCount
{
public:
    void add( BufferedGhostMessage * pMessage );
    bool playSubsequence();                    // Play messages until subsequence end
    void delaySubsequence( BufferedGhostMessage * pFirstMessage );
    void promoteDelayedSubsequences();

    bool isFrontCreateGhost() const;
    bool isEmpty() const;
    bool isDelaying() const;

private:
    typedef std::list< BufferedGhostMessage* > Messages;
    Messages messages_;
    Messages * pDelayedMessages_;    // Secondary queue for delayed subsequences
};
```

**BufferedGhostMessagesForEntity** (`buffered_ghost_messages_for_entity.hpp`) -- collects all queues for one entity, keyed by source CellApp address:

```cpp
class BufferedGhostMessagesForEntity
{
public:
    void add( const Mercury::Address & addr, BufferedGhostMessage * pMessage );

    bool playSubsequence( const Mercury::Address & srcAddr );
    bool playNewLifespan();

    void delaySubsequence( const Mercury::Address & addr,
            BufferedGhostMessage * pFirstMessage );

    bool hasMessagesFrom( const Mercury::Address & addr ) const;
    bool isDelayingMessagesFor( const Mercury::Address & addr ) const;

private:
    typedef std::map< Mercury::Address, BufferedGhostMessageQueuePtr > Queues;
    Queues queues_;   // One queue per source CellApp
};
```

**BufferedGhostMessages** (`buffered_ghost_messages.hpp`) -- global singleton managing all buffered ghost messages across all entities:

```cpp
class BufferedGhostMessages
{
public:
    void playSubsequenceFor( EntityID id, const Mercury::Address & srcAddr );
    void playNewLifespanFor( EntityID id );

    void delaySubsequence( EntityID entityID,
            const Mercury::Address & srcAddr,
            BufferedGhostMessage * pFirstMessage );

    void add( EntityID entityID, const Mercury::Address & srcAddr,
              BufferedGhostMessage * pMessage );

    bool isEmpty() const    { return map_.empty(); }

    static BufferedGhostMessages & instance();  // Singleton

private:
    typedef std::map< EntityID, BufferedGhostMessagesForEntity > Map;
    Map map_;
};
```

**Message ordering protocol:**

1. Before offloading, the real entity sends `ghostSetNextReal(nextRealAddr)` to all ghosts
2. The ghost now knows the next CellApp address to expect messages from
3. When the ghost receives the `ghostSetNextReal`, it marks the current subsequence as complete
4. It begins playing messages from the next CellApp's queue (if any have arrived early)
5. When the new real sends `ghostSetReal(numTimesRealOffloaded, owner)`, the ghost updates its `pRealChannel_` to point to the new owner

The `numTimesRealOffloaded` counter (uint16) is sent with `ghostSetReal` so the ghost can verify it is receiving the correct transition and detect stale messages.

### Entity Cache

The **EntityCache** (`cellapp/entity_cache.hpp`) stores information about entities in a witness's AoI, providing Level of Detail (LOD) and priority-based update scheduling:

```cpp
class EntityCache
{
public:
    static const int MAX_LOD_LEVELS = 4;

    typedef double Priority;

    EntityCache( const Entity * pEntity );

    float updatePriority( const Vector3 & origin );
    void updateDetailLevel( Mercury::Bundle & bundle, float lodPriority );
    void addOuterDetailLevel( BinaryOStream & stream );

    void addLeaveAoIMessage( Mercury::Bundle & bundle, EntityID id ) const;

    EntityConstPtr pEntity() const;
    Priority priority() const;
    void priority( Priority newPriority );

    void lastEventNumber( EventNumber eventNumber );
    EventNumber lastEventNumber() const;

    void detailLevel( DetailLevel detailLevel );
    DetailLevel detailLevel() const;

    IDAlias idAlias() const;   // Compact 8-bit alias for frequent updates
    void idAlias( IDAlias idAlias );

    AoIUpdateSchemeID updateSchemeID() const;

    // State flags
    bool isEnterPending() const;    // Waiting to send enterAoI to client
    bool isRequestPending() const;  // Expecting requestEntityUpdate from client
    bool isCreatePending() const;   // Waiting to send createEntity to client
    bool isGone() const;            // Waiting to remove from priority queue
    bool isWithheld() const;        // Do not send to client
    bool isRefresh() const;         // Waiting to be removed and re-added
    bool isUpdatable() const;       // None of the blocking flags are set

private:
    EntityConstPtr  pEntity_;
    Flags           flags_;
    AoIUpdateSchemeID updateSchemeID_;
    VehicleChangeNum vehicleChangeNum_;

    union {
        Priority    priority_;     // double -- distance-based priority
        EntityID    dummyID_;      // Only used if we have no entity
    };

    EventNumber     lastEventNumber_;
    VolatileNumber  lastVolatileUpdateNumber_;
    DetailLevel     detailLevel_;           // uint8
    IDAlias         idAlias_;               // uint8 (NO_ID_ALIAS = 0xff)

    EventNumber     lodEventNumbers_[ MAX_LOD_LEVELS ];  // Per-LOD event tracking
};
```

**Key EntityCache concepts:**

- **MAX_LOD_LEVELS = 4**: Up to 4 levels of detail for property updates. Closer entities get more frequent/detailed updates; distant ones get less
- **Priority** (`double`): Computed from distance to the witnessing entity. Lower priority entities get updates less frequently, saving bandwidth
- **IDAlias** (`uint8`): A compact 8-bit identifier used instead of the full 32-bit EntityID in frequent position updates. `NO_ID_ALIAS = 0xff` means no alias assigned
- **State flags**: Track the entity's lifecycle in the AoI pipeline (entering, creating, gone, withheld, refreshing)
- **LOD event numbers**: Track which events have been sent at each detail level, allowing incremental updates

The **EntityCacheMap** provides fast lookup of caches by entity:

```cpp
class EntityCacheMap
{
public:
    EntityCache * add( const Entity & e );
    void del( EntityCache * ec );
    EntityCache * find( const Entity & e ) const;
    EntityCache * find( EntityID id ) const;
    uint32 size() const;
    void visit( EntityCacheVisitor & visitor ) const;

private:
    typedef std::set< EntityCache > Implementor;
    Implementor set_;
};
```

---

## BigWorld Reference: Load Balancing

### BalanceConfig

The `BalanceConfig` class (`lib/server/balance_config.hpp`) defines the global load balancing parameters used by the CellAppMgr to decide when and how to adjust cell boundaries:

```cpp
class BalanceConfig
{
public:
    static ServerAppOption< float > maxCPUOffload;          // Max CPU threshold to trigger offload
    static ServerAppOption< int >   minEntityOffload;       // Min entities to offload per rebalance
    static ServerAppOption< float > minMovement;            // Min boundary movement per adjustment
    static ServerAppOption< float > slowApproachFactor;     // Dampening factor for boundary moves
    static ServerAppOption< bool >  demo;                   // Demo mode flag
    static ServerAppOption< float > demoNumEntitiesPerCell; // Target entity count in demo mode
};
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `maxCPUOffload` | float | CPU usage threshold above which the CellAppMgr begins offloading entities to less loaded CellApps |
| `minEntityOffload` | int | Minimum number of entities to move in a single offload operation (avoids thrashing) |
| `minMovement` | float | Minimum distance the BSP split plane must move per adjustment cycle |
| `slowApproachFactor` | float | Dampening multiplier to prevent oscillation when approaching balanced load |
| `demo` | bool | Enables demo/testing mode with simplified balancing |
| `demoNumEntitiesPerCell` | float | Target entity count per cell in demo mode |

The CellAppMgr uses these parameters to dynamically adjust the BSP tree split positions (`SpaceBranch::position_`), effectively resizing cells. When a CellApp is overloaded, the CellAppMgr shrinks its cell rectangles and grows neighboring cells, causing entities near the new boundary to be offloaded.

### CellApp Configuration

The `CellAppConfig` class (`cellapp/cellapp_config.hpp`) defines per-CellApp operational parameters that interact with the BSP tree and ghost system:

```cpp
class CellAppConfig : public EntityAppConfig
{
public:
    // --- Ghost system ---
    static ServerAppOption< float > ghostDistance;          // Ghost creation range past boundary
    static ServerAppOption< int >   maxGhostsToDelete;      // Max ghost cleanups per tick
    static ServerAppOption< float > minGhostLifespan;       // Min seconds before ghost deletion
    static ServerAppOption< int >   minGhostLifespanInTicks;
    static ServerAppOption< int >   ghostUpdateHertz;       // Ghost position update frequency

    // --- AoI ---
    static ServerAppOption< float > defaultAoIRadius;       // Default Area of Interest radius
    static ServerAppOption< float > loadSmoothingBias;      // Load calculation smoothing

    // --- Offloading ---
    static ServerAppOption< float > checkOffloadsPeriod;    // Seconds between offload checks
    static ServerAppOption< int >   checkOffloadsPeriodInTicks;
    static ServerAppOption< float > offloadHysteresis;      // Distance past boundary before offload
    static ServerAppOption< int >   offloadMaxPerCheck;     // Max entities to offload per check
    static ServerAppOption< int >   ghostingMaxPerCheck;    // Max ghosts to create per check

    // --- Backup ---
    static ServerAppOption< float > backupPeriod;           // Entity backup interval
    static ServerAppOption< int >   backupPeriodInTicks;

    // --- Performance ---
    static ServerAppOption< double > reservedTickFraction;  // Fraction of tick reserved for system
    static ServerAppOption< float >  sendWindowCallbackThreshold;
    static ServerAppOption< int >    obstacleTreeDepth;
    static ServerAppOption< float >  chunkLoadingPeriod;

    // --- Controllers ---
    static ServerAppOption< int >  expectedMaxControllers;
    static ServerAppOption< int >  absoluteMaxControllers;

    // --- Misc ---
    static ServerAppOption< int >   entitySpamSize;
    static ServerAppOption< bool >  fastShutdown;
    static ServerAppOption< bool >  shouldResolveMailBoxes;
    static ServerAppOption< bool >  loadDominantTextureMaps;
    static ServerAppOption< bool >  enforceGhostDecorators;
    static ServerAppOption< bool >  treatAllOtherEntitiesAsGhosts;
    static ServerAppOption< float > maxPhysicsNetworkJitter;
};
```

**How key parameters interact with the BSP tree:**

| Parameter | Interaction |
|-----------|-------------|
| `ghostDistance` | Defines the "overlap zone" beyond each cell boundary. Entities within this distance of a neighboring cell get ghosts created on that cell's CellApp. Must be >= `defaultAoIRadius` for seamless AoI across boundaries. |
| `defaultAoIRadius` | The default radius within which a witness entity can see other entities. Must be <= `ghostDistance` or entities near boundaries would have incomplete AoI. |
| `offloadHysteresis` | Prevents entity ping-pong at cell boundaries. An entity must move this far past the boundary before being offloaded. Works analogously to the AoI hysteresis in Cimmeria's WorldGrid. |
| `checkOffloadsPeriod` | How frequently the CellApp checks whether real entities have crossed cell boundaries. Trade-off between responsiveness and CPU cost. |
| `offloadMaxPerCheck` | Caps the number of entities offloaded per check cycle to prevent load spikes during mass migrations. |
| `ghostingMaxPerCheck` | Caps ghost creation per check cycle for the same reason. |
| `minGhostLifespan` | Prevents rapid ghost create/delete cycles for entities moving back and forth near a boundary. |

---

## Cimmeria vs BigWorld: Space Architecture Comparison

| Feature | Cimmeria | BigWorld 2.0.1 |
|---------|----------|----------------|
| **Space partitioning** | Single cell per space (no partitioning) | BSP tree (`SpaceNode` / `SpaceBranch` / `CellInfo`) |
| **CellApp scaling** | One CellApp owns all spaces | Multiple CellApps, each owning one or more cells |
| **AoI system** | `WorldGrid` -- chunk-based grid with vision distance | `RangeList` + `EntityCache` with LOD priorities |
| **AoI hysteresis** | Grid-level: `grid_hysteresis` (1 chunk = 50m) | Entity-level: per-entity AoI radius + hysteresis area |
| **Entity replication** | None -- all entities local | Ghost entities on adjacent CellApps within `ghostDistance` |
| **Cross-boundary interaction** | N/A (no boundaries) | Ghosts enable interaction; messages forwarded to real |
| **Entity offloading** | N/A | `offload()`/`onload()` with `convertRealToGhost`/`convertGhostToReal` |
| **Message ordering** | N/A | `BufferedGhostMessages` subsequence system |
| **Load balancing** | N/A | `BalanceConfig` -- dynamic BSP split adjustment by CellAppMgr |
| **AoI detail levels** | Binary (visible/invisible) | 4 LOD levels (`MAX_LOD_LEVELS = 4`) with priority-based updates |
| **Entity ID aliasing** | Full 32-bit ID always | 8-bit `IDAlias` for frequent updates (bandwidth optimization) |
| **Backup** | Not implemented | Periodic entity backup (`backupPeriod`) for fault tolerance |
| **Space transitions** | Full leave/join cycle via BaseApp | Same, plus offload for within-space cell transitions |

**When multi-CellApp support might be needed:**

Cimmeria's single-CellApp model works well for the game's 16 spaces with moderate entity counts. Multi-CellApp support (implementing BSP partitioning and ghosts) would become necessary if:

- A single space needs to support hundreds or thousands of simultaneous entities beyond what one CPU core can simulate at tick rate
- World extents grow large enough that a single-process spatial grid becomes memory-inefficient
- Fault tolerance is required (cell backup/recovery across CellApp processes)
- Different spaces need to run on dedicated hardware (e.g., a high-load PvP zone on a separate CellApp)

The BSP tree and ghost system are architecturally independent additions -- Cimmeria's existing `WorldGrid` AoI system would remain for intra-cell visibility, while the BSP tree would handle inter-cell partitioning at a higher level.

---

## Related Documents

- [BigWorld Architecture](bigworld-architecture.md) -- Entity and space model
- [Position Updates](../protocol/position-updates.md) -- Movement within spaces
- [Service Architecture](../architecture/service-architecture.md) -- Server configuration
- [Entity Property Sync](../protocol/entity-property-sync.md) -- AoI-driven property updates

## NavigationMesh Integration per Space

Each `Space` object owns an optional `NavigationMesh*` pointer (`navMesh_`). When a space is created, the constructor attempts to load a `.nav` file from `data/spaces/`:

```cpp
// From src/cellapp/space.cpp
Space::Space(uint32_t spaceId, std::string const & worldName, uint32_t creatorId)
{
    navMesh_ = new NavigationMesh();
    std::string navFileName = "../data/spaces/";
    navFileName += worldName_ + ".nav";
    if (!navMesh_->load(navFileName))
    {
        delete navMesh_;
        navMesh_ = nullptr;
    }
}
```

### NavMesh File Format (.nav)

The `.nav` file is a custom binary format produced by the `NavBuilder` tool (`src/nav_builder/builder.cpp`). It stores a serialized Recast `rcPolyMesh` + `rcPolyMeshDetail` with agent parameters:

```
.nav binary layout:
  float    agentHeight         (0.6)
  float    agentClimb          (0.9)
  float    agentRadius         (0.6)
  uint32   vertexCount
  uint32   polyCount
  uint32   nvp                 (max verts per poly, typically 6)
  uint32   borderSize
  float    cs                  (cell size, 0.3)
  float    ch                  (cell height, 0.2)
  float[3] bmin                (world bounds min)
  float[3] bmax                (world bounds max)
  ushort[] verts               [vertexCount * 3]
  ushort[] polys               [polyCount * nvp * 2]
  ushort[] regs                [polyCount]
  ushort[] flags               [polyCount]
  uchar[]  areas               [polyCount]
  uint32   detailMeshCount
  uint32   detailVertCount
  uint32   detailTriCount
  uint[]   detailMeshes        [detailMeshCount * 4]
  float[]  detailVerts         [detailVertCount * 3]
  uchar[]  detailTris          [detailTriCount * 4]
```

### Loading Pipeline

1. `xrcLoadPolyMesh()` reads the binary `.nav` file into `rcPolyMesh` + `rcPolyMeshDetail` structs
2. A `dtNavMeshCreateParams` is populated from the poly mesh data
3. `dtCreateNavMeshData()` builds the Detour runtime navmesh (with BV tree for fast queries)
4. `dtAllocNavMesh()` + `navMesh_->init()` initializes the Detour navmesh with `DT_TILE_FREE_DATA`
5. `query_.init(navMesh_, 2048)` creates a query object with a max node pool of 2048

### NavBuilder Tool

The `NavBuilder` (`src/nav_builder/builder.cpp`) is an offline tool:

```
Usage: NavBuilder <type> <space path> <destination obj> <format>
  type:   "chunked" (directory of .obj chunks) or "whole" (single .obj)
  format: "nav" (binary navmesh) or "obj" (Wavefront OBJ export)
```

Recast configuration used by NavBuilder:
- Cell size: 0.3, Cell height: 0.2
- Walkable slope: 45 degrees
- Agent height: 0.6, climb: 0.9, radius: 0.6
- Max edge length: 12.0 / cs = 40
- Max simplification error: 1.3
- Min region area: 64, Merge region area: 400
- Max verts per poly: 6
- Detail sample dist: 1.8, Detail sample max error: 0.2
- Uses monotone partitioning

### Available NavMeshes

Currently only 5 of 24 spaces have navmesh data in `data/spaces/`:

| File | Space |
|------|-------|
| `agnos.nav` | Agnos |
| `castle_cellblock.nav` | Castle_CellBlock |
| `harset.nav` | Harset |
| `harset_storagerm.nav` | Harset_StorageRm |
| `sgc_w1.nav` | SGC_W1 |

Spaces without navmesh data (19 of 24) log a FAULT at startup and set `navMesh_` to `nullptr`. Pathfinding calls to these spaces return `nullptr` / `-1.0f`.

### Pathfinding API

The `Space` class exposes pathfinding through `NavigationMesh`:

- `hasNavigationData()` -- Returns `true` if navmesh loaded successfully
- `findPath(params)` -- Returns a smoothed `NavigationPath*` or `nullptr`
- `findPathLength(params)` -- Returns total path length or `-1.0f`

`NavigationQueryParams` defaults: start extents 0.5m (tight, entity must be on a poly), destination extents 3.0m (loose, for jumping/railing entities), smoothing step size 0.75m, max 64 polys, max 256 smoothing steps.

---

## Space Data Format Sent to Clients

### BigWorld Space Data System

In BigWorld 2.0.1, space data is sent to clients via the `spaceData` message defined in `lib/connection/client_interface.hpp`:

```
spaceData message (variable length):
  SpaceID       spaceID       (uint32)
  SpaceEntryID  entryID       (Mercury::Address -- ip:port:salt)
  uint16        key           (data type identifier)
  char[]        value         (rest of message -- key-dependent)
```

The `key` field identifies what kind of space data is being sent. Key `1` is used for geometry mappings (the path to the space's chunk data). The `SpaceEntryID` is a unique identifier for each data entry, reusing the `Mercury::Address` struct (ip + port + salt).

On the client side, `ServerConnection::spaceData()` dispatches to `ServerMessageHandler::spaceData()`, which processes the key-value pairs. The client uses `BigWorld.addSpaceGeometryMapping(spaceID, matrix, path)` to load the geometry for a space.

### Cimmeria Space Data Messages

Cimmeria uses a simplified system with two related messages from `src/baseapp/mercury/sgw/messages.hpp`:

| Message ID | Name | Description |
|------------|------|-------------|
| `0x07` | `SPACE_DATA` | Space metadata (variable length: SpaceID, SpaceEntryID, Key, Value) |
| `0x08` | `SPACE_VIEWPORT_INFO` | Associates an entity with a space viewport |
| `0x0E` | `SET_SPACE_VIEWPORT` | Activates a previously created viewport (1 byte: ViewportID) |

When a cell player is created, the BaseApp sends:
1. `SPACE_VIEWPORT_INFO` -- EntityID, EntityID, SpaceID, ViewportID(0)
2. `CREATE_CELL_PLAYER` -- SpaceID, EntityID, Position, Direction, Properties

The client uses the viewport info to know which space to render and which entity is the player.

---

## Instanced Space Creation and Cleanup Lifecycle

### Creation Flow

Instanced spaces (8 defined: Castle_CellBlock, Dakara_E1_StoryRm, Harset_Market, Harset_StorageRm, Omega_Site_CmdCenter, SGC, SGC_W1, Tollana_Curia) are created on demand via Python:

1. **Python script calls `Atrea.createSpace(worldName, creatorId)`** -- This is a C++ binding that calls `SpaceManager::create()`
2. **SpaceManager::create()** allocates a local space ID from `freeIds_` pool (or appends to `spaces_` vector)
3. **Space constructor** loads navmesh from `data/spaces/{worldName}.nav` (may fail silently)
4. **worldSpaces_ check** -- Instanced spaces are NOT in `worldSpaces_` map (only shared spaces are), so `worldIt == worldSpaces_.end()`
5. **Event callback** fires `SpaceManager::Created` to notify BaseApp
6. **Python callback** `cell.spaceCreated(spaceId, worldName, instanced=1)` is called:
   - Queries the database for world_id
   - Creates a Python `Space` object
   - Registers with Python `SpaceManager`
   - Calls `space.loadSpawns()` to populate NPCs

### Startup Behavior

At startup, `SpaceManager::loadSpaces()` only creates non-instanced spaces:

```cpp
if (it->second.instanced)
{
    FAULTC(CATEGORY_SPACE, "Cannot create instanced space '%s' on startup", name);
    continue;
}
```

Instanced spaces are explicitly blocked from startup creation.

### Cleanup / Destruction

Instanced spaces are destroyed when their **creator entity** leaves:

```cpp
// From Space::removeEntity()
if (entityId == creatorId_)
{
    DEBUG2C(CATEGORY_SPACE, "Space creator has left space %d, destroying it", spaceId_);
    destroy();
}
```

The full destruction flow:
1. Creator entity leaves the space (removeEntity called with creatorId match)
2. `Space::destroy()` sets `destroyed_ = true`
3. No new entities can be added (`addEntity()` checks `destroyed_` flag)
4. No new players can be promoted (`promoteToPlayer()` checks `destroyed_` flag)
5. `SpaceManager::destroy(spaceId)` is called:
   - Python `cell.spaceDestroyed()` callback fires
   - `SpaceManager::Deleted` event notifies BaseApp
   - Space pointer set to `nullptr` in `spaces_` vector
6. The space ID is returned to `freeIds_` for reuse

### Space Destruction via Python

Spaces can also be destroyed programmatically via `Atrea.destroySpace(spaceId)`, which calls `SpaceManager::destroy()`. The `SpaceManager::removePlayers()` method iterates all spaces and calls `space->removePlayers()`, which destroys each player entity (potentially triggering cascading space destruction for instanced spaces).

---

## BigWorld Reference: Ghost Offload State Machine

### ghostSetReal / ghostSetNextReal Message Handling

The ghost offload state machine is defined in the BW `cellapp_interface.hpp` message declarations and `entity.hpp` method signatures. The BW 2.0.1 reference source only includes headers and compiled `.o` files (no `.cpp` source), but the full protocol can be reconstructed from the headers and the `BufferedGhostMessage` system documented above.

**Message declarations** from `cellapp_interface.hpp`:

```cpp
// Sent to ghost: "your real is about to move to this new address"
MF_BEGIN_ENTITY_MSG( ghostSetNextReal, GHOST_ONLY )
    Mercury::Address    nextRealAddr;
END_STRUCT_MESSAGE()

// Sent to ghost: "this address is now the real; update pRealChannel_"
MF_BEGIN_ENTITY_MSG( ghostSetReal, GHOST_ONLY )
    NumTimesRealOffloadedType numTimesRealOffloaded;  // uint16
    Mercury::Address    owner;
END_STRUCT_MESSAGE()
```

**Entity handler signatures** from `entity.hpp`:

```cpp
void ghostSetReal( const CellAppInterface::ghostSetRealArgs & args );
void ghostSetNextReal( const CellAppInterface::ghostSetNextRealArgs & args );
```

**Full offload state machine:**

```
State 0: NORMAL
  Real entity on CellApp A, ghosts on CellApps B, C, ...
  A sends ghost updates (ghostAvatarUpdate, ghostHistoryEvent, etc.) to B, C

State 1: PRE-OFFLOAD (triggered when entity crosses cell boundary)
  A sends ghostSetNextReal(nextRealAddr=D) to ALL ghosts
  -> Each ghost now knows: "next subsequence of messages will come from D"
  -> Ghost marks current subsequence as ending
  -> This is the LAST message in the subsequence from A

State 2: OFFLOAD IN PROGRESS
  A serializes real entity state via RealEntity::writeOffloadData()
  A calls Entity::convertRealToGhost() -- strips RealEntity, becomes ghost
  A sends serialized state to D (destination CellApp)
  numTimesRealOffloaded_ is incremented (uint16)

State 3: ONLOAD AT DESTINATION
  D receives entity data via Entity::onload() or convertGhostToReal()
  D creates/promotes the entity as real
  D sends ghostSetReal(numTimesRealOffloaded, owner=D) to ALL ghosts
  -> Each ghost updates pRealChannel_ to point to D
  -> numTimesRealOffloaded counter validates transition ordering
  -> This is the FIRST message in the new subsequence from D

State 4: NEW NORMAL
  D is now the real; A is now a ghost (if within ghostDistance)
  D sends ghost updates to A, B, C, ...
```

**Message ordering guarantee via BufferedGhostMessages:**

The `BufferedGhostMessagesForEntity` class (documented above) manages per-source-address queues. When `ghostSetNextReal` arrives, the ghost knows to:
1. Complete playback of the current queue (from A)
2. Start processing the queue from D (messages may have arrived out of order)

The `isSubsequenceStart` / `isSubsequenceEnd` flags on each `BufferedGhostMessage` delineate these transitions.

---

## BigWorld Reference: RealEntity Class Internals

### RealEntity Overview

From `cellapp/real_entity.hpp`, the `RealEntity` class stores all state that only exists on the authoritative (real) copy of an entity. When an entity is ghosted, the ghost has `pReal_ == NULL`.

### Class Members

```cpp
class RealEntity
{
private:
    Entity & entity_;                          // Back-reference to owning Entity

    Witness * pWitness_;                        // Non-null if entity has a client (player)

    Haunts haunts_;                             // List of CellApps with ghost copies

    NavLoc          navLoc_;                    // Current navigation location
    Navigator       navigator_;                 // Pathfinding navigator

    EntityRemovalHandle  removalHandle_;         // Quick-remove handle for Cell's real entity list

    EntityMailBoxRef     controlledBy_;          // Which entity controls this one

    Vector3         velocity_;                  // Current velocity
    Vector3         positionSample_;            // Position at last sample time
    GameTime        positionSampleTime_;        // When position was last sampled

    GameTime        creationTime_;              // When this real was created/offloaded

    AutoBackupAndArchive::Policy shouldAutoBackup_;  // Backup policy

    Mercury::Channel *  pChannel_;              // Channel to Base entity
};
```

### Witness Management

The `Witness` pointer (`pWitness_`) is non-null only for player entities (entities with a client connection):

- `enableWitness(data, replyID)` -- Creates and attaches a Witness to this real entity. Called when a client connects to this entity.
- `disableWitness(isRestore)` -- Detaches and destroys the Witness. Called on disconnect or restore.
- `isWitnessed()` -- Returns whether any ghost of this entity is being observed by a Witness on its CellApp
- `hasWitness` -- Python-visible property: `!!pWitness_`

The `checkGhostWitnessed` message (sent from real to ghosts) queries whether any local Witness on the ghost's CellApp can see this ghost -- used to determine if ghost updates need to be sent.

### Haunts List

The `Haunts` list tracks all CellApps that have a ghost copy of this entity:

```cpp
class Haunt
{
    CellAppChannel * pChannel_;     // Channel to the CellApp with the ghost
    GameTime creationTime_;         // When the ghost was created
};

typedef std::vector< Haunt > Haunts;
```

Key operations:
- `addHaunt(channel)` -- Called when a ghost is created on a remote CellApp
- `delHaunt(iter)` -- Called when a ghost is deleted (entity moved out of ghostDistance)
- `numHaunts()` -- Returns count of active ghosts
- `hauntsBegin()` / `hauntsEnd()` -- Iteration over haunts

When sending ghost update messages, the real entity iterates its haunts list and writes to each haunt's bundle:

```cpp
// Conceptual: sending ghostAvatarUpdate to all ghosts
for (auto it = hauntsBegin(); it != hauntsEnd(); ++it)
{
    Mercury::Bundle & bundle = it->bundle();
    // Write ghost update to bundle
}
```

### Offload Data

- `writeOffloadData(data, dstAddr, shouldSendPhysicsCorrection)` -- Serializes all real entity state for transfer to another CellApp
- `readOffloadData(data, pBadHauntAddr, pHasChangedSpace)` -- Deserializes real entity state after receiving an offload
- `deleteGhosts()` -- Sends `delGhost` to all haunts (cleanup before destruction)

### Backup

- `backup()` -- Writes entity state to the BaseApp for fault tolerance
- `autoBackup()` -- Triggered by `backupPeriod` timer; respects `shouldAutoBackup_` policy
- `writeBackupProperties(data)` -- Serializes persistent properties
- `writeBackupData(data)` / `readBackupData(data)` -- Full backup serialization

### Other Features

- `controlledBy` -- Python read/write property for entity control delegation
- `teleport(nearbyMBRef, position, direction)` -- Python-callable teleport method
- `sendPhysicsCorrection()` -- Corrects client position after server-side movement
- `newPosition(position)` -- Updates navloc after position change

---

## BigWorld CellAppInterface Message IDs vs Cimmeria Protocol

### BW CellAppInterface Ghost Protocol Messages

BigWorld uses macro-based message registration (`BEGIN_MERCURY_INTERFACE`) where message IDs are assigned sequentially at compile time. The exact numeric IDs are not explicit in the header -- they are determined by declaration order. From `cellapp_interface.hpp`, the ghost-related messages in declaration order are:

| BW Declaration | Direction | Target | Purpose |
|----------------|-----------|--------|---------|
| `ghostAvatarUpdate` | Real->Ghost | GHOST_ONLY | Position/direction sync |
| `ghostHistoryEvent` | Real->Ghost | GHOST_ONLY | Property change events |
| `ghostSetReal` | Real->Ghost | GHOST_ONLY | Transfer real ownership |
| `ghostSetNextReal` | Real->Ghost | GHOST_ONLY | Notify of upcoming transfer |
| `delGhost` | Real->Ghost | GHOST_ONLY | Delete ghost copy |
| `ghostVolatileInfo` | Real->Ghost | GHOST_ONLY | Volatile position config |
| `ghostControllerCreate` | Real->Ghost | GHOST_ONLY | Controller sync |
| `ghostControllerDelete` | Real->Ghost | GHOST_ONLY | Controller sync |
| `ghostControllerUpdate` | Real->Ghost | GHOST_ONLY | Controller sync |
| `ghostedDataUpdate` | Real->Ghost | GHOST_ONLY | Bulk data sync |
| `checkGhostWitnessed` | Real->Ghost | GHOST_ONLY | Query witness status |
| `aoiUpdateSchemeChange` | Real->Ghost | GHOST_ONLY | AoI scheme change |

### Cimmeria Protocol (No Ghost System)

Cimmeria does not implement ghosts (single CellApp architecture). Instead, Cimmeria's `BaseCellMessageId` enum (`src/mercury/base_cell_messages.hpp`) uses explicit numeric IDs for all Cell-Base communication:

| Cimmeria ID | Name | BW Equivalent |
|-------------|------|---------------|
| `0x00` | `CELL_BASE_AUTHENTICATE` | No BW equivalent (BW uses CellAppMgr) |
| `0x01` | `CELL_BASE_ENTITY_CREATE_ACK` | Part of `Cell::createEntity` |
| `0x02` | `CELL_BASE_ENTITY_DELETE_ACK` | Part of entity destruction |
| `0x03` | `CELL_BASE_SPACE_DATA` | `Space::spaceData` / `Space::allSpaceData` |
| `0x04` | `CELL_BASE_CREATE_CELL_PLAYER` | `Cell::createEntity` + `enableWitness` |
| `0x05` | `CELL_BASE_DISCONNECT_ENTITY_ACK` | Entity disconnect ack |
| `0x06` | `CELL_BASE_TICK` | `CellApp::setGameTime` |
| `0x07` | `CELL_BASE_ENTER_AOI` | (AoI handled by Witness in BW) |
| `0x08` | `CELL_BASE_LEAVE_AOI` | (AoI handled by Witness in BW) |
| `0x09` | `CELL_BASE_MOVE` | `ghostAvatarUpdate` equivalent for base |
| `0x0A` | `CELL_BASE_BACKUP_ENTITY` | `RealEntity::backup()` |
| `0x0B` | `CELL_BASE_RESTORE_ENTITY_ACK` | Restore from backup |
| `0x0C` | `CELL_BASE_SWITCH_SPACE` | Teleport / space change |
| `0x0D` | `CELL_BASE_BASEAPP_MESSAGE` | `callBaseMethod` |
| `0x0E` | `CELL_BASE_CLIENT_MESSAGE` | `callClientMethod` |
| `0x0F` | `CELL_BASE_NOTIFY_CREATE_CELL_ENTITY` | Entity lifecycle notification |
| `0x10` | `CELL_BASE_NOTIFY_DELETE_CELL_ENTITY` | Entity lifecycle notification |
| `0x11` | `CELL_BASE_UPDATE_CACHE_STAMP` | No direct BW equivalent (Cimmeria extension) |

Key differences:
- BW ghost messages (`ghostSetReal`, `ghostSetNextReal`, etc.) have **no Cimmeria equivalents** -- Cimmeria has no ghost system
- Cimmeria's AoI messages (`ENTER_AOI`, `LEAVE_AOI`, `MOVE`) are sent Cell->Base because the BaseApp manages client connections; in BW, the Witness on the CellApp handles these directly
- Cimmeria's cache stamp system (`0x11`) is a CME extension not present in standard BW

---

## TODO

- [x] ~~Document the NavigationMesh integration per space (Recast/Detour)~~ → Full pipeline documented: NavBuilder tool, .nav binary format, Space constructor loading, 5/24 spaces have navmeshes
- [x] ~~Document the space data format sent to clients (space geometry keys)~~ → BW uses key-value spaceData messages (key=1 for geometry mappings); Cimmeria uses SPACE_DATA (0x07) + SPACE_VIEWPORT_INFO (0x08) + SET_SPACE_VIEWPORT (0x0E)
- [x] ~~Verify world extents for all spaces~~ → 24 spaces verified from `entities/spaces.xml` (not 16 — includes instanced spaces like Castle_CellBlock, SGC_W1, etc.)
- [ ] Profile the WorldGrid algorithm with realistic entity counts
- [x] ~~Document the instanced space creation and cleanup lifecycle~~ → Full lifecycle: Python `Atrea.createSpace()` -> `SpaceManager::create()` -> navmesh load -> event callback -> Python `spaceCreated()`. Cleanup: creator entity leaves -> `destroy()` -> event -> Python `spaceDestroyed()` -> ID recycled
- [x] ~~Trace `ghostSetReal` / `ghostSetNextReal` message handling in BW `.cpp` files for full offload state machine~~ → No .cpp source available (only headers + .o files), but full 5-state machine reconstructed from headers: NORMAL -> PRE-OFFLOAD (ghostSetNextReal) -> OFFLOAD -> ONLOAD (ghostSetReal with numTimesRealOffloaded) -> NEW NORMAL
- [x] ~~Document BW `RealEntity` class internals (witness management, haunts list)~~ → Full class documented from `real_entity.hpp`: Witness* for player entities, Haunts vector tracking ghost locations, offload/backup data, navigation state, controlledBy delegation
- [x] ~~Map BW `CellAppInterface` message IDs for ghost protocol to Cimmeria's protocol numbering~~ → 12 BW ghost messages have no Cimmeria equivalents (no ghost system). Cimmeria uses 18 explicit Cell-Base message IDs (0x00-0x11) with cache stamp extension (0x11) not in BW
