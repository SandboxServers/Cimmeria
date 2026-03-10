# FIXED_DICT Struct Field Layouts — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation
> **Confidence**: HIGH — decompiled serializers, Lua bindings, internal constructors

---

## 1. BMSearchOptions

Event: `Event_NetOut_BMSearch`, serializer at `0x00e59f70`

| Field | Type | Offset | Description |
|-------|------|--------|-------------|
| sellerName | wstring | +0x00 | Seller name filter |
| bidderName | wstring | +0x1C | Bidder name filter |
| itemName | wstring | +0x38 | Item name filter |
| filterFlags | uint32 | +0x54 | Bitmask of filter flags |
| sortId | uint8 | +0x58 | Sort column identifier |
| sequenceId | uint32 | +0x5C | Search sequence/page number |
| bForward | bool | +0x60 | Pagination direction |
| minTC | uint32 | +0x64 | Minimum tech competency level |
| maxTC | uint32 | +0x68 | Maximum tech competency level |
| quality | uint32 | +0x6C | Item quality filter |
| clientKey | uint32 | +0x70 | Client-side search key |

Lua: `searchAuctions(sellerName, bidderName, bExactMatch, itemName, minTC, maxTC, quality, filterFlags, sortId, clientKey)` at `0x00aca360`

---

## 2. AuctionItem

Internal serializer at `0x00e58e40`, Lua bridge `getAuctionItemInfo` at `0x00ae1ad0`

### Wire format (internal struct)

| Field | Type | Offset | Description |
|-------|------|--------|-------------|
| itemDefId | uint32 | +0x08 | Item definition database ID |
| sequenceId | uint32 | +0x10 | Auction sequence ID |
| stackSize | uint32 | +0x14 | Stack size |
| durability | uint32 | +0x18 | Durability |
| charges | uint32 | +0x1C | Number of charges |
| currentBid | uint32 | +0x20 | Current highest bid |
| buyoutPrice | uint32 | +0x24 | Buyout price |
| nextMinBidPrice | uint32 | +0x28 | Minimum next bid |
| endTimeValue | int64 | +0x2C | Auction end timestamp |

### Lua-exposed fields

| Field | Source |
|-------|--------|
| auctionId | obj[4] (+0x10) |
| itemId | Derived from itemDefId |
| name | Item def +0x10 (wstring) |
| icon | Item def +0x48 (wstring) |
| techCompetancy | Item def +0x78 |
| timeLeft | obj byte +0x0B (uint8 bucket) |
| stackSize, durability, charges | From struct |
| currentBid, buyoutPrice, nextBidPrice | From struct |
| sellerName, bidderName | Derived (wstring) |
| bidCount | Hardcoded 0 |

### BMAuctions Event (incoming, `0x00e5aa10`)

| Field | Type | Description |
|-------|------|-------------|
| totalResults | uint32 | Total search results count |
| clientKey | uint32 | Echoed search key |
| auctionItems | ARRAY | Array of auction property trees |

---

## 3. BMCreateAuction

Event: `Event_NetOut_BMCreateAuction` at `0x00e59970`

| Field | Type | Description |
|-------|------|-------------|
| itemInstanceId | uint32 | Inventory item to auction |
| startingPrice | uint32 | Starting bid price |
| buyoutPrice | uint32 | Buyout price (0 = none) |
| auctionLength | uint8 | Duration enum |

## 4. BMPlaceBid

Event: `Event_NetOut_BMPlaceBid` at `0x00e59da0`

| Field | Type | Description |
|-------|------|-------------|
| sequenceId | uint32 | Auction ID |
| bidAmount | uint32 | Bid amount |

## 5. BMCancelAuction

Event: `Event_NetOut_BMCancelAuction` at `0x00e59c70`

| Field | Type | Description |
|-------|------|-------------|
| sequenceId | uint32 | Auction ID to cancel |

---

## 6. MailHeader

Handler: `onMailHeaderInfo` at `0x00e15450`

### Top-level message

| Field | Type | Description |
|-------|------|-------------|
| ResetCategory | bool | Reset a category of mail |
| bArchive | bool | Process archive category |
| MessageHeaders | ARRAY | Array of header property trees |
| MessageAttachments | ARRAY | Array of attachment property trees |

