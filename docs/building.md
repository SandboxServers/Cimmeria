---
title: Building the Cimmeria Server
type: how-to
audience: engineers, new contributors
last_updated: 2026-05-27
companion_docs:
  - ../README.md
  - ../bootstrap/README.md
  - ../crates/README.md
  - ../CLAUDE.md
  - guides/getting-started.md
  - troubleshooting.md
---

# Building the Cimmeria Server

The active Cimmeria server is a Rust workspace under [`crates/`](../crates/). This page is the **how-to** for building and running it. If this is your first time setting up the project, start with [the getting-started tutorial](guides/getting-started.md) instead — it walks the full prerequisite + setup + verification path.

> **Looking for the old C++ build?** That implementation is retired and lives under [`deprecated/`](../deprecated/). The C++ build instructions (`setup-dependencies.ps1`, `W-NG.sln`, Boost 1.55 / OpenSSL 1.0.1e) are kept for reference in [`technical/building.md`](technical/building.md) but are not relevant to current development.

## Prerequisites

- **PowerShell 7+** (`pwsh`) — ships with Windows 11; install from [PowerShell/PowerShell](https://github.com/PowerShell/PowerShell) on other platforms.
- **Rust stable** — install from [rustup.rs](https://rustup.rs).
- **Node.js 22+** — only required for the Tauri admin app (`-WithAdmin`) and the player-facing launcher (`-WithLauncher`).
- **PostgreSQL 17** — `setup.ps1` provisions a local managed instance on port 5433 automatically. Pass `-UseDocker` to run it in a container instead.
- ~1 GB free disk space for the Cargo target dir and the bundled Postgres.

## One-command build and launch

From the repo root:

```powershell
pwsh setup.ps1
```

This runs the full pipeline: prerequisite check → Postgres provisioning → `cargo build` → schema load → server launch. Connect the game client with `test` / `test`.

The bootstrap pipeline is documented in detail in [`bootstrap/README.md`](../bootstrap/README.md). Common flags:

| Flag | Effect |
|---|---|
| `-WithAdmin` | Also build the Tauri admin panel (`tools/`). Needs Node.js. |
| `-WithLauncher` | Also build the player-facing `sgw-launcher`. |
| `-UseDocker` | Run PostgreSQL in a `postgres:17` container instead of locally. |
| `-ForceDatabase` | Drop and recreate the `sgw` database, then reload the schema. |
| `-ResetDatabase` | Nuclear option — stop Postgres, delete the entire `pgdata` directory, re-initialise from scratch. |
| `-WithReToolchain` | Install the reverse-engineering toolchain (Ghidra, GhidraMCP, x64dbg, MCP venvs, `.mcp.json`). Opt-in; Windows-only. |
| `-NoLaunch` | Build only; don't start the server. |
| `-SkipBuild` | Skip the Cargo build (useful with `-ForceDatabase` to re-seed without rebuilding). |

## Direct `cargo` builds

Once the prerequisites are in place you can drive the build directly:

```powershell
# Windows native — debug build:
cargo build -p cimmeria-server

# Windows native — release build:
cargo build -p cimmeria-server --release
```

```bash
# WSL/Linux — cross-compile to Windows:
cargo build -p cimmeria-server --target x86_64-pc-windows-gnu --release
cp target/x86_64-pc-windows-gnu/release/cimmeria-server.exe .
```

The server runs on Windows alongside the game client. Cross-compiling from WSL is a supported workflow; see [`CLAUDE.md`](../CLAUDE.md) for memory limits (~47 GB full link, mitigated by stripping dep debug info to ~8 GB).

## Running the server

```powershell
cargo run -p cimmeria-server
```

The server listens on:

| Port | Protocol | Role |
|---|---|---|
| `8081` | TCP / HTTP+SOAP | Authentication / shard select |
| `13001` | TCP / Mercury | Auth ↔ BaseApp control channel |
| `32832` | UDP / Mercury | Game client ↔ BaseApp |
| `50000` | UDP / Mercury | Internal Cell traffic |
| `8443` | TCP / HTTPS | Admin API (if `-WithAdmin` built) |

Default test account is `test` / `test`.

## Verifying the server is up

```powershell
# Check the auth port responds:
Test-NetConnection -ComputerName localhost -Port 8081

# Tail the server log:
Get-Content -Wait -Tail 50 .\logs\cimmeria-server.log
```

When the client connects successfully you'll see a `client_handshake_ok` line in the log and the player will reach character select.

For end-to-end client smoke tests without the GUI, see the headless `cimmeria-wireclient` in [`crates/wireclient/`](../crates/wireclient/) — it replays captured pcaps against a live server. Documentation: [`architecture/wireclient.md`](architecture/wireclient.md).

## Running the test suite

The five gating checks CI runs are documented in [`CLAUDE.md`](../CLAUDE.md) under "Pre-PR checklist." The short version:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher --all-targets -- -D warnings
cargo build --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher --all-targets
cargo nextest run --profile=ci --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher

# Live-DB tests (need a running Postgres on :5433):
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  cargo nextest run --profile=ci-live-db -p cimmeria-services --lib
```

See [`TESTING.md`](../TESTING.md) for the test-type taxonomy and when to use which.

## When the build breaks

Common first-run failures and how to recover:

- **WSL link step OOMs** — the full link can consume ~47 GB RAM. Use `cargo check -p cimmeria-services` for iteration; only run a full build when you need a binary. Cap parallelism with `CARGO_BUILD_JOBS=2`. Full details in [`CLAUDE.md`](../CLAUDE.md) → "Rust build memory (WSL)".
- **`DATABASE_URL` not set** — live-DB tests self-skip via `require_db_or_skip!`. Set the env var to opt into them.
- **`external/` directory missing** — `external/` is not in git. It's populated by `setup.ps1`. A fresh checkout looks broken until setup runs.
- **Port 5433 in use** — another Postgres is running. Stop it, or use `-UseDocker` so the bootstrap brings up its own.

The full list of first-day problems lives in [`troubleshooting.md`](troubleshooting.md).

## See also

- [`README.md`](../README.md) — project overview and status
- [`bootstrap/README.md`](../bootstrap/README.md) — the `setup.ps1` pipeline and the `CimmeriaBootstrap` PowerShell module
- [`crates/README.md`](../crates/README.md) — crate layout, dependency graph, key source files
- [`CLAUDE.md`](../CLAUDE.md) — repo invariants, build rules, pre-PR checklist
- [`TESTING.md`](../TESTING.md) — test types, picker, gotchas
- [`guides/getting-started.md`](guides/getting-started.md) — first-time walkthrough
- [`troubleshooting.md`](troubleshooting.md) — common first-day problems
