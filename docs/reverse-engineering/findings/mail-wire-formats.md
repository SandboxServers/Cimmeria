# Mail System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `SGWMailManager.def`, `alias.xml`

---

**Interface**: `SGWMailManager` (implemented by `SGWPlayer`)

### Client → Server (Exposed Cell Methods)

#### `sendMailMessage` — Send Mail

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `RecipientFlags` | `INT32` | 4B | Recipient type flags |
| `Recipients` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) | Recipient names |
| `Subject` | `WSTRING` | 4B len + N×2B | Mail subject |
| `Body` | `WSTRING` | 4B len + N×2B | Mail body text |
| `Cash` | `INT32` | 4B | Attached currency |
| `bCOD` | `UINT8` | 1B | Cash on delivery flag |
| `ItemId` | `INT32` | 4B | Attached item instance ID |
| `ItemQuantity` | `INT32` | 4B | Stack size of attached item |

#### `requestMailHeaders` — Fetch Mail List

| Field | Type | Size |
|-------|------|------|
| `bArchive` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `requestMailBody` — Fetch Mail Content

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `deleteMailMessage` / `archiveMailMessage` / `returnMailMessage`

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `takeCashFromMailMessage` / `payCODForMailMessage`

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `takeItemFromMailMessage` — Take Attached Item

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `MailId` | `INT32` | 4B | |
| `ContainerId` | `INT32` | 4B | **Unreliable; ignored by the server.** The shipped client sends uninitialised stack values (M-Q5 below). The server picks the destination. |
| `SlotId` | `INT32` | 4B | **Unreliable; ignored by the server** (same reason) |

**Total wire size**: 1B header + 12B = **13 bytes**

### Server → Client

#### `onMailHeaderInfo` — Mail Header List

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `ResetCategory` | `UINT8` | 1B | Clear existing headers flag |
| `bArchive` | `UINT8` | 1B | Archive vs inbox |
| `MessageHeaders` | `ARRAY<MessageHeader>` | 4B count + N×MessageHeader | Mail headers |
| `MessageAttachments` | `ARRAY<MessageAttachment>` | 4B count + N×MessageAttachment | Attachment info |

**`MessageHeader` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `id` | `INT32` | 4B |
| `fromText` | `WSTRING` | 4B len + N×2B |
| `fromId` | `INT32` | 4B |
| `subjectText` | `WSTRING` | 4B len + N×2B |
| `subjectId` | `INT32` | 4B |
| `cash` | `INT32` | 4B |
| `sentTime` | `FLOAT` | 4B |
| `readTime` | `FLOAT` | 4B |
| `flags` | `INT32` | 4B |

