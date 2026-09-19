# Server observability — design and tool choice

**Status:** Accepted (2026-05-25)
**Last updated:** 2026-07-25
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

### Stable target catalog

Every event with a stable `target:` is a queryable surface in SigNoz —
filter by `scope_name = '<target>'` to count occurrences, pivot on
field values, plot rate-over-time. Adding a new target is cheap; the
discipline is that targets should be **stable strings** (not subject
to crate-rename churn) and **named for the question they answer**.

| `target` | Level | Emitted from | What it counts |
|---|---|---|---|
| `mercury.packet` | INFO | `Channel::{send,receive}_packet`, `UnifiedCodec::{encode,decode}` | Every byte in/out of the server |
| `mercury.retransmit` | INFO | `Channel::check_timeouts` | Reliable-channel retransmits |
| `mercury.backpressure` | WARN | `Channel::send_packet` when TX window ≥ 50% full | Send-window saturation — early warning for stalled clients |
| `wire.in` / `wire.out` | INFO | `wire_log::{log_inbound, log_outbound_entity_method}` | Decoded entity-method calls. `wire.in` resolves cell methods by **method index** (`msg_id - 0x80`, or `61 + sub_index` for the `0xBD` sub-slot form) and carries `method_index` + `entity_method`; base methods (`0xC2+`) are `baseMethod` with their index until a base name table exists |
| `aoi.entity_enter` / `aoi.entity_leave` | DEBUG | AoI tick witness fanout | Per-entity AoI transitions |
| `aoi.create_emit` | DEBUG | `base::world_entry::cell_dispatch::aoi::{entered_aoi, flush_deferred_aoi}` | Per-packet entity-introduction delivery (CREATE_ENTITY+UPDATE_AVATAR / createOnClient cascade) — fields `witness_id`, `entity_id`, `class_id`, `phase` (`create_base` \| `cascade`), `addr_resolved`, `bytes`, `seq`. Success-side visibility for the invisible-static-NPC drop |
| `aoi.create_send_failed` | WARN | `base::world_entry::cell_dispatch::aoi::{entered_aoi, flush_deferred_aoi}` | Entity-introduction packet/bundle that could NOT be delivered — `reason` (`entity_to_addr_miss` \| `client_disconnected` \| `send_error`), `phase`, `addr_resolved`. Negative-logging seam for the invisible-corpse class |
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
| `movement.validation` | WARN | `cell::service::base_messages` | Movement reject (bounds violation) — snap-back to last_valid |
| `playtest.bookmark` | INFO | `cell::console::bookmark::emit` | One row per GM `.bug <note>` — the tester, their target, mission/step/objective state, regions, counters and the free-text note. Correlator: `bookmark_id`. The entry point for reconstructing a playtest without a chat log |
| `playtest.bookmark.entity` | INFO | `cell::console::bookmark::emit` | One row per entity within 60 u of the tester at `.bug` time (nearest 32; the selected target always included). Position, velocity, `yaw_rad`, **`yaw_byte` (the facing actually transmitted)**, `wire_facing_vs_caller_deg`, `ground_y` / `y_above_ground`, AI state, nav path, threat, follow target, spawn distance. Join on `bookmark_id` |
| `player.journal` | DEBUG | `cell::player_journal::note` | **The cross-system order for one player.** Every notable per-player event gets one strictly increasing `seq` and a closed `kind` vocabulary: `world_enter`, `reanchor`, `region_hint`, `cover_edge`, `step_advance`, `mission_complete`, `dialog`, `action_list` (the ordered actions a trigger resolved to, with delays), `deferred_scheduled`, `deferred_fired`, `death`, `respawn`, `kill`, `teleport`. `scope_name = 'player.journal' AND entity_id = N` ordered by `seq` replaces rebuilding order from timestamps across scopes. `.bug` attaches the last 24 entries as `recent_events` |
| `content.deferred` | INFO | `cell::content::executor::deferred` | A delayed content action firing: `chain_id`, `action_kind`, `delay_ms`, `late_ms`, `scheduled_seq` / `fired_seq`, and **`between`** — everything journaled for that player since it was scheduled. The dialog-replaced-after-0.6-s shape in one row |
| `npc_ai.tick` | DEBUG | `cell::service::npc_ai::dispatch::log_ai_tick` | One row per ticked NPC per AI tick, emitted after the handler with no silent paths: position, `yaw_rad` + **`yaw_byte`**, `last_movement_type`, `nav_path_len`, `next_wp`, `dest`, `dist_to_dest`, `target_id` + `target_pos` + `dist_to_target`, `has_los`, `follow_target_id`, `dist_to_spawn`, `move_speed`, `navmesh_loaded`, `state_before` / `ai_state`, and the handler's `decision_outcome` (empty = the handler declared none). `fight.rs` outcomes now go through the shared helper, so they reach this row, the span and `npc_ai_decisions_total` |
| `spawner.npc_behaviour` | DEBUG | `cell::spawner::npcs::log_spawn_behaviour` | Resolved behaviour of each spawned NPC: `aggression`, `use_cover`, `is_stationary`, `move_speed`, `respawn_secs`, follow band, patrol / wander, `on_navmesh`, `ground_y`, `spawn_yaw_rad`, interaction flags, loot table |
| `player.death` | INFO | `cell::abilities::damage_apply` | First-class player death: identity, `killer` + `killer_name`, `ability_id`, world, position. The adjacent `onBeginAidWait` row now lists `respawner_ids` and its `filter` |
| `session.start` / `session.end` | INFO | `cell::service::base_messages::player_init`, `base::helpers::destroy_client_entities` | World entry (identity, character, archetype, level, `access_level`, world, mission count) and teardown (`disconnect_reason`, `session_secs`). Client telemetry is ingested by admin-api (`launcher.ingest` / `launcher.bundle`), so liveness is a query: a `session.start` with no `launcher.*` rows for the same window means the tester has no client logs — see the *sessions vs client telemetry* saved view |
| `player.respawn` | INFO | `cell::cell_methods::player::combat` | `callForAid` result: `state_flags_before` / `state_flags_after`, `was_dead`, `dead_flag_cleared`, health before/after/max, position from/to |
| `dialog.display` | DEBUG | `cell::content::executor::dialog::display` | Each dialog shown with `replaced_dialog_id` and `ms_since_previous`. `fire_dialog_choice` rows now carry `button_id` |
| `cover.flank_check` | DEBUG | `cell::cover::ai_integration` | Every flank test an NPC in a cover slot runs: slot, node position + orientation, threat position, `flanked`. Silent until an NPC actually holds a slot — check `use_cover` on `spawner.npc_behaviour` first |
| `console.feedback` | DEBUG | `cell::cell_methods::gm::feedback::send_gm_feedback` | The text every `.`-command sent back to the GM — results and rejection reasons alike (first 400 chars) |
| `playtest.friction` | WARN | `cell::playtest_friction` | Stuck-player detectors — one event per episode, discriminated by `signal`. Episode counters: `repeat_interact_no_effect` (5 dead-end interacts on one target / 60 s), `repeat_item_use_no_chain` (2 / 120 s), `console_reject_streak` (3 / 120 s), `escort_separated` (escort > 3x `follow_max_distance` for 5 AI ticks), `escort_leader_teleported` (a followed player is about to be teleported — the escort stays behind). Time-based, re-evaluated every 2 s on movement packets (so only while the player is sending movement): `step_stalled` (step unchanged 5 min), `region_dwell_no_hint` (server-side point-in-polygon containment for 6 s with no client hint — the post-respawn Throne Room shape), `death_then_silence` (hinting client sends none for 120 s + 100 u after `callForAid`). Event-driven, fire at the gameplay event whether or not the player is moving: `dialog_displaced` (a dialog replaced < 3 s after display) and `objective_never_completed` (objective still open when a chain force-completes the mission). Raised from behaviour, not from knowing the cause |
| `movement.movement_type` | DEBUG (`sent`, `cleared`) / TRACE (`deduped`) | `cell::abilities::messaging::broadcast_movement_type` | Every `setMovementType` outcome. The client picks mob animation from this byte, not from velocity, and `cleared` puts **nothing** on the wire — an NPC that translates afterwards renders in its prior pose. Fields: `kind`, `kind_byte`, `prior_kind`, `outcome`, `witness_count` |
| `wire.out.avatar_update` | DEBUG (1-in-100 over all sends) | `base::world_entry::cell_dispatch::aoi::entity_moved` | What a witness was actually told about an entity: `witness_id`, `entity_id`, position, velocity, `yaw_rad`, **`yaw_byte`**, `pitch_byte`, `pos_variant`. UPDATE_AVATAR is unreliable and never reaches `wire.out`, so this is the only record of transmitted position/facing |
| `content.resolve` | DEBUG | `cimmeria_content_engine::chain::ChainEngine::resolve_event` | A chain whose **trigger matched but a condition failed** — names the first failing condition (`failed_condition`, `failed_condition_index`, `conditions_total`), the `chain_id` / `chain_name`, `trigger_type` and `source_entity`; `reason = "condition_failed"`. Distinguishes "nothing listens for this event" from "a chain listens but its step is not active yet" — the ordering-bug shape. Generic across every content trigger |
| `cover.detection` | DEBUG | `cell::service::ticks::cover::log_cover_edge` | One row per player cover-set proximity edge (`edge = entered \| left`): position, `crouched`, `nodes_in_set_nearby`, `nearest_node_id` / `nearest_node_dist` / node position, `proximity_radius`. Cover detection is pure proximity and never consults crouch |
| `mission.step_context` | DEBUG | `cell::missions::progression::advance_step` | State that is **already true** when a mission step activates: `regions_inside`, `cover_sets`, `crouched`, `in_combat`, position. Region and cover triggers are edge events, so anything listed here will not re-fire for the new step |
| `movement.navmesh` | WARN | `cell::space_manager::lifecycle` | Space created with no `.nav` file (`reason = "navmesh_missing"`) — every navmesh consumer fails open, so NPCs there path in straight lines through geometry |
| `movement.navmesh` | DEBUG | `cell::space_manager::spatial::line_of_sight` | `reason = "los_unknown_off_mesh"`: a line-of-sight query had an endpoint outside navmesh coverage (`a_on_mesh` / `b_on_mesh`), so the verdict is `Unknown` and is treated as clear. Frequent rows for one NPC id mean its spawn sits in a navmesh hole (9 of the 13 stationary Harset mobs against `harset.nav`) |
| `movement.navmesh` | INFO | `cell::space_manager::navmesh_mode::log_navmesh_summary` | `reason = "navmesh_mode_summary"`: one line at startup per resident space that **has** a mesh — `space_id`, `world_name`, `navmesh_mode` (`enforce` \| `advisory`), `poly_count`, `spawn_rows`, `spawn_rows_off_mesh`. A high `spawn_rows_off_mesh` on an `enforce` world is an invisible-wall report waiting to happen: the mesh loaded, but it does not describe the map players walk, and the holes are hard gates for everyone except GMs. INFO rather than WARN because an advisory world is expected to have a high count — the actionable signal is the number moving. See [navmesh-containment-modes.md](navmesh-containment-modes.md) |
| `movement.navmesh` | TRACE (level-gated) | `cell::space_manager::client_move` | `reason = "advisory_off_mesh_accepted"`: an off-mesh position an `advisory` world accepted — `entity_id`, `space_id`, `client_x` / `client_y` / `client_z`. Guarded by `tracing::enabled!` **before** the Detour query, so it costs nothing until the level is on. This is the real player traffic a mesh rebake needs (which parts of the world people actually walk through), as opposed to a static grid probe |
| `movement.navmesh` | WARN | `cell::space_manager::navmesh_mode::mode_from_db_value` | `reason = "navmesh_mode_unrecognised"`: `resources.worlds.navmesh_mode` held a value this build does not know (`world_name`, `raw_value`). Falls back to `enforce` — containment stays on |
| `navmesh.load` | ERROR | `entity::navigation::check_count` | Hostile `.nav` header rejected — space loads navmesh-less |

