# Auth and network telemetry review

Reviewer: network-security-auth, 2026-10-10. Scope: SOAP login (Phase 1+2), ticket hand-off, Phase 3 `baseAppLogin`, session lifecycle and end, connection hardening, public listeners (login, Mercury, admin, telemetry ingest, minigame), GM gate. Data: colo SigNoz, last 7 days unless stated.

The happy path is well covered. The session's *end* is not. A logOff to character select emits no `session.end`. A clean in-world exit, a crash and a lab kill all end as `inactivity_timeout`. A crash-and-relaunch looks like a concurrent login. A server restart ends sessions with no row. Admin API requests never reach SigNoz at all, because the `tower=off` directive in `OTEL_FILTER` also matches `tower_http`.

## 1. Inventory

| Target / scope | Level | In SigNoz? | 7-day count |
|---|---|---|---|
| `cimmeria_auth::auth::handlers` (Phase 1/2) | INFO/DEBUG | yes (`cimmeria_auth=debug`) | 115 Phase 1 ok, 113 tickets, **0 failures of any kind** |
| `cimmeria_auth::auth::service` reaper | DEBUG | yes | 3 rows, counts only |
| `cimmeria_base::base::login` / `login::eviction` | INFO/WARN | yes | 112 Phase 3; 47 duplicate-login WARN |
| `cimmeria_base::base::connect_loop` "baseAppLogin received" | INFO | yes | 112 |
| `session.start` (cell `player_init/mod.rs:440`) | INFO | yes | 141 |
| `session.end` (`session_teardown.rs:254`) | INFO | yes | 83: `duplicate_login` 42, `inactivity_timeout` 41, **`logoff` 0, `client_disconnect` 0** |
| "Client entities cleaned up" (`session_teardown.rs:358`) | INFO | yes | 68 (`logoff` 1, account-only ends) |
| `cimmeria_base::base::dispatch::session` logOff | INFO | yes | 27, all `returning to character select`, **0 full exit** |
| `cimmeria_base::base::connect_loop` WSAECONNRESET | DEBUG | yes | **18,672** |
| `connect_loop` "Ignoring packet from unknown addr" | TRACE | trace index only | 62 |
| `tower_http::*` (admin and login-port request rows) | INFO/WARN | **no**: `tower=off` prefix-matches `tower_http` (`filters.rs:380`) | **0 in 30 days** |
| `cimmeria_cell_world::cell::dispatch::gm_gate` | INFO/WARN | yes | 282 authorized, 0 rejected, **0 carry account_id/player_id** |
| `cimmeria_cell::...::lab_console` reject | WARN | yes | 4, no identity |
| `cimmeria_admin_api::routes::dev_session` | INFO/WARN | yes | 36 mints, 1 refresh |
| `mercury.packet` | INFO | yes (`cimmeria-network`) | ~3.85 M/day (engine review's to own; see section 4) |

The custom target `auth=info` (`filters.rs:354`) matches no emitter; every auth row is module-path.

## 2. Positive gaps

- **logOff to character select is not a session end.** `dispatch/session.rs:205-259` clears `player_entity_id`, so the later teardown never emits `session.end`. That accounts for 27 of the 58 `session.start` rows (141) with no matching end (83).
- **`session.start` is per world entry, not per session.** Gate travel re-sends `InitPlayerState` (`base-world-entry/.../gate_travel/mod.rs`), so `session.start` fires on every travel and there is no travel-side end. Starts and ends cannot be paired.
- **Server shutdown ends no session.** `BaseService::stop` (`base/service.rs:382-386`) logs only "Stopping base service". The 8 restarts this week ended their live sessions silently.
- **Rule 5 misses** (verified on colo rows):
  - `SGWPlayer.logOff` (`dispatch/session.rs:34`): no identity, 27/27 rows.
  - "Client requests createCharacter/playCharacter/deleteCharacter" (`account_arms.rs:122,134,173`): `account_id` is in scope and not logged, 0/116 rows.
  - Account-level "Client requests logOff" (`login/mod.rs:581`): no identity.
  - "Phase 3 complete" (`login/mod.rs:285`): addr but no account. "Phase 3 authenticated" (`:179`): account but no addr. The peer is only on the span, and span fields do not reach Logs.
  - The GM gate (`gm_gate.rs:170,184`) and `lab_console` reject carry `entity_id` + `access_level` only.
  - "Tick-sync stopping: session cancelled" (`tick_sync.rs:106,193`): no identity.
- **Field naming:** Phase 1 refusals use `user=` (`handlers.rs:134,162,173,183,191,207`), not `account_name`, and carry no `reason`.

## 3. Negative gaps (silent refusals)

| Site | What the client sees | What SigNoz shows |
|---|---|---|
| `auth/handlers.rs:106` SKU != `SGW_BETA` | error 3 | nothing |
| `handlers.rs:140` `PlaintextLength` | error 2 | nothing |
| `handlers.rs:215` no shards | error 7 | nothing (audit table only) |
| `handlers.rs:284` no SID cookie, `:295` unknown SID | "logon session has expired" | nothing |
| `handlers.rs:292` SID expired | same | INFO "Session expired for SID", no account, no peer |
| `handlers.rs:332` unknown shard | error 8 | nothing |
| `auth/service.rs:208-225` reaper | ticket issued, UDP login never arrived | DEBUG count, no account, IP or age |
| `login/mod.rs:64,102` unknown or consumed ticket | login hangs, client retries | WARN with no `addr` and no `reason` |
| `login/mod.rs:138,194,199` key decode or send failure (`?`) | login hangs | generic "Datagram handler error" (`connect_loop/mod.rs:88`), no account |
| `login/mod.rs:589` `ok_or("no session for addr")` | none | the same generic WARN |
| `login/eviction.rs:172` `let _ = transport.send_to(LOGGED_OFF)` | old client may never learn | nothing |
| `connect_loop/mod.rs:213` datagram from an unknown addr (a reaped client still talking) | client hangs, then times out | TRACE per datagram, trace index only |
| `encrypted/mod.rs:395` `let _ = tx.send(EntityMove)` | movement lost | nothing (movement review's area) |
| `gm_gate.rs:163-166` `unwrap_or(0)` when the entity is missing | rejected | the same WARN as a real non-GM, no `reason` |
| `admin-api/.../dev_session/handlers.rs:400` `_ => {}` | kill switch, expired, lifetime-exceeded and bad-token refresh refused | nothing, so the client telemetry stream silently stops (feeds T8) |

## 4. Noise

- WSAECONNRESET DEBUG, 18.7k/week (`connect_loop/mod.rs:96`). Windows does not report the peer, so the row cannot be attributed. It mostly means the server is still sending tick-syncs to a dead client during the 60 s reap window.
- The duplicate-login WARN (39/week) is almost always benign: the old client is dead and the player relaunched before the 60 s reap. With no idle field there is no way to tell the benign case from a real concurrent login.
- `mercury.packet` INFO at ~3.85 M rows/day (`mercury/src/instrumentation.rs:100,120`, `transport.rs:72,113`). It is in the network index, but at INFO per packet. I'm not proposing a packet here, to avoid duplicating the engine review; the coordinator should confirm the engine review owns it.
- "Decrypted packet received" DEBUG per datagram (`encrypted/mod.rs:97`), 620k/week in `cimmeria-network`. This is the network index working as intended.

## 5. Seams (two hops)

| Hand-off | Sender logs | Receiver logs | Can we tell who dropped it? |
|---|---|---|---|
| Phase 1 → Phase 2 (SID) | "Phase 1 success" with account | nothing on missing or unknown SID | **No**. Fixed by TG-NET-04/05 |
| Phase 2 → Phase 3 (ticket in `pending_logins`) | "ticket issued" with `ticket_prefix` | "baseAppLogin received" with `ticket_prefix`, or WARN with no addr | Partly: prefixes join, but an expired-unconsumed ticket is a DEBUG count. TG-NET-05/06 |
| Phase 3 → world entry (`playCharacter` → `play_character` span → cell `InitPlayerState` → `session.start`) | account_arms row has no account_id | `session.start` complete | Join on `addr`, then `player_id`. TG-NET-07 |
| Character create/list (`account_arms` → `character_create`) | INFO with addr only | `character_create` INFO | Join on addr; add the account. TG-NET-07 |
| Base teardown → cell `DisconnectEntity` (oneshot reply) | WARN when the reply is dropped or the send fails (`session_teardown.rs:317,331`) | cell DEBUG "DisconnectEntity" (110) | Yes |
| logOff → cell `DisconnectEntity` + `DestroyEntity` | WARN on send failure only (`dispatch/session.rs:134,145`); reply discarded | cell DEBUG | Mostly |
| Teardown → presence fan-out (`spawn_offline` → contact/org) | `disconnect_reason` passed through | social review's area | Yes |
| Session ↔ client telemetry (dev-session mint/refresh → DLL upload) | mint INFO has `session_id`, no account | ingest rows carry the token `sid`; no game-session join key | **No**. T8, plus TG-NET-13 for refusals that kill the stream |
| Session ↔ client end (T5) | server: `inactivity_timeout` | client: no session-end row | **No**. TG-NET-02 adds the server half |
| labd → lab-mcp → base → cell `LabConsoleExec` | `lab.tool_call` INFO | cell WARN on reject, no identity | Partly. TG-NET-09 |
| Cell → minigame listener (ticket) | `cell_dispatch::minigame` INFO | minigame server rows | Minigame review (T10, D-TG1) |
| Admin API → orchestrator (`/api/*`, `/ws/logs`, `/ws/events`) | **nothing** | nothing | **No**. TG-NET-08 |
| DB (credential check) | ERROR `DbError` (`handlers.rs:183`) | — | Yes. The `player_id: 0` sentinel trap is a world-entry concern |

## 6. Adversarial

1. **A player clicks Exit in the world, or their client crashes.** Seen 41 times this week. SigNoz shows, after 60 s, "Tick-sync stopping: client inactive for 60s" and then `session.end disconnect_reason=inactivity_timeout`. The client sent nothing distinguishing: 0 `DISCONNECT`, 0 `logOff(1)`. Crash, quit, lab kill and network loss all look identical. It should show `session.end` with `logoff_requested` (set by a prior `logOff(1)`), `idle_ms`, `in_world`, `world` and the unacked TX count, so a server-wedged client (unacked > 0) is told apart from a vanished one. The T5 client row closes the rest.
2. **A player's client dies and they relaunch inside 60 s.** Seen 39 times this week. SigNoz shows a WARN "Duplicate login -- evicting old session", which reads like account sharing, then `session.end duplicate_login`. It should show `old_idle_ms` and `old_in_world`, at INFO when the old session was idle ≥ 15 s (the client's own timeout), and WARN only when the old session was live.
3. **A player sits on server select for 5 minutes, then picks a shard.** The client says "logon session has expired". SigNoz shows one INFO with no account. A missing or unknown cookie shows nothing. It should show `Phase 2 refused reason=sid_expired|sid_unknown|sid_missing` with the peer, plus the account and SID age when known.
4. **The shard's UDP port is blocked, or the shard host is misconfigured.** Phase 2 succeeds and no `baseAppLogin` ever arrives. SigNoz shows "Phase 2 success" and nothing else except a DEBUG reaper count. It should show `auth ticket expired unconsumed` with the account, issued IP and age, one per ticket.
5. **The admin port is hit by an outsider** (known unauthenticated, #439). Someone opens `/ws/logs` or POSTs `/api/config`. SigNoz shows nothing (0 `tower_http` rows in 30 days). It should show `admin.request` with method, matched path, status, latency and peer IP.
6. **A deploy restarts the server with three players in the world.** SigNoz shows three `session.start` rows and no ends. It should show three `session.end disconnect_reason=server_shutdown` rows.

Note for S1: the label the code emits is `logoff` (lowercase), and only for the account-level Mercury logOff at character select. In-world logOff paths emit no `session.end` today. The spec assertion `disconnect_reason=logOff` cannot pass until TG-NET-01 lands, and it should then assert `logoff_character_select`.

## Candidate packets

| ID | Title | Sev | Files |
|---|---|---|---|
| TG-NET-01 | `session.end` on logOff to character select, with identity on the logOff row | high | 1 |
| TG-NET-02 | End-cause hints on `session.end` (logoff_requested, idle, in_world, unacked) | high | 2 |
| TG-NET-03 | Duplicate-login eviction: old session idle and in-world, level by liveness | high | 1 |
| TG-NET-08 | `admin.request` row for every admin API request | high | 2 |
| TG-NET-04 | Phase 1/2 refusals each log a reason | med | 1 |
| TG-NET-05 | Reaper logs each unconsumed ticket and SID | med | 1 |
| TG-NET-06 | Phase 3 refusal rows carry the addr and a reason | med | 2 |
| TG-NET-09 | GM gate and lab console rows carry identity and reason | med | 2 |
| TG-NET-10 | `session.end server_shutdown` for every live session at stop | med | 1 |
| TG-NET-13 | Dev-session refusals no longer dropped | med | 1 |
| TG-NET-14 | Inactivity-timeout row says what the session was doing | med | 1 |
| TG-NET-07 | Account-arm rows carry account_id/name | low | 1 |
| TG-NET-11 | WSAECONNRESET noise to a 60 s summary | low | 1 |
| TG-NET-12 | Rate-limited INFO for datagrams from unknown addresses | low | 1 |
| TG-NET-15 | `session.start` says first entry vs travel | low | 1, needs-domain-agent |

**TG-NET-01: `session.end` on logOff to character select (high).** File `crates/base/src/base/dispatch/session.rs`, `handle_log_off`.
- In the `disconnect == 0` branch, before the reset block, emit `tracing::info!(target: "session.end", %addr, entity_id, entity_name, account_id, account_name, player_id, player_name, disconnect_reason = "logoff_character_select", session_secs, "player session ended")`, the same shape as `session_teardown.rs:254`. Only emit it when `entity_id.is_some()`.
- Add `account_id`, `account_name`, `player_id`, `player_name` to the `SGWPlayer.logOff` row at `:34`. Move it after the identity snapshot.
- Test: a LogCapture unit test next to `dispatch/tests/player_index_logoff.rs`. Drive logOff(0) for an in-world session and assert one `session.end` INFO with `disconnect_reason=logoff_character_select` and `player_id`. Reverting the change removes the row.
- OTEL: none (INFO blanket).

**TG-NET-02: end-cause hints (high).** Files `crates/base-session/src/base/helpers/session_teardown.rs` and `crates/base/src/base/dispatch/session.rs`.
- Define `pub struct LogoffRequested;` in `session_teardown.rs`. In `handle_log_off`'s full-exit branch, `c.extensions.insert(LogoffRequested)` under the existing lock.
- In `teardown_session`, snapshot `idle_ms = c.last_recv.lock().elapsed().as_millis() as u64`, `logoff_requested = c.extensions.get::<LogoffRequested>().is_some()`, `world = c.world_name.clone()`, and `unacked = c.channel.lock().map(|ch| ch.tx_window.len())`. Add them to the `session.end` row and the "Client entities cleaned up" row.
- Test: a LogCapture test in `session_end_names_tests.rs`. A session with the marker, torn down as `inactivity_timeout`, logs `logoff_requested=true` and an `idle_ms` field. Reverting the change drops the fields.
- Depends on TG-NET-01 only for a clean merge (same file).

**TG-NET-03: duplicate-login liveness (high).** File `crates/base/src/base/login/eviction.rs`.
- Add `idle_ms: u64` and `in_world: bool` to `Displaced`, from `c.last_recv` and `c.player_entity_id.is_some()`.
- Log the duplicate and relaunch rows with `old_idle_ms`, `old_in_world`, `old_session_secs` and `reason = "duplicate_login"`. Use INFO when `old_idle_ms >= 15_000`, WARN otherwise, via two macro calls.
- Log a `send_to` failure at `:172` at DEBUG with `reason="logged_off_send_failed"`.
- Test: a LogCapture test in `login/tests`. Pre-seed a session whose `last_recv` is 20 s old and assert INFO with `old_idle_ms >= 20000`. A fresh `last_recv` must give WARN. The revert loses the field and the level split.

**TG-NET-08: admin request rows (high).** Files `crates/admin-api/src/lib.rs` and `crates/admin-api/src/request_span.rs`.
- Replace `DefaultOnResponse`/`DefaultOnFailure` on the **admin router only** (`build_router`, `:143-148`) with an `on_response` closure. It emits `tracing::info!(target: "admin.request", method, path, status, latency_ms, peer)`, with WARN for status ≥ 500.
- Read `path` from the request's `MatchedPath` extension, recorded into the span at `make_span`. Fall back to `"unmatched"` so a 404 probe cannot write free text.
- Read `peer` from `ConnectInfo<SocketAddr>` (served with connect info, `server/src/main.rs:277`), IP only.
- Leave the login-port router alone: it carries 53k upload rows a week.
- Test: oneshot the router with a `ConnectInfo` extension and LogCapture. `GET /api/players` yields one `admin.request` row with `status=200`. The revert yields none, because `tower_http` is not captured under that target.
- OTEL: none needed (INFO blanket). `target_scan_tests` must pass.

**TG-NET-04: Phase 1/2 refusals (med).** File `crates/auth/src/auth/handlers.rs`.
- Add `tracing::info!` rows with a `reason` for each refusal, all with `peer_ip`:
  - `unknown_sku` at `:106`
  - `plaintext_length` at `:140`
  - `no_shards` (WARN) at `:215`
  - `sid_missing` at `:284`, `sid_unknown` at `:295`
  - `sid_expired` at `:292`, with `account_id`, `account_name` and `sid_age_secs`
  - `unknown_shard` at `:332`, with the account
- Rename `user=` to `account_name=` at the six sites listed in section 2, and add `reason` to each.
- Test: extend the module's tests. POST Phase 2 with no cookie and assert INFO `reason=sid_missing`. Do the same for an unknown SKU.

**TG-NET-05: unconsumed ticket and SID rows (med).** File `crates/auth/src/auth/service.rs`.
- Extract the reaper body into `fn reap_expired(sessions, pending, now) -> (usize, usize)`.
- Replace `retain` with drain-and-log. Each expired ticket gets `tracing::info!(reason = "ticket_unconsumed", account_id, account_name, issued_ip, age_ms, ticket_prefix = %CredentialPrefix(..))`. Each expired SID gets `reason = "sid_unconsumed"` with the same fields.
- Keep the DEBUG summary.
- Test: a unit test that calls `reap_expired` with a ticket backdated 31 s under LogCapture and asserts the INFO row with `account_id`.

**TG-NET-06: Phase 3 refusal context (med).** Files `crates/base/src/base/login/mod.rs` and `crates/base/src/base/connect_loop/mod.rs`.
- Add `%addr` and `reason = "ticket_unknown"` at `login/mod.rs:64,102`.
- Add `%addr` at `:179`. Add `account_id` and `account_name` at `:285`.
- Add `%addr` + `account_id` to the account-level "Client requests logOff" at `:581`.
- At `connect_loop/mod.rs:88`, add `account_id` and `account_name` from `session_identity::identity_for_addr`, plus `reason = "handler_error"`.
- Test: a LogCapture test that calls `handle_login` with an unknown ticket and asserts the WARN carries `addr` and `reason=ticket_unknown`.

**TG-NET-09: GM gate identity (med).** Files `crates/cell-world/src/cell/dispatch/gm_gate.rs` and `crates/cell/src/cell/service/base_messages/lab_console.rs`.
- Resolve `let id = space_mgr.player_identity(entity_id)`. Add `account_id`, `account_name`, `player_id`, `player_name` to the authorized INFO, the rejected WARN and the lab reject WARN.
- Add `reason = "entity_missing"` when `get_entity` is `None`, and `"not_gm"` otherwise.
- Test: a LogCapture test in the gm_gate tests. A non-GM entity with a known identity logs WARN with `player_id` and `reason=not_gm`. A missing entity logs `reason=entity_missing`.

**TG-NET-10: shutdown ends (med).** File `crates/base/src/base/service.rs`, `BaseService::stop`.
- Under the `connected_clients` lock, for each session with `player_entity_id` emit the `session.end` row with `disconnect_reason = "server_shutdown"` and `session_secs`. This is log-only; no teardown.
- Then emit one INFO `sessions_open_at_stop = n`.
- Test: a unit test that inserts one in-world session, calls `stop()` and asserts the row.

**TG-NET-13: dev-session refusals (med).** File `crates/admin-api/src/routes/dev_session/handlers.rs`, `log_refusal`.
- Replace `_ => {}` with a WARN for `KillSwitchActive` (`reason=dev_session_kill_switch`). Log `Expired` and `SessionLifetimeExceeded` at INFO with `elapsed` and `cap`. Log bad-token refusals at INFO (already rate-bounded by `refresh_bad`).
- Test: extend the `tests.rs` refresh cases with LogCapture and assert `reason`.
- Coordinate with the telemetry-pipeline review to avoid duplication.

**TG-NET-14: inactivity context (med).** File `crates/base-session/src/base/tick_sync.rs`.
- On the inactivity row at `:115`, add `in_world`, `world`, `session_secs` and `unacked` (the channel's `tx_window.len()`).
- Add identity to the two "session cancelled" rows.
- Test: the existing `inactivity_timeout_line_names_the_player_and_the_account` gains asserts for `unacked` and `in_world`.

**TG-NET-07: account-arm identity (low).** File `crates/base/src/base/connect_loop/account_arms.rs`.
- Add `account_id` and `account_name` (from `identity_for_addr`) to the rows at `:122,134,173`.
- Test: LogCapture on `createCharacter` dispatch asserts `account_id`.

**TG-NET-11: WSAECONNRESET summary (low).** File `crates/base/src/base/connect_loop/mod.rs`.
- Count resets in a local, and emit one DEBUG `udp_conn_reset_count` per 60 s window. Drop the per-event row to TRACE.
- Test: a pure function `ResetSummary::record(now) -> Option<u64>`, unit-tested.

**TG-NET-12: unknown-peer datagrams (low).** File `crates/base/src/base/connect_loop/mod.rs`.
- Replace the TRACE at `:213` with an INFO `event = "unknown_peer_datagram"`, emitted at most once per addr per 60 s from a bounded map (≤ 256 entries). Fields: `addr`, `flags`, and `suppressed`.
- Test: a unit test on the limiter.
- Pairs with TG-NET-02: a reaped client still sending shows here.

**TG-NET-15: `session.start` entry kind (low, needs-domain-agent).** File `crates/cell/src/cell/service/base_messages/player_init/mod.rs`.
- Add `entry = "login" | "travel"`. It needs a world-entry agent to confirm which `InitPlayerState` field (or caller) distinguishes gate travel.
