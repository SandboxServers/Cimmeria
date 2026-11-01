---
name: project-black-market-unimplemented
description: Black Market / Auction House surface is fully stubbed server-side — entire economy vertical lands as exploit chain on implementation
metadata:
  type: project
---

Method: when the Black Market server surface was ported (BM-01), the pre-port audit invariants became the review checklist. Still open at that time: search has no LIMIT or cursor paging (CAT-I-05), the 20-per-player listing cap (D5) and `clientKey`-scoped result views; `docs/analysis/black-market/` covers the search bound and the views.

For any auction or listing handler, require:
- create: ownership join on `(character_id, item_id)`, atomic item lock + fee + listing insert, `startingPrice > 0`, `buyoutPrice >= startingPrice` or 0, a length cap, a per-player listing cap.
- bid: `sequenceId` is a paginated cursor, not a row id; translate it through the caller-scoped (`clientKey`) search-result cache, lock the listing row, and check the bid server-side.
- every client-asserted field is validated; no handler may trust the wire shape just because the client emits it.

INVALIDATED WHEN: BM search gains paging and the listing cap lands
