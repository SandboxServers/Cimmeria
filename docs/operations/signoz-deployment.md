# SigNoz deployment runbook

This document covers running the SigNoz observability stack alongside
Cimmeria — both in local dev (single Docker host) and in the colo
deployment. For the design rationale (why SigNoz, why ClickHouse, why
not Cosmos for this), see
[docs/architecture/observability.md](../architecture/observability.md).

For securely exposing the SigNoz UI to remote dev machines or to the
Cimmeria-MCP server for LLM-mediated retrieval, see
[signoz-remote-access.md](signoz-remote-access.md).

## What gets shipped to SigNoz

Two streams converge into the same ClickHouse-backed store, **split
across three SigNoz services** so wire-level and TRACE-level volume
doesn't drown the high-signal events in the operator's primary triage
view:

1. **`service.name = cimmeria-server`** — the high-signal index. Auth,
   content chains, combat, missions, inventory, vendor, abilities, NPC
   AI. DEBUG and above. Default operator view. Receives **WARN+ from
   every scope regardless of routing** — elevated severity always lands
   here so a real wire problem surfaces without dual-querying.
2. **`service.name = cimmeria-network`** — the high-noise wire-level
   index. DEBUG and INFO only: every `mercury.packet` event, every
   bundle decrypt + cell-arms dispatch from
   `cimmeria_base::base::connect_loop::*`, tick-sync heartbeats.
   Query this index when chasing wire-level issues; it never drowns the
   main view at normal severity.
