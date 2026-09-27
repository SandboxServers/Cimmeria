---
name: reference-black-market-serve
description: Black Market search/serve implementation: CellToBaseMsg::BMSearch shape, wire format for onBMAuctions (method 92), where each piece lives after the crate split, and test patterns.
metadata:
  type: reference
---

Written on `feat/571-black-market-phase1` (June 2026); paths updated when BM-01 ported the branch onto the split crates (2026-09-27). The known client-contract mistakes in this shape (S1-S8 in `docs/analysis/black-market/README.md`) are packet BM-02's job; this note records what the code does, not what the client wants.

## Wire / method indices

- Client methods 90-95 (`ON_BM_OPEN` .. `ON_BM_WATCHED_ITEMS_UPDATE`): `crates/wire/src/cell/client_methods/black_market.rs`. The branch had copied them into `mercury::method_idx`; the port uses the per-interface table instead (see the rust-gameserver-dev note on `method_idx` drift).
- Cell methods 61-66 (`SEARCH`, `CREATE_AUCTION`, `PLACE_BID`, `CANCEL_AUCTION`, `START_WATCHING`, `STOP_WATCHING`): `crates/wire/src/cell/cell_methods/black_market.rs`, re-exported by the cell-methods handler.

## CellToBaseMsg::BMSearch shape

```rust
BMSearch {
    entity_id: u32,
    player_id: i32,
    options: BMSearchOptions,   // full 11-field struct, decoded cell-side
}
```

`BMSearchOptions` and its decoder are `cimmeria_wire::black_market` (a message payload, so it sits below both halves); `base::black_market::types` re-exports it.

## onBMAuctions wire layout (serialize_on_bm_auctions)

```text
[u32 LE count]
[AuctionItem × count]        -- via push_auction_item (33 fixed bytes + STRING sellerName)
[i32 LE view]                -- sort_id echoed back as i32
[i32 LE total]               -- total matching rows (= count, no pagination)
```

AuctionItem field order (push_auction_item):
`INT32 sequenceId, INT32 itemDefId, INT32 stackSize, INT32 durability,
 INT32 charges, INT32 currentBid, INT32 buyoutPrice, UINT8 endTimeValue,
 INT32 nextMinBidPrice, STRING sellerName`

Strings are STRING (4-byte LE length prefix + UTF-8), NOT WSTRING.

## Where the pieces live

- Cell decode + forward: `crates/cell-methods/src/cell/cell_methods/black_market/mod.rs` (tests in `tests.rs` beside it).
- Base routing: `crates/base-world-entry/src/base/world_entry/cell_dispatch/black_market_dispatch.rs`.
- Base handlers, serializers, sweep, seed: `crates/base-session/src/base/black_market/` (beside `contact_list` and `crafting`). `payout_mail.rs` holds `send_mail_to_player` and the settlement mail texts, isolated so BM-02b can swap in the social-systems mail API.
- Startup: `crates/base/src/base/service.rs` spawns `seed::spawn_seed` and `sweep::spawn_sweep` when a DB pool is configured.
- `onBMOpen`: `crates/cell-content/src/cell/content/executor/black_market.rs`, serializer `cimmeria_wire::black_market::serialize_on_bm_open`.

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

`TestTransport` has NO `as_any()`, so do not downcast. Keep the concrete Arc alongside the trait Arc.

Live-DB sentinels are the `0x7000_Axxx` block (`tests/mod.rs` `TEST_BASE`). The branch's original `0x7000_0900` / `0x7000_0E00` bases inserted the same account and player ids as the vendor tests.
