---
title: "Black Market (Auction House)"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Black Market (Auction House)

> **Last updated**: 2026-09-27
> **Status on `main`**: Server implemented, not player-visible. Search, create, bid, buyout, cancel and expiry work server-side and follow the client's wire contract; the watch list is refused on purpose (decision D4).

> [!IMPORTANT]
> The server side is on `main`: packet BM-01 of the [restoration plan](../analysis/black-market/README.md) ported `feat/571-black-market-phase1` (PR #586) onto the split crates, and packet BM-02 fixed its wire contract (S1–S8) against the client and added the escrow, authority and rule changes below. One thing still stands between it and a player:
>
> - **The client drops every `onBM*` method (90–95).** The auction window opens only with the client patch the plan builds (issue #587, packets BM-03 to BM-06). The server and the patch encode and decode with the same codec crate, `cimmeria-patch-wire`.

## Overview

The Black Market is the player-driven auction house system. Players list items for sale with starting prices and optional buyout prices, and other players search for and bid on listings. The system supports item watching (notifications when watched items are listed), search with filtering, variable auction lengths, and a dedicated `SGWBlackMarket` server-only entity for managing auction state.

The `SGWBlackMarketManager` interface defines the player-side protocol. The `SGWBlackMarket` entity is a server-only BaseApp entity that handles auction persistence and search.

## Implementation Status

The Rust implementation is split across two layers. Client RPCs land on the cell methods (indices 61–66) in `crates/cell-methods/src/cell/cell_methods/black_market/mod.rs`, which decode the payload with the shared codec, check that a create, bid or cancel comes from a player at an auctioneer, and forward to the base via `CellToBaseMsg::BlackMarket(BlackMarketCellToBase)` (routed by `crates/base-world-entry/src/base/world_entry/cell_dispatch/black_market_dispatch.rs`). The base side (`crates/base-methods/src/base/world_entry/methods/black_market/`) owns all database, escrow, cash, and mail work and sends the `onBM*` replies (client indices 90–95) back to the requesting player. Every argument layout comes from `crates/patch-wire` (`cimmeria-patch-wire`), re-exported as `cimmeria_wire::black_market`; the injected client-patch DLL links the same crate. The file names in the table below are relative to the base-side directory.

| Feature | Status | Notes |
|---------|--------|-------|
| Search auctions | DONE | `BMSearch` (CM 61) → `search.rs`. `clientKey` picks the view; My Auctions and My Bids are the caller's own; filters, a cursor and a one-message page |
| Create auction | DONE | `BMCreateAuction` (CM 62) → `create.rs`; moves the item row into the seller's container 18 (escrow) |
| Place bid | DONE | `BMPlaceBid` (CM 63) → `bid.rs`; refunds the outbid player; 5% minimum increment |
| Buyout | DONE | A bid at or over the buyout price settles at once (D8), through `settle.rs` |
| Cancel auction | DONE | `BMCancelAuction` (CM 64) → `cancel.rs`; moves the escrowed row back into the seller's bags |
| Expiry settlement | DONE | Background sweep in `sweep.rs` over `settle.rs`: the item moves to the buyer (or back to the seller), the seller is mailed the cash |
| Auctioneer check | DONE | The cell forwards CM 62–64 only for a player the server sent to an auctioneer, still interacting with it and in range |
| Watch items | REFUSED | `BMStartWatchingItem` / `BMStopWatchingItem` (CM 65/66) answer `onBMError(WatchUnavailable)` (D4) |
| Auction results display | DONE | `onBMAuctions` built by `wire::serialize_on_bm_auctions` |
| Auction updates | DONE | `onBMAuctionUpdate`, `onBMAuctionRemove` in `wire/` and `send.rs` |
| Error handling | DONE | `onBMError` ids 0 and 1 are the shipped `EBlackMarketError`; the server's rejection ids follow (D3), see [Error ids](#error-ids) |
| Server-side entity | DONE | `SGWBlackMarket` base-side state machine under `crates/base-methods/src/base/world_entry/methods/black_market/` |
| Persistence | DONE | `sgw_auction` + `sgw_auction_bid` tables under `db/sgw/BlackMarket/` |

## Entity Definitions

### SGWBlackMarketManager.def (Player Interface)

#### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `watchedItems` | ARRAY\<INT32\> | CELL_PRIVATE | Item definition IDs being watched |

#### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onBMOpen` | entityId | Open auction UI (the auctioneer's entity ID) |
| `onBMError` | errorId | A refused request; see [Error ids](#error-ids) |
| `onBMAuctions` | ARRAY\<AuctionItem\>, totalResults, clientKey | One page of search results |
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

Every layout below is `.def` order, encoded and decoded by `cimmeria-patch-wire` on both the server and the client patch. Names in Black Market payloads are STRING (4-byte LE length prefix + N UTF-8 bytes, at most 255), **not** WSTRING/UTF-16 — unlike most other SGW social systems. Evidence: [black-market-client-io.md](../reverse-engineering/findings/black-market-client-io.md) §4 and [black-market-wire-formats.md](../reverse-engineering/findings/black-market-wire-formats.md).

### BMSearchOptions

Eleven fields, in wire order:

```text
UINT8  sortId          -- EBlackMarketSortType; logged, not applied
INT32  clientKey       -- the view the reply fills (UIAuctionView: 0 Search,
                          1 MyAuctions, 2 MyBids); echoed back in onBMAuctions
INT32  sequenceId      -- paging cursor (last auction id the client saw; 0 = start)
UINT8  bForward        -- non-zero = page forward from the cursor
STRING sellerName      -- ignored: My Auctions is always the caller's own
STRING bidderName      -- ignored: My Bids is always the caller's own
STRING itemName        -- case-insensitive substring of the item name; empty = none
INT32  minTC           -- minimum tech competency; 0 = no bound
INT32  maxTC           -- maximum tech competency; 0 = no bound
INT32  quality         -- logged, not applied (the UI sends 2000)
INT32  filterFlags     -- .def name monikerCRC; the shipped UI always sends 0
```

### onBMAuctions (server → client)

`ARRAY<AuctionItem> auctionItems, INT32 totalResults, INT32 clientKey`. `totalResults` is every match, not the page size, and `clientKey` echoes the request. A reply is one unfragmented message, so the server reads at most 50 rows and sends the ones that fit a 1,200-byte argument budget (about 23 typical rows, more than two of the UI's 8-row pages).

### AuctionItem

```text
INT32  sequenceId
INT32  itemDefId
INT32  stackSize
INT32  durability
INT32  charges
INT32  currentBid
INT32  buyoutPrice
UINT8  endTimeValue      -- time left, as the smallest UIAuctionTime tier that covers it
INT32  nextMinBidPrice   -- the lowest bid the server accepts next
STRING sellerName        -- from sgw_player, so offline sellers show too
```

### BMCreateAuction (client → server)

13 bytes, in `.def` order: `INT32 itemInstanceId, INT32 buyoutPrice, UINT8 auctionLength, INT32 startingPrice`. `auctionLength` is a single byte holding the 1-based `UIAuctionTime` value; the create form sends 3, 4 or 5. A value outside 1–5 is clamped to the nearest tier and the clamped tier is stored. The cell refuses a payload with trailing bytes.

## Rules

| Rule | Value | Source |
|------|-------|--------|
| Durations | VeryShort 12 h, Short 24 h, Medium 48 h, Long 72 h, VeryLong 96 h | Design; no source gives the shipped durations |
| Time-left bucket (`endTimeValue`) | The smallest tier whose duration covers the time left | S6 |
| First bid | At least the starting price (and at least 1) | |
| Later bids | At least 5% over the standing bid, at least +1 | D6 |
| Buyout | A bid at or over a non-zero buyout price is charged the buyout price and settles at once | D8 |
| Starting price | At least 1; a buyout, if set, at least the starting price | |
| Listing cap | 20 active listings per seller; no listing fee | D5 |
| Listable items | Unbound rows in the main bag (1) or the crafting bag (15) | |
| Closed window | Once `expires_at` passes, bids and cancels are refused (`AuctionGone`) until the sweep settles the auction | |

### Error ids

`onBMError` carries one of these (decision D3; the enum is `BMError` in `cimmeria-patch-wire`). The server logs each refusal with the matching `reason`, and the UI overlay maps the id to text.

| Id | Name | `reason` | When |
|----|------|----------|------|
| 0 | InvalidSortType | `invalid_client_key` | `clientKey` names no view (shipped value) |
| 1 | BMUnavailable | `bm_unavailable` | The server has no database (shipped value) |
| 2 | NotEnoughFunds | `not_enough_funds` | The bidder cannot cover the bid |
| 3 | AuctionGone | `auction_gone` | No such auction, settled, or past `expires_at` |
| 4 | IsSeller | `is_seller` | A seller bid on their own auction |
| 5 | BidTooLow | `bid_too_low` | Below the starting price or the 5% increment |
| 6 | InvalidItem | `invalid_item` | Not the seller's item, or not in a listable bag |
| 7 | NotSeller | `not_seller` | Only the seller may cancel |
| 8 | Internal | `internal` | A server failure; nothing changed |
| 9 | NotAtAuctioneer | `not_at_auctioneer` | Create, bid or cancel away from an open auctioneer |
| 10 | TooManyListings | `too_many_listings` | The seller has 20 active listings |
| 11 | InvalidPrice | `invalid_price` | Starting price below 1, or buyout below the start |
| 12 | ItemBound | `item_bound` | Bound items cannot be listed |
| 13 | BagFull | `bag_full` | No free bag slot for a cancelled or bought item |
| 14 | WatchUnavailable | `watch_unavailable` | The watch list is deferred (D4) |

## Auction Flow

```text
Player interacts with the auctioneer
  |-> Chain runs open_black_market: onBMOpen(auctioneerId), and the cell
  |   records the auctioneer in the player's Black Market session

Seller: BMCreateAuction(itemInstanceId, buyoutPrice, auctionLength, startingPrice)
  |-> Cell: decode; refuse NotAtAuctioneer unless the player's interaction
  |   target is still that auctioneer, in the same space, within 5 units
  |-> Base (create.rs), one transaction: check prices, move the row from
  |   bag 1 or 15 into the seller's container 18, check the cap, insert
  |-> Seller: onRemoveItem(item), onBMAuctionUpdate

Buyer: BMSearch(searchOptions)
  |-> Cell -> Base (search.rs): open listings for the clientKey view
  |-> Results: onBMAuctions(items[], totalResults, clientKey)

Buyer: BMPlaceBid(sequenceId, bidAmount)   (auctioneer check as above)
  |-> Base (bid.rs), one transaction: lock the listing, check it is open,
  |   check the bid and the funds, refund the prior bidder, hold the bid
  |-> Bidder (and the outbid player, if online): onBMAuctionUpdate
  |-> At or over the buyout price: settle now (settle.rs), as at expiry

Seller: BMCancelAuction(sequenceId)        (auctioneer check as above)
  |-> Base (cancel.rs): move the row from container 18 back into the
  |   seller's bags, refund the bidder
  |-> Seller: onBMAuctionRemove(sequenceId), onUpdateItem(item)

Auction expires (expiry sweep, every 30s):
  |-> Sold (a bidder holds a positive bid): the row moves into the buyer's
  |   bags, the seller is mailed the cash, the buyer a notice; SOLD
  |-> Unsold: the row moves back into the seller's bags, the seller is
  |   mailed a notice; EXPIRED
  |-> Online parties: onBMAuctionRemove(sequenceId), onUpdateItem to the
  |   player who got the item
```

Settlement runs in one transaction per auction, and its status write is conditional on the auction still being active, so a crash or a race cannot pay twice. A player's listing whose container-18 row is missing is never delivered as a copy: the settlement is refused and logged as `bm.escrow_missing`. System-generated auction mail uses the sender name `Black Market`. Packet BM-02b moves the deliveries onto the social-systems mail API (the item mailed from container 18).

## Escrow (container 18)

A listed item stays its own `sgw_inventory` row, with its instance id and every column, moved into the seller's container 18 (`INV_AUCTION`). Container 18 is server-held: the login inventory send and every resync skip it, a refused move's snap-back resend of a remembered id finds nothing, and the move, use, trade, mail and vendor paths refuse it through their container allowlists. This is the shape the social-systems system-mail writer takes (`SystemItem::ExistingInstance` accepts only a container-18 row owned by `owner_player_id`).

## Persistence

Two tables under [`db/sgw/BlackMarket/`](../../db/sgw/BlackMarket/):

- **`sgw_auction`** — one row per listing. `sequence_id` is the primary key and the wire-visible identity the client tracks (`onBMAuctions` / `onBMAuctionUpdate` / `onBMAuctionRemove` all key on it). `item_id` is the escrowed row (0 for a boot-seed listing); the item snapshot (`item_def_id`, `stack_size`, `durability`, `charges`), pricing (`starting_price`, `buyout_price`, `current_bid`, `current_bidder`), and timing (`auction_length` — the 1-based tier, `created_at`, `expires_at` — both unix epoch seconds).
- **`sgw_auction_bid`** — bid history, one row per accepted bid (a buyout records the buyout price). My Bids reads it. The live "current" bid is denormalised onto `sgw_auction`.

`status` values: `0` = active, `1` = sold, `2` = cancelled, `3` = expired.

**Character deletion (D-BM09).** `seller_id` is `ON DELETE CASCADE` and `current_bidder` `ON DELETE SET NULL`. A `BEFORE DELETE` trigger on `sgw_player` (`bm_player_before_delete()`) first refunds the standing bidders of the deleted seller's open auctions, and reopens the open auctions the deleted character was winning. A deleted seller's listings and escrowed items go with the character, like the rest of its inventory; a settled auction keeps its row without its buyer.

## Telemetry

Every transition is a DEBUG event (`bm.listed`, `bm.bid`, `bm.outbid_refund`, `bm.cancelled`, `bm.sold`, `bm.expired`) with `auction_id`, the seller and bidder ids, the bid and the escrowed cash before and after, `item_def_id`, and the actor's `account_id` and `player_id`. Every refusal is `bm.refused` with `reason` and `error_id` (table above), and every request counts on `bm_outcome_total{op, outcome}`. The cell logs `bm.decode_failed` (payload length and reason), `bm.open` (the recorded auctioneer), and, once at logout, `bm.open_without_client_call` for a player who was sent `onBMOpen` but whose client never called 61–66: the sign the client patch is missing. Each `onBM*` send logs `bm.send` with the method and payload size.

## Data References

- **Custom types**: `BMSearchOptions`, `AuctionItem` — see [Wire Format](#wire-format)
- **Enumerations**: `EBlackMarketError`, `EBlackMarketTime`, `EBlackMarketSortType`, `EBlackMarketFilter`; the client's `UIAuctionView` and `UIAuctionTime`
- **Database**: `sgw_auction`, `sgw_auction_bid`

## Remaining Work

0. **The client patch.** The [restoration plan](../analysis/black-market/README.md) sequences it: BM-03 to BM-06 build the client patch and its launcher delivery, BM-02b moves the payouts onto the social-systems mail API, BM-07 adds the auctioneer content and the UAT checklist.
1. **Durations** — the tier-to-hours table is design, not recovered.
2. **Watch notifications** — deferred by D4; `BMStartWatchingItem` / `BMStopWatchingItem` answer `WatchUnavailable` until the core loop passes UAT (BM-08).
3. **Search sort and quality** — `sortId`, `quality` and the eleventh field are logged, not applied; results come in listing order.

## Economy sink design (unbuilt)

Folded in from the superseded server-systems survey. Nothing here is
implemented — the auction currently takes no cut at all, and decision D5 in the
[restoration plan](../analysis/black-market/README.md) chose a cap of 20 active
listings per player and no fee for now.

The Black Market is the natural place for Cimmeria's first real currency sink.
Currency enters the game freely (mission rewards, cash loot, vendor sell-back)
and leaves almost nowhere, so the sink side needs somewhere to start, and an
auction house is where the standard MMO answer lives: a **non-refundable
listing fee** charged at create time (roughly 1–2% of the starting price) plus a
**transaction cut** taken from the seller's proceeds on a successful sale
(roughly 5%). Both are well-understood, easy to tune from a single config value,
and each has an obvious hook in the flow that already exists — the fee at
`BMCreateAuction`, the cut in the settlement transaction.

**Do not tune the percentages before the currency flow is instrumented.**
Without per-source logging of currency gains and losses there is no way to know
whether a 5% cut is a rounding error or a wealth tax. The instrumentation
proposal is
[server-infrastructure-proposals.md §5](../architecture/server-infrastructure-proposals.md#5-economy-instrumentation-before-economy-balance);
build that first, then set these numbers against real data.

## Related Docs

- [inventory-system.md](inventory-system.md) - Items listed and purchased
- [mail-system.md](mail-system.md) - Delivery mechanism for seller proceeds
