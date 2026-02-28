# Bootstrap - Automated Dependency Setup & Server Launch

This directory contains everything needed to go from a fresh git clone to a
running Cimmeria server in a single command.

## Quick Start

```powershell
# From the project root - full pipeline:
pwsh setup.ps1
```

This will:

1. Detect (or optionally install) Visual Studio
2. Download all 7 dependency archives (~200 MB)
3. Extract, arrange, and patch for modern MSVC
4. Build Boost, OpenSSL, and SOCI libraries
5. MSBuild the W-NG.sln solution
6. Initialize PostgreSQL on port 5433 with game schemas
7. Stage runtime DLLs into bin64/
8. Launch Auth, Base, and Cell servers

When it finishes, connect with the game client using **test** / **test**.

## PowerShell Module

The bootstrap logic lives in the **CimmeriaBootstrap** PowerShell module
(`CimmeriaBootstrap/`). See [`CimmeriaBootstrap/README.md`](CimmeriaBootstrap/README.md)
for full documentation of all exported functions.

```powershell
# Import for individual function use:
Import-Module ./bootstrap/CimmeriaBootstrap

# Examples:
Install-CimmeriaDependencies -SkipDownload    # Re-apply patches only
Build-CimmeriaSolution -Configuration Release  # Build release
Start-CimmeriaServer                           # Launch servers
Stop-CimmeriaServer                            # Shut down everything
Patch-CimmeriaClient                           # Patch game client
```

## Prerequisites

- **PowerShell 7.0+** (`pwsh`) - ships with Windows 11, or install from https://github.com/PowerShell/PowerShell
- **Windows 10/11** or Windows Server
- **~2 GB free disk space** (downloads + extracted sources + built libraries)
- **Internet connection** (first run only; downloads are cached)

Visual Studio and Perl are detected automatically. If VS isn't installed, pass
`-InstallVS` to auto-install VS Community.

## Script Options

```powershell
# Auto-install VS Community if not found:
pwsh setup.ps1 -InstallVS

# Skip downloads (re-run after restart, or if archives are already cached):
pwsh setup.ps1 -SkipDownload

# Download + extract + patch + build only, don't launch servers:
pwsh setup.ps1 -NoLaunch

# Skip all builds:
pwsh setup.ps1 -SkipBuild

# Build Release configuration:
pwsh setup.ps1 -Configuration Release
```

## Legacy Script

The original `setup-dependencies.ps1` in the project root is now a compatibility
shim that calls `setup.ps1 -NoLaunch`. It preserves the original behavior
(download + extract + patch + build libraries) for existing workflows.

## What Gets Downloaded

| Dependency | Version | Size | Type |
|---|---|---|---|
| Boost | 1.55.0 | ~85 MB | Source, built by b2 |
| OpenSSL | 1.0.1e | ~5 MB | Source, built by nmake |
| PostgreSQL | 9.2.24 | ~50 MB | Pre-built binaries (headers + libpq + server) |
| Python | 3.4.1 | ~20 MB | MSI (headers + import libs + python34.dll) |
| SOCI | 3.2.1 | ~1 MB | Source, built by cl/lib |
| SDL | 1.2.15 | ~2 MB | Pre-built (headers + libs) |
| Recast/Detour | main | ~2 MB | Source (built by solution vcxproj) |

Archives are cached in `external/_downloads/` and reused on subsequent runs.

## Patches

The `patches/` directory contains fixed versions of files for modern MSVC compatibility.
See [patches/README.md](patches/README.md) for details.

## Build Scripts

Standalone `.bat` scripts for manual use:

| Script | What it builds |
|---|---|
| `init-boost.bat` | Bootstraps Boost.Build (b2) |
| `build-boost.bat` | Builds Boost libraries |
| `build-openssl.bat` | Builds OpenSSL debug + release static libraries |
| `build-soci.bat` | Builds SOCI core + PostgreSQL backend libraries |

## Connecting a Game Client

1. Install the Stargate Worlds QA client
2. Run: `Patch-CimmeriaClient` (or manually edit Login.lua to point at `http://localhost:8080/SGWLogin/UserAuth`)
3. Launch the game client
4. Login with: **test** / **test**

## Troubleshooting

**"Visual Studio with C++ tools not found"**
Install VS with the "Desktop development with C++" workload, or re-run with `-InstallVS`.

**"Perl not found"**
Install [Git for Windows](https://git-scm.com/download/win) (bundles Perl) or
[Strawberry Perl](https://strawberryperl.com/).

**Boost build fails**
Check that `external/boost/user-config.jam` exists and points to a valid `cl.exe`.

**Want to start completely fresh?**
Delete `external/`, `lib64/`, and `server/`, then re-run the script.
