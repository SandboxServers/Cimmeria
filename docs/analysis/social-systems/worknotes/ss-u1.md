# SS-U1 Worknotes

> Type: reference. Audience: social-systems coordinator, and the Black Market campaign (BM-02b) for the system-mail API.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [ss-m1.md](ss-m1.md), [ss-m2.md](ss-m2.md).

## Contract

- **Packet:** SS-U1, GM mail tooling, plus the shared system-mail entry point (scope added by the coordinator: the Black Market's BM-02b depends on it).
- **Decisions in force:** D-SS03 (server-generated mail ignores the cap), D-SS08 (escrow in `sgw_gate_mail_item`, every instance column kept), D-SS09 (COD needs an item and a price; the payment goes to the sender), D-SS10 (no `sender_id` means not returnable), the owner's plain-English preference for GM commands.
- **Base:** `origin/main` @ `23e97ac58` (SS-M1 #894, SS-M2 #912 merged). Branch `social/u1-gm-mail-tooling`, worktree `.claude/worktrees/ss-u1`.
- **Owned paths (new):**
  - `crates/base-methods/src/base/world_entry/methods/mail/system/{mod.rs, write.rs}`: the writer
  - `crates/base-methods/src/base/world_entry/methods/mail/gm.rs`: the base half of the GM tools
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{system_live.rs, gm_live.rs}`
  - `crates/wire/src/cell/messages/mail_gm_cell_to_base.rs`: `MailGmCellToBase`, `MailGmActor`
  - `crates/cell-console/src/cell/console/mail.rs`, `crates/cell-console/src/cell/console/tests/ss_u1_mail.rs`
  - this file
- **Edited:**
  - `wire/.../messages/{mod.rs, cell_to_base.rs}`: the `CellToBaseMsg::MailGm` variant (a nested enum, so `MailOp` in `data.rs`, which SS-M3 owns, is untouched).
  - `base-world-entry/.../cell_dispatch/mod.rs`: one arm routing `MailGm` to `methods::mail::handle_mail_gm`.
  - `base-methods/.../mail/mod.rs`: `mod gm; pub mod system;` and the re-exports.
  - `base-methods/.../mail/send/{mod.rs, deliver.rs, escrow.rs, recipients.rs}`: visibility only (`pub(super)` to `pub(in super::super)`) for `escrow_item`, `SourceItem`, `MAILBOX_CAP`, `candidate_rows`, `resolve_names`, `Resolution`, `FailReason`. No behaviour change.
  - `base-methods/.../mail/tests/{mod.rs, packets.rs}`: the two test modules, and `Client::gm` (runs a GM command as the test client).
  - `cell-console/.../console/{mod.rs, dispatch.rs, registry/mod.rs, registry/commands/social.rs, tests/mod.rs}`: three specs, their `.help` argument rows, three dispatch arms.
  - Docs: `docs/gameplay/mail-system.md`, `docs/commands.md`, `docs/architecture/observability.md` (the `mail` target row).
- **Read set:** `SS-WORKER-RULES.md`; work-packets (Contract, Contended files, SS-M2, SS-M3, SS-M4, SS-U1, SS-U3); README D-SS02 to D-SS10; `ss-m1.md`, `ss-m2.md`, `ss-u2.md`; the whole `mail/` module; `inventory/grant/grant_item.rs` (instance defaults); `inventory/move_/container_policy.rs` (container 18 is `Movable::No`); `crafting/inventory_locks.rs`; `db/sgw/Mail/Tables/*.sql`, `db/sgw/Inventory/Tables/*.sql`, `db/resources/Items/Tables/items.sql`, `_foreign_keys.sql`; `docs/analysis/black-market/README.md` (S9, BM-02b); `origin/feat/571-black-market-phase1` `base/black_market/helpers.rs` (`send_mail_to_player`, `escrow_item`); `cell-console` `console/{dispatch.rs, social.rs, duel.rs, give.rs, registry/*, tests/{mod.rs, ss_c2_announce.rs, pt07_*.rs}}`; `base-world-entry` `cell_dispatch/{mod.rs, chat_dispatch.rs, progression_dispatch.rs}`; `org_text.rs`; `target_scan_tests.rs`.

## The system-mail API (final public signature, for BM-02b)

Path: `cimmeria_base_methods::base::world_entry::methods::mail::{…}` (also re-exported from `mail::system`). Through the services facade: `cimmeria_services::base::world_entry::methods::mail::{…}`.

```rust
pub const SYSTEM_SOURCE_CHARACTER_ID: i32 = 0;          // escrow source of a minted item
pub const SERVER_HELD_CONTAINERS: &[i32] = &[INV_AUCTION]; // 18

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMail {
    pub sender_name: String,        // label, 1-128 chars, one line ("Black Market")
    pub recipient_player_id: i32,   // online or not
    pub subject: String,            // 1-128 chars, one line (D-SS12)
    pub body: String,               // up to 1,000 chars (D-SS12)
    pub cash: i64,                  // 0..=i32::MAX, minted or already taken by the caller
    pub item: SystemItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemItem {
    None,
    Minted { type_id: i32, qty: i32 },                       // 1..=max_stack_size
    ExistingInstance { item_id: i32, owner_player_id: i32 }, // row in container 18 of that owner
}

pub async fn send_system_mail_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    mail: &SystemMail,
) -> Result<SystemMailSent, SystemMailError>;             // commits nothing

pub async fn send_system_mail(
    pool: &sqlx::PgPool,
    mail: &SystemMail,
) -> Result<SystemMailSent, SystemMailError>;             // own transaction, commits, logs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemMailSent {
    pub mail_id: i32,
    pub recipient_player_id: i32,
    pub sender_name: String,
    pub cash: i64,
    pub item_source: &'static str,        // "none" | "minted" | "existing_instance"
    pub item: Option<SystemEscrow>,
    pub recipient_open_mail: i64,         // after this mail
}
impl SystemMailSent { pub fn log_sent(&self); } // call after YOUR commit when using _tx

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemEscrow { pub item_id: i32, pub type_id: i32, pub stack_size: i32, pub source_character_id: i32 }

#[derive(Debug)]
pub enum SystemMailError {
    InvalidText { field: &'static str, reason: &'static str },
    NegativeCash, CashTooLarge,
    InvalidQuantity, UnknownItemType, QuantityExceedsStack { max_stack_size: i32 },
    ItemNotFound, ItemNotServerHeld { container_id: i32, owner: i32 },
    ItemOwnerMismatch { owner: i32 }, ItemBound { owner: i32 },
    RecipientNotFound,
    Db(sqlx::Error),
}
impl SystemMailError { pub fn reason(&self) -> &'static str; } // + Display, Error, From<sqlx::Error>
```

Item ids are `i32`, not `i64` as the packet sketch had: `sgw_inventory.item_id` and `sgw_gate_mail_item.item_id` are `integer`.

### What the Black Market caller must do

Reviewed with the server-authority-enforcer (see Advisors).

1. **Lock order.** Take the inventory advisory locks for the seller and, when debiting, the buyer (`take_inventory_locks`); then lock **every** player row the transaction touches, including each mail recipient, `FOR UPDATE` in ascending `player_id`; then call `send_system_mail_tx`. Its own `FOR UPDATE` on the recipient must be a re-lock of a row you already hold. For an existing instance, the writer takes the owner's advisory locks (player-wide key and container 18) itself; if you have not taken them first, it takes them before any player row, which is also in order.
2. **The writer is not idempotent for cash.** A second call with the same item gets `ItemNotFound`, but a cash payout mints again on every call. Gate each settlement on a conditional `UPDATE sgw_auction SET status = … WHERE sequence_id = $1 AND status = <open>` with `rows_affected == 1` in the same transaction.
3. **Whole rows only.** `ExistingInstance` moves the whole row; split partial listings when they are created. Keep listed rows in container 18 under the seller's `character_id` (that is what "server-held" means here). If the branch keeps deleting the row at listing time and storing a snapshot on `sgw_auction` instead, `ExistingInstance` does not apply; say so and we add a snapshot variant rather than using `Minted` (which would lose durability, charges and ammo).
4. **Bound items.** A bound row can only be mailed back to its owner (cancel, expiry). Refuse bound items at listing time.
5. **A missing recipient aborts.** `RecipientNotFound` is an error, so a deleted seller rolls back the buyer's purchase. Decide the policy (refuse the sale, or burn the payout), and what character deletion does to the seller's container-18 rows.
6. **The seller's client is not told.** Moving a container-18 row sends no `onRemoveItem`; if the client shows container 18 while the seller is online, send one after the commit.
7. **Notification.** No `onNewMail` is sent (SS-M4 adds it for every delivery path, system mail included).
8. **Telemetry.** Log your settlement with the returned `mail_id`, then call `sent.log_sent()` after the commit.

## Design decisions

- **One writer, two headers.** `system::write_mail(conn, &MailHeader, SystemItem, now)` does the SQL for every server-written mail. `send_system_mail_tx` passes `sender_id = NULL`, `flags = 0`; the GM's COD mail passes `sender_id = the GM`, `flags = MAIL_COD`, `cash = price`. So there is one place that inserts a mail with an escrow row outside the player send path, and it keeps SS-M2's invariant: the escrow row is inserted in the same transaction as its mail, never attached to an existing mail.
- **Lock order.** The item first (the owner's inventory advisory locks, then the row `FOR UPDATE`), then the recipient's `sgw_player` row `FOR UPDATE`, then the inserts: the same order as `send/deliver.rs` and crafting. For an existing instance the owner is read without a lock, the advisory locks are taken, and the row is re-read `FOR UPDATE` keyed on that owner and checked again, so a row that moved or changed hands in between is `ItemNotFound` or `ItemNotServerHeld`.
- **Server-held = container 18, allowlist, not a denylist.** "Not in a player container" is enforced as "in `SERVER_HELD_CONTAINERS`", so a new player container added later is refused by default. On `main` nothing puts items in container 18 yet (`container_policy.rs` makes it `Movable::No`; BV-01's grant guard refuses grants into it).
- **Existing instance reuses SS-M2's move.** `send::escrow::escrow_item` with the whole stack: the row keeps its instance id and every column, then is deleted from `sgw_inventory` with a `rows_affected == 1` check.
- **Minted defaults** are `grant_item`'s: durability 100, `charges` and `ammo` from the template's `charges`, `default_ammo_type` (else `AMMO_NONE`), the template's `ammo_types`, `cur_ammo_type` 0, flags 0, not bound, and a fresh id from `sgw_inventory_item_id_seq`, so a taken item fits back into `sgw_inventory`. The quantity is capped at the template's `max_stack_size` (at least 1), because the escrow row is one stack.
- **Cap exemption.** D-SS03 already exempts server-generated mail; the writer never counts before inserting. It counts after, for the log (`recipient_open_mail`, `over_cap`).
- **No `sender_id`, so never returnable.** D-SS10: SS-M3's return refuses `sender_id IS NULL`. The delete guard is unaffected (it keys on `cash = 0` and no escrow row). `sgw_gate_mail.sender_id` is also `ON DELETE SET NULL`, so a deleted sender's player mail becomes unreturnable the same way. Sent to ss-m3 at the start of the packet.
- **`.mail` without COD is system mail** from the GM's name: minted, no postage, not returnable. **`.mail … cod <n>`** must come back to the GM, so it is a mail from the GM's character (`sender_id` = the GM), `MAIL_COD`, the price in `cash`, a minted item, no postage. It is not written through the player send path (`deliver`), which takes an existing inventory instance and charges postage; it uses the shared writer with the COD header instead. SS-M3's pay and take paths see an ordinary COD mail. A GM COD to themselves pays net zero (reviewed).
- **COD rules on both sides.** The cell refuses COD without an item, COD with cash and a price below 1; the base re-checks all three (`cod_without_item`, `cod_with_cash`, `cod_price_invalid`), because a COD mail with no item or price could never be paid or taken.
- **Trust boundary.** `CellToBaseMsg::MailGm` is built only by `console/mail.rs`, after the `.`-console gate has checked the caller's server-side `access_level` (GameMaster or higher). The actor's ids come from the cell's `CellEntity` (`player_identity`), never from a payload, and the base reads the GM's name from `sgw_player`. A non-GM's `.mail`, `.mailbox` or `.mail_expire` is consumed by the existing non-GM gate ("is a GM command") and reaches nothing.
- **`.mail_expire`** cannot set `expires_at` (SS-M4 adds the column). It parses the id and answers "mail does not expire yet (expiry arrives with SS-M4), so mail N was not changed.", logging `mail.gm_rejected reason=expiry_not_available`. Integration edit 1 turns it on.
- **`.mailbox`** counts in one query: open and archived (`MAIL_ARCHIVE`), mails with an escrow row, gift cash (the `cash` of non-COD mail), unpaid COD (`MAIL_COD` still set), system mail (`sender_id IS NULL`). The next-expiry line says none until SS-M4.
- **New message, not a `MailOp`.** `MailOp` and `data.rs` are SS-M1 → SS-M3 contended; a nested `MailGmCellToBase` in its own file follows the Org/Chat pattern and touches `cell_to_base.rs` by one variant.

## Advisors

- **server-authority-enforcer (read-only review of the first commit):** CONDITIONAL, no exploitable defect. Four findings, all fixed in the second commit:
  1. (Medium) `ExistingInstance` ignored `bound`: a buggy listing could move a bound item to another player. Now `ItemBound` unless the recipient is the owner.
  2. (Medium) `ExistingInstance` was not tied to an expected owner: a wrong id could take another seller's listed row. Now `ExistingInstance { item_id, owner_player_id }`, refused as `ItemOwnerMismatch`.
  3. (Low) The base did not re-check the COD price. Now `cod_price_invalid`.
  4. (Low) Thin refusal telemetry. `mail.gm_rejected` on a refused `.mail` now carries `to`, `cash`, `cod`, `type_id`, `quantity` and the underlying error; `mail.gm_action` carries `minted = true`.
  It also produced the Black Market list above.

## Telemetry (debuggable from SigNoz alone)

Target `mail` (no new target; the scan pin already lists `mail` at INFO, WARN and DEBUG). The cell-console now also emits on it.

| Event | Level | Where | Fields |
|---|---|---|---|
| `mail.system_sent` | INFO | after the commit | `sender_name`, `target_player_id`, `mail_id`, `cash`, `item_source`, `item_id`, `type_id`, `stack_size`, `source_character_id`, `recipient_open_mail`, `over_cap` |
| `mail.system_staged` | DEBUG | inside the caller's transaction | `sender_name`, `target_player_id`, `mail_id`, `cash`, `item_source`, `item_id` |
| `mail.system_refused` | WARN | writer | `reason`, `sender_name`, `target_player_id`, `cash`, `item_source`, `item_id`, `type_id`, `container_id`, `owner_player_id`, `expected_owner_player_id`, `error` |
| `mail.gm_action` | INFO | base | `action` (`mail` \| `mailbox`), `minted`, GM `entity_id` / `account_id` / `player_id`, `subject_player_id`, `mail_id`, `cash`, `cod`, `type_id`, `quantity`, `escrow_item_id`; for `mailbox` the counts |
| `mail.gm_rejected` | WARN | cell and base | `command`, `reason`, GM ids; on the base's `.mail` refusal also `to`, `cash`, `cod`, `type_id`, `quantity`, `error` |

The system-mail events have no player actor (the server sends them), so they carry `target_player_id` only; the caller (BM settlement, content action, GM tool) logs its own actor row with the returned `mail_id`. A GM `.mail` also logs `GM .-console command accepted` on the cell.

SigNoz queries (Logs, `scope_name = 'mail'`):

- every mail a player got from the server: `attributes.event = 'mail.system_sent' AND attributes.target_player_id = <id>`;
- every GM mint: `attributes.event = 'mail.gm_action' AND attributes.minted = true`, grouped by `attributes.account_id`;
- what a GM did to player X: `attributes.event IN ('mail.gm_action', 'mail.gm_rejected') AND attributes.subject_player_id = <id>`;
- why a payout failed: `attributes.event = 'mail.system_refused'`, by `attributes.reason`.

## Tests

- **Parse, one per command** (cell, `tests/ss_u1_mail.rs`): `mail_parse_reads_every_option_and_the_subject`, `mail_parse_defaults_and_subject_boundary`, `mail_parse_refuses_malformed_options` (12 cases), `mail_expire_parse_needs_a_positive_id`, `mailbox_forwards_the_name` (`.mailbox`'s only argument).
- **Cell forwarding and type 12:** `mail_forwards_one_send_with_the_gm_ids`, `mail_refusal_logs_reason_and_sends_nothing_to_the_base`, `mail_expire_is_refused_until_expiry_exists`.
- **Non-GM rejection:** `non_gm_mail_commands_are_refused` (all three commands at access level 0: the "GM command" line, no `MailGm` message).
- **System mail, live DB** (`tests/system_live.rs`): `system_mail_cash_and_minted_item` (cash only, minted item with its defaults), `system_mail_moves_server_held_instance` (existing instance, every column), `system_mail_refuses_instance_in_player_inventory` (7 player containers + a missing id; type 12 on `mail.system_refused`), `system_mail_existing_instance_owner_and_bound_rules`, `system_mail_rolls_back_with_callers_transaction` (atomicity), `system_mail_refusals_write_nothing` (stack, unknown type, qty 0, missing recipient, cash range, text), `system_mail_ignores_mailbox_cap` (and `mail.system_sent`).
- **GM, live DB** (`tests/gm_live.rs`): `gm_mail_creates_one_mail_and_one_escrow_row` (the packet's acceptance test), `gm_mail_cod_is_sent_from_the_gm`, `gm_mail_unknown_recipient_writes_nothing` (type 12), `gm_mail_refuses_cod_without_a_price` (type 12), `gm_mailbox_reports_counts_and_escrow`.

Sentinels: accounts and players `0x7300_5100`-`0x7300_518A`, test entities `0x7300_5190`-`0x7300_5194`, items `0x7300_5200`-`0x7300_5231` (`0x7300_52FF` is a deliberately missing id). Cleanup is by exact account id (players, inventory, mail and escrow cascade).

## Commands run

All from the worktree root, through the lane (exit codes are the lane's `released (exit N)`).

| Command | Result |
|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-console -p cimmeria-base-world-entry --all-targets` | exit 0 |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-console -p cimmeria-base-world-entry --all-targets -- -D warnings` | exit 0 |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | exit 0 |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-cell-console` | 339 passed (9 of them `ss_u1_mail`) |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-base-methods -p cimmeria-cell-console -p cimmeria-base-world-entry` | 1,028 run, 1,028 passed, 0 skipped |
| `bash tools/build-lane/live-db-test.sh mail` (final, after the proofs) | reload into `sgw_ss_u1`; 99 run, 99 passed; the 12 new live-DB tests ran (not self-skipped) |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-server logging` (the target-scan pin, since the cell-console now logs on `mail`) | 53 passed, 5 skipped (the crate's own ignored tests) |
| `npx markdownlint-cli2` on the four touched docs | no finding on the lines this packet touched (`observability.md` has pre-existing findings on lines 323 and 463) |

## Regression proof

Commit first (`4b58f4c29`, `aa31f7d1f`), then each mutation applied by script, the named guard run, the file restored with `git checkout HEAD -- <file>` and touched. After the last restore `git status` showed no source change.

| Mutation | Guard | Result |
|---|---|---|
| `require_server_held` always passes | `system_mail_refuses_instance_in_player_inventory` | FAILED |
| owner check off (`owner != owner_player_id`) | `system_mail_existing_instance_owner_and_bound_rules` | FAILED |
| bound check off | `system_mail_existing_instance_owner_and_bound_rules` | FAILED |
| minted stack cap off | `system_mail_refusals_write_nothing` | FAILED |
| minted durability 100 → -1 | `system_mail_cash_and_minted_item` | FAILED |
| GM COD written with `sender_id = NULL` | `gm_mail_cod_is_sent_from_the_gm` | FAILED |
| base COD price re-check off | `gm_mail_refuses_cod_without_a_price` | FAILED |
| cell `cod_without_item` check off | `mail_parse_refuses_malformed_options`, `mail_refusal_logs_reason_and_sends_nothing_to_the_base` | FAILED |

Not mutation-proved, and why: the rollback test (`system_mail_rolls_back_with_callers_transaction`) guards a structural property (the writer takes `&mut Transaction` and has no other connection), and there is no one-line revert that breaks it short of rewriting the API; the cap exemption is a positive test (a mutation would have to add a cap check); the non-GM test guards the registration of the three commands against the existing gate.

## Known gaps

1. **Nothing takes an attachment yet.** Until SS-M3 merges, system and GM mail sit in escrow like SS-M2's (ss-m2 known gap 1). `.mail` is still useful for checking headers, the escrow and `.mailbox`.
2. **No `onNewMail`.** An online recipient sees a system or GM mail the next time the mailbox opens (SS-M4, D-SS11).
3. **`.mail_expire` is a refusal** until SS-M4 (integration edit 1).
4. **Server-held is container 18 only**, and nothing on `main` puts rows there yet; the Black Market campaign confirms the shape (item 3 of its list).
5. **A GM `.mail` to an offline character's name works** (by design: offline recipients are allowed), so a typo that matches another character mails them; the feedback line names the stored name it resolved to.

## Integration edits for the coordinator

1. **SS-M4:** make `.mail_expire <mailId>` set `expires_at = now` (owner-scoped to any mailbox, GM tool), replace the refusal arm in `cell-console/.../console/mail.rs` `mail_expire` (it then forwards a new `MailGmCellToBase::Expire { actor, mail_id }`), and replace the third `.mailbox` line (`gm.rs` `mailbox_lines`) with the real next expiry. Quarantined mail should be excluded from the open count once the column exists (`MAILBOX_SUMMARY_SQL`).
2. **SS-M3:** return refuses `sender_id IS NULL` (system mail); minted escrow rows have `source_character_id = 0`, so nothing may route by it; a GM COD mail is an ordinary COD mail from the GM. (Told to ss-m3.)
3. **SS-U3:** the `send_system_mail` content action calls `send_system_mail(pool, &SystemMail { sender_name: "Gate Mail Clerk", …, item: SystemItem::Minted { type_id, qty } })`.
4. **Black Market (cimmeria-11, BM-02b):** the API section above, and the eight caller rules.
5. `work-packets.md` SS-U1 section: the scope grew by the system-mail API; the acceptance tests are the ones listed above.
