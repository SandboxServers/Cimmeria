---
title: "Equip-From-Inventory Pattern"
type: reference
audience: engineers
last_updated: 2026-05-27
---

# Equip-From-Inventory Pattern

> **Type**: explanation
> **Audience**: content authors / mission designers
> **Last updated**: 2026-05-09
> **Companion docs**: [docs/architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md), [docs/content/mission-chains.md](mission-chains.md), [docs/content/content-engine.md](content-engine.md), [docs/content/extending-the-engine.md](extending-the-engine.md)

When a quest grants the player a weapon (or any equipment), prefer routing it through a **manual equip step** instead of force-equipping into the bandolier. This doc explains why, the chain shape that implements the pattern, and two worked examples (mission 622 and mission 641).

If you only need the recipe, jump to [The chain shape](#the-chain-shape).

## Why not just force-equip?

The first instinct is to grant the weapon directly to bandolier (`container_id = 3`) so the player has it ready to fire. That codepath bypasses `sync_bandolier_after_inventory_change` — the bookkeeping that keeps the player's bandolier ammo state, fire-animation flags, and active-weapon-slot pointer consistent. Two real bugs traced to this:

- **Bandolier ammo desync.** The freshly-granted weapon ended up in the bandolier with stale or zero ammo metadata, because the grant path didn't run the ammo-replenishment branch that the manual-equip path runs. Players couldn't fire.
- **Fire-animation broken.** The active-weapon-slot pointer stayed on the previous slot (often empty), so the client's animation rig treated the player as unarmed even though the weapon icon showed in the bandolier.

Both bugs disappear if the weapon is granted to the **backpack** (`container_id = 1`) and the player drops it into the bandolier themselves — that path is the one the inventory move handler covers in detail (`crates/services/src/base/world_entry/methods/inventory/move_/mod.rs`), and it's exhaustively guarded by the inventory-move regression suite.

So: don't force-equip. Direct the player to do it.

## The chain shape

Two chains, plus one `MissionOverride` entry that adds the new "Equip the X" step to the client's mission catalogue (because the canonical PAK doesn't know about it). See [docs/architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md) for how the PAK override mechanism works.

### 1. Pickup chain (existing trigger)

When the player loots / interacts to acquire the weapon:

| Field | Value |
|---|---|
| Trigger | The natural pickup event — `dialog_open` for body-loot, `interact_tag` for a locker, `entity_dead_tag` for a kill drop, etc. |
| Condition | `step_status` of the **previous** step must be `active` |
| Actions | 1. `add_item` weapon → backpack (`container_id = 1`), **not** bandolier; 2. any flavour items (letters, keys); 3. `advance_step` to the new "Equip the X" step |

### 2. `MissionOverride` row

