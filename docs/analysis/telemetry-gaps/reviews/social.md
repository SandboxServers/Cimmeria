# Social telemetry review

> Scope: chat channels, contacts, organizations, mail, trading, duels, Black Market, GM commands on social state, Discord. Data: colo SigNoz, 7 and 30 days to 2026-10-10.

- Org, mail, duel, tell and Black Market are well instrumented (one outcome row per action, `event=`, identity and names). The gaps are contacts, the cell half of trade, spatial chat, Discord, and every cascade out of a character delete.
- Three findings are high: a Discord webhook URL reaches the logs on every network failure, a character delete destroys listings, held bids and mail with no row, and presence notifications go to players who ignored the subject (invariant 5).
- 30 days of colo data contain zero `trade.*`, `duel.*`, `chat.tell_*`, `chat.gm_*` or contact add/remove events, so those rows are proven only by `LogCapture` tests.

## 1. Inventory

Everything social is already reachable. `OTEL_FILTER` (`crates/server/src/logging/filters.rs:290`) pins the module-path rows `cimmeria_base_session`, `_base_methods`, `_base_world_entry`, `_cell_methods`, `_cell_duel`, `_cell_chatter`, `_cell_org`, `_cell_interactions`, `_cell_world`, `_cell_console`, `cimmeria_base::base` at `debug`, plus the hand-named targets `org`, `squad`, `chat`, `rate_limit`, `online_index`, `mail`, `duel`, `chatter`, `trade.atomic_swap`, `bank` and `cimmeria_discord` (explicit target, excluded from the Discord layer). Nothing social is DEBUG-only and invisible. The one TRACE negative is `Chat: no witnesses` (`cell-console/src/cell/console/chat/spatial.rs:61`), which the module-path target keeps out of the derived TRACE filter; the DEBUG `witness_count` row beside it covers the same fact.

Fired in 7 days (`cimmeria-server`): `mail expiry sweep finished` 2,620; `chatJoin: joined a user channel` 261 (INFO) with its DEBUG twin; `ContactList: pushed 2 lists` 141; `organization state restored at world entry` 141; Ignore-cache pair 141 each; `online_index` 196; `ambient chatter` 109+33; `organization create/registrar` a handful; `sendPlayerCommunication` 97; BM sweep/seed/search/transition about 40; `squad.disconnect_no_member` 20; Discord 65,409 DEBUG (see section 4). Never fired in 30 days: all of `trade.*`, `duel.*`, `chat.tell_*`, `chat.gm_*`, `mail.expired/returned`, `contact add/remove`, `bm.refused`.

## 2. Positive gaps

- **Contacts have no `event=` anywhere**: `contact_list/handlers/header_ops.rs` (19 rows), `member_ops.rs` (12 of 15), `handlers/mod.rs`, `presence_fanout.rs`, and the cell dispatch `cell-methods/.../contact_list/mod.rs` (18). Rows carry `player_id` but not `account_id` (Rule 5).
- **Cell trade has no `event=`, no `account_id`/`player_id`, and no cause**: `cell-interactions/src/cell/trade/state.rs` (13 rows) and `cell-methods/.../trade/handlers.rs`, `handoff.rs` (28). `trade session cancelled` (`state.rs:257`) carries only `result`, so user cancel, range break, disconnect, bad version and handoff failure are one row. The cell's `trade execute requested -> base` (`handoff.rs:217`) and the base's `trade.completed/refused` share no id; they join only on the player pair and time.
- Cell spatial chat (`spatial.rs:112`) logs `Broadcasting chat to witnesses` without `account_id`/`player_id` and without how many witnesses were withheld by Ignore (that count is only in per-witness `chat.spatial_ignored` rows).
- `fanout_contact_event` ends in `presence fanout complete` (`presence_fanout.rs:153`) with the subject's name only (no `player_id`) and `watcher_count`, not how many were online or sent to. A zero-watcher fan-out returns with no row (`:104`).
- User-channel teardown (`registry.rs:181 leave_all`, called from `session_teardown.rs:289`, `dispatch/session.rs:180`) writes nothing, so joins (426 in 30 days) never pair with leaves.
- Mute expiry is lazy (`mutes/mod.rs:70`) and silent; a restart clears every mute with no row (decision D-SS26, so only the expiry is a gap).
- Discord chat toggles (`chat_say`, `chat_whisper`, ...) have no production emitter (`emit_chat` is called only from `crates/discord/src/lib.rs:355` tests). Probably intended for privacy; say so in the config doc.

