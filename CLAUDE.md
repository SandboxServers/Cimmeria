# Cimmeria - Stargate Worlds Emulator

## Project Overview

A server emulator for the Stargate Worlds MMO, implementing authentication, world simulation, entity management, and game logic via a distributed service architecture.

### Architecture

- **AuthenticationServer** - Player login, account management (port 13001)
- **BaseApp** - Persistent entity state, player base data, shard management (port 32832)
- **CellApp** - Spatial entity simulation, world cells, movement, AoI
- **NavBuilder** - Offline navigation mesh generation (Recast/Detour)
- **ServerEd** - Qt-based editor tool for server administration
- **UnifiedKernel** - Shared static library: networking (Boost.Asio), protocol, Mercury messaging

### Tech Stack (Current)

| Component | Version | Notes |
|---|---|---|
| MSVC Toolset | v145 (VS2026) | C++11 (migration from v120 complete) |
| Boost | 1.55.0 | Asio, Python, Thread, DateTime, Filesystem |
| Python | 3.4.1 | Embedded for entity scripting |
| PostgreSQL | 9.2.3 | Via SOCI 3.2.1 ORM |
| OpenSSL | 0.9.8i | Authentication encryption |
| Qt | 5.x (early) | ServerEd tool only |
| Recast/Detour | ~2013 era | Navigation meshes |
| TinyXML2 | ~1.x | Config/entity XML parsing |
| ICU | 51 | Unicode/i18n (bundled with Qt) |

### Key Directories

- `src/` - C++ source (UnifiedKernel, servers, NavBuilder)
- `projects/` - Visual Studio .vcxproj files
- `python/` - Python entity scripts and game logic (164 files)
- `entities/` - XML entity definitions and type registry
- `config/` - XML service configuration files
- `data/cache/` - Cooked game data (.pak files)
- `data/scripts/` - Effect, mission, and space scripts
- `db/` - PostgreSQL schema files (split structure: `db/database.sql`, `db/resources/`, `db/sgw/`)
- `docs/` - **110 documents** covering protocol, gameplay, engine, architecture, and RE findings
  - `docs/protocol/` - Mercury wire format, entity sync, login handshake, position updates
  - `docs/gameplay/` - 24 per-system gameplay breakdowns (combat, abilities, inventory, missions, etc.)
  - `docs/engine/` - BigWorld internals, CME framework, cooked data, space management
  - `docs/reverse-engineering/findings/` - 17 per-system wire format docs from Ghidra analysis
  - `docs/guides/` - Evidence standards, reading decompiled code, entity def guide
  - `docs/client/` - Game client analysis (launcher, tools)
  - `docs/tools/` - Development tool design docs (admin panel)
- `tools/ServerEd/` - Qt editor tool source
- `external/` - Vendored dependencies (NOT in git - see .gitignore)
- `bin64/` - Build output (NOT in git)
- `lib64/` - Library output (NOT in git)

### Build

Solution: `W-NG.sln` (Visual Studio 2026, MSVC v145)
Configurations: Debug, Release, UnoptRelease, MinSizeRel
Platforms: Win32, x64

Bootstrap dependencies:
1. `setup.ps1` - Full bootstrap: clone to running server (wraps CimmeriaBootstrap module)
2. `setup-dependencies.ps1` - Automated: download, patch, and build all external dependencies
3. Or manually: `bootstrap/init-boost.bat`, `bootstrap/build-boost.bat`, `bootstrap/build-openssl.bat`, `bootstrap/build-soci.bat`

### Config Notes

- Config files in `config/` contain default/template values
- `db_connection_string` in BaseService.config has test credentials - use `*.local` overrides for real environments
- Python console available on port 8989 (requires password to be set)
- Developer mode flag enables relaxed auth and elevated logging

---

## Agent Roster

### Current Stack Specialists

Agents with deep expertise in the exact dependency versions currently in use.

---

#### 1. C++ Server Core Agent

**Focus:** UnifiedKernel, server architecture, networking, protocol layer

