---
name: reference-black-market-wire-spec
description: Ghidra-extracted wire field layouts for the four client-emitted Event_NetOut_BM* messages
metadata:
  type: reference
---

Wire shapes for the Black Market / Auction House outbound messages,
extracted from SGW.exe via Ghidra. All four are emitted by the
client to the server via Mercury (CellMethod indices 61–64). The
encoding is Mercury **property tree** (name-tagged fields,
type-driven widths from a registry — *not* a flat little-endian
byte array). Field types listed below are the C++ parameter types
to the emit constructors; the prop-tree encoder widens these
according to the property registry.

## Event_NetOut_BMCreateAuction (cell method 62)
Ghidra anchor: `00e59970` `FUN_00e59970`; string `019dd370`.
Fields in emit order:
- `itemInstanceId` — u32 (the item to list, NOT a type id)
- `startingPrice` — u32 (signed-context per Rust stub; range
  check needed regardless)
- `buyoutPrice` — i32
- `auctionLength` — u8 widened (param_4 is `char`)

## Event_NetOut_BMPlaceBid (cell method 63)
Ghidra anchor: `00e59da0` `FUN_00e59da0`; string `019dd3e8`.
Fields in emit order:
- `sequenceId` — i8 widened (param_1 is `char`). **This is a
  paginated-cursor index, not a stable auction row id.**
  The pair `(clientKey from last BMSearch reply, sequenceId)`
  is what selects an auction.
- `bidAmount` — u32 (param_2 is `uint`)

Client-side pre-emit gate (informational only, trivially
bypassed): `bidAmount <= caller's_naqahdah_balance`. Comparison
at `if ((int)param_2 <= *(int *)(*(int *)(*(int *)(iVar1 + 0x8c)
+ 0x24) + 0x60))`.

## Event_NetOut_BMCancelAuction (cell method 64)
Ghidra anchor: `00e59c70` `FUN_00e59c70`; string `019dd3ac`.
Single field:
- `sequenceId` — i8 widened (param_1 is `char`). Same paginated
  cursor as PlaceBid.

## Event_NetOut_BMSearch (cell method 61)
Ghidra anchor: `00e59f70` `FUN_00e59f70`; string `019dd41c`.
Fields in emit order (eleven total):
- `sortId` — u32
- `clientKey` — u32 (per-search correlation id; server hands
  this back in the auctions reply so paged subsequent calls
  match the cache)
- `sequenceId` — u32 (page cursor)
- `bForward` — bool (forward/backward pagination)
- `sellerName` — wchar_t* (LIKE-pattern source)
- `bidderName` — wchar_t*
- `itemName` — wchar_t*
- `minTC` — u32 (min tech-credit / level filter)
- `maxTC` — u32
- `quality` — u32
- `filterFlags` — u32 (bitmask of category filters)

## Inbound (server → client) — for reference
- `Event_NetIn_BMOpen` — opens the BM UI on the client
- `Event_NetIn_BMAuctions` — auction listing batch. Per-row
  fields (see `FUN_00e58e40` at `00e58e40`): `sequenceId`,
  `itemDefId`, `stackSize`, `durability`, `charges`,
  `currentBid`, `buyoutPrice`, `nextMinBidPrice`, `endTimeValue`.
  Outer envelope (`FUN_00e5aa10` at `00e5aa10`): `totalResults`,
  `clientKey`, `auctionItems` (array of the above).
- `Event_NetIn_BMAuctionUpdate` — single-row delta (bid placed,
  current bid changed).
- `Event_NetIn_BMAuctionRemove` — listing closed (expiry, win,
  cancel).
- `Event_NetIn_BMError` — generic BM operation error.

## Server-side response constants
`crates/services/src/cell/client_methods/black_market.rs`:
```
ON_BM_OPEN: u16 = 90;
ON_BM_ERROR: u16 = 91;
ON_BM_AUCTIONS: u16 = 92;
ON_BM_AUCTION_REMOVE: u16 = 93;
ON_BM_AUCTION_UPDATE: u16 = 94;
ON_BM_WATCHED_ITEMS_UPDATE: u16 = 95;
```
None are emitted from the server today.

## Stub mis-decodes to fix at implementation time
The current `cell_methods/black_market.rs` stub decodes args as
flat LE i32 sequences. This will collide with the real Mercury
property-tree frame:
- `BMCreateAuction` stub treats four fields as four i32s; the
  real wire is `u32 + u32 + i32 + u8` plus prop-tree name tags.
- `BMPlaceBid` stub names the field `auction_id` and reads
  i32; the real field is `sequenceId` (paginated cursor, i8-
  widened).
- `BMCancelAuction` same — `auction_id` vs `sequenceId`.

When implementing, decode through the same Mercury property-
tree parser that other handlers use, not via `from_le_bytes`
offsets.
