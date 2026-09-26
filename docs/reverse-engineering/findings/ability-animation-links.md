# Ability animation links — how an ability picks its event set

> **Diátaxis type**: reference
> **Audience**: engineers wiring ability presentation (players and NPCs)
> **Last updated**: 2026-09-26
> **Confidence**: HIGH for the client mechanism and the weapon-family rule; the per-ability links are reconstruction (see labels)

## Summary

An ability animates only if `resources.abilities.event_set_id` points at an event set with an Ability_End (event 1001) sequence. `use_ability/handle.rs` resolves that pair through `SpaceManager::sequence_map` and sends `onSequence`; a NULL `event_set_id` skips the broadcast, so the hit lands with no animation.

The seed recovered only 35 of 1,886 links. Those 35 follow one rule (weapon family plus melee/ranged). The seed now links 220 more abilities: 162 by that rule, 30 by what their weapon's items fire, and 28 by judgement. Two beams, Terror Stone and Cloak remain unlinked.

## Provenance: why the links were missing

`db/resources/` is **not** CME's 2009 database. Project Giza built it by extracting the client's cooked packages into tables, and the legacy Python server is also Giza-era. CME's server code and data were never released. A NULL `event_set_id` therefore means "Giza's extraction found no link", not "retail had no animation".

## The client has no ability-keyed lookup

**Label: CLIENT-PROVEN.**

- `onSequence` carries an opaque sequence id. The client plays it from its own cooked event-set and sequence data and never consults the ability (handler registration at `0x00d76f40`; see also [`animation-system.md`](animation-system.md) and [`item-sequence-lookup.md`](../../protocol/item-sequence-lookup.md)).
- Every `KIS-abilities_*.upk` package in `CookedPC`, including the Jaffa and Straegis ones, holds only mechanic-typed Kismet templates: `KIS-SA_Burst_Source`, `KIS-SA_Melee_Source`, `KIS-SA_Deployable_Source`, `KIS-SA_Beam_Source` and siblings. No object is named for an individual ability.

So the ability-to-event-set link was server data, and the client's own assets are generic per mechanic. Reconstructing a link means choosing which generic template an ability uses.

## The weapon-family rule

**Label: CLIENT-INFERRED.** Derived from all 35 recovered links; the only exception is Aimed Burst's charged set.

`abilities.item_monikers` names the weapon family (the `ITEM_*` rows in `db/resources/Entities/Seed/monikers.sql`). Together with `is_ranged` it decides the event set:

| Delivery | Families | Event set |
|---|---|---|
| Melee (`is_ranged = false`) | Pistol, Rifle, Shotgun, Staff, Zat, Dart Pistol, Dart Rifle, Automatic Weapon, Light MG, Grenade Launcher, Flamethrower, Ribbon Device, Blade, Fists | **300** "Generic melee weapon source" |
| Ranged single-shot | Pistol, Rifle, Shotgun, Staff, Zat, Dart Pistol | **3** "Generic single-shot weapon use (channelable)" |
| Ranged automatic | Automatic Weapon, Light MG, Dart Rifle | **15** "Generic burst weapon use (channelable)" |

The one recovered exception is 1005 Aimed Burst on **1033** "Aimed shot continued ability source".

Sets 3 and 15 are channelable, so an ability with a warmup plays the weapon's Begin sequence during the wind-up and the fire sequence at the end. This matches the recovered 1662 (1 s warmup on set 3).

## What the seed now links

The seed links 220 abilities in three tiers. `crates/services/src/cell/spawner/tests/live_db_ability_animation_links.rs` pins the family rule, the per-ability links and the Ability_End requirement.

### Tier 1: the recovered rule (162 abilities)

**Label: RECONSTRUCTION**, the rule applied rather than per-ability evidence.

This tier covers every non-passive `ABILITY_TYPE_DD` or `ABILITY_TYPE_DOT` row with a NULL `event_set_id` whose monikers all fall in one family class above: 89 on set 3, 47 on set 15 and 26 on set 300. It includes:

