# crates/ — Rust Server (Active Development)

This directory is the primary codebase. All active server development happens here. The C++ code in `src/` is the legacy reference implementation.

For testing conventions across these crates — test types, when to use which, common gotchas — see **[../TESTING.md](../TESTING.md)**.

## Crate Overview

```
common ──┬──► mercury ──► entity ──► game ──────► services ──► server
         ├──► defs    ──►         ──► content-engine ──►
         └──► commands ──────────────────────────►
```

| Crate | Package Name | Purpose |
|---|---|---|
| `common` | `cimmeria-common` | Shared types, config loading, error handling. No deps on other crates. |
| `mercury` | `cimmeria-mercury` | Mercury reliable UDP protocol + AES-256-CBC/HMAC-MD5 encryption |
| `defs` | `cimmeria-defs` | Parses entity definitions from `entities/defs/` XML into Rust types |
| `entity` | `cimmeria-entity` | Entity lifecycle management, property synchronization |
| `commands` | `cimmeria-commands` | Server command dispatch framework |
| `game` | `cimmeria-game` | Game mechanics: combat, abilities, stats, effects |
| `content-engine` | `cimmeria-content-engine` | Data-driven content runtime: missions, dialogs, sequences |
| `services` | `cimmeria-services` | Auth, Base, and Cell service implementations — the bulk of server logic |
| `admin-api` | `cimmeria-admin-api` | REST API for server administration |
| `supervisor` | `cimmeria-supervisor` | Process supervision and service lifecycle |
| `server` | `cimmeria-server` | **Binary entry point.** `cargo run -p cimmeria-server` |
| `launcher` | `sgw-launcher` | Player-facing game launcher. egui native window, installs from a seed + patch manifest on Azure Blob, launches `SGW.exe` or the Atera debug bat, uploads debug logs back to storage. See [docs/client/sgw-launcher.md](../docs/client/sgw-launcher.md). |
| `upk` | `cimmeria-upk` | UPK (Unreal Package) file parser |
| `upk-objects` | `cimmeria-upk-objects` | UPK object type definitions |

## Building

```bash
# Iterative development (fast, <2 GB RAM):
cargo check -p cimmeria-services

# Run the server:
cargo run -p cimmeria-server

# Build release binary:
cargo build -p cimmeria-server --release

# Run tests for one crate:
cargo test -p cimmeria-services

# Full workspace check (high memory on WSL — skip the Tauri apps):
cargo check --workspace --exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor
```

See the root [CLAUDE.md](../CLAUDE.md) for WSL memory management rules.

## Testing

The workspace currently carries **1351 `#[test]` / `#[tokio::test]` cases across 215 files**, of which 151 are live-DB regression guards and 3 are end-to-end PL/pgSQL smokes. Run the full suite:

```bash
# Unit + non-DB integration (covers ~961 tests):
cargo test --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher

# Live-DB tests (start the bundled Postgres on :5433 first, then):
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  cargo test -p cimmeria-services --lib -- --test-threads=1
```

`--test-threads=1` is required for the live-DB run — some guards share sentinel id ranges and would collide under parallel execution. See [../TESTING.md](../TESTING.md) for the full picker, gotchas, and review checklist, and [../docs/testing/inventory/README.md](../docs/testing/inventory/README.md) for the catalogue of every test in the workspace (one file per crate).

## Key Source Files

| Path | Purpose |
|---|---|
| `services/src/auth/` | Authentication service — login, character select (`mod.rs`, `service.rs`, `handlers.rs`) |
| `services/src/base/` | BaseApp service — entity persistence, player state, character creation, world entry |
| `services/src/cell/` | CellApp service — world simulation, movement, abilities, combat, missions, gate travel |
| `services/src/mercury/` | Mercury transport glue — AoI, protocol dispatch, world data |
| `mercury/src/lib.rs` | Mercury packet framing, encryption, reliability |
| `game/src/combat/` | Combat system |
| `game/src/inventory/`, `missions/`, `interactions/`, `social/`, `world/` | Per-system game logic |
| `content-engine/src/lib.rs` | Content pipeline (missions, dialogs, sequences) |
