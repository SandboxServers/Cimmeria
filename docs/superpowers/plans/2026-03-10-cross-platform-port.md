# Cross-Platform Port Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Windows-only PowerShell bootstrap with a cross-platform CMake build system, Docker services, and shell scripts supporting Windows, Linux, and macOS.

**Architecture:** CMake as the primary build system (replacing W-NG.sln), Docker Compose for PostgreSQL, Makefile + bash scripts for Unix developer workflow. Homebrew (macOS) and apt (Linux) for modern dependency versions. Existing PowerShell scripts preserved alongside new infrastructure.

**Tech Stack:** CMake 3.20+, Docker Compose, Bash, PostgreSQL 17, Boost (Homebrew/apt), OpenSSL 3.x, Python 3.12+, SOCI 4.x (FetchContent), Recast/Detour (subproject)

**Spec:** `docs/superpowers/specs/2026-03-10-macos-port-design.md`

**Important:** The C++ code currently uses Windows-specific APIs and old library versions (Boost 1.55, OpenSSL 0.9.8i, Python 3.4). The CMake files will be created and should configure correctly, but the code itself will not compile until separate migration work updates the source for modern library APIs. This plan creates the build infrastructure; code migration is a follow-up effort.

---

## File Structure

### New files to create:

```
docker-compose.yml                                # PostgreSQL 17 service
CMakeLists.txt                                    # Root CMake project
CMakePresets.json                                 # Platform-specific build presets
cmake/Platform.cmake                              # Platform detection + package finding
Makefile                                          # Unix orchestrator

bootstrap/scripts/
  common.sh                                       # ANSI colors, logging, timing, path helpers
  deps-macos.sh                                   # Homebrew install
  deps-linux.sh                                   # apt install
  build.sh                                        # cmake configure + build
  docker.sh                                       # docker compose up/down/wait
  database.sh                                     # schema loading + verification
  runtime.sh                                      # verify shared lib resolution
  server.sh                                       # start/stop server processes

src/lib/CMakeLists.txt                            # UnifiedKernel static library
src/server/AuthenticationServer/CMakeLists.txt    # Auth server executable
src/server/BaseApp/CMakeLists.txt                 # BaseApp executable
src/server/CellApp/CMakeLists.txt                 # CellApp executable
src/server/NavBuilder/CMakeLists.txt              # NavBuilder executable
tools/ServerEd/CMakeLists.txt                     # ServerEd Qt admin tool (optional)
```

### Existing files (unchanged):
```
W-NG.sln                                         # Preserved for Windows legacy
projects/**/*.vcxproj                             # Preserved
bootstrap/CimmeriaBootstrap/                      # Preserved
setup.ps1                                         # Preserved
```

---

## Chunk 1: Docker + Shell Infrastructure

### Task 1: Docker Compose for PostgreSQL

**Files:**
- Create: `docker-compose.yml`

- [ ] **Step 1: Create docker-compose.yml**

```yaml
services:
  postgres:
    image: postgres:17-alpine
    container_name: cimmeria-postgres
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

- [ ] **Step 2: Verify docker-compose syntax**

Run: `docker compose config`
Expected: YAML parsed without errors, services listed.

- [ ] **Step 3: Test PostgreSQL starts**

Run: `docker compose up -d && docker compose exec postgres pg_isready -U w-testing -d sgw`
Expected: `localhost:5432 - accepting connections`
Cleanup: `docker compose down`

- [ ] **Step 4: Commit**

```bash
git add docker-compose.yml
git commit -m "infra: add docker-compose for PostgreSQL 17"
```

---

### Task 2: Shell helper library (common.sh)

**Files:**
- Create: `bootstrap/scripts/common.sh`

- [ ] **Step 1: Create common.sh**

```bash
#!/usr/bin/env bash
# Common helpers for Cimmeria bootstrap scripts.
# Source this file; do not execute directly.

set -euo pipefail

# --- Colors (disabled if not a terminal) ---
if [[ -t 1 ]]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
    CYAN='\033[0;36m'; GRAY='\033[0;90m'; BOLD='\033[1m'; NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; CYAN=''; GRAY=''; BOLD=''; NC=''
fi

# --- Project paths ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
EXTERNAL_DIR="$PROJECT_ROOT/external"
BUILD_DIR="$PROJECT_ROOT/build"
CONFIG_DIR="$PROJECT_ROOT/config"
DB_DIR="$PROJECT_ROOT/db"
SERVER_DIR="$PROJECT_ROOT/server"
PID_DIR="$SERVER_DIR/pids"

# --- Timing ---
_START_TIME=$(date +%s)

_elapsed() {
    local now=$(date +%s)
    local diff=$((now - _START_TIME))
    printf "%02d:%02d" $((diff / 60)) $((diff % 60))
}

# --- Logging ---
log_status() {
    local msg="$1"
    local color="${2:-$GRAY}"
    printf "${color}[$(_elapsed)]  %s${NC}\n" "$msg"
}

log_step() {
    local msg="$1"
    printf "\n${CYAN}[$(_elapsed)] === %s ===${NC}\n" "$msg"
}

log_ok()    { log_status "$1" "$GREEN"; }
log_warn()  { log_status "$1" "$YELLOW"; }
log_err()   { log_status "$1" "$RED"; }

die() {
    log_err "$1"
    exit 1
}

# --- Port checking ---
wait_for_port() {
    local port="$1"
    local timeout="${2:-15}"
    local deadline=$(($(date +%s) + timeout))

    while [[ $(date +%s) -lt $deadline ]]; do
        if nc -z 127.0.0.1 "$port" 2>/dev/null; then
            return 0
        fi
        sleep 0.5
    done
    return 1
}

# --- OS detection ---
detect_os() {
    case "$(uname -s)" in
        Darwin) echo "macos" ;;
        Linux)  echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *) echo "unknown" ;;
    esac
}

