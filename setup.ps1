<#
.SYNOPSIS
    Cimmeria full bootstrap: fresh clone to running server.

.DESCRIPTION
    Orchestrates the complete Cimmeria development environment setup:

      Step 1: Prerequisites    - Rust toolchain, MSVC build tools, optionally Node.js
      Step 2: Dependencies     - PostgreSQL binaries (or Docker container), 7-Zip sidecar
      Step 3: Build Server     - cargo build for the game server workspace
      Step 4: Build Admin      - [optional] Tauri admin panel (Node.js + Rust)
      Step 5: Build Launcher   - [optional] SGW game launcher (Tauri)
      Step 6: Database         - PostgreSQL init, schema load, seed data
      Step 7: Launch           - Start the cimmeria-server binary

    Every step is idempotent. Safe to re-run -- completed work is detected and skipped.
    On failure the pipeline aborts with a clear error message.

    By default only the game server is built. The admin panel and game launcher are
    opt-in via -WithAdmin and -WithLauncher. PostgreSQL runs as a local managed
    instance unless -UseDocker is specified.

    ----------------------------------------------
    QUICK START
    ----------------------------------------------

      pwsh setup.ps1                          # Build server, init DB, launch
      pwsh setup.ps1 -UseDocker               # Same but PostgreSQL runs in Docker
      pwsh setup.ps1 -WithAdmin -WithLauncher # Build everything

    ----------------------------------------------
    BUILD FLAGS
    ----------------------------------------------

      -SkipDownload       Skip downloading dependency archives (use cached files).
      -SkipBuild          Skip all build steps (useful for DB-only operations).
      -WithAdmin          Also build the Tauri admin panel. Requires Node.js 22+.
      -WithLauncher       Also build the SGW game launcher.
      -Configuration      "Debug" (default) or "Release".
      -NoLaunch           Stop after setup -- do not launch the server.

    ----------------------------------------------
    DATABASE FLAGS
    ----------------------------------------------

      -ForceDatabase      Drop and recreate the 'sgw' database, then reload all
                          schemas and seed data. PostgreSQL itself stays intact.

      -ResetDatabase      Nuclear option. Stops PostgreSQL, deletes the entire
                          pgdata directory (or removes the Docker container),
                          and re-initializes everything from scratch. Use this
                          when pgdata is corrupted or the PG version changed.

      -UseDocker          Use a Docker container (postgres:17) instead of the
                          local PostgreSQL binaries. Requires Docker Desktop or
                          Docker Engine. The container is named 'cimmeria-postgres'
                          and persists across runs (start/stop, not recreated).

    ----------------------------------------------
    DATA SAFETY
    ----------------------------------------------

    The database step checks for existing data before loading:
      - Schema check:  looks for the 'account' table and 'resources' schema.
      - Seed check:    looks for the test account, shard record, and resources data.
      - If both exist, skips loading entirely (no duplicate insert errors).
      - If schema exists but seed data is incomplete, attempts a reload, suppressing
        "already exists" errors for objects that were already created.
      - pgdata version is validated against the installed PostgreSQL major version.
        A mismatch aborts with a message to use -ResetDatabase.

    ----------------------------------------------
    COMMON RECIPES
    ----------------------------------------------

      # First-time setup (everything default)
      pwsh setup.ps1

      # Rebuild server only, don't touch DB or launch
      pwsh setup.ps1 -NoLaunch

      # Wipe and reload the database (keep builds)
      pwsh setup.ps1 -SkipBuild -ForceDatabase -NoLaunch

      # Full nuclear reset of PostgreSQL
      pwsh setup.ps1 -SkipBuild -ResetDatabase -NoLaunch

      # Switch to Docker PostgreSQL
      pwsh setup.ps1 -UseDocker

      # Release build with all optional components
      pwsh setup.ps1 -WithAdmin -WithLauncher -Configuration Release

      # CI/headless: build and init DB, don't launch
      pwsh setup.ps1 -WithAdmin -WithLauncher -NoLaunch -Configuration Release

    ----------------------------------------------
    STANDALONE COMMANDS
    ----------------------------------------------

    Each pipeline step can also be run independently after importing the module:

      Import-Module ./bootstrap/CimmeriaBootstrap

      Build-CimmeriaServer                              # Just the server
      Build-CimmeriaServer -WithAdmin -WithLauncher     # Server + optional apps
      Build-CimmeriaApp                                 # Tauri admin panel only
      Build-CimmeriaLauncher                            # SGW launcher only
      Initialize-CimmeriaDatabase                       # DB init (idempotent)
      Initialize-CimmeriaDatabase -UseDocker            # DB via Docker
      Initialize-CimmeriaDatabase -ResetDatabase        # Nuke and rebuild
      Start-CimmeriaServer                              # Launch the server
      Stop-CimmeriaServer                               # Stop server + PG
      Update-CimmeriaClient                             # Patch game client files

