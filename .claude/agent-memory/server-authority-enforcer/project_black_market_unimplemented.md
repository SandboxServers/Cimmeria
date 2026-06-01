---
name: project-black-market-unimplemented
description: Black Market / Auction House surface is fully stubbed server-side — entire economy vertical lands as exploit chain on implementation
metadata:
  type: project
---

The SGW Black Market / Auction House surface (`SGWBlackMarketManager`,
CellMethod indices 61–66) is fully unimplemented in
`crates/services/src/cell/cell_methods/black_market.rs`. Every arm
returns `true` (handled) after a `tracing::info!("UNIMPLEMENTED: …")`,
no DB tables back the surface, no `ON_BM_*` ClientMethod is ever
emitted from the server, no expiry sweep exists.

**Why:** This is the same shape as [[project-mail-handlers-unimplemented]]
and [[project-trade-handlers-unimplemented]] — the wire surface is
fully exposed (Ghidra confirms the client emits
`Event_NetOut_BMCreateAuction`, `BMPlaceBid`, `BMCancelAuction`,
`BMSearch` with their full payloads), but the server stubs the
mutation. Whoever fills in the handler bodies inherits a wire
surface that accepts every client-asserted field with no
validation plumbed in and no DB schema to enforce ownership /
atomicity. The audit (CAT-I, 2026-05-31) pre-flagged six findings
spanning create / bid / cancel / search / expiry-sweep so the
invariants are nailed down before the implementation PR lands.

**How to apply:** When auditing or designing the BM implementation,
require:
- BMCreateAuction: ownership join on `(character_id, item_id)`,
  atomic item lock + listing-fee deduction + listing insert,
  `startingPrice > 0`, `buyoutPrice >= startingPrice` (or 0),
  `auctionLength` cap, per-player active-listing cap.
- BMPlaceBid: `sequenceId` is a *paginated cursor*, not a row
  id — must translate through caller-scoped (`clientKey`-keyed)
  search-result cache; lock listing row, server-side bid >
  current_bid comparison, currency hold at bid time, refund
  prior bidder atomically, reject self-bid.
- BMCancelAuction: ownership check, return escrowed item +
  refund any active bidder atomically.
- BMSearch: server-side result-size cap (e.g. 50 rows),
  faction / region filter enforced server-side,
  `(character_id, clientKey)` cache keying.
- Expiry sweep: needs Tokio periodic tick + outbox-pattern
  delivery (mail vertical is the natural surface but mail is
  also unimplemented — see [[project-mail-handlers-unimplemented]]).

Findings file: `.scratch/audit/findings/CAT-I-black-market.md`.
