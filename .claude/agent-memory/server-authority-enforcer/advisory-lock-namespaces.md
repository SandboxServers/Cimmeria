---
name: advisory-lock-namespaces
description: PostgreSQL pg_advisory_xact_lock namespace assignments across the inventory / cash mutation paths — keep them consistent
metadata:
  type: reference
---

Multiple paths in `crates/services/src/base/world_entry/methods/` take `pg_advisory_xact_lock(player_id, ns)` to serialize themselves against concurrent mutations on the same player. The choice of `ns` matters because PG treats `(key1, key2)` advisory locks as independent — two paths with different `ns` values do NOT block each other at the advisory layer (only the row-level FOR UPDATE locks save you, and only on rows they both touch).

As of PR #438:

| Path | Advisory key | Namespace |
|---|---|---|
| Vendor stack (sell, purchase, recharge, repair, slot reserve) | player_id | container_id (typically `INV_MAIN=1`) |
| Trade `atomic_swap` (PR #438) | player_id | `0` |

This is a divergence — they don't block each other. Correctness is preserved by the row-level FOR UPDATE on `sgw_inventory` rows (both paths take it), but the deadlock surface widens: trade and vendor can lock in different orders (trade does naquadah-first, vendor does items-first) and the resulting ABBA gets resolved by PG's deadlock detector with a noisy `Cancelled` to the trade clients.

**How to apply:** when reviewing any new inventory mutation that uses `pg_advisory_xact_lock(player_id, X)`, check what `X` is and confirm it matches the convention for the path. If trade gets fixed to use `(player_id, 1)` (INV_MAIN container_id), expect a third namespace to emerge soon for some other reason — document it here.

Recommendation in the trade review: standardize trade on `(player_id, 1)` so it serializes against vendor on the same container, OR adopt `(player_id, FIXED_TRADE_NS)` and document the assignment table.

Lock acquisition order matters for deadlock avoidance:
- Vendor: items FOR UPDATE → naquadah FOR UPDATE.
- Trade: advisory → naquadah FOR UPDATE → items FOR UPDATE.

These should converge to a single order.
