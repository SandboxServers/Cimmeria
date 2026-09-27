# SS-C1 Worknote: Tells and Ignore

> Type: reference. Audience: the social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md#ss-c1-tells-and-ignore), [audit.md](../audit.md).

## Contract

- **Packet:** SS-C1, tells and Ignore (work-packets.md § SS-C1).
- **Decisions in force:** D-SS12 (text rules, applied by SS-00 before the tell branch), D-SS13 (name resolution), D-SS14 (the chat bucket covers tells), D-SS15 (Ignore: the flags-301 list, one direction, chat/duel/mail only, no AoI hiding), D-SS17 (channel ids follow D-ORG14; this packet does not edit `CHAN_*`). Security rows: CAT-L-01 (the Ignore half) and CAT-L-07 (`chatIgnore` only), with CAT-L-04's cap.
- **Tell channel byte:** 10, per ORG-E1 Q5 / D-ORG14, which SS-E1 C-Q1 cites (`git show origin/social/se1-re:docs/reverse-engineering/findings/chat-wire-formats.md`). No named constant for 10 exists (`world_entry_chat::CHAN_TELL` and `cimmeria_wire::cell::chat::CHAN_TELL` both still say 9), so `crates/base/src/base/dispatch/tell.rs` defines `TELL_CHANNEL = 10` locally with the citation.
- **Base:** first `origin/main` @ `88d7da73a` (SS-00 merged as #880); rebased onto `origin/main` @ `ee36c78b4` with `git rebase --onto origin/main fb657944a` after the split landed as #885 (`0f4b7516b`, with the coordinator's `CHAT_LOG_TARGET` fix). Branch `social/c1-tells-ignore`.
- **Commits (after the rebase):**
  1. the split: merged separately as #885 (`0f4b7516b`); the original `fb657944a` was dropped by the rebase
  2. `e2993e356` feat(chat): spatial chat skips a witness who ignores the speaker
  3. `f026cfaf2` feat(contact-list): Ignore cache on the base session, synced to the cell
  4. `f92a6a841` feat(chat): tells and chatIgnore on the base
  5. `e1c821863` test(wireclient): two clients exchange a tell
  6. `8497d5da7` docs(chat): tells and Ignore
  7. `92cdeddaa` fix(chat): lone speaker echo, case-insensitive Ignore, chatIgnore spends a chat token (the coordinator's review round)
  8. `4338e9b0e` docs: round 2
  9. `2beed7f79` fix(chat): address tells to the session's entity at send time; identity on Ignore telemetry (#893 review)
  10. docs and worknote for round 3 (the commit after these)
- **Owned paths (new):** `crates/base/src/base/dispatch/{tell,ignore}.rs`, `crates/base/src/base/dispatch/tests/{tell,chat_ignore}.rs`, `crates/base-session/src/base/contact_list/ignore/`, `crates/cell/src/cell/service/base_messages/ignore.rs` and its test, `crates/cell-console/src/cell/console/chat/` (the split), `crates/wireclient/tests/it/two_client_tell.rs`, this file.
- **Read set:** SS-WORKER-RULES.md; work-packets.md (contract, contended files, SS-C1); README.md D-SS12..D-SS17; audit.md A-26, A-27, A-31, § 6; CAT-L-chat-contact.md (L-04, L-07); SS-E1's chat findings (unmerged #875); PR #585's diff; `deprecated/python/base/Chat.py:314-358` and `SGWPlayer.py:195-210`; the SS-00 modules (`player_index`, `rate_limit`, `feedback`); the contact-list handlers and persistence; `client_ready/mod.rs`; `Communicator.def`.

## Evidence

- **Tells never worked.** Before this packet the base forwarded every `sendPlayerCommunication` to the cell. The client's `/tell` byte, 10, is `CHAN_SPLASH` in the Rust constants, so the cell answered "Speaking on channel 10 is not supported yet!".
- **Legacy tell shape** (`Chat.py:339-358`): an empty target is refused; a missing recipient gets a feedback line; otherwise `onTellSent(target, message)` goes to the sender and `onPlayerCommunication(sender, flags, CHAN_tell, message)` to the recipient. Legacy had no AFK auto-reply (`chatSetAFKMessage` only stored the text). The packet asks for the auto-reply, so it is a Cimmeria addition built on the stored text.
- **`chatIgnore`** takes `WSTRING aPlayerName, UINT8 aFlag` (`Communicator.def`; SS-E1 records 1 = ignore, 0 = stop). The dispatch table omitted the flag (A-27); rows 5 and 6 are fixed.
- **Ignore list identity:** `ensure_system_lists` creates it by name `'Ignore'` with flags 301. Reads use `flags = 301` (D-SS15); writes use the id `ensure_system_lists` returns.
- **Gate travel** re-runs `onClientReady` for a character that is still listed (`client_ready/mod.rs`, the `listed_online` comment), and it creates a fresh cell entity. So seeding the cell set at `onClientReady` covers gate travel with no extra hook.

## Design decisions

- **The chat.rs split comes first and is pure** (commit 1): `chat/mod.rs` (the handler, re-exports, the method index), `spatial.rs`, `feedback.rs`, `tests/{spatial,dot_command,feedback}.rs`. Bodies are moved verbatim; import paths are unchanged, and the log targets stay under `…::console::chat` by prefix. The 7 existing chat tests pass unchanged.
- **Three copies of the list, one resync.** `contact_list::ignore::resync_ignore_cache` reloads the flags-301 names and rewrites `ConnectedClientState::ignore` (an `IgnoreCache`) and the cell entity's `ignore_names` (`BaseToCellMsg::UpdateIgnoreList`). It runs (a) at every `onClientReady`, after `InitPlayerState`, and (b) inside `handle_add_members` / `handle_remove_members` when the changed list has flags 301. Because it is in (b), the contact-list UI path and `chatIgnore` share one code path. The resync writes the session only if it still plays the same `player_id`, so a logOff between the load and the write cannot move char A's list onto char B.
- **One direction, the line only (D-SS15).** The cell skips a witness whose set holds the speaker's name. The speaker's own echo is unchanged, so being ignored is not revealed, and nothing touches AoI. PR #585's symmetric check and AoI hiding are not taken. The new `chat.spatial_ignored` log passes `target: CHAT_LOG_TARGET`, as #885 requires for `chat/` submodules.
- **The lone-speaker echo (review round).** `broadcast_to_witnesses` used to return before the speaker's own echo when there were no player witnesses. The client does not echo say, so a lone player's first line showed nothing. The early return is gone; the trace log stays.
- **Names compare case-insensitively (review round).** The D-SS13 fold applies in `IgnoreCache::ignores` (the cache stores folded names), `player_ignores` (`lower()` on both sides) and the cell's spatial filter, so an entry the contact-list window stored in the wrong case still matches. The cost: `sgw_player.player_name` is case-sensitive UNIQUE, so one entry covers "Bob" and "bob" alike; that is the safe direction for an Ignore. `chatIgnore` still stores the canonical name that `resolve_character` returns, and its duplicate check folds too. A remove matches the typed name against the list itself (exact, then a unique case-fold), so the entry for a deleted character can still be removed.
- **Ignored `player_id`s.** `resync_ignore_cache` also loads the ids of the characters whose names fold-match an entry (`load_ignored_player_ids`), and `IgnoreCache::ignores_player(id)` answers from them. That is the query for SS-D1's seam, which holds the challenger's id, not a name.
- **`chatIgnore` spends a chat token (review round).** The D-SS14 chat bucket runs under the same lock that reads the caller, before any database work; GameMaster and above are exempt as for chat. A limited call gets the usual "too quickly" line at most once per 5 s and logs `rate_limit.exceeded category=chat`.
- **The cap is 100 names** (`MAX_IGNORE_LIST_MEMBERS`), project policy from CAT-L-04's remediation text ("e.g. 100 per list"). The audit's aside "the original SGW limit was 50 friends / 50 ignores" has no source, so it is not used.
- **The tell runs after the SS-00 gates**: bucket, then text rules, then `channel == TELL_CHANNEL`. A tell costs a chat token, as D-SS14 says.
- **Away reply:** the DND text wins over AFK. The reply carries the recipient's speaker flags, so `SPEAKER_DND` is set on a DND reply. The tell itself is always delivered.
- **The send primitive:** `feedback::send_player_method` is the reliable single-session send that already sat under `send_feedback_line`, now public so tells can send methods 28 and 30 to any session. `send_feedback_to_entity` calls it.
- **Feedback for a malformed `chatIgnore`:** a real client never sends one, but the rule is visible feedback for every refusal, so one line is sent anyway.

## API for SS-M1 (mail) and SS-D1 (duels)

In `cimmeria_base_session::base::contact_list::ignore` (reached as `crate::base::contact_list::ignore` in `cimmeria-base`):

- `player_ignores(pool, recipient_player_id, sender_name) -> Result<bool, sqlx::Error>`: the database check, which works for an offline recipient. Use it for mail send; it reads the flags-301 list only.
- `feedback::send_to_current_player(ctx, addr, player_id, method, payload)`: send to a session's current player entity, refusing a stale character. Mail and duel notifications to another player should use it instead of a remembered entity id.
- `session_ignores(&clients, recipient_addr, sender_name) -> bool`: the cached check for an online recipient (the caller holds the `connected` lock). Tells use it; duel challenges can too.
- `IgnoreCache::ignores(name)` on `ConnectedClientState::ignore` (case-insensitive).
- **SS-D1's seam:** `crates/base/src/base/dispatch/duel.rs::ignores(target, challenger_player_id)` on `social/d1-duel-challenge` becomes `target.ignore.ignores_player(challenger_player_id)`. SS-C1 does not edit that file.
- `resolve_character(pool, typed) -> CharacterLookup { Found { player_id, name }, Ambiguous, NotFound }`: D-SS13 against `sgw_player`. SS-M1 may want to reuse it rather than write a second resolver.
- `match_name(names, typed) -> NameMatch`: the exact-then-unique-case-fold rule over any name set.
- Refusal text: `contact_list::ignore::not_accepting_text(recipient)` (public) gives "X is not accepting your messages.", for tells, mail and duels alike.

## Telemetry

All events are on target `chat`, which SS-00 already registered in `OTEL_FILTER` at debug. No new target was added.

| Event | Level | Fields |
|---|---|---|
| `chat.tell_delivered` | INFO | `player_id`, `account_id`, `entity_id`, `target_player_id`, `target_account_id`, `target_entity_id`, `text_units`, `away_reply` |
| `chat.tell_refused` | DEBUG | the sender ids, `target_player_id` (when resolved), `target_name` (64-char prefix), `reason`: `no_target`, `self`, `not_online`, `ambiguous`, `recipient_ignores_sender`, `recipient_not_in_world`, `recipient_left` or `recipient_send_failed` |
| `chat.ignore_added` / `chat.ignore_removed` | INFO | `player_id`, `account_id`, `entity_id`, `target_player_id` (add), `before`, `after` |
| `chat.ignore_refused` | DEBUG (ERROR for `db_error`) | `reason`: `not_in_world`, `decode_failed`, `bad_flag`, `no_target`, `no_db_pool`, `unknown_character`, `ambiguous`, `self`, `already_ignored`, `list_full`, `not_ignored`, `write_failed` or `db_error` |
| `chat.ignore_synced` | DEBUG | `path` (`world_entry`, `contact_list`), `before`, `after`, the ids |
| `chat.ignore_sync_failed` | WARN (`no_db_pool`, `cell_send_failed`), ERROR (`db_error`), DEBUG (`session_changed`, `entity_to_addr_miss`) | `reason`, `path` |
| `chat.spatial_ignored` | DEBUG | `entity_id`, `player_id`, `account_id`, `channel`, one row per withheld witness with `target_entity_id`, `target_player_id`, `target_account_id`, `reason = witness_ignores_speaker` |
| `chat.ignore_set_applied` / `chat.ignore_set_dropped` | DEBUG (cell) | `entity_id`, `player_id`, `account_id`, `before`, `after` / `reason = entity_missing` or `player_mismatch` (WARN) |
| `chat.afk_set` | DEBUG | the ids, `afk_active` |

Spans: `chat.tell` and `chat.ignore` at INFO.

SigNoz queries for a tester: `target = 'chat' AND event IN ('chat.tell_delivered','chat.tell_refused') AND player_id = <id>` for tells; `event LIKE 'chat.ignore%'` for Ignore edits and syncs; `event = 'chat.spatial_ignored'` for say lines withheld by an Ignore.

## Commands run

All run from the ss-c1 worktree through the lane. The lane's own exit code does not show a cargo failure, so each output file was also grepped for `^error`, the result lines and `[lane] released (exit 0`.

| Command | Result |
|---|---|
| `lane.sh cargo test -p cimmeria-cell-console --lib chat::` (after the split) | 7 passed, exit 0 |
| `lane.sh cargo check -p cimmeria-cell -p cimmeria-cell-console --all-targets` | exit 0 |
| `lane.sh cargo test -p cimmeria-base --lib dispatch::` | 41 passed (live-DB cases self-skipped in this run) |
| `lane.sh cargo test -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-wire -p cimmeria-cell -p cimmeria-cell-console -p cimmeria-entity --lib` | 146 / 125 / 450 / 269 / 341 / 210 passed, 0 failed |
| `live-db-test.sh ignore` | 41 of 42 passed; the one failure was a test-decoder bug (extended method encoding for CM 87), fixed |
| `live-db-test.sh chat_ignore` | 4 passed, 0 skipped |
| `reload-db.sh`, then `DATABASE_URL=…/sgw_ss_c1 lane.sh cargo test -p cimmeria-wireclient --test it two_client_tell -- --test-threads=1` | 1 passed in 2.94 s (real orchestrator, not skipped) |
| `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-entity -p cimmeria-cell -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-base-methods -p cimmeria-services -p cimmeria-wireclient --all-targets -- -D warnings` | exit 0, no warnings |
| `lane.sh cargo test -p cimmeria-server --bins logging` | 52 passed (target scan and filter parity) |
| `lane.sh cargo fmt --all -- --check` | clean |

After the rebase and the review round:

| Command | Result |
|---|---|
| `lane.sh cargo test -p cimmeria-base -p cimmeria-base-session -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-base-world-entry --lib` | 97 / 146 / 127 / 451 / 271 passed, 0 failed |
| `live-db-test.sh ignore` | 44 of 45; `chat_ignore_refuses_unknown_duplicate_absent_and_full` hit the new chat bucket (7 calls in one instant). Its helper now refills the bucket before each call |
| `live-db-test.sh chat_ignore` | 6 passed, 0 skipped |
| `DATABASE_URL=…/sgw_ss_c1 lane.sh cargo test -p cimmeria-wireclient --test it two_client_tell -- --test-threads=1` | 1 passed in 2.69 s |
| clippy on the same 10 crates, `-D warnings` | exit 0 |
| `lane.sh cargo fmt --all -- --check` | clean |

## Regression proof

Each guard was run with its fix disabled in place, then the file was restored from a copy.

| Guard | Fix disabled | Result |
|---|---|---|
| `tell_to_ignoring_player_not_delivered` | `if false && recipient.ignores_sender` in `tell.rs` | FAILED |
| `spatial_chat_skips_ignoring_witness` | `false &&` in the partition predicate in `spatial.rs` | FAILED |
| `chat_ignore_rejects_self` (live DB) | `if false && target_player_id == caller.player_id` in `dispatch/ignore.rs` | FAILED |
| `on_client_ready_seeds_ignore_list_after_init_player_state` (live DB) | the `resync_ignore_cache(.., "world_entry")` call wrapped in `if false` | FAILED |
| `contact_list_ui_edit_of_ignore_list_resyncs_session_and_cell` (live DB) | early `return` in `resync_if_ignore_list` | FAILED |

Review round:

| Guard | Fix disabled | Result |
|---|---|---|
| `lone_speaker_still_gets_own_echo` | `return;` restored after the empty-witness trace | FAILED |
| `spatial_chat_ignore_matches_case_insensitively` | the spatial predicate compares `n == speaker_name` | FAILED |
| `tell_ignore_matches_case_insensitively` | `fold_name` returns the name unchanged | FAILED |
| `player_ignores_reads_only_the_ignore_list` (live DB) | exact `m.player_name = $3` in the SQL (plus the identity `fold_name`) | FAILED |
| `resync_ignore_cache_updates_session_and_cell` (live DB) | the same two changes | FAILED |
| `chat_ignore_spends_a_chat_token` | the bucket check replaced with `RateDecision::Allowed` | FAILED |

Not proven by revert: `tell_reaches_exactly_one_recipient`. Without the tell branch, Bob receives nothing, so the test fails by construction, but no revert run was recorded for it.

## Tests added (the audit § 6 names are kept)

- Type 8 (`cimmeria-base` `dispatch::tests::tell`): `tell_reaches_exactly_one_recipient`, `tell_to_ignoring_player_not_delivered`, `tell_from_player_who_ignores_recipient_is_delivered`, `tell_resolves_a_case_folded_name`, `tell_to_ambiguous_name_is_refused`, `tell_refusals_feed_back_with_reason`, `tell_to_away_player_replies_with_away_message`, `tell_to_dnd_player_replies_with_dnd_message`.
- Cell: `spatial_chat_skips_ignoring_witness`, `spatial_chat_reaches_witness_the_speaker_ignores`, `spatial_chat_ignore_matches_case_insensitively`, `lone_speaker_still_gets_own_echo`, `update_ignore_list_replaces_the_cell_entity_set`, `update_ignore_list_for_missing_entity_logs_reason`.
- Live DB: `chat_ignore_adds_to_own_ignore_list`, `chat_ignore_rejects_self`, `chat_ignore_refuses_unknown_duplicate_absent_and_full`, `player_ignores_reads_only_the_ignore_list`, `resolve_character_follows_d_ss13`, `resync_ignore_cache_updates_session_and_cell`, `contact_list_ui_edit_of_ignore_list_resyncs_session_and_cell`, `on_client_ready_seeds_ignore_list_after_init_player_state`.
- Type 12, no DB: `chat_ignore_refusals_before_the_database`, `chat_ignore_spends_a_chat_token`, `tell_ignore_matches_case_insensitively`.
- Unit: `match_name_prefers_exact_then_unique_case_fold`, `session_ignores_reads_the_recipient_cache_only`, `chat_set_afk_stores_clears_and_bounds_the_away_message`, the extended logOff reset test (AFK and Ignore cleared).
- Wire (type 2): `serialize_on_tell_sent_is_two_wstrings`.
- Type 11: `two_client_tell::two_clients_exchange_a_tell` (not in CI, A-60).

Sentinels: `0x7300_C1xx` (ignore module), `0x7300_C2xx` (chatIgnore dispatch), `0x7300_C300`/`C301` (client_ready); wireclient accounts 900_311-900_314.

## Review round 3 (PR #893, Copilot)

**Stale recipient entity (`tell.rs`).** `resolve` used to copy the recipient's `player_entity_id` under the session-map lock, and the send ran after the lock was released and after an await. A gate travel in that window would have sent the tell to the old entity and still logged it as delivered. Now `Recipient` holds no entity id. The send goes through `feedback::send_to_current_player(ctx, addr, player_id, method, payload)`, which reads the session's `player_entity_id` under the same lock that allocates the sequence number. It also checks that the session still plays that `player_id`. If the session is gone, the result is `NoSession` (tell refused, `reason = recipient_left`). If it has no player entity or plays another character, the result is `NotInWorld` (`reason = recipient_not_in_world`). Either way the sender gets "Player X is not online.". The sender's `onTellSent` and the away reply use the same function with the sender's `player_id`. The guards use a `#[cfg(test)]` thread-local hook (`tell::after_resolve_hook`) that runs between the lookup and the send:

- `tell_is_addressed_to_the_recipients_entity_at_send_time`: the hook moves Bob to entity 9999, and the tell must address 9999.
- `tell_to_recipient_who_left_the_world_before_the_send_is_refused`: the hook clears Bob's entity; Bob gets nothing, and Alice gets the not-online line with `reason = recipient_not_in_world`.

**`UpdateIgnoreList` identity.** The message carries both `entity_id` and `player_id`. The cell now applies it only when `entity.player_id == Some(player_id)`, because entity ids are recycled and gate travel gives a character a new one. A push that lands on an id now held by another character logs `chat.ignore_set_dropped reason = player_mismatch` at WARN, and nothing changes. Gate travel is covered by the `onClientReady` resync: gate travel re-runs `onClientReady` for the new entity, and that resync pushes the set for the new `entity_id` with the same `player_id` after `InitPlayerState` (guard `on_client_ready_seeds_ignore_list_after_init_player_state`). The cell entity carries `player_id` from `CreateEntity`, so the check holds from birth. Guard: `update_ignore_list_for_another_players_entity_is_dropped`.

**Telemetry audit (instrumentation-discipline rule 5).** Every new event, and what changed:

| Event | account_id / player_id / entity_id | The other player |
|---|---|---|
| `chat.tell_delivered` | yes | `target_player_id`, `target_account_id`, `target_entity_id` (the entity actually used at send time) |
| `chat.tell_refused` | yes | `target_player_id` once resolved |
| `chat.ignore_refused` | yes; the `not_in_world` refusal now reads the ids from the session (they were missing) | `target_player_id` added where known: self, `already_ignored`, `list_full`, `write_failed` |
| `chat.ignore_added` / `chat.ignore_removed` | yes | `target_player_id`: resolved on an add; on a remove it is now looked up from the entry's name (unset if that character no longer exists) |
| `chat.ignore_synced` | yes | n/a (it is the owner's own list) |
| `chat.ignore_sync_failed` | `account_id` added to the `no_db_pool` and `db_error` rows and to the member-ops rows | n/a |
| `chat.spatial_ignored` | yes | now one row per withheld witness with `target_entity_id`, `target_player_id`, `target_account_id` (it was one row with a count) |
| `chat.ignore_set_applied` / `chat.ignore_set_dropped` | `account_id` added (from the cell entity) | `entity_player_id` on `player_mismatch` |
| `chat.afk_set` | yes | n/a |

**Doc.** `chat-system.md:11` no longer lists `tell` among the unsupported non-spatial channels (CRLF kept).

**Commands, round 3.**

| Command | Result |
|---|---|
| `lane.sh cargo test -p cimmeria-base -p cimmeria-base-session -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-base-world-entry --lib` | all pass except one run where `spatial_chat_skips_ignoring_witness` did not see its log row. It passed on two reruns of the crate and under nextest. That is the known `LogCapture` callsite-interest flake under plain `cargo test` (#891); CI uses nextest |
| `lane.sh cargo nextest run -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-base -p cimmeria-base-session --lib` | 968 passed |
| `live-db-test.sh ignore` (the full filter) | 46 passed, 0 failed |
| `live-db-test.sh tell` | 17 passed, 0 failed |
| `DATABASE_URL=…/sgw_ss_c1 lane.sh cargo test -p cimmeria-wireclient --test it two_client_tell -- --test-threads=1` | 1 passed (2.70 s) |
| clippy on the 10 crates, `-D warnings`; `cargo fmt --all -- --check` | clean |

**Regression proof, round 3.**

| Guard | Fix disabled | Result |
|---|---|---|
| `tell_is_addressed_to_the_recipients_entity_at_send_time` | the old shape restored: snapshot the entity before the hook, send with `send_player_method(snapshot)` | FAILED |
| `tell_to_recipient_who_left_the_world_before_the_send_is_refused` | same | FAILED |
| `update_ignore_list_for_another_players_entity_is_dropped` | the guard changed to `entity.player_id == Some(player_id) \|\| true` | FAILED |

## Review round 4 (PR #893)

`BaseToCellMsg::UpdateIgnoreList` now carries `account_id`, taken from the base session in the same lock where the session copy of the list is written. The cell logs it on every ignore-set event: `chat.ignore_set_applied`, and `chat.ignore_set_dropped` with either `reason = entity_missing` (where there is no entity to read it from) or `reason = player_mismatch`, which also logs the entity's own `entity_account_id`.

The tell-channel registration finding was answered by the coordinator (D-ORG14: the client hardcodes built-in channel ids; the constants belong to ORG-09). No code change.

| Command | Result |
|---|---|
| `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry --lib` | 1047 passed |
| `live-db-test.sh resync_ignore_cache_updates_session_and_cell` (asserts the pushed `account_id`) | 1 passed |
| clippy `-D warnings` on wire, cell, base-session, base, base-world-entry and services; `cargo fmt --all -- --check` | clean |

Guards: `update_ignore_list_for_missing_entity_logs_reason` and `update_ignore_list_for_another_players_entity_is_dropped` assert `account_id = 700`, and `update_ignore_list_replaces_the_cell_entity_set` now captures `chat.ignore_set_applied` and asserts its `account_id` and `player_id`. Proof: with `account_id` removed from the `entity_missing` row, `update_ignore_list_for_missing_entity_logs_reason` FAILED; restored.

## Rebase onto SS-M1 and SS-D1, and their Ignore seams

Rebased onto `origin/main` after SS-M1 (#894) and SS-D1 (#888) merged.

- **Conflicts:** `crates/wire/src/cell/messages/base_to_cell.rs` and `crates/cell/src/cell/service/base_messages/mod.rs` (the `Duel` variant and arm beside `UpdateIgnoreList`), `crates/base/src/base/dispatch/mod.rs` and `dispatch/tests/mod.rs` (`mod duel` / `mod duel_challenge` beside `ignore` / `tell` / `chat_ignore`), `docs/architecture/observability.md` (my `chat` row, then SS-M1's `mail` and SS-D1's `duel` rows, twice), and `docs/gap-analysis.md` (the TOTALS line and summary paragraph). All were resolved by keeping both sides. TOTALS and the percentage table were recomputed from the matrix rows: 472 / CW 169 / NT 68 / IM 99 / KM 132 / NU 4 (the rows sum to it; percentages 35.8 / 14.4 / 21.0 / 28.0 / 0.8; code exists 336, 71.2%).
- **Duel seam:** `dispatch/duel.rs::ignores` is now `target.ignore.ignores_player(challenger_player_id)`. Guard: `challenge_rejects_a_target_who_ignores_the_challenger`. It checks that `TEXT_TARGET_IGNORING` is sent, `reason = target_ignoring` is logged and nothing is forwarded. It FAILED with the `false` stub restored.
- **Mail seam:** `send::recipients::ignoring_sender(conn, sender_name, recipient_ids)` now calls the new batched `contact_list::ignore::recipients_ignoring`. That is the one-query form of `player_ignores`, on any executor, so it runs in the send transaction. `deliver` calls it after the `FOR UPDATE` lock, where the sender's stored name is read. `failure_line` now writes the shared `not_accepting_text` sentence for each ignoring recipient, after the list of the others; the result code and `FailedRecipients` are unchanged.
  - Live-DB guard: `mail::tests::send_ignore::send_skips_recipient_who_ignores_the_sender`. The recipient's list holds the sender in a different case: that recipient is in `FailedRecipients` under `MAILRESULT_Sent` and reads the not-accepting line, and the second recipient still gets the mail. It FAILED with the empty-set stub restored.
  - Unit: `failure_line_uses_the_shared_not_accepting_sentence`.
- **Two D-SS13 resolvers, both kept:** SS-M1's `recipients::resolve_names` + `candidate_rows` (one batched query for up to 10 typed names, in the send transaction) and SS-C1's `contact_list::ignore::resolve_character` (one name, used by `chatIgnore`). The batched one suits mail better; unifying them is not worth the churn.
- **Docs:** the `mail-system.md` send steps and feedback line, and the `duel-system.md` `sendDuelChallenge` row.

| Command | Result |
|---|---|
| `lane.sh cargo nextest run --no-fail-fast -p cimmeria-wire -p cimmeria-entity -p cimmeria-cell -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-base-methods --lib` | 2039 passed |
| `live-db-test.sh ignore` / `tell` / `mail` | 48 / 20 / 59 passed, 0 failed |
| clippy `-D warnings` on the 10 crates plus base-methods; `cargo fmt --all -- --check` | clean |

## Review round 6 (PR #893): the Ignore cap race and the summary audit

**1. Cap and duplicate checks were made on a snapshot (Copilot finding, real).** `chatIgnore` loaded the list, checked "already there" and "100 names" on that copy, then inserted through the member ops. Two overlapping adds (two `chatIgnore`s, or one and a contact-list UI add, which arrive on different tasks) could both see 99 and both insert, and a name differing only in case slipped past.

- **Fix:** `persistence::add_member_capped` makes both checks and the insert in one transaction.
  - It first locks the owner's list row `FOR UPDATE`; that `lock_owned_list` query is also the ownership check.
  - It then checks for the name case-insensitively (`Duplicate`, which returns the stored spelling), counts the entries (`Full` at the cap), and inserts with `ON CONFLICT DO NOTHING`.
- `persistence::add_members`, the UI path, now takes the same row lock in a transaction, so a UI add serialises with `chatIgnore` too.
- `db/sgw/_indexes.sql` gains `sgw_contact_list_member_list_lower_name_key`, a unique index on `(list_id, lower(player_name))`, edited in place with no migration. The seed has no conflicting rows.
- `chatIgnore` now calls `contact_list::ignore::add_ignore_entry`. It maps `Duplicate` to `already_ignored`, `Full` to `list_full` and a DB error to `db_error`, each with feedback. It then announces through the new shared `handlers::announce_added_members` (the CM 87 echo, online status and the Ignore resync), which `handle_add_members` also uses.
- Guards:
  - `chat_ignore_race::concurrent_ignore_adds_respect_the_cap` (type 5). The race is forced the way SS-M1's is: a `SHARE` lock on `sgw_contact_list_member` lets both adds count but not insert, until both are parked. With `FOR UPDATE` removed the list ends at 101 and the test FAILED.
  - `chat_ignore_race::chat_ignore_refuses_a_case_folded_duplicate` (live DB). It FAILED with the case-insensitive existence check made exact.

**2(a). Away-message validation (real).** The AFK and DND texts were bounded to 128 scalars but never checked by the text rules, and the tell auto-reply shows them to other players. `chat::away_message_allowed` now applies the D-SS12 / D-ORG10 character rules through `org_text::validate(TextField::ChatText, ..)`. `TooLong` is ignored, because the away message's own bound is 128 scalars, which can be up to 256 UTF-16 units, and truncation stays as CAT-L-02 decided. A refusal logs `chat.away_rejected` (WARN, `kind`, `reason`), sends "Your away message contains a character that cannot be sent. It was not set." and keeps the previous message. Guard: `chat_dnd_limit::away_messages_follow_the_chat_character_rules`, which FAILED with the check bypassed. The existing DND bound tests still pass.

**2(b). The 0xC5 decoder (partly real).**
- Already fine: `read_wstring` checks that the declared character count fits in the packet before it allocates, so a hostile length cannot allocate more than the packet carries. A flag byte other than 0 or 1 was already refused (`bad_flag`).
- The gap: the typed name had no semantic bound before database work. It is now checked by `validate(TextField::MailRecipient, ..)`: at most 64 UTF-16 units, the width of `sgw_player.player_name`, and no forbidden characters, the same bound a gate-mail recipient gets. A refusal gives "That is not a valid character name." with `reason` taken from the reject. Guard: extended `chat_ignore_refusals_before_the_database`, which covers a bidi override and 65 characters.

**2(c). Resync ordering (real).**
- The resync runs from the client-packet task (`onClientReady`, `chatIgnore`) and from the cell-message task (contact-list UI edits). Two resyncs can overlap, and the one that read the database first can finish last. It would then overwrite the session copy and push a stale set to the cell. The `mpsc` to the cell is FIFO per sender, but the two tasks are different senders.
- **Fix:** `IgnoreCache::begin_sync` hands out a per-session version before the database read, and `apply_sync` writes only a version newer than the last applied; an older one logs `chat.ignore_sync_failed reason = stale_version` and is not pushed. `UpdateIgnoreList` carries the version, and `CellEntity::ignore_version` keeps the newest; the cell drops an older push (`chat.ignore_set_dropped reason = stale_version`).
- Why the newest version is always right: every resync starts after the change it follows has committed, so the highest version read after the last commit. A new cell entity after gate travel starts at 0 and takes any version.
- Guards: `ignore_cache_applies_only_the_newest_resync` (unit) and `update_ignore_list_drops_an_older_version` (cell). Both FAILED with the version check removed.

**Splits.** `contact_list/ignore/mod.rs` reached 509 lines, so the resync moved to `ignore/resync.rs` (re-exported, so callers are unchanged). The two race tests moved from `dispatch/tests/chat_ignore.rs` (577 lines) to `chat_ignore_race.rs`.

| Command | Result |
|---|---|
| `lane.sh cargo nextest run --no-fail-fast -p cimmeria-wire -p cimmeria-entity -p cimmeria-cell -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-base-methods --lib` | 2044 passed |
| `live-db-test.sh ignore` / `tell` / `mail` / `contact_list` | 52 / 20 / 59 / 57 passed, 0 failed |
| clippy `-D warnings` on the 10 crates; `cargo fmt --all -- --check` | clean |

## Rebase after round 6

Rebased onto `origin/main` again after other campaigns merged. The only conflict was `docs/gap-analysis.md`: the TOTALS line and the summary block, which crafting CR-08 had also changed. I recomputed both from the matrix rows: 472 / CW 169 / NT 68 / IM 101 / KM 130 / NU 4 (35.8 / 14.4 / 21.4 / 27.5 / 0.8%; code exists 338, 71.6%). This supersedes the earlier 472 / 169 / 68 / 99 / 132 / 4 figure. After the rebase: nextest on the 8 crates 2190 passed; `live-db-test.sh` ignore 54, tell 22, mail 59, contact_list 57, all passed; fmt and clippy clean.

## Review round 7 (PR #893, five Copilot findings)

1. **UI batch past the cap.** `persistence::add_members_bounded` replaces the batched insert on the contact-list UI path. It takes the list row lock like before and now reads the list's flags from that lock. On the Ignore list (flags 301) it inserts names one by one against the count and puts every name past `MAX_IGNORE_LIST_MEMBERS` into `over_cap`. Other lists keep the single batched insert. `handle_add_members` sends one feedback line for the refused names ("Your Ignore list is full (100 names). N of the names were not added."), logs `chat.ignore_refused reason = list_full` with `added` and `refused` counts, and announces only what was inserted. `add_members` stays as a test-only wrapper. Guard: `contact_list_ui_batch_cannot_push_the_ignore_list_past_the_cap` (live DB). With 98 on the list, a 5-name batch adds 2 and the list stays at 100; a second batch adds nothing. It FAILED (103) with the cap check disabled.
2. **Two snapshots in the resync.** `load_ignore_snapshot` reads the names and the fold-matched `player_id`s in one query (a `LEFT JOIN sgw_player`), so both sets always describe the same list. `load_ignored_player_ids` is gone. Test: `load_ignore_snapshot_reads_names_and_ids_together` (a name with no character adds no id). That test proves the shape, not the atomicity: the atomicity comes from the single statement, which a test cannot tell apart from two reads with nothing committed in between.
3. **Away message truncated before the check.** Both handlers now run `away_message_allowed` on the whole decoded text, then cut it to 128 scalars. Guard: `away_message_with_a_forbidden_character_past_128_is_refused` (bidi at scalar 130, AFK and DND both refused, two feedback lines). It FAILED with the check moved back after the cut.
4. **Tell target unchecked.** `handle_tell` validates the target with `TextField::MailRecipient` (64 units, no forbidden characters) before the lookup. A refusal sends the fixed "That is not a valid character name." and logs `chat.tell_refused` with `reason` and `target_units`, never the text. Guard: `tell_to_an_invalid_target_name_is_refused_without_echo` (bidi, a BEL control character, 65 characters). It FAILED with the validation bypassed.
5. **Doc comment.** The query list in `contact_list/ignore/mod.rs` now names each consumer correctly: tells use `session_ignores`, duels `IgnoreCache::ignores_player`, mail `recipients_ignoring` (batched, inside the send transaction), and `player_ignores` is the single-recipient DB check.

Also: four SQL strings in `persistence/mod.rs` and `ignore/mod.rs` had lost their `\` line continuations in an earlier scripted edit. The SQL was still valid, but they are re-wrapped now.

| Command | Result |
|---|---|
| `lane.sh cargo nextest run --no-fail-fast -p cimmeria-wire -p cimmeria-entity -p cimmeria-cell -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-base-methods --lib` | 2194 passed |
| `live-db-test.sh ignore` / `tell` / `mail` / `contact_list` | 56 / 23 / 59 / 59 passed, 0 failed |
| clippy `-D warnings` on the 10 crates; `cargo fmt --all -- --check` | clean |

## Final rebase

Rebased onto `origin/main` after crafting CR-09. The only conflict was the `docs/gap-analysis.md` TOTALS and summary block again, recomputed from the rows: 472 / CW 169 / NT 68 / IM 102 / KM 129 / NU 4 (35.8 / 14.4 / 21.6 / 27.3 / 0.8%; code exists 339, 71.8%). After it: nextest on the 8 crates 2226 passed; `live-db-test.sh` ignore 56, tell 23, mail 59, contact_list 59, all passed; fmt and clippy clean.

## Known gaps

1. **Mute (SS-C3).** `tell.rs` has a `TODO(SS-C3)` where a muted sender is refused.
2. **One entry covers every character whose name folds to it** (a consequence of the case-insensitive match the coordinator asked for). Two characters that differ only in case are both ignored.
3. **Deleting the Ignore list or changing its flags** in the contact-list window does not resync; the next world entry does. Only member adds and removes resync. Accepted by the coordinator as a known gap.
4. **The Ignore list is identified two ways.** Writes use the list named `Ignore` (from `ensure_system_lists`) and reads use flags 301. A player who renames the system list or sets 301 on a custom list makes the two disagree. Pre-existing contact-list behaviour; accepted by the coordinator as a known gap.
5. **Client rendering of `onTellSent` and of an away reply on channel 10 is unverified** (SS-E1 C-Q4 is still open). The wireclient test proves the bytes, not the UI.

## Integration edits for the coordinator

1. **ORG-09 / SS-C4:** when `CHAN_TELL` becomes 10, replace `dispatch::tell::TELL_CHANNEL` with it and delete the local constant. The test files import `TELL_CHANNEL` from `tell`.
2. **Contended files touched, in merge order:** `crates/base/src/base/dispatch/mod.rs` (the 0xC5 arm, `mod ignore; mod tell;`, the `CHAT_SET_AFK` arm now passes `payload, connected`); `dispatch/chat.rs` (the tell branch after the text rules, and `handle_chat_set_afk` now stores the message); `crates/base-session/src/base/mod.rs` (`ConnectedClientState` gains `afk_message` and `ignore`, initialised at 7 struct literals); `cell-console/…/chat.rs` → `chat/` (merged as #885).
3. **Signature change:** `contact_list::handlers::handle_add_members` / `handle_remove_members` take `cell_tx` and return the changed names. The only caller is `contact_list_dispatch.rs`, which is updated.
4. **`crates/base/src/base/mod.rs`** now re-exports `player_index` from base-session.
5. **gap-analysis:** § 21 rows and its matrix row are updated (11 rows: 5 NT, 1 IM, 5 KM; a new "Ignore enforcement" row). The document's headline totals (the NT count and so on) are not recounted; that belongs to SS-99.
6. **Doc owned by D-ORG14:** in `chat-system.md` only the id-conflict note gained a sentence (tells use 10); the channel table is left to ORG-09.
