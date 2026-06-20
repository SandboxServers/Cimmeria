---
title: "Crafting System"
type: reference
audience: engineers
last_updated: 2026-05-27
---

# Crafting System

> **Last updated**: 2026-06-20
> **Status**: ~28% — state/persistence/skills/GM-grants done; the six activity handlers are stubs (tracked in #567). Findings: [`reverse-engineering/findings/crafting-restoration.md`](../reverse-engineering/findings/crafting-restoration.md).

## Overview

The crafting system enables players to create items through blueprints, research items for expertise, reverse engineer items into components, and alloy materials into higher tiers. Crafting is gated by disciplines (learned skill trees), racial paradigms (faction-specific tech trees), and Applied Science points (discipline training currency).

The `Crafter` class in `python/cell/Crafter.py` implements all crafting logic with timer-based induction for each operation. Crafting definitions come from `db/resources.sql`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Discipline learning | DONE | `learnDiscipline()`, prerequisite checks |
| Discipline forgetting | DONE | `forgetDiscipline()` |
| Applied Science points | DONE | Spend to learn disciplines |
| Racial paradigm tracking | DONE | `racialParadigms` dict with level updates |
| Blueprint knowledge | DONE | `blueprints` list, `giveBlueprints()` |
| Crafting (blueprint) | DONE | Component validation, consumption, timer, product creation |
| Research | DONE | Random expertise gain with kicker bonus |
| Reverse engineering | DONE | Bias-based component recovery |
| Alloying | DONE | Tier-based material combination with elementary components |
| Expertise gain | DONE | Per-craft/alloy expertise +1, capped at 100 |
| Timer-based induction | DONE | 3.0s duration for all operations |
| Crafting respec | NOT IMPL | `RespecCraft` event defined in client |
| Client crafting UI sync | NOT IMPL | `onUpdateDiscipline`, `onUpdateCraftingOptions` stubs |
| Busy state lock | PARTIAL | Checks `entity.busy` but `beginBusy`/`endBusy` calls commented out |

## Crafting Operations

### Craft

Combines component items using a blueprint to produce a new item.

```
Crafter.craft(blueprintId, itemIds, quantity)
  |-> Validate: not busy, blueprint known, items valid, in main/crafting bag
  |-> Find matching component set from blueprint
  |-> Validate: sufficient quantities
  |-> Consume component items
  |-> Start 3.0s timer
  |-> craftingCompleted():
       |-> Create product item (blueprint.product x blueprint.quantity x craftingQuantity)
       |-> Gain 1 expertise in blueprint's discipline
```

### Research

Destroys an item for a chance to gain expertise in a related discipline.

```
Crafter.research(itemId, kickerIds)
  |-> Validate: not busy, item researchable, kickers valid
  |-> Consume item and kickers
  |-> Calculate chance: 100 - currentExpertise + 5 * kickerCount
  |-> Select random applicable discipline
  |-> Start 3.0s timer
  |-> researchCompleted():
       |-> If successful: gain 5 expertise points
```

### Reverse Engineer

Destroys an item to recover some of its component materials.

```
Crafter.reverseEngineer(itemId)
  |-> Validate: not busy, item reverse-engineerable
  |-> Find blueprints that produce this item
  |-> Select random blueprint and component set
  |-> Calculate bias: techCompetency / playerExpertise
  |-> For each component: quantity = floor(random * bias * originalQuantity)
  |-> Consume item
  |-> Start 3.0s timer
  |-> reverseEngineeringCompleted():
       |-> Add recovered components to inventory
```

### Alloy

Combines a current-tier material with lower-tier elementary components.

```
Crafter.alloy(blueprintId, currentTierItemId, lowerTierItems)
  |-> Validate: not busy, blueprint known, is alloy blueprint
  |-> Validate: component matches blueprint requirement
  |-> Validate: elementary components correct tier (current - 1)
  |-> Validate: correct count based on quality (ALLOYING_ELEMENTARY_COUNTS)
  |-> Consume all components
  |-> Start 3.0s timer
  |-> alloyingCompleted():
       |-> Create alloy product
       |-> Gain 1 expertise in blueprint's discipline
```

## Discipline System

| Concept | Description |
|---------|-------------|
| Discipline | A learned crafting skill (expertise 1-100) |
| Expertise | Proficiency level in a discipline (affects research/reverse engineering) |
| Applied Science Points | Currency spent to learn new disciplines |
| Racial Paradigm | Faction-specific tech tier gating discipline access |
| Prerequisites | Disciplines may require other disciplines at expertise >= 50 |

### Learning Requirements

1. Have at least 1 Applied Science point
2. Racial paradigm level meets discipline requirement
3. All prerequisite disciplines known at expertise >= 50

## Data References

- **Recipes/Blueprints**: 499 in `db/resources.sql`
- **Disciplines**: Defined in resources
- **Racial paradigms**: Faction-based, initialized at level 1
- **Constants**: `ALLOYING_ELEMENTARY_COUNTS` (quality-based count table)
- **Item flags**: `researchable`, `reverseEngineerable`, `kicker`, `quality`, `tier`, `techCompetency`

## RE Priorities

1. **Client crafting UI** - Decompile `onUpdateDiscipline`, `onUpdateCraftingOptions`, `onUpdateKnownCrafts` wire format
2. **Crafting respec** - `RespecCraft` / `onCraftingRespecPrompt` / `onDisciplineRespec` protocol
3. **Tech competency** - How `techCompetency` affects crafting beyond research chance
4. **Quality system** - Item quality tiers and their effect on alloying
5. **Crafting busy state** - Why `beginBusy`/`endBusy` are commented out

## Concrete recipe examples

The catalog is a multi-stage production chain: raw materials → intermediate parts
("subcombines") → alloys → finished gear. All examples below are pulled directly from the
seed data (`db/resources/Entities/Seed/blueprints.sql` + `blueprints_components.sql`,
resolved against `Items/Seed/items.sql`).

**One product, several recipes.** A blueprint can have multiple *component sets*, so you
build with whatever you have. "IC-K-layer" (skill: *Electronic Engineering*) can be made
four ways:

- 13× Integrated Circuit, **or**
- 5× Integrated Pseudocore, **or**
- 6× Integrated Circuit + 1× Signal Damp, **or**
- 1× Particle Dynamo + 1× Wave Guide

**Materials refining.** "Steel Plating" (*Materials Engineering*) ← 13× Steel Core (or 5×
Titanium Core). "Optical Fiber" (*Power Systems Engineering*) ← Wave Guides + Particle parts.

**Alloying / tier-up.** 1× tier-1 "Cell" or "Drug" → **2× "Blend" (Bio-Medical Alloy)**,
which jumps from quality *Normal* to *Great* and tier 1 → tier 2. (40 of the 498 recipes
are alloy recipes.)

**Finished consumables.** The chain ends in usable gear — e.g. the **"Mark III Stimpack"**
line (Coordination / Engagement / Fortitude / Intellect — stat-boost consumables). The
Intellect stim (skill: *Robotics*) = 1× IC-K-layer + 2× Signal Damp + 1× Integrated Pseudocore.

Skills (disciplines) form faction-gated tech trees with prerequisites — e.g. *Biomedical
Engineering → Retroviral Engineering*, *Electronic Engineering → Robotics → Drone Robotics*,
*Power Systems Engineering → Naquadah Energy Systems → Energy Shielding*, plus faction-locked
branches like *Goa'uld Synesthesia*.

## Where materials come from

- **Loot** — mobs and containers drop crafting components (see [loot-system.md](loot-system.md)).
- **Salvage** — reverse-engineer gear you don't want back into parts.
- **Vendors** — buy some base materials.
- **Research is a sink, not a source** — it consumes items to train skills; it doesn't yield materials.

(No evidence of gathering/mining nodes — SGW used loot + salvage + vendors.)

## Balancing

The balance is the authentic 2009 game's, recovered from the client data: every recipe's
exact ingredient counts and outputs, item quality/tier, skill-training costs, and the
research success formula (`chance = 100 − yourExpertise + 5×kickers`, so mastering a skill
gets progressively harder). We restore these numbers rather than design them — but they're
fully editable in the seed if we ever want to tune the economy.

## Querying the data today

The full crafting catalog is queryable now, even before the activity handlers land:

- Recipes: `db/resources/Entities/Seed/blueprints.sql` + `blueprints_components.sql`
- Skills: `db/resources/Archetypes/Seed/disciplines.sql`
- Items/materials: `db/resources/Items/Seed/items.sql`

Totals: **498 blueprints** (40 alloy), **78 disciplines**, ~**5,958 items**.

## Related Docs

- [inventory-system.md](inventory-system.md) - Items consumed and produced by crafting
- [stat-system.md](stat-system.md) - Intelligence stat may affect crafting
