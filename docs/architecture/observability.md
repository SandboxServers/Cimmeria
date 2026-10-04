# Server observability — design and tool choice

**Status:** Accepted (2026-05-25)
**Last updated:** 2026-10-03 (target catalog moved to [observability-target-catalog.md](observability-target-catalog.md))
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
- **Self-hosted** on the colo Docker host. The entire deployment —
  cimmeria-server, watchtower, the full SigNoz stack (ZooKeeper +
  ClickHouse + OTel collector + query service + alertmanager +
  frontend), and all SigNoz config files — lives in a single
  self-contained [`docker/compose.yml`](../../docker/compose.yml).
  No external repos, no submodules, no companion files: ship one
  file, run `docker compose up -d`. SigNoz config files are inlined
  as Docker Compose `configs:` blocks; upgrades are a manual
  re-vendor (acceptable trade for true single-file deployability).
- **Remote access via Cloudflare Tunnel + Cloudflare Access**
  (the `cloudflared` service in the same compose file, guarded by
  `profiles: [tunnel]`) — service tokens for machine clients,
  identity providers for browsers, no inbound firewall ports.
  Optional; skip the profile flag if you'd rather open the SigNoz UI
  port directly behind a VPN or LAN gate.

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
     `crates/admin-api/src/routes/telemetry/` after HMAC
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

### Log indexes and parity with the log files

Owner decision (2026-09-25, NA25): whatever the server writes to
`logs/*.log` must also be available in SigNoz. The OTLP log signal is
split across four providers, each its own SigNoz service, and every
record lands in exactly one of them:

| Level | `service.name` | Filter |
|---|---|---|
| ERROR, WARN | `cimmeria-server` | `OTEL_FILTER` |
| INFO, DEBUG | `cimmeria-server`, or `cimmeria-network` for the scopes `otel::is_network_noise_target` names | `OTEL_FILTER` |
| TRACE | `cimmeria-trace` | derived, see below |
| every level, client-side replays only | `cimmeria-client` | `otel::is_client_target` |

**The client index (2026-09-29).** What the players' machines upload,
replayed by the telemetry ingest (`/api/telemetry/upload-*`, see
[dev-session-telemetry.md](dev-session-telemetry.md)), is data about the
client, not the server, so it has a service of its own and the other three
reject it at every level. The targets are `otel::CLIENT_TARGETS`:
`client.native` (the injected `cimmeria-client-telemetry` DLL's events) and
the game-log lines the launcher tails (`launcher.client_log`,
`launcher.debug_log`, `launcher.session_meta`). The ingest's own account
of an upload (`launcher.ingest`, `launcher.bundle`) and the mint rows stay
in `cimmeria-server`; `launcher.key_dump` stays `off`. Client rows ship at
every level the client sent, TRACE included: the DLL already throttled and
paid for the upload, and a client WARN is not a server problem.

The `cimmeria-client` resource carries `cimmeria.source = client`,
`deployment.environment` and `cimmeria.deploy_env`, and names the
ingesting server as `cimmeria.ingest_host` / `cimmeria.ingest_version`
rather than `host.name` / `service.version`, which a reader would take for
the player's machine. A resource is process-wide, so everything
per-session is a record attribute instead:

| Attribute | Meaning |
|---|---|
| `session_id`, `install_id` | The dev-session token's `sid` and `sub` claims |
| `cimmeria.session_kind` | `lab` (a Live Research Lab session) or `player`, from the signed `kind` claim |
| `lab` | `true` for a lab session |
| `ts_ms`, `seq` | The uploader's clock and sequence number |
| `client_target` | The DLL's event name (`client.lua.pcall`); also the log body |
| `client_level` | The DLL's level string, kept when it is not one the server knows |
| `class_id` + `class_name`, `method_index` + `method_name`, `msg_id` + `msg_name` | Lifted from the DLL's `fields` bag (`type_id` becomes `class_id`) and named from the NT-30 wire tables: `method_index` is a flat ClientMethod index, looked up in the row's entity type when it has one; `msg_id` is read from the server's interface on `client.net.out` and `client.ability.sent*`, the client's otherwise |
| `entity_id`, `target_id`, `source_id`, `pet_id` + `<prefix>_name` | Entity IDs, named by the cell after whoever held the slot when the client wrote the row: the row's `ts_ms` mapped onto the server clock by a per-session offset (receive time minus the chunk's newest `ts_ms`, smallest seen), in the space the session last reported. Unnamed when the cell can't say |
| `ability_id` + `ability_name`, `item_type_id` + `item_name` | Named from the NameBook |
| `address` + `address_name` | A native SGW.exe address, named from the committed symbol table (`crates/admin-api/src/routes/telemetry/client_symbols.tsv`), exact entry points only |
| `level_name`, `dll_version`, `fingerprint_usable` | Lifted from the DLL's `fields` bag on rows with no game ID (boot, hooks, streaming); absent otherwise, never `0` |
| `rollup_target`, `rollup_count` | On a `client.telemetry.rollup` row: the target the governor summarized and how many events the row stands for (`count`, lifted only alongside `rollup_target`). `sum(rollup_count)` by `rollup_target` recovers totals |
| `fields` | The DLL's whole `fields` bag as JSON |

