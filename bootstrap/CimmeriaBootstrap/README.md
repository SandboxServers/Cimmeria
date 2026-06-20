# CimmeriaBootstrap PowerShell Module

PowerShell 7.0+ module that automates the full Cimmeria server lifecycle: from a
fresh git clone to a running server accepting player connections.

## Quick Start

```powershell
# Full pipeline: prereqs -> build -> database -> launch
pwsh setup.ps1

# Or import the module and call functions individually:
Import-Module ./bootstrap/CimmeriaBootstrap
```

## Pipeline Steps

| Step | Function | What it does |
|------|----------|--------------|
| 1 | `Assert-CimmeriaPrerequisites` | Check for Rust toolchain, optionally Node.js |
| 2 | `Install-CimmeriaDependencies` | Download PostgreSQL (Windows) or verify system PG |
| 3 | `Build-CimmeriaServer` | `cargo build --workspace` excluding the GUI apps (see below) |
| 4 | `Build-CimmeriaApp` | `npm install` + `cargo tauri build` (optional, `-WithAdmin`) |
| 5 | `Build-CimmeriaLauncher` | Build the `sgw-launcher` Tauri app (optional, `-WithLauncher`) |
| 6 | `Sync-CimmeriaGameData` | Copy client PAK files into `data/cache/` |
| 7 | `Initialize-CimmeriaDatabase` | PostgreSQL init + schema load (port 5433) |
| 8 | `Start-CimmeriaServer` | Launch cimmeria-server binary |

Step 3 (`Build-CimmeriaServer`) runs:

```powershell
cargo build --workspace `
  --exclude cimmeria-app --exclude sgw-launcher `
  --exclude cimmeria-content-editor --exclude cimmeria-scene-editor
```

The four GUI apps (Tauri admin panel, game launcher, and the two Tauri editors)
are excluded from the default build and only built on demand via `-WithAdmin` /
`-WithLauncher` or their dedicated functions.

`Invoke-CimmeriaBootstrap` runs all steps in order with fail-fast semantics.

## Exported Functions

### Build

- **`Build-CimmeriaServer`** - Build the Rust server workspace via Cargo (excludes the GUI apps)
- **`Build-CimmeriaApp`** - Build the Tauri admin app (SolidJS frontend + Rust backend)
- **`Build-CimmeriaLauncher`** - Build the `sgw-launcher` Tauri game launcher

### Setup & Database

- **`Install-CimmeriaDependencies`** - Download/verify PostgreSQL
- **`Sync-CimmeriaGameData`** - Copy cooked client PAK files into `data/cache/`
- **`Initialize-CimmeriaDatabase`** - Set up PostgreSQL and load schemas

### Server Lifecycle

- **`Start-CimmeriaServer`** - Launch the cimmeria-server binary
- **`Stop-CimmeriaServer`** - Stop the server and (Windows) PostgreSQL

### Client & Orchestration

- **`Update-CimmeriaClient`** - Patch game client Login.lua to connect locally
- **`Install-CimmeriaReToolchain`** - Install the reverse-engineering toolchain (opt-in, Windows)
- **`Invoke-CimmeriaBootstrap`** - Full pipeline orchestrator

## Common Options

```powershell
# Server only (no Tauri app, no Node.js needed):
pwsh setup.ps1 -SkipApp

# Skip downloads (use cached archives):
pwsh setup.ps1 -SkipDownload

# Build only, don't launch servers:
pwsh setup.ps1 -NoLaunch

# Build Release instead of Debug:
pwsh setup.ps1 -Configuration Release
```

## Individual Function Usage

```powershell
Import-Module ./bootstrap/CimmeriaBootstrap

# Build release config:
Build-CimmeriaServer -Configuration Release

# Manage server lifecycle:
Start-CimmeriaServer
Stop-CimmeriaServer

# Wipe and reload database:
Initialize-CimmeriaDatabase -Force

# Patch game client:
Update-CimmeriaClient
```

## Cross-Platform Support

| Platform | PostgreSQL | Notes |
|----------|-----------|-------|
| Windows | Auto-downloaded binary | Managed instance in `server/pgdata/` |
| Linux | System package | Must be running before bootstrap |
| macOS | Homebrew | Must be running before bootstrap |

## Connecting a Game Client

1. Install the Stargate Worlds QA client
2. Run `Update-CimmeriaClient` (or manually edit Login.lua)
3. Launch the game client
4. Login with: **test** / **test**

The auth endpoint is `http://localhost:8081/SGWLogin/UserAuth`.

## Database Details

- **Port**: 5433 (non-default to avoid conflicts with system PostgreSQL)
- **Database**: `sgw`
- **Role**: `w-testing` / `w-testing`
- **Test account**: `test` / `test` (SHA1 hashed in the `account` table)
- **Schemas**: `db/database.sql` (setup), `db/resources/` (resource types), `db/sgw/` (game schema)

The `w-testing` credentials are intentional for local development. The PostgreSQL
instance uses trust authentication, listens only on localhost, and runs on a
non-standard port.

## Module Structure

```
CimmeriaBootstrap/
├── CimmeriaBootstrap.psd1          # Module manifest
├── CimmeriaBootstrap.psm1          # Module loader
├── README.md                       # This file
├── Data/
│   └── Dependencies.psd1           # Dependency registry (PostgreSQL)
├── Public/                         # Exported functions (11 files)
│   ├── Install-CimmeriaDependencies.ps1
│   ├── Build-CimmeriaServer.ps1
│   ├── Build-CimmeriaApp.ps1
│   ├── Build-CimmeriaLauncher.ps1
│   ├── Initialize-CimmeriaDatabase.ps1
│   ├── Sync-CimmeriaGameData.ps1
│   ├── Start-CimmeriaServer.ps1
│   ├── Stop-CimmeriaServer.ps1
│   ├── Update-CimmeriaClient.ps1
│   ├── Install-CimmeriaReToolchain.ps1
│   └── Invoke-CimmeriaBootstrap.ps1
└── Private/                        # Internal helpers (9 files)
    ├── Assert-CimmeriaPrerequisites.ps1
    ├── Write-Step.ps1
    ├── Write-Status.ps1
    ├── Format-FileSize.ps1
    ├── Get-DownloadFile.ps1
    ├── Expand-DependencyArchive.ps1
    ├── Find-PostgreSQL.ps1
    ├── Get-ProjectPaths.ps1
    └── Wait-ForPort.ps1
```
