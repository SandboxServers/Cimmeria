# Server observability — design and tool choice

**Status:** Accepted (2026-05-25)
**Confidence:** High

## Context

Cimmeria emits two telemetry streams at runtime, and both now converge
on the same analytical store:

1. **Server-side logs and Mercury packets.** Every `tracing::*` call
   in the server crates plus per-packet wire-level events recorded
   via [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs).
   Volume scales with concurrent player count; at dev volumes
   (single-digit players) it's ~3–5 KB/sec uncompressed.

2. **Launcher dev-session telemetry.** Client-side session metadata,
   parsed Atera client logs, debug-channel events, key dumps, and the
   end-of-session zipped log bundle. Uploaded by the launcher to
   cimmeria-server itself, replayed through the same `tracing`
   subscriber, and shipped to the same SigNoz store. Documented in
   [dev-session-telemetry.md](dev-session-telemetry.md) and
   [docs/operations/telemetry.md](../operations/telemetry.md).

Both streams need a sink optimised for analytical retrieval — the
downstream consumer is an LLM (the Cimmeria-MCP server, eventually
augmented by Claude in interactive dev sessions). Keeping them in one
store means one query language, one access path, one retention
policy, one place to look.

## Decision

For stream (1):

- **Storage backend: SigNoz** (Apache 2.0, ClickHouse-backed, OTLP-native).
- **Transport: OpenTelemetry Protocol** (gRPC :4317, with HTTP :4318 fallback).
- **Self-hosted** on the colo Docker host. SigNoz upstream compose is
  pulled in as a git submodule at `external/signoz/`; Cimmeria layers
  a small overlay on top
  ([`docker/compose.signoz.yml`](../../docker/compose.signoz.yml)).
- **Remote access via Cloudflare Tunnel + Cloudflare Access**
  ([`docker/compose.signoz-tunnel.yml`](../../docker/compose.signoz-tunnel.yml)) —
  service tokens for machine clients, identity providers for browsers,
  no inbound firewall ports.

For stream (2): the launcher now uploads to cimmeria-server's own
`/api/telemetry/upload-{chunk,bundle}` endpoints. The server validates
the HMAC token (same dev-session flow as before), replays each event
through `tracing::*`, and the OTLP layer ships to SigNoz alongside
the server's own events. The Cosmos-backed Cimmeria-MCP write path is
retired.

The previous Cosmos DB log layer is removed entirely — single source
of truth for the analytical store, no parallel sinks to keep in sync.

## Alternatives considered

### A. Keep using Cosmos DB for server logs

Cosmos is what the launcher uses today, so reusing it for server logs
would have been the simplest path code-wise.

**Why rejected:**

- **Cost.** At ingestion rates of 3–5 KB/sec we'd burn ~$30–150/mo on
  Cosmos RU/s for what is fundamentally append-only timeseries data
  with rare reads. Azure Storage Blob would be $0.30/mo for the same
  bytes — but that's just storage, not query.
