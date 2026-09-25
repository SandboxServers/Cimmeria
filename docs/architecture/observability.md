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
  The emitters and N live in `cimmeria_services::firehose`.

| Firehose (file) | Sample | Index | N | Why this N |
|---|---|---|---|---|
| `wire.firehose.decrypt` `DECRYPT_OK` (`base.log`) | `wire.sampled.decrypt`, with `hex` | `cimmeria-trace` | 53 | ~6 packets/s idle, ~20 moving: one hex sample every 2.5–9 s per client |
| `wire.firehose.udp_in` `UDP_IN` (`base.log`) | `wire.sampled.udp_in`, `len` only | `cimmeria-trace` | 53 | Same stream as `DECRYPT_OK`. No hex: pre-login datagrams carry the `baseAppLogin` ticket |
| `wire.firehose.aoi_position` `AoI: entity position update` (`world_entry.log`) | `wire.out.avatar_update` (the NA00 row) | `cimmeria-server` | 101 | Was 100. A shared counter samples only pairs at multiples of `gcd(N, pairs per tick)`, so at exactly 50 pairs per tick 100 always hit the same pair; a prime covers every pair below 101 |

All three N are primes for the same reason: a client's packet mix and the
AoI relay order are periodic, and a composite N can phase-lock the sample
onto one packet kind or one pair.

