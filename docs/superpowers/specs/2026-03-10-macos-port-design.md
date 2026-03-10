# Cross-Platform Port: CMake, Shell Scripts, and Docker

**Date:** 2026-03-10
**Status:** Approved
**Scope:** Replace Windows-only PowerShell bootstrap with a cross-platform build system (CMake) and developer workflow (Makefile + shell scripts + Docker) supporting Windows, Linux, and macOS.

## Context

The Cimmeria server emulator currently bootstraps via a PowerShell 7 module (`CimmeriaBootstrap`) that handles dependency download, library builds, solution compilation, database initialization, and server lifecycle — all targeting Windows with Visual Studio.

This design creates a cross-platform pipeline using:
- **CMake** as the primary build system for all platforms (replacing W-NG.sln)
- **CMake Presets** for platform-specific configurations
- **Docker** for external services (PostgreSQL) on all platforms
- **Makefile** for Unix developer workflow (Linux/macOS)
- **Shell scripts** for Linux/macOS orchestration
- Existing **PowerShell scripts** simplified to call CMake (Windows)

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Build system | CMake (primary, all platforms) | Standard cross-platform C++; VS can generate from CMake |
| C++ standard | C++17 | Modern Boost/OpenSSL work best with C++17; enables std::filesystem |
| Dependency versions | Modern (Homebrew/apt/vcpkg) | Old versions won't compile on modern compilers across platforms |
| External services | Docker (all platforms) | PostgreSQL in Docker; consistent across Win/Lin/Mac |
| Unix orchestration | Makefile + shell scripts | Idiomatic for C++ on Unix; tab-completable |
| Windows orchestration | Existing PS1 simplified | Calls CMake instead of MSBuild; Docker for PostgreSQL |
| VS solution | Legacy/generated | CMake can generate .sln via `cmake -G "Visual Studio 17 2022"` |

## File Structure

### New files

```
CMakeLists.txt                                # Root CMake project
CMakePresets.json                             # Platform-specific presets
docker-compose.yml                            # PostgreSQL 17 on port 5433
Makefile                                      # Unix orchestrator (Linux/macOS)

src/lib/CMakeLists.txt                        # UnifiedKernel static library
src/server/AuthenticationServer/CMakeLists.txt
src/server/BaseApp/CMakeLists.txt
src/server/CellApp/CMakeLists.txt
src/server/NavBuilder/CMakeLists.txt

bootstrap/scripts/
  common.sh                                   # Colors, logging, timing, path helpers
  deps-macos.sh                               # Homebrew dependency installation
  deps-linux.sh                               # apt/dnf dependency installation
  build.sh                                    # CMake configure + build wrapper
  docker.sh                                   # Docker Compose lifecycle + healthcheck
  database.sh                                 # Schema loading + verification
  runtime.sh                                  # Verify shared lib resolution
  server.sh                                   # Start/stop servers with PID management
```

### Modified files

```
setup.ps1                                     # Simplified: calls CMake + Docker
bootstrap/CimmeriaBootstrap/                  # Updated to use CMake instead of MSBuild
```

### Preserved (unchanged)

```
W-NG.sln                                     # Kept for legacy/reference
bootstrap/patches/                            # Still needed for source-built deps
bootstrap/CimmeriaBootstrap/Data/             # Dependency metadata
```

## Makefile Targets (Linux/macOS)

