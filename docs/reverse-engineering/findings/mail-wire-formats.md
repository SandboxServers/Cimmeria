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
| `ContainerId` | `INT32` | 4B | Destination bag |
| `SlotId` | `INT32` | 4B | Destination slot |

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

**`MessageAttachment` FIXED_DICT layout** (20 bytes):

| Field | Type | Size |
|-------|------|------|
| `id` | `INT32` | 4B |
| `itemId` | `INT32` | 4B |
| `stackSize` | `INT32` | 4B |
| `durability` | `INT32` | 4B |
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
