# Weapons/Ammo Pack Audit — READ-ONLY, no files touched

## Seed crosscheck (CONFIRMED / SOURCE-BACKED — verified by direct query against `db/resources/`)

| Pack claim | Verified against | Result |
|---|---|---|
| 6,059 cooked items | `db/resources/Items/Seed/items.sql` row count | **Exact match** (6,059) |
| 1,886 cooked abilities | `db/resources/Abilities/Seed/abilities.sql` row count | **Exact match** |
| 3,216 cooked effects | `db/resources/Effects/Seed/effects.sql` row count | **Exact match** |
| 1,073 physical weapon variants | `weapons_full_recovered_variants.json` records with `Scope=WEAPON` | **Exact match** (205 more tagged `RELATED COMBAT EQUIPMENT` = Asgard drone programs, correctly excluded from the weapon count) |
| 214 unique weapon names / 15 weapon families | Computed from the same file | **Exact match** |
| Item IDs exist in our seed | All 1,278 pack Item IDs (1,073 weapon + 205 related) vs `items.item_id` | **1,278/1,278 present, 0 missing** |
| "Legacy Clip Size" / "Legacy Ammo Types" columns | `resources.items.clip_size`, `.ammo_types[]`, `.default_ammo_type` | **1,278/1,278 exact match, 0 discrepancies** — confirmed these come directly from our own `items` table columns, not an independent source |
| "Legacy Event6/Event7 Ability IDs" | `resources.items_event_sets` (item_id, ability_id, event_id) | **This is exactly our table.** event_id=6 → melee ability, event_id=7 → ranged ability, confirmed by direct row inspection (e.g. item 21 SGHC 6 SMG: event 6→595, event 7→559, matches pack exactly). 1,274/1,274 non-empty bindings match; 4 records have no event row in either the pack or our seed |

**Important caveat for the report**: because the pack's weapon table is built by directly reading our own `items` + `items_event_sets` seed, a "match" here confirms *provenance*, not *correctness*. It tells us the pack didn't invent numbers — it doesn't independently validate that our seed's clip sizes/ammo types/bindings are the retail-correct values. The pack's own `known_gaps` section says this explicitly (P0: "which legacy clip_size values are correct... cross-check before implementation").

**Internal mismatches the pack itself flags (worth relaying, not our bug):** 58/1,278 records are tagged `CONFLICT` or `NO LEGACY EVENT` — cases where the pack's moniker-derived "expected" ability differs from what our own `items_event_sets` actually stores. Breakdown by family: Dart Gun 0/25 match (worst), Ribbon Device 151/178, Grenade Launcher 25/27, Staff Weapon 177/178, Blade 50/52, Pistol 52/53. Example: item 3584 "CO2 Pistol Dartgun" — moniker-derived melee ability is 1087, but our `items_event_sets` row actually binds melee to 708 (the generic Pistol Melee AA). This looks like the same "orphaned generic placeholder" pattern I've seen before in Harset content (ability 597 bound to unrelated items) — i.e., a handful of dart/ribbon/grenade items in the original seed got a generic fallback binding instead of a weapon-specific one. Not something the pack fabricated; it's pre-existing seed noise the pack surfaced correctly.

## Existing weapon/ammo runtime (all CONFIRMED from current code, not stale memory)

