---
title: "Mail System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Mail System

> **Last updated**: 2026-09-27
> **Status**: Read side implemented (headers / body / delete / archive), player sending with text (social-systems SS-M1) and with cash, an item or COD attached, held in escrow (SS-M2), and taking cash, taking the item, paying COD and return-to-sender (SS-M3). New-mail notification and expiry are SS-M4.

## Overview

The mail system enables asynchronous message delivery between players with support for item attachments, currency attachments, Cash On Delivery (COD), archiving, and return-to-sender. Mail is stored server-side and retrieved on demand. The system supports multiple recipients and tracks read/unread state.

The `SGWMailManager` interface in `entities/defs/interfaces/SGWMailManager.def` defines the complete protocol.

The Rust implementation forwards every mail request from the cell to the base, because mail needs database access and the DB pool lives on the BaseApp. [`crates/cell-interactions/src/cell/mail.rs`](../../crates/cell-interactions/src/cell/mail.rs) packages the request as `CellToBaseMsg::MailRequest { op: MailOp }`; [`crates/base-methods/src/base/world_entry/methods/mail/`](../../crates/base-methods/src/base/world_entry/methods/mail/) runs the query and sends the result straight back to the client. `mod.rs` routes each `MailOp`, `read.rs` holds the read side and `send/` the send path.

