# Operator Guide

Everything you need to go from a fresh git clone to a running server with a connected game client.


## Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| Windows | 10+ (64-bit) | WSL2 not supported for the server itself |
| Visual Studio | 2022+ Community | C++ Desktop workload required; `setup.ps1 -InstallVS` can install it |
| PowerShell | 7.0+ | `winget install Microsoft.PowerShell` if not present |
| Perl | Any recent | Required for building OpenSSL; Strawberry Perl works |
| Disk space | ~2 GB | For `external/` dependencies + build outputs |
| Stargate Worlds client | QA build | For connecting to the server (optional for server-only work) |


## Quick Start

```powershell
git clone <repo-url> Cimmeria
cd Cimmeria
pwsh setup.ps1
```

This runs the full 7-step bootstrap pipeline:

1. **Download dependencies** — Boost, OpenSSL, PostgreSQL, Python, SOCI, SDL, Recast, Qt (~210 MB)
2. **Build libraries** — Compile Boost, OpenSSL, SOCI from source
3. **Build solution** — Compile all 6 projects via MSBuild
4. **Build ServerEd** — Compile the Qt editor tool (optional, non-fatal if it fails)
5. **Initialize database** — Set up PostgreSQL on port 5433, load game schema
6. **Stage runtime DLLs** — Copy shared libraries to `bin64/`
7. **Start servers** — Launch Auth, Base, Cell in order

When it finishes you will see:

```
Auth server:   http://localhost:8081 (client login)
Shard server:  localhost:32832 (game client)
PostgreSQL:    localhost:5433
Test account:  test / test
```


## Bootstrap Deep-Dive

Each step can be run independently by importing the module and calling the function directly:

```powershell
Import-Module ./bootstrap/CimmeriaBootstrap
```

### Step 1: Install-CimmeriaDependencies

Downloads, extracts, and patches all external dependencies into `external/`.

**Phases:**
- **Phase 0** — Detect Visual Studio and Perl installations
- **Phase 1** — Download archives to `external/_downloads/` (skipped if cached)
- **Phase 2** — Extract and arrange into `external/` subdirectories
- **Phase 3** — Apply VS2015+ compatibility patches to source files

**Downloads:**

| Library | Version | Size | Output Directory |
|---------|---------|------|------------------|
| Boost | 1.55.0 | ~55 MB | `external/boost/` |
| OpenSSL | 1.0.1e | ~5 MB | `external/openssl/` |
| PostgreSQL | 9.2.3 | ~50 MB | `external/postgresql/` (headers/libs), `external/postgresql_server/` (full binary) |
| Python | 3.4.1 | ~25 MB | `external/python/` |
| SOCI | 3.2.1 | ~2 MB | `external/soci/` |
| SDL | 1.2.15 | ~3 MB | `external/sdl/` |
| Recast/Detour | main | ~5 MB | `external/recast/` |
| Qt | 5.15 | ~60 MB | `external/qt/` |

**Patches applied in Phase 3:**
- `boost_auto_link.hpp` — Add vc140/vc145 toolset entries
- `soci_platform.h` — `snprintf`/`strtoll` macros for VS2013+ CRT
- `soci_backends_config.h` — CMake config header generation
- `soci_postgresql_statement.cpp` — Custom enum type handling
- `openssl_e_os.h` — `stdin`/`stdout`/`stderr` redefinition fix
- `openssl_e_padlock.c` — x86 inline assembly guards

Also generates `external/boost/user-config.jam` with detected MSVC toolset and Python paths.

**Parameters:**

| Flag | Description |
|------|-------------|
| `-SkipDownload` | Use cached archives in `external/_downloads/`; re-apply patches only |
| `-InstallVS` | Auto-install Visual Studio Community with C++ workload |
| `-BuildArch` | Architecture: `x64` (default), `x86`, or `both` |
| `-IncludeBigWorld` | Also download BigWorld Engine 1.9.1 and 2.0.1 reference sources (~300 MB extra) |


### Step 2: Build-CimmeriaLibraries

Compiles Boost, OpenSSL, and SOCI from source. Each is skipped if its output library already exists.

