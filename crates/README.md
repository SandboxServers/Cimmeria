# crates/ — Rust Server (Active Development)

This directory is the primary codebase. All active server development happens here. The C++ code in `deprecated/cpp/` is the legacy reference implementation.

For testing conventions across these crates — test types, when to use which, common gotchas — see **[../TESTING.md](../TESTING.md)**.

## Crate Overview

The 23 workspace members and their **actual** inter-crate dependencies, generated
from each crate's `Cargo.toml` (an arrow **A → B** means *A depends on B*). The 23
comes from the `members` list in the root [Cargo.toml](../Cargo.toml): the 19
crates under `crates/`, plus `src-tauri` and the three tool crates
(`tools/ContentEditor`, `tools/SceneEditor`, `tools/spec-lint`). The `fuzz/`
target is a deliberate workspace `exclude` — it needs nightly Rust.

```mermaid
%%{init: {"flowchart": {"htmlLabels": false}, "theme": "neutral"}}%%
flowchart TD
    %% entry points / binaries
    server --> adminApi["admin-api"]
    server --> services
    server --> discord
    server --> observability
    server --> common
    app["app (src-tauri, cimmeria-app)"] --> adminApi
    app --> services
    app --> common
    wireclient --> services
    wireclient --> mercury
    wireclient --> common

    %% service + domain layer
    adminApi --> services
    adminApi --> contentEngine["content-engine"]
    adminApi --> entity
    adminApi --> commands
    adminApi --> common
    services --> game
    services --> contentEngine
    services --> entity
    services --> mercury
    services --> discord
    services --> observability
    services --> commands
    services --> common
    game --> entity
    game --> commands
    game --> common
    contentEngine --> entity
    contentEngine --> common
    entity --> defs
    entity --> mercury
    entity --> commands
    entity --> common

    %% foundation
    mercury --> common
    defs --> common
    commands --> common

    %% UPK / navmesh toolchain (independent of the server spine)
    navmeshExtractor["navmesh-extractor"] --> upkObjects["upk-objects"]
    navmeshExtractor --> upk
    sceneEditor["scene-editor (tool)"] --> upkObjects
    sceneEditor --> upk
    upkObjects --> upk

    %% standalone crates with no intra-workspace dependencies
    subgraph standalone["Standalone (no intra-workspace deps)"]
        supervisor
        clientTelemetry["client-telemetry"]
        launcher["launcher (sgw-launcher)"]
        contentEditor["content-editor (tool)"]
        specLint["spec-lint (tool)"]
    end
```

The DAG is rooted at **common**. **services** is the hub — it pulls in `game`,
`content-engine`, `entity`, `mercury`, `discord`, and `observability`, and is
what `server`, `admin-api`, the `app` desktop GUI, and `wireclient` build on. The
`upk` / `upk-objects` / `navmesh-extractor` crates (plus the `scene-editor` tool)
are an independent Unreal-package / navmesh toolchain; `supervisor`,
`client-telemetry`, `launcher`, `content-editor`, and `spec-lint` have no
intra-workspace deps. All 19 crates in `crates/` are catalogued below; the
diagram additionally shows the `src-tauri` app and the `tools/` editors, which
are workspace members that live outside `crates/`.