Add an entry to `MISSION_OVERRIDES` in `crates/services/src/base/mission_overrides.rs` so the client's mission UI renders the new step's display text. Use `insert_after_step_id = <previous step id>` — see the [XML-index gotcha](../architecture/mission-pak-overrides.md#the-xml-index-gotcha) for why placement matters. The matching seed rows go in `db/resources/Missions/Seed/mission_steps.sql` and `mission_objectives.sql`.

### 3. Equip chain (new trigger)

When the player drops the weapon into the bandolier:

| Field | Value |
|---|---|
| Trigger | `item_equipped` keyed by the weapon's design / `type_id` (`Trigger::OnItemEquipped { item_id: Some(<id>) }`) |
| Condition | `step_status` of the new "Equip the X" step must be `active` (the step gates the chain so it only fires once, in the right context) |
| Actions | Whatever the original chain was going to do — `advance_step` to the next step, `play_sequence`, `complete_mission`, etc. |

The `item_equipped` trigger fires from `crates/services/src/cell/service/base_messages/mod.rs` whenever a bandolier-targeted move lands. See [docs/content/extending-the-engine.md](extending-the-engine.md) for the trigger plumbing.

## Worked example: mission 622 (Frost pistol)

**Goal.** The player loots Frost's body, gets a pistol, equips it, the stasis-room door opens, mission completes.

**Before the pattern.** Pre-fix chain 1003 granted the pistol directly to the bandolier and immediately played kismet sequence 10000 (door open). Pistol arrived in the bandolier with zero ammo and the wrong fire animation.

**After.** Two chains plus a step override:

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 1003 | `dialog_open('3995')` (Frost body) | `step_status(622, 2113) = 'active'` | Grant pistol (item 55) → backpack; grant Frost's letter (item 3730) → mission inventory; `advance_step(622, 80622)` |
| 1004 | `item_equipped('55')` | `step_status(622, 80622) = 'active'` | `play_sequence(10000)` (open stasis door); `complete_mission(622)` |

`MissionOverride { mission_id: 622, insert_after_step_id: 2113, … StepID="80622" … }` adds the "Equip the pistol from your inventory." step at XML index 1 (between the existing step 2113 at index 0 and the closing tag).

This is the **terminal completion** variant — the equip step's chain ends the mission. Step references: `db/resources/Content/Seed/castle_cellblock_chains.sql:50-105`, `crates/services/src/base/mission_overrides.rs:74-90`. Regression tests at `crates/services/src/cell/content/chain_replay_tests/mission_622.rs` (chains 1003, 1004).

## Worked example: mission 641 (P90)

**Goal.** The player opens the locker, gets a P90, equips it, talks to Col. Marsh, mission progresses through its existing later steps.

**Before the pattern.** Same shape as mission 622 — granted P90 directly into the bandolier, immediately advanced to step 3563. Same ammo/animation breakage.

**After.** Two chains plus a step override:

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 1055 | `interact_tag('Preparation_SMG1A')` | `step_status(641, 2121) = 'active'` | Grant P90 (item 21) → backpack; clear locker highlight; `advance_step(641, 80641)` |
| 1066 | `item_equipped('21')` | `step_status(641, 80641) = 'active'` | `advance_step(641, 3563)` (talk to Marsh); re-set Marsh's mission-available marker |

`MissionOverride { mission_id: 641, insert_after_step_id: 2121, … StepID="80641" … }` adds "Equip the P90 from your inventory." between step 2121 (index 0) and the existing step 3563 (now at index 2). The placement is load-bearing — see [the XML-index gotcha](../architecture/mission-pak-overrides.md#the-xml-index-gotcha) for what happens when the new step lands at the tail instead.

This is the **intermediary step** variant — the equip step is not the end of the mission, just a gate before talking to Col. Marsh. Step references: `db/resources/Content/Seed/castle_cellblock_chains.sql:560-615`, `crates/services/src/base/mission_overrides.rs:91-109`. Regression tests at `crates/services/src/cell/content/chain_replay_tests/mission_641.rs` (chains 1055, 1066).

## When to use the pattern

Use it whenever:

- The mission **gives the player a weapon** and the player needs to fire it shortly after.
- The mission **gives the player armor** and a stat-derived calculation downstream (HP scaling, damage resistance) reads from the equipped slot rather than the inventory.
- Any flow where downstream logic depends on the **bandolier sync state** rather than just inventory presence.

Don't use it for:

- Quest items that never get equipped (letters, keys, Ambernol vials). Grant those directly to mission inventory (`container_id = 2`) — there's no equip path to break.
- Pre-equipped starter gear at character creation. That path goes through the `BAG_FILL_ORDER` constant in [`crates/services/src/base/resources.rs`](../../crates/services/src/base/resources.rs) and is its own thing.

## Cross-links

- [docs/architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md) — how the new step gets into the client's mission catalogue.
- [docs/content/mission-chains.md](mission-chains.md) — chain inventory; mission 622 (chains 1003, 1004) and mission 641 (chains 1051, 1055, 1066) document the live shapes.
- [docs/content/extending-the-engine.md](extending-the-engine.md) — adding a new trigger / condition / action to the engine. The `item_equipped` trigger followed this guide.
- [docs/content/content-engine.md](content-engine.md) — the runtime that executes these chains.
- [TESTING.md](../../TESTING.md) — test picker; chain-replay is the right kind for the chains in this pattern.
