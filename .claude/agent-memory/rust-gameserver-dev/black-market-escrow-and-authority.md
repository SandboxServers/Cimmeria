---
name: black-market-escrow-and-authority
description: BM-02 (2026-09-27) - listed items live in the seller's container 18; every client-bound inventory read must exclude 18; the BM lock order; 62-64 are gated on a cell session set by open_black_market; server BM codec is patch-wire via cimmeria_wire::black_market
metadata:
  type: project
---

Learned implementing Black Market packet BM-02 (branch `bm/02-contract-fixes`).

- **Container 18 is live now.** A listing moves the `sgw_inventory` row into the seller's container 18 (`INV_AUCTION`), same id and owner. Four client-bound reads exclude it with `container_id <> 18`: base-methods `player_load/core` and `inventory/core` `INVENTORY_ITEM_SELECT` (byte-pinned to each other), `INVENTORY_ONE_ITEM_SELECT` (the refused-move snap-back; its SQL tail is pinned in `one_item_select_tests.rs`), and base-session crafting `client_sync::INVENTORY_ITEMS_SELECT` plus `gate::item_disciplines`. Writers are protected by the container allowlists (move `Movable::No`, `CARRIED_CONTAINERS`, `CRAFTING_INPUT_BAGS`, `VENDOR_*_BAGS`, trade `TRADEABLE_CONTAINERS`, mail `ITEM_IN_VAULT`). **A new inventory read that goes to the client must exclude 18**, or a listed item reappears in the bags.
- **Never mint on a missing escrow row.** `escrow::escrowed_item` returns `Minted` only for boot-seed listings (`item_id == 0` or seller = system seller 1); a player's listing with no container-18 row is `None` (`bm.escrow_missing`), or any future path that removes an 18 row becomes an item dupe.
- **Since BM-02b the module is `cimmeria-base-methods` `methods/black_market/`** (moved from base-session: the system-mail writer lives in base-methods, which depends on base-session). Every payout is `payout_mail::mail_payout` -> `send_system_mail_tx`; see [[bm-settlement-mail-traps]].
- **Lock order.** Create: seller advisory inventory locks -> item row -> seller `sgw_player` (serializes the 20-listing cap). Bid/cancel/settle: `sgw_auction` row -> seller escrow advisory (`lock_escrow`) -> escrowed item row -> every player row ascending (`lock_players`, mail recipients included) -> the mail writer (re-locks only). No writer may take an advisory lock and then an `sgw_auction` row lock.
- **Authority.** `cimmeria_cell_world::cell::black_market` holds `BlackMarketSessions` (keyed by player_id, on `SpaceManager.black_market`), written only by the `open_black_market` executor after `onBMOpen` is sent. `black_market_access` also requires `last_interaction_target == auctioneer` and `interact_range`. Nothing checks the NPC is an auctioneer (no interaction type) - BM-07.
- **Codec.** The server's BM encode/decode is `cimmeria-patch-wire`, re-exported as `cimmeria_wire::black_market` (only `cimmeria-wire` depends on it directly). Error ids are `BMError` there; reason labels are the log `reason`.
- `adjust_player_cash` does bigint math with a `BETWEEN 0 AND 2147483647` guard; `$1::int` arithmetic raised 22003 on a big refund.

Related: [[mail-escrow-lock-order-and-proof-traps]], [[move-path-lock-layers-and-vault-verdict]], [[per-session-player-state-lifecycle]].
