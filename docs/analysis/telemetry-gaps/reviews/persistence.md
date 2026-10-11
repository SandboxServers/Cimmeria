# Persistence telemetry review

Reviewer: database-persistence, 2026-10-10. Colo data: last 7 days of `signoz_logs.distributed_logs_v2`, aggregates only.
Routing is fine: every persistence module reaches SigNoz. The rows are the problem. A server shutdown drops every in-world player's position and dirty ammo and logs nothing per player, and 20 of 110 in-world disconnects (18%) produced no position save and no WARN.
A DB stall would freeze the whole cell-to-base path with no row naming it, and the player-load fallbacks put a degraded character in the world under a free-text ERROR.

## 1. Inventory

Persistence has no target of its own. It logs on the module paths of the crates that write: `cimmeria_base_world_entry`, `cimmeria_base_methods`, `cimmeria_base::base`, `cimmeria_base_session`, `cimmeria_cell::cell`, `cimmeria_services`, `cimmeria_resources` and `cimmeria_cell_catalog`. All of them are `=debug` in `OTEL_FILTER` (`crates/server/src/logging/filters.rs:290-381`), and so is `sqlx::query`. No persistence row is DEBUG-only-and-dropped. The only rows the colo never sees are the TRACE shutdown steps in `orchestrator.rs:337-353` ("Cell loop joined cleanly" and the rest).

What fired in the last 7 days:

| Row | Count | Notes |
|---|---|---|
| `sqlx::query` DEBUG | 189,194 | 149,257 are the outbox poll (`base-session/src/base/outbox/mod.rs:358`, every 5 s); 24,883 are the Black Market `SELECT sequence_id, seller_id` poll |
| `sqlx::query` slower than 100 ms | 10 | All 10 are the startup dialog-name load (max 0.86 s). Nothing exceeded 1 s, so sqlx's default slow-statement WARN never fired |
| `Mission state persisted` | 351 | DEBUG, no `account_id` |
| `Loaded player data for mapLoaded` | 382 | INFO, no `account_id` |
| `PersistPosition: persisted` | 90 | INFO, `player_id` but **no `player_name` in any of the 90** and no `account_id` |
| `DisconnectEntity` (cell) | 110 | DEBUG, full identity |
| `Client entities cleaned up` | 105 | `disconnect_reason`: inactivity_timeout 55, duplicate_login 47, logoff 3. No `player_id` |
| `Character created successfully` / `Character deleted` | 57 / 28 | Good: `event`, full identity |
| Shutdowns (`Cell service received shutdown signal`) | 8 | On the colo |
| Persistence WARN/ERROR | 0 | Nothing failed, or it failed silently (§3) |

## 2. Positive gaps

- **No row says that a session's state was queued for saving.** `persist_last_position` (`crates/cell/src/cell/service/base_messages/lifecycle.rs:393-442`) and `flush_dirty_bandolier_ammo` (`crates/cell-combat/.../bandolier/active_slot.rs:20-80`) log only on send failure. When a base-side `persisted` row is missing, nothing tells you whether the cell skipped the save, the channel dropped it, or the base never ran it. This is the 18% gap in §6.
- **Rule 5 on the success rows:** `account_id` is missing from `PersistPosition: persisted` (`base-world-entry/.../cell_dispatch/position.rs:88`), `Mission state persisted` (`base-methods/.../missions/mod.rs:203`), `SystemOptionsUpdate: persisted`, `Loaded player data for mapLoaded` (`player_load/core/player_data.rs:74`), `Loaded inventory items` (`:83`) and `Loaded saved missions from DB` (`missions/mod.rs:68`). `Restored saved mission` has no `player_id` at all. `Client entities cleaned up` (`base-session/src/base/helpers/session_teardown.rs:358-367`) has no `player_id`, and it logs `player_entity_id = ?player_eid`, a Debug-formatted `Option` that SigNoz stores as the string `"Some(n)"`.
- **`player_name` is always absent on `PersistPosition`.** `inventory_dispatch.rs:223-230` looks for the name in the live session, and by the time the message arrives the session has already been torn down. `known_names::player_name(player_id)` is the fallback that works.
- **The pool is invisible.** `DatabasePool::connect` (`crates/services/src/database.rs:52`) calls bare `PgPool::connect`, which takes sqlx's defaults: 10 connections and a 30 s acquire timeout. The whole server shares that one pool (auth, base, cell, outbox, admin API). Nothing logs its configuration, its size or idle count, or acquire waits.
- **No startup cache summary.** Thirty-odd loaders in `crates/cell/src/cell/service/startup.rs:81-635` each log one success row. Nothing says "the server booted with N caches missing".

