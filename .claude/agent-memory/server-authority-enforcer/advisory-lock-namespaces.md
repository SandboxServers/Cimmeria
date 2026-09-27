---
name: advisory-lock-namespaces
description: pg_advisory_xact_lock key assignments and lock ORDER across inventory/cash writers (vendor, move, crafting, trade, mail); where the orders still diverge
metadata:
  type: reference
---

`pg_advisory_xact_lock(player_id, key)`: PG treats each `(player, key)` pair as an independent lock, so two paths only serialize if they share a key.

State as of 2026-09-27 (PR #912 review):

| Path | Keys taken | Row order after the advisory locks |
|---|---|---|
| Inventory move | `(p,0)` then `(p,target)` then `(p,source)` | inventory rows |
| Vendor purchase | `(p,0)`, later `(p,bag)` | inventory rows -> `sgw_player` -> bag key |
| Crafting (`take_inventory_locks`) | `(p,0)` then bags sorted | inventory rows -> `sgw_player` |
| Trade `atomic_swap` | `(lo,1)`, `(hi,1)` (INV_MAIN) | `sgw_player` p1 then p2 (**unsorted**) -> item rows |
| Mail send with item (SS-M2) | `take_inventory_locks(sender,[1])` | item row -> `sgw_player` rows ascending |
| Mail send, text or cash-only | none | `sgw_player` rows ascending |

Known divergence: trade locks the two player rows in p1/p2 order, not ascending, and before item rows. Against any path that locks the same two player rows ascending WITHOUT first holding one of trade's advisory keys (text mail, cash-only mail), a trade whose p1 has the higher id deadlocks; PG aborts one side. Availability only, no corruption. Fix candidates: sort trade's `read_naquadah_for_update` by lo/hi, or have every attached mail take the sender's advisory keys.

**How to apply:** for any new writer, list its advisory keys and its row order, and check each pair of writers that can touch the same two players for an ABBA. The shared order the codebase converged on is documented in `crates/base-session/src/base/crafting/inventory_locks.rs`.
