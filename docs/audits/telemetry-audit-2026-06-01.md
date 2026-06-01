---
title: "Telemetry & Logging Audit — Code Landed 2026-05-31 → 2026-06-01"
type: explanation
audience: engineers, architects
last_updated: 2026-06-01
companions:
  - ../architecture/observability.md
  - ../architecture/negative-logging-convention.md
  - ../operations/telemetry.md
scope: 13 feature commits (#421, #423, #424, #425, #427, #428, #429, #432, #435, #436, #437, #438; agent-memory chore #424's executable bits)
---

# Telemetry & Logging Audit — Code Landed 2026-05-31 → 2026-06-01

> Read-only scope report. No source changes proposed here are landed
> in this PR; the audit names the gaps so reviewers can split the
> follow-up into sized PRs.

## Executive summary

- **Total files in audit scope:** 79 unique source files across 13 feature commits.
  (Tests excluded except where the source change shipped no production logging at all.)
- **New public/dispatch handlers added:** 22.
  Trade: 4 (`trade.{request,cancel,update_proposal,lock_state}`) + 1 base handler (`trade.execute`).
  Cover: 1 detection tick + 1 NPC-AI integration entry + 4 content fire_* events.
  NPC AI: 7 new state handlers (`patrol`, `wander`, `investigate`, `follow`, `despawn`, `submit`, `error`).
  Crafting: 6 stub-routed cell methods (`spend_applied_science_points`, `craft`, `research`, `reverse_engineer`, `alloying`, `respec_crafting`) + 2 base persistence functions.
  Trainer: 1 (`try_open_trainer`) — consolidated routing.
  Movement: 1 (`apply_client_position_update`).
  Cell→base bridge: 1 (`ExecuteTrade` dispatch arm).
- **Handlers currently lacking `#[instrument]`:** 11 (named in per-feature sections).
  Note: the **gold standard** in [`base/world_entry/play_character.rs:26-31`](../../crates/services/src/base/world_entry/play_character.rs#L26) puts an explicit `#[tracing::instrument(name="world_entry.play_character", level="info", skip_all, fields(peer, account_id, player_id))]` on every dispatch entrypoint. Several of the new handlers below have a parent span (e.g. trade handlers have `#[instrument]` but their swap-helper callees inherit only) — calling these out as "lacking" means the *callee* lacks its own span where a sub-span would meaningfully aid query/groupBy on a hot subsystem.
- **Error paths currently logging at < `warn!`:** 6 confirmed silent-or-trace returns (per-feature breakdown below). Two of these (the cover-reservation race, the `swap.rs` per-abort emission) are P0.
- **Hot paths without metrics:** 11. There is **no metrics registry pattern in the codebase yet** — see [Convention recap](#convention-recap), open question Q1. Counts proposed below assume the open question lands as "yes, add a `metrics` facade", but every row is also actionable as a `tracing::info!(target: "<system>", count_n=N)` log if Q1 lands as "no".
- **Estimated PR count:** 5 follow-up PRs (P0 emergency × 1, P1 per-system × 3, P2 cleanups × 1). Sized in [Suggested PR split](#suggested-pr-split).

## Convention recap

Anchored entirely against existing repo conventions — every cell below has a `file:line` reference to where the rule is encoded today. The audit must not introduce a new convention.

### Span naming convention

**Pattern.** Dispatch entrypoints and DB write boundaries get
`#[tracing::instrument(name = "<system>.<verb>", level = "info", skip_all, fields(...))]`.
The `name=` is `dotted.lowercase` — `world_entry.play_character`,
`world_entry.teleport_player`, `base.login`, `trade.request`,
`trade.execute`, `cover.detection_tick`, `spawner.npc_respawn_tick`.

**Anchors.**
- [`crates/services/src/base/world_entry/play_character.rs:26-31`](../../crates/services/src/base/world_entry/play_character.rs#L26) — `world_entry.play_character`, info.
- [`crates/services/src/base/world_entry/teleport.rs:35-40`](../../crates/services/src/base/world_entry/teleport.rs#L35) — `world_entry.teleport_player`, info.
- [`crates/services/src/base/login/mod.rs:27-32`](../../crates/services/src/base/login/mod.rs#L27) — `base.login`, info.
- [`crates/services/src/cell/cell_methods/player/trade/handlers.rs:58`](../../crates/services/src/cell/cell_methods/player/trade/handlers.rs#L58) — `trade.request`, info.
- [`crates/services/src/cell/service/ticks/cover.rs:31-36`](../../crates/services/src/cell/service/ticks/cover.rs#L31) — `cover.detection_tick`, debug, with `tracing::Span::current().record("player_count", …)`.

**Field discipline.** Spans carry the **correlator** fields needed for downstream filtering — `entity_id`, `player_id`, `peer`, `space_id`, `account_id`, etc. — declared either as positional `fields(entity_id, player_id)` or with `tracing::field::Empty` placeholders that the body later fills via `Span::current().record("...", …)` (see cover tick `player_count`/`events` pattern).

### Log-level discipline

[`docs/architecture/negative-logging-convention.md:62-68`](../architecture/negative-logging-convention.md#L62) is canonical:

| Level | When |
|---|---|
| `trace!` | **Never** for expectation failures. High-volume sample-only diagnostics. |
| `debug!` | Expectation unmet, normal/transient (client disconnected mid-AoI). |
| `warn!` | Expectation unmet, player-visible, recoverable. |
| `error!` | Expectation unmet, player-visible, unrecoverable or state-corrupting (e.g. `rows_affected == 0` on a mission-complete UPSERT). |

### Negative-log field-naming rules

[`docs/architecture/negative-logging-convention.md:51-59`](../architecture/negative-logging-convention.md#L51):

| Field | Required? | Notes |
|---|---|---|
| `player_id` | when applicable | No aliases (`pid`). |
| `entity_id` | when applicable | No aliases (`eid`). |
| `mob_id`/`mission_id`/`chain_id`/`step_id`/`space_id`/`cell_id`/`world_name` | when applicable | Canonical names. |
| `rows_affected` + `expected` | always paired on DB writes | A single ops query catches divergence. |
| `phase` | optional | Sub-step name (e.g. `"create_base"`, `"cascade"`). |
| `reason` | optional | Short string naming why the expectation was unmet (`"entity_to_addr_miss"`, `"oneshot_dropped"`, `"rows_affected_zero"`). |

### Metrics registry pattern (if one exists)

**There is no metrics registry today.** The OTLP exporter at [`crates/server/src/otel.rs`](../../crates/server/src/otel.rs) ships *spans + log events* to SigNoz; SigNoz can derive count/rate aggregates from those (via the `scope_name` filter pattern in [`observability.md:168-186`](../architecture/observability.md#L168)). Whether to introduce a true counter/histogram registry (e.g. `metrics` crate piped through `metrics-exporter-prometheus` or `opentelemetry`'s metrics SDK) is **open question Q1** — see [Open questions](#open-questions-for-the-user).

If Q1 lands "yes", `crates/observability/` would be the natural home (parallel to `cimmeria-mercury`'s `instrumentation.rs`). If Q1 lands "no, derive from spans", every "metrics opportunity" row below converts to a `tracing::info!(target: "<system>.metric", ...)` event that SigNoz aggregates at query time. Both options are tractable; the cost difference is mostly storage/query efficiency for high-cardinality rates.

## Per-feature audit

### Feature: player-to-player trading system (#438)

- Branch SHA: `34aae5e6`
- Touched: 19 production files + 14 test files. **Already heavily instrumented** — every handler in `cell/cell_methods/player/trade/handlers.rs` carries `#[instrument]` and every reject path emits `warn!` with the correct field set. The cell→base handoff [`handoff.rs:165-202`](../../crates/services/src/cell/cell_methods/player/trade/handoff.rs#L165) emits `error!` on the channel-closed path and `info!` on the success.
- Files in scope:
  - [`crates/services/src/cell/cell_methods/player/trade/handlers.rs`](../../crates/services/src/cell/cell_methods/player/trade/handlers.rs) — 4 inbound dispatchers, fully instrumented.
  - [`crates/services/src/cell/cell_methods/player/trade/state.rs`](../../crates/services/src/cell/cell_methods/player/trade/state.rs) — session lifecycle, fully instrumented (every reject is `warn!` or `info!`).
  - [`crates/services/src/cell/cell_methods/player/trade/handoff.rs`](../../crates/services/src/cell/cell_methods/player/trade/handoff.rs) — cell→base handoff, fully instrumented (`error!` on channel-closed path).
  - [`crates/services/src/base/world_entry/methods/trade/execute/mod.rs`](../../crates/services/src/base/world_entry/methods/trade/execute/mod.rs) — entrypoint span `trade.execute` exists; `warn!` on each abort variant with `reason`/`p1_code`/`p2_code` fields, `info!` on success.
  - [`crates/services/src/base/world_entry/methods/trade/execute/swap.rs`](../../crates/services/src/base/world_entry/methods/trade/execute/swap.rs) — atomic-swap DB-tx internals. **No tracing calls. 18+ error-return points.**
- **Spans missing**:
  - [`swap.rs:28`](../../crates/services/src/base/world_entry/methods/trade/execute/swap.rs#L28) `atomic_swap(pool, p1, p2)` — recommended: `#[tracing::instrument(name = "trade.atomic_swap", level = "info", skip_all, fields(p1_player = p1.player_id, p2_player = p2.player_id, p1_items = p1.item_instance_ids.len(), p2_items = p2.item_instance_ids.len()))]`. Span lets a SigNoz operator slice `trade.atomic_swap` duration distribution by item-count and see which transactions are slow.
  - [`swap.rs`'s `take_advisory_lock`, `lock_items`, `reserve_main_slots_excluding`](../../crates/services/src/base/world_entry/methods/trade/execute/swap.rs) — sub-helpers, debug-span only if Q1 says we want this depth.
- **Happy-path checkpoints missing**:
  - [`swap.rs:51` and `:62`](../../crates/services/src/base/world_entry/methods/trade/execute/swap.rs#L51) — both `InsufficientCash` returns silently roll back. The parent fn arm catches via `Err(reason)` and `warn!`s the abort summary, but the *which sub-step failed* isn't separately greppable. Acceptable — the parent log already names `reason = %reason` which renders as `p1 player 1234 has 50 naquadah, offering 100`. Re-classifying as P2 below.
  - [`swap.rs:100-101`](../../crates/services/src/base/world_entry/methods/trade/execute/swap.rs#L100) — slot-reservation success doesn't log; same logic, the parent's `info!("trade executed atomically")` covers the macro outcome.
- **Error paths missing/under-logged**:
  - [`swap.rs:43-44`](../../crates/services/src/base/world_entry/methods/trade/execute/swap.rs#L43) — `take_advisory_lock(…).await?` propagates `sqlx::Error` as `TradeAbort::DbError(e)`. The parent's `warn!` catches it, but at that level the `sqlx::Error` is rendered via `Display` — a `lock_wait_timeout` from PG looks indistinguishable from `relation_does_not_exist`. P1: add `phase = "advisory_lock"` field to a per-step `debug!` before each `await?`, so the parent's reason concat shows the failing phase, not just the wrapped sqlx error.
- **Metrics opportunities**:
  - `trade_swaps_total{outcome=completed|insufficient_cash|insufficient_slots|bound_item|ineligible_container|db_error}` — counter, low-cardinality labels. The aggregate "what fraction of attempted trades complete vs. abort, and why" is what an ops dashboard wants. P1.
  - `trade_swap_duration_seconds` — histogram on `trade.atomic_swap` span duration. Derivable from spans if Q1 lands "no". P2.
  - **Do NOT** label by `player_id` / `entity_id` — high cardinality.
- **Priority:** P1 — trade is launch-critical, the wire and abort surface is correct, but the **per-phase visibility inside `atomic_swap`** matters for prod triage. Today an "operators see slow trades but can't tell why" page would need to grep across multiple parents.
- **Estimated lines added:** ~40 LOC (one `#[instrument]` on `atomic_swap`, 3-4 per-phase `debug!` checkpoints, counter wiring contingent on Q1).
- **Risks/notes:** the `swap.rs` file is 470 lines of DB transaction logic — every per-step span you add must use `level = "debug"` (not info) or trace volume balloons by 6-8× per trade attempt. Sampling is not the answer (atomic-swap rate is low, ~10/min steady-state); the level discipline is.

### Feature: server-driven NPC cover + reservation + flanking (#429)

- Branch SHA: `03d1f109`
- Touched: 23 production files.
- Files in scope:
  - [`crates/services/src/cell/cover/mod.rs`](../../crates/services/src/cell/cover/mod.rs) — module re-exports only.
  - [`crates/services/src/cell/cover/loader.rs`](../../crates/services/src/cell/cover/loader.rs) — startup-time PG load, fully instrumented (info on count, warn per skipped row with reason).
  - [`crates/services/src/cell/cover/reservation.rs`](../../crates/services/src/cell/cover/reservation.rs) — **Zero tracing calls. 93 lines.**
  - [`crates/services/src/cell/cover/ai_integration.rs:80`](../../crates/services/src/cell/cover/ai_integration.rs#L80) `maintain_cover_for_npc` — per-tick per-NPC hot path. One `warn!` on mutex poison; no other instrumentation.
  - [`crates/services/src/cell/cover/detection.rs`](../../crates/services/src/cell/cover/detection.rs) — pure stateful detection, no tracing (correctly — caller emits).
  - [`crates/services/src/cell/cover/scoring.rs`](../../crates/services/src/cell/cover/scoring.rs) — pure math, no tracing (correct).
  - [`crates/services/src/cell/cover/spatial.rs`](../../crates/services/src/cell/cover/spatial.rs) — pure spatial index, no tracing (correct).
  - [`crates/services/src/cell/service/ticks/cover.rs:31-36`](../../crates/services/src/cell/service/ticks/cover.rs#L31) — has `#[instrument(name="cover.detection_tick", level="debug")]`. Fully wired.
  - [`crates/services/src/cell/content/event_dispatch/cover.rs`](../../crates/services/src/cell/content/event_dispatch/cover.rs) — four `fire_cover_*` / `fire_npc_flanked` helpers, each emits info on chain match + debug on no-match.
  - [`crates/services/src/cell/abilities/death.rs:79-89`](../../crates/services/src/cell/abilities/death.rs#L79) — `space_mgr.cover.release_for_entity(...)` on death. **Silent — no log on slot release.**
- **Spans missing**:
  - [`ai_integration.rs:80`](../../crates/services/src/cell/cover/ai_integration.rs#L80) `maintain_cover_for_npc` — **NOT recommended at info**. Per-tick per-NPC, would be ~50 spans/sec at moderate load. Recommended: keep as-is (rely on the parent `npc_ai.decision` debug span at [`npc_ai.rs:79-84`](../../crates/services/src/cell/service/npc_ai.rs#L79) which already wraps the entire AI decision; add `cover_decision = ?dec` as a `Span::current().record(...)` field at the call site in `npc_ai_fight`).
- **Error paths missing/under-logged**:
  - [`reservation.rs:38-46`](../../crates/services/src/cell/cover/reservation.rs#L38) — `reserve_for_entity` returns `Err(ReserveError::AlreadyReserved { holder })` silently. The caller in [`ai_integration.rs:164-170`](../../crates/services/src/cell/cover/ai_integration.rs#L164) **swallows it without logging** and falls back to `NoCover`. **P0** — this is exactly the "expectation unmet, recoverable" shape that `warn!` exists for. If the reservation table gets out of sync (TOCTOU bug, future async refactor), the failure is silent. Recommended: `warn!(npc_id = %npc_id, slot = ?slot, holder = %current_holder, reason = "cover_slot_taken", "cover reserve_for_entity lost the race — falling back to NoCover")` at the failure site. The convention's example regression-guard pattern fits.
  - [`death.rs:87-89`](../../crates/services/src/cell/abilities/death.rs#L87) — `cover.release_for_entity(...)` doesn't log whether a slot was actually released. Add `if let Some(slot) = cover.release_for_entity(...) { debug!(entity_id, ?slot, "released cover slot on death") }`. Low priority but cheap.
- **Metrics opportunities**:
  - `cover_reservation_state{state=held|released|race_lost}` — counter. Tracks how often the AI gets the slot it wanted vs. raced + dropped. Low cardinality. P1.
  - `cover_detection_events_total{kind=entered|left|duration}` — counter on the cover-detection-tick output. Already partially derivable from `cover.detection_tick` span's `events` field, but a true counter aggregates cleanly across SigNoz time buckets. P2.
- **Priority:** P0 for the reservation race log; P2 for the rest.
- **Estimated lines added:** ~8 LOC (one `warn!` in `ai_integration.rs` reserve-fail arm, one `debug!` in `death.rs` cover-release).
- **Risks/notes:** the player-cover-detection tick already has an instrumented span; resist the urge to add a span on `maintain_cover_for_npc` (called per-NPC per-tick — burns ~50/s).

### Feature: client-telemetry Phase 1 foundation (#421)

- Branch SHA: `6f2d0c1e`
- Touched: 12 files across `crates/launcher/` + new `crates/client-telemetry/`.
- Files in scope:
  - [`crates/client-telemetry/src/lib.rs`](../../crates/client-telemetry/src/lib.rs) — `#[cfg(windows)] mod boot` only. Phase 1 has no telemetry-emission surface — see [`docs/architecture/client-telemetry.md`](../architecture/client-telemetry.md) for the multi-phase plan. **Correctly out of scope for this audit.**
  - [`crates/admin-api/src/routes/telemetry.rs`](../../crates/admin-api/src/routes/telemetry.rs) — adjusted to accept the new `cimmeria-client` `service.name`. Already instrumented via the existing `telemetry::*` span tree from prior PRs.
  - [`crates/launcher/src/telemetry/events.rs`](../../crates/launcher/src/telemetry/events.rs) — launcher-side `Event` enum; launcher already has its own `tracing` subscriber.
- **Verdict: nothing to add this round.** Phase 1 is "DLL loads + bootstrap thread runs". Phase 2 (CME hooks, FFI callbacks, log tees) is where instrumentation discipline starts to matter — call it out then.
- **Priority:** OUT OF SCOPE for this audit.

### Feature: navmesh-extractor Phase 0 + 1.2 (#436, #426)

- Branch SHAs: `9eb4359f`, `a13de803`
- Touched: 12 files in `crates/navmesh-extractor/` (offline tool) and `crates/upk-objects/` (offline parser).
- **Both crates are build-time tools that run during asset prep, not in the server runtime.** They already use `tracing::info!`/`debug!` correctly ([`navmesh-extractor/src/lib.rs:112-222`](../../crates/navmesh-extractor/src/lib.rs#L112) is the gold example — chunks-with-geometry, total-triangles, actors-resolved/unresolved are all logged).
- **Priority:** OUT OF SCOPE — these don't ship telemetry to SigNoz; they emit to a build operator's terminal during `cargo run -p cimmeria-navmesh-extractor`.

### Feature: movement bounds-check + snap-back validation (#437)

- Branch SHA: `95e441fa`
- Files in scope:
  - [`crates/entity/src/movement_validation.rs`](../../crates/entity/src/movement_validation.rs) — pure validator, correctly stateless and unlogged (every reject the caller logs).
  - [`crates/services/src/cell/space_manager/entities.rs:199-244`](../../crates/services/src/cell/space_manager/entities.rs#L199) — `apply_client_position_update`, unlogged but the caller in `base_messages/mod.rs` emits the full negative log on reject.
  - [`crates/services/src/cell/service/base_messages/mod.rs:208-247`](../../crates/services/src/cell/service/base_messages/mod.rs#L208) — **gold-standard negative log** on `ClientMoveOutcome::Rejected`. `warn!` with `target: "movement.validation"`, every field from the convention populated (`entity_id`, `space_id`, `client_x/y/z`, `last_valid_x/y/z`, `bounds_min_x/.../max_z`, `reason = "bounds"`).
- **Spans missing**: none — this is a hot-loop validator, an info-span per call would cost more than it's worth.
- **Error paths missing/under-logged**: none — every reject already produces a `warn!`.
- **Metrics opportunities**:
  - `movement_validation_rejects_total{reason=bounds}` — counter. Aggregates the snap-back rate without high-cardinality entity_id labels. PR2/3/4 will add `speed` / `teleport` / `navmesh` reasons. P1.
- **Priority:** P1 (metric only — logs already perfect).
- **Estimated lines added:** ~3 LOC for the counter wire-up.
- **Risks/notes:** the existing `tracing::warn!(target: "movement.validation")` *is* metric-shaped if Q1 lands "no" — a SigNoz query `count() WHERE target = "movement.validation"` GroupBy `reason` works today.

### Feature: bounds-check NavMesh::load against malicious .nav inputs (#432)

- Branch SHA: `7966349c`
- Files in scope:
  - [`crates/common/src/error.rs`](../../crates/common/src/error.rs) — adds `NavHeaderOutOfRange` variant.
  - [`crates/entity/src/navigation.rs:89-127`](../../crates/entity/src/navigation.rs#L89) — `check_count` + `checked_alloc_size`. Returns `CimmeriaError::NavHeaderOutOfRange` on hostile input. **Silent — no `tracing::warn!`/`error!` on the reject.**
- **Spans missing**: none — this is a load-once boot-path function.
- **Error paths missing/under-logged**:
  - [`navigation.rs:89-97`](../../crates/entity/src/navigation.rs#L89) — `check_count` returns `Err(NavHeaderOutOfRange)` silently. Caller in `NavMesh::load` propagates `?`. Without a `warn!` at the reject site, an operator who sees a navmesh failing to load has to find the parent path that handled the Result — which today emits **nothing** (callers in `space_manager/spawn.rs` use `unwrap_or_default()` patterns). **P1** — a hostile `.nav` would silently disable navmesh for the affected space, players would silently fall through the world. Recommended: `tracing::error!(target: "navmesh.load", file = %path.display(), field, value, reason = "header_out_of_range", "rejected hostile .nav file — space will be navmesh-less")` at the reject site (or one level up where the file path is in scope).
- **Metrics opportunities**: none — boot-path, fires once.
- **Priority:** P1 — a single log line on a load reject is cheap and turns a silent fall-through into an alertable event.
- **Estimated lines added:** ~6 LOC.

### Feature: NPC AI phases 2-7 — Patrol + Wander + Investigating + Follow + Despawning/Submit/Error (#428)

- Branch SHA: `99e1c905`
- Touched: 9 production files.
- Files in scope:
  - [`crates/services/src/cell/service/npc_ai.rs`](../../crates/services/src/cell/service/npc_ai.rs) — 7 new state handlers (`patrol`, `wander`, `investigate`, `follow`, `despawn`, `submit`, `error`). The dispatcher at line 79 wraps every handler in a `tracing::debug_span!("npc_ai.decision", npc_id, ai_state, space_id)`. Inside, each handler emits **state-transition `debug!` events with `event` discriminator fields** (`event = "patrol_arrived"`, `"patrol_waypoint_set"`, `"investigate_arrived"`, `"investigate_routed"`, `"follow_routed"`, etc.). `npc_ai_despawn` emits info; `npc_ai_submit` emits info on cleanup; `npc_ai_error` emits debug-once on entry.
  - [`crates/services/src/cell/combat/threat.rs:60-86`](../../crates/services/src/cell/combat/threat.rs#L60) — `generate_threat` was updated to preempt patrol/wander/investigating/follow into Fighting. Emits `tracing::info!(npc_id, attacker, ?prev, "NPC aggro: preempt -> Fighting")`. Good.
  - [`crates/services/src/cell/content/executor/world/mod.rs:89-198`](../../crates/services/src/cell/content/executor/world/mod.rs#L89) — three new content actions (`SetNpcPoi`, `SetFollowTarget`, `SetNpcAiState`). Each emits `info!` on success and `debug!` on tag-not-found.
  - [`crates/services/src/cell/space_manager/spawn.rs`](../../crates/services/src/cell/space_manager/spawn.rs) — small diff (26 LOC), already covered by parent span.
- **Spans missing**: the per-state handlers (`npc_ai_patrol`, `npc_ai_wander`, etc.) all inherit the parent `npc_ai.decision` debug span. **Recommended addition:** a `decision_outcome` field on the parent span filled by each handler — matches the existing enum convention at [`observability.md:188-201`](../architecture/observability.md#L188) (currently only `npc_ai_fight` populates `decision_outcome`). The new states should drop an additional vocab: `patrol_continue`, `patrol_dwell`, `wander_pick`, `wander_dwell`, `investigate_arrived`, `follow_band`, `despawn`, `submit_init`, `error_hold`. Adds one `Span::current().record("decision_outcome", "patrol_dwell")` per handler. **P1** — the existing convention table in observability.md explicitly enumerates this enum; new states need to extend it (or the SigNoz `groupBy=decision_outcome` query goes stale).
- **Error paths missing/under-logged**: none of significance. The handlers consistently bail to `Idle` on missing config (empty patrol path, no wander radius, no POI, no follow target) with a `broadcast_movement_type(None)` clear — that's the correct quiescent shape.
- **Happy-path checkpoints missing**: state-transition logs are already present at debug.
- **Metrics opportunities**:
  - `npc_ai_decisions_total{decision_outcome}` — counter aggregating all the decision_outcome strings. Naturally bucketed by the new state vocab. P1.
  - `npc_ai_state_transitions_total{from, to}` — counter on state transitions. Useful for "is the preemption working?". Could be high cardinality (9 states × 9 = 81 buckets); acceptable. P2.
- **Priority:** P1 — extend the `decision_outcome` enum in observability.md to match the new state handlers, *and* land the `Span::current().record(...)` calls. The doc edit lands with the code so observability.md doesn't claim only 6 values when there are 9+.
- **Estimated lines added:** ~9 LOC (one `record` per handler) + docs.

### Feature: Phase 1 Crafting — CraftingState + persistence + ASP dispatch fix (#427)

- Branch SHA: `b5042962`
- Files in scope:
  - [`crates/services/src/base/crafting/persistence.rs`](../../crates/services/src/base/crafting/persistence.rs) — load/save round-trip, fully instrumented. `error!` on `rows_affected == 0` UPDATE at line 219-223 (gold-standard negative log). `warn!` on paradigm-level clamp at line 104-110 and on save-time backfill at line 187-193.
  - [`crates/services/src/cell/cell_methods/player/crafting.rs`](../../crates/services/src/cell/cell_methods/player/crafting.rs) — 6 stub handlers, each emits `info!("UNIMPLEMENTED: …")` (correct for Phase 1) or `warn!` on truncated args. The `send_on_update_discipline` helper at line 113-128 has a `warn!` on the silent-send path (`mpsc::Sender::send` dropped).
- **Spans missing**:
  - [`persistence.rs:39`](../../crates/services/src/base/crafting/persistence.rs#L39) `load_crafting_state` — recommended: `#[instrument(name = "crafting.load", level = "info", skip_all, fields(player_id))]`.
  - [`persistence.rs:162`](../../crates/services/src/base/crafting/persistence.rs#L162) `save_crafting_state` — recommended: `#[instrument(name = "crafting.save", level = "info", skip_all, fields(player_id, expertise_count = state.expertise.len()))]`. The 0-rows-affected guard at 218-223 is the canonical "expectation unmet, unrecoverable" `error!` case from negative-logging-convention.md — pair it with a span so the failure is correlatable to the calling context (which world-entry phase, which gain-expertise call).
- **Error paths**: all covered.
- **Metrics opportunities**:
  - `crafting_persist_attempts_total{kind=load|save, outcome=ok|row_not_found|sqlx_error}` — counter. Useful for "are we silently losing crafting state during disconnect races?". P1.
- **Priority:** P1.
- **Estimated lines added:** ~10 LOC (two `#[instrument]` blocks + metric stub).
- **Risks/notes:** Phase 2 lands the actual mutation handlers — the audit recommends instrumenting `spendAppliedSciencePoints`, `craft`, `research`, `reverseEngineer`, `alloying`, `respecCrafting` **at the same time as they get bodies**, not retroactively after Phase 2 lands without spans. Flag for the Phase 2 PR reviewer.

### Feature: trainer routing consolidation + resend on prereq unlock (#424)

- Branch SHA: `40cd2278`
- Files in scope:
  - [`crates/services/src/cell/interactions/trainer.rs`](../../crates/services/src/cell/interactions/trainer.rs) — `try_open_trainer` is **the gold-standard new handler**. Already has structured logs for trainer_empty_offering (warn, target="abilities"), trainer_offered_unbound (warn, target="abilities"), trainer_open success (info), trainer_open_send_failed (error). Nothing to add.
  - [`crates/services/src/cell/interactions/dispatch.rs`](../../crates/services/src/cell/interactions/dispatch.rs), [`crates/services/src/cell/interactions/mod.rs`](../../crates/services/src/cell/interactions/mod.rs) — routing scaffolding, no logic worth instrumenting separately.
- **Spans missing**: `try_open_trainer` could benefit from `#[instrument]` for the structured-field auto-population, but the function-level info log already carries every field a span would. P3 / nice-to-have.
- **Metrics opportunities**:
  - `trainer_opens_total{outcome=opened|no_template|no_archetype|empty_offering}` — counter. Useful for content health ("which trainers are still landing on the empty-offering path"). P2.
- **Priority:** P2.
- **Estimated lines added:** ~3 LOC.

### Feature: chat — wire speaker_flags from access_level + DND state (#425)

- Branch SHA: `a41dcdf1`
- Files in scope:
  - [`crates/services/src/base/dispatch.rs`](../../crates/services/src/base/dispatch.rs) — chat dispatch. **Already heavily instrumented**: the `chatSetDNDMessage` WSTRING-decode failure path at line 137-148 emits `warn!` with `reason = "read_wstring_failed"` (textbook negative-logging-convention). Bound `chatSetAFKMessage` is intentionally `debug!`-only.
  - [`crates/services/src/base/world_entry/play_character.rs`](../../crates/services/src/base/world_entry/play_character.rs) — adds `dnd_message = None` reset on character switch. Existing `world_entry.play_character` span covers this; no callout needed.
- **Verdict: nothing to add.**
- **Priority:** none.

### Feature: chore — remove dead interaction stubs (#435)

- Branch SHA: `ad2c81b3`
- Pure deletion of dead code paths. No telemetry surface change.
- **Priority:** none.

### Feature: chore — remove shadow SPEND_APPLIED_SCIENCE_POINTS arm in social dispatch (#433)

- Branch SHA: `b70a2c6a` (not in the original list — adjacent commit in the chain). One-line dispatch correction. The doc-comment on the crafting dispatch already covers the routing-fix regression guard.
- **Priority:** none.

### Feature: NPC AI state expansion + respawn + setMovementType broadcast (#423)

- Branch SHA: `3e4c6f84`
- Touched: 15 production files.
- Files in scope:
  - [`crates/services/src/cell/service/ticks/npc_respawn.rs`](../../crates/services/src/cell/service/ticks/npc_respawn.rs) — new respawn tick. Has `#[instrument(name = "spawner.npc_respawn_tick", level = "debug", skip_all, fields(ready_count))]` and records `ready_count` per pass.
  - [`crates/services/src/cell/abilities/messaging.rs:178-247`](../../crates/services/src/cell/abilities/messaging.rs#L178) — `broadcast_movement_type` helper. `warn!` on player-misuse path; otherwise correctly silent (dedup via `last_movement_type`).
  - [`crates/services/src/cell/abilities/damage_apply/mod.rs`](../../crates/services/src/cell/abilities/damage_apply/mod.rs) — refactored to route NPC kill through `combat::mark_npc_dead`. Parent span exists.
  - [`crates/services/src/cell/combat/state.rs`](../../crates/services/src/cell/combat/state.rs) (new), [`crates/services/src/cell/combat/mod.rs`](../../crates/services/src/cell/combat/mod.rs) — `mark_npc_dead` helper. Worth a quick check that the helper logs the kill itself (parent damage_apply emits the death broadcast; the helper might be silent — verify in implementation PR).
  - [`crates/services/src/cell/spawner/npcs.rs`](../../crates/services/src/cell/spawner/npcs.rs) — adds `respawn_secs` resolution from spawnlist override → template default. Already covered by parent spawn span.
- **Spans missing**: none — every new entrypoint has a span or inherits the parent.
- **Error paths**: the `npc_respawn_tick` body iterates `ready` and silently `continue`s on missing entity in three places (entity gone between snapshot and mutation). Acceptable — same pattern as `npc_ai_tick`. P3 / not actionable.
- **Happy-path checkpoints**: a respawn promotion is a load-bearing state transition — it'd be nice to emit a single `info!(npc_id, spawn_pos, respawn_secs, "NPC respawned")` at the end of each promotion loop iteration. Today the tick's debug span records `ready_count` but not the per-NPC promotion. **P2.**
- **Metrics opportunities**:
  - `npc_respawns_total{world_name}` — counter. Useful for "are we leaking dead NPCs" / "is the respawn timer working as configured per world". Low cardinality. P2.
- **Priority:** P2.
- **Estimated lines added:** ~4 LOC (one `info!` per promotion).

### Feature: cell — remove shadow SPEND_APPLIED_SCIENCE_POINTS dispatch arm (#433) and codebase-wide test audit (#457)

- Pure cleanup / docs. No telemetry impact.

## Cross-feature gaps

These wouldn't fit cleanly in any single per-feature row above; they cut across multiple PRs.

### G1 — observability.md `decision_outcome` enum is stale

[`observability.md:188-201`](../architecture/observability.md#L188) lists 6 `decision_outcome` values: `attack_in_place`, `chase`, `no_path`, `min_range_backup`, `no_ability`, `leashed`. The PR #428 wave added 7 new AI states (`Patrol`, `Wander`, `Investigating`, `Follow`, `Despawning`, `Submit`, `Error`), and the dispatcher at `npc_ai.rs:79` wraps every state in the `npc_ai.decision` span — but no new `decision_outcome` values are emitted by those handlers today. Result: the SigNoz query "group by `decision_outcome` to find why NPCs aren't engaging" would categorize every patrol/wander/investigate/follow tick as "no `decision_outcome` recorded" — a regression in queryability.

**Action:** the AI Phase 2-7 PR should have extended the enum table in observability.md with the new outcomes alongside the code change (the doc-update map row "Server-side observability" calls this exact link out, but only the ADR is listed, not the enum table within it). Update the table; add `decision_outcome` `Span::current().record(...)` calls in each handler.

### G2 — no metrics registry pattern yet (Q1)

Repeat from per-feature notes. Decision point. See [Open questions](#open-questions-for-the-user).

### G3 — new wire/dispatch surfaces missing from the `target` catalog

[`observability.md:176-186`](../architecture/observability.md#L176) lists the stable target catalog. The wave added these new producer surfaces that **should be registered there** so the SigNoz `scope_name = '<target>'` query model holds:

- `trade.*` (request, cancel, update_proposal, lock_state, execute, atomic_swap) — info.
- `cover.detection_tick` — debug; **already used** in code, **not in catalog table**.
- `movement.validation` — warn; used in code, not in catalog.
- `crafting.{load,save}` — info; new.

**Action:** observability.md's catalog table needs a sweep update when the spans land. P1 — without this, a SigNoz operator who reads the doc as the source of truth doesn't know to filter on the new targets.

### G4 — no cross-feature instrumentation discipline doc for "what counts as info vs warn vs error"

The existing `negative-logging-convention.md` covers *failure logs*. There's no companion convention for *success spans* (when to spend an info-level span vs a debug span). The repo answers this implicitly via [`observability.md:204-213`](../architecture/observability.md#L204) ("cost on the hot path") but new contributors won't know to read that.

**Action:** OUT OF SCOPE for this audit. Open question Q2.

## Suggested PR split

Five follow-up PRs, sized smallest → largest:

### PR1 (P0 emergency) — cover-slot reservation race log + decision_outcome doc fix

- Single `warn!` in [`cover/ai_integration.rs:164`](../../crates/services/src/cell/cover/ai_integration.rs#L164) for the silent reserve-fail. 5 LOC + a regression-guard test using `LogCapture` per negative-logging-convention.md.
- Doc fix: extend observability.md `decision_outcome` enum table to include the 7 new AI states' outcomes (`patrol_continue`, `wander_pick`, `investigate_arrived`, `follow_band`, `despawn`, `submit_init`, `error_hold`).
- **Size:** ~15 LOC + ~10 doc lines.
- **Rationale for going first:** the cover-race log is a silent prod-debugging blocker; the doc fix is the convention drift fix that any subsequent AI-state PR must reference.

### PR2 (P1 system) — trade per-phase observability

- Add `#[instrument]` to `swap.rs::atomic_swap` and `~3` per-phase debug logs (`take_advisory_lock`, `lock_items`, `reserve_main_slots_excluding`).
- Register the `trade.*` family in observability.md's target catalog.
- Add `trade_swaps_total` counter contingent on Q1.
- **Size:** ~40 LOC + ~5 doc lines.

### PR3 (P1 system) — npc_ai decision_outcome record calls + respawn promotion log

- `Span::current().record("decision_outcome", "<state-vocab>")` in each of the 7 new AI state handlers.
- `info!` per-NPC promotion log in `npc_respawn_tick`.
- Add `npc_respawns_total{world_name}` counter contingent on Q1.
- **Size:** ~30 LOC + ~3 doc lines.

### PR4 (P1 cleanup) — crafting persistence spans + navmesh load reject log

- `#[instrument]` on `load_crafting_state` + `save_crafting_state`.
- `error!` at `navigation.rs::check_count` reject site, threading the file path one level up.
- Register `crafting.*` + `navmesh.load` in observability.md target catalog.
- **Size:** ~25 LOC + ~5 doc lines.

### PR5 (P2 polish) — counters, trainer + movement metrics

- `trainer_opens_total{outcome}`, `cover_detection_events_total`, `cover_reservation_state`, `movement_validation_rejects_total{reason}` counters. Contingent on Q1.
- **Size:** ~30 LOC if Q1 = yes; ~0 LOC + a doc note if Q1 = no.

**Total estimated:** ~145 LOC across 5 PRs, plus doc updates within each.

**Shared helper landing first?** If Q1 lands "yes, add a metrics facade", a tiny new `crates/observability/` crate (or a `crates/common/src/metrics.rs` module) wraps the chosen exporter and exposes `counter!`/`histogram!` macros. That'd be PR0, ~50 LOC, gating PR2/3/4/5's metric work. If Q1 = no, PR0 is skipped and counter rows in subsequent PRs convert to `info!` events.

## Out of scope / deferred

- **`crates/client-telemetry/` Phase 1** — DLL loads + bootstrap thread runs; no telemetry-emission surface yet. Revisit in the Phase 2 PR (CME hooks, FFI callbacks).
- **`crates/navmesh-extractor/` Phase 0/1.2** — offline build-time tool. Its `tracing::*` calls are operator-terminal output, not SigNoz-bound.
- **Dependabot rollup (#481)** — pure dep bumps.
- **Doc-only commits #344 P0-P3, #457** — no executable code.
- **chat (#425)** — already textbook-instrumented; no recommendation.
- **trainer (#424)** — already textbook-instrumented; no recommendation (only counter is P2).
- **base/login** — pre-existing instrumentation per PR #414/#410; outside this audit window.
- **Hot-loop spans in `maintain_cover_for_npc` and individual `npc_ai_*` state handlers** — explicitly NOT recommended. Per-NPC per-tick spans would cost more than the visibility is worth; rely on the parent dispatch span + `decision_outcome` field record.
- **swap.rs per-step success logs** — the parent's `info!("trade executed atomically")` is sufficient; per-step success logs would dilute the signal.

## Open questions for the user

**Q1. Metrics registry — do we want a true counter/histogram surface?**

Today everything ships through `tracing::*` and SigNoz aggregates. Adding a metrics SDK (`metrics` crate + `opentelemetry`'s metrics layer) makes "rate over time" queries cheaper and unbundles them from the span volume. The cost is one new crate + one extra layer in `otel.rs`. Recommendation: **defer until the trade abort + movement-validation metrics are needed in a dashboard**, then land as PR0 of this series. Tracing-derived aggregates work fine until then.

**Q2. Should we write a companion "what gets a span vs. an info log vs. a debug log" doc?**

The level-discipline rules in `negative-logging-convention.md` cover *failure* logs explicitly; the *success-side* discipline is implicit (`observability.md:204-213` covers hot-path cost). A small ADR pinning "every dispatch entrypoint gets an info-span; every state transition gets a debug-event; every per-tick decision gets the dispatcher's parent span only, no per-handler span" would close the gap. P2 — not blocking on the audit itself.

**Q3. Does `Span::current().record(...)` for `decision_outcome` belong inside each `npc_ai_*` handler, or hoisted to the dispatcher with a match on `ai_state`?**

The cover-tick uses the in-body `record(...)` pattern. The dispatcher-hoist pattern keeps the per-handler functions purer but means the dispatcher has to know each state's outcome vocab. Either works; the question is style. Recommendation: in-body matches the cover-tick precedent and is more flexible (a handler can record different outcomes based on inner state).

**Q4. Counter cardinality for `npc_ai_state_transitions_total{from, to}`?**

9 states × 9 = 81 buckets is fine for SigNoz/ClickHouse, but the `Patrol → Wander → Patrol → Wander` ping-pong (legitimate, e.g. NPCs cycling between waypoints) inflates the from-to matrix. Decision: keep at P2, observe the cardinality from the simpler `npc_ai_decisions_total{decision_outcome}` counter first.

**Q5. Do we want `service.namespace=cimmeria` and `deployment.environment=colo` resource attributes on every span?**

The OTEL_RESOURCE_ATTRIBUTES env var supports both per [`main.rs:29`](../../crates/server/src/main.rs#L29). Currently no defaults are set. The audit recommends `deployment.environment` be set in the compose file (PR0 candidate) so SigNoz dashboards can split dev/colo on every aggregate.