## 3. Negative gaps

| Site | Shape | Consequence |
|---|---|---|
| `cell/service/message_loop.rs:43-46` | Shutdown `break`s the loop. No disconnect, no `PersistPosition`, no ammo flush, no per-player row | Every deploy silently rolls back in-world players' positions to their last gate or teleport, and loses dirty ammo |
| `lifecycle.rs:398-403` | `let Some(entity) … else { return }`, then `let (true, Some(player_id)) … else { return }` | A player disconnect that skips the position save writes nothing |
| `position.rs:44-48` | No pool, so a silent return | Acceptable in dev, but invisible |
| `position.rs:78-86` | `rows_affected == 0` WARN without `rows_affected`, `expected` or `reason` (Pattern B) | The divergence query misses it |
| `position.rs:98-106` | DB error WARN without `reason`; `?position` is Debug-formatted | |
| `bandolier.rs:45-53` (base) | Zero-rows WARN without `rows_affected`, `expected` or `reason` | |
| `active_slot.rs:73` | "leaving slot dirty for retry" on the logout path, where the entity is destroyed next | The row claims a retry that never happens. The ammo is lost |
| `player_data.rs:216-233` | Player row missing or query failed: `default_player_load_data()` (player_id 0, "Unknown", level 1). The ERROR has no `account_id`, no `reason`, and the error is interpolated into the message | The player enters the world as a stub. Nothing marks the session as degraded |
| `player_data.rs:110-123, 151-153`; `inventory_items.rs:53-60` | ERROR, then `Vec::new()` or `None`, with the error in the message text | The player has an empty bag and no armour or weapon visuals. Later saves run against that state |
| `missions/mod.rs:76-83` | Saved-mission load failure returns `vec![]` | Missions appear never taken. A re-accept UPSERTs over the completed row (`:176-200`), so a transient load error becomes permanent data loss. The row doesn't say so |
| `missions/mod.rs:160-166, 212-218` | Mission delete or UPSERT failure: ERROR with no `reason` or `account_id`, error in the message text, and no consequence | |
| `character_create/mod.rs:79-212, 262, 319-381, 633-636` | About 15 refusals with no `event`, no `reason` and no `account_id`. Only the two transaction failures (`:475`, `:555`) carry `event = character_create_failed` | "Why can't account X create a character?" can only be answered by `addr` |
| `startup.rs:96, 133, 178, 236, …` | `warn!("Failed to load X: {e}")`: no `event`, no `cache`, no structured `error`, mostly no consequence | A partial boot is findable only by grepping message bodies |
| `base/src/base/service.rs:251` | `warn!("Failed to load resource cache: {e}")`, then `None` | Every cooked-data request runs without a cache, and the row doesn't say what that breaks |

## 4. Noise

- `sqlx::query` DEBUG is about 27k rows a day, and 92% of them are two idle pollers (the outbox every 5 s and the Black Market expiry). They are cheap, but they bury the statements that matter. Per-statement `elapsed_secs` is still the only slow-query evidence, so don't drop them until TG-DB-06 lands a real slow-statement WARN (TG-DB-11).
- `Player load data: final appearance after visual merge` (INFO, `player_data.rs:169`) logs `final_components = ?components` once per load. That is low volume and fine.

## 5. Seams (two hops)

