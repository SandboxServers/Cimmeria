# SS-M3 Worknotes

> Type: reference. Audience: social-systems coordinator and the reviewing advisor.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [ss-m1.md](ss-m1.md), [ss-m2.md](ss-m2.md), SS-E1's [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md).

## Contract

- **Packet:** SS-M3. Take cash (CM 49), take item (CM 50), pay COD (CM 51), return (CM 47).
- **Decisions in force:** D-SS09 (COD rules), D-SS10 (return), D-SS03 (server mail is exempt from the cap), D-SS04 (the return path is reused by SS-M4's expiry), D-SS08 (escrow). There is also an owner constraint from the Bank campaign: items leave escrow only into the backpack (`INV_MAIN`), never into vault containers 16-20.
- **Base:** `origin/main` @ `5faa795d1` (SS-M2 #912 merged as `23e97ac58` with SS-C1's mail Ignore wiring; then SS-C3, BV-02 and pets, which touch no mail file; only `observability.md` needed a re-merge). The branch was written on SS-M2's branch @ `24b4163d0` and rebased with `git rebase --onto origin/main 24b4163d0`. Branch `social/m3-take-cod-return`, worktree `.claude/worktrees/ss-m3`.
- **Commits (after the rebase):** `f272b44a1` (the `returned` column), `a814a9306` (wire and cell forward), `a8742d017` (base ops and tests), `becf1c914` (type 11), `d192c60f7` (COD with a deleted sender, and the review follow-ups), `47b372d59` (docs), `8f889a115` (worknote), `3d658dc8c` (the coordinator's decisions: `cod_paid`, and `target_player_id` on SS-M2's send refusals), `d80a46e4d` (docs), `39269c843` (the server-authority-enforcer's memory notes, carried from the main checkout as the coordinator asked), then the race and `op_failed` tests and the post-rebase docs.
- **Rebase notes:** the conflicts were the `mod` lists in `mail/tests/mod.rs` and `wireclient/tests/it/main.rs` (both sides kept), and the docs. `docs/gap-analysis.md` and `docs/project-status.md` were reset to main's version under the new owner rule (see "Close-out edits for SS-99"); `mail-system.md` and `observability.md` took main's version and my edits were reapplied on top. `read.rs` had not changed on main, so the move of the header list to `headers.rs` carried over as written.
- **Owned paths (new):**
  - `crates/base-methods/src/base/world_entry/methods/mail/{claim.rs, take.rs, cod.rs, return_.rs, headers.rs}`
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{take_live.rs, cod_live.rs, return_live.rs, take_race.rs}`
  - `crates/wireclient/tests/it/two_client_mail_cod.rs`
  - this file.
- **Edited:**
  - SS-M2 code, at the coordinator's request: `mail/send/{mod.rs, deliver.rs}` (`target_player_id` on `mail.send_refused` and `mail.attachment_refused`), `mail/tests/attach_live.rs` (the assertion).
  - Rust: `mail/{mod.rs, read.rs}` (the header list moved out of `read.rs` to `headers.rs`, no behaviour change), `mail/tests/mod.rs` (fixtures), `crates/wire/src/cell/messages/data.rs` (`MailOp` gains the four variants, per the contract), `crates/cell-interactions/src/cell/mail.rs` (`handle_attachment_op`), `crates/cell-methods/src/cell/cell_methods/mail.rs` (the four arms, plus tests), `crates/wireclient/tests/it/main.rs`.
  - Schema: `db/sgw/Mail/Tables/sgw_gate_mail.sql` (the `returned` and `cod_paid` columns).
  - Docs: those listed under "Docs".
- **Not touched:** the inventory module. `reserve_free_inventory_slots` (vendor/serializers.rs), `take_inventory_locks` and `send_full_inventory_update` are called, never changed.
- **Read set:**
  - work-packets.md: Contract, Contended files, SS-M3, SS-M4.
  - README.md: D-SS03, D-SS04, D-SS09, D-SS10.
  - audit.md § 6: CAT-G-02 to G-06.
  - `ss-m1.md` and `ss-m2.md`.
  - `mail-wire-formats.md` M-Q4, M-Q5, M-Q6, M-Q7.
  - Mail code: the SS-M2 mail code, `trade/execute/swap.rs`, `vendor/serializers.rs`, `crafting/inventory_locks.rs`, the cell's `inventory_events.rs`.
  - Schema: `db/sgw/Inventory/`, `db/sgw/Mail/`, `_foreign_keys.sql`.
  - Tests: the wireclient `support` module.

## Advisors consulted (read-only, on the committed diff)

- **server-authority-enforcer: CONDITIONAL.** It found no dupe, double credit or debit, overflow, owner-scoping hole, redirection or deadlock. Its findings:
  - **F1 (Medium): a COD whose sender's character is deleted strands its item.** The FK sets `sender_id` NULL, and after that pay, return, take and delete are all refused. **Fixed** in `f77232c05` (see Design decisions) with a guard.
  - **F2 (Low): a paid COD could still be returned**, and the seller then got both the item and the price. **Fixed** in `82010fc78` by the coordinator's decision: the `cod_paid` column (see Design decisions).
  - **F3 (Info):** nobody tells an online sender that a payment or a returned mail has arrived. That is SS-M4 (D-SS11).
- **database-persistence: no correctness bugs.**
  - It confirmed the lock order, the conditional writes, the delete-guard interplay, and that the restore copies all 14 `sgw_inventory` columns. It also confirmed that no `SELECT *` or unnamed-column `INSERT` needs the new column.
  - It flagged two things, both fixed in `f77232c05`: the COD race test could flake (a refusing take never parks behind the gate), and the restore test could not see a dropped column (the fixture used default values).
  - Optional: map a unique-slot collision (SQLSTATE 23505) to its own refusal. Not done; it rolls back safely and answers `db_error` (Known gaps 5).
  - Local dev databases need `db/database.sql` re-run for the new column. The colo rebuilds from the seed.
- **items-systems-advisor: no findings.**
  - The restore is complete, and `INV_MAIN` is symmetric with the send's `INV_MAIN`-only rule. Bound items never reach escrow.
  - A fresh row, never a merge into an existing stack, matches grant and purchase.
  - `send_full_inventory_update` matches every grant-shaped flow.
  - Skipping `InventoryItemGranted` is harmless today (its cell handler is one debug line). I added the same forward-looking comment SS-M2 put in `escrow.rs`.
- **testing-validation-engineer: yellow, then addressed** in `f77232c05`:
  - Doc comments now name the revert that proves each guard.
  - The take-cash-from-COD test asserts the exact refusal text.
  - The `cash_taken` count is filtered by `mail_id`.
  - Sentinel collision: `take_race` moved to `0x7300_1Cxx`.
  - New tests: owner-scope refusals for pay and return, and the deleted-sender guard.
  - `pay_cod_amount_read_from_row` keeps its audit name, and its doc now says what it pins. `PayCod` carries no amount, so there is nothing client-side to trust; the test pins debit == payment mail == row at pay time.
  - Not added: a lock-order deadlock test, return racing take or pay, a mailbox-full payment, and `mail.op_failed` LogCapture (Known gaps 4).

## Security reasoning (what if the client lies)

The client supplies only `mail_id`, plus `ContainerId`/`SlotId` on CM 50. The cell fills `player_id` from its own entity (`handle_attachment_op`), never from the payload. `ContainerId` and `SlotId` go into a DEBUG log line and are not parameters of `take_item_tx` at all.

| Threat | Defence | Guard |
|---|---|---|
| Act on another player's mail | `lock_mail` matches `mail_id AND character_id = caller FOR UPDATE`, and every write repeats `character_id = $2`. A miss is `not_found_for_owner`, and the stale header is removed | `take_item_never_writes_outside_callers_inventory`, `pay_and_return_refuse_another_players_mail` |
| Double click / two sessions / take-cash and take-item in one bundle | Same-owner ops queue on the caller's advisory lock, and any two ops on one mail queue on its row lock. Each re-reads the mail. Every write is conditional (`cash > 0 AND` not COD; COD flag `AND cash = $seen`; `NOT returned AND sender_id = $seen`; restore then escrow `DELETE`) with `rows_affected == 1` | `take_cash_twice_credits_once`, `take_item_twice_moves_once`, `pay_cod_twice_debits_once`, `return_rejects_already_returned`, type 5 `concurrent_take_cash_and_item_pays_out_once`, `concurrent_pay_cod_and_takes_never_pay_out_the_price` |
| Take the COD price as gift cash | Take-cash and take-item refuse while `MAIL_COD` is set. Paying clears the flag **and zeroes `cash`** in one statement. A return cancels an unpaid COD with its price zeroed | `take_cash_rejects_cod_mail`, `return_cancels_cod_and_zeroes_price`, `concurrent_pay_cod_and_takes_never_pay_out_the_price` |
| Pay a smaller price | The price is read from the locked row, and the op carries no amount | `pay_cod_amount_read_from_row` |
| Overflow the balance | `cash` is bigint and `naquadah` is integer: `i32::try_from`, then `naquadah <= i32::MAX - $1` in the credit. A refused credit rolls back the zeroing, so the cash stays in the mail | `take_cash_refuses_balance_overflow` |
| Go below zero | `naquadah >= $1` in the debit | `pay_cod_rejects_insufficient_cash` |
| Place the item into a chosen container or slot (vault, bandolier, occupied) | The destination is chosen by the server: `reserve_free_inventory_slots(caller, INV_MAIN, 1)` under the bag's advisory lock. A full bag keeps the escrow row | `take_item_ignores_client_container_and_slot`, `take_item_full_bags_keeps_escrow` |
| Redirect a return by name | The destination is the stored `sender_id`, re-checked in the `WHERE`. The new `sender_name` is the returner's stored name | `return_uses_sender_id_not_name` |
| Loop a return, or return server mail | `returned` is set on return and gates it. A NULL `sender_id` (server mail, a payment mail, a deleted sender) is `system_mail` | `return_rejects_already_returned`, `return_rejects_system_mail`, and the payment half of `paid_cod_credits_sender_once_by_mail_while_offline` |
| Orphan or duplicate escrow | Only the send inserts escrow. The take restores then deletes by `mail_id` in one transaction, and the return moves nothing (the escrow row is keyed by `mail_id`). The instance keeps its id, so a second restore would also hit the `sgw_inventory` key | `take_item_twice_moves_once`, and SS-M2's `no_orphaned_escrow_after_delete` still passes |
| Deadlock | One order for every op: the caller's advisory locks (the keys and order used by the send and crafting), then the mail row, then escrow and inventory rows, then `sgw_player` ascending. Pay and return lock both players ascending like the send, because A sending to B while B pays A's COD would otherwise cycle through the payment mail's FK lock on A's row. The send never locks an existing mail row | Reviewed by two advisors; no test (Known gaps 4) |
| A COD stranded by a deleted sender | Pay cancels it (see Design decisions) | `pay_cod_with_deleted_sender_cancels_cod_and_frees_item` |
| Pay a COD, then return it (the seller gets the item and the price) | The payment sets `cod_paid` in the same statement that clears the COD; return refuses it in Rust and in the `UPDATE` (`AND NOT cod_paid`) | `return_rejects_paid_cod` |

## Design decisions

- **One shared lock order, in `claim.rs`.** All four ops take `take_inventory_locks(caller, [INV_MAIN])` first, including take-cash and pay, which touch no inventory. One order for all four means no pair of mail ops can cycle, and take-cash and take-item on one mail from one player serialise even before the row lock.
- **Take-cash** zeroes with the audit's conditional `UPDATE` (`cash > 0 AND (flags & MAIL_COD) = 0`, `rows_affected == 1`) before it credits, and credits the amount read under the row lock. Postgres 17 has no `RETURNING OLD`, so the locked `SELECT` supplies the old value.
- **Take-item** restores the escrow row into `sgw_inventory` with its instance id and every instance column. It is a new row, never merged into an existing stack (the grant and purchase precedent). It enqueues no `InventoryItemGranted`, like SS-M2's escrow move. The client gets `send_full_inventory_update`.
- **Pay COD** delivers the price as **server mail**:
  - `sender_id` is NULL, so it cannot be returned (D-SS10), and `sender_name` is the payer's stored name.
  - The subject is `COD payment: <subject>`, cut to 128 characters. The body names the payer and the price.
  - The caller gets no mailbox-cap check (D-SS03, server mail).
  - Because it goes by mail, it reaches an offline sender.
- **A COD whose sender is gone is cancelled on pay** (`CodOutcome::CancelledSenderGone`, authority review F1):
  - Nothing is debited, the price is zeroed and the flag cleared, and the item becomes an ordinary take.
  - It logs `mail.cod_cancelled reason=sender_gone`, and the player is told.
  - **Veto point:** this gives the recipient the item for free when the seller deleted their character. The only alternative that is not a permanent sink is to let delete destroy it.
- **`cod_paid` (coordinator decision, 2026-09-27).** `sgw_gate_mail.cod_paid boolean NOT NULL DEFAULT false`, edited in place like `returned`; `returned` is not overloaded.
  - The payment sets it in the statement that clears the COD (`clear_cod(…, paid = true)`). A COD cancelled because its sender is gone is not paid, so it stays false there; that mail cannot be returned anyway (no sender).
  - Return refuses it: `reason=cod_paid`, and the feedback line tells the buyer the item is already paid for and to take it. There is no `sendMailResult` for a return (that method answers sends only), so the refusal is the feedback line, like every other SS-M3 refusal.
  - The buyer can still take the item; the seller has exactly the one payment mail.
- **`target_player_id` on SS-M2's send refusals** (coordinator request from the SS-M2 security review). `DeliverError::Refused` carries the resolved `recipient_id` (the single recipient when the attachment checks run under the lock; none for the two-players-from-one-name refusal). `mail.attachment_refused` logs it, and `mail.send_refused` goes through `refuse_about(…, target)`. A send whose single resolved recipient could not take it (a full mailbox, an Ignore) names that recipient too; with several, each has its own `mail.recipient_failed` row. Refusals before name resolution leave the field absent, never 0.
- **Return re-addresses the row.**
  - `character_id` becomes the stored sender, and `sender_id`/`sender_name` become the returner.
  - `returned = true`, `read_time = 0` (it arrives unread) and `sent_time = now` (SS-M4's TTL restarts).
  - No "Returned:" subject prefix, because the 128-character subject could overflow.
  - Return is allowed on a mail with nothing attached; D-SS10 says "any".
- **Archive refuses an unpaid COD (PR #926 security review, MEDIUM).** Before, `archive` set `MAIL_Archive` unconditionally. An archived unpaid COD could then never leave: return refuses archived mail (D-SS10), archived mail never expires (D-SS04), delete refuses `cash > 0`, and take refuses an unpaid COD. The seller's item was stranded for good. The archive `UPDATE` now carries `AND (flags & MAIL_COD) = 0`; a zero-row result on a COD mail logs WARN `mail.archive_refused reason=cod_unpaid` and answers "Pay for or return this COD delivery before archiving it." with no `onMailHeaderRemove`, so the mail stays in the inbox. A paid COD has the flag cleared and archives normally. A mail archived before this fix, if any exists, needs a GM (none can exist on the colo: the colo rebuilds from the seed and SS-M2 is new).
- **Refusal telemetry (review nits).** `mail.op_refused` carries `target_player_id`, the mail's `sender_id`, read owner-scoped after the rollback (absent for server mail and for someone else's mail). `mail.cod_cancelled` logs the stored `sender_name`. The seller's take of a payment mail cannot name the payer (a payment mail's `sender_id` is NULL by design, so it cannot be returned); the join key is `mail.cash_taken.mail_id` = `mail.cod_paid.payment_mail_id`, whose `player_id` is the payer.
- **Header refresh after a take or a payment:** `onMailHeaderRemove`, then `onMailHeaderInfo` with only that row (`headers.rs::refresh_one`). The client upserts by id and writes the attachment fields only from an attachment row (M-Q7), so the remove guarantees a fresh record. **Inference, not client-tested:** if the client's read view closes on the remove, that is a UAT item. A return and a not-found answer send only the remove.
- **Refusals** carry a result through one helper (`claim::answer_failure`):
  - a WARN `mail.op_refused` with `op` and `reason`;
  - a feedback line on the first press;
  - for `not_found_for_owner`, also a header remove.
  - Rolled-back invariant breaks and database errors log ERROR `mail.op_failed` and answer with a generic "Nothing was changed" line.
- **Cell side:** one forwarder, `handle_attachment_op`, for the four ops, with an info span `mail.attachment_op`. A payload shorter than 4 bytes is dropped with WARN `reason=truncated`; the shipped client cannot produce one.

## Telemetry (debuggable from SigNoz alone)

Target `mail` (no new target). Every row carries `account_id`, `player_id` and `entity_id`. `target_player_id` is the other player: the mail's sender, or the new owner on a return.

| Event | Level | Fields |
|---|---|---|
| `mail.cash_taken` | INFO | `mail_id`, `target_player_id`, `cash`, `naquadah_before`, `naquadah_after` |
| `mail.item_taken` | INFO | `mail_id`, `target_player_id`, `item_id`, `type_id`, `stack_size`, `container_id` (always 1), `slot_id` |
| `mail.cod_paid` | INFO | `mail_id`, `payment_mail_id`, `target_player_id`, `price`, `naquadah_before`, `naquadah_after` |
| `mail.cod_cancelled` | INFO | `mail_id`, `reason = sender_gone`, `price` |
| `mail.returned` | INFO | `mail_id`, `target_player_id`, `cash`, `cod_cancelled`, `item_id` |
| `mail.op_refused` | WARN | `op` (`take_cash` \| `take_item` \| `pay_cod` \| `return`), `mail_id`, `reason` (`not_found_for_owner` \| `cod_unpaid` \| `no_cash` \| `no_item` \| `balance_overflow` \| `bags_full` \| `not_cod` \| `not_enough_cash` \| `cod_without_item` \| `archived` \| `already_returned` \| `cod_paid` \| `system_mail`) |
| `mail.send_refused`, `mail.attachment_refused` (SS-M2's) | WARN / DEBUG | now also `target_player_id` once a single recipient was resolved |
| `mail.op_failed` | ERROR | `op`, `mail_id`, `reason` (`db_error` \| `restore_row_count` \| `escrow_delete_row_count`), `error` |
| take-item request | DEBUG | `mail_id`, `client_container_id`, `client_slot_id` (the garbage, for forensics) |
| span `mail.attachment_op` (cell), `mail.request` (base, `op = take_cash mail_id=…` etc.) | INFO | `entity_id`, `method` / `op` |

SigNoz queries a tester would use (Logs, `scope_name = 'mail'`):

- What happened to player X at time T: `attributes.player_id = X OR attributes.target_player_id = X`, around T.
- Where a mailed item went: `attributes.item_id = <id>` gives `mail.item_escrowed`, then `mail.item_taken` or `mail.returned`, with `mail_id` linking them.
- A COD's money trail: `attributes.mail_id = <cod mail>` gives `mail.cod_paid` with `payment_mail_id`. Then `attributes.mail_id = <payment_mail_id> AND attributes.event = 'mail.cash_taken'` shows the sender's credit.
- Why a button did nothing: `attributes.event = 'mail.op_refused' AND attributes.player_id = X`, grouped by `attributes.reason`.

## Commands run

All ran from the worktree root, through the lane. The exit codes are the lane's `released (exit N)`.

| Command | Result |
|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-base-methods -p cimmeria-cell-methods -p cimmeria-cell-interactions --all-targets` | exit 0 |
| `bash tools/build-lane/live-db-test.sh mail` (first run) | 95 run, 93 passed: the two race tests hit `PoolTimedOut` (the shared test pool is too small for 4 ops, the gate and the poll). Fixed with a dedicated 8-connection pool; `live-db-test.sh take_race`: 2 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-cell-methods cell_methods::mail` | 2 passed |
| `DATABASE_URL=…/sgw_ss_m3 bash tools/build-lane/lane.sh cargo test -p cimmeria-wireclient --test it two_client_mail_cod -- --test-threads=1` | 1 passed. It failed first on a wrong `MAIL_COD` constant in the test (4, not 2); rerun after `f77232c05`: 1 passed |
| `DATABASE_URL=…/sgw_ss_m3 bash tools/build-lane/lane.sh cargo nextest run --profile ci-live-db -p cimmeria-base-methods --lib mail` (after the review fixes) | 72 run, 72 passed, 0 skipped |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | exit 0 |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-wireclient --all-targets -- -D warnings` | exit 0 |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-base-world-entry` | 1,032 run, 1,032 passed (live-DB tests self-skip here; the next row runs them) |
| `bash tools/build-lane/live-db-test.sh "::"` (the whole live-DB tier, for the schema change) | exit 0: reloaded `sgw_ss_m3`, 4,303 run, 4,303 passed, 0 skipped (214 s) |
| After `82010fc78`: `bash tools/build-lane/reload-db.sh`, then `cargo nextest run --profile ci-live-db -p cimmeria-base-methods --lib mail` | 73 run, 73 passed |
| After `82010fc78`: fmt check, and clippy `-D warnings` on the same five crates | exit 0 |
| After `82010fc78`: `bash tools/build-lane/live-db-test.sh "::"` | exit 0: 4,304 run, 4,304 passed, 0 skipped (196 s) |
| After `82010fc78`: the type 11 test (command above) | 1 passed |
| After the rebase onto `23e97ac58` and the new tests: fmt check, and clippy `-D warnings` on the same five crates | exit 0 |
| After the rebase: `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-base-world-entry` | 1,162 run, 1,162 passed |
| After the rebase: `bash tools/build-lane/live-db-test.sh mail` | 106 run, 106 passed, 0 skipped |
| After the rebase: `bash tools/build-lane/live-db-test.sh "::"` | exit 0: 4,674 run, 4,674 passed, 0 skipped (223 s) |
| After the second rebase onto `5faa795d1`: fmt check, clippy on the five crates, nextest on the touched crates, `live-db-test.sh mail` | exit 0; 1,184 of 1,184; 106 of 106, 0 skipped |

## Tests

- **CAT-G-02:** `take_cash_twice_credits_once`, `take_cash_rejects_cod_mail`, plus `take_cash_refuses_balance_overflow`.
- **CAT-G-03:** `take_item_twice_moves_once` (also every instance column restored), `take_item_ignores_client_container_and_slot`, `take_item_never_writes_outside_callers_inventory`, `take_item_full_bags_keeps_escrow`.
- **CAT-G-04 (type 5):** `concurrent_take_cash_and_item_pays_out_once`, `concurrent_pay_cod_and_takes_never_pay_out_the_price`, and (`return_race.rs`) `concurrent_pay_and_return_exactly_one_wins` (either the payment or the return commits, never both; the item exists once) and `concurrent_take_cash_and_return_move_the_cash_once` (the 500 is credited or returned, not both).
- **Archive of an unpaid COD (PR #926 security review, MEDIUM):** `archive_refuses_unpaid_cod_so_it_can_still_be_returned`. Archiving is refused with feedback and no header remove, the mail stays unarchived in the inbox, the return to the seller still works (item and all), and a paid COD archives normally.
- **Type 12 `mail.op_failed`:** `take_item_db_failure_logs_op_failed_and_keeps_escrow`. A real failure, not an injected one: the escrowed id already exists in `sgw_inventory` under another character, so the restore hits the key. Feedback, ERROR `mail.op_failed reason=db_error` with the identity fields and `error`, and a full rollback.
- **CAT-G-05:** `pay_cod_twice_debits_once`, `pay_cod_rejects_insufficient_cash`, `pay_cod_amount_read_from_row`.
- **Packet acceptance:** `paid_cod_credits_sender_once_by_mail_while_offline`.
- **Also:** `pay_cod_with_deleted_sender_cancels_cod_and_frees_item`, `pay_cod_refuses_without_item`, `pay_and_return_refuse_another_players_mail`, `payment_subject_is_capped_at_the_column_width`.
- **CAT-G-06:** `return_uses_sender_id_not_name`, `return_rejects_already_returned`, `return_rejects_system_mail`, plus `return_rejects_archived`, `return_cancels_cod_and_zeroes_price` and `return_rejects_paid_cod` (the coordinator's guard: the return is refused, the seller is credited once, and the item stays for the buyer).
- **SS-M2 telemetry:** `send_rejects_bound_item` now asserts `target_player_id` on both `mail.send_refused` and `mail.attachment_refused`.
- **Type 12:** every refusal reason above is asserted through `assert_refused` (LogCapture: `event`, `op`, `reason`, `mail_id`, plus `account_id`/`player_id`/`entity_id` present) inside the live tests. The success events are asserted with their before and after values.
- **Cell:** `attachment_ops_forward_to_base`, `truncated_attachment_op_is_not_forwarded`.
- **Type 11:** `two_client_mail_cod::cod_item_round_trip_between_two_clients`. A sends B a COD item (garbage `ContainerId`/`SlotId` on the take), B pays (700) and takes it (the header comes back with no attachment, and the item is in B's `INV_MAIN`), A takes the payment (1,275 = 1,000 − 25 postage + 300). Live-DB only and not in CI (audit A-60). Run it locally with the command in its header.

Sentinels:

- accounts, players and entities: `0x7300_1800`-`0x7300_18FF` (take, COD, return) and `0x7300_1C00`-`0x7300_1C1F` (race);
- items: `0x7300_19xx`, `0x7300_1Axx` (the full-bag filler) and `0x7300_1C80`+;
- wireclient: `0x7300_1B01`/`02` (accounts), `0x7300_1B11`/`12` (players), `0x7300_1B21` (item).

Cleanup is by exact account id; the mail, escrow and inventory rows cascade.

## Regression proof

Each batch was applied by a script, then the `mail` live-DB tests were run against `sgw_ss_m3` (`cargo nextest run --profile ci-live-db -p cimmeria-base-methods --lib --no-fail-fast mail`), then the sources were restored with `git checkout`. Batches touch unrelated tests too; each named guard's failure was checked against its revert.

| Reverted | Guard that failed |
|---|---|
| R1 take-cash zeroing made a no-op (`SET cash = cash`) | `take_cash_twice_credits_once` (and the offline COD test's double credit) |
| R2 both `cash > 0` gates removed | `take_cash_twice_credits_once` |
| R3 both COD gates on take-cash removed | `take_cash_rejects_cod_mail` |
| R4 credit overflow check removed | `take_cash_refuses_balance_overflow` |
| R5 escrow `DELETE` on take made a `SELECT` | `take_item_twice_moves_once` |
| R6 slot reservation replaced by slot 0 | `take_item_full_bags_keeps_escrow`, `take_item_ignores_client_container_and_slot` |
| R6b restore into container 17 (a vault) | `take_item_ignores_client_container_and_slot`, `take_item_never_writes_outside_callers_inventory`, `take_item_twice_moves_once` |
| R7 `lock_mail` owner scope removed | `take_item_never_writes_outside_callers_inventory`, `pay_and_return_refuse_another_players_mail` |
| R8 pay: `cash` not zeroed and COD gates removed | `pay_cod_twice_debits_once` |
| R9 debit guard `naquadah >= $1` removed | `pay_cod_rejects_insufficient_cash` |
| R10 price constant 300 instead of the row | `pay_cod_amount_read_from_row` |
| R11 deleted-sender branch reverted to a refusal | `pay_cod_with_deleted_sender_cancels_cod_and_frees_item` |
| R12 both `returned` gates removed | `return_rejects_already_returned` |
| R13 a NULL `sender_id` returns to the returner | `return_rejects_system_mail` |
| R14 destination resolved by `sender_name` | `return_uses_sender_id_not_name` |
| R15 both archive gates removed | `return_rejects_archived` |
| R16 return keeps a COD price as cash | `return_cancels_cod_and_zeroes_price` |
| R17 advisory lock, mail row `FOR UPDATE` and `cash > 0` removed together | `concurrent_take_cash_and_item_pays_out_once` (2,000, not 1,500) |
| R18 restore writes `'{}'` for `ammo_types` | `take_item_twice_moves_once` |
| cell: the CM 51 arm stops forwarding | `attachment_ops_forward_to_base` |
| R19 both `cod_paid` gates on return removed | `return_rejects_paid_cod` |
| R20 the payment stops setting `cod_paid` | `return_rejects_paid_cod` |
| R21 `target_player_id` dropped from both send refusal rows | `send_rejects_bound_item` (rerun after the rebase: still fails) |
| R22 advisory lock, mail row `FOR UPDATE` and return's `AND NOT cod_paid` removed together | three runs: `concurrent_pay_and_return_exactly_one_wins` failed in 2 (the payment and the return both committed: the seller got the mail back as well as the payment), `concurrent_take_cash_and_return_move_the_cash_once` failed in 2; every run failed at least one. Each test catches the revert only when the unguarded order runs first, so neither alone is a deterministic guard; the deterministic single-layer gates are R12, R19 and R20 |
| R23 `event = "mail.op_failed"` removed from the `db_error` arm | `take_item_db_failure_logs_op_failed_and_keeps_escrow` |
| R24 the archive's `(flags & MAIL_COD) = 0` removed | `archive_refuses_unpaid_cod_so_it_can_still_be_returned` |
| R25 `target_player_id` dropped from `mail.op_refused` | `take_cash_rejects_cod_mail` |
| R26 `sender_name` dropped from `mail.cod_cancelled` | `pay_cod_with_deleted_sender_cancels_cod_and_frees_item` |

The layers back each other up, so removing any one of R17's three alone still passes the race test. The single-layer gates are pinned by R1 and R2.

## Docs

- `docs/gameplay/mail-system.md`:
  - a new "Taking attachments, paying COD, returning (SS-M3)" section;
  - the status line and implementation rows, the CM 50 row, and the persistence block (`returned`);
  - `cod_paid` in the persistence block and the return rule;
  - remaining work.
- `docs/gap-analysis.md` and `docs/project-status.md`: **not edited** (owner rule, 2026-09-27); see "Close-out edits for SS-99".
- `docs/game-systems.md` Mail section; `docs/known-issues.md` (mail moved out of the stubbed list, which was stale since SS-M1).
- `docs/protocol/cell-method-dispatch-table.md` CM 50; `docs/reverse-engineering/findings/mail-wire-formats.md` (`ContainerId`/`SlotId` now ignored, not "planned").
- `docs/architecture/observability.md`: the `mail` target row with the SS-M3 events.

## Known gaps (for the coordinator)

1. **Policy choices for the owner** (listed together):
   - **A COD whose seller deleted their character goes to the buyer for free** (approved by the coordinator 2026-09-27; see Design decisions). The only non-stranding alternative is letting delete destroy the item.
   - **SS-M2's open question:** deleting the *recipient's* character cascades their mail and destroys escrowed items and COD (Known gap 8).
2. (Resolved) The paid-COD return is closed by `cod_paid` (`82010fc78`).
3. **Header refresh is inferred from the decoder (M-Q7), not client-tested.** UAT checks:
   - after a take, the attachment icon disappears and the mail stays listed;
   - after a payment, the COD marker clears;
   - a read view that is open is not closed awkwardly by the remove-then-add.
4. **Tests not written** (the coordinator kept these as documented gaps):
   - the lock-order deadlock scenario (A sends B while B pays A's COD);
   - a payment into a sender's full mailbox (the cap exemption);
   - a revert proof for the type 11 test (it is not a CI guard).

   Return racing pay or take, and the `mail.op_failed` LogCapture, are now written (see Tests). The two return-race tests each catch their revert only when the unguarded interleaving happens to run (see Regression proof).
5. **A unique-slot collision answers `db_error`.** This happens only against an `INV_MAIN` writer that skips the bag's advisory lock. It rolls back and the item stays in escrow, but the player sees the generic line.
6. **An online sender is not told when a payment or a returned mail arrives** (D-SS11, SS-M4); they see it on the next header request.
7. **The type 11 test hard-codes 25 postage** (1,275). It must change if D-SS02 changes.
8. **SS-M2's Known gap 2 still stands:** deleting the *recipient's* character cascades their mail and destroys escrowed items.

## Integration edits for the coordinator

1. **Rebase:** done, onto `23e97ac58`. No gap-analysis or project-status edits are left to conflict.
2. **work-packets.md contract:**
   - `MailOp::TakeItem` keeps `container_id` and `slot_id` as the contract says, for the log only.
   - SS-M4 should call `return_::return_tx(pool, owner, mail_id, now)` for expiry path 1. It already cancels an unpaid COD with its price zeroed, and refuses returned, archived, paid-COD and system mail.
   - **`lock_mail` must filter `quarantined`** once SS-M4 adds that column (`AND NOT quarantined` in `claim.rs::lock_mail`), so a quarantined mail can be neither taken, paid nor returned by its owner; only the GM recovery path touches it. The archive `UPDATE` (`read.rs::archive_unless_cod`) and the delete guard need the same filter.
   - **SS-M4 integration edit (coordinator decision):** a paid, untaken COD (`cod_paid = true`, escrow row present) belongs to its recipient. Expiry must **never** return it: it takes the quarantine path (D-SS04 path 3), like an already-returned mail that still holds an item. `return_tx` refuses it with `cod_paid`, so the sweep must branch on `cod_paid` (or on that refusal) before choosing path 1. SS-M4's quarantine and cap queries read both `returned` and `cod_paid`.
3. **Local databases need `db/database.sql` re-run** for the new column (`reload-db.sh` does it). The colo rebuilds from the seed on deploy.
4. **Advisor memory:** the server-authority-enforcer wrote its notes into the main checkout. The coordinator's patch is applied in this branch (`39269c843`); the coordinator reverts the main checkout's copies.
5. **SS-U1 compatibility (confirmed with ss-u1 by message):** system mail has `sender_id` NULL and is not returnable; nothing routes by `source_character_id`; a GM COD (`sender_id` = the GM) pays like a player COD; system cash up to `i32::MAX` meets the take-cash overflow check. The shared files are only `mod` lines in `mail/mod.rs` and `tests/mod.rs`.

## Close-out edits for SS-99

Per the owner rule of 2026-09-27, these status changes are recorded here for SS-99 instead of being edited into `docs/gap-analysis.md` and `docs/project-status.md`:

- **`docs/gap-analysis.md` §24 (Mail):**
  - Cash on Delivery **IM → NT**: pay (CM 51) debits the stored price, clears the COD with its price zeroed, sets `cod_paid`, and mails the price to the sender; a COD whose sender was deleted is cancelled on pay. Code `mail/cod.rs`. Tests `pay_cod_twice_debits_once`, `pay_cod_rejects_insufficient_cash`, `pay_cod_amount_read_from_row`, `paid_cod_credits_sender_once_by_mail_while_offline`; type 11 `cod_item_round_trip_between_two_clients`.
  - Take item from mail **KM → NT** (`mail/take.rs`; first free main-bag slot chosen by the server, client container and slot ignored, a full bag keeps the escrow row).
  - Take cash from mail **KM → NT** (`mail/take.rs`; once, never from an unpaid COD, overflow-checked; type 5 `concurrent_take_cash_and_item_pays_out_once`).
  - Return to sender **KM → NT** (`mail/return_.rs`; to the stored `sender_id`, once; never archived, server mail or a paid COD).
  - The §24 heading, confidence (SS-M3 plus one two-client wireclient run), code list (`headers.rs`, `claim.rs` with `take.rs`, `cod.rs`, `return_.rs`), recent PRs and path forward (SS-M4 only).
  - Matrix row for Mail: NT +4, IM −1, KM −3 relative to whatever main holds at close-out; recompute the totals and percentages from the rows.
- **`docs/project-status.md` Mail row:** taking cash and items, paying COD (the price reaches the sender by mail, online or not) and return-to-sender landed with SS-M3, not yet client-tested; the row counts move as above.