.PARAMETER SkipDownload
    Skip downloading dependency archives. Uses already-cached files in external/_downloads/.

.PARAMETER SkipBuild
    Skip all build steps (server, admin panel, launcher). Useful when you only want
    to initialize or reset the database.

.PARAMETER WithAdmin
    Build the Tauri admin panel (cimmeria-app). This is a desktop application for
    server administration with entity inspection, content browsing, and a visual
    chain editor. Requires Node.js 22+ and npm.

.PARAMETER WithLauncher
    Build the SGW game launcher (sgw-launcher). This is a Tauri desktop app that
    downloads, patches, and launches the Stargate Worlds game client. Does not
    require Node.js (uses plain HTML/CSS/JS for the UI).

.PARAMETER NoLaunch
    Complete all setup steps but do not start the server at the end.

.PARAMETER Configuration
    Build configuration passed to cargo. "Debug" (default) for fast iteration with
    debug symbols, or "Release" for optimized binaries.

.PARAMETER ForceDatabase
    Drop and recreate the 'sgw' database before loading schemas. The PostgreSQL
    data directory (pgdata) is preserved -- only the database contents are wiped.
    Active connections are terminated before the drop.

.PARAMETER ResetDatabase
    Delete the PostgreSQL data directory entirely and re-initialize from scratch.
    On local mode: stops pg_ctl, removes server/pgdata/, runs initdb again.
    On Docker mode: removes the cimmeria-postgres container and creates a new one.
    This is the fix for pgdata version mismatches or corrupted data directories.

.PARAMETER UseDocker
    Run PostgreSQL in a Docker container instead of local binaries. Creates a
    container named 'cimmeria-postgres' using the postgres:17 image, mapped to
    the configured port (default 5433). The container persists across setup runs.
    Requires Docker Desktop (Windows/macOS) or Docker Engine (Linux).

.EXAMPLE
    pwsh setup.ps1

    Default bootstrap: install prerequisites, download PostgreSQL, build the server,
    initialize the database with schemas and seed data, then launch.

.EXAMPLE
    pwsh setup.ps1 -WithAdmin -WithLauncher -Configuration Release

    Full production build: server + admin panel + game launcher, all optimized.

.EXAMPLE
    pwsh setup.ps1 -UseDocker

    Use Docker for PostgreSQL instead of downloading and managing local binaries.

.EXAMPLE
    pwsh setup.ps1 -UseDocker -ResetDatabase -SkipBuild -NoLaunch

    Destroy the Docker PostgreSQL container, recreate it, and reload all schemas.

.EXAMPLE
    pwsh setup.ps1 -ForceDatabase -SkipBuild -NoLaunch

    Wipe and reload the database only. No builds, no server launch.

.EXAMPLE
    pwsh setup.ps1 -SkipDownload -SkipBuild -NoLaunch

    Just run the database initialization step using cached dependencies.

.LINK
    https://github.com/SandboxServers/Cimmeria
#>
#Requires -Version 7.0
param(
    [switch]$SkipDownload,
    [switch]$SkipBuild,
    [switch]$WithAdmin,
    [switch]$WithLauncher,
    [switch]$NoLaunch,
    [ValidateSet("Debug","Release")][string]$Configuration = "Debug",
    [switch]$ForceDatabase,
    [switch]$ResetDatabase,
    [switch]$UseDocker
)

Import-Module (Join-Path $PSScriptRoot "bootstrap/CimmeriaBootstrap") -Force
Invoke-CimmeriaBootstrap @PSBoundParameters