| Library | Build Script | Output |
|---------|-------------|--------|
| Boost 1.55.0 | `bootstrap/build-boost.bat` | `external/boost/lib64/lib/libboost_*-vc145-mt-*.lib` |
| OpenSSL 1.0.1e | `bootstrap/build-openssl.bat` | `external/openssl/lib64/libeay32MT.lib` (release), `libeay32MTd.lib` (debug) |
| SOCI 3.2.1 | `bootstrap/build-soci.bat` | `lib64/debug/libsoci_core_3_2.lib`, `lib64/release/libsoci_core_3_2.lib` (+ postgresql variants) |

**Boost libraries built:** date_time, math, python, python3, system, thread, chrono

OpenSSL requires Perl on PATH and takes 5–15 minutes (two full `nmake` passes for debug + release).


### Step 3: Build-CimmeriaSolution

Compiles the `W-NG.sln` solution via MSBuild.

```
msbuild W-NG.sln /p:Configuration=Debug /p:Platform=x64 /m /nologo /v:minimal
```

**6 projects built (in dependency order):**
1. Recast — Navigation mesh library
2. UnifiedKernel — Shared static library (networking, protocol, entity system)
3. NavBuilder — Offline navigation mesh generator
4. AuthenticationServer — Login and shard management
5. CellApp — Spatial entity simulation
6. BaseApp — Persistent entity state and client routing

**Output verification:** Checks that `bin64/AuthenticationServer_d.exe`, `bin64/BaseApp_d.exe`, and `bin64/CellApp_d.exe` exist (debug builds have `_d` suffix; release builds do not).

| Parameter | Default | Description |
|-----------|---------|-------------|
| `-Configuration` | `Debug` | Build configuration: `Debug` or `Release` |
| `-Platform` | `x64` | Build platform |


### Step 4: Build-ServerEd

Builds the Qt-based admin tool via `qmake` + `nmake`. This step is optional — if it fails, the pipeline continues with a warning.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `-Configuration` | `Release` | `Debug` or `Release` |

Skipped if `bin64/ServerEd.exe` exists and is newer than all source files.


### Step 5: Initialize-CimmeriaDatabase

Sets up a PostgreSQL instance and loads the game schema.

**Phases:**

1. **Init data directory** — `initdb -D server/pgdata -U postgres -A trust -E UTF8`
2. **Start PostgreSQL** — `pg_ctl start` on port 5433 (15-second timeout)
3. **Create role + database** — `CREATE ROLE "w-testing"`, `CREATE DATABASE sgw`
4. **Load schema** — `psql -f db/database.sql` (~126,000 SQL statements)
5. **Verify** — Checks `account` and `sgw_player` tables exist

