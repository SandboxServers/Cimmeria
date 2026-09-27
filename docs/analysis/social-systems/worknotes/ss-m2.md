# SS-M2 Worknotes

> Type: reference. Audience: social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [ss-m1.md](ss-m1.md), SS-E1's [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md).

## Contract

- **Packet:** SS-M2, cash and item attachments, COD escrow.
- **Decisions in force:** D-SS02 (25 naquadah postage, APPROVED), D-SS03 (100-message cap), D-SS05 (one recipient with any attachment), D-SS06 (one transaction, ascending `player_id` locks), D-SS08 (escrow leaves `sgw_inventory`; bound and mission items refused), D-SS09 (COD needs an item and a price above zero; the price is never the sender's money).
- **Base:** `origin/main` @ `e0d5cecf7` (SS-M1 #894, SS-E1 #875 merged). Branch `social/m2-attachments`, worktree `.claude/worktrees/ss-m2`.
- **Owned paths (new):**
  - `db/sgw/Mail/Tables/sgw_gate_mail_item.sql`
  - `crates/base-methods/src/base/world_entry/methods/mail/send/{attachment.rs, escrow.rs, sender_sync.rs, texts.rs}` (`texts.rs` is SS-M1's refusal text, moved out of `send/mod.rs` to keep it under 500 lines, no change)
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{attach_live.rs, attach_race.rs, attach_rollback.rs, delete_guard.rs}`
  - this file
- **Edited:** `mail/{mod.rs, read.rs}`, `mail/send/{mod.rs, deliver.rs, tests.rs}`, `mail/tests/{mod.rs, packets.rs, read_scoping.rs, send_limits.rs}`, `crates/wire/src/cell/mail/{mod.rs, tests.rs}` (`MailAttachment`, the header serializer's new argument), `crates/wire/src/cell/messages/data.rs` (one doc comment), `crates/cell-interactions/src/cell/mail.rs` (re-export), `db/database.sql`, `db/sgw/_foreign_keys.sql`, and the docs under "Docs".
- **Read set:** work-packets (Contract, Contended files, SS-M2); README D-SS02/03/05/06/08/09; audit § 6 CAT-G-01; `ss-m1.md`; `mail-wire-formats.md` M-Q2, M-Q4, M-Q5; the SS-M1 mail code; `inventory/core/{mod.rs, remove_instance.rs}`, `inventory/move_/mod.rs`, `trade/execute/{mod.rs, swap.rs}`, `vendor/helpers.rs`, `base-session/.../crafting/inventory_locks.rs`, `base-session/.../outbox/mod.rs`, the cell's `inventory_events.rs`; `db/sgw/Inventory/`, `db/sgw/Mail/`, `_foreign_keys.sql`, `_primary_keys.sql`, `database.sql`; the client's `GateMail.lua`.

## Advisors consulted

- **database-persistence:** approved a standalone table (not `INHERITS (sgw_inventory_base)`); asked for `CHECK (item_id >= 10000)` to mirror `sgw_inventory.local_id_check`, `source_character_id` without an FK, and `sgw_gate_mail.item_id` left NULL (one copy of the truth). **It caught a lock-order bug in my first plan:** I locked `sgw_player` before the item row, the reverse of the shared inventory order (`crafting/inventory_locks.rs`: advisory locks, then inventory rows, then the player row), which could deadlock against a crafting write. Fixed: `lock_source_item` runs before `deliver` locks the player rows.
- **items-systems-advisor:** confirmed the main-bag allowlist (trade's `TRADEABLE_CONTAINERS`), no split guard needed for non-stackables (`grant_item.rs` never stacks them), durability and charges copied on a split as `move_/` does. On `InventoryItemRemoved` it said "harmless today, but you are the second path to skip it": recorded in `escrow.rs`'s header. It disagreed on `MessageAttachment.itemId` (see Design decisions); I kept the type id, with the reasons below.
- **server-authority-enforcer:** CONDITIONAL. No defect found in client-lies, dupe/TOCTOU, overflow, lock order, delete race or silent-refusal classes. Two findings are for the coordinator (Known gaps 1 and 2).
- **testing-validation-engineer:** yellow, then addressed: a rollback test after the item has moved (`send_rolls_back_after_item_moved`), the whole-row race (`concurrent_whole_stack_sends_move_item_once`), an attached send to a full mailbox (`attached_send_to_full_mailbox_moves_nothing`), the `onUpdateItem` contents decoded and asserted, the delete-refusal event matched by `mail_id`, and the recipient-side `INVENTORY_ITEM_SELECT` revert run.

## Design decisions

- **Order of gates.** Before SQL: the bucket, the decode refusal, aliases, then with an attachment the multi-recipient check (D-SS05, by case-folded names) and `attachment::validate`: negative cash (`NoRecipients`), a quantity without an item (`NoRecipients`), an item without a quantity of at least 1 (`ItemNotAvailable`), COD without an item (`ItemNotAvailable`), COD without a price above zero (`NoRecipients`). The result codes follow SS-M1's rule: the specific code where one fits, else `NoRecipients` plus a feedback line naming the real reason.
- **In the transaction** (`send/deliver.rs`): resolve names, the Ignore seam, then `take_inventory_locks(sender, [INV_MAIN])` and the item row `FOR UPDATE` (owner-scoped: `character_id = sender`), then the player rows ascending `FOR UPDATE` (now also reading the sender's `naquadah` for the refusal log), the cap. If nobody is deliverable nothing is written. Otherwise `check_source` (main bag, not bound, quantity ≤ stack), the conditional debit (`naquadah >= cost`), the mail insert (`cash`, `MAIL_COD`), the escrow move, commit. Two typed names that fold to one but resolve to two players (an exact "Bob" and an exact "bob") are refused whole as `AttachmentsAndMultipleRecipients`.
- **Cost.** `POSTAGE + gift`; COD pays postage only (the price is the recipient's to pay, SS-M3). Computed in `i64`; a cost above `i32::MAX` is `NotEnoughCash` without touching the row.
- **Escrow move.** A whole stack: `INSERT … SELECT` from the row (keeping the instance id), then `DELETE` with `rows_affected == 1`. Part of a stack: `UPDATE … stack_size - q WHERE stack_size > q` (`rows_affected == 1`), then the escrow row with `nextval('sgw_inventory_item_id_seq')`. So an instance id is never in both tables, and a taken item fits back into `sgw_inventory` unchanged.
- **No `CellOutboxPayload::InventoryItemRemoved`.** The cell handler is one debug line and a main-bag item is no cell state; trade skips it too. The inventory remove paths do enqueue it, so this is a recorded divergence (`escrow.rs` header), not a precedent.
- **The inventory module is untouched.** `send_full_inventory_update` is called (it is `pub`). `send_on_remove_item` is `pub(super)`, so `sender_sync.rs` builds the 8-byte `onRemoveItem` itself rather than widen the shared module's visibility.
- **`MessageAttachment.itemId` is the type id.** SS-E1 M-Q2 established that `sendMailMessage`'s `ItemId` is the instance id; `mail-system.md` had carried that over to the attachment. The recipient's `GateMail.lua` builds the attachment's `Name`, `Icon`, `TechComp` and `Quality` from `mailGetItemAttachmentInfo(mailId)`, whose only per-attachment input is this field, and the recipient has no inventory record for an instance it does not own. So the server sends `type_id` (`InvItem.dbid`'s value). **Inference from the Lua, not a decompile**: Ghidra was not running this session. The items advisor disagreed by citing M-Q2, which is about the send side. UAT check: a received attachment must show its icon and name. If it is blank, switch `read.rs`'s `item_id: r.att_type_id?` to the escrow instance id (one line).
- **Delete guard.** One statement: `DELETE … WHERE mail_id AND character_id AND cash = 0 AND NOT EXISTS (escrow)`, so a concurrent take (SS-M3) either commits first or the mail stays. On 0 rows a follow-up owner-scoped read tells "not found" (the old WARN, still answered with `onMailHeaderRemove`) from "holds something" (WARN `mail.delete_refused`, a feedback line, no `onMailHeaderRemove`). `cash > 0` covers both gift cash and an unpaid COD price; D-SS09 has SS-M3 zero the amount on payment.
- **FK `ON DELETE CASCADE`** from the escrow row to its mail, so deleting a character (which cascades its mail) is not blocked. The guard is in the application, and the tests pin it.
- **Invariant for SS-M3 and SS-M4** (from the database advisor): only the send transaction inserts an escrow row, together with its mail; nothing ever attaches an item to an existing mail. The delete guard's race argument depends on it.

## Telemetry (debuggable from SigNoz alone)

Target `mail` (no new target). Every row carries `account_id`, `player_id`, `entity_id`; rows about the other player carry `target_player_id`.

| Event | Level | Fields |
|---|---|---|
| `mail.cash_debited` | INFO | `mail_id`, `target_player_id`, `naquadah_before`, `naquadah_after`, `postage`, `cash`, `cod` |
| `mail.item_escrowed` | INFO | `mail_id`, `target_player_id`, `item_id` (the sender's row), `escrow_item_id`, `type_id`, `quantity`, `stack_before`, `stack_after`, `whole_row` |
| `mail.attachment_refused` | DEBUG | `reason`, `item_id`, `item_quantity`, `cash`, `cod`, `naquadah` (balance under the lock), `cost` |
| `mail.send_refused` | WARN | SS-M1's row; new reasons `negative_cash`, `item_quantity_without_item`, `invalid_item_quantity`, `cod_without_item`, `cod_without_price`, `item_not_owned`, `item_not_in_main_bag`, `item_bound`, `item_quantity_exceeds_stack`, `not_enough_cash`. `attachment_not_supported` is gone |
| `mail.delete_refused` | WARN | `mail_id`, `reason` (`attachment_item_present` \| `attachment_cod_unpaid` \| `attachment_cash_present`), `has_item`, `cash`, `cod` |
| `mail.deleted` | DEBUG | `mail_id` |
| `mail.headers_sent` | DEBUG | now also `attachments` |

SigNoz queries (Logs, `scope_name = 'mail'`):

- what happened to a mailed item: `attributes.event = 'mail.item_escrowed' AND attributes.item_id = <id>`, then `attributes.mail_id` from that row;
- every naquadah movement of a player through mail: `attributes.event = 'mail.cash_debited' AND attributes.player_id = <id>`;
- why an attached send failed: `attributes.event IN ('mail.send_refused', 'mail.attachment_refused') AND attributes.player_id = <id>`;
- a refused delete: `attributes.event = 'mail.delete_refused' AND attributes.mail_id = <id>`.

## Commands run

All from the worktree root, through the lane (exit codes are the lane's `released (exit N)`).

| Command | Result |
|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-base-methods --all-targets` | exit 0 |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-base-methods mail` | 47 passed (live-DB tests self-skip here; the next row runs them) |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-wire mail` | 17 passed |
| `bash tools/build-lane/live-db-test.sh mail` (final) | reload into `sgw_ss_m2`; 74 run, 74 passed, none skipped |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-base-world-entry` | 773 run, 773 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-interactions -p cimmeria-base-world-entry` | `cimmeria-wire`'s `player_journal::tests::note_logs_to_the_stable_target_with_seq` failed once under `cargo test`'s shared process and passed alone and under nextest; untouched by this packet (a log-capture interaction between parallel tests) |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | exit 0 |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-interactions --all-targets -- -D warnings` | exit 0 |
| `bash tools/build-lane/live-db-test.sh "::"` (the whole live-DB tier, for the schema change) | exit 0: 4,279 run, 4,279 passed, 0 skipped (reloaded `sgw_ss_m2`, 174 s) |

## Tests

Names as audit § 6 CAT-G-01 and the packet ask, plus the packet's own:

- **CAT-G-01:** `send_rejects_unknown_recipient`, `send_rejects_eleven_recipients`, `send_rejects_attachment_with_two_recipients` (SS-M1's, kept), `send_rejects_negative_cash` (updated: now refused as `negative_cash` before SQL), `send_rejects_item_not_owned`, `send_rejects_bound_item`, `send_debits_cash_and_postage_atomically`, `send_rolls_back_on_insert_failure`.
- **Delete guard (live DB):** `delete_refused_while_attachment_present` (item, gift cash, COD; a plain mail still deletes), `delete_refused_for_unpaid_cod_without_item`, `no_orphaned_escrow_after_delete` (also owner scope).
- **Rollback (live DB):** `send_rolls_back_on_insert_failure` (after the debit and the item lock, before the move), `send_rolls_back_after_item_moved` (after the split's decrement: the escrow insert is forced into a `UNIQUE` conflict by pointing the inventory sequence at a taken id, then the sequence is restored), `attached_send_to_full_mailbox_moves_nothing`.
- **Type 5:** `concurrent_sends_move_item_once` (split), `concurrent_whole_stack_sends_move_item_once` (whole row), forced with SS-M1's `SHARE` gate on `sgw_gate_mail`.
- **Wire (type 2):** `message_attachment_bytes_are_alias_ordered`.
- **Escrow invisibility:** `escrowed_item_absent_from_inventory_select` (both players, through the real `INVENTORY_ITEM_SELECT`).
- **Headers:** `headers_carry_attachment_and_cod_flag`.
- **Other live DB:** `send_rejects_item_outside_main_bag_or_over_stack`.
- **Type 12 / unit:** `send_passes_well_formed_attachments_to_delivery` (replaces `send_refuses_every_attachment_until_ss_m2`), `attachment_validation_refuses_malformed_attachments`, `attachment_cost_is_postage_plus_gift_cash`. `mail_send_bucket_rejects_fourth_in_burst` now uses COD-without-item as its code-2 send, because an allowed cash send with no pool now ends on code 1, the limited send's own code.

Sentinels: accounts, players and entities `0x7300_14xx`/`0x7300_15xx`; item instances `0x7300_16xx`/`0x7300_17xx`, one fixed escrow id `0x7300_17F0`. Cleanup is by exact account id (players, inventory, mail and escrow cascade).

## Regression proof

Each revert was applied with a script, the `mail` live-DB tier run against `sgw_ss_m2`, and the sources restored and touched. Reverts in one batch touch unrelated tests, and every failure below was read for its cause.

| Reverted | Guard | Result |
|---|---|---|
| delete guard → plain `DELETE … WHERE mail_id AND character_id` | `delete_refused_while_attachment_present`, `delete_refused_for_unpaid_cod_without_item`, `no_orphaned_escrow_after_delete` | FAILED (`onMailHeaderRemove` sent, escrow rows gone) |
| postage dropped from `sender_cost` | `send_debits_cash_and_postage_atomically`, `attachment_cost_is_postage_plus_gift_cash` | FAILED (700 not 675) |
| bound check off | `send_rejects_bound_item` | FAILED (sent) |
| main-bag check off | `send_rejects_item_outside_main_bag_or_over_stack` | FAILED (bandolier item sent) |
| header attachments emptied | `headers_carry_attachment_and_cod_flag` | FAILED |
| negative-cash check off | `send_rejects_negative_cash` | FAILED (reached delivery) |
| debit committed in its own transaction | `send_rolls_back_on_insert_failure` | FAILED (375 not 500) |
| item lookup not owner-scoped | `send_rejects_item_not_owned` | FAILED (code 1, not 2) |
| whole-row move leaves the row in `sgw_inventory` | `send_debits_cash_and_postage_atomically`, `escrowed_item_absent_from_inventory_select` | FAILED |
| whole-row move hands the row to the recipient (`character_id` reassigned) | `escrowed_item_absent_from_inventory_select` (recipient side), `concurrent_whole_stack_sends_move_item_once` | FAILED |
| `COMMIT` between the split's decrement and the escrow insert | `send_rolls_back_after_item_moved` | FAILED (375 not 500) |
| mailbox cap off | `attached_send_to_full_mailbox_moves_nothing` (and SS-M1's two cap guards) | FAILED |
| advisory lock + `FOR UPDATE` + `stack_size > $1` all removed | `concurrent_sends_move_item_once` | FAILED (both sent) |
| advisory lock + `FOR UPDATE` removed, stack guard kept | both race tests | FAILED: the item still moves once (the stack guard, or the vanished row, stops the second), but the loser gets `db_error` / code 1 instead of `ItemNotAvailable` / code 2, so the locks are pinned by the refusal the player sees |

## Docs

- `docs/gameplay/mail-system.md`: status, a "Sending with an attachment (SS-M2)" section, the delete guard, implementation-status rows, the wire section (attachments now sent; `itemId` is the type id, correcting the page's earlier "instance id"), persistence (the escrow table), remaining work; the stale "Mail Send Flow (not implemented)" and the `bArchive` "known gap" (fixed by SS-M1) replaced.
- `docs/reverse-engineering/findings/mail-wire-formats.md`: the `itemId` row and an "SS-M2 note on `itemId`" under M-Q4, marked as an inference.
- `docs/gap-analysis.md` §24: heading, confidence, code list, three rows (Attach item and Attach gold KM→NT, Cash on Delivery KM→IM), Delete mail row; the matrix row (5/0/7/1 → 7/1/4/1), the TOTALS line (NT 67, IM 101, KM 130) and the percentage table.
- `docs/project-status.md` Mail row; `docs/game-systems.md` Mail section; `docs/architecture/observability.md` `mail` target row.

## Known gaps (for the coordinator)

1. **Nothing can take an attachment out yet (server-authority-enforcer: High).** CM 47/49/50/51 are still SS-M3's `UNIMPLEMENTED` stubs, so from this packet on, cash, items and COD that players mail sit in escrow until SS-M3 lands, and the delete refusal's advice ("take it or return the message") points at buttons that do nothing and show nothing. Options: merge SS-M2 and SS-M3 together or release them together, or have SS-M3 land before the next `/release`. Nothing is lost meanwhile: the escrow rows are durable.
2. **Deleting the recipient's character destroys escrowed items and COD (owner decision).** The FK cascade keeps character deletion unblocked; returning escrow to `source_character_id` on character delete needs a rule (a trigger, or the delete path mailing it back). Raised by both the database and authority advisors.
3. **`MessageAttachment.itemId` is inferred** (see Design decisions); one UAT look settles it.
4. `onMailHeaderInfo`'s `ResetCategory` is still 0 (SS-M1's gap); a refused delete sends no header, so if the native `mailDeleteMessage` drops the row locally the mail reappears on the next mailbox open. The feedback line is the first-press answer either way.
5. No wireclient (type 11) session: two-client mail stays for SS-M3's "A sends B a COD item" test.
6. The advisory lock's job against other inventory writers (crafting, vendor, trade on the same player) has no cross-system race test; the mail-vs-mail races are covered.

## Owner constraint: backpack only (relayed 2026-09-27)

The Bank campaign's owner decision (vendors, trade, crafting and mail see only the backpack) arrived after the first push. What changed:

- `check_source` (`send/escrow.rs`) already refused everything outside the main bag. Vault items (17 personal, 18 auction, 19 team, 20 command) now get `reason = item_in_vault` and "Items in a vault cannot be sent by gate-mail. Move it to your backpack first. The message was not sent."; buyback (16) gets `reason = item_in_buyback` and its own line. Both answer `MAILRESULT_ItemNotAvailable`.
- Guard: `send_rejects_banked_item` (live DB, `tests/attach_vault.rs`), all five containers: refused, the specific reason logged, the feedback line, nothing debited, no mail, no escrow, and each item still in its own container. **Proof:** with the container check bypassed it FAILED (the first vault item was mailed, code 0 not 2); restored, it passed. `live-db-test.sh mail` after the change: 75 run, 75 passed.
- Nothing in SS-M2 puts an item back into an inventory. The return-side rule (a take, a return, a COD delivery lands in the backpack, never 17-20) is SS-M3's; it is written into `mail-system.md` and integration edit 3 below. BV-01's grant guard (#872) is not relied on.

## Integration edits for the coordinator

1. `docs/gap-analysis.md`: the TOTALS line and percentages are recomputed on top of `e0d5cecf7`; any packet merged before this one that also moved rows needs them recomputed again.
2. `work-packets.md` contract: `sgw_gate_mail_item` columns as in the SQL file; `sgw_gate_mail.item_id` stays NULL (the escrow table is the one copy). SS-M3's take and return, and SS-M4's expiry, should delete or move the escrow row in the same transaction as the mail change, and must never attach an item to an existing mail.
3. Tell SS-M3: every item it puts back (take, return, COD delivery) goes to the backpack (`INV_MAIN`), never 17-20, per the owner's backpack-only rule; SS-M3 picks and documents the full-backpack overflow (the packet's current answer: leave it in escrow and give feedback). `deliver` holds the send lock order (advisory, item row, player rows); take-item needs the recipient's main-bag slot reservation (`reserve_main_slots_excluding` shape, `pg_advisory_xact_lock(player, INV_MAIN)`) and a `i32` overflow check on the credit (`cash` is `bigint`, `naquadah` is `integer`). A paid COD must zero `cash` (the delete guard keys on `cash = 0`).
4. Tell the Bank campaign (cimmeria-97) and Crafting (cimmeria-af): mail is a second path that removes main-bag items in its own transaction (`mail/send/escrow.rs`), taking `take_inventory_locks(player, [INV_MAIN])` first.
