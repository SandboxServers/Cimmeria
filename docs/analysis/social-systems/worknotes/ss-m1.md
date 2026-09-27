# SS-M1 Worknotes

> Type: reference. Audience: social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [ss-00.md](ss-00.md), SS-E1's [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md) (read from `origin/social/se1-re`, PR #875).

## Contract

- **Packet:** SS-M1, plain mail send and read-side fixes.
- **Decisions in force:** D-SS03 (100-message cap, player mail only), D-SS05 (10 recipients, one with any attachment), D-SS07 (aliases refused, one seam), D-SS12 (text rules), D-SS13 (exact, then a unique case fold, against `sgw_player`), D-SS14 (mail bucket burst 3, 1 per 10 s), D-SS15 (Ignore: a seam until SS-C1), and D-SS06's lock shape (sender and every recipient `FOR UPDATE`, ascending `player_id`).
- **Base:** `origin/main` @ `88d7da73a` (SS-00 merged as #880). Branch `social/m1-plain-send`, worktree `.claude/worktrees/ss-m1`.
- **Commits:** `93ac923db` (split, no behaviour change), `a1c57ab13` (wire, cell and base send path, read-side fixes, schema, telemetry, tests), then the docs, worknote and agent-memory commits.
- **Owned paths (new or split):**
  - `crates/base-methods/src/base/world_entry/methods/mail/` → `mod.rs` (router, `Caller`, `MailCtx`), `read.rs`, `send/{mod.rs, deliver.rs, recipients.rs, tests.rs}`, `tests/{mod.rs, packets.rs, read.rs, read_scoping.rs, send_limits.rs, send_live.rs, send_race.rs}` (the old `tests.rs` is `tests/read.rs`, fixtures moved to `tests/mod.rs`)
  - `crates/wire/src/cell/mail.rs` → `cell/mail/{mod.rs, codes.rs, send_result.rs, tests.rs}`
  - `crates/wire/src/cell/cell_methods/mail.rs` → `cell_methods/mail/{mod.rs, send.rs, tests.rs}`
  - `crates/wire/src/cell/messages/data.rs` (`MailOp`, `MailSend`, `MailSendReject`)
  - this file
- **Edited:** `crates/entity/src/organization/{org_text.rs, limits.rs, tests.rs}` (three new `TextField` variants and their caps; ORG-01's file, additive as SS-00 did), `crates/wire/src/cell/messages/mod.rs` (re-exports), `crates/cell-interactions/src/cell/mail.rs` (`handle_send_mail`), `crates/cell-methods/src/cell/cell_methods/mail.rs` (CM 44 arm), `crates/base-methods/src/base/mod.rs` (re-export `feedback`, `rate_limit`), `crates/server/src/logging/{filters.rs, target_scan_tests.rs}`, `db/sgw/Mail/Tables/sgw_gate_mail.sql`, and the docs under "Docs".
- **Read set:** work-packets (Contract, SS-M1, contended list), README decisions D-SS03/05/07/12/13/14/15 (and D-SS06), audit A-06, A-08, A-09, A-10, A-12 and § 6; SS-E1's `mail-wire-formats.md` and `worknotes/ss-e1.md` from `origin/social/se1-re`; `ss-00.md`; `base-session` `rate_limit/`, `feedback.rs`, `player_index/`; `base/dispatch/chat.rs` (the caller pattern); `entity/src/organization/org_text.rs`; `wire/.../organization/reader.rs`; `SGWMailManager.def`; `enumerations.xml` 1046-1077; `db/sgw/Mail/`, `db/sgw/_indexes.sql`, `_foreign_keys.sql`.

## SS-E1 answers used

| Question | Used as |
|---|---|
| M-Q1 (result text) | `MailResult` doc comments; `FailedRecipients` is sent on `Sent` too, because the client shows it there |
| M-Q2 (aliases, `ItemId`) | `RecipientFlags` and `Recipients` never overlap, so the flags gate runs before name resolution; the client's own cap is 51 tokens, the server's 10 is stricter. `ItemId` = instance id is SS-M2's |
| M-Q7 (`bArchive`) | The header query filters by the requested list. SS-E1 says the client files rows by their own flag but resets only the requested list, so the filter is the fix |
| M-Q5, M-Q3, M-Q4 | Not used by SS-M1 (SS-M3, SS-M4, SS-M2) |

**Wire-order note.** SS-E1's M-Q1 text lists the handler decoding `ResultCode`, `FailedRecipientFlags`, then `FailedRecipients`. The serializer follows `SGWMailManager.def:43-47` (`ResultCode`, `FailedRecipients`, `FailedRecipientFlags`), as does SS-E1's own table in `mail-wire-formats.md`: the order a UI handler reads named event args in is not the stream order. A capture of any `sendMailResult` settles it; `send_mail_result_bytes_are_def_ordered` pins the `.def` order. **Open question (coordinator verifying in Ghidra, CodeRabbit on #875):** raw-stream order or field access on a decoded struct. The order is behind one constant, `cimmeria_wire::cell::mail::SEND_MAIL_RESULT_FLAGS_BEFORE_NAMES` (in `send_result.rs`) (`false` = `.def` order); a flip is that constant plus the two byte-exact tests' expected bytes.

## Design decisions

- **Decode refusals are forwarded, not answered on the cell.** The cell sends `MailOp::SendRejected(MailSendReject)` (a variant beyond the contract's `Send(MailSend)`; additive, no rename). Reason: D-SS14 puts every limit on the base, and a refusal must cost a token like the chat path (SS-00: "the bucket runs first, so a refused line also costs a token"). It also keeps every `sendMailResult` in one file. Guard: `decode_refusals_are_charged_to_the_bucket`.
- **Result codes for refusals with no code of their own.** Text-rule, malformed and too-many-recipient refusals, unknown alias bits, no DB pool and a rolled-back transaction answer `MAILRESULT_NoRecipients` ("… Gate-mail message was not sent.") plus a feedback line with the real reason. The alternative was a value outside 0-7, which the client shows as "Unknown mail error." (M-Q1's default branch); I kept the wire inside the enum. Coordinator may veto.
- **Attachments until SS-M2.** Two or more distinct recipients (case-folded) with any attachment → `AttachmentsAndMultipleRecipients` (3), per D-SS05 and the client's own rule. A single recipient with any attachment field set (cash of either sign, `bCOD`, `ItemId`, `ItemQuantity`) → `ItemNotAvailable` (2) plus "Gate-mail attachments (naquadah, items and COD) are not available yet. Send the message without them." SS-M2 replaces that branch (`TODO(SS-M2)` in `send/mod.rs`).
- **Aliases (D-SS07).** `send::resolve_recipient_flags` is the one seam. Any bit refuses the whole send with `NoRecipients` and `FailedRecipientFlags` = the offending bits: vault bits (`vault_alias_unsupported`), organization bits (`organization_alias_unsupported`), anything else (`unknown_recipient_flags`, including `MAIL_Archive`/`MAIL_COD`, which are header flags, never aliases). Because `MAIL_ToCommandRank6 = 4092` contains bit 2 (`MAIL_ToVault`), a Rank6 alias classifies as vault; harmless while both are refused, but the Bank campaign's replacement must test bits in the order its owner decides (A-12).
- **Sender name from the locked row.** The contract says "sender_id and sender_name from the session". The cell's `player_id` is the session's server-side id; the name is read from that `sgw_player` row inside the transaction (`FOR UPDATE`), not from `ConnectedClientState::player_name`, so a stale or wrong session name can never be stored. `send_delivers_to_offline_recipient` gives the session a different name and asserts the stored one.
- **Name resolution** is one query (`player_name = ANY($1) OR lower(player_name) = ANY($2)`) and a pure resolver (`recipients::resolve_names`) with the same rule as `OnlinePlayerIndex::lookup`. Repeats collapse by `player_id` (first typed form wins); failures keep the typed form for `FailedRecipients`.
- **Lock and cap.** One statement locks sender + recipients `ORDER BY player_id FOR UPDATE`; the open-mail count (`flags & MAIL_Archive = 0`) runs after it. `quarantined` does not exist yet (SS-M4 adds it and must add `AND NOT quarantined` to this count, `send/deliver.rs`).
- **Ignore seam.** `recipients::ignoring_sender` returns an empty set; the send path already turns any returned id into `FailReason::Ignoring` ("not accepting your messages"). SS-C1 fills the body.
- **Read side.** `request_headers` filters `(flags & 1) = $3`. `mark_read` is the owner-scoped update, split out so the guard can call it without the owner-scoped SELECT in front masking a regression. `request_body` joins `sgw_player` for `ToText`; the session-name lookup is gone. `ResetCategory` stays 0 (see Known gaps).
- **Schema.** `CHECK (cash >= 0)` added as `sgw_gate_mail_cash_nonnegative_chk`. The `(character_id)` index the contract lists already exists (`mail_lookup_index`, `db/sgw/_indexes.sql`), so none was added.
- **Text fields.** `TextField::MailSubject` 1-128, `MailBody` 0-1000 with `\n` allowed, `MailRecipient` 1-64 (the `player_name` column width; D-SS12 names no recipient cap, but the names are echoed back in `FailedRecipients` and logged nowhere, so they are bounded like any other text).

## Telemetry (debuggable from SigNoz alone)

Target `mail` (raised from `info` to `debug` in `OTEL_FILTER`, pinned INFO/WARN/DEBUG in `scan_finds_known_targets`, row added to the observability target catalog). No event carries the subject, the body or a typed name.

| Event | Level | Where | Fields |
|---|---|---|---|
| `mail.sent` | INFO | base `send/mod.rs` | `entity_id`, `player_id`, `account_id`, `target_player_ids`, `mail_ids`, `delivered`, `failed`, `subject_units`, `body_units`, `result` |
| `mail.send_refused` | WARN | base `send/mod.rs::refuse` | `entity_id`, `player_id`, `account_id`, `reason`, `result`, `failed_recipients`, `failed_flags` |
| `mail.recipient_failed` | DEBUG | base `send/mod.rs` | `entity_id`, `player_id`, `account_id`, `target_player_id` (absent for an unknown name), `reason` |
| `mail.attachment_seen` | DEBUG | base `send/mod.rs` | ids plus `cash`, `cod`, `item_id`, `item_quantity`, `recipients` |
| `rate_limit.exceeded category=mail_send` | WARN / DEBUG | `rate_limit::log_exceeded` (SS-00) | bucket state, `player_id`, `account_id`, `entity_id` |
| `mail.send_decoded` / `mail.send_decode_rejected` | DEBUG | cell `cell/mail.rs::handle_send_mail` | `entity_id`, `player_id`, sizes / `reason`, `detail` |
| `mail.headers_sent` | DEBUG | base `read.rs` | `entity_id`, `player_id`, `b_archive`, `count` |
| read misses | WARN | base `read.rs` | `reason = not_found_for_owner`, `mail_id` |
| delivery failure | ERROR | base `send/mod.rs` | `reason = no_db_pool \| sender_missing \| db_error`, `error` |

Spans: the cell's `mail.send` (info) and the base's existing `mail.request` (info), whose `op` field is now a short `op_name` (never the send's text).

SigNoz queries a tester would use (Logs, `scope_name` is the target):

- every send by a character: `scope_name = 'mail' AND attributes.player_id = <id>`;
- refusals and why: `scope_name = 'mail' AND attributes.event = 'mail.send_refused'`, group by `attributes.reason`;
- flood drops: `scope_name = 'rate_limit' AND attributes.category = 'mail_send'`;
- a delivered mail: `attributes.event = 'mail.sent'` and search `attributes.mail_ids` for the id.

## Evidence and hypotheses

- A-08/A-09/A-10 reproduced as written against `read.rs` before the fix.
- A-11's "no foreign keys" is stale: `db/sgw/_foreign_keys.sql` has `sgw_gate_mail_character_id_fkey` (`ON DELETE CASCADE`) and `sgw_gate_mail_sender_id_fkey` (`ON DELETE SET NULL`). The `ToText` join relies on the first.
- Hypothesis, untested in the client: a mail body typed with Enter carries `\n`, not `\r\n` (CEGUI multi-line edit boxes use `\n`). If the client sends `\r`, every multi-line body is refused with "contains a character that cannot be sent"; the `mail.send_refused reason=control_char` rows would show it at once.

## Commands run

All from the worktree root, through the lane. Exit codes are the lane's `released (exit N)`.

| Command | Result |
|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-base-methods --all-targets` (after the split) | exit 0 |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-wire mail` | 16 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-entity organization` | 28 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-base-methods mail` | 32 passed (live-DB tests self-skipped here; the next row runs them) |
| `bash tools/build-lane/live-db-test.sh mail` | reload into `sgw_ss_m1`, 55 tests run, 55 passed, none skipped |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-server logging` | 52 passed (OTEL_FILTER parity and target scan) |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-wire -p cimmeria-entity -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-base-world-entry` | all passed (216 + 124 + 124 + 233 + 341 + 5 + 221) after touching the sources (see Regression proof) |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | exit 0 |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-entity -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-server --all-targets -- -D warnings` | exit 0 |
| `bash tools/build-lane/live-db-test.sh "::"` (the whole live-DB tier) | exit 0: 3,745 tests run, 3,745 passed, 0 skipped (reloaded `sgw_ss_m1`, 159 s) |
| `pwsh tools/lint-md.ps1` on the touched docs | no findings on the edited lines (pre-existing findings elsewhere in the same files) |

## Tests (names as the audit asks, plus the packet's)

- **Type 2 (wire):** `send_mail_result_bytes_are_def_ordered`, `send_mail_result_bytes_with_flags_and_no_names`; enum pins `mail_flags_match_enumerations_xml`, `mail_result_codes_match_enumerations_xml` (both parse `enumerations.xml`); the CM 44 decoder tests (`recipient_cap_is_ten`, `forged_recipient_count_is_refused_before_reading_names`, `forged_wstring_length_is_malformed_not_an_allocation`, `text_rules_apply_to_every_string`, `lone_surrogate_is_a_text_rejection_on_its_field`, …).
- **Type 12 (no DB):** `mail_send_bucket_rejects_fourth_in_burst`, `decode_refusals_are_charged_to_the_bucket`, `send_rejects_eleven_recipients`, `send_rejects_attachment_with_two_recipients`, `send_rejects_negative_cash`, `send_refuses_every_attachment_until_ss_m2`, `send_rejects_mail_aliases`, `text_refusals_explain_themselves`.
- **Type 3 (live DB):** `send_delivers_to_offline_recipient`, `send_partial_failure_delivers_to_the_rest`, `send_rejects_unknown_recipient`, `send_resolves_names_per_d_ss13_and_dedupes`, `send_delivers_to_ten_recipients`, `send_refuses_full_mailbox`, `schema_rejects_negative_mail_cash`, `request_headers_archive_filter_returns_requested_category`, `read_time_update_scoped_to_owner`, `mail_read_to_text_is_stored_recipient`.
- **Type 5:** `concurrent_sends_respect_mailbox_cap`, forced: the test holds `LOCK TABLE sgw_gate_mail IN SHARE MODE` (reads pass, inserts block) and releases once `pg_stat_activity` shows two lock waiters, so without the row lock both sends have counted 99 before either inserts.
- **Cell forward:** `send_mail_forwards_decoded_send`, `send_mail_forwards_decode_refusal`.

Sentinels: `0x7300_1001`-`0x7300_13xx` (accounts, players, entities), cleaned by exact account id. The old read tests keep `0x7000_05xx`.

## Regression proof

Each guard was run with its fix reverted in place (a scripted edit, test run, restore), against `sgw_ss_m1`:

| Reverted | Guard | Result |
|---|---|---|
| mail bucket check → always `Allowed` | `mail_send_bucket_rejects_fourth_in_burst` | FAILED |
| `ORDER BY player_id FOR UPDATE` → no `FOR UPDATE` | `concurrent_sends_respect_mailbox_cap` | FAILED (box at 101, not 100) |
| header filter removed | `request_headers_archive_filter_returns_requested_category` | FAILED (inbox got both ids) |
| `AND character_id = $3` removed from `mark_read` | `read_time_update_scoped_to_owner` | FAILED (1 row changed, not 0) |
| `ToText` from the session name | `mail_read_to_text_is_stored_recipient` | FAILED (`SessionImpostor`) |
| `MAX_MAIL_RECIPIENTS` 10 → 11 | `send_rejects_eleven_recipients` | FAILED |
| mailbox cap disabled | `send_refuses_full_mailbox` | FAILED |
| multi-recipient attachment check disabled | `send_rejects_attachment_with_two_recipients` | FAILED |
| attachment refusal disabled | `send_rejects_negative_cash` | FAILED |
| alias seam always `Ok` | `send_rejects_mail_aliases` | FAILED |

The first version of the race test (a plain `tokio::join!`) passed with the lock removed, so it was not a guard; the `SHARE` gate replaced it. One trap hit during the proofs: restoring with `mv f.bak f` restores an older mtime, so cargo kept the mutated `cimmeria-wire` build and a later multi-crate run failed `send_rejects_eleven_recipients` on correct source. Every source was touched and the suite rerun green; the proof script now touches after restoring (recorded in agent memory).

## Docs

- `docs/gameplay/mail-system.md`: status, a "Sending a text mail (SS-M1)" section, implementation-status rows. Edited lines do not overlap PR #875's one-line change (the `takeItemFromMailMessage` row).
- `docs/game-systems.md` Mail section; `docs/gap-analysis.md` §24 (heading, prose, every row, and the Summary Completion Matrix Mail row: 13 = 0 CW, 5 NT, 0 IM, 7 KM, 1 NU); `docs/project-status.md` Mail row.
- `docs/architecture/observability.md`: `mail` target row.
- **Not edited:** `mail-wire-formats.md`. The packet asked SS-M1 to correct its result codes and `MessageAttachment` (A-06, A-12); PR #875 already did both (the M-Q1 table and the M-Q4 layout), so there is nothing left that would not conflict with it.

## Known gaps

- No client test yet: every row above is NT until a two-client UAT. No wireclient (type 11) session test was written for mail; SS-M4 or SS-U3 should add the "A sends, B reads" session.
- `ResetCategory` in `onMailHeaderInfo` is still always 0. With the filter the reply is the whole requested list, so 1 would let the client drop rows deleted elsewhere (SS-E1 M-Q7). Not changed here: it is a client-visible behaviour change outside the audit rows; SS-M4 (which deletes and expires mail server-side) is the natural owner.
- The recipient gets no notification (SS-M4, D-SS11); the mail appears on the next mailbox open.
- `FailedRecipients` echoes typed names; a name that fails the text rules refuses the whole send (it never reaches `FailedRecipients`).
- The Ignore seam answers "nobody" until SS-C1.

## Contended files touched

- `crates/wire/src/cell/messages/data.rs` (`MailOp`): this packet's for the wave; SS-M3 is next. `MailOp::SendRejected` is new beyond the contract text.
- `crates/entity/src/organization/{org_text.rs, limits.rs, tests.rs}` (ORG-01's): three additive `TextField` variants and three constants, the same shape as SS-00's `ChatText`.
- `crates/wire/src/cell/cell_methods/organization/mod.rs`: untouched in the end (the mail decoder uses the existing `pub(crate) use reader::ArgReader`).
- `crates/server/src/logging/{filters.rs, target_scan_tests.rs}`: one `OTEL_FILTER` change (`mail=info` in the vendor row removed, `mail=debug` added after `chat`) and three pins. Any other packet editing those rows merges textually.

## Integration edits for the coordinator

1. `docs/gap-analysis.md` totals: the Mail row moved 3 NT up, 2 IM and 1 KM down. The TOTALS line, the percentage table and the dated history row (`gap-analysis.md` around 1297-1323) and the headline counts in `project-status.md` are not updated here, because every packet in flight edits them; reconcile at merge (the campaign's SS-99).
2. `work-packets.md` contract: record `MailOp::SendRejected(MailSendReject)` next to `Send(MailSend)`, and that `send::resolve_recipient_flags` lives at `mail/send/mod.rs`.
3. `audit.md` A-11: the "no foreign keys" claim is stale (see Evidence).
4. Tell SS-M4: add `AND NOT quarantined` to the cap count in `send/deliver.rs` when the column lands.
5. **SS-C1 swap (D-SS15).** The seam is `send::recipients::ignoring_sender(conn, sender_id, recipient_ids) -> HashSet<i32>` in `mail/send/recipients.rs`, called in `send/deliver.rs` before the lock. Once SS-C1 lands `cimmeria_base_session::base::contact_list::ignore`, its body becomes a loop over `recipient_ids` calling `player_ignores(pool, recipient_player_id, sender_name)` (the offline-capable DB check); this needs the sender's stored name, which `deliver` reads under the lock, so either pass it in or move the call after the lock. The refusal text can switch to SS-C1's public `not_accepting_text`. `session_ignores` is not needed for mail.
6. **Duplicate resolver.** `send::recipients::resolve_names` + `candidate_rows` implement D-SS13 against `sgw_player`, the same job as SS-C1's `resolve_character(pool, typed) -> CharacterLookup`. Mine resolves up to 10 names in one query inside the send transaction; drop one of the two at merge (keeping mine's single-query batch, or calling `resolve_character` per name, is the coordinator's call).
7. Tell the Bank campaign (cimmeria-97): the vault seam is `resolve_recipient_flags` in `crates/base-methods/src/base/world_entry/methods/mail/send/mod.rs`, and 4092 (`MAIL_ToCommandRank6`) contains the vault bit.
