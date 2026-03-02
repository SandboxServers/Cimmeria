# Tech Stack Replacement Analysis

> **Last updated**: 2026-03-02
> **Sources**: Full codebase audit of `src/`, `python/`, `entities/`, `config/`, `db/`, `data/`

---

## Overview

This document evaluates whether to modernize the existing Cimmeria tech stack incrementally or replace it with a new foundation. The analysis examines what the current codebase actually does (vs what it appears to do), quantifies the replacement surface area, and presents five architectural options with a recommended path forward.

The core finding: Cimmeria is a **51K-line codebase** (25.6K C++, 25.4K Python) where the C++ layer is infrastructure plumbing and the Python layer is mechanical glue code. Neither layer contains complex algorithms. The real bottleneck is not server performance or code quality — it's **content authoring velocity**. 98.5% of missions have no scripts, 90% of items are orphaned, and 99.88% of effects have no runtime logic.

---

## Current Tech Stack

| Component | Version | Age | Status |
|---|---|---|---|
| MSVC Toolset | v145 (VS2026) | Current | Recently migrated from v120 |
| Boost | 1.55.0 | 2013 | 35 minor versions behind |
| Python | 3.4.1 | 2014 | EOL, no security patches |
| PostgreSQL | 9.2.3 | 2013 | 9 major versions behind |
| OpenSSL | 0.9.8i | 2008 | **CRITICAL** — known CVEs including Heartbleed-era vulnerabilities |
| SOCI | 3.2.1 | 2013 | ORM layer, must upgrade with PostgreSQL |
| TinyXML2 | ~1.x | 2012 | Functional, low risk |
| Recast/Detour | ~2013 | 2013 | API stable, low priority |
| Qt | 5.x (early) | 2013 | ServerEd tool only |

---

## What the Code Actually Does

### C++ Layer: 25,637 Lines (124 Files)

The C++ layer is pure infrastructure — networking, entity management, database access, and process lifecycle. It contains no game logic.

| Component | Lines | Role |
|---|---|---|
| Mercury transport | 5,202 | Reliable UDP with selective ACK, AES-256 encryption, packet bundling |
| BaseApp | 6,340 | Entity persistence, client proxy, account management, cell routing |
| CellApp | 4,721 | Spatial simulation, AoI queries, movement, entity grid |
| Entity system | 3,113 | Entity type registry, property serialization, cache stamps |
| Authentication | 1,894 | HTTP/SOAP login, shard selection, session keys |
| XML parsing | 2,020 | TinyXML2 (vendored), config and entity definition loading |
| NavBuilder | 909 | Recast/Detour integration for navmesh generation |
| Common/util/log | 1,382 | Logging, shared utilities |

**Key observation**: The Base-Cell inter-service protocol accounts for ~3,400 lines that exist solely to bridge two processes that always run as single instances. See [scaling-analysis.md](scaling-analysis.md) for why this complexity is unnecessary.

### Python Layer: 25,440 Lines (164 Files)

The Python layer is mechanical glue code between the C++ engine and game content. It contains no complex algorithms, no performance-critical paths, and no undocumented behaviors.

| Category | Lines (est.) | Files | Role |
|---|---|---|---|
| Auto-generated scripts | ~4,600 | 31 | Mission/effect scripts from Atrea Script Editor |
| Data definitions | ~7,600 | ~40 | Mission, Item, Ability, Effect classes — DB row to object mapping |
| Entity behavior hooks | ~10,200 | ~60 | Account, Player, Mob, Chat — simple RPC dispatch |
| Utility/helpers | ~3,040 | ~33 | Config, Logger, Constants, Event system |

**Auto-generated scripts** follow a rigid pattern:

```python
# Every mission script does exactly this:
def __init__(self):
    self.subscribe("entity.interact.tag::NPC_TAG", self.interactCb)
    self.subscribe("client_hinted_region::ZONE.Region", self.regionCb)

def interactCb(self, args):
    if self.player.missions.getMissionStatus(MISSION_ID) == STATUS:
        self.player.missions.advance(MISSION_ID, STEP_ID)
        # or: self.player.missions.accept(MISSION_ID)
        # or: self.player.missions.completeObjective(MISSION_ID, OBJ_ID)
```

