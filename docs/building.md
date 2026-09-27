---
title: Building the Cimmeria Server
type: how-to
audience: engineers, new contributors
last_updated: 2026-09-26
companion_docs:
  - ../README.md
  - ../bootstrap/README.md
  - ../crates/README.md
  - ../CLAUDE.md
  - architecture/build-system.md
  - guides/getting-started.md
  - troubleshooting.md
---

# Building the Cimmeria Server

The active Cimmeria server is a Rust workspace under [`crates/`](../crates/). This page is the **how-to** for building and running it. If this is your first time setting up the project, start with [the getting-started tutorial](guides/getting-started.md) instead — it walks the full prerequisite + setup + verification path.

> **Looking for the old C++ build?** That implementation is retired and lives under [`deprecated/`](../deprecated/). The C++ build instructions (`setup-dependencies.ps1`, `W-NG.sln`, Boost 1.55 / OpenSSL 1.0.1e) are kept for reference in [`technical/building.md`](technical/building.md) but are not relevant to current development.

## Prerequisites

- **PowerShell 7+** (`pwsh`) — ships with Windows 11; install from [PowerShell/PowerShell](https://github.com/PowerShell/PowerShell) on other platforms.
- **Windows, with the MSVC build tools.** Development builds run natively on Windows (PowerShell or Git Bash). CI builds on Linux, but there is no WSL or cross-compile path for development.
- **Rust via rustup** — install from [rustup.rs](https://rustup.rs). You don't pick a version: [`rust-toolchain.toml`](../rust-toolchain.toml) pins Rust 1.98.1 with rustfmt and clippy, rustup picks it up automatically, and CI builds with the same version.
- **Node.js 22+** — only required for the Tauri admin app (`-WithAdmin`) and the player-facing launcher (`-WithLauncher`).
- **PostgreSQL 17** — `setup.ps1` provisions a local managed instance on port 5433 automatically. Pass `-UseDocker` to run it in a container instead.
- Disk space for build output. A cold debug build of the CI-gated workspace, all targets, plus two rebuilds left a 14.2 GB target dir in the September 2026 measurement ([`architecture/build-system.md`](architecture/build-system.md)), and every extra worktree has its own. [`tools/build-hygiene/sweep.ps1`](../tools/build-hygiene/sweep.ps1) trims stale artifacts.

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
# Debug build:
cargo build -p cimmeria-server

# Release build, copied to the repo root:
cargo build -p cimmeria-server --release
Copy-Item .\target\release\cimmeria-server.exe .

# Full debug info, for a debugger session (builds into target\dev-debug\):
cargo build -p cimmeria-server --profile dev-debug
```

The server runs on Windows alongside the game client, and you build it there. The normal dev profile keeps line tables only, so panics and backtraces show file:line; build the `dev-debug` profile when you need variables in a debugger.

When you iterate, check the crate you changed rather than the whole server: `cargo check -p cimmeria-cell`, `-p cimmeria-cell-content`, `-p cimmeria-base-methods`, and so on. `cimmeria-services` is a small facade over about 20 crates, so `-p cimmeria-services` doesn't cover them. The crate table is in [`crates/README.md`](../crates/README.md).

If AI agents drive your builds, they go through the build lane (`tools/build-lane/lane.sh`), which limits how many builds run at once and gives each worktree its own target dir. See "Build lane and concurrency" in [`CLAUDE.md`](../CLAUDE.md) and [`agents/development-workflow.md`](agents/development-workflow.md#builds-worktrees-and-test-databases).

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
| `8443` | TCP / HTTP | Admin REST API — always started in-process; `-WithAdmin` only builds the Tauri desktop client |

Default test account is `test` / `test`.

## Verifying the server is up

```powershell
# Check the auth port responds:
Test-NetConnection -ComputerName localhost -Port 8081

# Tail the server log:
Get-Content -Wait -Tail 50 .\logs\cimmeria-server.log
```

When the client connects successfully you'll see a `client_handshake_ok` line in the log and the player will reach character select.

The headless `cimmeria-wireclient` in [`crates/wireclient/`](../crates/wireclient/) is the eventual home for end-to-end client smoke tests without the GUI, but **it cannot talk to a running server yet**. As of Phase 1 the crate contains no UDP socket at all — what ships is SOAP auth (Phase 1+2 against an in-process `AuthService`), byte-exact Mercury handshake builders/parsers, and JSONL session-trace load + diff. `Client::build_login_packet` deliberately stops at producing bytes; the socket loop is Phase 1.5. Documentation: [`architecture/wireclient.md`](architecture/wireclient.md).

## Running the test suite

The five gating checks CI runs are documented in [`CLAUDE.md`](../CLAUDE.md) under "Pre-PR checklist." The short version:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab --all-targets -- -D warnings
cargo build --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab --all-targets
cargo nextest run --profile=ci --workspace \
  --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher \
  --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab

# Live-DB tests (need a running Postgres on :5433):
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  tools/test-live-db.sh
```

See [`TESTING.md`](../TESTING.md) for the test-type taxonomy and when to use which.

## When the build breaks

Common first-run failures and how to recover:

- **`sccache: incremental compilation is prohibited`** — `CARGO_INCREMENTAL=1` is set while sccache is the `RUSTC_WRAPPER`. Unset `CARGO_INCREMENTAL` (the dev profile already builds workspace crates incrementally), or build through the lane, which drops sccache for that job.
- **`DATABASE_URL` not set** — live-DB tests self-skip via `require_db_or_skip!`. Set the env var to opt into them.
- **`external/` directory missing** — `external/` is not in git. It's populated by `setup.ps1`. A fresh checkout looks broken until setup runs, and a new worktree needs it junctioned in: create worktrees with `tools/build-lane/mk-worktree.sh`.
- **Port 5433 in use** — another Postgres is running. Stop it, or use `-UseDocker` so the bootstrap brings up its own.

The full list of first-day problems lives in [`troubleshooting.md`](troubleshooting.md).

## See also

- [`README.md`](../README.md) — project overview and status
- [`bootstrap/README.md`](../bootstrap/README.md) — the `setup.ps1` pipeline and the `CimmeriaBootstrap` PowerShell module
- [`crates/README.md`](../crates/README.md) — crate layout, dependency graph, key source files
- [`CLAUDE.md`](../CLAUDE.md) — repo invariants, build rules, pre-PR checklist
- [`architecture/build-system.md`](architecture/build-system.md) — why the toolchain is pinned, the profiles, the build lane, Dev Drive and cleanup
- [`TESTING.md`](../TESTING.md) — test types, picker, gotchas
- [`guides/getting-started.md`](guides/getting-started.md) — first-time walkthrough
- [`troubleshooting.md`](troubleshooting.md) — common first-day problems