- the NID SMG specials: 598 Quick Burst, 718 Selective Fire and 891 Suppression Shot (set 15);
- the Jaffa staff specials: 653 Blast, 775 Double Blast, 2001, 2024, 2042, 2043, 2058 and 1626 (set 3), plus 1984 Staff Swing and 2025 Whirlwind (set 300);
- the Ashrak dagger: 1613, 1620, 1621, 2857 and 2858 (set 300);
- the player weapon trees' specials (bursts, pumps, aimed shots, blasts).

### Tier 2: families placed by their items (30 abilities)

**Label: CLIENT-INFERRED.** `items_event_sets` comes from the cooked item XML (`<ItemEventSet AbilityID=… EventID=…>`), and its `EVENT_ItemRanged` (7) rows say which attack each weapon fires:

| Family | What its items fire | Abilities linked |
|---|---|---|
| SMG (103 items), Assault Rifle (100 items) | 559 Automatic Weapon Auto Attack, set 15 | 18 ranged abilities on **15**, including 720 Cover Fire and 1847 (Light MG plus SMG) |
| Ribbon Device (151 items) | 712 Ribbon Device Auto Attack, set 300 | 12 ranged abilities on **300**, including 1624 Fear and 2694 Annihilation Beam |

### Tier 3: judgement calls (28 abilities)

**Label: RECONSTRUCTION.** These are chosen by delivery and set name and still need a playtest.

| Abilities | Event set | Reasoning |
|---|---|---|
| Grenades 523, 854, 861, 868 | **296** "Grenade deployment source" | The only grenade *source* set (Begin/End/Interrupt/Fail on `KIS-SA_Deployable_Source`); the client has `xGrenade_Begin/End/Idle` in `HM_Animation` and `JM_Animation`. The explosion sets (297, 751, 813, 1149) are *target* sets for the grenade effects, not the ability. |
| Natural melee 660 Bash, 983 Bite, 1076 and 1077 Lok'nel, 1433 Mob Strike, 1186 Drone Strike, 2146 Takedown | **300** | Fists and Drone melee auto-attacks already use it; creature bodies need a playtest |
| 1174 Drone Shot | **15** | Matches the recovered 1216 Drone Ranged AA |
| Grenade Launcher ranged (7), Energy Pistol ranged (1) | **3** | Single fired shot; the blast belongs on the effect |
| Flamethrower ranged (5) | **15** | Sustained, channelable stream |
| 2847 Straegis: Dissonance (pulsing AoE) | **1497** "Straegis emit aura ability source" | Name match |
| 1240 Straegis Explode | **1507** "Straegis death ability source" | The Straegis explodes on death |
| 1156 Straegis: Disengage (knockdown) | **1499** "Straegis aura gravity ability source" | Closest name to a knockdown |

The Straegis ability sets (1497 to 1521) exist in the seed and carry an Ability_End. An earlier pass had missed them.

## Still unlinked

| Abilities | Candidate | Label |
|---|---|---|
| Beams 1176 Energy Pulse, 1829 Force: Fusion Beam | 802 (human `KIS-SA_Beam_Source`) over 1265/1267 (Asgard beam) | RECONSTRUCTION, low confidence |
| 1640 Terror Stone | 296 | RECONSTRUCTION, low confidence |
| 1138 Cloak | none on the ability; put a "Stealth end" target set (795, 893, 987, 1130) on effect 1290 instead, with `AR_H_Stealth00.upk` supplying the look | OPEN |

## Related

- [`animation-system.md`](animation-system.md): the client animation pipeline.
- [`ability-resolution-pipeline.md`](ability-resolution-pipeline.md): ability activation on the client.
- [`item-sequence-lookup.md`](../../protocol/item-sequence-lookup.md): `items_event_sets` is an ability-override table, not a sequence table.
