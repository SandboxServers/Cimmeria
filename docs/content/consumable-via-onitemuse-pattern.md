---
title: "Consumable via OnItemUse Pattern"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# Consumable via OnItemUse Pattern

> **Type**: explanation
> **Audience**: content authors / mission designers
> **Last updated**: 2026-09-28
> **Companion docs**: [docs/content/content-engine.md](content-engine.md), [docs/content/extending-the-engine.md](extending-the-engine.md), [docs/content/equip-from-inventory-pattern.md](equip-from-inventory-pattern.md), [`.github/instructions/content-chains.instructions.md`](../../.github/instructions/content-chains.instructions.md)

When a player double-clicks an inventory item, the base handler fires a cell-side `OnItemUse` content-engine event **without consuming the stack**. Whether the item is removed is entirely the chain author's decision via `Action::RemoveItem`. This page explains when to pair the two, when to omit removal, and how the regression lint keeps new chains honest.

A plain heal or buff item needs no chain at all. If the item's `items_event_sets` row binds it (event 5) to an ability whose effects are all `HealHealth`, `HealFocus` or `StatBuff`, the cell applies that ability itself and the base consumes one unit first; see [Native consumables](#native-consumables-items_event_sets-event-5). Write a chain only when using the item has to do something a heal or buff cannot: advance a mission, check a mission step, show a dialog.

