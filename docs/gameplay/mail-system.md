---
title: "Mail System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Mail System

> **Last updated**: 2026-07-25
> **Status**: Read side implemented (headers / body / delete / archive). Player-to-player *sending* is still a stub; server-generated mail works.

## Overview

The mail system enables asynchronous message delivery between players with support for item attachments, currency attachments, Cash On Delivery (COD), archiving, and return-to-sender. Mail is stored server-side and retrieved on demand. The system supports multiple recipients and tracks read/unread state.

The `SGWMailManager` interface in `entities/defs/interfaces/SGWMailManager.def` defines the complete protocol.

The Rust implementation forwards every mail request from the cell to the base, because mail needs database access and the DB pool lives on the BaseApp. [`crates/services/src/cell/mail.rs`](../../crates/services/src/cell/mail.rs) packages the request as `CellToBaseMsg::MailRequest { op: MailOp }`; [`crates/services/src/base/world_entry/methods/mail/mod.rs`](../../crates/services/src/base/world_entry/methods/mail/mod.rs) runs the query and sends the result straight back to the client.

Note that the *stub* status of `sendMailMessage` applies only to the player-facing compose path. A server-generated mail helper exists — `send_mail_to_player`, used by the Black Market expiry sweep to pay sellers and deliver won items — but it lives on the **unmerged** branch `feat/571-black-market-phase1` (PR #586). On `main` there is no server-generated mail sender, so nothing writes to `sgw_gate_mail` at all and the read path below has no way to acquire rows outside of manual seeding.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Request mail headers | DONE | `requestMailHeaders` → `MailOp::RequestHeaders` → `SELECT … FROM sgw_gate_mail` → `onMailHeaderInfo` (CM 76) |
| Read mail body | DONE | `requestMailBody` → `MailOp::RequestBody` → `onMailRead` (CM 78); also stamps `read_time` on first read |
| Delete mail | DONE | `deleteMailMessage` → `MailOp::Delete` → `onMailHeaderRemove` (CM 77) |
| Archive mail | DONE | `archiveMailMessage` → `MailOp::Archive` → `onMailHeaderRemove` (CM 77) |
| Server-generated mail | BRANCH ONLY | `send_mail_to_player`, used by the Black Market settlement path. Exists on `feat/571-black-market-phase1` (PR #586), **not on `main`** |
| Send mail (player compose) | STUB | `sendMailMessage` (CM 44) logs `UNIMPLEMENTED` and drops |
| Return to sender | STUB | `returnMailMessage` logs `UNIMPLEMENTED` |
| Cash attachment claim | STUB | `takeCashFromMailMessage` logs `UNIMPLEMENTED` (the `cash` column is populated and read back in headers) |
| Item attachment claim | STUB | `takeItemFromMailMessage` logs `UNIMPLEMENTED` (the `item_id` column is populated) |
| Cash On Delivery | STUB | `payCODForMailMessage` logs `UNIMPLEMENTED` |
| New mail notification | STUB | `onNewMail`, `notifyPlayersOfNewMail` not wired |
| Multiple recipients | STUB | Depends on the send path |
| Send result feedback | STUB | `sendMailResult` (CM 79) index is reserved but never emitted |

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
| `takeItemFromMailMessage` | YES | MailId, ContainerId, SlotId | Claim item attachment |
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
```

### MessageAttachment

Not yet decompiled. The server always writes `attachmentCount = 0` — attachment claim (`takeItemFromMailMessage` / `takeCashFromMailMessage`) is unimplemented, so the client is never given an attachment record to act on.

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

**Known gap:** the header query ignores `bArchive` — it returns every row for the character regardless of the archive bit, and echoes `bArchive` back unchanged. Archived mail therefore still appears in the inbox listing.

## Mail Send Flow (not implemented)

The intended flow, per the entity definitions:

```
Client: sendMailMessage(RecipientFlags, Recipients[], Subject, Body, Cash, bCOD, ItemId, Quantity)
  |
  v
Server:
  |-> Validate: recipients exist, sufficient cash/items
  |-> Remove cash and/or item from sender inventory
  |-> Create mail record in database
  |-> Send sendMailResult(resultCode, failedRecipients) to sender
  |-> For each valid recipient:
       |-> notifyPlayersOfNewMail(recipientNames)
            |-> onNewMail() -- triggers "you have mail" notification
```

## Persistence

Single table, [`db/sgw/Mail/Tables/sgw_gate_mail.sql`](../../db/sgw/Mail/Tables/sgw_gate_mail.sql):

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
    sender_name  character varying(128) NOT NULL
);
```

One row per recipient — a multi-recipient send would fan out to N rows.

## Data References

- **Custom types**: `MessageHeader` (recovered — see [Wire Format](#wire-format)), `MessageAttachment` (not yet decompiled)
- **Database**: `sgw_gate_mail`
- **Enumerations**: `RecipientFlags` (individual, guild, etc.)

## Remaining Work

1. **Send path** — `sendMailMessage` is the single largest gap; everything downstream of it (result codes, new-mail notification, multi-recipient fan-out) is blocked on it
2. **Attachment claim** — `takeItemFromMailMessage` / `takeCashFromMailMessage`, plus the `MessageAttachment` wire format
3. **Send result codes** — enumerate `ResultCode` values in `sendMailResult`
4. **RecipientFlags** — what flags control recipient targeting (individual, guild, etc.)
5. **COD flow** — how `payCODForMailMessage` transfers cash to the original sender
6. **Rate limiting** — the `lastMailGetTime` throttle on header requests is not implemented

## Related Docs

- [inventory-system.md](inventory-system.md) - Items attached to mail
- [organization-system.md](organization-system.md) - Guild-wide mail recipients