Every lifted key is absent when the event lacks it, and every name is absent when it doesn't resolve (Rule 6), never `0` or `""`. `tracing` caps an event at 32 fields, so a row is replayed in one of two shapes: one with any game ID carries the ID and name pairs, any other carries `level_name`, `dll_version`, `fingerprint_usable` and the rollup pair. Both carry the identity, the address pair and `fields`. The DLL's own `account_id` and `player_id` claims are not lifted: identity is the token's.

`client_target` values added 2026-09-29, all under `client.native`:
`client.lua.debug_log` (the UI's `Debug:log` / `warn` / `error` lines and the
Black Market overlay's `[Cimmeria BM]` lines), `client.sequence.dropped` (a
SequenceManager drop, with `path`, `sequence_id`, `entity_id`),
`client.launcher.install_result` (one row per launcher install run, every
patch's outcome) and `client.patches.counts` (the client-patches DLL's
claimed, delivered and dropped counts). `client.lua.error` now names status
-1 `foreign_exception`. Fields and throttles:
[client-telemetry.md](client-telemetry.md) and
[dev-session-telemetry.md](dev-session-telemetry.md).

`OTEL_FILTER` stays hand-written, because it is also the span filter and
because putting a new scope's DEBUG rows in the primary view is a
decision. The TRACE filter is **derived** from the file-layer table
(`FILE_LAYERS` in
[`crates/server/src/logging/filters.rs`](../../crates/server/src/logging/filters.rs)):
every target a file keeps at TRACE, plus every custom (non-module-path)
target `OTEL_FILTER` names, plus the `wire.sampled.*` firehose samples.
Module-path blankets are not raised, so `cimmeria_services=debug` does
not become `cimmeria_services=trace`, and a module no file keeps at
TRACE (the orchestrator lifecycle rows, for one) stays out.

Two exceptions to parity, both pinned:

- **The exporter's own transport** (`hyper`, `h2`, `tonic`, `tower`,
  `reqwest`, `opentelemetry`, `tungstenite`) is `off` in `OTEL_FILTER`.
  `server.log` keeps its INFO rows; exporting them would loop every
  batch's gRPC chatter into the next batch.
- **The per-packet firehoses.** Three TRACE rows fire once per datagram
  or once per (witness, entity) pair per 100 ms tick. Each keeps its
  message text but moves to a `wire.firehose.*` target, which its file
  names explicitly and every OTLP filter turns off. Every N-th
  occurrence also emits a sampled row on a different, exported target,
  carrying `sampled_1_in = N` and `suppressed` (occurrences since the
  previous sample), so `sum(1 + suppressed)` recovers the true count.
  The emitters and N live in `cimmeria_wire::firehose`.

| Firehose (file) | Sample | Index | N | Why this N |
|---|---|---|---|---|
| `wire.firehose.decrypt` `DECRYPT_OK` (`base.log`) | `wire.sampled.decrypt`, with `hex` | `cimmeria-trace` | 53 | ~6 packets/s idle, ~20 moving: one hex sample every 2.5–9 s per client |
| `wire.firehose.udp_in` `UDP_IN` (`base.log`) | `wire.sampled.udp_in`, `len` only | `cimmeria-trace` | 53 | Same stream as `DECRYPT_OK`. No hex: pre-login datagrams carry the `baseAppLogin` ticket |
| `wire.firehose.aoi_position` `AoI: entity position update` (`world_entry.log`) | `wire.out.avatar_update` (the NA00 row) | `cimmeria-server` | 101 | Was 100. A shared counter samples only pairs at multiples of `gcd(N, pairs per tick)`, so at exactly 50 pairs per tick 100 always hit the same pair; a prime covers every pair below 101 |

All three N are primes for the same reason: a client's packet mix and the
AoI relay order are periodic, and a composite N can phase-lock the sample
onto one packet kind or one pair.

