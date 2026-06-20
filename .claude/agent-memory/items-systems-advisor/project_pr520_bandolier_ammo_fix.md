---
name: pr520-bandolier-ammo-fix
description: PR #520 review findings — bandolier ammo persistence keyed on instance_id not type_id (issue #445)
metadata:
  type: project
---

PR #520 (`fix/445-bandolier-ammo-item-id`) is SHIP-WITH-NITS. Reviewed 2026-06-19.

**The fix is correct.** `update_bandolier_ammo` WHERE clause changed from `type_id = $5` to `item_id = $5`, closing the same-type-swap TOCTOU dupe. All 4 sender sites (flush / slot-swap / ammo-change / reload-completion tick) updated. New live-DB revert-verifying test added.

**Key verified facts:**

- `BandolierItem.instance_id` = `sgw_inventory.item_id` (per-row sequence PK). `BandolierItem.item_id` = design id (`sgw_inventory.type_id`). Both coexist; design lookups use item_id, persistence guard uses instance_id.
- The 4 sender sites are: `flush_dirty_bandolier_ammo`, `handle_request_active_slot_change` (slot-swap path), `handle_request_ammo_change`, and `reload_completion_tick` in `ticks/mod.rs`.
- `requestAmmoChange` (cell method 42): client-supplied `item_id` is used ONLY to resolve the slot via `item.item_id == item_id` filter. The captured persist guard is `item.instance_id` — design id never reaches the WHERE bind.
- Content-engine optimistic grant seeds `instance_id: 0`; base-side guard `expected_instance_id <= 0` drops those persists. Correct.
- `sgw_inventory.item_id` IS declared as PRIMARY KEY via `ALTER TABLE ONLY sgw_inventory ADD CONSTRAINT sgw_inventory_pkey PRIMARY KEY (item_id)` in `db/sgw/_primary_keys.sql`. The table uses `INHERITS (sgw_inventory_base)` — PostgreSQL doesn't inherit constraints, so both parent and child have their own PKs. The PR doc comment saying "unique by construction rather than by a declared PK/UNIQUE constraint" is WRONG — there IS a declared PK.

**Nit (doc inaccuracy):** `BandolierItem.instance_id` doc comment in `crates/entity/src/cell_entity/mod.rs` and the PR description both say "unique per slot via the sgw_inventory_unique_slot index — unique by construction rather than by a declared PK/UNIQUE constraint." This contradicts `_primary_keys.sql`. The declared PK on `sgw_inventory` does exist. No bug risk, but the comment is misleading.

**Type_id audit:** `remove_by_type` intentionally keys on `type_id` (design id) by name and purpose — chains know design ids, not instance ids. `remove_instance`, `move_`, grant stack-merge all key on `item_id` (instance). The ammo `UPDATE` is the only case that was wrongly keying on `type_id`; fix is complete.

**Why:** Closes #445 — same-design weapon swap let stale ammo writebacks scribble the new physical instance's ammo count, creating a dupe vector.
