# SS-M3 Worknotes

> Type: reference. Audience: social-systems coordinator and the reviewing advisor.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [ss-m1.md](ss-m1.md), [ss-m2.md](ss-m2.md), SS-E1's [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md).

## Contract

- **Packet:** SS-M3. Take cash (CM 49), take item (CM 50), pay COD (CM 51), return (CM 47).
- **Decisions in force:** D-SS09 (COD rules), D-SS10 (return), D-SS03 (server mail is exempt from the cap), D-SS04 (the return path is reused by SS-M4's expiry), D-SS08 (escrow). There is also an owner constraint from the Bank campaign: items leave escrow only into the backpack (`INV_MAIN`), never into vault containers 16-20.
- **Base:** SS-M2's branch `origin/social/m2-attachments` @ `24b4163d0` (PR #912, not yet on main). Branch `social/m3-take-cod-return`, worktree `.claude/worktrees/ss-m3`. When #912 merges: `git rebase --onto origin/main 24b4163d0`.
- **Commits:** `1ce41f51c` (schema), `64b34fa6f` (wire and cell forward), `bb8a01c7f` (base ops and tests), `87aa17d9d` (type 11), `f77232c05` (COD with a deleted sender, and the review follow-ups), `9089471a0` (docs), then this worknote.
- **Owned paths (new):**
  - `crates/base-methods/src/base/world_entry/methods/mail/{claim.rs, take.rs, cod.rs, return_.rs, headers.rs}`
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{take_live.rs, cod_live.rs, return_live.rs, take_race.rs}`
  - `crates/wireclient/tests/it/two_client_mail_cod.rs`
  - this file.
- **Edited:**
  - Rust: `mail/{mod.rs, read.rs}` (the header list moved out of `read.rs` to `headers.rs`, no behaviour change), `mail/tests/mod.rs` (fixtures), `crates/wire/src/cell/messages/data.rs` (`MailOp` gains the four variants, per the contract), `crates/cell-interactions/src/cell/mail.rs` (`handle_attachment_op`), `crates/cell-methods/src/cell/cell_methods/mail.rs` (the four arms, plus tests), `crates/wireclient/tests/it/main.rs`.
  - Schema: `db/sgw/Mail/Tables/sgw_gate_mail.sql` (the `returned` column).
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
  - **F2 (Low): a paid COD can still be returned**, and the seller then gets both the item and the price. **Open.** I sent the coordinator a decision request (Known gaps 1).
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
- **Return re-addresses the row.**
  - `character_id` becomes the stored sender, and `sender_id`/`sender_name` become the returner.
  - `returned = true`, `read_time = 0` (it arrives unread) and `sent_time = now` (SS-M4's TTL restarts).
  - No "Returned:" subject prefix, because the 128-character subject could overflow.
  - Return is allowed on a mail with nothing attached; D-SS10 says "any".
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
| `mail.op_refused` | WARN | `op` (`take_cash` \| `take_item` \| `pay_cod` \| `return`), `mail_id`, `reason` (`not_found_for_owner` \| `cod_unpaid` \| `no_cash` \| `no_item` \| `balance_overflow` \| `bags_full` \| `not_cod` \| `not_enough_cash` \| `cod_without_item` \| `archived` \| `already_returned` \| `system_mail`) |
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

## Tests

- **CAT-G-02:** `take_cash_twice_credits_once`, `take_cash_rejects_cod_mail`, plus `take_cash_refuses_balance_overflow`.
- **CAT-G-03:** `take_item_twice_moves_once` (also every instance column restored), `take_item_ignores_client_container_and_slot`, `take_item_never_writes_outside_callers_inventory`, `take_item_full_bags_keeps_escrow`.
- **CAT-G-04 (type 5):** `concurrent_take_cash_and_item_pays_out_once`, `concurrent_pay_cod_and_takes_never_pay_out_the_price`.
- **CAT-G-05:** `pay_cod_twice_debits_once`, `pay_cod_rejects_insufficient_cash`, `pay_cod_amount_read_from_row`.
- **Packet acceptance:** `paid_cod_credits_sender_once_by_mail_while_offline`.
- **Also:** `pay_cod_with_deleted_sender_cancels_cod_and_frees_item`, `pay_cod_refuses_without_item`, `pay_and_return_refuse_another_players_mail`, `payment_subject_is_capped_at_the_column_width`.
- **CAT-G-06:** `return_uses_sender_id_not_name`, `return_rejects_already_returned`, `return_rejects_system_mail`, plus `return_rejects_archived` and `return_cancels_cod_and_zeroes_price`.
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

The layers back each other up, so removing any one of R17's three alone still passes the race test. The single-layer gates are pinned by R1 and R2.

## Docs

- `docs/gameplay/mail-system.md`:
  - a new "Taking attachments, paying COD, returning (SS-M3)" section;
  - the status line and implementation rows, the CM 50 row, and the persistence block (`returned`);
  - remaining work, including the paid-COD return gap.
- `docs/gap-analysis.md` §24:
  - the heading, confidence, code list, recent PRs and path forward;
  - COD IM→NT, and take item, take cash and return KM→NT;
  - the matrix row 7/1/4/1 → 11/0/1/1 and the totals (NT 71, IM 100, KM 127); the percentages are recomputed.
- `docs/project-status.md` Mail row; `docs/game-systems.md` Mail section; `docs/known-issues.md` (mail moved out of the stubbed list, which was stale since SS-M1).
- `docs/protocol/cell-method-dispatch-table.md` CM 50; `docs/reverse-engineering/findings/mail-wire-formats.md` (`ContainerId`/`SlotId` now ignored, not "planned").
- `docs/architecture/observability.md`: the `mail` target row with the SS-M3 events.

## Known gaps (for the coordinator)

1. **A paid COD can still be returned (authority F2; decision requested by message).** Once paid, the mail looks like an ordinary item mail, so the recipient can return it and the seller gets both the item and the price.
   - For a manual return this is the recipient's own loss.
   - **For SS-M4 it is worse:** the expiry sweep reuses the return path, so a paid COD whose item is never taken would be auto-returned to the seller after 30 days.
   - My proposal: a `cod_paid boolean NOT NULL DEFAULT false` column. `pay_cod` sets it, return refuses it (`reason=cod_paid`), and SS-M4 quarantines instead of returning. The alternative is setting `returned = true` on payment.
   - Not implemented, pending the decision: the contract lists only `returned` for SS-M3.
2. **Owner veto point: a COD whose seller deleted their character goes to the recipient for free** (Design decisions). The only non-stranding alternative is to let delete destroy it.
3. **Header refresh is inferred from the decoder (M-Q7), not client-tested.** UAT checks:
   - after a take, the attachment icon disappears and the mail stays listed;
   - after a payment, the COD marker clears;
   - a read view that is open is not closed awkwardly by the remove-then-add.
4. **Tests not written:**
   - the lock-order deadlock scenario (A sends B while B pays A's COD);
   - return racing take or pay;
   - a payment into a sender's full mailbox (the cap exemption);
   - LogCapture for `mail.op_failed`;
   - a revert proof for the type 11 test (it is not a CI guard).
5. **A unique-slot collision answers `db_error`.** This happens only against an `INV_MAIN` writer that skips the bag's advisory lock. It rolls back and the item stays in escrow, but the player sees the generic line.
6. **An online sender is not told when a payment or a returned mail arrives** (D-SS11, SS-M4); they see it on the next header request.
7. **The type 11 test hard-codes 25 postage** (1,275). It must change if D-SS02 changes.
8. **SS-M2's Known gap 2 still stands:** deleting the *recipient's* character cascades their mail and destroys escrowed items.

## Integration edits for the coordinator

1. **Rebase:** `git rebase --onto origin/main 24b4163d0` once #912 merges. `docs/gap-analysis.md`'s totals are recomputed on top of SS-M2's numbers, so recompute them again if another packet has moved rows since.
2. **work-packets.md contract:**
   - `MailOp::TakeItem` keeps `container_id` and `slot_id` as the contract says, for the log only.
   - SS-M4 should call `return_::return_tx(pool, owner, mail_id, now)` for expiry path 1. It already cancels an unpaid COD with its price zeroed, and refuses returned, archived and system mail. For a quarantine check, SS-M4 reads `returned`.
   - Depending on the Known gaps 1 decision, SS-M4 also skips (quarantines) a paid, untaken COD.
3. **Local databases need `db/database.sql` re-run** for the new column (`reload-db.sh` does it). The colo rebuilds from the seed on deploy.
4. **Worktree hygiene (not mine, reported):** the server-authority-enforcer said it wrote its agent-memory note into the **main checkout** (`.claude/agent-memory/server-authority-enforcer/project_mail_escrow_ss_m2.md` and `MEMORY.md`). Those paths were already modified there when this session started.
