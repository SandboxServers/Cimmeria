# Mail System

> **Last updated**: 2026-03-01
> **Status**: ~0% implemented

## Overview

The mail system enables asynchronous message delivery between players with support for item attachments, currency attachments, Cash On Delivery (COD), archiving, and return-to-sender. Mail is stored server-side and retrieved on demand. The system supports multiple recipients and tracks read/unread state.

The `SGWMailManager` interface in `entities/defs/interfaces/SGWMailManager.def` defines the complete protocol. No Python implementation exists.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Send mail | STUB | `sendMailMessage` defined with full args |
| Request mail headers | STUB | `requestMailHeaders` with archive flag |
| Read mail body | STUB | `requestMailBody` |
| Delete mail | STUB | `deleteMailMessage` |
| Archive mail | STUB | `archiveMailMessage` |
| Return to sender | STUB | `returnMailMessage` |
| Cash attachment | STUB | `Cash` arg in `sendMailMessage`, `takeCashFromMailMessage` |
| Item attachment | STUB | `ItemId`/`ItemQuantity` in send, `takeItemFromMailMessage` |
| Cash On Delivery | STUB | `bCOD` flag, `payCODForMailMessage` |
| New mail notification | STUB | `onNewMail`, `notifyPlayersOfNewMail` |
| Multiple recipients | STUB | `ARRAY<WSTRING> Recipients` in send |
| Send result feedback | STUB | `sendMailResult` with failed recipients |

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

```
TODO: Decompile from client binary
Fields likely include: MailId, SenderName, Subject, Timestamp, Read flag, HasAttachment, IsCOD
```

### MessageAttachment

```
TODO: Decompile from client binary
Fields likely include: MailId, ItemId, ItemQuantity, Cash amount
```

## Mail Send Flow

```
Client: sendMailMessage(RecipientFlags, Recipients[], Subject, Body, Cash, bCOD, ItemId, Quantity)
  |
  v
Server (Cell):
  |-> Validate: recipients exist, sufficient cash/items
  |-> Remove cash and/or item from sender inventory
  |-> Create mail record in database
  |-> Send sendMailResult(resultCode, failedRecipients) to sender
  |-> For each valid recipient:
       |-> Base: notifyPlayersOfNewMail(recipientNames)
            |-> Cell: onNewMail() -- triggers "you have mail" notification
```

## Data References

- **Custom types**: `MessageHeader`, `MessageAttachment` (defined in entity type system)
- **Database**: Mail table schema needs to be created
- **Enumerations**: `RecipientFlags` (individual, guild, etc.)

## RE Priorities

1. **Wire format** - Decompile `MessageHeader` and `MessageAttachment` structures
2. **Send result codes** - Enumerate `ResultCode` values in `sendMailResult`
3. **RecipientFlags** - What flags control recipient targeting (individual, guild, etc.)
4. **COD flow** - How `payCODForMailMessage` transfers cash to original sender
5. **Rate limiting** - How `lastMailGetTime` throttles header requests

## Related Docs

- [inventory-system.md](inventory-system.md) - Items attached to mail
- [organization-system.md](organization-system.md) - Guild-wide mail recipients