OS="$(detect_os)"
```

- [ ] **Step 2: Verify common.sh sources without error**

Run: `bash -c 'source bootstrap/scripts/common.sh && echo "OS=$OS PROJECT_ROOT=$PROJECT_ROOT"'`
Expected: `OS=macos PROJECT_ROOT=/Users/heidornj/Code/Cimmeria`

- [ ] **Step 3: Commit**

```bash
git add bootstrap/scripts/common.sh
git commit -m "infra: add shell helper library (common.sh)"
```

---

### Task 3: Docker management script

**Files:**
- Create: `bootstrap/scripts/docker.sh`

- [ ] **Step 1: Create docker.sh**

```bash
#!/usr/bin/env bash
# Docker Compose management for Cimmeria services.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

docker_up() {
    log_step "STARTING DOCKER SERVICES"

    if ! command -v docker &>/dev/null; then
        die "Docker not found. Install Docker Desktop or docker CLI."
    fi

    cd "$PROJECT_ROOT"
    docker compose up -d

    log_status "Waiting for PostgreSQL to be healthy..."
    local timeout=30
    local deadline=$(($(date +%s) + timeout))
    while [[ $(date +%s) -lt $deadline ]]; do
        if docker compose exec -T postgres pg_isready -U w-testing -d sgw &>/dev/null; then
            log_ok "PostgreSQL is ready (port 5433)."
            return 0
        fi
        sleep 1
    done
    die "PostgreSQL did not become healthy within ${timeout}s."
}

docker_down() {
    log_step "STOPPING DOCKER SERVICES"
    cd "$PROJECT_ROOT"
    docker compose down
    log_ok "Docker services stopped."
}

docker_status() {
    cd "$PROJECT_ROOT"
    docker compose ps
}

# Allow direct invocation: ./docker.sh up|down|status
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        up)     docker_up ;;
        down)   docker_down ;;
        status) docker_status ;;
        *)      echo "Usage: $0 {up|down|status}" ;;
    esac
fi
```

- [ ] **Step 2: Make executable and test**

Run: `chmod +x bootstrap/scripts/docker.sh && bootstrap/scripts/docker.sh up`
Expected: PostgreSQL container starts, "PostgreSQL is ready (port 5433)" printed.
Cleanup: `bootstrap/scripts/docker.sh down`

- [ ] **Step 3: Commit**

```bash
git add bootstrap/scripts/docker.sh
git commit -m "infra: add Docker Compose management script"
```

---

### Task 4: Database initialization script

**Files:**
- Create: `bootstrap/scripts/database.sh`

- [ ] **Step 1: Create database.sh**

```bash
#!/usr/bin/env bash
# Database initialization for Cimmeria.
# Requires: Docker PostgreSQL running, psql on PATH.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

DB_PORT="${CIMMERIA_DB_PORT:-5433}"
DB_USER="w-testing"
DB_NAME="sgw"
DB_HOST="localhost"

_psql() {
    PGPASSWORD="$DB_USER" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" "$@"
}

_psql_postgres() {
    PGPASSWORD="$DB_USER" psql -h "$DB_HOST" -p "$DB_PORT" -U postgres "$@"
}

db_init() {
    local force="${1:-false}"
    log_step "INITIALIZING DATABASE"

    # Ensure Docker is running
    if ! nc -z "$DB_HOST" "$DB_PORT" 2>/dev/null; then
        log_status "PostgreSQL not running, starting Docker..."
        source "$(dirname "${BASH_SOURCE[0]}")/docker.sh"
        docker_up
    fi

    # Wait for PostgreSQL to accept queries
    log_status "Waiting for PostgreSQL to accept queries..."
    local ready=false
    for i in $(seq 1 15); do
        if _psql_postgres -tAc "SELECT 1" &>/dev/null; then
            ready=true
            break
        fi
        sleep 1
    done
    if [[ "$ready" != "true" ]]; then
        die "PostgreSQL not accepting queries after 15s."
    fi

    # Force mode: drop and recreate
    if [[ "$force" == "true" ]]; then
        log_warn "Force mode: dropping and recreating database '$DB_NAME'..."
        _psql_postgres -c "DROP DATABASE IF EXISTS $DB_NAME" 2>/dev/null || true
        _psql_postgres -c "CREATE DATABASE $DB_NAME OWNER \"$DB_USER\"" || die "Failed to recreate database."
        log_ok "Database '$DB_NAME' recreated."
    fi

    # Check if schemas already loaded
    local table_check
    table_check=$(_psql -d "$DB_NAME" -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='account'" 2>/dev/null || echo "")
    if [[ "$table_check" == *"1"* ]] && [[ "$force" != "true" ]]; then
        log_status "Schemas already loaded (account table exists). Use --force to reload." "$GRAY"
        return 0
    fi

    # Load schemas
    local db_sql="$DB_DIR/database.sql"
    if [[ ! -f "$db_sql" ]]; then
        die "Schema file not found: $db_sql"
    fi

    log_status "Loading database.sql..."
    _psql -d "$DB_NAME" -v ON_ERROR_STOP=1 -f "$db_sql" 2>&1 | while IFS= read -r line; do
        if [[ "$line" == *ERROR* ]] || [[ "$line" == *FATAL* ]]; then
            log_err "  $line"
        fi
    done

    if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
        die "database.sql failed. Aborting."
    fi
    log_ok "database.sql loaded."

    # Verify key tables
    log_status "Verifying schema..."
    local all_good=true
    for table in account sgw_player; do
        local check
        check=$(_psql -d "$DB_NAME" -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='$table'" 2>/dev/null || echo "")
        if [[ "$check" == *"1"* ]]; then
            log_ok "  [OK] $table"
        else
            log_err "  [--] $table"
            all_good=false
        fi
    done

    if [[ "$all_good" != "true" ]]; then
        die "Schema verification failed — some tables are missing."
    fi

    log_ok "Database initialized."
    log_status "  PostgreSQL: ${DB_HOST}:${DB_PORT}" "$GRAY"
    log_status "  Database:   $DB_NAME" "$GRAY"
    log_status "  Role:       $DB_USER" "$GRAY"
    log_status "  Test login: test / test (SHA1 hashed)" "$GRAY"
}

# Allow direct invocation
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        init)   db_init false ;;
        reset)  db_init true ;;
        *)      echo "Usage: $0 {init|reset}" ;;
    esac
