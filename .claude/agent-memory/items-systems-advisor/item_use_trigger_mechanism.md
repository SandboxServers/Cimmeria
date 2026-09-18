---
name: item-use-trigger-mechanism
description: How UseInventoryItem actually becomes a content-engine chain today — items_event_sets is legacy/unused for this, the live path is content_triggers(item_use, <item_id>); corrects the double-consume framing in this agent's own system prompt
metadata:
  type: project
---

Confirmed 2026-09-17 during a Harset zone evidence pass (READ-ONLY, no code changed).

**`items_event_sets` (item_id, ability_id, event_id) is legacy Atrea-era reference data, not the live wiring.** Nothing in `crates/services` or `crates/content-engine` reads this table. It's useful only as a *hint of original developer intent* — see below.

**The live mechanism:**
1. Seed a `content_triggers` row: `(chain_id, event_type='item_use', event_key='<item_id>', scope='player', once, sort_order)`. Real examples: `db/resources/Content/Seed/castle_cellblock_chains.sql:501` (chain 1034, key '19'), `db/resources/Content/Seed/consumables_chains.sql:39` (chain 4001, key '2893', the Health Slappack).
2. Parsed by `crates/content-engine/src/loader/trigger.rs:36-38` — `"item_use" => Trigger::OnItemUse { item_id: key?.parse().ok()? }`. Keys strictly on `item_id`, nothing else.
3. Dispatched by `crates/services/src/cell/content/event_dispatch/inventory.rs::fire_item_use` (starts line 28). Matches purely on `item_id` passed from the UseInventoryItem handler; sets `item_id`/`instance_id`/mission/stat context; resolves and executes whatever `content_actions` the matched chain(s) declare.

**Consumption is NOT automatic.** Per the header comment in `consumables_chains.sql`: "`useItem` events fire without consuming, so per-item chains decide whether to remove." Every chain that should burn a stack must include its own explicit `remove_item` action (see chain 4001: `change_stat` then `remove_item` in that order, heal-before-consume so a channel-saturation failure on remove doesn't cost the player the heal).

**Correction to this agent's own "double-consume trap" framing:** the hazard in *this* codebase isn't "UseInventoryItem auto-consumes AND a `remove_item` action consumes again" — there is no separate auto-consume path to collide with. The real double-consume risk shapes here are: (a) a chain with two `remove_item` actions for the same item_id, or (b) two distinct chains both trigger on `item_use::<same item_id>` with `once=false`, each independently removing a stack. Check for both shapes when reviewing new `item_use` chains, not just "is there a remove_item next to UseInventoryItem."

**Using `items_event_sets.ability_id` as an intent signal (Harset-specific finding, may generalize):** ability_id `597` = "Heal Focus" (`db/resources/Abilities/Seed/abilities.sql:298`, effect `db/resources/Effects/Seed/effects.sql:7745` "Focus Heal") is a generic/default ability that shows up bound to several *unrelated* Harset items (Jaffa Disguise 2819, Scarab Listening Device Map 2864, 3 of 4 Straegis Scanner duplicates, 1 of 2 Tollan Control Technology duplicates). This is almost certainly a leftover placeholder binding from original content authoring, not a real "use" effect — when an item in a duplicate-id group has a *distinct* ability/effect with a name matching its flavor (e.g. item 4396 → ability 2092 → effect 2816 "Use Straegis Scanner" / "Search for Straegis using the Scanner"), that's strong evidence it's the canonical/intended id and the ability-597-bound siblings are orphaned duplicates. Full duplicate-resolution table lives in the Harset campaign report delivered 2026-09-17 (not persisted as a file — ask to regenerate from this memory + a fresh items.sql/items_event_sets.sql grep if needed).