| Target | What it does | PS1 Equivalent |
|--------|-------------|----------------|
| `make deps` | Install packages (Homebrew or apt) | `Install-CimmeriaDependencies` |
| `make configure` | `cmake --preset <platform>` | (none) |
| `make build` | `cmake --build --preset <platform>` | `Build-CimmeriaLibraries` + `Build-CimmeriaSolution` |
| `make docker-up` | `docker compose up -d`, wait for healthy | Part of `Initialize-CimmeriaDatabase` |
| `make docker-down` | `docker compose down` | Part of `Stop-CimmeriaServer` |
| `make db-init` | Docker up + load schemas via psql | `Initialize-CimmeriaDatabase` |
| `make db-reset` | Drop + recreate database, reload schemas | `Initialize-CimmeriaDatabase -Force` |
| `make runtime` | Verify shared lib resolution | `Initialize-CimmeriaRuntime` |
| `make start` | Launch Auth, Base, Cell with port checks | `Start-CimmeriaServer` |
| `make stop` | Kill servers + stop Docker | `Stop-CimmeriaServer` |
| `make all` | Full pipeline: deps -> build -> db-init -> start | `Invoke-CimmeriaBootstrap` |
| `make clean` | Remove build/ directory | (none) |

## Dependencies by Platform

### macOS (Homebrew)

| Package | Replaces | Notes |
|---------|----------|-------|
| `boost` | Boost 1.55.0 | ~1.87; Asio, Python, Thread, DateTime, Filesystem |
| `openssl@3` | OpenSSL 0.9.8i | Complete API migration needed |
| `python@3.12` | Python 3.4.1 | Boost.Python compatible |
| `postgresql@17` | PostgreSQL 9.2 (client) | libpq headers + psql CLI only |
| `cmake` | (none) | Build system |
| `sdl12-compat` | SDL 1.2.15 | SDL 1.2 compat layer |
| `qt@5` | Qt 5.15.18 | For ServerEd |

### Linux (apt — Ubuntu/Debian)

| Package | Notes |
|---------|-------|
| `libboost-all-dev` | System Boost |
| `libssl-dev` | OpenSSL 3.x headers |
| `python3-dev` | Python development headers |
| `libpq-dev` | PostgreSQL client library |
| `cmake` | Build system |
| `libsdl1.2-dev` or `libsdl1.2-compat-dev` | SDL 1.2 |
| `qtbase5-dev` | For ServerEd |

### Windows (vcpkg or existing bootstrap)

On Windows, two options:
1. **vcpkg** (recommended): `vcpkg install boost openssl libpq python3 sdl qt5`
2. **Existing bootstrap**: Keep `Install-CimmeriaDependencies` for Windows-specific deps

CMake Presets will handle both paths via toolchain files.

### Built from source (all platforms)

- **SOCI 4.x** — via CMake `FetchContent`
- **Recast/Detour** — compiled as CMake subproject
- **TinyXML2** — header-only or `FetchContent`

## Docker Compose

```yaml
services:
  postgres:
    image: postgres:17-alpine
    ports:
      - "5433:5432"
    environment:
      POSTGRES_USER: w-testing
      POSTGRES_PASSWORD: w-testing
      POSTGRES_DB: sgw
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U w-testing -d sgw"]
      interval: 2s
      timeout: 5s
      retries: 10

volumes:
  pgdata:
```

Same on all platforms. Port 5433 and credentials match existing config files.

## CMake Build System

### CMakePresets.json

```json
{
  "version": 6,
  "configurePresets": [
    {
      "name": "default",
      "binaryDir": "build",
      "cacheVariables": {
        "CMAKE_CXX_STANDARD": "17",
        "CMAKE_EXPORT_COMPILE_COMMANDS": "ON"
      }
    },
    {
      "name": "macos-debug",
      "inherits": "default",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug",
        "OPENSSL_ROOT_DIR": "/opt/homebrew/opt/openssl@3"
      }
    },
    {
      "name": "linux-debug",
      "inherits": "default",
      "cacheVariables": {
        "CMAKE_BUILD_TYPE": "Debug"
      }
    },
    {
      "name": "windows-debug",
      "inherits": "default",
      "generator": "Visual Studio 17 2022",
      "architecture": "x64"
    }
  ]
}
```

### Root CMakeLists.txt

- `cmake_minimum_required(VERSION 3.20)`
- `project(Cimmeria CXX)`
- `set(CMAKE_CXX_STANDARD 17)`
- Platform detection and package finding
- SOCI via `FetchContent`
- Recast/Detour as subdirectory
- All server targets

