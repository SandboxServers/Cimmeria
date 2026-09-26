---
title: "Consumable via OnItemUse Pattern"
type: reference
audience: engineers
last_updated: 2026-09-19
---

# Consumable via OnItemUse Pattern

> **Type**: explanation
> **Audience**: content authors / mission designers
> **Last updated**: 2026-09-19
> **Companion docs**: [docs/content/content-engine.md](content-engine.md), [docs/content/extending-the-engine.md](extending-the-engine.md), [docs/content/equip-from-inventory-pattern.md](equip-from-inventory-pattern.md), [`.github/instructions/content-chains.instructions.md`](../../.github/instructions/content-chains.instructions.md)

When a player double-clicks an inventory item, the base handler fires a cell-side `OnItemUse` content-engine event **without consuming the stack**. Whether the item is removed is entirely the chain author's decision via `Action::RemoveItem`. This page explains when to pair the two, when to omit removal, and how the regression lint keeps new chains honest.

If you only need the recipe, jump to [The chain shape](#the-chain-shape).

## Why the base does not auto-consume

The 2009 Python server fired `item.use::<typeId>` as a pure event and let per-mission handlers decide whether to call `removeItemByDesign` ([`deprecated/python/cell/Inventory.py`](../../deprecated/python/cell/Inventory.py) around the `useItem` path). Mission scripts like Find Ambernol removed the vial; radio flows did not.

A later fanmmorpg fork changed that to auto-consume one unit on successful ability launch. **Cimmeria deliberately keeps the pre-fork pattern** — see the file-level comment in [`crates/services/src/base/world_entry/methods/inventory/core/use_instance.rs`](../../crates/services/src/base/world_entry/methods/inventory/core/use_instance.rs).

This works because:

1. There is no wire-level expectation either way. The client receives ability launches and inventory updates as separate flows.
2. The chain-decides pattern supports reusable tools (radios, worn boots, disguises) without a per-item `consume_on_use` column.
3. Authoring discipline is enforceable by lint without a wire contract.

## The chain shape

### Consumable (must include `remove_item`)

| Field | Value |
|---|---|
| Trigger | `item_use` keyed by the item's design / `type_id` (`Trigger::OnItemUse { item_id }`) |
| Condition | Whatever gates the use (mission step, `stat_below_max` for heals, etc.) |
| Actions | Effect first (heal, launch ability, advance step), then **`remove_item`** with `{"qty": 1}` targeting the same design id |

```sql
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (<chain_id>, 'item_use', '<design_id>', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (<chain_id>, 'change_stat', NULL, NULL, '{"stat_id": 7, "amount": 500}', 0, 0),
  (<chain_id>, 'remove_item', <design_id>, NULL, '{"qty": 1}', 0, 1);
```

`Action::RemoveItem` routes through `CellToBaseMsg::RemoveInventoryItemByType`, which resolves the player's first matching stack and applies the full wire-update sequence.

### Reusable (omit `remove_item`)

Same trigger shape, but **no** `remove_item` action. The item stays in inventory for repeated uses (radio check-ins, worn equipment that starts a minigame, mission disguises worn for a whole sequence).

Gate the chain with `step_status` (or similar) so a second double-click re-resolves nothing harmful — the step flip is the idempotence stop.

## Worked example: consumable — Ambernol vial (mission 639)

**Goal.** Player uses item 19 (Ambernol vial) on the active "Use the vial" step → launch cure ability, consume the vial, complete mission 639, accept 640, enable the ring switch.

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 1034 | `item_use('19')` | `step_status(639, 2343) = active` | `launch_ability(1374)` → **`remove_item(19, qty 1)`** → `complete_mission(639)` → `accept_mission(640)` → `set_interaction_type` on HackTheRings_Switch |

Seed reference: `db/resources/Content/Seed/castle_cellblock_chains.sql` (chain 1034). Mirrors `FindAmbernol.py:115` (`removeItemByDesign(19, 1, False)`).

## Worked example: consumable — Health Slappack (global)

**Goal.** Player uses item 2893 when HP is below max → heal +500, consume one slappack. At full HP the chain does not fire and the stack is preserved.

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 4001 | `item_use('2893')` | `stat_below_max(7)` | `change_stat(+500 HP)` → **`remove_item(2893, qty 1)`** |

Seed reference: `db/resources/Content/Seed/consumables_chains.sql` (chain 4001).

## Worked example: reusable — Radio (mission 1561)

**Goal.** Player uses item 5168 (radio) at two different mission steps — advance to bomb defusal, then complete the mission after defusing. The radio is **not** consumed either time.

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 3021 | `item_use('5168')` | `step_status(1561, 4621) = active` | dialog → advance step → set NaqBomb interaction |
| 3025 | `item_use('5168')` | `step_status(1561, 4623) = active` | complete mission → dialog |

Neither chain includes `remove_item`. Seed reference: `db/resources/Content/Seed/sgc_w1_chains.sql` (chains 3021, 3025).

## Baseline audit (HEAD)

Every `item_use`-triggered chain in seed data as of issue #332. The content-engine regression lint in `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs` enforces this table — new chains must update the allowlists in that test.

| Chain ID | Item ID | Description | Chain file | Has `remove_item` | Intent |
|---|---|---|---|---|---|
| 1034 | 19 | Ambernol vial (mission 639) | `castle_cellblock_chains.sql` | yes | Consumable — correctly removed |
| 4001 | 2893 | Health Slappack TC1 | `consumables_chains.sql` | yes | Consumable — correctly removed |
| 1024 | 3438 | Prison Boots (mission 689 Livewire) | `castle_cellblock_chains.sql` | no (intentional) | Reusable worn equipment |
| 3021 | 5168 | Radio (bomb defusal step) | `sgc_w1_chains.sql` | no (intentional) | Reusable tool |
| 3025 | 5168 | Radio (mission complete) | `sgc_w1_chains.sql` | no (intentional) | Reusable tool |
| 6103 | 2819 | Jaffa Disguise (mission 742) | `harset_goauld_chains.sql` | no (intentional) | Reusable disguise |

## When adding a new `item_use` chain

1. Decide **consumable vs reusable** before writing SQL.
2. If consumable, end the action list with `remove_item` for the same design id (after any heal/ability/advance).
3. If reusable, omit `remove_item` and gate with `step_status` (or equivalent) so repeat uses are harmless.
4. Add the item id to `KNOWN_CONSUMABLES` or `KNOWN_REUSABLES` in `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs`. The test fails on unknown ids with *"add to one of the two lists"* — that forces an explicit author decision. The lint matches on the **item id**, not just on the presence of a `remove_item` row: a consumable's chain must remove the item that was used (a row copied from another chain and never retargeted is reported), and a reusable's chain may remove a *different* item, such as a turn-in token.

## Cross-links

- [docs/content/content-engine.md](content-engine.md) — runtime reference for triggers and actions.
- [docs/content/equip-from-inventory-pattern.md](equip-from-inventory-pattern.md) — sibling pattern for weapon grants (uses `item_equipped`, not `item_use`).
- [`.github/instructions/content-chains.instructions.md`](../../.github/instructions/content-chains.instructions.md) — PR review checklist including inventory consumption.
- [`crates/services/src/base/world_entry/methods/inventory/core/use_instance.rs`](../../crates/services/src/base/world_entry/methods/inventory/core/use_instance.rs) — base handler that fires `OnItemUse` without consuming.
- [TESTING.md](../../TESTING.md) — the seed SQL linter tests are unit tests over the Content/Seed tree (no DB required).