3. **`service.name = cimmeria-trace`** — TRACE-level rows only (NA25).
   Every TRACE row an on-disk `logs/*.log` file keeps, the TRACE rows on
   any custom target `OTEL_FILTER` names (today that is chiefly
   `movement.navmesh` `advisory_off_mesh_accepted`, the record of where
   players walk off an advisory world's mesh), and the 1-in-N samples of
   the per-packet firehoses (`wire.sampled.*`). Query
   `service.name = 'cimmeria-trace'` when a question needs the rows that
   used to exist only in the log files: `Unhandled client message`,
   `Cell method before world entry -- ignored`, the `UDP_OUT ...` sends,
   the `AVATAR_UPDATE_EXPLICIT -> CellService` relay, ACK queueing.

No record lands in two indexes: the first two reject TRACE, the third
accepts only TRACE.

**Parity with the log files.** Whatever a `logs/*.log` file keeps also
reaches SigNoz, with two exceptions. The exporter's own transport crates
(`hyper`, `h2`, `tonic`, `tower`, `reqwest`, `opentelemetry`,
`tungstenite`) are never exported, because exporting them loops every
batch into the next. And the per-packet firehoses are in the files in
full but in SigNoz as a counted sample:

| Firehose (files) | Sample in SigNoz | Index | 1-in-N |
|---|---|---|---|
| `wire.firehose.decrypt`, `DECRYPT_OK` | `wire.sampled.decrypt`, `DECRYPT_OK (sampled)`, with `hex` | `cimmeria-trace` | 53 |
| `wire.firehose.udp_in`, `UDP_IN` | `wire.sampled.udp_in`, `UDP_IN (sampled)`, `len` only: no hex, because pre-login datagrams carry the login ticket | `cimmeria-trace` | 53 |
| `wire.firehose.aoi_position`, `AoI: entity position update` | `wire.out.avatar_update`, `UPDATE_AVATAR sent (sampled)` | `cimmeria-server` | 101 |

Every sample carries `sampled_1_in` and `suppressed` (occurrences since
the previous sample). The true count over a window is
`sum(1 + suppressed)`, or roughly `count() * sampled_1_in`.

Every hand-named `target: "…"` the server emits also reaches SigNoz at
the level it is emitted, enforced by a source scan
(`crates/server/src/logging/target_scan_tests.rs`). The one exception is
`launcher.key_dump`, which carries a client session key and stays on the
host.

Routing is by level plus the target predicate
`otel::is_network_noise_target` (see
[`crates/server/src/otel.rs`](../../crates/server/src/otel.rs)); the
filters and the routing table are in
[`crates/server/src/logging/filters.rs`](../../crates/server/src/logging/filters.rs),
and `crates/server/src/logging/parity_tests/` fails the build if a log
file gains a target SigNoz does not receive. The streams share one OTLP
endpoint + collector but three `SdkLoggerProvider`s (one per resource).

Schemas:

- `cimmeria-server` events follow the standard tracing field set
  (`entity_id`, `player_id`, `account_id`, `target`, etc.)
- `cimmeria-network` `mercury.packet` events come in two shapes, and no
  single event carries every field. Both are emitted by the
  instrumentation helpers in
  [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs):
  - **UDP** (`mercury_packet`, `instrumentation.rs:99-108`) —
    `dir`, `transport="udp"`, `seq`, `flags`, `len`, `peer`. No `msg_id`.
  - **TCP** (`unified_frame`, `instrumentation.rs:119-126`) —
    `dir`, `transport="tcp"`, `msg_id`, `len`. No `seq`, `flags`, or `peer`.

  Filter on `transport` before assuming a field is present; a query that
  groups by `msg_id` silently drops every UDP packet.

A previous iteration of the server also wrote logs to an Azure Cosmos
DB sink alongside the OTLP exporter. That sink was removed when SigNoz
became the single analytical store — the only telemetry sinks the
server runs today are the in-process file/broadcast layers and the
OTLP exporter.

## Architecture at a glance

```text
cimmeria-server (container `cimmeria`, or native)
   │
   ├── tracing-subscriber (in-proc)
   │     ├── console layer        → stdout
   │     ├── per-system log files → logs/*.log
   │     ├── BroadcastLayer       → admin WebSocket
   │     └── OpenTelemetryLayer   → OTLP gRPC otel-collector:4317 (over signoz-net)
   │                                       │
   │                                       ▼
   │                              signoz-otel-collector
   │                                       │
   │                                       ▼
   │                              signoz-clickhouse
   │                                       │
   │                                       ▼
   │                              signoz (UI + query API) :8080
```

Player telemetry does not take a separate path into SigNoz. Opted-in
launchers upload to the game server's login port
(`:8081/api/telemetry/*`), and the server replays those rows into the
same collector over `signoz-net` as `service.name = cimmeria-client`.

The OpenTelemetry layer is opt-in: if `OTEL_EXPORTER_OTLP_ENDPOINT` is
unset, the layer is never instantiated and the OTLP code path never
runs. This is the "off by default" stance — the integration only
activates when an operator explicitly points it at a collector.

## Colo deployment

The colo runs two Compose projects joined by the external Docker
network `signoz-net`:

- [`docker/compose.yml`](../../docker/compose.yml) — the `cimmeria`
  game server and `watchtower`, in `/opt/cimmeria`.
- [`docker/signoz/compose.yaml`](../../docker/signoz/compose.yaml) —
  SigNoz, in `/opt/cimmeria/signoz`. It is SigNoz's own single-node
  compose file, vendored at v0.125.1 with the config files it mounts
  copied beside it. The header of that file lists every Cimmeria
  change to upstream.

Setup, ports, `.env` values and day-to-day operation are in
[colo-deploy.md](colo-deploy.md). The short version:

```bash
docker network create signoz-net            # once per host
cd /opt/cimmeria/signoz
cp .env.example .env && $EDITOR .env        # SIGNOZ_JWT_SECRET at least
docker compose up -d
```

Allow about two minutes for the first boot: `init-clickhouse` downloads
a ClickHouse function from GitHub and the migrator creates the schema.
Then open the UI on `${SIGNOZ_UI_BIND}:8080` and create the admin
account. The first visitor creates it, so do it straight away.

### Verify the wire path

From the host, send one log record to the collector's HTTP receiver:

```bash
curl -sf -X POST http://127.0.0.1:4318/v1/logs \
  -H 'Content-Type: application/json' \
  -d '{"resourceLogs":[{"resource":{"attributes":[{"key":"service.name","value":{"stringValue":"cimmeria-smoke"}}]},"scopeLogs":[{"logRecords":[{"body":{"stringValue":"SigNoz wire path smoke"},"severityText":"INFO"}]}]}]}'
```

Use the `OTLP_BIND` address instead of `127.0.0.1` if you changed it.
Then in the SigNoz UI → Logs, filter `service.name = cimmeria-smoke`
and confirm the body appears within ~10s. For the game server itself,
`docker logs cimmeria 2>&1 | grep '\[otel\] Streaming'` shows the
endpoint it exports to.

## Local dev (running cimmeria-server natively, only SigNoz in Docker)

When developing locally with `cimmeria-server.exe` running natively
(not in Docker), run the same SigNoz stack the colo runs:

```bash
docker network create signoz-net
cd docker/signoz
cp .env.example .env        # set SIGNOZ_JWT_SECRET (openssl rand -hex 32)
docker compose up -d
```

The defaults publish OTLP on `localhost:4317` (gRPC) and
`localhost:4318` (HTTP), and the UI on `http://localhost:8080`, where
the first visit asks you to create the admin account. Then run the
server natively with the OTLP endpoint pointed at the collector:

```powershell
$env:OTEL_EXPORTER_OTLP_ENDPOINT = "http://localhost:4317"
$env:OTEL_SERVICE_NAME = "cimmeria-server"
.\cimmeria-server.exe
```

## Upgrading SigNoz

[`docker/signoz/`](../../docker/signoz/) is a vendored copy of
`deploy/docker/docker-compose.yaml` and the `deploy/common/` files it
mounts, from `github.com/SigNoz/signoz`. The images and the config
files are coupled; upgrade them together, in one PR:

1. Read the SigNoz release notes between the current version and the
   target for breaking changes and manual migration steps.
2. From a checkout of SigNoz at the target tag, carry the image tags
   in `deploy/docker/docker-compose.yaml` (`signoz/signoz`,
   `signoz/signoz-otel-collector`, `clickhouse/clickhouse-server`,
   `signoz/zookeeper`) and any service changes into
   `docker/signoz/compose.yaml`.
3. Re-copy `deploy/common/clickhouse/*.xml`,
   `deploy/common/signoz/otel-collector-opamp-config.yaml` and
   `deploy/docker/otel-collector-config.yaml`.
4. Re-apply the Cimmeria deltas listed in the header of
   `docker/signoz/compose.yaml`: `name: signoz`, the `./common` paths,
   the external `signoz-net`, the `SIGNOZ_UI_BIND` / `OTLP_BIND` port
   binds, the `SIGNOZ_JWT_SECRET` variable, and the
   `transform/claude-code-scrub` processor in the collector config. The
   processor must stay in the `logs` and `metrics` pipelines (see
   [claude-code-telemetry.md](claude-code-telemetry.md)). Update the
   version and commit noted in the header.
5. Test locally with the
   [Local dev](#local-dev-running-cimmeria-server-natively-only-signoz-in-docker)
   steps, including the wire-path check.
6. On the colo, copy the new `docker/signoz/` files into
   `/opt/cimmeria/signoz` and run `docker compose up -d` there. The
   named volumes (`signoz-clickhouse`, `signoz-sqlite`,
   `signoz-zookeeper-1`) persist, so data, dashboards and alert rules
   carry over. The game server keeps running while the collector
   restarts.

### Resource budget

SigNoz's footprint on the colo box (measured 2026-10-04, v0.125.1):

| Container | RAM | Disk |
|---|---|---|
| `signoz-clickhouse` | ~2.7 GB | ~37 GB after four months (`signoz-clickhouse` volume) |
| `signoz-zookeeper-1` | ~0.9 GB | ~0.3 GB |
| `signoz-otel-collector` | ~0.2 GB | — |
| `signoz` (UI + query API) | ~60 MB | a few MB (`signoz-sqlite`) |

Production scaling math lives in
[docs/architecture/observability.md](../architecture/observability.md).

### Retention

Set retention after first bring-up in the SigNoz UI's settings
(per-signal TTL, applied by ClickHouse). ClickHouse is the only part of
the host that grows without bound. Recommended starting points:

| Signal | Delete after |
|---|---|
| Traces | 14 days |
| Logs | 30 days |
| Metrics | 90 days |

Adjust upward if disk capacity allows — Mercury packet rows are the
most useful for retroactive forensics and benefit from longer
retention. Adjust downward if disk pressure becomes a concern.

### Alerts

SigNoz v0.125 runs its alert manager inside the `signoz` container
(`SIGNOZ_ALERTMANAGER_PROVIDER=signoz`); there is no separate
Alertmanager service or config file. Configure notification channels
(Slack, email, webhooks) and alert rules in the UI. They are stored in
the `signoz-sqlite` volume, not in the repo, so webhook URLs never
touch git.

### Security

- The UI and query API on `8080` have their own login. Session tokens
  are signed with `SIGNOZ_JWT_SECRET` from `docker/signoz/.env`.
  Upstream ships the literal `secret`, which is why the vendored
  compose refuses to start without a value. Publish `8080` on
  `127.0.0.1` (the default) or a LAN/VPN address via `SIGNOZ_UI_BIND`.
  As of 2026-10-04 the colo's network edge forwards `8080` from the
  internet; nothing a player runs needs it.
- The OTLP collector (`4317`/`4318`) accepts unauthenticated ingest
  from anyone who can reach it. The vendored compose publishes it on
  `OTLP_BIND`, default `127.0.0.1`; set a LAN/VPN address only for
  exporters on that network. Never forward `4317` or `4318` from the
  internet. The game container doesn't need them published: it
  reaches `otel-collector:4317` over `signoz-net`.
- ClickHouse runs with an empty `default` user password and is not
  published; only containers on `signoz-net` reach it. If you ever
  publish `9000` or `8123`, set a password in
  `docker/signoz/common/clickhouse/users.xml` first.

## Operational notes

### Disabling the integration

Two ways to fully disable SigNoz ingestion without removing code:

1. **Unset the endpoint.** Set `OTEL_EXPORTER_OTLP_ENDPOINT=` (empty)
   in `/opt/cimmeria/.env` and run `docker compose up -d`. The exporter
   never initialises, the OTLP layer is omitted from the subscriber
   stack, and the container skips its wait for the collector. Zero
   cost.
2. **Stop SigNoz.** `docker compose down` in `/opt/cimmeria/signoz`.
   The game server logs export errors but keeps running — exporter
   failure is non-fatal. Without the first step, each container start
   waits up to `OTEL_WAIT_TIMEOUT` (120 s) for the collector before
   starting without it.

### Finding navmesh holes from telemetry

The September 2026 Castle_CellBlock navmesh rebuild started from
SigNoz showing real players being snapped back in places the 2013 mesh
did not cover. That investigation was manual; this is the repeatable
version.

**1. Which worlds are rejecting players, and why.**

```text
scope_name = 'movement.validation'
  AND reason = 'navmesh'
groupBy = world, gate
```

`gate` splits the four failures apart. Read it as:

| Dominant `gate` | What it means | Action |
|---|---|---|
| `no_poly_in_extents` | Players are standing where the mesh has **nothing at all** | Rebuild / re-bake that world's `.nav`. This is the mesh-hole signature |
| `horizontal` | Players are just off the edge of the walkable surface | Mesh coverage is too tight against geometry — usually also a rebuild |
| `below_surface` | Players are clipped *under* the floor | Not a mesh-coverage problem. Look at authoritative writes (content teleports, respawners, persisted positions) for that world |
| `above_jump_tolerance` | Players are higher above the surface than the client's jump physics can produce | Either a fly-hack or a tolerance that needs calibration; check `nav_dy` for how far past 4.0 it goes |

**2. Where, precisely.** Add `client_x`, `client_y`, `client_z` as
columns and narrow to one world. Clusters of coordinates at the same
`gate` are the actual holes; a scatter of one-offs is usually a single
misbehaving client.

**3. Confirm which mesh build.** Every reject carries `navmesh_hash`
(8 hex digits). Join it to the load line:

```text
scope_name = 'movement.navmesh' AND event = 'navmesh_loaded'
```

That row has the full hash, `path`, `file_bytes`, `polys`, `verts` and
the agent parameters. If rejects span two hashes, the mesh was
redeployed inside your time window and the two halves must be read
separately — a rebuild that fixed the hole looks like "the problem
stopped" only when you can see the hash change underneath it.

**4. Check the positive space too.**

```text
scope_name = 'movement.position_sample'
groupBy = world
```

with `x` / `z` as columns gives the walked-surface map: where players
successfully go. A region with rejects and **no** accepted samples
nearby is unreachable, not merely awkward — which distinguishes "the
mesh has a hole" from "the mesh is fine and one player is cheating".

**5. NPCs see holes before players do.**

```text
scope_name = 'npc_ai.path_fail'
groupBy = world, state, reason
```

`reason = no_mesh` means that world has no `.nav` at all — a content
gap, not a hole. `reason = no_path` with a mesh loaded means the mesh
is split or holed between the NPC and its destination. Because NPCs
patrol fixed routes, a steady `patrol` / `no_path` count in one world
localises a break without waiting for a player to find it.

**Reading the counts.** Both `movement.validation_reject` and
`npc_ai.path_fail` are **throttled per entity** — one row then at most
one per window, with `suppressed = N` naming the elided rows. Do not
read row counts as occurrence counts. For rates use the metrics,
`movement_validation_rejects_total{world, gate}` and
`npc_path_fail_total{world, state, reason}`, which are incremented on
every occurrence including suppressed ones. A single row with a large
`suppressed` is one entity stuck, not a widespread problem; many rows
from many `entity_id`s is.

### Backfilling missed data

There is no backfill story — events not shipped at the time they
happen are not in SigNoz. The disk-side log files in `logs/*.log`
remain the source of truth for retroactive deep-dives.

### Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| SigNoz UI loads but "no data" | OTLP collector unreachable from `cimmeria-server` | `docker logs cimmeria` should contain `[otel] Streaming to http://otel-collector:4317`. `docker network inspect signoz-net` should list both `cimmeria` and `signoz-otel-collector`. |
| `network signoz-net declared as external, but could not be found` | The shared network was never created | `docker network create signoz-net`, then `docker compose up -d` again |
| Wire-path check succeeds but server data missing | Subscriber filter dropped events, or you are querying the wrong index | TRACE rows are only in `service.name = 'cimmeria-trace'`; wire DEBUG/INFO only in `cimmeria-network`. Otherwise check the filters in [`crates/server/src/logging/filters.rs`](../../crates/server/src/logging/filters.rs) |
| ClickHouse OOM | `max_memory_usage` too low for an ingestion burst | Raise `max_memory_usage` in the `default` profile of `docker/signoz/common/clickhouse/users.xml`, copy it to the host, then `docker restart signoz-clickhouse` |
| `signoz-otel-collector` keeps restarting | Schema migrations unfinished or failed (`migrate sync check` fails) | `docker logs signoz-telemetrystore-migrator`; it must exit 0 |
| UI unreachable right after `up` | Cold start (~2 min on first boot) | Wait, then `docker logs signoz --tail 50` |
| Server logs say "[otel] Exporter init failed" | Collector address misconfigured | Verify `OTEL_EXPORTER_OTLP_ENDPOINT` and that `otel-collector:4317` is reachable on `signoz-net` |