**Expertise:**
- C++11 with VS2026 (v145) — migrated from v120, C++14/17 features available but not yet adopted
- Boost 1.55.0: Asio (async networking), Thread, Smart Pointers, Signals2, DateTime, Filesystem v3
- Custom UnifiedProtocol and Mercury reliable messaging framework
- Multi-service architecture (Auth, Base, Cell inter-service communication)
- Memory management patterns: Boost shared_ptr, scoped_ptr
- Precompiled headers (stdafx.hpp) and VS2026 project structure

**Key files:** `src/lib/`, `src/common/`, `src/server/`, `projects/UnifiedKernel/`

**When to use:** Any changes to server core, networking, protocol messages, inter-service communication, session management, or the kernel library.

---

#### 2. Python Game Logic Agent

**Focus:** Entity scripting, game mechanics, mission systems

**Expertise:**
- Python 3.4.1 embedded via Boost.Python
- Entity scripting patterns in the Cimmeria framework
- No f-strings, no async/await, no type hints (3.4 limitations)
- Entity lifecycle: creation, persistence, destruction
- Mission, interaction, minigame, and combat scripting
- NPC behavior and AI scripting
- Client-server entity property synchronization

**Key files:** `python/`, `entities/defs/`, `entities/entities.xml`

**When to use:** Game feature implementation, entity behavior, missions, interactions, minigames, NPC logic, or any gameplay scripting.

---

#### 3. Database & Persistence Agent

**Focus:** PostgreSQL schema, SOCI ORM layer, data persistence

**Expertise:**
- PostgreSQL 9.2.3 SQL dialect and features
- SOCI 3.2.1 C++ database abstraction (session management, prepared statements, row mapping)
- Schema design for MMO persistence (accounts, characters, items, missions, effects)
- Connection pooling and transaction management
- Entity state serialization/deserialization patterns

**Key files:** `db/database.sql`, `db/resources/`, `db/sgw/`, `src/lib/database/` (if present), config `db_connection_string`

**When to use:** Schema changes, query optimization, new persistent data types, database migration scripts, SOCI layer modifications.

---

#### 4. Entity System & Data Agent

**Focus:** Entity definitions, XML data pipeline, cooked data (.pak) files

**Expertise:**
- XML entity type definitions and registry (`entities.xml`, `spaces.xml`, `custom_alias.xml`)
- Entity definition schema: properties, methods, client/server/cell split
- TinyXML2 ~1.x parsing patterns
- Custom `.pak` file format for cooked game data
- Entity type hierarchy and inheritance
- Custom enumerations and type aliasing system

**Key files:** `entities/`, `data/cache/*.pak`, `data/scripts/`

**When to use:** Adding/modifying entity types, changing data schemas, updating cooked data pipelines, entity definition troubleshooting.

---

#### 5. Navigation & Spatial Systems Agent

**Focus:** Recast/Detour navmesh, world grid, spatial queries

**Expertise:**
- Recast ~2013 era (NavMesh v7) API for navmesh generation
- Detour pathfinding and navigation queries
- NavBuilder tool architecture and usage
- Spatial grid chunking system (configurable `grid_chunk_size`)
- Area of Interest (AoI) calculations (`grid_vision_distance`, `grid_hysteresis`)
- Cell-based world partitioning (cell_spaces.xml)
- Entity movement and position tracking

**Key files:** `src/server/NavBuilder/`, `projects/NavBuilder/`, `projects/Recast/`, `data/spaces/`, `entities/cell_spaces.xml`

**When to use:** Navigation issues, world space changes, movement bugs, AoI tuning, pathfinding, adding new navigable areas.

---

#### 6. Qt ServerEd Tool Agent

**Focus:** ServerEd editor tool, Qt 5.x UI development

**Expertise:**
- Qt 5.x (early release) Widgets, Network, Sql, Xml modules
- ServerEd architecture: config dialog, database worker, filesystem browser, node editor
- Qt .ui forms, .qrc resources, signal/slot patterns
- Database-backed object editor (objectdatabase.cpp)
- Visual node graph editor (qneblock/qneport/qneconnection)
- PostgreSQL integration via Qt5Sql + qsqlpsql driver

**Key files:** `tools/ServerEd/`

**When to use:** Editor tool features, admin UI, database browsing, visual scripting tools, any ServerEd changes.

---

#### 7. Build & DevOps Agent

**Focus:** Build system, dependency management, CI/CD, developer workflow

