---
name: reference-mail-wire-spec
description: Authoritative wire shapes for mail messages — extracted from SGWMailManager.def
metadata:
  type: reference
---

Source: `entities/defs/interfaces/SGWMailManager.def` (entity def is the
data BigWorld code-gen reads, so it's authoritative for the wire shape;
cross-checks against Ghidra `Event_NetOut_*` confirm field count).

CellMethod indices (client → server):
- 43 `requestMailHeaders(UINT8 bArchive)`
- 44 `sendMailMessage(INT32 RecipientFlags, ARRAY<WSTRING> Recipients, WSTRING Subject, WSTRING Body, INT32 Cash, UINT8 bCOD, INT32 ItemId, INT32 ItemQuantity)`
- 45 `archiveMailMessage(INT32 MailId)`
- 46 `deleteMailMessage(INT32 MailId)`
- 47 `returnMailMessage(INT32 MailId)`
- 48 `requestMailBody(INT32 MailId)`
- 49 `takeCashFromMailMessage(INT32 MailId)`
- 50 `takeItemFromMailMessage(INT32 MailId, INT32 ContainerId, INT32 SlotId)`
- 51 `payCODForMailMessage(INT32 MailId)`

ClientMethod indices (server → client):
- 76 `onMailHeaderInfo(UINT8 ResetCategory, UINT8 bArchive, ARRAY<MessageHeader> Headers, ARRAY<MessageAttachment> Attachments)`
- 77 `onMailHeaderRemove(INT32 MailId)`
- 78 `onMailRead(INT32 MailId, WSTRING BodyText, INT32 BodyId, WSTRING ToText)`
- 79 `sendMailResult(UINT8 ResultCode, ARRAY<WSTRING> FailedRecipients, INT32 FailedRecipientFlags)`

BaseMethod (notification fan-out):
- `notifyPlayersOfNewMail(ARRAY<WSTRING> Recipients)`

Notes for the auditor:
- `payCODForMailMessage` has NO client-supplied price field — the COD
  amount MUST come from `sgw_gate_mail.cash` (server side). If a future
  PR widens this arg list, that's a CAT-G-05 regression.
- `takeCashFromMailMessage` and `takeItemFromMailMessage` are two separate
  indices that share the underlying mail row — they need shared atomicity
  (CAT-G-04 — double-take race).
- `returnMailMessage` only carries MailId. The return destination must be
  resolved from `sender_id` (integer), not `sender_name` (free text).
