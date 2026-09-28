---
name: project-black-market-bm02-review
description: BM-02 (2026-09-27) cleared shape for BM auctioneer gate + container-18 escrow; residual mint-on-missing-escrow fallback and int4 refund overflow
metadata:
  type: project
---

BM-02 review (commit 5eb202d55, 2026-09-27). Cleared shape, reuse when
re-reviewing Black Market or any "listed item stays in the owner's
inventory in a server-held container" design:

- **Cell gate** `cell-world/src/cell/black_market.rs::black_market_access`:
  server-recorded session (only `open_black_market` executor writes it) +
  `last_interaction_target == pinned auctioneer` + `interact_range`. The pin
  is only written by `interact` after its own range gate, so a client cannot
  name an auctioneer. Gate travel is safe either way (fresh pin, or
  `OtherSpace`). Residual: `open` never checks the resolved entity IS an
  auctioneer, so any chain that runs `open_black_market` off a non-auctioneer
  makes that NPC a terminal (content-authoring risk); entity-id recycling
  applies ([[exploit-entity-id-recycling]]).
- **Escrow** = whole row moved to container 18, still owned by seller; no
  client quantity. Every player-reachable path gates with
  `player_accessible`/`CARRIED_CONTAINERS`/`CRAFTING_INPUT_BAGS`/
  `accessible_containers`, which all exclude 18 (checked remove_instance,
  remove_by_type, use_instance, crafting item_use/consume/alloy, vendor cost).
  New inventory writers must be checked against 18 too.
- **Latent dupe amplifier**: `escrow.rs::deliver_from_escrow` MINTS from the
  auction snapshot whenever the escrow row is missing, keyed on absence, not
  on `seller_id == SYSTEM_SELLER_ID`. Any future path that deletes/moves a
  container-18 row turns the next sale/cancel into a mint.
- BM lock order is `sgw_auction FOR UPDATE -> advisory -> sgw_player -> item
  row` (not the documented advisory-first order); safe because no writer
  takes advisory then `sgw_auction`. Recheck if one ever does.
- `helpers.rs::adjust_player_cash` does `naquadah + $1::int` in int4: a
  refund that pushes the prior bidder over i32::MAX raises, failing the
  outbid/cancel (griefing lock), not a mint.
- Delete trigger `bm_player_before_delete` conserves cash except the LEAST
  cap (silent loss at i32::MAX); no mint path found.

**How to apply:** start BM re-reviews from these residuals; related
[[project-black-market-unimplemented]], [[exploit-bind-on-acquire-unenforced]]
(BoA rows still list, `bound` is the only check).
