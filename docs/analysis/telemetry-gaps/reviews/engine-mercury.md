# Engine and Mercury telemetry review

Reviewer: bigworld-engine-advisor, 2026-10-10. Data: colo SigNoz, last 7 days, aggregates only.
Summary: the transport writes 86% of all log rows (126M of 147M), and those rows carry only a peer and a length. Meanwhile the rows that would explain a spurious resend, a stalled receive loop or a misframed bundle have no account, no message and no join key to the client.
The `old_duplicate` resends players hit can't be explained today. Neither side records enough to tell a lost ACK from a client hitch or a base receive loop blocked on the cell channel.

## 1. Inventory

All `mercury.*` targets and the module rows are named in `OTEL_FILTER` (`crates/server/src/logging/filters.rs:290-338`). Nothing at DEBUG+ is invisible. The blind spots are TRACE rows: they reach only `cimmeria-trace`, because `base.log` keeps `connect_loop` at TRACE (`filters.rs:475-481`), and 99% of that index is `encrypt` noise.

| Target / row | Level | Index | 7-day rows | Notes |
|---|---|---|---|---|
| `mercury.packet` (`transport.rs:71`, `:112`) | INFO | network | **63.2M** (62.9M out, 0.29M in) | peer and len only; no seq, msg or account |
| `cimmeria_mercury::encryption` `encrypt`/`decrypt` (`encryption/mod.rs:341,398,445,530`) | TRACE | trace | **63.2M** | plaintext/ciphertext lengths only |
| `connect_loop::encrypted` per-datagram rows (`encrypted/mod.rs:97,267,296`) | DEBUG | network | 620k | `AUTHENTICATE received -- ignored` 135k |
| `mercury.reliable_send` (`reliable_send.rs:207`) | DEBUG/WARN | network | 597k / 2 | fingerprint, send_site; **player_id is a string** (§2) |
| `wire.out.avatar_update` (sample 1/101) | DEBUG | server | 603k | |
| `mercury.retransmit` (`channel_core.rs:583`) | INFO | network | 5,627 | peer, seq, retransmit_count, rto_ms |
| `tick_sync` "Channel retransmit: RTO fired before ACK" (`tick_sync.rs:198`) | DEBUG | network | 5,627 | duplicate of the row above, with less in it |
| `mercury.tx_hole` `tx_hole_open` / `tx_hole_stall` | DEBUG / WARN | network | 522 / **0** | |
| `mercury.rx_order` `duplicate` / `buffered` / `rx_stall` | DEBUG / DEBUG / WARN | network | 470 / 3 / 0 | |
| `reliable_resend_abandoned` (`helpers/mod.rs:249`) | WARN | server | 0 | |
| `mercury.backpressure`, `mercury.fragment_caps` | WARN | server | 0 | |
| `connect_loop` `WSAECONNRESET (10054)` (`connect_loop/mod.rs:96`) | DEBUG | server | 18,672 | no addr |
| `cooked_data.version_reply` / `sync_start` / `sync_finish` / `miss_served` | INFO | server | 2,289 / 262 / 261 / 21 | well formed; T4 covers the gap |
| `cooked_data.sync_finish outcome=abandoned` | WARN | server | 13 | |
| `base.entity_method` | DEBUG/WARN | server | 344 | outbound drops well covered |
| TRACE-only: "Bundle truncated" (`encrypted/mod.rs:291`), "Unhandled client message" (`:598`), "Unhandled Account base method" (`account_arms.rs:201`), "Cell method before world entry" (`cell_arms.rs:81`), "Ignoring cell method until mapLoaded" (`cell_arms.rs:92`), "Ignoring packet from unknown addr" | TRACE | trace | 2 / 4 / 1 / 0 / 61 | expectation failures logged at TRACE |

Client side (`cimmeria-client`): `client.mercury.packet_in` 219k DEBUG, `socket_recv` 113k INFO, `entity_method` 96k, `old_duplicate` 1,092 WARN (each also written a second time as `client.mercury.error`), `result_0xfffffff5` 1,046 WARN, `rx_gap_open/closed` 385 each, `unpack_fault` 15, `request_misparse` 17.