### Per-header fields

| Field | Type | Description |
|-------|------|-------------|
| id | uint32 | Mail message ID |
| fromText | wstring | Sender display name |
| fromId | uint32 | Sender player ID |
| subjectText | wstring | Subject line |
| subjectId | uint32 | Subject text ID (localization) |
| cash | uint32 | Cash attached |
| sentTime | int64 | Timestamp sent |
| readTime | int64 | Timestamp read (0 = unread) |
| flags | uint32 | Mail flags bitmask |

### Mail object layout (constructor `0x00eb5ab0`, size `0xBC`)

| Offset | Type | Description |
|--------|------|-------------|
| +0x44 | uint32 | cash amount |
| +0x48 | uint32 | flags |
| +0x50 | uint32 | stackSize (attachment) |
| +0x54 | uint32 | durability (attachment) |
| +0x58 | uint32 | charges (attachment) |
| +0x9C | bool | has body loaded |

---

## 7. MailAttachment

Parsed in `0x00e15450`:

| Field | Type | Description |
|-------|------|-------------|
| id | uint32 | Attachment slot ID |
| itemId | uint32 | Item definition ID |
| stackSize | uint32 | Stack size |
| durability | uint32 | Durability |
| charges | uint32 | Number of charges |

---

## 8. SendMailMessage

Event: `Event_NetOut_SendMailMessage` at `0x00e14910`

| Field | Type | Description |
|-------|------|-------------|
| RecipientFlags | uint32 | Bitmask (bit 0 = archive, bit 2 = vault alias) |
| Recipients | wstring | Semicolon-delimited recipient names |
| Subject | wstring | Subject line |
| Body | wstring | Body text |
| Cash | uint32 | Cash to attach |
| bCOD | bool | Cash-on-Delivery flag |
| ItemId | uint32 | Attached item ID (0 = none) |
| ItemQuantity | uint32 | Attached item quantity |

---

## 9. Pet Stances

### PetStances (incoming, `0x00d3a070`)

| Field | Type | Description |
|-------|------|-------------|
| aStanceList | ARRAY of uint8 | Stance enum values (1 byte each) |

### Stance Enum

| Value | Name | Description |
|-------|------|-------------|
| 0 | Passive | Pet does not attack |
| 1 | Aggressive | Pet attacks on sight |
| 2 | Defensive | Pet retaliates when attacked |

### PetStanceChange (outgoing, `0x00c83a70`)

| Field | Type | Description |
|-------|------|-------------|
| StanceID | uint32 | Stance slot being changed |
| aNewStance | uint32 | New stance value |

### changePetStance RPC (`0x00d3a6e0`)

| Field | Type | Description |
|-------|------|-------------|
| aStance | uint8 | Stance enum value |
| aEntityId | uint32 | Pet entity ID |

### getPetStanceInfo (Lua, `0x00adbf80`)

Returns: `id` (uint32), `name` (wstring), `icon` (wstring)

---

## 10. Contact Lists

### ContactList entry (`getContactList` at `0x00adfcf0`)

| Field | Type | Description |
|-------|------|-------------|
| Id | uint32 | Contact list ID |
| Name | wstring | List name |
| Flags | uint32 | Flags bitmask |
| Members | ARRAY | Member entries |

### contactListCreate (`0x00e61830`)

| Field | Type |
|-------|------|
| aListId | uint32 |
| aName | wstring |

### contactListDelete / contactListRename

| Field | Type |
|-------|------|
| aListId | uint32 |
| aName | wstring (rename only) |

### contactListFlagsUpdate

| Field | Type |
|-------|------|
| aListId | uint32 |
| aFlags | uint32 |

### contactListAddMembers / RemoveMembers (`0x00e61f90`)

| Field | Type |
|-------|------|
| aListId | uint32 |
| aPlayerNames | ARRAY of wstring |

### ContactList Event Fields (`0x019dc6c8`)

| Field | Type | Description |
|-------|------|-------------|
| aMemberName | wstring | Member player name |
| aMember | uint32 | Member player ID |
| aNewMember | uint32 | New member ID |
| aRank | uint32 | Rank in list |
| aReason | wstring | Event reason |
