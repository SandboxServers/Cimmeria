# Telemetry Gaps Work Packets

> Type: how-to (packet specifications). Audience: the coordinator and packet workers.
> Updated: 2026-10-10 (collation of the 12 reviews). Companions: [README.md](README.md) (findings, decisions, packet status), [work-packets-w3-w4.md](work-packets-w3-w4.md) (W3 per-system packets and W4 client DLL packets), [reviews/](reviews/), [negative-logging-convention.md](../../architecture/negative-logging-convention.md), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [observability.md](../../architecture/observability.md), [TESTING.md](../../../TESTING.md).

This file holds the dispatch rules, the shared contract, the file-collision table and waves W0 to W2. W3 and W4 are in [work-packets-w3-w4.md](work-packets-w3-w4.md). Packet IDs keep the review prefixes; a merged packet lists the IDs it absorbed under **Aliases**.

## Dispatch rules

- **Haiku-sized.** Every packet goes to a Haiku `packet-coder` unless its meta line names another agent. One concern, at most about 3 production files, one named test, under about 100k tokens. A packet marked `needs-domain-agent` names the agent; that agent designs or does the RE and may hand a follow-up packet back to a `packet-coder`.
- **One worktree per packet**, from the main checkout: `pwsh tools/build-lane/mk-worktree.ps1 telemetry-gaps/<ID>-<slug> tg-<id>`. One test database per worktree (`sgw_tg-<id>`). Never two implementers in one checkout.
- **PowerShell and the build lane only.** No bash, no direct `cargo`, no `git worktree prune`, no `git stash`. Lane checks run in this order for every crate on the packet's meta line:

  ```powershell
  pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check
  pwsh tools/build-lane/lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings
  pwsh tools/build-lane/lane.ps1 cargo nextest run -p <crate>
  pwsh tools/build-lane/live-db-test.ps1 <filter>        # live-DB packets only
  ```

  A packet that edits `crates/server/src/logging/filters.rs` also runs `nextest -p cimmeria-server target_scan parity unpaired_id`.
- **Workers commit locally and stop.** They don't push, open PRs or wait. The coordinator runs a `packet-reviewer` (plus the domain advisor for the area), fixes what is real, ships with the `ship-pr` skill and retires the worktree the day it merges.
- **Serialize on file collisions.** Two packets that touch the same production file never run at the same time. The table below is the schedule constraint; within a row, the listed order is the merge order.
- **No behaviour change** unless the packet says so in bold. A telemetry packet that changes game behaviour, a wire byte or a metric label it didn't announce goes back.
- **Doc rows** (shorthand used in every packet; the [doc-update map](../../agents/doc-update-map.md) is the source): `obs` = [observability.md](../../architecture/observability.md) and [observability-target-catalog.md](../../architecture/observability-target-catalog.md); `neg` = [negative-logging-convention.md](../../architecture/negative-logging-convention.md) (new negative row, key renames go in its key-change table); `nt` = lower `crates/server/src/logging/unpaired_id_baseline.txt`; `dst` = [dev-session-telemetry.md](../../architecture/dev-session-telemetry.md) and [operations/telemetry.md](../../operations/telemetry.md); `lab` = [live-research-lab.md](../../architecture/live-research-lab.md); `client` = [client-telemetry.md](../../architecture/client-telemetry.md); `discord` = [discord-notifications.md](../../architecture/discord-notifications.md); `admin` = [docs/tools/admin-api.md](../../tools/admin-api.md); `uat` = [unified-uat.md](../../guides/unified-uat.md); `none` = ledger row only. The packet's own README row is always owed.

## Shared contract

Every packet follows these conventions. A reviewer rejects a packet that invents its own.

### Fields

