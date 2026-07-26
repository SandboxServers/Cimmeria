---
title: "Black Market (Auction House)"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Black Market (Auction House)

> **Last updated**: 2026-07-25
> **Status on `main`**: **Not implemented.** 94 lines of handler stubs.
> **Status on `feat/571-black-market-phase1`** (unmerged, PR #586): Phase 1 — search / create / bid / cancel / expiry all work; item watching is still a stub.

> [!IMPORTANT]
> Everything this page describes as working lives on the **unmerged** branch `feat/571-black-market-phase1` (PR #586). None of it is on `main`. If you are reading `main`, see [What exists on `main`](#what-exists-on-main) — the auction house is two stub files that log and drop.

## Overview

The Black Market is the player-driven auction house system. Players list items for sale with starting prices and optional buyout prices, and other players search for and bid on listings. The system supports item watching (notifications when watched items are listed), search with filtering, variable auction lengths, and a dedicated `SGWBlackMarket` server-only entity for managing auction state.

The `SGWBlackMarketManager` interface defines the player-side protocol. The `SGWBlackMarket` entity is a server-only BaseApp entity that handles auction persistence and search.

## What exists on `main`

Two files, 94 lines total, neither of which touches a database:

| File | Lines | Behaviour |
|------|-------|-----------|
| `cell/cell_methods/black_market.rs` | 80 | Decodes cell methods 61–66 and logs `UNIMPLEMENTED` for each |
| `cell/client_methods/black_market.rs` | 14 | Client-method index constants (90–95) only |

There is no `sgw_auction` table, no base-side handler, and no `onBM*` reply is ever sent. Note also that `main`'s `BMCreateAuction` arm reads a **16-byte** payload with a 4-byte `duration_days` field; that is wrong — see [Wire Format](#bmcreateauction-client--server) for the corrected 13-byte layout the branch uses.

## Implementation Status (branch `feat/571-black-market-phase1` only)

The Rust implementation is split across two layers. Client RPCs land on the cell methods (indices 61–66) in [`crates/services/src/cell/cell_methods/black_market/mod.rs`](../../crates/services/src/cell/cell_methods/black_market/mod.rs), which decode the payload and forward to the base via `CellToBaseMsg::BM*` variants. The base side ([`crates/services/src/base/black_market/`](../../crates/services/src/base/black_market/)) owns all database, escrow, cash, and mail work and sends the `onBM*` replies (client indices 90–95) back to the requesting player.

| Feature | Status | Notes |
|---------|--------|-------|
| Search auctions | DONE | `BMSearch` (CM 61) → `base/black_market/search.rs`; `BMSearchOptions` deserialized in `types.rs` |
| Create auction | DONE | `BMCreateAuction` (CM 62) → `create.rs`; escrows the item out of inventory |
| Place bid | DONE | `BMPlaceBid` (CM 63) → `bid.rs`; refunds the outbid player |
| Cancel auction | DONE | `BMCancelAuction` (CM 64) → `cancel.rs`; returns the escrowed item |
| Expiry settlement | DONE | Periodic background sweep in `sweep.rs` — pays the seller, mails the item to the winner |
| Watch items | STUB | `BMStartWatchingItem` / `BMStopWatchingItem` (CM 65/66) log `UNIMPLEMENTED` and drop |
| Auction results display | DONE | `onBMAuctions` serialized by `wire::serialize_on_bm_auctions` |
| Auction updates | DONE | `onBMAuctionUpdate`, `onBMAuctionRemove` in `wire.rs` / `send.rs` |
| Error handling | PARTIAL | `onBMError` is wired, but the `EBlackMarketError` ordinals are placeholders — see [Blocked Unknowns](#blocked-unknowns) |
| Server-side entity | DONE | `SGWBlackMarket` base-side state machine under `base/black_market/` |
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

Both structures have been recovered from the client emitters. Names in Black Market payloads are STRING (4-byte LE length prefix + N UTF-8 bytes), **not** WSTRING/UTF-16 — unlike most other SGW social systems. See [black-market-wire-formats.md](../reverse-engineering/findings/black-market-wire-formats.md) for the evidence chain.

### BMSearchOptions

Eleven fields, in wire-packing order (deserialized by `BMSearchOptions::from_wire`):

```
UINT8  sortId          -- EBlackMarketSortType
INT32  clientKey       -- opaque pagination token, echoed back in results
INT32  sequenceId      -- pagination cursor (last-seen auction sequence id)
UINT8  bForward        -- non-zero = paginate forward
STRING sellerName      -- empty = no filter
STRING bidderName      -- empty = no filter
STRING itemName        -- substring filter; empty = no filter
INT32  minTC           -- minimum trade-credits price
INT32  maxTC           -- maximum trade-credits price
INT32  quality         -- EItemQuality filter
INT32  filterFlags     -- EBlackMarketFilter category/faction/mode bitfield
```

The 11th field was recovered from the emitter at `puVar1+0x15`; an older revision of this doc mislabelled that slot `monikerCRC`.

### AuctionItem

Serialized by `wire::push_auction_item`:

```
INT32  sequenceId
INT32  itemDefId
INT32  stackSize
INT32  durability
INT32  charges
INT32  currentBid
INT32  buyoutPrice
UINT8  endTimeValue      -- the auctionLength duration enum, echoed back
INT32  nextMinBidPrice
STRING sellerName
```

### BMCreateAuction (client → server)

13 bytes, not 16: `INT32 itemInstanceId, INT32 startingPrice, INT32 buyoutPrice, UINT8 auctionLength`. The client packs `auctionLength` as a single byte; reading it as an INT32 both over-reads by 3 bytes and corrupts the duration.

## Blocked Unknowns

Three values are still guesses, each isolated to a single named constant or function in [`base/black_market/wire.rs`](../../crates/services/src/base/black_market/wire.rs) so the real captured value is a one-line swap:

| Unknown | Current placeholder | How to settle it |
|---------|--------------------|------------------|
| `EBlackMarketError` ordinals | `error_code::*` — 1..7, invented | x64dbg breakpoint at `0x00d84940` (BMError register) |
| `auctionLength` → duration | `auction_length_seconds` — 12/24/48/72/96 hours | x64dbg breakpoint at `0x00e59970` (BMCreateAuction) |
| `nextMinBidPrice` formula | `next_min_bid` — 5% increment, floor +1 | Capture several `currentBid → nextMinBidPrice` pairs from the client |

The shipped `resources."EBlackMarketError"` type only defines `InvalidSortType` and `BMUnavailable`, which do not cover the create/bid/cancel validation failures the server needs to report. `resources."EBlackMarketTime"` names five tiers (VeryShort / Short / Medium / Long / VeryLong) but carries no durations.

## Auction Flow (branch only)

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

Auction expires (expiry sweep, every 30s):
  |-> Sold (a bidder exists):  seller is mailed the winning cash,
  |                            buyer is mailed the item; status -> SOLD
  |-> Unsold (no bidder):      escrowed item is mailed back to the
  |                            seller; status -> EXPIRED
  |-> Online viewers: onBMAuctionRemove(sequenceId)
```

Settlement runs in one transaction per auction, so a crash mid-settlement cannot double-deliver. System-generated auction mail uses the sender name `Black Market`.

## Persistence (branch only)

Two tables under [`db/sgw/BlackMarket/`](../../db/sgw/BlackMarket/). **Neither exists on `main`** — a `main` checkout has no auction schema at all:

- **`sgw_auction`** — one row per listing. `sequence_id` is the primary key and the wire-visible identity the client tracks (`onBMAuctions` / `onBMAuctionUpdate` / `onBMAuctionRemove` all key on it). Carries the escrowed item snapshot (`item_id`, `item_def_id`, `stack_size`, `durability`, `charges`), pricing (`starting_price`, `buyout_price`, `current_bid`, `current_bidder`), and timing (`auction_length`, `created_at`, `expires_at` — both unix epoch seconds).
- **`sgw_auction_bid`** — bid history, one row per accepted bid, retained for refund and audit. The live "current" bid is denormalised onto `sgw_auction`.

`status` values: `0` = active, `1` = sold, `2` = cancelled, `3` = expired.

## Data References

- **Custom types**: `BMSearchOptions`, `AuctionItem` — see [Wire Format](#wire-format)
- **Enumerations**: `EBlackMarketError`, `EBlackMarketTime`, `EBlackMarketSortType`, `EBlackMarketFilter`
- **Database**: `sgw_auction`, `sgw_auction_bid` (branch only)

## Remaining Work

0. **Merge PR #586.** Until `feat/571-black-market-phase1` lands, none of the above is on `main` and the auction house is non-functional for anyone building from the default branch. Every item below is scoped to the branch.
1. **Error codes** — capture the real `EBlackMarketError` ordinals (see [Blocked Unknowns](#blocked-unknowns))
2. **Auction lengths** — capture the real `auctionLength` UINT8 → duration mapping
3. **Next-min-bid formula** — capture real `currentBid → nextMinBidPrice` pairs
4. **Watch notifications** — `BMStartWatchingItem` / `BMStopWatchingItem` are still stubs; the push flow when a watched item is listed is unimplemented
5. **Immediate buyout settlement** — a bid at or above a non-zero `buyout_price` is currently accepted as an ordinary high bid and left for the expiry sweep to settle. The original game settled it on the spot (`bid.rs:24-28`)

## Economy sink design (unbuilt)

Folded in from the superseded server-systems survey. Nothing here is
implemented on either `main` or the branch — the auction currently takes no cut
at all.

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
- [mail-system.md](mail-system.md) - Delivery mechanism for won items and seller proceeds (branch only)