**Hand-named targets (round 2).** A `target: "…"` row matches none of the
module-path file layers, so the file guard cannot see it, and before
round 2 fourteen of them emitted DEBUG rows that reached neither a file
nor SigNoz: `abilities` (effect dispatch, pulses, shields), `abilities.sequence`,
`content.resolve`, `dialog.display`, `mission.step_context`,
`movement.movement_type`, `movement.position_sample`, `movement.validation`,
`player.journal`, `trade.atomic_swap`, `console.feedback`, `client.native`,
`launcher.*` and `cimmeria_discord`. All are now named in `OTEL_FILTER` at
DEBUG, except `client.native`, which moved to the `cimmeria-client` index
(every level) in 2026-09. `launcher.key_dump` is turned `off` beside `launcher=debug`: it
carries a client session key and must never leave the host. The one
per-tick-per-entity row among them, `movement.movement_type`
`outcome = "deduped"` (TRACE, once per NPC per 2 s AI tick), is sampled
1-in-53 with `sampled_1_in` and `suppressed`. The others are event-driven:
together a few tens of rows/s in a busy fight (effect pulses and ability
sequences dominate), near zero when idle.
`crates/server/src/logging/target_scan_tests.rs` reads the source of every
crate linked into the server, finds each literal-target event call and
requires it to reach one index at the level it is emitted; the only
exemptions are the `off` targets, each listed with its reason. Every crate
directory must be classified as in- or out-of-process, so a new crate
cannot skip the scan.

`crates/server/src/logging/parity_tests/` builds the production filters
on recording layers and, for every directive of every file layer, fires a
representative event at TRACE, DEBUG and INFO: an event the file keeps
must reach exactly one OTLP index, or, for a firehose, none, with its
sample reaching one. `guard::client_replays_land_only_in_the_client_index`
pins the client index, and `logging/client_index_tests.rs` drives a real
ingest replay through the OTLP bridge into a recording exporter and checks
the client resource and record attributes. It also checks `server.log`'s targets, that no
target at any level reaches two indexes, and that the guard itself
catches a file layer added without `OTEL_FILTER` coverage. Its
`crate_rows` module guards the module-path rows themselves: every crate
linked into the server has its own `OTEL_FILTER` row reaching each of its
top-level modules (or a listed reason why not), and no row reaches the
events of two crates. `EnvFilter` matches by string prefix, so a bare
`cimmeria_cell` or `cimmeria_wire` row would also cover every
`cimmeria_cell_*` crate or `cimmeria_wire_log`, and dropping one of those
crates' own rows would then go unnoticed; the base, cell and wire rows name
their crates' modules for that reason.

### Stable target catalog

Every event with a stable `target:` is a queryable surface in SigNoz. The catalog, one row per target with its level, where it is emitted and the question it answers, is in [observability-target-catalog.md](observability-target-catalog.md), together with the rule that a new target must also be named in `OTEL_FILTER`, the saved SigNoz views for reading a playtest, and the `npc_ai.decision_outcome` enum. It moved out of this ADR on 2026-10-03 because it had grown to most of the file.

#### `npc_ai.decision_outcome` enum