fi
```

- [ ] **Step 2: Test database init**

Run:
```bash
chmod +x bootstrap/scripts/database.sh
bootstrap/scripts/docker.sh up
bootstrap/scripts/database.sh init
```
Expected: Schema loads, "account" and "sgw_player" tables verified.

- [ ] **Step 3: Test database reset**

Run: `bootstrap/scripts/database.sh reset`
Expected: Database dropped, recreated, schemas reloaded.

- [ ] **Step 4: Commit**

```bash
git add bootstrap/scripts/database.sh
git commit -m "infra: add database initialization script"
```

---

### Task 5: Dependency installation scripts

**Files:**
- Create: `bootstrap/scripts/deps-macos.sh`
- Create: `bootstrap/scripts/deps-linux.sh`

- [ ] **Step 1: Create deps-macos.sh**

```bash
#!/usr/bin/env bash
# Install Cimmeria dependencies via Homebrew (macOS).
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

install_deps_macos() {
    log_step "INSTALLING DEPENDENCIES (macOS / Homebrew)"

    if ! command -v brew &>/dev/null; then
        die "Homebrew not found. Install from https://brew.sh"
    fi

    log_status "Updating Homebrew..."
    brew update

    local packages=(
        cmake
        boost
        openssl@3
        python@3.12
        postgresql@17
        sdl12-compat
    )

    log_status "Installing packages: ${packages[*]}"
    brew install "${packages[@]}"

    # Qt 5 — try qt@5, fall back to qt
    if brew list qt@5 &>/dev/null || brew install qt@5 2>/dev/null; then
        log_ok "Qt 5 installed."
    else
        log_warn "qt@5 not available, installing qt (6.x). ServerEd may need adjustments."
        brew install qt
    fi

    # Ensure postgresql@17 tools are on PATH
    if ! command -v psql &>/dev/null; then
        log_warn "psql not on PATH. Add to your shell profile:"
        log_warn "  export PATH=\"/opt/homebrew/opt/postgresql@17/bin:\$PATH\""
    fi

    # Print summary
    log_ok "Dependencies installed."
    log_status "Boost:    $(brew info --json=v2 boost 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["formulae"][0]["versions"]["stable"])' 2>/dev/null || echo 'installed')" "$GRAY"
    log_status "OpenSSL:  $(openssl version 2>/dev/null || echo 'installed')" "$GRAY"
    log_status "Python:   $(python3 --version 2>/dev/null || echo 'installed')" "$GRAY"
    log_status "CMake:    $(cmake --version 2>/dev/null | head -1 || echo 'installed')" "$GRAY"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_deps_macos
fi
```

- [ ] **Step 2: Create deps-linux.sh**

```bash
#!/usr/bin/env bash
# Install Cimmeria dependencies via apt (Ubuntu/Debian).
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

install_deps_linux() {
    log_step "INSTALLING DEPENDENCIES (Linux / apt)"

    if ! command -v apt-get &>/dev/null; then
        die "apt-get not found. This script supports Ubuntu/Debian. For other distros, install equivalent packages manually."
    fi

    log_status "Updating package lists..."
    sudo apt-get update

    local packages=(
        cmake
        build-essential
        libboost-all-dev
        libssl-dev
        python3-dev
        libpq-dev
        libsdl1.2-dev
        qtbase5-dev
        docker.io
        docker-compose-v2
    )

    log_status "Installing packages: ${packages[*]}"
    sudo apt-get install -y "${packages[@]}"

    log_ok "Dependencies installed."
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_deps_linux
fi
```

- [ ] **Step 3: Make executable**

Run: `chmod +x bootstrap/scripts/deps-macos.sh bootstrap/scripts/deps-linux.sh`

- [ ] **Step 4: Test macOS install (dry run)**

Run: `bash -c 'source bootstrap/scripts/deps-macos.sh && echo "Would install deps"'`
Expected: Script sources without error.

- [ ] **Step 5: Commit**

```bash
git add bootstrap/scripts/deps-macos.sh bootstrap/scripts/deps-linux.sh
git commit -m "infra: add platform dependency installation scripts"
```

---

### Task 6: Server lifecycle script

**Files:**
- Create: `bootstrap/scripts/server.sh`

- [ ] **Step 1: Create server.sh**

```bash
#!/usr/bin/env bash
# Start/stop Cimmeria game servers.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

CONFIGURATION="${CIMMERIA_CONFIG:-Debug}"
BIN_DIR="$BUILD_DIR/bin"

_suffix() {
    if [[ "$CONFIGURATION" == "Debug" ]]; then echo "_d"; else echo ""; fi
}

_ensure_pid_dir() {
    mkdir -p "$PID_DIR"
}

_start_process() {
    local name="$1"
    local exe="$2"
    local port="${3:-}"
    local timeout="${4:-15}"

    if [[ ! -x "$exe" ]]; then
        die "Executable not found: $exe. Run 'make build' first."
    fi

    log_status "Starting $name..."
    cd "$BIN_DIR"
    "$exe" &
    local pid=$!
    echo "$pid" > "$PID_DIR/${name}.pid"

    if [[ -n "$port" ]]; then
        if wait_for_port "$port" "$timeout"; then
            log_ok "$name ready (port $port, PID $pid)."
        else
            log_warn "$name did not respond on port $port within ${timeout}s (PID $pid)."
            # Check if process is still alive
            if kill -0 "$pid" 2>/dev/null; then
                log_warn "  Process is alive — may still be initializing."
            else
                die "$name failed to start."
            fi
        fi
    else
        if kill -0 "$pid" 2>/dev/null; then
            log_ok "$name started (PID $pid)."
        else
            die "$name failed to start."
        fi
    fi
}

_stop_process() {
    local name="$1"
    local pidfile="$PID_DIR/${name}.pid"

    if [[ -f "$pidfile" ]]; then
        local pid
        pid=$(cat "$pidfile")
        if kill -0 "$pid" 2>/dev/null; then
            log_status "Stopping $name (PID $pid)..."
            kill "$pid" 2>/dev/null || true
            # Wait up to 5s for graceful shutdown
            for i in $(seq 1 10); do
                if ! kill -0 "$pid" 2>/dev/null; then break; fi
                sleep 0.5
            done
            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                kill -9 "$pid" 2>/dev/null || true
            fi
        fi
        rm -f "$pidfile"
    else
        # Fall back to pkill
        pkill -f "$name" 2>/dev/null || true
    fi
}

server_start() {
    log_step "STARTING CIMMERIA SERVER"

    # Ensure Docker PostgreSQL is running
    if ! nc -z 127.0.0.1 "${CIMMERIA_DB_PORT:-5433}" 2>/dev/null; then
        log_status "PostgreSQL not running, starting Docker..."
        source "$(dirname "${BASH_SOURCE[0]}")/docker.sh"
        docker_up
    fi

    _ensure_pid_dir

    local sfx
    sfx=$(_suffix)

    _start_process "AuthenticationServer" "$BIN_DIR/AuthenticationServer${sfx}" 8081 15
    _start_process "BaseApp" "$BIN_DIR/BaseApp${sfx}" 32832 15
    _start_process "CellApp" "$BIN_DIR/CellApp${sfx}" "" ""

    # Status banner
    printf "\n"
    printf "${GREEN}=============================================${NC}\n"
    printf "${GREEN} Cimmeria Server Running${NC}\n"
    printf "${GREEN}=============================================${NC}\n"
    printf " Auth server:   http://localhost:8081 (client login)\n"
    printf " Shard server:  localhost:32832 (game client)\n"
    printf " PostgreSQL:    localhost:${CIMMERIA_DB_PORT:-5433}\n"
    printf " Test account:  test / test\n"
    printf "${GREEN}=============================================${NC}\n"
    printf "\n"
    printf " Stop with: make stop\n"
    printf "${GREEN}=============================================${NC}\n"
}

server_stop() {
    log_step "STOPPING CIMMERIA SERVER"

    _stop_process "CellApp"
    _stop_process "BaseApp"
    _stop_process "AuthenticationServer"

    log_ok "All game servers stopped."
}

# Trap for Ctrl+C
trap 'echo ""; server_stop; exit 0' INT TERM

# Allow direct invocation
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        start) server_start ;;
        stop)  server_stop ;;
        *)     echo "Usage: $0 {start|stop}" ;;
    esac
