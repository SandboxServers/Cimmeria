---
title: "Crafting System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Crafting System

> **Last updated**: 2026-09-26
> **Status**: State model, persistence, GM grants, the login sync (CR-03) and learning disciplines with applied science points (CR-04) work. Craft, research, reverse engineering, alloying and respec are still stubs (tracked in #567). Findings: [`reverse-engineering/findings/crafting-restoration.md`](../reverse-engineering/findings/crafting-restoration.md).

## Overview

The crafting system enables players to create items through blueprints, research items for expertise, reverse engineer items into components, and alloy materials into higher tiers. Crafting is gated by disciplines (learned skill trees), racial paradigms (faction-specific tech trees), and Applied Science points (discipline training currency).

The Rust implementation lives in [`crates/base-session/src/base/crafting/`](../../crates/base-session/src/base/crafting/) (persistence + GM grants) and [`cell/cell_methods/player/crafting/`](../../crates/cell-methods/src/cell/cell_methods/player/crafting/) (cell methods 95–100: argument parsing and the forward to the base). The state model is `cimmeria_entity::crafting::CraftingState`.

The sections below that describe `Crafter` behaviour document the **original server's design**, which Phase 2 is expected to reproduce. They are not descriptions of current runtime behaviour.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Crafting state model | DONE | `CraftingState` — discipline ids, per-discipline expertise, blueprint ids, applied-science points, racial-paradigm levels |
| Persistence | DONE | Split across `sgw_player` (four scalar/array columns) and `sgw_player_discipline_expertise` (normalised per-discipline expertise rows, `CHECK (expertise BETWEEN 0 AND 100)`) |
| Expertise cap | DONE | Enforced twice — `EXPERTISE_CAP = 100` in the handler and a DB `CHECK` constraint |
| GM expertise grant | DONE | `handle_grant_expertise` mutates, persists, and pushes `onUpdateDiscipline` (client method 136, payload `[disciplineSeqId i32][expertise i32]`) |
| GM applied-science grant | DONE | `handle_grant_applied_science` adds in one `UPDATE … RETURNING` and pushes the new **total** as `onEntityProperty(GENERICPROPERTY_AppliedSciencePoints = 2, total)`, so the discipline trainer's count updates without a relog (CR-03) |
| Client state sync | DONE | `base/crafting/sync/`: owner-only pushes of 136, 138, 139 and the ASP property. Every ASP change pushes the total, never the change (audit C-57) |
| World-entry state load | DONE | After the `onClientReady` burst, `push_crafting_on_login` loads the state and sends one bundle: 136 per known discipline, 138 per paradigm, 139, the ASP total. A relog restores disciplines, expertise, paradigm levels, blueprints and ASP (CR-03) |
| Starting paradigm levels | DONE | D-CR03: Common (paradigm 1) at 5, Human, Goa'uld, Asgard and Ancient at 1. The load applies them to a character with no stored levels; the `sgw_player.racial_paradigm_levels` column default gives new characters the same array `{5,1,1,1,1}` |
| Request path (methods 95-100) | DONE | The cell parses every argument, including the `ARRAY<ItemID>`s, and forwards a `CellToBaseMsg::Crafting(CraftRequest)`; the base logs it at target `crafting` (`event = "request"`). Crafting campaign CR-01 |
| Crafting catalog | DONE | `cimmeria_cell_catalog::crafting::CraftingCatalog`: disciplines, blueprints with their alternative component sets, and item crafting attributes, loaded once per process |
| Spend applied-science points | DONE | `base/crafting/spend/` (CR-04), see [Learning a discipline](#learning-a-discipline) |
| Crafting (blueprint) | STUB | The base answers "Crafting is not available yet." |
| Research | STUB | The base answers "Research is not available yet." |
| Reverse engineering | STUB | The base answers "Reverse engineering is not available yet." |
| Alloying | STUB | The base answers "Alloying is not available yet." |
| Crafting respec | STUB | The base answers "Crafting respec is not available yet." |
| Timer-based induction | NOT IMPL | The original 3.0s per-operation induction has no Rust equivalent |
| Busy state lock | NOT IMPL | No `beginBusy`/`endBusy` equivalent |
| `onUpdateCraftingOptions` | NOT IMPL | Never sent; the station and tool gate owns it (CR-05). Until then every crafting tab stays disabled |

## Learning a discipline

The discipline trainer (Ctrl+J) sends `spendAppliedSciencePoints(disciplineId)` (cell method 95) for any click, whatever its tree colours say (audit C-36). The base decides it in one transaction that locks the player's `sgw_player` row `FOR UPDATE`, checking in this order:

| Check | Refusal text | `onErrorCode` |
|---|---|---|
| The discipline is in the catalog | "There is no discipline N." | — |
| It is not already known | "You already know <name>." | — |
| At least one unspent ASP | "You have no applied science points." | 214 `NotEnoughAppliedSciencePoints` |
| The discipline's racial paradigm is at its required level | "<name> requires <paradigm> paradigm level N; yours is M." | — |
| Every required discipline is known at expertise 50 or more | "<name> requires <prerequisite> at expertise 50." | — |

A refusal is a `CHAN_FEEDBACK` text line and writes nothing. A database failure is refused as "Learning disciplines is unavailable right now. Nothing was changed." On success the discipline is known at expertise 1 and one ASP is spent; no blueprint is granted (D-CR04, blueprints come from Blueprint items and research). The client then gets `onUpdateDiscipline(id, 1)` and the new ASP total. A repeated request finds the discipline known and changes nothing.

The four root disciplines (21 Biomedical, 40 Electronic, 59 Power Systems, 78 Materials Engineering) need Common level 5, which every character now starts at. The test rows 1 and 2 ("Basketweaving") need Common 1 and are treated like any other discipline.

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

- **Recipes/Blueprints**: 498 in `db/resources/Entities/Seed/blueprints.sql`
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
