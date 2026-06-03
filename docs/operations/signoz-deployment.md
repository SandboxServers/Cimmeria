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
across two SigNoz services** so wire-level volume doesn't drown the
high-signal events in the operator's primary triage view:

1. **`service.name = cimmeria-server`** — the high-signal index. Auth,
   content chains, combat, missions, inventory, vendor, abilities.
   Default operator view. Receives **WARN+ from every scope regardless
   of routing** — elevated severity always lands here so a real wire
   problem surfaces without dual-querying.
2. **`service.name = cimmeria-network`** — the high-noise wire-level
   index. Every `mercury.packet` event, every bundle decrypt + cell-
   arms dispatch from `cimmeria_services::base::connect_loop::*`,
   tick-sync heartbeats. Query this index when chasing wire-level
   issues; it never drowns the main view at normal severity.

Routing is target-based via `otel::is_network_noise_target` (see
[`crates/server/src/otel.rs`](../../crates/server/src/otel.rs)) composed
with a severity carve-out in
[`crates/server/src/main.rs`'s `init_logging`](../../crates/server/src/main.rs).
The two streams share one OTLP endpoint + collector but two
`SdkLoggerProvider`s (one per resource).

Schemas:

- `cimmeria-server` events follow the standard tracing field set
  (`entity_id`, `player_id`, `account_id`, `target`, etc.)
- `cimmeria-network` `mercury.packet` events: `dir`, `transport`,
  `seq`, `flags`, `msg_id`, `len`, `peer`. Recorded via the
  instrumentation helpers in
  [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs).

A previous iteration of the server also wrote logs to an Azure Cosmos
DB sink alongside the OTLP exporter. That sink was removed when SigNoz
became the single analytical store — the only telemetry sinks the
server runs today are the in-process file/broadcast layers and the
OTLP exporter.

## Architecture at a glance

```text
cimmeria-server
   │
   ├── tracing-subscriber (in-proc)
   │     ├── console layer        → stdout
   │     ├── per-system log files → logs/*.log
   │     ├── BroadcastLayer       → admin WebSocket
   │     └── OpenTelemetryLayer   → OTLP gRPC :4317
   │                                       │
   │                                       ▼
   │                              otel-collector (SigNoz)
   │                                       │
   │                                       ▼
   │                              ClickHouse
   │                                       │
   │                                       ▼
   │                              SigNoz frontend :3301
```

The OpenTelemetry layer is opt-in: if `OTEL_EXPORTER_OTLP_ENDPOINT` is
unset, the layer is never instantiated and the OTLP code path never
runs. This is the "off by default" stance — the integration only
activates when an operator explicitly points it at a collector.

## Colo deployment (single file, single command)

[`docker/compose.yml`](../../docker/compose.yml) is fully
self-contained. The whole deployment unit is that one file. Copy it
to the colo box and bring it up — no companion config files, no
external repos, no setup script:

```bash
scp docker/compose.yml colo:/opt/cimmeria/
ssh colo "cd /opt/cimmeria && docker compose -f compose.yml up -d"
```

That single file contains:

- The `cimmeria` + `watchtower` services (game server with auto-update).
- The full vendored SigNoz stack (zookeeper-1, clickhouse,
  otel-collector-migrator, otel-collector, query-service,
  alertmanager, frontend) — pinned at SigNoz v0.55.0.
- Profile-gated `cloudflared` (Cloudflare Tunnel) and `otel-smoke`
  (wire-path verifier).
- All SigNoz config files (users.xml, cluster.xml, otel-collector
  config, prometheus.yml, alertmanager.yml, nginx-config.conf,
  alerts.yml) inlined as Docker Compose `configs:` blocks.

Profile flags:

```bash
docker compose -f compose.yml up -d                          # core: game + signoz
docker compose -f compose.yml --profile tunnel up -d         # + cloudflare tunnel
docker compose -f compose.yml --profile smoke up otel-smoke  # wire-path verify
```

Wait ~90 seconds for ClickHouse to finish initialising
(`docker compose -f compose.yml logs clickhouse | grep "Ready"`).
SigNoz UI is then at `http://<colo-host>:3301`. With
`--profile tunnel` the UI is also reachable via your Cloudflare
domain — see [signoz-remote-access.md](signoz-remote-access.md).

### Verify the wire path

```bash
docker compose -f compose.yml --profile smoke up otel-smoke
```

Then in the SigNoz UI → Logs → filter `service.name = cimmeria-smoke`
and confirm the "SigNoz wire path smoke" body appears within ~10s.

## Local dev (running cimmeria-server natively, only SigNoz in Docker)

When developing locally with `cimmeria-server.exe` running natively
(not in Docker), bring up only the SigNoz half of the same file by
listing the SigNoz service names:

```bash
docker compose -f docker/compose.yml up -d \
  zookeeper-1 clickhouse otel-collector-migrator \
  otel-collector query-service alertmanager frontend
```

Then run the server natively with the OTLP endpoint pointed at the
exposed collector:

```powershell
$env:OTEL_EXPORTER_OTLP_ENDPOINT = "http://localhost:4317"
$env:OTEL_SERVICE_NAME = "cimmeria-server"
.\cimmeria-server.exe
```

## Upgrading SigNoz

The vendored config sections at the bottom of `docker/compose.yml`
correspond to a pinned SigNoz release (currently v0.55.0). To
upgrade:

1. Bump the image tags in the services block (`signoz/query-service`,
   `signoz/frontend`, `signoz/signoz-otel-collector`,
   `signoz/signoz-schema-migrator`, `signoz/alertmanager`).
2. Re-vendor the `configs:` content from a fresh clone of
   `github.com/SigNoz/signoz/deploy/docker/clickhouse-setup/` and
   `deploy/docker/common/nginx-config.conf` at the new tag.
3. Commit + push. The next watchtower poll picks up the cimmeria
   image; redeploying `compose.yml` swaps the SigNoz tags.

The SigNoz images and configs are tightly coupled — bump them
together, not separately.

### Resource budget

SigNoz's footprint on the colo box:

| Container | RAM | Disk (steady-state) |
|---|---|---|
| ClickHouse | ~1.5 GB | ~10 GB/month at current packet rate |
| Query Service | ~150 MB | — |
| Alertmanager | ~50 MB | — |
| Frontend | ~30 MB | — |
| OTel Collector | ~80 MB | — |

ClickHouse compression on the JSONL packet stream runs ~10× —
3–5 KB/sec of structured tracing events compresses to ~0.3–0.5 KB/sec
on disk. The 10 GB/month figure assumes the current dev traffic
volume (single-digit concurrent players). Production scaling math
lives in [docs/architecture/observability.md](../architecture/observability.md).

### Retention

ClickHouse defaults to **indefinite** retention, which will eventually
fill the disk on a long-lived colo box. Recommended defaults to
configure after first bring-up via the SigNoz UI's *Settings →
Retention* page (per-signal TTL, applied via ClickHouse `MODIFY TTL`):

| Signal | Cold storage (S3/move) | Delete |
|---|---|---|
| Traces | 7 days | 14 days |
| Logs | 14 days | 30 days |
| Metrics | 30 days | 90 days |

Adjust upward if disk capacity allows — Mercury packet rows are the
most useful for retroactive forensics and benefit from longer
retention. Adjust downward (or wire up S3 archival) if disk pressure
becomes a concern.

### Alert receivers

The vendored `alertmanager-config` ships with a single `null` receiver
— alerts are accepted by Alertmanager and discarded silently. Before
relying on alerts, edit the `alertmanager-config` block in your copy
of `compose.yml` and add a real receiver (Slack webhook, email SMTP,
PagerDuty, etc.). Do not commit your webhook URL back to the repo;
keep operator credentials in your colo-local copy only.

### Security

- SigNoz UI on port 3301 has no built-in auth. Use the
  `--profile tunnel` Cloudflare Tunnel (see
  [signoz-remote-access.md](signoz-remote-access.md)) or restrict to
  LAN/VPN access. Do not publish 3301 to the public internet.
- The OTLP collector ports (4317/4318) bind to `127.0.0.1` by default
  via the `OTLP_BIND` interpolation in `compose.yml`. Override to a
  specific LAN IP only behind a firewall — the collector accepts
  unauthenticated ingest from any reachable client.
- ClickHouse runs with an empty `default` user password. The DB is
  only reachable on the compose internal network — if you ever expose
  port 9000 to a host network, set a password in the
  `clickhouse-users` config block first.

## Operational notes

### Updating SigNoz

The SigNoz stack is **vendored** into `docker/compose.yml`. Upgrading
is a two-part change:

1. Bump the image tags in the services block at the top of
   `docker/compose.yml` (`signoz/query-service`, `signoz/frontend`,
   `signoz/signoz-otel-collector`, `signoz/signoz-schema-migrator`,
   `signoz/alertmanager`).
2. Re-vendor the `configs:` blocks at the bottom of `compose.yml`
   from a fresh clone of
   `github.com/SigNoz/signoz/deploy/docker/clickhouse-setup/` (and
   `deploy/docker/common/nginx-config.conf`) at the new tag.

Bump them together, in one commit, with a smoke test of the OTLP
path. The configs are tightly coupled to the image versions — image
bumps without config re-vendoring can break silently.

### Disabling the integration

Two ways to fully disable SigNoz ingestion without removing code:

1. **Unset the env var.** Set `OTEL_EXPORTER_OTLP_ENDPOINT=""` in your
   environment override before `docker compose up`. The exporter
   never initialises; the OTLP layer is omitted from the subscriber
   stack. Zero cost.
2. **Take down the SigNoz services.** `docker compose -f compose.yml
   stop clickhouse otel-collector query-service alertmanager frontend
   zookeeper-1`. The exporter will log connection-refused errors but
   the game server keeps running fine — exporter failure is non-fatal.

### Backfilling missed data

There is no backfill story — events not shipped at the time they
happen are not in SigNoz. The disk-side log files in `logs/*.log`
remain the source of truth for retroactive deep-dives.

### Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| SigNoz UI loads but "no data" | OTLP collector unreachable from `cimmeria-server` | Verify both containers are in the same compose project (default network). `docker compose -f compose.yml ps` should show all 9 services. |
| `otel-smoke` succeeds but server data missing | Subscriber filter dropped events | Check `init_logging` in [`crates/server/src/main.rs`](../../crates/server/src/main.rs) — OTel layer's EnvFilter |
| ClickHouse OOM | Default `max_memory_usage` too low for ingestion burst | Edit the `clickhouse-users` `configs:` block in `compose.yml` (raise `max_memory_usage` in the `default` profile), restart the `clickhouse` container |
| Tunnel up, browser shows 502 | Frontend not yet ready (~90s cold start) | Wait, then `docker compose -f compose.yml logs frontend` |
| Server logs say "[otel] Exporter init failed" | Collector address misconfigured | Verify `OTEL_EXPORTER_OTLP_ENDPOINT` and that `:4317` is reachable |