**`MessageAttachment` FIXED_DICT layout** (20 bytes; confirmed by the `onMailHeaderInfo` wire
decoder, SS-E1 M-Q4, 2026-09-27; the field types follow `entities/defs/alias.xml:103-111`, the client's own shipped schema):

| Field | Type | Size |
|-------|------|------|
| `id` | `INT32` | 4B |
| `itemId` | `INT32` | 4B |
| `stackSize` | `INT32` | 4B |
| `durability` | `INT32` (see the M-Q4 note: the client's UI decode reads it as a float) | 4B |
| `charges` | `INT32` | 4B |

#### `onMailHeaderRemove` — Remove Mail from List

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onMailRead` — Mail Body Content

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `MailId` | `INT32` | 4B |
| `BodyText` | `WSTRING` | 4B len + N×2B |
| `BodyId` | `INT32` | 4B |
| `ToText` | `WSTRING` | 4B len + N×2B |

#### `sendMailResult` — Send Outcome

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ResultCode` | `UINT8` | 1B |
| `FailedRecipients` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |
| `FailedRecipientFlags` | `INT32` | 4B |

---

## Implementation Notes

- **Mail attachments**: Single item per mail (itemId + quantity). Cash can be attached separately. COD (Cash on Delivery) is supported.

---

## SS-E1 client evidence (2026-09-27)

> Static Ghidra RE against `SGW.exe` (no debugger), for the social-systems campaign's SS-E1 packet.
> Answers M-Q1–M-Q7 from `docs/analysis/social-systems/work-packets.md`. Method addresses below were
> reached by following the client's own CME event-signal plumbing (register fn → `TypedEmitInfo`
> constructor → `MemberCallback`/`FreeCallback` vtable → the bound `MailManager`/`GamePlayer` method),
> the same pattern documented in `spec.engine.cme-event-signal`.

### M-Q1 — `sendMailResult` result-code text (CLOSED, byte-exact)

`FUN_00d7d0e0` (the function named in the packet) is the generic `TypedEmitInfo` destructor
boilerplate, not the text mapper. The real subscriber is `MailManager`'s bound handler, found via
`MailManager`'s registration function `FUN_00e16e60` (binds 5 handlers) → the 4th binding
(`FUN_00e17cc0` → `FUN_00e17a10` → constructor) → **`Mercury__unknown_00e13b90`** (misnamed by the
xref-propagation heuristic; it is a `MailManager` method, not `Mercury`). It decodes `ResultCode`
(UINT8), `FailedRecipientFlags` (INT32 — decoded but not referenced by any display branch found),
`FailedRecipients` (`ARRAY<WSTRING>`), then switches on `ResultCode` with literal, hardcoded
client strings:

| `ResultCode` | Enum (`EMailResultCodes`) | Client text |
|---|---|---|
| 0 | `MAILRESULT_Sent` | If `FailedRecipients` is empty: "Gate-mail message sent." Else: "Gate-mail message sent.  Your message could not be delivered to the following recipients: " + the list, comma-joined. Either way the client also closes/resets the compose window (`FUN_00e13180`). |
| 1 | `MAILRESULT_NoRecipients` | "No valid recipients specified.  Gate-mail message was not sent." |
| 2 | `MAILRESULT_ItemNotAvailable` | "Attachment item not available.  Gate-mail message was not sent." |
| 3 | `MAILRESULT_AttachmentsAndMultipleRecipients` | "You cannot send a gate-mail message with cash or item attachments to multiple recipients." |
| 4 | `MAILRESULT_NotEnoughCash` | "You do not have enough naquadah to send that gate-mail message." |
| 5 | `MAILRESULT_VaultButNoItem` | "You cannot send a message without an item to your vault." |
| 6 | `MAILRESULT_VaultPlusCash` | "You cannot send a message with cash attached to your vault." |
| 7 | `MAILRESULT_SentToVault` | "Sending item to vault." (also closes/resets the compose window, like a success) |
| any other value | — | "Unknown mail error.  Gate-mail message was not sent." |

This is a clean 0–7 `switch` matching the Postgres enum's declaration order exactly (audit A-12),
which settles the wire-value mapping with HIGH confidence: the client is only internally
consistent if the wire byte equals the enum's declaration ordinal. **`FailedRecipients` is always
shown to the sender, even on an overall `Sent` result** — the packet's send-side handler must
still populate it whenever any recipient failed, even though the mail as a whole went out to the
others. `FailedRecipientFlags` is decoded but this function never uses it in a display branch; no
evidence it drives anything client-visible.

Evidence: `ghidra://SGW.exe@0x00e13b90` (handler body), `0x00e16e60` (registration, 5 handlers),
`0x00e17cc0`/`0x00e17a10` (subscribe chain), `0x00d7d0e0` (ruled out — boilerplate only).

### M-Q2 — recipient/alias parsing and `ItemId` semantics (CLOSED — blocking for SS-M2)

Traced the full native chain for both `mailSetItemAttachment(container, slot, quantity)`
(`0x00aa4690`) and `mailSendMessage(recipients, subject, body, cash, cod)` (`0x00ac6580`), both of
which forward into the same `MailManager` singleton (`FUN_00c66ad0()+0x8c+0x60`).

**`ItemId` is the item's own unique inventory-instance id, not a type id (HIGH confidence).**
`mailSetItemAttachment`'s native binding converts its 3 Lua args and calls
`FUN_00ad81f0(container, slot, quantity)` → `FUN_00e13900(mailMgr, container, slot, quantity)`.
Inside `FUN_00e13900`, **only `container` is used** — it is passed alone into
`FUN_00e1c530(invMgr, container)` → `FUN_00e1c4c0(invMgr, container)`, which is a classic MSVC
`std::map<int, T>::find` (red-black-tree traversal keyed by one scalar int). **`slot` (the second
Lua argument) is never read anywhere in this call chain.** A map keyed by a single scalar value
can only be an item-instance lookup (two stacks of the same item type must resolve to different
records for correct escrow/quantity accounting), so whatever value the Lua drag-drop handler
passes as "container" is functioning as a unique item id, not a container index. The resolved
item record is cached on the `MailManager` at offset `+0x34` (pointer) and `+0x38` (clamped
quantity, capped at the item's own max-stack field).

At send time, `ZipFileSystem__unknown_00e14910` (the full validator + RPC builder reached via
`mailSendMessage` → `FUN_00ad8100` → this function) builds the outgoing `sendMailMessage` fields
in this order: `RecipientFlags`, `Recipients`, `Subject`, `Body`, `Cash`, `bCOD`, then:

```c
if (this->pendingItem == 0) {
    ItemId = 0;
    ItemQuantity = 0;
} else {
    ItemId = *(int*)(this->pendingItem + 0xc);   // read directly off the per-instance item record
    ItemQuantity = *(int*)(this + 0x38);          // the clamped quantity from mailSetItemAttachment
}
```

`ItemId` is read from offset `+0xc` of the SAME per-instance record found by the scalar map
lookup above — this is the item's own id field on its native object, not a type/template id.
(The one residual gap: I did not independently re-derive that offset `+0xc` holds literally the
same scalar the map is keyed on, only that it is a per-instance field on a per-instance-keyed
record; either way it cannot be a shared type id.) **Recommendation for SS-M2: key escrow on the
inventory instance id, exactly as D-SS08 already assumes.**

**Alias-to-`RecipientFlags` parsing is entirely client-side (confirms the packet's open
question).** `ZipFileSystem__unknown_00e14910` splits the recipients string on `;` with
`wcstok_s`, trims each token's leading/trailing spaces, then calls `FUN_00e12f30(token,
&flagsAccumulator)`. If the token matches a known alias, its return sets bits in the
`RecipientFlags` accumulator and the raw token is **not** added to the `Recipients` array;
otherwise the literal name is appended to `Recipients`. **`RecipientFlags` and `Recipients` are
therefore always disjoint on the wire: `RecipientFlags` is a pure alias bitmask, `Recipients`
never contains anything but literal player names.** (I did not trace `FUN_00e12f30`'s internal
string table to confirm the exact bit each alias token maps to; treat the bit *values* — the
`EMailFlags` ordinals per audit A-12 — as still unconfirmed against a live capture, only the
*mechanism* is now settled.)

**Other client-side rules recovered from the same function, all HIGH confidence (byte-exact
strings, direct disassembly, no ambiguity):**

- **Recipient-token cap: 51** (`iVar12 < 0x33`), counting alias tokens and names together — "Too
  many recipients." beyond that. This is a genuine recovered client bound, distinct from D-SS05's
  policy cap of 10; keeping the stricter server-side 10 is still fine (it's well under the
  client's own ceiling).
- Exactly 1 token → proceed to build the RPC. **2+ tokens**: if the vault-alias bit is set →
  reject "You cannot send a message to both your vault and other people."; else if an item is
  attached → reject "You cannot send an item to more than one recipient."; else if cash != 0 →
  reject "You cannot send cash to more than one recipient."; else (plain text, no cash/item)
  → **proceed** — the shipped client allows multi-recipient plain-text mail (no attachment) up to
  the 51-token cap. D-SS05's 10-recipient cap for plain-text mail is a stricter *server* policy
  than the client enforces, not a recovered client behavior.
- If **any alias other than Vault** (Team/Command/CommandOfficers/CommandRankN) is set **and** an
  item is attached → reject "You cannot send an item to a (non-vault) alias." (Vault alone may
  carry an item — consistent with `MAILRESULT_VaultButNoItem`/`SentToVault`.)
- If **any** alias bit is set (**Vault included**) **and** cash != 0 → **always** reject "You
  cannot send naquadah to an alias." The shipped client never lets a sender attach cash to any
  alias, Vault included. Treat `MAILRESULT_VaultPlusCash` as a code the compose UI itself never
  triggers, not evidence that vault-plus-cash is a real supported combination — useful context for
  the Bank campaign's D-SS07 handoff.
- **COD requires item + cash > 0, both ways, confirmed straight from the client**: cash <= 0 with
  COD set → "You must specify an amount of cash greater than zero for messages with COD
  attachments."; cash > 0 with COD set but no item attached → "You must attach an item to COD
  messages." This independently confirms D-SS09's rule from the original client, not just the
  server-side policy proposal.
- Cash is pre-checked against the sender's own live balance client-side (a UX guard only, not
  authoritative — matches D-SS06's "server re-checks" requirement).

Evidence: `ghidra://SGW.exe@0x00aa4690` (`mailSetItemAttachment` binding), `0x00ad81f0` →
`0x00e13900` → `0x00e1c530` → `0x00e1c4c0` (item lookup chain), `0x00ac6580`
(`mailSendMessage` binding) → `0x00ad8100` → `0x00e14910` (`ZipFileSystem__unknown_00e14910`,
the full validator/RPC-builder).

### M-Q3 — `ExpiresHours` source and TTL (CLOSED)

The `onMailHeaderInfo` wire decoder (`Detail__unknown_00e15450`, see M-Q7 below) reads exactly 9
fields per `MessageHeader` row — `id, fromText, fromId, subjectText, subjectId, cash, sentTime,
readTime, flags` — **confirming there is no `ExpiresHours` field on the wire.** The header-record
constructor it calls, `FUN_00eb5ab0`, computes it purely client-side:

```c
// uVar1:uVar3 = a 64-bit time value obtained via Mercury__unknown_012379f6()
hours = __aulldiv(uVar1, uVar3, 3600, 0);   // /3600 = whole hours
record->expiresHours = 0x2d0 - (int)hours;  // 0x2d0 = 720
```

**`0x2d0` = 720 decimal = exactly 30 days.** This is a hardcoded client constant, matching
D-SS04's proposed fallback exactly. Confidence is HIGH on the constant itself; MEDIUM on the exact
time-base semantics of the division (the decompiler did not clearly show `sentTime` being passed
into `Mercury__unknown_012379f6()`, so whether the subtrahend is "hours elapsed since sent" or
some other time-since-epoch framing is not fully pinned down) — but either reading is consistent
with a 720-hour/30-day TTL counting down from `sentTime`. The same constructor also derives
`HasBeenRead` from comparing `readTime` against a small float constant, and decomposes a time
value into a `SYSTEMTIME`-style calendar breakdown (`sentDayOfWeek/Month/Day/Year/Hour/Minute`,
cached on the record) for the client's date display — confirming (again) that no additional wire
fields are needed for the "Sent: <date>" display either; it is entirely reconstructed client-side
from the wire `sentTime` float, as the existing research report already concluded.

**Recommendation for SS-M4: set `expires_at = sent_time + 30 days` exactly (D-SS04's fallback),
not a different value.**

Evidence: `ghidra://SGW.exe@0x00e15450` (wire decoder, field list), `0x00eb5ab0` (header-record
constructor, the `0x2d0`/`0xe10` constants), string `"ExpiresHours"` at `0x0195ef84`.

### M-Q4 — `MessageAttachment.id` join and byte layout (CLOSED, with a correction)

The same `onMailHeaderInfo` decoder reads each `MessageAttachments` entry as `id, itemId,
stackSize, durability, charges`, then immediately calls `FUN_00e12e70(headerList, id)` — a
find-header-by-id lookup — and writes `itemId/stackSize/durability/charges` onto the matching
header record. **Confirms `MessageAttachment.id` is the mail id, used to join the attachment to
its header row, exactly as this file's existing layout assumed.**

**UNRESOLVED discrepancy on `durability`.** The client's UI-side decode at `0x00e15450` reads `durability` into a `float` local, through the same helper it uses for the header's FLOAT `sentTime`/`readTime`. The client's own schema, `entities/defs/alias.xml:103-111`, declares it `INT32`, and BigWorld deserializes the wire by that alias before the UI code runs. So the float read is most likely a conversion after decoding, not a FLOAT on the wire. **Until a capture of a non-empty attachment settles it, the wire layout stays INT32 per `alias.xml`**, and serializers (SS-M2) must follow the alias. Both fields are 4 bytes, so the byte offsets are unaffected either way.

Evidence: `ghidra://SGW.exe@0x00e15450` (field-by-field decode, `"durability"` string decoded into
a `float` local), `0x00e12e70` (header lookup by id).

### M-Q5 — `takeItemFromMailMessage`'s `ContainerId`/`SlotId` (CLOSED — security-relevant)

**The shipped client's `mailTakeItem` native binding drops `ContainerId` and `SlotId` on the
floor and the wire call ends up carrying uninitialized stack garbage for both fields, not even a
reliable `-1,-1` sentinel.** Traced end to end:

1. `mailTakeItem(mailId, containerId, slotId)`'s Lua binding (`0x00aa4890`) reads and converts
   all 3 Lua arguments, but its call to the next native hop, `FUN_00ad8240(mailId)`, **passes only
   `mailId`** — the two converted container/slot values are computed and then discarded.
2. `FUN_00ad8240` → `ZipFileSystem__unknown_00e151e0(mailMgr, mailId)` — again, only `mailId` is
   passed as an explicit parameter.
3. Inside `ZipFileSystem__unknown_00e151e0`, the outgoing `takeItemFromMailMessage(MailId,
   ContainerId, SlotId)` RPC is built reading `ContainerId`/`SlotId` from `&stack0x00000008` and
   `&stack0x0000000c` — stack addresses **above this function's own locals**, never written by any
   parameter passed into this call chain. That is uninitialized/leftover stack memory, not a
   literal `-1` or any value under the game's control.

This is consistent with mail's write side never having been exercised end to end in 2009 (the
legacy Python server never implemented any of it either) — a latent, shipped-but-dead client bug.

**Recommendation for SS-M3 (strengthens CAT-G-03): do not attempt to interpret `ContainerId`/
`SlotId` at all, not even to special-case `-1,-1`.** The shipped client cannot reliably send
`-1,-1` or any other meaningful value here. Always place the taken item into the caller's own
inventory, first free main-container slot, exactly as if `(-1,-1)` had always been received, and
never branch server logic on the raw field values.

Evidence: `ghidra://SGW.exe@0x00aa4890` (Lua binding, drops args 2/3), `0x00ad8240` (mailId-only
forward), `0x00e151e0` (`ZipFileSystem__unknown_00e151e0`, reads uninitialized stack for
`ContainerId`/`SlotId`).

### M-Q6 — unsolicited `onMailHeaderInfo` (UNRESOLVED)

Not independently re-traced this session beyond confirming the wire decoder itself (M-Q3/M-Q4/
M-Q7) always upserts by id regardless of whether the mailbox UI is open. I did not find or check
a "mailbox window is closed" early-out, nor any new-mail icon/sound trigger distinct from the
window's own `onUpdateMailbox` refresh. Given `onNewMail` is confirmed dead (audit A-13) and no
other client method exists for "you have mail," D-SS11's plan (a feedback line, optionally
followed by a fresh `onMailHeaderInfo`) is not contradicted by anything found here, but it is not
independently confirmed either. Flag for a future pass if D-SS11's specific delivery mechanism
needs stronger evidence before SS-M4 ships.

