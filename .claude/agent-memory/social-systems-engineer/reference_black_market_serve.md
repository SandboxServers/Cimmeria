---
name: reference-black-market-serve
description: Black Market search/serve implementation: CellToBaseMsg::BMSearch shape, wire format for onBMAuctions (method 92), routing, handler, and test patterns.
metadata:
  type: reference
---

## Wire / method indices (mercury/mod.rs method_idx)

- `ON_BM_OPEN` = 90
- `ON_BM_ERROR` = 91
- `ON_BM_AUCTIONS` = 92 — search results page
- `ON_BM_AUCTION_REMOVE` = 93
- `ON_BM_AUCTION_UPDATE` = 94
- `ON_BM_WATCHED_ITEMS_UPDATE` = 95

Cell method indices (cell/cell_methods/black_market/mod.rs):
- SEARCH = 61, CREATE_AUCTION = 62, PLACE_BID = 63, CANCEL_AUCTION = 64

## CellToBaseMsg::BMSearch shape

```rust
BMSearch {
    entity_id: u32,
    player_id: i32,
    options: BMSearchOptions,   // full 11-field struct, decoded cell-side
}
```

Mirrors `BMCreateAuction` — entity_id + player_id resolved at cell, struct forwarded to base.

## onBMAuctions wire layout (serialize_on_bm_auctions)

```
[u32 LE count]
[AuctionItem × count]        -- via push_auction_item (33 fixed bytes + STRING sellerName)
[i32 LE view]                -- sort_id echoed back as i32
[i32 LE total]               -- total matching rows (= count in Phase 1, no pagination)
```

`view` = `options.sort_id as i32` (echoed so client sort-column highlight tracks).

AuctionItem field order (push_auction_item):
`INT32 sequenceId, INT32 itemDefId, INT32 stackSize, INT32 durability,
 INT32 charges, INT32 currentBid, INT32 buyoutPrice, UINT8 endTimeValue,
 INT32 nextMinBidPrice, STRING sellerName`

Strings are STRING (4-byte LE length prefix + UTF-8), NOT WSTRING.

## Files changed (Phase 1 search serve)

- `cell/messages/cell_to_base.rs` — added `BMSearch` variant
- `cell/cell_methods/black_market/mod.rs` — SEARCH arm now builds + sends `BMSearch`
- `base/black_market/search.rs` — `handle_search`: `SELECT … WHERE status=0 ORDER BY sequence_id`, name resolution, `send_bm_auctions`
- `base/black_market/wire.rs` — added `serialize_on_bm_auctions`
- `base/black_market/send.rs` — added `send_bm_auctions` (mirrors send_bm_auction_update pattern)
- `base/black_market/mod.rs` — declared `pub mod search`
- `base/world_entry/cell_dispatch/black_market_dispatch.rs` — added `BMSearch` arm → `search::handle_search`
- `base/world_entry/cell_dispatch/mod.rs` — `BMSearch` added to BM family arm
- `base/black_market/tests/search.rs` — 3 live-DB tests + registered in tests/mod.rs
- `cell/cell_methods/black_market/tests.rs` — renamed `search_arm_is_handled_and_forwards_nothing` → `search_arm_forwards_bm_search_msg`

## TestTransport pattern for live-DB tests

`make_state` in `tests/mod.rs` returns `Arc<dyn Transport>`. For tests that need to inspect packets, build a concrete `Arc<TestTransport>` separately and upcast:

```rust
let tt = Arc::new(TestTransport::new());
let transport: Arc<dyn Transport> = tt.clone();
// ... after handler call:
tt.drain()         // Vec<(SocketAddr, Vec<u8>)>
tt.len()           // packet count
tt.clear()         // flush between phases
```

`TestTransport` has NO `as_any()` — do NOT use downcast. Keep the concrete Arc alongside the trait Arc.

## Phase 2 deferred items (pending x64dbg verification)

- D.7: BMSearchOptions filter predicates (seller_name LIKE, item_name LIKE, price range, quality, filter_flags) not yet applied as SQL WHERE clauses — Phase 1 returns all active rows
- Pagination: client_key + sequence_id + b_forward cursor semantics unconfirmed
- exact `view` field semantics: assumed = sort_id; verify via x64dbg capture of onBMAuctions handler
