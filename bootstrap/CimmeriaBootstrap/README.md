# CimmeriaBootstrap PowerShell Module

PowerShell 7.0+ module that automates the full Cimmeria server lifecycle: from a
fresh git clone to a running server accepting player connections.

## Quick Start

```powershell
# Full pipeline: download -> build -> database -> launch
pwsh setup.ps1

# Or import the module and call functions individually:
Import-Module ./bootstrap/CimmeriaBootstrap
```

## Pipeline Steps

| Step | Function | What it does |
|------|----------|--------------|
| 1 | `Install-CimmeriaDependencies` | Download 7 archives, extract, apply MSVC patches |
| 2 | `Build-CimmeriaLibraries` | Build Boost, OpenSSL, SOCI from source |
| 3 | `Build-CimmeriaSolution` | MSBuild W-NG.sln (all 6 projects) |
| 4 | `Initialize-CimmeriaDatabase` | PostgreSQL init + schema load (port 5433) |
| 5 | `Initialize-CimmeriaRuntime` | Stage DLLs into bin64/ |
| 6 | `Start-CimmeriaServer` | Launch Auth, Base, Cell servers |

`Invoke-CimmeriaBootstrap` runs all steps in order with fail-fast semantics.

## Exported Functions

### Setup & Build

- **`Install-CimmeriaDependencies`** - Download, extract, and patch all external dependencies
- **`Build-CimmeriaLibraries`** - Build Boost, OpenSSL, and SOCI libraries
- **`Build-CimmeriaSolution`** - Build the Visual Studio solution via MSBuild

### Database & Runtime

- **`Initialize-CimmeriaDatabase`** - Set up PostgreSQL and load schemas
- **`Initialize-CimmeriaRuntime`** - Stage runtime DLLs alongside executables

### Server Lifecycle

- **`Start-CimmeriaServer`** - Launch all three game servers
- **`Stop-CimmeriaServer`** - Gracefully shut down servers and PostgreSQL

### Client & Orchestration

- **`Update-CimmeriaClient`** - Patch game client Login.lua to connect locally
- **`Invoke-CimmeriaBootstrap`** - Full pipeline orchestrator

## Common Options

```powershell
# Skip downloads (use cached archives):
pwsh setup.ps1 -SkipDownload

# Build only, don't launch servers:
pwsh setup.ps1 -NoLaunch

# Build Release instead of Debug:
pwsh setup.ps1 -Configuration Release

# Auto-install VS Community if not found:
pwsh setup.ps1 -InstallVS
```

## Individual Function Usage

```powershell
Import-Module ./bootstrap/CimmeriaBootstrap

# Re-apply patches only (archives already downloaded):
Install-CimmeriaDependencies -SkipDownload

# Build release config:
Build-CimmeriaSolution -Configuration Release

# Manage server lifecycle:
Start-CimmeriaServer
Stop-CimmeriaServer

# Patch game client:
Update-CimmeriaClient
```

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
- **Schemas**: `db/resources.sql` (resource types) then `db/sgw.sql` (game schema)

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
│   └── Dependencies.psd1           # Dependency registry (versions, URLs)
├── Public/                         # Exported functions (9 files)
│   ├── Install-CimmeriaDependencies.ps1
│   ├── Build-CimmeriaLibraries.ps1
│   ├── Build-CimmeriaSolution.ps1
│   ├── Initialize-CimmeriaDatabase.ps1
│   ├── Initialize-CimmeriaRuntime.ps1
│   ├── Start-CimmeriaServer.ps1
│   ├── Stop-CimmeriaServer.ps1
│   ├── Update-CimmeriaClient.ps1
│   └── Invoke-CimmeriaBootstrap.ps1
└── Private/                        # Internal helpers (11 files)
    ├── Write-Step.ps1
    ├── Write-Status.ps1
    ├── Format-FileSize.ps1
    ├── Get-DownloadFile.ps1
    ├── Expand-DependencyArchive.ps1
    ├── Find-VisualStudio.ps1
    ├── Find-Perl.ps1
    ├── Find-PostgreSQL.ps1
    ├── Invoke-BatchScript.ps1
    ├── Get-ProjectPaths.ps1
    └── Wait-ForPort.ps1
```
