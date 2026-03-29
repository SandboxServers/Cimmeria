# src/ — C++ Reference Implementation

**This is not the active server.** The running server is the Rust workspace in `crates/`.

This C++ code is the original Cimmeria server implementation and serves as the **reference implementation** — the ground truth for how the BigWorld protocol works, how entities behave, and what wire formats the game client expects. When implementing features in Rust, read the corresponding C++ code to understand the expected behavior.

194 files across 11 directories.

## Directory Structure

```
src/
├── authentication/     AuthenticationServer — HTTP/SOAP login, shard key exchange
├── baseapp/            BaseApp — persistent entity state, player data management
│   ├── entity/         BaseApp entity implementations
│   └── mercury/sgw/    SGW-specific Mercury message handlers
├── cellapp/            CellApp — spatial simulation, world cells, AoI
│   └── entity/         CellApp entity implementations
├── common/             Shared utilities used across all services
├── entity/             Core entity base classes
├── log/                Logging system
├── mercury/            Mercury reliable UDP protocol implementation
│   └── sgw/            SGW-specific Mercury extensions
├── nav_builder/        NavBuilder — offline navmesh generation (Recast/Detour)
├── openssl/            OpenSSL wrapper (0.9.8i — do not expose to internet)
├── util/               General utilities
└── xml/                TinyXML2 parsing utilities
```

## Shared Headers

- `stdafx.hpp` / `stdafx.cpp` — Precompiled header (includes Boost, standard library)
- `fwd_decls.hpp` — Forward declarations

## Building (Legacy)

Requires Windows, Visual Studio 2026 (v145 toolset), and all external dependencies bootstrapped:

```powershell
# Full bootstrap (downloads and builds all deps):
pwsh setup.ps1 -SkipApp -NoLaunch

# Or build the solution directly:
msbuild W-NG.sln /p:Configuration=Debug /p:Platform=x64
```

See `bootstrap/README.md` for dependency setup details.

## Migration Status

The C++ dependency stack has several pending upgrades. See [docs/architecture/migration-roadmap.md](../docs/architecture/migration-roadmap.md) for the full roadmap. TL;DR: OpenSSL 0.9.8i is the most critical — do not expose this server to the internet.
