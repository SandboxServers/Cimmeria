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

Two streams converge into the same ClickHouse-backed store:

1. **Server logs.** Every `tracing::*` macro call in `cimmeria-server`
   and its workspace crates. Filtered to `info+` for general modules
   and `debug+` for `cimmeria_services` / `cimmeria_mercury`. Targets
   matching `mercury.packet` are routed through unconditionally.
2. **Mercury packets.** Every UDP packet (client ↔ server) and every
   Unified TCP frame (Auth ↔ Base ↔ Cell) is recorded via the
   instrumentation helpers in
   [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs).
   Schema: `target = "mercury.packet"`, fields `dir`, `transport`,
   `seq`, `flags`, `msg_id`, `len`, `peer`.

The Cosmos DB sink ([`crates/server/src/cosmos_log.rs`](../../crates/server/src/cosmos_log.rs))
keeps running in parallel and is unaffected. The two sinks are
independent — disabling one does not affect the other.

## Architecture at a glance

```text
cimmeria-server
   │
   ├── tracing-subscriber (in-proc)
   │     ├── console layer        → stdout
   │     ├── per-system log files → logs/*.log
   │     ├── BroadcastLayer       → admin WebSocket
   │     ├── CosmosLogLayer       → Azure Cosmos (existing, optional)
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

## One-time setup: populate `external/signoz`

The single-entry compose file references the upstream SigNoz compose
by relative path under `external/signoz/`. That tree is **not** in
git — `setup.ps1` populates it by cloning the SigNoz repo at a
pinned tag.

```powershell
pwsh setup.ps1 -SkipBuild -NoLaunch
```

The setup step is idempotent: if `external/signoz/.../docker-compose.yaml`
already exists, the clone is skipped. To upgrade SigNoz, bump the
pinned tag inside
[`Install-CimmeriaDependencies.ps1`](../../bootstrap/CimmeriaBootstrap/Public/Install-CimmeriaDependencies.ps1)
(search for `$signozTag`), `rm -rf external/signoz`, re-run setup.

If git is unavailable in your shell, clone manually:

```bash
git clone --depth=1 --branch v0.55.0 \
  https://github.com/SigNoz/signoz external/signoz
```

## Colo deployment (single command)

The colo runs the full stack — cimmeria-server (watchtower-managed),
the SigNoz observability stack, and optionally the Cloudflare Tunnel
for remote access — under one Compose project entry point at
[`docker/compose.yml`](../../docker/compose.yml). That file uses
Compose's `include:` directive to pull in three overlays plus the
upstream SigNoz compose so callers don't have to remember the
correct chain of `-f` flags.

```bash
# Standard bring-up (no remote-access tunnel):
docker compose -f docker/compose.yml up -d

# Bring up with the Cloudflare Tunnel for remote browser + MCP access:
docker compose -f docker/compose.yml --profile tunnel up -d
```

Wait ~90 seconds for ClickHouse to finish initialising
(`docker compose -f docker/compose.yml logs clickhouse | grep "Ready"`).
The SigNoz UI is then at `http://localhost:3301`. With
`--profile tunnel` the UI is also reachable via your Cloudflare
domain — see [signoz-remote-access.md](signoz-remote-access.md).

### Verify the wire path

The optional `--profile smoke` brings up a one-shot curl container
that fires a synthetic OTLP event so you can confirm the collector
is reachable before the game server boots:

```bash
docker compose -f docker/compose.yml --profile smoke up otel-smoke
```

Then in the SigNoz UI → Logs → filter `service.name = cimmeria-smoke`
and confirm the "SigNoz wire path smoke" body appears within ~10s.

## Local dev (no Docker, just SigNoz)

When running `cimmeria-server.exe` natively (not in Docker), you only
need the SigNoz stack itself in containers. Skip the top-level entry
point and bring up just the upstream compose:

```bash
docker compose \
  -f external/signoz/deploy/docker/clickhouse-setup/docker-compose.yaml \
  up -d
```

Then run the server natively with the OTLP endpoint pointed at the
collector:

```powershell
$env:OTEL_EXPORTER_OTLP_ENDPOINT = "http://localhost:4317"
$env:OTEL_SERVICE_NAME = "cimmeria-server"
.\cimmeria-server.exe
```

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

ClickHouse defaults to indefinite retention. To cap storage growth,
add a TTL policy in `external/signoz/deploy/docker/clickhouse-setup/clickhouse-config.xml`
on the relevant tables. Default suggested:

- `signoz_logs` → 30 days
- `signoz_traces` → 14 days

This is an operator decision, not enforced by the Cimmeria repo.

## Operational notes

### Updating SigNoz

```bash
git submodule update --remote external/signoz
docker compose -f external/signoz/.../docker-compose.yaml \
               -f docker/compose.signoz.yml pull
docker compose ... up -d
```

A `git diff external/signoz` shows the upstream changes by submodule
pointer. Commit the bump like any other change.

### Disabling the integration

Two ways to fully disable SigNoz ingestion without removing code:

1. **Unset the env var.** Remove `OTEL_EXPORTER_OTLP_ENDPOINT` from
   `cimmeria-server`'s environment. The exporter never initialises;
   the OTLP layer is omitted from the subscriber stack. Zero cost.
2. **Take down the stack.** `docker compose ... -f compose.signoz.yml
   down`. The exporter will log connection-refused errors but the
   server keeps running fine — exporter failure is non-fatal.

### Backfilling missed data

There is no backfill story — events not shipped at the time they
happen are not in SigNoz. The disk-side log files in `logs/*.log`
remain the source of truth for retroactive deep-dives.

### Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| SigNoz UI loads but "no data" | OTLP collector unreachable from `cimmeria-server` | Check container is on `signoz-net` network (`docker inspect cimmeria-server`) |
| `otel-smoke` succeeds but server data missing | Subscriber filter dropped events | Check `init_logging` in [`crates/server/src/main.rs`](../../crates/server/src/main.rs) — OTel layer's EnvFilter |
| ClickHouse OOM | Default `max_memory_usage` too low for ingestion burst | Edit `clickhouse-config.xml`, restart `clickhouse` container |
| Tunnel up, browser shows 502 | Frontend not yet ready (~90s cold start) | Wait, then `docker compose logs frontend` |
| Server logs say "[otel] Exporter init failed" | Collector address misconfigured | Verify `OTEL_EXPORTER_OTLP_ENDPOINT` and that `:4317` is reachable |
