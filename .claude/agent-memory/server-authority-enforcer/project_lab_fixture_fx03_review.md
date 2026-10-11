---
name: project-lab-fixture-fx03-review
description: FX-03 lab fixture item write (2026-10-11) cleared - player-keyed, containers 1-15 only, rolled-back dry run; only residue is burned item_id sequence values
metadata:
  type: project
---

FX-03 (`base-world-entry/.../lab_fixture/items.rs`): fixture item = delete every instance of the design in containers 1..=15, grant `count` (<=10) fresh into bag 1 or 2, reconcile `sgw_player.bandolier_slot`. Precheck dry-runs the same `set_counts` in a tx and rolls it back.

Cleared shape (reuse when reviewing similar "set exact state" writers):
- Every statement keyed by the ownership-checked `player_id`; all SQL bound; ranges come from `FIXTURE_CARRIED_CONTAINERS`, never from the staged data.
- `sgw_inventory` has no inheriting children and no FK pointing at its `item_id` (mail/org vault tables are standalone), so the unqualified DELETE reaches nothing else.
- Lock order = `take_inventory_locks` (player-wide `(p,0)` then bags sorted) -> rows -> `sgw_player`; same order as crafting/mail; vendor/move serialize on `(p,0)`, trade waits on `(p,1)` before any row. Keys are this player's only.
- Dry run side effects: advisory locks are xact-scoped (released at rollback; sqlx 0.9 `return_to_pool` pings to flush a drop-queued ROLLBACK); no triggers on `sgw_inventory`; only `sgw_inventory_item_id_seq` advances (shared with mail items; gaps only).

Not a cap: the list length of distinct designs is unbounded in the store; bag room bounds it.

**How to apply:** if a later packet adds bank/vault containers, a list-length cap, or a non-GM caller, re-review; see [[project-lab-fixture-fx02-review]] and [[advisory-lock-namespaces]].
