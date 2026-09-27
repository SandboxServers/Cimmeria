# Server observability — design and tool choice

**Status:** Accepted (2026-05-25)
**Last updated:** 2026-09-19
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
split across three providers, each its own SigNoz service, and every
record lands in exactly one of them:

| Level | `service.name` | Filter |
|---|---|---|
| ERROR, WARN | `cimmeria-server` | `OTEL_FILTER` |
| INFO, DEBUG | `cimmeria-server`, or `cimmeria-network` for the scopes `otel::is_network_noise_target` names | `OTEL_FILTER` |
| TRACE | `cimmeria-trace` | derived, see below |

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
DEBUG. `launcher.key_dump` is turned `off` beside `launcher=debug`: it
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
sample reaching one. It also checks `server.log`'s targets, that no
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

Every event with a stable `target:` is a queryable surface in SigNoz —
filter by `scope_name = '<target>'` to count occurrences, pivot on
field values, plot rate-over-time. Adding a new target is cheap; the
discipline is that targets should be **stable strings** (not subject
to crate-rename churn) and **named for the question they answer**.

A new target is only cheap to *emit*. For it to **reach SigNoz** you
also have to name it in `OTEL_FILTER`, the `EnvFilter` directive string
shared by the OTLP trace and log layers in
[`crates/server/src/logging/filters.rs`](../../crates/server/src/logging/filters.rs). A
custom target that is not listed there inherits the leading `info`, so
every DEBUG event it emits is dropped before the exporter sees it. That
is what happened to `aoi.create_emit`: the seam was built to localise the
invisible-static-NPC drop and was absent from the 2026-09-19 repro
because the filter named only `aoi.entity_enter` and `aoi.entity_leave`.
The unit test `otel_filter_exports_the_debug_level_aoi_seams` now pins
the DEBUG-level `aoi.*` directives so the next seam you add does not go
quiet the same way.

Directive targets match by **string prefix**, and the longest matching
directive wins. `npc_ai=debug` therefore already exports `npc_ai.tick`,
`npc_ai.transition`, `npc_ai.aggro` and every other `npc_ai.*` target, and
`cover=debug` covers every `cover.*` target. A more specific directive is
needed only to raise one child above its parent: `wire.out=info` drops the
DEBUG `wire.out.avatar_update` sample unless `wire.out.avatar_update=debug`
sits beside it. NA00 added that directive plus `movement.navmesh=debug`,
`cover=debug`, `spawner=debug` and `content=info` (audit gap T1), and
`otel_filter_prefix_matching_exports_npc_ai_children` pins the prefix
behaviour itself, not just the directive strings. NA02 added
`wire.out.forced_position=debug` (the same `wire.out=info` problem); its
other detector targets ride existing prefixes, and
`otel_filter_exports_every_na02_detector_target` emits one row per target
at its real level and asserts all of them pass.

