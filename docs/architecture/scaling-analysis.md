# Scaling Analysis

> **Last updated**: 2026-03-02
> **Sources**: `src/baseapp/cell_manager.hpp/cpp`, `src/cellapp/`, `entities/spaces.xml`, `entities/cell_spaces.xml`, `config/CellService.config`, `config/BaseService.config`, `docs/engine/bigworld-architecture.md`

---

## Overview

This document analyzes how the Cimmeria server handles scaling today, what the original BigWorld engine was designed to support, and what scaling strategies are realistic for a Stargate Worlds emulator going forward. The core finding is that Cimmeria already operates as a single-instance-per-role deployment and pays the full complexity cost of a distributed architecture while gaining none of its scaling benefits.

---

## What BigWorld Was Designed To Do

The production BigWorld engine (used by World of Tanks, Warhammer Online, etc.) supports a fully distributed server mesh:

| Component | Production BigWorld | Purpose |
|---|---|---|
| **LoginApp** | Multiple, load-balanced | Accept client connections |
| **BaseAppMgr** | 1 per cluster | Assign players to BaseApps by load |
| **BaseApp** | N per cluster (10-50+) | Own persistent entity state, proxy client connections |
| **CellAppMgr** | 1 per cluster | Partition spaces across CellApps, manage cell boundaries |
| **CellApp** | N per cluster (10-100+) | Spatial simulation for a rectangular region of a space |
| **DBMgr** | 1 per cluster | Database persistence layer |
| **Reviver** | 1 per cluster | Restart crashed processes, restore state from backups |

Key scaling features of production BigWorld:

- **Cell splitting**: A single large zone is divided into rectangular cells, each managed by a different CellApp. As player density shifts, the CellAppMgr can move cell boundaries dynamically.
- **Entity ghosting**: Entities near cell boundaries are "ghosted" (mirrored) to adjacent cells so that AoI queries work seamlessly across boundaries.
- **Entity migration**: When an entity crosses a cell boundary, ownership transfers from one CellApp to another via a migration protocol.
- **Multi-BaseApp**: Players are distributed across BaseApps by BaseAppMgr. Each player's Base entity lives on one BaseApp and forwards messages to whatever CellApp currently owns their Cell entity.

This architecture can scale to thousands of concurrent players in a single seamless world.

---

## What Cimmeria Actually Implements

Cimmeria implements the distributed architecture's communication protocol but none of its scaling logic. The result is a single-instance deployment with distributed-system complexity.

### Single BaseApp

- One BaseApp process handles all connected clients.
- No BaseAppMgr exists to distribute players across multiple BaseApps.
- `config/BaseService.config` contains no multi-instance configuration.

### Single CellApp

- One CellApp process runs all 24 zones.
- No CellAppMgr exists to distribute zones or split cells.
- `config/CellService.config` hardcodes `cell_id` to `1`.

### No Cell Splitting

Each zone runs entirely within the single CellApp. From `docs/engine/bigworld-architecture.md`:

> SGW does not use multi-cell spaces. Each space runs entirely on the single CellApp.

The `entities/spaces.xml` file defines spatial extents (MinX/MaxX/MinY/MaxY) for each zone, but these are used only for AoI queries within the single process, not for cell boundary partitioning.

### No Entity Migration

There is no protocol for transferring entity ownership between CellApps. Zone transitions (stargate travel) are handled by destroying the entity in one space and recreating it in another — both within the same CellApp process.

### No Entity Ghosting

There is no ghost entity mechanism. All entities in a space are fully owned by the single CellApp.

### Vestigial Multi-Cell Registration

The BaseApp's `CellManager` class (`src/baseapp/cell_manager.hpp`) has a `std::map<uint32_t, CellAppConnection*>` that technically supports multiple registered CellApps:

```cpp
// src/baseapp/cell_manager.cpp
Mercury::CellAppConnection * CellManager::randomCell()
{
    // FIXME: Currently this only returns the first cell ID;
    // cell distribution should be randomized by load
    if (!cells_.empty())
        return cells_.begin()->second;
    else
        return nullptr;
}
```

The FIXME confirms: multi-cell support was intended but never implemented. The method always returns the first (and only) registered CellApp.

Similarly, `registerSpace()` enforces single-cell ownership:

```cpp
// src/baseapp/cell_manager.cpp
void CellManager::registerSpace(Mercury::CellAppConnection * cell,
                                 uint32_t spaceId,
                                 std::string const & worldName)
{
    // Non-instanced spaces can only exist on one cell
    if (!worldIt->second.instanced)
    {
        if (worldIt->second.handler)
        {
            WARN("Space '%s' already exists on cell %d", worldName.c_str());
            return;
        }
        worldIt->second.handler = cell;  // Exclusive assignment
    }
}
```

### The Complexity Cost

Despite running a single instance of each role, Cimmeria maintains the full inter-service communication layer:

| Component | Lines (approx.) | Purpose |
|---|---|---|
| Mercury transport layer | ~5,200 | Reliable UDP with selective ACK, encryption |
| Base-Cell TCP protocol | ~2,400 | Entity creation, property sync, method routing |
| Cell registration/handshake | ~400 | CellApp ↔ BaseApp authentication and setup |
| Entity proxy forwarding | ~600 | Base entity forwarding client calls to Cell entity |

