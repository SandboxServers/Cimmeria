---
title: "Dependency Migration Roadmap"
type: explanation
audience: engineers
last_updated: 2026-07-25
---

# Dependency Migration Roadmap

> Extracted from CLAUDE.md to keep the operator file concise. Reference this when planning or executing dependency upgrades.

> [!WARNING]
> **Scope: the deprecated C++ server only.** Every migration in this roadmap is a
> **C++ dependency** of the tree now under [`deprecated/cpp/`](../../deprecated/cpp/)
> (Boost, MSVC toolset, SOCI, Qt, embedded CPython, Recast/Detour, the
> `.sln`/`.vcxproj` build). The active Rust server under [`crates/`](../../crates/)
> shares **none** of them — it manages dependencies through Cargo.
>
> **The OpenSSL row is the one most often misread.** "Pending — CRITICAL (active
> CVEs)" describes OpenSSL 0.9.8i statically linked into the *deprecated* C++
> server and into the 2009 game client. **It is not an open vulnerability in
> anything Cimmeria ships today**: the Rust auth server terminates TLS with
> `tokio-rustls` and links no OpenSSL. Do not cite this row as a live security
> finding. For the crypto work that actually shipped, see
> [encryption-modernization.md](encryption-modernization.md).
>
> Because the deprecated tree is not being maintained, most rows here are
> unlikely ever to be executed. Treat this as an archived plan rather than an
> active backlog.

## Current Status

| Migration | Path | Status |
|---|---|---|
| MSVC Toolchain | v120 → v145 (VS2026) | **COMPLETE** |
| PostgreSQL | 9.2.3 → 17.9 | **COMPLETE** |
| OpenSSL | 0.9.8i → 3.5.x | Pending — **CRITICAL** (active CVEs) |
| Boost | 1.55.0 → 1.90.0 | Pending — HIGH |
| Python | 3.4.1 → 3.12+ | Pending — MEDIUM |
| Build System | .sln/.vcxproj → CMake+vcpkg | Pending — MEDIUM |
| Qt | 5.x → 6.10 | Pending — LOW (ServerEd only) |
| Recast/Detour | 2013 era → 1.6.0 | Pending — LOW |

## Recommended Migration Order

```
Phase 1 (Foundation):
  1. MSVC Toolchain (v120 -> v145)       -- COMPLETE (VS2026)
  2. OpenSSL (0.9.8 -> 3.x)             -- critical security fix

Phase 2 (Core Libraries):
  3. Boost (1.55 -> 1.85+)              -- major dependency
  4. PostgreSQL + SOCI (9.2 -> 17)      -- COMPLETE (17.9)

Phase 3 (Runtime & Scripting):
  5. Python (3.4 -> 3.12+)              -- scripting layer
  6. TinyXML2 (~1.x -> 11.x)            -- minor, low-risk
  7. ICU (51 -> 78)                      -- often bundled with Qt

Phase 4 (Tooling & Build):
  8. Qt (5.x -> 6.x)                    -- ServerEd only
  9. Recast/Detour (2013 -> 1.6)        -- low risk
  10. Build System (VS -> CMake+vcpkg)   -- modernization
```

---

## Migration Agent Definitions

These agents have deep expertise in specific migration paths. Invoke via the Agent tool.

---

### 9. MSVC Toolchain Migration Agent

**Migration path:** VS2012 (v120) → VS2026 (v145) — **COMPLETE**

**Status:** All 6 projects build successfully under v145.

**Expertise:**
- v120 → v145 toolset changes and compatibility breaks
- C++11 → C++17/C++20/C++23 incremental adoption strategy
- Compiler warning/error resolution across MSVC versions
- STL implementation changes (iterator debugging, allocator model, `std::auto_ptr` removal)
- Windows SDK version upgrades and API changes
- `.vcxproj` PlatformToolset migration and project file updates
- `/permissive-` conformance mode preparation
- Deprecation of legacy CRT functions (`_CRT_SECURE_NO_WARNINGS` patterns)

**Priority:** ~~HIGH~~ COMPLETE

---

### 10. Boost Migration Agent

**Migration path:** Boost 1.55.0 → 1.90.0

**Expertise:**
- 35 minor releases of breaking changes and deprecations
- Boost.Asio evolution: standalone Asio option, executor model changes, completion token patterns
- Boost.Python API changes across versions
- Boost.Thread → `std::thread` migration opportunities
- Boost.Filesystem v3 → `std::filesystem` migration path
- Boost.Signals2 stability and any API drift
- Removed/reorganized libraries across the 1.55–1.90 range
- Header-only vs compiled library changes