#### Saved views for reading a playtest

Three Logs Explorer views live under the `playtest` category in SigNoz: **Playtest: bookmarks (.bug notes)** — start here, pick a `bookmark_id`; **Playtest: friction (stuck-player detectors)** — every `playtest.friction` and `movement.navmesh` warning; **Playtest: position trail** — `movement.player`, `movement.npc`, `wire.out.avatar_update`, `movement.movement_type` and bookmark rows interleaved with position / waypoint / `yaw_byte` columns, so narrowing the time range to ±30 s around a bookmark shows where everyone was, where they were going, and what the client was sent. Add `AND entity_id = <id>` or `AND npc_id = <id>` to follow one actor.

#### `npc_ai.decision_outcome` enum

The `npc_ai.decision` event carries a `decision_outcome` field with
one of these values, letting SigNoz answer "which zones / NPCs are
failing to engage and why" via a single `groupBy=decision_outcome`:

| `decision_outcome` | Meaning |
|---|---|
| `attack_in_place` | In range + LOS + ability ready — NPC fires |
| `chase` | Out of range / LOS — pathfinding toward target |
| `no_path` | Pathfinder returned no path (typically: zone missing navmesh) |
| `min_range_backup` | Target inside ability `min_range` — stepping back |
| `no_ability` | Every known ability on cooldown / needs ammo |
| `leashed` | Target moved past `LEASH_DISTANCE` from spawn |
| `repath_degenerate` | WARN — chase repath returned ≤1 waypoint; the previous path is left in place, so the NPC may keep walking toward where the target used to be |
| `hold_no_repath` | Out of range / no LoS, but the existing path still ends within 5 u of the target — no new order this tick (previously silent) |
| `follow_no_path` | WARN — follow found no navmesh path and fell back to a raw 3-axis straight line (`reason` = `no_mesh` \| `no_path`; `dy` is the air-climb signature) |
| `follow_target_lost` | WARN — follow target no longer resolves; follow is cleared and the escort idles until a chain re-arms it |
| `follow_dropped_no_target` | Follow state with no follow target — dropped to Idle |
| `stationary_holds` | Stationary NPC out of range / no LOS — holds fire |
| `stay_in_cover` | NPC in cover, threat in defensive arc — hold |
| `move_to_cover` | NPC picked a fresh cover slot — paths to it |
| `cover_released_flanked` | Threat flanked the cover — released, re-eval next tick |
| `patrol_continue` | Patrol tick walking toward the current waypoint |
| `patrol_dwell` | Patrol tick paused at a waypoint after arrival |
| `wander_pick` | Wander tick chose a fresh destination within radius |
| `wander_dwell` | Wander tick paused at the current destination |
| `investigate_arrived` | Investigate tick reached the POI — dwell starts |
| `investigate_routed` | Investigate tick pathfinding toward the POI |
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