Roughly **~3,400 lines** of code exist solely to bridge the Base-Cell process boundary. This code adds latency (TCP round-trips between processes), complexity (entity proxy state machines), and failure modes (process disconnection handling) — for zero scaling benefit in a single-instance deployment.

---

## Realistic Scaling Requirements

### Target Population

| Scenario | Concurrent Players | Scaling Need |
|---|---|---|
| Developer testing | 1-5 | None |
| Friends and contributors | 5-20 | None |
| Small community server | 20-100 | None |
| Active private server | 100-500 | Marginal |
| Wildly successful revival | 500-2,000 | Maybe |
| Retail MMO scale (2009 target) | 2,000+ | Yes |

### Computation Budget

SGW's combat system is cooldown-based (not twitch action), NPC AI is state-machine-driven with infrequent updates, and most game logic runs in Python at tick intervals of 100-500ms. A rough estimate for a modern single-core:

| Operation | Per-Player Cost | At 500 Players |
|---|---|---|
| Position update processing | ~50 μs/tick | 25 ms |
| AoI query + entity sync | ~100 μs/tick | 50 ms |
| Combat/ability processing | ~200 μs/tick (in combat) | 20 ms (20% in combat) |
| NPC AI (per-NPC, ~2/sec) | ~100 μs/update | 30 ms (300 NPCs) |
| Python script execution | ~50 μs/event | 10 ms (200 events/sec) |

**Total**: ~135 ms per tick at 500 concurrent players — well within a 200ms tick budget on a single core. With multi-threaded zone processing, 1,000+ players is achievable without architectural changes.

### Key Observation

A cancelled 2009 MMO running as a community preservation project will almost certainly never need to handle more than a few hundred concurrent players. The scaling ceiling of a single-process server on modern hardware far exceeds any realistic player population.

---

## Scaling Strategies

Five strategies are presented in order from simplest to most complex. Each tier subsumes the previous.

### Tier 0: Single Process (Current Effective Architecture)

**What it is**: Collapse Base and Cell into one process, eliminating the inter-service TCP protocol. All zones run in a single process with shared memory.

**Capacity**: ~200-500 concurrent players on modest hardware, ~1,000+ on a dedicated server.

**Advantages**:
- Eliminates ~3,400 lines of inter-service protocol code
- Removes TCP round-trip latency for entity operations
- Simplifies deployment (one process instead of three for game logic)
- Easier to debug and profile
- No entity proxy state machine (direct method calls)

**Disadvantages**:
- Single point of failure (process crash loses all players)
- Cannot scale beyond one machine's resources
- All zones share a single Python GIL (if using CPython)

**When to use**: Development, testing, community servers up to ~500 players.

### Tier 1: Zone-Based Process Sharding

**What it is**: Each zone (or group of related zones) runs in its own process. A lightweight coordinator routes players between zone processes. Players transition between processes at natural loading-screen boundaries (stargate travel).

```
                    Coordinator / Login
                         |
         +---------------+---------------+
         |               |               |
   [Zone Group A]  [Zone Group B]  [Zone Group C]
   Castle_CellBlock  Harset          Menfa zones
   Castle            Harset_Cmd      Ihpet zones
   SGC_W1            Harset_Market   Dakara zones
   SGC               Harset_Storage
```

**Capacity**: Linear scaling — add more zone processes on more machines as needed.

**Why it works for SGW**: Stargate Worlds already has hard zone boundaries. Players cannot see across zone boundaries. Every zone transition involves a loading screen (stargate activation, ring transport, or beam). This means:
- No entity ghosting needed between zones
- No cell boundary migration needed
- Player hand-off is a clean serialize → transfer → deserialize at the loading screen
- Cross-zone communication (chat, mail, organizations) routes through shared services

**Implementation cost**: LOW. Requires:
- A zone-to-process mapping (config file or simple service registry)
- Player state serialization for zone transfer (already exists — the DB persistence layer does this)
- Shared services for cross-zone state (chat, organizations, mail) — a message broker or shared DB
- Instance management for instanced zones

**When to use**: If a single process can't handle the player load in aggregate, but individual zones aren't overcrowded.

### Tier 2: Instance-Based Scaling

**What it is**: Popular zones spawn multiple instances, each handling a capped number of players. Players see "Instance 1 (42/100)" in the UI and can switch instances.

**Why it works for SGW**: The `spaces.xml` file already marks 8 zones as instanced. The architecture already supports multiple instances of the same zone. Tutorial zones (Castle_CellBlock, SGC_W1) are natural 1-player instances. Popular hubs can overflow to new instances.

**Implementation cost**: LOW-MEDIUM. Requires:
- Instance spawning logic (create new zone instance when population exceeds threshold)
- Instance selection UI (client already has zone selection — may need minor extension)
- Instance-aware player transfer

**When to use**: When specific zones become overcrowded but overall server load is manageable.

### Tier 3: Channel/Layer System