| `target` | Level | Emitted from | What it counts |
|---|---|---|---|
| `mercury.packet` | INFO | `Channel::{send,receive}_packet`, `UnifiedCodec::{encode,decode}` | Every byte in/out of the server |
| `mercury.retransmit` | INFO | `Channel::check_timeouts` | Reliable-channel retransmits |
| `mercury.backpressure` | WARN | `Channel::send_packet` when TX window ≥ 50% full | Send-window saturation — early warning for stalled clients |
| `mercury.rx_order` | DEBUG / WARN | `Channel::receive_parsed` on each client session's receive path | The reliable receive gate (NA38). `event`: `buffered` (DEBUG, a client packet held behind a gap), `duplicate` (DEBUG, a retransmitted packet dropped instead of dispatched twice), `out_of_window` (WARN, `reason=beyond_rx_window`, dropped unacked so the client resends it). Fields `peer`, `seq`, `expected`. `rx_stall` (WARN, `reason=reliable_gap_unfilled`, from `Channel::check_rx_stall` on the 100 ms tick) fires when one gap has blocked delivery for more than 2 s, and repeats at most every 10 s per gap. Its fields are `peer`, `expected`, `first_buffered`, `last_buffered`, `buffered`, `depth`, `stalled_ms` and `first_warning`. Counter `mercury_rx_stalls_total` counts each such gap once. Silent on a clean link |
| `wire.in` / `wire.out` | INFO | `cimmeria_wire_log::wire_log::{log_inbound, log_outbound_entity_method}` | Decoded entity-method calls. `wire.in` resolves cell methods by **method index** (`msg_id - 0x80`, or `61 + sub_index` for the `0xBD` sub-slot form) and carries `method_index` + `entity_method`; base methods (`0xC2+`) are `baseMethod` with their index until a base name table exists |
| `aoi.entity_enter` / `aoi.entity_leave` | DEBUG | AoI tick witness fanout | Per-entity AoI transitions |
| `aoi.create_emit` | DEBUG | `base::world_entry::cell_dispatch::aoi::entered_aoi` (per-entity packets); `base::world_entry::cell_dispatch::deferred_flush` (flush bundles) | Per-packet entity-introduction delivery (CREATE_ENTITY+UPDATE_AVATAR / createOnClient cascade) — fields `witness_id`, `entity_id`, `class_id`, `phase` (`create_base` \| `cascade`), `addr_resolved`, `bytes`, `seq`. The bundle path carries N entities in one send, so it reports `entered` (the folded-in NPC count) and `packets` in place of a per-entity `entity_id`. Success-side visibility for the invisible-static-NPC drop — and it only reaches SigNoz because `OTEL_FILTER` names it, see above |
| `aoi.create_send_failed` | WARN | `base::world_entry::cell_dispatch::aoi::entered_aoi` (per-entity packets); `base::world_entry::cell_dispatch::deferred_flush` (flush bundles) | Entity-introduction packet/bundle that could NOT be delivered — `reason` (`entity_to_addr_miss` \| `client_disconnected` \| `send_error`), `phase`, `addr_resolved`. Negative-logging seam for the invisible-corpse class |
| `aoi.player_ghost_incomplete` | WARN | `base::world_entry::cell_dispatch::player_ghost::resolve_identity` | A **player** entering another player's AoI whose session half could not be fully resolved — `reason` (`observee_session_unresolved` \| `no_cached_appearance`), `witness_id`, `entity_id`, plus `addr_resolved` on the former. The first falls back to the bare NPC-shaped cascade (witness sees a nameless, bodyless player); the second still sends name + stats but no body. See [player-ghost-aoi-cascade.md](player-ghost-aoi-cascade.md) |
| `aoi.witness_broadcast_failed` | WARN | `base::helpers::witness_broadcast::broadcast_to_witnesses` | Base-built entity method (rebuilt `BeingAppearance`, …) could not be handed to the cell for witness fan-out — `reason` (`cell_channel_closed`), `entity_id`, `method_index`. Other players keep a stale view of the entity until it re-enters their AoI |
| `aoi.cinematic_hold` | INFO | `base::world_entry_appearance::cinematic_aoi_hold::{arm_timeout, release}` | The first-login cinematic AoI hold, one row per side. `event = "hold_started"` carries `witness_id`, `token` and `hold_ms`; `event = "hold_released"` carries `reason` (`cancel_movie` \| `timeout`), `flushed` (how many held messages went out) and `held_ms`. A `reason = "timeout"` row means the player let the whole intro movie run. Pair it with `aoi.create_emit` for the same `witness_id` to see the held introductions land. See [first-login-cinematic-aoi-hold.md](first-login-cinematic-aoi-hold.md) |
| `movement.player` | DEBUG (1-in-10 sampled) | `cell::service::base_messages` position-update path | Player avatar position updates |
| `movement.npc` | DEBUG (1-in-10 sampled `step`, always `waypoint_reached`) | `cell::service::ticks::npc_movement` | NPC nav-path movement. `step` logs the **first 5 steps of every leg** plus a 1-in-10 global sample, and carries `yaw_rad`, `yaw_byte`, `leg_step`, `y_source`, `ground_y` and `y_offset_from_ground` (navmesh height under the NPC; absent in meshless worlds) |
| `npc_ai` | DEBUG / INFO | `cell::service::npc_ai_fight` | NPC AI tick outcomes — see `decision_outcome` |
| `threat` | INFO | `cell::combat::threat::{enter,exit}_player_combat` | Player combat-enter / combat-exit transitions (gated on actual state change) |
| `trade.request` / `trade.cancel` / `trade.update_proposal` / `trade.lock_state` | INFO | `cell::cell_methods::player::trade::handlers` | Per-handler trade dispatch from the cell side |
| `trade.execute` | INFO | `base::world_entry::methods::trade::execute::handle_execute_trade` | Base-side execute span (entrypoint) — wraps the atomic_swap call |
| `trade.atomic_swap` | INFO | `base::world_entry::methods::trade::execute::swap::atomic_swap` | The DB-tx span — `phase = "..."` debug breadcrumbs name the failing step on abort |
| `crafting.load` / `crafting.save` | INFO | `base::crafting::persistence::{load_crafting_state, save_crafting_state}` | Crafting state round-trip — correlator: `player_id` |
| `crafting` | INFO / WARN | `base::crafting::{request, feedback, spend, sync, gate, options, tools, allcraft, session, transaction, item_use, alloy, craft}`, `base::world_entry::methods::progression::asp_earning`, `cell::service::ticks::crafting_stations`, `cell::cell_methods::player::crafting`, `crafting::CraftingCatalog::load` | The crafting campaign's events (`docs/analysis/crafting/`). `event`: `request` (INFO, one per crafting press; `entity_id`, `player_id`, `method`, `allowed` = the station-gate mask, `args` = every parsed argument), `rejected` (INFO, `reason`, e.g. `not_available_yet`; the player got a text line), `malformed` / `no_player` / `forward_failed` (WARN, cell side: a request dropped before the base), `catalog_loaded` (INFO, counts) / `catalog_load_failed` (WARN). Every player event carries `account_id`, `player_id` and `entity_id` on the event. `rejected` also carries `verb` and the values the rule compared (`discipline_id`, `asp`, `paradigm_id` / `paradigm_level` / `required_level`, `prerequisite_id` / `prerequisite_expertise` / `required_expertise`); spend reasons are `unknown_discipline`, `already_known`, `no_asp`, `paradigm_too_low`, `prerequisite_missing`, `prerequisite_expertise`, and `unavailable` for a server failure. `learned` (INFO: `discipline_id`, `expertise_before`, `expertise_after`, `asp_before`, `asp_after`), `login_sync` (INFO: `disciplines`, `paradigms`, `blueprints`, `asp`, `defaults_applied`, omitted when the state did not load; `crafting_options` = whether the bundle carried 140), `asp_granted` (INFO, GM grant: `amount`, `asp_before`, `asp_after`), `asp_earned` (INFO, level-up grant from `progression::handle_grant_xp`: `level_before`, `level_after`, `asp_before`, `asp_after`; its missing-row seam is the untargeted `persist_failed` WARN with `phase=grant_xp_update`). WARN seams: `login_sync_failed` (`reason` = `load` \| `send`, `error_class`), `push_failed` / `feedback_send_failed` (`reason` = `entity_to_addr_miss` \| `client_disconnected` \| `send_error` \| `empty_bundle`), `persist_failed` (`phase`, `error_class`, or the paired `rows_affected` / `expected`). Station gate, tools and options: `rejected` `reason=no_station_or_tool` also carries `station_mask` (the mask the cell granted) and `tools` (the crafting-bag instance ids considered); `options_changed` (INFO, on every `onUpdateCraftingOptions` (140) the base sends: `cause` = `login` \| `moved` \| `station_despawned` \| `world_change` \| `bag15_changed` \| `gm_anywhere`, `stations` and `tools` = the machine and tool id per section, 0 for none; the login one rides in the `login_sync` bundle); `gm_allcraft` (INFO `outcome=granted` with `disciplines_before/after`, `blueprints_before/after`, `paradigm_levels_before/after`; WARN `outcome=refused` for a caller below GameMaster, with `access_level`); `lookup_failed` (WARN, `phase` = `session` \| `tool_table` \| `login_tools` \| `gate_tools` \| `gate_catalog` \| `gate_item` \| `db_pool` \| `catalog` \| `paradigms` \| `load_state`; a failed read refuses the request or leaves the tools unchanged); `push_failed` `what` also takes `crafting_options` and `allcraft_state`; `persist_failed` `phase=save_crafting_state` for `.allcraft`. Crafting items (`verb=useItem`, from the item-use path): `request` (INFO, `item_id` = the instance), `blueprint_learned` (INFO: `item_id`, `type_id`, `blueprints` = `blueprint_id:known_before→known_after` per blueprint the item names, `known_before` / `known_after` = the known-blueprint count, `consumed` = `item_id:type_id:qty_before→qty_after`), `paradigm_raised` (INFO: `item_id`, `type_id`, `paradigm_id`, `level_before`, `level_after`, `consumed`); `rejected` reasons `already_known` (with `design_id` and `blueprint_ids`), `paradigm_max` (`paradigm_id`, `paradigm_level`, `required_level` = 10), `item_missing` (`item_id`) and `not_carried` (`item_id`, `design_id`, `container_id`), none of which consumes the item; `persist_failed` (WARN, `phase` = `no_pool` \| `begin` \| `advisory_lock` \| `lock_item` \| `lock_player` \| `consume` \| `save_state` \| `outbox` \| `commit`, `reason` = `sql_error` \| `rows_affected_short` \| `bad_stack_size` \| `no_player`) and `lookup_failed` (WARN, `phase=item_effects`, `reason=no_effects`; or `phase=item_miss` when the check whether a missed instance was a crafting item fails, and the miss is then treated as an ordinary item). The cell's 1 Hz station tick logs only `forward_failed` (WARN, `kind=crafting_stations`), nothing per tick. Span `crafting.request` (INFO, `verb`) per request. Metrics: `crafting_requests_total{verb, outcome = accepted \| rejected}` (once per request, when it is answered; a server failure is `rejected` with reason `unavailable`) and `crafting_rejections_total{verb, reason}`. Induction engine (`base::crafting::session`, `base::crafting::transaction`), every event with `account_id`, `player_id`, `entity_id`: `queued` / `induction_started` / `induction_expired` (INFO; `job_id`, `verb`, `queue_len`; `induction_started` adds `timer_id` and `expires_at`, the game-clock `BigWorldTimeComplete` sent), `completed` (INFO; `job_id`, `verb`, `blueprint_id` / `item_id`, `consumed` = `item_id:type_id:qty_before→qty_after,…`, `granted` = `type_id:bag:slot:qty_before→qty_after,…` (a merge has a non-zero before), `expertise` = `discipline_id:before→after,…`, `result` / `chance` / `roll` for the verbs that roll), `queue_dropped` (INFO; `reason` = `logout` / `world_change` / `session_changed` / `stale_session` / `not_connected`, `cause` = the caller's label, `jobs_dropped`, `job_ids`), `persist_failed` (WARN, the transaction rolled back; `job_id`, `phase` = `plan` / `begin` / `resolve` / `lock` / `check_player` / `check_named` / `consume_named` / `consume` / `place` / `expertise` / `outbox` / `commit`, `reason` = `db_error` (with `error_class`, `sqlstate`, `error`) / `rows_affected_mismatch` (with `rows_affected`, `expected`) / `invalid_quantity` / `unknown_product` / `player_missing` / `no_database`, and for `phase=plan` a verb bug: `named_not_consumed` / `unnamed_instance`), `client_sync_failed` (WARN; DEBUG when the session had already ended; `job_id`, `what` = `induction_timer` / `remove_item` / `update_item` / `update_discipline` / `resync_remove` / `resync`, `reason` = `entity_to_addr_miss` / `send_error` / `client_disconnected` / `inventory_read_failed`). Induction refusals go through the same `rejected` event: `queue_full` (with `queue_limit`) at the request, and at completion `component_missing` / `component_not_in_crafting_bags` (with `item_id`, `container_id`), `component_mismatch` (with `item_id`, `design_id` = the design it was named for, `type_id` = what it is), `not_enough_components` (with `design_id`, `needed`, `available`), `inventory_full` (with `design_id`, `container_id`), `no_carried_bag_for_product` (with `design_id`) and `induction_failed` (after a `persist_failed`). `induction_start_skipped` (DEBUG: a job dropped between being activated and its bar being sent; no bar, no wake-up). No span inside the induction engine. Metric `crafting_jobs_total{verb, outcome = completed \| failed \| dropped}`, counted once per induction job when it ends; `verb` is the cell method name, as on the request counters. Research and reverse engineering (`base::crafting::{research, reverse_engineer, induction_verb, item_lookup}`, verbs `research` and `reverseEngineer`): refusals `not_researchable`, `not_kicker`, `not_reverse_engineerable`, `no_blueprint_for_item` (each with `item_id`, `type_id`), `kicker_same_science` / `kicker_duplicate_science` (also `applied_science_id`), and `component_missing` / `component_not_in_crafting_bags` at the request for a named item that is gone or outside bags 1 and 15; `completed` adds `discipline_id`, `eligible_disciplines` and `blueprints_learned` = `blueprint_id:false→true,…` (research) and `component_set_id`, `bias`, `rolls` = `design_id:roll:recovered/quantity,…` (reverse engineering); `blueprint_learned` (INFO, a research success that taught: `job_id`, `item_id`, `type_id`, `blueprints` = `blueprint_id:false→true,…`, `known_before`, `known_after`); `lookup_failed` `phase` also takes `no_pool` / `catalog_load` / `named_items` (at the request, then `rejected reason=unavailable`) and `completion_state` (with `job_id`, then `rejected reason=induction_failed`); `persist_failed` `phase` also takes `learn_blueprints`; `client_sync_failed` `what` also takes `known_crafts` and `result_line`. Alloying (`base::crafting::alloy`, `verb=alloying`): `rejected` reasons `unknown_blueprint` / `not_alloy` (with `blueprint_id`), `discipline_unknown` (with `blueprint_id`, `discipline_id`), `wrong_tier` (with `item_id`, `type_id`, `tier`, `required_tier`), `count_not_met` / `multiple_buckets` (with `elementary_counts` = `normal:N,good:N,great:N,fantastic:N`, the summed stack quantity per quality), and the component reasons above raised at the request; `completed` adds `quality_bucket` (`normal` \| `good` \| `great` \| `fantastic`) and `elementary` = `item_id:type_id:quality:tier:quantity_used,…`, with `item_id` = the current-tier item; `lookup_failed` (WARN, answered `unavailable`) with `phase` = `no_pool` / `catalog` / `alloy_inputs` / `alloy_discipline` / `alloy_component` / `alloy_product` / `alloy_product_quantity` / `alloy_item`, `blueprint_id`, `id`, `error_class`; at completion the blueprint and discipline are checked inside the transaction (`rejected` `unknown_blueprint` / `discipline_unknown`, or `persist_failed` `phase=knowledge`); `malformed` (WARN, base side, no line: `reason=too_many_elementary_items`, `count`, `limit`); `client_sync_failed` `what` also takes `success_line`. Craft (`base::crafting::craft`, `verb=craft`): `rejected` reasons `bad_quantity` (with `quantity`), `unknown_blueprint`, `is_alloy`, `discipline_unknown` (with `discipline_id`), `no_component_set` (with `type_ids` = the distinct submitted designs) and `insufficient_components` (with `design_id`, `needed`, `available`), each with `blueprint_id`, plus `component_missing` / `component_not_in_crafting_bags` for a bad named instance at the request; `unknown_blueprint` / `discipline_unknown` can also come at completion, from the transaction's knowledge check (a respec during the bar; `persist_failed` `phase=knowledge` for a database error or a missing player row there). `completed` adds `component_set_id` and `quantity` (the times the blueprint ran). `lookup_failed` (WARN, with `blueprint_id`, `error_class` = the SQL class or `miss`) `phase` = `no_pool` / `catalog` / `crafting_state` / `blueprint_product` / `inventory` / `product_name`, all at the request. `client_sync_failed` `what` also takes `craft_queued` (the queued line) and `craft_result` (the success line). An accepted craft counts `crafting_requests_total{outcome=accepted}` once, when it starts or queues |
| `cover.reservation` | WARN | `cell::cover::ai_integration::try_reserve_or_warn` | Cover-slot race-lost — defensive against future async refactors |
| `spawner.npc_respawn` | INFO | `cell::service::ticks::npc_respawn::npc_respawn_tick` | Per-NPC respawn promotion — correlator: `world_name`, `respawn_secs` |
| `movement.validation` | WARN (`validation_reject`, `validation_recovered`, `speed_warning`, `navmesh_gm_bypass`, `space_mismatch`) / ERROR (`correction_suppressed`) | `cell::service::base_messages`, `cell::space_manager::movement_telemetry` | Movement reject — snap-back to last_valid. Every row **except `entity_missing`** carries `world` (name, not just `space_id`); `entity_missing` cannot, because the entity is in no space by definition. The three hard-reject outcomes (`validation_reject` \| `validation_recovered` \| `correction_suppressed`) are all counted by `movement_validation_rejects_total` and all carry the containment diagnosis when `reason = "navmesh"`: `gate` (`no_poly_in_extents` \| `horizontal` \| `below_surface` \| `above_jump_tolerance`), `nav_horiz_dist`, `nav_dy`, and `navmesh_hash`. **Throttled per entity and row kind** (first immediately, then ≤ 1/s per kind — a shared window would hide the transition into `correction_suppressed` behind the rejects that led to it) with `suppressed = N` naming the rows elided since the last emission — see [negative-logging-convention.md §pattern-d](negative-logging-convention.md#pattern-d--high-frequency-repeat-throttled-with-a-suppressed-count) and the full contract in [movement-telemetry.md](movement-telemetry.md) |
| `playtest.bookmark` | INFO | `cell::console::bookmark::emit` | One row per GM `.bug <note>` — the tester, their target, mission/step/objective state, regions, counters and the free-text note. Correlator: `bookmark_id`. The entry point for reconstructing a playtest without a chat log |
| `playtest.bookmark.entity` | INFO | `cell::console::bookmark::emit` | One row per entity within 60 u of the tester at `.bug` time (nearest 32; the selected target always included). Position, velocity, `yaw_rad`, **`yaw_byte` (the facing actually transmitted)**, `wire_facing_vs_caller_deg`, `ground_y` / `y_above_ground`, AI state, nav path, threat, follow target, spawn distance, `witness_count` (players whose AoI holds the entity) and `caller_witnesses_it` (read from the tester's AoI; before NA24 both were read off the entity itself and were always `0` / `false` for NPCs). NA44 adds `ability_ids` (sorted; `[592]` alone is the Pistol Shot fallback), `weapon_visual` (a player's bandolier weapon, or an NPC's first `WP` template component) and `current_target_id` (the entity's selected target; NPCs aim at `threat_top_id`). Join on `bookmark_id` |
| `player.journal` | DEBUG | `cell::player_journal::note` | **The cross-system order for one player.** Every notable per-player event gets one strictly increasing `seq` and a closed `kind` vocabulary: `world_enter`, `reanchor`, `region_hint`, `cover_edge`, `step_advance`, `mission_complete`, `dialog`, `action_list` (the ordered actions a trigger resolved to, with delays), `deferred_scheduled`, `deferred_fired`, `death`, `respawn`, `kill`, `teleport`. `scope_name = 'player.journal' AND entity_id = N` ordered by `seq` replaces rebuilding order from timestamps across scopes. `.bug` attaches the last 24 entries as `recent_events` |
| `content.deferred` | INFO | `cell::content::executor::deferred` | A delayed content action firing: `chain_id`, `action_kind`, `delay_ms`, `late_ms`, `scheduled_seq` / `fired_seq`, and **`between`** — everything journaled for that player since it was scheduled. The dialog-replaced-after-0.6-s shape in one row |
| `npc_ai.tick` | DEBUG | `cell::service::npc_ai::dispatch::log_ai_tick` | One row per ticked NPC per AI tick, emitted after the handler with no silent paths: position, `yaw_rad` + **`yaw_byte`**, `last_movement_type`, `nav_path_len`, `next_wp`, `dest`, `dist_to_dest`, `target_id` + `target_pos` + `dist_to_target`, three-state `los` (`clear` \| `blocked` \| `unknown`; was the bool `has_los` before NA02), `los_policy` (the attack rule that acted on `los`: `strict` \| `stationary` \| `stationary_relaxed` \| `stationary_other_storey`; NA16, D-NA11), `vx` / `vy` / `vz` (the velocity every witness is sent), `follow_target_id`, `npc_to_spawn` (renamed from `dist_to_spawn` in NA00), `move_speed`, `navmesh_loaded`, `state_before` / `ai_state`, and the handler's `decision_outcome` (empty = the handler declared none). `fight.rs` outcomes now go through the shared helper, so they reach this row, the span and `npc_ai_decisions_total`. NA24: an NPC that was Idle, stayed Idle and is in no player's AoI writes one row per 60 s, with `suppressed` counting the rows skipped (`0` on every other row); every non-Idle or witnessed tick is still written |
| `npc_ai.transition` | DEBUG | `cell::service::npc_ai::transition::set_ai_state_on` | `event = "state_change"`: one row per **actual** AI-state change (a write that leaves the state unchanged logs nothing). `from`, `to` (snake_case `AiState` labels), `reason` (`auto_aggro` \| `assist` (NA14) \| `threat_preempt` \| `threat_empty` \| `leash_out` \| `target_lost` \| `unreachable` (NA15) \| `leash_arrived` \| `leash_snap_fallback` \| `died` \| `respawn` \| `content` \| `gm_command` \| `patrol_start` \| `wander_start` \| `patrol_no_path` \| `wander_no_radius` \| `wander_no_spawn` \| `investigate_no_poi` \| `investigate_done` \| `follow_no_target` \| `follow_target_gone` \| `follow_resumed` (NA42: a follower's leash reset returned it to Follow because its leader is still in the space) \| `pet_engage` (pets PT-05: a pet's stance engaged a target) \| `pet_follow` (pets PT-05: a pet was put back on its owner, after summon, after a fight or out of a state a pet never keeps)), plus `npc_id`, `tag`, `template_id`, `world`, `space_id`, `npc_to_spawn`, `threat_count`, `nav_path_len`. `CellEntity::ai_state` is a private field and this helper is its only writer, so the row is a complete per-NPC state timeline. Counter `npc_ai_transitions_total` |
| `npc_ai.aggro` | INFO | `cell::service::npc_ai::aggro_acquired::log_aggro_acquired`, called from `combat::generate_threat` | `event = "acquired"`: one row per entry into Fighting. `cause` (`proximity` \| `damage` \| `content_threat` \| `assist`, NA14: a same-faction neighbour engaged and this NPC joined; an assist never recruits further \| `pet_stance`, pets PT-05: a pet engaged this NPC (`engage_pet_target`), which lists the pet as its `target_id`; never recruits), `from`, `target_id`, `player_id` and `account_id` (only when the target is a player), `npc_to_target`, `dy`, three-state `has_los` (`clear` \| `blocked` \| `unknown`), `aggression` (effective `EMobAggressionLevel` toward players, 1 = hostile, NA13) and `aggression_override` (unset when faction-derived), plus the common NPC fields. Replaces the unstructured "NPC aggro: preempt -> Fighting" line. Counter `npc_ai_aggro_total`. Also `event = "gm_toggle"` (INFO, no counter) when a GM sets `.aggro on|off`: `player_id`, `aggro_off`, `changed` (NA13). NA42: `event = "being_refused"` (DEBUG, no counter): `generate_threat` refused a being (class 0x01, a prop or a story actor such as Col Marsh), so no threat, no preemption and no player combat; `npc_id`, `attacker_id`, `ai_state`, `cause` |
| `content` | WARN / INFO | `cell::content::executor::world::set_aggression` | `event = "set_aggression_tag_miss"` (WARN, `reason = "tag_not_found"`): the action's tag matched no entity, so the NPC's aggression is unchanged; carries `tag`, `chain_id`, `agg_level`. `event = "set_aggression_invalid_level"` (WARN, `reason = "invalid_level"`): a level outside 0-5, nothing changed (NA13). `event = "set_aggression"` (INFO): the hit, with `from` / `to` as level labels (`faction` when there was no override). A mistyped chain tag used to be silent and read exactly like an aggro bug |

| `spawner.npc_behaviour` | DEBUG | `cell::space_manager::npc_population::log_spawn_behaviour` | Resolved behaviour of each spawned NPC: `world`, `space_id`, `aggression`, `use_cover`, `is_stationary`, `move_speed`, `respawn_secs`, follow band, patrol / wander, `on_navmesh`, `ground_y`, `spawn_yaw_rad`, interaction flags, loot table, and (NA44) what it fights with: `ability_ids` (sorted), `event_set_ids` (each ability's event set in the same order; `0` = NULL or no loaded definition, so that attack plays no fire animation) and `weapon_visual` (first `WP` template component) |
| `player.death` | INFO | `cell::abilities::damage_apply` | First-class player death: identity, `killer` + `killer_name`, `ability_id`, world, position. The adjacent `onBeginAidWait` row now lists `respawner_ids` and its `filter` |
| `session.start` / `session.end` | INFO | `cell::service::base_messages::player_init`, `base::helpers::destroy_client_entities` | World entry (identity, character, archetype, level, `access_level`, world, mission count) and teardown (`disconnect_reason`, `session_secs`). Client telemetry is ingested by admin-api (`launcher.ingest` / `launcher.bundle`), so liveness is a query: a `session.start` with no `launcher.*` rows for the same window means the tester has no client logs — see the *sessions vs client telemetry* saved view |
| `player.respawn` | INFO | `cell::cell_methods::player::combat` | `callForAid` result: `state_flags_before` / `state_flags_after`, `was_dead`, `dead_flag_cleared`, health before/after/max, position from/to |
| `dialog.display` | DEBUG | `cell::content::executor::dialog::display` | Each dialog shown with `replaced_dialog_id` and `ms_since_previous`. `fire_dialog_choice` rows now carry `button_id` |
| `cover.flank_check` | DEBUG | `cell::cover::ai_integration` | Every flank test an NPC in a cover slot runs: slot, node position + orientation, threat position, `flanked`. Silent until an NPC actually holds a slot — check `use_cover` on `spawner.npc_behaviour` first |
| `console.feedback` | DEBUG | `cell::console::gm::feedback::send_gm_feedback` | The text every `.`-command sent back to the GM — results and rejection reasons alike (first 400 chars) |
| `org` / `squad` | DEBUG (decoded calls, no-op arms, `squad` transitions) / INFO (`squad` spans and outcome rows) / WARN (`*_malformed`, `org.forward_rejected`, `org.feedback_send_failed`, `org.squad_forward_failed`, `squad.actor_mismatch`, `squad.send_failed`, `squad.disconnect_stale_entity`) | `cell::cell_methods::organization` (and its `squad/`), `base::dispatch::organization`, the `Org` arms of both message dispatchers | Organizations (Teams and Commands on `org`, Squads on `squad`; organizations campaign, ORG-01). Every decoded organization call, with `method`, `org_id` or `instance_id`, the actor's `player_id` / `entity_id` and `text_units` (the length of any player text, never the text). A payload that does not decode logs WARN `event = org.cell_method_malformed` or `org.base_method_malformed` with `reason` (`truncated` \| `trailing_bytes` \| `lone_surrogate` \| `unknown_method` \| `invalid_value`). A base-side forward outside cell methods 8-17 logs `org.forward_rejected` with `reason = method_out_of_range`. **Squads (ORG-03):** INFO spans `squad.invite`, `squad.invite_response`, `squad.leave`, `squad.kick`, `squad.loot_mode`, and exactly one INFO outcome row per action with the same `event`, `outcome` (`ok` \| `rejected`), the actor's `account_id` / `player_id` / `entity_id`, `target_account_id` / `target_player_id` only for invite, invite response and kick, `squad_id` and `request_id` where known, and on a refusal `reason` (`target_ambiguous` \| `target_in_transition` \| `target_not_found` \| `self_target` \| `not_a_player` \| `squad_full` \| `already_in_squad` \| `not_leader` \| `rate_limited` \| `invite_limit` \| `invite_unknown` \| `invite_expired` \| `invite_foreign` \| `loot_mode_invalid` \| `not_in_squad` \| `wrong_squad` \| `target_not_in_squad` \| `inviter_left` \| `inviter_not_leader` \| `inviter_offline` \| `squad_gone` \| `ids_exhausted` \| `not_ready` \| `actor_mismatch` \| `cell_unreachable`, the last logged (and counted on `squad_actions_total` with `action` = `invite` \| `kick`) by the base when the forward fails; `ignored` is reserved until the cell has an ignore list). Each row also counts on `squad_actions_total{action, outcome, reason}` (`reason = none` on `ok`). DEBUG transitions: `squad_created`, `member_joined` (`rank`), `member_left` (`reason` = `requested` \| `kicked` \| `logout`), `leader_changed` (`from_player_id`, `to_player_id`, the new leader as target), `loot_mode_changed` (`from`, `to`), `disbanded` (`reason`), `invite_created`, `invite_consumed` (`accepted`), `invite_expired`, plus `squad.world_entry_replay`, `squad.left_owed`, `squad.disconnect_in_transit` (a `DisconnectEntity` for a member in gate transit, found by their last entity id) and `squad.disconnect_no_member` (an entity that is gone and in no squad, which the second `DisconnectEntity` of every full-exit log-off hits). The base forward logs DEBUG `org.squad_forwarded` or WARN `org.squad_forward_failed` (`reason = cell_unreachable`); an `organizationInviteByType` above type 2 is one INFO outcome row `event = org.invite_by_type`, `outcome = rejected`, `reason = org_type_invalid`. Base rows carry the session's `account_id`. WARN `squad.actor_mismatch` (a forwarded entity is no longer that character) and `squad.send_failed` (`reason = cell_to_base_closed`, with `method_index`); WARN `squad.disconnect_stale_entity` (`reason = stale_entity_id`: a disconnect names a member's old entity id while they are live under `live_entity_id`; the member is kept). Later packets add their decisions here |
| `rate_limit` | WARN (the drop that notifies the player) / DEBUG (the silent drops between) | `base::rate_limit::log_exceeded`, called from `base::dispatch::chat` (and later the mail-send and duel-challenge paths) | Per-player flood limits (social-systems campaign, SS-00; D-SS14, D-SS21). Every dropped action logs `event = rate_limit.exceeded` with `category` (`chat` \| `mail_send` \| `duel_challenge`), `addr`, `player_id`, `account_id`, the bucket state it was decided on (`tokens`, `burst`, `refill_ms`, `next_token_ms`), `reason = bucket_empty` and `notified`. WARN fires at most once per category per player per 5 s, the same throttle as the player's feedback line, so a flood cannot flood the log; count the DEBUG rows, or the counter `rate_limit_exceeded_total{category}` (every drop, one enumerated label), for the real drop volume |
| `online_index` | DEBUG | `base::player_index` (`log_listed` / `log_unlisted` / `OnlinePlayerIndex::lookup`) | The online name index that tells, mail notification and duel challenges resolve names against (SS-00, D-SS13). `event = online_index.insert` when a character is listed (`path = world_entry`) and `online_index.remove` when it leaves, with `path` = the teardown (`client_disconnect` \| `inactivity_timeout` \| `send_error` \| `duplicate_login` \| `logoff` from `destroy_client_entities`, `logoff_character_select` \| `logoff_full_exit` from `logOff`, `gate_travel_abandon`); both carry `addr`, `player_id`, `account_id`, `player_name`. `event = online_index.lookup` with `reason = missing \| ambiguous`, the looked-up `name` (first 64 characters), `name_chars` and `listed` (how many characters were online) for every lookup that does not resolve to one player |
| `chat` | INFO / WARN / DEBUG | `base::dispatch::chat::send_player_communication_at`, `base::dispatch::tell`, `base::dispatch::ignore`, `base::contact_list::ignore`, `cell::console::chat::spatial`, the cell's `UpdateIgnoreList` handler | The chat path's refusals before the cell forward (SS-00). `event = chat.rejected` for a line that breaks the D-SS12 text rules, with `reason` from `TextReject::reason()` (`too_long` \| `control_char` \| `bidi_control` \| `zero_width` \| `format_char` \| `line_separator`), `detail`, `text_units`, `max_units`, `channel`, `player_id`, `account_id` (never the text). **GM broadcast (SS-C2)**, from `cell-console` `gm::shout` and `base-world-entry` `chat_dispatch`: `chat.gm_broadcast` (INFO audit row: `entity_id`, `account_id`, `player_id`, `speaker`, `source` = `native` \| `console`, `scope` = `space` \| `global`, `space_id`, `text_units`, and the text, because a GM broadcast is a public announcement), `chat.gm_broadcast_delivered` (INFO: `scope`, `delivered`, `failed`, and `not_in_world` for global), `chat.gm_broadcast_rejected` (WARN, `reason` = `empty_text` \| `malformed_args` \| `no_text` \| `caller_not_found` \| a `TextReject::reason()`), `chat.gm_broadcast_send_failed` (WARN, one recipient). SS-C1 adds the tell and Ignore decisions: `chat.tell_delivered` (INFO, `target_player_id`, `text_units`, `away_reply`, and since SS-C3 `away_reply_withheld_muted`), `chat.tell_refused` (DEBUG, `reason` = `no_target` \| `self` \| `not_online` \| `ambiguous` \| `recipient_ignores_sender` \| `recipient_not_in_world` \| `recipient_left` \| `recipient_send_failed`), `chat.ignore_added` / `chat.ignore_removed` (INFO, `target_player_id`, `before`, `after`), `chat.ignore_refused` (DEBUG, or ERROR for `db_error`), `chat.ignore_synced` / `chat.ignore_sync_failed` (the session and cell copies of the list, `path`, `before`, `after`), `chat.spatial_ignored` (DEBUG, one row per withheld witness with `target_player_id`, `target_entity_id`, `target_account_id`), `chat.ignore_set_applied` / `chat.ignore_set_dropped` (cell; `reason` = `entity_missing` \| `player_mismatch` \| `stale_version`), `chat.away_rejected` (WARN, `kind` = `afk` \| `dnd`, `reason` from `TextReject::reason()`) and `chat.afk_set`. The `chat.tell` and `chat.ignore` INFO spans wrap the two handlers. SS-C3 adds the channel allowlist, the GM mute and the unsupported Communicator methods: `chat.channel_rejected` (WARN, `channel`, `reason` = `system_channel` \| `user_channel` \| `unknown_channel`), `chat.muted_refused` (DEBUG, `reason = muted`, `channel`, `tell`, `remaining_secs`, `muted_by_account_id`), `chat.gm_mute` / `chat.gm_unmute` (INFO audit rows: the GM's `entity_id` / `account_id` / `player_id`, `subject_player_id`, `subject_account_id`, `subject_entity_id`, `duration_minutes`, the GM's free-text `reason`, `previous_remaining_secs`, `remaining_secs`), `chat.gm_mute_refused` / `chat.gm_unmute_refused` (WARN, `reason` = `usage` \| `bad_duration` \| `bad_reason` \| `bad_name` \| `not_online` \| `ambiguous` \| `target_is_gm` \| `not_muted` \| `base_channel_closed`), and `chat.method_unsupported` (WARN, `method`, `payload_len`, `reason = not_implemented`) for base methods 0xC6-0xCE |
| `mail` | INFO (`mail.sent`, `mail.cash_debited`, `mail.item_escrowed`, `mail.cash_taken`, `mail.item_taken`, `mail.cod_paid`, `mail.cod_cancelled`, `mail.returned`) / WARN (`mail.send_refused`, `mail.delete_refused`, `mail.op_refused`, read-side owner misses) / ERROR (`mail.op_failed`) / DEBUG (decode verdict, failed recipients, headers sent) | `base::world_entry::methods::mail` (`send`, `read`, `headers`, `take`, `cod`, `return_`), `cell::mail::handle_send_mail`, `cell::mail::handle_attachment_op` | Gate mail (SS-M1, SS-M2, SS-M3). `event = mail.sent` with `target_player_ids`, `mail_ids`, `delivered`, `failed`, `subject_units`, `body_units`, `result`. `event = mail.send_refused` for every refused send with `reason` (`too_many_recipients` \| `truncated` \| `trailing_bytes` \| a `TextReject::reason()` \| `vault_alias_unsupported` \| `organization_alias_unsupported` \| `unknown_recipient_flags` \| `attachment_with_multiple_recipients` \| `negative_cash` \| `item_quantity_without_item` \| `invalid_item_quantity` \| `cod_without_item` \| `cod_without_price` \| `item_not_owned` \| `item_not_in_main_bag` \| `item_in_vault` \| `item_in_buyback` \| `item_bound` \| `item_quantity_exceeds_stack` \| `not_enough_cash` \| `no_recipients` \| `no_deliverable_recipients` \| `no_db_pool` \| `sender_missing` \| `db_error`), `result` (the `EMailResultCodes` token), `failed_recipients`, `failed_flags`, and `target_player_id` once a single recipient was resolved (absent before resolution). `event = mail.recipient_failed` per undelivered recipient with `reason` (`unknown_recipient` \| `ambiguous_recipient` \| `mailbox_full` \| `recipient_ignoring_sender`) and `target_player_id`. Attachments (SS-M2): `mail.cash_debited` with `mail_id`, `target_player_id`, `naquadah_before`, `naquadah_after`, `postage`, `cash`, `cod`; `mail.item_escrowed` with `mail_id`, `item_id` (the sender's row), `escrow_item_id`, `type_id`, `quantity`, `stack_before`, `stack_after`, `whole_row`; DEBUG `mail.attachment_refused` with the balance and cost a refusal was decided on and `target_player_id`. Delete guard: `mail.delete_refused` with `mail_id`, `reason` (`attachment_item_present` \| `attachment_cod_unpaid` \| `attachment_cash_present`), `has_item`, `cash`, `cod`; DEBUG `mail.deleted`. System mail (SS-U1): INFO `mail.system_sent` after the commit, with `sender_name`, `target_player_id`, `mail_id`, `cash`, `item_source` (`none` \| `minted` \| `existing_instance`), `item_id`, `type_id`, `stack_size`, `source_character_id`, `recipient_open_mail`, `over_cap`; DEBUG `mail.system_staged` inside the caller's transaction; WARN `mail.system_refused` with `reason` (a `TextReject::reason()` \| `negative_cash` \| `cash_too_large` \| `invalid_item_quantity` \| `unknown_item_type` \| `item_quantity_exceeds_stack` \| `item_not_found` \| `item_not_server_held` \| `item_owner_mismatch` \| `item_bound` \| `recipient_not_found` \| `db_error`), `container_id`, `owner_player_id`, `expected_owner_player_id`. GM tools: INFO `mail.gm_action` (`action` = `mail` \| `mailbox`, `minted = true` on `.mail`) with the GM's `account_id` / `player_id` / `entity_id`, `subject_player_id`, `mail_id`, `cash`, `cod`, `type_id`, `quantity`, `escrow_item_id`; WARN `mail.gm_rejected` with `command` and `reason` (cell: `no_recipient_name` \| `invalid_cash` \| `invalid_item_type` \| `invalid_item_quantity` \| `invalid_cod` \| `cod_without_item` \| `cod_with_cash` \| `no_mail_id` \| `expiry_not_available` \| `no_player_id` \| `base_channel_closed`; base: `unknown_recipient` \| `ambiguous_recipient` \| `gm_missing` \| `cod_price_invalid` \| `no_db_pool` \| a `mail.system_refused` reason). Take, pay and return (SS-M3): `mail.cash_taken` with `mail_id`, `target_player_id` (the mail's sender), `cash`, `naquadah_before`, `naquadah_after`; `mail.item_taken` with `mail_id`, `target_player_id`, `item_id`, `type_id`, `stack_size`, `container_id`, `slot_id`; `mail.cod_paid` with `mail_id`, `payment_mail_id`, `target_player_id` (the COD's sender), `price`, `naquadah_before`, `naquadah_after`; `mail.cod_cancelled` with `mail_id`, `reason = sender_gone`, `price`, `sender_name` (the stored name, the only trail once `sender_id` is gone); `mail.returned` with `mail_id`, `target_player_id` (the new owner), `cash`, `cod_cancelled`, `item_id`. Every refusal: WARN `mail.op_refused` with `target_player_id` (the mail's sender, when known), `op` (`take_cash` \| `take_item` \| `pay_cod` \| `return`), `mail_id` and `reason` (`not_found_for_owner` \| `cod_unpaid` \| `no_cash` \| `no_item` \| `balance_overflow` \| `bags_full` \| `not_cod` \| `not_enough_cash` \| `cod_without_item` \| `archived` \| `already_returned` \| `cod_paid` \| `system_mail`); a rolled-back op: ERROR `mail.op_failed` with `op`, `mail_id`, `reason` (`db_error` or the invariant that broke). Archive of an unpaid COD: WARN `mail.archive_refused` with `mail_id`, `reason = cod_unpaid`. On the cell: `mail.send_decoded` / `mail.send_decode_rejected`, and the `mail.attachment_op` span (`method`). Read side: `mail.headers_sent` (`b_archive`, `count`) and WARN `reason = not_found_for_owner`. Every row carries `entity_id` and `player_id`, and the send rows `account_id`; never the subject, body or names |
| `duel` | DEBUG (refusals and state transitions) / WARN (a payload that does not decode, a send that could not be queued) | `base::dispatch::duel` (`sendDuelChallenge`, 0xD9) and `cell::duel` (`challenge`, `response`, `tick`) | The duel challenge and answer (social-systems campaign, SS-D1). Base: `event = duel.challenge_refused` with `reason = not_in_world | challenger_loading | squad_duel | target_not_online | target_loading | target_ambiguous | target_not_in_world | target_ignoring | no_cell_channel | cell_channel_closed`, `duel.challenge_malformed`, `duel.challenge_forwarded`; the bucket's drops log on `rate_limit` with `category = duel_challenge`. Cell: `duel.challenge_refused` with `reason = challenger_gone | self_challenge | target_gone | cross_space | out_of_range | challenger_busy | target_busy | pair_cooldown` (and `distance`), `duel.challenge_sent`, `duel.challenge_undelivered` (`reason = prompt_not_queued`, the challenge is withdrawn), `duel.response_refused` (`reason = no_pending_challenge | expired | not_a_player`), `duel.response_malformed`, `duel.declined`, `duel.accepted`, `duel.accept_refused`, `duel.challenge_expired`, `duel.aborted` (`reason = engage_not_implemented` until SS-D2) and `duel.notify_skipped`. Every row carries `player_id` (the actor, or the challenger on tick rows) and `target_player_id` (the other duelist), with `account_id` and `entity_id` whenever that player is still in the world; every row after the challenge is stored carries `duel_id` |
| `playtest.friction` | WARN | `cell::playtest_friction` | Stuck-player detectors — one event per episode, discriminated by `signal`. Episode counters: `repeat_interact_no_effect` (5 dead-end interacts on one target / 60 s), `repeat_item_use_no_chain` (2 / 120 s), `console_reject_streak` (3 / 120 s), `escort_separated` (escort > 3x `follow_max_distance` for 5 AI ticks), `escort_leader_teleported` (a followed player is about to be teleported — the escort stays behind). Time-based, re-evaluated every 2 s on movement packets (so only while the player is sending movement): `step_stalled` (step unchanged 5 min), `region_dwell_no_hint` (6 s inside a client-registered region, judged by the client's own hit test `client_would_hint_region` with its exact ceiling (see [client-generic-region-hit-test.md](../reverse-engineering/findings/client-generic-region-hit-test.md)), with no client hint — the post-respawn Throne Room shape), `death_then_silence` (hinting client sends none for 120 s + 100 u after `callForAid`). Event-driven, fire at the gameplay event whether or not the player is moving: `dialog_displaced` (a dialog replaced < 3 s after display, unless the player's choice for the previous dialog was already accepted -- a chain's follow-up to a closed dialog is not a displacement) and `objective_never_completed` (an objective some chain completes on its own via `complete_objective` is still open when a chain force-completes the mission; turn-in objectives that only `complete_mission` closes are excluded). Raised from behaviour, not from knowing the cause |
| `movement.movement_type` | DEBUG (`suppressed`, `cleared`) / TRACE (`deduped`, 1-in-53 with `sampled_1_in` + `suppressed`, `cimmeria-trace`) | `cell::abilities::messaging::broadcast_movement_type` | Every change to an NPC's recorded movement type. **Nothing goes on the wire** (NA10, #779): the client has no server-to-client movement-type receiver and animates NPC gait from `EntityMoved` velocity, so `outcome = suppressed` is the normal case, not a failure. Fields: `kind`, `kind_byte`, `prior_kind`, `outcome` |
| `wire.out.avatar_update` | DEBUG (1-in-101 over all sends; `sampled_1_in`, `suppressed`) | `firehose::log_entity_moved`, from `base::world_entry::cell_dispatch::aoi::entity_moved` | The SigNoz sample of the `wire.firehose.aoi_position` firehose (NA25). What a witness was actually told about an entity: `witness_id`, `entity_id`, position, velocity, `yaw_rad`, **`yaw_byte`**, `pitch_byte`, `pos_variant`, and (NA02) `npc_moved_since_last` — `false` beside a non-zero velocity is an NPC the client animates as running while it stands still. There is no movement-type field: the client animates NPC movement from velocity alone. UPDATE_AVATAR is unreliable and never reaches `wire.out`, so this is the only record of transmitted position/facing |
| `content.resolve` | DEBUG | `cimmeria_content_engine::chain::ChainEngine::resolve_event` | A chain whose **trigger matched but a condition failed** — names the first failing condition (`failed_condition`, `failed_condition_index`, `conditions_total`), the `chain_id` / `chain_name`, `trigger_type` and `source_entity`; `reason = "condition_failed"`. Distinguishes "nothing listens for this event" from "a chain listens but its step is not active yet" — the ordering-bug shape. Generic across every content trigger |
| `cover.detection` | DEBUG | `cell::service::ticks::cover::log_cover_edge` | One row per player cover-set proximity edge (`edge = entered \| left`): position, `crouched`, `nodes_in_set_nearby`, `nearest_node_id` / `nearest_node_dist` / node position, `proximity_radius`. Cover detection is pure proximity and never consults crouch |
| `mission.step_context` | DEBUG | `cell::missions::progression::advance_step` | State that is **already true** when a mission step activates: `regions_inside`, `cover_sets`, `crouched`, `in_combat`, position. Region and cover triggers are edge events, so anything listed here will not re-fire for the new step |
| `movement.navmesh` | INFO (`event = "navmesh_loaded"`) / WARN (`reason = "navmesh_missing"`) | `cell::space_manager::movement_telemetry::log_navmesh_loaded`, `cell::space_manager::lifecycle` | **Which mesh a space is running.** The INFO line fires once per space creation with `path`, `polys`, `verts`, `file_bytes`, `agent_height` / `agent_climb` / `agent_radius`, the full `navmesh_hash` (FNV-1a 64 of the file, 16 hex digits) and `navmesh_short_hash` (first 8). Every per-event navmesh log carries the short form, so this row is the join target for "which mesh build was this session running on?". The WARN is the no-`.nav` case — every navmesh consumer fails open, so NPCs there path in straight lines through geometry |
| `movement.position_sample` | DEBUG | `cell::space_manager::movement_telemetry::sample_accepted_position_at` | **Accepted** player positions, the positive-space counterpart to `movement.validation_reject`. ≤ 1 row per player per 5 s and only after ≥ 1 u of movement; players only (NPCs are covered by `movement.npc` / `npc_ai.tick`). Carries `world`, `space_id`, position, `on_navmesh`, `nav_dy` (height above the walkable surface), `navmesh_hash` and the identity pair. Grouping accepted positions by world builds the walked-surface map that makes a mesh hole visible *before* somebody falls into it |
| `npc_ai.path_fail` | WARN | `cell::service::npc_ai::path_failure::report_path_failure` | One shape for "the pathfinder gave this NPC nothing usable", shared by `fight`, `follow`, `patrol`, `investigate` and `wander`. Carries `state`, `decision_outcome`, `reason` (`no_mesh` \| `no_path` \| `no_start_poly` \| `no_end_poly` \| `no_corridor` \| `partial` \| `degenerate_path`; the four stage reasons arrived with NA02's typed `PathOutcome`), **`fallback`** (`direct_waypoint` \| `path_unchanged` \| `partial_route` \| `path_cleared` \| `snapped_to_mesh` \| `nearest_on_mesh` \| `surface_clamped` \| `held_no_route`), `world`, from/to positions, `dist`, `dy` (the air-climb signature) and `navmesh_hash`. `reason` and `fallback` are independent, and the message follows `fallback`: `fight` enqueues nothing on either of its failure branches (the NPC stands still or keeps a stale route), while the other four (NA41) slide toward the destination across the mesh and stop at the wall or island edge (`surface_clamped`), or stop and hold with their state kept (`held_no_route`); only a space with no navmesh still pushes the raw destination (`direct_waypoint`). **Throttled per NPC** (first immediately, then ≤ 1 / 5 s) with `suppressed = N`. Before this target, `patrol` / `investigate` / `wander` logged *nothing* when `find_path` returned `None` — they pushed the raw destination and walked through geometry silently |
| `npc_ai.path` | DEBUG; `ok` ≤ 1 / 10 s per NPC | `cell::service::npc_ai::path_request::request_path` | `event = "request"`: every AI `find_path` whose `status` is not `ok`, and a per-NPC sample of the `ok` ones (a chaser repaths every tick its target moves; the healthy case is not news), with `suppressed` (NA02). `state`, typed `status` (`ok` \| `partial` \| `no_start_poly` \| `no_end_poly` \| `no_corridor` \| `straighten_failed` \| `no_mesh`), `from` / `to`, `target_id`, `target_is_gm` (audit S14: a GM standing off-mesh fails every chase), `start_snap_dy`, `end_snap_dist`, `n_waypoints`, `max_leg_dy`, `end_to_target_dist`, `end_to_dest_dist`. A `partial` corridor is still walked, and also raises `npc_ai.path_fail reason=partial`. Counter `npc_path_requests_total`, unthrottled |
| `npc_ai.los` | DEBUG, ≤ 1 / 5 s per (looker, target) | `cell::space_manager::spatial::line_of_sight` → `npc_ai::detectors::los` | `event = "blocked"`: every line of sight that is **not** clear (NA02, audit T9). `result` (`blocked` \| `unknown_off_mesh`), raw `from_xyz` / `to_xyz`, `ray_from` / `ray_to` (the projected points the ray ran between), `hit_xyz`, `eye_height_used` / `target_eye_height_used` (`0.0` for the navmesh ray, which runs along the floor; for the occluder the looker's and the target's body-set eye heights, 1.5 m where unmeasured, NA31), `source` (`navmesh` \| `occluder`, NA27), `dy`, `dist`, `navmesh_hash` or `occluder_hash`. Replaces the unsampled `movement.navmesh reason=los_unknown_off_mesh` row |
| `abilities` `event = "los_refused"` | DEBUG, one per refused press | `cell::abilities::use_ability::fire_los` | NA31: a player ability was refused for no line of sight (`onErrorCode` 39). The row carries `entity_id`, `ability_id`, `target_id`, `world`, `source = occluder`, `occluder_hash`, `shooter_eye` / `target_eye`, `from_xyz` / `to_xyz`, `ray_from` / `ray_to` / `hit_xyz` for the eye ray, and `rays` (how many tolerance rays were tried). Counter `abilities_los_refused_total`, label `world`. A WARN `los_refused_send_failed` fires if the error could not be queued |
| `npc_ai.occluder` | INFO once per world at load; DEBUG ≤ 1 / s per world | `cell::space_manager::occlusion` | NA27 occluder paging. `event = "occluder_loaded"` (INFO: `pages`, `packed_bytes`, `full_ram_bytes`, `load_ms`, `occluder_hash`), `"occluder_absent"` (INFO), `"occluder_load_failed"` (WARN), and `"residency"` (DEBUG, only when something changed): `players`, `unpacked` / `evicted` and their page lists, `query_unpacks`, `resident_pages`, `resident_bytes`, `unpack_us_max`. Gauges `npc_ai_occluder_resident_pages` / `npc_ai_occluder_resident_bytes`, label `world` |
| `npc_ai.leash` | INFO / WARN / DEBUG | `cell::service::npc_ai::detectors::leash`, called from `npc_ai::leash` and `combat::generate_threat` | NA02 rows, driven by the NA12 walk home. `event = "enter"` (INFO): the NPC gave up its fight, written once the route home is installed, with `reason` (`leash_out` \| `target_lost` \| `threat_empty`), `trigger` (`beyond_band` \| `chase_outward` \| `vertical_cap` \| `target_dead` \| `target_gone` \| `target_out_of_aoi` \| `threat_empty`), `npc_to_spawn` (horizontal, the distance the leash measures), `target_to_spawn` and target position (absent when the last target was lost), `leash_distance`, NPC position, `nav_path_len` (0 = no route; the snap fallback follows). `event = "arrived"` (INFO): walked home, or a follower reset in place; `event = "snap_fallback"` (INFO): no route or the 20 s walk timeout. Both carry `arrival` (`walked` \| `in_place` \| `snap_no_path` \| `snap_timeout`), `walk_secs`, `path_ok`, `snap_dist`, `stale_path_len` (must be 0), `spawn_on_mesh`. `event = "loop"` (WARN, ≤ 1 / 60 s per NPC): 3 or more leash entries inside 60 s (S5), `leash_count`, `target_id`; counter `npc_leash_loop_total`. Should read zero after NA12. `event = "damage_ignored"` (DEBUG): threat refused because a Leashing NPC evades (S12). `npc_ai::leash` itself adds `event = "replan"` (DEBUG, a Leashing NPC without a route home got one), `event = "player_combat_exit"` (DEBUG, a player's last threatening mob drained, `BSF_InCombat` cleared) and `event = "follow_target_lost"` (WARN, NA42: a follower's leash reset found its leader gone from the space, so the target was cleared and it went Idle; `target_id`). A follower whose leader is still there goes back to Follow (`npc_ai.transition reason = follow_resumed`). The route home is also an `npc_ai.path event=request` with `state = leash` |
| `npc_ai.aggro_scan` | DEBUG, sampled | `cell::service::npc_ai::detectors::aggro_scan` | NA02. `event = "candidate_rejected"` (≤ 1 / 10 s per NPC–player pair): `reason` (`not_player` \| `dead` \| `same_faction` \| `not_hostile` \| `gm_ignored` \| `out_of_vertical_band` \| `out_of_radius` \| `no_los` \| `post_reset_suppressed`, NA13), `player_id`, `npc_to_target`, `dy`, `aggro_radius` (the NPC's radius in u; NA02-era rows say `"unbounded"`). `event = "no_candidates"` (≤ 1 / 30 s per NPC): `witness_count`, `rejected`. NA14: `event = "assist_rejected"` (≤ 1 / 10 s per assister–victim pair and reason): `npc_id` is the neighbour passed over, `victim_id`, `player_id`, `reason` (`dead` \| `not_idle` \| `not_hostile` \| `post_reset_suppressed` \| `gm_ignored` \| `out_of_vertical_band` \| `out_of_radius` \| `no_los`), `ai_state`, `npc_to_victim`, `dy`, `assist_radius`; `event = "assist_joined"` (unsampled, once per join): `npc_id`, `victim_id`, `player_id`, `npc_to_victim` |
| `npc_ai.idle` | DEBUG, ≤ 1 / 30 s per world | `cell::service::npc_ai::detectors::sweep` | `event = "unticked"`: how many Idle NPCs the AI tick skips (no aggression, patrol or wander), per world, with `suppressed`. The sample window is per world and released by `destroy_space`. Gauge `npc_ai_idle_unticked{world}` |
| `npc_ai.idle_parked` | INFO | `cell::service::npc_ai::detectors::idle_parked`, from `set_ai_state_on` | NA02. An NPC changed to Idle more than 2 u from spawn and will not be ticked again (S6 + A1): `from`, `reason` (the transition reason), `npc_to_spawn`, position. Counter `npc_idle_parked_total{world,reason}`. After NA12 only a follower reset in place (`reason = leash_arrived`) or a content / GM Idle away from spawn can raise it. Since NA42 a follower only parks when its leader has left the space; one whose leader is still there resumes Follow |
| `npc_ai` | WARN | `cell::service::npc_ai::detectors::sweep` | NA02, per ticked NPC after its handler. `event = "npc_off_mesh"` (≤ 1 / 30 s; an NPC parked where it spawned since it spawned, `last_move_source = spawn`, warns once and then repeats at DEBUG on the same window, NA24): `gate`, `horizontal_dist`, `dy`, `last_move_source` (`path` \| `fallback` \| `leash` \| `backup` \| `content` \| `spawn`); counter `npc_off_mesh_total{world,gate}`. `event = "stuck"` (≤ 1 / 15 s): Fighting and chasing (with a path, or `no_path` with none: an NPC that never gets a route is the most stuck), and `npc_to_target` has not shrunk by 0.5 over 3 AI ticks — `npc_to_target_history`, `nav_path_len`, `los`, `next_wp`; counter `npc_stuck_total` |
| `npc_ai` | DEBUG | `cell::service::npc_ai::fight_cover::route_via_cover` | NA02, `decision_outcome = "no_cover"` (a log field, not the tick's terminal outcome): replaces the silent `NoCover => {}` arm (audit C7). Sampled ≤ 1 / 10 s per NPC with `suppressed`. `reason` (`no_candidate_in_radius` \| `reserve_lost` \| `in_range_no_better_slot` \| `no_world` \| `index_miss`), `candidates_scanned`, `reserved_skipped`, `search_radius`, `cover_nodes_loaded`. An NPC with `use_cover = false`, or a stationary one, is not logged at all: it never asks, and `spawner.npc_behaviour` already records both |
| `movement.npc` | WARN | `cell::service::npc_ai::detectors::movement` | NA02. `event = "stale_velocity"` (≤ 1 / 10 s per NPC): a non-zero velocity with no displacement for 3 movement ticks — the running-in-place detector (S1). `path_state` = `empty` (the path was cleared mid-leg; this is also the telemetry plan's `animating_without_path`, re-based on velocity because the client animates from velocity alone) or `stalled`; counter `npc_stale_velocity_total`, one per episode (the tick the NPC becomes stale). `event = "ground_deviation"` (≤ 1 / 5 s per NPC): a step or waypoint snap more than 0.3 from the storey-aware floor (NA01's `get_height_near`) on a meshed world — `dir` (`up` \| `down` \| `unknown` when no floor is within jump height), `ground_y`, `dy`, `y_source` (`lerp` \| `waypoint`), `leg_len`, `leg_dy`, `wp_*`; counter `npc_ground_deviation_total{world,dir}`, one per episode (the first off-floor step after a grounded one) |
| `threat` | WARN, ≤ 1 / 30 s per player–NPC pair | `cell::service::npc_ai::detectors::threat` | NA02, `event = "cleared_without_exit"`: an NPC cleared its threat list (`reason` = `threat_empty` \| `leash_out` \| `target_lost` \| `leash_complete`) while a player still lists it in `threatened_mobs` (S7), so the player stays in combat. NA12 drains every player before each leash clear, so a row means a new clear path that skipped the drain. Counter `npc_threat_cleared_without_exit_total{world,reason}` |
| `pets.lifecycle` | INFO / DEBUG / WARN | `cell::pets::spawn`, `cell::pets::teardown`, `cell::pets::owner_hooks`, `cell::pets::arrival` (`cimmeria-cell-world`); `cell::abilities::use_ability::summon` (`cimmeria-cell-combat`); `request_entity_update` (`cimmeria-cell`) | Pets (#570, PT-01, PT-02, PT-03). Every row carries the Rule 5 correlator `entity_id` (the pet; the owner on the rows about no single pet: `summon_failed`, `owner_forgotten`, `owner_left`, `grounding_missed`, and `teleport_skipped` with `reason` = `owner_not_moved` \| `owner_not_found`), `event` (and the same value as `decision_outcome` where it has one), the owner's `account_id` / `player_id` (captured at summon, so a row about a pet whose owner is already gone still names the player; omitted, never `0`, when unknown), `pet_id`, `owner_id` and, where the pet still exists, `template_id`. Despawn rows also carry `path` (`direct` \| `sweep`, or the owner `path` listed at the end). Events: PT-03's summon cast rows (they also carry `ability_id`): `summon_launched` (DEBUG: `warmup_secs`, `cooldown_secs`); `summon_warmup_started` (DEBUG: `warmup_secs`); `summon_interrupted` (DEBUG: `reason` = the warmup interrupt, e.g. `caster_moved` \| `caster_died` \| `ability_unlearned`); `summon_fired` (DEBUG); `summon_replaced_pet` (DEBUG: `replaced_pet_id`, `max_active`); `summon_spawned` (DEBUG: `pet_id`); `summon_refused` (`stage` = `launch` \| `fire` \| `spawn`; DEBUG for `reason = not_trained`, which a client can send at will; WARN for `unknown_template` \| `owner_not_found` \| `owner_dead` and a spawn failure's `PetSpawnError` reason); `summon_feedback_send_failed` (WARN: the refusal's `onErrorCode` / chat line could not be queued, `method_index`); `arrival_vfx_skipped` (DEBUG: `reason = no_sequence`, the 1122/2000 sequence is not loaded); `arrival_vfx_sent` (DEBUG: the summon's target VFX reached at least one of the pet's witnesses, `sequence_id`, `witness_count`, `delivered_count`); `arrival_vfx_undelivered` (WARN: every witness send failed, `reason = cell_to_base_closed`, `witness_count`); `arrival_vfx_dropped` (`sequence_id`, `waited_ms`; WARN with `reason = owner_never_witnessed` after 2 s, `reason = owner_mismatch`, or `reason = owner_identity_mismatch` when the owner's entity id now belongs to a player who is not the summoner, each WARN row with `registered_owner_id`; DEBUG with `reason = pet_gone`); `arrival_vfx_send_failed` (WARN: `witness_id`, one row per failed send). The summon cast rows carry `entity_id` = the owner. The arrival rows carry `entity_id` = the pet and the pet's `template_id`, which is captured at summon, so a row written after the pet is gone still has it. PT-01/PT-02 rows: `summoned` (INFO: `ability_id` = summon ability, `space_id`); `summon_failed` (WARN: `reason` = `owner_not_found` \| `owner_not_player` \| `unknown_template` \| `spawn_failed`, `error`); `despawned` (INFO: `reason` = `owner_disconnected` \| `owner_gone` \| `owner_dead` \| `owner_left_space` \| `dismissed` \| `corpse_expired` \| `expired`, `witnesses_notified`); `despawn_failed` (WARN: `reason` = the despawn reason as above, `outcome` = the `DespawnOutcome` that removed nothing); `owner_forgotten` (DEBUG, from `disconnect_entity`: `pet_count`, `despawned`); `registry_scrubbed` (WARN: the sweep found a registry entry with no pet entity, a missed teardown path; `reason = pet_entity_gone`); `pet_list_replay_failed` (WARN: `reason = cell_to_base_closed`, the owner-only list re-emit could not be sent); `pet_list_replay_refused` (WARN: `reason = owner_identity_mismatch`, the witness holds the owner's entity id but is not the player who summoned the pet, so the owner-only lists were withheld on AoI entry or `requestEntityUpdate`; carries `witness_id` and the new holder's `witness_account_id` / `witness_player_id` beside the summoner's `account_id` / `player_id`); `owner_left` (DEBUG, PT-02: an owner hook is despawning `pet_count` pets; `entity_id` = the owner, `reason`, `path`); `owner_teleported` (DEBUG, also `decision_outcome = teleported_with_owner`: `path`, `space_id`, `from_x/y/z`, `to_x/y/z`, `distance`, `grounding` = `navmesh` \| `no_navmesh` \| `owner_off_mesh`, `witnesses_notified`); `teleport_skipped` (DEBUG `reason` = `pet_dead` \| `pet_in_other_space` \| `owner_not_moved` (a ring passenger reappeared without having been moved: an aborted or failed trip; with `pet_count`) \| `owner_identity_mismatch` (the teleported player holds the owner's entity id but did not summon this pet, which is despawned as `owner_gone`; carries `holder_account_id` / `holder_player_id`); WARN `reason = owner_not_found`, a caller bug, with `pet_count`); `grounding_missed` (DEBUG: the teleport spot was not walked on the navmesh, `reason` = `no_navmesh` \| `owner_off_mesh`); `teleport_relay_failed` (WARN: an `EntityMoved` could not be queued, `witness_id`, `reason = entity_moved_send_failed`); `corpse_timer_started` (DEBUG: a dead pet's 10 s timer started, `corpse_secs`); `corpse_expired` (DEBUG: the timer ran out, followed by `despawned reason=corpse_expired path=sweep`). Owner `path` values: `disconnect`, `base_destroy`, `owner_death`, `respawn`, `gate_travel`, `space_transfer`, `gm_travel`, `console_travel`, `console_location`, `content_teleport`, `ring`, `pet_left_behind` (PT-05: not an owner move, the pet AI brought a left-behind pet back; the `pets.ai event=teleported` row says why). A pet hook runs only once the owner's own move was sent: when a `TeleportPlayer`, `ReanchorPlayer` or cross-world `GateTravel` send fails (content `teleport`, respawn both ways, content and ring cross-world teleport), the owner path logs a WARN (ERROR on the two content/ring cross-world sends) on its own module target with `reason = cell_to_base_closed`, the owner and its pets stay where they are, and no `pets.lifecycle` row follows |
| `pets.command` | INFO / DEBUG / WARN | `SpaceManager::owned_pet` (`cimmeria-cell-world`), `cell::console::pet`, `cell::console::dispatch` (`cimmeria-cell-console`) | Pets (#570). `event = "ownership_rejected"`: a caller named a pet it does not own. `reason` = `not_owner` \| `not_a_pet` \| `pet_gone` \| `owner_identity_mismatch` (the caller holds the owner's entity id but is not the player who summoned the pet: the id was reused), `entity_id` = `caller_id` + its `account_id` / `player_id`, `pet_id` (the claimed id), `owner_id` (the real owner, `not_owner` only). DEBUG because a client can send any id at will; PT-04's handlers add the visible feedback, not a second log. PT-04 adds its span and command rows here. The GM `.pet` console (PT-07) runs in the info span `pets.command` (`entity_id`, `verb`, `account_id`, `player_id`) and logs one event per outcome, all carrying the caller's `account_id` / `player_id`, with `decision_outcome`: `gm_summoned` (INFO: `pet_id`, `owner_id`, `template_id`, `ability_id` (0 for a template id), `replaced`); `gm_dismissed` (INFO: `owner_id`, `pets`); `gm_stance_set` (INFO, `event = stance_changed`: `pet_id`, `owner_id`, `from`, `stance`); `gm_inspected` (DEBUG, `verb` = `info` \| `list`; `info` on another owner's pet adds `subject_player_id`); `gm_refused` (DEBUG, a GM or player can trigger each at will: `reason` = `not_gm` \| `bad_args` \| `unknown_id` \| `not_a_player` \| `owner_dead` \| `spawn_failed` \| `no_pet` \| `pet_gone` \| `bad_stance` \| `stance_not_allowed` \| `unknown_verb`, optional `pet_id`); `send_failed` (WARN, `reason = cell_to_base_closed`). The spawn and despawns are logged again on `pets.lifecycle`. PT-04's pet-bar cell methods (88/89/90) add their own values; PT-11 adds CM 88 `reason = ability_not_implemented` (DEBUG: the ordered ability has no event set and no effect that deals damage or runs a script, so it is refused before the pet casts) |
| `pets.credit` | DEBUG / WARN / ERROR | `SpaceManager::credit_recipient` (`cimmeria-cell-world`); `cell::abilities::death::side_effects` (`cimmeria-cell-combat`) | Pets (#570, PT-01 + PT-06). Kill credit for kills that involve a pet. Every row carries `entity_id` = `pet_id` and `owner_id`, and the summoner's `account_id` / `player_id` from the pet's summon-time capture (omitted, never `0`, when unknown; never read from whoever holds `owner_id` now). `event = "credit_refused"` (WARN, `credit_recipient`): the owner's entity id no longer belongs to the summoner, so the kill pays nobody; `reason` = `owner_identity_mismatch` (another entity holds the id) \| `owner_gone` (no entity holds it), plus the live holder's `holder_account_id` / `holder_player_id`. A refused kill writes exactly one row: it comes from `grant_kill_xp`'s call, while mission credit and the per-cast routing gates use the non-logging `credit_recipient_quiet`, and no `kill_xp_not_granted` row follows it. `event = "pet_kill_credited"` (DEBUG): a pet's kill paid its owner, written only after the owner's `GrantXP` was handed to the base; `victim_id`, `victim_template_id` (omitted when `None`), `victim_level`, `base_xp`, `xp_granted`, `transfer_xp`. `event = "pet_kill_credit_undelivered"` (ERROR, `reason = send_failed`): the same fields plus `error`, when that send failed (the cell-to-base channel is closed) and the XP is lost; it replaces the module-target "player kill credit lost" line for a pet kill. The cell holds no XP total, so `xp_before` is not logged here: the base's `progression.grant_xp` span for the owner's `entity_id` has the before/after. `event = "kill_xp_not_granted"` (also `attacker`, `victim_id`, `base_xp`) with `reason`: `npc_killed_pet` (DEBUG, a mob killed a pet; `pet_id` is the dead pet, `attacker` the mob), `pet_unregistered` (DEBUG, the killer still carries pet state but the registry already dropped it, the teardown gap; `account_id` / `player_id` are omitted because `forget_pet` removed the capture), `zero_xp` (DEBUG, the payout rounds to 0), `transfer_xp_invalid` (WARN, the pet's `transfer_xp` is non-finite or not above zero), `xp_overflow` (WARN, the payout exceeds `MAX_KILL_XP` = `i32::MAX`, the width of `sgw_player.exp` and the wire payload, on every path including `transfer_xp = 1.0` with a corrupt huge victim level; no XP is paid instead of a clamped or saturated grant). Both WARN reasons carry `transfer_xp` and `max_kill_xp`. Kills with no pet log the same `event` on the module target, not here: `reason = npc_attacker` (DEBUG, an NPC killed an NPC) and `reason = xp_overflow` (WARN, a player's own over-cap kill). Exported by the `pets=debug` prefix row (pinned in `logging/pets_target_tests.rs`) |
| `pets.ai` | DEBUG / WARN | `cell::service::npc_ai::pet` (`cimmeria-cell-combat`), and its hooks in `combat::generate_threat` | Pets (#570, PT-05): the owner-relative pre-pass the AI tick runs for every pet, the pet branches of the leash and `generate_threat`. One row per decision. Every row carries `event`, the matching `decision_outcome`, the Rule 5 correlator `entity_id` (the pet), `pet_id`, `owner_id` and the owner's `account_id` / `player_id` (the pet's summoner, captured at summon, so a row still names the right player when the owner id is gone or reused; absent, never `0`, when unknown), plus `target_id` where there is one. Events: `follow_armed` (`pet_follow_armed`: put in Follow on its owner, `from` = the state it left); `follow_rearmed` (`pet_follow_rearmed`: a fight ended and the pet follows again instead of leashing; `reason` and `trigger` as `begin_leash` got them, e.g. `leash_out` / `beyond_band`, `target_lost` / `target_dead`, `threat_empty`, or `pet_follow` with `trigger` = `passive_stance` \| `leashing`; `threat_count`); `teleported` (`pet_teleported`: why the pet was brought back, `reason` = `distance` \| `floor_band`, `distance`, `dy`; the move itself is the `pets.lifecycle event=owner_teleported` row with `path = pet_left_behind`); `teleport_rate_limited` (`pet_teleport_rate_limited`: left behind within 5 s of the last teleport, `reason`, `distance`, `dy`, `since_last_ms`); `engaged` (`pet_engaged`: a stance picked `target_id`, `why` = `defend_owner` \| `defend_self` \| `owner_target` \| `aggressive_scan`); `engage_refused` (WARN, `pet_engage_refused`: `pet::engage_pet_target` refused a target the stance picked, an invariant violation; `reason` = its refusal: `target_other_space` \| `target_not_combatant` \| `target_not_hostile` \| `target_dead` \| `target_resetting` \| `target_not_engageable` \| `target_gone` \| `target_refused_threat` \| `owner_gone` \| `owner_identity_mismatch` \| `not_a_pet`); `fight_entered` (`pet_fight_entered`: a hit or a content threat preempted the pet into Fighting, `cause`, `from`); `passive_ignored` (`pet_passive_ignored`, `reason = passive_stance`: a Passive pet refused threat, `cause`); `threat_refused` (`pet_threat_refused`, `reason` = `attacker_other_space` (the attacker is not in the pet's space) \| `attacker_not_hostile`: a pet refused threat from something it may not fight (`pet::fight_refusal`): a player, another pet, an `SGWBeing`, or an NPC its owner could not attack (`combat::player_may_attack`); `target_id` = that attacker, `cause`); `target_dropped` (`pet_target_dropped`: a fighting pet let a target go, `reason` = `target_other_space` \| `target_not_combatant` (not an `SGWMob`: a being, player or pet) \| `target_not_hostile` (its owner could not attack it) \| `target_dead` \| `target_resetting` \| `target_not_engageable` (surrendered, `Submit`; or spawning / error) \| `target_just_reset` \| `target_far_from_owner`); `cross_space_refused` (DEBUG, `pet_cross_space_refused`, `reason = target_other_space`: `engage_pet_target` refused a target outside the pet's space before seeding either side, for a stance pick or an owner order (whose target id comes from the client); `pet_space_id`, `target_space_id`, `kind`); `attackers_released` (`pet_attackers_released`: the pet was removed from mobs' threat lists as it left them, `reason` = `target_invalid` (a target dropped as not combatant, not hostile, resetting, surrendered or just reset) \| `called_off` (the pet re-armed after a switch to Passive or a content-forced Leashing), `released` = how many, `target_ids`; a leash pull-back or a target left behind by distance releases nothing); `owner_combat_entered` / `owner_combat_left` (the pet's fight put its owner in combat with `target_id`, or nothing explains the owner's entry any more; `new_state`); `owner_missing` (`pet_owner_missing`, `reason` = `owner_gone` \| `owner_identity_mismatch` (the owner's entity id now belongs to a player who did not summon the pet) \| `owner_other_space` \| `owner_dead`: the pet holds until `pet_owner_sweep` despawns it). No per-handler spans (the AI tick's `npc_ai.decision` span covers the turn). Exported by the `pets=debug` row |
| `pets.command` | INFO span / WARN / DEBUG | `cell::cell_methods::player::pet` (`cimmeria-cell-methods`) | Pets (#570, PT-04). Owner pet commands: CM 88 `invoke_ability`, 89 `ability_toggle`, 90 `change_stance` (field `command`). **Span:** each handler runs in an INFO span `pets.command` with `command`, `entity_id` (the caller), `pet_id` (as claimed), plus `ability_id` / `target_id` (88), `ability_id` / `toggle` (89) or `requested` (90). **Identity:** every event carries `owner_id` (the caller's entity id) and the owner's `account_id` / `player_id` from `SpaceManager::player_identity` (omitted when unresolved, never 0). **Accepts** (DEBUG, `event` = `decision_outcome`): `invoked` (`pet_id`, `template_id`, `ability_id`, `target_id`, `engaged`, `engage_deferred` when the cast is warming up); `order_engaged` (from `use_ability::warmup::pet_order` in `cimmeria-cell-combat`, when a warmed-up ordered cast fires: `entity_id` = `owner_id`, `pet_id`, the summoner's `account_id` / `player_id`, `template_id`, `target_id`, `fired_target_id`, `engaged`, and when it did not engage `reason` = `target_changed` or any `engage_pet_target` refusal; a refusal other than `target_dead` / `target_changed` also sends the owner `onErrorCode` (`InstanceID` = the ability id) plus a `CHAN_FEEDBACK` line, and a closed channel logs WARN `order_feedback_send_failed`); `order_interrupted` (same module, from `interrupt_pending_cast`: an ordered cast's warmup was interrupted, the order is dropped and the owner gets the same feedback; `reason` = the `InterruptReason` label, `ability_id`, `target_id`); `toggled` (`pet_id`, `template_id`, `ability_id`, `on`); `stance_set` (`pet_id`, `template_id`, `requested`, `source` = `stance_id` \| `slot_index`, `stance_before`, `stance_after`, `order_dropped` when a switch to Passive dropped an order still warming up). **Refusals** (`decision_outcome = "rejected"`, `reason`, `pet_id`, and `error_code` / `instance_id` for the `onErrorCode` sent to the owner). The ownership guard (CAT-C-11 / #462) refusals are the `ownership_rejected` row above, logged once by `SpaceManager::owned_pet`; the handler adds only the feedback (236 `IsNotPetOwner`, or 190 `DoesNotHavePet` for `pet_gone`). Every `onErrorCode` a pet command sends is paired with a `CHAN_FEEDBACK` chat line (`pets::order_feedback_text`). WARN: `malformed_args` (`args_len`, `expected_len`); `ability_not_in_list` for an ability the server has a definition for; `cast_refused` (a pre-checked cast `handle_use_ability` still refused); `feedback_send_failed` and `owner_send_failed` (base channel closed). DEBUG (ordinary play, or forgeable values that must not flood the WARN index): `owner_dead`, `pet_other_space`, `ability_not_in_list` for an id with no ability definition, `ability_toggled_off`, `pet_dead`, `pet_casting`, `ability_on_cooldown`, `target_gone`, `target_other_space`, `target_not_combatant` (a player, pet or SGWBeing: PT-05's `fight_refusal`), `target_not_hostile` (fails `combat::player_may_attack`), `target_dead`, `target_resetting` (Leashing or Despawning), `target_refused_threat` (the engagement after an instant cast), `out_of_range`, `no_line_of_sight`, `bad_toggle_value` (`toggle`), `stance_not_allowed` (`requested`, `current`). Pinned by `player/pet/tests/telemetry.rs`. Exported by the `pets=debug` prefix row |
| `pets.buff` | DEBUG / INFO / WARN | `cell::pets::buffs`, `cell::effects::pet_scripts` (`cimmeria-cell-world`); `cell::abilities::use_ability::owner_pet` (`cimmeria-cell-combat`) | Pets (#570, PT-08): owner abilities that act on the owner's pet (Holy Warrior, To The Death, Lord's Concentration, the pet heals) and the Heed Our Calling passive. Every row carries `event`, the matching `decision_outcome`, and the owner's `account_id` / `player_id` (the pet's summoner, captured at summon, on rows about a pet; `SpaceManager::player_identity` on rows about the cast; omitted, never `0`, when unknown). Rows about a pet carry `entity_id` = `pet_id`, `owner_id` and `template_id`; rows about a cast carry `entity_id` = `owner_id` and `ability_id`. Events: `buff_applied` (DEBUG: `ability_id`, `effect_id`, `toggle`, `duration_secs` when timed, `stat_deltas`, `stats_before`, `stats_after` as `(stat id, value)` pairs); `buff_removed` (DEBUG: `reason` = `toggled_off` \| `expired` \| `refreshed` \| `removed`, the same stat fields); `doom_armed` (DEBUG: To The Death's timer started, `doom_secs`); `doom_fired` (INFO: the timer ran out and the pet was killed, `decision_outcome` = `pet_killed` \| `kill_not_applied`, `health_before`, `killed`, `xp_granted = 0`); `doom_skipped` (DEBUG, `reason = pet_dead`: the pet had already died); `owner_ability_applied` (DEBUG: the cast landed, `pet_id`, `pet_ids`, `effect_ids`); `owner_ability_refused` (`stage` = `launch` \| `fire`, `reason`, `error_code`, `pet_ids`: DEBUG for `no_pet` \| `pet_dead` \| `pet_other_space` \| `pet_doomed`, which a client can cause at will; WARN for `owner_identity_mismatch`, the reused-id window); `feedback_send_failed` (WARN, `reason = cell_to_base_closed`, `method_index`); `summon_speed_applied` / `summon_speed_removed` (DEBUG, the Heed Our Calling passive on the owner: `speed_pet_before`, `speed_pet_after`); `pet_script_skipped` (WARN, a seed defect: `reason` = `target_not_a_pet` \| `no_stat_nvps` \| `no_duration` \| `no_speed_nvp`, `script`). Pinned by `use_ability/owner_pet/tests/telemetry.rs` and `logging/pets_target_tests.rs`. Exported by the `pets=debug` prefix row |
| `spawner.npc_behaviour` | WARN, once per spawn id | `cell::service::npc_ai::detectors::spawn` | NA02, `event = "spawn_off_mesh"`: the spawn fails `find_path`'s ±0.5 start box (S9) even when `on_navmesh` (the looser `is_point_valid`) is true. `gate`, `horizontal_dist`, `dy`, `snapped_y`. Counter `npc_spawn_off_mesh_total{world,gate}` |
| `cover.coverage` | INFO, WARN when unusable | `cell::cover::coverage::log_space_coverage` | NA02, `event = "space_summary"`, one row per space once it has its NPCs and the cover index has loaded (startup spaces from `SpaceManager::cover_loaded`, instanced spaces after `spawn_instance_npcs_from_records`). Reads the world-scoped index (NA21): `world_id`, `nodes_in_world`, `nodes_on_mesh` (a node counts when `NavMesh::get_height_near` around its own Y finds a floor within 1.0), `sets_in_world`, `cover_npcs` (NPCs with `use_cover` that are not stationary). WARN `reason = "no_usable_cover"` when a meshed space has cover-seeking NPCs and no usable node. With the NA21 seed, Castle_CellBlock (world 12) reads 236 nodes, 211 on the mesh, 58 sets |
| `cover.selection` | DEBUG, ≤ 1 / 10 s per NPC | `cell::service::npc_ai::fight_cover` | NA02. `event = "picked"`: the best free node's `chunk_id`, `node_id`, `score`, `move_dist`, `threat_dist`, `scanned`. `event = "rejected"`: the top 3 losers with `rank` and `reason` (`reserved` \| `lower_score`) |
| `wire.out.forced_position` | DEBUG | `base::world_entry::teleport::handle_teleport_player` | NA02. Every `FORCED_POSITION` sent: position, previous position, `snap_dist`, `reason`. All are player snaps today — no NPC snap (the leash included) is sent as a forced position; witnesses learn of it from the next AoI `EntityMoved` |
| `movement.navmesh` | INFO | `cell::space_manager::navmesh_mode::log_navmesh_summary` | `reason = "navmesh_mode_summary"`: one line at startup per resident space that **has** a mesh — `space_id`, `world_name`, `navmesh_mode` (`enforce` \| `advisory`), `poly_count`, `spawn_rows`, `spawn_rows_off_mesh`. A high `spawn_rows_off_mesh` on an `enforce` world is an invisible-wall report waiting to happen: the mesh loaded, but it does not describe the map players walk, and the holes are hard gates for everyone except GMs. INFO rather than WARN because an advisory world is expected to have a high count — the actionable signal is the number moving. See [navmesh-containment-modes.md](navmesh-containment-modes.md) |
| `movement.navmesh` | TRACE (level-gated), ≤ 1 / 500 ms per player | `cell::space_manager::client_move` | `reason = "advisory_off_mesh_accepted"`: an off-mesh position an `advisory` world accepted — `entity_id`, `space_id`, `world`, `client_x` / `client_y` / `client_z`, `suppressed`. Guarded by `tracing::enabled!` **before** the Detour query. No layer enabled it before NA25, so it never fired; the `cimmeria-trace` index now does whenever OTLP is on, so it is throttled per player (`ADVISORY_OFF_MESH_LOG_INTERVAL`, a breadcrumb every ~3 units at run speed). This is the real player traffic a mesh rebake needs (which parts of the world people actually walk through), as opposed to a static grid probe |
| `movement.navmesh` | WARN | `cell::space_manager::navmesh_mode::mode_from_db_value` | `reason = "navmesh_mode_unrecognised"`: `resources.worlds.navmesh_mode` held a value this build does not know (`world_name`, `raw_value`). Falls back to `enforce` — containment stays on |
| `navmesh.load` | ERROR | `entity::navigation::check_count` | Hostile `.nav` header rejected — space loads navmesh-less |
| `bank` | WARN | `base::world_entry::methods::inventory::move_::container_policy`, `inventory::grant::validation` | Bank and Vault campaign (BV-01). Every row carries an `event` field naming it. `move_rejected`: a `moveItem` into or out of a container the player may not move (`reason` = `source_container_not_player_movable` \| `target_container_not_player_movable` \| `source_container_needs_vault_session` \| `target_container_needs_vault_session`), with `account_id`, `player_id`, `entity_id`, `item_id`, `type_id`, `quantity` (as requested; `<= 0` is the whole stack), `stack_size`, `source_container_id` / `source_slot_id` (the item's position when refused) and `target_container_id` / `target_slot_id`. `move_resync_skipped`: nothing was resent (`reason` = `refused_item_not_owned` \| `lock_timeout` \| `resync_read_failed`). `grant_rejected`: a grant into 17-20 (`reason = grant_into_storage_container`), with `account_id`, `player_id`, `entity_id`, `type_id`, `quantity`, `target_container_id`. `OTEL_FILTER` exports the whole target at DEBUG (`bank=debug`, added by BV-02 for the cell rows below) |
| `bank` | DEBUG (`vault_session_opened`, `vault_session_closed`) / WARN (`vault_open_rejected`, `vault_open_send_failed`, `bank_feedback_send_failed`); INFO spans `bank.banker_interact`, `bank.console_open` | `cell::interactions::bank`, `cell::console::bank`, `cell::space_manager::vault_session_end` | The vault (bank-vault campaign, BV-02; the event catalog is D-BV19 in `docs/analysis/bank-vault/work-packets.md`). Every row carries `account_id`, `player_id` and `entity_id`. `vault_session_opened`: `scope`, `banker_id` (absent for GM `.bank`), `gm_override`, `space_id`, `distance`. `vault_session_closed`: `reason` (`space_change` \| `logout` \| `re_pin`), `scope`, `banker_id`, `gm_override`, `space_id`, `open_ms`. `vault_open_rejected`: `reason` (`out_of_range` \| `org_vault_not_available` \| `not_gm` \| `banker_missing`), `banker_id`, `distance`. The send-failure rows carry `reason = base_channel_closed`. A lookup miss on the player's own entity is a WARN negative log on the `cimmeria_cell_interactions` target (`reason = player_entity_missing`), not a `bank` event. BV-03 adds `move_accepted` and its own `move_rejected` reasons, BV-05 `expand` and `expand_rejected` |
| `bank` | INFO (`gm_action`; WARN when the server, not the GM, caused the refusal); INFO spans `bank.console_dump`, `bank.gm_dump` | `cell::console::bank`, `base::bank_dump` | GM vault tools (bank-vault BV-04). `gm_action` `action = bankdump`: the GM's `.bankdump [name]`. Every row carries the GM's `account_id`, `player_id` and `entity_id`, plus `result` (`ok` \| `refused`). `result = ok` adds `target_player_id`, `target_name` (when a name was typed), `item_count` and `bank_slots`. Refusals carry `reason`: `target_not_found` (INFO, from the base), `caller_not_player` and `not_gm` (INFO, from the cell), `db_unavailable`, `query_failed` and `base_channel_closed` (WARN, with `error`) |

#### Saved views for reading a playtest

Three Logs Explorer views live under the `playtest` category in SigNoz: **Playtest: bookmarks (.bug notes)** — start here, pick a `bookmark_id`; **Playtest: friction (stuck-player detectors)** — every `playtest.friction` and `movement.navmesh` warning; **Playtest: position trail** — `movement.player`, `movement.npc`, `wire.out.avatar_update`, `movement.movement_type` and bookmark rows interleaved with position / waypoint / `yaw_byte` columns, so narrowing the time range to ±30 s around a bookmark shows where everyone was, where they were going, and what the client was sent. Add `AND entity_id = <id>` or `AND npc_id = <id>` to follow one actor.

Nine more views live under the `npc-ai` category (names prefixed **NPC AI —**), one per question an NPC AI playtest raises, beside the **Cimmeria — NPC AI health** dashboard that charts the `npc_*` metrics below. The [NPC AI telemetry runbook](../operations/npc-ai-telemetry-runbook.md) says which one answers which question; [operations/signoz/npc-ai-views.md](../operations/signoz/npc-ai-views.md) holds every filter and the dashboard JSON export for re-import.

#### `npc_ai.decision_outcome` enum

The `npc_ai.decision` event carries a `decision_outcome` field with
one of these values, letting SigNoz answer "which zones / NPCs are
failing to engage and why" via a single `groupBy=decision_outcome`:

| `decision_outcome` | Meaning |
|---|---|
| `attack_in_place` | In range + LOS + ability ready — NPC fires |
| `chase` | Out of range / LOS — pathfinding toward target |
| `no_path` | WARN — pathfinder returned no path (typically: zone missing navmesh). Raised from INFO when it moved onto the throttled `npc_ai.path_fail` target; a stuck NPC is a standing condition, and at INFO it was one row per tick forever |
| `min_range_backup` | Target inside ability `min_range` — stepping back |
| `step_back` | NA32: a ranged NPC's target is inside its 2 u comfort range — stepping back to 5 u. Carries `comfort_range`, `retreat_distance`, `backup_x/y/z` |
| `step_back_walking` | NA32: the step-back is still being walked during its 3 s cooldown; the attack arm is skipped so the route is not cut short |
| `step_back_cooling` | NA32: target inside a hard `min_range` during the cooldown — holding fire |
| `step_back_cornered` | NA32: the step-back slide gained under 0.5 u (back to a wall) — firing from where it stands, cooldown started |
| `no_ability` | Every known ability on cooldown / needs ammo |
| `leashed` | The NPC itself went past its leash radius from spawn (NA12; before NA12 the test was the target's distance), or its target was unreachable (NA15: `trigger=unreachable`, `cause` = `held_unreachable` \| `off_mesh`). The row carries `trigger` (`beyond_band` \| `chase_outward` \| `vertical_cap` \| `unreachable`), `npc_to_spawn` (horizontal, the distance the test uses), `npc_dy_from_spawn`, `target_to_spawn` (the pre-NA12 metric, for comparison) and `leash_distance` |
| `threat_empty` / `target_lost` | Fight over: the threat list was empty, or its last target died, vanished or stayed out of the NPC's AoI for 5 s. The NPC starts walking home (`npc_ai.leash event=enter`) |
| `leash_walking` / `leash_replan` | Leashing tick: walking the route home / planned a fresh route (none, or a stale one) |
| `leash_arrived` / `leash_snap_fallback` | Leash finished: walked home, or snapped because no route existed or the walk passed 20 s |
| `reaggro_suppressed` | Idle auto-aggro skipped: inside the 5 s window after a leash reset |
| `repath_degenerate` | WARN — chase repath returned ≤1 waypoint; since NA15 the stale path is cleared (`fallback=path_cleared`) and the NPC holds |
| `hold_no_repath` | Out of range / no LoS, but the route is still good for where the target is (NA15: the goal moved at most 5 u horizontally and 1.5 u vertically since the route was planned) — no new order this tick (previously silent) |
| `hold_unreachable` | NA15 — at the end of a route that cannot reach the target (partial corridor, off-mesh target, degenerate repath): standing still with zero velocity, no new route until the target moves; gives up after 8 s (`leashed trigger=unreachable`) |
| `follow_no_path` | WARN — follow found no usable navmesh path. `fallback` says what it did: `surface_clamped` or `held_no_route` on a meshed world (the follower keeps its target and the Follow state), `direct_waypoint` (a straight line at the follower's own height) only with no navmesh (NA41). `dy` is the air-climb signature |
| `follow_target_lost` | WARN — follow target no longer resolves; follow is cleared and the escort idles until a chain re-arms it |
| `follow_dropped_no_target` | Follow state with no follow target — dropped to Idle |
| `stationary_holds` | Stationary NPC out of range / no LOS — holds fire |
| `stay_in_cover` | NPC holds a cover slot that still defends and reaches the target. Terminal while it walks to the slot; once it stands there the attack branch's outcome (`attack_in_place` with `in_cover=true`) is terminal ([cover-system.md](cover-system.md)) |
| `move_to_cover` | NPC picked a fresh cover slot that reaches its target (NA22: also when already in range) — paths to it |
| `cover_released_flanked` | Threat flanked the cover — released, re-eval next tick |
| `cover_released_out_of_range` / `cover_released_unreachable` / `cover_released_stale` | NA22 — the target left attack range from the slot / no route to the slot (the next seek waits 4 s) / the reservation named a node the index lacks |
| `patrol_continue` | Patrol tick walking toward the current waypoint |
| `patrol_dwell` | Patrol tick paused at a waypoint after arrival |
| `wander_pick` | Wander tick chose a fresh destination within radius |
| `wander_dwell` | Wander tick paused at the current destination |
| `investigate_arrived` | Investigate tick reached the POI — dwell starts |
| `investigate_routed` | Investigate tick pathfinding toward the POI |
| `patrol_no_path` | WARN — patrol leg found no usable navmesh path; the NPC slides toward the waypoint across the mesh or holds (`fallback`, NA41), and walks the raw straight line only with no navmesh. Log field on `npc_ai.path_fail`; the handler's terminal outcome stays `patrol_continue` |
| `investigate_no_path` | WARN — same shape for an investigate leg. Terminal outcome stays `investigate_routed` |
| `wander_no_path` | WARN — same shape for a wander hop. Terminal outcome stays `wander_pick` |
| `chase_partial` / `follow_partial` / `patrol_partial` / `investigate_partial` / `wander_partial` | WARN, NA02 — the route is a partial corridor to the edge of the NPC's mesh island; it is still walked. Log field on `npc_ai.path_fail reason=partial`; the terminal outcome is unchanged (`chase`, `follow_band`, ...) |
| `no_cover` | DEBUG, NA02 — the cover step found no slot, with a `reason`. Log field only: the chase or attack branch that follows sets the terminal outcome |
| `follow_band` | Follow target is inside the band — no work |
| `despawn` | Despawn tick — entity is being removed from the space |
| `submit_init` | Submit tick — the pass that actually disengages both sides: player-side threat scrub, auto-cycle sweep, channel cancel, cover release, re-face. Re-fires if somebody re-engages a surrendered NPC |
| `submit_hold` | Submit tick — nothing left to clean, the NPC is parked. The steady state for a surrendered NPC, one row per ~2 s AI tick for the space's life. A `submit_init` where you expect `submit_hold` means something keeps re-aggroing it |
| `error_hold` | Error state — diagnostic quiescent fallback |
| `pet_teleported` | Pets PT-05 — the pet's pre-pass teleported it beside its owner (more than 40 u behind, or on another floor) and used the turn. Why on the `pets.ai event=teleported` row, the move on `pets.lifecycle event=owner_teleported path=pet_left_behind` |
| `pet_owner_missing` | Pets PT-05 — the pet's owner is gone, dead or in another space; the pet holds until `pet_owner_sweep` despawns it. Every other pet turn records the outcome of the state handler it runs (`follow_band`, `chase`, `attack_in_place`, ...) |

Successor PRs may add `patrol_arrived` / `wander_waypoint_set` / etc.
as sub-state breadcrumb `event = "..."` discriminators (see
[instrumentation-discipline.md §rule-2](instrumentation-discipline.md#rule-2--every-state-transition-gets-a-debug-level-event-with-event--)).
The enum above is the **terminal** decision-outcome — the single
value `Span::current().record("decision_outcome", ...)` settles on per
tick — not the per-transition event log.

### Metrics

A third OTLP signal — alongside traces and logs — ships counters,
histograms, and up/down counters from
[`crates/observability/`](../../crates/observability/) (the
`cimmeria-observability` crate). The facade exposes thin macros:

```rust
use cimmeria_observability::{counter, histogram, gauge_add};

counter!("trade_swaps_total", "outcome" => "completed");
histogram!("trade_swap_duration_seconds", elapsed_secs, "outcome" => "completed");
gauge_add!("cover_slots_held", 1, "world_name" => "Castle");
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
`reason`, `kind`, `world_name`, `decision_outcome`). High-cardinality
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