Two paths write `sgw_gate_mail`: the player send path below, and the system-mail writer in `mail/system/` for mail the server sends (see [Server and GM mail](#server-and-gm-mail-ss-u1)). The Black Market branch's own helper, `send_mail_to_player` on the unmerged `feat/571-black-market-phase1` (PR #586), is to be replaced by the system-mail writer (Black Market S9, BM-02b).

### Sending a text mail (SS-M1)

`sendMailMessage` (CM 44) is decoded on the cell by `decode_send_mail_message` (`crates/wire/src/cell/cell_methods/mail/`): the declared recipient count is checked against the cap of 10 before any name is read, and the recipient names, subject (1-128 characters) and body (up to 1,000, newlines allowed) must pass the shared text rules. The cell forwards `MailOp::Send`, or `MailOp::SendRejected` with the reason, and the base answers every send with `sendMailResult` (CM 79). The base runs, in order:

1. the mail-send flood limit: 3 sends back to back, then one every 10 seconds. An over-limit send is dropped before anything else runs, but still answered with `MAILRESULT_NoRecipients` (the client shows "Gate-mail message was not sent.") on every press, because the client disables its Send button on each press and only a result tells the player the press did nothing. The explanatory line "You are sending messages too quickly." follows at most once every 5 seconds;
2. the cell's refusal, if there was one;
3. alias bits in `RecipientFlags` (vault, team, command): refused until the Bank and organizations campaigns land them;
4. attachments: cash, COD or an item with two or more recipients is `MAILRESULT_AttachmentsAndMultipleRecipients`; the attachment checks that need no database follow (see [Sending with an attachment](#sending-with-an-attachment-ss-m2));
5. delivery, in one transaction: each name resolves against every character, online or not (an exact match first, then a case-insensitive one if exactly one character has it), repeats collapse to one copy, a recipient whose contact-list Ignore list holds the sender's name (case-insensitively, read from the database, so offline recipients count too; D-SS15, SS-C1) is skipped, and a recipient with 100 or more open (not archived) messages is skipped. The Ignore check and the count run under a `FOR UPDATE` lock on each recipient's player row, so two senders cannot both take the last slot.

A send that reaches at least one recipient is `MAILRESULT_Sent`; `FailedRecipients` names everyone it did not reach, and a feedback line gives the reason for each ("no such character", "more than one character matches", "gate-mail box is full"; a recipient who ignores the sender gets the same sentence a tell or duel challenge gets, "X is not accepting your messages."). A send that reaches nobody is `MAILRESULT_NoRecipients` with the same list and line. The sender's id and the name stored on the sender's own player row go into `sender_id` and `sender_name`. These limits are project policy, not recovered data (social-systems D-SS03, D-SS05, D-SS12 to D-SS14).

### Sending with an attachment (SS-M2)

A send may carry gift cash, one item, or both, or be COD: an item plus a price the recipient pays. Any attachment limits the send to one recipient. Before any SQL the base refuses negative cash, an item quantity without an item or an item without a quantity of at least 1, COD without an item, and COD without a price above zero. The shipped client refuses the same cases itself (`mail-wire-formats.md` M-Q2), so only a modified client reaches these refusals.

Delivery then runs in the same transaction as a text send, with these steps added:

1. With an item attached, the sender's inventory advisory locks are taken, then the item row is locked `FOR UPDATE`, before the player rows. This is the shared inventory lock order in `crates/base-session/src/base/crafting/inventory_locks.rs`. `ItemId` is the inventory instance id (M-Q2), looked up under the sender's own `character_id`.
2. After the recipient checks pass, the item must be in the sender's main bag (the same allowlist trade uses, so equipped, bandolier, mission, crafting, vault and buyback items are refused), not bound, and at least as large as the quantity asked for. A failure is `MAILRESULT_ItemNotAvailable`. Vault items (containers 17-20) and buyback items (16) get their own reason and feedback line: the owner's rule (2026-09-27, Bank campaign) is that vendors, trade, crafting and mail see only the backpack.
3. The sender pays 25 naquadah postage plus the gift cash, in one conditional `UPDATE`. A COD sender pays the postage only. If the balance cannot cover it, the send is `MAILRESULT_NotEnoughCash` and nothing is written. The postage is a sink (D-SS02).
4. The mail row is inserted with `cash` (the gift or the COD price) and, for COD, `MAIL_COD` in `flags`.
5. The item moves into escrow in `sgw_gate_mail_item`. A whole stack moves as the row itself, keeping its instance id. For part of a stack, the sender's row is decremented and the split-off quantity gets a fresh id from `sgw_inventory_item_id_seq`. Every instance column is kept. The item leaves `sgw_inventory`, so it appears in neither player's bags until a take moves it back (D-SS08). Whatever mail puts back into an inventory (a take, a return, a COD delivery) lands in the backpack, never a vault container (owner rule, 2026-09-27).

Any failure rolls the whole send back: the balance, the stack, the mail and escrow stay as they were, and the player gets `sendMailResult` plus a feedback line naming the reason. After a commit the sender's client gets `onCashChanged` with the balance read inside the transaction. For an item it also gets `onRemoveItem` when the whole row left the bag, then the full `onUpdateItem` list.

The recipient's `onMailHeaderInfo` carries the cash on the header, `MAIL_COD` in the header flags, and one `MessageAttachment` per mail that holds an item (see [MessageAttachment](#messageattachment)).

`deleteMailMessage` refuses a mail that still holds an item, gift cash or an unpaid COD price. The row and its escrow row are left unchanged, no `onMailHeaderRemove` is sent, and a feedback line tells the player to take the attachment or return the mail first. Deleting such a mail would destroy the escrowed value, because the escrow row cascades with its mail. Mail with nothing attached deletes as before. The client's own UI never runs this check; it relies on the server.

### Taking attachments, paying COD, returning (SS-M3)

Four cell methods act on one mail: `takeCashFromMailMessage` (CM 49), `takeItemFromMailMessage` (CM 50), `payCODForMailMessage` (CM 51) and `returnMailMessage` (CM 47). The cell forwards each to the base with the caller's `player_id` from its own entity; the only thing taken from the payload is the mail id. Each runs in one transaction (`crates/base-methods/src/base/world_entry/methods/mail/`, `take.rs`, `cod.rs`, `return_.rs`), and each takes its locks in the same order (`claim.rs`):

1. the caller's inventory advisory locks (the same keys and order as the send, crafting and the move path);
2. the mail row, `FOR UPDATE`, found by `mail_id` **and** the caller's `character_id`, so nobody can act on another player's mail;
3. the escrow row and any inventory rows;
4. `sgw_player` rows, `FOR UPDATE`, ascending.

Two requests for one mail (a double click, or take-cash and take-item in one bundle) queue on the mail row, and the second re-reads it after the first commits. Every write is also conditional on the state it was decided on, with the changed-row count checked.

- **Take cash.** Refused while the mail is an unpaid COD: its `cash` is the price, not a gift. Otherwise `cash` is zeroed and the owner credited, once. A credit that would take the balance past `i32::MAX` is refused and the cash stays in the mail. The client gets `onCashChanged` and the refreshed header.
- **Take item.** Refused while the mail is an unpaid COD. The server chooses the destination: the first free slot of the caller's main bag (`INV_MAIN`), reserved under the bag's advisory lock. `ContainerId` and `SlotId` are never read, because the shipped client fills them with uninitialised stack ([`mail-wire-formats.md`](../reverse-engineering/findings/mail-wire-formats.md) M-Q5). An item never goes to a vault or any other container. A full bag leaves the item in escrow and says so. The escrow row is restored into `sgw_inventory` with its instance id and every instance column, then deleted. The client gets the full inventory list and the refreshed header.
- **Pay COD.** The price is read from the stored mail. The payer is debited; `MAIL_COD` is cleared and `cash` zeroed in the same statement, so the price can never be taken as gift cash and the delete guard (which keys on `cash = 0`) works once the item is taken. `cod_paid` is set, so the paid mail can no longer be returned: the item is the buyer's, and the seller already has the price. The price is delivered to the sender as a new mail carrying the cash, so it arrives whether the sender is online or not. That payment mail is server mail: `sender_id` is NULL (so it cannot be returned), `sender_name` is the payer's stored name, the subject is "COD payment: " and the original subject, and it is exempt from the mailbox cap (D-SS03). The item is then taken with an ordinary take (D-SS09). If the COD's sender no longer exists (their character was deleted, and the foreign key set `sender_id` NULL), paying cancels the COD instead: nothing is charged, the price is zeroed, and the item becomes an ordinary take. Without that, the item would be stranded, because nobody could be paid and nobody returned to.
- **Return.** Allowed for a mail that is not archived, has not been returned already, is not a paid COD, and has a player sender (D-SS10). The mail is re-addressed to the stored `sender_id`, never to `sender_name`. The returner becomes its sender, `returned` is set so it can never bounce back, it arrives unread with a fresh `sent_time`, and an unpaid COD is cancelled with its price zeroed. The escrow row is keyed by `mail_id`, so the item travels with the mail. The returner's client drops the header.

After a take or a payment the client gets `onMailHeaderRemove` and then `onMailHeaderInfo` with that one mail as it now stands. The client upserts headers by id (M-Q7) and fills the attachment fields only from an attachment row, so removing first guarantees that a taken item's icon does not linger.

Archiving an unpaid COD is refused ("Pay for or return this COD delivery before archiving it."), and the mail stays in the inbox: archived mail can be neither returned nor expired, so an archived unpaid COD would otherwise strand the seller's item for good. A paid COD archives normally.

Every refusal answers with a feedback line on the first press and logs `mail.op_refused` with its `reason`. A request naming a mail the caller does not own also removes that stale header from the caller's list.

### Server and GM mail (SS-U1)

Every mail the server sends goes through one writer, `send_system_mail_tx` (in the caller's transaction) or `send_system_mail` (in its own), in [`mail/system/`](../../crates/base-methods/src/base/world_entry/methods/mail/system/). Its callers are Black Market payouts (BM-02b), the Gate Mail Clerk's content action (SS-U3) and the GM `.mail` command. A system mail:

- has no sender character: `sender_id` is NULL and `sender_name` is a label such as "Black Market". It therefore cannot be returned (D-SS10);
- charges no postage and never carries COD;
- ignores the 100-message cap, so a payout is never lost to a full mailbox (D-SS03). The log records the recipient's open count and `over_cap`;
- reaches offline recipients, because it touches only the database.

It carries cash (0 to 2,147,483,647), no item, or one item:

- **Minted:** a new instance of an item type, 1 up to the type's stack size, created in escrow with `grant_item`'s defaults (durability 100, the template's charges and ammo types, not bound). `source_character_id` is 0, meaning no character sent it.
- **Existing instance:** a row the server already holds for a player, moved whole into escrow with every instance column kept. Server-held means the auction container (18), which no player move reaches. The caller names the owner it expects. A row in any other container, a row owned by someone else, or a bound row mailed to anyone but its owner is refused, and nothing is written.

The GM tools use the same writer (see [commands](../commands.md)):

- `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]` mints the cash and the item into a system mail from the GM's name, to the GM or to `<name>`, online or not.
- With `cod <n>`, the mail comes from the GM's character instead (`sender_id` is the GM, `MAIL_COD`, the price in `cash`), so the payment comes back to the GM. It needs an item and no cash, and charges no postage.
- `.mailbox [name]` reports open and archived counts, system mail, and what is in escrow: items, gift cash and unpaid COD.
- `.mail_expire <mailId>` is refused with a feedback line until SS-M4 adds `expires_at`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Request mail headers | DONE | `requestMailHeaders` → `MailOp::RequestHeaders` → `SELECT … FROM sgw_gate_mail` → `onMailHeaderInfo` (CM 76). Only the requested list: `bArchive` 0 returns inbox mail, 1 archived mail |
| Read mail body | DONE | `requestMailBody` → `MailOp::RequestBody` → `onMailRead` (CM 78); also stamps `read_time` on first read, owner-scoped. `ToText` is the recipient's stored name |
| Delete mail | DONE | `deleteMailMessage` → `MailOp::Delete` → `onMailHeaderRemove` (CM 77). A mail that still holds an item, gift cash or an unpaid COD is refused with a feedback line and kept (SS-M2) |
| Archive mail | DONE | `archiveMailMessage` → `MailOp::Archive` → `onMailHeaderRemove` (CM 77). An unpaid COD is refused with a feedback line and stays in the inbox (SS-M3): archived mail cannot be returned and never expires, so it would strand the seller's item |
| Server-generated mail | DONE | `send_system_mail` / `send_system_mail_tx` (SS-U1): cash, a minted item or a server-held instance, no postage, no COD, not returnable. See [Server and GM mail](#server-and-gm-mail-ss-u1) |
| GM mail tools | DONE | `.mail`, `.mailbox`; `.mail_expire` refused until SS-M4 |
| Send mail (player compose) | DONE (text only) | `sendMailMessage` (CM 44) → `MailOp::Send` → one row per recipient → `sendMailResult` (CM 79). See [Sending a text mail](#sending-a-text-mail-ss-m1) |
| Cash, item or COD attachment on send | DONE | One recipient; 25 naquadah postage; item into escrow (`sgw_gate_mail_item`); one transaction. See [Sending with an attachment](#sending-with-an-attachment-ss-m2) |
| Return to sender | DONE | `returnMailMessage` (CM 47) → `MailOp::Return`. To the stored `sender_id`, once; not archived or server mail; COD cancelled. See [Taking attachments](#taking-attachments-paying-cod-returning-ss-m3) |
| Cash attachment claim | DONE | `takeCashFromMailMessage` (CM 49) → `MailOp::TakeCash`. Once; never from an unpaid COD; overflow-checked |
| Item attachment claim | DONE | `takeItemFromMailMessage` (CM 50) → `MailOp::TakeItem`. First free main-bag slot chosen by the server; the client's container and slot are ignored; a full bag keeps the item in escrow |
| Cash On Delivery | DONE | `payCODForMailMessage` (CM 51) → `MailOp::PayCod`. Stored price; the payment is mailed to the sender |
| New mail notification | STUB | `onNewMail`, `notifyPlayersOfNewMail` not wired |
| Multiple recipients | DONE | Up to 10 per text mail, de-duplicated; one row each |
| Send result feedback | DONE | `sendMailResult` (CM 79) answers every send, flood-limited ones included, with `FailedRecipients`. A feedback line with the reason follows every refusal, except that the flood line is sent at most once every 5 seconds |

## Entity Definition (SGWMailManager.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `mailMessages` | PYTHON | CELL_PRIVATE | Cached mail messages |
| `pendingMailMessages` | PYTHON | CELL_PRIVATE | Messages awaiting DB confirmation |
| `lastMailGetTime` | FLOAT | CELL_PRIVATE | Rate limiting: last header request time |
| `haveMailMessages` | UINT8 | CELL_PRIVATE | Flag: has unread mail |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onMailHeaderInfo` | ResetCategory, bArchive, ARRAY\<MessageHeader\>, ARRAY\<MessageAttachment\> | Mail header list |
| `onMailHeaderRemove` | MailId | Single header removed |
| `onMailRead` | MailId, BodyText, BodyId, ToText | Mail body content |
| `sendMailResult` | ResultCode, FailedRecipients, FailedRecipientFlags | Send outcome |

### Cell Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `requestMailHeaders` | YES | bArchive | Fetch mail list |
| `sendMailMessage` | YES | RecipientFlags, Recipients, Subject, Body, Cash, bCOD, ItemId, ItemQuantity | Send mail |
| `archiveMailMessage` | YES | MailId | Move to archive |
| `deleteMailMessage` | YES | MailId | Delete mail |
| `returnMailMessage` | YES | MailId | Return to sender |
| `requestMailBody` | YES | MailId | Fetch body text |
| `takeCashFromMailMessage` | YES | MailId | Claim cash attachment |
| `takeItemFromMailMessage` | YES | MailId, ContainerId, SlotId | Claim item attachment. ContainerId and SlotId are garbage in the shipped client (`mail-wire-formats.md` M-Q5), so the server ignores them and places the item in the caller's first free main-bag slot (SS-M3) |
| `payCODForMailMessage` | YES | MailId | Pay COD fee |
| `onNewMail` | NO | (none) | Server notification of new mail |

### Base Methods

| Method | Args | Purpose |
|--------|------|---------|
| `notifyPlayersOfNewMail` | ARRAY\<WSTRING\> Recipients | Notify recipients of new mail |

## Wire Format

### MessageHeader

Recovered and implemented as `mail::MailHeader`, serialized by `mail::serialize_on_mail_header_info`. The full `onMailHeaderInfo` envelope is:

```
UINT8  ResetCategory        -- always 0
UINT8  bArchive             -- echoed from the request
UINT32 headerCount
  repeated headerCount times:
    INT32   id              -- mail_id
    WSTRING fromText        -- sender_name
    INT32   fromId          -- sender_id (0 when the sender is the system)
    WSTRING subjectText     -- subject
    INT32   subjectId       -- always 0 (server sends literal subjects, not string ids)
    INT32   cash            -- attached cash (clamped from the bigint DB column)
    FLOAT   sentTime        -- unix epoch seconds
    FLOAT   readTime        -- unix epoch seconds; 0 = unread
    INT32   flags
UINT32 attachmentCount
  repeated attachmentCount times:
    MessageAttachment (below)
```

### MessageAttachment

Recovered from the client's `onMailHeaderInfo` decode (`mail-wire-formats.md` M-Q4) and
`entities/defs/alias.xml:103-111`, all fields `INT32`:

```
INT32 id            -- mail_id; joins the attachment to its header row
INT32 itemId        -- the item's TYPE (design) id, like InvItem.dbid; see below
INT32 stackSize
INT32 durability    -- UNRESOLVED: the client's own UI decode reads this as a float even though
                    --   alias.xml declares INT32; wire stays INT32 per alias.xml until a capture
                    --   of a non-empty attachment settles it
INT32 charges
```

The server writes one attachment per mail that holds an escrowed item (SS-M2), built by
`MailAttachment` in `crates/wire/src/cell/mail/mod.rs`. Cash and COD need no attachment
record: they ride on the header's `cash` and `flags`.

`itemId` here is the item's type id, not the escrowed instance id. `sendMailMessage`'s `ItemId`
is the instance id (M-Q2), but that is the sender's side. The recipient's client builds the
attachment's name, icon, tech comp and quality from `itemId` alone (`GateMail.lua` calls
`mailGetItemAttachmentInfo` and reads `.Name`, `.Icon`, `.TechComp` and `.Quality`), and it has no
inventory record for an instance it does not own, so only a type id can resolve. This is an
inference from the client Lua, not a decompile of `mailGetItemAttachmentInfo`; a blank icon on a
received attachment in UAT would contradict it. The instance id stays on the server.

### onMailRead

```
INT32   MailId
WSTRING BodyText
INT32   BodyId              -- always 0
WSTRING ToText              -- recipient display name
```

## Mail Read Flow (implemented)

```
Client: requestMailHeaders(bArchive)
  |-> Cell: resolve player_id, CellToBaseMsg::MailRequest{RequestHeaders}
  |-> Base: SELECT ... FROM sgw_gate_mail WHERE character_id = $1 ORDER BY mail_id DESC
  |-> Base: onMailHeaderInfo(...) straight to the client

Client: requestMailBody(mailId)
  |-> Base: SELECT message; UPDATE read_time if still 0
  |-> Base: onMailRead(mailId, body, 0, recipientName)

Client: deleteMailMessage(mailId) / archiveMailMessage(mailId)
  |-> Base: DELETE / flag update, then onMailHeaderRemove(mailId)
```

Ownership is enforced by a `character_id = $2` predicate on every mutating query (`DELETE … WHERE mail_id = $1 AND character_id = $2`, and the same shape for archive), so a forged `mailId` cannot reach another player's mail. Both arms log a warning when `rows_affected == 0`.

Archiving is a flag flip, not a move: `UPDATE sgw_gate_mail SET flags = flags | 1`. Bit 0 of `flags` means archived.

The header query filters by `bArchive` (SS-M1), and the delete is refused while the mail holds an attachment (SS-M2).

## Mail Send Flow

See [Sending a text mail](#sending-a-text-mail-ss-m1) and [Sending with an attachment](#sending-with-an-attachment-ss-m2). The recipient is not notified yet (`onNewMail`, SS-M4).

## Persistence

Two tables. The mail itself, [`db/sgw/Mail/Tables/sgw_gate_mail.sql`](../../db/sgw/Mail/Tables/sgw_gate_mail.sql):

```sql
CREATE TABLE sgw_gate_mail (
    mail_id      integer NOT NULL,
    character_id integer NOT NULL,   -- recipient
    sender_id    integer,
    subject      character varying(128) NOT NULL,
    message      text NOT NULL,
    cash         bigint DEFAULT 0 NOT NULL,
    sent_time    integer NOT NULL,   -- unix epoch seconds
    read_time    integer NOT NULL,   -- unix epoch seconds; 0 = unread
    flags        integer DEFAULT 0 NOT NULL,
    item_id      integer,
    sender_name  character varying(128) NOT NULL,
    returned     boolean DEFAULT false NOT NULL,  -- SS-M3: returned once, never again
    cod_paid     boolean DEFAULT false NOT NULL   -- SS-M3: a paid COD is the buyer's; never returned
);
```

One row per recipient: a multi-recipient send fans out to N rows. `cash` is the gift or, with `MAIL_COD` in `flags`, the COD price; it cannot be negative (`sgw_gate_mail_cash_nonnegative_chk`). `item_id` is legacy and stays NULL; the attached item lives in the escrow table.

The escrowed item, [`db/sgw/Mail/Tables/sgw_gate_mail_item.sql`](../../db/sgw/Mail/Tables/sgw_gate_mail_item.sql) (SS-M2, D-SS08), at most one row per mail (primary key `mail_id`):

- every instance column of `sgw_inventory` (`item_id`, `type_id`, `stack_size`, `charges`, `durability`, `flags`, `bound`, `ammo`, `cur_ammo_type`, `ammo_type`, `ammo_types`);
- `source_character_id` and `escrowed_at`, for forensics;
- `UNIQUE (item_id)` and `item_id >= 10000`, as in `sgw_inventory`.

It is a standalone table, not `INHERITS (sgw_inventory_base)`, so no inventory query can see escrow rows. `mail_id` references `sgw_gate_mail` with `ON DELETE CASCADE`: deleting a character, which cascades its mail, is not blocked, and the player's own delete is refused in the application instead. **Known gap:** deleting a recipient character therefore destroys any item and COD mailed to it; returning them to the sender is an owner decision.

## Data References

- **Custom types**: `MessageHeader` (recovered — see [Wire Format](#wire-format)), `MessageAttachment` (recovered — see [Wire Format](#wire-format); `durability`'s INT32-vs-float wire type is the one still-open field)
- **Database**: `sgw_gate_mail`, `sgw_gate_mail_item`
- **Enumerations**: `RecipientFlags` (individual, guild, etc.)

## Remaining Work

1. **RecipientFlags** — the vault and organization aliases are refused until the Bank and organizations campaigns land them
2. **New-mail notification and expiry (SS-M4)** — a sender is not told when a payment or a returned mail arrives; it shows on their next header request
3. **Rate limiting** — the `lastMailGetTime` throttle on header requests is not implemented

## Related Docs

- [inventory-system.md](inventory-system.md) - Items attached to mail
- [organization-system.md](organization-system.md) - Guild-wide mail recipients
