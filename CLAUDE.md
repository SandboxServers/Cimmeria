# Cimmeria — Stargate Worlds Emulator

A server emulator for Stargate Worlds. Active development is in Rust (`crates/`); the C++ code in `src/` and Python scripts in `python/` are reference implementations — read them for behavior, implement in Rust.

For human-readable project overview, see [README.md](README.md). For the dependency migration roadmap and migration-specialist agent definitions, see [docs/architecture/migration-roadmap.md](docs/architecture/migration-roadmap.md).

## Repo invariants (non-obvious)

- `external/` and `bin64/`/`lib64/` are **not in git** — populated by `setup.ps1`. A fresh checkout looks broken until setup runs.
- `db/deprecated/` contains old monolithic SQL files. **Do not load them.** Active schemas: `db/database.sql`, `db/sgw/`, `db/resources/`.
- `config/*.config` files contain test credentials. Real environments use `*.local` overrides (gitignored). Most-edited: `db_connection_string` in `BaseService.config`.
- Python console (port 8989) is password-gated; password lives in `AuthenticationService.config`.
- Legacy C++ uses **OpenSSL 0.9.8i** with active CVEs — never expose this server to the internet.
- Frontend convention: every meaningful frontend change requires a REPL-style logic UAT in addition to tests/builds — see [AGENTS.md](AGENTS.md).

## Build rules

### Rust (active)

Always target **Windows** — the server runs on Windows alongside the game client.

```bash
# WSL/Linux: cross-compile to Windows.
cargo build -p cimmeria-server --target x86_64-pc-windows-gnu --release
cp target/x86_64-pc-windows-gnu/release/cimmeria-server.exe .

# Windows natively:
cargo build -p cimmeria-server --release
cp target/release/cimmeria-server.exe .
```

After building, copy the exe to the project root.

### Rust build memory (WSL)

The full link can consume ~47 GB RAM. The workspace's `[profile.dev.package."*"]` strips dep debug info to bring this down to ~8 GB, but you still need to be careful:

1. **`cargo check -p cimmeria-services`** for iteration (1.5s, <2 GB). Only run full `cargo build`/`cargo test` when you actually need a binary or test results.
2. **Never run multiple `cargo`/`rustc` processes concurrently.** Kill stale ones before starting a new build: `pkill -f rustc`.
3. **Target specific crates** with `-p` rather than `--workspace`. Only build the workspace for final validation.
4. Sanity-check before building: `ps aux | grep -E "cargo|rustc" | grep -v grep`.
5. `CARGO_BUILD_JOBS=2` is set in `.bashrc` to cap parallel codegen.

Quick reference:

```bash
# Iteration
cargo check -p cimmeria-services

# Single-crate test
cargo test -p cimmeria-services

# Full workspace check — skip the Tauri apps so the linker doesn't OOM
cargo check --workspace \
  --exclude cimmeria-app \
  --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor

# Kill stale builds
pkill -f "cargo|rustc"
```

### C++ (legacy)

Solution: `W-NG.sln` (VS2026, MSVC v145). Bootstrap via `setup.ps1` (wraps the `CimmeriaBootstrap` PowerShell module — see [bootstrap/CimmeriaBootstrap/README.md](bootstrap/CimmeriaBootstrap/README.md) for individual functions).

## Migration status (one-line)

PostgreSQL 9.2 → 17.9 ✅ and MSVC v120 → v145 ✅ done. OpenSSL 0.9.8i → 3.x is the next critical migration (active CVEs). Full roadmap and per-migration agent definitions: [docs/architecture/migration-roadmap.md](docs/architecture/migration-roadmap.md).