| Hand-off | Sender logs | Receiver logs | Can you tell which side dropped it? |
|---|---|---|---|
| Session end → `DisconnectEntity` (base-session `session_teardown.rs`) → cell | `Client entities cleaned up`, no `player_id`; send failure WARNs (#999) | `DisconnectEntity` DEBUG with identity | Only by time, because the base row lacks `player_id`. Fix in TG-DB-10 |
| Cell `DisconnectEntity` → `PersistPosition` → base DB | Send failure only | `persisted` with no name or account; failure WARNs | **No.** A skip on the cell side and a never-ran on the base side look the same (TG-DB-03, -04) |
| Cell → `BandolierAmmoUpdate` → base `sgw_inventory` | Send failure only | DB failure WARN; zero rows logged at DEBUG; no success row | No |
| Server shutdown → in-world players | "Stopping all services" | Nothing | **No player-level evidence at all** (TG-DB-01) |
| Cell → `MissionUpdate` → base UPSERT → next login's `query_saved_missions` | Cell send WARN (#304) | DEBUG on success, free-text ERROR on failure | Mostly yes. The load-side fallback is silent about its consequence (TG-DB-07) |
| Every `CellToBaseMsg` → base serial dispatch loop (`base/src/base/service.rs:310-328`) | `tx.send().await` backpressures the cell | Nothing times the dispatch | **No.** A slow DB write stalls AoI and entity methods for every player, and SigNoz shows only a gap (TG-DB-05) |
| Base write → pool (auth, admin API, outbox all share it) | — | sqlx `elapsed_secs` (execution only, not acquire wait) | No pool-wait signal (TG-DB-06) |
| Items: grant, move, loot | Strong (BV-01..09, grant and loot seams) | Strong | Yes |
| Social: org and character delete (`base-session/.../organization/character_delete.rs`) | `event = delete_character`, `rows_affected` | Trigger audit export WARN | Yes, a model to copy |
| Cooked data: `ResourceCache::load_all` → cooked-data version replies (T4) | Free-text WARN | — | Partly (TG-DB-09) |

## 6. Adversarial

1. **"I logged back in at the gate, not where I logged out"** (players have hit this; see the PersistPosition module doc). *Today:* if the last session ended in a deploy, SigNoz shows `Stopping all services` and nothing per player. Across the 7 colo shutdowns, an estimated 15 player sessions were still in the world, and none of them got a save or a row. *Should show:* one WARN per player, `event = player_state_not_persisted`, `reason = server_shutdown`, with identity, world and x/y/z (TG-DB-01). The real fix is a shutdown flush (TG-DB-02).
2. **Silent position-save skip.** *Today:* of 110 `DisconnectEntity` rows (each with `player_id`), 20 have no `PersistPosition: persisted` row for the same player within 30 s, and no WARN anywhere. All 20 are DebugArea lab sessions on 2026-10-05/06 (10 duplicate_login, 10 inactivity_timeout), on builds whose other disconnects saved normally. SigNoz can't say which branch dropped them. *Should show:* a DEBUG `persist_position_queued` row or a WARN with a `reason` on the cell, then the base row with `account_id` (TG-DB-03, -04).
3. **DB stall or pool exhaustion.** *Today:* the serial base dispatch loop blocks, AoI stops for everyone, and the cell's sends backpressure. If any statement eventually errors, dozens of unrelated ERRORs follow, each with "pool timed out" in its message text. *Should show:* `db.pool` saturation (first row plus `suppressed`), and `cell_dispatch_slow` naming the message kind and how long it took (TG-DB-05, -06).
4. **Missions vanish after a login during a DB blip.** *Today:* one ERROR, "Failed to query saved missions: …", with no account and no consequence. The next accept overwrites the completed row, and nothing links the two events. *Should show:* `event = player_load_degraded`, `reason = missions_query_failed`, the account, and the consequence (TG-DB-07).
5. **A character create fails.** *Today:* `createCharacter: no DB pool` or `Invalid visual group`, keyed only by `addr`. *Should show:* `event = character_create_failed`, `reason`, `account_id`/`account_name` and the wire `error_code` (TG-DB-08).

## Candidate packets

| ID | Title | Sev | Files | Notes |
|---|---|---|---|---|
| TG-DB-01 | Shutdown logs the unsaved state of each in-world player | high | 2 | |
| TG-DB-02 | Flush positions and ammo on shutdown | high | — | `needs-domain-agent` |
| TG-DB-03 | Cell position-save skips and queues are logged | high | 1 | |
| TG-DB-04 | `PersistPosition` rows: identity, Pattern B, `reason` | med | 2 | |
| TG-DB-05 | Slow cell-to-base dispatch WARN | med | 3 | |
| TG-DB-06 | Pool configuration, saturation WARN, 250 ms slow-statement WARN | med | 3 | |
| TG-DB-07 | Player-load fallbacks say they degraded | med | 3 | |
| TG-DB-08 | Character-create refusals carry `event` and `reason` | med | 2 | |
| TG-DB-09 | Startup cache-load failures and summary | low | 3 | |
| TG-DB-10 | `Client entities cleaned up` gains `player_id` | low | 1 | May duplicate the auth/network review |
| TG-DB-11 | Demote the `sqlx::query` statement rows | low | 1 | BlockedDecision, after TG-DB-06 |

**TG-DB-01: Shutdown logs the unsaved state of each in-world player** (high)
- Files: a new `crates/cell/src/cell/service/shutdown_report.rs` with `fn report_unsaved_on_shutdown(space_mgr: &SpaceManager)`, called from the `shutdown.notified()` arm in `message_loop.rs:43` before the `break`.
- For each entity with `is_player` and `player_id` set: WARN `event = "player_state_not_persisted"`, `reason = "server_shutdown"`, the full Rule 5/6 identity (`entity.identity()`), `world` (`get_entity_world_name`), `x`, `y`, `z`, `dirty_ammo_slots = bandolier_ammo_dirty.len()`. Then one INFO `event = "shutdown_unsaved_summary"` with `players_in_world` and `players_with_dirty_ammo`.
- Test: a LogCapture unit test. Build a SpaceManager with one player entity and one NPC, call the fn, and assert exactly one WARN with `reason = server_shutdown` and the player's `player_id`, and none for the NPC. It fails if the call or the emit is removed.
- `OTEL_FILTER`: none (`cimmeria_cell::cell` is already at debug).

**TG-DB-02: Flush positions and ammo on shutdown** (high, `needs-domain-agent`)
- Before the loop breaks, run `persist_last_position` and `flush_bandolier_ammo_for_entity` for every player. Then make `stop_all` wait for the base's cell-dispatch task to drain before it drops the pool (`orchestrator.rs:340-354`).
- Needs design: stop ordering, a drain deadline inside the docker stop grace period, and how the base task learns the channel closed. Test: a live-DB test that starts and stops the services with a player in the world and asserts that `pos_*` changed.

**TG-DB-03: Cell position-save skips and queues are logged** (high)
- File: `crates/cell/src/cell/service/base_messages/lifecycle.rs`, `persist_last_position`.
- Split the two silent returns at `:398` and `:401`. Entity missing: WARN `reason = "entity_missing_at_disconnect"` with `entity_id`. `is_player` with no `player_id`: WARN `reason = "player_id_missing"` with identity. A non-player stays silent. On a successful send: DEBUG `event = "persist_position_queued"` with identity, `world`, `x`, `y`, `z`.
- Test: LogCapture. (a) A player entity whose `player_id` is `None` produces the WARN `player_id_missing`. (b) A normal player produces the DEBUG `persist_position_queued` (with an mpsc receiver held open). Reverting either arm to a silent return fails the test.

**TG-DB-04: `PersistPosition` rows: identity, Pattern B, `reason`** (med)
- Files: `cell_dispatch/position.rs` and `cell_dispatch/inventory_dispatch.rs:219-231`.
- Change the UPDATE to `RETURNING account_id` (`fetch_optional`), and log `account_id` and `account_name = known_names::account_name(..)` on the persisted row. In `inventory_dispatch.rs`, fall back to `known_names::player_name(player_id)` when the session is gone. Zero rows: add `rows_affected = 0`, `expected = 1`, `reason = "player_row_missing"`. DB error: add `reason = "db_write_failed"`. Replace `?position` with numeric `x`, `y`, `z`. Add a DEBUG `reason = "no_db_pool"` on the no-pool path.
- Test: live-DB. Extend `live_db_logout_position_round_trips_with_world_id` to capture logs and assert that the INFO row carries `account_id` = the sentinel and `player_name`. Extend `live_db_persist_no_row_is_silent_warn` to assert `rows_affected` and `expected`. Each fails when the field is removed.

**TG-DB-05: Slow cell-to-base dispatch WARN** (med)
- Files: a new `crates/wire/src/cell/messages/cell_to_base_kind.rs` with `impl CellToBaseMsg { pub fn kind(&self) -> &'static str }` (exhaustive match; `cell_to_base.rs` is 878 lines, so don't grow it); `crates/base/src/base/service.rs:312-326`; and a new `crates/base/src/base/dispatch_timing.rs`.
- Time each `route_cell_message`. Over 250 ms: WARN `event = "cell_dispatch_slow"`, `msg_kind`, `elapsed_ms`, `queue_depth = cell_rx.len()`, Pattern D per `msg_kind` (a 10 s window, with `suppressed`).
- Test: a LogCapture unit test on the throttle-and-emit fn covering a burst (one row, then `suppressed = N-1`) and independence (a second kind's first row is not swallowed). Plus a unit test that `kind()` returns distinct strings for three sample variants.
- `OTEL_FILTER`: none (module path).

**TG-DB-06: Pool configuration, saturation WARN, 250 ms slow-statement WARN** (med)
- Files: `crates/services/src/database.rs`, a new `crates/services/src/db_pool_monitor.rs`, and `crates/server/src/logging/filters.rs`.
- Build the pool with `PgPoolOptions` (max 10 and a 30 s acquire timeout, as today) and `PgConnectOptions::log_slow_statements(LevelFilter::Warn, 250 ms)`. Log INFO `event = "db_pool_configured"` with `max_connections`, `acquire_timeout_ms` and `slow_statement_ms`. Add a 15 s sampler on target `db.pool`: DEBUG with `size` and `idle`; WARN `event = "db_pool_saturated"` when `idle == 0 && size == max`, as Pattern D (first row, then `suppressed`).
- Test: LogCapture on the pure `fn sample(size, idle, max, &mut state)`. Saturation produces a WARN; the next saturated sample is suppressed; the first unsaturated sample resets the state.
- `OTEL_FILTER`: add `db.pool=debug`. The target-scan test will demand it.

**TG-DB-07: Player-load fallbacks say they degraded** (med)
- Files: `player_load/core/player_data.rs`, `player_load/core/inventory_items.rs` and `missions/mod.rs` (`query_saved_missions`).
- Every fallback becomes `event = "player_load_degraded"` with `reason` (`player_row_missing`, `player_query_failed`, `equipment_visuals_query_failed`, `weapon_visual_query_failed`, `inventory_query_failed`, `missions_query_failed`), `player_id`, `player_name`, `account_id` where in scope, `account_name`, and `error = %e` as a field. The message states the consequence: "player enters with an empty bag" or "a re-accept will overwrite saved progress".
- Test: LogCapture against an unreachable lazy pool (the `refusal_infra_tests.rs:37` pattern). Each function emits its `reason` at ERROR. It fails if a reason is renamed or the emit is removed.
- Follow-up, `needs-domain-agent`: refuse world entry instead of loading a stub.

**TG-DB-08: Character-create refusals carry `event` and `reason`** (med)
- Files: `crates/base/src/base/character_create/mod.rs` (659 lines) and a new `character_create/refusal_log.rs` holding `fn log_refusal(addr, account_id, account_name, reason, error_code)`, so that mod.rs shrinks.
- Every early return at `:79-212`, `:262`, `:319-381` and `:630-640` calls it with `event = "character_create_failed"` and a stable `reason` (`name_parse_failed`, `name_rejected`, `extra_name_rejected`, `payload_short`, `invalid_skin_tint`, `unknown_char_def`, `no_db_pool`, `visgroups_query_failed`, `invalid_visual_group`, `forced_group_choice`, `invalid_choice`, `missing_optional_choice`, `name_taken`, `insert_failed`). Keep the existing field values: `name_taken` stays at INFO, client faults stay at WARN, DB errors stay at ERROR.
- Test: a LogCapture case in `fail_code_tests` for `unknown_char_def` and `invalid_skin_tint`, asserting `reason` and `account_id`.

**TG-DB-09: Startup cache-load failures and summary** (low)
- Files: a new `crates/cell/src/cell/service/startup_report.rs` (`fn cache_failed(cache: &'static str, error: &dyn Display, consequence: &'static str)`), `startup.rs` (replace each `warn!("Failed to load …")`; the file is 738 lines, so this must shrink it), and `base/src/base/service.rs:251`.
- WARN `event = "cache_load_failed"` with `cache`, `error` and `consequence`. One INFO `event = "startup_caches"` with `failed_count` and `failed_caches` at the end of `start()`.
- Test: LogCapture on the helper and the summary. Two failures produce `failed_count = 2`.

**TG-DB-10: `Client entities cleaned up` gains `player_id`** (low; may duplicate the auth/network review)
- File: `session_teardown.rs:358-367`. Add `player_id = identity.player_id` and `player_name`, and change `player_entity_id = ?player_eid` to the bare `Option`.
- Test: a LogCapture assertion on the existing teardown test that `player_id` is numeric and present.

**TG-DB-11: Demote the `sqlx::query` statement rows** (low, BlockedDecision until TG-DB-06 lands)
- File: `filters.rs:379`, changing `sqlx::query=debug` to `sqlx::query=info`. That keeps the slow-statement WARNs and drops about 189k statement rows a week. The cost is per-statement `elapsed_secs` history. The owner decides.
- Test: the parity guards (`server_log_targets_reach_an_otlp_index`) stay green.