Moved to [observability-target-catalog.md](observability-target-catalog.md#npc_aidecision_outcome-enum).

### Metrics

A third OTLP signal — alongside traces and logs — ships counters,
histograms, and up/down counters from
[`crates/observability/`](../../crates/observability/) (the
`cimmeria-observability` crate). The facade exposes thin macros:

```rust
use cimmeria_observability::{counter, histogram, gauge_add};

counter!("trade_swaps_total", "outcome" => "completed");
histogram!("trade_swap_duration_seconds", elapsed_secs, "outcome" => "completed");
gauge_add!("cover_slots_held", 1, "world" => "Castle");
```

Instruments are lazily registered on first emission via the global
Meter set by [`otel::init`](../../crates/server/src/otel.rs). When
`OTEL_EXPORTER_OTLP_ENDPOINT` is unset, the global Meter is never
installed and the macros expand to a no-op — same opt-in shape as
the rest of the OTLP pipeline.

The metrics provider uses a `PeriodicReader` with the default OTLP
emit cadence (60s). The metric exporter shares the same OTLP endpoint
+ protocol as the trace/log exporters — SigNoz ingests all three
signals via one collector.

**Label cardinality.** Per
[instrumentation-discipline.md](instrumentation-discipline.md#rule-4--metric-labels-are-enumerated-spanlog-fields-are-correlators):
metric labels must be enumerated low-cardinality strings (`outcome`,
`reason`, `kind`, `world`, `decision_outcome`). High-cardinality
correlators (`entity_id`, `player_id`, `peer`) belong in span/log
fields. A counter labelled by `player_id` would degrade ClickHouse's
merge-tree query performance non-linearly.

**Movement / navigation counters.** Three counters carry a `world`
label (≈24 shipped worlds — inside the ≤ ~30 design target in
[instrumentation-discipline.md §rule-4](instrumentation-discipline.md#rule-4--metric-labels-are-enumerated-spanlog-fields-are-correlators),
and the reason the September 2026 navmesh investigation had to join
space ids to world names by hand):

| Metric | Labels | Notes |
|---|---|---|
| `movement_validation_rejects_total` | `reason` (3), `world` (~24), `gate` (5, incl. `n/a` for non-navmesh rejects) | Incremented on **every** reject, including ones the log throttle suppresses — a throttle must never deflate the rate an operator alerts on |
| `npc_path_fail_total` | `world` (~24), `state` (5), `reason` (6) | Same discipline: counted every AI tick, logged ≤ 1 / 5 s per NPC. Routes with **no** usable path only; a partial route counts in `npc_path_partial_total` |
| `npc_path_partial_total` | `world` (~24), `state` (5) | NA02: a partial corridor that was still walked. Its `npc_ai.path_fail reason=partial` row has its own throttle window (`path_partial`), so it cannot hold back a real routing failure for the same NPC |
| `npc_path_requests_total` | `world` (~24), `state` (5), `status` (7) | One per AI `find_path` (NA02), beside the `npc_ai.path` row — the path-status mix |
| `npc_stale_velocity_total` | `world` (~24) | NA02: one per stale episode, not per 100 ms tick |
| `npc_stuck_total`, `npc_leash_loop_total` | `world` (~24) | NA02 detectors: every occurrence, including throttled ones |
| `npc_ground_deviation_total` | `world` (~24), `dir` (3) | NA02: one per off-floor episode, not per step |
| `npc_off_mesh_total`, `npc_spawn_off_mesh_total` | `world` (~24), `gate` (≤ 5) | NA02 |
| `npc_idle_parked_total` | `world` (~24), `reason` (18) | NA02 |
| `npc_threat_cleared_without_exit_total` | `world` (~24), `reason` (4) | NA02 (NA12 adds `target_lost`), one per player still listing the NPC |
| `npc_ai_idle_unticked` (up/down gauge) | `world` (~24) | NA02: Idle NPCs the AI tick skips, driven by deltas each AI tick |
| `npc_ai_transitions_total` | `world` (~24), `from` (12), `to` (12), `reason` (23) | One per actual AI-state change, beside the `npc_ai.transition` row. Only a few dozen `from`/`to`/`reason` triples occur in practice |
| `npc_ai_aggro_total` | `world` (~24), `cause` (3) | One per entry into Fighting, beside the `npc_ai.aggro` row |
| `movement_validation_warns_total` / `..._recoveries_total` / `..._corrections_suppressed_total` | `reason` | Unchanged |

Never entity ids, positions or mesh hashes as labels — those are
per-entity and per-build correlators and belong on the log event.

**Deploy-identity resource attributes.** Every metric, span and log
(both log indexes) carries four resource attributes, built by
`identity_attributes` in [`crates/server/src/otel.rs`](../../crates/server/src/otel.rs):

| Attribute | Source |
|---|---|
| `deployment.environment` | `CIMMERIA_DEPLOY_ENV` (default `"dev"`; the colo compose file sets `colo`) |
| `cimmeria.deploy_env` | The same value, under the name the NPC-AI runbook filters on (`cimmeria.deploy_env = 'colo'`) |
| `host.name` | The OS hostname (inside the container, the container hostname) |
| `service.version` | The git commit baked in at build time by `crates/server/build.rs`: the `CIMMERIA_GIT_SHA` Docker build arg in the release image (`release-container.yml` passes `github.sha`), `git rev-parse HEAD` in a source build, otherwise `"unknown"` |

Explicit builder attributes win over `OTEL_RESOURCE_ATTRIBUTES` in the
SDK's resource merge, so these four are authoritative. Before NA00 only
`deployment.environment` existed, so a colo row could not be tied to a
host or a build (audit gap T2).

### Cost on the hot path

`tracing::info!` with no subscriber attached: a single atomic load +
branch. With the OTLP layer attached: serialise to OTLP wire format +
push onto an in-process mpmc channel. The actual network send is on
a background task. Net cost per packet: handful of nanoseconds.

Sampling is `always_on` by default — Mercury packet rate is the
analytical surface we care about, sampling defeats the purpose. If
volume becomes an issue, the lever is `OTEL_TRACES_SAMPLER` (set per
deployment via the compose env var), not source code changes.

The exception is the log firehoses (see "Log indexes and parity with the
log files" above). Those are sampled in source, because the question
asked of them is "what did this packet look like", which a counted
sample answers, and a 1:1 export would outweigh every other row in the
trace index. Estimated volume with five active players: ~75 inbound
packets/s give ~1.4 `DECRYPT_OK` and ~1.4 `UDP_IN` samples/s. The AoI
relay (~10 Hz per witness–entity pair, ~1,200/s at 3 players watching 40
NPCs) sends ~12 samples/s to `cimmeria-server`, as it has since NA00.
The remaining per-packet TRACE rows are **not** sampled: `encrypt` /
`decrypt` in `cimmeria_mercury` (one per outbound and per inbound
datagram), ACK queueing, `EntityMove` and
`AVATAR_UPDATE_EXPLICIT -> CellService` (one per player movement
packet). They are a few fields each and arrive at about the rate of
`mercury.packet`, which `cimmeria-network` already receives unsampled —
roughly one `cimmeria-trace` row per datagram. If that proves too much,
move them behind `cimmeria_wire::firehose` the same way.

### Timestamps — server-receive vs. client-generate

OTLP events are timestamped at the moment the tracing macro fires.
For server-originated events (Mercury packets, internal logs) that
*is* the event time. For launcher uploads, the tracing call fires
when the server receives the bundle/chunk, **not** when the launcher
captured the line. The client-side capture time is preserved in the
`ts_ms` structured field on every `launcher.*` event.

Implication for queries: SigNoz's main timeline pivot is server-
receive-time. To plot client-generate-time, group by `ts_ms` instead
of the default timestamp column. The two are usually within seconds
of each other (the launcher flushes every 2s), but a launcher that
queued events offline can produce arbitrarily large skew on the next
upload.

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
- Single analytical store — no parallel sinks to keep in sync, no
  question of "which store has the data I want" at query time.

### Negative

- Operators have a new docker stack to keep alive on the colo
  (SigNoz's ~6 services). All vendored into one self-contained
  `docker/compose.yml` so the deploy unit stays a single file, but
  upgrades require a manual re-vendor of the inlined SigNoz config
  sections alongside the image-tag bump.
- The Cosmos write path is gone. If we ever want it back, we'd
  re-introduce `cosmos_log.rs` alongside (not instead of) the OTLP
  layer — they coexisted fine in earlier iterations.
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

## Known gaps

Folded in from the superseded server-systems survey (see
[server-systems.md](server-systems.md)), which is where the metrics half of that
document lived. The OTLP/SigNoz work above closed most of it; these are what is
left.

**Client-reported performance data is still dropped.** Every connected client
pushes `SGWPlayer.perfStats` — 12 floats covering FPS min/avg/max, bytes and
packets in/out, lag min/avg/max, resends, and appearance-job count. The handler
at
[`crates/base/src/base/dispatch/diagnostics.rs`](../../crates/base/src/base/dispatch/diagnostics.rs)
validates the 48-byte payload length and then discards the contents; its own
comment marks the intended next step ("parse the 12 floats here and emit a
`perf_stats` metric"). This is the cheapest remaining win in the whole
observability surface — the data already arrives, on every client, for free, and
per-client lag and resend counts are exactly what you want when someone reports
that the server "feels bad".

**Gameplay metrics are uncounted.** Kills, deaths, items looted, missions
completed, and abilities used produce log lines but no counters, so there is no
way to ask "how much combat happened last night?" without grepping. Follow the
label-cardinality rules in
[instrumentation-discipline.md](instrumentation-discipline.md) before adding
any — per-player labels are the trap.

**No anomaly alerting.** Nothing watches for tick-rate degradation or an
unexpected entity-count spike. SigNoz supports alert rules; none are defined.

## References

- Deployment runbook: [signoz-deployment.md](../operations/signoz-deployment.md)
- Remote access runbook: [signoz-remote-access.md](../operations/signoz-remote-access.md)
- NPC AI telemetry runbook (post-session views + dashboard): [npc-ai-telemetry-runbook.md](../operations/npc-ai-telemetry-runbook.md); exported objects in [operations/signoz/](../operations/signoz/npc-ai-views.md)
- Instrumentation helpers: [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs)
- OTLP exporter: [`crates/server/src/otel.rs`](../../crates/server/src/otel.rs)
- Launcher ingest endpoint: [`crates/admin-api/src/routes/telemetry/`](../../crates/admin-api/src/routes/telemetry/)
- Launcher telemetry pipeline (dev-session flow + secret rotation): [dev-session-telemetry.md](dev-session-telemetry.md), [docs/operations/telemetry.md](../operations/telemetry.md)