- **Rule 5 identity.** A row about a player carries `account_id`, `account_name`, `player_id`, `player_name`, resolved with `SpaceManager::player_identity(entity_id)` on the cell or `session_identity::session_identity(state)` / `identity_for_addr(connected, addr)` on the base. A row about a second player uses a role prefix: `target_player_id`, `attacker_account_id`, `old_` for a displaced session.
- **Rule 6 pairs.** Every ID field has its name beside it (`entity_id` + `entity_name`, `template_id` + `template_name`, `mission_id` + `mission_name`). An ID with no resolvable name leaves the name field out; it is never `""`, `"unknown"` or `0`. A packet that adds pairs lowers the `nt` baseline (`NT_BASELINE_BLESS=1` through the lane); a packet that adds an unpaired ID fails the scan.
- **Numbers are numbers.** Pass `Option<i32>`/`Option<u32>` values bare (`player_id = state.active_player_id`), never `?opt`, which ships `"Some(71)"` as a string. `world` is the world name (`&str`), `space_id` is numeric. Positions are `x`, `y`, `z` floats, never `?position`.
- **Errors are fields.** `error = %e`, never interpolated into the message. The message text is a constant.
- **Counts on write guards (Pattern B):** `rows_affected` and `expected`.
- **Throttles (Pattern D):** first row immediately, then at most one per window per key, carrying `suppressed` (the number swallowed since the last row). Each packet that throttles has a burst test and an independence test (a second key's first row is not swallowed).

### Event and reason names

- `event` is snake_case, stable and a `&'static str`. Keep a family's existing dotted prefix where it has one (`chat.*`, `trade.*`, `contacts.*`, `minigame.*`, `cooked_data.*`, `lab.*`); elsewhere use a bare verb phrase (`deferred_dropped`, `respawn_not_scheduled`).
- `reason` is snake_case and from a closed list per call site. Shared values, used with exactly this spelling:

  | `reason` | Meaning |
  |---|---|
  | `cell_channel_closed` | a base→cell `mpsc` send failed (receiver dropped) |
  | `base_channel_closed` | a cell→base send failed. Replaces the reviews' `cell_to_base_closed` |
  | `no_cell_channel` | the base's `Option<Sender>` to the cell is `None` |
  | `entity_missing` | the entity lookup returned `None` |
  | `session_gone` | the client session was torn down first (a normal race; DEBUG) |
  | `buffer_full` | a bounded buffer refused the item |
  | `malformed_args` | a payload shorter than its `.def` layout; with `args_len` and `need` |
  | `no_db_pool` | the DB pool is absent (dev only; DEBUG). Replaces `no_database` |
  | `db_write_failed`, `query_failed` | a DB error; with `error` |
  | `send_failed` | a transport send returned an error |

- Levels follow the negative-logging convention: WARN for an expectation failure a player can feel; DEBUG for a normal race (`session_gone`) and for NPC-only rows; INFO for lifecycle rows a person reads. A WARN that fires on every boot or every packet is a bug.

### Session end: one emitter, reconciled reasons

The auth review and the pipeline review checked the code; S1's draft value `disconnect_reason=logOff` does not exist anywhere. The canonical `disconnect_reason` values on `session.end` and `Client entities cleaned up` are:

| `disconnect_reason` | Emitted by | Status |
|---|---|---|
| `client_disconnect` | `connect_loop/encrypted/mod.rs` (client DISCONNECT) | today |
| `inactivity_timeout` | `base-session/src/base/tick_sync.rs` (60 s reap) | today |
| `send_error` | teardown after a transport failure | today |
| `duplicate_login` | `base/src/base/login/eviction.rs` | today |
| `relaunch_takeover`, `address_reclaimed` | `login/eviction.rs`, `login/relaunch.rs` | today |
| `logoff` | Account `logOff` 0xC2 at character select (`base/src/base/login/mod.rs`), account-only teardown | today; no player, so `Client entities cleaned up` only |
| `logoff_character_select` | `SGWPlayer.logOff(0)` (`base/src/base/dispatch/session.rs`) | **new**, TG-NET-01. Today this path writes no `session.end` |
| `server_shutdown` | `BaseService::stop` | **new**, TG-NET-10 |

- `logoff_full_exit` stays a `path` label on the unlist row, not a `disconnect_reason`: a full exit keeps the session until the client disconnects or times out. TG-NET-02 records it as `logoff_requested = true` on the later `session.end`, whose `disconnect_reason` stays truthful about the mechanism.
- `decrypt_fail` is a `reason`, not a `disconnect_reason`; nothing disconnects there.
- **One emitter.** TG-NET-01 extracts `pub fn log_session_end(row: &SessionEndRow)` in `crates/base-session/src/base/helpers/session_teardown.rs`, with `SessionEndRow { addr, identity, entity_id, entity_name, disconnect_reason: &'static str, session_secs }`. TG-NET-02, TG-MER-07, TG-NET-10 and TG-NET-17 add fields to that struct, never a second `session.end` call. The target stays `session.end` and the message stays `"player session ended"`.
- The `documented_reasons` list in `base-session/src/base/helpers/tests.rs` (`destroy_client_entities_accepts_all_documented_reasons`) gains each new value in the packet that adds it.
- **S1 corrected assertion** (TG-PIPE-08): after `lab_logout`, `session.end disconnect_reason=logoff_character_select` with the run's `player_id`; after `lab_disconnect`, `Client entities cleaned up disconnect_reason=logoff` with the run's `account_id`.

### `session_kind` (D-TG3)

- **Field name** on server rows: `cimmeria.session_kind`, the same key the ingest already stamps on client and launcher rows (`admin-api/src/routes/telemetry/replay.rs`), so one SigNoz filter spans all services. **Values:** `lab` or `player`. Absent before Phase 3 resolves the account.
- **Source:** the server, never the game client. TG-NET-16 adds a test-account flag to `account`; Phase 3 copies it into `ConnectedClientState` (`crates/base-session/src/base/mod.rs`) as `session_kind`. A lab mint's `session_kind=lab` claim on the client-telemetry side is trusted only when it agrees with the flag (D-TG13).
- **Rows that carry it** (TG-NET-17): `session.start`, `session.end`, `Client entities cleaned up`, every `playtest.friction` row, and `launcher.ingest` refusals (TG-PIPE-10 reads it from the claim).

### OTEL_FILTER pin procedure

1. A row on a **module-path target** of a crate that already has an `OTEL_FILTER` row (`crates/server/src/logging/filters.rs`) needs nothing.
2. A **new hand-named target** at DEBUG needs `"<target>=debug"` in `OTEL_FILTER`; an INFO target rides the `info` default but still gets a catalog row. Add the filter entry beside its family, add a row to `observability-target-catalog.md`, and run `nextest -p cimmeria-server target_scan parity`. `every_source_target_reaches_signoz_at_its_level` fails until both exist.
3. **Check prefix shadowing.** EnvFilter matches targets by prefix, so `tower=off` also silences `tower_http` (auth review). A new target must not sit under an `=off` prefix.
4. A **renamed or removed** target removes its old row in the same packet.
5. `filters.rs` is a collision file: only one packet edits it at a time (see below).
6. A new **metric** uses a low-cardinality, enumerated label (`metric_label!`) and gets a row in `observability.md`'s counter table.

## File collision table

Packets on one row run one at a time, merging in the order listed (earlier waves first). Docs tables (`observability-target-catalog.md`, `negative-logging-convention.md`) are append-only rows and rebase cleanly, so they are not listed.

| File | Packets, in merge order |
|---|---|
| `crates/server/src/logging/filters.rs` | first come, one at a time: TG-MER-09, TG-CMB-01, TG-DB-06, TG-DB-11 (after TG-DB-06), TG-MOV-02, TG-MIS-03, TG-MIS-13, TG-AOI-09 |
| `crates/base-session/src/base/helpers/session_teardown.rs` | TG-NET-01, TG-NET-02, TG-MER-07, TG-NET-17, TG-SOC-14 |
| `crates/base/src/base/dispatch/session.rs` | TG-NET-01, TG-NET-02, TG-SOC-14 |
| `crates/base-session/src/base/tick_sync.rs` | TG-NET-14, TG-MER-02 |
| `crates/mercury/src/channel/state.rs`, `channel_core.rs` | TG-MER-07, TG-MER-02, TG-MER-03 |
| `crates/base-session/src/base/helpers/mod.rs` | TG-AOI-04, TG-MER-03 |
| `crates/base-session/src/base/helpers/reliable_fit.rs` | TG-AOI-10, TG-AOI-09 |
| `crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs` | TG-AOI-04, TG-AOI-09 |
| `crates/base-session/src/base/deferred_aoi.rs` | TG-AOI-01, TG-AOI-06 |
| `crates/base/src/base/connect_loop/mod.rs` | TG-MER-10, TG-MER-11, TG-NET-12 |
| `crates/base/src/base/connect_loop/encrypted/mod.rs`, `cell_arms.rs` | TG-MER-05, TG-MER-06 |
| `crates/base/src/base/connect_loop/account_arms.rs` | TG-MER-06, TG-NET-06, TG-MER-13 |
| `crates/base/src/base/service.rs` | TG-NET-10, TG-DB-05, TG-DB-09 |
| `crates/cell/src/cell/service/message_loop.rs` | TG-DB-01, TG-DB-02 |
| `crates/cell/src/cell/service/startup.rs` | TG-CMB-04, TG-DB-09 |
| `crates/cell/src/cell/service/base_messages/player_init/mod.rs` | TG-NET-15, TG-NET-17, TG-CMB-05, TG-MIS-05 |
| `crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs` | TG-ITM-11, TG-CMB-05 |
| `crates/cell-combat/src/cell/combat/state.rs` | TG-CMB-09, TG-NPC-04 |
| `crates/cell-combat/src/cell/abilities/death/mod.rs` | TG-CMB-09, TG-CMB-06, TG-SOC-15 |
| `crates/cell-combat/src/cell/service/npc_ai/dispatch.rs` | TG-NPC-01, TG-NPC-11 |
| `crates/cell-combat/src/cell/service/npc_ai/idle_aggro.rs` | TG-NPC-11, TG-CMB-08 |
| `crates/cell-world/src/cell/service/npc_ai/detectors/aggro_scan.rs` | TG-NPC-03, TG-NPC-07 |
| `crates/cell-world/src/cell/space_manager/npc_population.rs` | TG-NPC-06, TG-NPC-15 |
| `crates/cell-world/src/cell/space_manager/client_move.rs`, `movement_telemetry/mod.rs` | TG-MOV-03, TG-MOV-10, TG-MOV-02 |
| `crates/cell-console/src/cell/console/gm/travel.rs` | TG-MOV-06, TG-MOV-07 |
| `crates/cell-content/src/cell/missions/lifecycle.rs`, `progression.rs` | TG-MIS-01, TG-MIS-08 |
| `crates/cell-world/src/cell/playtest_friction_watch/mod.rs` | TG-NET-17, TG-MIS-02 |
| `crates/content-engine/src/chain/mod.rs` | TG-MIS-09, TG-MIS-03 |
| `crates/base-methods/src/base/world_entry/methods/missions/mod.rs` | TG-DB-07, TG-MIS-11 |
| `crates/base-methods/src/base/world_entry/methods/inventory/appearance.rs` | TG-ITM-01, TG-ITM-14 |
| `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs` | TG-MG-01, TG-MG-04 |
| `crates/minigame/src/minigame/server/mod.rs` | TG-MG-03, TG-MG-06, TG-MG-07 |
| `crates/minigame/src/minigame/session.rs` | TG-MG-04, TG-MG-10 |
| `crates/cell-interactions/src/cell/respawn/mod.rs` | TG-CMB-10 |
| `crates/base-session/src/base/contact_list/handlers/presence_fanout.rs`, `contact_list/persistence/mod.rs` | TG-SOC-05, TG-SOC-04 |
| `crates/admin-api/src/routes/telemetry/bundle_unzip.rs` | TG-PIPE-01, TG-PIPE-15 |
| `crates/client-telemetry/src/uploader.rs`, `boot.rs` | TG-PIPE-02, TG-PIPE-03, TG-PIPE-13, TG-PIPE-14, TG-CLI-04 |
| `crates/client-telemetry/src/hooks/inline_hooks/mercury_recv.rs` | TG-MER-12, TG-CLI-02 |
| `crates/client-telemetry/src/hooks/sinks/bw_message.rs` | TG-CLI-01, TG-CLI-02 |

## Waves

| Wave | Theme | Packets | Gate to start |
|---|---|---|---|
| W0 | Security and data loss | 15 (2 handled outside) | none |
| W1 | Noise reduction (frees ingest budget) | 19 | none; runs beside W0 |
| W2 | Session lifecycle end to end | 17 | TG-NET-01 first |
| W3 | Per-system positive and negative gaps | 95 | W1 noise packets for the same crate merged |
| W4 | Client DLL and launcher | 14 | ships with the next signed launcher release after a lab load check (D-TG4) |

Out-of-scope items the reviews raised go in [§ Not packeted](#not-packeted) at the end of this file.

---

## W0: Security and data loss

### TG-PIPE-01 Bundle replay allowlists log files

W0 · high · **in flight** on branch `fix/bundle-replay-allowlist` (outside the campaign). Track only; don't re-plan. The purge of rows already in SigNoz is D-TG5.

### TG-SOC-01 Strip the webhook URL from Discord send errors

W0 · high · **in review** as PR #1349 (outside the campaign). Track only. The webhook rotation is D-TG6.

### TG-NET-08 `admin.request` row for every admin API request

W0 · high · packet-coder · Ready · crates `cimmeria-admin-api` · docs: admin, obs

- **Change:** `crates/admin-api/src/lib.rs` (`build_router`, the admin router only) and `crates/admin-api/src/request_span.rs`. Replace `DefaultOnResponse`/`DefaultOnFailure` with an `on_response` closure: INFO `target: "admin.request"`, `method`, `path` (from `MatchedPath`, `"unmatched"` when absent so a probe can't write free text), `status`, `latency_ms`, `peer` (IP only, from `ConnectInfo<SocketAddr>`); WARN when `status >= 500`. Leave the login-port router alone (53k upload rows a week). Today no `tower_http` row reaches SigNoz because `tower=off` shadows it.
- **Test:** unit, LogCapture, `admin_request_row_names_method_path_and_status`: oneshot `GET /api/players` with a `ConnectInfo` extension gives one `admin.request` row with `status=200` and `path="/api/players"`. Reverting gives no row. **Ship:** `telemetry-gaps/TG-NET-08-admin-request-row` · `tg-net-08` · `feat(admin-api): TG-NET-08 log every admin API request as admin.request`

### TG-DB-01 Shutdown logs each in-world player's unsaved state

W0 · high · packet-coder · Ready · crates `cimmeria-cell` · docs: neg

- **Change:** new `crates/cell/src/cell/service/shutdown_report.rs` with `fn report_unsaved_on_shutdown(space_mgr: &SpaceManager)`, called in the `shutdown.notified()` arm of `crates/cell/src/cell/service/message_loop.rs` before the `break`. Per player entity with a `player_id`: WARN `event = "player_state_not_persisted"`, `reason = "server_shutdown"`, Rule 5/6 identity, `world`, `x`/`y`/`z`, `dirty_ammo_slots`. Then one INFO `event = "shutdown_unsaved_summary"`, `players_in_world`, `players_with_dirty_ammo`.
- **Test:** unit, LogCapture, `shutdown_report_warns_once_per_player_not_npc`: a SpaceManager with one player and one NPC gives exactly one WARN carrying the player's `player_id`. Removing the call or the emit fails it. **Ship:** `telemetry-gaps/TG-DB-01-shutdown-unsaved-report` · `tg-db-01` · `feat(cell): TG-DB-01 report each in-world player's unsaved state at shutdown`

### TG-DB-02 Flush positions and ammo on shutdown

W0 · high · `needs-domain-agent: database-persistence` (design), then `rust-gameserver-dev` · BlockedDecision (D-TG15) · after TG-DB-01 · crates `cimmeria-cell`, `cimmeria-services`, `cimmeria-base` · docs: `docs/architecture/` stop-order note, uat

- **Change (behaviour):** before the cell loop breaks, run `persist_last_position` and `flush_bandolier_ammo_for_entity` for every player; `stop_all` (`crates/services/src/orchestrator.rs`) waits for the base's cell-dispatch task to drain, bounded by a deadline inside the container stop grace, before dropping the pool. TG-DB-01's WARN then fires only for players the drain missed.
- **Test:** live-DB `live_db_shutdown_flushes_player_position`: start the services with a sentinel player in the world, move it, stop, assert `pos_*` changed. Reverting leaves the old position. **Ship:** `telemetry-gaps/TG-DB-02-shutdown-flush` · `tg-db-02` · `fix(services): TG-DB-02 flush player positions and ammo before shutdown`

### TG-DB-03 Cell position-save skips and queues are logged

W0 · high · packet-coder · Ready · crates `cimmeria-cell` · docs: neg

- **Change:** `crates/cell/src/cell/service/base_messages/lifecycle.rs`, `persist_last_position`. Split the two silent returns: entity missing → WARN `reason = "entity_missing"` with `entity_id`; `is_player` with no `player_id` → WARN `reason = "player_id_missing"` with identity; a non-player stays silent. On a successful send, DEBUG `event = "persist_position_queued"` with identity, `world`, `x`/`y`/`z`.
- **Test:** unit, LogCapture, `persist_position_skip_and_queue_rows`: a player with `player_id = None` gives the WARN; a normal player (receiver held open) gives the DEBUG row. Reverting either arm fails it. **Ship:** `telemetry-gaps/TG-DB-03-position-save-skips` · `tg-db-03` · `feat(cell): TG-DB-03 log skipped and queued position saves at disconnect`

### TG-DB-04 `PersistPosition` rows: identity, Pattern B, `reason`

W0 · med · packet-coder · Ready · crates `cimmeria-base-world-entry` · live-DB · docs: neg, nt

- **Change:** `crates/base-world-entry/src/base/world_entry/cell_dispatch/position.rs` and `cell_dispatch/inventory_dispatch.rs` (:219-231). UPDATE gains `RETURNING account_id` (`fetch_optional`); the persisted row gets `account_id` and `account_name` (`known_names::account_name`). Fall back to `known_names::player_name(player_id)` when the session is gone (all 90 colo rows lacked the name). Zero rows: `rows_affected = 0`, `expected = 1`, `reason = "player_row_missing"`. DB error: `reason = "db_write_failed"`, `error`. Numeric `x`/`y`/`z`. DEBUG `reason = "no_db_pool"` on the no-pool return.
- **Test:** live-DB: extend `live_db_logout_position_round_trips_with_world_id` (assert `account_id` = sentinel and `player_name`) and `live_db_persist_no_row_is_silent_warn` (assert `rows_affected`, `expected`). Each fails with the field removed. **Ship:** `telemetry-gaps/TG-DB-04-persist-position-rows` · `tg-db-04` · `feat(base): TG-DB-04 identity and row counts on PersistPosition`

### TG-DB-07 Player-load fallbacks say they degraded

W0 · med · packet-coder · Ready · crates `cimmeria-base-methods` · docs: neg

- **Change:** `crates/base-methods/src/base/world_entry/methods/player_load/core/player_data.rs`, `player_load/core/inventory_items.rs`, `crates/base-methods/src/base/world_entry/methods/missions/mod.rs` (`query_saved_missions`). Every fallback becomes ERROR `event = "player_load_degraded"`, `reason` ∈ {`player_row_missing`, `player_query_failed`, `equipment_visuals_query_failed`, `weapon_visual_query_failed`, `inventory_query_failed`, `missions_query_failed`}, identity in scope, `error`. The message states the consequence ("a re-accept will overwrite saved progress"). The missions fallback is the #411-class data loss.
- **Test:** unit, LogCapture against an unreachable lazy pool (the `refusal_infra_tests.rs:37` pattern), `player_load_fallbacks_log_degraded_reason`: each function emits its `reason` at ERROR. Renaming a reason or removing an emit fails it. **Ship:** `telemetry-gaps/TG-DB-07-player-load-degraded` · `tg-db-07` · `feat(base-methods): TG-DB-07 player-load fallbacks log player_load_degraded`

### TG-ITM-06 Stale bandolier ammo writeback to WARN with counts

W0 · med · packet-coder · Ready · crates `cimmeria-base-methods` · live-DB · docs: neg

- **Change:** `crates/base-methods/src/base/world_entry/methods/inventory/ammo.rs` (:46-55). WARN `event = "ammo_writeback_stale"`, `rows_affected = 0`, `expected = 1`, `reason = "slot_empty_or_instance_swapped"`, identity. The PR #520 data-loss shape.
- **Test:** live-DB: the existing stale-instance case gains a LogCapture assertion of the WARN and both counts. Reverting to DEBUG fails the level check. **Ship:** `telemetry-gaps/TG-ITM-06-ammo-writeback-stale` · `tg-itm-06` · `feat(base-methods): TG-ITM-06 warn on a stale ammo writeback with row counts`

### TG-ITM-11 Bandolier flush summary and dropped-slot trace

Aliases: persistence review `active_slot.rs:73`. W0 · med · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: neg

- **Change:** `crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs`, `flush_dirty_bandolier_ammo`. WARN `event = "bandolier_slot_dropped"`, `reason = "no_item"` at the dirty-slot-without-item branch (:43). On the logout path, the "leaving slot dirty for retry" row becomes WARN `reason = "ammo_lost_at_logout"` (the entity is destroyed next; no retry happens). End with DEBUG `event = "bandolier_flush"`, `sent`, `dropped`, `unsent`, identity.
- **Test:** unit, LogCapture, `bandolier_flush_reports_dropped_and_unsent`: a dirty slot with no item plus a closed channel gives the WARN and the summary with `dropped = 1`. Reverting removes both. **Ship:** `telemetry-gaps/TG-ITM-11-bandolier-flush-summary` · `tg-itm-11` · `feat(cell-combat): TG-ITM-11 bandolier ammo flush summary and lost-slot rows`

### TG-ITM-09 Loot silent drops and lost cash

W0 · med · packet-coder · Ready · crates `cimmeria-cell-interactions`, `cimmeria-cell-content` · docs: neg

- **Change:** `crates/cell-interactions/src/cell/interactions/loot/mod.rs`: :186 WARN `event = "loot_item_lost"`, `reason = "entity_missing"` (corpse gone after `list.remove`), item pair, `quantity`, `index`; :267 match the `GrantCash` send, on error WARN `event = "loot_cash_lost"`, `reason = "base_channel_closed"`, `amount`; :48 WARN on the `onLootDisplay` send failure. `crates/cell-content/src/cell/content/executor/loot.rs` :227: log `reason = "container_gone"` through the existing `refuse` closure (which also sends the feedback line).
- **Test:** unit, LogCapture in `loot/tests.rs`, `loot_cash_send_failure_warns`: close the receiver, loot cash, assert the WARN with `amount`. Reverting leaves `let _ =`. **Ship:** `telemetry-gaps/TG-ITM-09-loot-silent-drops` · `tg-itm-09` · `feat(cell-interactions): TG-ITM-09 log lost loot items and cash`

### TG-ITM-03 Outbox replay row and identity on outbox WARNs

W0 · med · packet-coder · Ready · crates `cimmeria-base-session` · live-DB · docs: neg

- **Change:** `crates/base-session/src/base/outbox/mod.rs`. Add `entity_id` and `event_type` (a `&'static str` derived from the payload) to the five WARN rows (:296, :303, :377, :401, :409). On `drain_undelivered` success log INFO `event = "outbox_replayed"`, `outbox_id`, `entity_id`, `event_type`, `attempts`. A replayed `ItemUsed` re-runs its chain, so a replay is a possible double consume; today it leaves no trace. The drainer race itself is a follow-up issue (§ Not packeted).
- **Test:** live-DB `live_db_outbox_replay_logs_event_type`: enqueue an `ItemUsed` row, call `drain_undelivered`, assert the INFO row with `event_type = "item_used"`. Reverting removes the row. **Ship:** `telemetry-gaps/TG-ITM-03-outbox-replay-row` · `tg-itm-03` · `feat(base-session): TG-ITM-03 log outbox replays with event type and entity`

### TG-SOC-02 `character.delete_cascade` row

W0 · high · packet-coder (column names confirmed by `database-persistence` in review) · Ready · crates `cimmeria-base-session` · live-DB · docs: neg

- **Change:** `crates/base-session/src/base/organization/character_delete.rs`, `delete_character`: inside the transaction, after the `FOR UPDATE` and before the `DELETE`, count `auctions_destroyed`, `held_cash_destroyed`, `bids_held_lost`, `mail_destroyed`, `mail_attachments_destroyed`, `cod_mails` (FK cascades in `db/sgw/_foreign_keys.sql`). After commit emit `event = "character.delete_cascade"`: WARN if held cash or attachments are nonzero, else DEBUG; identity and the counts. No behaviour change; the refund decision is TG-SOC-03.
- **Test:** live-DB `live_db_character_delete_logs_cascade_counts`: seed an auction with a bid and a mail with an item, delete, assert the WARN counts. Removing the block fails it. **Ship:** `telemetry-gaps/TG-SOC-02-delete-cascade-row` · `tg-soc-02` · `feat(base-session): TG-SOC-02 log what a character delete destroys`

### TG-SOC-03 Character delete: refund or destroy

Dropped: D-TG9 decided to accept destruction, with TG-SOC-02 as the audit.

### TG-DB-12 Lock unreadable missions instead of overwriting them (D-TG18)

W0 · high · `needs-domain-agent: mission-systems-advisor` + `database-persistence` · Ready · after TG-DB-07 · crates `cimmeria-base-methods`, `cimmeria-cell-content`

- **Change (behaviour):** `query_saved_missions` retries the read a few times with a short backoff. If it still fails, the player enters the world with missions marked `state_unknown`. Mission accept and advance for those missions are refused with visible feedback ("Mission data unavailable, try again shortly") and a WARN `event = "mission_state_unknown"`, `reason = "saved_missions_unreadable"`, identity. A background retry reloads the state and unlocks it with an INFO `event = "mission_state_recovered"`. Nothing writes mission rows while the state is unknown. If the design needs more than about 3 production files, the domain agent splits it into a read-retry packet and a lock packet before dispatch.
- **Test:** live-DB `live_db_unreadable_missions_block_reaccept`: make the saved-missions read fail, enter the world, attempt a re-accept of a completed mission, and assert the refusal, the WARN and that the completed row is unchanged. Removing the lock lets the re-accept overwrite the row, which fails the test. **Ship:** `telemetry-gaps/TG-DB-12-missions-unknown-lock` · `tg-db-12` · `fix(base-methods): TG-DB-12 lock unreadable missions instead of overwriting progress`

### TG-MIS-07 Log dropped deferred content actions

W0 · med · packet-coder · Ready · crates `cimmeria-cell-world` · docs: neg

- **Change:** `crates/cell-world/src/cell/space_manager/entities.rs` :144 (destroy) and :451 (disconnect): when `pending_content_actions.remove(&entity_id)` returns a non-empty queue, WARN `event = "deferred_actions_dropped"`, `reason = "destroy_entity" | "disconnect"`, identity, `dropped`, `chain_ids`, `action_kinds` (via `player_journal::action_kind`). A delayed `grant_item` or `advance_step` dropped on relog loses player progress.
- **Test:** add LogCapture to `deferred_content_actions.rs::destroy_entity_drops_pending_content_actions` and `disconnect_entity_drops_pending_content_actions`; assert the WARN and `dropped`. Reverting fails both. **Ship:** `telemetry-gaps/TG-MIS-07-deferred-actions-dropped` · `tg-mis-07` · `feat(cell-world): TG-MIS-07 warn when deferred content actions are dropped`

---

## W1: Noise reduction

Ordered by rows saved. The NPC and Mercury packets alone remove about 75M rows a week.

### TG-MER-08 Delete the per-packet `encrypt`/`decrypt` TRACE rows

W1 · high · packet-coder · Ready · crates `cimmeria-mercury`, `cimmeria-base` (test) · docs: obs

- **Change:** `crates/mercury/src/encryption/mod.rs` :341, :398, :445, :530: delete the four TRACE rows. Keep the HMAC-fail WARNs and add `reason` to them. About −63M rows a week in `cimmeria-trace`.
- **Test:** unit in `cimmeria-base` (LogCapture at TRACE), `encrypt_decrypt_emit_no_per_packet_rows`: encrypt and decrypt one packet, assert no event with message `encrypt` or `decrypt`. Restoring the rows fails it. **Ship:** `telemetry-gaps/TG-MER-08-drop-crypto-trace` · `tg-mer-08` · `fix(mercury): TG-MER-08 drop the per-packet encrypt and decrypt trace rows`

### TG-MER-09 `mercury.packet` to the firehose plus a counter

W1 · high · packet-coder · BlockedDecision (D-TG7: reverses NA25) · crates `cimmeria-mercury`, `cimmeria-server` · docs: obs

- **Change:** `crates/mercury/src/transport.rs` (:71, :112): per-datagram rows move to `wire.firehose.mercury_packet` at TRACE; `mercury.packet` keeps a 1-in-53 sample with `sampled_1_in` and `suppressed` (local `AtomicU64`); counter `mercury_datagrams_total{dir}`. `crates/server/src/logging/filters.rs`: firehose named in the protocol-log layer, off in OTLP. Update the NA25 guard (`crates/server/src/logging/parity_tests/guard.rs:141`) and `target_scan_tests.rs:364`. About −62M rows a week.
- **Test:** unit `mercury_packet_sample_accounts_for_every_datagram`: 106 sends write 2 sampled rows whose `sum(1 + suppressed)` is 106. Reverting writes 106 rows. **Ship:** `telemetry-gaps/TG-MER-09-mercury-packet-firehose` · `tg-mer-09` · `fix(mercury): TG-MER-09 sample mercury.packet and count datagrams`

### TG-NPC-01 Sample unwitnessed Patrol/Wander `npc_ai.tick` rows

W1 · high · packet-coder · Ready · crates `cimmeria-cell-combat`, `cimmeria-cell` (test) · docs: obs

- **Change:** `crates/cell-combat/src/cell/service/npc_ai/dispatch.rs`, `admit_ai_tick_row`: extend the NA24 Idle sample to an NPC whose state is unchanged, is `Patrol` or `Wander`, and has no witnesses; one row per `IDLE_UNWITNESSED_TICK_SAMPLE` with `suppressed`. Any state change or witness still writes every row. About 4.4M rows a week.
- **Test:** service test `unwitnessed_patrollers_sample_their_tick_row` in `crates/cell/src/cell/service/tests/npc_ai/tick_row.rs` (model: `idle_unwitnessed_npcs_sample_their_tick_row`): 5 ticks give 1 row then `suppressed = 4`; a witnessed patroller gives 5. Reverting gives 5. **Ship:** `telemetry-gaps/TG-NPC-01-patrol-tick-sample` · `tg-npc-01` · `fix(npc-ai): TG-NPC-01 sample unwitnessed patrol and wander tick rows`

### TG-NPC-02 Gate `movement.npc` step rows on witnesses

W1 · high · packet-coder · Ready · crates `cimmeria-cell` · docs: obs

- **Change:** `crates/cell/src/cell/service/ticks/npc_movement.rs` (step block, about :257-284): an unwitnessed NPC skips the leg-head step rows and keeps a per-NPC 60 s sample via `npc_detectors.admit_sample` with `suppressed`. `waypoint_reached` and `stop` unchanged. About 4M rows a week.
- **Test:** unit `unwitnessed_npc_steps_are_sampled`: 10 steps of an unwitnessed NPC write at most one `event=step` row; a witnessed NPC writes its head steps. Reverting gives one per head step. **Ship:** `telemetry-gaps/TG-NPC-02-npc-step-sample` · `tg-npc-02` · `fix(cell): TG-NPC-02 sample movement.npc steps for unwitnessed NPCs`

### TG-NPC-03 Drop `no_candidates` aggro-scan rows with zero witnesses

W1 · high · packet-coder · Ready · crates `cimmeria-cell-world` · docs: obs

- **Change:** `crates/cell-world/src/cell/service/npc_ai/detectors/aggro_scan.rs`, `report_scan`: return before the `no_candidates` sample when `witness_count == 0`. 1.71M rows a week.
- **Test:** unit, LogCapture, `no_candidates_needs_a_witness`: `witness_count = 0` writes nothing; `= 1` with no candidate writes DEBUG `event=no_candidates`. Reverting writes a row in the first case. **Ship:** `telemetry-gaps/TG-NPC-03-aggro-scan-unwitnessed` · `tg-npc-03` · `fix(npc-ai): TG-NPC-03 skip no_candidates rows when nobody is watching`

### TG-NPC-09 Sample unwitnessed patrol, wander and path-request leg rows

W1 · med · packet-coder · Ready · after TG-NPC-01 (review context) · crates `cimmeria-cell-combat`, `cimmeria-cell` (test) · docs: obs

- **Change:** `crates/cell-combat/src/cell/service/npc_ai/patrol.rs` (`patrol_arrived`, `patrol_waypoint_set`), `wander.rs` (`wander_arrived`, `wander_waypoint_set`), `path_request.rs` (`request` when `status = ok`): an unwitnessed NPC samples through `npc_detectors.admit_sample` (60 s) with `suppressed`. Failures are never sampled.
- **Test:** service test in `crates/cell/src/cell/service/tests/npc_ai/`, `unwitnessed_patroller_samples_leg_rows`: three legs give one `patrol_arrived`; a `path_fail` is still written. Reverting gives three. **Ship:** `telemetry-gaps/TG-NPC-09-leg-row-sample` · `tg-npc-09` · `fix(npc-ai): TG-NPC-09 sample leg and path rows for unwitnessed NPCs`

### TG-MER-10 Throttle WSAECONNRESET with a suppressed count

Aliases: TG-NET-11. W1 · low · packet-coder · Ready · crates `cimmeria-base` · docs: neg

- **Change:** `crates/base/src/base/connect_loop/mod.rs` :96: extract `log_connreset()`; one DEBUG row per 10 s with `suppressed` and `reason = "icmp_port_unreachable"`, plus counter `udp_connreset_total`. 18.7k rows a week today.
- **Test:** unit `connreset_burst_writes_one_row`: a burst of 50 writes 1 row; the next row after the window carries `suppressed = 49`. Reverting writes 50. **Ship:** `telemetry-gaps/TG-MER-10-connreset-throttle` · `tg-mer-10` · `fix(base): TG-MER-10 throttle WSAECONNRESET rows`

### TG-SOC-11 Discord config watcher self-trigger

W1 · med · packet-coder · Ready · crates `cimmeria-discord` · docs: discord

- **Change:** `crates/discord/src/config/watcher.rs`: in the `notify` callback drop `EventKind::Access(_)` and `Other`; only `Create|Modify|Remove` queue a reload. Up to 56k `no semantic change` rows a day on the Linux colo.
- **Test:** unit, LogCapture, current-thread runtime, `config_reads_do_not_trigger_reload`: write once → one `Discord config reloaded`; read the file 10 times and wait past the debounce → no more rows. Only meaningful on Linux CI (Windows reports no access events); say so in the comment. **Ship:** `telemetry-gaps/TG-SOC-11-discord-watcher-loop` · `tg-soc-11` · `fix(discord): TG-SOC-11 ignore access events in the config watcher`

### TG-AOI-10 Demote single-packet AoI bundle rows to DEBUG

Aliases: T9 (server half). W1 · med · packet-coder · Ready · crates `cimmeria-base-session` · docs: obs

- **Change:** `crates/base-session/src/base/helpers/reliable_fit.rs` (:439-451, "AoI bundle: flushed"): INFO only when `packets > 1` or the bundle fragmented, DEBUG otherwise. About 7.5k of 7.8k rows a week move to DEBUG.
- **Test:** unit, LogCapture, `single_packet_aoi_bundle_logs_at_debug`: a 2-message single-packet flush logs DEBUG; a 2-packet flush logs INFO. Reverting logs INFO for both. **Ship:** `telemetry-gaps/TG-AOI-10-aoi-bundle-level` · `tg-aoi-10` · `fix(base-session): TG-AOI-10 log single-packet AoI bundles at debug`

### TG-MOV-11 Drop the per-packet `EntityMove` TRACE

W1 · low · packet-coder · Ready · crates `cimmeria-cell` · docs: obs

- **Change:** `crates/cell/src/cell/service/base_messages/movement.rs` :54-59: delete the unsampled per-packet row; `movement.player` and `movement.position_sample` cover it.
- **Test:** unit, LogCapture, `entity_move_emits_no_per_packet_row`: one `handle_entity_move` emits no event with message `EntityMove`. Restoring fails it. **Ship:** `telemetry-gaps/TG-MOV-11-drop-entitymove-trace` · `tg-mov-11` · `fix(cell): TG-MOV-11 drop the per-packet EntityMove trace row`

### TG-MOV-03 Speed warn: separate bunched packets

W1 · med · packet-coder · Ready · crates `cimmeria-cell-world` · docs: obs

- **Change:** `crates/cell-world/src/cell/space_manager/client_move.rs`, the `kin.speed_warn` arm (:382): when `sample.dt_secs <= 1e-4` or `implied_speed` is not finite, DEBUG `reason = "speed_bunched"` (ratio omitted) and count `movement_validation_warns_total{reason="speed_bunched"}` instead of the WARN. The validator's decision is unchanged. 47% of the 2,261 weekly WARNs are bunching, and their infinite ratio makes the calibration quantile NaN.
- **Test:** unit, LogCapture, time-injected `apply_client_position_update_at`, `bunched_packets_do_not_speed_warn`: two packets at one `Instant` 0.3 u apart give no WARN and one DEBUG; a third at +0.1 s and 5 u still WARNs with a finite ratio. Reverting brings the WARN back. **Ship:** `telemetry-gaps/TG-MOV-03-speed-bunching` · `tg-mov-03` · `fix(cell-world): TG-MOV-03 log bunched movement packets apart from speed warnings`

### TG-MOV-10 GM navmesh bypass: INFO, throttled

W1 · low · packet-coder · Ready · after TG-MOV-03 · crates `cimmeria-cell-world` · docs: obs

- **Change:** `client_move.rs` :287-318: WARN → INFO, throttled through the `movement_telemetry` admit (5 s per entity) with `suppressed`; the counter still counts every packet.
- **Test:** unit, LogCapture, `gm_bypass_row_is_throttled_info`: 3 GM off-mesh packets in 1 s give one INFO; a 4th at +6 s gives a row with `suppressed = 2`. **Ship:** `telemetry-gaps/TG-MOV-10-gm-bypass-info` · `tg-mov-10` · `fix(cell-world): TG-MOV-10 throttle the GM navmesh bypass row at info`

### TG-MOV-14 Dial-hub GM grant WARN to INFO

W1 · low · packet-coder · Ready · crates `cimmeria-cell-interactions` · docs: none

- **Change:** `crates/cell-interactions/src/cell/gate_travel/dial_hub.rs`: the two "granting a GM every enterable gate" rows go WARN → INFO. The gmDHD outbound-only refusal stays WARN.
- **Test:** unit, LogCapture level assertion `gm_gate_grant_logs_at_info`. **Ship:** `telemetry-gaps/TG-MOV-14-dial-hub-level` · `tg-mov-14` · `fix(cell-interactions): TG-MOV-14 log the GM dial-hub grant at info`

### TG-CMB-04 Quiet the boot `effect_script_unregistered` WARN

W1 · med · packet-coder · Ready · crates `cimmeria-cell-world`, `cimmeria-cell`, `cimmeria-cell-effect-scripts` · live-DB · docs: neg

- **Change:** `crates/cell-world/src/cell/effects/registry.rs` (`unregistered` skips a blank `script_name` and `NATIVE_EFFECT_NAMES = ["Reload"]`); `crates/cell/src/cell/service/startup.rs:346` (native/blank ones once at INFO `event = "effect_script_native"`, WARN only for the rest); `crates/cell-effect-scripts/src/cell/effects/registry.rs` (`KNOWN_UNSCRIPTED` empty, doc comment updated). Fires on every boot today.
- **Test:** unit in cell-world `registry.rs`, `unregistered_skips_blank_and_native`: defs `""`, `Reload`, `Bogus` yield only `Bogus`. The live-DB guard asserts an empty list on the seed. Reverting fails both. **Ship:** `telemetry-gaps/TG-CMB-04-effect-boot-warn` · `tg-cmb-04` · `fix(cell): TG-CMB-04 stop warning about native and blank effect scripts at boot`

### TG-CMB-09 Demote NPC-only launch and kill INFO rows

W1 · low · packet-coder · Ready · crates `cimmeria-cell-combat` · docs: obs

- **Change:** `crates/cell-combat/src/cell/abilities/use_ability/handle.rs:539` (`ability_launched`), `abilities/death/mod.rs:451` (`target_killed`), `crates/cell-combat/src/cell/combat/state.rs` (`NPC death: respawn scheduled`): INFO when a player or a player's pet is caster, attacker or target; DEBUG otherwise. Move `timer_update_not_sent reason=not_player` (`abilities/timer_update.rs`) to TRACE. About 1.6k INFO rows a day.
- **Test:** unit, LogCapture, `npc_only_launch_logs_at_debug`: NPC-on-NPC launch logs DEBUG; a player launch INFO. **Ship:** `telemetry-gaps/TG-CMB-09-npc-only-rows` · `tg-cmb-09` · `fix(cell-combat): TG-CMB-09 log NPC-only casts and kills at debug`

### TG-SOC-13 Mail sweep: no row when nothing was scanned

W1 · low · packet-coder · Ready · crates `cimmeria-base-methods` · docs: none

- **Change:** `crates/base-methods/src/base/world_entry/methods/mail/expiry/mod.rs`, `log_summary`: return early when `scanned == 0 && failed == 0` for both sources (100% of 4,556 rows in 30 days).
- **Test:** unit, LogCapture, `empty_mail_sweep_is_silent`: empty sweep → no row; one expiry → one. **Ship:** `telemetry-gaps/TG-SOC-13-mail-sweep-quiet` · `tg-soc-13` · `fix(base-methods): TG-SOC-13 skip the mail sweep row when nothing was scanned`

### TG-MIS-12 Demote no-listener cover rows; collapse `add_dialog_set`

W1 · low · packet-coder · Ready · crates `cimmeria-cell-content` · docs: none

- **Change:** `crates/cell-content/src/cell/content/event_dispatch/cover.rs`: `no chains matched` at DEBUG only when the engine has a chain for that trigger type, else TRACE (3,348 rows a week). `crates/cell-content/src/cell/content/executor/dialog/mod.rs` :191-247: merge three INFO rows into one `Content: adding dialog set`, keeping every field.
- **Test:** unit, LogCapture, `add_dialog_set_logs_one_info`: exactly one INFO per action; no DEBUG for a cover event with an empty engine. **Ship:** `telemetry-gaps/TG-MIS-12-content-noise` · `tg-mis-12` · `fix(cell-content): TG-MIS-12 quiet cover no-match rows and collapse add_dialog_set`

### TG-MER-14 `cooked_data.sync_finish session_gone_before_start` to DEBUG

W1 · low · packet-coder · Ready · crates `cimmeria-base-session` · docs: none

- **Change:** `crates/base-session/src/base/cooked_sync/task.rs`: the `outcome=abandoned reason=session_gone_before_start` row goes WARN → DEBUG (a normal logout race, 12 rows a week). Other abandon reasons stay WARN.
- **Test:** unit, LogCapture, `sync_abandoned_by_logout_is_debug`. **Ship:** `telemetry-gaps/TG-MER-14-cooked-sync-race-level` · `tg-mer-14` · `fix(base-session): TG-MER-14 log the cooked-sync logout race at debug`

### TG-DB-11 Demote the `sqlx::query` statement rows

W1 · low · packet-coder · BlockedDecision (D-TG8) and BlockedDependency (TG-DB-06) · crates `cimmeria-server` · docs: obs

- **Change:** `crates/server/src/logging/filters.rs:379`: `sqlx::query=debug` → `sqlx::query=info`, keeping TG-DB-06's slow-statement WARN. Drops about 189k rows a week (92% are two idle pollers) and loses per-statement `elapsed_secs`.
- **Test:** the parity guards (`server_log_targets_reach_an_otlp_index`) stay green; add `sqlx_slow_statement_warn_still_exports`. **Ship:** `telemetry-gaps/TG-DB-11-sqlx-demote` · `tg-db-11` · `fix(server): TG-DB-11 export sqlx statements from info`

---

## W2: Session lifecycle end to end

TG-NET-01 lands first: it creates the `log_session_end` emitter the rest extend.

### TG-NET-01 `session.end` on logOff to character select

Aliases: TG-PIPE-09. W2 · high · packet-coder · Ready · crates `cimmeria-base`, `cimmeria-base-session` · docs: neg, obs

- **Change:** `crates/base-session/src/base/helpers/session_teardown.rs`: extract `log_session_end(&SessionEndRow)` (shared contract) and call it from the existing :254 site. `crates/base/src/base/dispatch/session.rs`, `handle_log_off`: in the `disconnect == 0` branch, when `ended` is `Some`, call it with `disconnect_reason = "logoff_character_select"`. Move the `SGWPlayer.logOff` row (:34) after the identity snapshot and give it the identity quartet. Explains the 58 `session.start` rows with no end.
- **Test:** unit, LogCapture next to `dispatch/tests/player_index_logoff.rs`, `logoff_to_character_select_ends_the_session`: `logOff(0)` for an in-world session gives one `session.end` with `disconnect_reason=logoff_character_select` and `player_id`. Reverting gives none. Add the value to `documented_reasons`. **Ship:** `telemetry-gaps/TG-NET-01-session-end-char-select` · `tg-net-01` · `feat(base): TG-NET-01 end the session row on logout to character select`

### TG-NET-02 End-cause hints on `session.end`

Aliases: TG-DB-10. W2 · high · packet-coder · BlockedDependency (TG-NET-01) · crates `cimmeria-base-session`, `cimmeria-base` · docs: neg

- **Change:** `session_teardown.rs`: `pub struct LogoffRequested;`; `SessionEndRow` gains `logoff_requested`, `idle_ms` (from `c.last_recv`), `in_world`, `world`. `dispatch/session.rs`: the full-exit branch inserts `LogoffRequested` into `c.extensions`. The "Client entities cleaned up" row (:358-367) gains `player_id`, `player_name` and the bare `player_entity_id` (today `?player_eid` ships `"Some(n)"`). Channel counters come from TG-MER-07, not here.
- **Test:** unit, LogCapture in `session_end_names_tests.rs`, `logoff_requested_survives_to_inactivity_teardown`: a session with the marker, torn down as `inactivity_timeout`, logs `logoff_requested=true` and an `idle_ms`; the cleanup row has numeric `player_id`. Reverting drops the fields. **Ship:** `telemetry-gaps/TG-NET-02-session-end-hints` · `tg-net-02` · `feat(base-session): TG-NET-02 say how a session ended on session.end`

### TG-MER-07 Channel health on `session.end`

W2 · med · packet-coder · BlockedDependency (TG-NET-02) · crates `cimmeria-mercury`, `cimmeria-base-session` · docs: obs

- **Change:** `crates/mercury/src/channel/state.rs` and `channel_core.rs`: a `retransmits_total: u64` bumped in `check_timeouts`, and `health()` returning `tx_outstanding`, `oldest_unacked_age_ms`, `retransmits_total`, `tx_holes`, `tx_hole_stalls`, `rx_stalls`, `srtt_ms`, `last_rx_age_ms`. `session_teardown.rs`: snapshot before `clients.remove` into `SessionEndRow`. Tells "client vanished" from "server stopped acking".
- **Test:** unit, the existing teardown LogCapture tests, `session_end_carries_channel_health`: a channel with 2 unacked entries gives `tx_outstanding=2`. Reverting drops the field. **Ship:** `telemetry-gaps/TG-MER-07-session-end-channel-health` · `tg-mer-07` · `feat(mercury): TG-MER-07 channel health on session.end`

### TG-NET-03 Duplicate-login eviction says whether the old session was alive

W2 · high · packet-coder · Ready · crates `cimmeria-base` · docs: neg

- **Change:** `crates/base/src/base/login/eviction.rs`: `Displaced` gains `idle_ms` (from `last_recv`) and `in_world`. The duplicate and relaunch rows log `old_idle_ms`, `old_in_world`, `old_session_secs`, `reason = "duplicate_login"`; INFO when `old_idle_ms >= 15_000` (a crash-and-relaunch), WARN otherwise (a real concurrent login). The `let _ = transport.send_to(LOGGED_OFF)` at :172 logs DEBUG `reason = "send_failed"` on error.
- **Test:** unit, LogCapture in `login/tests`, `stale_duplicate_login_is_info`: an old session with `last_recv` 20 s ago gives INFO with `old_idle_ms >= 20000`; a fresh one gives WARN. **Ship:** `telemetry-gaps/TG-NET-03-duplicate-login-liveness` · `tg-net-03` · `feat(base): TG-NET-03 duplicate-login rows say whether the old session was alive`

### TG-NET-10 `session.end server_shutdown` for every live session

W2 · med · packet-coder · BlockedDependency (TG-NET-01) · crates `cimmeria-base` · docs: neg

- **Change:** `crates/base/src/base/service.rs`, `BaseService::stop`: under the `connected_clients` lock, call `log_session_end` with `disconnect_reason = "server_shutdown"` for each session with a `player_entity_id` (log only, no teardown), then INFO `sessions_open_at_stop = n`. Add the value to `documented_reasons`.
- **Test:** unit `stop_ends_each_live_session`: one in-world session, `stop()`, assert the row. Reverting gives none. **Ship:** `telemetry-gaps/TG-NET-10-shutdown-session-end` · `tg-net-10` · `feat(base): TG-NET-10 end every live session with server_shutdown at stop`

### TG-NET-14 Inactivity-timeout row says what the session was doing

W2 · med · packet-coder · BlockedDependency (TG-MER-07) · crates `cimmeria-base-session` · docs: none

- **Change:** `crates/base-session/src/base/tick_sync.rs`: the inactivity row (:115) gains `in_world`, `world`, `session_secs`, `tx_outstanding` (from TG-MER-07's `health()`); the two "session cancelled" rows (:106, :193) gain the identity quartet.
- **Test:** extend `inactivity_timeout_line_names_the_player_and_the_account` with asserts for `tx_outstanding` and `in_world`. **Ship:** `telemetry-gaps/TG-NET-14-inactivity-context` · `tg-net-14` · `feat(base-session): TG-NET-14 context on the inactivity-timeout row`

### TG-NET-13 Dev-session refusals no longer dropped

W2 · med · packet-coder · Ready · crates `cimmeria-admin-api` · docs: dst

- **Change:** `crates/admin-api/src/routes/dev_session/handlers.rs`, `log_refusal`: replace `_ => {}`: WARN `reason = "dev_session_kill_switch"` for `KillSwitchActive`; INFO with `elapsed` and `cap` for `Expired` and `SessionLifetimeExceeded`; INFO for bad-token refusals (already bounded by `refresh_bad`). A refused refresh silently ends a client's telemetry stream today (feeds T8).
- **Test:** extend the `tests.rs` refresh cases with LogCapture, `refresh_refusals_log_their_reason`, asserting `reason` per case. **Ship:** `telemetry-gaps/TG-NET-13-dev-session-refusals` · `tg-net-13` · `feat(admin-api): TG-NET-13 log every dev-session refusal`

### TG-NET-15 `session.start` says first entry or travel

W2 · low · `needs-domain-agent: network-security-auth` (with movement-teleport-advisor) · Ready · crates `cimmeria-cell` · docs: obs

- **Change:** `crates/cell/src/cell/service/base_messages/player_init/mod.rs`: `session.start` gains `entry = "login" | "travel"`. The agent first confirms which `InitPlayerState` field or caller distinguishes gate travel (`base-world-entry/.../gate_travel/mod.rs` re-sends it), so starts and ends can be paired.
- **Test:** unit, LogCapture, `session_start_marks_travel_entries`: a travel `InitPlayerState` logs `entry=travel`. **Ship:** `telemetry-gaps/TG-NET-15-session-start-entry` · `tg-net-15` · `feat(cell): TG-NET-15 session.start says login or travel`

### TG-NET-16 Test-account flag on `account`

Aliases: D-TG3 (data half). W2 · med · `needs-domain-agent: database-persistence` · Ready · crates `cimmeria-auth`, `cimmeria-base` · live-DB · docs: [db/README or schema doc], obs

- **Change:** `db/sgw/Accounts/Tables/account.sql` gains `test_account boolean DEFAULT false NOT NULL` (schema edit, no `db/scripts` migration); the lab accounts in `db/resources/` seeds set it. Phase 3 (`crates/base/src/base/login/mod.rs`) reads it and stores `session_kind: Option<&'static str>` on `ConnectedClientState` (`crates/base-session/src/base/mod.rs`).
- **Test:** live-DB `live_db_lab_account_is_test_account`: the seeded lab account loads with `session_kind = Some("lab")`, a player account with `"player"`. Reverting the column read fails it. **Ship:** `telemetry-gaps/TG-NET-16-test-account-flag` · `tg-net-16` · `feat(db): TG-NET-16 flag test accounts and resolve session_kind at login`

### TG-NET-17 `cimmeria.session_kind` on server session rows

Aliases: T6. W2 · high · packet-coder · BlockedDecision (D-TG13) and BlockedDependency (TG-NET-16, TG-NET-01) · crates `cimmeria-base-session`, `cimmeria-cell`, `cimmeria-cell-world` · docs: obs

- **Change:** add `cimmeria.session_kind` to `SessionEndRow` (`session_teardown.rs`), to `session.start` (`player_init/mod.rs`, carried on `InitPlayerState` or resolved from the base identity) and to every `playtest.friction` row (`crates/cell-world/src/cell/playtest_friction_watch/mod.rs`, cached on the watch the first time `player_tick` sees the entity). All 8 friction stalls in the triage were lab runs.
- **Test:** unit, LogCapture, `friction_rows_carry_session_kind`: a lab session's `step_stalled` WARN has `cimmeria.session_kind=lab`. Reverting drops it. **Ship:** `telemetry-gaps/TG-NET-17-session-kind` · `tg-net-17` · `feat(base-session): TG-NET-17 stamp session_kind on session and friction rows`

### TG-NET-18 World entry says whether the session has a client telemetry stream

Aliases: T8. W2 · med · `needs-domain-agent: network-security-auth` · BlockedDecision (D-TG14: join key) · crates `cimmeria-admin-api`, `cimmeria-base-world-entry`

- **Change (design first):** pick the join key between a game session and a dev-session telemetry stream (the mint carries no account today), then log `client_telemetry = "present" | "absent"` and the stream's `session_id` on the world-entry row. A second packet may follow once the key exists.
- **Test:** unit on the chosen join; the guard fails when the field is missing. **Ship:** `telemetry-gaps/TG-NET-18-telemetry-stream-presence` · `tg-net-18` · `feat(base-world-entry): TG-NET-18 log whether a session has client telemetry`

### TG-PIPE-17 labd ships its log to SigNoz

Aliases: D-TG2. W2 · high · `needs-domain-agent: rust-gameserver-dev` (new dependencies, `cargo hakari`) · Ready · crates `cimmeria-lab` · docs: lab, obs

- **Change:** `crates/lab/src/main.rs` (subscriber setup, :191-225): add an OTLP log exporter with `service.name=cimmeria-lab`, endpoint from `labd.env`, off when unset; a redaction layer reduces local paths to the instance name. Keep `labd.log`. The exporter keeps `lab.*` targets at INFO and `lab.client` (TG-PIPE-05).
- **Test:** unit `labd_redacts_local_paths`: a row with `C:\Users\x\...\inst-3\SGW.exe` exports as `inst-3`. Reverting exports the path. **Ship:** `telemetry-gaps/TG-PIPE-17-labd-otlp` · `tg-pipe-17` · `feat(lab): TG-PIPE-17 export labd logs to SigNoz as cimmeria-lab`

### TG-PIPE-05 labd launch, stop and pid-overwrite rows

W2 · med · packet-coder · Ready · crates `cimmeria-lab` · docs: lab

- **Change:** `crates/lab/src/supervisor/mod.rs` (`launch_client` gains `cause: &'static str`), `supervisor/lifecycle.rs`, `supervisor/watchdog.rs`. INFO target `lab.client`, `event = "launched"`, `instance`, `pid`, `cause` ∈ {`start`, `restart`, `watchdog_relaunch`}, `telemetry_session_id`, `grant` ∈ {`minted`, `reused`, `unavailable`}; WARN `event = "pid_overwritten"`, `previous_pid`, `previous_alive`, when `st.pid` is `Some` and alive (:469; #1342); INFO `event = "stopped"`, `pid`, `exited`, `cause`.
- **Test:** unit, LogCapture with `fake_bridge`, `second_launch_without_stop_warns_pid_overwritten`: two launches give one `pid_overwritten`. Reverting gives none. **Ship:** `telemetry-gaps/TG-PIPE-05-labd-launch-rows` · `tg-pipe-05` · `feat(lab): TG-PIPE-05 log client launches, stops and pid overwrites`

### TG-PIPE-06 labd per-tool-call row

W2 · med · packet-coder · Ready · crates `cimmeria-lab` · docs: lab

- **Change:** `crates/lab/src/server/handler.rs`, `call_tool`: INFO `lab.call`, `tool`, `instance`, `lease_owner`, `outcome` ∈ {`ok`, `error`, `refused`}, `duration_ms`, `error` (capped at 200 chars). No args (they can carry Lua or chat text).
- **Test:** unit `unleased_tool_call_logs_refused`: a lease-gated tool with no lease gives `outcome=refused`. **Ship:** `telemetry-gaps/TG-PIPE-06-labd-call-row` · `tg-pipe-06` · `feat(lab): TG-PIPE-06 one row per labd tool call`

### TG-PIPE-11 lab-mcp audit: error reason and duration

W2 · low · packet-coder · Ready · crates `cimmeria-lab-mcp` · docs: lab

- **Change:** `crates/lab-mcp/src/audit.rs` and `tools/mod.rs`: `lab.tool_call` gains `error` (capped) and `duration_ms`; WARN on `outcome=error`.
- **Test:** unit, LogCapture, `bad_sql_audit_row_has_error`: `server_db_query` with bad SQL gives WARN with `error`. **Ship:** `telemetry-gaps/TG-PIPE-11-lab-mcp-audit` · `tg-pipe-11` · `feat(lab-mcp): TG-PIPE-11 error and duration on lab tool-call rows`

### TG-PIPE-07 `lab_disconnect` flow

W2 · med · packet-coder · Ready · crates `cimmeria-lab` · docs: lab

- **Change:** `crates/lab/src/supervisor/flows/login.rs` (new `disconnect_flow`: at character select click Back, wait for the login screen, which sends Account `logOff` 0xC2), `crates/lab/src/server/flows.rs` (register), `crates/lab/src/lease/policy.rs` (guarded list), `crates/lab/src/uat/tools.rs` (driver entry). Four files, all one-line registrations except the flow.
- **Test:** unit with fake CEGUI, `disconnect_flow_clicks_back`; the policy test lists the tool as guarded. **Ship:** `telemetry-gaps/TG-PIPE-07-lab-disconnect` · `tg-pipe-07` · `feat(lab): TG-PIPE-07 lab_disconnect flow back to the login screen`

### TG-PIPE-08 S1: runner ends every run with logout and disconnect

Aliases: S1. W2 · med · packet-coder · BlockedDependency (TG-PIPE-07, TG-NET-01) · crates `cimmeria-lab` · docs: uat, lab

- **Change:** `crates/lab/src/uat/runner/mod.rs` (`run_all`, after the row loop), `uat/runner/session.rs` (new `end_session`), `crates/lab/src/server/uat.rs` (`end_session: bool`, default true, false for `plan_only`). Skip when the lease is revoked or the client isn't running; else `lab_logout`, server clause on `server_log_tail` for `session.end disconnect_reason=logoff_character_select`, `lab_disconnect`, clause for `Client entities cleaned up disconnect_reason=logoff`. Record as row `_session_end`. Runner-level, so it runs whatever the row filter is.
- **Test:** runner test with the fake invoker, `run_all_ends_the_session`: call order `lab_logout`, `lab_disconnect` after the last row; none under `plan_only`. **Ship:** `telemetry-gaps/TG-PIPE-08-s1-session-end` · `tg-pipe-08` · `feat(lab): TG-PIPE-08 UAT runs end with logout and disconnect`

---

## Not packeted

| Item | Source | Disposition |
|---|---|---|
| TG-MIS-04 `anchor_unused` friction heuristic | missions review | Deferred: speculative (a dwell-without-interact guess); revisit after TG-MIS-03 shows anchors |
| TG-NPC-14 threat on a non-preemptable NPC | NPC review | Deferred: never observed; low value |
| TG-SOC-16 org roster on a poisoned lock | social review | Deferred: a poisoned mutex is already a panic elsewhere; no evidence it happens |
| T2 ingest-side join against server introductions | README T2 | A SigNoz query over `client.viewport.unknown_entity` and `aoi.introduce`, not code |
| Outbox drainer picks rows `try_dispatch_now` is still sending | items review | File an issue for `database-persistence` |
| Free repair when the trailing vendor template id is omitted | items review | D-TG12; `server-authority-enforcer` verifies first |
| Non-vault inventory move refusals don't snap back | items review | File an issue (behaviour) |
| Respawn same-world write zeroes facing | movement review | File an issue for `movement-teleport-advisor` |
| Trade never consults the Ignore list | social review | D-TG10 (product) |
| Claude Code reports MCP tools as `mcp_tool` | pipeline review | Ops change: check the installed version's tool-detail setting |
| OTLP exporter drops leave no trace | pipeline review | D-TG16 |
| Why a no-arg `endCurrentMinigame` carries 4 bytes | minigames review | Question for `bigworld-engine-advisor`; TG-MG-02 doesn't wait on it |