| Feature | Implementation | File | Data-driven? |
|---|---|---|---|
| Auto-attack ability per weapon (right-click / fire) | `space_mgr.item_event_set_abilities: HashMap<(item_id,event_id),ability_id>` loaded at startup from `resources.items_event_sets`; consulted by the archetype-default→weapon redirect and the right-click-NPC path | `crates/services/src/cell/spawner/abilities.rs:317` (`load_item_event_set_abilities`), `crates/services/src/cell/abilities/use_ability/weapon_redirect.rs`, `crates/services/src/cell/abilities/resolve.rs` | **Yes** — fully DB-sourced. This is the live implementation of exactly what the pack calls "Legacy Event6/Event7 binding" |
| Reload | Two-phase (draw-window / actual reload) state machine, `autoReload` client option honored | `crates/services/src/cell/cell_methods/player/world/reload.rs`, `crates/services/src/cell/abilities/use_ability/auto_reload.rs` | Clip size from `BandolierItem.clip_size` (DB-sourced at grant time) |
| Ammo consumption per ability | `ability_def.required_ammo` gates fire and decrements `entity.active_ammo()` | `crates/services/src/cell/abilities/use_ability/handle.rs:369-445`, field defined `crates/entity/src/abilities/defs.rs:114` | **Yes** — `required_ammo` is a real column loaded from `resources.abilities` (`crates/services/src/cell/spawner/abilities.rs:91`) |
| Ammo-*type* selection (which loaded ammo, e.g. Bullet_Armor_Piercing) | `requestAmmoChange` cell method: validates against the weapon's `ammo_types[]` allow-list, persists via `update_bandolier_ammo` (item_id-keyed, PR #520) | `crates/services/src/cell/cell_methods/inventory/bandolier/ammo_change.rs`, `crates/services/src/base/world_entry/methods/inventory/ammo.rs` | **Yes**, fully data-driven and implemented |
| Ammo-*mode* toggle **abilities** (the pack's 17 buffs: Hollow Point 715, Armor Piercing 719, Incendiary 723, Dart toggles 990-998+) | **Not implemented.** Ability/effect rows exist in the seed (confirmed `abilities.sql` has all of 715/719/723/990-992 etc.), but no effect script exists in the registry to apply their damage/penetration modifiers, and effect 747 ("Armor Piercing Ammunition") has `script_name = NULL` even in the raw seed — the original data itself never wired a script here | Registry checked: `crates/services/src/cell/effects/registry.rs` has no ammo-related arm | **Gap**, not a bug — matches the pack's own note "toggle semantics are original; exact ammo consumption/compatibility still needs reconstruction" |
| 4-slot bandolier / active weapon | Implemented: `active_bandolier_slot`, `bandolier_items: HashMap<slot,BandolierItem>`, `active_clip_size()/active_ammo()/refill_active_slot()` helpers; "4 bandolier slots" referenced directly in code comments | `crates/entity/src/cell_entity/bandolier.rs`, `crates/services/src/base/world_entry/methods/inventory/core/use_instance.rs:290` | Yes |
| Clip sizes from DB | `BandolierItem.clip_size` populated from `resources.items.clip_size` at grant/load time, never hardcoded | `crates/services/src/base/world_entry/methods/inventory/grant/grant_item.rs` | Yes |
| TechComp/quality scaling into combat damage | **Not implemented anywhere in combat/damage code.** `tech_comp` only appears in vendor recharge pricing and the Livewire minigame — no hit in `crates/services/src/cell/combat/` or `crates/services/src/cell/effects/` | grep-confirmed absence | Matches pack's own Phase 4 instruction: "keep TechComp scaling configurable until recovered" |

## Phase 4 checklist gap table

| Checklist item | Verdict | Pointer |
|---|---|---|
| Import weapon-family/auto-attack mappings | **Exists already**, sourced from the same `items_event_sets` table the pack itself reads — nothing to "import," it's live | `spawner/abilities.rs`, `abilities/resolve.rs` |
| 4-slot bandolier/active weapon semantics | **Exists** | `entity/src/cell_entity/bandolier.rs` |
| Reload | **Exists** | `cell_methods/player/world/reload.rs` |
| Ammo-type toggles | **Partial** — ammo *type selection* (requestAmmoChange) exists; ammo *mode toggle abilities* (Hollow Point/AP/Incendiary/darts) do not | `bandolier/ammo_change.rs` (done) vs `effects/registry.rs` (missing) |
| TechComp scaling configurable | **Missing / not started** — no TechComp hook anywhere in damage resolution | n/a |

## Starter loadout authority conflict — flag for the report

Two separate conflicts, both worth surfacing:

1. **Items/abilities grant mechanism.** The pack leaves `starting_inventory`/`starting_armor`/`starting_weapon` explicitly `null` in `starter_loadouts.json` and calls `char_creation_abilities` "not authoritative enough." But our server **already has a working, data-driven starter grant path** the pack didn't audit: `resources.char_creation_choices` + `resources.char_creation_visgroups` grant starter *items* (visgroup/appearance-choice driven, including weapons/armor tied to a choice's `item_id`), and `resources.char_creation_abilities` grants starter *abilities* (verified: char_def 3 → ability_id 597, 592, etc.). Implemented in `crates/services/src/base/character_create.rs:196-489`. This is live and shipping today, not "unresolved" — the pack's caution about `char_creation_abilities` alone doesn't account for the choices/visgroups path that supplies items.
2. **Starting world.** The pack's `starter_loadouts.json` states as "USER-CONFIRMED / PROJECT TARGET": SGU Human → Earth SGC, Free Jaffa → Dakara, Asgard → Pertho, and its `QA_TESTS.md` explicitly requires "SGC_W1 placeholder starts are not reintroduced." **Every row in our current `db/resources/Archetypes/Seed/char_creation.sql` has `starting_world = 'Castle_CellBlock'`** for all archetypes — i.e., our live seed routes every new character through the CellBlock tutorial, not to Earth SGC/Dakara/Pertho. Given the active Castle rebuild campaign (Castle 701-708 merged, per prior session records), this is a real scope conflict between the pack's stated target state and the project's current in-flight direction, not a bug on either side. Needs a human decision on which is authoritative before Phase 5 work starts.

