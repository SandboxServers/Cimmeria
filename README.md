# Cimmeria — Stargate Worlds Server Emulator

A server emulator for [Stargate Worlds](https://en.wikipedia.org/wiki/Stargate_Worlds), the cancelled Stargate MMO developed by Cheyenne Mountain Entertainment. The game was built on [BigWorld Technology](https://en.wikipedia.org/wiki/BigWorld) (networking/server) and Unreal Engine 3 (rendering/client), and reached a playable beta before the studio shut down in 2010.

Cimmeria reimplements the server infrastructure — authentication, world simulation, entity management, and game logic — allowing the original game client to connect and play.

## Status

The project tracks **369 features** across 38 systems. **47% have code** (175 of 369). See the [Gap Analysis](docs/gap-analysis.md) for the full breakdown.

**Tested end-to-end with the game client:**
- Login and authentication (HTTP SOAP → shard select → Mercury UDP)
- Mercury reliable UDP transport with AES-256 encryption
- Game data pipeline (22 resource categories, 112,626 DB rows)
- World entry, entity spawning, grid-based Area of Interest
- One-command build and setup

**Code exists, needs verification:**
Character creation (8 archetypes, 23 defs) | Inventory | Vendors | Chat | Crafting | Trading

**Implemented with known gaps:**
Combat & abilities | Effects | Missions | NPC AI | Stats & leveling | Stargate travel

See [docs/project-status.md](docs/project-status.md) for the detailed breakdown.

## Why Rust

The original C++ server was built against a 2013-era dependency stack: Boost 1.55, Python 3.4, OpenSSL 1.0.1e (with known CVEs including Heartbleed), PostgreSQL 9.2, SOCI 3.2 — all end-of-life, all tightly coupled. Upgrading any single dependency triggers cascading breaks across the others. The build requires Visual Studio on Windows with precompiled headers and a specific MSVC toolset. The Python 3.4 embedding layer (via Boost.Python) is especially fragile — no type hints, no f-strings, no async, and Boost.Python itself hasn't tracked CPython's embedding API changes.

Rather than fight through 10 phases of dependency upgrades to modernize a codebase that would still be C++11 at the end, we're rewriting the server in Rust:

- **Single toolchain** — `cargo build` on any platform, no dependency bootstrapping
- **Memory safety** — no use-after-free, no buffer overflows, no manual memory management
- **Modern async** — Tokio for networking instead of Boost.Asio callback chains
- **Protocol parity** — the Mercury protocol crate already handles encryption, packet framing, and reliable delivery
- **Cross-platform** — Linux, Windows, macOS from the same source

The C++ and Python code remains as the **reference implementation** — it's the ground truth for how the original server behaved, and we match its wire behavior exactly. But active development happens in Rust.

## Quick Start

### Rust Server

```bash
cargo run -p cimmeria-server
```

Handles login, Mercury protocol, character select, and world entry. Connect the game client to `localhost`.

**Test account:** `test` / `test`

### C++ Server (legacy)

```powershell
pwsh setup.ps1
```

Full pipeline: download dependencies → build → database → launch. Requires Windows 10/11, PowerShell 7+, Visual Studio with C++ tools. See [bootstrap/CimmeriaBootstrap/README.md](bootstrap/CimmeriaBootstrap/README.md) for options.

## Architecture

```
                          ┌─────────────────┐
                          │   Game Client    │
                          │  (UE3 + BigWorld)│
                          └────────┬────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │               │
              HTTP :8081    Mercury UDP      Mercury UDP
                    │         :32832              :?
                    ▼              ▼               ▼
            ┌──────────┐   ┌──────────┐    ┌──────────┐
            │   Auth   │──▶│ BaseApp  │───▶│ CellApp  │
            │  Server  │   │          │    │          │
            └──────────┘   └──────────┘    └──────────┘
            Login, accounts  Entities,      World cells,
            Shard auth       persistence    movement, AoI
```

- **AuthenticationServer** — HTTP/SOAP login, account management, shard key exchange
- **BaseApp** — Persistent entity state, player data, character management
- **CellApp** — Spatial simulation, world cells, movement, Area of Interest
- **NavBuilder** — Offline navigation mesh generation (Recast/Detour)
- **ServerEd** — Qt-based server administration editor

## Crate Dependency Graph

```
                    ┌──────────┐
                    │  common  │  (no deps on other crates)
                    └────┬─────┘
                         │
              ┌──────────┼───────────┐
              ▼          ▼           ▼
         ┌────────┐ ┌────────┐ ┌──────────┐
         │mercury │ │  defs  │ │ commands │
         └───┬────┘ └───┬────┘ └────┬─────┘
             │          │           │
             ▼          ▼           │
         ┌───────────────────┐      │
         │      entity       │◄─────┘
         └────────┬──────────┘
                  │
         ┌────────┼──────────────┐
         ▼        ▼              ▼
    ┌────────┐ ┌────────────────┐│
    │  game  │ │content-engine  ││
    └───┬────┘ └───────┬────────┘│
        │              │         │
        ▼              ▼         ▼
    ┌──────────────────────────────┐
    │          services            │
    └──────────────┬───────────────┘
                   │
          ┌────────┼────────┐
          ▼        ▼        ▼
    ┌────────┐┌─────────┐┌─────────┐
    │ server ││admin-api││src-tauri│
    └────────┘└─────────┘└─────────┘
```

Each box is a separate Rust crate that compiles independently. **common** has the basic types everything needs. **mercury** handles the BigWorld reliable UDP protocol and AES-256 encryption. **defs** parses entity definitions from XML. **entity** manages game objects. **game** and **content-engine** implement gameplay rules and the data-driven content pipeline. **services** ties it all together into Auth, Base, and Cell services. The bottom row are entry points: **server** is the headless game server, **admin-api** exposes a REST API, and **src-tauri** wraps it in a desktop GUI.

## Project Structure

```
Cimmeria/
├── crates/                 Rust server (active development)
│   ├── common/             Shared types, config, error handling
│   ├── mercury/            Mercury reliable UDP + AES-256 encryption
│   ├── defs/               Entity definition parser (XML → Rust types)
│   ├── entity/             Entity system (lifecycle, properties)
│   ├── commands/           Server command framework
│   ├── game/               Game mechanics and rules
│   ├── content-engine/     Data-driven content pipeline
│   ├── services/           Auth, Base, Cell service implementations
│   ├── admin-api/          REST administration API
│   └── server/             Binary entry point (cargo run -p cimmeria-server)
├── src/                    C++ server (legacy reference implementation)
│   ├── lib/                UnifiedKernel shared library
│   ├── common/             Shared utilities
│   └── server/             Auth, BaseApp, CellApp, NavBuilder
├── python/                 Entity scripts and game logic (164 files)
├── entities/               XML entity definitions and type registry
├── config/                 XML service configuration
├── data/                   Cooked game data (.pak) and scripts
├── db/                     PostgreSQL schemas
│   ├── database.sql        Database and role setup
│   ├── sgw/                Game schema (accounts, characters, items)
│   └── resources/          Resource data (abilities, effects, archetypes)
├── docs/                   123 documents
├── tools/ServerEd/         Qt editor source
├── bootstrap/              C++ dependency automation
└── W-NG.sln                Visual Studio solution (C++ legacy build)
```

## Tech Stack

### Rust Server (active)

| Crate | Purpose |
|---|---|
| `cimmeria-mercury` | Mercury reliable UDP, AES-256-CBC + HMAC-MD5 |
| `cimmeria-services` | Auth, Base, Cell service orchestration |
| `cimmeria-defs` | Entity definition parsing from XML |
| `cimmeria-content-engine` | Data-driven mission/effect/dialog runtime |
| `tokio` | Async runtime and networking |
| `axum` | HTTP/REST for auth and admin API |
| `sqlx` | PostgreSQL async driver |
| `quick-xml` | SOAP/XML parsing |

### C++ Server (legacy reference)

| Component | Version | Notes |
|---|---|---|
| MSVC Toolset | v145 (VS2026) | C++11 codebase |
| Boost | 1.55.0 | Asio, Python, Thread, DateTime |
| Python | 3.4.1 | Embedded via Boost.Python |
| PostgreSQL | 9.2.x | Via SOCI 3.2.1 |
| OpenSSL | 1.0.1e | Known CVEs — do not expose to internet |

## Configuration

Config files live in `config/` (XML format). Defaults work for local development.

- `AuthenticationService.config` — Auth server ports, encryption
- `BaseService.config` — Database connection, shard settings
- `CellAppService.config` — World cell parameters, AoI distances

The Rust server reads these same configs. Environment variables override XML values.

## Database

PostgreSQL schemas are in `db/`:
- `db/database.sql` — Database and role setup (port 5433, role `w-testing`)
- `db/sgw/` — Game schema (accounts, characters, items, missions)
- `db/resources/` — Resource data (abilities, effects, loot, archetypes)

Test account: **test** / **test** (SHA1 hashed).

## Documentation

[docs/](docs/readme.md) contains **123 documents** covering protocol, gameplay, engine internals, architecture, and reverse engineering.

**Start here:**
- [How SGW Works](docs/how-sgw-works.md) — BigWorld + UE3 hybrid architecture
- [Game Systems](docs/game-systems.md) — Combat, abilities, stargates, missions, crafting
- [Connection Flow](docs/connection-flow.md) — End-to-end login and world entry
- [Project Status](docs/project-status.md) — What works and what's left
- [Gap Analysis](docs/gap-analysis.md) — Per-feature completion tracking

For reverse engineering: [docs/reverse-engineering/](docs/reverse-engineering/PLAN.md)

## License

This project is a server emulator for research and preservation purposes.