**Expertise:**
- Visual Studio 2026 (v145) .sln/.vcxproj project structure
- MSBuild configurations (Debug/Release/UnoptRelease/MinSizeRel x Win32/x64)
- CimmeriaBootstrap PowerShell module (setup.ps1, setup-dependencies.ps1)
- Boost bootstrap and build scripts (bootstrap/build-boost.bat, bootstrap/init-boost.bat)
- External dependency layout and linking (external/ directory structure)
- Library output management (lib64/debug, lib64/release)
- Static vs dynamic linking patterns in the project
- Windows-specific build considerations

**Key files:** `W-NG.sln`, `projects/**/*.vcxproj`, `setup.ps1`, `setup-dependencies.ps1`, `bootstrap/`

**When to use:** Build failures, adding new projects/libraries, dependency updates, CI/CD setup, build configuration changes.

---

#### 8. Network Security & Auth Agent

**Focus:** Authentication flow, encryption, network security

**Expertise:**
- OpenSSL 0.9.8i API (legacy, pre-1.0 interface)
- Authentication server protocol and session management
- Shard authentication key exchange
- Network packet encryption/decryption
- Client connection lifecycle and inactivity timeout handling
- Python console security (password-gated access on port 8989)

**Key files:** `src/server/AuthenticationServer/`, `projects/AuthenticationServer/`, `config/AuthenticationService.config`

**When to use:** Authentication bugs, security hardening, encryption changes, session management, login flow modifications.

---

### Migration Specialists

Agents specialized in upgrading specific dependencies from current versions to modern targets. Use these when planning or executing dependency upgrades.

---

#### 9. MSVC Toolchain Migration Agent

**Migration path:** VS2012 (v120) -> VS2026 (v145) — **COMPLETE**

**Status:** Migration to VS2026 (v145) is done. All 6 projects build successfully. See `memory/build-fixes.md` for patches applied.

**Expertise:**
- v120 -> v145 toolset changes and compatibility breaks
- C++11 -> C++17/C++20/C++23 incremental adoption strategy (available but not yet adopted)
- Compiler warning/error resolution across MSVC versions
- STL implementation changes (iterator debugging, allocator model, `std::auto_ptr` removal)
- Windows SDK version upgrades and API changes
- `.vcxproj` PlatformToolset migration and project file updates
- `/permissive-` conformance mode preparation
- Deprecation of legacy CRT functions (`_CRT_SECURE_NO_WARNINGS` patterns)

**Priority:** ~~HIGH~~ COMPLETE - Foundation migration done.

---

#### 10. Boost Migration Agent

**Migration path:** Boost 1.55.0 -> 1.90.0

**Expertise:**
- 35 minor releases of breaking changes and deprecations
- Boost.Asio evolution: standalone Asio option, executor model changes, completion token patterns
- Boost.Python API changes across versions
- Boost.Thread -> `std::thread` migration opportunities
- Boost.Filesystem v3 -> `std::filesystem` migration path
- Boost.Signals2 stability and any API drift
- Removed/reorganized libraries across the 1.55-1.90 range
- Header-only vs compiled library changes

**Priority:** HIGH - Core dependency touching every component.

---

#### 11. Python Embedding Migration Agent

**Migration path:** Python 3.4.1 -> 3.12+ (or 3.14 if stable)

**Expertise:**
- CPython embedding API changes (3.4 -> 3.12): `Py_Initialize`, module system, GIL changes
- Boost.Python compatibility with newer Python versions
- Python 3.4 removed features: `imp` module, old-style string formatting edge cases
- New features to adopt: f-strings (3.6+), dataclasses (3.7+), walrus operator (3.8+), match/case (3.10+)
- `asyncio` evolution (if server needs async Python)
- Type hint introduction strategy for existing 164-file scripting codebase
- Python DLL/library linking changes across versions
- Virtual environment and dependency isolation modernization

**Priority:** MEDIUM - Python 3.4 is EOL but scripting layer is somewhat isolated.

---

#### 12. PostgreSQL Migration Agent

**Migration path:** PostgreSQL 9.2.3 -> 17.x / 18.x

