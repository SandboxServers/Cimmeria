# Bootstrap - Automated Server Setup & Launch

This directory contains everything needed to go from a fresh git clone to a
running Cimmeria server in a single command.

## Quick Start

```powershell
# From the project root - full pipeline:
pwsh setup.ps1
```

This will:

1. Check prerequisites (Rust toolchain, Node.js)
2. Download PostgreSQL 17.9 server binaries (~50 MB, Windows only)
3. Build the Rust server (`cargo build --workspace`)
4. Build the Tauri admin app (optional, requires Node.js)
5. Initialize PostgreSQL on port 5433 with game schemas
6. Launch the cimmeria-server binary

When it finishes, connect with the game client using **test** / **test**.

## PowerShell Module

The bootstrap logic lives in the **CimmeriaBootstrap** PowerShell module
(`CimmeriaBootstrap/`). See [`CimmeriaBootstrap/README.md`](CimmeriaBootstrap/README.md)
for full documentation of all exported functions.

```powershell
# Import for individual function use:
Import-Module ./bootstrap/CimmeriaBootstrap

# Examples:
Build-CimmeriaServer -Configuration Release  # Build release
Build-CimmeriaApp -Configuration Release     # Build Tauri admin app
Initialize-CimmeriaDatabase -Force           # Wipe and reload database
Start-CimmeriaServer                         # Launch server
Stop-CimmeriaServer                          # Shut down everything
Update-CimmeriaClient                        # Patch game client
```

## Prerequisites

- **PowerShell 7.0+** (`pwsh`) - ships with Windows 11, or install from https://github.com/PowerShell/PowerShell
- **Rust toolchain** - install from https://rustup.rs (stable channel)
- **Node.js 18+** - install from https://nodejs.org (only needed for Tauri admin app; skip with `-SkipApp`)
- **~1 GB free disk space** (Cargo target directory + PostgreSQL)
- **Internet connection** (first run only; Cargo crate cache and PG download are reused)

## Script Options

```powershell
# Server only (no Tauri app, no Node.js required):
pwsh setup.ps1 -SkipApp

# Skip downloads (re-run after restart, or if PG is already cached):
pwsh setup.ps1 -SkipDownload

# Download + build only, don't launch server:
pwsh setup.ps1 -NoLaunch

# Skip all builds:
pwsh setup.ps1 -SkipBuild

# Build Release configuration:
pwsh setup.ps1 -Configuration Release

# Wipe and reload the database:
pwsh setup.ps1 -SkipBuild -ForceDatabase
```

## Cross-Platform Support

The bootstrap runs on **Windows**, **Linux**, and **macOS** via PowerShell 7.

| Platform | PostgreSQL | Notes |
|----------|-----------|-------|
| Windows | Auto-downloaded binary distribution | Managed instance in `server/pgdata/` |
| Linux | System package (`apt install postgresql-17`) | Must be running before bootstrap |
| macOS | Homebrew (`brew install postgresql@17`) | Must be running before bootstrap |

## Legacy Script

The original `setup-dependencies.ps1` in the project root is now a compatibility
shim that calls `setup.ps1 -NoLaunch -SkipApp`.

## Connecting a Game Client

1. Install the Stargate Worlds QA client
2. Run: `Update-CimmeriaClient` (or manually edit Login.lua to point at `http://localhost:8081/SGWLogin/UserAuth`)
3. Launch the game client
4. Login with: **test** / **test**

## Troubleshooting

**"Missing required tools: cargo"**
Install Rust from https://rustup.rs, restart your shell, and re-run.

**"Missing required tools: node"**
Install Node.js from https://nodejs.org, or use `-SkipApp` to skip the Tauri app build.

**"PostgreSQL not found"**
On Linux: `sudo apt install postgresql-17`
On macOS: `brew install postgresql@17`
On Windows: re-run `Install-CimmeriaDependencies`

**"cargo build failed"**
Check `cargo build --workspace` output. Common issues: missing system libs on Linux
(`libssl-dev`, `pkg-config`).

**Want to start completely fresh?**
Delete `target/`, `external/`, and `server/`, then re-run the script.
