---
name: move-path-lock-layers-and-vault-verdict
description: moveItem has three serialization layers (player,0) + (player,container) advisory + source FOR UPDATE; a concurrency revert proof must strip all of them; vault access is a per-request cell verdict on CellToBaseMsg
metadata:
  type: project
---

**Lock layers in `move_/`.** A move takes `pg_advisory_xact_lock(player, 0)`, then `(player, target_container)` and `(player, source_container)`, then `FOR UPDATE` on the source row and the occupant. Two moves into the same container serialize on `(player, container)` alone. A concurrency guard's revert proof (BV-03 `vault_concurrency_tests`) still passed with the `(player, 0)` lock and `FOR UPDATE` removed. It failed only once the per-container locks, the merge `rows_affected` check and the split `stack_size > $1` guard were stripped too (the revert set is in the BV-03 worknote).

**Vault access is a verdict, not state on the base.** The vault session lives on the cell's `CellEntity`. For every forwarded `moveItem`, `useItem`, `removeItem`, content `RemoveItem` and `gmRemoveItem`, the cell takes `vault_access(entity_id, space_mgr)` and attaches the result to the `CellToBaseMsg`. The result is `cimmeria_wire::cell::vault::VaultAccess`, carrying `scope`, `banker_id` and `distance`. The base applies it only where 17 is touched. In-process callers pass `VaultAccess::NO_SESSION`, as does content by-type removal, which is deliberate.

- The range rule `interact_range` and `vault_move_allowed` live in `cimmeria-cell-world`, because `cimmeria-cell-content` cannot depend on `cimmeria-cell-interactions`. `cimmeria-cell-interactions` re-exports both at their old paths.
- Before BV-03 the Rust `moveItem` had no stack merge, though the docs said "stack merging" was done. The merge now exists (legacy `Inventory.py:391`) and requires equal `bound`, `durability` and `charges`.

**Why:** a later packet (BV-05 expansion, org vaults BV-07) or a crafting/move change will touch the same seams.

**How to apply:**
- For a new vault-like container, carry its scope in the verdict and check it in `container_policy`.
- For any move concurrency guard, prove it with every lock layer removed.

See [[inventory-lock-keys-and-failure-injection]], [[container-capacity-and-grant-targets]].