## 3. Negative gaps

| Site | Gap |
|---|---|
| `crates/discord/src/sender/http.rs:42` | `SendError::Network(e.to_string())`: reqwest's message includes the full webhook URL, token included. It reaches the file log and SigNoz as `error` on `Discord send failed after retries` (2 rows 2026-10-04). Credential rule violated; the token should be rotated. |
| `sender/handle.rs:49-56`, `task.rs:70-73` | Queue-full, queue-closed and no-webhook drops only bump atomics. `stats()` is read by nothing (the "heartbeat" in `sender/mod.rs:27` was never built), so a dead alert channel is invisible. Rate-limit drops are DEBUG with no payload (`task.rs:99`). |
| `base/src/base/dispatch/chat.rs:113-138` | Empty payload and an undecodable target or text return silently. The AFK/DND handlers log the same fault at WARN (`:818`, 1 row on 2026-10-04). File is 977 lines, over the 700 hard cap. |
| `chat.rs:328` | `let _ = tx.send(BaseToCellMsg::ChatMessage ..)`: a closed cell channel eats the line after the INFO `sendPlayerCommunication` row already said it was handled. `:326` also skips silently when `player_eid` is `None`. |
| `chat.rs:444-451`, `:683` | `chatJoin` with bad payload, and a channel post with no entity, return with no row and no feedback. |
| `cell-console/.../spatial.rs:124`, `:156` | Both witness and echo sends are `let _ = tx.send(..)`. |
| `cell-methods/.../contact_list/mod.rs:150`, `:189`, `:239`, `:281`, `:331` | Five arms `return true` on short args with no row (`CREATE` logs its malformed case). |
| `contact_list/handlers/header_ops.rs:93` | A failed `ContactListCreate` is a WARN with "(duplicate name?)" guessed in the text and no `reason`; the player gets no echo or line. |
| `base-session/.../organization/handlers/fanout.rs:29` | `online_members` returns an empty roster when the session lock is poisoned: every rank/MOTD fan-out then reaches nobody with no row (invariant 2). Same pattern in `log_names.rs:18`, `disband.rs:231`, `invite_response.rs:181`. |
| `black_market/sweep.rs:222` | `who_is` swallows a DB error with `.ok().flatten()`, so a failed lookup looks like an unknown seller. |
| `cell-duel/.../engage.rs:42`, `:54` | A countdown end for a vanished or non-pending duel returns silently. |
| `trade/execute/mod.rs:156` | The negative-cash refusal carries no identity. |
| `cell-combat/.../death/mod.rs:85-89` | Death presence uses `format!("entity:{eid}")` when the character name is missing; `find_watchers` finds nobody, silently. |

## 4. Noise

- `Discord config reload: no semantic change` (`config/watcher.rs:173`): 8,960 rows on 2026-10-04 and 56,312 on 2026-10-05, in two bursts of about 2.5 hours at roughly 6 per second. The watcher's 150 ms debounce plus a re-read on every event matches a self-trigger loop: `notify` reports access/open events on Linux and `reload_from_disk` reads the file. 90% of all Discord-scope volume.
- `mail expiry sweep finished` (`mail/expiry/mod.rs:397`): 4,556 rows in 30 days, 100% with `scanned = 0`.
- `squad.disconnect_no_member`: one per logout (the second `DisconnectEntity`). Harmless at today's volume.

## 5. Seams (two hops)