**Expertise:**
- Schema compatibility across 9 major versions
- New features to leverage: JSONB (9.4+), parallel queries (9.6+), partitioning (10+), logical replication
- `pg_dump`/`pg_restore` cross-version migration procedures
- SOCI 3.2.1 -> 4.1.2 migration (ORM layer must be upgraded in tandem)
- Connection string and authentication method changes (`md5` -> `scram-sha-256`)
- Extension availability and version compatibility
- Query planner improvements and index strategy updates
- libpq client library upgrade (bundled in external/)

**Priority:** MEDIUM - Current version works but is 9 major versions behind with no security patches.

---

#### 13. OpenSSL Migration Agent

**Migration path:** OpenSSL 0.9.8i -> 3.5.x

**Expertise:**
- CRITICAL: 0.9.8 has multiple known CVEs including Heartbleed-era vulnerabilities
- Complete API overhaul: `EVP_*` interface migration, provider model (3.0+)
- Removed functions: `SSLv2_*`, `SSLv3_*`, many low-level crypto functions
- `OPENSSL_init_ssl()` replacing `SSL_library_init()`
- Certificate and key loading API changes
- TLS 1.2/1.3 support enablement
- Library naming changes: `libeay32.dll`/`ssleay32.dll` -> `libcrypto.dll`/`libssl.dll`
- Build system changes (Configure -> CMake option)
- FIPS module availability (3.0+)

**Priority:** CRITICAL - Active security vulnerabilities. Should be the highest-priority migration after MSVC toolchain.

---

#### 14. Qt Migration Agent

**Migration path:** Qt 5.x (early) -> Qt 6.10

**Expertise:**
- Qt 5 -> Qt 6 porting guide application
- Build system migration: qmake -> CMake (Qt 6 standard)
- Removed/moved modules and classes
- `QString`/`QByteArray` behavior changes
- Signal/slot syntax modernization
- Qt5Sql -> Qt6Sql driver changes
- ICU 51 -> ICU 78 bundled with Qt upgrade
- High-DPI and accessibility improvements
- QML/Quick changes (if ServerEd expands)

**Priority:** LOW - Only affects ServerEd tool, not the game servers.

---

#### 15. Build System Modernization Agent

**Migration path:** .sln/.vcxproj -> CMake + vcpkg/Conan

**Expertise:**
- CMake project generation from existing VS solutions
- vcpkg manifest mode for dependency management (replaces external/ vendoring)
- Conan as alternative package manager
- Cross-platform build support (Linux server targets)
- CI/CD pipeline design (GitHub Actions, Azure DevOps)
- Precompiled header migration to CMake `target_precompile_headers`
- CTest integration for automated testing
- CPack for distribution packaging
- Docker containerization for server deployment

**Priority:** MEDIUM - Enables easier dependency management and CI/CD, but functional without it.

---

#### 16. Recast/Detour Migration Agent

**Migration path:** ~2013 era NavMesh v7 -> Recast 1.6.0

**Expertise:**
- Recast/Detour API evolution over the past decade
- NavMesh data format version changes (v7 -> current)
- Tile-based navmesh improvements
- Dynamic obstacle support additions
- Thread safety improvements
- CMake build integration (modern Recast uses CMake)
- NavMesh regeneration strategy for existing game data

**Priority:** LOW - Still industry-standard, API is relatively stable.

---

### Recommended Migration Order

```
Phase 1 (Foundation):
  1. MSVC Toolchain (v120 -> v145)       -- COMPLETE (VS2026)
  2. OpenSSL (0.9.8 -> 3.x)             -- critical security fix

Phase 2 (Core Libraries):
  3. Boost (1.55 -> 1.85+)              -- major dependency
  4. PostgreSQL + SOCI (9.2 -> 17+)     -- database layer

Phase 3 (Runtime & Scripting):
  5. Python (3.4 -> 3.12+)              -- scripting layer
  6. TinyXML2 (~1.x -> 11.x)            -- minor, low-risk
  7. ICU (51 -> 78)                      -- often bundled with Qt

Phase 4 (Tooling & Build):
  8. Qt (5.x -> 6.x)                    -- ServerEd only
  9. Recast/Detour (2013 -> 1.6)        -- low risk
  10. Build System (VS -> CMake+vcpkg)   -- modernization
```