### M-Q7 — `ResetCategory`/`bArchive` and the archive/inbox split (CLOSED)

The `onMailHeaderInfo` wire decoder is `Detail__unknown_00e15450` (found via `MailManager`'s
registration function `FUN_00e16e60`, 3rd of 5 handlers). Confirmed:

- **The client keeps two separate header lists** inside its `MailManager` state — one at the base
  of a small struct, one 16 bytes later (i.e. inbox vs. archive) — and routes **each decoded row**
  into whichever list matches **that row's own `flags & MAIL_Archive` bit**, never based on the
  request's `bArchive` parameter. A row is upserted by `id` (found via `FUN_00e12e70`, updated in
  place if found, newly constructed via `FUN_00eb5ab0` if not) — not wholesale replaced.
- **`ResetCategory`, when true, clears only the ONE list matching the *request's* `bArchive`
  value** (`FUN_00e19110(headerLists, bArchive ? archiveList : inboxList)`) before the new rows are
  processed.

Practical read for SS-M1 (audit A-08, the header query ignoring `bArchive`): because the client
routes each row by its own flag bit, sending every row regardless of `bArchive` does not visibly
merge inbox and archive rows into the wrong list on the client. It does mean a `ResetCategory`
request for one category will still receive (and silently upsert) rows belonging to the other
category that it never asked for and never reset, which can leave stale entries after a
delete/archive server-side change that the client never re-requests. **This is not a reason to
keep the bug** — SS-M1's plan to filter the SQL by `bArchive` is still correct and simpler than
relying on client-side row routing.

Evidence: `ghidra://SGW.exe@0x00e15450` (full decoder, archive/inbox list selection and
`ResetCategory` clear), `0x00e16e60` (registration order), `0x00e12e70` (upsert-by-id lookup),
`0x00eb5ab0` (row constructor).
