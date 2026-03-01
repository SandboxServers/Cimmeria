# Cimmeria — Stargate Worlds Server Emulator

A server emulator for [Stargate Worlds](https://en.wikipedia.org/wiki/Stargate_Worlds), the cancelled Stargate MMO developed by Cheyenne Mountain Entertainment. The game was built on [BigWorld Technology](https://en.wikipedia.org/wiki/BigWorld) (networking/server) and Unreal Engine 3 (rendering/client), and reached a playable beta before the studio shut down in 2010.

Cimmeria reimplements the server infrastructure — authentication, world simulation, entity management, and game logic — allowing the original game client to connect and play.

### Current State

The server **infrastructure** is 75–80% complete: Mercury networking, entity lifecycle, database persistence, authentication, navigation meshes, and Area of Interest all work. Players can log in, create characters, enter the world, move around, interact with NPCs, accept missions, and engage in basic combat.

**What works:**
- Full login/auth flow and character creation
- World entry, movement, entity spawning and despawn
- NPC interactions and dialog trees
- Basic combat (abilities, effects, damage, death/respawn)
- Inventory and item management
- Chat channels and private messaging
- Mission accept/advance (scripted missions)

**What's missing or incomplete:**
- **Content**: Only a handful of the game's zones and missions are wired up; most world content still needs scripting
- **XP and leveling**: Level progression and XP curve tuning
- **Enemy AI**: Mobs spawn but lack patrol paths, aggro leashing, and sophisticated combat behavior
- **Crafting**: Not implemented (0%) — disciplines, blueprints, research, alloys
- **Stargate travel**: Partially implemented (~20%) — DHD UI works but gate transitions are incomplete
- **Organizations/Guilds**: Stub only (~5%)
- **Minigames**: Framework exists but none of the 5+ minigames are functional
- **Mail, auction house, trading, duels, pets**: Not yet implemented
- **Stat scaling and balance**: Base stats work but derived stats, diminishing returns, and combat formulas need validation against the original client

See [docs/project-status.md](docs/project-status.md) for a detailed breakdown and [docs/](docs/readme.md) for full documentation.

## Architecture

```
┌─────────────┐     ┌──────────┐     ┌──────────┐
│    Client    │────>│   Auth   │────>│ BaseApp  │
│  (SG:W Game) │     │  Server  │     │          │
└─────────────┘     │ :13001   │     │ :32832   │
                    └──────────┘     └────┬─────┘
                                         │
                                    ┌────┴─────┐
                                    │ CellApp  │  (one per world cell)
                                    │          │
                                    └──────────┘
```

- **AuthenticationServer** — Player login, account management, shard authentication
- **BaseApp** — Persistent entity state, player base data, shard management
- **CellApp** — Spatial entity simulation, world cells, movement, Area of Interest
- **NavBuilder** — Offline navigation mesh generation (Recast/Detour)
- **ServerEd** — Qt-based editor tool for server administration
- **UnifiedKernel** — Shared static library: networking (Boost.Asio), protocol, Mercury messaging

## Getting Started

### 1. Clone the repository

```bash
git clone <repo-url> Cimmeria
cd Cimmeria
```

### 2. Bootstrap dependencies

```powershell
pwsh setup-dependencies.ps1
```

This downloads, patches, and builds all external dependencies automatically.
See [bootstrap/README.md](bootstrap/README.md) for details and options.

**Requirements:** Windows 10/11, PowerShell 7+, Visual Studio with C++ tools.
The script will detect (or optionally install) VS for you.

### 3. Build the solution

Open `W-NG.sln` in Visual Studio and build `Debug|x64`.

Or from a VS Developer Command Prompt:

```cmd
msbuild W-NG.sln /p:Configuration=Debug /p:Platform=x64
```

## Project Structure

```
Cimmeria/
├── src/                    C++ source code
│   ├── lib/                UnifiedKernel (shared library)
│   ├── common/             Shared utilities
│   └── server/
│       ├── AuthenticationServer/
│       ├── BaseApp/
│       ├── CellApp/
│       └── NavBuilder/
├── python/                 Python entity scripts and game logic (164 files)
├── entities/               XML entity definitions and type registry
├── config/                 XML service configuration files
├── data/
│   ├── cache/              Cooked game data (.pak files)
│   └── scripts/            Effect, mission, and space scripts
├── db/                     PostgreSQL schema files
├── projects/               Visual Studio .vcxproj files
├── tools/ServerEd/         Qt editor tool source
├── bootstrap/              Dependency setup automation
│   ├── patches/            VS2015+ compatibility patches
│   ├── templates/          Generated config templates
│   ├── init-boost.bat      Bootstrap Boost.Build (b2)
│   ├── build-boost.bat     Build Boost libraries
│   ├── build-openssl.bat   Build OpenSSL libraries
│   └── build-soci.bat      Build SOCI libraries
├── external/               Vendored dependencies (not in git)
├── bin64/                  Build output (not in git)
├── lib64/                  Library output (not in git)
├── W-NG.sln                Visual Studio solution
└── setup-dependencies.ps1  Automated dependency bootstrap
```

## Tech Stack

| Component | Version | Notes |
|---|---|---|
| MSVC Toolset | v145 (VS2026) | C++11 codebase, modern compiler |
| Boost | 1.55.0 | Asio, Python, Thread, DateTime, Math |
| Python | 3.4.1 | Embedded via Boost.Python for entity scripting |
| PostgreSQL | 9.2.x | Via SOCI 3.2.1 ORM |
| OpenSSL | 1.0.1e | Authentication encryption |
| Qt | 5.x | ServerEd tool only |
| Recast/Detour | ~2013 era | Navigation meshes |
| SDL | 1.2.15 | Audio/input (client-side remnant) |

## Configuration

Config files live in `config/` with `.config` extension (XML format). Default
values are suitable for local development. For real deployments, create
`*.local` override files (not checked into git).

Key config files:
- `AuthenticationService.config` — Auth server ports, encryption settings
- `BaseService.config` — Database connection, shard settings
- `CellAppService.config` — World cell parameters, AoI distances

The Python debug console is available on port 8989 (password-gated).

## Database

PostgreSQL schemas are in `db/`:
- `sgw.sql` — Main game schema (accounts, characters, items, missions)
- `resources.sql` — Resource/asset tracking

Connection string is configured in `BaseService.config`.

## Build Configurations

| Configuration | Use |
|---|---|
| Debug | Development — full symbols, debug CRT, assertions enabled |
| Release | Production — optimized, no debug overhead |
| UnoptRelease | Debugging release builds — release CRT but no optimization |
| MinSizeRel | Minimize binary size |

Platform: `x64` (primary), `Win32` (legacy)

## Documentation

The [docs/](docs/readme.md) directory contains **89 documents** covering every aspect of the project:

| Category | Docs | Covers |
|----------|------|--------|
| [Top-level](docs/readme.md) | 9 | Technology overview, game systems survey, connection flow, project status |
| [protocol/](docs/protocol/) | 5 | Mercury wire format, entity property sync, login handshake, position updates |
| [gameplay/](docs/gameplay/) | 18 | Per-system breakdowns: combat, abilities, inventory, missions, crafting, etc. |
| [engine/](docs/engine/) | 8 | BigWorld internals, CME framework, cooked data, space management, LOD, checkpointing |
| [architecture/](docs/architecture/) | 1 | Cimmeria service topology, inter-service protocol, configuration reference |
| [analysis/](docs/analysis/) | 2 | Event-net mapping (420 messages), BigWorld source cross-reference index |
| [reverse-engineering/](docs/reverse-engineering/) | 21 | Ghidra annotation scripts, RE plan/status, 17 per-system wire format findings |
| [guides/](docs/guides/) | 3 | Evidence standards, reading decompiled code, entity definition guide |
| [technical/](docs/technical/) | 16 | Legacy analysis documents from initial RE investigation |

**Start here:**
- **[How SGW Works](docs/how-sgw-works.md)** — Technology overview of the BigWorld + UE3 hybrid architecture
- **[Game Systems](docs/game-systems.md)** — Every game feature: combat, abilities, stargates, missions, crafting
- **[Connection Flow](docs/connection-flow.md)** — End-to-end login and world entry sequence
- **[Project Status](docs/project-status.md)** — What works, what's left, and the roadmap

For reverse engineering work, see [docs/reverse-engineering/](docs/reverse-engineering/PLAN.md).

## License

This project is a server emulator for research and preservation purposes.