## 2. Positive gaps

- **`mercury.reliable_send` logs `player_id = ?state.active_player_id` and `entity_id = ?state.player_entity_id`** (`base-session/src/base/helpers/reliable_send.rs:198,200,221,223`). In SigNoz the values are strings: `"Some(71)"` and `"None"` in `attributes_string`, never in `attributes_number`. A `player_id = 71` filter misses every reliable send. This is the exact anti-pattern in the negative-logging field rules.
- **Transport crate rows have no identity at all.** `mercury.retransmit`, `tx_hole_*`, `rx_order *` and `backpressure` carry only `peer` (an address), which breaks Rule 5. The channel has no way to know who it belongs to.
- **`mercury.retransmit` lacks the cause fields.** The TxEntry already holds `send_kind`, `first_message` and `fragment_index` (`channel/state.rs:44`), but the row (`channel_core.rs:583`) logs none of them. It also lacks `wire_fingerprint`, the age since the first send, `srtt_ms`, and the time since the peer last sent anything. `retransmit_count` reached 11-20 on 308 rows: packets resent to a peer that had gone silent. Nothing says the peer was silent.
- **`session.end` (`session_teardown.rs:254`) has no channel health.** It doesn't record outstanding unacked packets, total retransmits, `tx_holes`, `rx_stalls`, srtt, or the age of the last receive. "Client crashed", "network died" and "server stopped acking" all look the same. A session that ends at character select (no player entity) writes no `session.end` at all (T5).
- **No row says the base→cell queue is backing up.** Both directions are bounded at 256 (`services/src/orchestrator.rs:151-152`), and nothing measures depth or wait.
- `onClientVersion` (`account_arms.rs:198`) is acknowledged without logging the version the client sent. That is useful context for T4.
- The MAX_RETRIES (20) cap is never enforced in production: `is_timed_out` has no production caller. The 60 s inactivity reap ends such sessions instead. This is not a telemetry bug, but nobody can tell from SigNoz which limit ended a channel.

## 3. Negative gaps

| Site | Shape | Consequence |
|---|---|---|
| `base/connect_loop/cell_arms.rs:159`, `:172` | `let _ = tx.send(CellMethodCall).await` | every player cell method (abilities, interact, missions) is lost silently if the cell channel is closed |
| `base/connect_loop/encrypted/mod.rs:395` | `let _ = tx.send(EntityMove).await` | movement lost silently; and because it is `.await`ed on the single receive loop, a full queue stalls **all** clients' packet intake, ACKs included |
| `encrypted/mod.rs:339` | `if let Some(entity_id)` with no else; `payload.len() < 40` with no else | movement before world entry, or a short movement packet, is dropped with no row |
| `cell_arms.rs:91-99` | TRACE, `ControlFlow::Break` | a click during map load drops that message **and the rest of its bundle** at TRACE. The player gets no feedback (owner rule) |
| `cell_arms.rs:80-88` | TRACE | cell method before world entry |
| `cell_arms.rs:122` | `if !method_payload.is_empty()` with no else | a 0xBD sub-slot call with no index byte vanishes |
| `encrypted/mod.rs:291` | TRACE, `break` | a truncated bundle drops every remaining message |
| `encrypted/mod.rs:598`, `account_arms.rs:201` | TRACE | unhandled system message or Account method; the matching SGWPlayer and cell catch-alls are WARN (`dispatch/mod.rs:294`, `cell/dispatch/router.rs:169`) |
| `encrypted/mod.rs:119-121`, `:203`, `:212` | `if let Ok(...)` / `.ok()?` on locks | ACK processing or the receive gate silently skipped on a poisoned lock |
| `connect_loop/mod.rs:88` | `warn!("Datagram handler error: {e}")` | any `?` in a handler (`cooked_data.rs:109-110`, etc.) aborts the rest of the bundle. The row has no account, no reason and no message context |
| `encrypted/mod.rs:87` | parse-failed WARN with no account and no `reason` | |
| `cooked_data.rs:51` | `versionInfoRequest: payload too short` with no `payload_len`, `reason` or account | |
| `cell/dispatch/router.rs:169` | unhandled cell method WARN | has `entity_id` but no `account_id`/`player_id` |
| `cooked_sync/task.rs:292` | ERROR `entry_exceeds_256_fragments` | no account |
| `wire/src/mercury/mod.rs:395-398` | `.expect()` on sub-slot or length overflow | a panic. Panics go to stderr and Discord only (`discord/src/lib.rs:175`), **never to SigNoz** |