## Errors in pack data relative to our RE-backed knowledge

**None found.** The pack does not touch wire propIds (no `propId`/`AccessLevel`/`AmmoTypeId` references anywhere in the pack), so there's no propId-3-vs-7 collision to flag. Its clip_size/ammo_type/event6/event7 claims are 100% traceable to our own seed columns, so they're internally consistent with what we already have — the risk is entirely in what those seed values *mean* (unvalidated legacy data), which the pack labels correctly as PARTIAL/UNRESOLVED itself. One data curiosity, not an error: Serpent Staff (item 2797, a plasma energy weapon) has `ammo_types = {Bullet_Default}` in our own `items` table — the pack faithfully reports this, but it's a pre-existing seed oddity (all staff weapons apparently share the placeholder "Bullet_Default" ammo type), not something introduced by the pack.

## Recommendation

Phase 4 is further along than the checklist implies for three of its four bullets — auto-attack mapping, bandolier, and reload are done and DB-driven, not gaps. Real remaining work is narrow: (1) implement ammo-mode toggle buffs as effect scripts (needs combat-systems-advisor for the effect registry, since these are damage/penetration modifiers, not inventory operations), and (2) decide + implement TechComp→damage scaling (also combat-systems-advisor territory, item data already carries `tech_comp` so no schema work is needed). The starter-loadout/starting-world conflict should go back to the user before anyone touches Phase 5 — it's a project-direction question, not something either agent should resolve unilaterally.

Handoffs: **combat-systems-advisor** for ammo-mode-toggle effect scripts and TechComp damage scaling; **mission-systems-advisor** has no action item here (items_event_sets in this context is the live auto-attack table, distinct from the item_use content-trigger mechanism which is a separate, actually-legacy path).

## Files referenced (absolute paths)

- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\data\items_weapons_ammo.json`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\data\weapons_full_recovered_variants.json`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\data\starter_loadouts.json`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\docs\SOURCE_POLICY.md`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\docs\IMPLEMENTATION_CHECKLIST.md`
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\docs\QA_TESTS.md`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Items\Seed\items.sql`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Items\Seed\items_event_sets.sql`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Items\Tables\items.sql`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Items\Tables\items_event_sets.sql`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Archetypes\Seed\char_creation.sql`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Archetypes\Seed\char_creation_abilities.sql`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\spawner\abilities.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\abilities\use_ability\weapon_redirect.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\abilities\resolve.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\abilities\use_ability\handle.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\abilities\use_ability\auto_reload.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\cell_methods\player\world\reload.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\cell_methods\inventory\bandolier\ammo_change.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\world_entry\methods\inventory\ammo.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\entity\src\cell_entity\bandolier.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\character_create.rs`
- `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\effects\registry.rs`
- `C:\Users\Steve\source\projects\Cimmeria\db\resources\Effects\Seed\effects.sql` (effect 747, line 7477)
