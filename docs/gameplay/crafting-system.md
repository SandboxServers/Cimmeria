---
title: "Crafting System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Crafting System

> **Last updated**: 2026-09-26
> **Status**: State model, persistence, GM grants, the login sync (CR-03), learning disciplines with applied science points (CR-04) and earning those points by levelling (CR-12) work. Craft, research, reverse engineering, alloying and respec are still stubs (tracked in #567). Findings: [`reverse-engineering/findings/crafting-restoration.md`](../reverse-engineering/findings/crafting-restoration.md).

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
| World-entry state load | DONE | After the `onClientReady` burst, `push_crafting_on_login` loads the state and sends one bundle: 136 per known discipline, 138 per paradigm, 139, the ASP total, then `onUpdateCraftingOptions` (140). A relog restores disciplines, expertise, paradigm levels, blueprints and ASP (CR-03) |
| Starting paradigm levels | DONE | D-CR03: Common (paradigm 1) at 5, Human, Goa'uld, Asgard and Ancient at 1. The load applies them to a character with no stored levels; the `sgw_player.racial_paradigm_levels` column default gives new characters the same array `{5,1,1,1,1}` |
| Request path (methods 95-100) | DONE | The cell parses every argument, including the `ARRAY<ItemID>`s, and forwards a `CellToBaseMsg::Crafting(CraftRequest)`; the base logs it at target `crafting` (`event = "request"`). Crafting campaign CR-01 |
| Crafting catalog | DONE | `cimmeria_cell_catalog::crafting::CraftingCatalog`: disciplines, blueprints with their alternative component sets, and item crafting attributes, loaded once per process |
| Spend applied-science points | DONE | `base/crafting/spend/` (CR-04), see [Learning a discipline](#learning-a-discipline) |
| Earn applied-science points | DONE | 1 at level 1 and 1 per level gained, written with the level by the XP grant (CR-12), see [Earning applied science points](#earning-applied-science-points) |
| Crafting (blueprint) | STUB | The base answers "Crafting is not available yet." |
| Research | STUB | The base answers "Research is not available yet." |
| Reverse engineering | STUB | The base answers "Reverse engineering is not available yet." |
| Alloying | STUB | The base answers "Alloying is not available yet." |
| Crafting respec | STUB | The base answers "Crafting respec is not available yet." |
| Timer-based induction | NOT IMPL | The original 3.0s per-operation induction has no Rust equivalent |
| Busy state lock | NOT IMPL | No `beginBusy`/`endBusy` equivalent |
| Crafting stations | DONE | The cell tracks the nearest station per verb within `MAX_INTERACT_DISTANCE` (5 units, 3-D) and reports changes to the base once a second; the forward recomputes the mask per request. CR-05. No seeded template is a station yet (CR-11 adds the debug-hub four) |
| Field Crafting Tools | DONE | A tool in the crafting bag (container 15) covers crafting, research and reverse engineering for its science up to its `tech_comp` (D-CR21). CR-05 |
| Station gate | DONE | Crafting, research, reverse engineering and alloying are refused with "No crafting station or tool for <verb> nearby." unless a station, a covering tool or "craft anywhere" allows them. CR-05 |
| `onUpdateCraftingOptions` | DONE | Sent last in the login crafting bundle after `onClientReady` (every world entry), then on every change of stations, tools or "craft anywhere". CR-05 |
| `.allcraft` | DONE | GM: every paradigm at 7, every discipline at 100, every blueprint, persisted, plus "craft anywhere" until logout (D-CR17). CR-05 |

## Earning applied science points

Owner decision D-CR01: a character has 1 ASP at level 1 and earns 1 more for every level gained, so an unspent character at level `L` holds `L` points (50 at the cap). GM grants (`gmGiveAppliedSciencePoints`) come on top. The 2009-era server granted ASP only by GM command (audit C-03).

- **New characters.** `createCharacter` inserts `applied_science_points = 1` (`STARTING_APPLIED_SCIENCE_POINTS` in `cimmeria_game::player`); the column default stays 0. The seeded characters (player ids 62-70, all level 1) hold 1.
- **Level-ups.** `handle_grant_xp` (`base/world_entry/methods/progression/`) is the only code that raises a level: mob kills, content-chain `GrantXP` and the GM `.givexp` all reach it. The statement that writes the new XP, level and training points also adds one point per level gained, counted against the level the row held under the row lock (`FOR UPDATE`), not the session's cached level. A level and its points are therefore committed together, a multi-level grant earns one point per level, a grant at the cap earns nothing, and a write of a level the row already holds earns nothing.
- **Client.** After the XP bundle (`onExpUpdate`, `onLevelUpdate`, training points), the owning client gets the ASP property with the new total (`onEntityProperty(2, total)`), which the discipline trainer shows live. XP that crosses no level boundary sends no ASP push.
- **Telemetry.** Each earning level-up logs `event=asp_earned` under `crafting` with `account_id`, `player_id`, `entity_id`, `level_before` / `level_after` and `asp_before` / `asp_after`. A grant whose character row is gone logs `event=persist_failed` (WARN, `phase=grant_xp_update`, `reason=rows_affected_zero`, `rows_affected=0`, `expected=1`) and sends nothing. A failed ASP push is the shared `push_failed` (`what=asp`).
- **Existing characters.** Nothing backfills characters that levelled before this change; they hold whatever they had (0 unless a GM granted points).

## Learning a discipline

The discipline trainer (Ctrl+J) sends `spendAppliedSciencePoints(disciplineId)` (cell method 95) for any click, whatever its tree colours say (audit C-36). The base decides it in one transaction that locks the player's `sgw_player` row `FOR UPDATE`, checking in this order:

| Check | Refusal text | `onErrorCode` |
|---|---|---|
| The discipline is in the catalog | "There is no discipline N." | — |
| It is not already known | "You already know <name>." | — |
| At least one unspent ASP | "You have no applied science points." | 214 `NotEnoughAppliedSciencePoints` |
| The discipline's racial paradigm is at its required level | "<name> requires <paradigm> paradigm level N; yours is M." | — |
| Every required discipline is known | "<name> requires <prerequisite> at expertise 50." | — |
| … at expertise 50 or more | "<name> requires <prerequisite> at expertise 50; yours is N." | — |

A refusal is a `CHAN_FEEDBACK` text line and writes nothing. Each one is logged as a `crafting` `rejected` event whose `reason` (`unknown_discipline`, `already_known`, `no_asp`, `paradigm_too_low`, `prerequisite_missing`, `prerequisite_expertise`) and compared values say which check failed; a success is a `learned` event with the ASP before and after. The event catalog is the `crafting` row of [observability.md](../architecture/observability.md). A database failure is refused as "Learning disciplines is unavailable right now. Nothing was changed." On success the discipline is known at expertise 1 and one ASP is spent; no blueprint is granted (D-CR04, blueprints come from Blueprint items and research). The client then gets `onUpdateDiscipline(id, 1)` and the new ASP total. A repeated request finds the discipline known and changes nothing.

The four root disciplines (21 Biomedical, 40 Electronic, 59 Power Systems, 78 Materials Engineering) need Common level 5, which every character now starts at. The test rows 1 and 2 ("Basketweaving") need Common 1 and are treated like any other discipline.

## Stations, tools and crafting options

Each crafting verb except learning a discipline and respec needs a way to work (crafting campaign CR-05; decisions D-CR05, D-CR17, D-CR21):

- **Station.** Any entity whose template `entity_flags` carries an `ENTITYFLAG_Craft_*` bit (2048 craft, 4096 research, 8192 reverse engineering, 16384 alloying) is a station for those verbs, within `MAX_INTERACT_DISTANCE` (5 units, measured in 3-D like `interact`). The cell computes the station mask again for every request (`CraftRequest::allowed`, `player/crafting/forward.rs`), and a 1 Hz tick (`crates/cell/src/cell/service/ticks/crafting_stations.rs`) reports the nearest station per verb to the base when the set changes, for the window's label.
- **Field Crafting Tool.** One of the 48 tools (items 5369 and 8402-8466) in the crafting bag (`INV_Crafting`, container 15) covers crafting, research and reverse engineering for the disciplines of its applied science whose `tech_competency` is at most the tool's `tech_comp`. The science comes from the name prefix (BMAS Biomedical 1, MAS Materials 2, PSAS Power Systems 3, EAS Electronic 4), because the seed and the cooked data carry no science field for tools (CR-E2 Q3). For research and reverse engineering, any of the item's disciplines counts. Alloying needs a station. Tools are not consumed. Code: `base/crafting/tools.rs`.
- **Craft anywhere.** `.allcraft` turns it on for the target's session until logout: every verb passes the gate, and the crafting options name the player's own entity as the machine in all four sections, as the legacy command did.

The gate runs on the base in `handle_craft_request`, before any verb handler (`base/crafting/gate.rs`). A refused request gets the text line "No crafting station or tool for crafting nearby." (or research, reverse engineering, alloying) on the feedback channel.

`onUpdateCraftingOptions` (140) carries, per section, the station as the machine and the best tool (highest `tech_comp`, then lowest instance id) as the tool; alloying never names a tool. The client keeps only the last id of each array and checks neither distance nor existence (CR-E1 Q2), so the gate on the server is the only enforcement. The base sends it in the login crafting bundle after every `onClientReady` (after 136, 138, 139 and the ASP total) and then only on change: a station report, a tool entering or leaving the crafting bag (re-read after every inventory commit, in `send_full_inventory_update`), or `.allcraft`. Code: `base/crafting/options.rs`.

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
| Applied Science Points | Currency spent to learn new disciplines; 1 at level 1 plus 1 per level gained |
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