- **Login/session -> contacts.** `session_presence::announce_session_end` -> `fanout_login_status(false)` -> `find_watchers`. Both sides log, but "online" has two definitions: contacts use `player_entity_id.is_some()` (`contact_list/handlers/mod.rs:259`), orgs use `listed_online` (`organization/handlers/fanout.rs:35`). After `logOff` and before the reap, a new login can be told a logged-off friend is online, and no row would show it. The fan-out does not honour Ignore (section 6, scenario 1).
- **Persistence -> social.** `delete_character` -> FK cascades (`db/sgw/_foreign_keys.sql`): `sgw_auction.seller_id` and `sgw_auction_bid.bidder_id` `ON DELETE CASCADE`, `sgw_gate_mail.character_id` CASCADE, `sgw_gate_mail.sender_id` and `sgw_auction.current_bidder` `SET NULL`. Only organizations are logged (`character_delete.rs:98`). No cascade logs what it destroyed.
- **Items -> trade/mail.** Trade does not lock items; the base re-validates under `FOR UPDATE` at commit, and logs `trade.item_moved` per item (good). Items offered but never committed leave no row anywhere. Mail claim and BM escrow rows are complete. Trade also never consults the Ignore list (duel, tell, mail, org invite and spatial chat do); a product decision, not telemetry.
- **GM console -> social.** `org.gm_action`, `chat.gm_mute/unmute`, `mail.expire_*`, `bm.*` GM rows all exist with `account_id` of the GM. The cell console's `emit_gm_command` reaches Discord; base-side org GM actions do not (fine).
- **Hop 2.** Combat death -> contacts (`ContactListPresenceEvent`, the fallback name above); progression level-up -> contacts is a bare `tokio::spawn` (`progression/mod.rs:260`) whose panic would be lost; gate travel -> `GateTravel` fan-out (`gate_travel/mod.rs:624`); AoI witnesses -> spatial chat (stale witnesses show only as `witness_count`); Mercury reliable sends -> `send_to_witness_reliable` logs its own misses.

## 6. Adversarial

1. **Player-hit: "I ignored a harasser and still see them come online, die and gate".** `find_watchers` (`persistence/mod.rs:437`) matches the subject in any list, and the Ignore list (flags 301) is a list. The ignorer gets CM 89 for login, level, death and the destination world id. SigNoz today: DEBUG `ContactList: presence fanout complete watcher_count=N`, nothing about ignorers, nothing per recipient. Should: `ignoring_watchers=K` and, after the behaviour fix, `ignoring_watchers_skipped=K`.
2. **A seller deletes a character with a listing carrying a bid.** The auction row cascades; the bidder's held cash is never refunded and the item row goes with the seller. SigNoz: INFO `Character deleted org_events=0` and nothing else; the BM sweep never mentions it. Should: WARN `character.delete_cascade` with `auctions_destroyed`, `held_cash_destroyed`, `mail_destroyed`, `mail_attachments_destroyed`.
3. **"I said hello and nobody saw it."** Base INFO `sendPlayerCommunication`, cell DEBUG `Broadcasting chat ... witness_count=0` with no player identity. If the cell channel is closed, both sends are `let _` and the trail ends at the base INFO row. Should: identity on the cell row, `recipients`/`ignored`, and a WARN `chat.relay_failed`.
4. **Both players' trade windows close.** `trade session cancelled result=2` for range, disconnect, version, handoff. Should: `reason` and `event=trade.session_cancelled`.
5. **The Discord alert channel goes quiet.** Network failure logs the webhook URL (credential) once per outage; a full queue logs nothing. Should: a counter per outcome and a throttled WARN with `suppressed`.

## Candidate packets

| ID | Title | Sev | Files | Notes |
|---|---|---|---|---|
| TG-SOC-01 | Strip the webhook URL from Discord send errors | high | 1 | owner must rotate the webhook |
| TG-SOC-02 | `character.delete_cascade` row | high | 1 | live-DB test |
| TG-SOC-03 | Refund-or-destroy decision for cascades | high | design | needs-domain-agent |
| TG-SOC-04 | Presence must skip Ignore-list watchers | high | design | needs-domain-agent |
| TG-SOC-05 | Presence fan-out counts incl. `ignoring_watchers` | med | 2 | measures 04 |
| TG-SOC-06 | Cell trade events, identity, cancel reason | med | 3 | |
| TG-SOC-07 | Cell spatial chat send failures and identity | med | 1 | |
| TG-SOC-08 | Base chat malformed/relay refusals | med | 3 | file over cap |
| TG-SOC-09 | Cell contact-list short-args rows | med | 1 | |
| TG-SOC-10 | Contact header ops `event=`, `account_id`, create reason | med | 1 | |
| TG-SOC-11 | Discord config watcher self-trigger | med | 1 | |
| TG-SOC-12 | Discord drop accounting | med | 3 | |
| TG-SOC-13 | Mail sweep: no row when nothing scanned | low | 1 | |
| TG-SOC-14 | User-channel teardown leaves | low | 3 | |
| TG-SOC-15 | Death presence without a character name | low | 1 | |
| TG-SOC-16 | Org `online_members` poisoned lock | low | 1 | |

