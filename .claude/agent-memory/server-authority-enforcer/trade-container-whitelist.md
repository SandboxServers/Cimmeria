---
name: trade-container-whitelist
description: Trade swap must whitelist source container (INV_MAIN only) — blacklist-only (INV_BUYBACK) is a dupe-strip exploit
metadata:
  type: feedback
---

PR #438's `base/world_entry/methods/trade/execute.rs::lock_items` rejected only `INV_BUYBACK` (16) as a source container. Every other container the player owns rows in — `INV_MISSION` (2), `INV_BANDOLIER` (3), `INV_HEAD`..`INV_ARTIFACT2` (4..14), `INV_CRAFTING` (15), `INV_BANK` (17), bank variants (18..20) — passed the gauntlet. The atomicity of the `FOR UPDATE` swap does NOT save you here; it guarantees serializability, not eligibility.

**Why:** rule of thumb — any "what can move in this transaction" check on an inventory mutation must be a **whitelist of allowed containers**, not a blacklist of forbidden ones. New containers added in future PRs default to "can be traded" under a blacklist; they default to "cannot" under a whitelist. The latter is safe; the former is a future-bug-magnet.

**How to apply:** when reviewing any new player-to-player item mutation path (trade, mail, drop, gift, account-share), enumerate the source containers and confirm the handler does `if !TRADEABLE_FROM.contains(&row.container_id) return Err(...)`. If the only check is `row.container_id != INV_BUYBACK` or `row.bound == false`, that's the dupe-strip shape.

Canonical exploit chain:
1. Player A in trade with player B (alt account).
2. A's proposal includes `instance_id` of an INV_CHEST armor row.
3. Base validates ownership: row.character_id == A, row.bound == false, row.container_id == 7 (not 16). Passes.
4. UPDATE row SET character_id = B, container_id = INV_MAIN, slot_id = $new.
5. Cell-side cached stats on A are stale until forced recompute → A keeps the armor stat bonuses locally while B has the actual item.

Linked to [[trade-cancel-uses-completed-not-cancelled]] (the documented enum quirk) and [[trade-toctou-row-level-lock]] (the atomicity guarantee).

Spec anchor: `entities/defs/alias.xml` `LocalTradeItem` + Python `cell/Trade.py:53-58` (item validation in `_validateProposal` — the Python equivalent did check `canTrade()` per-item, not just per-bag).
