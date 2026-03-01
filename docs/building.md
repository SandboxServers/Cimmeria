# Building the Server

How to get the Cimmeria server emulator compiled and running on your machine.

## The Easy Way (Recommended)

One command does everything — downloads dependencies, patches them, builds them, compiles the server, sets up the database, and starts the servers:

```powershell
pwsh setup.ps1
```

That's it. When it finishes, you'll see a banner with connection details and test credentials.

## What Gets Built

The setup process builds 6 projects:

| Project | What It Is |
|---------|-----------|
| **Recast** | Navigation mesh library (used by NavBuilder) |
| **UnifiedKernel** | Shared networking and protocol library |
| **NavBuilder** | Offline tool that generates navigation meshes |
| **AuthenticationServer** | Handles login and shard selection |
| **BaseApp** | Manages persistent player data and connections |
| **CellApp** | Simulates the game world (movement, combat, etc.) |
| **ServerEd** | Qt-based admin/editor tool (built separately via qmake) |

## What You Need Installed

- **Visual Studio 2026** (Community edition is fine) with the "Desktop development with C++" workload
- **PowerShell 7+** (comes with Windows, or install from Microsoft)
- **Git** (for cloning the repository)

Everything else (Boost, OpenSSL, PostgreSQL, Python, SOCI, SDL, Qt) is downloaded automatically.

## Setup Options

The setup script accepts several flags:

| Flag | What It Does |
|------|-------------|
| `-SkipDownload` | Skip dependency downloads (use if you already have them) |
| `-SkipBuild` | Skip building dependencies (use if already built) |
| `-NoLaunch` | Set up everything but don't start the servers |
| `-Configuration Release` | Build Release instead of Debug |

## Building Without the Script

If you prefer doing things manually or need to rebuild just one piece:

1. **Download dependencies**: The script puts them in `external/`
2. **Build Boost**: Run `bootstrap\init-boost.bat` then `bootstrap\build-boost.bat`
3. **Build OpenSSL**: Run `bootstrap\build-openssl.bat` (needs Perl on your PATH)
4. **Build SOCI**: Run `bootstrap\build-soci.bat`
5. **Open the solution**: Open `W-NG.sln` in Visual Studio
6. **Select Debug|x64** from the toolbar
7. **Build Solution** (Ctrl+Shift+B)

The executables end up in `bin64/debug/` (or `bin64/release/` for Release builds).

## Build Configurations

| Configuration | When to Use |
|--------------|-------------|
| **Debug** | Day-to-day development (recommended) |
| **Release** | Running a server for others to connect to |
| **UnoptRelease** | Debugging issues that only appear in Release |
| **MinSizeRel** | Smallest possible executable size |

## Troubleshooting

**"Boost build failed"** — Make sure no other Python is on your PATH that might conflict. The build needs the embedded Python 3.4 headers, not your system Python.

**"OpenSSL build failed"** — You need Perl. Either install Strawberry Perl or make sure Git for Windows is installed (it bundles Perl).

**"Can't find Visual Studio"** — The scripts use `vswhere.exe` to auto-detect VS. Make sure you have the C++ workload installed.

**Build succeeds but servers won't start** — Run `setup.ps1` to ensure the database is initialized and DLLs are staged in the right places.

For the detailed technical build guide with exact library paths and patch details, see [technical/building.md](technical/building.md).