**TG-SOC-01 Strip the webhook URL from Discord send errors (high).** `crates/discord/src/sender/http.rs`, `HttpDiscordSender::send`: map the reqwest error with `e.without_url().to_string()`. Test (unit, wiremock-free): post to a refused local port whose path ends in a marker segment (`/hook/SECRETMARKER`), assert the `SendError::Network` text lacks the marker and the redacted text still names the failure kind. It fails when reverted because reqwest embeds the URL. No filter change. Coordinator action outside the packet: rotate the webhook, since the old token sits in retained logs.

**TG-SOC-02 `character.delete_cascade` (high).** `crates/base-session/src/base/organization/character_delete.rs`, `delete_character`: inside the transaction, after the `FOR UPDATE` and before the `DELETE`, count `auctions_destroyed` (active seller rows), `held_cash_destroyed` (cash held on those by a standing bid), `bids_held_lost` (rows where this player is the current bidder), `mail_destroyed`, `mail_attachments_destroyed`, `cod_mails`. Emit `event = "character.delete_cascade"` after commit: `warn!` if any held cash or attachment is nonzero, else `debug!`; fields `player_id`, `player_name`, `account_id`, `account_name`, the counts. Confirm column names with database-persistence first. Test: live-DB, seed an auction with a bid and a mail with an item, delete, assert the WARN counts; fails when the block is removed. No filter change.

**TG-SOC-03 Cascade refund decision (high, needs-domain-agent).** Invariants 3 and 4 say cancel-and-refund by mail; the schema destroys. Decide between a pre-delete step in `delete_character` that mails refunds and returns attachments, or accepting destruction with TG-SOC-02 as the audit. Needs database-persistence (trigger or code) and a ruling on seller-deleted listings with a standing bid.

**TG-SOC-04 Presence skips Ignore-list watchers (high, needs-domain-agent).** Filter flags-301 lists out of `find_watchers` and `notify_online_contacts`; first confirm by RE or lab whether the client's Ignore panel needs presence. Guard: live-DB test that an ignorer receives no CM 89.

**TG-SOC-05 Presence fan-out counts (med).** `persistence/mod.rs` add `find_watchers_with_flags` returning `(player_id, flags)`; `presence_fanout.rs` `fanout_contact_event` ends with `event = "contacts.presence_fanout"` at `debug!`: `watchers`, `ignoring_watchers`, `online_watchers`, `sent`, plus `player_name` and the subject's `player_id` when known; log the zero-watcher case too. Test: live-DB, subject in one friend list and one Ignore list, assert `ignoring_watchers = 1`; absent field fails when reverted.

**TG-SOC-06 Cell trade events (med).** `cell-interactions/src/cell/trade/state.rs`, `cell-methods/.../trade/handlers.rs`, `handoff.rs`. Add `reason: &'static str` to `cancel_session` (`user_cancel`, `bad_proposal`, `out_of_range`, `handoff_out_of_range`, `disconnect`, `send_failed`); rename rows to `event = "trade.session_opened" | "trade.session_refused" | "trade.session_cancelled" | "trade.execute_requested"`; add `account_id`, `player_id` from `space_mgr.player_identity`. Levels unchanged. Test: extend the existing handler tests with `LogCapture`: a disconnect cancel finds `trade session cancelled` with `reason=disconnect`; fails if `reason` is dropped.

