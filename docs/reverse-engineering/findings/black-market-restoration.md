# Black Market / Auction House — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire format: RTTI + emitter decompile); MEDIUM (CoD/payout flow: architecture-inferred); LOW (error/watch-update enums)
> **Sources**: `SGW.exe` Ghidra; `entities/defs/SGWBlackMarket.def`; `entities/defs/SGWEscrow.def`;
>   `deprecated/python/{base,cell}/SGWBlackMarket.py`; `deprecated/python/base/SGWPlayer.py`;
>   `crates/services/src/cell/{cell_methods,client_methods}/black_market.rs`; `db/sgw/` (no auction tables);
>   `docs/reverse-engineering/findings/black-market-wire-formats.md`
> **Tracking issue**: replaces #67

## Completeness assessment

Player-to-player auction house. Client class `BlackMarket` (RTTI). UI is **CEGUI** (`UIAuctionView`,
`UIAuctionTime`), not Scaleform. The Python server is **empty stubs** (both `SGWBlackMarket.py` files
`__init__`-only; `BMSearch`/`BMCreate`/`BMPlaceBid`/`BMCancel`/`BMStartWatch`/`BMStopWatch` all `pass`).
`SGWEscrow` is **loot-transfer**, NOT auction settlement (it has no BaseMethods). CoD delivery reuses the
existing `sgw_gate_mail` table.

| Layer | % |
|---|---|
| Wire format docs | ~90% (`filterFlags` gap below) |
| Entity model | ~85% (SGWEscrow misclassification clarified) |
| Server logic | ~0% |
| DB persistence (auction tables) | ~0% (none exist) |
| **Overall** | **~0% functional** |

## ⚠️ Corrections to existing artifacts

1. **`BMCreateAuction` Rust decode is wrong**: `cell_methods/black_market.rs` reads `auctionLength` as
   INT32; the binary emitter `FUN_00e59970` packs it as **UINT8**. Correct payload is **13 bytes** (4+4+4+1),
   not 16. HIGH confidence.
2. **`BMSearchOptions` has 11 fields, not 10**: emitter `FUN_00e59f70` shows an 11th field `filterFlags`
   (INT32) at `puVar1+0x15`. The existing doc lists `monikerCRC` in that position — likely a wrong/aliased
   name. HIGH confidence.

## Entity model

**SGWBlackMarket** (`<ServerOnly/>`, no parent): one property `watchedItems: PYTHON` (BASE) — itemDefId →
subscriber mailbox registry. Six base methods (all pass the caller MAILBOX so the singleton replies to the
player base): `searchBlackMarket(MAILBOX, INT32, BMSearchOptions, INT32 langId)`, `placeBid(INT32 seqId,
INT32 bid)`, `createAuction(MAILBOX, INT32, INT32 itemInstance, INT32 buyout, UINT8 length, INT32 starting)`,
`cancelAuction(MAILBOX, INT32, INT32 seqId)`, `registerWatchedItems(ARRAY<INT32>, MAILBOX)`,
`unregisterWatchedItems(ARRAY<INT32>, MAILBOX)`.

**SGWEscrow** (clarified): loot-roll/instanced-loot transfer, two CELL_PRIVATE props + two CellMethods
(`lootItemTransfer`, `lootItemTransferCallback`), **zero BaseMethods** → not auction settlement.

## Wire messages

### Client → Server (NetOut)

- `BMCreateAuction` (RTTI `0x01e660b8`, emitter `0x00e59970`): `INT32 itemInstanceId, INT32 startingPrice,
  INT32 buyoutPrice, UINT8 auctionLength` — **13 bytes**.
- `BMCancelAuction` (`0x01e66138`, `0x00e59c70`): `INT32 sequenceId` — 4B.
- `BMSearch` (`0x01e6622c`, `0x00e59f70`) `BMSearchOptions` (11 fields, packing order): `UINT8 sortId,
  INT32 clientKey, INT32 sequenceId, UINT8 bForward, STRING sellerName, STRING bidderName, STRING itemName,
  INT32 minTC, INT32 maxTC, INT32 quality, INT32 filterFlags`.
- `BMPlaceBid` (`0x01e661b8`, register `0x00e5c740`): `INT32 sequenceId, INT32 bidAmount` — 8B (emitter body
  not decompiled; MEDIUM).

### Server → Client (NetIn — `BlackMarket` subscribes to all five)

`BMOpen` (`0x01e4d4b0`/reg `0x00d83ec0`), `BMAuctions` (`0x01e4d51c`/`0x00d84160`), `BMAuctionUpdate`
(`0x01e4d590`/`0x00d84400`), `BMAuctionRemove` (`0x01e4d610`/`0x00d846a0`), `BMError` (`0x01e4d690`/`0x00d84940`).
`onBMWatchedItemsUpdate` (Rust client idx 95, `ARRAY<INT32>`) — **no matching `Event_NetIn_BMWatched*` RTTI
found** → delivery path unknown (open Q).

## UI / CoD

CEGUI Lua bindings confirmed: `createAuction` (`0x00aabf70`, 5 args), `refreshMyAuctions` (`0x00aac090`,
seeds sellerName from `GamePlayer+0x10c`), `getAuctionItemInfo` (`0x00aac260`), `cancelAuction` (`0x00aac1e0`),
`searchAuctions` (`0x00aca360`, 11 args).

**CoD delivery** uses `sgw_gate_mail` (has `cash`, `item_id`, `flags`) via `payCODForMailMessage` +
`sendMailMessage(bCOD)`. Inferred payout: winner pays bid; seller gets a cash mail; buyer gets an item mail
(or a CoD mail). MEDIUM confidence (no Python impl).

## Open questions

1. `EBlackMarketError` enum values (no string defs found). → x64dbg D.1.
2. `onBMWatchedItemsUpdate` delivery trigger (no RTTI). → x64dbg D.2.
3. `BMPlaceBid` emitter field layout (decompile timed out). → x64dbg D.3.
4. `filterFlags` semantics (category/faction/mode?). → x64dbg D.4.
5. `auctionLength` UINT8 enum values (12h/24h/48h?). → x64dbg D.5.
6. `nextMinBidPrice` formula. → x64dbg D.6.

## Dynamic-analysis needs (x64dbg)

- **D.1** BP `0x00d84940` (BMError register) — trace callers; capture INT32 errorId per dispatch.
- **D.2** Trace client method-95 dispatch via the `BlackMarket` MemberCallback cluster (`0x01e65a10`–`0x01e65ee8`).
- **D.3** Step `register_NetOut_BMPlaceBid` (`0x00e5c740`) emit path for field layout.
- **D.4** BP `0x00e59f70` — capture `filterFlags` across search tabs (All / My listings / Watched).
- **D.5** BP `0x00e59970` — capture the `auctionLength` byte per duration option.
- **D.6** Capture `currentBid`→`nextMinBidPrice` pairs across several bids.

## Ghidra annotations

None applied. Recommended renames: `0x00e59970`→`BMCreateAuction_NetOut_emit`,
`0x00e59c70`→`BMCancelAuction_NetOut_emit`, `0x00e59f70`→`BMSearch_NetOut_emit`, plus the CEGUI Lua-binding
functions listed above.
