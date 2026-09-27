# SS-M4 Worknotes

> Type: reference. Audience: social-systems coordinator and the reviewing advisor.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [ss-m1.md](ss-m1.md), [ss-m2.md](ss-m2.md), [ss-m3.md](ss-m3.md), [ss-u1.md](ss-u1.md), SS-E1's [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md).

## Contract

- **Packet:** SS-M4, new-mail notification (D-SS11) and expiry (D-SS04), plus the integration edits SS-M1, SS-M2, SS-M3 and SS-U1 left for it.
- **Decisions in force:** D-SS04 (30-day TTL, three terminal paths, archived never expires, quarantine never deletes), D-SS11 (notify an online recipient), D-SS03 (server mail is exempt from the cap; quarantined mail is out of it), D-SS10 (system mail cannot be returned), and the SS-M3 coordinator decision that a paid, untaken COD is the recipient's and is never returned by expiry.
- **Base:** written on `origin/main` @ `91919e909` (SS-M1/M2/M3 and SS-U1 merged), rebased onto `cdbd5ce88` (BV-04, ORG-04, PT-13, ORG-02). Conflicts: the Mail/Bank/Squads rows of `docs/commands.md`, the startup spawns in `crates/base/src/base/service.rs` (ORG-02's audit sweep and the mail sweeper, both kept) and the server-authority-enforcer `MEMORY.md` index (both lines kept). Branch `social/m4-notify-expiry`, worktree `.claude/worktrees/ss-m4`.
- **Commits (after the rebase):** `c19589521` (the feature), `0f63f0e6e` (the security-review follow-ups), `afd486435` (docs and this worknote), `53b8f36c8` (agent memory), `4dbc3e105` (crafting bag as a source), `c3e56fb42` (its docs), `006529e47` (the reviewer's memory note), `785a4a002` (worknote), `388efc0da` (take-item placement by `container_sets`), `2f9e7f622` (its docs), then this update. Shas elsewhere in this note (`1c4aa7616`, `70bd7799b`, `8b113928e`) are the pre-rebase ones.
- **Owned paths (new):**
  - `crates/base-methods/src/base/world_entry/methods/mail/{expiry/mod.rs, expiry/terminal.rs, notify.rs}`
  - `crates/base-methods/src/base/world_entry/methods/mail/gm/{expire.rs, mailbox.rs}` (`gm.rs` became `gm/mod.rs`: at 671 lines it had three families, `.mail`, `.mailbox` and `.mail_expire`)
  - `crates/base-methods/src/base/world_entry/methods/mail/send/bucket.rs` (the mail-send bucket moved out of `send/mod.rs`, unchanged, to keep that file under 500 lines after the notify call)
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{expiry_live.rs, expiry_race.rs, quarantine_live.rs, notify_live.rs}`
  - this file.
- **Edited:**
  - Mail: `claim.rs` (`lock_mail` filters `quarantined`, reads `expires_at`), `return_.rs` (`return_locked` split out of `return_tx`; restamps `expires_at`; notifies), `read.rs` (archive clears `expires_at`; archive, delete guard, body, `mark_read` filter `quarantined`), `headers.rs` (list and one-row reads filter `quarantined`; the full list sets `ResetCategory`; `read_one` for the notifier), `send/deliver.rs` (`expires_at` at insert; the cap skips quarantined), `send/mod.rs` (notify after commit), `cod.rs` (payment mail `expires_at`; paying restarts the TTL; notify), `system/{mod.rs, write.rs}` (`expires_at` at insert; `SystemMailSent::notify`; open count skips quarantined), `mod.rs`, `tests/{mod.rs, packets.rs, gm_live.rs}`.
  - `crates/base-methods/src/base/mod.rs` (re-export `player_index`).
  - `crates/base-session/src/base/player_index/{mod.rs, tests.rs}`: `OnlinePlayerIndex::find_player(player_id)`.
  - `crates/wire/src/cell/mail/{mod.rs, tests.rs}`: `serialize_on_mail_header_info(reset_category, ...)`.
  - `crates/wire/src/cell/messages/mail_gm_cell_to_base.rs`: `MailGmCellToBase::Expire { actor, mail_id }`.
  - `crates/cell-console/src/cell/console/{mail.rs, registry/commands/social.rs, tests/ss_u1_mail.rs}`: `.mail_expire` forwards instead of refusing.
  - `crates/base/src/base/service.rs` (spawns the sweeper), `crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs` (login sweep).
  - Schema: `db/sgw/Mail/Tables/sgw_gate_mail.sql` (`expires_at`, `quarantined`), `db/sgw/_indexes.sql` (`mail_expiry_index`, partial). No migration.
  - Docs: see "Docs".
- **Read set:** `SS-WORKER-RULES.md`; work-packets.md (Contract, SS-M4, SS-U1); README D-SS03, D-SS04, D-SS10, D-SS11; audit.md §§ 1, 6; worknotes `ss-m1.md`, `ss-m2.md`, `ss-m3.md`, `ss-u1.md`, `ss-e1.md`; `mail-wire-formats.md` M-Q3, M-Q6, M-Q7; the whole `mail/` module; `player_index/mod.rs`, `feedback.rs`, `gm_broadcast.rs`; `service.rs`; `client_ready/mod.rs`; the client's `GateMail.lua`, `Access.lua`, `MinimapButtons.lua` and every UI Lua file that mentions mail.

## Evidence

- **TTL (M-Q3).** `MessageHeader` has no expiry field; the header-record constructor computes `ExpiresHours = 0x2d0 - hours` (`SGW.exe@0x00eb5ab0`). 720 h is HIGH confidence. That the countdown starts at the wire `sentTime` is MEDIUM (the decompile does not show `sentTime` reaching the time call). So `expires_at = sent_time + 720 h`, restamped whenever `sent_time` is (a return). A capture could move the anchor, never the length.
- **Expires column vs `expires_at`.** The client shows "Never" for archived mail (`GateMail.lua:136-137`), "Soon" under 2 h, hours under 48, days above (`:138-146`). The server's `expires_at` is the same instant, so the column reaches "Soon" and the sweep takes the mail within 5 minutes of zero. One deliberate divergence: paying a COD restarts `expires_at` but not `sent_time` (the header's "Sent" date stays the original), so after a payment the client's column counts down from the original send while the server gives 30 more days. The client under-states the time left, never over-states it; restamping `sent_time` would fix the display but lie about when the mail was sent. Recorded as a known gap.
- **Notification mechanism (M-Q6, settled here from client Lua).** No client method exists for new mail (audit A-13). The only Lua reader of the mailbox lists is `GateMail.lua`; its refresh (`onUpdateMailbox`, `:40-69`, on `Events.MailUpdateMailbox`, `:408`) redraws the inbox window's own rows and an open read window. No other UI Lua subscribes to a mail event, and there is no new-mail icon or sound (the minimap mail button is commented out, `MinimapButtons.lua:34`). So a one-row `onMailHeaderInfo` (`ResetCategory` 0) shows at once in an open mailbox and is invisible otherwise, and the visible cue must be a feedback line. Both are sent. What stays MEDIUM: whether the native decoder raises anything besides the refresh with the window closed; a capture would settle it. The addendum is in `mail-wire-formats.md` M-Q6.
- **`ResetCategory` (M-Q7).** When set, the client clears the list named by `bArchive` before upserting. The full header list is exactly that list (SS-M1's filter), so it now sets it; one-row refreshes and notifications leave it 0.

## Design decisions

- **One transaction per mail, SS-M3's locks.** `expiry::terminal::expire_one` locks with `claim::lock_mail` (the owner's inventory advisory locks, the mail row `FOR UPDATE`, filtered on `NOT quarantined`), then the escrow row, then (for a return) both players ascending inside `return_locked`. That is the order every attachment op uses, so a sweep racing a take, pay or return is one more op on the row, and it decides under the lock: the due check (`expires_at <= now`) and the archived check are made after the lock, so a mail archived or paid since the scan is skipped.
- **Path choice** (`terminal.rs`), with `gift_cash = cash` unless COD (then 0) and `returnable = !returned && !cod_paid && sender_id.is_some()`:
  1. no item, no gift cash, and not (an unpaid COD that is returnable): **delete**, with a guarded `DELETE` (`cash = 0 OR COD`, `NOT quarantined`, no escrow row) that must remove one row;
  2. returnable: **return** by SS-M3's `return_locked` in the same transaction (an unpaid COD cancelled, price zeroed; `returned` set; fresh `sent_time` and `expires_at`). If the sender's row is gone (a race with a character delete), quarantine with `reason = sender_gone`;
  3. otherwise: **quarantine**: `quarantined = true`, `expires_at = NULL`, any COD flag cleared and price zeroed, escrow row kept; the `UPDATE` must change one row. Reasons: `already_returned`, `cod_paid` (integration edit 1), `system_mail` (integration edit 4), `sender_gone`.
- **Integration edit 1 (paid COD).** Checked before path 1, and `return_locked` refuses `cod_paid` in its checks and its `WHERE`, so the paid COD is never returned even if the branch were lost.
- **Integration edit 3 (reuse SS-M3's return).** `return_tx` became `lock_mail` + `return_locked` + commit; the sweep calls `return_locked` inside its own transaction. The player's return path is unchanged apart from the `expires_at` restamp and the notification.
- **Integration edit 4 (system mail).** Expired server mail holding an item or cash is quarantined (it cannot be returned, D-SS10, and deleting it would destroy a payout); empty server mail is deleted. Also covers a player mail whose sender's character was deleted (`sender_id` set NULL by the foreign key).
- **A COD price is not value.** An unpaid COD with no item and nobody to return it to is deleted, not quarantined empty (review LOW).
- **Paying a COD restarts the TTL** (review MEDIUM): `clear_cod` sets `expires_at = now + 720 h`, so a COD paid on day 29, or paid while the bags are full, is not quarantined hours later.
- **Sweeps.** `SWEEP_INTERVAL` 5 min (the client shows whole hours and "Soon" under 2 h), `SWEEP_BATCH` 100 per query, `SWEEP_MAX_PER_RUN` 1,000, all `pub const` in `expiry/mod.rs`. `sweep_due` pages by keyset `(expires_at, mail_id)`, so a mail skipped or failed in one run is not re-read by it. The first tick is skipped (the login sweep covers early players). `sweep_mailbox` is the login sweep, spawned from `onClientReady` (every world entry, so gate travel too: an index scan of one mailbox). Both take `now` as a parameter (the injected clock). A partial index `mail_expiry_index (expires_at, mail_id) WHERE expires_at IS NOT NULL AND NOT quarantined` backs the scan.
- **After each commit, never inside:** an online owner gets `onMailHeaderRemove` (and a line for a quarantine), and a returned mail's new owner gets the notification.
- **Notification** (`notify.rs`): looked up by `player_id` in the online index (`OnlinePlayerIndex::find_player`, listed sessions only: in the world, not logged off), the header re-read owner-scoped after the commit (a mail rolled back, taken or moved is `MailGone`, never announced), and both sends go through `send_to_current_player`, which re-checks under the send lock that the session still plays that `player_id` (character switch, gate travel). Wired into the send (each delivered copy), the return, the COD payment mail, the expiry return, the GM `.mail`, and `SystemMailSent::notify` for other system-mail callers.
- **Integration edit 2 (quarantine filters):** `lock_mail`, the archive `UPDATE` and its follow-up, `delete_if_empty` and the `Held` follow-up, the body read, `mark_read`, both header reads, the D-SS03 cap count in `deliver.rs`, the system writer's open count, and `.mailbox`'s open and archived counts.
- **Integration edit 5 (`.mail_expire`).** The cell forwards `MailGmCellToBase::Expire { actor, mail_id }` (GM ids from the cell's entity, behind the existing `.`-console GM gate). The base sets `expires_at = now` with one conditional `UPDATE` (any mailbox; not archived, not quarantined) and then runs `expire_and_tell`, the sweep's own path, so the GM sees the result on the first press: "Mail N expired and was returned to its sender (id) with its attachments." / "... deleted (nothing was attached)." / "... quarantined (reason) ...". Refusals: `archived`, `quarantined`, `mail_not_found` (WARN `mail.gm_rejected` with `mail_id`), `db_error` (ERROR). `.mailbox` now reports quarantined mail and the next expiry.
- **Integration edit 6 (`ResetCategory`).** Done: the full list sets 1 (see Evidence). `refresh_one` and notifications send 0.
- **Integration edit 7 (mail as storage):** an owner question, below. Rules unchanged.

## Security reasoning (what if the client lies)

The client supplies nothing new: expiry is server-driven, notification is server-driven, and `.mail_expire` is GM-only. What a player controls is when they take, pay, return, archive or delete, and what they mail to whom. The server-authority-enforcer reviewed `1c4aa7616` (verdict SHIP, no path that dupes or destroys value):

- **Sweep vs take/pay/return/archive/delete:** same lock order, decisions made under the row lock, every write conditional with its row count checked. `sweep_racing_take_cash_pays_out_once` is the guard that fails without the locks (the return's `UPDATE` re-checks the owner, not the cash, so a stale read would put the taken cash back on the returned mail).
- **Paid COD:** never returned (the branch, plus `return_locked`'s own check and `WHERE`); `sweep_racing_pay_cod_never_returns_a_paid_cod`.
- **Quarantine reach:** filtered on every player path (edit 2). The escrowed item lives only in `sgw_gate_mail_item`, which no inventory query reads.
- **Steering expiry:** there is no unarchive path, so archive timing launders nothing. Self-mail returns to itself once, then quarantines. An expiry return only moves a player's own mail back to them. Postage is not refunded by expiry.
- **Notification leaks:** only listed sessions; the header re-read is owner-scoped; `send_to_current_player` re-checks the player at send time. The line names only the stored sender name, which the mail header already shows.
- **Deadlock:** the sweep holds the owner's advisory keys, then the mail row, then players ascending. Trade takes both advisory keys before any player row; send and pay lock players ascending. No cycle.
- **`.mail_expire`:** GM-gated on the cell (the existing non-GM test covers it), mail id > 0, archived and quarantined refused, `mail.gm_action` with `subject_player_id`. A GM can force-return any player's mail: within GM power, and audited.

## Owner question: mail as storage (integration edit 7)

Archived mail is exempt from the D-SS03 cap and never expires, so mailing yourself an item and archiving it is unlimited, permanent storage at 25 naquadah postage per item. It is not a dupe and gives no authority over anyone else, but it undercuts the Bank campaign's per-player vault sizes (BV-01), and the archived list's `onMailHeaderInfo` grows without bound (it is read with no `LIMIT`), which can only hurt the player's own client.

**Recommendation:** cap archived mail at archive time: refuse a new archive once a player has 100 archived mails, with a feedback line ("Your gate-mail archive is full. Delete or take from archived mail first."). Server-side only, no client patch, and it bounds the payload whatever is decided about postage. Alternatives: count archived mail with an item toward the cap, or refuse archiving a mail that still holds an item. SS-M4 did not change the rules.

## Telemetry (debuggable from SigNoz alone)

Target `mail` (already registered; `mail=debug` in `OTEL_FILTER`). No new target.

| Event | Level | Fields |
|---|---|---|
| `mail.expired` | INFO (returned, deleted) / WARN (quarantined) | `path`, `reason` (`expired`, or `already_returned` \| `cod_paid` \| `system_mail` \| `sender_gone`), `player_id` (the mailbox), `target_player_id` (its sender), `returned_to`, `mail_id`, `item_id`, `cash`, `cod_cancelled`, `expires_at`, `now`, `source` (`tick` \| `login` \| `gm`) |
| `mail.expire_skipped` | DEBUG | `player_id`, `mail_id`, `reason` (`not_found_for_owner` \| `archived` \| `not_due`), `source` |
| `mail.expire_failed` | ERROR | `player_id`, `mail_id`, `reason` (`db_error` \| `scan_db_error` \| the invariant), `error`, `source` |
| `mail.expiry_sweep` | DEBUG | `source`, `player_id` (login only), `now`, `scanned`, `returned`, `deleted`, `quarantined`, `skipped`, `failed` |
| `mail.notified` | DEBUG | `player_id` (recipient), `account_id`, `entity_id`, `mail_id`, `source_player_id`, `delivery` (`sent` \| `returned` \| `cod_payment` \| `expired_return` \| `system`), `header_pushed` |
| `mail.notify_skipped` | DEBUG | `player_id`, `account_id`, `mail_id`, `delivery`, `reason` (`offline` \| `mail_gone` \| `not_in_world`) |
| `mail.notify_failed` | WARN | `player_id`, `account_id`, `mail_id`, `delivery`, `reason = db_error`, `error` |
| `mail.gm_action action=mail_expire` | INFO | GM `account_id` / `player_id` / `entity_id`, `subject_player_id`, `mail_id`, `expires_at`, `path` |
| `mail.gm_rejected command=mail_expire` | WARN / ERROR | GM ids, `mail_id`, `reason` (`archived` \| `quarantined` \| `mail_not_found` \| `db_error`) |

The sweep has no session, so `mail.expired` carries no `account_id`; the mailbox is `player_id`. Balances do not change on expiry (a return moves the mail, not money), so `cash` and `cod_cancelled` are the before-and-after record.

SigNoz queries a tester would use:

- "What happened to mail N": `event IN ('mail.expired','mail.expire_skipped','mail.expire_failed','mail.gm_action','mail.notified') AND mail_id = N`.
- "What happened to player X's mail at time T": `target = 'mail' AND (player_id = X OR target_player_id = X OR subject_player_id = X)` around T.
- Quarantines needing a GM: `event = 'mail.expired' AND path = 'quarantined'` (WARN).
- Was the sweep running: `event = 'mail.expiry_sweep' AND source = 'tick'` (one per 5 minutes).

## Commands run

All from the worktree root through the lane.

| Command | Exit | Notes |
|---|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-base-methods -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-cell-console --all-targets` | 0 | |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-base-methods -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-wire --all-targets -- -D warnings` | 0 | |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | 0 | |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base-session -p cimmeria-cell-console mail find_player ss_u1` | 0 | 28 passed |
| revert proofs: `cargo nextest run --profile ci-live-db -p cimmeria-base-methods -p cimmeria-base-session <test>` with `DATABASE_URL=…/sgw_ss_m4`, one per mutation | 100 each (expected) | see Regression proof |
| `bash tools/build-lane/live-db-test.sh "methods::mail::"` | 0 | 124 passed before the scope addition; after it and the rebase onto `1c6bed3cc`, 125 passed; after the placement change and the rebase onto `cdbd5ce88`, 128 passed, none skipped (`DATABASE_URL=…/sgw_ss_m4`) |

## Tests

Live-DB (type 3) unless marked; sentinels `0x7300_2000`-`0x7300_24FF`, cleanup by exact account.

- `expiry_live.rs`: `every_writer_sets_expires_at`, `expired_plain_mail_is_deleted`, `expired_cod_returns_without_its_price`, `expired_gift_mail_returns_with_cash_and_item`, `expired_returned_mail_is_quarantined_not_deleted`, `expired_paid_cod_is_quarantined_not_returned`, `expired_system_mail_is_quarantined_only_with_value`, `archived_mail_never_expires`, `no_orphaned_escrow_after_sweep`, `paid_cod_restarts_its_expiry`, `expired_itemless_cod_with_no_sender_is_deleted`.
- `expiry_race.rs` (type 5, ordered gate: the take parks first, then the sweep): `sweep_racing_take_cash_pays_out_once`, `sweep_racing_take_item_moves_it_once`, `sweep_racing_pay_cod_never_returns_a_paid_cod`.
- `quarantine_live.rs`: `quarantined_mail_is_out_of_every_player_op`, `quarantined_mail_is_not_listed_or_capped` (also pins `ResetCategory` 1 on the list).
- `notify_live.rs` (type 8 against `TestTransport`, several sessions on one map): `notification_reaches_only_the_online_recipient`, `every_delivery_path_notifies_the_online_recipient`, `quarantine_tells_the_online_owner`.
- `gm_live.rs`: `gm_mail_expire_expires_the_mail_now`, `gm_mail_expire_refuses_archived_quarantined_and_unknown_mail` (type 12); `gm_mailbox_reports_counts_and_escrow` updated (quarantined count, next expiry).
- Wire (type 2): `header_info_reset_category_is_the_first_byte`.
- Unit: `find_player_sees_only_listed_sessions` (base-session).
- Cell: `mail_expire_forwards_the_mail_id_to_the_base` (replaces `mail_expire_is_refused_until_expiry_exists`; includes the type 12 `no_mail_id` refusal).

Acceptance names from work-packets.md SS-M4 are kept: `expired_cod_returns_without_its_price`, `expired_returned_mail_is_quarantined_not_deleted`, `no_orphaned_escrow_after_sweep`. The unnamed ones map to `expired_plain_mail_is_deleted`, `archived_mail_never_expires`, `sweep_racing_take_item_moves_it_once` / `sweep_racing_take_cash_pays_out_once`, `notification_reaches_only_the_online_recipient`.

## Regression proof

Each row: the mutation applied on top of `70bd7799b`, the guard run alone against `sgw_ss_m4` (`cargo nextest run --profile ci-live-db -p cimmeria-base-methods -p cimmeria-base-session <filter>`), the file restored with `git checkout HEAD --` and touched.

| Mutation | Guard | Result |
|---|---|---|
| text send binds `expires_at` NULL (`send/deliver.rs`) | `every_writer_sets_expires_at` | FAILED: `text send: expires_at … left: None` |
| return keeps an unpaid COD's price as cash (`return_.rs`) | `expired_cod_returns_without_its_price` | FAILED: `the COD price is zeroed … left: 400` |
| path 3 `DELETE`s instead of quarantining (`expiry/terminal.rs`) | `expired_returned_mail_is_quarantined_not_deleted` | FAILED: `never deleted` |
| delete path ignores the escrow row (decision and SQL) | `no_orphaned_escrow_after_sweep` | FAILED: `escrowed item … survives the sweep, left: 0` |
| a paid COD is treated as returnable (branch, `return_locked` check and `WHERE`) | `expired_paid_cod_is_quarantined_not_returned` | FAILED: `still the payer's`, mail moved to the seller |
| `lock_mail` without `NOT quarantined` | `quarantined_mail_is_out_of_every_player_op` | FAILED: TakeCash took the 80 |
| archive `UPDATE` without `NOT quarantined` | same | FAILED: archive changed 1 row |
| delete guard without `NOT quarantined` | same | FAILED: delete removed 1 row |
| cap count includes quarantined (`send/deliver.rs`) | `quarantined_mail_is_not_listed_or_capped` | FAILED: `MailboxFull` |
| header list includes quarantined (`headers.rs`) | same | FAILED: 103 headers, not 100 |
| full list sends `ResetCategory` 0 | same | FAILED: `reset left: 0` |
| archive keeps `expires_at` (`read.rs`) | `archived_mail_never_expires` | FAILED: `archiving clears the expiry, left: Some(1000000)` |
| sweep scans and expires archived rows (both scans and the under-lock check) | same | FAILED: archived row swept |
| `clear_cod` keeps the old `expires_at` (`cod.rs`) | `paid_cod_restarts_its_expiry` | FAILED: `30 days from the payment … expires_at: Some(1000000)` |
| the send path does not notify (`send/mod.rs`) | `notification_reaches_only_the_online_recipient` | FAILED: `expected the notification …, got []` |
| `find_player` matches any session with the player id, listed or not | same, and `find_player_sees_only_listed_sessions` | both FAILED (the logged-off session was notified / found) |
| `lock_mail` without the advisory lock and `FOR UPDATE` | `sweep_racing_take_cash_pays_out_once` | FAILED, but on the ordering assert (`the take committed first, left: 0`): without the locks the sweep's return committed before the parked take, and the take then found no mail. The 1,000-total double payout the doc comment describes was not observed in this run; the test fails because the ordering the lock guarantees is lost. |
| `.mail_expire` makes the mail due but does not expire it (`gm/expire.rs`) | `gm_mail_expire_expires_the_mail_now` | FAILED: mail still the owner's |

Every file was restored with `git checkout HEAD --` and touched; `git status` was clean after the run. `cod_price_kept` mutates SS-M3's zeroing, which SS-M4 relies on for path 1; the guard it proves is SS-M4's acceptance test.

## Docs

- `docs/gameplay/mail-system.md`: status line; "New-mail notification (SS-M4)" and "Expiry (SS-M4)" sections; COD payment restarts the TTL; `.mail_expire` and `.mailbox`; the status table (notification, expiry, GM tools); `ResetCategory` in the envelope; the schema block; remaining work.
- `docs/commands.md`: the Mail row.
- `docs/architecture/observability.md`: the `mail` target row (SS-M4 events, the `.mail_expire` reasons; `expiry_not_available` removed).
- `docs/reverse-engineering/findings/mail-wire-formats.md`: M-Q6 addendum (PARTIALLY RESOLVED from client Lua).

## Known gaps (for the coordinator)

1. **No GM recovery of quarantined mail.** D-SS04 says "a GM recovers it by id"; nothing does yet. `.mailbox` counts quarantined mail and `mail.expired path=quarantined` (WARN) names each. A follow-up `.mail_release <id> [to <name>]` must take `lock_mail`'s lock order and restore through the take SQL.
2. **Expires column after a COD payment** counts down from the original `sent_time` while the server allows 30 days from the payment (see Evidence). The client under-states, never over-states.
3. **M-Q6 stays MEDIUM** until a capture with the mailbox closed; the Lua side is settled.
4. **Time base MEDIUM** (M-Q3): if a capture shows the client counts from something other than `sentTime`, only `expires_at`'s anchor moves.
5. **No type 11 test.** Two-client notification is covered by the type 8 fan-out tests; the SS-UAT row should include "B is told when A's mail arrives, with the mailbox closed and open".
6. **Mail rows written before this packet** have `expires_at` NULL and never expire. None exist on the colo (its database is rebuilt from the seed on every deploy, and the seed has no mail).

## Integration edits for the coordinator

1. **SS-U3's content mail is wired** (`mail/content.rs` calls `SystemMailSent::notify` after its commit and its own clerk line; covered by `every_delivery_path_notifies_the_online_recipient`). **Black Market (BM-02b):** after committing a `send_system_mail_tx` / `send_system_mail`, call `sent.notify(pool, &FeedbackCtx { transport, connected })` beside `sent.log_sent()` so an online recipient is told. System mail now expires after 30 days; unclaimed payouts with value are quarantined, not lost.
2. **SS-U3 / UAT docs:** `.mail_expire <id>` now works (returns, deletes or quarantines at once); `.mailbox` shows quarantined mail and the next expiry.
3. **Bank campaign (cimmeria-97):** the mail-as-storage question above touches vault sizing.
4. **Follow-up ticket:** `.mail_release` (gap 1).

## Scope addition: the crafting bag (15) as a mail source

Owner decision (2026-09-27, relayed by the crafting campaign and the coordinator): crafting components (`container_sets` `{17,15}`, e.g. 5174, 5188-5192, 5228) live in bag 15 after crafting CR-16, and bag 15 is a mail attachment source.

- **Change** (`8b113928e`): `send/escrow.rs` has `MAILABLE_CONTAINERS = [INV_MAIN, INV_CRAFTING]`. `check_source` accepts either; vaults 17-20 (`item_in_vault`) and buyback 16 (`item_in_buyback`) stay refused; equipped, bandolier and mission items stay `item_not_in_main_bag`, whose feedback line now says "main bag or crafting bag".
- **Lock order:** `lock_source_item` takes `take_inventory_locks(sender, [1, 15])`, so key 0, then bag 1, then bag 15. Both bags are always locked: the container is only known after the row is read, and the advisory locks must come before it. Every other path takes the same keys in the same sorted order, so this adds no cycle. The only cost is that a send with an item also queues behind a write to the sender's crafting bag.
- **Take destination (coordinator follow-up, same day):** `take_item_tx` now places by the item type's `container_sets` with the grant rule already on `main` from crafting CR-06, `cimmeria_cell_catalog::item_placement::first_player_container`: the first carried bag listed (1 or 15), never storage, bandolier or equipment. It uses that bag's first free slot only. A full bag refuses (`bags_full` / `crafting_bag_full`, each with its own line) and keeps the escrow, never spilling into the other bag. An item whose `container_sets` names no carried bag refuses `no_carried_bag`. `claim::lock_mail` now takes both carried bags' keys (0, 1, 15), because the destination is known only after the escrow row is read; this is the same sorted order every other path takes. `tests/mod.rs` `any_type_id` now picks a backpack type: the lowest id (10) is mission-only `{2}` and would now be refused. This removes the CR-16 merge-order dependency.
- **Placement guards** (`tests/take_placement.rs`): `take_places_a_crafting_component_in_the_crafting_bag` and `take_refuses_a_full_crafting_bag_and_keeps_escrow` (100 slots filled, backpack empty). **Revert proof:** with the destination forced back to `INV_MAIN`, both FAILED: the component landed at `(owner, 1, 0, 4)`, and the full-bag take spilled into the backpack (`onUpdateItem` instead of the refusal). `take_refuses_an_item_with_no_carried_bag` (type 12, `no_carried_bag`) guards the refusal itself. `methods::mail::` in `cimmeria-base-methods`: 118 passed.
- **Guard:** `send_escrows_a_crafting_component_from_bag_15` (live-DB, in `tests/attach_vault.rs`, next to `send_rejects_banked_item`, which still covers vaults and buyback). It sends 3 of a real component type from bag 15. It checks for `Sent`, postage only, the row gone from bag 15, and the escrow row with the same instance id, type and stack. **Revert proof:** with `MAILABLE_CONTAINERS` back to `[INV_MAIN]` it FAILED: `code: Some(2)` (`ItemNotAvailable`) with the "main bag or crafting bag" line. The file was restored and touched. `attach_live` and `attach_vault` (8 tests) pass.
- **UAT check (unknown):** does the shipped client's gate-mail compose window accept an item dragged from the crafting bag? `GateMail.lua` `onSlotItemDragReceived` takes a drag, but whether the native `mailSendMessage` binding or the crafting-bag UI allows it was not traced. UAT row: "Drag a crafting component (e.g. 5188) from the crafting bag into the mail attachment slot and send it. Expect it to leave the crafting bag and arrive as an attachment. If the client refuses the drag, note it; the server path works."
- **Integration edit:** none left for the take. A return moves the mail and its escrow row, not the item, so it needs nothing.

## Close-out edits for SS-99

This packet did not edit `docs/gap-analysis.md`, `docs/project-status.md`, test counts or the crate graph.

- `gap-analysis.md` §24 (Mail): new-mail notification DONE (feedback line plus header upsert, D-SS11); expiry DONE (30-day TTL, base sweep and login sweep, return/delete/quarantine per D-SS04); `.mail_expire` DONE; open: GM recovery of quarantined mail, mail-as-storage decision.
- `project-status.md` Mail row: add "new-mail notification and 30-day expiry (SS-M4)".
- Test inventory: +27 tests (with the crafting-bag and placement guards) (21 live-DB in `cimmeria-base-methods`, 1 wire, 1 base-session; the cell test was replaced, not added) (the list above), under the 5% threshold.
- Crate graph: no new dependency edge.