See [Database Setup](#database-setup) below for schema details.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `-Port` | `5433` | PostgreSQL port (avoids conflicts with system PG) |
| `-Force` | `false` | Drop and recreate the `sgw` database before loading |


### Step 6: Initialize-CimmeriaRuntime

Stages runtime DLLs into `bin64/` alongside server executables.

**DLLs staged:**
- `python34.dll` — Python 3.4 embedded runtime
- `libpq.dll` + dependencies (`libintl-8.dll`, `iconv.dll`, `ssleay32.dll`, `libeay32.dll`) — PostgreSQL client
- Qt DLLs (if Qt present): `Qt5Core.dll`, `Qt5Gui.dll`, `Qt5Widgets.dll`, `Qt5Xml.dll`, `Qt5Network.dll`, `Qt5Sql.dll` + platform/SQL driver plugins
- `Lib/` and `DLLs/` — Python standard library and compiled extensions

Uses SHA256 hash comparison to skip unchanged files.


### Step 7: Start-CimmeriaServer

Launches the server stack in order. See [Running the Server](#running-the-server) for details.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `-Configuration` | `Debug` | Which executables to run (`Debug` = `*_d.exe`) |
| `-Port` | `5433` | PostgreSQL port |


## Bootstrap Options

Full flag reference for `setup.ps1`:

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `-SkipDownload` | Switch | `false` | Skip dependency downloads; use cached archives |
| `-SkipBuild` | Switch | `false` | Skip building libraries and solution (Steps 2–4) |
| `-NoLaunch` | Switch | `false` | Stop after setup; do not launch servers |
| `-Configuration` | String | `Debug` | Build configuration: `Debug` or `Release` |
| `-ForceDatabase` | Switch | `false` | Drop and recreate the `sgw` database |
| `-InstallVS` | Switch | `false` | Auto-install Visual Studio Community if not found |
| `-IncludeBigWorld` | Switch | `false` | Download BigWorld Engine reference sources (~300 MB extra) |

**Common recipes:**

```powershell
# Full fresh bootstrap
pwsh setup.ps1

# Rebuild everything, no download
pwsh setup.ps1 -SkipDownload

# Database only (wipe and reload)
pwsh setup.ps1 -SkipDownload -SkipBuild -ForceDatabase -NoLaunch

# Release build
pwsh setup.ps1 -Configuration Release
```


## Database Setup

### Schema Architecture

The database uses a single PostgreSQL database (`sgw`) with two logical schemas:

| Schema | Purpose | Location |
|--------|---------|----------|
| `resources` | Game content — abilities, items, missions, effects, NPCs, worlds, dialogs | `db/resources/` |
| `sgw` (public) | Player/runtime state — accounts, characters, inventory, mail, missions | `db/sgw/` |

### Load Order

The main entry point is `db/database.sql`, which includes files via `\ir` (relative include) in this order:

1. **Schema creation** — `db/resources/_schema.sql`
2. **Type enums** — `db/resources/*/Types/*.sql` (all enum types)
3. **Table definitions** — `db/resources/*/Tables/*.sql`
4. **Sequences** — `db/resources/*/Sequences/*.sql`
5. **Seed data** — `db/resources/*/Seed/*.sql`
6. **Primary keys** — `db/resources/_primary_keys.sql`
7. **Foreign keys** — `db/resources/_foreign_keys.sql`
8. **Indexes** — `db/resources/_indexes.sql`
9. **Functions** — `db/resources/_functions.sql`
10. **Triggers** — `db/resources/_triggers.sql`
11. **Sequence ownership** — `db/resources/_sequence_ownership.sql`
12. **SGW schema** — `db/sgw/_schema.sql`, then types, tables, sequences, seed, keys, indexes

### Content Domains

**Resources (game content):**

| Domain | Content |
|--------|---------|
| AI | NPC AI state/behavior enums |
| Abilities | 1,887 abilities, skill sets, trainer lists |
| Archetypes | 8 archetypes, disciplines, character creation choices |
| Combat | Damage types, stats, threat mechanics |
| Dialogs | NPC dialog trees, screens, buttons, speakers |
| Effects | 3,217 effects (buffs, debuffs, DoTs, HoTs) |
| Entities | Entity templates, blueprints, monikers, resource types |
| Events | Kismet/Matinee sequences, paths, point sets |
| Gameplay | Game mechanic enums (timers, events, cooldowns) |
| Items | 6,060 items, containers, vendors, crafting |
| Loot | Loot tables, drop rates |
| Missions | 1,041 missions, steps, objectives, tasks, rewards |
| Social | Guilds, mail, auction house, factions |
| Texts | In-game text strings (localization) |
| Visuals | Character models, body components, meshes |
| Worlds | 24 zones, spawn points, stargates, interactions |

**SGW (player state):**

| Domain | Content |
|--------|---------|
| Accounts | Player accounts and authentication |
| Inventory | Item storage, equipment |
| Mail | In-game mail |
| Missions | Per-character mission progress |
| Players | Character data, progression |
| Shards | Shard registry |

### Test Account

The seed data includes a test account:

| Field | Value |
|-------|-------|
| Username | `test` |
| Password | `test` |
| Access Level | Admin |

### Reloading the Database

```powershell
Import-Module ./bootstrap/CimmeriaBootstrap
Initialize-CimmeriaDatabase -Force
```


## Configuration Reference

All config files are in `config/`. Each file is XML with a root `<Config>` element.

For real deployments, create `*.local` overrides (e.g., `BaseService.config.local`) to avoid committing credentials. Local overrides are gitignored.

### AuthenticationService.config

| Parameter | Default | Description |
|-----------|---------|-------------|
| `log_level` | `DEBUG1` | Logging verbosity |
| `base_service_port` | `13001` | Internal port for BaseApp connection |
| `logon_service_port` | `8081` | Client-facing login port (HTTP/SOAP) |
| `db_connection_string` | `host=localhost port=5433 user=w-testing password=w-testing dbname=sgw` | PostgreSQL connection |
| `protocol_digest` | `58AFA196AD3AC4F65CADD99BFF23B799` | Protocol version signature (must match client) |

### BaseService.config

**Shard Identity:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `shard_name` | `Shard` | Display name shown to clients |
| `shard_id` | `1` | Unique shard identifier |
| `shard_port` | `32832` | Port BaseApp binds to |
| `shard_address` | `0.0.0.0` | Bind interface (all interfaces) |
| `shard_external_address` | `127.0.0.1` | External IP clients connect to |
| `shard_auth_key` | *(empty)* | Shared key for AuthServer registration |
| `service_port` | `13002` | Inter-service port (CellApp connects here) |

**Authentication:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `auth_server_address` | `127.0.0.1` | AuthenticationServer internal address |
| `auth_server_port` | `13001` | AuthenticationServer internal port |

**Minigame Server:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `minigame_external_address` | `127.0.0.1` | External address clients see for minigames |
| `minigame_external_port` | `30000` | External minigame port |
| `minigame_port` | `30000` | Internal minigame server port |

**Client Session:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `client_inactivity_timeout` | `300000` | Disconnect inactive clients after 300 seconds |

**Network:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `unreliable_tick_sync` | `false` | Send tick sync on unreliable channel |
| `unreliable_movement_update` | `false` | Send movement updates on unreliable channel |
| `nub_tickrate` | `25` | Client channel flush interval (ms) |
| `nub_send_buffer` | `512` | Send buffer size (KB) |
| `nub_receive_buffer` | `512` | Receive buffer size (KB) |

**Area of Interest (AoI):**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `grid_chunk_size` | `50` | World grid chunk width/height (game units) |
| `grid_hysteresis` | `1` | Hysteresis range in chunks (prevents AoI flapping) |
| `grid_vision_distance` | `3` | Vision distance in chunks |
| `cache_on_client` | `true` | Hide entities instead of deleting when leaving AoI |

**Tick Rate:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `tick_rate` | `100` | Server tick duration in milliseconds (= 10 Hz) |
| `tick_notification_skip` | `1` | Ticks skipped after last notification |

**Database & Console:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `db_connection_string` | `host=localhost port=5433 user=w-testing password=w-testing dbname=sgw` | PostgreSQL connection |
| `console_port` | `8989` | Python console port |
| `py_console_password` | *(empty)* | Python console password; console only starts if set |
| `developer_mode` | `false` | Relaxed auth, multiple logins per account, elevated logging |

### CellService.config

| Parameter | Default | Description |
|-----------|---------|-------------|
| `cell_id` | `1` | Unique CellApp instance identifier |
| `baseapp_address` | `127.0.0.1` | BaseApp internal address |
| `baseapp_port` | `13002` | BaseApp service port |
| `tick_notification_skip` | `0` | Ticks skipped after last notification |
| `idle_update_ticks` | `10` | Ticks between idle movement updates (`-1` disables) |
| `cache_entity_messages` | `true` | Cache static NPC messages (appearance) on BaseApp |
| `cache_player_messages` | `false` | Cache static player messages on BaseApp |
| `db_connection_string` | `host=localhost port=5433 user=w-testing password=w-testing dbname=sgw` | PostgreSQL connection |
| `console_port` | `8990` | Python console port |
| `py_console_password` | *(empty)* | Python console password |
| `developer_mode` | `false` | Relaxed auth and increased logging |


## Running the Server

### Startup Order

Services must start in this order — each depends on the previous:

1. **PostgreSQL** — Database must be running first
2. **AuthenticationServer** — Binds ports 8081 (client login) and 13001 (inter-service)
3. **BaseApp** — Connects to Auth on port 13001, binds port 32832 (client shard) and 13002 (inter-service)
4. **CellApp** — Connects to BaseApp on port 13002

### Port Summary

| Service | Port | Protocol | Direction | Purpose |
|---------|------|----------|-----------|---------|
| PostgreSQL | 5433 | TCP | Services → DB | Database (localhost only) |
| AuthenticationServer | 8081 | TCP/HTTP | Client → Server | Login endpoint (SOAP) |
| AuthenticationServer | 13001 | TCP | BaseApp → Auth | Inter-service (shard registration) |
| BaseApp | 32832 | UDP | Client → Server | Game shard (Mercury reliable UDP) |
| BaseApp | 13002 | TCP | CellApp → Base | Inter-service (entity sync) |
| BaseApp | 8989 | TCP | Console → Base | Python console (if password set) |
| CellApp | 8990 | TCP | Console → Cell | Python console (if password set) |
| Minigame Server | 30000 | TCP | Client → Server | Minigame server |

### Manual Launch

If you prefer to start services manually instead of using `Start-CimmeriaServer`:

```powershell
# 1. Start PostgreSQL
& external/postgresql_server/bin/pg_ctl start -D server/pgdata -l server/logs/postgresql.log -o "-p 5433"

# 2. Start Auth (from bin64/)
cd bin64
./AuthenticationServer_d.exe

# 3. Start Base (new terminal, from bin64/)
./BaseApp_d.exe

# 4. Start Cell (new terminal, from bin64/)
./CellApp_d.exe
```

Each executable looks for its config file in `../config/` relative to the working directory.

### Verifying Startup

Watch each server's console output for startup confirmation:

- **AuthenticationServer:** `Listening on port 8081` and `Listening on port 13001`
- **BaseApp:** `Connected to auth server` and `Listening on port 32832`
- **CellApp:** `Connected to baseapp`


## Connecting a Game Client

### Client Requirements

You need a copy of the Stargate Worlds QA client. The game was never publicly released, but QA builds exist in the preservation community.

### Patching the Client

The client must be patched to point at your server instead of the defunct FireSky login servers:

```powershell
Import-Module ./bootstrap/CimmeriaBootstrap
Update-CimmeriaClient
```

Or with explicit paths:

```powershell
Update-CimmeriaClient -ClientPath "D:\Games\Stargate Worlds-QA" -ServerUrl "http://localhost:8081"
```

This patches `Working/SGWGame/Content/UI/Startup/Login/Login.lua` to replace the auth URL with your server's address. A backup is saved as `Login.lua.bak`.

**Auto-detected client paths:**
- `${ProgramFiles(x86)}\FireSky\Stargate Worlds-QA`
- `${ProgramFiles}\FireSky\Stargate Worlds-QA`
- `${ProgramFiles(x86)}\Cheyenne Mountain Entertainment\Stargate Worlds`
- `${ProgramFiles}\Cheyenne Mountain Entertainment\Stargate Worlds`

### Logging In

1. Launch the game client
2. At the login screen, enter:
   - **Username:** `test`
   - **Password:** `test`
3. Select the shard (named "Shard" by default)
4. Create a character or load an existing one


## Monitoring and Debugging

### Log Levels

Set `log_level` in config files. Available levels (most to least verbose):

| Level | Description |
|-------|-------------|
| `TRACE` | Everything, including per-packet network traces |
| `DEBUG2` | Detailed debug information |
| `DEBUG1` | Standard debug (default) |
| `INFO` | Informational messages |
| `WARNING` | Potential problems |
| `ERROR` | Errors that don't crash the server |
| `CRITICAL` | Fatal errors |

### Developer Mode

Set `developer_mode` to `true` in BaseService.config and CellService.config to enable:

- Multiple simultaneous logins on the same account
- Maximum access level for all accounts
- Per-player logging
- Automatic TRACE-level logging

### Python Console

The Python console provides runtime access to game state and GM commands. To enable it:

1. Set `py_console_password` in the config file (BaseService.config for port 8989, CellService.config for port 8990)
2. Restart the service
3. Connect via telnet: `telnet localhost 8989`
4. Enter the password when prompted

See [python-console.md](architecture/python-console.md) for full console documentation.

### Common Startup Failures

| Symptom | Cause | Fix |
|---------|-------|-----|
| Auth fails to start | Port 8081 already in use | Kill other process on 8081 or change `logon_service_port` |
| BaseApp can't connect to auth | Auth not running or wrong port | Start Auth first; check `auth_server_port` matches |
| CellApp can't connect to base | BaseApp not running or wrong port | Start BaseApp first; check `baseapp_port` matches |
| Database connection refused | PostgreSQL not running | `pg_ctl start -D server/pgdata -o "-p 5433"` |
| `psql` fails loading schema | Schema partially loaded | Use `-ForceDatabase` to wipe and reload |
| LNK4099 warnings during build | Missing PDB files for OpenSSL | Cosmetic; safe to ignore |
| Boost build fails | Python 3.13 on PATH conflicts with Boost.Build | Temporarily remove Python 3.13 from PATH during build |


## Stopping and Restarting

### Graceful Shutdown

```powershell
Import-Module ./bootstrap/CimmeriaBootstrap
Stop-CimmeriaServer
```

This stops services in reverse order: CellApp → BaseApp → AuthenticationServer → PostgreSQL.

### Restart

```powershell
Stop-CimmeriaServer
Start-CimmeriaServer
```

### Database Persistence

Player state (accounts, characters, inventory, missions) is persisted to PostgreSQL. Stopping and restarting the server preserves all player data. Game content data (abilities, items, missions) is loaded from seed SQL and does not change at runtime.


## Directory Layout

```
Cimmeria/
├─ setup.ps1                    # Full bootstrap entry point
├─ W-NG.sln                     # Visual Studio solution (6 projects)
├─ CLAUDE.md                    # Project instructions and agent roster
│
├─ bootstrap/                   # Bootstrap module and build scripts
│  ├─ CimmeriaBootstrap/        #   PowerShell module (10 exported functions)
│  ├─ build-boost.bat           #   Boost build script
│  ├─ build-openssl.bat         #   OpenSSL build script
│  ├─ build-soci.bat            #   SOCI build script
│  └─ init-boost.bat            #   Boost bootstrap script
│
├─ config/                      # Service configuration files
│  ├─ AuthenticationService.config
│  ├─ BaseService.config
│  └─ CellService.config
│
├─ src/                         # C++ source code
│  ├─ authentication/           #   AuthenticationServer
│  ├─ baseapp/                  #   BaseApp service
│  ├─ cellapp/                  #   CellApp service
│  ├─ common/                   #   Shared code (Service base, database, vectors)
│  ├─ entity/                   #   Entity system (types, mailboxes, world grid)
│  ├─ log/                      #   Logging system
│  ├─ mercury/                  #   Networking layer (Nub, Channel, Packet, Bundle)
│  ├─ server/                   #   Server-specific utilities
│  └─ util/                     #   Singletons, crash dumps
│
├─ projects/                    # Visual Studio .vcxproj files
│  ├─ AuthenticationServer/
│  ├─ BaseApp/
│  ├─ CellApp/
│  ├─ NavBuilder/
│  ├─ Recast/
│  └─ UnifiedKernel/
│
├─ python/                      # Python game scripts (164 files)
│  ├─ Atrea/                    #   Framework/engine bindings
│  ├─ base/                     #   BaseApp entity scripts
│  ├─ cell/                     #   CellApp entity scripts (primary game logic)
│  └─ common/                   #   Shared Python code and definitions
│
├─ entities/                    # XML entity definitions
│  ├─ entities.xml              #   Entity type registry
│  ├─ cell_spaces.xml           #   Zone/cell partitioning
│  ├─ custom_alias.xml          #   Custom type aliases
│  └─ defs/                     #   Per-entity .def files and interfaces/
│
├─ db/                          # PostgreSQL schema
│  ├─ database.sql              #   Main entry point (includes everything)
│  ├─ resources/                #   Game content schema (17 domains)
│  └─ sgw/                      #   Player/runtime state schema (6 domains)
│
├─ data/                        # Game data
│  ├─ cache/                    #   Cooked data (.pak files, ~16 MB)
│  ├─ scripts/                  #   Effect/mission script XMLs
│  └─ spaces/                   #   Space/navmesh data
│
├─ tools/ServerEd/              # Qt editor tool source
├─ docs/                        # Documentation (114 files)
│
├─ external/                    # Vendored dependencies (~724 MB, not in git)
├─ bin64/                       # Build output executables (not in git)
├─ lib64/                       # Library output (not in git)
└─ server/                      # Runtime data (pgdata, logs — not in git)
```