**Entity behavior** delegates all real work to C++:

| Operation | Python does | C++ does |
|---|---|---|
| Damage calculation | Calls `effect.qrCombatDamage()`, reads result dict | QR math, stat lookup, resist calculation |
| Pathfinding | Calls `Atrea.findPath()` | Detour navmesh queries |
| Entity persistence | Calls `Atrea.dbQuery()` with SQL | SOCI prepared statements |
| Movement | Calls `entity.addWaypoint()` | Position interpolation, AoI updates |
| Resource delivery | Calls `Atrea.sendResource()` | PAK serialization, Mercury transport |

### Python-C++ Bridge API

The Atrea module exposes **26 functions** and **3 entity classes** to Python:

**Entity lifecycle**: `createBaseEntity`, `createBaseEntityFromDb`, `createCellEntity`, `destroyCellEntity`, `switchEntity`, `createCellPlayer`

**Entity queries**: `findEntity`, `findEntityOnSpace`, `findEntitiesByDbid`, `findEntities`, `findAllEntities`

**Space management**: `createSpace`, `destroySpace`, `findSpace`

**Database**: `dbPerform` (execute), `dbQuery` (query with results)

**Timers**: `addTimer`, `cancelTimer`, `getGameTime`

**Pathfinding**: `findPath`, `findDetailedPath`

**Other**: `log`, `sendResource`, `reloadClasses`, `registerMinigameSession`, `cancelMinigameSession`

**Entity classes**: `Vector3` (position/rotation), `BaseEntity` (persistence, disconnect), `CellEntity` (spatial: movement, space transitions, waypoints, caching)

This is a small, well-defined API surface. Any replacement only needs to implement these ~40 entry points.

### Protocol Layer

**Client-facing**: Mercury reliable UDP with 975 catalogued message types. Most follow a uniform RPC dispatch pattern — the client calls a named method with serialized arguments, the server deserializes and dispatches.

**Inter-service**: 33 message types between Base and Cell (23 Cell→Base, 10 Base→Cell). Protocol version 391 with CRC32 handshake authentication.

**Critical constraint**: Entity property IDs must be assigned in the exact same order as the client's parser. The assignment algorithm walks: parent entity → interfaces (in definition order) → own properties, excluding 5 internal properties. Getting this wrong causes deserialization failures. This algorithm is well-documented in [entity-property-sync.md](../protocol/entity-property-sync.md).

---

## The Real Problem

The server's tech debt is real but manageable. The existential problem is **content authoring velocity**.

From the [content data audit](../content/README.md):

| Content Type | Total | Functional | Rate |
|---|---|---|---|
| Missions | 1,040 | 16 scripted | 1.5% |
| Items | 6,059 | 595 connected | 9.8% |
| Effects | 3,216 | 4 scripted | 0.12% |
| Abilities | 1,886 | ~172 connected | 9.1% |
| Dialogs | 5,411 | ~50 connected | ~0.9% |
| Loot tables | 2 | 2 (3 entries) | N/A |
| Zones | 24 | 2 playable | 8.3% |
| Archetypes | 8 | 2 implemented | 25% |

Every mission, effect, and complex interaction currently requires a Python script. The original developers used the Atrea Script Editor (a visual node-graph tool) to generate these scripts, but that tool ran as part of the game client's editor mode — we don't have a working version.

The auto-generated scripts follow rigid mechanical patterns. They don't need to be Python scripts — they could be data tables interpreted at runtime. A data-driven approach would turn content creation from "write Python code" into "insert database rows," dramatically accelerating the path from 1.5% to meaningful mission coverage.

---

## Five Options

### Option A: Incremental Upgrade (Current CLAUDE.md Plan)

Upgrade each dependency in place following the migration order defined in CLAUDE.md:

```
MSVC (done) → OpenSSL → Boost → PostgreSQL/SOCI → Python → TinyXML2 → Qt → Recast → CMake
```

