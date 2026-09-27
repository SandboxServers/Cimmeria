---
name: grant-placement-and-loot-handback-traps
description: Where a grant lands is decided on the base by container_sets (storage falls through to a carried bag); loot removes before the grant and relies on LootGrantRefused; traps when touching grants, vendor tests or loot table 3
metadata:
  type: project
---

Since CR-16 (2026-09-27) `handle_grant_item` re-places every request by the item's `container_sets` (`item_placement::grant_container`): a storage (17-20) request or a carried-bag request the item does not allow goes to the first carried bag it lists. Only storage-only items reach BV-01's `grant_rejected`. The cell cache (`load_item_containers`) holds the first non-storage entry.

Traps:
- Any test that grants a seeded `{17,15}` item and asserts bag 1 or bag 17 is now wrong: vendor 25's two lines (5228, 5192) are `{17,15}`, so purchases land in 15. No seeded item is storage-only; a refusal test needs a synthetic `{17}` item (`0x7000_C4F0`/`C4F1` used).
- `CellToBaseMsg::GrantItem` has `loot: Option<LootGrantSource>`; every new constructor sets `None` except loot pickup.
- Loot takes the item off the corpse first. Only a `PersistOutcome::Refused` (built before COMMIT) may send `LootGrantRefused`; a COMMIT error that is not `sqlx::Error::Database` is `CommitUnknown` and must never hand back (dup).
- Loot table 3 is no longer all probability 1: guards assert "certain rows ⊆ dropped ⊆ table". Next free loot_id is 24.
- The fresh-slot grant INSERT does not cap `stack_size` at `max_stack_size` (merge path does); keep loot quantities of stack-1 items at 1.

**Why:** these broke tests or would silently dup/lose items when missed.
**How to apply:** read before changing the grant path, vendor purchase placement, loot pickup, or loot seeds. See [[ability-launch-fire-split]] for another split-in-time server path.
