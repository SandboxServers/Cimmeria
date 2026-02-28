# Bootstrap - Automated Dependency Setup

This directory contains everything needed to go from a fresh Windows machine to a
buildable Cimmeria solution in a single script run.

## Quick Start

```powershell
# From the project root:
pwsh setup-dependencies.ps1
```

That's it. The script will:

1. Detect (or optionally install) Visual Studio
2. Find Perl (for the OpenSSL build)
3. Download all 7 dependency archives (~200 MB)
4. Extract and arrange them into `external/`
5. Apply compatibility patches for modern MSVC
6. Build Boost, OpenSSL, and SOCI libraries
7. Print a verification checklist

When it finishes, open `W-NG.sln` in Visual Studio and build.

## Prerequisites

- **PowerShell 7.0+** (`pwsh`) — ships with Windows 11, or install from https://github.com/PowerShell/PowerShell
- **Windows 10/11** or Windows Server
- **~2 GB free disk space** (downloads + extracted sources + built libraries)
- **Internet connection** (first run only; downloads are cached)

Visual Studio and Perl are detected automatically. If VS isn't installed, the
script tells you what to do (or pass `-InstallVS` to auto-install VS Community).

## Script Options

```powershell
# Auto-install VS Community if not found (requires restart after):
pwsh setup-dependencies.ps1 -InstallVS

# Skip downloads (re-run after restart, or if archives are already cached):
pwsh setup-dependencies.ps1 -SkipDownload

# Download + extract + patch only, don't build libraries:
pwsh setup-dependencies.ps1 -SkipBuild

# Build for x86 instead of x64:
pwsh setup-dependencies.ps1 -BuildArch x86
```

## What Gets Downloaded

| Dependency | Version | Size | Type |
|---|---|---|---|
| Boost | 1.55.0 | ~85 MB | Source, built by b2 |
| OpenSSL | 1.0.1e | ~5 MB | Source, built by nmake |
| PostgreSQL | 9.2.24 | ~50 MB | Pre-built binaries (headers + libpq) |
| Python | 3.4.1 | ~20 MB | MSI (headers + import libs extracted) |
| SOCI | 3.2.1 | ~1 MB | Source, built by cl/lib |
| SDL | 1.2.15 | ~2 MB | Pre-built (headers + libs) |
| Recast/Detour | main | ~2 MB | Source (built by solution vcxproj) |

Archives are cached in `external/_downloads/` and reused on subsequent runs.

## Patches

The dependencies date from 2012-2013 and don't compile cleanly with modern MSVC
(VS2015+). The `patches/` directory contains fixed versions of 4 files that are
copied over the originals after extraction:

| Patch | Fixes |
|---|---|
| `boost_auto_link.hpp` | Adds vc140/vc145 toolset names so auto-linking finds the right .lib files |
| `soci_platform.h` | Guards `snprintf` and `strtoll` macros that conflict with VS2013+ native CRT |
| `openssl_e_os.h` | Skips `stdin`/`stdout`/`stderr` redefinition on VS2015+ UCRT |
| `openssl_e_padlock.c` | Guards x86 inline assembly so it compiles on x64 targets |

See [patches/README.md](patches/README.md) for detailed explanations of each patch.

## Templates

`templates/user-config.jam.template` is used to generate `external/boost/user-config.jam`
at setup time. The script detects the installed MSVC `cl.exe` path and toolset version,
then fills in the placeholders:

- `{{MSVC_TOOLSET_VERSION}}` — e.g., `14.5` for VS2026
- `{{MSVC_CL_PATH}}` — full path to `cl.exe`

## Re-running / Idempotency

The script is safe to run multiple times. Every step checks whether its work is
already done:

- Downloads: skipped if the archive already exists in `_downloads/`
- Extraction: skipped if the target directory already has content
- Patches: compared by SHA256 hash, skipped if already applied
- Builds: skipped if output .lib files already exist

If a VS install was triggered with `-InstallVS`, the script exits and asks you to
restart PowerShell (PATH won't update in the current session). Just re-run the
script after restarting — it picks up where it left off.

## Build Scripts

The automated builds call these standalone `.bat` scripts in this directory.
They can also be run manually (double-click or from a command prompt):

| Script | What it builds |
|---|---|
| `init-boost.bat` | Bootstraps Boost.Build (b2) |
| `build-boost.bat` | Builds Boost libraries (date_time, math, python, system, thread) |
| `build-openssl.bat` | Builds OpenSSL debug + release static libraries |
| `build-soci.bat` | Builds SOCI core + PostgreSQL backend libraries |

Each script uses `%~dp0..` to find the project root, so they work regardless of
your current directory. OpenSSL and SOCI scripts find Visual Studio via
`vswhere.exe` and set up their own build environment.

## Troubleshooting

**"Visual Studio with C++ tools not found"**
Install VS with the "Desktop development with C++" workload, or re-run with
`-InstallVS`.

**"Perl not found"**
Install [Git for Windows](https://git-scm.com/download/win) (bundles Perl) or
[Strawberry Perl](https://strawberryperl.com/). The OpenSSL build needs Perl for
its `Configure` script.

**Boost build fails**
Check that `external/boost/user-config.jam` exists and points to a valid `cl.exe`.
The script generates this automatically, but you can edit it manually if needed.

**OpenSSL build fails with "link.exe" errors**
The build script creates a safe Perl wrapper to avoid Git's `usr/bin/link.exe`
shadowing MSVC's `link.exe`. If you still hit this, try installing Strawberry Perl
(which doesn't have this conflict).

**Want to start completely fresh?**
Delete `external/` and `lib64/`, then re-run the script. The cached archives in
`external/_downloads/` will be recreated. To also re-download, delete `_downloads/`
too (or don't pass `-SkipDownload`).