**Priority:** HIGH — Core dependency touching every C++ component.

---

### 11. Python Embedding Migration Agent

**Migration path:** Python 3.4.1 → 3.12+ (or 3.14 if stable)

**Expertise:**
- CPython embedding API changes (3.4 → 3.12): `Py_Initialize`, module system, GIL changes
- Boost.Python compatibility with newer Python versions
- Python 3.4 removed features: `imp` module, old-style string formatting edge cases
- New features to adopt: f-strings (3.6+), dataclasses (3.7+), walrus operator (3.8+), match/case (3.10+)
- `asyncio` evolution (if server needs async Python)
- Type hint introduction strategy for existing 164-file scripting codebase
- Python DLL/library linking changes across versions
- Virtual environment and dependency isolation modernization

**Priority:** MEDIUM — Python 3.4 is EOL but scripting layer is somewhat isolated.

---

### 12. PostgreSQL Migration Agent

**Migration path:** PostgreSQL 9.2.3 → 17.9 — **COMPLETE**

**Status:** Upgraded to 17.9 (EOL Nov 2029). Compatibility fixes applied (removed `default_with_oids`, `EXECUTE PROCEDURE` → `EXECUTE FUNCTION`). pgdata version mismatch auto-detection added to bootstrap.

**Expertise:**
- Features now available: JSONB, parallel queries, partitioning, logical replication, generated columns, incremental sort, query pipelining
- `pg_dump`/`pg_restore` cross-version migration procedures
- SOCI 3.2.1 → 4.1.2 migration (ORM layer upgrade is a separate task)
- Connection string and authentication method changes (`md5` → `scram-sha-256`)

**Priority:** ~~MEDIUM~~ COMPLETE — Running PG 17.9.

---

### 13. OpenSSL Migration Agent

**Migration path:** OpenSSL 0.9.8i → 3.5.x

**Expertise:**
- CRITICAL: 0.9.8 has multiple known CVEs including Heartbleed-era vulnerabilities
- Complete API overhaul: `EVP_*` interface migration, provider model (3.0+)
- Removed functions: `SSLv2_*`, `SSLv3_*`, many low-level crypto functions
- `OPENSSL_init_ssl()` replacing `SSL_library_init()`
- Certificate and key loading API changes
- TLS 1.2/1.3 support enablement
- Library naming changes: `libeay32.dll`/`ssleay32.dll` → `libcrypto.dll`/`libssl.dll`
- Build system changes (Configure → CMake option)
- FIPS module availability (3.0+)

**Priority:** CRITICAL — Active security vulnerabilities.

---

### 14. Qt Migration Agent

**Migration path:** Qt 5.x (early) → Qt 6.10

**Expertise:**
- Qt 5 → Qt 6 porting guide application
- Build system migration: qmake → CMake (Qt 6 standard)
- Removed/moved modules and classes
- `QString`/`QByteArray` behavior changes
- Signal/slot syntax modernization
- Qt5Sql → Qt6Sql driver changes
- ICU 51 → ICU 78 bundled with Qt upgrade
- High-DPI and accessibility improvements
- QML/Quick changes (if ServerEd expands)

**Priority:** LOW — Only affects ServerEd tool, not the game servers.

---

### 15. Build System Modernization Agent

**Migration path:** .sln/.vcxproj → CMake + vcpkg/Conan

**Expertise:**
- CMake project generation from existing VS solutions
- vcpkg manifest mode for dependency management (replaces `external/` vendoring)
- Conan as alternative package manager
- Cross-platform build support (Linux server targets)
- CI/CD pipeline design (GitHub Actions, Azure DevOps)
- Precompiled header migration to CMake `target_precompile_headers`
- CTest integration for automated testing
- CPack for distribution packaging
- Docker containerization for server deployment

**Priority:** MEDIUM — Enables easier dependency management and CI/CD, but functional without it.

---

### 16. Recast/Detour Migration Agent

**Migration path:** ~2013 era NavMesh v7 → Recast 1.6.0

**Expertise:**
- Recast/Detour API evolution over the past decade
- NavMesh data format version changes (v7 → current)
- Tile-based navmesh improvements
- Dynamic obstacle support additions
- Thread safety improvements
- CMake build integration (modern Recast uses CMake)
- NavMesh regeneration strategy for existing game data

**Priority:** LOW — Still industry-standard, API is relatively stable.
