---
name: npc-ability-sets
description: Traps when authoring NPC ability_sets seed data — the one-ability-per-set PK, the event_set_id animation gate, max_range poisoning of the NPC walk-toward gate, and melee abilities firing from 30m
metadata:
  type: project
---

Four structural traps when assigning `ability_set_id` to NPC templates (defect
class H-B8: 150/153 templates fall back to `NPC_DEFAULT_ABILITY = 592`).

**Why:** the seed tables look like ordinary junction/def tables but three of
their columns are load-bearing in ways the schema doesn't advertise, and one
carries a PK that silently forbids the obvious design.

**How to apply:** check all four before proposing any new `ability_sets` row.

## 1. A set can hold exactly ONE ability

`db/resources/_primary_keys.sql:44` —
`ability_set_abilities_pkey PRIMARY KEY (ability_set_id)`, **not**
`(ability_set_id, ability_id)`. A second row for the same set is a duplicate-key
violation. `db/database.sql` applies `_primary_keys.sql` (line 264) *before*
the seed (line 273), so the failure aborts the whole DB load at INSERT.

The Rust side is already multi-ready — the loader does
`array_agg(asa.ability_id ORDER BY asa.ability_id)`
(`crates/cell-catalog/src/cell/spawner/npcs.rs:152-154`) and `choose_npc_ability`
handles N ids. **The DB PK is the only blocker.** Widening it is a schema
change (edit `_primary_keys.sql`), so it does not belong in a seed-only packet.

## 2. `event_set_id NULL` on the ability = no attack animation, ever

`crates/services/src/cell/abilities/use_ability/handle.rs:524` gates the entire
`onSequence` broadcast (Ability_Begin 1000 / Ability_End 1001) on
`ability_def.event_set_id` being `Some`. An ability with a NULL ability-level
`event_set_id` deals damage with **no fire animation**. The effect row's own
`event_set_id` does not substitute.

Always confirm non-NULL `event_set_id` before putting an ability in an NPC set.
Known-good: 579 Pistol AA (3), 592 Pistol Shot (3), 559 Auto Weapon AA (15),
584 Staff AA (3), 710 Staff Melee AA (300), 712 Ribbon Device AA (300),
711 Ribbon Device Melee AA (300).
Known-NULL (avoid): 594 Strike, 479 Staff Blast, 540 Staff Strike,
1482 Ground Blast, 1768 Double Blast, 1922 Rapid Blasts.

## 3. `max_range > 0` poisons the NPC walk-toward gate

`ability_ranges` (`crates/services/src/cell/service/npc_ai/ability_select.rs:59-71`)
returns the def's `max_range` verbatim when non-zero, and `fight.rs` uses it as
the `in_range` gate. `max_range = 0` is the sentinel meaning "use
`NPC_ATTACK_RANGE = 30.0`". An ability like 1482 Ground Blast (`max_range 3000`)
makes `in_range` true at any distance, so the NPC never closes. Worse, since the
selector picks per-tick, the poisoning is *intermittent* — only on ticks where
the lower-id workhorse is cooling.

## 4. `is_ranged` does NOT gate range — melee abilities fire from 30m

`is_ranged` is read only at
`crates/services/src/cell/abilities/damage_apply/mod.rs:92-93` to pick the QR
accuracy/defense branch. It has no effect on range. A melee ability with
`max_range 0` in an NPC set therefore resolves at the full 30m default and the
NPC swings at empty air. Never put `is_ranged = false` abilities in an NPC set.

## 5. NPC AI never reaches the ground-target path

`fight.rs` calls `handle_use_ability`. `handle_use_ability_on_ground` is invoked
only from the player wire handler
(`crates/services/src/cell/cell_methods/player/combat/mod.rs:77`). So
`TCM_AERadius` fan-out never happens for an NPC — a ground ability degrades to a
single-target hit. See also `cell/abilities/cone_aoe/mod.rs:50-53`.

## 6. Weapon mesh lives in `components`, not `weapon_item_id`

`weapon_item_id` has **zero consumers under `crates/`** — dead data server-side.
The client renders whatever `WP-*` entry is in the template's `components`
array. Templates 15 / 24 pair a `WP-Human.WP_Pistol_1A` / `WP_SMG_1A` component
with the matching ability set; the Praxis Jaffa (159/160/163) and every
Goa'uld template carry **no `WP-*` component at all**. Assigning a weapon
ability set without adding the matching component gives a mimed animation.
Inverse case: templates 34/35 SGC Jaffa carry `WP-Jaffa.WP_Staff_Plasma_4A`
but have `ability_set_id = NULL` — staff mesh, pistol fallback.

Related: [[pvp-duel-readiness]]
