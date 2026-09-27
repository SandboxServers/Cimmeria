# Crafting and Applied Science: Audit Against `main`

> Type: reference. Audience: the coordinator and packet workers.
> Updated: 2026-09-26, against `main` @ `70795027`. Companions: [campaign README](README.md), [work packets](work-packets.md), [crafting restoration findings](../../reverse-engineering/findings/crafting-restoration.md), [crafting state machine](../../reverse-engineering/findings/crafting-state-machine.md), [crafting wire formats](../../reverse-engineering/findings/crafting-wire-formats.md), [gameplay: crafting](../../gameplay/crafting-system.md).

Every row was checked against the code, the seed, the client Lua or the client binary (Ghidra, read-only). Row IDs (`C-nn`) are cited by the work packets. Paths are relative to the repository root unless they start with `Lua:` (the client's `SGWGame/Content/UI/Core/`) or `Py:` (`deprecated/python/`).

## 1. What exists on `main`

| ID | Finding | Evidence |
|---|---|---|
| C-01 | `CraftingState` (disciplines, expertise map, blueprints, ASP, paradigm levels) with one serializer: `onUpdateDiscipline` (136), byte-exact tested. | `crates/entity/src/crafting.rs:37-127`, tests `:170`, `:187` |
| C-02 | Transactional load and save across `sgw_player` and `sgw_player_discipline_expertise`, with five live-DB tests. Both functions are `#[allow(dead_code)]`; only the GM grant handlers call them. | `crates/base-session/src/base/crafting/persistence.rs:40`, `:208`, tests `:391-588` |
| C-03 | GM grants: native `gmGiveExpertise` (139) and `gmGiveAppliedSciencePoints` (140), plus `.learndiscipline`, `.forgetdiscipline`. `.allcraft` replies "not wired". | `crates/cell-console/src/cell/console/gm/mod.rs:92-97`, `gm/give.rs:259`, `:349`, `console/crafting.rs:47-128` |
| C-04 | The cell routes methods 95-100 to `crafting::dispatch`. Every arm logs `UNIMPLEMENTED` and returns `true`. Method 96 and 99 parse only their first `INT32`; 97 and 98 parse nothing. | `crates/cell-methods/src/cell/cell_methods/player/crafting.rs:15-86`; routing `player/dispatch.rs:60` |
| C-05 | Login sends `onEntityProperty(2, applied_science_points)` and `onUpdateKnownCrafts` (139). It does **not** call `load_crafting_state`, and never sends 136, 138 or 140. | `crates/wire/src/mercury/world_data/map_loaded.rs:330`, `:380-388`; `player_load/core/player_data.rs:46-61` |
| C-06 | A GM ASP grant persists but pushes nothing to the client. The client listens for the property update (id 2). The handler's doc comment says no push exists, which is wrong. | `base/crafting/handlers.rs:164-168`, `:243`; Lua `Crafting/DisciplineTrainer.lua:49-54` |
| C-07 | No Rust loader for `resources.disciplines`, `resources.blueprints` or `resources.blueprints_components`. No Rust item catalog loads `flags`, `tier`, `tech_comp`, `quality_id` or `discipline_ids`. | grep of `crates/`; `crates/cell-catalog/src/cell/spawner/loot.rs:65`, `:173` load other item columns only |
| C-08 | Missing Rust constants: `EItemFlag` (only `CanBeSold` is declared, twice), `ECraftTypeFlags`, `ENTITYFLAG_Craft_*`, `ETimerUpdateType::CraftInductionTimer = 16`, the crafting `EConditionHandlerFeedback` values. | `entities/defs/enumerations.xml:700-718`, `:1124-1144`, `:1487-1490`, `:1676-1691`; `vendor/data/mod.rs:12`, `vendor/sell/mod.rs:21` |
| C-09 | No serializer for 112, 137, 138, 140. 139 is built inline in `map_loaded.rs`. | `crates/wire/src/cell/client_methods/player.rs:32-88` (constants only) |
| C-10 | The cell entity holds no crafting state: no busy flag, no induction deadline, no `craftingEntityFlags`. No base↔cell message exists for any crafting verb. | `crates/entity/src/cell_entity/`; `crates/wire/src/cell/messages/cell_to_base.rs:479-493` (GM grants only) |
| C-11 | Reusable inventory pieces: `consume_design_quantity` (multi-stack, `FOR UPDATE`, bags 1, 2 and 15, no client update), `handle_grant_item` (stack merge, slot reservation, outbox), and the vendor purchase transaction as the "consume inputs and grant a product in one transaction" template. The vendor path never sends `onRemoveItem` for a drained stack, and `onUpdateItem` is upsert-only. | `vendor/purchase_helpers.rs:284-336`, `inventory/grant/grant_item.rs:33`, `vendor/purchase/mod.rs:31-355`, `inventory/core/mod.rs:315-322` |
| C-12 | `INV_Crafting = 15` is a real 100-slot bag, sent in the login bag info. | `crates/entity/src/inventory.rs:27`, `:50`; `crates/wire/src/containers.rs:16` |
| C-13 | Timer plumbing: `serialize_timer_update` (21 bytes, `secondaryId` hard-coded 0) and client method 12. Every Rust sender passes `BigWorldTimeComplete = 0.0` or a relative time. The login clock (`SET_GAME_TIME 0`, `TICK_SYNC 0`) and the 10 Hz tick loop that declares 100 ticks/s disagree, so no absolute expiry can be computed today. | `crates/entity/src/abilities/wire.rs:27`; `warmup/mod.rs:191-200`; `crates/wire/src/mercury/protocol/session.rs`; `.claude/agent-memory/combat-systems-advisor/ontimerupdate-wire-and-clock.md` |
| C-14 | No generic per-player delayed-action scheduler. Features add a deadline field and a tick in `crates/cell/src/cell/service/ticks/`. `schedule_content_action` is entity-keyed but carries content `Action`s only. | `crates/cell/src/cell/service/message_loop.rs:31`; `space_manager/deferred_content_actions.rs:62` |
| C-15 | Interaction range: `interact_target_in_range` (5.0 units) and the `last_interaction_target` pin. `INT_MACHINE_*` and `INT_VendorCraft*` constants exist but nothing reads them. | `crates/cell-interactions/src/cell/interactions/dispatch/mod.rs:27-96`; `crates/entity/src/interaction_flags.rs:57-105` |
| C-16 | No shared `onErrorCode` helper; each site builds the 7 bytes itself. `EErrorCodeSystem` has only `Ability = 0`. | `vendor/train_feedback.rs:83`, `ability_tree/respec.rs:60` |

## 2. Data

| ID | Finding | Evidence |
|---|---|---|
| C-20 | 78 disciplines over 4 applied sciences (19/21/19/19) and 5 racial paradigms. 72 have prerequisites; every prerequisite id exists. | `db/resources/Archetypes/Seed/disciplines.sql`; `applied_science.sql`, `racial_paradigm.sql` |
| C-21 | **The real root disciplines need Common paradigm level 5.** 21 Biomedical, 40 Electronic, 59 Power Systems and 78 Materials Engineering all require level 5. Only the test rows 1 and 2 ("Basketweaving") need level 1. Python starts every paradigm at 1 and nothing raises it except GM commands, so under the legacy rules no player can learn any real discipline. | `disciplines.sql:25`, `:45`, `:59`, `:121`; Py `cell/Crafter.py:46-47` |
| C-22 | 498 blueprints (40 alloy, all quantity 2 and `requires_elementary_components`), in only 16 of the 78 disciplines; discipline 21 alone owns 201. No broken references. Blueprint 21 (Ambernol Vial) has no components. | `db/resources/Entities/Seed/blueprints.sql` |
| C-23 | 2,556 component rows. `component_set_id` 1-4 are **alternative recipes**, not steps (219 blueprints have one set, 181 two, 96 four). 346 distinct component items, 255 of them also blueprint products. No component is stackable (`max_stack_size = 1`). | `blueprints_components.sql`; blueprint 412 at `:3059-3073` |
| C-24 | **Item flags are unreliable.** `ElementaryComponent` (32768) is set on all 6,059 items. `Craft_Craft` and `NotResearchable` are on none. `Craft_Research` (3,912) and `Craft_RevEng` (3,911) do mark researchable and reverse-engineerable gear. `Kicker` is on exactly 4 items (5668-5671). `Craft_Alloying` is on 1. | `db/resources/Items/Seed/items.sql`; bits `enumerations.xml:1124-1144` |
| C-25 | Nothing makes an entity a crafting station: no template sets `ENTITYFLAG_Craft_*` (2048/4096/8192/16384) or an `INT_VendorCraft*`/`INT_MACHINE_*` bit. Names for four stations exist ("BioMedical / Electronics / Power Systems / Materials Crafting Station"), as do the marker visuals (interactions 56-60). | `entity_templates.sql` (192 rows); `texts.sql:36967-36981`; `Worlds/Seed/interactions.sql:65-73` |
| C-26 | 48 "Field Crafting Tool" items (5369, 8405 and on, `tech_comp` 5-55) carry no crafting flag. | `items.sql` |
| C-27 | Seeded characters have no disciplines, no paradigm levels, 0 ASP and no blueprints. | `db/sgw/Players/Seed/sgw_player.sql` |
| C-28 | Free id blocks on `main` and in every open PR: templates 310-329, spawns 410-429. The neighbours are the debug hub (#846: 300-304, 400-404), Harset (200-299, 300-399), guilds (330-349, 430-449) and pets (350-369, 450-469). | `entity_templates.sql` max 248; `spawnlist.sql` max 349 |

UAT-friendly recipes (from C-22 and C-23):

- **Craft:** blueprint 412 (discipline 21) makes 5401 "Titanium Plating" from 14× 5254 "Steel Core" (set 1), or 1× 5254 + 5× 5256 (set 2). Blueprint 161 chains after it: 1× 5401 + 1× 5254 → 5339.
- **Alloy:** blueprint 42 (discipline 21) turns 1× 5192 "Cell (Bio-Medical)" (tier 2, Good) plus tier-1 elementary components into 2× 5191 "Blend".
- **Research and reverse engineer:** 5481 "Crafted Pistol of the Whale" (tc 20, disciplines {21, 22}), made by blueprint 1. Kickers 5668-5671.

## 3. Client behaviour (Lua and SGW.exe)

| ID | Finding | Evidence |
|---|---|---|
| C-30 | The client can request only blueprints the server listed in `onUpdateKnownCrafts`. Its dispatcher sends `alloying` for a known alloy and `craft` for a known craft; anything else throws locally. | `0x00e48f70` |
| C-31 | The craft page sends **one item instance per component type** (the last stack found). A requirement that spans several stacks therefore cannot be met from the submitted ids alone. | Lua `CraftingPage.lua:306-333` |
| C-32 | The induction bar comes from `onTimerUpdate` with `Type = 16` only. The handler reads `SourceID`, `TotalTime` and `BigWorldTimeComplete`, computes `remaining = complete − client clock`, and draws the bar only when the unit is the player. There is no "craft started" or "craft finished" message. | `0x00e47800`; Lua `CraftingPage.lua:470-480`, `SelfStatus.lua:167-175`; Py `cell/SGWPlayer.py:851-858` |
| C-33 | After a craft the page refreshes on inventory slot updates only. On confirm, research, alloy and reverse engineer **empty their slots immediately**. If the server answers nothing, the player sees nothing. | Lua `ResearchPage.lua:62-72`, `AlloyPage.lua:88-100`, `ReverseEngineeringPage.lua:470-472` |
| C-34 | Reverse engineer sends one `reverseEngineer` per slotted item, **up to 10 in a burst**. Python's single timer slot crashes on this. | Lua `ReverseEngineeringPage.lua:470-473`; Py `Crafter.py:421` |
| C-35 | `onUpdateCraftingOptions` keeps only the **last** `items` id (tool) and the **last** `entities` id (machine) per section, and registers the machine as a unit alias. `isCraftingAllowed(t)` returns that pair. The window opens regardless (J key) and shows "Disabled" plus a "find the proper machine or tool" prompt. The existing Ghidra PRE_COMMENT on `0x00e465d0` mislabels these offsets as blueprint lists. | `0x00e49180` → `0x00e47250`; `0x00e465d0`; Lua `Crafting.lua:41-47` |
| C-36 | Spend ASP has no client-side check: every click sends 95, even for a known or locked discipline. The tree colours (green/red) use paradigm level and prerequisite expertise ≥ 50. | Lua `DisciplineTrainer.lua:117-131`, `:229-231` |
| C-37 | Respec: the client stores the cost from 112 and prompts "This will unlearn all your crafting knowledge"; Yes sends 100. There is no respec button in Lua, but the binary has `Event_SlashCmd_RespecCraft`, so the likely flow sends 100 twice (query, then confirm). `onDisciplineRespec` (137) zeroes every known discipline's expertise; nothing in the client clears known blueprints. | `0x00e476f0`, `0x00e46700`, string `0x018417f0`; Lua `Crafting.lua:181-191` |
| C-38 | **Alloy counts disagree.** The client (`0x00e46990`) wants Normal 10, Good 5, Great 2, Fantastic 1 (no Poor), counts **stack quantity**, and requires one quality's count to be met; the Lua has 10 elementary slots and 4 quality radios. Python wants Poor 10, Normal 5, Good 3, Great 2, Fantastic 1 and counts items. | `0x00e46990`; Lua `AlloyPage.lua:467-477`; Py `common/Constants.py:83-89` |
| C-39 | **Kicker rules disagree.** The client allows one kicker per applied science, never from the item's own science; Python checks neither. The research chance display ignores the tech-competency limit. | Lua `ResearchPage.lua:86-117`, `:202-211`; `0x00e483f0` |
| C-40 | Every client-side check logs a warning through `Mercury__unknown_00ceae50` and **sends the request anyway**, so the server's checks must be complete. | client send functions `0x00e47b10`, `0x00e47ec0`, `0x00e483f0`, `0x00e48a90` |

## 4. Legacy Python defects not to port

`deprecated/python/cell/Crafter.py` is the reference algorithm, but it is itself a reconstruction and has these bugs:

| ID | Defect | Where |
|---|---|---|
| C-50 | `randint(0, len(x))` is inclusive, so it can index past the end. Three places: research discipline pick, reverse-engineer blueprint pick and component-set pick. | `:324`, `:387`, `:388` |
| C-51 | Reverse engineer divides by expertise, which is 0 when the discipline is unknown. | `:395-399` |
| C-52 | The reverse-engineer bias rewards **low** expertise (tc > exp gives bias > 1, so more recovery). It looks inverted. | `:396-409` |
| C-53 | The alloy component checks are chained comparisons ending `is None`, so they are always false and never run. | `:469`, `:473` |
| C-54 | Alloy consumes with `removeItemByDesign(component.id, …)`, passing an instance id as a type id. | `:502`, `:504` |
| C-55 | `gainExpertise` asserts the discipline is known, and runs **after** the items are consumed and the product granted. | `:111`, `:263-269` |
| C-56 | Craft never checks `quantity ≥ 1`, and consumes by type across the whole inventory rather than the validated bags. | `:191-245`; Py `cell/Inventory.py:272-307` |
| C-57 | `onAppliedSciencePointsChanged` sends the change (±1), not the new total. | `:91`, `:101` |
| C-58 | Items are consumed at induction start; a crash or logout during the 3 s loses them. | `:241-253` |

## 5. Documentation errors found

| ID | Error | Fix owner |
|---|---|---|
| C-60 | `crafting-restoration.md` says world-entry discipline sync is 100% done; only 139 is sent (C-05). | CR-13 |
| C-61 | `crafting-restoration.md` and the Ghidra PRE_COMMENT on `0x00e465d0` call the `+0x38`…`+0x50` offsets blueprint lists; they are the (tool, machine) pairs (C-35). | CR-E1 |
| C-62 | `crafting-wire-formats.md` types `craftingEntityFlags` as INT32; the def has it as `PYTHON`. | CR-E1 |
| C-63 | `base/crafting/handlers.rs:164-168`, `:243` claim ASP has no client push (C-06). | CR-03 |
| C-64 | CAT-F findings cite `crates/services/...`; the code is in `crates/cell-methods/...` since #825. | CR-13 |
