---
title: Getting Started
type: tutorial
audience: new contributors, first-time setup
last_updated: 2026-07-25
companion_docs:
  - ../building.md
  - ../troubleshooting.md
  - ../../bootstrap/README.md
  - ../../CONTRIBUTING.md
  - ../../CLAUDE.md
  - ../../TESTING.md
---

# Getting Started with Cimmeria

This tutorial walks you from a fresh clone of the repository to a running Cimmeria server with the game client connected. By the end you'll have:

- The Rust server built and running on your machine.
- A bundled PostgreSQL instance loaded with the game schema and seed data.
- The Stargate Worlds game client connected, sitting at the character-select screen, ready to play.
- Confidence in where to look next when something needs to change.

Plan on **30–60 minutes** for the first run. Subsequent runs take seconds — the bootstrap caches everything it can.

If anything goes wrong, jump to [`troubleshooting.md`](../troubleshooting.md). The fix is almost certainly listed there.

---

## Step 1 — Prerequisites

You'll need:

| Requirement | Where to get it | Why |
|---|---|---|
| **Windows 11** (or Windows 10 with WSL2) | — | The server targets Windows; the game client is Windows-only. |
| **PowerShell 7+** | [PowerShell/PowerShell](https://github.com/PowerShell/PowerShell) | The bootstrap is a PowerShell script. PowerShell 7 ships with Windows 11. Check with `pwsh --version`. |
| **Rust stable** | [rustup.rs](https://rustup.rs) | `cargo build` needs it. The setup script enforces a stable toolchain. |
| **Visual Studio Build Tools** (MSVC) | [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) | Cargo needs the MSVC linker on Windows. Install the "Desktop development with C++" workload. |
| **Git** | [git-scm.com](https://git-scm.com) | Obviously. |
| **Node.js 22+** *(optional)* | [nodejs.org](https://nodejs.org) | Only needed if you pass `-WithAdmin` to build the Tauri admin panel. |
| **Docker Desktop** *(optional)* | [docker.com](https://www.docker.com/products/docker-desktop) | Only needed if you pass `-UseDocker` to run PostgreSQL in a container instead of locally. |
| **The Stargate Worlds client** | See [`game/sgw/README.md`](../../game/sgw/README.md) | You need the game's data files to actually connect a client. |

Disk space: about **1 GB** for the Cargo target directory, PostgreSQL binaries, and dependency cache.

> **Working in WSL?** Reading the WSL build-memory rules in [`CLAUDE.md`](../../CLAUDE.md) is mandatory before your first full build — the link step can consume ~47 GB RAM without care. The mitigations are simple but you need to know about them.

---

## Step 2 — Clone the repository

```powershell
git clone https://github.com/SandboxServers/Cimmeria.git
cd Cimmeria
```

A fresh checkout looks broken at this point — the `external/` directory is missing, the database is empty, and there's no compiled binary. That's expected. `setup.ps1` populates everything in the next step.

---

## Step 3 — Run the bootstrap

```powershell
pwsh setup.ps1
```

This is the one-command path. The script runs eight steps:

| Step | What happens | Caches result? |
|---|---|---|
| **1. Prerequisites** | Verifies Rust, MSVC, optionally Node.js. | n/a |
| **2. Dependencies** | Downloads PostgreSQL 17 binaries (~50 MB) and 7-Zip sidecar. | yes |
| **3. Build Server** | `cargo build` the workspace. ~5–15 min first time, ~30 sec incremental. | yes |
| **4. Build Admin** *(optional)* | Tauri admin panel — only with `-WithAdmin`. | yes |
| **5. Build Launcher** *(optional)* | SGW game launcher — only with `-WithLauncher`. | yes |
| **6. Database** | Initialises PostgreSQL on port 5433, loads `db/database.sql`, inserts seed data including the `test` account and shard record. | yes |
| **7. RE Toolchain** *(optional)* | Ghidra + GhidraMCP + x64dbg + MCP venvs — only with `-WithReToolchain`. | yes |
| **8. Launch** | Starts `cimmeria-server.exe`. Skip with `-NoLaunch`. | n/a |

Every step is idempotent — safe to re-run. The script detects completed work and skips it. If it fails midway, it tells you exactly which step blew up; the [`troubleshooting`](../troubleshooting.md) doc has fixes for the common ones.

The flags you'll reach for most often:

```powershell
# Build everything (server + admin panel + launcher)
pwsh setup.ps1 -WithAdmin -WithLauncher

# Use Docker for PostgreSQL instead of local binaries
pwsh setup.ps1 -UseDocker

# Re-seed the database without rebuilding
pwsh setup.ps1 -SkipBuild -ForceDatabase

# Build + DB init but don't launch (CI-style)
pwsh setup.ps1 -NoLaunch
```

The full flag reference is in [`bootstrap/README.md`](../../bootstrap/README.md). When something goes sideways, the standalone PowerShell module functions (`Build-CimmeriaServer`, `Initialize-CimmeriaDatabase`, `Start-CimmeriaServer`, …) let you re-run a single step.

---

## Step 4 — Verify the server is up

When `setup.ps1` finishes the launch step, you should see log lines streaming. Look for:

```text
[INFO] auth service listening on 0.0.0.0:8081
[INFO] base service listening on 0.0.0.0:13001 (TCP) and 0.0.0.0:32832 (UDP)
[INFO] cell service started
[INFO] cimmeria-server ready
```

If you're running with `-NoLaunch`, start it manually:

```powershell
.\cimmeria-server.exe
# or:
cargo run -p cimmeria-server
```

Quick sanity check from another PowerShell window:

```powershell
Test-NetConnection -ComputerName localhost -Port 8081
# TcpTestSucceeded : True
```

Cimmeria binds to **five ports** by default:

| Port | Protocol | Role |
|---|---|---|
| `8081` | TCP / HTTP+SOAP | Authentication (client login + shard select) |
| `13001` | TCP / Mercury | Auth ↔ BaseApp control channel |
| `32832` | UDP / Mercury | Game client ↔ BaseApp (the main game traffic) |
| `50000` | UDP / Mercury | Internal Base ↔ Cell traffic |
| `8443` | TCP / HTTP | Admin REST API — always started in-process; `-WithAdmin` only builds the Tauri desktop client that talks to it |

If you're running with `-UseDocker`, PostgreSQL also exposes port `5433`.

---

## Step 5 — Connect the game client

You need the Stargate Worlds game client installed. The repo doesn't ship it — see [`game/sgw/README.md`](../../game/sgw/README.md) for installation. Once it's installed:

1. Launch the client via **`AteraLoader.exe`**, not `SGW.exe` directly. AteraLoader starts the game and injects `AtreaRL.dll`, which applies the binary patches that point the client at your local server. See [`../client-tools.md`](../client-tools.md) for the full tool stack.
2. At the login screen, enter `test` / `test` and submit.
3. The client connects to `localhost` (default), authenticates against the auth server, picks the shard from `BaseApp`, and lands you at character select.

For **LAN play** — another machine on your network connecting to your server — set `BASE_EXTERNAL` to your LAN IP **before launching the server**:

```powershell
$env:BASE_EXTERNAL = "10.0.0.42"
.\cimmeria-server.exe
```

The full LAN-setup details are in [`multiplayer.md`](../multiplayer.md).

---

## Step 6 — Run the test suite

You've verified the server runs. Now verify your build can also run the tests CI runs — this is the loop you'll use during every PR.

```powershell
# Fast iteration check (1.5s, <2 GB RAM):
cargo check -p cimmeria-services

# Full workspace check (skip the GUI apps so the linker doesn't OOM on WSL,
# and the Windows-only client-telemetry cdylib):
cargo check --workspace `
  --exclude cimmeria-app --exclude cimmeria-content-editor `
  --exclude cimmeria-scene-editor --exclude sgw-launcher `
  --exclude cimmeria-client-telemetry

# Run the test suite (no live DB needed):
cargo nextest run --profile=ci --workspace `
  --exclude cimmeria-app --exclude cimmeria-content-editor `
  --exclude cimmeria-scene-editor --exclude sgw-launcher `
  --exclude cimmeria-client-telemetry
```

If you don't have nextest installed yet: `cargo install cargo-nextest --locked`.

Live-DB tests (the ones that need a running PostgreSQL) self-skip via `require_db_or_skip!` when `DATABASE_URL` is unset. To opt in:

```powershell
$env:DATABASE_URL = "postgres://w-testing:w-testing@localhost:5433/sgw"
cargo nextest run --profile=ci-live-db -p cimmeria-services --lib
```

The full pre-PR checklist (formatting, clippy, build, tests, doctests, lint scripts) is in [`CLAUDE.md`](../../CLAUDE.md) → "Pre-PR checklist." CI runs it exactly — if you skip it locally, the PR will round-trip.

---

## Step 7 — Where to go next

You have a working dev environment. Here are the next steps depending on what you want to do:

**Contribute code:**

- Read [`../../CONTRIBUTING.md`](../../CONTRIBUTING.md) for contribution scope, code style, PR conventions.
- Browse [`good-first-issue` labelled issues](https://github.com/SandboxServers/Cimmeria/labels/good-first-issue) for a scoped first task.
- Read [`../../TESTING.md`](../../TESTING.md) before writing any test — picking the wrong type is the #1 PR rework cause.

**Understand the codebase:**

- [`connection-flow.md`](../connection-flow.md) — end-to-end login through world entry. Walking through this once unlocks most of the architecture.
- [`how-sgw-works.md`](../how-sgw-works.md) — BigWorld, CME, and how everything fits together.
- [`../../crates/README.md`](../../crates/README.md) — crate layout, dependency graph, key source files per crate.
- [`architecture/service-architecture.md`](../architecture/service-architecture.md) — the three-process topology (auth / base / cell).

**Work on a specific game system:**

- [`gameplay/`](../gameplay/) — per-system breakdowns (combat, missions, inventory, abilities, ...).
- [`content/`](../content/) — the data-driven content engine that drives most non-combat content.
- [`protocol/`](../protocol/) — wire formats, message catalog, dispatch tables.

**Reverse-engineering work:**

- [`re-toolchain-setup.md`](re-toolchain-setup.md) — install Ghidra, GhidraMCP, x64dbg, MCP bridges.
- [`reverse-engineering-with-claude.md`](reverse-engineering-with-claude.md) — workflow doc for AI-assisted RE.
- [`../reverse-engineering/`](../reverse-engineering/) — findings, address maps, the RE plan.

**Operate or deploy:**

- [`../operations/container.md`](../operations/container.md) — running the published container image.
- [`../operations/colo-deploy.md`](../operations/colo-deploy.md) — self-maintaining single-host deployment.
- [`../operations/telemetry.md`](../operations/telemetry.md) and [`../operations/signoz-deployment.md`](../operations/signoz-deployment.md) — observability.

---

## Bumped into a problem?

[`troubleshooting.md`](../troubleshooting.md) covers the common ones:

- WSL build OOM
- PostgreSQL won't start (port in use, pgdata version mismatch)
- `DATABASE_URL` not set
- Client can't connect (`BASE_EXTERNAL`, port table, AtreaRL setup)
- `cargo check` vs `cargo build` performance
- `external/` directory missing after fresh clone

If the fix isn't there, open an issue with the exact error message, the platform, and which step it died on. Maintainers will help, and the issue becomes the next entry in `troubleshooting.md`.