### CMake Targets

| CMake Target | Type | Links Against |
|---|---|---|
| `unified_kernel` | STATIC | Boost, OpenSSL, Python3, PostgreSQL, SOCI |
| `authentication_server` | EXECUTABLE | unified_kernel |
| `base_app` | EXECUTABLE | unified_kernel |
| `cell_app` | EXECUTABLE | unified_kernel |
| `nav_builder` | EXECUTABLE | unified_kernel, Recast, Detour |
| `recast` | STATIC | (none) |
| `detour` | STATIC | (none) |
| `server_ed` | EXECUTABLE | unified_kernel, Qt5 (optional) |

Source file lists derived from existing `.vcxproj` files.

### Platform-Specific CMake Handling

```cmake
if(WIN32)
    # Windows-specific: DLL copying, manifest embedding
    target_compile_definitions(unified_kernel PRIVATE WIN32_LEAN_AND_MEAN NOMINMAX)
elseif(APPLE)
    # macOS: RPATH for Homebrew, framework paths
    set(CMAKE_INSTALL_RPATH_USE_LINK_PATH TRUE)
    set(CMAKE_MACOSX_RPATH TRUE)
elseif(UNIX)
    # Linux: standard paths, -pthread
    find_package(Threads REQUIRED)
endif()
```

### Key Considerations
- Precompiled headers via `target_precompile_headers` (matching `stdafx.hpp`)
- `target_include_directories` derived from VS "Additional Include Directories"
- RPATH configured per-platform for runtime library resolution
- `#ifdef _WIN32` / `__APPLE__` / `__linux__` guards where needed
- Build output to `build/bin/` on all platforms

## Server Lifecycle

### Start (`make start` / PowerShell `Start-CimmeriaServer`)
1. Ensure Docker PostgreSQL is running and healthy
2. Launch `authentication_server` in background, wait for port 8081 (15s timeout)
3. Launch `base_app` in background, wait for port 32832 (15s timeout)
4. Launch `cell_app` in background
5. Write PIDs to `server/pids/` (Unix) or track process IDs (Windows)
6. Print status banner with connection details

### Stop (`make stop` / PowerShell `Stop-CimmeriaServer`)
1. Read PIDs, send SIGTERM (Unix) or Stop-Process (Windows)
2. Fall back to `pkill -f` / `Get-Process` pattern matching
3. `docker compose stop`

### Signal handling
Trap `SIGINT`/`SIGTERM` in `server.sh` for graceful cleanup on Unix.

## Database Initialization

Same on all platforms (Docker handles the PostgreSQL server):

1. `docker compose up -d` — start PostgreSQL container
2. Wait for Docker healthcheck (`pg_isready`)
3. Check if schemas already loaded (query `information_schema.tables`)
4. If not loaded: `psql -h localhost -p 5433 -U w-testing -d sgw -f db/database.sql`
5. Verify key tables: `account`, `sgw_player`

Reset mode: drops and recreates `sgw` database before loading.

## Migration Path from Current Windows Setup

The CMake system runs alongside the existing VS solution during transition:

1. **Phase 1:** Create CMake files + Docker + scripts. Both build systems work.
2. **Phase 2:** Update C++ code for modern library APIs (Boost 1.87, OpenSSL 3.x, Python 3.12).
3. **Phase 3:** Validate on all three platforms.
4. **Phase 4:** Deprecate W-NG.sln; CMake generates VS solutions when needed.

## What This Does NOT Cover

- **C++ code changes for modern library APIs** — Boost 1.87, OpenSSL 3.x, Python 3.12 have breaking changes. CMake files will be created but the code needs separate migration work.
- **CI/CD pipeline** — GitHub Actions or similar for automated cross-platform builds.
- **Cross-compilation** — Each platform builds natively.
- **Package distribution** — No installers or deployment packaging.