**Resource attribute `deployment.environment`.** Every metric (and
every span and log) carries this resource attribute, defaulted from
`CIMMERIA_DEPLOY_ENV` (default `"dev"`). Operators set it in the
colo's docker-compose to `colo` so SigNoz dashboards can split
production data from dev-laptop noise. Override via the standard OTel
`OTEL_RESOURCE_ATTRIBUTES=deployment.environment=...` if needed.

### Cost on the hot path

`tracing::info!` with no subscriber attached: a single atomic load +
branch. With the OTLP layer attached: serialise to OTLP wire format +
push onto an in-process mpmc channel. The actual network send is on
a background task. Net cost per packet: handful of nanoseconds.

Sampling is `always_on` by default — Mercury packet rate is the
analytical surface we care about, sampling defeats the purpose. If
volume becomes an issue, the lever is `OTEL_TRACES_SAMPLER` (set per
deployment via the compose env var), not source code changes.

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
- Instrumentation helpers: [`crates/mercury/src/instrumentation.rs`](../../crates/mercury/src/instrumentation.rs)
- OTLP exporter: [`crates/server/src/otel.rs`](../../crates/server/src/otel.rs)
- Launcher ingest endpoint: [`crates/admin-api/src/routes/telemetry/`](../../crates/admin-api/src/routes/telemetry/)
- Launcher telemetry pipeline (dev-session flow + secret rotation): [dev-session-telemetry.md](dev-session-telemetry.md), [docs/operations/telemetry.md](../operations/telemetry.md)