fi
```

- [ ] **Step 2: Make executable**

Run: `chmod +x bootstrap/scripts/server.sh`

- [ ] **Step 3: Commit**

```bash
git add bootstrap/scripts/server.sh
git commit -m "infra: add server start/stop lifecycle script"
```

---

### Task 7: Build wrapper and runtime verification scripts

**Files:**
- Create: `bootstrap/scripts/build.sh`
- Create: `bootstrap/scripts/runtime.sh`

- [ ] **Step 1: Create build.sh**

```bash
#!/usr/bin/env bash
# CMake configure + build wrapper.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

CONFIGURATION="${CIMMERIA_CONFIG:-Debug}"
PRESET="${CIMMERIA_PRESET:-}"

cmake_configure() {
    log_step "CONFIGURING CMAKE"

    if ! command -v cmake &>/dev/null; then
        die "CMake not found. Run 'make deps' first."
    fi

    local cmake_args=(-B "$BUILD_DIR")

    if [[ -n "$PRESET" ]]; then
        cmake_args=(--preset "$PRESET")
    else
        cmake_args+=(-DCMAKE_BUILD_TYPE="$CONFIGURATION")
        cmake_args+=(-DCMAKE_EXPORT_COMPILE_COMMANDS=ON)

        # Platform-specific hints
        if [[ "$OS" == "macos" ]]; then
            cmake_args+=(-DOPENSSL_ROOT_DIR="$(brew --prefix openssl@3 2>/dev/null || echo '/opt/homebrew/opt/openssl@3')")
            cmake_args+=(-DPython3_ROOT_DIR="$(brew --prefix python@3.12 2>/dev/null || echo '/opt/homebrew/opt/python@3.12')")
        fi
    fi

    log_status "cmake ${cmake_args[*]}"
    cmake "${cmake_args[@]}" "$PROJECT_ROOT"
    log_ok "CMake configured."
}

cmake_build() {
    log_step "BUILDING"

    if [[ ! -d "$BUILD_DIR" ]]; then
        cmake_configure
    fi

    local jobs
    if [[ "$OS" == "macos" ]]; then
        jobs=$(sysctl -n hw.ncpu)
    else
        jobs=$(nproc)
    fi

    log_status "Building with $jobs parallel jobs..."
    cmake --build "$BUILD_DIR" --parallel "$jobs"

    # Verify executables
    log_status "Verifying build output..."
    local all_found=true
    for exe in AuthenticationServer BaseApp CellApp NavBuilder; do
        if [[ -f "$BUILD_DIR/bin/$exe" ]] || [[ -f "$BUILD_DIR/bin/${exe}_d" ]]; then
            log_ok "  Found: $exe"
        else
            log_err "  Missing: $exe"
            all_found=false
        fi
    done

    if [[ "$all_found" != "true" ]]; then
        log_warn "Not all executables were produced. Check build output."
    fi

    log_ok "Build complete."
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    case "${1:-}" in
        configure) cmake_configure ;;
        build)     cmake_build ;;
        *)         cmake_build ;;
    esac
fi
```

- [ ] **Step 2: Create runtime.sh**

```bash
#!/usr/bin/env bash
# Verify runtime library resolution for built executables.
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