`crates/server/src/logging/parity_tests.rs` builds the production filters
on recording layers and, for every directive of every file layer, fires a
representative event at TRACE, DEBUG and INFO: an event the file keeps
must reach exactly one OTLP index, or, for a firehose, none, with its
sample reaching one. It also checks `server.log`'s targets, that no
target at any level reaches two indexes, and that the guard itself
catches a file layer added without `OTEL_FILTER` coverage.

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
| `wire.in` / `wire.out` | INFO | `wire_log::{log_inbound, log_outbound_entity_method}` | Decoded entity-method calls. `wire.in` resolves cell methods by **method index** (`msg_id - 0x80`, or `61 + sub_index` for the `0xBD` sub-slot form) and carries `method_index` + `entity_method`; base methods (`0xC2+`) are `baseMethod` with their index until a base name table exists |
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
| `cover.reservation` | WARN | `cell::cover::ai_integration::try_reserve_or_warn` | Cover-slot race-lost — defensive against future async refactors |
| `spawner.npc_respawn` | INFO | `cell::service::ticks::npc_respawn::npc_respawn_tick` | Per-NPC respawn promotion — correlator: `world_name`, `respawn_secs` |
| `movement.validation` | WARN (`validation_reject`, `validation_recovered`, `speed_warning`, `navmesh_gm_bypass`, `space_mismatch`) / ERROR (`correction_suppressed`) | `cell::service::base_messages`, `cell::space_manager::movement_telemetry` | Movement reject — snap-back to last_valid. Every row **except `entity_missing`** carries `world` (name, not just `space_id`); `entity_missing` cannot, because the entity is in no space by definition. The three hard-reject outcomes (`validation_reject` \| `validation_recovered` \| `correction_suppressed`) are all counted by `movement_validation_rejects_total` and all carry the containment diagnosis when `reason = "navmesh"`: `gate` (`no_poly_in_extents` \| `horizontal` \| `below_surface` \| `above_jump_tolerance`), `nav_horiz_dist`, `nav_dy`, and `navmesh_hash`. **Throttled per entity and row kind** (first immediately, then ≤ 1/s per kind — a shared window would hide the transition into `correction_suppressed` behind the rejects that led to it) with `suppressed = N` naming the rows elided since the last emission — see [negative-logging-convention.md §pattern-d](negative-logging-convention.md#pattern-d--high-frequency-repeat-throttled-with-a-suppressed-count) and the full contract in [movement-telemetry.md](movement-telemetry.md) |
| `playtest.bookmark` | INFO | `cell::console::bookmark::emit` | One row per GM `.bug <note>` — the tester, their target, mission/step/objective state, regions, counters and the free-text note. Correlator: `bookmark_id`. The entry point for reconstructing a playtest without a chat log |
| `playtest.bookmark.entity` | INFO | `cell::console::bookmark::emit` | One row per entity within 60 u of the tester at `.bug` time (nearest 32; the selected target always included). Position, velocity, `yaw_rad`, **`yaw_byte` (the facing actually transmitted)**, `wire_facing_vs_caller_deg`, `ground_y` / `y_above_ground`, AI state, nav path, threat, follow target, spawn distance, `witness_count` (players whose AoI holds the entity) and `caller_witnesses_it` (read from the tester's AoI; before NA24 both were read off the entity itself and were always `0` / `false` for NPCs). Join on `bookmark_id` |
| `player.journal` | DEBUG | `cell::player_journal::note` | **The cross-system order for one player.** Every notable per-player event gets one strictly increasing `seq` and a closed `kind` vocabulary: `world_enter`, `reanchor`, `region_hint`, `cover_edge`, `step_advance`, `mission_complete`, `dialog`, `action_list` (the ordered actions a trigger resolved to, with delays), `deferred_scheduled`, `deferred_fired`, `death`, `respawn`, `kill`, `teleport`. `scope_name = 'player.journal' AND entity_id = N` ordered by `seq` replaces rebuilding order from timestamps across scopes. `.bug` attaches the last 24 entries as `recent_events` |
| `content.deferred` | INFO | `cell::content::executor::deferred` | A delayed content action firing: `chain_id`, `action_kind`, `delay_ms`, `late_ms`, `scheduled_seq` / `fired_seq`, and **`between`** — everything journaled for that player since it was scheduled. The dialog-replaced-after-0.6-s shape in one row |
| `npc_ai.tick` | DEBUG | `cell::service::npc_ai::dispatch::log_ai_tick` | One row per ticked NPC per AI tick, emitted after the handler with no silent paths: position, `yaw_rad` + **`yaw_byte`**, `last_movement_type`, `nav_path_len`, `next_wp`, `dest`, `dist_to_dest`, `target_id` + `target_pos` + `dist_to_target`, three-state `los` (`clear` \| `blocked` \| `unknown`; was the bool `has_los` before NA02), `los_policy` (the attack rule that acted on `los`: `strict` \| `stationary` \| `stationary_relaxed` \| `stationary_other_storey`; NA16, D-NA11), `vx` / `vy` / `vz` (the velocity every witness is sent), `follow_target_id`, `npc_to_spawn` (renamed from `dist_to_spawn` in NA00), `move_speed`, `navmesh_loaded`, `state_before` / `ai_state`, and the handler's `decision_outcome` (empty = the handler declared none). `fight.rs` outcomes now go through the shared helper, so they reach this row, the span and `npc_ai_decisions_total`. NA24: an NPC that was Idle, stayed Idle and is in no player's AoI writes one row per 60 s, with `suppressed` counting the rows skipped (`0` on every other row); every non-Idle or witnessed tick is still written |
| `npc_ai.transition` | DEBUG | `cell::service::npc_ai::transition::set_ai_state_on` | `event = "state_change"`: one row per **actual** AI-state change (a write that leaves the state unchanged logs nothing). `from`, `to` (snake_case `AiState` labels), `reason` (`auto_aggro` \| `assist` (NA14) \| `threat_preempt` \| `threat_empty` \| `leash_out` \| `target_lost` \| `leash_arrived` \| `leash_snap_fallback` \| `died` \| `respawn` \| `content` \| `gm_command` \| `patrol_start` \| `wander_start` \| `patrol_no_path` \| `wander_no_radius` \| `wander_no_spawn` \| `investigate_no_poi` \| `investigate_done` \| `follow_no_target` \| `follow_target_gone`), plus `npc_id`, `tag`, `template_id`, `world`, `space_id`, `npc_to_spawn`, `threat_count`, `nav_path_len`. `CellEntity::ai_state` is a private field and this helper is its only writer, so the row is a complete per-NPC state timeline. Counter `npc_ai_transitions_total` |
| `npc_ai.aggro` | INFO | `cell::service::npc_ai::aggro_acquired::log_aggro_acquired`, called from `combat::generate_threat` | `event = "acquired"`: one row per entry into Fighting. `cause` (`proximity` \| `damage` \| `content_threat` \| `assist`, NA14: a same-faction neighbour engaged and this NPC joined; an assist never recruits further), `from`, `target_id`, `player_id` and `account_id` (only when the target is a player), `npc_to_target`, `dy`, three-state `has_los` (`clear` \| `blocked` \| `unknown`), `aggression` (effective `EMobAggressionLevel` toward players, 1 = hostile, NA13) and `aggression_override` (unset when faction-derived), plus the common NPC fields. Replaces the unstructured "NPC aggro: preempt -> Fighting" line. Counter `npc_ai_aggro_total`. Also `event = "gm_toggle"` (INFO, no counter) when a GM sets `.aggro on|off`: `player_id`, `aggro_off`, `changed` (NA13) |
| `content` | WARN / INFO | `cell::content::executor::world::set_aggression` | `event = "set_aggression_tag_miss"` (WARN, `reason = "tag_not_found"`): the action's tag matched no entity, so the NPC's aggression is unchanged; carries `tag`, `chain_id`, `agg_level`. `event = "set_aggression_invalid_level"` (WARN, `reason = "invalid_level"`): a level outside 0-5, nothing changed (NA13). `event = "set_aggression"` (INFO): the hit, with `from` / `to` as level labels (`faction` when there was no override). A mistyped chain tag used to be silent and read exactly like an aggro bug |
| `spawner.npc_behaviour` | DEBUG | `cell::spawner::npcs::log_spawn_behaviour` | Resolved behaviour of each spawned NPC: `aggression`, `use_cover`, `is_stationary`, `move_speed`, `respawn_secs`, follow band, patrol / wander, `on_navmesh`, `ground_y`, `spawn_yaw_rad`, interaction flags, loot table |
| `player.death` | INFO | `cell::abilities::damage_apply` | First-class player death: identity, `killer` + `killer_name`, `ability_id`, world, position. The adjacent `onBeginAidWait` row now lists `respawner_ids` and its `filter` |
| `session.start` / `session.end` | INFO | `cell::service::base_messages::player_init`, `base::helpers::destroy_client_entities` | World entry (identity, character, archetype, level, `access_level`, world, mission count) and teardown (`disconnect_reason`, `session_secs`). Client telemetry is ingested by admin-api (`launcher.ingest` / `launcher.bundle`), so liveness is a query: a `session.start` with no `launcher.*` rows for the same window means the tester has no client logs — see the *sessions vs client telemetry* saved view |
| `player.respawn` | INFO | `cell::cell_methods::player::combat` | `callForAid` result: `state_flags_before` / `state_flags_after`, `was_dead`, `dead_flag_cleared`, health before/after/max, position from/to |
| `dialog.display` | DEBUG | `cell::content::executor::dialog::display` | Each dialog shown with `replaced_dialog_id` and `ms_since_previous`. `fire_dialog_choice` rows now carry `button_id` |
| `cover.flank_check` | DEBUG | `cell::cover::ai_integration` | Every flank test an NPC in a cover slot runs: slot, node position + orientation, threat position, `flanked`. Silent until an NPC actually holds a slot — check `use_cover` on `spawner.npc_behaviour` first |
| `console.feedback` | DEBUG | `cell::cell_methods::gm::feedback::send_gm_feedback` | The text every `.`-command sent back to the GM — results and rejection reasons alike (first 400 chars) |
| `playtest.friction` | WARN | `cell::playtest_friction` | Stuck-player detectors — one event per episode, discriminated by `signal`. Episode counters: `repeat_interact_no_effect` (5 dead-end interacts on one target / 60 s), `repeat_item_use_no_chain` (2 / 120 s), `console_reject_streak` (3 / 120 s), `escort_separated` (escort > 3x `follow_max_distance` for 5 AI ticks), `escort_leader_teleported` (a followed player is about to be teleported — the escort stays behind). Time-based, re-evaluated every 2 s on movement packets (so only while the player is sending movement): `step_stalled` (step unchanged 5 min), `region_dwell_no_hint` (server-side point-in-polygon containment for 6 s with no client hint — the post-respawn Throne Room shape), `death_then_silence` (hinting client sends none for 120 s + 100 u after `callForAid`). Event-driven, fire at the gameplay event whether or not the player is moving: `dialog_displaced` (a dialog replaced < 3 s after display) and `objective_never_completed` (objective still open when a chain force-completes the mission). Raised from behaviour, not from knowing the cause |
| `movement.movement_type` | DEBUG (`sent`, `cleared`) / TRACE (`deduped`) | `cell::abilities::messaging::broadcast_movement_type` | Every `setMovementType` outcome. The client picks mob animation from this byte, not from velocity, and `cleared` puts **nothing** on the wire — an NPC that translates afterwards renders in its prior pose. Fields: `kind`, `kind_byte`, `prior_kind`, `outcome`, `witness_count` |
| `wire.out.avatar_update` | DEBUG (1-in-101 over all sends; `sampled_1_in`, `suppressed`) | `firehose::log_entity_moved`, from `base::world_entry::cell_dispatch::aoi::entity_moved` | The SigNoz sample of the `wire.firehose.aoi_position` firehose (NA25). What a witness was actually told about an entity: `witness_id`, `entity_id`, position, velocity, `yaw_rad`, **`yaw_byte`**, `pitch_byte`, `pos_variant`, and (NA02) `npc_moved_since_last` — `false` beside a non-zero velocity is an NPC the client animates as running while it stands still. There is no movement-type field: the client animates NPC movement from velocity alone. UPDATE_AVATAR is unreliable and never reaches `wire.out`, so this is the only record of transmitted position/facing |
| `content.resolve` | DEBUG | `cimmeria_content_engine::chain::ChainEngine::resolve_event` | A chain whose **trigger matched but a condition failed** — names the first failing condition (`failed_condition`, `failed_condition_index`, `conditions_total`), the `chain_id` / `chain_name`, `trigger_type` and `source_entity`; `reason = "condition_failed"`. Distinguishes "nothing listens for this event" from "a chain listens but its step is not active yet" — the ordering-bug shape. Generic across every content trigger |
| `cover.detection` | DEBUG | `cell::service::ticks::cover::log_cover_edge` | One row per player cover-set proximity edge (`edge = entered \| left`): position, `crouched`, `nodes_in_set_nearby`, `nearest_node_id` / `nearest_node_dist` / node position, `proximity_radius`. Cover detection is pure proximity and never consults crouch |
| `mission.step_context` | DEBUG | `cell::missions::progression::advance_step` | State that is **already true** when a mission step activates: `regions_inside`, `cover_sets`, `crouched`, `in_combat`, position. Region and cover triggers are edge events, so anything listed here will not re-fire for the new step |
| `movement.navmesh` | INFO (`event = "navmesh_loaded"`) / WARN (`reason = "navmesh_missing"`) | `cell::space_manager::movement_telemetry::log_navmesh_loaded`, `cell::space_manager::lifecycle` | **Which mesh a space is running.** The INFO line fires once per space creation with `path`, `polys`, `verts`, `file_bytes`, `agent_height` / `agent_climb` / `agent_radius`, the full `navmesh_hash` (FNV-1a 64 of the file, 16 hex digits) and `navmesh_short_hash` (first 8). Every per-event navmesh log carries the short form, so this row is the join target for "which mesh build was this session running on?". The WARN is the no-`.nav` case — every navmesh consumer fails open, so NPCs there path in straight lines through geometry |
| `movement.position_sample` | DEBUG | `cell::space_manager::movement_telemetry::sample_accepted_position_at` | **Accepted** player positions, the positive-space counterpart to `movement.validation_reject`. ≤ 1 row per player per 5 s and only after ≥ 1 u of movement; players only (NPCs are covered by `movement.npc` / `npc_ai.tick`). Carries `world`, `space_id`, position, `on_navmesh`, `nav_dy` (height above the walkable surface), `navmesh_hash` and the identity pair. Grouping accepted positions by world builds the walked-surface map that makes a mesh hole visible *before* somebody falls into it |
| `npc_ai.path_fail` | WARN | `cell::service::npc_ai::path_failure::report_path_failure` | One shape for "the pathfinder gave this NPC nothing usable", shared by `fight`, `follow`, `patrol`, `investigate` and `wander`. Carries `state`, `decision_outcome`, `reason` (`no_mesh` \| `no_path` \| `no_start_poly` \| `no_end_poly` \| `no_corridor` \| `partial` \| `degenerate_path`; the four stage reasons arrived with NA02's typed `PathOutcome`), **`fallback`** (`direct_waypoint` \| `path_unchanged` \| `partial_route`), `world`, from/to positions, `dist`, `dy` (the air-climb signature) and `navmesh_hash`. `reason` and `fallback` are independent, and the message follows `fallback`: `fight` enqueues nothing on either of its failure branches (the NPC stands still or keeps a stale route), while the other four push the raw destination and walk through geometry. **Throttled per NPC** (first immediately, then ≤ 1 / 5 s) with `suppressed = N`. Before this target, `patrol` / `investigate` / `wander` logged *nothing* when `find_path` returned `None` — they pushed the raw destination and walked through geometry silently |
| `npc_ai.path` | DEBUG; `ok` ≤ 1 / 10 s per NPC | `cell::service::npc_ai::path_request::request_path` | `event = "request"`: every AI `find_path` whose `status` is not `ok`, and a per-NPC sample of the `ok` ones (a chaser repaths every tick its target moves; the healthy case is not news), with `suppressed` (NA02). `state`, typed `status` (`ok` \| `partial` \| `no_start_poly` \| `no_end_poly` \| `no_corridor` \| `straighten_failed` \| `no_mesh`), `from` / `to`, `target_id`, `target_is_gm` (audit S14: a GM standing off-mesh fails every chase), `start_snap_dy`, `end_snap_dist`, `n_waypoints`, `max_leg_dy`, `end_to_target_dist`, `end_to_dest_dist`. A `partial` corridor is still walked, and also raises `npc_ai.path_fail reason=partial`. Counter `npc_path_requests_total`, unthrottled |
| `npc_ai.los` | DEBUG, ≤ 1 / 5 s per (looker, target) | `cell::space_manager::spatial::line_of_sight` → `npc_ai::detectors::los` | `event = "blocked"`: every line of sight that is **not** clear (NA02, audit T9). `result` (`blocked` \| `unknown_off_mesh`), raw `from_xyz` / `to_xyz`, `ray_from` / `ray_to` (the projected points the ray ran between), `hit_xyz`, `eye_height_used` (always `0.0`: the navmesh ray runs along the floor, which is why it cannot see ceilings or other storeys), `dy`, `dist`, `navmesh_hash`. Replaces the unsampled `movement.navmesh reason=los_unknown_off_mesh` row |
| `npc_ai.leash` | INFO / WARN / DEBUG | `cell::service::npc_ai::detectors::leash`, called from `npc_ai::leash` and `combat::generate_threat` | NA02 rows, driven by the NA12 walk home. `event = "enter"` (INFO): the NPC gave up its fight, written once the route home is installed, with `reason` (`leash_out` \| `target_lost` \| `threat_empty`), `trigger` (`beyond_band` \| `chase_outward` \| `vertical_cap` \| `target_dead` \| `target_gone` \| `target_out_of_aoi` \| `threat_empty`), `npc_to_spawn` (horizontal, the distance the leash measures), `target_to_spawn` and target position (absent when the last target was lost), `leash_distance`, NPC position, `nav_path_len` (0 = no route; the snap fallback follows). `event = "arrived"` (INFO): walked home, or a follower reset in place; `event = "snap_fallback"` (INFO): no route or the 20 s walk timeout. Both carry `arrival` (`walked` \| `in_place` \| `snap_no_path` \| `snap_timeout`), `walk_secs`, `path_ok`, `snap_dist`, `stale_path_len` (must be 0), `spawn_on_mesh`. `event = "loop"` (WARN, ≤ 1 / 60 s per NPC): 3 or more leash entries inside 60 s (S5), `leash_count`, `target_id`; counter `npc_leash_loop_total`. Should read zero after NA12. `event = "damage_ignored"` (DEBUG): threat refused because a Leashing NPC evades (S12). `npc_ai::leash` itself adds `event = "replan"` (DEBUG, a Leashing NPC without a route home got one) and `event = "player_combat_exit"` (DEBUG, a player's last threatening mob drained, `BSF_InCombat` cleared). The route home is also an `npc_ai.path event=request` with `state = leash` |
| `npc_ai.aggro_scan` | DEBUG, sampled | `cell::service::npc_ai::detectors::aggro_scan` | NA02. `event = "candidate_rejected"` (≤ 1 / 10 s per NPC–player pair): `reason` (`not_player` \| `dead` \| `same_faction` \| `not_hostile` \| `gm_ignored` \| `out_of_vertical_band` \| `out_of_radius` \| `no_los` \| `post_reset_suppressed`, NA13), `player_id`, `npc_to_target`, `dy`, `aggro_radius` (the NPC's radius in u; NA02-era rows say `"unbounded"`). `event = "no_candidates"` (≤ 1 / 30 s per NPC): `witness_count`, `rejected`. NA14: `event = "assist_rejected"` (≤ 1 / 10 s per assister–victim pair and reason): `npc_id` is the neighbour passed over, `victim_id`, `player_id`, `reason` (`dead` \| `not_idle` \| `not_hostile` \| `post_reset_suppressed` \| `gm_ignored` \| `out_of_vertical_band` \| `out_of_radius` \| `no_los`), `ai_state`, `npc_to_victim`, `dy`, `assist_radius`; `event = "assist_joined"` (unsampled, once per join): `npc_id`, `victim_id`, `player_id`, `npc_to_victim` |
| `npc_ai.idle` | DEBUG, ≤ 1 / 30 s per world | `cell::service::npc_ai::detectors::sweep` | `event = "unticked"`: how many Idle NPCs the AI tick skips (no aggression, patrol or wander), per world, with `suppressed`. The sample window is per world and released by `destroy_space`. Gauge `npc_ai_idle_unticked{world}` |
| `npc_ai.idle_parked` | INFO | `cell::service::npc_ai::detectors::idle_parked`, from `set_ai_state_on` | NA02. An NPC changed to Idle more than 2 u from spawn and will not be ticked again (S6 + A1): `from`, `reason` (the transition reason), `npc_to_spawn`, position. Counter `npc_idle_parked_total{world,reason}`. After NA12 only a follower reset in place (`reason = leash_arrived`) or a content / GM Idle away from spawn can raise it |
| `npc_ai` | WARN | `cell::service::npc_ai::detectors::sweep` | NA02, per ticked NPC after its handler. `event = "npc_off_mesh"` (≤ 1 / 30 s; an NPC parked where it spawned since it spawned, `last_move_source = spawn`, warns once and then repeats at DEBUG on the same window, NA24): `gate`, `horizontal_dist`, `dy`, `last_move_source` (`path` \| `fallback` \| `leash` \| `backup` \| `content` \| `spawn`); counter `npc_off_mesh_total{world,gate}`. `event = "stuck"` (≤ 1 / 15 s): Fighting and chasing (with a path, or `no_path` with none: an NPC that never gets a route is the most stuck), and `npc_to_target` has not shrunk by 0.5 over 3 AI ticks — `npc_to_target_history`, `nav_path_len`, `los`, `next_wp`; counter `npc_stuck_total` |
| `npc_ai` | DEBUG | `cell::service::npc_ai::fight_cover::route_via_cover` | NA02, `decision_outcome = "no_cover"` (a log field, not the tick's terminal outcome): replaces the silent `NoCover => {}` arm (audit C7). Sampled ≤ 1 / 10 s per NPC with `suppressed`. `reason` (`no_candidate_in_radius` \| `reserve_lost` \| `in_range_no_better_slot` \| `no_world` \| `index_miss`), `candidates_scanned`, `reserved_skipped`, `search_radius`, `cover_nodes_loaded`. An NPC with `use_cover = false`, or a stationary one, is not logged at all: it never asks, and `spawner.npc_behaviour` already records both |
| `movement.npc` | WARN | `cell::service::npc_ai::detectors::movement` | NA02. `event = "stale_velocity"` (≤ 1 / 10 s per NPC): a non-zero velocity with no displacement for 3 movement ticks — the running-in-place detector (S1). `path_state` = `empty` (the path was cleared mid-leg; this is also the telemetry plan's `animating_without_path`, re-based on velocity because the client animates from velocity alone) or `stalled`; counter `npc_stale_velocity_total`, one per episode (the tick the NPC becomes stale). `event = "ground_deviation"` (≤ 1 / 5 s per NPC): a step or waypoint snap more than 0.3 from the storey-aware floor (NA01's `get_height_near`) on a meshed world — `dir` (`up` \| `down` \| `unknown` when no floor is within jump height), `ground_y`, `dy`, `y_source` (`lerp` \| `waypoint`), `leg_len`, `leg_dy`, `wp_*`; counter `npc_ground_deviation_total{world,dir}`, one per episode (the first off-floor step after a grounded one) |
| `threat` | WARN, ≤ 1 / 30 s per player–NPC pair | `cell::service::npc_ai::detectors::threat` | NA02, `event = "cleared_without_exit"`: an NPC cleared its threat list (`reason` = `threat_empty` \| `leash_out` \| `target_lost` \| `leash_complete`) while a player still lists it in `threatened_mobs` (S7), so the player stays in combat. NA12 drains every player before each leash clear, so a row means a new clear path that skipped the drain. Counter `npc_threat_cleared_without_exit_total{world,reason}` |
| `spawner.npc_behaviour` | WARN, once per spawn id | `cell::service::npc_ai::detectors::spawn` | NA02, `event = "spawn_off_mesh"`: the spawn fails `find_path`'s ±0.5 start box (S9) even when `on_navmesh` (the looser `is_point_valid`) is true. `gate`, `horizontal_dist`, `dy`, `snapped_y`. Counter `npc_spawn_off_mesh_total{world,gate}` |
| `cover.coverage` | INFO, WARN when unusable | `cell::cover::coverage::log_space_coverage` | NA02, `event = "space_summary"`, one row per space once it has its NPCs and the cover index has loaded (startup spaces from `SpaceManager::cover_loaded`, instanced spaces after `spawn_instance_npcs_from_records`). Reads the world-scoped index (NA21): `world_id`, `nodes_in_world`, `nodes_on_mesh` (a node counts when `NavMesh::get_height_near` around its own Y finds a floor within 1.0), `sets_in_world`, `cover_npcs` (NPCs with `use_cover` that are not stationary). WARN `reason = "no_usable_cover"` when a meshed space has cover-seeking NPCs and no usable node. With the NA21 seed, Castle_CellBlock (world 12) reads 236 nodes, 211 on the mesh, 58 sets |
| `cover.selection` | DEBUG, ≤ 1 / 10 s per NPC | `cell::service::npc_ai::fight_cover` | NA02. `event = "picked"`: the best free node's `chunk_id`, `node_id`, `score`, `move_dist`, `threat_dist`, `scanned`. `event = "rejected"`: the top 3 losers with `rank` and `reason` (`reserved` \| `lower_score`) |
| `wire.out.forced_position` | DEBUG | `base::world_entry::teleport::handle_teleport_player` | NA02. Every `FORCED_POSITION` sent: position, previous position, `snap_dist`, `reason`. All are player snaps today — no NPC snap (the leash included) is sent as a forced position; witnesses learn of it from the next AoI `EntityMoved` |
| `movement.navmesh` | INFO | `cell::space_manager::navmesh_mode::log_navmesh_summary` | `reason = "navmesh_mode_summary"`: one line at startup per resident space that **has** a mesh — `space_id`, `world_name`, `navmesh_mode` (`enforce` \| `advisory`), `poly_count`, `spawn_rows`, `spawn_rows_off_mesh`. A high `spawn_rows_off_mesh` on an `enforce` world is an invisible-wall report waiting to happen: the mesh loaded, but it does not describe the map players walk, and the holes are hard gates for everyone except GMs. INFO rather than WARN because an advisory world is expected to have a high count — the actionable signal is the number moving. See [navmesh-containment-modes.md](navmesh-containment-modes.md) |
| `movement.navmesh` | TRACE (level-gated), ≤ 1 / 500 ms per player | `cell::space_manager::client_move` | `reason = "advisory_off_mesh_accepted"`: an off-mesh position an `advisory` world accepted — `entity_id`, `space_id`, `world`, `client_x` / `client_y` / `client_z`, `suppressed`. Guarded by `tracing::enabled!` **before** the Detour query. No layer enabled it before NA25, so it never fired; the `cimmeria-trace` index now does whenever OTLP is on, so it is throttled per player (`ADVISORY_OFF_MESH_LOG_INTERVAL`, a breadcrumb every ~3 units at run speed). This is the real player traffic a mesh rebake needs (which parts of the world people actually walk through), as opposed to a static grid probe |
| `movement.navmesh` | WARN | `cell::space_manager::navmesh_mode::mode_from_db_value` | `reason = "navmesh_mode_unrecognised"`: `resources.worlds.navmesh_mode` held a value this build does not know (`world_name`, `raw_value`). Falls back to `enforce` — containment stays on |
| `navmesh.load` | ERROR | `entity::navigation::check_count` | Hostile `.nav` header rejected — space loads navmesh-less |

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
| `no_ability` | Every known ability on cooldown / needs ammo |
| `leashed` | The NPC itself went past its leash radius from spawn (NA12; before NA12 the test was the target's distance), or its target was unreachable (NA15: `trigger=unreachable`, `cause` = `held_unreachable` \| `off_mesh`). The row carries `trigger` (`beyond_band` \| `chase_outward` \| `vertical_cap` \| `unreachable`), `npc_to_spawn` (horizontal, the distance the test uses), `npc_dy_from_spawn`, `target_to_spawn` (the pre-NA12 metric, for comparison) and `leash_distance` |
| `threat_empty` / `target_lost` | Fight over: the threat list was empty, or its last target died, vanished or stayed out of the NPC's AoI for 5 s. The NPC starts walking home (`npc_ai.leash event=enter`) |
| `leash_walking` / `leash_replan` | Leashing tick: walking the route home / planned a fresh route (none, or a stale one) |
| `leash_arrived` / `leash_snap_fallback` | Leash finished: walked home, or snapped because no route existed or the walk passed 20 s |
| `reaggro_suppressed` | Idle auto-aggro skipped: inside the 5 s window after a leash reset |
| `repath_degenerate` | WARN — chase repath returned ≤1 waypoint; since NA15 the stale path is cleared (`fallback=path_cleared`) and the NPC holds |
| `hold_no_repath` | Out of range / no LoS, but the route is still good for where the target is (NA15: the goal moved at most 5 u horizontally and 1.5 u vertically since the route was planned) — no new order this tick (previously silent) |
| `hold_unreachable` | NA15 — at the end of a route that cannot reach the target (partial corridor, off-mesh target, degenerate repath): standing still with zero velocity, no new route until the target moves; gives up after 8 s (`leashed trigger=unreachable`) |
| `follow_no_path` | WARN — follow found no navmesh path and fell back to a raw 3-axis straight line (`reason` = `no_mesh` \| `no_path`; `dy` is the air-climb signature) |
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
| `patrol_no_path` | WARN — patrol leg found no navmesh path; the raw waypoint is pushed and the NPC walks toward it through geometry. Log field on `npc_ai.path_fail`; the handler's terminal outcome stays `patrol_continue` because the NPC does still move |
| `investigate_no_path` | WARN — same shape for an investigate leg. Terminal outcome stays `investigate_routed` |
| `wander_no_path` | WARN — same shape for a wander hop. Terminal outcome stays `wander_pick` |
| `chase_partial` / `follow_partial` / `patrol_partial` / `investigate_partial` / `wander_partial` | WARN, NA02 — the route is a partial corridor to the edge of the NPC's mesh island; it is still walked. Log field on `npc_ai.path_fail reason=partial`; the terminal outcome is unchanged (`chase`, `follow_band`, ...) |
| `no_cover` | DEBUG, NA02 — the cover step found no slot, with a `reason`. Log field only: the chase or attack branch that follows sets the terminal outcome |
| `follow_band` | Follow target is inside the band — no work |
| `despawn` | Despawn tick — entity is being removed from the space |
| `submit_init` | Submit tick — the pass that actually disengages both sides: player-side threat scrub, auto-cycle sweep, channel cancel, cover release, re-face. Re-fires if somebody re-engages a surrendered NPC |
| `submit_hold` | Submit tick — nothing left to clean, the NPC is parked. The steady state for a surrendered NPC, one row per ~2 s AI tick for the space's life. A `submit_init` where you expect `submit_hold` means something keeps re-aggroing it |
| `error_hold` | Error state — diagnostic quiescent fallback |

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
| `npc_ai_transitions_total` | `world` (~24), `from` (12), `to` (12), `reason` (18) | One per actual AI-state change, beside the `npc_ai.transition` row. Only a few dozen `from`/`to`/`reason` triples occur in practice |
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
move them behind `cimmeria_services::firehose` the same way.

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
[`crates/services/src/base/dispatch/diagnostics.rs`](../../crates/services/src/base/dispatch/diagnostics.rs)
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
