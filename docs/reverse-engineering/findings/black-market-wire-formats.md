# Black Market Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `SGWBlackMarketManager.def`, `alias.xml`

---

**Interface**: `SGWBlackMarketManager` (implemented by `SGWPlayer`)

### Client → Server (Exposed Cell Methods)

#### `BMSearch` — Search Auctions

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `searchOptions` | `BMSearchOptions` | FIXED_DICT | Search filters |

**`BMSearchOptions` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `sortId` | `UINT8` | 1B |
| `clientKey` | `INT32` | 4B |
| `sequenceId` | `INT32` | 4B |
| `bForward` | `UINT8` | 1B |
| `sellerName` | `STRING` | 4B len + N×1B (UTF-8) |
| `bidderName` | `STRING` | 4B len + N×1B |
| `itemName` | `STRING` | 4B len + N×1B |
| `minTC` | `INT32` | 4B |
| `maxTC` | `INT32` | 4B |
| `quality` | `INT32` | 4B |
| `monikerCRC` | `INT32` | 4B |

**Note**: `STRING` (not `WSTRING`) — uses UTF-8, not UTF-16LE.

#### `BMCreateAuction` — List Item for Sale

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `itemInstanceId` | `INT32` | 4B | Item to auction |
| `buyoutPrice` | `INT32` | 4B | Buyout price (0 = no buyout) |
| `auctionLength` | `UINT8` | 1B | Duration enum |
| `startingPrice` | `INT32` | 4B | Starting bid |

**Total wire size**: 1B header + 13B = **14 bytes**

#### `BMPlaceBid` — Bid on Auction

| Field | Type | Size |
|-------|------|------|
| `sequenceId` | `INT32` | 4B |
| `bidAmount` | `INT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `BMCancelAuction` — Cancel Own Auction

| Field | Type | Size |
|-------|------|------|
| `sequenceId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `BMStartWatchingItem` / `BMStopWatchingItem` — Watch List

| Field | Type | Size |
|-------|------|------|
| `itemDefId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

### Server → Client

#### `onBMOpen` — Open Black Market UI

| Field | Type | Size |
|-------|------|------|
| `entityId` | `INT32` | 4B — NPC entity ID |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onBMError` — Error Notification

| Field | Type | Size |
|-------|------|------|
| `errorId` | `INT32` | 4B — EBlackMarketError enum |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onBMAuctions` — Search Results

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `auctionItems` | `ARRAY<AuctionItem>` | 4B count + N×AuctionItem |
| `totalResults` | `INT32` | 4B |
| `clientKey` | `INT32` | 4B |

**`AuctionItem` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `sequenceId` | `INT32` | 4B |
| `itemDefId` | `INT32` | 4B |
| `stackSize` | `INT32` | 4B |
| `durability` | `INT32` | 4B |
| `charges` | `INT32` | 4B |
| `currentBid` | `INT32` | 4B |
| `buyoutPrice` | `INT32` | 4B |
| `endTimeValue` | `UINT8` | 1B |
| `nextMinBidPrice` | `INT32` | 4B |
| `sellerName` | `STRING` | 4B len + N×1B (UTF-8) |

#### `onBMAuctionRemove` — Auction Ended/Cancelled

| Field | Type | Size |
|-------|------|------|
| `sequenceId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onBMAuctionUpdate` — Auction Status Change

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `auctionItem` | `AuctionItem` | FIXED_DICT (variable) |

#### `onBMWatchedItemsUpdate` — Watch List Sync

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `itemList` | `ARRAY<INT32>` | 4B count + N×4B |

---

## Implementation Notes

- **Black Market uses STRING** (UTF-8) for seller/bidder/item names, unlike most other systems that use `WSTRING` (UTF-16LE).
