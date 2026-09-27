---
title: "Crafting System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Crafting System

> **Last updated**: 2026-09-27
> **Status**: State model, persistence, GM grants, the login sync (CR-03), learning disciplines with applied science points (CR-04) and earning those points by levelling (CR-12) work, and so do Blueprint items and Racial Paradigm Guides (CR-15) research and reverse engineering (CR-08), alloying (CR-09) and crafting from a blueprint (CR-07). Respec is still a stub (tracked in #567). Findings: [`reverse-engineering/findings/crafting-restoration.md`](../reverse-engineering/findings/crafting-restoration.md).

## Overview

The crafting system enables players to create items through blueprints, research items for expertise, reverse engineer items into components, and alloy materials into higher tiers. Crafting is gated by disciplines (learned skill trees), racial paradigms (faction-specific tech trees), and Applied Science points (discipline training currency).

The Rust implementation lives in [`crates/base-session/src/base/crafting/`](../../crates/base-session/src/base/crafting/) (persistence, GM grants, the induction engine and the consume-and-grant transaction) and [`cell/cell_methods/player/crafting/`](../../crates/cell-methods/src/cell/cell_methods/player/crafting/) (cell methods 95–100: argument parsing and the forward to the base). The state model is `cimmeria_entity::crafting::CraftingState`.

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
| Crafting (blueprint) | DONE | `base/crafting/craft/` (CR-07), see [Craft in Cimmeria](#craft-in-cimmeria) |
| Research | DONE | Item and kickers checked at the request, rolled and consumed when the bar ends; +5 expertise and the blueprint on a success. CR-08 |
| Reverse engineering | DONE | Exactly the named item is consumed when the bar ends; recovery rises with expertise (D-CR06). CR-08 |
| Alloying | DONE | `base/crafting/alloy/` (CR-09), see [Alloying](#alloying) |
| Crafting respec | STUB | The base answers "Crafting respec is not available yet." |
| Timer-based induction | DONE (engine) | `base/crafting/session/`: one running induction per player, ten held in all; the bar is `onTimerUpdate` type 16 with an absolute expiry. Craft (CR-07), research, reverse engineering (CR-08) and alloying (CR-09) submit to it. See [Induction engine](#induction-engine) |
| Consume-and-grant transaction | DONE (engine) | `base/crafting/transaction/`: one database transaction per completed induction. Craft (CR-07), research, reverse engineering (CR-08) and alloying (CR-09) build one |
| Busy state lock | REPLACED | The induction queue serializes a player's crafting; there is no separate busy flag |
| Crafting stations | DONE | The cell tracks the nearest station per verb within `MAX_INTERACT_DISTANCE` (5 units, 3-D) and reports changes to the base once a second; the forward recomputes the mask per request. CR-05. The stasis-room debug hub seeds four, templates 310-313 ([debug-hub.md](../content/debug-hub.md#crafting-corner)), CR-11 |
| Field Crafting Tools | DONE | A tool in the crafting bag (container 15) covers crafting, research and reverse engineering for its science up to its `tech_comp` (D-CR21). CR-05 |
| Station gate | DONE | Crafting, research, reverse engineering and alloying are refused with "No crafting station or tool for <verb> nearby." unless a station, a covering tool or "craft anywhere" allows them. CR-05 |
| `onUpdateCraftingOptions` | DONE | Sent last in the login crafting bundle after `onClientReady` (every world entry), then on every change of stations, tools or "craft anywhere". CR-05 |
| `.allcraft` | DONE | GM: every paradigm at 7, every discipline at 100, every blueprint, persisted, plus "craft anywhere" until logout (D-CR17). CR-05 |
| `.craftkit` | DONE | GM: `.craftkit <blueprintId> [count]` grants the target the items of the blueprint's component set 1, `count` (1-10) times over, through the crafting transaction as a grant-only plan: each item goes to the first carried bag its `container_sets` allow (the crafting bag for the `{17,15}` components), the whole kit or nothing, with the client's inventory update and the cell's inventory events (D-CR17). CR-11 |
| `.learnblueprint` | DONE | GM: `.learnblueprint <blueprintId>` teaches the target one blueprint (not already known, present in the catalog) under the player-row lock and pushes the full 139 list (D-CR17). CR-11 |
| Crafting supplies vendor | DONE | The debug hub's supplies vendor (template 314) sells the UAT components, kickers, a -5 and -50 Field Crafting Tool per science, the Racial Paradigm Guides and Blueprint item 6483 at 1 naquadah each. A purchase lands in the main bag, where the crafting verbs and item use accept it; a tool must be moved to the crafting bag. CR-11 |
| Blueprint items | DONE | Using one of the 193 mapped "Blueprint: …" items teaches its blueprint(s) and consumes it (D-CR04, D-CR26). CR-15, see [Blueprint items and Racial Paradigm Guides](#blueprint-items-and-racial-paradigm-guides) |
| Racial Paradigm Guides | DONE | Using a guide (items 7805-7809) raises its paradigm by 1, to at most 10, and consumes it (D-CR03). CR-15 |

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

## Blueprint items and Racial Paradigm Guides

Blueprints and paradigm levels come from items the player uses (crafting campaign CR-15; decisions D-CR03, D-CR04, D-CR22, D-CR26). The client sends the ordinary `useItem(item_id, target_id)`.

- **What an item does** is the seed table `resources.crafting_item_effects` (`item_id`, then `blueprint_id` or `racial_paradigm_id`). It holds 193 Blueprint items (194 rows: item 8882 "Health Antidote" teaches blueprints 367 and 369) and the five guides, 7805 Human, 7806 Common, 7807 Asgard, 7808 Goa'uld and 7809 Ancient. The seed is generated from `docs/analysis/crafting/source/blueprint-items.csv` by [`tools/crafting/generate_item_effects.py`](../../tools/crafting/README.md); the 96 Blueprint items whose blueprint the cooked data does not settle are not seeded, so using one does what any other item does (it fires `OnItemUse`, and no chain listens).
- **The item-use path** (`handle_use_inventory_item`) sends an item with rows to `base/crafting/item_use/` instead of firing `OnItemUse`, so no content chain can consume it a second time.
- **One transaction** takes the player's inventory advisory locks (the player-wide key 0, then the main and crafting bags), locks the item row (owner in the `WHERE`), then the crafting state on `sgw_player`, decides, consumes one of the item and saves the new state. Nothing is consumed without the change, and no change is saved without the item. The order is the shared inventory order (advisory locks, item rows, the player row), so a use waits for a move, trade, vendor purchase or crafting completion of the same player instead of deadlocking with it.
- **A miss** (the instance is another character's, or this player already used it up) is still a crafting use when the item is a crafting item: it gets the "no longer in your inventory" line. The base remembers the last 1,024 instances it consumed this way, since the row is gone. Any other item that misses is ignored with a server-side WARN, as before.
- **After the commit** the client gets `onUpdateKnownCrafts` (139, the whole list) or `onUpdateRacialParadigmLevel` (138), then `onRemoveItem` when the stack is gone and the inventory update. The cell is told the item was removed through the outbox.

| Refusal | Text | `reason` |
|---|---|---|
| Every blueprint the item teaches is known | "You already know this blueprint. The item was not used." ("these blueprints" for 8882) | `already_known` |
| The guide's paradigm is at 10 | "Your <paradigm> racial paradigm is already at 10, the maximum. The guide was not used." | `paradigm_max` |
| The instance is gone (used, traded, never the player's) | "That item is no longer in your inventory." | `item_missing` |
| The item is not in the main bag or the crafting bag (bank, buyback list) | "Move that item to your crafting bag to use it." | `not_carried` |

A refusal consumes nothing. An item that names a known and an unknown blueprint teaches the unknown one and is used. `target_id` plays no part: the effect always applies to the user. Events: `blueprint_learned` and `paradigm_raised` with the values before and after, in the `crafting` row of [observability.md](../architecture/observability.md).

Sources today are GM grants (`gmGiveItem` puts the item in the main bag, where it can be used) and, later, the crafting-supplies vendor (CR-11). No loot table drops these items yet: loot and the content engine's `grant_item` put an item in the first container of its `container_sets`, which for all 198 items is the bank (17), until the grant path falls through to the crafting bag (CR-16). An item in the bank must be moved to the crafting bag before it can be used.

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

#### Craft in Cimmeria

`base/crafting/craft/` (CR-07). The client sends `craft(blueprintId, itemIds, quantity)` (cell method 96) with one instance id per component of the set the player picked, the last stack the page found (`CraftingPage.lua:299-313`); the set id is not on the wire. After the station gate, the base checks, in this order:

1. `quantity` is 1 to 100.
2. The player knows the blueprint, and the catalog has it.
3. It is not an alloy (those go through `alloying`).
4. Its discipline is known (D-CR15).
5. Every named instance is the player's and sits in the main bag (1) or the crafting bag (15).
6. The set is the one whose designs are **exactly** the named designs. A set whose designs are only covered is not enough: many seed sets are subsets of a sibling (blueprint 412's set 1, 14 Steel Cores, is covered by set 2's Steel Core plus Titanium Cores), and the legacy "first covered set" rule would charge the wrong recipe. Where two sets have the same designs (blueprint 159 sets 1 and 2), the first the bags can pay for wins. A blueprint with no component set (21, Ambernol Vial) is never craftable.
7. The two bags hold `quantity Ã— component.quantity` of each component, counted across stacks. The bank does not count.

A passing craft is queued on the [induction engine](#induction-engine) with the blueprint as the bar's `ID`. Nothing is consumed at the request. A craft that waits behind another gets "Crafting <product> x<n> is queued behind <k> other crafting job(s)." At the end of the bar one transaction consumes the set by design, grants `blueprint.quantity Ã— quantity` of the product, checks that the blueprint and its discipline are still known (reading the player row `FOR SHARE`, so a respec that commits during the bar, or while the transaction runs, refuses the craft and rolls it back), and adds 1 expertise to the discipline (136). The named instances only choose the set: the completion does not hold the craft to them, so a later queued craft whose named stack an earlier one drained still runs from the rest. The player then reads "You crafted <product> x<n>."

| Why | Line |
|---|---|
| `quantity` outside 1 to 100 | You can craft between 1 and 100 at a time. |
| Blueprint not known | You do not know that blueprint. |
| Alloy blueprint | That blueprint is an alloy. Use alloying to make it. |
| Discipline not known (also at completion) | You must learn the blueprint's discipline first. |
| A named instance is not the player's | A component is no longer in your inventory. Nothing was used. |
| A named instance is outside the two bags | Components must be in your backpack or crafting bag. Nothing was used. |
| The named designs match no set | Those components do not match any recipe of this blueprint. Nothing was used. |
| Too few in the two bags | You do not have enough components: <have> of <need> needed. Nothing was used. |
| The server could not read what it needs | Crafting is unavailable right now. Nothing was changed. |

The craft page keeps its slots on confirm (C-33), so a request-time refusal sends only the line; a refusal at completion also resyncs the inventory. Not ported from `Crafter.py`: consuming across the whole inventory and never checking `quantity â‰¥ 1` (C-56), gaining expertise without the discipline and after the grant (C-55), consuming at the start of the bar (C-58), and the first-covered-set choice.

### Research

Uses up an item, and any kickers, for a chance at expertise in one of the item's disciplines and at the blueprint that makes it. Code: `base/crafting/research/`.

**At the request**, refused with a text line and nothing used:

| Check | Line | `reason` |
|---|---|---|
| The item or a kicker is gone or not the player's | A component is no longer in your inventory. Nothing was used. | `component_missing` |
| The item or a kicker is outside the main bag (1) and the crafting bag (15) | Components must be in your backpack or crafting bag. Nothing was used. | `component_not_in_crafting_bags` |
| The item is not flagged `Craft_Research` | That item cannot be researched. Nothing was used. | `not_researchable` |
| A kicker is not flagged `Kicker`, or has no applied science | That item is not a research kicker. Nothing was used. | `not_kicker` |
| A kicker of the item's own applied science | Kickers cannot come from the same applied science as the item being researched. Nothing was used. | `kicker_same_science` |
| A second kicker of one applied science | Only one kicker per applied science can be used. Nothing was used. | `kicker_duplicate_science` |

The kicker rules are the client's (`ResearchPage.lua`: one kicker slot per science, never the item's own); the client sends the request even when its own checks fail, so the server repeats them.

**When the bar ends** the job reads the player's crafting state and rolls:

1. The eligible disciplines are the item's disciplines the player knows with `0 < expertise < item tech competency`. With none, nothing is rolled: the item and kickers are used and the line says the research taught nothing new.
2. One discipline is picked uniformly, then the chance is `100 − expertise + 5 × kickers` percent against a roll in `[0, 100)`.
3. One transaction consumes exactly the named item and kickers. On a success it adds 5 expertise to the picked discipline and teaches every blueprint that makes the item whose discipline the player knows (checked again under the player row lock), then sends `onUpdateDiscipline` (136) and the whole known list, `onUpdateKnownCrafts` (139).

The player reads "Research succeeded: <discipline> expertise increased to <n>." (plus "You learned 1 new blueprint." when one was taught) or "Research complete, but no expertise was gained." The item and kickers are used whatever the roll, as in the original server.

### Reverse Engineer

Uses up an item to recover some of the components of a recipe that makes it. Code: `base/crafting/reverse_engineer/`.

**At the request** the item must still be the player's, in bag 1 or 15, flagged `Craft_RevEng` ("That item cannot be reverse engineered. Nothing was used.", `not_reverse_engineerable`), and made by at least one blueprint with a recipe ("No known recipe makes that item, so it cannot be reverse engineered. Nothing was used.", `no_blueprint_for_item`). No discipline needs to be known. The reverse-engineering page sends one request per slotted item, up to ten at once; each is its own induction.

**When the bar ends** the job picks one of those blueprints and one of its component sets uniformly, then rolls each component: `floor(roll × min(1, max(expertise, 1) / tc) × quantity)`, where `expertise` is the player's in the blueprint's discipline (0 when unknown) and `tc` is the item's tech competency (D-CR06). Recovery rises with expertise and is full at the tech competency. When every component comes to zero, the component with the highest roll recovers one unit. One transaction consumes exactly the named instance (never another stack of the same design) and grants the components, which land in the crafting bag. The player reads "Reverse engineering complete: recovered <n> components."

The legacy `Crafter.py` could pick past the end of its lists, divided by zero expertise, and rewarded low expertise (audit C-50 to C-52); none of that is ported.

### Alloy

Combines a current-tier material with lower-tier elementary components. This is the legacy Python flow; the Rust verb follows the client instead where they disagree (see [Alloying](#alloying)).

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

## Induction engine

Every crafting verb that takes time (craft, research, reverse engineer, alloy) runs through the same engine on the base. Unlike the original server, which consumed the components when the request arrived (so a logout or crash during the bar lost them), the Rust engine validates at the request and consumes only when the bar completes.

**Queue.** Each player has one running induction and a first-in-first-out queue, ten in all. The reverse-engineering page sends up to ten requests in one burst, so all ten are accepted. The eleventh is refused with "You can have at most 10 crafting jobs at once." and an inventory resync (one resync per burst). The queue is keyed by the player entity and lives only in memory.

**Bar.** When an induction becomes the running one, the player's client gets `onTimerUpdate` (client method 12) with `Type = 16` (`TIMER_CRAFT_INDUCTION`), `SourceID` = the player entity, `SecondaryID = 0`, `TotalTime = 3.0` and `BigWorldTimeComplete` = the server's game clock + 3.0. The client draws the crafting bar from that timer alone. The server waits on a monotonic deadline; it never stores the absolute game time, because the game clock restarts with the server.

**Completion.** At the deadline the engine runs the job, then starts the next one. It runs a job only while the entity still belongs to a connected session playing the same character, under the world name the queue started with.

**Dropped, never consumed.** The queue is dropped without consuming anything when the session ends (every disconnect path, `logOff` to character select or to desktop) and on gate travel. A job whose transaction is already running when the player leaves still finishes; the transaction is atomic either way.

**The transaction.** A completing item verb runs one database transaction:

1. Take the player-wide inventory lock and the per-bag locks (main bag, crafting bag, and every product's bag). The player row is read, and locked only by a research that teaches a blueprint, after every inventory row: the vendor stack locks inventory rows before the player row, and vendor purchase takes the same player-wide lock first, so neither can deadlock with a completion.
2. Lock each item instance the request named and check that it still belongs to the player, is of the design the verb named it for, and sits in the main bag (1) or the crafting bag (15).
3. Take any exact consumption from its named instance only (reverse engineering consumes the one item it names). Then consume each component by design across those two bags, the crafting bag first. The client names only one instance per component type, so a requirement that spans several stacks is met by design, not by the named instance. A bank stack never counts.
4. Place each product in the first main or crafting bag its `container_sets` list. The 752 crafting components list `{17,15}` (bank first) and land in the crafting bag. A product merges into one unbound stack with room for the whole quantity, or takes free slots, one per full stack.
5. Teach the blueprints the plan names (research only): lock the player row, after every inventory row, and add each blueprint whose discipline the player knows.
6. Add expertise to disciplines the player knows, capped at 100.

After the commit the client gets `onRemoveItem` for emptied stacks, one `onUpdateItem` for changed ones and `onUpdateDiscipline` for each changed discipline (if an item update fails to go out, the removal is sent again with a full `onUpdateItem`); the cell gets the inventory events through the outbox. If anything fails, nothing is applied and the player gets one of these lines, followed by a full inventory resync:

| Why | Line |
|---|---|
| A named component is gone or not the player's | A component is no longer in your inventory. Nothing was used. |
| A named component left the main and crafting bags | Components must be in your backpack or crafting bag. Nothing was used. |
| A named component is of another design | A chosen component is not the one this needs. Nothing was used. |
| Too few components in the two bags | You do not have enough components. Nothing was used. |
| No room for the product | Not enough room in your bags for the result. Nothing was used. |
| The product fits no carried bag | The result cannot be placed in your bags. Nothing was used. |
| A database error or an invalid plan | Crafting failed. Nothing was used. |

Rolls (research success, reverse-engineering recovery) go through an injectable RNG (`base/crafting/rng.rs`), so tests pin them.

## Alloying

`alloying` (cell method 99) turns the alloy blueprint's one component, plus elementary components one tier below it, into the blueprint's product. All 40 alloy blueprints take one component and make 2 of their product. Code: `base/crafting/alloy/` (`rules.rs` decides, `job.rs` is the induction, `mod.rs` loads the inputs and answers).

The request is checked when it arrives, in this order, and nothing is consumed then:

1. The blueprint exists and is an alloy, the player knows it, and knows its discipline.
2. The current-tier item is the player's, sits in the main or crafting bag, and is the blueprint's component design; the two bags hold enough of that design.
3. Each elementary item is the player's, sits in the main or crafting bag, and is exactly one tier below the component. A repeated id counts once.
4. The elementary items' stack quantities are summed per quality. Exactly one quality must reach its count: **Normal 10, Good 5, Great 2, Fantastic 1**. Poor has no count, so Poor items count toward nothing.

These are the client's own rules (`AlloyPage.lua:168-199` and the native count check in [crafting-client-ui.md §5](../reverse-engineering/findings/crafting-client-ui.md)), not the legacy Python ones: Python counted items rather than stack quantity and used Poor 10, Normal 5, Good 3, Great 2, Fantastic 1. The client only logs a warning and sends anyway, so the server enforces every rule. A list of more than ten elementary ids (the page has ten slots) can only be forged and is dropped with a `malformed` warning and no line.

A valid alloy is queued as an induction, with the blueprint id on the bar. When the bar ends, the player must still know the blueprint and its discipline (a respec during the bar refuses the alloy), then one transaction (see [Induction engine](#induction-engine)) re-checks the named elementary instances, consumes one of the component by design (crafting bag first, whichever stack that is), consumes exactly the met quality's count from the named elementary stacks in the order the request listed them, grants 2 of the product and adds 1 expertise to the blueprint's discipline. Elementary items of another quality, and any surplus beyond the count, are left untouched. Then the player reads "Alloying complete: 2 x <product>."

| Refusal | `reason` | Line |
|---|---|---|
| Blueprint unknown or not learned | `unknown_blueprint` | You do not know that blueprint. |
| Blueprint is not an alloy | `not_alloy` | That blueprint is not an alloy. |
| Its discipline is not known | `discipline_unknown` | You must learn the blueprint's discipline first. |
| Current-tier item is not the component | `component_mismatch` | A chosen component is not the one this needs. Nothing was used. |
| An item is gone, not the player's, or outside the carried bags | `component_missing` / `component_not_in_crafting_bags` | as in the table under [Induction engine](#induction-engine) |
| An elementary item is not one tier lower | `wrong_tier` | Elementary components must be one tier lower than the component (tier N). Nothing was used. |
| No quality's count met | `count_not_met` | The quantity of elementary components per item quality was not met: 10 Normal, 5 Good, 2 Great or 1 Fantastic. Nothing was used. |
| Two or more counts met at once | `multiple_buckets` | Multiple categories of elementary components were met; use one quality only. Nothing was used. |
| No database, catalog gap | `unavailable` | Alloying is unavailable right now. Nothing was changed. |

Every refusal is followed by a full inventory resync, because the alloy page empties its slots on confirm. A refusal when the bar ends (an input moved or used up in the meantime) uses the transaction's lines.

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
- **Blueprint items and guides**: `db/resources/Items/Seed/crafting_item_effects.sql` (generated, see [Blueprint items and Racial Paradigm Guides](#blueprint-items-and-racial-paradigm-guides))
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