| Crate | Package Name | Purpose |
|---|---|---|
| `common` | `cimmeria-common` | Shared types, config loading, error handling. No deps on other crates. |
| `mercury` | `cimmeria-mercury` | Mercury reliable UDP protocol + two wire-compatible encryption versions: legacy v1 (AES-256-CBC + HMAC-MD5, byte-identical to the stock client) and modernized v2 (per-packet random IV, HKDF-split keys, truncated HMAC-SHA256). Version selection is public via `encryption::EncryptionVersion` (`V1`/`V2`, `from_config_u8`) + `MercuryEncryption::from_session_key_versioned`; a context is pinned to one version for the session (v2 rejects v1-shaped input as a downgrade defense). Server-initiated session-key rotation (v2 only) lives in `encryption::rotation` (`RotationState`, `rotation_enabled`, `build_rotation_payload`, `recover_rotation_key`, `generate_session_key`) + the `MsgId::RotateSessionKey` opcode: the new 32-byte key travels encrypted under the current context, with a dual-key inbound window across the switch (cadence from `ServerConfig::mercury_key_rotation_secs`, default 3600). Owns the `Transport` + `BidirectionalTransport` traits (`UdpTransport` prod impl; `TestTransport` recorder + `LossyTransport` chaos wrapper behind the `test-support` feature) — the wire seam for byte-exact fan-out tests and lossy-network integration. Also owns the `Clock` trait (`SystemClock` prod impl; `TestClock` in the harness), the Tier 2 loopback session harness (`test_harness` module behind the `test-harness` feature) for paired-channel end-to-end tests, and the network-chaos apparatus (`test_harness::pcap_replay` + chaos scenario tests) for lomiada-class regression guards. See [docs/architecture/transport-trait.md](../docs/architecture/transport-trait.md), [docs/architecture/mercury-loopback-harness.md](../docs/architecture/mercury-loopback-harness.md), and [docs/architecture/network-chaos-testing.md](../docs/architecture/network-chaos-testing.md) |
| `defs` | `cimmeria-defs` | Parses entity definitions from `entities/defs/` XML into Rust types |
| `entity` | `cimmeria-entity` | Entity lifecycle management, property synchronization |
| `commands` | `cimmeria-commands` | Server command dispatch framework |
| `game` | `cimmeria-game` | Game mechanics: combat, abilities, stats, effects |
| `content-engine` | `cimmeria-content-engine` | Data-driven content runtime: missions, dialogs, sequences |
| `services` | `cimmeria-services` | Auth, Base, and Cell service implementations — the bulk of server logic |
| `admin-api` | `cimmeria-admin-api` | REST API for server administration |
| `supervisor` | `cimmeria-supervisor` | Process supervision and service lifecycle |
| `server` | `cimmeria-server` | **Binary entry point.** `cargo run -p cimmeria-server` |
| `launcher` | `sgw-launcher` | Player-facing game launcher. egui native window, installs from a seed + patch manifest on Azure Blob, launches `SGW.exe` or the Atera debug bat, uploads debug logs back to storage. Owns the DLL-injection path — extracted into the shared `cimmeria-client-launch` crate (#685) so `cimmeria-lab` drives the same code — that side-loads `cimmeria-client-telemetry` into `SGW.exe`. See [docs/client/sgw-launcher.md](../docs/client/sgw-launcher.md). |
| `client-launch` | `cimmeria-client-launch` | Shared client launch primitives — suspended-process launch, DLL injection, and SGW.exe `.rdata` hostname patch — extracted from `sgw-launcher` (#685) so both it and `cimmeria-lab` drive the same code path. Compiles everywhere (cfg'd stubs off Windows); the launcher re-exports it as `launch` / `inject` / `patch_rdata`. |
| `client-telemetry` | `cimmeria-client-telemetry` | **Windows-only cdylib** (`i686-pc-windows-msvc`) injected into `SGW.exe` for client-side observability. Subscribes to CME EventSignals, installs function hooks, and tees client logs to cimmeria-server's `/api/telemetry/upload-chunk`. Built and tested by its own [client-telemetry-build CI workflow](../.github/workflows/client-telemetry-build.yml). See [docs/reverse-engineering/findings/client-instrumentation-hookpoints.md](../docs/reverse-engineering/findings/client-instrumentation-hookpoints.md) for the hook anchor table. |
| `upk` | `cimmeria-upk` | UPK (Unreal Package) file parser |
| `upk-objects` | `cimmeria-upk-objects` | UE3 object deserializers: `StaticMesh` (LODs + kDOP collision), `Terrain` (heightmap + hole flags), `Model` / `Polys` (BSP world geometry), `Texture2D`, bulk data, and the cross-package export index |
| `navmesh-extractor` | `cimmeria-navmesh-extractor` | Extracts UE3 `.umap` chunk collision (StaticMesh, Terrain, BSP) to `.obj` for the C++ NavBuilder Recast pipeline, with the `extract_map` CLI (coverage report, floor probe), the `nav_inspect` connectivity gate (probe reachability plus `--gaps`, the boundary-edge gap finder and bottleneck chain search) and `obj_slab`, which measures the source chunk OBJs at a gap's coordinates to classify it. Also ships `archetype_census`, which reports what the prefab-archetype set actually contains per mesh. Owns the XRC `.nav` round-trip parser/emitter — the canonical Rust-side ground truth for the wire format `crates/entity/src/navigation/` consumes at runtime. See [README](navmesh-extractor/README.md); the measured results live in [docs/engine/castle-extraction-measurements.md](../docs/engine/castle-extraction-measurements.md) and [docs/engine/castle-navmesh-connectivity.md](../docs/engine/castle-navmesh-connectivity.md). |
| `wireclient` | `cimmeria-wireclient` | **Tier 3 headless test client.** Drives the SOAP auth, Mercury phase-3 handshake, and replays captured `.pcap` + AES-key sessions for end-to-end behavioral validation. Pairs with `tools/pcap_to_session.py` (JSONL exporter built atop `tools/pcap_dissect.py`). See [docs/architecture/wireclient.md](../docs/architecture/wireclient.md). |
| `discord` | `cimmeria-discord` | Discord notification sink. Owns the `EventKind` catalogue and per-event `EventToggles`, hot-reloadable TOML config (`config::ConfigWatcher` over [config/discord.toml.example](../config/discord.toml.example)), channel routing (`router::channel_for`), embed formatting + budget trimming (`embed::format_event`), and a rate-limited async sender (`sender::` — HTTP, mock, and token-bucket). Also exposes `DiscordLayer`, a `tracing` layer that lifts warn/error records into notifications. Typed `emit_*` helpers are the intended call surface. See [docs/architecture/discord-notifications.md](../docs/architecture/discord-notifications.md). |
| `observability` | `cimmeria-observability` | Metrics facade — `counter!`/`histogram!`/`gauge_add!` macros wrapping the OpenTelemetry SDK's metrics API. Lazily registers instruments on first emission, no-ops when telemetry is disabled. Initialised from `cimmeria-server`'s `otel::init` alongside traces + logs. See [docs/architecture/instrumentation-discipline.md](../docs/architecture/instrumentation-discipline.md). |
| `lab-mcp` | `cimmeria-lab-mcp` | In-server MCP endpoint (streamable HTTP via `rmcp`) for the live research lab (#687, #688). Fixed tool set — drive/inspect: `server_console_list`, `server_console_exec`, `server_sessions`, `server_log_tail`, `server_content_reload`, `server_db_query`; live state (#688): `server_entity_get`, `server_entity_query`, `server_witnesses`; per-session decoded packet taps (#688): `server_packet_tap_start`, `server_packet_tap_read`, `server_packet_tap_stop` — so an agent can drive/inspect a running server from Claude Code. Live-state + witness queries answer between ticks via `BaseToCellMsg::LabQuery` (read-only, capped); packet taps hang off the `wire_log` decode seams into a bounded per-session ring. Runs on its **own** `TcpListener` — never the admin router (#439) — fail-closed on `CIMMERIA_LAB_MCP_BIND` + `CIMMERIA_LAB_MCP_TOKEN` (>=32-byte token), gated by a single shared bearer token (constant-time compare), one `lab.tool_call` audit event per call. See [docs/architecture/live-research-lab.md](../docs/architecture/live-research-lab.md). |
| `lab` | `cimmeria-lab` | **Windows-only supervisor + stdio MCP server** for the Live Research Lab (phases 1–2). Proxies the `client_*` probe tools to the injected client bridge (`cimmeria-client-telemetry` built `--features lab-bridge`) over a token-gated framed-JSON-RPC loopback channel, and owns the SGW.exe process lifecycle: `lab_client_start`/`_stop`/`_restart`/`_status`, `lab_login` (Lua autologin), `lab_screenshot`, `lab_crash_report`, a heartbeat watchdog, and a crash-recovery journal + quarantine. `lab_timeline` (phase 6) merges the local client event ring (heartbeat today; the full `client_events_read` ring is gated on #686) with server packet-tap rows fetched from `cimmeria-lab-mcp` over HTTP, projected onto one clock via a packet-tap round-trip offset estimate. Excluded from the workspace CI jobs (like `sgw-launcher`); not in `default-members`. See [docs/architecture/live-research-lab.md](../docs/architecture/live-research-lab.md) (design) and [docs/guides/live-research-lab.md](../docs/guides/live-research-lab.md) (rulebook + operating manual). |

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

# Full workspace check (high memory on WSL — skip the GUI apps and the
# Windows-only client-telemetry cdylib):
cargo check --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher --exclude cimmeria-client-telemetry --exclude cimmeria-lab
```

See the root [CLAUDE.md](../CLAUDE.md) for WSL memory management rules.

## Testing

The workspace currently carries **2,936 `#[test]` / `#[tokio::test]` cases across 461 files**, of which **2,691 are gated in CI** (the six excluded crates below contribute the rest). 224 are live-DB regression guards — all in `cimmeria-services` — and 3 are end-to-end PL/pgSQL smokes. Run the full suite:

```bash
# Unit + non-DB integration:
cargo test --workspace --exclude cimmeria-app --exclude cimmeria-content-editor \
  --exclude cimmeria-scene-editor --exclude sgw-launcher --exclude cimmeria-client-telemetry --exclude cimmeria-lab

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
| `game/src/inventory/`, `missions/`, `commands/`, `social/`, `world/` | Per-system game logic |
| `content-engine/src/lib.rs` | Content pipeline (missions, dialogs, sequences) |