## 4. Noise

1. **`mercury.packet` INFO, one row per datagram**: 63.2M rows, 49% of all SigNoz rows. On 2026-10-06 one idle client in an NPC-dense zone received 3,167,556 datagrams an hour (880/s, 48 bytes each) for 10 hours straight, while it sent 3,100 an hour. That is one unbundled avatar update per (witness, NPC) per tick (`base-world-entry/.../cell_dispatch/aoi.rs:280`), which is the AoI reviewer's issue. The transport row recorded all 31M of them and said nothing about any of them.
2. **`encrypt`/`decrypt` TRACE**: 63.2M rows, 43% of all rows, no diagnostic value. `DECRYPT_OK` already has its own sampled firehose.
3. `AUTHENTICATE received -- ignored` DEBUG, 135k. The client prepends `authenticate` to every bundle (standard BigWorld behaviour), so this row fires per packet and carries nothing.
4. `WSAECONNRESET` DEBUG, 18.7k rows with no addr. These are ICMP port-unreachable replies to sends at dead clients.
5. The retransmit is logged twice (`channel_core.rs:583` INFO and `tick_sync.rs:198` DEBUG).
6. Client side: every non-happy packet writes two WARNs (`packet_in` plus `client.mercury.error`; `report.rs:95-107`). The 125 `old_duplicate` rows for seq 1-2 are benign: the 700 ms initial RTO resends the login reply and time-sync once (server retransmit rc=1 for seq 1 and 2, 66 and 67 rows).
7. `cooked_data.sync_finish reason=session_gone_before_start` is a WARN (12 rows). It is a normal logout race and should be DEBUG under its reason.

## 5. Seams

| Hand-off | Sender logs | Receiver logs | Can we tell who dropped it? |
|---|---|---|---|
| Server reliable send → client DLL (`client.mercury.*`) | `reliable_send` seq, fingerprint, send_site | `socket_recv` fingerprint; `packet_in` seq, disposition (no fingerprint) | Only through `socket_recv`. `packet_in`, `unpack_fault`, `request_misparse` and `old_duplicate` have neither fingerprint nor account, so a misframed bundle (15 `unpack_fault`) can't be traced to its send_site |
| Server retransmit → client `old_duplicate` | `mercury.retransmit` peer, seq | seq, `behind`, session_id | **No.** Client rows have no account or peer; server rows have no account or fingerprint. 967 mid-stream `old_duplicate`s in 19 sessions are unattributable |
| Client ACK → server TX window | TRACE `Channel TX window updated` (srtt, rto) | n/a | ACK latency is visible only in the trace index |
| Base receive loop → cell (`BaseToCellMsg`, cap 256) | none on failure (`let _`) | cell `cell.dispatch` span (DEBUG); unhandled WARN | No. A full queue shows only as downstream symptoms: retransmits and client gaps |
| Cell → base (`CellToBaseMsg`) → client | cell `abilities.wire wire_sent`; `aoi.*` | `base.entity_method` `client_send_dropped`, `batch_sent` | Yes for entity methods; no for EntityMoved (unreliable, nothing per drop). The AoI packer's 880/s per witness is visible only as raw `mercury.packet` volume |
| Login reply / time-sync → client | `reliable_resend_abandoned`, `login_retry_on_channel` | `old_duplicate` seq 1-2 | Yes. Benign spurious resends dominate |
| Cooked `versionInfo` → client cache write | `version_reply`, `sync_*` | none (T4: the DLL cache-write result) | Server side yes; client side T4 |
| Session teardown → channel | `session.end` with no channel state | `WSAECONNRESET` with no addr | No (T5) |
| Two hops: cell systems ← `CellMethodCall` | base silent on send failure | each system's own rows | A dropped call looks like "the player never pressed" |

