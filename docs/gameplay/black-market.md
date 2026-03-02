# Black Market (Auction House)

> **Last updated**: 2026-03-01
> **Status**: ~0% implemented

## Overview

The Black Market is the player-driven auction house system. Players list items for sale with starting prices and optional buyout prices, and other players search for and bid on listings. The system supports item watching (notifications when watched items are listed), search with filtering, variable auction lengths, and a dedicated `SGWBlackMarket` server-only entity for managing auction state.

The `SGWBlackMarketManager` interface defines the player-side protocol. The `SGWBlackMarket` entity is a server-only BaseApp entity that handles auction persistence and search. No Python implementation exists for either.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Search auctions | STUB | `BMSearch` with `BMSearchOptions` defined |
| Create auction | STUB | `BMCreateAuction` with item, price, length |
| Place bid | STUB | `BMPlaceBid` with sequenceId, amount |
| Cancel auction | STUB | `BMCancelAuction` by sequenceId |
| Watch items | STUB | `BMStartWatchingItem` / `BMStopWatchingItem` |
| Auction results display | STUB | `onBMAuctions` with paginated results |
| Auction updates | STUB | `onBMAuctionUpdate`, `onBMAuctionRemove` |
| Error handling | STUB | `onBMError` with `EBlackMarketError` codes |
| Server-side entity | DEFINED | `SGWBlackMarket` with search/bid/create base methods |

## Entity Definitions

### SGWBlackMarketManager.def (Player Interface)

#### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `watchedItems` | ARRAY\<INT32\> | CELL_PRIVATE | Item definition IDs being watched |

#### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onBMOpen` | entityId | Open auction UI (vendor entity ID) |
| `onBMError` | errorId | Error from `EBlackMarketError` |
| `onBMAuctions` | ARRAY\<AuctionItem\>, totalResults, clientKey | Search results |
| `onBMAuctionRemove` | sequenceId | Auction ended/cancelled |
| `onBMAuctionUpdate` | AuctionItem | Auction state changed (new bid, etc.) |
| `onBMWatchedItemsUpdate` | ARRAY\<INT32\> itemList | Current watch list |

#### Cell Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `BMSearch` | YES | BMSearchOptions | Search auctions |
| `BMCreateAuction` | YES | itemInstanceId, buyoutPrice, auctionLength, startingPrice | List item |
| `BMPlaceBid` | YES | sequenceId, bidAmount | Bid on auction |
| `BMCancelAuction` | YES | sequenceId | Cancel own auction |
| `BMStartWatchingItem` | YES | itemDefId | Add to watch list |
| `BMStopWatchingItem` | YES | itemDefId | Remove from watch list |

#### Base Methods (Cell -> Base Forwarding)

All cell methods have corresponding base methods that forward to the `SGWBlackMarket` entity.

### SGWBlackMarket.def (Server Entity)

**ServerOnly** entity -- no client presence.

#### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `watchedItems` | PYTHON | BASE | Map of itemIds to player base mailboxes |

#### Base Methods

| Method | Args | Purpose |
|--------|------|---------|
| `searchBlackMarket` | MAILBOX, INT32, BMSearchOptions, LanguageId | Execute search query |
| `placeBid` | sequenceId, bidAmount | Process bid |
| `createAuction` | MAILBOX, INT32, itemInstanceId, buyoutPrice, auctionLength, startingPrice | Create listing |
| `cancelAuction` | MAILBOX, INT32, sequenceId | Cancel listing |
| `registerWatchedItems` | ARRAY\<INT32\>, MAILBOX | Register watch notifications |
| `unregisterWatchedItems` | ARRAY\<INT32\>, MAILBOX | Unregister watch notifications |

## Wire Format

### BMSearchOptions

```
TODO: Decompile from client binary
Likely fields: itemName/category filter, level range, price range,
               sort order, page offset, results per page
```

### AuctionItem

```
TODO: Decompile from client binary
Likely fields: sequenceId, itemDefId, quantity, sellerName,
               currentBid, buyoutPrice, timeRemaining, bidCount
```

## Auction Flow

```
Seller: BMCreateAuction(itemInstanceId, buyoutPrice, auctionLength, startingPrice)
  |-> Cell: validate item exists, remove from inventory
  |-> Base: forward to SGWBlackMarket.createAuction()
  |-> SGWBlackMarket: persist auction, notify watchers via onBMAuctionUpdate

Buyer: BMSearch(searchOptions)
  |-> Cell -> Base -> SGWBlackMarket.searchBlackMarket()
  |-> Results: onBMAuctions(items[], totalResults, clientKey)

Buyer: BMPlaceBid(sequenceId, bidAmount)
  |-> Cell -> Base -> SGWBlackMarket.placeBid()
  |-> Validate: bid > current, sufficient cash
  |-> Update auction, notify: onBMAuctionUpdate

Auction expires or buyout reached:
  |-> Winner: item delivered (mail?)
  |-> Seller: cash delivered (mail?)
  |-> All viewers: onBMAuctionRemove(sequenceId)
```

## Data References

- **Custom types**: `BMSearchOptions`, `AuctionItem` (defined in entity type system)
- **Enumerations**: `EBlackMarketError`
- **Database**: Auction table schema needs to be created

## RE Priorities

1. **Wire format** - Decompile `BMSearchOptions` and `AuctionItem` structures
2. **Auction lengths** - Map `auctionLength` UINT8 to actual durations
3. **Error codes** - Enumerate `EBlackMarketError` values
4. **Delivery mechanism** - How items/cash are delivered to winner/seller (mail system?)
5. **Watch notifications** - Push notification flow when watched items are listed

## Related Docs

- [inventory-system.md](inventory-system.md) - Items listed and purchased
- [mail-system.md](mail-system.md) - Likely delivery mechanism for won items