- **Query model.** Cosmos's SQL-API is row-oriented and indexed for
  point-lookups. Time-window aggregations across millions of events
  ("show me all incoming packets between 14:00 and 14:05 grouped by
  msg_id") are expensive and slow.
- **LLM retrieval ergonomics.** Cosmos returns JSON documents one at
  a time. An LLM doing "summarise the last hour of packet activity"
  benefits hugely from columnar pushdown — get back just the columns
  it cares about, pre-aggregated.

### B. ClickHouse raw

We could have run ClickHouse directly and built a custom OTLP-to-CH
bridge.

**Why rejected:** SigNoz IS ClickHouse + an OTLP collector + a query
UI + alerting + service maps, all pre-wired. Reinventing those four
pieces for no gain.

### C. OpenObserve

OpenObserve is a similar OTLP-native observability platform,
single-binary, written in Rust, with cheaper storage (object-store
backed).

**Why rejected:** Smaller ecosystem and community than SigNoz. SigNoz
already has dashboards for common Rust + tracing patterns; we'd build
those ourselves on OpenObserve. Re-evaluate in 12 months — if
OpenObserve's plugin ecosystem catches up, the OTLP-native
architecture means switching is a docker-compose change, not a
recoding effort.

### D. Elastic / OpenSearch

Mature, but resource-hungry (Java heap), and the licensing situation
post-AWS-fork is a yearly headache.

**Why rejected:** Doesn't fit the "easy-to-maintain on a single colo
box" constraint.

## Why OTLP at the wire

The choice of OTLP as the transport (independent of "SigNoz as the
backend") is the load-bearing decision here. OTLP is:

- **The standard.** OpenTelemetry is the consensus winner for
  language-agnostic telemetry, with stable SDKs in Rust, Go, Python,
  C#, etc. Cimmeria-MCP queries land naturally on the same backend.
- **Vendor-neutral.** If we ever want to bail on SigNoz, every other
  observability vendor accepts OTLP — switching is a collector-config
  change, not a re-instrumentation effort.
- **Native to the Rust tracing ecosystem.** `tracing-opentelemetry`
  is mature; we hook our existing `tracing::*` calls directly without
  rewriting them.

## Implementation

### Wire path

1. `tracing::info!(target: "mercury.packet", ...)` (or any other
   tracing macro) emits an event. Producers include:
   - Server crates (every existing `tracing::*` call).
   - Mercury wire seams (UDP `Channel::{send,receive}_packet`, TCP
     `UnifiedCodec::{encode,decode}`).
   - Launcher uploads — replayed through tracing by
     `crates/admin-api/src/routes/telemetry.rs` after HMAC
     verification of the dev-session token.
2. The OpenTelemetry layer
   ([`crates/server/src/otel.rs`](../../crates/server/src/otel.rs))
   serialises it to an OTLP Span and pushes onto a batch channel.
3. A background tokio task drains the channel and ships batches to
   `otel-collector:4317` via gRPC.
4. The collector forwards to ClickHouse, indexed by service.name +
   timestamp + body fields.
5. SigNoz frontend queries ClickHouse for dashboards / search.

### Wire seams

Mercury packet recording happens at two callsites, both routing
through helpers in
[`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs):

- **UDP (client ↔ server):** `Channel::send_packet` and
  `Channel::receive_packet` in
  [`crates/mercury/src/channel/mod.rs`](../../crates/mercury/src/channel/mod.rs).
- **TCP (inter-service):** `UnifiedCodec::encode` and `decode` in
  [`crates/mercury/src/unified.rs`](../../crates/mercury/src/unified.rs).

Centralising the field schema in the instrumentation helpers means
the downstream "show me packets" query has a single stable shape to
filter on (`target = "mercury.packet"`), regardless of which seam
emitted the event.

### Cost on the hot path

`tracing::info!` with no subscriber attached: a single atomic load +
branch. With the OTLP layer attached: serialise to OTLP wire format +
push onto an in-process mpmc channel. The actual network send is on
a background task. Net cost per packet: handful of nanoseconds.

Sampling is `always_on` by default — Mercury packet rate is the
analytical surface we care about, sampling defeats the purpose. If
volume becomes an issue, the lever is `OTEL_TRACES_SAMPLER` (set per
deployment via the compose env var), not source code changes.

## Cimmeria-MCP integration

The Cimmeria-MCP C# Azure Function repo is separate from this one.
The integration plan from this side:

1. **Surface area added to MCP:** two new tool families.
   - `signoz_query_logs(query, time_range)` — accepts a SigNoz query
     URL (their PromQL-flavoured DSL) and returns structured rows.
   - `signoz_query_packets(filters, time_range)` — typed convenience
     wrapper that constructs the SigNoz query from a Mercury-specific
     `PacketFilters { direction?, transport?, msg_id?, peer?, seq_range? }`
     struct, so the LLM doesn't have to remember the field names.
2. **Auth path:** Cimmeria-MCP holds a Cloudflare Access service token
   pair as env config (see
   [signoz-remote-access.md](../operations/signoz-remote-access.md)).
   Every request to SigNoz attaches the headers; Cloudflare validates
   at the edge.
3. **Where it runs:** since Cimmeria-MCP is Azure-hosted and SigNoz
   is on the colo, the latency budget is ~30–80ms cross-region. That's
   fine for LLM-mediated queries (LLM inference dominates) — but not
   suitable for real-time alerting from MCP. Alerting (if/when we
   want it) should run colo-local against SigNoz's Alertmanager.

## Consequences

### Positive

- Server logs and Mercury packets become queryable analytically, not
  just grep-able from disk.
- LLM tooling (Cimmeria-MCP, Claude during dev sessions) has a
  structured surface to ask "what happened in the last hour" against.
- The OTLP standard means we can swap SigNoz for any other vendor
  with a collector-config change.
- The cosmos_log path is unchanged — we did not break existing flows.

### Negative

- Operators have a new docker stack to keep alive on the colo
  (SigNoz's ~6 services). The submodule pattern keeps upgrades cheap
  but adds a tier of containers to monitor.
- Cloudflare Tunnel introduces vendor coupling for remote access.
  Tradeoff is accepted for the auth-at-edge story; pivoting to
  Tailscale is a one-file overlay swap.
- We now ship potentially-sensitive logs through Cloudflare's edge.
  Logs are tunneled (TLS the whole way) but Cloudflare technically
  sees the metadata. Mitigation: SigNoz UI runs at HTTP locally; the
  TLS terminates at Cloudflare which then re-encrypts on the tunnel
  hop. Don't put production-secrets-in-plaintext in tracing fields.

### Neutral

- Per-host disk usage grows by ClickHouse's footprint (~10 GB/month
  at current volume after compression). Retention TTL is an operator
  decision per
  [signoz-deployment.md](../operations/signoz-deployment.md#retention).

## References

- Deployment runbook: [signoz-deployment.md](../operations/signoz-deployment.md)
- Remote access runbook: [signoz-remote-access.md](../operations/signoz-remote-access.md)
- Instrumentation helpers: [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs)
- OTLP exporter: [`crates/server/src/otel.rs`](../../crates/server/src/otel.rs)
- Launcher ingest endpoint: [`crates/admin-api/src/routes/telemetry.rs`](../../crates/admin-api/src/routes/telemetry.rs)
- Launcher telemetry pipeline (dev-session flow + secret rotation): [dev-session-telemetry.md](dev-session-telemetry.md), [docs/operations/telemetry.md](../operations/telemetry.md)