verify_runtime() {
    log_step "VERIFYING RUNTIME LIBRARIES"

    local bin_dir="$BUILD_DIR/bin"
    if [[ ! -d "$bin_dir" ]]; then
        die "Build directory not found: $bin_dir. Run 'make build' first."
    fi

    local all_ok=true

    for exe in "$bin_dir"/*; do
        [[ -x "$exe" ]] || continue
        [[ -f "$exe" ]] || continue
        local name
        name=$(basename "$exe")

        log_status "Checking $name..."

        if [[ "$OS" == "macos" ]]; then
            local missing
            missing=$(otool -L "$exe" 2>/dev/null | grep "not found" || true)
            if [[ -n "$missing" ]]; then
                log_err "  Missing libraries:"
                echo "$missing" | while read -r line; do log_err "    $line"; done
                all_ok=false
            else
                log_ok "  $name: all libraries resolved."
            fi
        elif [[ "$OS" == "linux" ]]; then
            local missing
            missing=$(ldd "$exe" 2>/dev/null | grep "not found" || true)
            if [[ -n "$missing" ]]; then
                log_err "  Missing libraries:"
                echo "$missing" | while read -r line; do log_err "    $line"; done
                all_ok=false
            else
                log_ok "  $name: all libraries resolved."
            fi
        fi
    done

    if [[ "$all_ok" == "true" ]]; then
        log_ok "All runtime libraries resolved."
    else
        die "Some libraries are missing. Check output above."
    fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    verify_runtime
fi
```

- [ ] **Step 3: Make executable and commit**

```bash
chmod +x bootstrap/scripts/build.sh bootstrap/scripts/runtime.sh
git add bootstrap/scripts/build.sh bootstrap/scripts/runtime.sh
git commit -m "infra: add build wrapper and runtime verification scripts"
```

---

## Chunk 2: CMake Build System

### Task 8: Root CMakeLists.txt and presets

**Files:**
- Create: `CMakeLists.txt`
- Create: `CMakePresets.json`
- Create: `cmake/Platform.cmake`

- [ ] **Step 1: Create cmake/Platform.cmake**

```cmake
# Platform detection and package configuration for Cimmeria.

# --- Compiler settings ---
if(MSVC)
    add_compile_options(/W3 /EHsc /permissive-)
    add_compile_definitions(
        WIN32_LEAN_AND_MEAN
        NOMINMAX
        _CRT_SECURE_NO_WARNINGS
        _CRT_SECURE_NO_DEPRECATE
        _WIN32_WINNT=0x0600
        BOOST_PYTHON_STATIC_LIB
    )
elseif(APPLE)
    add_compile_options(-Wall -Wextra -Wpedantic)
    set(CMAKE_MACOSX_RPATH TRUE)
    set(CMAKE_INSTALL_RPATH_USE_LINK_PATH TRUE)
elseif(UNIX)
    add_compile_options(-Wall -Wextra -Wpedantic)
    find_package(Threads REQUIRED)
endif()

# --- Find packages ---

# Boost
set(Boost_USE_STATIC_LIBS ON)
find_package(Boost REQUIRED COMPONENTS
    system
    thread
    filesystem
    date_time
    python
)

# OpenSSL
find_package(OpenSSL REQUIRED)

# Python 3
find_package(Python3 REQUIRED COMPONENTS Development)

# PostgreSQL (client library)
find_package(PostgreSQL REQUIRED)
```

- [ ] **Step 2: Create CMakePresets.json**

```json
{
    "version": 6,
    "cmakeMinimumRequired": {
        "major": 3,
        "minor": 20,
        "patch": 0
    },
    "configurePresets": [
        {
            "name": "default",
            "hidden": true,
            "binaryDir": "${sourceDir}/build",
            "cacheVariables": {
                "CMAKE_CXX_STANDARD": "17",
                "CMAKE_CXX_STANDARD_REQUIRED": "ON",
                "CMAKE_EXPORT_COMPILE_COMMANDS": "ON"
            }
        },
        {
            "name": "macos-debug",
            "displayName": "macOS Debug",
            "inherits": "default",
            "cacheVariables": {
                "CMAKE_BUILD_TYPE": "Debug",
                "OPENSSL_ROOT_DIR": "/opt/homebrew/opt/openssl@3"
            },
            "condition": {
                "type": "equals",
                "lhs": "${hostSystemName}",
                "rhs": "Darwin"
            }
        },
        {
            "name": "macos-release",
            "displayName": "macOS Release",
            "inherits": "default",
            "cacheVariables": {
                "CMAKE_BUILD_TYPE": "Release",
                "OPENSSL_ROOT_DIR": "/opt/homebrew/opt/openssl@3"
            },
            "condition": {
                "type": "equals",
                "lhs": "${hostSystemName}",
                "rhs": "Darwin"
            }
        },
        {
            "name": "linux-debug",
            "displayName": "Linux Debug",
            "inherits": "default",
            "cacheVariables": {
                "CMAKE_BUILD_TYPE": "Debug"
            },
            "condition": {
                "type": "equals",
                "lhs": "${hostSystemName}",
                "rhs": "Linux"
            }
        },
        {
            "name": "linux-release",
            "displayName": "Linux Release",
            "inherits": "default",
            "cacheVariables": {
                "CMAKE_BUILD_TYPE": "Release"
            },
            "condition": {
                "type": "equals",
                "lhs": "${hostSystemName}",
                "rhs": "Linux"
            }
        },
        {
            "name": "windows-debug",
            "displayName": "Windows Debug",
            "inherits": "default",
            "generator": "Visual Studio 17 2022",
            "architecture": {
                "value": "x64",
                "strategy": "set"
            },
            "cacheVariables": {
                "CMAKE_CONFIGURATION_TYPES": "Debug;Release"
            },
            "condition": {
                "type": "equals",
                "lhs": "${hostSystemName}",
                "rhs": "Windows"
            }
        },
        {
            "name": "windows-release",
            "displayName": "Windows Release",
            "inherits": "windows-debug"
        }
    ],
    "buildPresets": [
        {
            "name": "macos-debug",
            "configurePreset": "macos-debug"
        },
        {
            "name": "macos-release",
            "configurePreset": "macos-release"
        },
        {
            "name": "linux-debug",
            "configurePreset": "linux-debug"
        },
        {
            "name": "linux-release",
            "configurePreset": "linux-release"
        },
        {
            "name": "windows-debug",
            "configurePreset": "windows-debug",
            "configuration": "Debug"
        },
        {
            "name": "windows-release",
            "configurePreset": "windows-release",
            "configuration": "Release"
        }
    ]
}
```

- [ ] **Step 3: Create root CMakeLists.txt**

```cmake
cmake_minimum_required(VERSION 3.20)
project(Cimmeria CXX)

# --- Global settings ---
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Output directories
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)

# --- Platform configuration ---
include(cmake/Platform.cmake)

# --- Vendored dependencies ---

# SOCI (fetched at configure time)
include(FetchContent)
FetchContent_Declare(
    soci
    GIT_REPOSITORY https://github.com/SOCI/soci.git
    GIT_TAG        v4.1.0
)
set(SOCI_SHARED OFF CACHE BOOL "" FORCE)
set(SOCI_STATIC ON CACHE BOOL "" FORCE)
set(SOCI_TESTS OFF CACHE BOOL "" FORCE)
set(SOCI_CXX11 ON CACHE BOOL "" FORCE)
set(WITH_POSTGRESQL ON CACHE BOOL "" FORCE)
set(WITH_SQLITE3 OFF CACHE BOOL "" FORCE)
set(WITH_MYSQL OFF CACHE BOOL "" FORCE)
set(WITH_ODBC OFF CACHE BOOL "" FORCE)
set(WITH_BOOST OFF CACHE BOOL "" FORCE)
set(SOCI_POSTGRESQL_INCLUDE_DIR ${PostgreSQL_INCLUDE_DIRS} CACHE STRING "" FORCE)
FetchContent_MakeAvailable(soci)

# Recast/Detour (vendored in external/)
if(EXISTS ${CMAKE_SOURCE_DIR}/external/recast/Recast/Include)
    add_subdirectory(external/recast EXCLUDE_FROM_ALL)
elseif(EXISTS ${CMAKE_SOURCE_DIR}/external/recast)
    # Recast source may have different structure
    message(WARNING "Recast found but structure not recognized. NavBuilder may not build.")
endif()

# TinyXML2 is compiled as part of UnifiedKernel (src/xml/tinyxml2.cpp)

# --- Project targets ---
add_subdirectory(src/lib)
add_subdirectory(src/server/AuthenticationServer)
add_subdirectory(src/server/BaseApp)
add_subdirectory(src/server/CellApp)
add_subdirectory(src/server/NavBuilder)
```

Note: The Recast `add_subdirectory` assumes we create a CMakeLists.txt inside `external/recast/`. That's handled in Task 9.

- [ ] **Step 4: Skip verification until Task 11 is complete**

CMake configure will fail at this point because `add_subdirectory()` targets (src/lib, src/server/*) don't have CMakeLists.txt files yet. Full configure verification happens in Task 14.

- [ ] **Step 5: Commit**

```bash
mkdir -p cmake
git add CMakeLists.txt CMakePresets.json cmake/Platform.cmake
git commit -m "build: add root CMake project with platform presets"
```

---

### Task 9: Recast/Detour CMakeLists.txt

**Files:**
- Create: `external/recast/CMakeLists.txt`

Note: `external/` is in .gitignore but is populated by the dependency installer. This CMakeLists.txt lives inside the extracted Recast source tree. We'll create it as part of the build infrastructure — if the directory doesn't exist, the root CMakeLists.txt skips it gracefully.

- [ ] **Step 1: Create external/recast/CMakeLists.txt**

This file will be generated by the deps script after extraction. For now, create it as a template in the bootstrap directory.

Create: `bootstrap/templates/recast-CMakeLists.txt`

```cmake
# CMakeLists.txt for vendored Recast/Detour navigation library.
# Placed into external/recast/ by the dependency installer.

# --- Recast (navmesh generation) ---
file(GLOB RECAST_SOURCES Recast/Source/*.cpp)
add_library(recast STATIC ${RECAST_SOURCES})
target_include_directories(recast PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/Recast/Include
)

# --- Detour (pathfinding) ---
file(GLOB DETOUR_SOURCES Detour/Source/*.cpp)
add_library(detour STATIC ${DETOUR_SOURCES})
target_include_directories(detour PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/Detour/Include
)

# --- DetourCrowd ---
file(GLOB DETOUR_CROWD_SOURCES DetourCrowd/Source/*.cpp)
add_library(detour_crowd STATIC ${DETOUR_CROWD_SOURCES})
target_include_directories(detour_crowd PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/DetourCrowd/Include
)
target_link_libraries(detour_crowd PUBLIC detour)

# --- DetourTileCache ---
file(GLOB DETOUR_TILECACHE_SOURCES DetourTileCache/Source/*.cpp)
add_library(detour_tilecache STATIC ${DETOUR_TILECACHE_SOURCES})
target_include_directories(detour_tilecache PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/DetourTileCache/Include
)
target_link_libraries(detour_tilecache PUBLIC detour)
```

- [ ] **Step 2: Commit**

```bash
git add bootstrap/templates/recast-CMakeLists.txt
git commit -m "build: add Recast/Detour CMake template"
```

---

### Task 10: UnifiedKernel CMakeLists.txt

**Files:**
- Create: `src/lib/CMakeLists.txt`

- [ ] **Step 1: Create src/lib/CMakeLists.txt**

Note: `src/lib/` does not exist yet. Create it first: `mkdir -p src/lib`

```cmake
# UnifiedKernel — shared static library for all Cimmeria servers.

add_library(unified_kernel STATIC
    # Common
    ../common/console.cpp
    ../common/database.cpp
    ../common/service.cpp

    # Entity system
    ../entity/defs.cpp
    ../entity/entity.cpp
    ../entity/mailbox.cpp
    ../entity/method.cpp
    ../entity/python.cpp
    ../entity/pyutil.cpp
    ../entity/py_client.cpp

    # Logging
    ../log/logger.cpp

    # Mercury networking
    ../mercury/bundle.cpp
    ../mercury/channel.cpp
    ../mercury/condemned_channels.cpp
    ../mercury/encryption_filter.cpp
    ../mercury/packet.cpp
    ../mercury/memory_stream.cpp
    ../mercury/message.cpp
    ../mercury/message_handler.cpp
    ../mercury/nub.cpp
    ../mercury/sgw/entity_message_handler.cpp
    ../mercury/stream.cpp
    ../mercury/transparent_filter.cpp
    ../mercury/unified_connection.cpp

    # Utilities
    ../util/crashdump.cpp

    # Vendored: TinyXML2
    ../xml/tinyxml2.cpp
)

target_include_directories(unified_kernel PUBLIC
    ${CMAKE_SOURCE_DIR}/src
)

target_compile_definitions(unified_kernel PUBLIC
    TIXML_USE_STL
)

# Precompiled header
target_precompile_headers(unified_kernel PRIVATE
    ${CMAKE_SOURCE_DIR}/src/stdafx.hpp
)

# Link dependencies
target_link_libraries(unified_kernel PUBLIC
    Boost::system
    Boost::thread
    Boost::filesystem
    Boost::date_time
    Boost::python
    OpenSSL::Crypto
    Python3::Python
    PostgreSQL::PostgreSQL
    soci_core_static
    soci_postgresql_static
)

# Platform-specific
if(WIN32)
    target_link_libraries(unified_kernel PUBLIC
        dbghelp
        Winmm
    )
elseif(UNIX)
    target_link_libraries(unified_kernel PUBLIC
        Threads::Threads
    )
endif()
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/CMakeLists.txt
git commit -m "build: add UnifiedKernel CMakeLists.txt"
```

---

### Task 11: Server executable CMakeLists.txt files

**Files:**
- Create: `src/server/AuthenticationServer/CMakeLists.txt`
- Create: `src/server/BaseApp/CMakeLists.txt`
- Create: `src/server/CellApp/CMakeLists.txt`
- Create: `src/server/NavBuilder/CMakeLists.txt`

- [ ] **Step 1: Create AuthenticationServer CMakeLists.txt**

Note: These directories do not exist yet. Create them first:
```bash
mkdir -p src/server/AuthenticationServer src/server/BaseApp src/server/CellApp src/server/NavBuilder
```

```cmake
# AuthenticationServer — player login and account management.

add_executable(AuthenticationServer
    ../../authentication/frontend_connection.cpp
    ../../authentication/launcher.cpp
    ../../authentication/logon_connection.cpp
    ../../authentication/logon_queue.cpp
    ../../authentication/service_main.cpp
)

target_precompile_headers(AuthenticationServer REUSE_FROM unified_kernel)

target_link_libraries(AuthenticationServer PRIVATE
    unified_kernel
)
```

- [ ] **Step 2: Create BaseApp CMakeLists.txt**

```cmake
# BaseApp — persistent entity state, player base data, shard management.

add_executable(BaseApp
    # Auth client
    ../../authentication/logon_queue.cpp
    ../../authentication/shard_client.cpp

    # Base service
    ../../baseapp/base_service.cpp
    ../../baseapp/cell_manager.cpp
    ../../baseapp/launcher.cpp
    ../../baseapp/minigame.cpp
    ../../baseapp/minigame_connection.cpp

    # Base entities
    ../../baseapp/entity/base_entity.cpp
    ../../baseapp/entity/base_py_util.cpp
    ../../baseapp/entity/cached_entity.cpp
    ../../baseapp/entity/ticker.cpp

    # Mercury handlers
    ../../baseapp/mercury/cell_handler.cpp
    ../../baseapp/mercury/sgw/client_handler.cpp
    ../../baseapp/mercury/sgw/connect_handler.cpp
    ../../baseapp/mercury/sgw/messages.cpp
    ../../baseapp/mercury/sgw/resource.cpp
)

target_precompile_headers(BaseApp REUSE_FROM unified_kernel)

target_link_libraries(BaseApp PRIVATE
    unified_kernel
)

# BaseApp uses Recast/Detour for navigation
if(TARGET recast AND TARGET detour)
    target_link_libraries(BaseApp PRIVATE recast detour)
endif()
```

- [ ] **Step 3: Create CellApp CMakeLists.txt**

```cmake
# CellApp — spatial entity simulation, world cells, movement, AoI.

add_executable(CellApp
    ../../cellapp/base_client.cpp
    ../../cellapp/cell_service.cpp
    ../../cellapp/cell_launcher.cpp
    ../../cellapp/space.cpp

    # Cell entities
    ../../cellapp/entity/cell_entity.cpp
    ../../cellapp/entity/cell_py_util.cpp
    ../../cellapp/entity/movement.cpp
    ../../cellapp/entity/navigation.cpp
)

target_include_directories(CellApp PRIVATE
    ${CMAKE_SOURCE_DIR}/src
)

target_precompile_headers(CellApp REUSE_FROM unified_kernel)

target_link_libraries(CellApp PRIVATE
    unified_kernel
)

# CellApp uses Recast/Detour for navigation
if(TARGET recast)
    target_link_libraries(CellApp PRIVATE recast detour)
endif()
```

- [ ] **Step 4: Create NavBuilder CMakeLists.txt**

```cmake
# NavBuilder — offline navigation mesh generation tool.

add_executable(NavBuilder
    ../../nav_builder/builder.cpp
    ../../nav_builder/chunk.cpp
    ../../nav_builder/mesh.cpp
    ../../nav_builder/mesh_exporter.cpp
)

target_precompile_headers(NavBuilder REUSE_FROM unified_kernel)

target_link_libraries(NavBuilder PRIVATE
    unified_kernel
)

# NavBuilder requires Recast/Detour
if(TARGET recast AND TARGET detour)
    target_link_libraries(NavBuilder PRIVATE recast detour)
else()
    message(WARNING "Recast/Detour not found. NavBuilder will not link correctly.")
endif()

# SDL for mesh visualization (optional)
find_package(SDL QUIET)
if(SDL_FOUND)
    target_link_libraries(NavBuilder PRIVATE ${SDL_LIBRARIES})
    target_include_directories(NavBuilder PRIVATE ${SDL_INCLUDE_DIRS})
endif()
```

- [ ] **Step 5: Commit**

```bash
git add src/server/AuthenticationServer/CMakeLists.txt \
        src/server/BaseApp/CMakeLists.txt \
        src/server/CellApp/CMakeLists.txt \
        src/server/NavBuilder/CMakeLists.txt
git commit -m "build: add CMakeLists.txt for all server executables"
```

---

### Task 11b: ServerEd CMakeLists.txt (optional Qt target)

**Files:**
- Create: `tools/ServerEd/CMakeLists.txt`

- [ ] **Step 1: Create ServerEd CMakeLists.txt**

ServerEd is a Qt-based admin tool. This target is optional — it only builds if Qt5 is found.

```cmake
# ServerEd — Qt-based server administration tool (optional).

find_package(Qt5 QUIET COMPONENTS Core Gui Widgets Xml Network Sql)
if(NOT Qt5_FOUND)
    message(STATUS "Qt5 not found — skipping ServerEd build.")
    return()
endif()

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

file(GLOB SERVERED_SOURCES *.cpp)
file(GLOB SERVERED_HEADERS *.h)
file(GLOB SERVERED_UI *.ui)
file(GLOB SERVERED_QRC *.qrc)

add_executable(ServerEd
    ${SERVERED_SOURCES}
    ${SERVERED_HEADERS}
    ${SERVERED_UI}
    ${SERVERED_QRC}
)

target_link_libraries(ServerEd PRIVATE
    Qt5::Core
    Qt5::Gui
    Qt5::Widgets
    Qt5::Xml
    Qt5::Network
    Qt5::Sql
)

# Copy to common bin directory
set_target_properties(ServerEd PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
)
```

- [ ] **Step 2: Wire into root CMakeLists.txt**

Add to root CMakeLists.txt after the server targets:
```cmake
# ServerEd (Qt admin tool — optional, only if Qt5 found)
if(EXISTS ${CMAKE_SOURCE_DIR}/tools/ServerEd/CMakeLists.txt)
    add_subdirectory(tools/ServerEd)
endif()
```

- [ ] **Step 3: Commit**

```bash
git add tools/ServerEd/CMakeLists.txt
git commit -m "build: add ServerEd CMakeLists.txt (optional Qt target)"
```

---

## Chunk 3: Makefile + Integration

### Task 12: Makefile

**Files:**
- Create: `Makefile`

- [ ] **Step 1: Create Makefile**

```makefile
# Cimmeria Server Emulator — Cross-Platform Build & Run
# Usage: make all       (full pipeline)
#        make build     (compile only)
#        make start     (launch servers)
#        make stop      (shut down everything)

SHELL := /bin/bash
.DEFAULT_GOAL := help

SCRIPTS := bootstrap/scripts
OS := $(shell uname -s)

# --- Meta targets ---

.PHONY: all help clean

all: deps build db-init runtime start  ## Full pipeline: deps -> build -> db -> start

help:  ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

clean:  ## Remove build directory
	rm -rf build

# --- Dependencies ---

.PHONY: deps

deps:  ## Install platform dependencies (Homebrew or apt)
ifeq ($(OS),Darwin)
	@bash $(SCRIPTS)/deps-macos.sh
else ifeq ($(OS),Linux)
	@bash $(SCRIPTS)/deps-linux.sh
else
	@echo "Unsupported OS for 'make deps'. On Windows, use: pwsh setup.ps1"
endif

# --- Build ---

.PHONY: configure build

configure:  ## Run CMake configure
	@bash $(SCRIPTS)/build.sh configure

build: configure  ## Build all targets
	@bash $(SCRIPTS)/build.sh build

# --- Docker ---

.PHONY: docker-up docker-down

docker-up:  ## Start Docker services (PostgreSQL)
	@bash $(SCRIPTS)/docker.sh up

docker-down:  ## Stop Docker services
	@bash $(SCRIPTS)/docker.sh down

# --- Database ---

.PHONY: db-init db-reset

db-init: docker-up  ## Initialize database (load schemas if needed)
	@bash $(SCRIPTS)/database.sh init

db-reset: docker-up  ## Drop and reload database schemas
	@bash $(SCRIPTS)/database.sh reset

# --- Runtime ---

.PHONY: runtime

runtime:  ## Verify runtime library resolution
	@bash $(SCRIPTS)/runtime.sh

# --- Server ---

.PHONY: start stop

start:  ## Start all game servers
	@bash $(SCRIPTS)/server.sh start

stop:  ## Stop all game servers and Docker
	@bash $(SCRIPTS)/server.sh stop
	@bash $(SCRIPTS)/docker.sh down
```

- [ ] **Step 2: Verify Makefile syntax**

Run: `make help`
Expected: List of targets with descriptions.

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "build: add Makefile for cross-platform developer workflow"
```

---

### Task 13: Update .gitignore for new build artifacts

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Add CMake and Docker entries to .gitignore**

Add these entries (if not already present):
```
# CMake
build/
CMakeUserPresets.json

# Docker
pgdata/

# Server runtime
server/pids/
server/logs/
```

- [ ] **Step 2: Commit**

```bash
git add .gitignore
git commit -m "build: update .gitignore for CMake build dir and Docker volumes"
```

---

### Task 14: End-to-end verification

- [ ] **Step 1: Verify Docker + Database pipeline**

Run:
```bash
make docker-up
make db-init
```
Expected: PostgreSQL starts, schemas load, tables verified.

- [ ] **Step 2: Verify CMake configure (will partially succeed)**

Run: `make configure`
Expected: CMake runs, finds Boost/OpenSSL/Python/PostgreSQL. May warn about missing Recast (external/ not populated). SOCI FetchContent downloads. Configuration completes or fails with known source-code compatibility issues (expected — code migration is a separate effort).

- [ ] **Step 3: Verify make help**

Run: `make help`
Expected: All targets listed with descriptions.

- [ ] **Step 4: Clean up**

Run: `make stop` (if anything was started)

- [ ] **Step 5: Final commit (if any remaining changes)**

```bash
git status
# Add only specific files that were modified during verification
git commit -m "build: cross-platform CMake + Docker + shell scripts infrastructure"
```

---

## Post-Implementation Notes

### Known hazards:

- **`src/openssl/` vendored headers** — The codebase has old OpenSSL 0.9.8i headers in `src/openssl/`. Since `${CMAKE_SOURCE_DIR}/src` is on the include path (for UnifiedKernel), `#include <openssl/ssl.h>` may resolve to these vendored headers instead of the system OpenSSL 3.x headers. During the OpenSSL migration, either remove `src/openssl/` or exclude it from include paths.

### What comes next (separate efforts):

1. **C++ code migration** — Update source files for modern library APIs:
   - Boost 1.87: `boost::shared_ptr` → `std::shared_ptr`, Asio executor model changes
   - OpenSSL 3.x: `EVP_*` API migration, remove deprecated functions
   - Python 3.12: Update Boost.Python bindings
   - Platform guards: Add `#elif __APPLE__` / `#elif __linux__` where `#ifdef _WIN32` exists
   - Replace `dbghelp.h` and `Winmm.lib` with cross-platform equivalents

2. **SOCI version migration** — SOCI 4.x has API changes from 3.2.1 (session management, type traits)

3. **Windows PowerShell simplification** — Update `Build-CimmeriaSolution` to call CMake instead of MSBuild

4. **CI/CD** — GitHub Actions workflow for all three platforms