**TG-SOC-07 Spatial chat (med).** `cell-console/src/cell/console/chat/spatial.rs`: replace both `let _ = tx.send` with `if let Err` -> `warn!(event = "chat.spatial_send_failed", reason = "cell_to_base_closed")`; add `account_id`, `player_id`, `account_name` and `ignored = ignored_by.len()` to the broadcast row, with `event = "chat.spatial_broadcast"`. Test: `LogCapture` with a dropped receiver; revert leaves no WARN.

**TG-SOC-08 Base chat refusals (med).** `base/src/base/dispatch/chat.rs`, new `chat_malformed.rs`, `dispatch/mod.rs`. Put the new rows in the sibling (chat.rs is over the hard cap): `warn!(event = "chat.malformed", reason = "empty_payload" | "bad_target" | "bad_text" | "bad_channel_name", payload_len)` at `:113-138` and `:444-451`; `warn!(event = "chat.relay_failed", reason = "cell_channel_closed" | "no_player_entity")` at `:326-328` and `:683`. Test: `LogCapture`, one case per reason.

**TG-SOC-09 Cell contact short args (med).** `cell-methods/.../contact_list/mod.rs`: at the five early returns, `warn!(event = "contacts.call_malformed", op, args_len, need)` with entity names and `account_id`. Test: drive `dispatch` with 2-byte args per op; silent code fails.

**TG-SOC-10 Contact header ops (med).** `base-session/.../contact_list/handlers/header_ops.rs`: add `event = "contacts.list_created|list_deleted|list_renamed|list_flags_updated"` and `account_id`/`account_name` on every row; on create failure classify with `sqlx::Error::Database(e).is_unique_violation()` -> `reason = "duplicate_name"`, else `"db_error"`. Test: live-DB duplicate create finds `reason=duplicate_name`.

**TG-SOC-11 Discord watcher loop (med).** `crates/discord/src/config/watcher.rs`: in the `notify` callback drop `EventKind::Access(_)` (and `Other`); only `Create|Modify|Remove` queue a reload. Test: temp file, `LogCapture`, current-thread runtime; write once -> exactly one `Discord config reloaded`, then read the file 10 times and sleep past the debounce -> no further reload rows. Only meaningful on Linux CI (Windows reports no access events); say so in the test comment.

**TG-SOC-12 Discord drop accounting (med).** `sender/handle.rs`, `sender/task.rs`, `sender/stats.rs`: counter `discord_events_total{outcome}` with an enumerated `metric_label!` (`sent`, `filtered`, `dropped_full`, `dropped_closed`, `no_webhook`, `rate_limited`, `failed`); throttled WARN `event = "discord.dropped"` (first per outcome, then one per 60 s with `suppressed`) on target `cimmeria_discord` (already pinned). Test: fill the queue with `MockSender` stalled; assert one WARN with `suppressed = 0` and the counter; burst and independence per negative-logging Pattern D.

**TG-SOC-13 Mail sweep noise (low).** `mail/expiry/mod.rs` `log_summary`: return early when `scanned == 0 && failed == 0` for both sources. Test: `LogCapture`; empty sweep has no `mail.expiry_sweep`, a sweep with one expiry has one.

**TG-SOC-14 Channel teardown (low).** `registry.rs` `leave_all` returns `Vec<(u8, String, bool)>`; `session_teardown.rs` and `dispatch/session.rs` log `debug!(event = "chat.channel_left", reason = "session_end", wire_id, display_name, deleted)` with identity. Test: registry unit asserts the return; `LogCapture` in the existing teardown test.

**TG-SOC-15 Death presence (low).** `cell-combat/src/cell/abilities/death/mod.rs:85`: when `character_name` is `None`, skip the send and `debug!(event = "contacts.presence_skipped", reason = "no_character_name")`. Test: death of an unnamed player test entity sends no `ContactListPresenceEvent`.

**TG-SOC-16 Org online roster (low).** `organization/handlers/fanout.rs:29`: on a poisoned lock `error!(event = "org.fanout_lock_poisoned", member_ids = n)` and return. Test: poison the mutex in a thread, assert the row; removal fails it.

No `OTEL_FILTER` change is needed by any packet; all targets and module paths are already pinned.