Two kinds of item never reach `OnItemUse`: bandolier-eligible items (weapons) are auto-equipped instead, and crafting items (Blueprint items and Racial Paradigm Guides, the types listed in `resources.crafting_item_effects`) are used by the crafting subsystem, which consumes them itself ([crafting-system.md](../gameplay/crafting-system.md#blueprint-items-and-racial-paradigm-guides)). Do not author an `item_use` chain for a crafting item: it would never fire, and a live-DB guard (`crafting::item_use::tests::seed`) fails if one exists.

If you only need the recipe, jump to [The chain shape](#the-chain-shape).

## Why the base does not auto-consume

The 2009 Python server fired `item.use::<typeId>` as a pure event and let per-mission handlers decide whether to call `removeItemByDesign` ([`deprecated/python/cell/Inventory.py`](../../deprecated/python/cell/Inventory.py) around the `useItem` path). Mission scripts like Find Ambernol removed the vial; radio flows did not.

A later fanmmorpg fork changed that to auto-consume one unit on successful ability launch. **Cimmeria deliberately keeps the pre-fork pattern** — see the file-level comment in [`crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs).

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

`Action::RemoveItem` consumes the stack the player clicked (`CellToBaseMsg::RemoveInventoryItem`, the instance id `fire_item_use` put in the chain context). A chain fired some other way has no instance, and falls back to `RemoveInventoryItemByType`, which resolves the player's first matching stack. Both apply the full wire-update sequence.

### Reusable (omit `remove_item`)

Same trigger shape, but **no** `remove_item` action. The item stays in inventory for repeated uses (radio check-ins, worn equipment that starts a minigame, mission disguises worn for a whole sequence).

Gate the chain with `step_status` (or similar) so a second double-click re-resolves nothing harmful — the step flip is the idempotence stop.

## Worked example: consumable — Ambernol vial (mission 639)

**Goal.** Player uses item 19 (Ambernol vial) on the active "Use the vial" step → launch cure ability, consume the vial, complete mission 639, accept 640, enable the ring switch.

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 1034 | `item_use('19')` | `step_status(639, 2343) = active` | `launch_ability(1374)` → **`remove_item(19, qty 1)`** → `complete_mission(639)` → `accept_mission(640)` → `set_interaction_type` on HackTheRings_Switch |

Seed reference: `db/resources/Content/Seed/castle_cellblock_chains.sql` (chain 1034). Mirrors `FindAmbernol.py:115` (`removeItemByDesign(19, 1, False)`).

## Native consumables (`items_event_sets` event 5)

**Goal.** Heal and buff items work from their seed data alone: the Health Slappack heals 500, a Focus Heal restores focus, a stimpack raises an attribute for an hour. No chain is written.

### Which items

`cell::content::consumable_use::classify` decides from the startup caches. An item is a native consumable when all of these hold:

1. It has an `items_event_sets` row with `event_id = 5` (`EVENT_ITEM_USE_ABILITY`). Rows 6 and 7 are the weapon auto-attack bindings and never make an item usable.
2. The bound ability is not **597 "Heal Focus"**. The reconstructed seed binds 597 to 158 unrelated mission items (Opheltes's Injection, a Banged-up Radio, a DHD Bypass Card, Reins of the Sun Chariot, ...) as filler. Its effect 659 really does heal 35% Focus, so firing it would hand out free heals from quest props.
3. Every effect of the ability runs `HealHealth`, `HealFocus` or `StatBuff`. The other event-5 bindings (scanners, detonators, antidotes, disguise pieces, minigame starters) have script-less effects and keep their old behaviour: a chain decides, or nothing happens.
4. No `item_use` chain exists for the item (`ChainEngine::has_item_use_chain`). A chain owns its item outright, and the native path stands aside, so one use can never both run a chain and apply the item's ability.

The live-DB guard `consumable_use_live_db_tests::live_db_exactly_the_intended_items_are_native_consumables` pins the resulting set, so a script added to some other event-5 ability's effect cannot silently turn a quest item into a consumable.

### What happens on a use

1. The cell refuses a use that would do nothing, and says why: the user is dead ("You cannot use that while dead."), or every pool the item heals is already full ("You are already at full health." / "You are already at full focus."). The refusal is `onErrorCode(0, ability, code)` for parity (code 32 `StatValueGreaterThanOrEqual` or 14 `NotLiving`; the shipped client has no Lua consumer for it) plus a `CHAN_FEEDBACK` chat line, which is what the player sees. Nothing is consumed. A stat buff is never refused for headroom: using it again refreshes it.
2. Otherwise the cell asks the base to consume one unit of the clicked stack (`CellToBaseMsg::ConsumeItemForUse`). The base takes it in the same locked transaction as `removeItem`, held to the item's design id, and only after the commit answers `BaseToCellMsg::ItemUseConsumed`.
3. The cell then applies the ability to the user through the same server-side entry point as a chain's `launch_ability` (`effect_apply`: no cooldown, no warmup, no combat gates).

The effect is paid for before it lands. A double-click on the last unit sends two consumes; the second finds no row and gets no answer, so the item heals once. A chain's `change_stat` followed by `remove_item` applies first and pays afterwards, which is why the same double-click used to heal twice for one slappack.

### What is wired

| Items | Ability | Effect script | Magnitude (the effect's own `effect_desc`) |
|---|---|---|---|
| 2893, 4735 (Health Slappack TC1) | 648 | `HealHealth` 712 | `HealAmount` 500 |
| 6132, 6239, 6737-6744 (Health Consumable) | 2246, 2285-2293 | `HealHealth` 3125, 3249-3257 | `HealAmount` 162 to 353 |
| 6106, 6237, 6243, 6244, 6253, 6255, 6257, 6734-6736 (Focus Heal Consumable) | 2206, 2276-2284 | `HealFocus` 3062, 3239-3241, 3243-3248 | `HealAmount` 384 to 1420 |
| 6677-6682 (Mark III Stimpack) | 2734-2739 | `StatBuff` 3949-3954 | one attribute +5 for 3600 s |
| 6697, 6719, 6722, 6725, 6728, 6731 (Mark V) | 2740-2745 | `StatBuff`, two effects each | +7 and +3 |
| 6717, 6720, 6723, 6726, 6729, 6732 (Mark VII) | 2746-2751 | `StatBuff`, two effects each | +7 and +7 |
| 6718, 6721, 6724, 6727, 6730, 6733 (Mark X) | 2752-2757 | `StatBuff`, two effects each | +10 and +10 |

The 2009 rows shipped no `script_name` and no NVPs for any of these, so each `script_name` and each `effect_nvps` row (ids 400-462) is a seed edit, and each magnitude is the number in the effect's own description ("Heals 500 health." gives `HealAmount` 500, "+7 Coordination" gives `Coordination` 7). `live_db_every_consumable_magnitude_is_its_effect_description` checks every one against its description. The stimpack NVP names are the stimpack's words: `Coordination`, `Engagement`, `Fortitude`, `Intellect` (the stat the server calls `INTELLIGENCE`), `Morale`, `Perception`.

A second buff on the same attribute replaces the first, whatever its tier; buffs on different attributes never interact. The rule, and why the buff lives in its own ledger instead of the pulsing engine, is decision 28 of [abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md#28-native-consumables-the-base-consumes-before-the-cell-applies-and-timed-stat-buffs-live-in-their-own-ledger).

### What is deliberately not wired

| Items | Why |
|---|---|
| Stealth Boost (6206, 6762-6770; effects 3221, 4067-4075) and Energy Boost (6209, 6753-6761; 3227, 4076-4084) | Nothing on the server reads the stats they would move. `STEALTH_RATING` (46) and `ENERGY_POOL` (82) are read only by the GM `.stats` display, and `ENERGY_POOL` is `[0, 0, 0]` on every entity. A heal to an inert stat is a use that does nothing, so their `script_name` stays NULL until stealth or energy has a consumer. |
| Disguise Boost (8403, 6196, 6745-6752; effects 3187, 4056-4064) | The same: `DISGUISE_RATING` (78) has no reader. |
| Antidotes (6577, 6597-6599, 6656, 6657, 6659-6662, 6664-6666) | Their effects ("50% chance to remove Health", ...) remove conditions this server does not model. |
| 6245-6246, 6248-6250, 6252, 6254, 6256 | Not consumables: melee boots (`container_sets` `{1,12,17}`) with no event-5 row. 6247 and 6251 do not exist. |
| Everything bound to 597 | The filler binding, rule 2 above. |

### Refusing what is not wired

A use of a bag consumable (its preferred container is the main bag: `container_sets` `{1,17}`) whose event-5 ability is not native, and that no chain owns, is refused so the press is never silent: the chat line "This item has no effect yet." (no `onErrorCode`: the client enum has no fitting code), a WARN `consumable_refused` with `reason = consumable_not_implemented`, and nothing consumed. Today that is the Stealth, Energy and Disguise boosts and the antidotes; `live_db_the_unimplemented_bag_consumables_are_the_boosts_and_antidotes` pins it. A mission item (`{2}`) with such a binding stays silent for its chains, and the 597 filler never reaches this check, even on the 14 bag items bound to it (2042, 2592, ...).

### Chain or native?

The Ambernol vial (item 19) is the contrast. It too has an event-5 row, to 1374 "Cure Stasis Sickness", but using it has to check mission 639's step, complete the mission and accept the next one, so chain 1034 owns it. Its effect has no script, and even if it had one the chain would still own the item (rule 4). Use a chain when the use carries mission or world logic; wire the effect script and NVPs when the item is only a heal or a buff.

## Worked example: reusable — Radio (mission 1561)

**Goal.** Player uses item 5168 (radio) at two different mission steps — advance to bomb defusal, then complete the mission after defusing. The radio is **not** consumed either time.

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 3021 | `item_use('5168')` | `step_status(1561, 4621) = active` | dialog → advance step → set NaqBomb interaction |
| 3025 | `item_use('5168')` | `step_status(1561, 4623) = active` | complete mission → dialog |

Neither chain includes `remove_item`. Seed reference: `db/resources/Content/Seed/sgc_w1_chains.sql` (chains 3021, 3025).

## Baseline audit (HEAD)

Every `item_use`-triggered chain in seed data. The content-engine regression lint in `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs` enforces this table — new chains must update the allowlists in that test. The Health Slappack's chain 4001 (`consumables_chains.sql`) is retired: the slappack is a native consumable now. Item 2893 stays in `KNOWN_CONSUMABLES`, so a chain restored for it must still consume it (and would take it off the native path).

| Chain ID | Item ID | Description | Chain file | Has `remove_item` | Intent |
|---|---|---|---|---|---|
| 1034 | 19 | Ambernol vial (mission 639) | `castle_cellblock_chains.sql` | yes | Consumable — correctly removed |
| 1024 | 3438 | Prison Boots (mission 689 Livewire) | `castle_cellblock_chains.sql` | no (intentional) | Reusable worn equipment |
| 3021 | 5168 | Radio (bomb defusal step) | `sgc_w1_chains.sql` | no (intentional) | Reusable tool |
| 3025 | 5168 | Radio (mission complete) | `sgc_w1_chains.sql` | no (intentional) | Reusable tool |
| 6103 | 2819 | Jaffa Disguise (mission 742) | `harset_goauld_chains.sql` | no (intentional) | Reusable disguise |

## When adding a new `item_use` chain

1. If the item only heals or buffs, do not write a chain: give its event-5 ability's effects a `HealHealth`, `HealFocus` or `StatBuff` script and the magnitude NVP (see [Native consumables](#native-consumables-items_event_sets-event-5)), and add it to the expected set in `consumable_use_live_db_tests.rs`.
2. Decide **consumable vs reusable** before writing SQL.
3. If consumable, end the action list with `remove_item` for the same design id (after any heal/ability/advance).
4. If reusable, omit `remove_item` and gate with `step_status` (or equivalent) so repeat uses are harmless.
5. Add the item id to `KNOWN_CONSUMABLES` or `KNOWN_REUSABLES` in `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs`. The test fails on unknown ids with *"add to one of the two lists"* — that forces an explicit author decision. The lint matches on the **item id**, not just on the presence of a `remove_item` row: a consumable's chain must remove the item that was used (a row copied from another chain and never retargeted is reported), and a reusable's chain may remove a *different* item, such as a turn-in token.

## Cross-links

- [docs/content/content-engine.md](content-engine.md) — runtime reference for triggers and actions.
- [docs/content/equip-from-inventory-pattern.md](equip-from-inventory-pattern.md) — sibling pattern for weapon grants (uses `item_equipped`, not `item_use`).
- [`.github/instructions/content-chains.instructions.md`](../../.github/instructions/content-chains.instructions.md) — PR review checklist including inventory consumption.
- [`crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs) — base handler that fires `OnItemUse` without consuming.
- [`crates/cell-content/src/cell/content/consumable_use.rs`](../../crates/cell-content/src/cell/content/consumable_use.rs) and [`crates/base-methods/src/base/world_entry/methods/inventory/core/consume_for_use.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/core/consume_for_use.rs) — the native consumable path.
- [docs/gameplay/consumables.md](../gameplay/consumables.md) — what the player sees.
- [TESTING.md](../../TESTING.md) — the seed SQL linter tests are unit tests over the Content/Seed tree (no DB required).
