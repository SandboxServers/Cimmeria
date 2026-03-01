# Crafting System

> **Last updated**: 2026-03-01
> **Status**: ~30% implemented

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

## Related Docs

- [inventory-system.md](inventory-system.md) - Items consumed and produced by crafting
- [stat-system.md](stat-system.md) - Intelligence stat may affect crafting