**What it is**: A single zone appears to support many players, but they are silently distributed across invisible "channels" or "layers". Players in different channels can't see each other but share the same map. Group members are always in the same channel.

This is the approach used by Guild Wars 2, Star Wars: The Old Republic, and Elder Scrolls Online for overflow handling.

**Implementation cost**: MEDIUM. Requires:
- Channel assignment algorithm (respect groups, friends, organizations)
- Channel transfer protocol (player moves between channels seamlessly)
- Cross-channel visibility for certain entities (NPCs, world objects)
- Channel merge when population drops

**When to use**: If you want the illusion of a single large world without instance selection UI.

### Tier 4: Multi-Cell Distributed Architecture

**What it is**: The full BigWorld production model — cell splitting, entity ghosting, entity migration, CellAppMgr, BaseAppMgr.

**Implementation cost**: VERY HIGH. This is months of work and requires:
- CellAppMgr: zone-to-cell assignment, dynamic cell boundary management
- BaseAppMgr: player-to-BaseApp distribution by load
- Cell boundary logic: rectangular region ownership per CellApp
- Entity ghosting: mirror entities near cell boundaries to adjacent cells
- Entity migration: transfer entity ownership when crossing cell boundaries
- Ghost data filtering: send reduced property sets to ghost copies
- Reviver: restart crashed processes, restore state from checkpoints

**When to use**: Only if you need 2,000+ concurrent players in a single seamless open world. SGW's zone-based design with loading screens makes this architecture largely unnecessary — the natural zone boundaries eliminate the hardest problems (ghosting, migration, boundary queries).

---

## Recommendation

**Start with Tier 0** (single process). This is effectively what Cimmeria already is — just with unnecessary inter-service protocol overhead.

**Design for Tier 1** (zone-based sharding) by keeping zone logic cleanly isolated. This means:
- Each zone's state should be self-contained (no direct memory references to other zones)
- Player state should be serializable (already is — DB persistence does this)
- Cross-zone communication should go through explicit channels (chat, mail, organizations)
- Zone transitions should have a clean hand-off point (stargate travel already provides this)

**Don't build Tier 1 until you need it.** The jump from Tier 0 to Tier 1 is straightforward if the zone isolation is clean. Premature distribution adds complexity without benefit.

**Never build Tier 4** unless this project somehow attracts thousands of concurrent players. SGW's instanced/zoned world design means Tiers 1-3 can handle any realistic population, and they're orders of magnitude simpler to implement and maintain.

### Decision Matrix

| Question | Answer | Result |
|---|---|---|
| Are you in development or early testing? | Yes → | **Tier 0**: Single process |
| Do you have > 500 concurrent players? | No → | **Tier 0**: Single process |
| Is total server load too high for one machine? | Yes → | **Tier 1**: Zone sharding |
| Are specific zones overcrowded? | Yes → | **Tier 2**: Instancing |
| Do you want seamless overflow without UI? | Yes → | **Tier 3**: Channels |
| Do you need 2,000+ in one seamless world? | Yes → | **Tier 4**: Multi-cell (but SGW doesn't have a seamless world) |

---

## Comparison With Other Private Server Projects

For context, other MMO emulator projects have solved scaling pragmatically:

| Project | Game | Architecture | Peak Concurrent | Approach |
|---|---|---|---|---|
| MaNGOS/TrinityCore | World of Warcraft | Single process + DB | ~3,000+ | One world server, zone threading |
| SWGEmu | Star Wars Galaxies | Single process + zones | ~1,500 | Zone-threaded, no distributed cells |
| Ryzom Core | Ryzom | Process-per-shard | ~500 | Open-sourced original architecture |
| LOTRO private servers | Lord of the Rings Online | Single process | ~200 | Zone-based, instanced dungeons |
| ProjectSWG | Star Wars Galaxies | Java single process | ~500 | Zone-threaded |

None of these projects implemented distributed cell architectures. The most successful (TrinityCore) handles thousands of players in a single process with zone-level threading.

---

## Impact on Modernization Options

This analysis directly informs the server modernization discussion:

**For any rewrite or major refactor**: Collapsing to a single process is not losing scaling capability — it's removing dead code. The distributed infrastructure in Cimmeria handles exactly one BaseApp and one CellApp with zero ability to handle more.

**For the data-driven content engine**: A single-process design simplifies the content runtime. Mission scripts, effect handlers, and dialog trees can call game systems directly instead of routing through inter-service protocol.

**For future scaling**: Zone-based sharding (Tier 1) is a natural extension of SGW's already-zoned architecture. Designing clean zone isolation now makes scaling a straightforward configuration change later, not an architectural overhaul.

---

## Related Documents

- [Service Architecture](service-architecture.md) — Current Auth, Base, Cell service topology
- [Server-Only Infrastructure](server-systems.md) — Session management, rate limiting, admin tools
- [BigWorld Architecture](../engine/bigworld-architecture.md) — How production BigWorld handles distributed cells
- [Space Management](../engine/space-management.md) — WorldGrid AoI, spaces, ghost entities
- [Gap Analysis](../gap-analysis.md) — System-level feature completeness