**Effort**: 3-6 months
**Risk**: LOW — each step is isolated and testable
**Result**: Same architecture, modern dependencies, security fixes

**Pros**:
- Lowest risk — working server stays working throughout
- Each upgrade is independently valuable (especially OpenSSL)
- Well-understood migration paths for each dependency
- No architectural changes required

**Cons**:
- Does not address content authoring bottleneck
- Boost 1.55→1.90 is a large jump with many breaking changes
- Python 3.4→3.12+ requires Boost.Python compatibility work
- End result is the same architecture with the same limitations
- Still requires Python scripts for every mission/effect

**Best for**: Risk-averse path, buys time for a longer-term strategy.

### Option B: Modern C++ Rewrite-in-Place

Keep the C++ architecture but modernize the codebase: replace Boost.Asio with standalone Asio or C++20 coroutines, replace Boost.Python with pybind11, replace SOCI with a modern ORM or raw libpq, replace Boost.Thread with std::thread.

**Effort**: 2-4 months
**Risk**: MEDIUM — touching every compilation unit
**Result**: Cleaner C++, same architecture, same scripting model

**Pros**:
- Reduces dependency surface (many Boost uses → stdlib)
- Modern C++ idioms (move semantics, smart pointers, RAII)
- pybind11 is header-only and supports Python 3.12+
- Standalone Asio tracks Boost.Asio closely

**Cons**:
- Still doesn't address content authoring
- Touching every file risks introducing regressions
- C++ compilation times remain high
- Developer pool for C++ server code is small

**Best for**: Teams comfortable with C++ who want a cleaner foundation.

### Option C: Rust Core + Python Scripts

Rewrite the C++ infrastructure layer in Rust. Keep the Python scripting layer via PyO3.

**Effort**: 3-5 months
**Risk**: MEDIUM-HIGH — new language, new toolchain
**Result**: Memory-safe protocol implementation, same scripting model

**Pros**:
- Memory safety eliminates an entire class of server crash bugs
- Rust's async ecosystem (tokio) handles networking elegantly
- PyO3 provides Python embedding comparable to Boost.Python
- Strong pattern matching for protocol parsing
- Growing game server ecosystem in Rust

**Cons**:
- Rust learning curve for contributors
- Smaller contributor pool than C++ or C#
- Still doesn't address content authoring (Python scripts remain)
- PyO3 + CPython GIL limits Python parallelism
- Build toolchain change (cargo vs MSBuild)

**Best for**: Teams that value correctness guarantees and have Rust experience.

### Option D: C# / .NET + Data-Driven Engine (Recommended)

Replace both C++ and Python layers with a C# server built around a data-driven content engine. Mission logic, effect behavior, and dialog flow are defined as data (database rows or JSON), not scripts. A runtime interpreter executes them.

**Effort**: 4-6 months for full replacement, but can be phased
**Risk**: MEDIUM — new language, but well-understood patterns
**Result**: Modern server with dramatically faster content authoring

