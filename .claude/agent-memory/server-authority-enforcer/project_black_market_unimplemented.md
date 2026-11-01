---
name: project-black-market-unimplemented
description: Pre-port CAT-I audit invariants for Black Market / auction handlers, kept as a review checklist (the server surface was ported in BM-01, 2026-09-27)
metadata:
  type: project
---

The BM surface was ported in BM-01 and search is now LIMIT-bounded (search.rs:132). Whether the 20-per-player listing cap (D5) and clientKey-scoped result views have landed is unchecked; read black_market/ before relying on it.

For any auction or listing handler, require:
- create: ownership join on `(character_id, item_id)`, atomic item lock + fee + listing insert, `startingPrice > 0`, `buyoutPrice >= startingPrice` or 0, a length cap, a per-player listing cap.
- bid: `sequenceId` is a paginated cursor, not a row id; translate it through the caller-scoped (`clientKey`) search-result cache, lock the listing row, and check the bid server-side.
- every client-asserted field is validated; no handler may trust the wire shape just because the client emits it.

INVALIDATED WHEN: the BM create/bid handlers change their validation

checked: unverified 2026-11-01
