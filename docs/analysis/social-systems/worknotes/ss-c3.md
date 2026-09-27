# SS-C3 Worknote: Channel allowlist, moderation basics, feedback for the rest

> Type: reference. Audience: the social-systems coordinator and the PR reviewers.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md#ss-c3-channel-allowlist-moderation-basics-feedback-for-the-rest), [audit.md](../audit.md).

## Contract

- **Packet:** SS-C3 (work-packets.md § SS-C3).
- **Decisions in force:** D-SS14 (the chat bucket, which every SS-C3 refusal still charges), D-SS17 (channel ids follow D-ORG14; this packet does not edit `CHAN_*` or `world_entry_chat.rs`), D-SS26 (GM mutes, not persisted; a feedback line for 0xC6-0xCE), D-ORG14 (the client sends the `enumerations.xml` channel ids). Security rows: CAT-L-03 (channel byte) and the D-SS26 half of CAT-L-01. Audit A-27 (the dispatch-table rows).
- **Base:** `origin/main` @ `f2af64cf4` (SS-C1 merged as #893). Branch `social/c3-allowlist-mute`.
- **Commits:**
  1. `cd39eea25` feat(chat): GM .mute / .unmute and the base MuteTable
  2. `2e2b99f53` feat(chat): channel allowlist, the mute gate and 0xC6-0xCE feedback arms
  3. `87a07f329` docs(chat): channel allowlist, GM mutes and the 0xC6-0xCE rows
  4. `20188ccef` fix(chat): a muted player's away text and a stale GM answer (advisor review)
  5. `a3c1c0868` docs(chat): a muted player's away text is withheld
  6. this worknote
- **Owned paths (new):** `crates/base-session/src/base/mutes/` (`mod.rs`, `gm.rs`, `tests.rs`), `crates/base/src/base/dispatch/chat_gates.rs`, `crates/base/src/base/dispatch/communicator_unsupported.rs`, `crates/base/src/base/dispatch/tests/{chat_channel_mute,communicator_unsupported}.rs`, `crates/cell-console/src/cell/console/tests/ss_c3_mute.rs`, this file.
- **Edited:** `dispatch/mod.rs` (contended: `mod chat_gates; mod communicator_unsupported;`, nine `sgw_player_base` constants, one arm group), `dispatch/chat.rs` (two gate calls, `access_level` read under the existing lock, `now` passed to the tell), `dispatch/tell.rs` (the TODO replaced, `now` parameter, away reply withheld from a muted recipient), `wire/…/chat_cell_to_base.rs` (`Mute`, `Unmute`, `MAX_MUTE_MINUTES`), `base-world-entry/…/chat_dispatch.rs` (two arms), `cell-console/…/social.rs` and `registry/{mod.rs,commands/social.rs}` and `dispatch.rs` (`.mute`, `.unmute`), the `mutes` re-exports in `base/mod.rs` and `base-world-entry/base/mod.rs`, the `OTEL_FILTER` comment in `server/src/logging/filters.rs`.
- **Read set:** SS-WORKER-RULES.md; work-packets.md (contract, contended files, SS-C1, SS-C3, SS-C4); README.md D-SS14, D-SS17, D-SS26; audit.md A-27, § 6 (CAT-L-03); the organizations README D-ORG14; the SS-C1 and SS-00 worknotes; `dispatch/{mod,chat,tell}.rs`; `feedback.rs`, `gm_feedback.rs`, `gm_broadcast.rs`, `player_index/`, `rate_limit/`; `chat_cell_to_base.rs`, `chat_dispatch.rs`; the cell console (`chat/mod.rs`, `dispatch.rs`, `social.rs`, the registry, the SS-C2 tests); `Communicator.def:149-202`; `enumerations.xml:113-130`; `world_entry_chat.rs`; the negative-logging convention.

## Evidence

- **Channel ids.** `enumerations.xml:113-130`: say 0, emote 1, yell 2, team 3, squad 4, command 5, officer 6, server 8, feedback 9, tell 10, splash 11, user channels from 12; 7 is unnamed. D-ORG14 (ORG-E1 Q5): the client hardcodes these literals. The workspace constants still say server 7, tell 9, splash 10 (`crates/wire/src/cell/chat.rs:26-37`, `world_entry_chat.rs:33`).
- **Before SS-C3**, `sendPlayerCommunication` forwarded every non-tell channel to the cell. The cell answered server-7 with a system-only line and everything else with "Speaking on channel N is not supported yet!" (`cell-console/…/chat/mod.rs`). Channel 8 (the client's real server byte), 9, 11 and 12+ therefore reached the cell as player traffic.
- **0xC6-0xCE** fell into the dispatcher's catch-all: a WARN and nothing for the player (`dispatch/mod.rs`, the `_` arm).
- **Argument order (A-27).** `Communicator.def:155-202`: `chatMute`, `chatKick`, `chatOp` and `chatBan` take `UINT8 aChannelID` first; `chatMute` and `chatBan` end with `UINT8 aFlag`; `chatFriend` is `WSTRING, WSTRING, UINT8`. The dispatch-table doc had the name first and dropped the flags.

## Design decisions

### Order of the gates in `sendPlayerCommunication`

Bucket, then the channel allowlist, then the mute, then the D-SS12 text rules, then the tell branch or the cell forward. The bucket stays first (SS-00's rule), so a flood of refused lines costs tokens and gets at most one "too quickly" line. The allowlist runs before the mute so a muted player on a forbidden channel learns the channel is wrong. The mute runs before the text rules so a muted player always reads the mute line.

### The allowlist (CAT-L-03)

`chat_gates::check_channel` allows 0-6 and 10. Team, squad, command and officer are forwarded unchanged for ORG-04 / ORG-09. Everything else is refused at the base, with a feedback line and `chat.channel_rejected`:

| Ids | `reason` | Line |
|---|---|---|
| 8, 9, 11 | `system_channel` | "That channel is for system messages only. Players cannot post there." |
| 12-255 | `user_channel` | "Custom chat channels are not available yet." |
| 7 | `unknown_channel` | "That chat channel does not exist." |

The ids are local constants (`chat_gates::echannel`) citing D-ORG14, not the stale `CHAN_*`. `echannel_ids_match_enumerations_xml` parses the XML and compares every constant, as the ledger's pinning rule asks. `allowlist_matches_the_tell_route` pins `tell::TELL_CHANNEL == echannel::TELL` and the exact allowed set.

### Mutes (D-SS26)

- **Where.** `MuteTable` in `crates/base-session/src/base/mutes/`, one process-wide instance (`mute_table()`, a `LazyLock`, the crafting-sessions precedent). No `ConnectedClientState` field, as the contended-files list says.
- **Key and clock.** `player_id` → `MuteEntry { until: Instant, by_account_id }`. Every method takes `now`. Expiry is lazy: `active(pid, now)` drops an entry at or past `until`, and every insert sweeps expired entries, so the table only holds live mutes.
- **Relog.** A mute **survives relog, character select and gate travel**, because it is keyed by the character, not the session or the entity. It does **not** survive a server restart, as D-SS26 says. `mute_holds_across_relog` pins the relog half.
- **Commands.** `.mute <name> <minutes> [reason]` and `.unmute <name>` are `.`-console commands in `registry/commands/social.rs`, behind the console's GameMaster gate. The cell only parses and bounds (minutes 1 to `MAX_MUTE_MINUTES` = 10,080, which is 7 days; the reason under the chat text rules) and sends `ChatCellToBase::Mute` / `Unmute` with the GM's ids from `CellEntity`. The base resolves the name (D-SS13, online characters only), refuses a GameMaster target, writes the table and tells both sides. The duration is minutes, as D-SS26 says; the team-lead's `<duration>` is that number.
- **Who is muted.** Spatial lines, organization-channel lines and tells, from one gate before the tell branch (the SS-C1 `TODO(SS-C3)` now says so). GameMaster and above are never refused by the gate: they run the `.` console over say, and `.mute` refuses a GM target, so a GM mute would be a no-op or a trap.
- **Spec min 0.** `.mute` and `.unmute` register with `min = 0`, like `.announce`, so a bare command reaches its own usage line and logs `chat.gm_mute_refused reason=usage` instead of the generic argc refusal.
- **Field names.** The team-lead asked for `duration`; the log field is `duration_minutes` so the unit is in the name. `reason` on `chat.gm_mute` is the GM's free text, as asked, and on every `_refused` row it is the refusal code. A SigNoz query filters on `event` first, so the two never mix.

### 0xC6-0xCE

`communicator_unsupported.rs` has nine small functions, one per method, each with its own line (listed in `chat-system.md` § "Methods that are not available"). `dispatch/mod.rs` gets one arm group, `CHAT_FRIEND..=ANNOUNCE_PETITION`, which calls the file's router. Arguments are not decoded, so the `chatPassword` password is never read. A press costs a chat token: without that, each request packet would buy one reliable feedback packet.

## Security reasoning (for the reviewers)

- **Trust boundary.** The channel byte, the tell target and the text are client data; the speaker's `player_id`, `account_id`, `access_level` and entity id come from `ConnectedClientState`. The GM gate for `.mute` is the cell's `access_level` (the console's channel gate, `console::is_gm`). The base trusts `ChatCellToBase::Mute` because only the console constructs it, and nothing the client sends becomes a `CellToBaseMsg::Chat`. The base still re-checks the duration and the name.
- **No allowlist bypass.** The allowlist runs before the tell branch and before the cell forward, and both are behind it. The GM `.`-console path is channel 0, which is allowed; a non-GM's `.` line is ordinary say. Old id 7 is refused as unknown and 9 (feedback) as a system channel, so a client cannot inject a line that renders as a server or feedback line.
- **Mute coverage.** Spatial chat, the organization channels and tells, and, since the review fix, the muted player's AFK / DND auto-reply. A mute on a GM is refused, and the gate exempts GMs, so the two rules cannot disagree.
- **No amplification.** Every refusal in this packet (channel, mute, unsupported method) is after the chat bucket: five replies, then one "too quickly" line per 5 s, all to the sender only.
- **TOCTOU.** The name resolves under one lock, and the table write is by `player_id`. If the target logs off between the lookup and the write, the mute still binds their character. The GM's answer is addressed through `send_to_current_player` with the GM's `player_id`, so a GM who left in that window cannot have the line land on a recycled entity id (review fix).
- **Log injection.** The target name passes `TextField::MailRecipient` on the base before it is logged or echoed. The reason passes `TextField::ChatText` on the cell (255 units, no control, bidi or format characters).
- **Fails closed.** A poisoned mute-table lock still enforces mutes (`unwrap_or_else(into_inner)`).

### Advisor review (server-authority-enforcer)

Nothing must-fix. Acted on: the away-reply bypass (a muted player's AFK / DND text still went back to anyone who told them) and the GM answer to a recycled entity id. Guards: `muted_recipient_away_reply_withheld` and `gm_answer_not_sent_to_a_recycled_entity`. Left open, listed below: per-character scope (an alt on the same account), mail, and the tell path's fail-open when a session has no `player_id`.

## Telemetry

| Event | Level | Fields |
|---|---|---|
| `chat.channel_rejected` | WARN | `addr`, `player_id`, `account_id`, `entity_id`, `channel`, `reason` = `system_channel` \| `user_channel` \| `unknown_channel` |
| `chat.muted_refused` | DEBUG | the ids, `channel`, `tell`, `remaining_secs`, `muted_by_account_id`, `reason = muted` |
| `chat.gm_mute` | INFO | the GM's `entity_id`, `player_id`, `account_id`; `subject_player_id`, `subject_account_id`, `subject_entity_id`, `duration_minutes`, `reason` (GM text), `previous_remaining_secs` (before), `remaining_secs` (after) |
| `chat.gm_unmute` | INFO | the GM's ids, the subject's ids, `previous_remaining_secs`, `previous_remaining_minutes`, `remaining_secs = 0` |
| `chat.gm_mute_refused` | WARN | the GM's ids, `subject_player_id` (when resolved), `duration_minutes` (base), `source = console` (cell), `reason` = `usage` \| `bad_duration` \| `bad_reason` \| `bad_name` \| `not_online` \| `ambiguous` \| `target_is_gm` \| `base_channel_closed` |
| `chat.gm_unmute_refused` | WARN | as above, `reason` = `usage` \| `bad_name` \| `not_online` \| `ambiguous` \| `not_muted` |
| `chat.method_unsupported` | WARN | the ids, `method`, `payload_len`, `reason = not_implemented` |
| `chat.tell_delivered` | INFO | gains `away_reply_withheld_muted` |

All on the existing `chat` target (`chat=debug` in `OTEL_FILTER`); no new target. SigNoz queries a tester would use:

- "Why can't player X chat at T?": `target = chat AND player_id = X`, around T. `chat.muted_refused` shows the mute and `remaining_secs`; `chat.channel_rejected` shows the channel.
- "Who muted X, and for how long?": `event IN (chat.gm_mute, chat.gm_unmute) AND subject_player_id = X`.
- "What did the client press?": `event = chat.method_unsupported AND player_id = X`, grouped by `method`.

## Commands run

All through `bash tools/build-lane/lane.sh` from the worktree; exit codes as printed by the lane.

| Command | Result |
|---|---|
| `cargo check -p cimmeria-base-session --all-targets` | exit 0 |
| `cargo check -p cimmeria-cell-console -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-base-session --all-targets` | exit 0 |
| `cargo test -p cimmeria-base --lib dispatch::` | 90 passed |
| `cargo test -p cimmeria-base-session --lib -- mutes::` | 9 passed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-wire -p cimmeria-server -p cimmeria-services --all-targets -- -D warnings` | exit 0 |
| `cargo nextest run --profile=ci -p cimmeria-base -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-wire -p cimmeria-server` | 1343 passed, 0 skipped |
| `npx markdownlint-cli2` on the touched docs | no finding on an edited line (pre-existing findings at `observability.md:323,463` and `commands.md:565`) |

No live-DB run: SS-C3 has no SQL, and no test here needs a database. The ci-live-db tier is untouched.

## Regression proof

Each guard was proved by mutating the code, running the named tests, then `git checkout HEAD -- <file>` and `touch <file>`. Every named test FAILED under the mutation and passed after the restore.

| Mutation | Tests that failed |
|---|---|
| `chat.rs`: `if false && refuse_channel(..)` | `chat_rejects_system_channel_at_base`, `chat_rejects_unknown_channel_at_base` |
| `chat.rs`: `if false && refuse_if_muted(..)` | `muted_player_spatial_chat_refused_until_expiry`, `muted_player_tell_not_delivered`, `mute_holds_across_relog` |
| `chat_gates.rs`: GM exemption removed | `mute_gate_skips_gm_speakers` |
| `mutes/mod.rs`: `active` never expires | `mute_expires_on_injected_clock` |
| `dispatch/mod.rs`: the 0xC6-0xCE arm group replaced by `0x00` | the nine `*_answers_with_feedback` tests |
| `communicator_unsupported.rs`: bucket check replaced by `Allowed` | `unsupported_presses_share_the_chat_bucket` |
| `mutes/gm.rs`: `target_is_gm` check removed | `gm_mute_refusals_log_reason_and_leave_the_table_alone` |
| `registry/commands/social.rs`: `.mute` `min` 0 → 2 | `bad_mute_logs_reason_and_forwards_nothing` |
| `tell.rs`: `recipient_muted = false && ..` | `muted_recipient_away_reply_withheld` |
| `mutes/gm.rs`: `player_id` branch disabled | `gm_answer_not_sent_to_a_recycled_entity` |

## Tests added

- `base` `dispatch::tests::chat_channel_mute`: `chat_rejects_system_channel_at_base`, `chat_rejects_unknown_channel_at_base` (the ledger's names), `chat_forwards_allowlisted_channels_to_the_cell`, `allowlist_matches_the_tell_route`, `echannel_ids_match_enumerations_xml`, `muted_player_spatial_chat_refused_until_expiry` (the mute-expiry test on an injected clock, at the chat path), `muted_player_tell_not_delivered`, `mute_holds_across_relog`, `mute_gate_skips_gm_speakers`, `muted_recipient_away_reply_withheld`.
- `base` `dispatch::tests::communicator_unsupported`: one type 12 test per arm (`chat_friend_answers_with_feedback` … `announce_petition_answers_with_feedback`, each through `dispatch_sgw_player_base_method` and asserting no catch-all WARN), and `unsupported_presses_share_the_chat_bucket`.
- `base-session` `mutes::tests`: `mute_expires_on_injected_clock` (the table-level expiry test), `mute_replaces_and_reports_previous_remaining`, `mute_insert_sweeps_expired_entries`, `unmute_reports_remaining_only_for_a_live_mute`, `muted_text_rounds_minutes_up`, `gm_mute_records_logs_and_tells_both_sides`, `gm_mute_refusals_log_reason_and_leave_the_table_alone`, `gm_unmute_lifts_and_refuses_when_not_muted`, `gm_answer_not_sent_to_a_recycled_entity`.
- `cell-console` `console::social::tests`: `mute_parse_bounds_minutes_and_reason`, `unmute_parse_takes_exactly_one_name`; `console::tests::ss_c3_mute`: `mute_forwards_to_the_base_with_the_gm_ids`, `unmute_forwards_to_the_base`, `bad_mute_logs_reason_and_forwards_nothing`, `bare_unmute_logs_usage`.

The mute tests use the process-wide table, each with its own `0x7300_03xx` player id, and lift their mute before returning, so they are safe under threaded `cargo test` and under nextest.

## Known gaps

1. **An alt escapes a mute.** The table is keyed by `player_id` (D-SS26), so the muted player can switch to another character on the same account and chat. Owner call: key by `account_id` too, or record the per-character scope in D-SS26.
2. **Offline characters** cannot be muted or unmuted: the name resolves through the online index only.
3. **Mail is not muted.** D-SS26 covers chat and tells. A muted player can still send gate mail (SS-M1). Record it in D-SS26 or extend the gate to `MailOp::Send`.
4. **A session with no `player_id`** skips the mute gate (there is no character to mute), and the tell path does not require one. That is harmless as long as `player_name` and `active_player_id` are set together at world entry; an explicit fail-closed refusal for tells would be cheap.
5. **The organization channels are forwarded unchecked.** The cell still answers "not supported yet" for 3-6, so nothing leaks today. ORG-09 must check membership on the cell before it delivers anything on them.
6. **No in-client test.** The feedback lines are proven at the byte level only. SS-E1 C-Q4 (how the client renders feedback on channel 9) still applies.

## Integration edits for the coordinator

1. **ORG-09 / SS-C4 (the constant swap).** When `CHAN_*` take the `enumerations.xml` values, replace `dispatch::chat_gates::echannel` and `dispatch::tell::TELL_CHANNEL` with them (the XML-pinning test can then move to the shared constants), and delete the cell's `CHAN_SERVER` arm in `cell-console/…/chat/mod.rs`: the base now refuses 8 before the cell sees it, and 7 is refused as unknown.
2. **Contended files, in merge order:** `crates/base/src/base/dispatch/mod.rs` (two `mod` lines, nine constants after `CHAT_IGNORE`, one arm group before `LOG_OFF`); `dispatch/chat.rs` (two gate calls after the bucket, `access_level` in the lock tuple, `now` to `handle_tell`); `dispatch/tell.rs` (a `now` parameter). `ConnectedClientState` is untouched.
3. **Docs owned elsewhere, not edited:** `audit.md` § 5 (A-27 is now fixed in `sgwplayer-base-method-dispatch-table.md`), the ledger status for SS-C3, and D-SS26's text if the owner rules on gaps 1 and 3.
4. **`gap-analysis.md` totals** moved by one row (Chat mute KM → NT): NT 69, KM 126. A packet merged in between that also moves a row needs the totals recounted at merge.