**Pros**:
- **Solves the content bottleneck** — missions become data, not code
- Large contributor pool (C# is the dominant game server language)
- .NET 8+ has excellent async networking (Kestrel, System.IO.Pipelines)
- Strong PostgreSQL support (Npgsql, Dapper, EF Core)
- Single language for all server code (no C++/Python split)
- Hot-reload for game logic during development
- Excellent tooling (Visual Studio, Rider, dotnet CLI)
- Cross-platform (Linux server deployment via .NET)

**Cons**:
- Full rewrite — nothing from C++ or Python carries over directly
- Must reimplement Mercury protocol exactly (property ID assignment is critical)
- GC pauses could cause tick jitter (mitigable with pooling)
- Data-driven engine design requires upfront architecture work

**Data-driven content model**:

Instead of a Python script per mission:

```sql
-- Mission 622 "Arm Yourself!" becomes data rows:
INSERT INTO mission_triggers (mission_id, event_type, event_key, action, params) VALUES
(622, 'dialog.open',  '3995', 'complete_mission', '{"mission_id": 622}'),
(622, 'mission.complete', '622', 'add_item',      '{"item_id": 3347, "container": "equipment"}'),
(622, 'mission.complete', '622', 'accept_mission', '{"mission_id": 638}');
```

A generic runtime interpreter processes these rows — no per-mission code needed. This pattern covers ~95% of mission scripts (the mechanical subscribe→check→advance pattern). The remaining 5% with truly custom logic can use a lightweight scripting language (Lua, C# scripting, or a node-graph interpreter).

**Best for**: Maximizing content authoring velocity and long-term maintainability.

### Option E: Hybrid — Keep C++ Protocol, New Game Server via IPC

Keep the existing C++ BaseApp/CellApp as a thin protocol proxy. Build a new game server in any language that communicates with the proxy via IPC (shared memory, Unix sockets, or a message broker). The C++ layer handles Mercury protocol and client connections; the new server handles all game logic.

```
Client ←→ [C++ Protocol Proxy] ←→ [IPC] ←→ [New Game Server]
                Mercury UDP                    Any language
                Entity serialization           Game logic
                AoI queries                    Content engine
```

**Effort**: 2-3 months for the bridge + game server
**Risk**: LOW-MEDIUM — C++ protocol stays proven, new code is isolated
**Result**: Proven protocol handling + modern game logic layer

**Pros**:
- Mercury protocol stays exactly as-is (the hardest part to reimplement)
- Game server can be any language (C#, Go, Rust, Python)
- Clean separation of concerns
- Incremental — start with one system (missions), expand
- Can run old Python logic alongside new game server during transition

**Cons**:
- IPC adds latency (minor for game tick rates)
- Two processes to deploy and monitor
- API surface between proxy and game server must be designed
- Debugging spans two processes and potentially two languages

**Best for**: De-risking the protocol reimplementation while gaining a modern game logic layer.

---

## Comparison Matrix

| Criterion | A: Incremental | B: Modern C++ | C: Rust | D: C#/.NET | E: Hybrid |
|---|---|---|---|---|---|
| **Content velocity** | No change | No change | No change | **Major improvement** | Improvement (depends on game server) |
| **Security fixes** | Yes (OpenSSL) | Yes | Yes | Yes | Partial (proxy keeps old OpenSSL until upgraded) |
| **Risk** | Low | Medium | Medium-High | Medium | Low-Medium |
| **Effort** | 3-6 months | 2-4 months | 3-5 months | 4-6 months | 2-3 months |
| **Contributor pool** | Small (C++) | Small (C++) | Small (Rust) | **Large (C#)** | Mixed |
| **Protocol risk** | None | Low | Medium | Medium | **None** |
| **Cross-platform** | No (MSVC) | Possible | Yes | **Yes (.NET)** | Partial |
| **Debugging ease** | Good | Good | Learning curve | **Excellent** | Split across processes |
| **Long-term ceiling** | Low | Medium | High | **High** | Medium-High |

---

## Recommended Path

Option D (C#/.NET + data-driven engine) has the highest long-term payoff, but it's also the largest upfront investment. The pragmatic approach combines elements of multiple options into a phased strategy:

### Phase 0: Critical Security Fix (1-2 weeks)

Upgrade OpenSSL 0.9.8i → 3.x regardless of all other decisions. This is a standalone change with no architectural implications. The current version has known exploitable vulnerabilities.

This is Option A applied to the single most critical dependency.

### Phase 1: Data-Driven Mission Runtime in Python (2-4 weeks)

Without touching the C++ layer, build a **data-driven mission interpreter** in Python that reads mission trigger definitions from PostgreSQL and executes them at runtime. This replaces the need for per-mission Python scripts.

```python
# Instead of 1,040 individual mission scripts:
class MissionInterpreter:
    def __init__(self, player):
        self.triggers = load_triggers_from_db(player.active_missions)
        for trigger in self.triggers:
            self.subscribe(trigger.event, self.on_event)

    def on_event(self, event_type, event_key, **kwargs):
        for trigger in self.triggers.match(event_type, event_key):
            self.execute_action(trigger.action, trigger.params)
```

This proves the data-driven model works within the existing architecture. If it works, content creators can start populating missions via database rows immediately.

**Validates**: Data-driven content model feasibility
**Delivers**: Immediate content authoring improvement

### Phase 2: Expand Data-Driven Engine (4-8 weeks)

Extend the data-driven approach to effects, dialog gating, spawn management, and loot tables — all within the existing Python scripting layer.

| System | Current | Data-Driven |
|---|---|---|
| Missions | 1 Python script per mission | DB trigger rows + generic interpreter |
| Effects | 1 Python script per effect | DB parameter tables + generic pulse handler |
| Dialog gating | Hardcoded in space scripts | DB condition rows on dialog_set_maps |
| Loot | 3 entries in 2 tables | Generated loot tables from item + mob level data |
| Spawns | Empty Python shells | DB spawn definitions + generic spawner |

This phase addresses the content bottleneck without any C++ changes.

**Validates**: Full data-driven game logic architecture
**Delivers**: Massive content authoring acceleration

### Phase 3: Evaluate Full Replacement (2-4 months, if pursued)

With the data-driven content model proven in Phase 2, evaluate whether a full C# rewrite is justified. At this point:

- The content model is validated and populated with real data
- The protocol is fully documented (975 messages, property ID assignment algorithm)
- The API surface is small (26 C++ functions, 33 inter-service messages)

If yes, the C# server reimplements:
1. Mercury reliable UDP protocol (~5.2K lines of C++ → well-documented, mechanical translation)
2. Entity property serialization (critical — must match client parser exactly)
3. The data-driven content engine (port from Python, likely cleaner in C#)
4. Database persistence (Npgsql/Dapper replacing SOCI)
5. HTTP/SOAP authentication (straightforward in ASP.NET)

If no (the Python data-driven engine is sufficient), continue with Option A incremental upgrades to modernize the remaining dependencies around the working system.

### Phase 3 Alternative: Option E Hybrid

If the Mercury protocol reimplementation feels too risky, use Option E instead: keep the C++ protocol proxy and build the C# game server behind it. This eliminates protocol risk while still gaining a modern game logic layer.

---

## LLM-Assisted Content Generation

Regardless of which option is chosen, the data-driven content model enables a powerful force multiplier: **LLM-assisted content generation**.

The [content data audit](../content/README.md) identified ~300 missions as SCRIPTABLE — they have rich step text describing what the player should do ("Speak to Commander Mitchell", "Retrieve the artifact from the lab", "Return to the gate room"). An LLM can:

1. Parse mission step text to extract NPC names, locations, and objectives
2. Cross-reference against entity templates and zone data
3. Generate mission trigger rows (or Python scripts, for the current system)
4. Generate dialog condition rows linking missions to NPC interactions
5. Generate loot table entries based on item levels and mob difficulty

This turns months of manual content authoring into days of supervised generation + review. The data-driven model makes this practical because the output is structured data (database rows), not arbitrary code.

---

## Protocol Reimplementation Feasibility

The Mercury protocol is the highest-risk component to reimplement. Assessment by target language:

### What Must Be Reimplemented

| Component | Complexity | Notes |
|---|---|---|
| UDP socket management | Low | Standard networking |
| Reliable sequencing | Medium | Selective ACK, sequence numbers, retransmission |
| Packet bundling | Medium | Multiple messages per packet, backward footer parsing |
| AES-256-CBC encryption | Low | Standard crypto library call |
| Entity property serialization | **High** | Property ID assignment must match client exactly |
| RPC dispatch | Medium | Named method → handler routing |
| AoI entity sync | Medium | Enter/leave notifications, property delta compression |
| Position updates | Medium | 32 avatarUpdate variants with packed formats |
| HTTP/SOAP login | Low | Standard HTTP + XML parsing |

### Feasibility by Language

| Language | Protocol Feasibility | Ecosystem | Notes |
|---|---|---|---|
| **C#/.NET** | HIGH | System.Net.Sockets, System.Security.Cryptography | .NET has excellent low-level socket APIs. BitConverter and Span<T> handle binary parsing well. |
| **Rust** | HIGH | tokio, ring/rustls | Rust's type system naturally models packet structures. Pattern matching for message dispatch. |
| **Go** | MODERATE | net, crypto/aes | Go's networking is strong but binary protocol parsing is verbose. No generics until 1.18. |
| **Python** | LOW-MODERATE | asyncio, struct | Possible but performance concerns at scale. struct module for binary parsing. |
| **Java/Kotlin** | MODERATE | Netty, JCA | Netty handles UDP well. ByteBuffer for binary parsing. |

### The Hard Part: Entity Property ID Assignment

The single most critical correctness requirement. The client's parser assigns property IDs by walking:

1. Parent entity type properties
2. Interface properties (in XML definition order)
3. Own properties
4. Excluding 5 internal BigWorld properties (`position`, `direction`, `spaceID`, `vehicleID`, `cellData`)

If the server assigns different IDs than the client expects, property updates silently apply to wrong fields. This algorithm is documented in [entity-property-sync.md](../protocol/entity-property-sync.md) and [entity-property-sync.md](../reverse-engineering/findings/entity-property-sync.md).

Any reimplementation must either:
- Replicate this algorithm exactly (reading the same entity .def XML files)
- Generate a static property ID table from the .def files (compile-time safety)
- Use the existing C++ implementation as a reference test oracle

---

## Codebase Portability Assessment

### What Ports Easily

| Component | Why |
|---|---|
| Entity definitions | XML files, language-agnostic |
| Database schema | PostgreSQL SQL, direct reuse |
| Game data | 112K rows in resources.sql, direct reuse |
| Python game logic | Small API surface (26 functions), mechanical patterns |
| Configuration | XML files, language-agnostic |
| Protocol documentation | 420 messages fully documented with wire formats |
| Navmesh data | Binary .nav files, Recast/Detour has C# and Rust bindings |

### What Requires Careful Translation

| Component | Why |
|---|---|
| Mercury protocol | Must match client's binary parser exactly |
| Entity property serialization | Property ID assignment is critical |
| AoI system | Spatial queries, entity grid, vision distance |
| Cooked data (.pak) | Custom binary format, must match client's reader |

### What Can Be Dropped

| Component | Why |
|---|---|
| Base-Cell inter-service protocol | Only serves single-instance deployment (see [scaling-analysis.md](scaling-analysis.md)) |
| Boost dependency | Replace with language stdlib or modern alternatives |
| SOCI ORM | Replace with language-native PostgreSQL driver |
| Boost.Python bridge | Replace with native scripting or data-driven engine |

---

## Cost-Benefit Summary

| Approach | Fixes Security | Fixes Content | Effort | Long-term Value |
|---|---|---|---|---|
| Do nothing | No | No | 0 | Decay |
| Phase 0 only (OpenSSL) | **Yes** | No | 1-2 weeks | Minimal |
| Phase 0 + 1 (OpenSSL + data-driven missions) | **Yes** | **Partial** | 3-6 weeks | Good |
| Phase 0 + 1 + 2 (full data-driven engine) | **Yes** | **Yes** | 2-3 months | **High** |
| Phase 0 + 1 + 2 + 3 (C# rewrite) | **Yes** | **Yes** | 5-8 months | **Highest** |
| Option A full (all dependency upgrades) | **Yes** | No | 3-6 months | Moderate |

The recommended minimum investment is **Phase 0 + 1** (OpenSSL fix + data-driven missions). This addresses the critical security vulnerability and begins solving the content bottleneck with minimal architectural risk, all within 3-6 weeks.

---

## Related Documents

- [Scaling Analysis](scaling-analysis.md) — Why single-process is sufficient, 5-tier scaling roadmap
- [Service Architecture](service-architecture.md) — Current Auth, Base, Cell topology
- [Content Data Audit](../content/README.md) — What content exists and what's missing
- [Gap Analysis](../gap-analysis.md) — 369 features across 38 systems
- [Mercury Wire Format](../protocol/mercury-wire-format.md) — Protocol internals
- [Entity Property Sync](../protocol/entity-property-sync.md) — Property ID assignment algorithm
- [Reconstruction Map](../content/reconstruction-map.md) — What can be rebuilt from existing data
