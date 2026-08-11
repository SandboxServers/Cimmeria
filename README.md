# Cimmeria — Stargate Worlds Server Emulator

[![ci](https://github.com/SandboxServers/Cimmeria/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/SandboxServers/Cimmeria/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/SandboxServers/Cimmeria/branch/main/graph/badge.svg)](https://codecov.io/gh/SandboxServers/Cimmeria)

A server emulator for [Stargate Worlds](https://en.wikipedia.org/wiki/Stargate_Worlds), the cancelled Stargate MMO developed by Cheyenne Mountain Entertainment. The game was built on [BigWorld Technology](https://en.wikipedia.org/wiki/BigWorld) (networking/server) and Unreal Engine 3 (rendering/client), and reached a playable beta before the studio shut down in 2010.

Cimmeria reimplements the server infrastructure — authentication, world simulation, entity management, and game logic — allowing the original game client to connect and play.

## Status

The project tracks **437 features** across 44 systems against the Rust codebase. **57% have code** (248 of 437); **32% are confirmed working** end-to-end with the live client (139 of 437). See the [Gap Analysis](docs/gap-analysis.md) for the full per-system breakdown.

**Tested end-to-end with the game client:**
- Login and authentication (HTTP SOAP → shard select → Mercury UDP)
- Mercury reliable UDP transport with AES-256 encryption, per-channel fragment reassembly
- Game data pipeline (22 resource categories, 112,626 DB rows)
- World entry, entity spawning, grid-based Area of Interest
- Durable Base→Cell content event delivery via persistent outbox
- One-command build and setup

**Code exists, needs verification:**
Character creation (8 archetypes, 23 defs) | Inventory | Vendors | Chat | Crafting | Trading

**Implemented with known gaps:**
Combat & abilities | Effects | Missions | NPC AI | Stats & leveling | Stargate travel

See [docs/project-status.md](docs/project-status.md) for the detailed breakdown.

## Tests & CI

The Rust workspace currently carries **2,936 `#[test]` / `#[tokio::test]` cases** across **461 files**, of which **2,691 are gated on every PR** (CI excludes the two Tauri editors, the egui launcher, the Tauri app, and the Windows-only client-telemetry cdylib). **224 are live-DB regression guards** (gated by `require_db_or_skip!`, all in `cimmeria-services`) and **3 are end-to-end PL/pgSQL smoke scripts** (vendor stack, inventory move, progression). GitHub Actions runs five gating jobs on every PR — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, `cargo nextest run` (workspace, no DB), and `cargo nextest run -p cimmeria-services --lib` against a `postgres:17.9` service container loaded from `db/database.sql`. nextest's JUnit output is uploaded to Codecov Test Analytics for per-test history and flake detection.

For the test-type taxonomy (unit / wire-format / live-DB / smoke / concurrency / chain-replay), when each is appropriate, common gotchas, and the patterns reviewers expect to see, read **[TESTING.md](TESTING.md)**.

## Quick Start

```bash
cargo run -p cimmeria-server
```

Handles login, Mercury protocol, character select, and world entry. Connect the game client to `localhost`.

**Test account:** `test` / `test`

### Run from a pre-built container (no Rust toolchain required)

A self-contained pre-release image with the server, cooked game data, and a pre-loaded Postgres is published to GHCR on demand — comment `/release` on a merged PR (or run the workflow manually) and the build at `main` HEAD ships. Versioned `YYYY-MM-DD.N` (UTC). See [docs/operations/container.md](docs/operations/container.md) for the release model.

```bash
docker run -d --name cimmeria \
  -p 13001:13001 -p 32832:32832/udp -p 50000:50000/udp \
  -p 8081:8081 -p 8443:8443 \
  -e BASE_EXTERNAL=<your-LAN-or-WAN-ip> \
  -v cimmeria-data:/var/lib/postgresql/data \
  ghcr.io/sandboxservers/cimmeria-server:latest-prerelease
```

`BASE_EXTERNAL` defaults to `127.0.0.1` and must be overridden for any client not on the host. See [docs/operations/container.md](docs/operations/container.md) for the full env reference, volume layout, and reset workflow.

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

## Crate Dependency Graph

The 23 workspace crates and their **actual** inter-crate dependencies (an arrow
**A → B** means *crate A depends on crate B*). Generated from each crate's
`Cargo.toml`; GitHub renders the Mermaid below.

```mermaid
%%{init: {"flowchart": {"htmlLabels": false}, "theme": "neutral"}}%%
flowchart TD
    %% entry points / binaries
    server --> adminApi["admin-api"]
    server --> services
    server --> discord
    server --> observability
    server --> common
    app["app (src-tauri, cimmeria-app)"] --> adminApi
    app --> services
    app --> common
    wireclient --> services
    wireclient --> mercury
    wireclient --> common

    %% service + domain layer
    adminApi --> services
    adminApi --> contentEngine["content-engine"]
    adminApi --> entity
    adminApi --> commands
    adminApi --> common
    services --> game
    services --> contentEngine
    services --> entity
    services --> mercury
    services --> discord
    services --> observability
    services --> commands
    services --> common
    game --> entity
    game --> commands
    game --> common
    contentEngine --> entity
    contentEngine --> common
    entity --> defs
    entity --> mercury
    entity --> commands
    entity --> common

    %% foundation
    mercury --> common
    defs --> common
    commands --> common

    %% UPK / navmesh toolchain (independent of the server spine)
    navmeshExtractor["navmesh-extractor"] --> upkObjects["upk-objects"]
    navmeshExtractor --> upk
    sceneEditor["scene-editor (tool)"] --> upkObjects
    sceneEditor --> upk
    upkObjects --> upk

    %% standalone crates with no intra-workspace dependencies
    subgraph standalone["Standalone (no intra-workspace deps)"]
        supervisor
        serverHarness["server-harness"]
        clientTelemetry["client-telemetry"]
        launcher["launcher (sgw-launcher)"]
        contentEditor["content-editor (tool)"]
        specLint["spec-lint (tool)"]
    end
```

Every node is a workspace crate; the graph is a DAG rooted at **common** (the
shared types / config / error layer everything builds on). **mercury** (reliable
UDP + AES-256), **defs** (entity-definition XML parser) and **commands**
(command + permission model) sit on `common`; **entity** composes them into live
game objects; **game** and **content-engine** add gameplay rules and the
data-driven content pipeline; **services** ties Auth / Base / Cell together and is
what the **server** binary, the **admin-api** REST layer, the **app** desktop GUI
(repo-root `src-tauri/`, package `cimmeria-app`) and the headless **wireclient**
test client all build on. **discord** (notifications) and **observability** (OTLP
metrics) are cross-cutting libraries pulled in by `services` + `server`. The
**upk** / **upk-objects** / **navmesh-extractor** crates plus the `scene-editor`
tool form an independent Unreal-package / navmesh toolchain. **supervisor**,
**client-telemetry**, **launcher** (`sgw-launcher`), and the `content-editor` /
`spec-lint` tools carry no intra-workspace dependencies.

## Project Structure

```
Cimmeria/
├── crates/                 Rust server (active development — 19 crates)
│   ├── common/             Shared types, config, error handling
│   ├── mercury/            Mercury reliable UDP + AES-256 encryption
│   ├── defs/               Entity definition parser (XML → Rust types)
│   ├── entity/             Entity system (lifecycle, properties)
│   ├── commands/           Server command framework
│   ├── game/               Game mechanics and rules
│   ├── content-engine/     Data-driven content pipeline
│   ├── services/           Auth, Base, Cell service implementations
│   ├── admin-api/          REST administration API
│   ├── supervisor/         Process supervision and service lifecycle
│   ├── server/             Binary entry point (cargo run -p cimmeria-server)
│   ├── server-harness/     Spawn/readiness/reap harness for integration tests
│   ├── discord/            Discord notification dispatch
│   ├── observability/      Metrics facade over the OpenTelemetry SDK
│   ├── wireclient/         Headless test client (Tier 3)
│   ├── upk/                UPK (Unreal Package) file parser
│   ├── upk-objects/        UPK object type definitions
│   ├── navmesh-extractor/  UE3 .umap geometry → .obj for NavBuilder
│   ├── launcher/           egui game launcher + DLL injection (sgw-launcher)
│   └── client-telemetry/   Windows-only cdylib injected into SGW.exe
├── src-tauri/              Tauri desktop GUI wrapping the server (cimmeria-app)
├── entities/               XML entity definitions and type registry
├── data/                   Cooked game data (.pak) and navmeshes
├── db/                     PostgreSQL schemas
│   ├── database.sql        Database and role setup
│   ├── sgw/                Game schema (accounts, characters, items)
│   └── resources/          Resource data (abilities, effects, archetypes — 18 game systems)
├── docs/                   ~280 documents
├── tools/                  Editor tools, RE utilities, and live-DB smoke SQL scripts (vendor_store_smoke.sql, inventory_move_smoke.sql, progression_smoke.sql)
└── deprecated/             Retired C++/Python/MSVC sources kept for reference
```

## Tech Stack

| Crate | Purpose |
|---|---|
| `cimmeria-mercury` | Mercury reliable UDP, AES-256-CBC + HMAC-MD5 |
| `cimmeria-services` | Auth, Base, Cell service orchestration |
| `cimmeria-defs` | Entity definition parsing from XML |
| `cimmeria-content-engine` | Data-driven mission/effect/dialog runtime |
| `cimmeria-discord` | Discord notification dispatch (server + colo events) |
| `cimmeria-observability` | Metrics facade over the OpenTelemetry SDK (OTLP) |
| `cimmeria-wireclient` | Headless Tier 3 test client (SOAP + Mercury + pcap replay) |
| `cimmeria-server-harness` | Spawn/readiness/reap harness for the server process in integration tests |
| `tokio` | Async runtime and networking |
| `axum` | HTTP/REST for auth and admin API |
| `sqlx` | PostgreSQL async driver |
| `quick-xml` | SOAP/XML parsing |

## Database

PostgreSQL 17.9 schemas in `db/`:
- `db/database.sql` — Database and role setup (port 5433, role `w-testing`)
- `db/sgw/` — Game schema (accounts, characters, items, missions)
- `db/resources/` — Resource data (abilities, effects, loot, archetypes)

Test account: **test** / **test** (SHA1 hashed).

## Documentation

[docs/](docs/readme.md) contains **~270 documents** covering protocol, gameplay, engine internals, architecture, and reverse engineering.

**New here? Start with:**

- [Getting Started](docs/guides/getting-started.md) — first-time setup tutorial (prerequisites → `setup.ps1` → connecting the client → running tests)
- [Building the Server](docs/building.md) — how-to for cargo builds, the test suite, and CI checks
- [Troubleshooting](docs/troubleshooting.md) — common first-day problems and fixes

**Understand the codebase:**

- [How SGW Works](docs/how-sgw-works.md) — BigWorld + UE3 hybrid architecture
- [Connection Flow](docs/connection-flow.md) — End-to-end login and world entry
- [Game Systems](docs/game-systems.md) — Combat, abilities, stargates, missions, crafting
- [Service Architecture](docs/architecture/service-architecture.md) — Auth / Base / Cell topology

**Plan and contribute:**

- [Project Status](docs/project-status.md) — What works and what's left
- [Gap Analysis](docs/gap-analysis.md) — Per-feature completion tracking
- [Contributing](CONTRIBUTING.md) — Contribution scope, code style, PR conventions
- [Testing Guide](TESTING.md) — Test types, when to use which, common gotchas

**Operate and deploy:**

- [Container Distribution](docs/operations/container.md) — `docker run` the published GHCR image, env reference, release model
- [Integration Test Infra](docs/architecture/integration-test-infra.md) — Live-DB test setup and rationale

For reverse engineering: [docs/reverse-engineering/](docs/reverse-engineering/PLAN.md)

## Contributing

Contributions welcome. See **[CONTRIBUTING.md](CONTRIBUTING.md)** for scope, code style, PR conventions, and where to find a first issue. The pre-PR checklist lives in [CLAUDE.md](CLAUDE.md); test conventions in [TESTING.md](TESTING.md). Project conduct expectations are in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Security

If you find a security issue, **please do not open a public issue**. See [SECURITY.md](SECURITY.md) for the private reporting path.

## License

This project is a server emulator for research and preservation purposes. A formal license file is pending — until it lands, treat the source as available for reading, building, and contributing back, but ask before redistributing.
