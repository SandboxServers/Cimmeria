---
name: project-black-market-bm07-review
description: BM-07 auctioneer marker + system-seller read-back + GM .bm_* tools review (2026-09-27); cleared shape and residuals
metadata:
  type: project
---

BM-07 (branch bm/07-content-uat) reviewed 2026-09-27. Verdict: CONDITIONAL/SHIP-shaped, no BLOCK.

Cleared:
- `NpcInteractionType::Auctioneer` is written only in spawn.rs `static_interaction_for_flags` (from template INT_AUCTION). Content `set_interaction_type` and console `.setinteraction` touch only `interaction_type_flags`, so they cannot mint an auctioneer. Re-grep `interaction_type =` writers on any future review.
- `auctioneer_check` (cell-world black_market.rs) runs on open and on every 62-64 call; id recycling / stale `last_interaction_target` / truncated `as u64 as i32` param all resolve to a live id that must still pass type+space+range.
- `.bm_expire`: conditional UPDATE on ACTIVE, then `settle_one` re-locks FOR UPDATE + status-guarded UPDATE, bid validate requires `expires_at > now` -> no double settle, bid race safe.

Residuals:
- FIXED in the same PR: `is_seed_listing` was `item_id == 0 || seller_id == 1` (a real player 1 would mint on missing escrow and strand unsold items); now `item_id == 0` only, pinned by `only_an_instance_free_listing_is_a_seed_listing`.
- FIXED in the same PR: older boots inserted account 1 enabled; `ensure_system_seller` now switches an enabled `Black Market` account 1 off (`an_older_enabled_system_account_is_disabled`).
- Auctioneer role is not restored on respawn (death writes Loot); template relies on faction 1 being unkillable.

**How to apply:** start BM-08+ reviews from these residuals; related [[project-black-market-bm02-review]].