## 6. Adversarial

1. **Spurious resends (`old_duplicate`, seen live).** *Today:* the client shows `old_duplicate seq=N behind=17`. The server has a `mercury.retransmit` row in `cimmeria-network` with a peer and a seq, but no way to tie it to the client session. *Should:* the retransmit row carries `account_id`/`player_id`, `wire_fingerprint`, `age_ms`, `srtt_ms`, `peer_last_rx_ms` and `msg_name`. A silent peer means a client hitch (join with T7), a chatty peer means its ACK was lost, and a base stall row means the server was slow.
2. **Cell slow, base blocks.** A busy cell tick fills `BaseToCellMsg`. `tx.send().await` blocks the only receive loop, so ACKs go unprocessed and every client gets retransmit bursts (the seq 194-229 rc=1 burst is this shape). *Today:* nothing names the cause. *Should:* WARN `cell_channel_backpressure` with `waited_ms`, `queue_free` and `msg_kind`, throttled.
3. **Click during map load.** *Today:* TRACE `Ignoring cell method until mapLoaded`, plus the silent loss of the rest of the bundle. *Should:* DEBUG `reason=map_loaded_pending` with `method_name`, `dropped_after` and the identity.
4. **Client crash mid-session.** *Today:* 60 s of sends to a dead port, thousands of addr-less `WSAECONNRESET` rows, then `session.end disconnect_reason=inactivity_timeout`. *Should:* `session.end` carries `tx_outstanding`, `oldest_unacked_age_ms`, `last_rx_age_ms`, `tx_retransmits_total` and `srtt_ms`; the resets are throttled with a count.
5. **Misframed server bundle** (15 `unpack_fault`). *Today:* the client hex shows `packet_len 950`, but nothing joins it to a server send. *Should:* the client row carries `seq` and `wire_fingerprint`, which join to `mercury.reliable_send send_site`.

## Candidate packets

| ID | Title | Sev | Files | Depends |
|---|---|---|---|---|
| TG-MER-01 | Numeric `player_id`/`entity_id` on `mercury.reliable_send` | high | 1 | – |
| TG-MER-02 | Cause fields on `mercury.retransmit`; drop the duplicate tick row | high | 3 | – |
| TG-MER-03 | Channel log identity on retransmit and backpressure rows | high | 3 | 02 |
| TG-MER-04 | Identity on `tx_hole` and `rx_order` rows | med | 2 | 03 |
| TG-MER-05 | Base→cell send seam: closed and backpressure WARNs | high | 3 | – |
| TG-MER-06 | Inbound bundle-walk drops out of TRACE | med | 3 | 05 |
| TG-MER-07 | Channel health on `session.end` | med | 3 | 03 |
| TG-MER-08 | Delete per-packet `encrypt`/`decrypt` TRACE rows | high | 1 | – |
| TG-MER-09 | `mercury.packet` to the firehose pattern plus a counter | high | 2 (+2 tests) | owner decision |
| TG-MER-10 | Throttle `WSAECONNRESET` with a suppressed count | low | 1 | – |
| TG-MER-11 | Identity and reason on `Datagram handler error` and the cell catch-all | low | 2 | 06 |
| TG-MER-12 | Client DLL: seq and fingerprint on `unpack_fault`/`request_misparse`/non-happy `packet_in` | med | 2 | D-TG4, needs-domain-agent |

**TG-MER-01: numeric identity on `mercury.reliable_send`** (high)
Files: `crates/base-session/src/base/helpers/reliable_send.rs`, function `shadow_register_reliable_send_with_details`, lines 198, 200, 221 and 223. Change `player_id = ?state.active_player_id` to `player_id = state.active_player_id`, and `entity_id = ?...` to `entity_id = state.player_entity_id`. Test: a LogCapture unit test next to `reliable_fit_tests.rs` registers a send for a session with `active_player_id = Some(71)` and asserts `has_field("player_id", "71")`. Reverting to `?` records `"Some(71)"` and fails. No filter change.

**TG-MER-02: retransmit cause fields** (high)
Files: `crates/mercury/src/channel/state.rs` (add `first_sent: Instant` to `TxEntry`, set where `last_sent` is first set), `crates/mercury/src/channel/channel_core.rs` `check_timeouts` (the row at :583), and `crates/base-session/src/base/tick_sync.rs:198` (delete the duplicate DEBUG row and keep the send-error row). Add to `mercury.retransmit` INFO: `event = "retransmit"`, `age_ms` (now − first_sent), `srtt_ms` (Option), `peer_last_rx_ms` (now − last_received), `send_kind`, `msg_id`, `msg_name` (`cimmeria_wire::names::client_msg_name` is not reachable from mercury, so log `msg_id` and let MER-03's caller-side naming follow later), `fragment_index`, and `wire_fingerprint` (computed only on this row). Test: in base-session, drive `collect_pending_retransmits` with a channel whose entry has expired (copy the clock setup from `reliable_fit_tests.rs`). Assert the INFO row has `event=retransmit` and an `age_ms` field, and that no "RTO fired before ACK" row remains.

**TG-MER-03: channel log identity** (high)
Files: `crates/mercury/src/channel/state.rs` (a `LogIdentity { account_id: Option<u32>, account_name: Option<&'static str>, player_id: Option<i32>, player_name: Option<&'static str> }` field, `Default`), `channel_core.rs` (a `set_log_identity` setter; add the four fields to the retransmit and backpressure rows, as `Option`s), and `crates/base-session/src/base/helpers/mod.rs` `collect_pending_retransmits` (sync the identity from `session_identity::session_identity(state)` each tick, before `check_timeouts`). Test: after `InitPlayer`-style state with account 6 and player 72, an expired entry produces a `mercury.retransmit` row with `account_id=6` and `player_id=72`. Without the sync the fields are absent and the test fails.

**TG-MER-04: identity on `tx_hole` and `rx_order`** (med)
Files: `crates/mercury/src/channel/ack.rs` (`refresh_tx_hole` :193/:205, `check_tx_hole_named` :310) and `crates/mercury/src/channel/rx_order.rs` (:194-219, :383). Add the MER-03 identity fields. Test: base-session LogCapture. A footer that acks seq 3 before seq 2 yields `tx_hole_open` with `account_id`.

**TG-MER-05: base→cell send seam** (high)
Files: a new `crates/base/src/base/connect_loop/cell_send.rs` with `send_to_cell(tx, msg, kind: &'static str, addr, who: PlayerIdentity, method_index: Option<u16>)`; `cell_arms.rs:159,172`; and `encrypted/mod.rs:395`. On `Err`: WARN `event=cell_send_failed reason=cell_channel_closed`, identity, `msg_kind`, `method_index` + `method_name`. Before sending, read `tx.capacity()` and time the await. If it waited ≥ 50 ms, write WARN `event=cell_channel_backpressure` with `waited_ms`, `queue_free` and `msg_kind`, using a Pattern D throttle with a global key, a 5 s window and `suppressed`. Increment the counter `base_cell_channel_stalls_total` on every stall. Test: LogCapture. A dropped receiver gives `reason=cell_channel_closed`. A capacity-1 channel that a spawned task drains after 100 ms gives `cell_channel_backpressure` with `waited_ms ≥ 50`, and a second stall in the window is suppressed. Reverting to `let _` fails both.

**TG-MER-06: inbound bundle-walk drops** (med)
Files: `encrypted/mod.rs` (:291 TRACE→WARN `reason=bundle_truncated`, `offset`, `body_len`, `account_id`; :598 TRACE→WARN `reason=unhandled_msg_id`; :267 AUTHENTICATE DEBUG→TRACE; :339 add a DEBUG `reason=no_player_entity` when the entity is `None`), `account_arms.rs:201` (TRACE→WARN `reason=unhandled_account_method`), and `cell_arms.rs` (:81 TRACE→DEBUG `reason=before_world_entry`; :92 TRACE→DEBUG `reason=map_loaded_pending` with `method_name`, `dropped_rest_of_bundle=true`; :122 an else arm WARN `reason=subslot_missing_index`). Add the identity on each. Test: a LogCapture table test per reason, with level pinned.

**TG-MER-07: channel health on `session.end`** (med)
Files: `state.rs` and `channel_core.rs` (a `retransmits_total: u64` counter bumped in `check_timeouts`, and a `health()` snapshot: `tx_outstanding`, `oldest_unacked_age_ms`, `retransmits_total`, `tx_holes`, `tx_hole_stalls`, `rx_stalls`, `srtt_ms`, `last_rx_age_ms`), and `crates/base-session/src/base/helpers/session_teardown.rs` (snapshot before `clients.remove`, then add to the :254 row). Test: the existing teardown LogCapture tests. A channel with 2 unacked entries gives `tx_outstanding=2`.

**TG-MER-08: delete the `encrypt`/`decrypt` TRACE rows** (high, noise)
Files: `crates/mercury/src/encryption/mod.rs:341,398,445,530`. Keep the HMAC-fail WARNs, and add `reason` to them while there. Test: in base (which has LogCapture at TRACE), encrypt and decrypt one packet and assert no captured event has the message `encrypt` or `decrypt`. It fails if the rows return. Expected effect: roughly −63M rows a week in `cimmeria-trace`.

**TG-MER-09: `mercury.packet` to the firehose** (high, noise; BlockedDecision: it reverses the NA25 "INFO, more data" choice pinned in `parity_tests/guard.rs:141`)
Files: `crates/mercury/src/transport.rs` (move the per-datagram rows to the target `wire.firehose.mercury_packet` at TRACE; emit a 1-in-53 sample on `mercury.packet` with `sampled_1_in` and `suppressed` from a local `AtomicU64`; add the counter `mercury_datagrams_total{dir}`) and `crates/server/src/logging/filters.rs` (name the firehose in the protocol.log layer, off in OTLP). Tests: update `guard.rs:141` and `target_scan_tests.rs:364`. Add a burst test: 106 sends write 2 sampled rows whose `sum(1+suppressed)` equals 106. Expected effect: roughly −62M rows a week.

**TG-MER-10: throttle `WSAECONNRESET`** (low, noise)
Files: `crates/base/src/base/connect_loop/mod.rs:96`. Extract `log_connreset()`: one DEBUG row per 10 s with `suppressed` and `reason=icmp_port_unreachable`, plus the counter `udp_connreset_total`. Test: a burst of 50 writes 1 row, and the next row after the window carries `suppressed=49`.

**TG-MER-11: identity on catch-alls** (low)
Files: `connect_loop/mod.rs:88` (add `identity_for_addr` fields, `reason=handler_error`, `error`) and `crates/cell/src/cell/dispatch/router.rs:169` (add `SpaceManager::player_identity` fields). Test: extend `unhandled_cell_method_warns_with_method_index_and_args_len` to assert `account_id` once it is stamped.

**TG-MER-12: client join keys** (med; client DLL, ships per D-TG4; needs-domain-agent)
Files: `crates/client-telemetry/src/hooks/inline_hooks/mercury_recv.rs` (the `unpack_fault`/`request_misparse` emit at :528-565) and `hooks/mercury_recv/report.rs` (`packet()`). Add the packet `seq` to the unpack rows, plus the encrypted-datagram `wire_fingerprint` stashed per thread by the `recvfrom` detour (`mercury_recv/wire.rs:55`). Getting the fingerprint from recv to filter needs RE judgment on thread and order. Also drop the second WARN (`client.mercury.error`) for `old_duplicate`, and demote `old_duplicate` for seq ≤ 2 to DEBUG. Test: `report.rs` unit tests assert `wire_fingerprint` is present on a non-happy packet; a lab load check is required.
